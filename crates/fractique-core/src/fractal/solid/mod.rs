use super::pixel::{self, PROBE};
use super::{Draw, Field, Param, Sample, Spec};
use crate::math::{cos, exp, fmod, sin, tan};
use crate::rng::Rng;
use crate::scene::{Camera, Fractal};
use crate::tone;

super::family! {
    mandelbox,
    mandelbulb,
    menger_sponge,
}

const TURN: Param = Param::real("turn", 30.0, -180.0, 180.0);
const TILT: Param = Param::real("tilt", 20.0, -90.0, 90.0);
const FIELD_OF_VIEW: Param = Param::real("field_of_view", 40.0, 5.0, 120.0);
const LIGHT_TURN: Param = Param::real("light_turn", -40.0, -180.0, 180.0);
const LIGHT_TILT: Param = Param::real("light_tilt", 40.0, -90.0, 90.0);
const SHADOW: Param = Param::real("shadow", 0.7, 0.0, 1.0);
const FOG: Param = Param::real("fog", 0.3, 0.0, 1.0);
const GLOW: Param = Param::real("glow", 0.25, 0.0, 1.0);
const COLOR_DENSITY: Param = Param::real("color_density", 1.0, 0.01, 100.0).log();
const COLOR_SHIFT: Param = Param::real("color_shift", 0.0, 0.0, 2.0);
const LIGHT_COLOR: Param = Param::real("light_color", 0.0, 0.0, 1.0);
const ISOMETRIC: Param = Param::whole("isometric", 0.0, 0.0, 1.0);

const STEPS: u32 = 160;
// The lamps of the light in the colors of the light, after those printed on Rezonanca's Diskont posters: their
// strengths, the reach of the core as a share of the ball around the solid, how fast the light runs along the
// palette, and the place the far haze takes on it.
const KEY: f64 = 1.9;
const FILL: f64 = 0.3;
const RIM: f64 = 1.3;
const CORE: f64 = 1.6;
const AMBIENT: f64 = 0.05;
const SHEEN: f64 = 0.8;
const SOFT: f64 = 3.0;
const AMBER: f64 = 0.6;
const AMBER_TRAP: f64 = 0.17;
const CORE_REACH: f64 = 0.33;
const CORE_HAZE: f64 = 1.2;
const INK: f64 = 0.75;
const HAZE_AT: f64 = 0.1;
// The light runs up to here on the palette: its last colors are left for the core.
const INK_TOP: f64 = 0.85;
const SHADOW_STEPS: u32 = 20;
const PROBES: u32 = 5;
// The side, in pixels, of the cells whose rays share one look ahead.
const CELL: f64 = 4.0;
// A lit surface of a solid is rough all over: only its sharper steps are worth the grid of samples.
const SMOOTH: f32 = 0.2;
// The pixel of a picture 1080 high at zoom 1, for a sample asked for without a size.
const USUAL_PIXEL: f64 = 4.0 / 1080.0;

type Point = [f64; 3];

#[inline]
fn dot(a: Point, b: Point) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

#[inline]
fn along(from: Point, direction: Point, by: f64) -> Point {
    [
        from[0] + direction[0] * by,
        from[1] + direction[1] * by,
        from[2] + direction[2] * by,
    ]
}

#[inline]
fn unit(v: Point) -> Point {
    let length = dot(v, v).sqrt();
    if length > 0.0 {
        [v[0] / length, v[1] / length, v[2] / length]
    } else {
        [0.0, 0.0, 1.0]
    }
}

fn cross(a: Point, b: Point) -> Point {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

pub(super) trait Solid: Sync {
    // The share of the estimate a ray walks: under 1 where the estimate can be too long.
    const STRIDE: f64;

    // The radius of a ball around the origin that holds the solid.
    fn reach(&self) -> f64;

    // No more than the distance to the solid, and a value from about 0 to 1 for the color of the nearest part.
    fn distance(&self, point: Point) -> (f64, f64);
}

pub(super) struct Marcher<S> {
    solid: S,
    eye: Point,
    // Half the width of the view on the plane through the middle, for parallel rays; 0 for rays from the eye.
    parallel: f64,
    light_color: f64,
    // How far the light of the core reaches.
    core: f64,
    right: Point,
    up: Point,
    forward: Point,
    spread: f64,
    light: Point,
    shadow: f64,
    fog: f64,
    glow: f64,
    density: f64,
    shift: f64,
    zoom: f64,
    reach: f64,
    usual_pixel: f64,
}

struct Hit {
    point: Point,
    depth: f64,
    fine: f64,
    trap: f64,
}

impl<S: Solid> Marcher<S> {
    // `params`: turn, tilt, distance, field of view, the light's turn and tilt, shadow, fog, glow, color density
    // and shift, how far the light chooses the color, and whether the rays are parallel.
    pub(super) fn new(solid: S, params: &[f64], camera: &Camera) -> Self {
        let (turn, tilt) = (params[0].to_radians(), params[1].to_radians());
        let (light_turn, light_tilt) = (params[4].to_radians(), params[5].to_radians());
        let back = [cos(tilt) * sin(turn), sin(tilt), cos(tilt) * cos(turn)];
        let right = [cos(turn), 0.0, -sin(turn)];
        let up = cross(back, right);
        let forward = [-back[0], -back[1], -back[2]];
        let aside = cos(light_tilt) * sin(light_turn);
        let toward = cos(light_tilt) * cos(light_turn);
        let light = unit(along(
            along(along([0.0; 3], right, aside), up, sin(light_tilt)),
            back,
            toward,
        ));
        let reach = solid.reach();
        let spread = tan(params[3].to_radians() * 0.5) * 0.5;
        let isometric = params.get(12).copied().unwrap_or(0.0) >= 1.0;
        Self {
            solid,
            eye: along([0.0; 3], back, params[2]),
            parallel: if isometric { spread * params[2] } else { 0.0 },
            light_color: params.get(11).copied().unwrap_or(0.0),
            core: reach * CORE_REACH,
            right,
            up,
            forward,
            spread,
            light,
            shadow: params[6],
            fog: params[7],
            glow: params[8],
            density: params[9],
            shift: params[10],
            zoom: camera.zoom,
            reach,
            usual_pixel: USUAL_PIXEL / camera.zoom,
        }
    }

    #[inline]
    fn place(&self, trap: f64) -> f64 {
        self.shift + self.density * trap
    }

    #[inline]
    fn ray(&self, x: f64, y: f64) -> (Point, Point) {
        if self.parallel > 0.0 {
            let from = along(
                along(self.eye, self.right, x * self.parallel),
                self.up,
                y * self.parallel,
            );
            return (from, self.forward);
        }
        let (across, above) = (x * self.spread, y * self.spread);
        (
            self.eye,
            unit(along(along(self.forward, self.right, across), self.up, above)),
        )
    }

    // How wide a ray's footprint is at `depth`, for a pixel whose rays part by `cone`.
    #[inline]
    fn width(&self, depth: f64, cone: f64) -> f64 {
        if self.parallel > 0.0 {
            cone * dot(self.eye, self.eye).sqrt()
        } else {
            depth * cone
        }
    }

    // The depths at which a ray enters and leaves the ball around the solid, and how deep it cuts into it.
    #[inline]
    fn crossing(&self, origin: Point, direction: Point) -> Option<(f64, f64, f64)> {
        let toward = dot(origin, direction);
        let outside = dot(origin, origin) - self.reach * self.reach;
        let root = toward * toward - outside;
        if root <= 0.0 {
            return None;
        }
        let root = root.sqrt();
        (root > toward).then(|| ((-toward - root).max(0.0), root - toward, root / self.reach))
    }

    // The ray's walk from the depth `from`: where it meets the solid, or how near it came (as an angle), the
    // color there and how deep the ray cut into the ball.
    #[inline]
    fn walk(&self, (origin, direction): (Point, Point), cone: f64, from: f64) -> Result<Hit, (f64, f64, f64)> {
        let Some((enters, leaves, through)) = self.crossing(origin, direction) else {
            return Err((f64::INFINITY, 0.0, 0.0));
        };
        let floor = self.reach * 1e-12;
        let mut depth = enters.max(from);
        let (mut nearest, mut nearest_trap) = (f64::INFINITY, 0.0);
        let mut last = Hit {
            point: origin,
            depth,
            fine: floor,
            trap: 0.0,
        };
        for _ in 0..STEPS {
            let point = along(origin, direction, depth);
            let (distance, trap) = self.solid.distance(point);
            let fine = self.width(depth, cone).max(floor);
            last = Hit {
                point,
                depth,
                fine,
                trap,
            };
            if distance < fine {
                return Ok(last);
            }
            let angle = distance / self.width(depth, 1.0).max(floor);
            if angle < nearest {
                (nearest, nearest_trap) = (angle, trap);
            }
            depth += distance * S::STRIDE;
            if depth > leaves {
                return Err((nearest, nearest_trap, through));
            }
        }
        Ok(last)
    }

    // The depth up to which every ray within `angle` of this one is clear of the solid, and 0 where they all
    // pass it by.
    fn clear(&self, (origin, direction): (Point, Point), angle: f64) -> f64 {
        let Some((enters, leaves, _)) = self.crossing(origin, direction) else {
            return 0.0;
        };
        let mut depth = enters;
        for _ in 0..STEPS {
            let distance = self.solid.distance(along(origin, direction, depth)).0 * S::STRIDE;
            let aside = self.width(depth, angle);
            let room = distance - aside;
            if room < aside {
                return depth;
            }
            depth += if self.parallel > 0.0 {
                room
            } else {
                room / (1.0 + angle)
            };
            if depth > leaves {
                return 0.0;
            }
        }
        depth
    }

    #[inline]
    fn normal(&self, at: Point, step: f64) -> Point {
        let probe = |x: f64, y: f64, z: f64| {
            self.solid
                .distance([at[0] + x * step, at[1] + y * step, at[2] + z * step])
                .0
        };
        let (a, b, c, d) = (
            probe(1.0, -1.0, -1.0),
            probe(-1.0, -1.0, 1.0),
            probe(-1.0, 1.0, -1.0),
            probe(1.0, 1.0, 1.0),
        );
        unit([a - b - c + d, -a - b + c + d, -a + b - c + d])
    }

    // How open the surface is: 1 on a face in the open, toward 0 in a pit.
    #[inline]
    fn open(&self, from: Point, normal: Point, span: f64) -> f64 {
        let (mut closed, mut weight) = (0.0, 1.0);
        for probe in 1..=PROBES {
            let out = span * f64::from(probe);
            let free = self.solid.distance(along(from, normal, out)).0;
            closed += weight * (out - free).max(0.0) / out;
            weight *= 0.6;
        }
        (1.0 - 0.75 * closed).clamp(0.0, 1.0)
    }

    // The share of the light that reaches the point: 0 behind something, with a soft edge.
    #[inline]
    fn lit(&self, from: Point, fine: f64) -> f64 {
        let toward = dot(from, self.light);
        let root = toward * toward - dot(from, from) + self.reach * self.reach;
        let leaves = if root > 0.0 { root.sqrt() - toward } else { 0.0 };
        let (mut lit, mut depth) = (1.0f64, fine * 2.0);
        for _ in 0..SHADOW_STEPS {
            if depth > leaves {
                break;
            }
            let distance = self.solid.distance(along(from, self.light, depth)).0;
            if distance < fine * 0.5 {
                return 0.0;
            }
            lit = lit.min(6.0 * distance / depth);
            depth += distance.max(fine);
        }
        lit.clamp(0.0, 1.0)
    }
}

impl<S: Solid> Marcher<S> {
    fn lit_sample(&self, x: f64, y: f64, pixel: f64, from: f64) -> Option<Sample> {
        let ray = self.ray(x, y);
        let direction = ray.1;
        let hit = match self.walk(ray, pixel * self.spread, from) {
            Ok(hit) => hit,
            Err((nearest, trap, through)) => {
                if self.light_color > 0.0 {
                    return self.core_haze(ray, through);
                }
                // A halo about a fiftieth of the picture wide, gone where the ray only grazes the ball.
                let width = 0.08 * self.spread / self.zoom;
                let near = nearest / width;
                let fall = 1.0 / (1.0 + near * near);
                let light = 0.45 * self.glow * fall * fall * (through * 3.0).min(1.0);
                return (light > 0.004).then(|| Sample {
                    at: self.place(trap),
                    light,
                });
            }
        };
        // In the colors of the light the surface is read a little wider than a pixel: the light turns softly.
        let normal = self.normal(hit.point, hit.fine * (1.0 + SOFT * self.light_color));
        let above = along(hit.point, normal, hit.fine * 2.0);
        // The pits are measured at a hundredth of the picture, so that a view looks alike at any size and zoom.
        let span = (self.width(hit.depth, self.spread) * 0.04 / self.zoom).max(hit.fine * 2.0);
        let open = self.open(above, normal, span);
        let facing = dot(normal, self.light);
        let mut direct = facing.max(0.0);
        if direct > 0.0 && self.shadow > 0.0 {
            direct *= 1.0 - self.shadow * (1.0 - self.lit(above, hit.fine));
        }
        let half = unit([
            self.light[0] - direction[0],
            self.light[1] - direction[1],
            self.light[2] - direction[2],
        ]);
        let mut shine = dot(normal, half).max(0.0);
        for _ in 0..5 {
            shine *= shine;
        }
        let sky = 0.5 + 0.5 * normal[1];
        let rim = 0.5 - 0.5 * facing;
        let linear = open * (0.16 + 0.2 * sky + 0.08 * rim) + direct * (0.72 + 0.5 * shine);
        let near = (dot(self.eye, self.eye).sqrt() - self.reach).max(0.0);
        let far = (hit.depth - near) / self.reach;
        let haze = exp(-1.6 * self.fog * far);
        let light = (linear * haze).sqrt();
        if self.light_color > 0.0 {
            // The light of five lamps read through the palette, as an ink from its first stop (no light) to its
            // last: a key, a fill from the other side, a rim along the outline, a core in the middle of the solid
            // and an amber glow where the orbit came near the middle.
            let lit = if direct > 0.0 { direct } else { 0.0 };
            let fill = open * (0.5 - 0.5 * facing);
            let outline = 1.0 + dot(direction, normal);
            let rim = open * outline * outline * 0.25;
            let core = exp(-dot(hit.point, hit.point) / (2.0 * self.core * self.core)) * self.glow;
            let amber = if hit.trap < AMBER_TRAP { open } else { 0.0 };
            let lux = KEY * lit + FILL * fill + RIM * rim + AMBER * amber + AMBIENT * open;
            // The core lights the pits as well as the faces, and the air it fills on the way shows too.
            let (origin, _) = self.ray(x, y);
            let toward = -dot(origin, direction);
            let passed = if toward > 0.0 && toward < hit.depth {
                let apart = dot(origin, origin) - toward * toward;
                CORE_HAZE * self.glow * exp(-apart.max(0.0) / (2.0 * self.core * self.core))
            } else {
                0.0
            };
            let glow = CORE * core + passed;
            // The pits are darkened twice, at a hundredth of the picture and at a fifth of that, so the fine relief
            // shows as well as the deep wells.
            let fine = self.open(above, normal, (span * 0.2).max(hit.fine * 2.0));
            let shut = open * fine;
            let sheen = SHEEN * shine * lit;
            let ink = (1.0 - exp(-INK * (lux * (0.15 + 0.85 * shut * shut) + sheen + glow))) * INK_TOP * haze
                + (1.0 - haze) * HAZE_AT;
            let turn = self.place(hit.trap) * 0.5;
            let own = 1.0 - (2.0 * (turn - turn.floor()) - 1.0).abs();
            return Some(Sample {
                at: own + (ink - own) * self.light_color,
                light: light + (1.0 - light) * self.light_color,
            });
        }
        Some(Sample {
            at: self.place(hit.trap),
            light,
        })
    }
}

impl<S: Solid> Marcher<S> {
    // The light of the core seen through the gaps: the closer a ray passes the middle, the more of it.
    fn core_haze(&self, (origin, direction): (Point, Point), through: f64) -> Option<Sample> {
        let toward = dot(origin, direction);
        let apart = (dot(origin, origin) - toward * toward).max(0.0);
        let lux = CORE_HAZE * self.glow * exp(-apart / (2.0 * self.core * self.core)) * (through * 3.0).min(1.0);
        let light = lux.min(1.0) * self.light_color;
        (light > 0.004).then(|| Sample {
            at: 1.0 - exp(-INK * KEY * lux),
            light,
        })
    }
}

impl<S: Solid> Field for Marcher<S> {
    fn sample(&self, x: f64, y: f64) -> Option<Sample> {
        self.lit_sample(x, y, self.usual_pixel, 0.0)
    }

    fn sample_within(&self, x: f64, y: f64, pixel: f64) -> Option<Sample> {
        self.lit_sample(x, y, pixel, 0.0)
    }

    fn smooth(&self) -> f32 {
        SMOOTH
    }

    fn framed<'a>(&'a self, corners: [(f64, f64); 4], pixel: f64) -> Option<Box<dyn Field + 'a>> {
        let low = |pick: fn(&(f64, f64)) -> f64| corners.iter().map(pick).fold(f64::INFINITY, f64::min);
        let high = |pick: fn(&(f64, f64)) -> f64| corners.iter().map(pick).fold(f64::NEG_INFINITY, f64::max);
        let cell = CELL * pixel;
        let corner = (low(|c| c.0), low(|c| c.1));
        let columns = ((high(|c| c.0) - corner.0) / cell) as usize + 1;
        let rows = ((high(|c| c.1) - corner.1) / cell) as usize + 1;
        // Every ray of a cell is within this angle of the ray through its middle.
        let angle = self.spread * cell * 0.708;
        let mut clear = vec![0f32; columns * rows];
        tone::for_each_row(&mut clear, columns, |row, depths| {
            for (column, depth) in depths.iter_mut().enumerate() {
                let direction = self.ray(
                    corner.0 + (column as f64 + 0.5) * cell,
                    corner.1 + (row as f64 + 0.5) * cell,
                );
                // Rounded down, so that the 32 bits never say more than was found.
                *depth = (self.clear(direction, angle) * 0.9999) as f32;
            }
        });
        Some(Box::new(Framed {
            marcher: self,
            clear,
            corner,
            cell,
            columns,
            rows,
        }))
    }
}

// A marcher that knows, for every few pixels of one picture, how deep the rays there are clear of the solid.
struct Framed<'a, S> {
    marcher: &'a Marcher<S>,
    clear: Vec<f32>,
    corner: (f64, f64),
    cell: f64,
    columns: usize,
    rows: usize,
}

impl<S: Solid> Field for Framed<'_, S> {
    fn sample(&self, x: f64, y: f64) -> Option<Sample> {
        self.sample_within(x, y, self.marcher.usual_pixel)
    }

    fn sample_within(&self, x: f64, y: f64, pixel: f64) -> Option<Sample> {
        let (column, row) = ((x - self.corner.0) / self.cell, (y - self.corner.1) / self.cell);
        let inside = column >= 0.0 && row >= 0.0 && (column as usize) < self.columns && (row as usize) < self.rows;
        let from = if inside {
            f64::from(self.clear[row as usize * self.columns + column as usize])
        } else {
            0.0
        };
        self.marcher.lit_sample(x, y, pixel, from)
    }

    fn smooth(&self) -> f32 {
        SMOOTH
    }
}

// What a probe of a view shows: how much of it the solid fills, how bright it is, how the light varies over it
// and from cell to cell, how far the colors run, and how often two points a pixel or two apart differ.
struct Seen {
    cover: f64,
    light: f64,
    contrast: f64,
    busy: f64,
    colors: f64,
    grain: f64,
}

// A cell darker than this is the halo or the dark: not the solid.
const SOLID_LIGHT: f64 = 0.16;
const DIFFERENT: f64 = 0.1;

fn seen(field: &dyn Field, camera: &Camera) -> Seen {
    let cells = pixel::probe(field, camera);
    let solid: Vec<&Sample> = cells
        .iter()
        .flatten()
        .filter(|sample| sample.light > SOLID_LIGHT)
        .collect();
    let count = solid.len().max(1) as f64;
    let light = solid.iter().map(|sample| sample.light).sum::<f64>() / count;
    let spread = solid
        .iter()
        .map(|sample| (sample.light - light) * (sample.light - light))
        .sum::<f64>()
        / count;
    let mut places: Vec<f64> = solid.iter().map(|sample| sample.at).collect();
    places.sort_by(f64::total_cmp);
    let colors = if places.len() < 16 {
        0.0
    } else {
        places[places.len() * 9 / 10] - places[places.len() / 10]
    };
    let level = |cell: usize| cells[cell].as_ref().map_or(0.0, |sample| sample.light);
    let (mut pairs, mut busy) = (0usize, 0usize);
    for y in 0..PROBE {
        for x in 0..PROBE {
            let at = y * PROBE + x;
            if x + 1 < PROBE {
                pairs += 1;
                busy += usize::from((level(at) - level(at + 1)).abs() > DIFFERENT);
            }
            if y + 1 < PROBE {
                pairs += 1;
                busy += usize::from((level(at) - level(at + PROBE)).abs() > DIFFERENT);
            }
        }
    }

    // Two points a pixel and a half of a picture 1080 high apart, as that picture would draw them.
    let frame = pixel::Frame::new(camera, PROBE, PROBE);
    let fine = USUAL_PIXEL / camera.zoom;
    let (mut close, mut grain) = (0usize, 0usize);
    for y in (1..PROBE).step_by(2) {
        for x in (1..PROBE).step_by(2) {
            let (wx, wy) = frame.world(x as f64, y as f64);
            let light = |sample: Option<Sample>| sample.map_or(0.0, |sample| sample.light);
            let here = light(field.sample_within(wx, wy, fine));
            let beside = light(field.sample_within(wx + 1.5 * fine, wy + 0.5 * fine, fine));
            if here > SOLID_LIGHT || beside > SOLID_LIGHT {
                close += 1;
                grain += usize::from((here - beside).abs() > DIFFERENT);
            }
        }
    }
    Seen {
        cover: solid.len() as f64 / cells.len() as f64,
        light,
        contrast: spread.sqrt(),
        busy: busy as f64 / pairs as f64,
        colors,
        grain: grain as f64 / close.max(1) as f64,
    }
}

impl Seen {
    // A frame that is filled, lit and neither flat nor noise.
    fn flaws(&self, most_grain: f64) -> f64 {
        let short = |value: f64, least: f64| ((least - value) / least).max(0.0);
        short(self.cover, 0.3)
            + short(self.light, 0.4)
            + short(self.contrast, 0.09)
            + short(self.busy, 0.03)
            + short(self.colors, 0.03)
            + (self.busy - 0.42).max(0.0)
            + 3.0 * (self.grain - most_grain).max(0.0)
    }

    // The color density that runs the palette about `turns` times over what the view shows.
    fn density(&self, turns: f64) -> f64 {
        (turns / self.colors.max(0.02)).clamp(0.2, 40.0)
    }
}

// The fractal of these parameters with the colors spread over what its view shows, and how many flaws the view
// has: 0 for a picture. `most_grain` is the share of neighbouring points that may differ: above it is noise.
pub(super) fn fitted(
    spec: &Spec,
    colors: &str,
    camera: Camera,
    mut params: Vec<(&'static str, f64)>,
    turns: f64,
    most_grain: f64,
) -> (Fractal, f64) {
    let Draw::Pixel(field) = &spec.draw else {
        unreachable!("a solid is drawn per pixel")
    };
    let plain = spec.fractal(colors, camera, &params);
    let seen = seen(&*field(&spec.values(&plain), &plain.camera), &plain.camera);
    params.push(("color_density", (seen.density(turns) * 100.0).round() / 100.0));
    (spec.fractal(colors, plain.camera, &params), seen.flaws(most_grain))
}

// The first attempt without a flaw, or the one with the fewest.
pub(super) fn picked(attempts: u64, attempt: impl Fn(u64) -> (Fractal, f64)) -> Fractal {
    let (mut best, mut least) = (None, f64::INFINITY);
    for index in 0..attempts {
        let (fractal, flaws) = attempt(index);
        if flaws == 0.0 {
            return fractal;
        }
        if flaws < least {
            (best, least) = (Some(fractal), flaws);
        }
    }
    best.expect("there is an attempt")
}

fn value(fractal: &Fractal, param: &Param) -> f64 {
    fractal.params.get(param.name).copied().unwrap_or(param.default)
}

fn set(fractal: &mut Fractal, param: &Param, to: f64) {
    let rounded = (to.clamp(param.min, param.max) * 1e4).round() / 1e4;
    fractal.params.insert(param.name.to_owned(), rounded);
}

fn around(angle: f64) -> f64 {
    let turned = fmod(angle + 180.0, 360.0);
    if turned < 0.0 { turned + 180.0 } else { turned - 180.0 }
}

// The same solid from a little aside, a little nearer or farther, under a light that moved, in shifted colors.
pub(super) fn moved(fractal: &Fractal, distance: &Param, rng: &mut Rng, strength: f64) -> Fractal {
    let mut out = fractal.clone();
    let turn = around(value(&out, &TURN) + rng.range(-120.0, 120.0) * strength);
    set(&mut out, &TURN, turn.round());
    let tilt = value(&out, &TILT) + rng.range(-50.0, 50.0) * strength;
    set(&mut out, &TILT, tilt.clamp(-80.0, 80.0).round());
    let away = value(&out, distance) * exp(rng.range(-0.4, 0.4) * strength);
    set(&mut out, distance, away);
    let light = around(value(&out, &LIGHT_TURN) + rng.range(-90.0, 90.0) * strength);
    set(&mut out, &LIGHT_TURN, light.round());
    let shift = fmod(value(&out, &COLOR_SHIFT) + rng.range(0.15, 0.6) * strength, 2.0);
    set(&mut out, &COLOR_SHIFT, shift);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Ball;

    impl Solid for Ball {
        const STRIDE: f64 = 0.5;

        fn reach(&self) -> f64 {
            2.0
        }

        fn distance(&self, point: Point) -> (f64, f64) {
            (dot(point, point).sqrt() - 1.0, 0.5)
        }
    }

    fn ball(params: [f64; 11]) -> Marcher<Ball> {
        Marcher::new(Ball, &params, &Camera::default())
    }

    #[test]
    fn a_ball_is_a_lit_disc_of_the_size_the_eye_sees() {
        let field = ball([0.0, 0.0, 4.0, 40.0, -40.0, 40.0, 0.0, 0.0, 0.0, 1.0, 0.25]);
        // The ball's outline is where the ray is tangent: at 4 the angle whose sine is 1/4.
        let edge = (1.0f64 / 15.0).sqrt() / (tan(20f64.to_radians()) * 0.5);
        let middle = field.sample(0.0, 0.0).expect("the ball is in the middle");
        assert_eq!(middle.at, 0.75);
        assert!(middle.light > 0.5, "{}", middle.light);
        assert!(field.sample(edge * 0.99, 0.0).is_some());
        assert!(field.sample(edge * 1.01, 0.0).is_none());
        assert!(field.sample(0.0, -edge * 1.01).is_none());
        let lit = field.sample(-0.5 * edge, 0.5 * edge).unwrap().light;
        let unlit = field.sample(0.5 * edge, -0.5 * edge).unwrap().light;
        assert!(lit > unlit + 0.2, "{lit} {unlit}");
    }

    #[test]
    fn the_eye_turns_around_the_solid_and_the_zoom_narrows_the_view() {
        let from_the_side = ball([90.0, 0.0, 4.0, 40.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0]);
        assert!((from_the_side.eye[0] - 4.0).abs() < 1e-12 && from_the_side.eye[2].abs() < 1e-12);
        let from_above = ball([0.0, 90.0, 4.0, 40.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0]);
        assert!((from_above.eye[1] - 4.0).abs() < 1e-12);
        assert!(from_above.sample(0.0, 0.0).is_some());
        let near = ball([0.0, 0.0, 2.0, 40.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0]);
        let far = ball([0.0, 0.0, 8.0, 40.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0]);
        assert!(near.sample(1.2, 0.0).is_some() && far.sample(1.2, 0.0).is_none());
    }

    #[test]
    fn a_coarser_picture_takes_the_surface_sooner() {
        let field = ball([0.0, 0.0, 4.0, 40.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0]);
        let direction = [0.0, 0.0, -1.0];
        let fine = field.walk((field.eye, direction), 1e-4, 0.0).ok().unwrap();
        let coarse = field.walk((field.eye, direction), 1e-1, 0.0).ok().unwrap();
        assert!(coarse.depth < fine.depth && (fine.depth - 3.0).abs() < 1e-3);
    }

    #[test]
    fn the_look_ahead_stops_short_of_the_surface_for_every_ray_of_its_cell() {
        let field = ball([20.0, 10.0, 4.0, 40.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0]);
        let (cell, mut met) = (0.05, 0);
        let angle = field.spread * cell * 0.708;
        for row in -30..30 {
            for column in -30..30 {
                let (x, y) = (f64::from(column) * cell, f64::from(row) * cell);
                let clear = field.clear(field.ray(x + 0.5 * cell, y + 0.5 * cell), angle);
                for (right, up) in [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0), (0.3, 0.8)] {
                    let ray = field.ray(x + right * cell, y + up * cell);
                    let Ok(hit) = field.walk(ray, 1e-6, 0.0) else {
                        continue;
                    };
                    // A ray along the rim runs out of steps before it has passed: that is no meeting.
                    if field.solid.distance(hit.point).0 >= hit.fine {
                        continue;
                    }
                    met += 1;
                    assert!(clear <= hit.depth, "{clear} past {}", hit.depth);
                    assert!(field.walk(ray, 1e-6, clear).is_ok());
                }
            }
        }
        assert!(met > 500);
    }

    #[test]
    fn a_picture_made_ready_is_the_picture() {
        let field = ball([20.0, 10.0, 4.0, 40.0, -40.0, 40.0, 0.7, 0.3, 0.25, 1.0, 0.0]);
        let pixel = 0.01;
        let ready = field
            .framed([(-2.0, -2.0), (2.0, -2.0), (-2.0, 2.0), (2.0, 2.0)], pixel)
            .expect("a solid looks ahead");
        for step in 0..400 {
            let (x, y) = (f64::from(step % 20) * 0.19 - 1.9, f64::from(step / 20) * 0.19 - 1.9);
            match (field.sample_within(x, y, pixel), ready.sample_within(x, y, pixel)) {
                (Some(plain), Some(ahead)) => assert!((plain.light - ahead.light).abs() < 0.02, "{x} {y}"),
                (None, None) => {}
                _ => panic!("{x} {y} is the ball for one and not for the other"),
            }
        }
        assert!(ready.sample_within(9.0, 9.0, pixel).is_none());
    }
}

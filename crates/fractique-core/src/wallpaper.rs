use crate::math::{PI, TAU, cos, pow, sin};
use crate::rng::Rng;
use crate::scene::{SCENE_VERSION, Scene, Shape, Sheet, Sheets, Swell};

const WALLPAPER_DOMAIN: u64 = 0x4000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Layout {
    Bands,
    Discs,
    Waves,
    Ribbons,
    Burst,
    Arcs,
    Tiles,
    Bars,
    Fan,
    Slats,
    Planes,
    Steps,
    Frames,
    Lines,
    Garden,
    Ripples,
    Pebbles,
    Horizon,
    Twist,
    Mosaic,
}

impl Layout {
    pub const ALL: [(&'static str, Layout); 20] = [
        ("bands", Layout::Bands),
        ("discs", Layout::Discs),
        ("waves", Layout::Waves),
        ("ribbons", Layout::Ribbons),
        ("burst", Layout::Burst),
        ("arcs", Layout::Arcs),
        ("tiles", Layout::Tiles),
        ("bars", Layout::Bars),
        ("fan", Layout::Fan),
        ("slats", Layout::Slats),
        ("planes", Layout::Planes),
        ("steps", Layout::Steps),
        ("frames", Layout::Frames),
        ("lines", Layout::Lines),
        ("garden", Layout::Garden),
        ("ripples", Layout::Ripples),
        ("pebbles", Layout::Pebbles),
        ("horizon", Layout::Horizon),
        ("twist", Layout::Twist),
        ("mosaic", Layout::Mosaic),
    ];

    pub fn parse(name: &str) -> Option<Self> {
        Self::ALL.iter().find(|(n, _)| *n == name).map(|(_, l)| *l)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Material {
    Relief,
    Paper,
    Glass,
    Glow,
    Steel,
    Carbon,
    Clay,
    Chrome,
    Candy,
    Velvet,
}

impl Material {
    pub const ALL: [(&'static str, Material); 10] = [
        ("relief", Material::Relief),
        ("paper", Material::Paper),
        ("glass", Material::Glass),
        ("glow", Material::Glow),
        ("steel", Material::Steel),
        ("carbon", Material::Carbon),
        ("clay", Material::Clay),
        ("chrome", Material::Chrome),
        ("candy", Material::Candy),
        ("velvet", Material::Velvet),
    ];

    pub fn parse(name: &str) -> Option<Self> {
        Self::ALL.iter().find(|(n, _)| *n == name).map(|(_, m)| *m)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Request {
    pub layout: Option<Layout>,
    pub material: Option<Material>,
    pub ratio: f64,
    pub dark: bool,
    pub hue: Option<f64>,
    // A gradient of hues from the back of the wallpaper to its front; it takes the place of `hue`.
    pub hues: Option<Hues>,
}

pub const MOST_HUES: usize = 5;

// Up to five hues in degrees, in order.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hues {
    hues: [f64; MOST_HUES],
    count: usize,
}

impl Hues {
    // None for no hue or one that is not a number; more than five keep the first five.
    pub fn new(hues: &[f64]) -> Option<Self> {
        if hues.is_empty() || !hues.iter().all(|hue| hue.is_finite()) {
            return None;
        }
        let count = hues.len().min(MOST_HUES);
        let mut kept = [0.0; MOST_HUES];
        for (slot, hue) in kept.iter_mut().zip(hues) {
            *slot = hue.rem_euclid(360.0);
        }
        Some(Self { hues: kept, count })
    }

    pub fn list(&self) -> &[f64] {
        &self.hues[..self.count]
    }

    // The hue a share of the way along, each step taking the shorter way round the circle.
    pub fn at(&self, share: f64) -> f64 {
        if self.count == 1 {
            return self.hues[0];
        }
        let place = share.clamp(0.0, 1.0) * (self.count - 1) as f64;
        let step = (place.floor() as usize).min(self.count - 2);
        let (from, to) = (self.hues[step], self.hues[step + 1]);
        let turn = (to - from + 540.0).rem_euclid(360.0) - 180.0;
        (from + turn * (place - step as f64)).rem_euclid(360.0)
    }
}

// OKLCH (Björn Ottosson's OKLab) to sRGB: equal steps in it look equal, so generated palettes stay even.
fn oklch(lightness: f64, chroma: f64, hue: f64) -> [f64; 3] {
    let angle = hue.to_radians();
    let (a, b) = (chroma * cos(angle), chroma * sin(angle));
    let cube = |v: f64| v * v * v;
    let l = cube(lightness + 0.396_337_777_4 * a + 0.215_803_757_3 * b);
    let m = cube(lightness - 0.105_561_345_8 * a - 0.063_854_172_8 * b);
    let s = cube(lightness - 0.089_484_177_5 * a - 1.291_485_548_0 * b);
    [
        4.076_741_662_1 * l - 3.307_711_591_3 * m + 0.230_969_929_2 * s,
        -1.268_438_004_6 * l + 2.609_757_401_1 * m - 0.341_319_396_5 * s,
        -0.004_196_086_3 * l - 0.703_418_614_7 * m + 1.707_614_701_0 * s,
    ]
    .map(|v| {
        let v = v.clamp(0.0, 1.0);
        let encoded = if v <= 0.003_130_8 {
            12.92 * v
        } else {
            1.055 * pow(v, 1.0 / 2.4) - 0.055
        };
        (encoded.clamp(0.0, 1.0) * 1000.0 + 0.5).floor() / 1000.0
    })
}

fn boxed(center: [f64; 2], half: [f64; 2], corners: [f64; 4], rotation: f64) -> Shape {
    Shape::Box {
        center,
        half,
        corners,
        rotation,
    }
}

fn disc(center: [f64; 2], radius: f64) -> Shape {
    boxed(center, [radius, radius], [radius; 4], 0.0)
}

fn sign(rng: &mut Rng) -> f64 {
    if rng.next_bool() { 1.0 } else { -1.0 }
}

fn count(rng: &mut Rng, lo: u64, hi: u64) -> usize {
    (lo + rng.below(hi - lo + 1)) as usize
}

fn bands(rng: &mut Rng, ex: f64, ey: f64) -> Vec<Vec<Shape>> {
    const OVER: f64 = 0.06;
    let rows = (2.0 * ey / rng.range(0.72, 0.95) + 0.5).floor().max(2.0);
    let band = 2.0 * ey / rows;
    let (mut stack, mut tabs) = (Vec::new(), Vec::new());
    let mut lean = rng.next_bool();
    for row in 0..rows as usize {
        let y = -ey + band * (row as f64 + 0.5);
        let pick = rng.next_f64();
        let corners = if pick < 0.7 {
            lean = !lean;
            if lean {
                [band, 0.0, band, 0.0]
            } else {
                [0.0, band, 0.0, band]
            }
        } else if pick < 0.85 {
            let r = band / 2.0;
            if rng.next_bool() {
                [0.0, r, r, 0.0]
            } else {
                [r, 0.0, 0.0, r]
            }
        } else {
            [0.0; 4]
        };
        stack.push(vec![boxed([0.0, y], [ex + OVER, band / 2.0], corners, 0.0)]);
        if rng.next_f64() < 0.45 {
            let wide = (ex + OVER) / 2.0;
            let round = band.min(2.0 * wide);
            let corners = match (rng.next_bool(), rng.next_bool()) {
                (true, true) => [0.0, round, 0.0, 0.0],
                (true, false) => [0.0, 0.0, round, 0.0],
                (false, true) => [round, 0.0, 0.0, 0.0],
                (false, false) => [0.0, 0.0, 0.0, round],
            };
            let x = if corners[1] + corners[2] > 0.0 { -wide } else { wide };
            tabs.push(vec![boxed([x, y], [wide, band / 2.0], corners, 0.0)]);
        }
    }
    for i in (1..stack.len()).rev() {
        stack.swap(i, rng.below(i as u64 + 1) as usize);
    }
    stack.extend(tabs);
    stack
}

fn discs(rng: &mut Rng, ex: f64, ey: f64) -> Vec<Vec<Shape>> {
    let n = count(rng, 5, 8);
    let side = sign(rng);
    let snap = |v: f64| (v * 4.0 + 0.5).floor() / 4.0;
    (0..n)
        .map(|i| {
            let t = (i as f64 + 1.0) / n as f64;
            let radius = snap(rng.range(1.1, 1.9) * (1.0 - 0.45 * t) * ex.max(1.0)).max(0.25);
            let x = snap(side * ex * rng.range(-0.2, 1.1) * (1.0 - 0.6 * t) + side * 0.25 * i as f64);
            let y = snap(ey * (0.15 + 0.85 * t) + radius * rng.range(0.1, 0.5));
            vec![disc([x, y], radius)]
        })
        .collect()
}

fn waves(rng: &mut Rng, ex: f64, ey: f64) -> Vec<Vec<Shape>> {
    let n = count(rng, 4, 7);
    let tilt = rng.range(-10.0, 10.0);
    let top = rng.range(-0.35, 0.05);
    (0..n)
        .map(|i| {
            let t = i as f64 / (n - 1) as f64;
            vec![Shape::Wave {
                level: ey * (top + (0.82 - top) * t),
                swells: vec![
                    Swell {
                        height: rng.range(0.06, 0.22),
                        length: rng.range(1.6, 3.6) * ex,
                        shift: rng.range(0.0, TAU),
                    },
                    Swell {
                        height: rng.range(0.015, 0.07),
                        length: rng.range(0.7, 1.4) * ex,
                        shift: rng.range(0.0, TAU),
                    },
                ],
                thickness: 0.0,
                rotation: tilt,
            }]
        })
        .collect()
}

fn ribbons(rng: &mut Rng, _ex: f64, ey: f64) -> Vec<Vec<Shape>> {
    let n = count(rng, 3, 6);
    let angle = sign(rng) * rng.range(25.0, 65.0);
    (0..n)
        .map(|i| {
            let t = (i as f64 + 0.5) / n as f64;
            vec![Shape::Wave {
                level: ey * 1.1 * (2.0 * t - 1.0) + rng.range(-0.15, 0.15),
                swells: vec![Swell {
                    height: rng.range(0.1, 0.4),
                    length: rng.range(2.5, 6.0),
                    shift: rng.range(0.0, TAU),
                }],
                thickness: rng.range(0.3, 0.7),
                rotation: angle,
            }]
        })
        .collect()
}

fn burst(rng: &mut Rng, ex: f64, ey: f64) -> Vec<Vec<Shape>> {
    let center = [ex * rng.range(-0.5, 0.5), ey * rng.range(0.0, 0.75)];
    let rings = count(rng, 2, 4);
    let petals = count(rng, 6, 14);
    let longest = rng.range(1.3, 2.2);
    let slim = rng.range(0.55, 0.95);
    let pointed = rng.next_bool();
    let step = 360.0 / petals as f64;
    let mut out = Vec::new();
    for ring in 0..rings {
        let length = longest * (1.0 - 0.27 * ring as f64);
        let width = (2.0 * length * sin(PI / petals as f64) * slim).min(length * 0.6);
        for parity in 0..2 {
            let shapes: Vec<Shape> = (parity..petals)
                .step_by(2)
                .map(|i| {
                    let rotation = step * (i as f64 + 0.5 * ring as f64);
                    if pointed {
                        Shape::Petal {
                            base: center,
                            length,
                            width,
                            rotation,
                        }
                    } else {
                        let angle = rotation.to_radians();
                        let half = length / 2.0;
                        boxed(
                            [center[0] + sin(angle) * half, center[1] - cos(angle) * half],
                            [width / 2.0, half],
                            [width / 2.0; 4],
                            rotation,
                        )
                    }
                })
                .collect();
            if !shapes.is_empty() {
                out.push(shapes);
            }
        }
    }
    out
}

fn arcs(rng: &mut Rng, ex: f64, ey: f64) -> Vec<Vec<Shape>> {
    let center = [ex * (rng.below(3) as f64 - 1.0), ey * rng.range(0.5, 1.05)];
    let n = count(rng, 4, 8);
    let widest = rng.range(1.6, 2.6) * ex.max(1.0);
    let gap = widest / (n as f64 + 1.0);
    let open = rng.next_bool();
    (0..n)
        .map(|i| {
            vec![Shape::Ring {
                center,
                radius: widest - gap * i as f64,
                thickness: gap * rng.range(0.9, 1.5),
                span: if open { rng.range(120.0, 300.0) } else { 360.0 },
                rotation: if open { rng.range(-70.0, 70.0) } else { 0.0 },
            }]
        })
        .collect()
}

fn tiles(rng: &mut Rng, ex: f64, ey: f64) -> Vec<Vec<Shape>> {
    let columns = count(rng, 2, 4);
    let cell = 2.0 * ex / columns as f64;
    let rows = (2.0 * ey / cell).ceil() as usize;
    let top = -(rows as f64 * cell) / 2.0;
    let mut out = Vec::new();
    for row in 0..rows {
        let mut pair = [Vec::new(), Vec::new()];
        for column in 0..columns {
            let pick = rng.next_f64();
            let turn = rng.below(4) as usize;
            let mut corners = [0.0; 4];
            if pick < 0.15 {
                continue;
            } else if pick < 0.75 {
                corners[turn] = cell;
            } else if pick < 0.9 {
                corners = [cell / 2.0; 4];
            } else {
                corners[turn] = cell / 2.0;
                corners[(turn + 1) % 4] = cell / 2.0;
            }
            let center = [-ex + cell * (column as f64 + 0.5), top + cell * (row as f64 + 0.5)];
            pair[column % 2].push(boxed(center, [cell / 2.0; 2], corners, 0.0));
        }
        out.extend(pair.into_iter().filter(|shapes| !shapes.is_empty()));
    }
    if out.is_empty() {
        out.push(vec![boxed([0.0, 0.0], [cell / 2.0; 2], [cell, 0.0, 0.0, 0.0], 0.0)]);
    }
    out
}

fn bars(rng: &mut Rng, _ex: f64, _ey: f64) -> Vec<Vec<Shape>> {
    let n = count(rng, 5, 9);
    let rotation = sign(rng) * rng.range(30.0, 60.0);
    let angle = rotation.to_radians();
    let (c, s) = (cos(angle), sin(angle));
    let width = rng.range(0.35, 0.7);
    let spacing = width * rng.range(0.75, 1.1);
    (0..n)
        .map(|i| {
            let across = (i as f64 - (n - 1) as f64 / 2.0) * spacing;
            let along = rng.range(-0.8, 0.8);
            vec![boxed(
                [c * along - s * across, s * along + c * across],
                [rng.range(0.8, 2.2), width / 2.0],
                [width / 2.0; 4],
                rotation,
            )]
        })
        .collect()
}

fn fan(rng: &mut Rng, ex: f64, ey: f64) -> Vec<Vec<Shape>> {
    let n = count(rng, 5, 9);
    let side = sign(rng);
    let pivot = [ex * side * rng.range(0.3, 1.0), ey * rng.range(0.8, 1.05)];
    let (from, to) = (rng.range(-15.0, 10.0), rng.range(55.0, 80.0));
    let width = rng.range(0.5, 0.9);
    let round = width * rng.range(0.2, 0.5);
    let length = rng.range(1.1, 1.7) * ey.max(ex);
    (0..n)
        .map(|i| {
            let t = i as f64 / (n - 1) as f64;
            let rotation = -side * (to + (from - to) * t);
            let angle = rotation.to_radians();
            let long = length * (1.0 - 0.25 * t);
            vec![boxed(
                [pivot[0] + sin(angle) * long / 2.0, pivot[1] - cos(angle) * long / 2.0],
                [width / 2.0, long / 2.0],
                [round; 4],
                rotation,
            )]
        })
        .collect()
}

// Flowers of five to eight petals, larger at the back, scattered over the picture.
fn garden(rng: &mut Rng, ex: f64, ey: f64) -> Vec<Vec<Shape>> {
    let n = count(rng, 3, 6);
    let petals = count(rng, 5, 8);
    let pointed = rng.next_f64() < 0.6;
    (0..n)
        .map(|i| {
            let t = i as f64 / (n - 1) as f64;
            let length = rng.range(0.75, 1.15) * (1.0 - 0.45 * t) * ex.max(1.0);
            let center = [ex * rng.range(-0.85, 0.85), ey * rng.range(-0.8, 0.85)];
            let turn = rng.range(0.0, 360.0);
            let width = (2.0 * length * sin(PI / petals as f64) * 0.9).min(length * 0.7);
            (0..petals)
                .map(|p| {
                    let rotation = turn + 360.0 * p as f64 / petals as f64;
                    if pointed {
                        Shape::Petal {
                            base: center,
                            length,
                            width,
                            rotation,
                        }
                    } else {
                        let angle = rotation.to_radians();
                        boxed(
                            [
                                center[0] + sin(angle) * length / 2.0,
                                center[1] - cos(angle) * length / 2.0,
                            ],
                            [width / 2.0, length / 2.0],
                            [width / 2.0; 4],
                            rotation,
                        )
                    }
                })
                .collect()
        })
        .collect()
}

// Rings spreading from two or three points, as on water, each wider and thinner than the one inside it.
fn ripples(rng: &mut Rng, ex: f64, ey: f64) -> Vec<Vec<Shape>> {
    let sources = count(rng, 2, 3);
    let mut out = Vec::new();
    for _ in 0..sources {
        let center = [ex * rng.range(-0.9, 0.9), ey * rng.range(-0.85, 0.85)];
        let rings = count(rng, 3, 5);
        let first = rng.range(0.18, 0.35);
        let step = rng.range(0.22, 0.4) * ex.max(1.0);
        for k in 0..rings {
            let radius = first + step * k as f64;
            out.push(vec![Shape::Ring {
                center,
                radius,
                thickness: (0.16 - 0.025 * k as f64).max(0.05),
                span: 360.0,
                rotation: 0.0,
            }]);
        }
    }
    out.push(vec![disc(
        [ex * rng.range(-0.5, 0.5), ey * rng.range(-0.5, 0.5)],
        rng.range(0.12, 0.2),
    )]);
    out
}

// Rounded stones of many sizes, heaped towards the foot of the picture.
fn pebbles(rng: &mut Rng, ex: f64, ey: f64) -> Vec<Vec<Shape>> {
    let n = count(rng, 9, 15);
    (0..n)
        .map(|i| {
            let t = i as f64 / (n - 1) as f64;
            let size = rng.range(0.35, 0.75) * (1.0 - 0.35 * t) * ex.max(1.0);
            let half = [size, size * rng.range(0.6, 0.9)];
            let low = rng.next_f64();
            let center = [ex * rng.range(-1.05, 1.05), ey * (1.05 - 1.7 * low * low)];
            vec![boxed(
                center,
                half,
                [half[1] * rng.range(0.75, 1.0); 4],
                rng.range(-35.0, 35.0),
            )]
        })
        .collect()
}

// A low sun behind hills that lie one before another, the nearest the darkest.
fn horizon(rng: &mut Rng, ex: f64, ey: f64) -> Vec<Vec<Shape>> {
    let sun = [ex * rng.range(-0.6, 0.6), ey * rng.range(-0.45, 0.05)];
    let radius = rng.range(0.45, 0.8) * ex.max(1.0);
    let mut out = vec![vec![disc(sun, radius)]];
    if rng.next_bool() {
        out.push(vec![Shape::Ring {
            center: sun,
            radius: radius * 1.45,
            thickness: radius * 0.12,
            span: 360.0,
            rotation: 0.0,
        }]);
    }
    let hills = count(rng, 3, 5);
    for i in 0..hills {
        let t = i as f64 / (hills - 1) as f64;
        out.push(vec![Shape::Wave {
            level: sun[1] + radius * 0.6 + (ey - sun[1]) * 0.55 * t,
            swells: vec![
                Swell {
                    height: rng.range(0.12, 0.3) * (1.0 - 0.4 * t),
                    length: rng.range(2.4, 4.5) * ex,
                    shift: rng.range(0.0, TAU),
                },
                Swell {
                    height: rng.range(0.02, 0.06),
                    length: rng.range(0.6, 1.2) * ex,
                    shift: rng.range(0.0, TAU),
                },
            ],
            thickness: 0.0,
            rotation: rng.range(-4.0, 4.0),
        }]);
    }
    out
}

// Rounded squares one inside another, each turned a little further: a spiral of frames.
fn twist(rng: &mut Rng, ex: f64, ey: f64) -> Vec<Vec<Shape>> {
    let n = count(rng, 6, 10);
    let center = [ex * rng.range(-0.35, 0.35), ey * rng.range(-0.3, 0.3)];
    let largest = rng.range(1.2, 1.6) * ex.max(1.0);
    let shrink = rng.range(0.8, 0.88);
    let step = sign(rng) * rng.range(7.0, 15.0);
    let round = rng.range(0.1, 0.35);
    (0..n)
        .map(|i| {
            let half = largest * pow(shrink, i as f64);
            vec![boxed(center, [half, half], [half * round; 4], step * i as f64)]
        })
        .collect()
}

// Diamonds in a lattice, some missing, some rounded, the rows alternating in two sheets.
fn mosaic(rng: &mut Rng, ex: f64, ey: f64) -> Vec<Vec<Shape>> {
    let columns = count(rng, 3, 5);
    let cell = 2.0 * ex / columns as f64;
    let half = cell / 2.0 * rng.range(0.86, 0.96);
    let round = half * rng.range(0.0, 0.5);
    let rows = (2.0 * ey / (cell / 2.0)).ceil() as usize + 2;
    let mut out = Vec::new();
    for row in 0..rows {
        let shift = if row % 2 == 0 { 0.0 } else { cell / 2.0 };
        let y = -ey - cell / 2.0 + row as f64 * cell / 2.0;
        let shapes: Vec<Shape> = (0..=columns)
            .filter(|_| rng.next_f64() > 0.2)
            .map(|column| {
                let x = -ex + shift + cell * column as f64;
                boxed([x, y], [half / 2.0_f64.sqrt(); 2], [round; 4], 45.0)
            })
            .collect();
        if !shapes.is_empty() {
            out.push(shapes);
        }
    }
    out
}

fn turned(center: [f64; 2], angle: f64) -> [f64; 2] {
    let (c, s) = (cos(angle.to_radians()), sin(angle.to_radians()));
    [center[0] * c - center[1] * s, center[0] * s + center[1] * c]
}

fn slats(rng: &mut Rng, ex: f64, ey: f64) -> Vec<Vec<Shape>> {
    let n = count(rng, 6, 12);
    let rotation = if rng.next_f64() < 0.4 {
        0.0
    } else {
        sign(rng) * rng.range(12.0, 35.0)
    };
    let reach = (ex * ex + ey * ey).sqrt();
    let span = if rotation == 0.0 { ex } else { reach };
    let width = 2.0 * span / n as f64;
    let mut out: Vec<Vec<Shape>> = (0..n)
        .map(|i| {
            let tall = reach * rng.range(0.35, 1.5);
            let x = -span + width * (i as f64 + 0.5);
            vec![boxed(
                turned([x, reach - tall / 2.0], rotation),
                [width * 0.5, tall / 2.0],
                [0.0; 4],
                rotation,
            )]
        })
        .collect();
    for i in (1..out.len()).rev() {
        out.swap(i, rng.below(i as u64 + 1) as usize);
    }
    out
}

fn planes(rng: &mut Rng, _ex: f64, ey: f64) -> Vec<Vec<Shape>> {
    let n = count(rng, 4, 8);
    let lean = sign(rng);
    (0..n)
        .map(|i| {
            let t = (i as f64 + 0.5) / n as f64;
            vec![Shape::Wave {
                level: ey * (1.7 * t - 0.75) + rng.range(-0.1, 0.1),
                swells: Vec::new(),
                thickness: 0.0,
                rotation: lean * rng.range(8.0, 50.0) * if rng.next_f64() < 0.25 { -1.0 } else { 1.0 },
            }]
        })
        .collect()
}

fn steps(rng: &mut Rng, ex: f64, ey: f64) -> Vec<Vec<Shape>> {
    let n = count(rng, 5, 8);
    let side = sign(rng);
    let (wide, high) = (ex * rng.range(1.5, 1.9), ey * rng.range(1.1, 1.6));
    (0..n)
        .map(|i| {
            let shrink = 1.0 - i as f64 / (n as f64 + 0.6);
            vec![boxed(
                [side * ex, ey],
                [
                    wide * shrink * rng.range(0.88, 1.08),
                    high * shrink * rng.range(0.88, 1.08),
                ],
                [0.0; 4],
                0.0,
            )]
        })
        .collect()
}

fn frames(rng: &mut Rng, ex: f64, ey: f64) -> Vec<Vec<Shape>> {
    let n = count(rng, 5, 9);
    let center = [ex * rng.range(-0.3, 0.3), ey * rng.range(-0.1, 0.45)];
    let rotation = if rng.next_bool() { 45.0 } else { 0.0 };
    let widest = ex * rng.range(1.2, 1.8);
    let stretch = rng.range(1.0, 1.5);
    let round = if rng.next_bool() { 0.0 } else { 0.12 };
    (0..n)
        .map(|i| {
            let half = widest * (1.0 - i as f64 / (n as f64 + 0.4));
            vec![boxed(center, [half, half * stretch], [half * round; 4], rotation)]
        })
        .collect()
}

fn lines(rng: &mut Rng, _ex: f64, ey: f64) -> Vec<Vec<Shape>> {
    let groups = count(rng, 3, 5);
    let rotation = match rng.below(4) {
        0 => 0.0,
        1 => 90.0,
        _ => sign(rng) * rng.range(20.0, 70.0),
    };
    (0..groups)
        .map(|_| {
            let k = count(rng, 2, 6);
            let pitch = rng.range(0.08, 0.16);
            let thickness = rng.range(0.012, 0.035);
            let middle = ey * rng.range(-0.9, 0.9);
            (0..k)
                .map(|j| Shape::Wave {
                    level: middle + pitch * (j as f64 - (k - 1) as f64 / 2.0),
                    swells: Vec::new(),
                    thickness,
                    rotation,
                })
                .collect()
        })
        .collect()
}

fn dress(
    rng: &mut Rng,
    material: Material,
    (dark, picked, hues): (bool, Option<f64>, Option<Hues>),
    shapes: Vec<Vec<Shape>>,
    ex: f64,
    ey: f64,
) -> Sheets {
    let n = shapes.len() as f64;
    let hue = rng.range(0.0, 360.0);
    let turn = sign(rng)
        * if rng.next_f64() < 0.15 {
            rng.range(150.0, 200.0)
        } else {
            rng.range(20.0, 90.0)
        };
    let grey = rng.next_f64() < 0.35;
    let chroma = rng.range(0.02, 0.09);
    let opacity = rng.range(0.38, 0.6);
    let jitter: Vec<f64> = shapes.iter().map(|_| rng.range(-0.012, 0.012)).collect();
    let light = rng.range(25.0, 155.0);
    const SOBER: [f64; 6] = [255.0, 225.0, 155.0, 20.0, 65.0, 0.0];
    const ACCENTS: [f64; 6] = [50.0, 27.0, 255.0, 85.0, 195.0, 50.0];
    let sober = SOBER[rng.below(6) as usize] + rng.range(-8.0, 8.0);
    let sober_chroma = if rng.next_f64() < 0.3 {
        0.004
    } else {
        rng.range(0.015, 0.045)
    };
    let accent = ACCENTS[rng.below(6) as usize] + rng.range(-6.0, 6.0);
    let accented = 1 + rng.below(2) as usize;
    let picked = hues.map(|hues| hues.at(0.0)).or(picked);
    let (hue, grey, sober, sober_chroma, accent) = match picked.filter(|hue| hue.is_finite()) {
        Some(hue) => (hue, false, hue, sober_chroma.max(0.03), hue),
        None => (hue, grey, sober, sober_chroma, accent),
    };
    // The hue a share of the way from the back to the front: the gradient asked for, or the hue turning.
    let along = |share: f64| match hues {
        Some(hues) if hues.count > 1 => hues.at(share),
        _ => hue + turn * share,
    };
    let still = |share: f64| match hues {
        Some(hues) if hues.count > 1 => hues.at(share),
        _ => hue,
    };
    let accent = hues.filter(|hues| hues.count > 1).map_or(accent, |hues| hues.at(1.0));
    let pick = |on_dark: f64, on_light: f64| if dark { on_dark } else { on_light };

    let (base, mut sheets) = match material {
        Material::Relief => {
            let chroma = if grey { 0.004 } else { chroma };
            let lightness = pick(0.24, 0.8);
            (
                oklch(lightness, chroma, hue),
                Sheets {
                    light,
                    rim: pick(0.42, 0.5),
                    rim_width: 0.006,
                    bevel: pick(0.3, 0.07),
                    shadow: pick(0.45, 0.26),
                    shadow_offset: 0.025,
                    shadow_blur: 0.12,
                    gradient: pick(0.2, 0.05),
                    stack: jitter
                        .iter()
                        .enumerate()
                        .map(|(i, by)| Sheet {
                            color: oklch(lightness + by, chroma, still((i as f64 + 1.0) / n)),
                            color_to: None,
                            opacity: 1.0,
                            shapes: Vec::new(),
                        })
                        .collect(),
                },
            )
        }
        Material::Paper => {
            let (from, to) = (pick(0.2, 0.94), pick(0.78, 0.42));
            (
                oklch(from, 0.04, hue),
                Sheets {
                    light,
                    rim: pick(0.1, 0.16),
                    rim_width: 0.01,
                    bevel: 0.08,
                    shadow: 0.42,
                    shadow_offset: 0.06,
                    shadow_blur: 0.28,
                    gradient: 0.14,
                    stack: (0..shapes.len())
                        .map(|i| {
                            let t = (i as f64 + 1.0) / n;
                            let (l, c, h) = (from + (to - from) * t * 0.9, 0.04 + 0.11 * t, along(t));
                            Sheet {
                                color: oklch(l, c, h),
                                color_to: Some(oklch(l - 0.07, c, h + 12.0)),
                                opacity: 1.0,
                                shapes: Vec::new(),
                            }
                        })
                        .collect(),
                },
            )
        }
        Material::Glass => (
            oklch(pick(0.16, 0.96), pick(0.03, 0.015), hue),
            Sheets {
                light,
                rim: 0.28,
                rim_width: 0.012,
                bevel: 0.1,
                shadow: 0.16,
                shadow_offset: 0.04,
                shadow_blur: 0.2,
                gradient: 0.08,
                stack: (0..shapes.len())
                    .map(|i| {
                        let h = match hues {
                            Some(hues) if hues.count > 1 => hues.at((i as f64 + 1.0) / n),
                            _ => hue + turn * (i as f64 + 1.0) / n,
                        };
                        Sheet {
                            color: oklch(pick(0.66, 0.72), 0.15, h),
                            color_to: Some(oklch(pick(0.74, 0.8), 0.12, h + 25.0)),
                            opacity,
                            shapes: Vec::new(),
                        }
                    })
                    .collect(),
            },
        ),
        Material::Glow => (
            oklch(pick(0.13, 0.9), 0.03, hue),
            Sheets {
                light,
                rim: pick(0.9, 0.6),
                rim_width: 0.02,
                bevel: pick(0.25, 0.06),
                shadow: pick(0.5, 0.18),
                shadow_offset: 0.03,
                shadow_blur: 0.18,
                gradient: pick(0.25, 0.06),
                stack: (0..shapes.len())
                    .map(|i| {
                        let t = (i as f64 + 1.0) / n;
                        Sheet {
                            color: oklch(pick(0.17 + 0.1 * t, 0.9 - 0.05 * t), pick(0.06, 0.04), along(t)),
                            color_to: None,
                            opacity: 1.0,
                            shapes: Vec::new(),
                        }
                    })
                    .collect(),
            },
        ),
        Material::Steel => {
            let (from, to) = (pick(0.18, 0.74), pick(0.34, 0.58));
            (
                oklch(pick(0.15, 0.78), sober_chroma, sober),
                Sheets {
                    light,
                    rim: pick(0.3, 0.38),
                    rim_width: 0.0035,
                    bevel: pick(0.28, 0.08),
                    shadow: pick(0.55, 0.32),
                    shadow_offset: 0.03,
                    shadow_blur: 0.1,
                    gradient: pick(0.22, 0.07),
                    stack: (0..shapes.len())
                        .map(|i| Sheet {
                            color: {
                                let t = (i as f64 + 1.0) / n;
                                let h = if hues.is_some_and(|hues| hues.count > 1) {
                                    still(t)
                                } else {
                                    sober
                                };
                                oklch(from + (to - from) * t, sober_chroma, h)
                            },
                            color_to: None,
                            opacity: 1.0,
                            shapes: Vec::new(),
                        })
                        .collect(),
                },
            )
        }
        // Soft matt clay: pale colors, puffed edges, no shine, round shadows.
        Material::Clay => {
            let lightness = pick(0.42, 0.84);
            (
                oklch(pick(0.3, 0.92), 0.03, still(0.0)),
                Sheets {
                    light,
                    rim: 0.04,
                    rim_width: 0.004,
                    bevel: pick(0.4, 0.3),
                    shadow: pick(0.5, 0.3),
                    shadow_offset: 0.025,
                    shadow_blur: 0.2,
                    gradient: 0.08,
                    stack: (0..shapes.len())
                        .map(|i| {
                            let t = (i as f64 + 1.0) / n;
                            Sheet {
                                color: oklch(lightness + 0.06 * t + jitter[i], 0.07, along(t)),
                                color_to: None,
                                opacity: 1.0,
                                shapes: Vec::new(),
                            }
                        })
                        .collect(),
                },
            )
        }
        // Polished metal: a pale and a dark band across each sheet, a hard bevel, a thin bright edge; the hue only
        // tints it.
        Material::Chrome => (
            oklch(pick(0.16, 0.86), 0.015, still(0.0)),
            Sheets {
                light,
                rim: 0.7,
                rim_width: 0.004,
                bevel: 0.55,
                shadow: pick(0.55, 0.35),
                shadow_offset: 0.03,
                shadow_blur: 0.08,
                gradient: 0.6,
                stack: (0..shapes.len())
                    .map(|i| {
                        let t = (i as f64 + 1.0) / n;
                        let (l, h) = (pick(0.42, 0.6) + 0.1 * t + jitter[i] * 4.0, along(t));
                        Sheet {
                            color: oklch(l, 0.035, h),
                            color_to: Some(oklch(l + 0.25, 0.015, h)),
                            opacity: 1.0,
                            shapes: Vec::new(),
                        }
                    })
                    .collect(),
            },
        ),
        // Glossy sweets: strong colors, a bright bevel and a gradient across each sheet.
        Material::Candy => (
            oklch(pick(0.2, 0.95), 0.03, still(0.0)),
            Sheets {
                light,
                rim: 0.35,
                rim_width: 0.006,
                bevel: 0.45,
                shadow: pick(0.45, 0.25),
                shadow_offset: 0.03,
                shadow_blur: 0.12,
                gradient: 0.3,
                stack: (0..shapes.len())
                    .map(|i| {
                        let t = (i as f64 + 1.0) / n;
                        let (l, h) = (pick(0.62, 0.72) + 0.05 * t, along(t));
                        Sheet {
                            color: oklch(l, 0.16, h),
                            color_to: Some(oklch(l + 0.1, 0.12, h + 20.0)),
                            opacity: 1.0,
                            shapes: Vec::new(),
                        }
                    })
                    .collect(),
            },
        ),
        // Velvet: deep colors that drink the light, long soft shadows, a dark fold across each sheet.
        Material::Velvet => (
            oklch(pick(0.14, 0.32), 0.05, still(0.0)),
            Sheets {
                light,
                rim: 0.06,
                rim_width: 0.003,
                bevel: 0.12,
                shadow: 0.6,
                shadow_offset: 0.045,
                shadow_blur: 0.3,
                gradient: 0.35,
                stack: (0..shapes.len())
                    .map(|i| {
                        let t = (i as f64 + 1.0) / n;
                        let (l, h) = (pick(0.3, 0.42) + 0.08 * t, along(t));
                        Sheet {
                            color: oklch(l, 0.12, h),
                            color_to: Some(oklch(l - 0.1, 0.1, h - 15.0)),
                            opacity: 1.0,
                            shapes: Vec::new(),
                        }
                    })
                    .collect(),
            },
        ),
        Material::Carbon => (
            oklch(pick(0.12, 0.94), 0.004, sober),
            Sheets {
                light,
                rim: pick(0.26, 0.3),
                rim_width: 0.004,
                bevel: pick(0.3, 0.06),
                shadow: pick(0.6, 0.3),
                shadow_offset: 0.035,
                shadow_blur: 0.12,
                gradient: pick(0.2, 0.05),
                stack: (0..shapes.len())
                    .map(|i| {
                        let t = (i as f64 + 1.0) / n;
                        let color = if i + accented >= shapes.len() {
                            oklch(pick(0.62, 0.5), 0.18, accent)
                        } else {
                            oklch(pick(0.15 + 0.08 * t, 0.9 - 0.08 * t), 0.004, sober)
                        };
                        Sheet {
                            color,
                            color_to: None,
                            opacity: 1.0,
                            shapes: Vec::new(),
                        }
                    })
                    .collect(),
            },
        ),
    };
    for (sheet, shapes) in sheets.stack.iter_mut().zip(shapes) {
        sheet.shapes = shapes;
    }
    sheets.stack.insert(
        0,
        Sheet {
            color: base,
            color_to: None,
            opacity: 1.0,
            shapes: vec![boxed([0.0, 0.0], [ex + 1.0, ey + 1.0], [0.0; 4], 0.0)],
        },
    );
    sheets
}

pub fn chosen(seed: u64, request: &Request) -> (Layout, Material) {
    let mut choice = Rng::derive(seed, WALLPAPER_DOMAIN, 0);
    let any_layout = Layout::ALL[choice.below(Layout::ALL.len() as u64) as usize].1;
    let any_material = Material::ALL[choice.below(Material::ALL.len() as u64) as usize].1;
    (
        request.layout.unwrap_or(any_layout),
        request.material.unwrap_or(any_material),
    )
}

pub fn random_wallpaper(seed: u64, request: &Request) -> Scene {
    let ratio = if request.ratio.is_finite() {
        request.ratio.clamp(0.25, 4.0)
    } else {
        1.0
    };
    let (ex, ey) = if ratio >= 1.0 { (1.0, ratio) } else { (1.0 / ratio, 1.0) };
    let (layout, material) = chosen(seed, request);

    let mut rng = Rng::derive(seed, WALLPAPER_DOMAIN, 1);
    let shapes = match layout {
        Layout::Bands => bands(&mut rng, ex, ey),
        Layout::Discs => discs(&mut rng, ex, ey),
        Layout::Waves => waves(&mut rng, ex, ey),
        Layout::Ribbons => ribbons(&mut rng, ex, ey),
        Layout::Burst => burst(&mut rng, ex, ey),
        Layout::Arcs => arcs(&mut rng, ex, ey),
        Layout::Tiles => tiles(&mut rng, ex, ey),
        Layout::Bars => bars(&mut rng, ex, ey),
        Layout::Fan => fan(&mut rng, ex, ey),
        Layout::Slats => slats(&mut rng, ex, ey),
        Layout::Planes => planes(&mut rng, ex, ey),
        Layout::Steps => steps(&mut rng, ex, ey),
        Layout::Frames => frames(&mut rng, ex, ey),
        Layout::Lines => lines(&mut rng, ex, ey),
        Layout::Garden => garden(&mut rng, ex, ey),
        Layout::Ripples => ripples(&mut rng, ex, ey),
        Layout::Pebbles => pebbles(&mut rng, ex, ey),
        Layout::Horizon => horizon(&mut rng, ex, ey),
        Layout::Twist => twist(&mut rng, ex, ey),
        Layout::Mosaic => mosaic(&mut rng, ex, ey),
    };
    let sheets = dress(
        &mut Rng::derive(seed, WALLPAPER_DOMAIN, 2),
        material,
        (request.dark, request.hue, request.hues),
        shapes,
        ex,
        ey,
    );
    Scene {
        version: SCENE_VERSION,
        name: format!("wallpaper-{seed}"),
        seed,
        background: sheets.stack[0].color,
        sheets: Some(sheets),
        layers: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(layout: Layout, material: Material, ratio: f64, dark: bool) -> Request {
        Request {
            layout: Some(layout),
            material: Some(material),
            ratio,
            dark,
            hue: None,
            hues: None,
        }
    }

    #[test]
    fn every_combination_is_repeatable_and_valid() {
        for (_, layout) in Layout::ALL {
            for (_, material) in Material::ALL {
                for seed in 0..24 {
                    for ratio in [0.25, 0.46, 0.75, 1.0, 1.3, 1.6, 1.78, 2.17, 3.0, 4.0] {
                        let ask = request(layout, material, ratio, seed % 2 == 0);
                        let scene = random_wallpaper(seed, &ask);
                        assert_eq!(scene, random_wallpaper(seed, &ask));
                        scene
                            .validate()
                            .unwrap_or_else(|e| panic!("{layout:?} {material:?} {seed} {ratio}: {e}"));
                    }
                }
            }
        }
    }

    #[test]
    fn seeds_differ() {
        let ask = Request {
            layout: None,
            material: None,
            ratio: 2.0,
            dark: false,
            hue: None,
            hues: None,
        };
        assert_ne!(random_wallpaper(1, &ask), random_wallpaper(2, &ask));
        let tinted = Request {
            hue: Some(140.0),
            hues: None,
            ..ask
        };
        assert_ne!(random_wallpaper(1, &ask), random_wallpaper(1, &tinted));
        random_wallpaper(1, &tinted).validate().unwrap();
        // One hue given as a gradient is the same wallpaper; two make another, in every material.
        let one = Request {
            hue: None,
            hues: Hues::new(&[140.0]),
            ..ask
        };
        assert_eq!(random_wallpaper(1, &one), random_wallpaper(1, &tinted));
        for (_, material) in Material::ALL {
            let single = Request {
                material: Some(material),
                ..tinted
            };
            let both = Request {
                material: Some(material),
                hue: None,
                hues: Hues::new(&[140.0, 320.0]),
                ..ask
            };
            assert_ne!(random_wallpaper(3, &single), random_wallpaper(3, &both), "{material:?}");
            random_wallpaper(3, &both).validate().unwrap();
        }
    }

    #[test]
    fn a_gradient_of_hues_goes_the_short_way_round() {
        let hues = Hues::new(&[0.0, 120.0]).unwrap();
        assert!((hues.at(0.5) - 60.0).abs() < 1e-9);
        let across = Hues::new(&[350.0, 10.0]).unwrap();
        assert!(across.at(0.5).abs() < 1e-9 || (across.at(0.5) - 360.0).abs() < 1e-9);
        let three = Hues::new(&[10.0, 100.0, 200.0]).unwrap();
        assert!((three.at(1.0) - 200.0).abs() < 1e-9);
        assert!((three.at(0.75) - 150.0).abs() < 1e-9);
        assert_eq!(Hues::new(&[]), None);
        assert_eq!(Hues::new(&[f64::NAN]), None);
        assert_eq!(
            Hues::new(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0]).unwrap().list().len(),
            MOST_HUES
        );
    }

    #[test]
    fn light_and_dark_share_their_shapes() {
        let shapes =
            |scene: Scene| -> Vec<Vec<Shape>> { scene.sheets.unwrap().stack.into_iter().map(|s| s.shapes).collect() };
        for (_, layout) in Layout::ALL {
            assert_eq!(
                shapes(random_wallpaper(7, &request(layout, Material::Paper, 2.0, false))),
                shapes(random_wallpaper(7, &request(layout, Material::Paper, 2.0, true)))
            );
        }
    }

    #[test]
    fn colors_stay_in_range() {
        for hue in 0..36 {
            for lightness in [0.0, 0.2, 0.5, 0.9, 1.0] {
                for c in oklch(lightness, 0.2, f64::from(hue) * 10.0) {
                    assert!((0.0..=1.0).contains(&c));
                }
            }
        }
        assert_eq!(oklch(1.0, 0.0, 0.0), [1.0, 1.0, 1.0]);
    }
}

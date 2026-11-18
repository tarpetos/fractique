use super::pixel::{self, PROBE};
use super::{Field, Fractal, Param, Sample, Spec, random_palette};
use crate::math::{exp, fmod, ln, ln_1p, log10, pow};
use crate::rng::Rng;
use crate::scene::{Camera, Tone};

super::family! {
    burning_ship,
    julia,
    magnet,
    mandelbrot,
    multibrot,
    phoenix,
    tricorn,
}

pub(super) const ITERATIONS: Param = Param::whole("iterations", 300.0, 16.0, 100_000.0).log();
pub(super) const COLOR_DENSITY: Param = Param::real("color_density", 1.0, 0.01, 100.0).log();
pub(super) const COLOR_SHIFT: Param = Param::real("color_shift", 0.0, 0.0, 2.0);
const EDGE: Param = Param::real("edge", 0.35, 0.0, 1.0);
const FADE: Param = Param::real("fade", 0.6, 0.0, 1.0);

pub(super) const TONE: Tone = Tone {
    brightness: 0.92,
    gamma: 2.5,
    vibrancy: 1.0,
    highlight: 0.0,
    glow_radius: 0.006,
    glow_strength: 0.12,
    smoothing: 1.0,
};

// Far enough out that the smooth count is smooth: at radius 2 the bands still show.
const ESCAPE: f64 = 65_536.0;

// A deeper view needs more steps to tell the set from its surroundings.
pub(super) fn limit(iterations: f64, zoom: f64) -> u32 {
    let deeper = if zoom > 1.0 { 1.0 + 0.5 * log10(zoom) } else { 1.0 };
    (iterations * deeper).min(4e6) as u32
}

#[derive(Clone, Copy)]
struct Coloring {
    density: f64,
    shift: f64,
    // The distance to the set, in shorter sides of the picture, under which the color darkens.
    rim: f64,
    // The distance at which the color has faded to half, and 0 for colors that do not fade.
    reach: f64,
    per_side: f64,
    // The logarithm of the power the far orbit is raised to at every step.
    base: f64,
}

impl Coloring {
    fn new(density: f64, shift: f64, edge: f64, fade: f64, zoom: f64) -> Self {
        Self {
            density,
            shift,
            rim: 0.012 * edge * edge,
            reach: if fade > 0.0 { 0.02 / (fade * fade) } else { 0.0 },
            per_side: zoom / 4.0,
            base: core::f64::consts::LN_2,
        }
    }

    fn of_power(self, power: f64) -> Self {
        Self {
            base: ln(power),
            ..self
        }
    }

    // `steps` before |z|² first exceeded ESCAPE at `r2`, with |dz/dc| (or |dz/dz0|) squared in `d2`.
    #[inline]
    fn outside(&self, steps: u32, r2: f64, d2: f64) -> Sample {
        self.settled(steps, 0.5 * ln(r2), r2, d2)
    }

    // The same for an orbit whose logarithm of the distance (from infinity, or to the point it settles on) is
    // `half_log` in size and grows by the power at every step.
    #[inline]
    fn settled(&self, steps: u32, half_log: f64, r2: f64, d2: f64) -> Sample {
        let count = (f64::from(steps) + 1.0 - ln(half_log) / self.base).max(0.0);
        let mut light = 1.0;
        if d2 > 0.0 {
            let distance = (r2 / d2).sqrt() * half_log * self.per_side;
            if distance < self.rim {
                light = pow(distance / self.rim, 0.4);
            }
            if self.reach > 0.0 {
                light /= 1.0 + pow(distance / self.reach, 1.2);
            }
        }
        Sample {
            at: self.shift + self.density * ln_1p(count),
            light,
        }
    }
}

pub(super) type Complex = (f64, f64);

#[inline]
pub(super) fn mul(a: Complex, b: Complex) -> Complex {
    (a.0 * b.0 - a.1 * b.1, a.0 * b.1 + a.1 * b.0)
}

#[inline]
pub(super) fn div(a: Complex, b: Complex) -> Complex {
    let size = b.0 * b.0 + b.1 * b.1;
    ((a.0 * b.0 + a.1 * b.1) / size, (a.1 * b.0 - a.0 * b.1) / size)
}

#[inline]
pub(super) fn norm(a: Complex) -> f64 {
    a.0 * a.0 + a.1 * a.1
}

// The smooth counts of a probe, as the palette sees them at density 1, and how much of it is inside.
pub(super) struct Seen {
    pub(super) places: Vec<Option<f64>>,
    pub(super) inside: f64,
}

impl Seen {
    pub(super) fn of(samples: &[Option<Sample>]) -> Self {
        let places: Vec<Option<f64>> = samples
            .iter()
            .map(|sample| sample.as_ref().map(|sample| sample.at))
            .collect();
        let inside = places.iter().filter(|place| place.is_none()).count() as f64 / places.len() as f64;
        Self { places, inside }
    }

    // The places below which a tenth and nineteen twentieths of the outside lie.
    pub(super) fn span(&self) -> Option<(f64, f64)> {
        let mut outside: Vec<f64> = self.places.iter().flatten().copied().collect();
        if outside.len() < 16 {
            return None;
        }
        outside.sort_by(f64::total_cmp);
        Some((outside[outside.len() / 10], outside[outside.len() * 19 / 20]))
    }

    // The share of neighbouring cells that differ by more than `step` of the span: 0 is flat, 1 is noise.
    pub(super) fn busy(&self, side: usize, step: f64) -> f64 {
        let Some((low, high)) = self.span() else {
            return 0.0;
        };
        let unit = (high - low).max(1e-9) * step;
        let differ = |a: usize, b: usize| match (self.places[a], self.places[b]) {
            (Some(a), Some(b)) => (a - b).abs() > unit,
            (None, None) => false,
            _ => true,
        };
        let (mut pairs, mut busy) = (0usize, 0usize);
        for y in 0..side {
            for x in 0..side {
                let at = y * side + x;
                if x + 1 < side {
                    pairs += 1;
                    busy += usize::from(differ(at, at + 1));
                }
                if y + 1 < side {
                    pairs += 1;
                    busy += usize::from(differ(at, at + side));
                }
            }
        }
        busy as f64 / pairs as f64
    }
}

// What builds a kind for a probe from the five parameters the family shares, in the order of `Coloring::new`
// after the iterations.
pub(super) type Probed<'a> = dyn Fn(&[f64; 5], &Camera) -> Box<dyn Field> + 'a;

// How much a cell of a probe is worth a closer look: the edge of the set, or filaments beside it.
fn worth(seen: &Seen, x: usize, y: usize) -> f64 {
    let (mut inside, mut low, mut high) = (0, f64::INFINITY, f64::NEG_INFINITY);
    for ny in y - 1..=y + 1 {
        for nx in x - 1..=x + 1 {
            match seen.places[ny * PROBE + nx] {
                None => inside += 1,
                Some(place) => (low, high) = (low.min(place), high.max(place)),
            }
        }
    }
    match inside {
        9 => 0.0,
        0 if high - low < 0.25 => 0.0,
        0 => high - low,
        1..=6 => 3.0,
        _ => 1.0,
    }
}

// A walk toward the edge of the set: each step looks at the view and moves into one of its busiest cells.
pub(super) fn descend(rng: &mut Rng, steps: u64, whole: &Camera, field: &Probed) -> Camera {
    let mut camera = whole.clone();
    for _ in 0..steps {
        let seen = Seen::of(&pixel::probe(&*field(&[400.0, 1.0, 0.0, 0.0, 0.0], &camera), &camera));
        let cells: Vec<(usize, usize, f64)> = (4..PROBE - 4)
            .flat_map(|y| (4..PROBE - 4).map(move |x| (x, y)))
            .map(|(x, y)| (x, y, worth(&seen, x, y)))
            .filter(|(_, _, worth)| *worth > 0.0)
            .collect();
        let total: f64 = cells.iter().map(|(_, _, worth)| worth).sum();
        if cells.is_empty() {
            break;
        }
        let mut roll = rng.next_f64() * total;
        let (x, y, _) = *cells
            .iter()
            .find(|(_, _, worth)| {
                roll -= worth;
                roll < 0.0
            })
            .unwrap_or(&cells[0]);
        let place = |cell: usize| (cell as f64 + 0.5) / PROBE as f64 - 0.5;
        camera = camera.looking_at(place(x), place(y), rng.range(2.0, 5.0));
    }
    camera
}

// The colors spread over what the view shows, and enough steps for it: `None` for a view not worth a picture.
pub(super) fn fitted(seen: &Seen, zoom: f64, rng: &mut Rng) -> Option<[(&'static str, f64); 3]> {
    let (low, high) = seen.span()?;
    let busy = seen.busy(PROBE, 0.1);
    if seen.inside > 0.28 || !(0.16..=0.8).contains(&busy) || high - low < 0.05 {
        return None;
    }
    let density = (rng.range(1.2, 2.8) / (high - low)).clamp(0.05, 60.0);
    let shift = fmod(rng.range(0.0, 2.0) - density * low, 2.0);
    let deepest = seen.places.iter().flatten().fold(0.0f64, |a, b| a.max(*b));
    let steps = (3.0 * exp(deepest)).max(300.0) / (f64::from(limit(1000.0, zoom)) / 1000.0);
    Some([
        ("iterations", (steps / 10.0).ceil() * 10.0),
        ("color_density", density),
        ("color_shift", if shift < 0.0 { shift + 2.0 } else { shift }),
    ])
}

pub(super) fn moved(fractal: &Fractal, rng: &mut Rng, strength: f64) -> Fractal {
    let mut out = fractal.clone();
    let (right, down) = (rng.range(-0.5, 0.5) * strength, rng.range(-0.5, 0.5) * strength);
    out.camera = out.camera.looking_at(right, down, exp(rng.range(-0.8, 1.0) * strength));
    out.camera.rotation = (out.camera.rotation + rng.range(-40.0, 40.0) * strength).round();
    let shift = out.params.get("color_shift").copied().unwrap_or(COLOR_SHIFT.default);
    let shifted = fmod(shift + rng.range(0.2, 0.8) * strength, 2.0);
    out.params.insert("color_shift".to_owned(), shifted);
    out
}

// A candidate of a randomizer: a walk of up to `most` steps from `whole`, sometimes turned, with colors.
// It is judged by `fitted`, and refused as well when it is stripes or when under an eighth of it is lit.
pub(super) fn candidate(
    rng: &mut Rng,
    whole: &Camera,
    most: u64,
    field: &Probed,
) -> (Camera, &'static str, Option<[(&'static str, f64); 3]>) {
    let steps = 1 + rng.below(most);
    let mut camera = descend(rng, steps, whole, field);
    if rng.next_f64() < 0.4 {
        camera.rotation = rng.range(0.0, 360.0).round();
    }
    let colors = random_palette(rng);
    let params = judged(rng, &camera, field);
    (camera, colors, params)
}

// How far the places change along one direction only, from 0 for no direction to 1 for stripes. It is measured
// an eighth of a cell of the probe apart, where stripes too thin for the probe still show their direction.
fn streaked(field: &dyn Field, camera: &Camera) -> f64 {
    let frame = pixel::Frame::new(camera, PROBE, PROBE);
    let place = |x: f64, y: f64| {
        let (wx, wy) = frame.world(x, y);
        field.sample(wx, wy).map(|sample| sample.at)
    };
    let (mut even, mut odd, mut cells) = (0.0f64, 0.0f64, 0.0f64);
    for cell in 0..PROBE * PROBE {
        let (x, y) = ((cell % PROBE) as f64 + 0.5, (cell / PROBE) as f64 + 0.5);
        if let (Some(own), Some(right), Some(down)) = (place(x, y), place(x + 0.125, y), place(x, y + 0.125)) {
            let (dx, dy) = (right - own, down - own);
            let size = dx * dx + dy * dy;
            if size > 0.0 {
                even += (dx * dx - dy * dy) / size;
                odd += 2.0 * dx * dy / size;
                cells += 1.0;
            }
        }
    }
    if cells > 0.0 {
        (even * even + odd * odd).sqrt() / cells
    } else {
        0.0
    }
}

pub(super) fn judged(rng: &mut Rng, camera: &Camera, field: &Probed) -> Option<[(&'static str, f64); 3]> {
    let seen = Seen::of(&pixel::probe(&*field(&[700.0, 1.0, 0.0, 0.0, 0.0], camera), camera));
    let params = fitted(&seen, camera.zoom, rng)?;
    let shown = [params[0].1, params[1].1, params[2].1, EDGE.default, FADE.default];
    let lit = pixel::probe(&*field(&shown, camera), camera)
        .iter()
        .filter(|sample| sample.as_ref().is_some_and(|sample| sample.light > 0.2))
        .count();
    let streaked = streaked(&*field(&[700.0, 1.0, 0.0, 0.0, 0.0], camera), camera);
    (streaked < 0.5 && seen.busy(PROBE, 0.1) <= 0.65 && lit * 8 >= PROBE * PROBE).then_some(params)
}

pub(super) fn value(spec: &Spec, fractal: &Fractal, name: &str) -> f64 {
    let param = spec
        .params
        .iter()
        .find(|param| param.name == name)
        .expect("the kind has the parameter");
    fractal.params.get(name).copied().unwrap_or(param.default)
}

// A parameter moved by up to `reach` either way, kept in its range and to five decimals.
pub(super) fn nudged(spec: &Spec, fractal: &mut Fractal, name: &str, rng: &mut Rng, reach: f64) {
    let param = spec
        .params
        .iter()
        .find(|param| param.name == name)
        .expect("the kind has the parameter");
    let moved = (value(spec, fractal, name) + rng.range(-reach, reach)).clamp(param.min, param.max);
    fractal.params.insert(name.to_owned(), (moved * 1e5).round() / 1e5);
}

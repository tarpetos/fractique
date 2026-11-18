use super::{Param, Sample};
use crate::math::{ln, ln_1p, log10, pow};

super::family! {
    julia,
    mandelbrot,
}

const ITERATIONS: Param = Param::whole("iterations", 300.0, 16.0, 100_000.0).log();
const COLOR_DENSITY: Param = Param::real("color_density", 1.0, 0.01, 100.0).log();
const COLOR_SHIFT: Param = Param::real("color_shift", 0.0, 0.0, 2.0);
const EDGE: Param = Param::real("edge", 0.35, 0.0, 1.0);
const FADE: Param = Param::real("fade", 0.6, 0.0, 1.0);

// Far enough out that the smooth count is smooth: at radius 2 the bands still show.
const ESCAPE: f64 = 65_536.0;

// A deeper view needs more steps to tell the set from its surroundings.
fn limit(iterations: f64, zoom: f64) -> u32 {
    let deeper = if zoom > 1.0 { 1.0 + 0.5 * log10(zoom) } else { 1.0 };
    (iterations * deeper).min(4e6) as u32
}

struct Coloring {
    density: f64,
    shift: f64,
    // The distance to the set, in shorter sides of the picture, under which the color darkens.
    rim: f64,
    // The distance at which the color has faded to half, and 0 for colors that do not fade.
    reach: f64,
    per_side: f64,
}

impl Coloring {
    fn new(density: f64, shift: f64, edge: f64, fade: f64, zoom: f64) -> Self {
        Self {
            density,
            shift,
            rim: 0.012 * edge * edge,
            reach: if fade > 0.0 { 0.02 / (fade * fade) } else { 0.0 },
            per_side: zoom / 4.0,
        }
    }

    // `steps` before |z|² first exceeded ESCAPE at `r2`, with |dz/dc| (or |dz/dz0|) squared in `d2`.
    #[inline]
    fn outside(&self, steps: u32, r2: f64, d2: f64) -> Sample {
        let half_log = 0.5 * ln(r2);
        let count = (f64::from(steps) + 1.0 - ln(half_log) / core::f64::consts::LN_2).max(0.0);
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

// The smooth counts of a probe, as the palette sees them at density 1, and how much of it is inside.
struct Seen {
    places: Vec<Option<f64>>,
    inside: f64,
}

impl Seen {
    fn of(samples: &[Option<Sample>]) -> Self {
        let places: Vec<Option<f64>> = samples
            .iter()
            .map(|sample| sample.as_ref().map(|sample| sample.at))
            .collect();
        let inside = places.iter().filter(|place| place.is_none()).count() as f64 / places.len() as f64;
        Self { places, inside }
    }

    // The places below which a tenth and nineteen twentieths of the outside lie.
    fn span(&self) -> Option<(f64, f64)> {
        let mut outside: Vec<f64> = self.places.iter().flatten().copied().collect();
        if outside.len() < 16 {
            return None;
        }
        outside.sort_by(f64::total_cmp);
        Some((outside[outside.len() / 10], outside[outside.len() * 19 / 20]))
    }

    // The share of neighbouring cells that differ by more than `step` of the span: 0 is flat, 1 is noise.
    fn busy(&self, side: usize, step: f64) -> f64 {
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

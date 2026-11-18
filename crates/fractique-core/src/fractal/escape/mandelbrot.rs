use super::{COLOR_DENSITY, COLOR_SHIFT, Coloring, EDGE, ESCAPE, FADE, ITERATIONS, Seen, limit};
use crate::fractal::pixel::{self, PROBE};
use crate::fractal::{Draw, Family, Field, Preset, Sample, Spec, random_palette, stream};
use crate::math::{exp, fmod};
use crate::rng::Rng;
use crate::scene::{Camera, Fractal, Tone};

const NAME: &str = "mandelbrot";
const ATTEMPTS: u64 = 24;

pub(super) static SPEC: Spec = Spec {
    name: NAME,
    family: Family::Escape,
    params: &[ITERATIONS, COLOR_DENSITY, COLOR_SHIFT, EDGE, FADE],
    tone: Tone {
        brightness: 0.92,
        gamma: 2.5,
        vibrancy: 1.0,
        highlight: 0.0,
        glow_radius: 0.006,
        glow_strength: 0.12,
        smoothing: 1.0,
    },
    draw: Draw::Pixel(|params, camera| Box::new(Mandelbrot::new(params, camera))),
    random,
    mutate,
    presets: &[
        Preset {
            name: "whole",
            palette: "ice",
            center: [-0.65, 0.0],
            zoom: 1.35,
            params: &[("color_density", 1.2), ("color_shift", 1.1)],
        },
        Preset {
            name: "seahorse-valley",
            palette: "sunset",
            center: [-0.7453, 0.1127],
            zoom: 260.0,
            params: &[("iterations", 500.0), ("color_density", 2.4), ("color_shift", 0.3)],
        },
        Preset {
            name: "elephant-valley",
            palette: "aurora",
            center: [0.2785, 0.0092],
            zoom: 260.0,
            params: &[("iterations", 600.0), ("color_density", 1.6), ("color_shift", 0.9)],
        },
        Preset {
            name: "spiral",
            palette: "gold",
            center: [-0.743_643_887, 0.131_825_904],
            zoom: 60_000.0,
            params: &[("iterations", 900.0), ("color_density", 6.0), ("color_shift", 0.2)],
        },
        Preset {
            name: "island",
            palette: "violet",
            center: [-1.7549, 0.0],
            zoom: 55.0,
            params: &[("iterations", 500.0), ("color_density", 1.8), ("color_shift", 0.5)],
        },
        Preset {
            name: "lightning",
            palette: "orchid",
            center: [-0.1011, 0.9563],
            zoom: 45.0,
            params: &[("iterations", 500.0), ("color_density", 2.0), ("color_shift", 1.4)],
        },
    ],
};

pub(super) struct Mandelbrot {
    limit: u32,
    coloring: Coloring,
}

impl Mandelbrot {
    pub(super) fn new(params: &[f64], camera: &Camera) -> Self {
        Self {
            limit: limit(params[0], camera.zoom),
            coloring: Coloring::new(params[1], params[2], params[3], params[4], camera.zoom),
        }
    }
}

impl Field for Mandelbrot {
    #[inline]
    fn sample(&self, cx: f64, cy: f64) -> Option<Sample> {
        let (left, cy2) = (cx - 0.25, cy * cy);
        let q = left * left + cy2;
        if q * (q + left) <= 0.25 * cy2 || (cx + 1.0) * (cx + 1.0) + cy2 <= 0.0625 {
            return None;
        }
        let (mut x, mut y, mut dx, mut dy) = (0.0f64, 0.0f64, 0.0f64, 0.0f64);
        let (mut old_x, mut old_y, mut lap, mut period) = (0.0f64, 0.0f64, 0u32, 8u32);
        for step in 0..self.limit {
            let (x2, y2) = (x * x, y * y);
            if x2 + y2 > ESCAPE {
                return Some(self.coloring.outside(step, x2 + y2, dx * dx + dy * dy));
            }
            (dx, dy) = (2.0 * (x * dx - y * dy) + 1.0, 2.0 * (x * dy + y * dx));
            (x, y) = (x2 - y2 + cx, 2.0 * x * y + cy);
            // An orbit that comes back to where it was never leaves.
            if x == old_x && y == old_y {
                return None;
            }
            lap += 1;
            if lap == period {
                (old_x, old_y, lap, period) = (x, y, 0, period.saturating_mul(2));
            }
        }
        None
    }
}

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
pub(super) fn descend(rng: &mut Rng, steps: u64) -> Camera {
    let mut camera = Camera {
        center: [-0.65, 0.0],
        zoom: 1.3,
        ..Camera::default()
    };
    for _ in 0..steps {
        let field = Mandelbrot::new(&[400.0, 1.0, 0.0, 0.0, 0.0], &camera);
        let seen = Seen::of(&pixel::probe(&field, &camera));
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

fn random(seed: u64) -> Fractal {
    let mut fallback = None;
    for attempt in 0..ATTEMPTS {
        let mut rng = stream(NAME, seed, attempt);
        let steps = 1 + rng.below(9);
        let mut camera = descend(&mut rng, steps);
        if rng.next_f64() < 0.4 {
            camera.rotation = rng.range(0.0, 360.0).round();
        }
        let colors = random_palette(&mut rng);
        let field = Mandelbrot::new(&[700.0, 1.0, 0.0, 0.0, 0.0], &camera);
        let seen = Seen::of(&pixel::probe(&field, &camera));
        match fitted(&seen, camera.zoom, &mut rng) {
            Some(params) => return SPEC.fractal(colors, camera, &params),
            None => fallback.get_or_insert_with(|| SPEC.fractal(colors, camera, &[])),
        };
    }
    fallback.unwrap_or_else(|| SPEC.fractal("ice", Camera::default(), &[]))
}

pub(super) fn moved(fractal: &Fractal, rng: &mut Rng, strength: f64) -> Fractal {
    let mut out = fractal.clone();
    let (right, down) = (rng.range(-0.5, 0.5) * strength, rng.range(-0.5, 0.5) * strength);
    out.camera = out
        .camera
        .looking_at(right, down, exp(rng.range(-0.8, 1.0) * strength));
    out.camera.rotation = (out.camera.rotation + rng.range(-40.0, 40.0) * strength).round();
    let shift = out.params.get("color_shift").copied().unwrap_or(COLOR_SHIFT.default);
    let shifted = fmod(shift + rng.range(0.2, 0.8) * strength, 2.0);
    out.params.insert("color_shift".to_owned(), shifted);
    out
}

fn mutate(fractal: &Fractal, rng: &mut Rng, strength: f64) -> Fractal {
    moved(fractal, rng, strength)
}

#[cfg(test)]
mod tests {
    use crate::fractal::{golden, preset};

    #[test]
    fn it_renders_to_its_golden_hash() {
        let hash = golden(&preset("mandelbrot-seahorse-valley").unwrap());
        assert_eq!(hash, 0x5d81_f9ca_f8e1_500d, "got {hash:#018x}");
    }
}

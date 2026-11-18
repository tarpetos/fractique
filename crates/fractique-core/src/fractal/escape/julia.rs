use super::mandelbrot::walk;
use super::{COLOR_DENSITY, COLOR_SHIFT, Coloring, EDGE, ESCAPE, FADE, ITERATIONS, Seen, fitted, limit, moved};
use crate::fractal::pixel;
use crate::fractal::{Draw, Family, Field, Param, Preset, Sample, Spec, random_palette, stream};
use crate::rng::Rng;
use crate::scene::{Camera, Fractal, Tone};

const NAME: &str = "julia";
const ATTEMPTS: u64 = 24;

pub(super) static SPEC: Spec = Spec {
    name: NAME,
    family: Family::Escape,
    params: &[
        Param::real("real", -0.8, -2.0, 2.0),
        Param::real("imaginary", 0.156, -2.0, 2.0),
        ITERATIONS,
        COLOR_DENSITY,
        COLOR_SHIFT,
        EDGE,
        FADE,
    ],
    tone: Tone {
        brightness: 0.92,
        gamma: 2.5,
        vibrancy: 1.0,
        highlight: 0.0,
        glow_radius: 0.006,
        glow_strength: 0.12,
        smoothing: 1.0,
    },
    draw: Draw::Pixel(|params, camera| Box::new(Julia::new(params, camera))),
    random,
    mutate,
    presets: &[
        Preset {
            name: "dragon",
            palette: "sunset",
            center: [0.0, 0.0],
            zoom: 1.5,
            params: &[("real", -0.8), ("imaginary", 0.156), ("color_density", 1.4)],
        },
        Preset {
            name: "rabbit",
            palette: "orchid",
            center: [0.0, 0.0],
            zoom: 1.55,
            params: &[("real", -0.123), ("imaginary", 0.745), ("color_density", 1.2)],
        },
        Preset {
            name: "dendrite",
            palette: "ice",
            center: [0.0, 0.0],
            zoom: 1.55,
            params: &[("real", 0.0), ("imaginary", 1.0), ("color_density", 1.3)],
        },
        Preset {
            name: "spirals",
            palette: "violet",
            center: [0.0, 0.0],
            zoom: 1.5,
            params: &[("real", -0.7269), ("imaginary", 0.1889), ("iterations", 600.0)],
        },
        Preset {
            name: "galaxies",
            palette: "gold",
            center: [0.0, 0.0],
            zoom: 1.6,
            params: &[("real", 0.285), ("imaginary", 0.01), ("iterations", 600.0)],
        },
        Preset {
            name: "siegel-disc",
            palette: "aurora",
            center: [0.0, 0.0],
            zoom: 1.55,
            params: &[("real", -0.391), ("imaginary", -0.587), ("iterations", 800.0)],
        },
    ],
};

struct Julia {
    c: (f64, f64),
    limit: u32,
    coloring: Coloring,
}

impl Julia {
    fn new(params: &[f64], camera: &Camera) -> Self {
        Self {
            c: (params[0], params[1]),
            limit: limit(params[2], camera.zoom),
            coloring: Coloring::new(params[3], params[4], params[5], params[6], camera.zoom),
        }
    }
}

impl Field for Julia {
    #[inline]
    fn sample(&self, x: f64, y: f64) -> Option<Sample> {
        let (cx, cy) = self.c;
        let (mut x, mut y, mut dx, mut dy) = (x, y, 1.0f64, 0.0f64);
        let (mut old_x, mut old_y, mut lap, mut period) = (f64::NAN, f64::NAN, 0u32, 8u32);
        for step in 0..self.limit {
            let (x2, y2) = (x * x, y * y);
            if x2 + y2 > ESCAPE {
                return Some(self.coloring.outside(step, x2 + y2, dx * dx + dy * dy));
            }
            (dx, dy) = (2.0 * (x * dx - y * dy), 2.0 * (x * dy + y * dx));
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

fn random(seed: u64) -> Fractal {
    let mut fallback = None;
    for attempt in 0..ATTEMPTS {
        let mut rng = stream(NAME, seed, attempt);
        // The Julia sets worth a picture belong to the points along the edge of the Mandelbrot set.
        let steps = 2 + rng.below(4);
        let [real, imaginary] = walk(&mut rng, steps).center.map(|part| (part * 1e4).round() / 1e4);
        let mut camera = Camera {
            zoom: rng.range(1.3, 1.8),
            rotation: if rng.next_f64() < 0.5 { 90.0 } else { 0.0 },
            ..Camera::default()
        };
        if rng.next_f64() < 0.3 {
            camera = camera.looking_at(rng.range(-0.2, 0.2), rng.range(-0.2, 0.2), rng.range(1.5, 4.0));
        }
        let colors = random_palette(&mut rng);
        let field = Julia::new(&[real, imaginary, 700.0, 1.0, 0.0, 0.0, 0.0], &camera);
        let seen = Seen::of(&pixel::probe(&field, &camera));
        let constant = [("real", real), ("imaginary", imaginary)];
        match fitted(&seen, camera.zoom, &mut rng) {
            Some(params) => return SPEC.fractal(colors, camera, &[&constant[..], &params[..]].concat()),
            None => fallback.get_or_insert_with(|| SPEC.fractal(colors, camera, &constant)),
        };
    }
    fallback.unwrap_or_else(|| SPEC.fractal("ice", Camera::default(), &[]))
}

fn mutate(fractal: &Fractal, rng: &mut Rng, strength: f64) -> Fractal {
    let mut out = moved(fractal, rng, strength * 0.3);
    for (name, param) in [("real", &SPEC.params[0]), ("imaginary", &SPEC.params[1])] {
        let value = out.params.get(name).copied().unwrap_or(param.default);
        let nudged = (value + rng.range(-0.12, 0.12) * strength).clamp(param.min, param.max);
        out.params.insert(name.to_owned(), (nudged * 1e5).round() / 1e5);
    }
    out
}

#[cfg(test)]
mod tests {
    use crate::fractal::{golden, preset};

    #[test]
    fn it_renders_to_its_golden_hash() {
        let hash = golden(&preset("julia-dragon").unwrap());
        assert_eq!(hash, 0x6cc2_cf2d_1caa_d227, "got {hash:#018x}");
    }
}

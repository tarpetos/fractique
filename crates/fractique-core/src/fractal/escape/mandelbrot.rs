use super::{COLOR_DENSITY, COLOR_SHIFT, Coloring, EDGE, ESCAPE, FADE, ITERATIONS, Seen, descend, fitted, limit, moved};
use crate::fractal::pixel;
use crate::fractal::{Draw, Family, Field, Preset, Sample, Spec, random_palette, stream};
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
        Preset {
            name: "lace",
            palette: "cobalt",
            center: [-0.7453, 0.1127],
            zoom: 180.0,
            params: &[("iterations", 600.0)],
        },
        Preset {
            name: "tide",
            palette: "abyss",
            center: [-0.7436, 0.1318],
            zoom: 900.0,
            params: &[("iterations", 800.0)],
        },
        Preset {
            name: "feather",
            palette: "violet",
            center: [-1.4280725638756426, -0.0009669511123025884],
            zoom: 250.1,
            params: &[("iterations", 170.0), ("color_density", 3.29), ("color_shift", 0.962)],
        },
        Preset {
            name: "sunburst",
            palette: "orchid",
            center: [-0.2718003344166638, -0.6371607557893685],
            zoom: 1120.1,
            params: &[("iterations", 2090.0), ("color_density", 0.57), ("color_shift", 0.734)],
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

pub(super) const WHOLE: Camera = Camera {
    center: [-0.65, 0.0],
    zoom: 1.3,
    rotation: 0.0,
    flip_y: false,
};

pub(super) fn walk(rng: &mut Rng, steps: u64) -> Camera {
    descend(rng, steps, &WHOLE, &|shared, camera| Box::new(Mandelbrot::new(shared, camera)))
}

fn random(seed: u64) -> Fractal {
    let mut fallback = None;
    for attempt in 0..ATTEMPTS {
        let mut rng = stream(NAME, seed, attempt);
        let steps = 1 + rng.below(9);
        let mut camera = walk(&mut rng, steps);
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

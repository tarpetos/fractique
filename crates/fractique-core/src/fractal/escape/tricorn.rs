use super::{COLOR_DENSITY, COLOR_SHIFT, Coloring, EDGE, ESCAPE, FADE, ITERATIONS, TONE, candidate, limit, moved};
use crate::fractal::{Draw, Family, Field, Preset, Sample, Spec, stream};
use crate::scene::{Camera, Fractal};

const NAME: &str = "tricorn";
const ATTEMPTS: u64 = 24;
const WHOLE: Camera = Camera {
    center: [-0.25, 0.0],
    zoom: 1.1,
    rotation: 0.0,
    flip_y: false,
};

pub(super) static SPEC: Spec = Spec {
    name: NAME,
    family: Family::Escape,
    params: &[ITERATIONS, COLOR_DENSITY, COLOR_SHIFT, EDGE, FADE],
    tone: TONE,
    draw: Draw::Pixel(|params, camera| Box::new(Tricorn::new(params, camera))),
    random,
    mutate: moved,
    presets: &[
        Preset {
            name: "whole",
            palette: "violet",
            center: [-0.25, 0.0],
            zoom: 1.1,
            params: &[("color_density", 1.3), ("color_shift", 0.6)],
        },
        Preset {
            name: "coast",
            palette: "emerald",
            center: [0.68888, -1.21149],
            zoom: 57.0,
            params: &[("iterations", 500.0), ("color_density", 1.14), ("color_shift", 0.05)],
        },
        Preset {
            name: "spiral",
            palette: "violet",
            center: [0.636792, 1.161674],
            zoom: 8700.0,
            params: &[("iterations", 500.0), ("color_density", 1.07), ("color_shift", 0.34)],
        },
        Preset {
            name: "island",
            palette: "aurora",
            center: [-1.2372004, 0.0555578],
            zoom: 6300.0,
            params: &[("iterations", 500.0), ("color_density", 1.09), ("color_shift", 1.4)],
        },
        Preset {
            name: "lightning",
            palette: "ice",
            center: [0.66154047, 1.03082346],
            zoom: 21700.0,
            params: &[("iterations", 400.0), ("color_density", 1.23), ("color_shift", 0.18)],
        },
        Preset {
            name: "embers",
            palette: "ember",
            center: [0.70335957, 1.14448771],
            zoom: 3100.0,
            params: &[("iterations", 400.0), ("color_density", 1.55), ("color_shift", 0.69)],
        },
    ],
};

struct Tricorn {
    limit: u32,
    coloring: Coloring,
}

impl Tricorn {
    fn new(params: &[f64], camera: &Camera) -> Self {
        Self {
            limit: limit(params[0], camera.zoom),
            coloring: Coloring::new(params[1], params[2], params[3], params[4], camera.zoom),
        }
    }
}

impl Field for Tricorn {
    #[inline]
    fn sample(&self, cx: f64, cy: f64) -> Option<Sample> {
        let (mut x, mut y) = (0.0f64, 0.0f64);
        // The derivatives of x and y by cx (a) and by cy (b): the map is not analytic, so they are four numbers.
        let (mut xa, mut xb, mut ya, mut yb) = (0.0f64, 0.0f64, 0.0f64, 0.0f64);
        let (mut old_x, mut old_y, mut lap, mut period) = (0.0f64, 0.0f64, 0u32, 8u32);
        for step in 0..self.limit {
            let (x2, y2) = (x * x, y * y);
            if x2 + y2 > ESCAPE {
                let (ga, gb) = (x * xa + y * ya, x * xb + y * yb);
                return Some(self.coloring.outside(step, x2 + y2, (ga * ga + gb * gb) / (x2 + y2)));
            }
            (xa, xb, ya, yb) = (
                2.0 * (x * xa - y * ya) + 1.0,
                2.0 * (x * xb - y * yb),
                -2.0 * (y * xa + x * ya),
                -2.0 * (y * xb + x * yb) + 1.0,
            );
            (x, y) = (x2 - y2 + cx, -2.0 * x * y + cy);
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
        let (camera, colors, params) = candidate(&mut rng, &WHOLE, 8, &|shared, camera| Box::new(Tricorn::new(shared, camera)));
        match params {
            Some(params) => return SPEC.fractal(colors, camera, &params),
            None => fallback.get_or_insert_with(|| SPEC.fractal(colors, camera, &[])),
        };
    }
    fallback.unwrap_or_else(|| SPEC.fractal("violet", WHOLE, &[]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fractal::{golden, preset};

    #[test]
    fn the_set_has_three_corners() {
        let tricorn = Tricorn::new(&[300.0, 1.0, 0.0, 0.0, 0.0], &Camera::default());
        let (turn_x, turn_y) = (-0.5, 0.75f64.sqrt());
        for (x, y) in [(-1.0, 0.0), (0.2, 0.3), (0.6, 0.1), (-1.2, 0.3), (0.1, -0.2)] {
            let turned = (x * turn_x - y * turn_y, x * turn_y + y * turn_x);
            let (own, other) = (tricorn.sample(x, y), tricorn.sample(turned.0, turned.1));
            assert_eq!(own.is_some(), other.is_some(), "{x} {y}");
        }
        assert!(tricorn.sample(-1.9, 0.0).is_none() && tricorn.sample(-2.1, 0.0).is_some());
    }

    #[test]
    fn it_renders_to_its_golden_hash() {
        let hash = golden(&preset("tricorn-whole").unwrap());
        assert_eq!(hash, 0x4c88_f876_30c1_977c, "got {hash:#018x}");
    }
}

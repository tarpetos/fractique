use super::{COLOR_DENSITY, COLOR_SHIFT, Coloring, EDGE, ESCAPE, FADE, ITERATIONS, TONE, candidate, limit, moved};
use crate::fractal::{Draw, Family, Field, Preset, Sample, Spec, stream};
use crate::scene::{Camera, Fractal};

const NAME: &str = "burning_ship";
const ATTEMPTS: u64 = 24;
const WHOLE: Camera = Camera {
    center: [-0.45, 0.55],
    zoom: 1.05,
    rotation: 0.0,
    flip_y: false,
};
const MAST: Camera = Camera {
    center: [-1.78, 0.02],
    zoom: 7.0,
    rotation: 0.0,
    flip_y: false,
};

pub(super) static SPEC: Spec = Spec {
    name: NAME,
    family: Family::Escape,
    params: &[ITERATIONS, COLOR_DENSITY, COLOR_SHIFT, EDGE, FADE],
    tone: TONE,
    draw: Draw::Pixel(|params, camera| Box::new(BurningShip::new(params, camera))),
    random,
    mutate: moved,
    presets: &[
        Preset {
            name: "whole",
            palette: "ember",
            center: [-0.45, 0.55],
            zoom: 1.05,
            params: &[("color_density", 1.3), ("color_shift", 0.4)],
        },
        Preset {
            name: "ship",
            palette: "sunset",
            center: [-1.762, 0.028],
            zoom: 42.0,
            params: &[("iterations", 500.0), ("color_density", 2.0), ("color_shift", 0.2)],
        },
        Preset {
            name: "armada",
            palette: "ice",
            center: [-1.741, 0.0004],
            zoom: 50.0,
            params: &[("iterations", 600.0), ("color_density", 1.07), ("color_shift", 1.3)],
        },
        Preset {
            name: "cathedral",
            palette: "ember",
            center: [-1.72985, 0.00255],
            zoom: 126.0,
            params: &[("iterations", 700.0), ("color_density", 1.26), ("color_shift", 1.54)],
        },
        Preset {
            name: "towers",
            palette: "violet",
            center: [-1.656087, 0.007438],
            zoom: 3900.0,
            params: &[("iterations", 400.0), ("color_density", 1.15), ("color_shift", 1.23)],
        },
        Preset {
            name: "windows",
            palette: "orchid",
            center: [-1.630726, 0.026432],
            zoom: 10800.0,
            params: &[("iterations", 400.0), ("color_density", 1.06), ("color_shift", 1.7)],
        },
    ],
};

struct BurningShip {
    limit: u32,
    coloring: Coloring,
}

impl BurningShip {
    fn new(params: &[f64], camera: &Camera) -> Self {
        Self {
            limit: limit(params[0], camera.zoom),
            coloring: Coloring::new(params[1], params[2], params[3], params[4], camera.zoom),
        }
    }
}

impl Field for BurningShip {
    #[inline]
    fn sample(&self, cx: f64, up: f64) -> Option<Sample> {
        let cy = -up;
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
            let (side, lift) = (if x < 0.0 { -y.abs() } else { y.abs() }, if y < 0.0 { -x.abs() } else { x.abs() });
            (xa, xb, ya, yb) = (
                2.0 * (x * xa - y * ya) + 1.0,
                2.0 * (x * xb - y * yb),
                2.0 * (side * xa + lift * ya),
                2.0 * (side * xb + lift * yb) + 1.0,
            );
            (x, y) = (x2 - y2 + cx, 2.0 * (x * y).abs() + cy);
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
        // The ships worth a picture ride the long mast on the real axis: most walks start there.
        let from = if rng.next_f64() < 0.6 { &MAST } else { &WHOLE };
        let (camera, colors, params) = candidate(&mut rng, from, 6, &|shared, camera| Box::new(BurningShip::new(shared, camera)));
        match params {
            Some(params) => return SPEC.fractal(colors, camera, &params),
            None => fallback.get_or_insert_with(|| SPEC.fractal(colors, camera, &[])),
        };
    }
    fallback.unwrap_or_else(|| SPEC.fractal("ember", WHOLE, &[]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fractal::{golden, preset};

    #[test]
    fn the_ship_stands_mast_up() {
        let ship = BurningShip::new(&[300.0, 1.0, 0.0, 0.0, 0.0], &Camera::default());
        assert!(ship.sample(0.0, 0.0).is_none() && ship.sample(-1.0, 0.0).is_none());
        // c = i escapes and c = -i does not: the picture shows the imaginary axis pointing down.
        assert!(ship.sample(0.0, 1.0).is_none() && ship.sample(0.0, -1.0).is_some());
    }

    #[test]
    fn it_renders_to_its_golden_hash() {
        let hash = golden(&preset("burning_ship-ship").unwrap());
        assert_eq!(hash, 0x66c7_84c4_9380_c08d, "got {hash:#018x}");
    }
}

use super::{COLOR_DENSITY, COLOR_SHIFT, Coloring, EDGE, ESCAPE, FADE, ITERATIONS, TONE, candidate, limit, moved};
use crate::fractal::{Draw, Family, Field, Preset, Sample, Spec, stream};
use crate::rng::Rng;
use crate::scene::{Camera, Fractal};
use crate::math::{atan2, cos, pow, sin};
use crate::fractal::Param;
use super::{nudged, value};

const NAME: &str = "multibrot";
const ATTEMPTS: u64 = 24;
const WHOLE: Camera = Camera {
    center: [0.0, 0.0],
    zoom: 1.35,
    rotation: 0.0,
    flip_y: false,
};

pub(super) static SPEC: Spec = Spec {
    name: NAME,
    family: Family::Escape,
    params: &[
        Param::real("power", 3.0, 2.0, 8.0),
        ITERATIONS,
        COLOR_DENSITY,
        COLOR_SHIFT,
        EDGE,
        FADE,
    ],
    tone: TONE,
    draw: Draw::Pixel(|params, camera| Box::new(Multibrot::new(params, camera))),
    random,
    mutate,
    presets: &[
        Preset {
            name: "cubic",
            palette: "ice",
            center: [0.0, 0.0],
            zoom: 1.35,
            params: &[("power", 3.0), ("color_density", 1.3), ("color_shift", 1.0)],
        },
        Preset {
            name: "quartic",
            palette: "orchid",
            center: [0.0, 0.0],
            zoom: 1.4,
            params: &[("power", 4.0), ("color_density", 1.3), ("color_shift", 0.2)],
        },
        Preset {
            name: "flower",
            palette: "gold",
            center: [0.0, 0.0],
            zoom: 1.5,
            params: &[("power", 7.0), ("color_density", 1.4), ("color_shift", 0.5)],
        },
        Preset {
            name: "spirals",
            palette: "sunset",
            center: [0.60341569, 0.48404511],
            zoom: 5400.0,
            params: &[("power", 4.0), ("iterations", 800.0), ("color_density", 0.92), ("color_shift", 1.82)],
        },
        Preset {
            name: "sunflower",
            palette: "gold",
            center: [-0.00550053, -0.79111742],
            zoom: 20300.0,
            params: &[("power", 6.0), ("iterations", 400.0), ("color_density", 0.95), ("color_shift", 0.57)],
        },
        Preset {
            name: "fronds",
            palette: "emerald",
            center: [0.21584999, 1.1357673],
            zoom: 9100.0,
            params: &[("power", 3.0), ("iterations", 600.0), ("color_density", 1.3), ("color_shift", 1.61)],
        },
    ],
};

struct Multibrot {
    power: f64,
    // The multiplications that raise z to the power less one, where the power is a whole number.
    times: Option<u32>,
    limit: u32,
    coloring: Coloring,
}

impl Multibrot {
    fn new(params: &[f64], camera: &Camera) -> Self {
        let power = params[0];
        Self {
            power,
            times: (power == power.round()).then_some(power as u32 - 2),
            limit: limit(params[1], camera.zoom),
            coloring: Coloring::new(params[2], params[3], params[4], params[5], camera.zoom).of_power(power),
        }
    }

    #[inline]
    fn raised(&self, x: f64, y: f64) -> (f64, f64) {
        match self.times {
            Some(times) => {
                let (mut px, mut py) = (x, y);
                for _ in 0..times {
                    (px, py) = (px * x - py * y, px * y + py * x);
                }
                (px, py)
            }
            None => {
                let size = pow(x * x + y * y, 0.5 * (self.power - 1.0));
                let angle = atan2(y, x) * (self.power - 1.0);
                (size * cos(angle), size * sin(angle))
            }
        }
    }
}

impl Field for Multibrot {
    #[inline]
    fn sample(&self, cx: f64, cy: f64) -> Option<Sample> {
        let (mut x, mut y, mut dx, mut dy) = (0.0f64, 0.0f64, 0.0f64, 0.0f64);
        let (mut old_x, mut old_y, mut lap, mut period) = (0.0f64, 0.0f64, 0u32, 8u32);
        for step in 0..self.limit {
            let r2 = x * x + y * y;
            if r2 > ESCAPE {
                return Some(self.coloring.outside(step, r2, dx * dx + dy * dy));
            }
            let (px, py) = self.raised(x, y);
            let (qx, qy) = (self.power * px, self.power * py);
            (dx, dy) = (qx * dx - qy * dy + 1.0, qx * dy + qy * dx);
            (x, y) = (px * x - py * y + cx, px * y + py * x + cy);
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
        let power = 3.0 + rng.below(4) as f64;
        let (camera, colors, params) = candidate(&mut rng, &WHOLE, 7, &|shared, camera| {
            Box::new(Multibrot::new(&[&[power][..], &shared[..]].concat(), camera))
        });
        let power = [("power", power)];
        match params {
            Some(params) => return SPEC.fractal(colors, camera, &[&power[..], &params[..]].concat()),
            None => fallback.get_or_insert_with(|| SPEC.fractal(colors, camera, &power)),
        };
    }
    fallback.unwrap_or_else(|| SPEC.fractal("ice", WHOLE, &[]))
}

fn mutate(fractal: &Fractal, rng: &mut Rng, strength: f64) -> Fractal {
    let mut out = moved(fractal, rng, strength * 0.6);
    if value(&SPEC, fractal, "power").fract() != 0.0 {
        nudged(&SPEC, &mut out, "power", rng, 0.2 * strength);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fractal::{golden, preset};

    #[test]
    fn a_whole_power_and_the_same_power_by_angle_agree() {
        let by_angle = Multibrot {
            times: None,
            ..Multibrot::new(&[5.0, 300.0, 1.0, 0.0, 0.0, 0.0], &Camera::default())
        };
        let whole = Multibrot::new(&[5.0, 300.0, 1.0, 0.0, 0.0, 0.0], &Camera::default());
        for (x, y) in [(0.9, 0.3), (-0.4, 1.1), (1.3, -0.2)] {
            let (a, b) = (whole.sample(x, y).unwrap(), by_angle.sample(x, y).unwrap());
            assert!((a.at - b.at).abs() < 1e-9, "{x} {y}: {} {}", a.at, b.at);
        }
        let square = Multibrot::new(&[2.0, 300.0, 1.0, 0.0, 0.0, 0.0], &Camera::default());
        assert!(square.sample(-1.0, 0.0).is_none() && square.sample(0.3, 0.0).is_some());
    }

    #[test]
    fn it_renders_to_its_golden_hash() {
        let hash = golden(&preset("multibrot-cubic").unwrap());
        assert_eq!(hash, 0x0869_231c_3492_ec8c, "got {hash:#018x}");
    }
}

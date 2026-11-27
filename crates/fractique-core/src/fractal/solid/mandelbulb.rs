use super::{
    COLOR_DENSITY, COLOR_SHIFT, FIELD_OF_VIEW, FOG, GLOW, ISOMETRIC, LIGHT_COLOR, LIGHT_TILT, LIGHT_TURN, Marcher, Point,
    SHADOW, Solid, TILT, TURN, dot, fitted, moved, picked, set, value,
};
use crate::fractal::{Draw, Family, Param, Preset, Spec, random_palette, stream};
use crate::math::{atan2, cos, ln, pow, sin};
use crate::rng::Rng;
use crate::scene::{Camera, Fractal, Tone};

const NAME: &str = "mandelbulb";
const ATTEMPTS: u64 = 12;
const POWER: Param = Param::real("power", 8.0, 2.0, 16.0);
const ITERATIONS: Param = Param::whole("iterations", 10.0, 2.0, 200.0).log();
const DISTANCE: Param = Param::real("distance", 3.6, 0.5, 20.0).log();
const ESCAPE: f64 = 16.0;

pub(super) static SPEC: Spec = Spec {
    name: NAME,
    family: Family::Solid,
    params: &[
        POWER,
        ITERATIONS,
        TURN,
        TILT,
        DISTANCE,
        FIELD_OF_VIEW,
        LIGHT_TURN,
        LIGHT_TILT,
        SHADOW,
        FOG,
        GLOW,
        COLOR_DENSITY,
        COLOR_SHIFT,
        LIGHT_COLOR,
        ISOMETRIC,
    ],
    tone: Tone {
        brightness: 1.0,
        gamma: 2.5,
        vibrancy: 1.0,
        highlight: 0.0,
        glow_radius: 0.004,
        glow_strength: 0.1,
        smoothing: 1.0,
    },
    draw: Draw::Pixel(|params, camera| Box::new(Marcher::new(Bulb::new(params[0], params[1]), &params[2..], camera))),
    random,
    mutate,
    presets: &[
        Preset {
            name: "classic",
            palette: "sunset",
            center: [0.0, 0.0],
            zoom: 1.0,
            params: &[("color_density", 1.6), ("color_shift", 0.2)],
        },
        Preset {
            name: "crown",
            palette: "gold",
            center: [0.0, 0.0],
            zoom: 1.0,
            params: &[("turn", 10.0), ("tilt", 72.0), ("light_tilt", 30.0), ("color_density", 1.8), ("color_shift", 0.9)],
        },
        Preset {
            name: "cubic",
            palette: "ice",
            center: [0.0, 0.0],
            zoom: 1.0,
            params: &[("power", 3.0), ("turn", 40.0), ("tilt", 25.0), ("distance", 3.5), ("color_density", 1.2), ("color_shift", 0.3)],
        },
        Preset {
            name: "coral",
            palette: "ember",
            center: [0.3, 0.2],
            zoom: 3.5,
            params: &[("iterations", 14.0), ("turn", 96.0), ("tilt", 18.0), ("distance", 2.86), ("light_turn", 61.0), ("light_tilt", 18.0), ("fog", 0.45), ("color_density", 3.0), ("color_shift", 0.1)],
        },
        Preset {
            name: "garden",
            palette: "orchid",
            center: [0.0, 0.35],
            zoom: 3.0,
            params: &[("iterations", 14.0), ("turn", 20.0), ("tilt", -8.0), ("distance", 2.4), ("fog", 0.4), ("color_density", 2.4), ("color_shift", 1.3)],
        },
        Preset {
            name: "sixteen",
            palette: "violet",
            center: [0.0, 0.0],
            zoom: 1.0,
            params: &[("power", 16.0), ("turn", 25.0), ("tilt", 28.0), ("color_density", 2.2), ("color_shift", 0.5)],
        },
        Preset {
            name: "lagoon",
            palette: "lagoon",
            center: [0.0, 0.0],
            zoom: 1.0,
            params: &[("fog", 0.2), ("turn", 45.0), ("tilt", 25.0), ("light_turn", -40.6), ("light_tilt", 48.6), ("shadow", 1.0), ("glow", 1.0), ("light_color", 1.0), ("isometric", 1.0)],
        },
        Preset {
            name: "clay",
            palette: "garnet",
            center: [0.1, 0.25],
            zoom: 2.2,
            params: &[("fog", 0.2), ("turn", 45.0), ("tilt", 25.0), ("light_turn", -40.6), ("light_tilt", 48.6), ("shadow", 1.0), ("glow", 1.0), ("light_color", 1.0), ("isometric", 1.0)],
        },
        Preset {
            name: "orb",
            palette: "amethyst",
            center: [0.0, 0.0],
            zoom: 0.85,
            params: &[("power", 12.0), ("iterations", 10.0), ("turn", -2.0), ("tilt", 46.0), ("distance", 2.93), ("light_turn", 24.0), ("light_tilt", 45.0), ("fog", 0.32), ("color_density", 2.49), ("color_shift", 0.58)],
        },
        Preset {
            name: "flowers",
            palette: "emerald",
            center: [0.84, 0.2255],
            zoom: 2.7695,
            params: &[("power", 6.0), ("iterations", 14.0), ("turn", -104.0), ("tilt", 1.0), ("distance", 2.4), ("light_turn", 49.0), ("light_tilt", 34.0), ("fog", 0.21), ("color_density", 3.04), ("color_shift", 1.92)],
        },
    ],
};

struct Bulb {
    power: f64,
    whole: Option<u32>,
    iterations: u32,
    reach: f64,
}

// (cos, sin) of `times` the angle, by squaring: only products, the same on every machine.
#[inline]
fn turned(angle: (f64, f64), times: u32) -> (f64, f64) {
    if times.is_power_of_two() {
        let mut out = angle;
        for _ in 0..times.trailing_zeros() {
            out = (out.0 * out.0 - out.1 * out.1, 2.0 * out.0 * out.1);
        }
        return out;
    }
    let (mut out, mut power, mut left) = ((1.0, 0.0), angle, times);
    while left > 0 {
        if left & 1 == 1 {
            out = (out.0 * power.0 - out.1 * power.1, out.0 * power.1 + out.1 * power.0);
        }
        power = (power.0 * power.0 - power.1 * power.1, 2.0 * power.0 * power.1);
        left >>= 1;
    }
    out
}

#[inline]
fn raised(base: f64, times: u32) -> f64 {
    let (mut out, mut power, mut left) = (1.0, base, times);
    while left > 0 {
        if left & 1 == 1 {
            out *= power;
        }
        power *= power;
        left >>= 1;
    }
    out
}

impl Bulb {
    fn new(power: f64, iterations: f64) -> Self {
        Self {
            power,
            whole: (power.fract() == 0.0).then_some(power as u32),
            iterations: iterations as u32,
            reach: pow(2.0, 1.0 / (power - 1.0)) + 0.02,
        }
    }

    // The point to the power, without its length, and the length to the power less one.
    #[inline]
    fn powered(&self, [x, y, z]: Point, length: f64) -> (Point, f64) {
        let flat = (x * x + y * y).sqrt();
        if let Some(power) = self.whole {
            let around = if flat > 0.0 {
                let per_flat = 1.0 / flat;
                (x * per_flat, y * per_flat)
            } else {
                (1.0, 0.0)
            };
            let per_length = 1.0 / length;
            let (down_cos, down_sin) = turned((z * per_length, flat * per_length), power);
            let (around_cos, around_sin) = turned(around, power);
            (
                [down_sin * around_cos, down_sin * around_sin, down_cos],
                raised(length, power - 1),
            )
        } else {
            let (down, around) = (self.power * atan2(flat, z), self.power * atan2(y, x));
            (
                [sin(down) * cos(around), sin(down) * sin(around), cos(down)],
                pow(length, self.power - 1.0),
            )
        }
    }
}

impl Solid for Bulb {
    const STRIDE: f64 = 0.9;

    fn reach(&self) -> f64 {
        self.reach
    }

    #[inline]
    fn distance(&self, [x, y, z]: Point) -> (f64, f64) {
        // The bulb's axis stands upright in the picture.
        let start = [x, z, y];
        let (mut at, mut stretch, mut trap) = (start, 1.0f64, f64::INFINITY);
        let mut square = dot(at, at);
        for _ in 0..self.iterations {
            if square > ESCAPE || square == 0.0 {
                break;
            }
            let length = square.sqrt();
            let (toward, grown) = self.powered(at, length);
            stretch = self.power * grown * stretch + 1.0;
            let long = grown * length;
            at = [
                toward[0] * long + start[0],
                toward[1] * long + start[1],
                toward[2] * long + start[2],
            ];
            square = dot(at, at);
            trap = trap.min(square);
        }
        let length = square.sqrt();
        let distance = if length > 1.0 {
            0.5 * ln(length) * length / stretch
        } else {
            0.0
        };
        (distance, trap.sqrt().min(2.0))
    }
}

fn random(seed: u64) -> Fractal {
    picked(ATTEMPTS, |attempt| {
        let mut rng = stream(NAME, seed, attempt);
        let power = if rng.next_f64() < 0.5 { 8.0 } else { (3 + rng.below(10)) as f64 };
        let reach = Bulb::new(power, 1.0).reach;
        let mut camera = Camera::default();
        let near = rng.next_f64() < 0.45;
        let away = if near {
            camera = camera.looking_at(rng.range(-0.22, 0.22), rng.range(-0.22, 0.22), rng.range(1.5, 4.5));
            reach * rng.range(1.9, 2.6)
        } else {
            reach * rng.range(2.5, 3.0)
        };
        let colors = random_palette(&mut rng);
        let params = vec![
            ("power", power),
            ("iterations", if near { 14.0 } else { 10.0 }),
            ("turn", rng.range(-180.0, 180.0).round()),
            ("tilt", rng.range(-50.0, 70.0).round()),
            ("distance", (away * 100.0).round() / 100.0),
            ("light_turn", rng.range(-70.0, 70.0).round()),
            ("light_tilt", rng.range(15.0, 65.0).round()),
            ("fog", (rng.range(0.15, 0.5) * 100.0).round() / 100.0),
            ("color_shift", (rng.range(0.0, 2.0) * 100.0).round() / 100.0),
        ];
        fitted(&SPEC, colors, camera, params, rng.range(0.7, 1.7), 0.24)
    })
}

fn mutate(fractal: &Fractal, rng: &mut Rng, strength: f64) -> Fractal {
    let mut out = moved(fractal, &DISTANCE, rng, strength);
    let power = value(&out, &POWER) + (rng.range(-4.0, 4.0) * strength).round();
    set(&mut out, &POWER, power);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fractal::{golden, preset};

    #[test]
    fn it_renders_to_its_golden_hash() {
        let hash = golden(&preset("mandelbulb-classic").unwrap());
        assert_eq!(hash, 0xf5cb_995d_7316_b7c6, "got {hash:#018x}");
    }

    #[test]
    fn a_whole_power_by_products_is_the_power_by_angles() {
        let mut rng = Rng::derive(1, 2, 3);
        for power in [2.0, 3.0, 5.0, 8.0, 9.0, 16.0] {
            let by_products = Bulb::new(power, 1.0);
            let by_angles = Bulb {
                whole: None,
                ..Bulb::new(power, 1.0)
            };
            for _ in 0..200 {
                let point = [rng.range(-1.2, 1.2), rng.range(-1.2, 1.2), rng.range(-1.2, 1.2)];
                let length = dot(point, point).sqrt();
                let (a, grown_a) = by_products.powered(point, length);
                let (b, grown_b) = by_angles.powered(point, length);
                for axis in 0..3 {
                    assert!((a[axis] - b[axis]).abs() < 1e-9, "{power} {point:?}");
                }
                assert!((grown_a - grown_b).abs() < 1e-9 * grown_b.max(1.0));
            }
        }
    }

    #[test]
    fn a_power_multiplies_both_angles_and_raises_the_length() {
        let bulb = Bulb {
            whole: None,
            ..Bulb::new(3.0, 1.0)
        };
        let (length, down, around) = (0.9f64, 0.7f64, -2.1f64);
        let point = [
            length * sin(down) * cos(around),
            length * sin(down) * sin(around),
            length * cos(down),
        ];
        let (toward, grown) = bulb.powered(point, length);
        let expected = [sin(3.0 * down) * cos(3.0 * around), sin(3.0 * down) * sin(3.0 * around), cos(3.0 * down)];
        for axis in 0..3 {
            assert!((toward[axis] - expected[axis]).abs() < 1e-12);
        }
        assert!((grown - length * length).abs() < 1e-12);
    }

    // The estimate is not a bound: in a thin gap between two parts of the bulb it can be several times too long.
    #[test]
    fn half_of_the_estimate_is_nearly_always_clear_of_the_bulb() {
        let bulb = Bulb::new(8.0, 12.0);
        let mut rng = Rng::derive(4, 5, 6);
        let (mut asked, mut wrong) = (0, 0);
        for _ in 0..2000 {
            let from = [rng.range(-1.3, 1.3), rng.range(-1.3, 1.3), rng.range(-1.3, 1.3)];
            let (distance, _) = bulb.distance(from);
            if distance < 1e-4 {
                continue;
            }
            asked += 1;
            let blocked = (0..3).any(|axis| {
                [-0.5, 0.5].into_iter().any(|side| {
                    let mut to = from;
                    to[axis] += side * distance;
                    bulb.distance(to).0 < distance * 0.05
                })
            });
            wrong += u32::from(blocked);
        }
        assert!(asked > 1000 && wrong * 50 < asked, "{wrong} of {asked}");
    }
}

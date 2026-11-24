use super::{
    COLOR_DENSITY, COLOR_SHIFT, FIELD_OF_VIEW, FOG, GLOW, ISOMETRIC, LIGHT_COLOR, LIGHT_TILT, LIGHT_TURN, Marcher, Point,
    SHADOW, Solid, TILT, TURN, dot, fitted, moved, picked, set, value,
};
use crate::fractal::{Draw, Family, Param, Preset, Spec, random_palette, stream};
use crate::math::{cos, sin};
use crate::rng::Rng;
use crate::scene::{Camera, Fractal, Tone};

const NAME: &str = "menger_sponge";
const ATTEMPTS: u64 = 16;
const ITERATIONS: Param = Param::whole("iterations", 5.0, 1.0, 30.0);
const SCALE: Param = Param::real("scale", 3.0, 2.0, 4.0);
const TWIST: Param = Param::real("twist", 0.0, -45.0, 45.0);
const BEND: Param = Param::real("bend", 0.0, -45.0, 45.0);
const DISTANCE: Param = Param::real("distance", 5.0, 0.05, 30.0).log();

pub(super) static SPEC: Spec = Spec {
    name: NAME,
    family: Family::Solid,
    params: &[
        ITERATIONS,
        SCALE,
        TWIST,
        BEND,
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
    draw: Draw::Pixel(|params, camera| {
        Box::new(Marcher::new(
            Sponge::new(params[0], params[1], params[2], params[3]),
            &params[4..],
            camera,
        ))
    }),
    random,
    mutate,
    presets: &[
        Preset {
            name: "classic",
            palette: "gold",
            center: [0.0, 0.0],
            zoom: 1.0,
            params: &[("color_density", 3.5), ("color_shift", 0.25)],
        },
        Preset {
            name: "tunnel",
            palette: "violet",
            center: [0.0, 0.0],
            zoom: 1.0,
            params: &[("iterations", 7.0), ("turn", 104.0), ("tilt", 7.0), ("distance", 1.64), ("light_turn", 50.0), ("light_tilt", 20.0), ("fog", 0.4), ("color_density", 3.72), ("color_shift", 0.08)],
        },
        Preset {
            name: "courtyard",
            palette: "emerald",
            center: [0.1272, 0.1156],
            zoom: 2.9393,
            params: &[("iterations", 7.0), ("turn", 4.0), ("tilt", -35.0), ("distance", 2.31), ("light_turn", -31.0), ("light_tilt", 43.0), ("fog", 0.39), ("color_density", 5.53), ("color_shift", 0.31)],
        },
        Preset {
            name: "terraces",
            palette: "ember",
            center: [0.2982, 0.6997],
            zoom: 2.303,
            params: &[("iterations", 7.0), ("turn", 60.0), ("tilt", -31.0), ("distance", 2.03), ("light_turn", 62.0), ("light_tilt", 49.0), ("fog", 0.2), ("color_density", 6.37), ("color_shift", 1.88)],
        },
        Preset {
            name: "honeycomb",
            palette: "ember",
            center: [0.0, 0.0],
            zoom: 1.0,
            params: &[("iterations", 6.0), ("scale", 2.8), ("twist", 24.0), ("bend", -8.0), ("turn", 35.0), ("tilt", 25.0), ("distance", 4.2), ("color_density", 3.0), ("color_shift", 0.4)],
        },
        Preset {
            name: "knot",
            palette: "orchid",
            center: [0.0, 0.0],
            zoom: 1.0,
            params: &[("iterations", 7.0), ("scale", 2.6), ("twist", -30.0), ("bend", 20.0), ("turn", -20.0), ("tilt", 35.0), ("distance", 4.0), ("color_density", 2.5), ("color_shift", 0.8)],
        },
        Preset {
            name: "sandstone",
            palette: "bronze",
            center: [0.0, 0.0],
            zoom: 2.2,
            params: &[("distance", 5.0), ("fog", 0.2), ("turn", 45.0), ("tilt", 35.264), ("light_turn", -40.6), ("light_tilt", 48.6), ("shadow", 1.0), ("glow", 1.0), ("light_color", 1.0), ("isometric", 1.0)],
        },
        Preset {
            name: "candy",
            palette: "amethyst",
            center: [0.15, -0.1],
            zoom: 1.5,
            params: &[("distance", 5.0), ("fog", 0.2), ("turn", 45.0), ("tilt", 35.264), ("light_turn", -40.6), ("light_tilt", 48.6), ("shadow", 1.0), ("glow", 1.0), ("light_color", 1.0), ("isometric", 1.0)],
        },
    ],
};

struct Sponge {
    iterations: u32,
    scale: f64,
    twist: Option<(f64, f64)>,
    bend: Option<(f64, f64)>,
}

impl Sponge {
    fn new(iterations: f64, scale: f64, twist: f64, bend: f64) -> Self {
        let turn = |degrees: f64| (degrees != 0.0).then(|| (cos(degrees.to_radians()), sin(degrees.to_radians())));
        Self {
            iterations: iterations as u32,
            scale,
            twist: turn(twist),
            bend: turn(bend),
        }
    }
}

#[inline]
fn cube([x, y, z]: Point) -> f64 {
    let (x, y, z) = (x.abs() - 1.0, y.abs() - 1.0, z.abs() - 1.0);
    let (ox, oy, oz) = (x.max(0.0), y.max(0.0), z.max(0.0));
    (ox * ox + oy * oy + oz * oz).sqrt() + x.max(y).max(z).min(0.0)
}

impl Solid for Sponge {
    const STRIDE: f64 = 1.0;

    fn reach(&self) -> f64 {
        1.74
    }

    #[inline]
    fn distance(&self, start: Point) -> (f64, f64) {
        let (mut x, mut y, mut z) = (start[0], start[1], start[2]);
        let (mut size, mut trap) = (1.0f64, f64::INFINITY);
        let gap = self.scale - 1.0;
        for _ in 0..self.iterations {
            if let Some((cos, sin)) = self.twist {
                (x, y) = (x * cos - y * sin, x * sin + y * cos);
            }
            (x, y, z) = (x.abs(), y.abs(), z.abs());
            if x < y {
                (x, y) = (y, x);
            }
            if x < z {
                (x, z) = (z, x);
            }
            if y < z {
                (y, z) = (z, y);
            }
            if let Some((cos, sin)) = self.bend {
                (y, z) = (y * cos - z * sin, y * sin + z * cos);
            }
            // The smallest side is folded about the middle of the gap: the cross through the cube is left out.
            x = self.scale * x - gap;
            y = self.scale * y - gap;
            z = 0.5 * gap - (self.scale * z - 0.5 * gap).abs();
            size *= self.scale;
            trap = trap.min(dot([x, y, z], [x, y, z]));
        }
        (cube([x, y, z]) / size, trap.sqrt().min(2.0) * 0.5)
    }
}

fn random(seed: u64) -> Fractal {
    picked(ATTEMPTS, |attempt| {
        let mut rng = stream(NAME, seed, attempt);
        let folded = rng.next_f64() < 0.55;
        let angle = |rng: &mut Rng| {
            if rng.next_f64() < 0.7 {
                rng.range(-30.0, 30.0).round()
            } else {
                0.0
            }
        };
        let (twist, bend) = if folded {
            (angle(&mut rng), angle(&mut rng))
        } else {
            (0.0, 0.0)
        };
        let scale = if folded && rng.next_bool() {
            (rng.range(2.4, 3.3) * 100.0).round() / 100.0
        } else {
            3.0
        };
        let whole = rng.next_f64() < 0.35;
        let away = if whole { rng.range(4.1, 4.9) } else { rng.range(1.6, 3.4) };
        let mut camera = Camera::default();
        if !whole && rng.next_bool() {
            camera = camera.looking_at(rng.range(-0.25, 0.25), rng.range(-0.25, 0.25), rng.range(1.2, 3.0));
        }
        let colors = random_palette(&mut rng);
        let params = vec![
            ("iterations", if whole { 5.0 } else { 7.0 }),
            ("scale", scale),
            ("twist", twist),
            ("bend", bend),
            ("turn", rng.range(-180.0, 180.0).round()),
            ("tilt", rng.range(-60.0, 70.0).round()),
            ("distance", (away * 100.0).round() / 100.0),
            ("light_turn", rng.range(-70.0, 70.0).round()),
            ("light_tilt", rng.range(15.0, 65.0).round()),
            ("fog", (rng.range(0.2, 0.6) * 100.0).round() / 100.0),
            ("color_shift", (rng.range(0.0, 2.0) * 100.0).round() / 100.0),
        ];
        fitted(&SPEC, colors, camera, params, rng.range(0.7, 1.7), 0.5)
    })
}

fn mutate(fractal: &Fractal, rng: &mut Rng, strength: f64) -> Fractal {
    let mut out = moved(fractal, &DISTANCE, rng, strength * 0.6);
    for angle in [&TWIST, &BEND] {
        let turned = value(&out, angle) + rng.range(-30.0, 30.0) * strength;
        set(&mut out, angle, turned.round());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fractal::{golden, preset};

    #[test]
    fn it_renders_to_its_golden_hash() {
        let hash = golden(&preset("menger_sponge-classic").unwrap());
        assert_eq!(hash, 0x2cfe_a14d_9489_4293, "got {hash:#018x}");
    }

    // The sponge as it is built: a cell of the grid of 3^level belongs to it when none of its digits in base 3
    // has two ones among the three coordinates.
    fn built(point: Point, level: u32) -> bool {
        let side = 3u32.pow(level);
        let cell = point.map(|v| (((v + 1.0) * 0.5 * f64::from(side)) as u32).min(side - 1));
        (0..level).all(|digit| {
            let place = 3u32.pow(digit);
            cell.iter().filter(|v| (*v / place) % 3 == 1).count() < 2
        })
    }

    #[test]
    fn the_folds_give_the_sponge_as_it_is_built() {
        let mut rng = Rng::derive(10, 11, 12);
        for level in 1..=4 {
            let sponge = Sponge::new(f64::from(level), 3.0, 0.0, 0.0);
            for _ in 0..4000 {
                let point = [rng.range(-1.0, 1.0), rng.range(-1.0, 1.0), rng.range(-1.0, 1.0)];
                let (distance, _) = sponge.distance(point);
                assert_eq!(distance <= 0.0, built(point, level), "{level} {point:?}");
            }
            // The middle of a face is a hole: the nearest of the sponge is at its rim.
            let to_the_rim = (4.0f64 + 1.0 / 9.0).sqrt();
            assert!((sponge.distance([3.0, 0.0, 0.0]).0 - to_the_rim).abs() < 1e-12);
        }
    }

    // An estimate that is zero on the solid and never changes faster than the point moves is never too long.
    #[test]
    fn the_estimate_never_changes_faster_than_the_point_moves() {
        let mut rng = Rng::derive(13, 14, 15);
        for (scale, twist, bend) in [(3.0, 0.0, 0.0), (2.6, -30.0, 20.0), (3.3, 24.0, -8.0), (2.0, 45.0, 45.0)] {
            let sponge = Sponge::new(6.0, scale, twist, bend);
            for _ in 0..4000 {
                let from = [rng.range(-2.0, 2.0), rng.range(-2.0, 2.0), rng.range(-2.0, 2.0)];
                let step = [rng.range(-0.3, 0.3), rng.range(-0.3, 0.3), rng.range(-0.3, 0.3)];
                let to = [from[0] + step[0], from[1] + step[1], from[2] + step[2]];
                let change = (sponge.distance(from).0 - sponge.distance(to).0).abs();
                assert!(change <= dot(step, step).sqrt() + 1e-12, "{scale} {twist} {bend} {from:?}");
            }
        }
    }
}

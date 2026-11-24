use super::{
    COLOR_DENSITY, COLOR_SHIFT, FIELD_OF_VIEW, FOG, GLOW, ISOMETRIC, LIGHT_COLOR, LIGHT_TILT, LIGHT_TURN, Marcher, Point,
    SHADOW, Solid, TILT, TURN, dot, fitted, moved, picked, set, value,
};
use crate::fractal::{Draw, Family, Param, Preset, Spec, random_palette, stream};
use crate::rng::Rng;
use crate::scene::{Camera, Fractal, Tone};

const NAME: &str = "mandelbox";
const ATTEMPTS: u64 = 16;
const SCALE: Param = Param::real("scale", 2.0, -4.0, 4.0);
const FOLD: Param = Param::real("fold", 1.0, 0.5, 2.0);
const INNER_RADIUS: Param = Param::real("inner_radius", 0.5, 0.05, 1.0);
const ITERATIONS: Param = Param::whole("iterations", 14.0, 2.0, 100.0).log();
const DISTANCE: Param = Param::real("distance", 30.0, 0.1, 150.0).log();
// A scale nearer to zero than this does not push the points apart: the solid would have no end.
const LEAST_SCALE: f64 = 1.2;
const ESCAPE: f64 = 1e6;

pub(super) static SPEC: Spec = Spec {
    name: NAME,
    family: Family::Solid,
    params: &[
        SCALE,
        FOLD,
        INNER_RADIUS,
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
    draw: Draw::Pixel(|params, camera| {
        Box::new(Marcher::new(
            Mandelbox::new(params[0], params[1], params[2], params[3]),
            &params[4..],
            camera,
        ))
    }),
    random,
    mutate,
    presets: &[
        Preset {
            name: "cube",
            palette: "ice",
            center: [0.0, 0.0],
            zoom: 1.0,
            params: &[("color_density", 2.2), ("color_shift", 0.4)],
        },
        Preset {
            name: "station",
            palette: "emerald",
            center: [0.9866, -0.8072],
            zoom: 2.608,
            params: &[("scale", 2.39), ("inner_radius", 0.61), ("iterations", 16.0), ("turn", 55.0), ("tilt", -22.0), ("distance", 8.23), ("light_turn", -10.0), ("light_tilt", 23.0), ("fog", 0.33), ("color_density", 4.23), ("color_shift", 1.98)],
        },
        Preset {
            name: "vault",
            palette: "ember",
            center: [-0.6741, 0.8231],
            zoom: 1.981,
            params: &[("scale", 2.05), ("inner_radius", 0.28), ("iterations", 16.0), ("turn", 32.0), ("tilt", -24.0), ("distance", 10.51), ("light_turn", -53.0), ("light_tilt", 22.0), ("color_density", 2.88), ("color_shift", 1.78)],
        },
        Preset {
            name: "lagoon",
            palette: "violet",
            center: [-0.0489, -0.4171],
            zoom: 1.9126,
            params: &[("inner_radius", 0.41), ("iterations", 16.0), ("turn", -31.0), ("tilt", 64.0), ("distance", 9.38), ("light_turn", 24.0), ("light_tilt", 61.0), ("fog", 0.51), ("color_density", 4.83), ("color_shift", 1.97)],
        },
        Preset {
            name: "arcade",
            palette: "sunset",
            center: [0.0, 0.0],
            zoom: 1.0,
            params: &[("inner_radius", 0.48), ("iterations", 16.0), ("turn", 19.0), ("tilt", 4.0), ("distance", 10.46), ("light_turn", -19.0), ("light_tilt", 18.0), ("fog", 0.26), ("color_density", 3.83), ("color_shift", 0.64)],
        },
        Preset {
            name: "frost",
            palette: "ice",
            center: [0.0, 0.0],
            zoom: 1.0,
            params: &[("scale", -1.5), ("inner_radius", 0.28), ("iterations", 16.0), ("turn", -38.0), ("tilt", 52.0), ("distance", 4.15), ("light_turn", 34.0), ("light_tilt", 47.0), ("fog", 0.4), ("color_density", 4.92), ("color_shift", 0.2)],
        },
        Preset {
            name: "plaza",
            palette: "amethyst",
            center: [0.0, 0.0],
            zoom: 1.0,
            params: &[("scale", -2.5), ("iterations", 12.0), ("distance", 8.0), ("fog", 0.2), ("turn", 45.0), ("tilt", 35.264), ("light_turn", -40.6), ("light_tilt", 48.6), ("shadow", 1.0), ("glow", 1.0), ("light_color", 1.0), ("isometric", 1.0)],
        },
        Preset {
            name: "kiln",
            palette: "garnet",
            center: [0.3, -0.2],
            zoom: 1.6,
            params: &[("scale", -3.0), ("iterations", 12.0), ("distance", 6.0), ("fog", 0.2), ("turn", 45.0), ("tilt", 35.264), ("light_turn", -40.6), ("light_tilt", 48.6), ("shadow", 1.0), ("glow", 1.0), ("light_color", 1.0), ("isometric", 1.0)],
        },
        Preset {
            name: "blossom",
            palette: "rosewood",
            center: [0.0, 0.0],
            zoom: 1.3,
            params: &[("scale", -3.0), ("iterations", 12.0), ("distance", 6.0), ("fog", 0.2), ("turn", 45.0), ("tilt", 35.264), ("light_turn", -40.6), ("light_tilt", 48.6), ("shadow", 1.0), ("glow", 1.0), ("light_color", 1.0), ("isometric", 1.0)],
        },
        Preset {
            name: "cloister",
            palette: "cobalt",
            center: [0.25, 0.35],
            zoom: 2.4,
            params: &[("scale", -2.5), ("iterations", 12.0), ("distance", 8.0), ("fog", 0.2), ("turn", 45.0), ("tilt", 35.264), ("light_turn", -40.6), ("light_tilt", 48.6), ("shadow", 1.0), ("glow", 1.0), ("light_color", 1.0), ("isometric", 1.0)],
        },
    ],
};

struct Mandelbox {
    scale: f64,
    fold: f64,
    inner: f64,
    iterations: u32,
    half_side: f64,
}

impl Mandelbox {
    fn new(scale: f64, fold: f64, inner_radius: f64, iterations: f64) -> Self {
        let scale = if scale.abs() >= LEAST_SCALE {
            scale
        } else if scale < 0.0 {
            -LEAST_SCALE
        } else {
            LEAST_SCALE
        };
        let half_side = if scale > 0.0 {
            2.0 * fold * (scale + 1.0) / (scale - 1.0)
        } else {
            2.0 * fold
        };
        Self {
            scale,
            fold,
            inner: inner_radius * inner_radius,
            iterations: iterations as u32,
            half_side,
        }
    }
}

impl Solid for Mandelbox {
    const STRIDE: f64 = 0.9;

    fn reach(&self) -> f64 {
        self.half_side * 1.74
    }

    #[inline]
    fn distance(&self, start: Point) -> (f64, f64) {
        let (mut at, mut stretch, mut trap) = (start, 1.0f64, f64::INFINITY);
        let fold = self.fold;
        for _ in 0..self.iterations {
            for axis in &mut at {
                *axis = axis.clamp(-fold, fold) * 2.0 - *axis;
            }
            let square = dot(at, at);
            trap = trap.min(square);
            let grow = if square < self.inner {
                1.0 / self.inner
            } else if square < 1.0 {
                1.0 / square
            } else {
                1.0
            } * self.scale;
            at = [
                at[0] * grow + start[0],
                at[1] * grow + start[1],
                at[2] * grow + start[2],
            ];
            stretch = stretch * grow.abs() + 1.0;
            if dot(at, at) > ESCAPE {
                break;
            }
        }
        (dot(at, at).sqrt() / stretch, trap.sqrt().min(2.0) * 0.5)
    }
}

fn random(seed: u64) -> Fractal {
    picked(ATTEMPTS, |attempt| {
        let mut rng = stream(NAME, seed, attempt);
        let scale = match rng.below(10) {
            0..3 => 2.0,
            3..5 => rng.range(2.0, 3.2),
            5..7 => -1.5,
            7..9 => -rng.range(1.5, 2.6),
            _ => rng.range(1.6, 2.0),
        };
        let scale = (scale * 100.0).round() / 100.0;
        let inner = (rng.range(0.25, 0.7) * 100.0).round() / 100.0;
        let half_side = Mandelbox::new(scale, 1.0, inner, 1.0).half_side;
        let whole = rng.next_f64() < 0.15;
        let away = half_side * if whole { rng.range(4.0, 4.8) } else { rng.range(0.9, 2.2) };
        let mut camera = Camera::default();
        if !whole && rng.next_bool() {
            camera = camera.looking_at(rng.range(-0.25, 0.25), rng.range(-0.25, 0.25), rng.range(1.2, 3.0));
        }
        let colors = random_palette(&mut rng);
        let params = vec![
            ("scale", scale),
            ("inner_radius", inner),
            ("iterations", if whole { 12.0 } else { 16.0 }),
            ("turn", rng.range(-180.0, 180.0).round()),
            ("tilt", rng.range(-60.0, 70.0).round()),
            ("distance", (away * 100.0).round() / 100.0),
            ("light_turn", rng.range(-70.0, 70.0).round()),
            ("light_tilt", rng.range(15.0, 65.0).round()),
            ("fog", (rng.range(0.2, 0.6) * 100.0).round() / 100.0),
            ("color_shift", (rng.range(0.0, 2.0) * 100.0).round() / 100.0),
        ];
        fitted(&SPEC, colors, camera, params, rng.range(0.7, 1.7), 0.24)
    })
}

fn mutate(fractal: &Fractal, rng: &mut Rng, strength: f64) -> Fractal {
    let mut out = moved(fractal, &DISTANCE, rng, strength * 0.25);
    let scale = value(&out, &SCALE) + rng.range(-0.3, 0.3) * strength;
    set(&mut out, &SCALE, scale);
    let inner = value(&out, &INNER_RADIUS) + rng.range(-0.15, 0.15) * strength;
    set(&mut out, &INNER_RADIUS, inner);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fractal::{golden, preset};

    #[test]
    fn it_renders_to_its_golden_hash() {
        let hash = golden(&preset("mandelbox-cube").unwrap());
        assert_eq!(hash, 0x94e8_912e_9e81_b569, "got {hash:#018x}");
    }

    #[test]
    fn its_isometric_view_in_the_light_s_colors_renders_to_its_golden_hash() {
        let hash = golden(&preset("mandelbox-plaza").unwrap());
        assert_eq!(hash, 0xfca9_34b6_542d_8952, "got {hash:#018x}");
    }

    #[test]
    fn the_box_of_scale_two_is_twelve_wide() {
        let solid = Mandelbox::new(2.0, 1.0, 0.5, 30.0);
        assert_eq!(solid.half_side, 6.0);
        // The corner of the fold box maps to itself's opposite orbit and stays; a point past the side leaves.
        assert!(solid.distance([5.9, 0.0, 0.0]).0 < 1e-3);
        assert!(solid.distance([6.5, 0.0, 0.0]).0 > 0.05);
        assert!(solid.distance([0.0, 0.0, 0.0]).0 < 1e-6);
        assert_eq!(Mandelbox::new(-1.5, 1.0, 0.5, 30.0).half_side, 2.0);
        assert_eq!(Mandelbox::new(0.3, 1.0, 0.5, 30.0).scale, LEAST_SCALE);
    }

    #[test]
    fn the_estimate_is_never_longer_than_the_way_to_the_box() {
        let mut rng = Rng::derive(7, 8, 9);
        for scale in [2.0, 3.0, -1.5, -2.0] {
            let solid = Mandelbox::new(scale, 1.0, 0.5, 20.0);
            let side = solid.half_side * 1.2;
            for _ in 0..300 {
                let from = [rng.range(-side, side), rng.range(-side, side), rng.range(-side, side)];
                let (distance, _) = solid.distance(from);
                if distance < 1e-3 {
                    continue;
                }
                for axis in 0..3 {
                    for way in [-1.0, 1.0] {
                        let mut to = from;
                        to[axis] += way * distance * 0.9;
                        assert!(solid.distance(to).0 > distance * 0.01, "{scale} {from:?}");
                    }
                }
            }
        }
    }}

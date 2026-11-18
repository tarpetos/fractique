use super::{COLOR_BY, System, Taste, Traced, framed, mutate_by, random_by};
use crate::fractal::{Draw, Family, Param, Preset, Spec};
use crate::math::{cos, sin};
use crate::rng::Rng;
use crate::scene::{Camera, Fractal, Tone};

pub(super) static SPEC: Spec = Spec {
    name: "de_jong",
    family: Family::Orbit,
    params: &[
        Param::real("a", 1.4, -4.0, 4.0),
        Param::real("b", -2.3, -4.0, 4.0),
        Param::real("c", 2.4, -4.0, 4.0),
        Param::real("d", -2.1, -4.0, 4.0),
        COLOR_BY,
    ],
    tone: Tone {
        brightness: 1.1,
        gamma: 2.8,
        vibrancy: 1.0,
        highlight: 0.2,
        glow_radius: 0.006,
        glow_strength: 0.5,
        smoothing: 1.0,
    },
    draw: Draw::Density(|params, _| Box::new(Traced::new(DeJong::new(params), params[4]))),
    random,
    mutate,
    presets: &[
        Preset {
            name: "classic",
            palette: "sunset",
            center: [0.008, -0.02],
            zoom: 0.849,
            params: &[("a", 1.4), ("b", -2.3), ("c", 2.4), ("d", -2.1)],
        },
        Preset {
            name: "crescent",
            palette: "violet",
            center: [-0.152, -0.903],
            zoom: 1.023,
            params: &[("a", 2.01), ("b", -2.53), ("c", 1.61), ("d", -0.33)],
        },
        Preset {
            name: "arches",
            palette: "ember",
            center: [-0.997, 0.497],
            zoom: 1.128,
            params: &[("a", -2.7), ("b", -0.09), ("c", -0.86), ("d", -2.2), ("color_by", 1.0)],
        },
        Preset {
            name: "bow",
            palette: "aurora",
            center: [-0.059, -0.463],
            zoom: 1.099,
            params: &[("a", -0.827), ("b", -1.637), ("c", 1.659), ("d", -0.943), ("color_by", 2.0)],
        },
        Preset {
            name: "shell",
            palette: "orchid",
            center: [-0.005, -0.054],
            zoom: 0.839,
            params: &[("a", -2.0), ("b", -2.0), ("c", -1.2), ("d", 2.0), ("color_by", 1.0)],
        },
        Preset {
            name: "loop",
            palette: "ice",
            center: [-0.063, -0.267],
            zoom: 0.864,
            params: &[("a", 1.641), ("b", 1.902), ("c", 0.316), ("d", 1.525), ("color_by", 2.0)],
        },
    ],
};

// Tuned on the pictures of seeds 1 to 40 and of the presets.
const TASTE: Taste = Taste {
    trimmed_thousandths: 5,
    covered_hundredths: 15,
    cells_holding_half: 40,
    contrast_hundredths: 30,
};

struct DeJong {
    a: f64,
    b: f64,
    c: f64,
    d: f64,
}

impl DeJong {
    fn new(params: &[f64]) -> Self {
        Self {
            a: params[0],
            b: params[1],
            c: params[2],
            d: params[3],
        }
    }
}

impl System for DeJong {
    type State = (f64, f64);
    const BURN: u32 = 32;
    const FITTED: bool = false;

    fn start(&self, rng: &mut Rng) -> (f64, f64) {
        (rng.range(-0.5, 0.5), rng.range(-0.5, 0.5))
    }

    #[inline]
    fn step(&self, (x, y): (f64, f64)) -> (f64, f64) {
        (sin(self.a * y) - cos(self.b * x), sin(self.c * x) - cos(self.d * y))
    }

    #[inline]
    fn point(&self, state: (f64, f64)) -> (f64, f64) {
        state
    }
}

fn frame(values: &[f64]) -> Option<(Camera, bool)> {
    framed(DeJong::new(values), &TASTE)
}

fn random(seed: u64) -> Fractal {
    random_by(
        &SPEC,
        seed,
        |rng| {
            let mut values: Vec<f64> = (0..4).map(|_| rng.range(-3.2, 3.2)).collect();
            values.push(rng.below(3) as f64);
            values
        },
        frame,
    )
}

fn mutate(fractal: &Fractal, rng: &mut Rng, strength: f64) -> Fractal {
    mutate_by(&SPEC, fractal, rng, strength, &[0.5, 0.5, 0.5, 0.5, 0.0], frame)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fractal::{golden, preset};

    #[test]
    fn it_follows_its_formula() {
        let map = DeJong::new(&[1.4, -2.3, 2.4, -2.1]);
        assert_eq!(map.step((0.0, 0.0)), (-1.0, -1.0));
        let (x, y) = map.step((-1.0, -1.0));
        assert!((x + 0.319_174).abs() < 1e-6 && (y + 0.170_617).abs() < 1e-6, "{x} {y}");
    }

    #[test]
    fn it_renders_to_its_golden_hash() {
        let hash = golden(&preset("de_jong-classic").unwrap());
        assert_eq!(hash, 0xb52d_951c_59c2_ed78, "got {hash:#018x}");
    }
}

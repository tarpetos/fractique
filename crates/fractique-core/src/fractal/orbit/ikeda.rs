use super::{COLOR_BY, System, Taste, Traced, framed, mutate_by, random_by};
use crate::fractal::{Draw, Family, Param, Preset, Spec};
use crate::math::{cos, sin};
use crate::rng::Rng;
use crate::scene::{Camera, Fractal, Tone};

pub(super) static SPEC: Spec = Spec {
    name: "ikeda",
    family: Family::Orbit,
    params: &[
        Param::real("u", 0.9, 0.0, 1.0),
        Param::real("k", 0.4, -4.0, 4.0),
        Param::real("p", 6.0, 0.0, 20.0),
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
    draw: Draw::Density(|params, _| Box::new(Traced::new(Ikeda::new(params), params[3]))),
    random,
    mutate,
    presets: &[
        Preset {
            name: "classic",
            palette: "sunset",
            center: [0.651, -0.599],
            zoom: 1.176,
            params: &[("u", 0.9), ("k", 0.4), ("p", 6.0)],
        },
        Preset {
            name: "coil",
            palette: "aurora",
            center: [0.653, -0.389],
            zoom: 0.851,
            params: &[("u", 0.902), ("k", 1.528), ("p", 19.148), ("color_by", 2.0)],
        },
        Preset {
            name: "wave",
            palette: "violet",
            center: [0.567, -0.307],
            zoom: 1.062,
            params: &[("u", 0.957), ("k", -0.248), ("p", 7.185)],
        },
        Preset {
            name: "rings",
            palette: "gold",
            center: [0.709, 0.054],
            zoom: 1.124,
            params: &[("u", 0.852), ("k", 0.451), ("p", 18.004)],
        },
        Preset {
            name: "ribbon",
            palette: "orchid",
            center: [0.748, -0.148],
            zoom: 1.266,
            params: &[("u", 0.857), ("k", -0.445), ("p", 8.228)],
        },
        Preset {
            name: "flare",
            palette: "ember",
            center: [0.545, -0.258],
            zoom: 1.103,
            params: &[("u", 0.932), ("k", -0.181), ("p", 8.311), ("color_by", 2.0)],
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

struct Ikeda {
    u: f64,
    k: f64,
    p: f64,
}

impl Ikeda {
    fn new(params: &[f64]) -> Self {
        Self {
            u: params[0],
            k: params[1],
            p: params[2],
        }
    }
}

impl System for Ikeda {
    type State = (f64, f64);
    const BURN: u32 = 32;
    const FITTED: bool = false;

    fn start(&self, rng: &mut Rng) -> (f64, f64) {
        (rng.range(-0.5, 0.5), rng.range(-0.5, 0.5))
    }

    #[inline]
    fn step(&self, (x, y): (f64, f64)) -> (f64, f64) {
        let t = self.k - self.p / (1.0 + x * x + y * y);
        let (c, s) = (cos(t), sin(t));
        (1.0 + self.u * (x * c - y * s), self.u * (x * s + y * c))
    }

    #[inline]
    fn point(&self, state: (f64, f64)) -> (f64, f64) {
        state
    }
}

fn frame(values: &[f64]) -> Option<(Camera, bool)> {
    framed(Ikeda::new(values), &TASTE)
}

fn random(seed: u64) -> Fractal {
    random_by(
        &SPEC,
        seed,
        |rng| {
            vec![
                rng.range(0.85, 0.99),
                rng.range(-2.0, 2.0),
                rng.range(2.0, 20.0),
                rng.below(3) as f64,
            ]
        },
        frame,
    )
}

fn mutate(fractal: &Fractal, rng: &mut Rng, strength: f64) -> Fractal {
    mutate_by(&SPEC, fractal, rng, strength, &[0.03, 0.4, 2.0, 0.0], frame)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fractal::{golden, preset};

    #[test]
    fn it_follows_its_formula() {
        let map = Ikeda::new(&[0.9, 0.4, 6.0]);
        let (x, y) = map.step((0.0, 0.0));
        assert!((x - 1.0).abs() < 1e-12 && y.abs() < 1e-12, "{x} {y}");
        let (x, y) = map.step((1.0, 0.0));
        let t = 0.4f64 - 3.0;
        assert!((x - (1.0 + 0.9 * cos(t))).abs() < 1e-12 && (y - 0.9 * sin(t)).abs() < 1e-12, "{x} {y}");
    }

    #[test]
    fn it_renders_to_its_golden_hash() {
        let hash = golden(&preset("ikeda-classic").unwrap());
        assert_eq!(hash, 0xee08_0c55_80ac_0854, "got {hash:#018x}");
    }
}

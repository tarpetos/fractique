use super::{COLOR_BY, System, Taste, Traced, framed, mutate_by, random_by};
use crate::fractal::{Draw, Family, Param, Preset, Spec};
use crate::rng::Rng;
use crate::scene::{Camera, Fractal, Tone};

pub(super) static SPEC: Spec = Spec {
    name: "gumowski_mira",
    family: Family::Orbit,
    params: &[
        Param::real("mu", -0.496, -1.0, 1.0),
        Param::real("a", 0.008, 0.0, 0.1),
        Param::real("b", 0.05, 0.0, 1.0),
        Param::real("scatter", 4.0, 0.1, 20.0).log(),
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
    draw: Draw::Density(|params, _| Box::new(Traced::new(GumowskiMira::new(params), params[4]))),
    random,
    mutate,
    presets: &[
        Preset {
            name: "feathers",
            palette: "sunset",
            center: [0.025, -0.066],
            zoom: 1.449,
            params: &[("color_by", 2.0)],
        },
        Preset {
            name: "wings",
            palette: "violet",
            center: [-0.008, -0.023],
            zoom: 1.477,
            params: &[("mu", -0.801), ("a", 0.009), ("color_by", 2.0)],
        },
        Preset {
            name: "ring",
            palette: "aurora",
            center: [0.009, -0.033],
            zoom: 1.547,
            params: &[("mu", 0.31), ("color_by", 1.0)],
        },
        Preset {
            name: "star",
            palette: "orchid",
            center: [0.019, -0.009],
            zoom: 1.434,
            params: &[("mu", -0.7), ("color_by", 2.0)],
        },
        Preset {
            name: "thorns",
            palette: "ice",
            center: [-0.002, -0.016],
            zoom: 1.362,
            params: &[("mu", -0.9), ("color_by", 2.0)],
        },
        Preset {
            name: "fan",
            palette: "ember",
            center: [-0.003, 0.003],
            zoom: 1.556,
            params: &[("mu", 0.5), ("color_by", 2.0)],
        },
    ],
};

// Tuned on the pictures of seeds 1 to 40 and of the presets.
const TASTE: Taste = Taste {
    trimmed_thousandths: 5,
    covered_hundredths: 12,
    cells_holding_half: 16,
    contrast_hundredths: 85,
};

const SPAN: u64 = 200_000;

struct GumowskiMira {
    mu: f64,
    a: f64,
    b: f64,
    scatter: f64,
}

impl GumowskiMira {
    fn new(params: &[f64]) -> Self {
        Self {
            mu: params[0],
            a: params[1],
            b: params[2],
            scatter: params[3],
        }
    }

    #[inline]
    fn bend(&self, x: f64) -> f64 {
        let x2 = x * x;
        self.mu * x + 2.0 * (1.0 - self.mu) * x2 / (1.0 + x2)
    }
}

impl System for GumowskiMira {
    type State = (f64, f64);
    const BURN: u32 = 2000;
    const FITTED: bool = true;

    fn start(&self, rng: &mut Rng) -> (f64, f64) {
        (rng.range(-self.scatter, self.scatter), rng.range(-self.scatter, self.scatter))
    }

    #[inline]
    fn step(&self, (x, y): (f64, f64)) -> (f64, f64) {
        let next = y + self.a * (1.0 - self.b * y * y) * y + self.bend(x);
        (next, self.bend(next) - x)
    }

    #[inline]
    fn point(&self, state: (f64, f64)) -> (f64, f64) {
        state
    }

    fn span(&self) -> u64 {
        SPAN
    }
}

fn frame(values: &[f64]) -> Option<(Camera, bool)> {
    framed(GumowskiMira::new(values), &TASTE)
}

fn random(seed: u64) -> Fractal {
    random_by(
        &SPEC,
        seed,
        |rng| {
            vec![
                rng.range(-0.95, 0.6),
                rng.range(0.003, 0.02),
                rng.range(0.01, 0.2),
                rng.range(1.0, 8.0),
                rng.below(3) as f64,
            ]
        },
        frame,
    )
}

fn mutate(fractal: &Fractal, rng: &mut Rng, strength: f64) -> Fractal {
    mutate_by(&SPEC, fractal, rng, strength, &[0.08, 0.004, 0.03, 0.0, 0.0], frame)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fractal::{golden, preset};

    #[test]
    fn it_follows_its_formula() {
        let still = GumowskiMira::new(&[-0.5, 0.0, 0.0, 1.0]);
        assert_eq!(still.step((1.0, 0.0)), (1.0, 0.0));
        let map = GumowskiMira::new(&[-0.5, 0.01, 0.05, 1.0]);
        let (x, y) = map.step((0.0, 2.0));
        assert!((x - 2.016).abs() < 1e-12 && (y - 1.399_613).abs() < 1e-6, "{x} {y}");
    }

    #[test]
    fn it_renders_to_its_golden_hash() {
        let hash = golden(&preset("gumowski_mira-feathers").unwrap());
        assert_eq!(hash, 0x7f58_535b_cda7_63d6, "got {hash:#018x}");
    }
}

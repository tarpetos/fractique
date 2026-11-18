use super::{COLOR_BY, System, Taste, Traced, framed, mutate_by, random_by};
use crate::fractal::{Draw, Family, Param, Preset, Spec};
use crate::rng::Rng;
use crate::scene::{Camera, Fractal, Tone};

pub(super) static SPEC: Spec = Spec {
    name: "hopalong",
    family: Family::Orbit,
    params: &[
        Param::real("a", 0.4, -100.0, 100.0),
        Param::real("b", 1.0, 0.0, 10.0),
        Param::real("c", 0.0, -100.0, 100.0),
        Param::whole("orbit_length", 300_000.0, 1000.0, 10_000_000.0).log(),
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
    draw: Draw::Density(|params, _| Box::new(Traced::new(Hopalong::new(params), params[4]))),
    random,
    mutate,
    presets: &[
        Preset {
            name: "web",
            palette: "ember",
            center: [0.097, -0.035],
            zoom: 1.169,
            params: &[("a", 7.17), ("b", 8.44), ("c", 2.56), ("color_by", 2.0)],
        },
        Preset {
            name: "lattice",
            palette: "violet",
            center: [-0.002, 0.004],
            zoom: 1.926,
            params: &[("a", -100.0), ("b", 0.1), ("c", -80.0), ("color_by", 2.0)],
        },
        Preset {
            name: "mandala",
            palette: "aurora",
            center: [-0.012, -0.01],
            zoom: 1.259,
            params: &[("a", -3.15), ("b", 0.3), ("c", 0.3)],
        },
        Preset {
            name: "tiles",
            palette: "ice",
            center: [0.012, 0.008],
            zoom: 1.949,
            params: &[("a", -11.0), ("b", 0.05), ("c", 6.0), ("color_by", 2.0)],
        },
        Preset {
            name: "lace",
            palette: "orchid",
            center: [-0.005, 0.015],
            zoom: 1.459,
            params: &[("a", -7.0), ("b", 3.0), ("c", 5.0)],
        },
        Preset {
            name: "classic",
            palette: "sunset",
            center: [0.0, 0.0],
            zoom: 0.6,
            params: &[("color_by", 2.0)],
        },
    ],
};

// Tuned on the pictures of seeds 1 to 40 and of the presets.
const TASTE: Taste = Taste {
    trimmed_thousandths: 40,
    covered_hundredths: 30,
    cells_holding_half: 15,
    contrast_hundredths: 60,
};

const START: f64 = 0.1;

struct Hopalong {
    a: f64,
    b: f64,
    c: f64,
    steps: u64,
}

impl Hopalong {
    fn new(params: &[f64]) -> Self {
        Self {
            a: params[0],
            b: params[1],
            c: params[2],
            steps: params[3] as u64,
        }
    }
}

impl System for Hopalong {
    type State = (f64, f64);
    const BURN: u32 = 0;
    const FITTED: bool = true;

    fn start(&self, rng: &mut Rng) -> (f64, f64) {
        (rng.range(-START, START), rng.range(-START, START))
    }

    #[inline]
    fn step(&self, (x, y): (f64, f64)) -> (f64, f64) {
        let root = (self.b * x - self.c).abs().sqrt();
        let hop = if x > 0.0 {
            root
        } else if x < 0.0 {
            -root
        } else {
            0.0
        };
        (y - hop, self.a - x)
    }

    #[inline]
    fn point(&self, state: (f64, f64)) -> (f64, f64) {
        state
    }

    fn span(&self) -> u64 {
        self.steps
    }
}

fn frame(values: &[f64]) -> Option<(Camera, bool)> {
    framed(Hopalong::new(values), &TASTE)
}

fn random(seed: u64) -> Fractal {
    random_by(
        &SPEC,
        seed,
        |rng| {
            vec![
                rng.range(-12.0, 12.0),
                rng.range(0.05, 6.0),
                rng.range(-12.0, 12.0),
                (30_000.0 * crate::math::pow(20.0, rng.next_f64())).round(),
                rng.below(3) as f64,
            ]
        },
        frame,
    )
}

fn mutate(fractal: &Fractal, rng: &mut Rng, strength: f64) -> Fractal {
    mutate_by(&SPEC, fractal, rng, strength, &[1.0, 0.3, 1.0, 0.0, 0.0], frame)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fractal::{golden, preset};

    #[test]
    fn it_follows_its_formula() {
        let map = Hopalong::new(&[0.4, 1.0, 0.0, 1000.0]);
        assert_eq!(map.step((1.0, 2.0)), (1.0, -0.6));
        assert_eq!(map.step((-4.0, 0.0)), (2.0, 4.4));
        assert_eq!(map.step((0.0, 3.0)), (3.0, 0.4));
    }

    #[test]
    fn it_renders_to_its_golden_hash() {
        let hash = golden(&preset("hopalong-web").unwrap());
        assert_eq!(hash, 0x9a7c_0e18_28f3_df28, "got {hash:#018x}");
    }
}

use super::{COLOR_BY, System, Taste, Traced, framed, mutate_by, random_by};
use crate::fractal::{Draw, Family, Param, Preset, Spec};
use crate::math::{cos, sin};
use crate::rng::Rng;
use crate::scene::{Camera, Fractal, Tone};

// The flows, in the order of the parameter: the time step that follows each smoothly, how long one of its
// pictures runs at the length 1, and where its orbit starts.
const FLOWS: [(f64, f64, [f64; 3]); 9] = [
    (0.05, 1500.0, [0.1, 0.0, 0.0]),
    (0.005, 120.0, [-1.48, -1.51, 2.04]),
    (0.002, 40.0, [-0.1, 0.5, -0.6]),
    (0.005, 150.0, [1.0, 1.0, 0.0]),
    (0.02, 600.0, [0.1, 0.0, 0.0]),
    (0.01, 300.0, [0.63, 0.47, -0.54]),
    (0.02, 900.0, [1.3, -0.18, 0.01]),
    (0.01, 250.0, [0.1, 0.0, 0.0]),
    (0.005, 150.0, [-1.0, 0.0, 0.5]),
];

pub(super) static SPEC: Spec = Spec {
    name: "flow",
    family: Family::Orbit,
    params: &[
        Param::whole("system", 1.0, 1.0, 9.0),
        Param::real("turn", 0.0, -180.0, 180.0),
        Param::real("tilt", 0.0, -90.0, 90.0),
        Param::real("length", 1.0, 0.05, 50.0).log(),
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
    draw: Draw::Density(|params, _| Box::new(Traced::new(Flow::new(params), params[4]))),
    random,
    mutate,
    presets: &[
        Preset {
            name: "thomas",
            palette: "cobalt",
            center: [0.0, 0.0],
            zoom: 1.6,
            params: &[("system", 1.0), ("turn", 30.0), ("tilt", 20.0), ("length", 3.0), ("color_by", 1.0)],
        },
        Preset {
            name: "halvorsen",
            palette: "rosewood",
            center: [0.0, 0.0],
            zoom: 1.6,
            params: &[("system", 2.0), ("turn", 45.0), ("tilt", 35.0), ("color_by", 1.0)],
        },
        Preset {
            name: "chen",
            palette: "abyss",
            center: [0.0, 0.0],
            zoom: 1.6,
            params: &[("system", 3.0), ("turn", 20.0), ("tilt", -10.0), ("color_by", 1.0)],
        },
        Preset {
            name: "dadras",
            palette: "lagoon",
            center: [0.0, 0.0],
            zoom: 1.6,
            params: &[("system", 4.0), ("turn", 60.0), ("tilt", 30.0), ("color_by", 1.0)],
        },
        Preset {
            name: "aizawa",
            palette: "bronze",
            center: [0.0, 0.0],
            zoom: 1.6,
            params: &[("system", 8.0), ("turn", 0.0), ("tilt", 70.0), ("color_by", 1.0)],
        },
        Preset {
            name: "four_wings",
            palette: "amethyst",
            center: [0.0, 0.0],
            zoom: 1.6,
            params: &[("system", 7.0), ("turn", 20.0), ("tilt", 40.0), ("color_by", 1.0)],
        },
    ],
};

// Tuned on the pictures of seeds 1 to 40 and of the presets.
const TASTE: Taste = Taste {
    trimmed_thousandths: 0,
    covered_hundredths: 14,
    cells_holding_half: 40,
    contrast_hundredths: 30,
};

struct Flow {
    system: usize,
    time_step: f64,
    start: [f64; 3],
    turn: (f64, f64),
    tilt: (f64, f64),
    steps: u64,
}

impl Flow {
    fn new(params: &[f64]) -> Self {
        let system = (params[0] as usize).clamp(1, FLOWS.len()) - 1;
        let (time_step, span, start) = FLOWS[system];
        let (turn, tilt) = (params[1].to_radians(), params[2].to_radians());
        Self {
            system,
            time_step,
            start,
            turn: (cos(turn), sin(turn)),
            tilt: (cos(tilt), sin(tilt)),
            steps: (params[3] * span / time_step) as u64,
        }
    }

    #[inline]
    fn flow(&self, [x, y, z]: [f64; 3]) -> [f64; 3] {
        match self.system {
            0 => [sin(y) - 0.208186 * x, sin(z) - 0.208186 * y, sin(x) - 0.208186 * z],
            1 => [
                -1.4 * x - 4.0 * y - 4.0 * z - y * y,
                -1.4 * y - 4.0 * z - 4.0 * x - z * z,
                -1.4 * z - 4.0 * x - 4.0 * y - x * x,
            ],
            2 => [35.0 * (y - x), -7.0 * x - x * z + 28.0 * y, x * y - 3.0 * z],
            3 => [y - 3.0 * x + 2.7 * y * z, 1.7 * y - x * z + z, 2.0 * x * y - 9.0 * z],
            4 => [-y - z, x + 0.2 * y, 0.2 + z * (x - 5.7)],
            5 => [y + 2.07 * x * y + x * z, 1.0 - 1.79 * x * x + y * z, x - x * x - y * y],
            6 => [0.2 * x + y * z, 0.01 * x - 0.4 * y - x * z, -z - x * y],
            7 => [
                (z - 0.7) * x - 3.5 * y,
                3.5 * x + (z - 0.7) * y,
                0.6 + 0.95 * z - z * z * z / 3.0 - (x * x + y * y) * (1.0 + 0.25 * z) + 0.1 * z * x * x * x,
            ],
            _ => [
                y * (z - 1.0 + x * x) + 0.87 * x,
                x * (3.0 * z + 1.0 - x * x) + 0.87 * y,
                -2.0 * z * (1.1 + x * y),
            ],
        }
    }
}

#[inline]
fn ahead(at: [f64; 3], by: [f64; 3], time: f64) -> [f64; 3] {
    [at[0] + by[0] * time, at[1] + by[1] * time, at[2] + by[2] * time]
}

impl System for Flow {
    type State = [f64; 3];
    const BURN: u32 = 4000;
    const FITTED: bool = true;
    const STRIDE: u64 = 16;
    const BLEND: bool = true;
    const SHARED: bool = true;

    // Every stream follows the one orbit, so that a short one shows as lines.
    fn start(&self, _: &mut Rng) -> [f64; 3] {
        self.start
    }

    fn span(&self) -> u64 {
        self.steps
    }

    #[inline]
    fn step(&self, at: [f64; 3]) -> [f64; 3] {
        let h = self.time_step;
        let k1 = self.flow(at);
        let k2 = self.flow(ahead(at, k1, h * 0.5));
        let k3 = self.flow(ahead(at, k2, h * 0.5));
        let k4 = self.flow(ahead(at, k3, h));
        let sum = |i: usize| k1[i] + 2.0 * (k2[i] + k3[i]) + k4[i];
        ahead(at, [sum(0), sum(1), sum(2)], h / 6.0)
    }

    #[inline]
    fn point(&self, [x, y, z]: [f64; 3]) -> (f64, f64) {
        let across = x * self.turn.0 - y * self.turn.1;
        let depth = x * self.turn.1 + y * self.turn.0;
        (across, z * self.tilt.0 + depth * self.tilt.1)
    }

    #[inline]
    fn pace(&self, from: [f64; 3], to: [f64; 3]) -> f64 {
        let (dx, dy, dz) = (to[0] - from[0], to[1] - from[1], to[2] - from[2]);
        (dx * dx + dy * dy + dz * dz).sqrt()
    }

    #[inline]
    fn height(&self, state: [f64; 3]) -> Option<f64> {
        Some(state[2])
    }

    // The cosine and the sine of the turn, then those of the tilt.
    fn numbers(&self) -> Vec<f64> {
        vec![self.turn.0, self.turn.1, self.tilt.0, self.tilt.1]
    }

    fn coordinates(&self, state: [f64; 3]) -> [f64; 3] {
        state
    }
}

fn frame(values: &[f64]) -> Option<(Camera, bool)> {
    framed(Flow::new(values), &TASTE)
}

fn random(seed: u64) -> Fractal {
    random_by(
        &SPEC,
        seed,
        |rng| {
            vec![
                (rng.below(FLOWS.len() as u64) + 1) as f64,
                rng.range(-180.0, 180.0),
                rng.range(-90.0, 90.0),
                crate::math::pow(4.0, rng.range(-1.0, 1.0)),
                rng.below(3) as f64,
            ]
        },
        frame,
    )
}

fn mutate(fractal: &Fractal, rng: &mut Rng, strength: f64) -> Fractal {
    mutate_by(&SPEC, fractal, rng, strength, &[0.0, 40.0, 25.0, 0.0, 0.0], frame)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fractal::{golden, preset};

    #[test]
    fn every_flow_stays_bounded_and_moves() {
        for system in 1..=FLOWS.len() {
            let flow = Flow::new(&[system as f64, 0.0, 0.0, 1.0, 0.0]);
            let mut at = flow.start(&mut Rng::derive(0, 0, 0));
            for _ in 0..20_000 {
                at = flow.step(at);
            }
            let next = flow.step(at);
            assert!(at.iter().all(|v| v.is_finite() && v.abs() < 1e3), "{system} {at:?}");
            assert!(flow.pace(at, next) > 0.0, "{system}");
        }
    }

    #[test]
    fn it_renders_to_its_golden_hash() {
        let hash = golden(&preset("flow-chen").unwrap());
        assert_eq!(hash, 0xaa6b_46a5_290e_338a, "got {hash:#018x}");
    }
}

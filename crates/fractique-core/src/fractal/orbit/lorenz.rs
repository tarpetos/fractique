use super::{COLOR_BY, System, Taste, Traced, framed, mutate_by, random_by};
use crate::fractal::{Draw, Family, Param, Preset, Spec};
use crate::math::{cos, sin};
use crate::rng::Rng;
use crate::scene::{Camera, Fractal, Tone};

pub(super) static SPEC: Spec = Spec {
    name: "lorenz",
    family: Family::Orbit,
    params: &[
        Param::real("sigma", 10.0, 1.0, 40.0),
        Param::real("rho", 28.0, 1.0, 250.0),
        Param::real("beta", 8.0 / 3.0, 0.1, 10.0),
        Param::real("turn", 0.0, -180.0, 180.0),
        Param::real("tilt", 0.0, -90.0, 90.0),
        Param::real("duration", 150.0, 5.0, 100_000.0).log(),
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
    draw: Draw::Density(|params, _| Box::new(Traced::new(Lorenz::new(params), params[6]))),
    random,
    mutate,
    presets: &[
        Preset {
            name: "butterfly",
            palette: "sunset",
            center: [-0.066, -0.022],
            zoom: 1.206,
            params: &[("duration", 120.0)],
        },
        Preset {
            name: "above",
            palette: "aurora",
            center: [-0.035, -0.022],
            zoom: 1.244,
            params: &[("tilt", 90.0), ("duration", 200.0), ("color_by", 1.0)],
        },
        Preset {
            name: "owl",
            palette: "violet",
            center: [0.024, -0.032],
            zoom: 1.26,
            params: &[("turn", 90.0), ("duration", 400.0)],
        },
        Preset {
            name: "turned",
            palette: "ember",
            center: [0.032, 0.032],
            zoom: 1.283,
            params: &[("turn", 120.0), ("tilt", 30.0), ("duration", 300.0), ("color_by", 2.0)],
        },
        Preset {
            name: "knot",
            palette: "gold",
            center: [-0.001, 0.049],
            zoom: 1.572,
            params: &[("rho", 132.5)],
        },
        Preset {
            name: "veil",
            palette: "ice",
            center: [-0.028, -0.028],
            zoom: 1.214,
            params: &[("duration", 5000.0), ("color_by", 2.0)],
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

const TIME_STEP: f64 = 0.005;

struct Lorenz {
    sigma: f64,
    rho: f64,
    beta: f64,
    turn: (f64, f64),
    tilt: (f64, f64),
    steps: u64,
}

impl Lorenz {
    fn new(params: &[f64]) -> Self {
        let (turn, tilt) = (params[3].to_radians(), params[4].to_radians());
        Self {
            sigma: params[0],
            rho: params[1],
            beta: params[2],
            turn: (cos(turn), sin(turn)),
            tilt: (cos(tilt), sin(tilt)),
            steps: (params[5] / TIME_STEP) as u64,
        }
    }

    #[inline]
    fn flow(&self, [x, y, z]: [f64; 3]) -> [f64; 3] {
        [self.sigma * (y - x), x * (self.rho - z) - y, x * y - self.beta * z]
    }
}

#[inline]
fn ahead(at: [f64; 3], by: [f64; 3], time: f64) -> [f64; 3] {
    [at[0] + by[0] * time, at[1] + by[1] * time, at[2] + by[2] * time]
}

impl System for Lorenz {
    type State = [f64; 3];
    const BURN: u32 = 4000;
    const FITTED: bool = true;
    const STRIDE: u64 = 16;
    const BLEND: bool = true;

    // Every stream follows the one orbit, so that a short one shows as lines.
    fn start(&self, _: &mut Rng) -> [f64; 3] {
        [1.0, 1.0, self.rho]
    }

    fn span(&self) -> u64 {
        self.steps
    }

    #[inline]
    fn step(&self, at: [f64; 3]) -> [f64; 3] {
        let k1 = self.flow(at);
        let k2 = self.flow(ahead(at, k1, TIME_STEP * 0.5));
        let k3 = self.flow(ahead(at, k2, TIME_STEP * 0.5));
        let k4 = self.flow(ahead(at, k3, TIME_STEP));
        let sum = |i: usize| k1[i] + 2.0 * (k2[i] + k3[i]) + k4[i];
        ahead(at, [sum(0), sum(1), sum(2)], TIME_STEP / 6.0)
    }

    #[inline]
    fn point(&self, [x, y, z]: [f64; 3]) -> (f64, f64) {
        let across = x * self.turn.0 - y * self.turn.1;
        let depth = x * self.turn.1 + y * self.turn.0;
        (across, (z - (self.rho - 1.0)) * self.tilt.0 + depth * self.tilt.1)
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
}

fn frame(values: &[f64]) -> Option<(Camera, bool)> {
    framed(Lorenz::new(values), &TASTE)
}

fn random(seed: u64) -> Fractal {
    random_by(
        &SPEC,
        seed,
        |rng| {
            vec![
                rng.range(4.0, 20.0),
                rng.range(25.0, 120.0),
                rng.range(0.5, 4.0),
                rng.range(-180.0, 180.0),
                rng.range(-90.0, 90.0),
                (60.0 * crate::math::pow(30.0, rng.next_f64())).round(),
                rng.below(3) as f64,
            ]
        },
        frame,
    )
}

fn mutate(fractal: &Fractal, rng: &mut Rng, strength: f64) -> Fractal {
    mutate_by(&SPEC, fractal, rng, strength, &[2.0, 8.0, 0.4, 40.0, 25.0, 0.0, 0.0], frame)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fractal::{golden, preset};

    #[test]
    fn its_fixed_points_stay_where_they_are() {
        let flow = Lorenz::new(&[10.0, 28.0, 8.0 / 3.0, 0.0, 0.0, 100.0]);
        assert_eq!(flow.step([0.0; 3]), [0.0; 3]);
        let side = (8.0f64 / 3.0 * 27.0).sqrt();
        let after = flow.step([side, side, 27.0]);
        for (now, before) in after.into_iter().zip([side, side, 27.0]) {
            assert!((now - before).abs() < 1e-12, "{after:?}");
        }
    }

    #[test]
    fn a_step_is_of_the_fourth_order() {
        // x' = -10 x where y = 0 and z = 0 stay: the step is the series of exp(-10 h) up to h^4.
        let flow = Lorenz::new(&[10.0, 0.0, 1.0, 0.0, 0.0, 100.0]);
        let h = 10.0 * TIME_STEP;
        let series = 1.0 - h + h * h / 2.0 - h * h * h / 6.0 + h * h * h * h / 24.0;
        let after = flow.step([1.0, 0.0, 0.0]);
        assert!((after[0] - series).abs() < 1e-15, "{after:?}");
    }

    #[test]
    fn the_angles_turn_the_view() {
        let front = Lorenz::new(&[10.0, 28.0, 8.0 / 3.0, 0.0, 0.0, 100.0]);
        assert_eq!(front.point([3.0, 4.0, 30.0]), (3.0, 3.0));
        let (x, y) = Lorenz::new(&[10.0, 28.0, 8.0 / 3.0, 90.0, 90.0, 100.0]).point([3.0, 4.0, 30.0]);
        assert!((x + 4.0).abs() < 1e-12 && (y - 3.0).abs() < 1e-12, "{x} {y}");
    }

    #[test]
    fn it_renders_to_its_golden_hash() {
        let hash = golden(&preset("lorenz-butterfly").unwrap());
        assert_eq!(hash, 0xe232_2f9f_af81_7b14, "got {hash:#018x}");
    }
}

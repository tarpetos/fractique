use crate::fractal::density;
use crate::fractal::{Draw, Family, Orbit, Param, Preset, Spec, Walk, random_palette, stream};
use crate::math::{cos, sin};
use crate::random::GRID;
use crate::rng::Rng;
use crate::scene::{Camera, Fractal, Tone};

const NAME: &str = "clifford";
const ATTEMPTS: u64 = 40;
const NAMES: [&str; 4] = ["a", "b", "c", "d"];

pub(super) static SPEC: Spec = Spec {
    name: NAME,
    family: Family::Orbit,
    params: &[
        Param::real("a", -1.4, -3.0, 3.0),
        Param::real("b", 1.6, -3.0, 3.0),
        Param::real("c", 1.0, -3.0, 3.0),
        Param::real("d", 0.7, -3.0, 3.0),
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
    draw: Draw::Density(|params, _| Box::new(Clifford::new(params))),
    random,
    mutate,
    presets: &[
        Preset {
            name: "classic",
            palette: "sunset",
            center: [0.3, 0.0],
            zoom: 0.95,
            params: &[("a", -1.4), ("b", 1.6), ("c", 1.0), ("d", 0.7)],
        },
        Preset {
            name: "wings",
            palette: "violet",
            center: [-0.062, -0.234],
            zoom: 0.89,
            params: &[("a", 1.37), ("b", -1.417), ("c", -1.08), ("d", -0.853)],
        },
        Preset {
            name: "ember",
            palette: "ember",
            center: [-0.01, -0.175],
            zoom: 0.98,
            params: &[("a", 2.527), ("b", -1.536), ("c", -0.691), ("d", -0.4)],
        },
        Preset {
            name: "ribbon",
            palette: "gold",
            center: [0.045, 0.087],
            zoom: 0.9,
            params: &[("a", -2.5), ("b", -1.396), ("c", -0.556), ("d", 0.864)],
        },
        Preset {
            name: "strata",
            palette: "orchid",
            center: [0.098, 0.385],
            zoom: 0.745,
            params: &[("a", -2.568), ("b", -1.162), ("c", 1.37), ("d", 1.27)],
        },
        Preset {
            name: "lattice",
            palette: "aurora",
            center: [0.0, -0.026],
            zoom: 0.7,
            params: &[("a", 1.67), ("b", 2.33), ("c", 1.253), ("d", 1.256)],
        },
    ],
};

struct Clifford {
    a: f64,
    b: f64,
    c: f64,
    d: f64,
}

impl Clifford {
    fn new(params: &[f64]) -> Self {
        Self {
            a: params[0],
            b: params[1],
            c: params[2],
            d: params[3],
        }
    }

    #[inline]
    fn step(&self, x: f64, y: f64) -> (f64, f64, f64) {
        let lift = sin(self.a * y);
        (
            lift + self.c * cos(self.a * x),
            sin(self.b * x) + self.d * cos(self.b * y),
            0.5 + 0.5 * lift,
        )
    }
}

struct Path<'a> {
    map: &'a Clifford,
    at: (f64, f64),
}

impl Orbit for Clifford {
    fn walk(&self, rng: &mut Rng) -> Box<dyn Walk + '_> {
        let mut at = (rng.range(-0.5, 0.5), rng.range(-0.5, 0.5));
        for _ in 0..32 {
            let (x, y, _) = self.step(at.0, at.1);
            at = (x, y);
        }
        Box::new(Path { map: self, at })
    }
}

impl Walk for Path<'_> {
    fn run(&mut self, _: &mut Rng, count: u64, plot: &mut dyn FnMut(f64, f64, f64)) {
        let (mut x, mut y) = self.at;
        for _ in 0..count {
            let color;
            (x, y, color) = self.map.step(x, y);
            plot(x, y, color);
        }
        self.at = (x, y);
    }
}

// The camera that frames the attractor, when it is one worth a picture: not a few points, a curve or a fog.
fn framed(values: &[f64; 4]) -> Option<Camera> {
    let probe = density::probe(&Clifford::new(values))?;
    let covered = probe.cells.iter().filter(|hits| **hits > 0).count();
    let mut sorted = probe.cells.clone();
    sorted.sort_unstable_by(|a, b| b.cmp(a));
    let mut sum = 0;
    let half = sorted
        .iter()
        .take_while(|hits| {
            let short = sum * 2 < probe.inside;
            sum += **hits as usize;
            short
        })
        .count();
    // Tuned on the pictures of seeds 1 to 80: half of the points in under 20 cells is a dot in a dust.
    let good = probe.framed()
        && covered * 100 >= GRID * GRID * 22
        && half >= 20
        && probe.contrast() * 100 >= probe.inside * 30;
    good.then_some(probe.camera)
}

fn rounded(values: [f64; 4]) -> [f64; 4] {
    values.map(|value| (value * 1e3).round() / 1e3)
}

fn with(colors: &str, camera: Camera, values: &[f64; 4]) -> Fractal {
    let params: Vec<_> = NAMES.into_iter().zip(*values).collect();
    SPEC.fractal(colors, camera, &params)
}

fn random(seed: u64) -> Fractal {
    for attempt in 0..ATTEMPTS {
        let mut rng = stream(NAME, seed, attempt);
        let sign = |rng: &mut Rng| if rng.next_bool() { 1.0 } else { -1.0 };
        let values = rounded([
            rng.range(1.1, 2.6) * sign(&mut rng),
            rng.range(1.1, 2.6) * sign(&mut rng),
            rng.range(-1.6, 1.6),
            rng.range(-1.6, 1.6),
        ]);
        if let Some(camera) = framed(&values) {
            return with(random_palette(&mut rng), camera, &values);
        }
    }
    with("sunset", Camera::default(), &[-1.4, 1.6, 1.0, 0.7])
}

fn mutate(fractal: &Fractal, rng: &mut Rng, strength: f64) -> Fractal {
    let before = SPEC.values(fractal);
    let mut out = fractal.clone();
    for _ in 0..12 {
        let values = rounded([0, 1, 2, 3].map(|at| (before[at] + rng.range(-0.5, 0.5) * strength).clamp(-3.0, 3.0)));
        if let Some(camera) = framed(&values) {
            out.camera = Camera {
                rotation: fractal.camera.rotation,
                flip_y: fractal.camera.flip_y,
                ..camera
            };
            out.params = NAMES.into_iter().map(str::to_owned).zip(values).collect();
            break;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use crate::fractal::{golden, preset};

    #[test]
    fn it_renders_to_its_golden_hash() {
        let hash = golden(&preset("clifford-classic").unwrap());
        assert_eq!(hash, 0x9a8b_838c_1b5f_49a8, "got {hash:#018x}");
    }
}

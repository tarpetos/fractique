use super::{COLOR_DENSITY, COLOR_SHIFT, busy, moved, number};
use crate::fractal::pixel::{Frame, PROBE};
use crate::fractal::{Draw, Family, Field, Param, Preset, Sample, Spec, Text, random_palette, stream};
use crate::math::{exp, ln};
use crate::rng::Rng;
use crate::scene::{Camera, Fractal, Tone};

const NAME: &str = "lyapunov";
const ATTEMPTS: u64 = 40;
const LONGEST: usize = 64;
const SEQUENCE: &str = "sequence";

const ITERATIONS: Param = Param::whole("iterations", 400.0, 20.0, 20_000.0).log();
const WARMUP: Param = Param::whole("warmup", 100.0, 0.0, 5000.0);
const START: Param = Param::real("start", 0.5, 0.01, 0.99);
const CHAOS: Param = Param::real("chaos", 0.3, 0.0, 1.0);

// The views of the presets keep a picture up to 2.2 times as long as it is high where both rates are under 4.
pub(super) static SPEC: Spec = Spec {
    name: NAME,
    family: Family::Geometric,
    params: &[ITERATIONS, WARMUP, START, COLOR_DENSITY, COLOR_SHIFT, CHAOS],
    tone: Tone {
        brightness: 1.0,
        gamma: 2.5,
        vibrancy: 1.0,
        highlight: 0.0,
        glow_radius: 0.004,
        glow_strength: 0.1,
        smoothing: 1.0,
    },
    draw: Draw::PixelText(
        &[Text {
            name: SEQUENCE,
            default: "AB",
            check: checked,
            presets: &[
                ("zircon-zity", "BBBBBBAAAAAA"),
                ("feathers", "AABAB"),
                ("reeds", "BBABA"),
                ("lattice", "AAAABBB"),
                ("web", "ABBBA"),
            ],
        }],
        |fractal| Box::new(Lyapunov::new(fractal)),
    ),
    random,
    mutate,
    presets: &[
        Preset {
            name: "ribbons",
            palette: "gold",
            center: [3.0, 3.0],
            zoom: 4.4,
            params: &[("color_density", 1.6)],
        },
        Preset {
            name: "zircon-zity",
            palette: "sunset",
            center: [3.0, 2.95],
            zoom: 4.4,
            params: &[("color_density", 1.4), ("color_shift", 0.3)],
        },
        Preset {
            name: "feathers",
            palette: "ice",
            center: [3.3, 3.3],
            zoom: 6.3,
            params: &[("color_density", 1.8)],
        },
        Preset {
            name: "reeds",
            palette: "emerald",
            center: [3.2, 3.4],
            zoom: 7.4,
            params: &[("color_density", 1.6), ("color_shift", 1.0)],
        },
        Preset {
            name: "lattice",
            palette: "orchid",
            center: [3.45, 3.45],
            zoom: 8.0,
            params: &[("iterations", 800.0), ("color_density", 2.0)],
        },
        Preset {
            name: "web",
            palette: "aurora",
            center: [3.3, 3.4],
            zoom: 7.4,
            params: &[("color_density", 1.6)],
        },
    ],
};

fn checked(sequence: &str) -> Result<(), String> {
    if sequence.is_empty() || sequence.len() > LONGEST {
        return Err(format!("it holds 1 to {LONGEST} letters"));
    }
    if sequence.bytes().any(|letter| letter != b'A' && letter != b'B') {
        return Err("it holds the letters A and B only".into());
    }
    Ok(())
}

// An orbit that has left for infinity, or is not a number any more.
#[inline]
fn lost(x: f64) -> bool {
    x.is_nan() || x.abs() >= 1e6
}

struct Lyapunov {
    second: Vec<bool>,
    warmup: u32,
    steps: u32,
    start: f64,
    density: f64,
    shift: f64,
    chaos: f64,
}

impl Lyapunov {
    fn new(fractal: &Fractal) -> Self {
        let values = SPEC.values(fractal);
        Self {
            second: SPEC.text(fractal, SEQUENCE).bytes().map(|letter| letter == b'B').collect(),
            steps: values[0] as u32,
            warmup: values[1] as u32,
            start: values[2],
            density: values[3],
            shift: values[4],
            chaos: values[5],
        }
    }

    // The mean of ln |r (1 - 2 x)| along the orbit; the logarithm is taken of the product of 8 steps at a time.
    #[inline]
    fn exponent(&self, a: f64, b: f64) -> Option<f64> {
        let rates = [a, b];
        let (mut x, mut at) = (self.start, 0usize);
        for _ in 0..self.warmup {
            let rate = rates[usize::from(self.second[at])];
            x = rate * x * (1.0 - x);
            at = if at + 1 == self.second.len() { 0 } else { at + 1 };
        }
        if lost(x) {
            return None;
        }
        let (mut sum, mut product) = (0.0f64, 1.0f64);
        for step in 0..self.steps {
            let rate = rates[usize::from(self.second[at])];
            product *= (rate * (1.0 - 2.0 * x)).abs();
            x = rate * x * (1.0 - x);
            at = if at + 1 == self.second.len() { 0 } else { at + 1 };
            if step & 7 == 7 {
                if lost(x) {
                    return None;
                }
                sum += ln(product);
                product = 1.0;
            }
        }
        if lost(x) {
            return None;
        }
        sum += ln(product);
        let exponent = sum / f64::from(self.steps);
        (!exponent.is_nan()).then_some(exponent.max(-30.0))
    }
}

impl Field for Lyapunov {
    #[inline]
    fn sample(&self, a: f64, b: f64) -> Option<Sample> {
        let exponent = self.exponent(a, b)?;
        if exponent < 0.0 {
            let order = -exponent;
            Some(Sample {
                at: self.shift + self.density * order / (order + 0.5),
                light: 1.0 - 0.8 * exp(-40.0 * order),
            })
        } else if self.chaos > 0.0 {
            let wild = exponent / (exponent + 0.35);
            Some(Sample {
                at: self.shift - self.density * 0.5 * wild,
                light: self.chaos * (0.2 + 0.8 * wild),
            })
        } else {
            None
        }
    }
}

fn random(seed: u64) -> Fractal {
    let mut fallback = None;
    for attempt in 0..ATTEMPTS {
        let mut rng = stream(NAME, seed, attempt);
        let length = 2 + rng.below(5) + rng.below(5);
        let mut sequence: Vec<u8> = (0..length).map(|_| if rng.next_bool() { b'A' } else { b'B' }).collect();
        if sequence.iter().all(|letter| *letter == sequence[0]) {
            sequence[0] = if sequence[0] == b'A' { b'B' } else { b'A' };
        }
        let sequence = String::from_utf8(sequence).unwrap_or_default();
        let zoom = (exp(rng.range(0.5, 2.8)) * 100.0).round() / 100.0;
        // The picture's longer side, up to 2.2 times the shorter, stays where both rates are under 4.
        let most = 4.0 - 4.4 / zoom;
        let camera = Camera {
            center: [rng.range(2.2, 3.9).min(most), rng.range(2.2, 3.9).min(most)].map(|part| (part * 1e3).round() / 1e3),
            zoom,
            ..Camera::default()
        };
        let colors = random_palette(&mut rng);
        let params = [
            ("color_density", (rng.range(0.8, 2.4) * 100.0).round() / 100.0),
            ("color_shift", (rng.range(0.0, 2.0) * 100.0).round() / 100.0),
        ];
        let mut fractal = SPEC.fractal(colors, camera, &params);
        fractal.text.insert(SEQUENCE.to_owned(), sequence);
        let field = Lyapunov {
            warmup: 60,
            steps: 240,
            ..Lyapunov::new(&fractal)
        };
        let frame = Frame::new(&fractal.camera, PROBE, PROBE);
        let exponents: Vec<Option<f64>> = (0..PROBE * PROBE)
            .map(|cell| {
                let (a, b) = frame.world((cell % PROBE) as f64 + 0.5, (cell / PROBE) as f64 + 0.5);
                field.exponent(a, b)
            })
            .collect();
        let share = |of: fn(&Option<f64>) -> bool| exponents.iter().filter(|e| of(e)).count() as f64 / exponents.len() as f64;
        let (clear, wild) = (share(Option::is_none), share(|e| e.is_some_and(|e| e >= 0.0)));
        let detail = busy(&exponents, PROBE, 0.08);
        if clear < 0.05 && (0.12..=0.7).contains(&wild) && (0.12..=0.6).contains(&detail) {
            return fractal;
        }
        fallback.get_or_insert(fractal);
    }
    fallback.unwrap_or_else(|| SPEC.fractal("gold", Camera::default(), &[]))
}

fn mutate(fractal: &Fractal, rng: &mut Rng, strength: f64) -> Fractal {
    let mut out = moved(fractal, rng, strength * 0.6);
    let start = (number(fractal, &START) + rng.range(-0.2, 0.2) * strength).clamp(START.min, START.max);
    out.params.insert(START.name.to_owned(), (start * 1e3).round() / 1e3);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fractal::{golden, preset};

    fn of(sequence: &str) -> Lyapunov {
        let mut fractal = SPEC.fractal("gold", Camera::default(), &[("iterations", 4000.0), ("warmup", 1000.0)]);
        fractal.text.insert(SEQUENCE.to_owned(), sequence.to_owned());
        Lyapunov::new(&fractal)
    }

    #[test]
    fn the_exponent_is_the_logistic_map_s() {
        // At r = 4 the exponent is ln 2, at a fixed point x* = 1 - 1/r it is ln |2 - r|.
        let one = of("A");
        let chaos = Lyapunov { start: 0.3, ..of("A") }.exponent(4.0, 0.0);
        assert!(chaos.is_some_and(|e| (e - core::f64::consts::LN_2).abs() < 0.02), "{chaos:?}");
        for rate in [1.5, 2.5, 2.9] {
            let fixed = one.exponent(rate, 0.0).unwrap();
            assert!((fixed - ln((2.0 - rate).abs())).abs() < 1e-6, "{rate}: {fixed}");
        }
        assert!(one.exponent(2.0, 0.0).unwrap() <= -30.0);
        assert!(one.exponent(4.5, 0.0).is_none());
        let both = of("AB");
        assert_eq!(both.exponent(3.2, 3.2), one.exponent(3.2, 0.0));
        assert!(both.exponent(3.9, 3.9).unwrap() > 0.0);
        assert!(both.exponent(2.5, 3.2).unwrap() < 0.0);
    }

    #[test]
    fn a_sequence_is_letters_a_and_b() {
        assert!(checked("AABAB").is_ok());
        for wrong in ["", "ABC", "ab", &"A".repeat(LONGEST + 1)] {
            assert!(checked(wrong).is_err(), "{wrong}");
        }
    }

    #[test]
    fn it_renders_to_its_golden_hash() {
        let hash = golden(&preset("lyapunov-ribbons").unwrap());
        assert_eq!(hash, 0x1c39_b7a0_9119_8afe, "got {hash:#018x}");
    }
}

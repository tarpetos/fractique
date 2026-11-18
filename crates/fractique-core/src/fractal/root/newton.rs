use super::{COLOR_DENSITY, COLOR_SHIFT, Complex, ITERATIONS, RELAXATION, SHADE, TONE, div, light, mul, norm, part};
use crate::fractal::escape::{Seen, descend, moved, nudged, value};
use crate::fractal::pixel::{self, PROBE};
use crate::fractal::{Draw, Family, Field, Param, Preset, Sample, Spec, random_palette, stream};
use crate::math::{atan2, cos, ln, ln_1p, sin};
use crate::rng::Rng;
use crate::scene::{Camera, Fractal};

const NAME: &str = "newton";
const ATTEMPTS: u64 = 32;
// An orbit has found a root when it is this near, squared.
const NEAR: f64 = 1e-6;

const POLYNOMIALS: [&[Complex]; 5] = [
    &[(1.0, 0.0), (0.0, 0.0), (-2.0, 0.0), (2.0, 0.0)],
    &[
        (1.0, 0.0),
        (0.0, 0.0),
        (0.0, 0.0),
        (0.0, 0.0),
        (15.0, 0.0),
        (0.0, 0.0),
        (0.0, 0.0),
        (0.0, 0.0),
        (-16.0, 0.0),
    ],
    &[(1.0, 0.0), (0.0, 0.0), (0.0, 0.0), (1.0, 0.0), (0.0, 0.0), (0.0, 0.0), (-1.0, 0.0)],
    &[(1.0, 0.0), (0.0, 0.0), (0.0, -3.0), (-5.0, -2.0), (3.0, 0.0), (1.0, 0.0)],
    &[(1.0, 0.0), (0.0, 0.0), (3.0, 0.0), (0.0, 0.0), (-4.0, 0.0)],
];

pub(super) static SPEC: Spec = Spec {
    name: NAME,
    family: Family::Root,
    params: &[
        Param::whole("polynomial", 0.0, 0.0, 5.0),
        Param::whole("degree", 3.0, 2.0, 12.0),
        RELAXATION,
        Param::real("twist", 0.0, -1.0, 1.0),
        ITERATIONS,
        COLOR_DENSITY,
        COLOR_SHIFT,
        SHADE,
    ],
    tone: TONE,
    draw: Draw::Pixel(|params, camera| Box::new(Newton::new(params, camera))),
    random,
    mutate,
    presets: &[
        Preset {
            name: "cubic",
            palette: "aurora",
            center: [0.0, 0.0],
            zoom: 1.3,
            params: &[("shade", 0.6)],
        },
        Preset {
            name: "seven",
            palette: "orchid",
            center: [0.0, 0.0],
            zoom: 1.5,
            params: &[("degree", 7.0), ("color_shift", 0.1), ("shade", 0.7)],
        },
        Preset {
            name: "cycle",
            palette: "ember",
            center: [0.0, 0.0],
            zoom: 0.85,
            params: &[("polynomial", 1.0), ("color_shift", 0.54), ("shade", 0.7)],
        },
        Preset {
            name: "octet",
            palette: "violet",
            center: [0.0, 0.0],
            zoom: 0.75,
            params: &[("polynomial", 2.0), ("color_shift", 0.05), ("shade", 0.6)],
        },
        Preset {
            name: "petals",
            palette: "gold",
            center: [-0.02778, -0.19444],
            zoom: 3.7,
            params: &[("degree", 5.0), ("relaxation", 1.35), ("twist", -0.26), ("iterations", 400.0), ("color_density", 1.4), ("color_shift", 0.15), ("shade", 0.7)],
        },
        Preset {
            name: "swirl",
            palette: "orchid",
            center: [-1.782078, 2.195477],
            zoom: 25.0,
            params: &[("polynomial", 4.0), ("relaxation", 0.8), ("twist", 0.68), ("iterations", 400.0), ("color_density", 1.6), ("color_shift", 0.17), ("shade", 0.68)],
        },
    ],
};

struct Newton {
    // From the highest power down; the highest is 1.
    polynomial: Vec<Complex>,
    roots: Vec<Complex>,
    relaxation: Complex,
    limit: u32,
    density: f64,
    shift: f64,
    shade: f64,
    // The steps after which a point is half as light at shade 1: more where the roots are neared slowly.
    half: f64,
}

fn polynomial(which: f64, degree: f64) -> Vec<Complex> {
    match which as usize {
        0 => {
            let mut all = vec![(0.0, 0.0); degree as usize + 1];
            (all[0], all[degree as usize]) = ((1.0, 0.0), (-1.0, 0.0));
            all
        }
        named => POLYNOMIALS[(named - 1).min(POLYNOMIALS.len() - 1)].to_vec(),
    }
}

#[inline]
fn valued(polynomial: &[Complex], z: Complex) -> (Complex, Complex) {
    let (mut value, mut slope) = (polynomial[0], (0.0, 0.0));
    for term in &polynomial[1..] {
        let (risen, carried) = (mul(slope, z), mul(value, z));
        slope = (risen.0 + value.0, risen.1 + value.1);
        value = (carried.0 + term.0, carried.1 + term.1);
    }
    (value, slope)
}

// The roots by the method of Durand and Kerner, each then refined by Newton's, in the order of their angles.
fn roots(polynomial: &[Complex]) -> Vec<Complex> {
    let mut roots = Vec::with_capacity(polynomial.len() - 1);
    let mut start = (1.0, 0.0);
    for _ in 1..polynomial.len() {
        roots.push(start);
        start = mul(start, (0.4, 0.9));
    }
    for _ in 0..400 {
        for at in 0..roots.len() {
            let mut apart = (1.0, 0.0);
            for (other, root) in roots.iter().enumerate() {
                if other != at {
                    apart = mul(apart, (roots[at].0 - root.0, roots[at].1 - root.1));
                }
            }
            let off = div(valued(polynomial, roots[at]).0, apart);
            roots[at] = (roots[at].0 - off.0, roots[at].1 - off.1);
        }
    }
    for root in &mut roots {
        for _ in 0..8 {
            let (value, slope) = valued(polynomial, *root);
            if norm(slope) > 0.0 {
                let off = div(value, slope);
                *root = (root.0 - off.0, root.1 - off.1);
            }
        }
    }
    let angle = |root: &Complex| {
        let angle = atan2(root.1, root.0);
        if angle < -1e-9 { angle + core::f64::consts::TAU } else { angle }
    };
    roots.sort_by(|a, b| angle(a).total_cmp(&angle(b)));
    roots
}

impl Newton {
    fn new(params: &[f64], camera: &Camera) -> Self {
        let polynomial = polynomial(params[0], params[1]);
        Self {
            roots: roots(&polynomial),
            polynomial,
            relaxation: (params[2], params[3]),
            limit: crate::fractal::escape::limit(params[4], camera.zoom),
            density: params[5],
            shift: params[6],
            shade: params[7],
            half: 6.0 * (-4.0 / ln(norm((1.0 - params[2], params[3])).max(1e-9))).max(1.0),
        }
    }

    #[inline]
    fn nearest(&self, z: Complex) -> (usize, f64) {
        let mut nearest = (0, f64::INFINITY);
        for (index, root) in self.roots.iter().enumerate() {
            let apart = norm((z.0 - root.0, z.1 - root.1));
            if apart < nearest.1 {
                nearest = (index, apart);
            }
        }
        nearest
    }
}

impl Field for Newton {
    #[inline]
    fn sample(&self, x: f64, y: f64) -> Option<Sample> {
        let squares = self.relaxation == (1.0, 0.0);
        let (mut z, mut last) = ((x, y), (f64::NAN, f64::NAN));
        for step in 0..self.limit {
            let (index, apart) = self.nearest(z);
            if apart < NEAR {
                let root = self.roots[index];
                let before = norm((last.0 - root.0, last.1 - root.1));
                let count = (f64::from(step) - 1.0 + part(before, apart, NEAR, squares)).max(0.0);
                let place = self.shift + index as f64 / self.roots.len() as f64;
                return Some(Sample {
                    at: place - place.floor() + 0.05 * self.density * ln_1p(count),
                    light: light(count, self.shade, self.half),
                });
            }
            let (value, slope) = valued(&self.polynomial, z);
            if norm(slope) == 0.0 {
                return None;
            }
            let off = mul(self.relaxation, div(value, slope));
            (last, z) = (z, (z.0 - off.0, z.1 - off.1));
        }
        None
    }
}

fn field(own: [f64; 4]) -> impl Fn(&[f64; 5], &Camera) -> Box<dyn Field> {
    move |_, camera| Box::new(Newton::new(&[own[0], own[1], own[2], own[3], 300.0, 0.0, 0.0, 0.5], camera))
}

fn random(seed: u64) -> Fractal {
    let mut fallback = None;
    for attempt in 0..ATTEMPTS {
        let mut rng = stream(NAME, seed, attempt);
        let (which, degree) = if rng.next_f64() < 0.45 {
            (0.0, 3.0 + rng.below(6) as f64)
        } else {
            (1.0 + rng.below(POLYNOMIALS.len() as u64) as f64, 3.0)
        };
        let (relaxation, twist) = if rng.next_f64() < 0.45 {
            (1.0, 0.0)
        } else {
            let (size, angle) = (rng.range(0.15, 0.75), rng.range(0.0, core::f64::consts::TAU));
            let round = |value: f64| (value * 100.0).round() / 100.0;
            (round(1.0 + size * cos(angle)), round(size * sin(angle)))
        };
        let own = [which, degree, relaxation, twist];
        let reach = roots(&polynomial(which, degree)).into_iter().map(norm).fold(0.0, f64::max).sqrt();
        let whole = Camera {
            zoom: (1.5 / reach * 100.0).round() / 100.0,
            ..Camera::default()
        };
        let steps = rng.below(4);
        let mut camera = descend(&mut rng, steps, &whole, &field(own));
        if rng.next_f64() < 0.4 {
            camera.rotation = rng.range(0.0, 360.0).round();
        }
        let colors = random_palette(&mut rng);
        let samples = pixel::probe(&*field(own)(&[0.0; 5], &camera), &camera);
        let lit = samples
            .iter()
            .filter(|sample| sample.as_ref().is_some_and(|sample| sample.light > 0.4))
            .count();
        let seen = Seen::of(&samples);
        let busy = seen.busy(PROBE, 0.1);
        let params = [
            ("polynomial", which),
            ("degree", degree),
            ("relaxation", relaxation),
            ("twist", twist),
            ("iterations", if relaxation == 1.0 && twist == 0.0 { 120.0 } else { 400.0 }),
            ("color_density", (rng.range(0.4, 2.2) * 100.0).round() / 100.0),
            ("color_shift", (rng.range(0.0, 1.0) * 100.0).round() / 100.0),
            ("shade", (rng.range(0.35, 0.8) * 100.0).round() / 100.0),
        ];
        let made = SPEC.fractal(colors, camera, &params);
        if seen.inside <= 0.25 && (0.08..=0.6).contains(&busy) && lit * 2 >= PROBE * PROBE {
            return made;
        }
        fallback.get_or_insert(made);
    }
    fallback.unwrap_or_else(|| SPEC.fractal("aurora", Camera::default(), &[]))
}

fn mutate(fractal: &Fractal, rng: &mut Rng, strength: f64) -> Fractal {
    let mut out = moved(fractal, rng, strength * 0.5);
    let shift = value(&SPEC, &out, "color_shift");
    out.params.insert("color_shift".to_owned(), shift - shift.floor());
    if value(&SPEC, fractal, "relaxation") != 1.0 || value(&SPEC, fractal, "twist") != 0.0 || rng.next_f64() < 0.5 {
        nudged(&SPEC, &mut out, "relaxation", rng, 0.15 * strength);
        nudged(&SPEC, &mut out, "twist", rng, 0.15 * strength);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fractal::{golden, preset};

    #[test]
    fn every_polynomial_has_its_roots() {
        for which in 0..=POLYNOMIALS.len() {
            for degree in if which == 0 { 2..=12 } else { 3..=3 } {
                let polynomial = polynomial(which as f64, f64::from(degree));
                let roots = roots(&polynomial);
                assert_eq!(roots.len(), polynomial.len() - 1);
                for (at, root) in roots.iter().enumerate() {
                    assert!(norm(valued(&polynomial, *root).0) < 1e-20, "{which} {degree} {root:?}");
                    for other in &roots[at + 1..] {
                        assert!(norm((root.0 - other.0, root.1 - other.1)) > 1e-4, "{which} {degree}");
                    }
                }
            }
        }
        let cubic = roots(&polynomial(0.0, 3.0));
        assert!((cubic[0].0 - 1.0).abs() < 1e-12 && cubic[0].1.abs() < 1e-12);
        assert!((cubic[1].0 + 0.5).abs() < 1e-12 && (cubic[1].1 - 0.75f64.sqrt()).abs() < 1e-12);
    }

    #[test]
    fn a_point_is_colored_by_the_root_it_finds() {
        let field = Newton::new(&[0.0, 3.0, 1.0, 0.0, 100.0, 0.0, 0.0, 0.0], &Camera::default());
        for (x, y, place) in [(2.0, 0.1, 0.0), (-1.0, 2.0, 1.0 / 3.0), (-1.0, -2.0, 2.0 / 3.0)] {
            let sample = field.sample(x, y).expect("the point finds a root");
            assert!((sample.at - place).abs() < 1e-12 && sample.light == 1.0);
        }
        // z³ - 2z + 2 sends 0 to 1 and 1 to 0 for ever.
        let cycle = Newton::new(&[1.0, 3.0, 1.0, 0.0, 500.0, 0.0, 0.0, 0.0], &Camera::default());
        assert!(cycle.sample(0.0, 0.0).is_none() && cycle.sample(0.01, 0.01).is_none());
    }

    #[test]
    fn it_renders_to_its_golden_hash() {
        let hash = golden(&preset("newton-cubic").unwrap());
        assert_eq!(hash, 0xc2a5_0f01_ca1b_e641, "got {hash:#018x}");
    }
}

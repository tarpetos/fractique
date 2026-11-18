use super::{COLOR_DENSITY, COLOR_SHIFT, Complex, Lines, Segment, VIEW, WIDTH, complex, folded, recolored};
use crate::fractal::density;
use crate::fractal::{Draw, Family, Param, Preset, Spec, random_palette, stream};
use crate::random::GRID;
use crate::rng::Rng;
use crate::scene::{Camera, Fractal, Tone};

const NAME: &str = "kleinian";
const ATTEMPTS: u64 = 60;
// The pieces a picture's shorter side is cut into, and how far the search goes.
const FINENESS: f64 = 1500.0;
const DEEPEST: usize = 1200;
const MOST_NODES: usize = 12_000_000;
const MOST_SEGMENTS: usize = 2_000_000;
// A piece outside the view is cut until it is this many times smaller than its distance to the view.
const REACH: f64 = 6.0;

const RECIPE: Param = Param::whole("recipe", 0.0, 0.0, 1.0);
const TA_REAL: Param = Param::real("ta_real", 1.91, 1.5, 3.0);
const TA_IMAGINARY: Param = Param::real("ta_imaginary", -0.05, -1.0, 1.0);
const TB_REAL: Param = Param::real("tb_real", 1.91, 1.5, 3.0);
const TB_IMAGINARY: Param = Param::real("tb_imaginary", -0.05, -1.0, 1.0);

pub(super) static SPEC: Spec = Spec {
    name: NAME,
    family: Family::Geometric,
    params: &[
        RECIPE,
        TA_REAL,
        TA_IMAGINARY,
        TB_REAL,
        TB_IMAGINARY,
        WIDTH,
        COLOR_DENSITY,
        COLOR_SHIFT,
    ],
    tone: Tone {
        brightness: 1.0,
        gamma: 2.6,
        vibrancy: 1.0,
        highlight: 0.15,
        glow_radius: 0.005,
        glow_strength: 0.45,
        smoothing: 0.6,
    },
    draw: Draw::Density(|params, camera| Box::new(Group::new(params).lines(params, camera, FINENESS, MOST_NODES).0)),
    random,
    mutate,
    presets: &[
        Preset {
            name: "pearls",
            palette: "ice",
            center: [0.0, 0.0],
            zoom: 0.72,
            params: &[("recipe", 0.0), ("ta_real", 1.91), ("ta_imaginary", -0.05), ("tb_real", 1.91), ("tb_imaginary", -0.05), ("color_density", 3.0)],
        },
        Preset {
            name: "gasket",
            palette: "gold",
            center: [0.0, 0.0],
            zoom: 1.5,
            params: &[("recipe", 0.0), ("ta_real", 2.0), ("ta_imaginary", 0.0), ("tb_real", 2.0), ("tb_imaginary", 0.0), ("color_density", 2.0)],
        },
        Preset {
            name: "spirals",
            palette: "orchid",
            center: [0.0, 0.0],
            zoom: 0.7,
            params: &[("recipe", 0.0), ("ta_real", 1.87), ("ta_imaginary", 0.1), ("tb_real", 1.87), ("tb_imaginary", -0.1), ("color_density", 3.0)],
        },
        Preset {
            name: "cusp",
            palette: "aurora",
            center: [0.0, 0.0],
            zoom: 1.55,
            params: &[("recipe", 0.0), ("ta_real", 1.958591), ("ta_imaginary", -0.011279), ("tb_real", 2.0), ("tb_imaginary", 0.0), ("color_density", 4.0)],
        },
        Preset {
            name: "whirls",
            palette: "sunset",
            center: [0.0, 0.0],
            zoom: 1.55,
            params: &[("recipe", 0.0), ("ta_real", 1.6421), ("ta_imaginary", -0.7666), ("tb_real", 2.0), ("tb_imaginary", 0.0), ("color_density", 4.0)],
        },
        Preset {
            name: "strip",
            palette: "violet",
            center: [0.0, 1.0],
            zoom: 1.2,
            params: &[("recipe", 1.0), ("ta_real", 2.0), ("ta_imaginary", 0.0), ("tb_real", 2.0), ("tb_imaginary", 0.0), ("color_density", 6.0)],
        },
        Preset {
            name: "lace",
            palette: "emerald",
            center: [0.0, 0.84],
            zoom: 1.4,
            params: &[("recipe", 1.0), ("ta_real", 1.68), ("ta_imaginary", -0.6), ("tb_real", 2.0), ("tb_imaginary", 0.0), ("color_density", 6.0)],
        },
    ],
};

#[derive(Clone, Copy, Debug)]
struct Moebius {
    a: Complex,
    b: Complex,
    c: Complex,
    d: Complex,
}

impl Moebius {
    #[inline]
    fn then(&self, next: &Moebius) -> Moebius {
        Moebius {
            a: self.a * next.a + self.b * next.c,
            b: self.a * next.b + self.b * next.d,
            c: self.c * next.a + self.d * next.c,
            d: self.c * next.b + self.d * next.d,
        }
    }

    fn inverse(&self) -> Moebius {
        Moebius {
            a: self.d,
            b: -self.b,
            c: -self.c,
            d: self.a,
        }
    }

    // A point that is not finite stands for the point at infinity.
    #[inline]
    fn at(&self, z: Complex) -> Complex {
        if z.is_finite() {
            (self.a * z + self.b) / (self.c * z + self.d)
        } else {
            self.a / self.c
        }
    }

    fn trace(&self) -> Complex {
        self.a + self.d
    }

    // The fixed point the map pulls toward itself.
    fn sink(&self) -> Complex {
        let infinity = complex(f64::INFINITY, 0.0);
        if self.c.norm2() < 1e-24 {
            let apart = self.d - self.a;
            return if self.a.norm2() >= self.d.norm2() || apart.norm2() < 1e-24 {
                infinity
            } else {
                self.b / apart
            };
        }
        let trace = self.trace();
        let root = (trace * trace - complex(4.0, 0.0)).sqrt();
        let twice = self.c * 2.0;
        let (one, other) = ((self.a - self.d + root) / twice, (self.a - self.d - root) / twice);
        if (self.c * one + self.d).norm2() >= (self.c * other + self.d).norm2() {
            one
        } else {
            other
        }
    }
}

// The generators a, b and their inverses A, B, in the order a, b, A, B around the limit set.
struct Group {
    letters: [Moebius; 4],
    // The limit points of five ways on from each letter, in the order they lie along the curve.
    marks: [[Complex; 5]; 4],
}

impl Group {
    fn new(params: &[f64]) -> Self {
        let (ta, tb) = (complex(params[1], params[2]), complex(params[3], params[4]));
        let (a, b) = if params[0] < 0.5 { grandma(ta, tb) } else { maskit(ta) };
        let letters = [a, b, a.inverse(), b.inverse()];
        let marks = [0usize, 1, 2, 3].map(|i| {
            let letter = |by: usize| &letters[(i + by) % 4];
            [
                letter(1).then(letter(2)).then(letter(3)).then(letter(0)).sink(),
                letter(1).then(letter(0)).sink(),
                letter(0).sink(),
                letter(3).then(letter(0)).sink(),
                letter(3).then(letter(2)).then(letter(1)).then(letter(0)).sink(),
            ]
        });
        Self { letters, marks }
    }

    // The limit set as a chain of segments, by a search of the tree of words that goes down a branch until
    // the piece of the curve under it is small. The flag tells whether the search came to its end.
    fn lines(&self, params: &[f64], camera: &Camera, fineness: f64, most_nodes: usize) -> (Lines, bool) {
        let (density, shift) = (params[6], params[7]);
        let fine = 4.0 / camera.zoom / fineness;
        let seen = VIEW / camera.zoom;
        let middle = complex(camera.center[0], camera.center[1]);
        let mut segments = Vec::new();
        let mut pending: Vec<(Moebius, usize, usize, f64, f64)> = [1usize, 2, 3, 0]
            .iter()
            .enumerate()
            .map(|(order, letter)| (self.letters[*letter], *letter, 1, (3 - order) as f64 * 0.25, 0.25))
            .collect();
        let (mut nodes, mut whole) = (0usize, true);
        while let Some((word, last, level, from, span)) = pending.pop() {
            nodes += 1;
            if nodes > most_nodes || segments.len() > MOST_SEGMENTS {
                whole = false;
                break;
            }
            let points = self.marks[last].map(|mark| word.at(mark));
            let finite = points.iter().all(|point| point.is_finite());
            let (mut longest, mut nearest, mut before) = (0.0f64, f64::INFINITY, None::<Complex>);
            for point in points.iter().filter(|point| point.is_finite()) {
                nearest = nearest.min((*point - middle).norm2());
                if let Some(before) = before {
                    longest = longest.max((*point - before).norm2());
                }
                before = Some(*point);
            }
            let (longest, nearest) = (longest.sqrt(), nearest.sqrt());
            if nearest.is_nan() || nearest == f64::INFINITY {
                continue;
            }
            let outside = (nearest - 1.2 * seen).max(0.0);
            let small = longest < fine + outside / REACH;
            if small && (finite || outside > 0.0) || level >= DEEPEST {
                if finite && nearest < 1.25 * seen {
                    let color = |part: usize| folded(shift + density * (from + span * part as f64 * 0.25)) as f32;
                    for part in 0..4 {
                        segments.push(Segment {
                            from: (points[part].re, points[part].im),
                            to: (points[part + 1].re, points[part + 1].im),
                            colors: (color(part), color(part + 1)),
                        });
                    }
                }
                whole &= small;
                continue;
            }
            for (order, turn) in [3usize, 0, 1].into_iter().enumerate() {
                let letter = (last + turn) % 4;
                pending.push((
                    word.then(&self.letters[letter]),
                    letter,
                    level + 1,
                    from + span * (2 - order) as f64 / 3.0,
                    span / 3.0,
                ));
            }
        }
        (Lines::new(segments, params[5], camera), whole)
    }
}

// "Grandma's recipe" of Indra's Pearls: the generators with traces ta and tb whose commutator has trace -2.
fn grandma(ta: Complex, tb: Complex) -> (Moebius, Moebius) {
    let (two, four, i) = (complex(2.0, 0.0), complex(4.0, 0.0), complex(0.0, 1.0));
    let product = ta * tb;
    let tab = (product - (product * product - (ta * ta + tb * tb) * 4.0).sqrt()) * 0.5;
    let z0 = (tab - two) * tb / (tb * tab - ta * 2.0 + i * tab * 2.0);
    let across = ta * tab - tb * 2.0;
    let a = Moebius {
        a: ta * 0.5,
        b: (across + i * 4.0) / ((tab * 2.0 + four) * z0),
        c: (across - i * 4.0) * z0 / (tab * 2.0 - four),
        d: ta * 0.5,
    };
    let b = Moebius {
        a: (tb - i * 2.0) * 0.5,
        b: tb * 0.5,
        c: tb * 0.5,
        d: (tb + i * 2.0) * 0.5,
    };
    (a, b)
}

// Maskit's recipe: b is the shift by 2, and a has the trace ta = -i μ.
fn maskit(ta: Complex) -> (Moebius, Moebius) {
    let i = complex(0.0, 1.0);
    let a = Moebius {
        a: ta,
        b: -i,
        c: -i,
        d: Complex::ZERO,
    };
    let b = Moebius {
        a: Complex::ONE,
        b: complex(2.0, 0.0),
        c: Complex::ZERO,
        d: Complex::ONE,
    };
    (a, b)
}

fn rounded(value: f64) -> f64 {
    (value * 1e4).round() / 1e4
}

fn whole(params: &[f64], camera: &Camera, fineness: f64, most_nodes: usize) -> Option<Lines> {
    let (lines, whole) = Group::new(params).lines(params, camera, fineness, most_nodes);
    (whole && lines.len() >= 500).then_some(lines)
}

// The camera that frames the limit set, when the group is one worth a picture: the search ends, and the curve
// is not a blot. Maskit's limit set is a band without end, so its view is set by the band's height.
fn framed(params: &[f64]) -> Option<Camera> {
    if params[0] >= 0.5 {
        let camera = Camera {
            center: [0.0, rounded(0.5 * params[1])],
            zoom: (240.0 / params[1]).round() / 100.0,
            ..Camera::default()
        };
        return whole(params, &camera, FINENESS, 2_500_000).map(|_| camera);
    }
    let wide = Camera {
        zoom: 0.8,
        ..Camera::default()
    };
    let probe = density::probe(&whole(params, &wide, FINENESS, 2_500_000)?)?;
    let covered = probe.cells.iter().filter(|hits| **hits > 0).count();
    let good = covered * 100 >= GRID * GRID * 8 && covered * 100 <= GRID * GRID * 55;
    good.then(|| Camera {
        center: probe.camera.center.map(rounded),
        zoom: (probe.camera.zoom * 100.0).round() / 100.0,
        ..probe.camera
    })
}

fn with(colors: &str, camera: Camera, values: &[f64], rest: &[(&'static str, f64)]) -> Fractal {
    let names = ["recipe", "ta_real", "ta_imaginary", "tb_real", "tb_imaginary"];
    let params: Vec<(&str, f64)> = names.into_iter().zip(values.iter().copied()).chain(rest.iter().copied()).collect();
    SPEC.fractal(colors, camera, &params)
}

// The traces a share of the way from a group that is surely discrete to one that is surely not.
fn between(from: &[f64; 4], to: &[f64; 4], share: f64, maskit: bool) -> [f64; 8] {
    let at = |index: usize| rounded(from[index] + (to[index] - from[index]) * share);
    [f64::from(u8::from(maskit)), at(0), at(1), at(2), at(3), 1.0, 1.0, 0.0]
}

fn random(seed: u64) -> Fractal {
    for attempt in 0..ATTEMPTS {
        let mut rng = stream(NAME, seed, attempt);
        let maskit = rng.next_f64() < 0.2;
        let lean = rng.range(-0.6, 0.6);
        let (from, to) = match (maskit, rng.below(4)) {
            (true, _) => ([3.0, lean, 2.0, 0.0], [1.5, lean, 2.0, 0.0]),
            (_, 0) => ([3.0, 0.0, 3.0, 0.0], [1.6, lean, 1.6, lean]),
            (_, 1) => ([3.0, 0.0, 3.0, 0.0], [1.6, lean, 1.6, -lean]),
            (_, 2) => ([3.0, 0.0, 2.0, 0.0], [1.5, lean, 2.0, 0.0]),
            _ => {
                let other = [rng.range(1.9, 3.0), rng.range(-0.3, 0.3)];
                ([3.0, 0.0, other[0], other[1]], [1.5, lean, other[0], other[1]])
            }
        };
        // The pictures worth keeping lie where the groups stop being discrete: the search for a limit set
        // stops ending there, and the way is halved toward that edge.
        let coarse = Camera {
            center: [0.0, if maskit { 1.0 } else { 0.0 }],
            zoom: if maskit { 1.0 } else { 0.8 },
            ..Camera::default()
        };
        let ends = |share: f64| whole(&between(&from, &to, share, maskit), &coarse, 400.0, 200_000).is_some();
        if !ends(0.0) {
            continue;
        }
        let (mut near, mut far) = (0.0, 1.0);
        for _ in 0..8 {
            let middle = (near + far) * 0.5;
            if ends(middle) {
                near = middle;
            } else {
                far = middle;
            }
        }
        let values = between(&from, &to, near * rng.range(0.88, 1.0), maskit);
        if let Some(camera) = framed(&values) {
            let rest = [
                ("color_density", (rng.range(1.0, 6.0) * 100.0).round() / 100.0),
                ("color_shift", (rng.range(0.0, 2.0) * 100.0).round() / 100.0),
            ];
            return with(random_palette(&mut rng), camera, &values[..5], &rest);
        }
    }
    SPEC.fractal("ice", Camera::default(), &[])
}

fn mutate(fractal: &Fractal, rng: &mut Rng, strength: f64) -> Fractal {
    let before = SPEC.values(fractal);
    let mut out = fractal.clone();
    // Across the cut of the square root in Grandma's recipe a small step gives another picture altogether:
    // a variant is kept only when its limit set is framed much as the one it came from.
    let home = framed(&before);
    let near = |camera: &Camera| {
        home.as_ref().is_none_or(|home| {
            let (dx, dy) = (camera.center[0] - home.center[0], camera.center[1] - home.center[1]);
            (0.75..=1.33).contains(&(camera.zoom / home.zoom)) && (dx * dx + dy * dy).sqrt() * home.zoom <= 1.0
        })
    };
    for _ in 0..12 {
        let mut values = before.clone();
        for (at, param) in [(1, &TA_REAL), (2, &TA_IMAGINARY), (3, &TB_REAL), (4, &TB_IMAGINARY)] {
            values[at] = rounded((before[at] + rng.range(-0.12, 0.12) * strength).clamp(param.min, param.max));
        }
        if let Some(camera) = framed(&values).filter(near) {
            for (at, param) in [(1, &TA_REAL), (2, &TA_IMAGINARY), (3, &TB_REAL), (4, &TB_IMAGINARY)] {
                out.params.insert(param.name.to_owned(), values[at]);
            }
            out.camera = Camera {
                rotation: fractal.camera.rotation,
                flip_y: fractal.camera.flip_y,
                ..camera
            };
            break;
        }
    }
    recolored(&mut out, rng, strength);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fractal::{golden, preset};

    fn near(a: Complex, b: Complex) -> bool {
        (a - b).norm2() < 1e-18
    }

    #[test]
    fn the_recipes_give_the_traces_asked_and_a_parabolic_commutator() {
        let minus_two = complex(-2.0, 0.0);
        for (ta, tb) in [
            (complex(1.91, 0.05), complex(1.91, 0.05)),
            (complex(2.0, 0.0), complex(2.0, 0.0)),
            (complex(1.87, 0.1), complex(1.87, -0.1)),
            (complex(2.2, -0.3), complex(3.0, 0.0)),
        ] {
            let (a, b) = grandma(ta, tb);
            for map in [&a, &b] {
                assert!(near(map.a * map.d - map.b * map.c, Complex::ONE));
            }
            assert!(near(a.trace(), ta) && near(b.trace(), tb));
            let commutator = a.then(&b).then(&a.inverse()).then(&b.inverse());
            assert!(near(commutator.trace(), minus_two), "{:?}", commutator.trace());
            let tab = a.then(&b).trace();
            assert!(near(ta * ta + tb * tb + tab * tab, ta * tb * tab));

            let (a, b) = maskit(ta);
            assert!(near(a.a * a.d - a.b * a.c, Complex::ONE) && near(a.trace(), ta));
            let commutator = a.then(&b).then(&a.inverse()).then(&b.inverse());
            assert!(near(commutator.trace(), minus_two));
        }
    }

    #[test]
    fn a_sink_is_a_fixed_point_that_attracts() {
        let (a, b) = grandma(complex(1.91, 0.05), complex(1.91, 0.05));
        for map in [a, b, a.then(&b), a.inverse().then(&b)] {
            let sink = map.sink();
            assert!((map.at(sink) - sink).norm2() < 1e-20);
            let pull = Complex::ONE / ((map.c * sink + map.d) * (map.c * sink + map.d));
            assert!(pull.norm2() <= 1.0 + 1e-9);
        }
        let (_, shift) = maskit(complex(2.0, 0.0));
        assert!(!shift.sink().is_finite());
        assert!(near(shift.at(complex(0.5, 1.0)), complex(2.5, 1.0)));
    }

    // For real traces above 2 the group is Fuchsian: its limit set is one circle.
    #[test]
    fn a_fuchsian_group_draws_a_circle() {
        let params = [0.0, 3.0, 0.0, 3.0, 0.0, 1.0, 1.0, 0.0];
        let (lines, whole) = Group::new(&params).lines(&params, &Camera::default(), FINENESS, MOST_NODES);
        assert!(whole && lines.len() > 1000);
        let points: Vec<(f64, f64)> = lines.segments.iter().map(|segment| segment.from).collect();
        let (first, second, third) = (points[0], points[points.len() / 3], points[2 * points.len() / 3]);
        let round = |p: (f64, f64)| complex(p.0, p.1);
        let (a, b, c) = (round(first), round(second), round(third));
        let d = 2.0 * (a.re * (b.im - c.im) + b.re * (c.im - a.im) + c.re * (a.im - b.im));
        let middle = complex(
            (a.norm2() * (b.im - c.im) + b.norm2() * (c.im - a.im) + c.norm2() * (a.im - b.im)) / d,
            (a.norm2() * (c.re - b.re) + b.norm2() * (a.re - c.re) + c.norm2() * (b.re - a.re)) / d,
        );
        let radius = (a - middle).norm2().sqrt();
        for point in points {
            assert!(((round(point) - middle).norm2().sqrt() - radius).abs() < 1e-6 * radius);
        }
        for pair in lines.segments.windows(2) {
            let gap = (pair[0].to.0 - pair[1].from.0).abs() + (pair[0].to.1 - pair[1].from.1).abs();
            assert!(gap < 1e-6, "the curve breaks by {gap}");
        }
    }

    #[test]
    fn it_renders_to_its_golden_hash() {
        let hash = golden(&preset("kleinian-pearls").unwrap());
        assert_eq!(hash, 0x6cf2_7f9e_9825_1828, "got {hash:#018x}");
    }
}

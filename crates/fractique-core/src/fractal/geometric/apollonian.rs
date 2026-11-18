use super::{COLOR_DENSITY, COLOR_SHIFT, Complex, busy, complex, moved, number};
use crate::fractal::pixel::{self, PROBE};
use crate::fractal::{Draw, Family, Field, Param, Preset, Sample, Spec, random_palette, stream};
use crate::math::{ln, ln_1p};
use crate::rng::Rng;
use crate::scene::{Camera, Fractal, Tone};

const NAME: &str = "apollonian";
const ATTEMPTS: u64 = 40;

const RADIUS: Param = Param::real("radius", 0.5, 0.05, 0.95);
const BALANCE: Param = Param::real("balance", 1.0, 0.05, 1.0);
const ITERATIONS: Param = Param::whole("iterations", 400.0, 1.0, 100_000.0).log();
const STYLE: Param = Param::whole("style", 0.0, 0.0, 2.0);
const COLORING: Param = Param::whole("coloring", 0.0, 0.0, 2.0);
const RELIEF: Param = Param::real("relief", 0.5, 0.0, 1.0);
const EDGE: Param = Param::real("edge", 0.35, 0.0, 1.0);

pub(super) static SPEC: Spec = Spec {
    name: NAME,
    family: Family::Geometric,
    params: &[
        RADIUS,
        BALANCE,
        ITERATIONS,
        STYLE,
        COLORING,
        COLOR_DENSITY,
        COLOR_SHIFT,
        RELIEF,
        EDGE,
    ],
    tone: Tone {
        brightness: 1.0,
        gamma: 2.5,
        vibrancy: 1.0,
        highlight: 0.0,
        glow_radius: 0.004,
        glow_strength: 0.1,
        smoothing: 1.0,
    },
    draw: Draw::Pixel(|params, camera| Box::new(Gasket::new(params, camera))),
    random,
    mutate,
    presets: &[
        Preset {
            name: "classic",
            palette: "ice",
            center: [0.0, 0.0],
            zoom: 1.9,
            params: &[("coloring", 1.0), ("color_density", 1.2)],
        },
        Preset {
            name: "threefold",
            palette: "sunset",
            center: [0.0, 0.0],
            zoom: 1.9,
            params: &[("radius", 0.4641), ("balance", 0.866), ("relief", 1.0), ("color_density", 1.4)],
        },
        Preset {
            name: "rings",
            palette: "gold",
            center: [0.0, 0.0],
            zoom: 1.9,
            params: &[("radius", 0.6667), ("balance", 1.0), ("style", 1.0), ("coloring", 1.0), ("color_density", 1.5), ("edge", 0.45)],
        },
        Preset {
            name: "glow",
            palette: "violet",
            center: [0.0, 0.0],
            zoom: 1.9,
            params: &[("radius", 0.6), ("balance", 0.9375), ("style", 2.0), ("color_density", 1.2)],
        },
        Preset {
            name: "families",
            palette: "orchid",
            center: [0.0, 0.0],
            zoom: 1.9,
            params: &[("radius", 0.5455), ("balance", 0.9429), ("coloring", 2.0), ("relief", 1.0), ("color_density", 1.6)],
        },
        Preset {
            name: "gap",
            palette: "emerald",
            center: [0.0, 0.0],
            zoom: 9.0,
            params: &[("radius", 0.4641), ("balance", 0.866), ("coloring", 1.0), ("relief", 0.3), ("color_density", 1.3)],
        },
    ],
};

// The circle or line a |p|² - 2 b·p + c = 0 with b·b - a c = 1: a is the curvature, and the side where the
// left side is negative is its inside.
#[derive(Clone, Copy, Debug)]
struct Round {
    a: f64,
    bx: f64,
    by: f64,
    c: f64,
}

impl Round {
    fn circle(center: Complex, radius: f64, outward: bool) -> Self {
        let a = if outward { -1.0 / radius } else { 1.0 / radius };
        Self {
            a,
            bx: a * center.re,
            by: a * center.im,
            c: a * (center.norm2() - radius * radius),
        }
    }

    fn through(points: [Complex; 3]) -> Self {
        let det = |m: [[f64; 3]; 3]| {
            m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1]) - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
                + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0])
        };
        let rows = |pick: fn(Complex) -> [f64; 3]| points.map(pick);
        let a = det(rows(|p| [p.re, p.im, 1.0]));
        let bx = 0.5 * det(rows(|p| [p.norm2(), p.im, 1.0]));
        let by = -0.5 * det(rows(|p| [p.norm2(), p.re, 1.0]));
        let c = -det(rows(|p| [p.norm2(), p.re, p.im]));
        let size = (bx * bx + by * by - a * c).max(f64::MIN_POSITIVE).sqrt();
        Self {
            a: a / size,
            bx: bx / size,
            by: by / size,
            c: c / size,
        }
    }

    fn flipped(self) -> Self {
        Self {
            a: -self.a,
            bx: -self.bx,
            by: -self.by,
            c: -self.c,
        }
    }

    #[inline]
    fn at(&self, x: f64, y: f64) -> f64 {
        self.a * (x * x + y * y) - 2.0 * (self.bx * x + self.by * y) + self.c
    }

    // The point's mirror image in the circle, and how much the inversion magnifies there.
    #[inline]
    fn mirror(&self, x: f64, y: f64) -> (f64, f64, f64) {
        let (gx, gy) = (2.0 * (self.a * x - self.bx), 2.0 * (self.a * y - self.by));
        let size = gx * gx + gy * gy;
        let step = 2.0 * self.at(x, y) / size;
        (x - step * gx, y - step * gy, 4.0 / size)
    }
}

struct Gasket {
    circles: [Round; 4],
    mirrors: [Round; 4],
    limit: u32,
    style: u32,
    coloring: u32,
    density: f64,
    shift: f64,
    relief: f64,
    rim: f64,
    per_side: f64,
}

struct Found {
    circle: usize,
    depth: u32,
    // The curvature of the circle the point lies in, and (R² - d²) / 2R for the point at d from its middle.
    curvature: f64,
    height: f64,
}

// The Descartes configuration inside the unit circle: a circle of `radius` touching it on the left, one of
// `balance` times the room that is left touching both, and the circle that touches all three.
fn configuration(radius: f64, balance: f64) -> [(Complex, f64); 3] {
    let second = balance * (1.0 - radius);
    let first_at = complex(radius - 1.0, 0.0);
    let x = ((radius + second) * (radius + second) - (1.0 - second) * (1.0 - second) - (1.0 - radius) * (1.0 - radius))
        / (2.0 * (1.0 - radius));
    // The height of the triangle of the three middles, by Heron: exactly 0 where they lie on one line.
    let second_at = complex(x, 2.0 * (radius * second * (1.0 - radius) * (1.0 - balance)).sqrt() / (1.0 - radius));
    let (k1, k2) = (1.0 / radius, 1.0 / second);
    // k1 k2 - k1 - k2, written so that it is exactly 0 where the three middles lie on one line.
    let k3 = k1 + k2 - 1.0 + 2.0 * ((1.0 - radius) * (1.0 - balance) * k1 * k2).sqrt();
    let third = 1.0 / k3;
    let sum = first_at * k1 + second_at * k2;
    let root = (first_at * second_at * (k1 * k2)).sqrt() * 2.0;
    let miss = |at: Complex| {
        ((at - first_at).norm2().sqrt() - radius - third).abs()
            + ((at - second_at).norm2().sqrt() - second - third).abs()
            + (at.norm2().sqrt() + third - 1.0).abs()
    };
    let (one, other) = ((sum + root) * third, (sum - root) * third);
    let third_at = if miss(one) <= miss(other) { one } else { other };
    [(first_at, radius), (second_at, second), (third_at, third)]
}

impl Gasket {
    fn new(params: &[f64], camera: &Camera) -> Self {
        let inner = configuration(params[0], params[1]);
        let circles = [
            Round::circle(Complex::ZERO, 1.0, true),
            Round::circle(inner[0].0, inner[0].1, false),
            Round::circle(inner[1].0, inner[1].1, false),
            Round::circle(inner[2].0, inner[2].1, false),
        ];
        let touch = |i: usize, j: usize| {
            let (low, high) = (i.min(j), i.max(j));
            if low == 0 {
                let (at, _) = inner[high - 1];
                at * (1.0 / at.norm2().sqrt())
            } else {
                let ((from, r), (to, s)) = (inner[low - 1], inner[high - 1]);
                from + (to - from) * (r / (r + s))
            }
        };
        let mirrors = [0usize, 1, 2, 3].map(|i| {
            let [j, k, l] = [(i + 1) % 4, (i + 2) % 4, (i + 3) % 4];
            let mirror = Round::through([touch(j, k), touch(k, l), touch(l, j)]);
            let own = touch(i, j);
            if mirror.at(own.re, own.im) < 0.0 { mirror.flipped() } else { mirror }
        });
        Self {
            circles,
            mirrors,
            limit: params[2] as u32,
            style: params[3] as u32,
            coloring: params[4] as u32,
            density: params[5],
            shift: params[6],
            relief: params[7],
            rim: 0.012 * params[8] * params[8],
            per_side: camera.zoom / 4.0,
        }
    }

    // The point is carried out of the gap it lies in, one inversion at a time, until it is inside a circle of
    // the configuration: the circle it began in is the image of that one under the same inversions.
    #[inline]
    fn find(&self, x: f64, y: f64) -> Option<Found> {
        let (mut x, mut y, mut scale) = (x, y, 1.0f64);
        let mut row = [1.0f64, 0.0, 0.0, 0.0];
        for depth in 0..=self.limit {
            for (circle, round) in self.circles.iter().enumerate() {
                let value = round.at(x, y);
                if value < 0.0 {
                    if depth == 0 && circle == 0 {
                        return None;
                    }
                    let curvature = row[0] * round.a + row[1] * round.bx + row[2] * round.by + row[3] * round.c;
                    return Some(Found {
                        circle,
                        depth,
                        curvature: curvature.abs(),
                        height: -value / (2.0 * scale),
                    });
                }
            }
            let mirror = self.mirrors.iter().find(|mirror| mirror.at(x, y) < 0.0)?;
            let by;
            (x, y, by) = mirror.mirror(x, y);
            scale *= by;
            let image = row[0] * mirror.a + row[1] * mirror.bx + row[2] * mirror.by + row[3] * mirror.c;
            row[0] += image * mirror.c;
            row[1] -= 2.0 * image * mirror.bx;
            row[2] -= 2.0 * image * mirror.by;
            row[3] += image * mirror.a;
        }
        None
    }
}

impl Field for Gasket {
    #[inline]
    fn sample(&self, x: f64, y: f64) -> Option<Sample> {
        let found = self.find(x, y)?;
        let place = match self.coloring {
            0 => 0.5 * ln_1p(f64::from(found.depth)),
            1 => 0.25 * ln(found.curvature.max(1.0)),
            _ => 0.3 * found.circle as f64 + 0.05 * ln(found.curvature.max(1.0)),
        };
        let at = self.shift + self.density * place;
        let side = found.height * self.per_side;
        let light = match self.style {
            0 => {
                let rise = (2.0 * found.height * found.curvature).clamp(0.0, 1.0).sqrt();
                let lit = 1.0 - self.relief * (1.0 - rise);
                if side < self.rim { lit * (side / self.rim).sqrt() } else { lit }
            }
            style => {
                let width = if style == 1 { self.rim } else { 4.0 * self.rim };
                let near = side / width.max(1e-9);
                let light = if style == 1 { 1.0 / (1.0 + near * near) } else { 1.0 / (1.0 + near) };
                if light < 0.004 {
                    return None;
                }
                light
            }
        };
        Some(Sample { at, light })
    }
}

// The logarithm of the curvature of the circle under every cell of a probe of the view.
fn seen(gasket: &Gasket, camera: &Camera) -> Vec<Option<f64>> {
    let frame = pixel::Frame::new(camera, PROBE, PROBE);
    (0..PROBE * PROBE)
        .map(|cell| {
            let (x, y) = frame.world((cell % PROBE) as f64 + 0.5, (cell / PROBE) as f64 + 0.5);
            gasket.find(x, y).map(|found| ln(found.curvature.max(1e-9)))
        })
        .collect()
}

// A walk into the gaps: each step moves to a cell where several circles meet and magnifies.
fn descend(gasket: &Gasket, mut camera: Camera, rng: &mut Rng, steps: u64) -> Camera {
    for _ in 0..steps {
        let seen = seen(gasket, &camera);
        let differ = |a: Option<f64>, b: Option<f64>| match (a, b) {
            (Some(a), Some(b)) => (a - b).abs() > 1e-6,
            (None, None) => false,
            _ => true,
        };
        let cells: Vec<(usize, usize, f64)> = (6..PROBE - 6)
            .flat_map(|y| (6..PROBE - 6).map(move |x| (x, y)))
            .map(|(x, y)| {
                let own = seen[y * PROBE + x];
                let others = (y - 1..=y + 1)
                    .flat_map(|ny| (x - 1..=x + 1).map(move |nx| ny * PROBE + nx))
                    .filter(|at| differ(seen[*at], own))
                    .count();
                (x, y, if others >= 4 { others as f64 } else { 0.0 })
            })
            .filter(|(_, _, worth)| *worth > 0.0)
            .collect();
        let total: f64 = cells.iter().map(|(_, _, worth)| worth).sum();
        if cells.is_empty() {
            break;
        }
        let mut roll = rng.next_f64() * total;
        let (x, y, _) = *cells
            .iter()
            .find(|(_, _, worth)| {
                roll -= worth;
                roll < 0.0
            })
            .unwrap_or(&cells[0]);
        let place = |cell: usize| (cell as f64 + 0.5) / PROBE as f64 - 0.5;
        camera = camera.looking_at(place(x), place(y), rng.range(2.0, 3.5));
    }
    camera
}

fn random(seed: u64) -> Fractal {
    let mut fallback = None;
    // Half of the seeds look into the gaps: the choice is the seed's, so that the views that fail there do not
    // leave the whole gasket to most of them.
    let close = stream(NAME, seed, ATTEMPTS).next_bool();
    for attempt in 0..ATTEMPTS {
        let mut rng = stream(NAME, seed, attempt);
        let radius = (rng.range(0.25, 0.75) * 1e3).round() / 1e3;
        let balance = if rng.next_f64() < 0.3 {
            1.0
        } else {
            (rng.range(0.35, 1.0) * 1e3).round() / 1e3
        };
        let params = [
            ("radius", radius),
            ("balance", balance),
            ("style", rng.below(3) as f64),
            ("coloring", rng.below(3) as f64),
            ("color_density", (rng.range(0.6, 2.2) * 100.0).round() / 100.0),
            ("color_shift", (rng.range(0.0, 2.0) * 100.0).round() / 100.0),
            ("relief", (rng.range(0.0, 1.0) * 100.0).round() / 100.0),
            ("edge", (rng.range(0.3, 0.6) * 100.0).round() / 100.0),
        ];
        let whole = Camera {
            zoom: 1.9,
            rotation: (rng.below(12) * 30) as f64,
            ..Camera::default()
        };
        let mut fractal = SPEC.fractal(random_palette(&mut rng), whole, &params);
        let gasket = Gasket::new(&SPEC.values(&fractal), &fractal.camera);
        if close {
            let steps = 1 + rng.below(3);
            let mut camera = descend(&gasket, fractal.camera.clone(), &mut rng, steps);
            camera.center = camera.center.map(|part| (part * 1e6).round() / 1e6);
            camera.zoom = (camera.zoom * 100.0).round() / 100.0;
            fractal.camera = camera;
        }
        let seen = seen(&gasket, &fractal.camera);
        let clear = seen.iter().filter(|cell| cell.is_none()).count();
        let mut sizes: Vec<f64> = seen.iter().flatten().copied().collect();
        sizes.sort_by(f64::total_cmp);
        let largest = sizes
            .chunk_by(|a, b| (a - b).abs() < 1e-6)
            .map(<[f64]>::len)
            .max()
            .unwrap_or(0);
        let cells = seen.len();
        if clear * 100 <= cells * 40 && largest * 100 <= cells * 55 && busy(&seen, PROBE, 1e-6) >= 0.08 {
            return fractal;
        }
        fallback.get_or_insert(fractal);
    }
    fallback.unwrap_or_else(|| SPEC.fractal("ice", Camera::default(), &[]))
}

fn mutate(fractal: &Fractal, rng: &mut Rng, strength: f64) -> Fractal {
    let mut out = moved(fractal, rng, strength * 0.3);
    for param in [&RADIUS, &BALANCE] {
        let value = (number(fractal, param) + rng.range(-0.15, 0.15) * strength).clamp(param.min, param.max);
        out.params.insert(param.name.to_owned(), (value * 1e3).round() / 1e3);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fractal::{golden, preset};

    fn gasket(radius: f64, balance: f64) -> Gasket {
        Gasket::new(&[radius, balance, 400.0, 0.0, 0.0, 1.0, 0.0, 0.5, 0.35], &Camera::default())
    }

    #[test]
    fn the_four_circles_touch_and_obey_descartes() {
        for (radius, balance) in [(0.5, 1.0), (0.4641, 0.866), (0.6, 0.9375), (0.3, 0.4), (2.0 / 3.0, 1.0)] {
            let [first, second, third] = configuration(radius, balance);
            let apart = |a: (Complex, f64), b: (Complex, f64)| (a.0 - b.0).norm2().sqrt() - a.1 - b.1;
            for (a, b) in [(first, second), (second, third), (third, first)] {
                assert!(apart(a, b).abs() < 1e-9, "{radius} {balance}");
            }
            let curvatures = [-1.0, 1.0 / first.1, 1.0 / second.1, 1.0 / third.1];
            for (at, radius) in [first, second, third] {
                assert!((at.norm2().sqrt() + radius - 1.0).abs() < 1e-9);
            }
            let sum: f64 = curvatures.iter().sum();
            let squares: f64 = curvatures.iter().map(|k| k * k).sum();
            assert!((sum * sum - 2.0 * squares).abs() < 1e-6 * squares, "{radius} {balance}");
        }
    }

    #[test]
    fn the_circles_of_the_classic_gasket_have_whole_curvatures() {
        let gasket = gasket(0.5, 1.0);
        // The gasket (-1, 2, 2, 3) has circles of curvature 3, 15, 35 and 63 down its vertical axis.
        for (y, curvature) in [(2.0 / 3.0, 3.0), (4.0 / 15.0, 15.0), (6.0 / 35.0, 35.0), (8.0 / 63.0, 63.0)] {
            let found = gasket.find(0.0, y).expect("the point is in a circle");
            assert!((found.curvature - curvature).abs() < 1e-6, "{y}: {}", found.curvature);
            assert!((found.height - 0.5 / curvature).abs() < 1e-9, "{y}: {}", found.height);
        }
        let found = gasket.find(0.12, 0.33).unwrap();
        let nearest = found.curvature.round();
        assert!((found.curvature - nearest).abs() < 1e-6 && nearest > 15.0, "{}", found.curvature);
        assert!(gasket.find(1.2, 0.0).is_none());
        assert_eq!(gasket.find(-0.5, 0.1).map(|found| (found.depth, found.circle)), Some((0, 1)));
    }

    #[test]
    fn it_renders_to_its_golden_hash() {
        let hash = golden(&preset("apollonian-classic").unwrap());
        assert_eq!(hash, 0x63d1_cf66_f28f_112f, "got {hash:#018x}");
    }
}

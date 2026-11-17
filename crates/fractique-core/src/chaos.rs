use crate::math::{TAU, cos, sin};
use crate::rng::Rng;
use crate::scene::{Flame, Transform};
use crate::variation::Kind;

const FUSE: u32 = 24;
const ESCAPED: f64 = 1e10;

struct Xf {
    cumulative: f64,
    affine: [f64; 6],
    post: Option<[f64; 6]>,
    color: f64,
    speed: f64,
    opacity: f64,
    variations: Vec<(Kind, f64, [f64; 2])>,
}

impl Xf {
    fn new(t: &Transform, cumulative: f64) -> Self {
        Self {
            cumulative,
            affine: t.affine,
            post: t.post,
            color: t.color,
            speed: t.color_speed,
            opacity: t.opacity,
            variations: t
                .variations
                .iter()
                .map(|v| (v.kind, v.weight, v.kind.resolve(&v.params)))
                .collect(),
        }
    }

    #[inline]
    fn apply(&self, x: f64, y: f64, rng: &mut Rng) -> (f64, f64) {
        let [a, b, c, d, e, f] = self.affine;
        let (ax, ay) = (a * x + b * y + c, d * x + e * y + f);
        let (mut ox, mut oy) = (0.0, 0.0);
        for (kind, weight, params) in &self.variations {
            let (vx, vy) = kind.eval(ax, ay, params, rng);
            ox += weight * vx;
            oy += weight * vy;
        }
        match self.post {
            Some([a, b, c, d, e, f]) => (a * ox + b * oy + c, d * ox + e * oy + f),
            None => (ox, oy),
        }
    }
}

pub(crate) struct Compiled {
    xfs: Vec<Xf>,
    total: f64,
    fin: Option<Xf>,
    rotations: Vec<(f64, f64)>,
    mirror: bool,
}

impl Compiled {
    pub(crate) fn new(flame: &Flame) -> Self {
        let mut total = 0.0;
        let xfs = flame
            .transforms
            .iter()
            .map(|t| {
                total += t.weight;
                Xf::new(t, total)
            })
            .collect();
        let copies = flame.symmetry.unsigned_abs().max(1);
        let rotations = (0..copies)
            .map(|k| {
                let angle = TAU * f64::from(k) / f64::from(copies);
                (cos(angle), sin(angle))
            })
            .collect();
        Self {
            xfs,
            total,
            fin: flame.final_transform.as_ref().map(|t| Xf::new(t, 0.0)),
            rotations,
            mirror: flame.symmetry < 0,
        }
    }
}

pub(crate) struct Walker {
    x: f64,
    y: f64,
    color: f64,
}

impl Walker {
    pub(crate) fn new(flame: &Compiled, rng: &mut Rng) -> Self {
        let mut walker = Self {
            x: rng.range(-1.0, 1.0),
            y: rng.range(-1.0, 1.0),
            color: rng.next_f64(),
        };
        for _ in 0..FUSE {
            walker.step(flame, rng);
        }
        walker
    }

    #[inline]
    fn step<'a>(&mut self, flame: &'a Compiled, rng: &mut Rng) -> &'a Xf {
        let pick = rng.next_f64() * flame.total;
        let xf = flame
            .xfs
            .iter()
            .find(|xf| pick < xf.cumulative)
            .unwrap_or(&flame.xfs[flame.xfs.len() - 1]);
        let (x, y) = xf.apply(self.x, self.y, rng);
        self.color += (xf.color - self.color) * xf.speed;
        if x.is_finite() && y.is_finite() && x.abs() < ESCAPED && y.abs() < ESCAPED {
            self.x = x;
            self.y = y;
        } else {
            self.x = rng.range(-1.0, 1.0);
            self.y = rng.range(-1.0, 1.0);
        }
        xf
    }

    pub(crate) fn run(&mut self, flame: &Compiled, rng: &mut Rng, count: u64, mut plot: impl FnMut(f64, f64, f64)) {
        for _ in 0..count {
            let xf = self.step(flame, rng);
            if xf.opacity < 1.0 && rng.next_f64() >= xf.opacity {
                continue;
            }
            let (mut x, mut y, mut color) = (self.x, self.y, self.color);
            if let Some(fin) = &flame.fin {
                (x, y) = fin.apply(x, y, rng);
                color += (fin.color - color) * fin.speed;
            }
            if flame.rotations.len() > 1 {
                let (c, s) = flame.rotations[rng.below(flame.rotations.len() as u64) as usize];
                (x, y) = (x * c - y * s, x * s + y * c);
            }
            if flame.mirror && rng.next_bool() {
                x = -x;
            }
            plot(x, y, color);
        }
    }
}

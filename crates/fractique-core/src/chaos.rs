use crate::math::{TAU, cos, sin};
use crate::rng::Rng;
use crate::scene::{Flame, Transform};
use crate::variation::{Kind, MAX_PARAMS};

const FUSE: u32 = 24;
const ESCAPED: f64 = 1e10;

struct Xf {
    cumulative: f64,
    affine: [f64; 6],
    post: Option<[f64; 6]>,
    color: f64,
    speed: f64,
    opacity: f64,
    variations: Vec<(Kind, f64, [f64; MAX_PARAMS])>,
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
            let (vx, vy) = kind.eval(ax, ay, params, *weight, &self.affine, rng);
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

const SEEDED: u64 = 0x5EED;

impl Compiled {
    #[inline]
    fn chosen(&self, rng: &mut Rng) -> &Xf {
        let pick = rng.next_f64() * self.total;
        self.xfs
            .iter()
            .find(|xf| pick < xf.cumulative)
            .unwrap_or(&self.xfs[self.xfs.len() - 1])
    }

    // One step of the chaos game whose every random choice comes from `seed`: the same seed moves a point the same
    // way again.
    #[inline]
    pub(crate) fn seeded(&self, seed: u64, (x, y, color): (f64, f64, f64)) -> Option<(f64, f64, f64)> {
        let mut rng = Rng::derive(seed, SEEDED, 0);
        let xf = self.chosen(&mut rng);
        let (x, y) = xf.apply(x, y, &mut rng);
        (x.is_finite() && y.is_finite() && x.abs() < ESCAPED && y.abs() < ESCAPED).then_some((
            x,
            y,
            color + (xf.color - color) * xf.speed,
        ))
    }

    // What `Walker::run` does to a point before it plots it, with the random choices from `seed`; `last` is the
    // seed of the step that made the point. `None`: the step's transform hides the point this time.
    #[inline]
    pub(crate) fn finished(
        &self,
        seed: u64,
        last: u64,
        (mut x, mut y, mut color): (f64, f64, f64),
    ) -> Option<(f64, f64, f64)> {
        let mut rng = Rng::derive(seed, SEEDED, 1);
        let opacity = self.chosen(&mut Rng::derive(last, SEEDED, 0)).opacity;
        if opacity < 1.0 && rng.next_f64() >= opacity {
            return None;
        }
        if let Some(fin) = &self.fin {
            (x, y) = fin.apply(x, y, &mut rng);
            color += (fin.color - color) * fin.speed;
        }
        if self.rotations.len() > 1 {
            let (c, s) = self.rotations[rng.below(self.rotations.len() as u64) as usize];
            (x, y) = (x * c - y * s, x * s + y * c);
        }
        if self.mirror && rng.next_bool() {
            x = -x;
        }
        (x.is_finite() && y.is_finite()).then_some((x, y, color))
    }

    pub(crate) fn plotted(&self) -> f64 {
        let mut below = 0.0;
        let mut sum = 0.0;
        for xf in &self.xfs {
            sum += (xf.cumulative - below) * xf.opacity.clamp(0.0, 1.0);
            below = xf.cumulative;
        }
        sum / self.total
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

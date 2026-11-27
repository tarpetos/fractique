use super::{Orbit, Param, Prepared, Spec, Walk, random_palette, stream};
use crate::math::atan2;
use crate::random::{GRID, PROBE_POINTS};
use crate::rng::Rng;
use crate::scene::{Camera, Fractal};

super::family! {
    buddhabrot,
    clifford,
    de_jong,
    flow,
    gumowski_mira,
    hopalong,
    ikeda,
    lorenz,
}

const COLOR_BY: Param = Param::whole("color_by", 0.0, 0.0, 2.0);

const ATTEMPTS: u64 = 40;
const GAUGE_DOMAIN: u64 = 0x7100;
const PROBE_DOMAIN: u64 = 0x7101;
const GAUGE_STEPS: u64 = 4096;
const LARGEST_GAUGE: u64 = 500_000;
const LARGEST_STRIDE: u64 = 64;
const PROBED_SPANS: u64 = 4;
// An attractor may be one of several that the starting point chooses between: the measures take a few.
const STARTS: u64 = 8;
// The states `Prepared` gives of an orbit that every walk follows: one of this many steps, of the first steps only.
const TRACK_EVERY: u64 = 64;
const TRACKED_STEPS: u64 = 1 << 20;

trait System: Sync {
    type State: Copy;
    const BURN: u32;
    // Whether the picture is the orbit moved to the middle and scaled to a size near 1, whatever its own size.
    const FITTED: bool;
    // One of this many steps is taken to measure the orbit by, so that the measure sees enough of a slow one.
    const STRIDE: u64 = 1;
    // Whether a point is plotted somewhere between two steps, which draws the lines of a flow whole.
    const BLEND: bool = false;
    // Whether every walk starts at the same point and so follows the one orbit.
    const SHARED: bool = false;

    fn start(&self, rng: &mut Rng) -> Self::State;
    fn step(&self, state: Self::State) -> Self::State;
    fn point(&self, state: Self::State) -> (f64, f64);

    fn pace(&self, from: Self::State, to: Self::State) -> f64 {
        let ((x, y), (to_x, to_y)) = (self.point(from), self.point(to));
        ((to_x - x) * (to_x - x) + (to_y - y) * (to_y - y)).sqrt()
    }

    // What the third coloring goes by, when it is not the distance from the middle of the picture.
    fn height(&self, _: Self::State) -> Option<f64> {
        None
    }

    // The steps after which an orbit ends and another starts; 0 for an orbit that never ends.
    fn span(&self) -> u64 {
        0
    }

    // What `Prepared` adds for this system: the numbers its step and its point use besides the parameters.
    fn numbers(&self) -> Vec<f64> {
        Vec::new()
    }

    fn coordinates(&self, state: Self::State) -> [f64; 3] {
        let (x, y) = self.point(state);
        [x, y, 0.0]
    }
}

struct Traced<S> {
    system: S,
    color: u8,
    paces: Ranks,
    heights: Ranks,
    center: (f64, f64),
    unit: f64,
}

// Where a value stands among those of a sample, from 0 for the lowest to 1 for the highest: read from a table
// of even steps between the two, since a search for every point of a render costs more than the point.
struct Ranks {
    low: f64,
    per_step: f64,
    steps: Vec<f64>,
}

impl Ranks {
    const KNOTS: usize = 64;
    const STEPS: usize = 4096;

    fn none() -> Self {
        Self {
            low: 0.0,
            per_step: 0.0,
            steps: vec![0.0; 2],
        }
    }

    fn of(mut values: Vec<f64>) -> Self {
        values.sort_by(f64::total_cmp);
        let knots: Vec<f64> = (0..=Self::KNOTS)
            .map(|knot| values[(values.len() - 1) * knot / Self::KNOTS])
            .collect();
        let (low, high) = (knots[0], knots[Self::KNOTS]);
        if high <= low {
            return Self::none();
        }
        let among = |value: f64| {
            let above = knots.partition_point(|knot| *knot < value).clamp(1, Self::KNOTS);
            let (under, over) = (knots[above - 1], knots[above]);
            let share = if over > under {
                (value - under) / (over - under)
            } else {
                0.0
            };
            ((above - 1) as f64 + share.clamp(0.0, 1.0)) / Self::KNOTS as f64
        };
        Self {
            low,
            per_step: Self::STEPS as f64 / (high - low),
            steps: (0..=Self::STEPS)
                .map(|step| among(low + (high - low) * step as f64 / Self::STEPS as f64))
                .collect(),
        }
    }

    #[inline]
    fn rank(&self, value: f64) -> f64 {
        // `max` turns a value that is not a number into 0.
        let place = ((value - self.low) * self.per_step)
            .max(0.0)
            .min((self.steps.len() - 2) as f64 + 0.999_999);
        let step = place as usize;
        let (under, over) = (self.steps[step], self.steps[step + 1]);
        under + (over - under) * (place - step as f64)
    }
}

impl<S: System> Traced<S> {
    fn new(system: S, color: f64) -> Self {
        let mut traced = Self {
            system,
            color: color as u8,
            paces: Ranks::none(),
            heights: Ranks::none(),
            center: (0.0, 0.0),
            unit: 1.0,
        };
        traced.gauge();
        traced
    }

    fn fresh(&self, rng: &mut Rng) -> S::State {
        let mut state = self.system.start(rng);
        for _ in 0..S::BURN {
            state = self.system.step(state);
        }
        state
    }

    // The pace, the middle and the heights of a short orbit: what the colors are measured against.
    fn gauge(&mut self) {
        let mut rng = Rng::derive(0, GAUGE_DOMAIN, 0);
        let span = self.system.span();
        let stride = (span.min(LARGEST_GAUGE) / GAUGE_STEPS).max(S::STRIDE);
        let (mut paces, mut xs, mut ys, mut states) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
        for _ in 0..STARTS {
            let mut state = self.fresh(&mut rng);
            for step in 0..GAUGE_STEPS / STARTS * stride {
                let next = self.system.step(state);
                let (x, y) = self.system.point(next);
                if !(x.abs() < 1e9 && y.abs() < 1e9) {
                    break;
                }
                if step % stride == 0 {
                    paces.push(self.system.pace(state, next));
                    xs.push(x);
                    ys.push(y);
                    states.push(next);
                }
                state = next;
            }
        }
        if paces.len() < 64 {
            return;
        }
        let part = |values: &mut Vec<f64>, share: usize| {
            values.sort_by(f64::total_cmp);
            values[(values.len() - 1) * share / 100]
        };
        let (left, right, bottom, top) = (part(&mut xs, 2), part(&mut xs, 98), part(&mut ys, 2), part(&mut ys, 98));
        self.center = ((left + right) * 0.5, (bottom + top) * 0.5);
        if S::FITTED {
            self.unit = ((right - left).max(top - bottom) * 0.5).max(1e-12);
        }
        self.paces = Ranks::of(paces);
        self.heights = Ranks::of(states.iter().map(|state| self.height(*state)).collect());
    }

    fn height(&self, state: S::State) -> f64 {
        self.system.height(state).unwrap_or_else(|| {
            let (x, y) = self.system.point(state);
            let (dx, dy) = (x - self.center.0, y - self.center.1);
            (dx * dx + dy * dy).sqrt()
        })
    }

    #[inline]
    fn place(&self, from: S::State, to: S::State, at: (f64, f64)) -> f64 {
        match self.color {
            0 => self.paces.rank(self.system.pace(from, to)),
            1 => {
                let (x, y) = self.system.point(from);
                atan2(at.1 - y, at.0 - x).abs() * core::f64::consts::FRAC_1_PI
            }
            _ => self.heights.rank(self.height(to)),
        }
    }
}

struct Path<'a, S: System> {
    traced: &'a Traced<S>,
    state: Option<S::State>,
    age: u64,
}

impl<S: System> Orbit for Traced<S> {
    fn walk(&self, _: &mut Rng) -> Box<dyn Walk + '_> {
        Box::new(Path {
            traced: self,
            state: None,
            age: 0,
        })
    }

    // Numbers: the coloring, the middle (x, y), the unit, the span, the steps before the first point, 1 when the
    // picture is fitted, 1 when points are blended, the lowest pace and the steps of the table a unit of pace,
    // the same two for the heights, the steps between two tracked states, then the system's own.
    // Tables: the ranks of the paces, those of the heights, the tracked states (x, y, z each).
    fn prepared(&self) -> Prepared {
        let mut track = Vec::new();
        if S::SHARED {
            let mut state = self.fresh(&mut Rng::derive(0, GAUGE_DOMAIN, 0));
            for step in 0..self.system.span().min(TRACKED_STEPS) {
                if step % TRACK_EVERY == 0 {
                    track.extend(self.system.coordinates(state));
                }
                state = self.system.step(state);
            }
        }
        let flag = |on: bool| if on { 1.0 } else { 0.0 };
        let mut numbers = vec![
            f64::from(self.color),
            self.center.0,
            self.center.1,
            self.unit,
            self.system.span() as f64,
            f64::from(S::BURN),
            flag(S::FITTED),
            flag(S::BLEND),
            self.paces.low,
            self.paces.per_step,
            self.heights.low,
            self.heights.per_step,
            if S::SHARED { TRACK_EVERY as f64 } else { 0.0 },
        ];
        numbers.extend(self.system.numbers());
        Prepared {
            numbers,
            tables: vec![self.paces.steps.clone(), self.heights.steps.clone(), track],
        }
    }
}

impl<S: System> Walk for Path<'_, S> {
    fn run(&mut self, rng: &mut Rng, count: u64, plot: &mut dyn FnMut(f64, f64, f64)) {
        let traced = self.traced;
        let span = traced.system.span();
        for _ in 0..count {
            let state = match self.state {
                Some(state) if span == 0 || self.age < span => state,
                _ => {
                    self.age = 0;
                    traced.fresh(rng)
                }
            };
            let next = traced.system.step(state);
            let mut at = traced.system.point(next);
            let place = traced.place(state, next, at);
            if S::BLEND {
                let (from, share) = (traced.system.point(state), rng.next_f64());
                at = (from.0 + (at.0 - from.0) * share, from.1 + (at.1 - from.1) * share);
            }
            if S::FITTED {
                plot(
                    (at.0 - traced.center.0) / traced.unit,
                    (at.1 - traced.center.1) / traced.unit,
                    place,
                );
            } else {
                plot(at.0, at.1, place);
            }
            self.age += 1;
            self.state = (at.0.abs() < 1e9 && at.1.abs() < 1e9).then_some(next);
        }
    }
}

// What a randomizer asks of the 48 × 48 grid of an orbit's points, and how much of them the frame leaves out on
// each side.
struct Taste {
    trimmed_thousandths: usize,
    covered_hundredths: usize,
    cells_holding_half: usize,
    contrast_hundredths: usize,
}

// The camera that frames one of every `stride` points the orbit plots, and whether they are worth a picture:
// not a few points, a curve, a dot in a dust or an even fog.
fn judged(orbit: &dyn Orbit, stride: u64, taste: &Taste) -> Option<(Camera, bool)> {
    let mut rng = Rng::derive(0, PROBE_DOMAIN, 0);
    let mut points = Vec::with_capacity(PROBE_POINTS as usize);
    for _ in 0..STARTS {
        let mut turn = 0;
        orbit
            .walk(&mut rng)
            .run(&mut rng, PROBE_POINTS / STARTS * stride, &mut |x, y, _| {
                if turn % stride == 0 && x.is_finite() && y.is_finite() {
                    points.push((x, y));
                }
                turn += 1;
            });
    }
    if points.len() < 1000 {
        return None;
    }

    let span = |pick: fn(&(f64, f64)) -> f64| {
        let mut values: Vec<f64> = points.iter().map(pick).collect();
        values.sort_by(f64::total_cmp);
        let cut = values.len() * taste.trimmed_thousandths / 1000;
        (values[cut], values[values.len() - 1 - cut])
    };
    let ((left, right), (bottom, top)) = (span(|point| point.0), span(|point| point.1));
    let half = (right - left).max(top - bottom) * 0.5 * 1.2;
    if !(half > 1e-6 && half < 1e6) {
        return None;
    }
    let center = [(left + right) * 0.5, (bottom + top) * 0.5];

    let mut cells = vec![0u32; GRID * GRID];
    let mut inside = 0usize;
    for (x, y) in &points {
        let gx = ((x - center[0]) / half * 0.5 + 0.5) * GRID as f64;
        let gy = ((y - center[1]) / half * 0.5 + 0.5) * GRID as f64;
        if gx >= 0.0 && gx < GRID as f64 && gy >= 0.0 && gy < GRID as f64 {
            cells[gy as usize * GRID + gx as usize] += 1;
            inside += 1;
        }
    }
    let covered = cells.iter().filter(|hits| **hits > 0).count();
    let mut sorted = cells.clone();
    sorted.sort_unstable_by(|a, b| b.cmp(a));
    let mut sum = 0;
    let holding_half = sorted
        .iter()
        .take_while(|hits| {
            let short = sum * 2 < inside;
            sum += **hits as usize;
            short
        })
        .count();
    let step = |a: usize, b: usize| cells[a].abs_diff(cells[b]) as usize;
    let mut contrast = 0;
    for y in 0..GRID {
        for x in 0..GRID {
            let at = y * GRID + x;
            if x + 1 < GRID {
                contrast += step(at, at + 1);
            }
            if y + 1 < GRID {
                contrast += step(at, at + GRID);
            }
        }
    }
    let good = covered * 100 >= GRID * GRID * taste.covered_hundredths
        && holding_half >= taste.cells_holding_half
        && contrast * 100 >= inside * taste.contrast_hundredths;
    let camera = Camera {
        center,
        zoom: 2.0 / half,
        ..Camera::default()
    };
    Some((camera, good))
}

type Frame = fn(&[f64]) -> Option<(Camera, bool)>;

fn framed<S: System>(system: S, taste: &Taste) -> Option<(Camera, bool)> {
    let stride = (system.span() * PROBED_SPANS / PROBE_POINTS).clamp(S::STRIDE, LARGEST_STRIDE.max(S::STRIDE));
    judged(&Traced::new(system, 0.0), stride, taste)
}

fn with(spec: &Spec, colors: &str, camera: Camera, values: &[f64]) -> Fractal {
    let params: Vec<_> = spec
        .params
        .iter()
        .map(|param| param.name)
        .zip(values.iter().copied())
        .collect();
    spec.fractal(colors, camera, &params)
}

fn rounded(spec: &Spec, values: Vec<f64>) -> Vec<f64> {
    values
        .into_iter()
        .zip(spec.params)
        .map(|(value, param)| {
            let value = if param.whole {
                value.round()
            } else {
                (value * 1e3).round() / 1e3
            };
            value.clamp(param.min, param.max)
        })
        .collect()
}

fn random_by(spec: &Spec, seed: u64, draw: fn(&mut Rng) -> Vec<f64>, frame: Frame) -> Fractal {
    let mut fallback = None;
    for attempt in 0..ATTEMPTS {
        let mut rng = stream(spec.name, seed, attempt);
        let values = rounded(spec, draw(&mut rng));
        let colors = random_palette(&mut rng);
        match frame(&values) {
            Some((camera, true)) => return with(spec, colors, camera, &values),
            Some((camera, false)) => {
                fallback.get_or_insert_with(|| with(spec, colors, camera, &values));
            }
            None => {}
        }
    }
    fallback.unwrap_or_else(|| {
        let first = &spec.presets[0];
        let camera = Camera {
            center: first.center,
            zoom: first.zoom,
            ..Camera::default()
        };
        spec.fractal(first.palette, camera, first.params)
    })
}

// `reach` is how far a full-strength mutation moves each parameter; 0 leaves one as it is.
fn mutate_by(spec: &Spec, fractal: &Fractal, rng: &mut Rng, strength: f64, reach: &[f64], frame: Frame) -> Fractal {
    let before = spec.values(fractal);
    let mut fallback = None;
    let mut chosen = None;
    for _ in 0..12 {
        let moved = before
            .iter()
            .zip(reach)
            .map(|(value, reach)| value + rng.range(-1.0, 1.0) * reach * strength);
        let values = rounded(spec, moved.collect());
        match frame(&values) {
            Some((camera, true)) => {
                chosen = Some((Some(camera), values));
                break;
            }
            Some((camera, false)) => {
                fallback.get_or_insert((Some(camera), values));
            }
            None => {
                fallback.get_or_insert((None, values));
            }
        }
    }
    let mut out = fractal.clone();
    if let Some((camera, values)) = chosen.or(fallback) {
        if let Some(camera) = camera {
            out.camera = Camera {
                rotation: fractal.camera.rotation,
                flip_y: fractal.camera.flip_y,
                ..camera
            };
        }
        out.params = spec
            .params
            .iter()
            .map(|param| param.name.to_owned())
            .zip(values)
            .collect();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{KINDS, Ranks};
    use crate::fractal::Draw;
    use crate::rng::Rng;
    use crate::scene::Camera;

    #[test]
    fn a_walk_plots_as_many_points_as_it_is_asked_for() {
        for spec in KINDS {
            let Draw::Density(make) = spec.draw else {
                panic!("{} is not drawn as a density", spec.name);
            };
            let values: Vec<f64> = spec.params.iter().map(|param| param.default).collect();
            let orbit = make(&values, &Camera::default());
            let mut rng = Rng::derive(5, 0, 0);
            let mut walk = orbit.walk(&mut rng);
            for count in [1, 1000, 77] {
                let mut plotted = 0;
                walk.run(&mut rng, count, &mut |_, _, place| {
                    assert!((0.0..=1.0).contains(&place), "{} {place}", spec.name);
                    plotted += 1;
                });
                assert_eq!(plotted, count, "{}", spec.name);
            }
        }
    }

    #[test]
    fn what_a_kind_prepared_is_whole() {
        for spec in KINDS {
            let Draw::Density(make) = spec.draw else {
                panic!("{} is not drawn as a density", spec.name);
            };
            let values: Vec<f64> = spec.params.iter().map(|param| param.default).collect();
            let prepared = make(&values, &Camera::default()).prepared();
            assert_eq!(prepared, make(&values, &Camera::default()).prepared(), "{}", spec.name);
            assert!(
                prepared.numbers.iter().all(|number| number.is_finite()),
                "{}",
                spec.name
            );
            match spec.name {
                "clifford" => assert_eq!(prepared, super::Prepared::default()),
                "buddhabrot" => assert_eq!(prepared, super::Prepared::default()),
                name => {
                    assert_eq!(prepared.tables[0].len(), Ranks::STEPS + 1, "{name}");
                    assert_eq!(prepared.tables[1].len(), Ranks::STEPS + 1, "{name}");
                    let shared = name == "lorenz" || name == "flow";
                    assert_eq!(prepared.tables[2].is_empty(), !shared, "{name}");
                    assert_eq!(prepared.numbers[12] == 0.0, !shared, "{name}");
                }
            }
        }
        let Draw::Density(make) = KINDS.iter().find(|spec| spec.name == "lorenz").unwrap().draw else {
            unreachable!()
        };
        let track = make(&[10.0, 28.0, 8.0 / 3.0, 0.0, 0.0, 5.0, 0.0], &Camera::default())
            .prepared()
            .tables[2]
            .clone();
        assert_eq!(track.len(), 1000usize.div_ceil(64) * 3);
    }

    #[test]
    fn a_rank_runs_from_the_lowest_value_to_the_highest() {
        let ranks = Ranks::of((0..=640).map(f64::from).collect());
        assert_eq!(ranks.rank(-1.0), 0.0);
        assert_eq!(ranks.rank(0.0), 0.0);
        assert_eq!(ranks.rank(160.0), 0.25);
        assert_eq!(ranks.rank(165.0), 0.2578125);
        assert!((ranks.rank(640.0) - 1.0).abs() < 1e-6);
        assert!((ranks.rank(700.0) - 1.0).abs() < 1e-6);
        let skewed = Ranks::of((0..1000).map(|at| f64::from(at * at)).collect());
        assert!((skewed.rank(250_000.0) - 0.5).abs() < 0.01);
        assert_eq!(Ranks::of(vec![3.0; 100]).rank(3.0), 0.0);
        assert_eq!(Ranks::none().rank(f64::NAN), 0.0);
    }
}

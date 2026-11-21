use crate::chaos::Compiled;
use crate::math::{cos, ln, pow, sin};
use crate::render::Exposure;
use crate::rng::Rng;
use crate::scene::Camera;

// Below this share of the plotted points in view the plain chaos game wastes most of its work.
const IN_VIEW: f64 = 0.125;
const PILOT: usize = 1 << 16;
pub(crate) const PARTICLES: usize = 8192;
// How many steps of the chaos game a particle remembers: enough for the oldest to move its point less than a pixel.
const MEMORY: usize = 128;
const WIDTH: usize = MEMORY + 1;
const SWEEPS: usize = 6;
const FUSE: usize = 24;
// A move that is accepted less often than this is not worth trying; one that moves a point less than this share of
// the view is not worth its cost.
const RARE: f64 = 0.05;
const FINE: f64 = 1.0 / 512.0;
const AHEAD: usize = 8;
const FAR_EVERY: u64 = 4;

// The particles are moved in groups, each with its own random numbers, so that the threads cannot change the result.
const GROUP: usize = 128;

type Point = (f64, f64, f64);
type Group<'a> = (&'a mut [u64], &'a mut [Point]);

struct Tally {
    tried: [u32; WIDTH],
    accepted: [u32; WIDTH],
    travelled: [f64; WIDTH],
}

impl Default for Tally {
    fn default() -> Self {
        Self {
            tried: [0; WIDTH],
            accepted: [0; WIDTH],
            travelled: [0.0; WIDTH],
        }
    }
}

#[derive(Clone)]
pub(crate) struct Window {
    center: [f64; 2],
    cos: f64,
    sin: f64,
    half: [f64; 2],
    zoom: f64,
}

impl Window {
    fn new(camera: &Camera, wide: f64, high: f64, zoom: f64) -> Self {
        let angle = camera.rotation.to_radians();
        let shorter = wide.min(high);
        Self {
            center: camera.center,
            cos: cos(angle),
            sin: sin(angle),
            half: [2.0 * wide / shorter, 2.0 * high / shorter],
            zoom,
        }
    }

    #[inline]
    fn holds(&self, (x, y, _): Point) -> bool {
        let (dx, dy) = (x - self.center[0], y - self.center[1]);
        let across = dx * self.cos + dy * self.sin;
        let up = dy * self.cos - dx * self.sin;
        across.abs() * self.zoom < self.half[0] && up.abs() * self.zoom < self.half[1]
    }
}

// A walker of the chaos game that remembers the seeds of its latest steps.
pub(crate) struct Tracer {
    at: Point,
    seeds: [u64; MEMORY],
    head: usize,
}

impl Tracer {
    pub(crate) fn new(flame: &Compiled, rng: &mut Rng) -> Self {
        let mut tracer = Self {
            at: (rng.range(-1.0, 1.0), rng.range(-1.0, 1.0), rng.next_f64()),
            seeds: [0; MEMORY],
            head: 0,
        };
        for _ in 0..FUSE.max(MEMORY) {
            tracer.step(flame, rng);
        }
        tracer
    }

    #[inline]
    fn step(&mut self, flame: &Compiled, rng: &mut Rng) {
        let seed = rng.next_u64();
        self.at = flame
            .seeded(seed, self.at)
            .unwrap_or_else(|| (rng.range(-1.0, 1.0), rng.range(-1.0, 1.0), self.at.2));
        self.head = (self.head + 1) % MEMORY;
        self.seeds[self.head] = seed;
    }

    #[inline]
    fn latest(&self, back: usize) -> u64 {
        self.seeds[(self.head + MEMORY - back) % MEMORY]
    }
}

// A particle is a point of the flame inside the window together with the seeds that made it: seed 0 finishes the
// point, seeds 1.. are the steps of the chaos game, the latest first. A move keeps the first `keep` seeds and takes
// the rest, and the point they start from, from the tracer: a new point that shares the particle's latest steps.
#[inline]
fn moved(flame: &Compiled, window: &Window, seeds: &[u64], keep: usize, tracer: &Tracer, fresh: u64) -> Option<Point> {
    let mut at = tracer.at;
    for step in (1..keep).rev() {
        at = flame.seeded(seeds[step], at)?;
    }
    let finish = if keep == 0 { fresh } else { seeds[0] };
    let last = if keep <= 1 { tracer.latest(0) } else { seeds[1] };
    flame.finished(finish, last, at).filter(|point| window.holds(*point))
}

#[inline]
fn take(seeds: &mut [u64], keep: usize, tracer: &Tracer, fresh: u64) {
    if keep == 0 {
        seeds[0] = fresh;
    }
    for (back, seed) in seeds[keep.max(1)..].iter_mut().enumerate() {
        *seed = tracer.latest(back);
    }
}

pub(crate) struct Plan {
    pub(crate) window: Window,
    seeds: Vec<u64>,
    points: Vec<Point>,
    range: (usize, usize),
    pub(crate) exposure: f64,
    pub(crate) share: f64,
}

// One stream's share of the particles, moved and plotted in turn.
pub(crate) struct Chains {
    seeds: Vec<u64>,
    points: Vec<Point>,
    next: usize,
}

impl Plan {
    pub(crate) fn is_empty(&self) -> bool {
        self.points.is_empty()
    }

    pub(crate) fn chains(&self, stream: usize, streams: usize) -> Chains {
        let mine = (stream..self.points.len()).step_by(streams);
        Chains {
            seeds: mine
                .clone()
                .flat_map(|particle| self.seeds[particle * WIDTH..(particle + 1) * WIDTH].iter().copied())
                .collect(),
            points: mine.map(|particle| self.points[particle]).collect(),
            next: 0,
        }
    }

    #[inline]
    pub(crate) fn point(&self, flame: &Compiled, chains: &mut Chains, tracer: &mut Tracer, rng: &mut Rng) -> Point {
        let particle = chains.next;
        chains.next = (chains.next + 1) % chains.points.len();
        let seeds = &mut chains.seeds[particle * WIDTH..(particle + 1) * WIDTH];
        // Now and then a far move, which is rarely accepted: it carries particles between the parts of the view.
        if self.range.0 > 0 && rng.below(FAR_EVERY) == 0 {
            let keep = rng.below(self.range.0 as u64) as usize;
            let fresh = rng.next_u64();
            tracer.step(flame, rng);
            if let Some(point) = moved(flame, &self.window, seeds, keep, tracer, fresh) {
                take(seeds, keep, tracer, fresh);
                chains.points[particle] = point;
            }
        }
        let keep = self.range.0 + rng.below((self.range.1 - self.range.0 + 1) as u64) as usize;
        let fresh = rng.next_u64();
        tracer.step(flame, rng);
        if let Some(point) = moved(flame, &self.window, seeds, keep, tracer, fresh) {
            take(seeds, keep, tracer, fresh);
            chains.points[particle] = point;
        }
        chains.points[particle]
    }
}

// A view that holds little of the flame is drawn from particles: points of the flame that are inside it. They are
// found window by window, each half the size of the one before: the particles that fall out of the smaller window
// are replaced by copies of the ones that stay, and every particle is then moved about inside it. `None`: enough of
// the flame is in view for the plain chaos game.
pub(crate) fn plan(
    flame: &Compiled,
    camera: &Camera,
    wide: f64,
    high: f64,
    held: Exposure,
    rng_of: &(dyn Fn(u64) -> Rng + Sync),
    cancelled: &dyn Fn() -> bool,
) -> Option<Plan> {
    let mut rng = rng_of(0);
    let (mut steps, mut zoom) = (1, camera.zoom);
    while zoom > 2.0 {
        zoom *= 0.5;
        steps += 1;
    }
    let mut window = Window::new(camera, wide, high, zoom);
    let enough = IN_VIEW * flame.plotted();
    let mut tracer = Tracer::new(flame, &mut rng);

    let (mut seeds, mut points) = (Vec::with_capacity(PARTICLES * WIDTH), Vec::with_capacity(PARTICLES));
    let mut inside = 0usize;
    for _ in 0..PILOT {
        tracer.step(flame, &mut rng);
        let fresh = rng.next_u64();
        if let Some(point) = moved(flame, &window, &[], 0, &tracer, fresh) {
            inside += 1;
            if points.len() < PARTICLES {
                seeds.push(fresh);
                seeds.extend((0..MEMORY).map(|back| tracer.latest(back)));
                points.push(point);
            }
        }
    }
    let mut share = inside as f64 / PILOT as f64;
    let mut seen = vec![(zoom * 0.5, 1.0), (zoom, share)];
    let mut range = (0, MEMORY);
    let mut groups: Vec<(Tracer, Rng)> = (0..PARTICLES / GROUP)
        .map(|group| {
            let mut rng = rng_of(1 + group as u64);
            (Tracer::new(flame, &mut rng), rng)
        })
        .collect();

    for step in 0..steps {
        if step + 1 == steps && share >= enough {
            return None;
        }
        if cancelled() {
            return None;
        }
        if points.is_empty() {
            break;
        }
        let survivors = points.len();
        for particle in survivors..PARTICLES {
            let from = particle % survivors;
            seeds.extend_from_within(from * WIDTH..(from + 1) * WIDTH);
            points.push(points[from]);
        }

        let (low, high_end) = (range.0, (range.1 + AHEAD).min(MEMORY));
        let sweep = |((seeds, points), (tracer, rng)): (Group<'_>, &mut (Tracer, Rng))| {
            let mut tally = Tally::default();
            for _ in 0..SWEEPS {
                for (own, at) in seeds.chunks_exact_mut(WIDTH).zip(points.iter_mut()) {
                    let keep = low + rng.below((high_end - low + 1) as u64) as usize;
                    let fresh = rng.next_u64();
                    tracer.step(flame, rng);
                    tally.tried[keep] += 1;
                    if let Some(point) = moved(flame, &window, own, keep, tracer, fresh) {
                        take(own, keep, tracer, fresh);
                        tally.travelled[keep] += ((point.0 - at.0).abs() + (point.1 - at.1).abs()) * window.zoom;
                        tally.accepted[keep] += 1;
                        *at = point;
                    }
                }
            }
            tally
        };
        let work = seeds
            .chunks_mut(GROUP * WIDTH)
            .zip(points.chunks_mut(GROUP))
            .zip(groups.iter_mut());
        #[cfg(feature = "parallel")]
        let tallies: Vec<Tally> = {
            use rayon::prelude::*;
            work.collect::<Vec<_>>().into_par_iter().map(sweep).collect()
        };
        #[cfg(not(feature = "parallel"))]
        let tallies: Vec<Tally> = work.map(sweep).collect();
        // Summed in the order of the groups, whichever thread finished first.
        let mut tried = [0u32; WIDTH];
        let mut accepted = [0u32; WIDTH];
        let mut travelled = [0f64; WIDTH];
        for tally in &tallies {
            for keep in 0..WIDTH {
                tried[keep] += tally.tried[keep];
                accepted[keep] += tally.accepted[keep];
                travelled[keep] += tally.travelled[keep];
            }
        }
        let often = |keep: usize| tried[keep] >= 16 && f64::from(accepted[keep]) >= RARE * f64::from(tried[keep]);
        let fine = |keep: usize| accepted[keep] >= 8 && travelled[keep] < FINE * 4.0 * f64::from(accepted[keep]);
        let first = (low..=high_end).find(|&keep| often(keep)).unwrap_or(high_end);
        let last = (first..=high_end).find(|&keep| fine(keep)).unwrap_or(high_end);
        range = (first, last);

        if step + 1 < steps {
            window.zoom *= 2.0;
            let mut kept = 0;
            for particle in 0..PARTICLES {
                if window.holds(points[particle]) {
                    seeds.copy_within(particle * WIDTH..(particle + 1) * WIDTH, kept * WIDTH);
                    points[kept] = points[particle];
                    kept += 1;
                }
            }
            seeds.truncate(kept * WIDTH);
            points.truncate(kept);
            share *= kept as f64 / PARTICLES as f64;
            seen.push((window.zoom, share));
        }
    }

    window.zoom = camera.zoom;
    Some(Plan {
        window,
        seeds,
        points,
        range,
        exposure: match held {
            Exposure::View => exposure(&seen, enough),
            Exposure::Scene | Exposure::Visible => share * camera.zoom * camera.zoom,
        },
        share,
    })
}

// The brightness a deep view keeps is the one the picture had at the zoom where the view began to hold less than
// `enough` of the flame; from there on the average density in view stays what it was.
fn exposure(seen: &[(f64, f64)], enough: f64) -> f64 {
    let Some(at) = (1..seen.len()).find(|&at| seen[at].1 < enough) else {
        let (zoom, share) = seen[seen.len() - 1];
        return share * zoom * zoom;
    };
    let ((near_zoom, near), (far_zoom, far)) = (seen[at - 1], seen[at]);
    if far <= 0.0 || near <= far {
        return near.min(enough) * near_zoom * near_zoom;
    }
    let slope = ln(near / far) / ln(far_zoom / near_zoom);
    let zoom = near_zoom * pow(near / enough, 1.0 / slope);
    enough * zoom * zoom
}

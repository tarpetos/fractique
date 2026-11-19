use fractique_core::math;

pub const SHADER: &str = concat!(include_str!("flame.wgsl"), include_str!("deep.wgsl"));

// fractique-core's deep.rs: below this share of the points in view the view is drawn from particles, which
// remember this many steps and are moved this many times in every window.
pub const IN_VIEW: f64 = 0.125;
pub const MEMORY: u32 = 128;
pub const SWEEPS: u32 = 6;
const WIDEST: f64 = 2.0;
const RARE: f64 = 0.05;
const FINE: f64 = 1.0 / 512.0;
const AHEAD: u32 = 8;
// The steps a particle's walker takes to look for its first point: with 8192 particles, the 65 536 of the core.
const PILOT: u32 = 8;
const FEWEST: u32 = 8192;
const MOST: u32 = 65536;
// deep.wgsl: the numbers a particle holds, and where the notes of a sweep are.
pub const SLOT: u64 = 144;
const TRIED: usize = 4;
const ACCEPTED: usize = 133;
const TRAVELLED: usize = 262;
pub const HEAD: usize = 392;
// A move is told in this many units for a view's shorter side taken four times.
const UNITS: f64 = 2048.0;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Batch {
    pub steps: u32,
    pub fuse: u32,
    pub seed: [u32; 2],
    pub layer: u32,
    pub walkers: u32,
    low: u32,
    high: u32,
    far: u32,
    cap: u32,
    plots: u32,
    spare: u32,
    window: f32,
    next: f32,
    travel: f32,
    last: f32,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Stage {
    Pilot,
    Window,
    Plot,
}

// What fractique-core's deep.rs works out before it draws a deep view, in the steps the graphics card takes it
// in: after each, the card tells what its particles did and the next step is decided here.
pub struct Plan {
    pub particles: u32,
    pub stage: Stage,
    zoom: f64,
    window: f64,
    left: u32,
    enough: f64,
    share: f64,
    seen: Vec<(f64, f64)>,
    range: (u32, u32),
    marks: Vec<u32>,
    // What turns the points plotted into densities, times the cells and over the points; 0 for a view the flame
    // has nothing in.
    pub exposure: f64,
}

pub fn particles(cells: u64, group: u32) -> u32 {
    ((cells / 64) as u32).clamp(FEWEST, MOST).div_ceil(group) * group
}

impl Plan {
    pub fn new(zoom: f64, plotted: f64, particles: u32) -> Self {
        let (mut window, mut left) = (zoom, 0);
        while window > WIDEST {
            window *= 0.5;
            left += 1;
        }
        Self {
            particles,
            stage: Stage::Pilot,
            zoom,
            window,
            left,
            enough: IN_VIEW * plotted,
            share: 0.0,
            seen: Vec::new(),
            range: (0, MEMORY),
            marks: Vec::new(),
            exposure: 0.0,
        }
    }

    // The steps of the chaos game one plotted point costs, about.
    pub fn cost(&self) -> u64 {
        u64::from(2 + (self.range.0 + self.range.1) / 2)
    }

    // The most steps of the chaos game one particle takes in a sweep of a window.
    pub fn sweep(&self) -> u64 {
        u64::from(self.range.1.saturating_add(AHEAD).min(MEMORY) + 1)
    }

    pub fn batch(&self, seed: u64, layer: u32, shorter: u32, steps: u32) -> Batch {
        let window = self.window / self.zoom;
        let next = if self.left > 0 { 2.0 * window } else { window };
        let (low, high, far, cap, steps) = match self.stage {
            Stage::Pilot => (0, 0, 0, MEMORY, PILOT),
            Stage::Window => (self.range.0, (self.range.1 + AHEAD).min(MEMORY), 0, MEMORY, 1),
            Stage::Plot => (self.range.0, self.range.1, self.range.0, self.range.1.max(1), steps),
        };
        Batch {
            steps,
            fuse: 0,
            seed: [seed as u32, (seed >> 32) as u32],
            layer,
            walkers: self.particles,
            low,
            high,
            far,
            cap,
            plots: u32::from(self.stage == Stage::Plot),
            spare: 0,
            window: window as f32,
            next: next as f32,
            travel: (4.0 / f64::from(shorter) * window * UNITS) as f32,
            last: 0.0,
        }
    }

    #[cfg(test)]
    pub fn share(&self) -> f64 {
        self.share
    }

    pub fn more(&self) -> bool {
        self.left > 0
    }

    // For every particle the one whose place it takes: itself when it stayed in the window, and the ones that
    // stayed in turn for the others.
    pub fn sources(&self) -> Vec<u32> {
        let stayed: Vec<u32> = (0..self.particles)
            .filter(|particle| self.marks.get(*particle as usize) == Some(&1))
            .collect();
        let mut gone = 0;
        (0..self.particles)
            .map(|particle| {
                if stayed.is_empty() || self.marks.get(particle as usize) == Some(&1) {
                    return particle;
                }
                gone += 1;
                stayed[(gone - 1) % stayed.len()]
            })
            .collect()
    }

    fn settle(&mut self, notes: &[u32]) {
        self.marks = notes[HEAD..].iter().take(self.particles as usize).copied().collect();
        if self.marks.iter().all(|mark| *mark == 0) {
            (self.stage, self.exposure) = (Stage::Plot, 0.0);
        } else {
            self.stage = Stage::Window;
        }
    }

    pub fn piloted(&mut self, notes: &[u32]) {
        self.share = f64::from(notes[0]) / (f64::from(self.particles) * f64::from(PILOT));
        self.seen = vec![(self.window * 0.5, 1.0), (self.window, self.share)];
        self.settle(notes);
    }

    pub fn swept(&mut self, notes: &[u32]) {
        let (low, high) = (self.range.0, (self.range.1 + AHEAD).min(MEMORY));
        let at = |from: usize, keep: u32| f64::from(notes[from + keep as usize]);
        let often = |keep: u32| at(TRIED, keep) >= 16.0 && at(ACCEPTED, keep) >= RARE * at(TRIED, keep);
        let fine =
            |keep: u32| at(ACCEPTED, keep) >= 8.0 && at(TRAVELLED, keep) < FINE * 4.0 * UNITS * at(ACCEPTED, keep);
        let first = (low..=high).find(|keep| often(*keep)).unwrap_or(high);
        let last = (first..=high).find(|keep| fine(*keep)).unwrap_or(high);
        self.range = (first, last);
        if self.left == 0 {
            (self.stage, self.exposure) = (Stage::Plot, exposure(&self.seen, self.enough));
            return;
        }
        self.left -= 1;
        self.window *= 2.0;
        self.share *= f64::from(notes[2]) / f64::from(self.particles);
        self.seen.push((self.window, self.share));
        self.settle(notes);
    }
}

// fractique-core's deep.rs: the brightness a deep view keeps is the one the picture had at the zoom where the view
// began to hold less than `enough` of the flame.
fn exposure(seen: &[(f64, f64)], enough: f64) -> f64 {
    let Some(at) = (1..seen.len()).find(|&at| seen[at].1 < enough) else {
        let (zoom, share) = seen[seen.len() - 1];
        return share * zoom * zoom;
    };
    let ((near_zoom, near), (far_zoom, far)) = (seen[at - 1], seen[at]);
    if far <= 0.0 || near <= far {
        return near.min(enough) * near_zoom * near_zoom;
    }
    let slope = math::ln(near / far) / math::ln(far_zoom / near_zoom);
    let zoom = near_zoom * math::pow(near / enough, 1.0 / slope);
    enough * zoom * zoom
}

#[cfg(test)]
mod tests {
    use super::*;

    fn notes(particles: u32) -> Vec<u32> {
        vec![0; HEAD + particles as usize]
    }

    #[test]
    fn a_plan_halves_the_window_down_to_the_view() {
        let plan = Plan::new(14.0, 1.0, 8192);
        assert_eq!((plan.window, plan.left), (1.75, 3));
        let batch = plan.batch(7, 0, 360, 0);
        assert_eq!((batch.window, batch.next), (0.125, 0.25));
        assert_eq!(size_of::<Batch>(), 64);
        let whole = Plan::new(1.5, 1.0, 8192);
        assert_eq!((whole.window, whole.left), (1.5, 0));
    }

    #[test]
    fn particles_that_fell_out_become_those_that_stayed() {
        let mut plan = Plan::new(14.0, 1.0, 8);
        let mut told = notes(8);
        told[0] = 16;
        told[HEAD..].copy_from_slice(&[0, 1, 0, 0, 1, 1, 0, 0]);
        plan.piloted(&told);
        assert_eq!(plan.stage, Stage::Window);
        assert_eq!(plan.share, 0.25);
        assert_eq!(plan.sources(), [1, 1, 4, 5, 4, 5, 1, 4]);

        let mut none = Plan::new(14.0, 1.0, 8);
        none.piloted(&notes(8));
        assert_eq!((none.stage, none.exposure), (Stage::Plot, 0.0));
    }

    #[test]
    fn a_sweep_picks_the_steps_a_particle_keeps() {
        let mut plan = Plan::new(3.0, 1.0, 64);
        let mut told = notes(64);
        told[0] = 64;
        told[HEAD..].fill(1);
        plan.piloted(&told);
        for keep in 0..=MEMORY as usize {
            told[TRIED + keep] = 100;
            told[ACCEPTED + keep] = if keep < 5 { 2 } else { 60 };
            told[TRAVELLED + keep] = if keep < 20 { 60 * 400 } else { 60 * 3 };
        }
        told[2] = 16;
        plan.swept(&told);
        assert_eq!(plan.range, (5, 20));
        assert_eq!((plan.stage, plan.left, plan.window), (Stage::Window, 0, 3.0));
        assert_eq!(plan.share, 0.125 * 0.25);
        plan.swept(&told);
        assert_eq!(plan.stage, Stage::Plot);
        assert_eq!(plan.cost(), 14);
        // The view began to hold less than an eighth between the two windows: where the share, which falls as a
        // power of the zoom there, was an eighth.
        let slope = math::ln(0.125 / 0.03125) / math::ln(2.0);
        let zoom = 1.5 * math::pow(1.0, 1.0 / slope);
        assert!((plan.exposure - 0.125 * zoom * zoom).abs() < 1e-12);
    }
}

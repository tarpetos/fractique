use fractique_core::{deep, math};

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
// deep.wgsl: the carry of the points in the view, and the room of the tiles' shifts after the particles' notes.
const CARRY: usize = 3;
pub const TILES: usize = 576;
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
    tiles: u32,
    window: f32,
    next: f32,
    travel: f32,
    last: f32,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Stage {
    Pilot,
    Window,
    Spread,
    Plot,
}

// What fractique-core's deep.rs works out before it draws a deep view, in the steps the graphics card takes it
// in: after each, the card tells what its particles did and the next step is decided here.
pub struct Plan {
    pub particles: u32,
    pub stage: Stage,
    // The zoom of the view, and the zooms of the windows from the first to the picture's: the view's own halved
    // until it is at most WIDEST, then doubled at every step up to the picture, which is the view or, with a
    // margin, wider than it.
    zoom: f64,
    windows: Vec<f64>,
    at: usize,
    enough: f64,
    share: f64,
    seen: Vec<(f64, f64)>,
    range: (u32, u32),
    marks: Vec<u32>,
    // How many times wider than the view the picture is: the view in its middle is the one whose brightness the
    // picture keeps.
    wide: f64,
    // Where the picture is the first window: the share of the flame in the view, as the pilot found it.
    direct: Option<f64>,
    // What turns the points plotted into densities, times the cells and over the points; 0 for a view the flame
    // has nothing in.
    pub exposure: f64,
    // fractique-core's deep.rs: how many times the densities are raised, and the tiles of the picture with the
    // power of two the points of each count.
    pub lift: f64,
    grid: (u32, u32),
    tiles: Option<Vec<u8>>,
}

pub fn particles(cells: u64, group: u32) -> u32 {
    ((cells / 64) as u32).clamp(FEWEST, MOST).div_ceil(group) * group
}

impl Plan {
    pub fn new(zoom: f64, plotted: f64, particles: u32, wide: f64, picture: [u32; 2]) -> Self {
        let (across, up) = deep::grid(f64::from(picture[0]), f64::from(picture[1]));
        let mut windows = vec![zoom];
        while windows[0] > WIDEST {
            windows.insert(0, windows[0] * 0.5);
        }
        if wide > 1.0 {
            let picture = zoom / wide;
            windows.retain(|window| *window < picture * (1.0 - 1e-9));
            windows.push(picture);
        }
        Self {
            particles,
            stage: Stage::Pilot,
            zoom,
            windows,
            at: 0,
            enough: IN_VIEW * plotted,
            share: 0.0,
            seen: Vec::new(),
            range: (0, MEMORY),
            marks: Vec::new(),
            wide,
            direct: None,
            exposure: 0.0,
            lift: 1.0,
            grid: (across as u32, up as u32),
            tiles: None,
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

    fn window(&self) -> f64 {
        self.windows[self.at]
    }

    fn left(&self) -> usize {
        self.windows.len() - 1 - self.at
    }

    // The share of the flame in a window counts for the brightness where the plan of the view without a margin
    // would have measured it: at the view's zoom halved a whole number of times.
    fn counted(&self, window: f64) -> bool {
        let mut step = self.zoom;
        while step > window * (1.0 + 1e-9) {
            step *= 0.5;
        }
        (step / window - 1.0).abs() < 1e-9
    }

    pub fn batch(&self, seed: u64, layer: u32, shorter: u32, steps: u32) -> Batch {
        let picture = self.zoom / self.wide;
        let window = self.window() / picture;
        let next = match self.windows.get(self.at + 1) {
            Some(next) => next / picture,
            None => self.wide,
        };
        let (low, high, far, cap, steps) = match self.stage {
            Stage::Pilot => (0, 0, 0, MEMORY, PILOT),
            Stage::Window => (self.range.0, (self.range.1 + AHEAD).min(MEMORY), 0, MEMORY, 1),
            Stage::Spread => (self.range.0, self.range.1, 0, MEMORY, 1),
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
            tiles: if self.tiles.is_some() || (self.stage == Stage::Window && self.left() == 0) {
                self.grid.0 | self.grid.1 << 16
            } else {
                0
            },
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

    // The power of two the points of every tile count, for the shaders.
    pub fn table(&self) -> [u32; TILES] {
        let mut table = [0; TILES];
        for (told, shift) in table.iter_mut().zip(self.tiles.iter().flatten()) {
            *told = u32::from(*shift);
        }
        table
    }

    pub fn grid(&self) -> [u32; 2] {
        match self.tiles {
            Some(_) => [self.grid.0, self.grid.1],
            None => [0; 2],
        }
    }

    // The points in the view, each as many times as it counts, of what the card tells after a batch of points.
    pub fn landed(notes: &[u32]) -> u64 {
        u64::from(notes[CARRY]) << 32 | u64::from(notes[1])
    }

    // For every particle the one whose place it takes. Out of a window: itself when it stayed in the window, and
    // the ones that stayed in turn for the others. Over the tiles: fractique-core's resampling, a particle as
    // often as the points of its tile count less; one that is taken stays itself, so that none is copied from
    // a particle that is written in the same pass.
    pub fn sources(&self) -> Vec<u32> {
        if let Some(tiles) = self.tiles.as_ref().filter(|_| self.stage == Stage::Spread) {
            let parts: Vec<u64> = self
                .marks
                .iter()
                .map(|tile| 1 << (deep::HEAVIEST - u32::from(tiles[*tile as usize])))
                .collect();
            let (total, count) = (parts.iter().sum::<u64>(), parts.len() as u64);
            let mut copies = vec![0usize; parts.len()];
            let (mut from, mut passed) = (0, parts[0]);
            for chosen in 0..count {
                while passed * count <= total / 2 + chosen * total {
                    from += 1;
                    passed += parts[from];
                }
                copies[from] += 1;
            }
            let mut spare = (0..self.particles)
                .flat_map(|particle| std::iter::repeat_n(particle, copies[particle as usize].saturating_sub(1)));
            return (0..self.particles)
                .map(|particle| match copies[particle as usize] {
                    0 => spare.next().unwrap_or(particle),
                    _ => particle,
                })
                .collect();
        }
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
        let mut first = self.zoom;
        while first > WIDEST {
            first *= 0.5;
        }
        self.seen = vec![(first * 0.5, 1.0)];
        if self.counted(self.window()) {
            self.seen.push((self.window(), self.share));
        }
        if self.wide > 1.0 && self.left() == 0 {
            self.direct = Some(f64::from(notes[1]) / (f64::from(self.particles) * f64::from(PILOT)));
        }
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
        let inside = f64::from(notes[2]) / f64::from(self.particles);
        if self.left() == 0 {
            let view = if self.wide > 1.0 {
                self.direct.unwrap_or(self.share * inside)
            } else {
                self.share
            };
            if self.wide > 1.0 {
                self.seen.push((self.zoom, view));
            }
            self.exposure = exposure(&self.seen, self.enough);
            self.lift = deep::lift(self.exposure, view, self.enough, self.zoom);
            self.marks = notes[HEAD..].iter().take(self.particles as usize).copied().collect();
            let mut held = vec![0u32; (self.grid.0 * self.grid.1) as usize];
            for tile in &self.marks {
                held[*tile as usize] += 1;
            }
            self.tiles = deep::shifts(&held).filter(|_| self.exposure > 0.0);
            self.stage = match self.tiles {
                Some(_) => Stage::Spread,
                None => Stage::Plot,
            };
            return;
        }
        self.at += 1;
        self.share *= inside;
        if self.counted(self.window()) {
            self.seen.push((self.window(), self.share));
        }
        self.settle(notes);
    }

    pub fn spread(&mut self) {
        self.stage = Stage::Plot;
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
        let plan = Plan::new(14.0, 1.0, 8192, 1.0, [640, 360]);
        assert_eq!((plan.window(), plan.left()), (1.75, 3));
        let batch = plan.batch(7, 0, 360, 0);
        assert_eq!((batch.window, batch.next), (0.125, 0.25));
        assert_eq!(size_of::<Batch>(), 64);
        let whole = Plan::new(1.5, 1.0, 8192, 1.0, [640, 360]);
        assert_eq!((whole.window(), whole.left()), (1.5, 0));
    }

    #[test]
    fn a_plan_with_a_margin_ends_at_the_wider_picture_and_measures_the_view() {
        // The windows of the view, but for the last: the picture, one and a half times as wide as the view.
        let plan = Plan::new(14.0, 1.0, 8192, 1.5, [640, 360]);
        assert_eq!(plan.windows, [1.75, 3.5, 7.0, 14.0 / 1.5]);
        let batch = plan.batch(7, 0, 360, 0);
        assert_eq!((batch.window, batch.next), (0.1875, 0.375));
        // The share of the flame is taken where the plan of the view alone takes it, not at the picture.
        assert!(plan.counted(7.0) && plan.counted(14.0) && !plan.counted(14.0 / 1.5));
        // Twice as wide, the picture is the window before the view.
        let half = Plan::new(14.0, 1.0, 8192, 2.0, [640, 360]);
        assert_eq!(half.windows, [1.75, 3.5, 7.0]);
        // A view that is its own first window: the picture is the only one, and the view is counted in it.
        let mut near = Plan::new(1.5, 1.0, 64, 1.5, [640, 360]);
        assert_eq!(near.windows, [1.0]);
        assert_eq!(near.batch(7, 0, 360, 0).next, 1.5);
        let mut told = notes(64);
        (told[0], told[1]) = (256, 64);
        told[HEAD..].fill(1);
        near.piloted(&told);
        assert_eq!(near.direct, Some(0.125));
        assert_eq!(near.seen, [(0.75, 1.0)]);
        for keep in 0..=MEMORY as usize {
            (told[TRIED + keep], told[ACCEPTED + keep], told[TRAVELLED + keep]) = (100, 60, 180);
        }
        told[2] = 16;
        near.swept(&told);
        assert_eq!(near.stage, Stage::Plot);
        assert_eq!(near.seen, [(0.75, 1.0), (1.5, 0.125)]);
        // The plan of the view alone sees the same, so the brightness is the same.
        let mut alone = Plan::new(1.5, 1.0, 64, 1.0, [640, 360]);
        told[0] = 64;
        alone.piloted(&told);
        alone.swept(&told);
        assert_eq!(alone.seen, near.seen);
        assert_eq!(alone.exposure, near.exposure);
    }

    #[test]
    fn particles_that_fell_out_become_those_that_stayed() {
        let mut plan = Plan::new(14.0, 1.0, 8, 1.0, [640, 360]);
        let mut told = notes(8);
        told[0] = 16;
        told[HEAD..].copy_from_slice(&[0, 1, 0, 0, 1, 1, 0, 0]);
        plan.piloted(&told);
        assert_eq!(plan.stage, Stage::Window);
        assert_eq!(plan.share, 0.25);
        assert_eq!(plan.sources(), [1, 1, 4, 5, 4, 5, 1, 4]);

        let mut none = Plan::new(14.0, 1.0, 8, 1.0, [640, 360]);
        none.piloted(&notes(8));
        assert_eq!((none.stage, none.exposure), (Stage::Plot, 0.0));
    }

    #[test]
    fn a_bright_tile_gives_its_particles_to_the_others() {
        let mut plan = Plan::new(1.5, 1.0, 64, 1.0, [640, 360]);
        assert_eq!(plan.grid, (24, 14));
        let mut told = notes(64);
        told[0] = 16;
        told[HEAD..].fill(1);
        plan.piloted(&told);
        assert_eq!(plan.batch(7, 0, 360, 0).tiles, 24 | 14 << 16);
        for keep in 0..=MEMORY as usize {
            (told[TRIED + keep], told[ACCEPTED + keep], told[TRAVELLED + keep]) = (100, 60, 180);
        }
        // 49 particles in the first tile, one in each of the 15 after it.
        for (particle, mark) in told[HEAD..].iter_mut().enumerate() {
            *mark = particle.saturating_sub(48) as u32;
        }
        plan.swept(&told);
        assert_eq!(plan.stage, Stage::Spread);
        let mut held = vec![0; 24 * 14];
        held[0] = 49;
        held[1..16].fill(1);
        let shifts = deep::shifts(&held).expect("the first tile holds most");
        assert!(shifts[0] > 0 && shifts[1..].iter().all(|shift| *shift == 0));
        assert!(
            plan.table()
                .iter()
                .zip(&shifts)
                .all(|(told, shift)| *told == u32::from(*shift))
        );
        assert_eq!(plan.grid(), [24, 14]);
        let sources = plan.sources();
        assert!(sources.iter().all(|source| sources[*source as usize] == *source));
        let bright = sources.iter().filter(|source| **source < 49).count();
        let fair = 64.0 * 49.0 / (49.0 + 15.0 * f64::from(1u32 << shifts[0]));
        assert!(
            (bright as f64 - fair).abs() <= 1.0,
            "{bright} of the first tile, {fair} are its part"
        );
        assert!((49..64).all(|particle| sources[particle] == particle as u32));
        plan.spread();
        assert_eq!(plan.stage, Stage::Plot);
        assert_eq!(plan.batch(7, 0, 360, 1).tiles, 24 | 14 << 16);

        let mut even = Plan::new(1.5, 1.0, 64, 1.0, [640, 360]);
        told[HEAD..].fill(1);
        even.piloted(&told);
        for (particle, mark) in told[HEAD..].iter_mut().enumerate() {
            *mark = particle as u32;
        }
        even.swept(&told);
        assert_eq!((even.stage, even.grid()), (Stage::Plot, [0; 2]));
        assert_eq!(even.batch(7, 0, 360, 1).tiles, 0);
        assert_eq!(even.table(), [0; TILES]);
    }

    #[test]
    fn a_view_beside_the_flame_is_lifted_as_the_core_lifts_it() {
        let mut plan = Plan::new(1.5, 1.0, 64, 1.0, [640, 360]);
        let mut told = notes(64);
        told[0] = 16;
        told[HEAD..].fill(1);
        plan.piloted(&told);
        for keep in 0..=MEMORY as usize {
            (told[TRIED + keep], told[ACCEPTED + keep], told[TRAVELLED + keep]) = (100, 60, 180);
        }
        told[HEAD..].fill(0);
        plan.swept(&told);
        assert_eq!(plan.lift, deep::lift(plan.exposure, 0.03125, IN_VIEW, 1.5));
        assert!((plan.lift - 1.0 / plan.exposure).abs() < 1e-12 && plan.lift > 5.0);
        assert!(deep::scale(plan.lift) < 0.125);
        assert_eq!(deep::scale(1.0) as f32, 0.125);
        // The view of a picture with a margin is lifted as the view alone: by its own share of the flame.
        let mut wide = Plan::new(1.5, 1.0, 64, 1.5, [960, 540]);
        (told[0], told[1]) = (64, 16);
        told[HEAD..].fill(1);
        wide.piloted(&told);
        wide.swept(&told);
        assert_eq!((wide.exposure, wide.lift), (plan.exposure, plan.lift));
        // A view that holds an eighth of the flame keeps its densities.
        let mut whole = Plan::new(1.5, 1.0, 64, 1.0, [640, 360]);
        told[0] = 64;
        whole.piloted(&told);
        whole.swept(&told);
        assert_eq!(whole.lift, 1.0);

        let mut counted = notes(1);
        (counted[1], counted[CARRY]) = (7, 2);
        assert_eq!(Plan::landed(&counted), (2 << 32) + 7);
    }

    #[test]
    fn a_sweep_picks_the_steps_a_particle_keeps() {
        let mut plan = Plan::new(3.0, 1.0, 64, 1.0, [640, 360]);
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
        assert_eq!((plan.stage, plan.left(), plan.window()), (Stage::Window, 0, 3.0));
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

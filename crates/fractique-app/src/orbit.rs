use fractique_core::fractal::{self, Draw, Orbit, Prepared};
use fractique_core::rng::Rng;
use fractique_core::{Fractal, math, palette};

pub const SHADER: &str = include_str!("orbit.wgsl");

const PARAMS: usize = 8;
const NUMBERS: usize = 16;
const TABLES: usize = 4;
pub const GROUP: u32 = 64;
// fractique-core's render.rs: the streams the points of a layer are split over.
const STREAMS: u64 = 64;
// An attractor is walked from many starting points at once, a few steps each, where the processor walks 64 for
// long. That shows the same only if an orbit looks late as it looks early: windows of this many points are
// compared on a grid, the first with those that begin after 8 and after 64 times as many.
const WINDOW: usize = 4096;
const LATER: [usize; 2] = [8, 64];
const GRID: usize = 32;
// Two windows of one attractor differ by 0.1 to 0.3 of their points; an orbit that ends on a cycle by nearly all.
const STEADY: f64 = 0.5;
const STEADY_DOMAIN: u64 = 0x7200;

enum Form {
    // An attractor of fractique-core's `Traced`: its numbers and tables are those `prepared` gives.
    Traced,
    // A map with nothing prepared, colored by its own rule: the steps before its first point.
    Bare(u32),
}

// A kind the graphics card draws: its name in fractique-core, its number in orbit.wgsl, and the cells a unit times
// the distance of the view from the origin above which 32-bit numbers show (measured, docs/architecture.md).
struct Entry {
    kind: &'static str,
    code: u32,
    form: Form,
    finest: f64,
}

const KINDS: [Entry; 4] = [
    Entry {
        kind: "clifford",
        code: 1,
        form: Form::Bare(32),
        finest: 4.0e5,
    },
    Entry {
        kind: "de_jong",
        code: 2,
        form: Form::Traced,
        finest: 4.0e5,
    },
    Entry {
        kind: "lorenz",
        code: 3,
        form: Form::Traced,
        finest: 4.0e5,
    },
    Entry {
        kind: "ikeda",
        code: 4,
        form: Form::Traced,
        finest: 4.0e5,
    },
];

// Whether an orbit looks late as it looks early.
fn steady(orbit: &dyn Orbit) -> bool {
    let mut rng = Rng::derive(0, STEADY_DOMAIN, 0);
    let mut walk = orbit.walk(&mut rng);
    let mut windows = Vec::new();
    let mut walked = 0;
    for begin in [0].into_iter().chain(LATER.map(|times| times * WINDOW)) {
        walk.run(&mut rng, (begin - walked) as u64, &mut |_, _, _| {});
        let mut points = Vec::with_capacity(WINDOW);
        walk.run(&mut rng, WINDOW as u64, &mut |x, y, _| points.push((x, y)));
        walked = begin + WINDOW;
        windows.push(points);
    }
    let all = || windows.iter().flatten().filter(|(x, y)| x.is_finite() && y.is_finite());
    let span = |pick: fn(&(f64, f64)) -> f64| {
        let (low, high) = all()
            .map(pick)
            .fold((f64::MAX, f64::MIN), |(low, high), at| (low.min(at), high.max(at)));
        (low, (high - low).max(1e-12))
    };
    let ((left, wide), (bottom, tall)) = (span(|point| point.0), span(|point| point.1));
    let cells = |points: &[(f64, f64)]| {
        let mut cells = vec![0i32; GRID * GRID];
        for (x, y) in points.iter().filter(|(x, y)| x.is_finite() && y.is_finite()) {
            let across = (((x - left) / wide * GRID as f64) as usize).min(GRID - 1);
            let down = (((y - bottom) / tall * GRID as f64) as usize).min(GRID - 1);
            cells[down * GRID + across] += 1;
        }
        cells
    };
    let first = cells(&windows[0]);
    windows[1..].iter().all(|window| {
        let apart: i32 = cells(window).iter().zip(&first).map(|(a, b)| (a - b).abs()).sum();
        f64::from(apart) < STEADY * 2.0 * WINDOW as f64
    })
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Packed {
    counts: [u32; 4],
    view: [f32; 4],
    scale: [f32; 4],
    extent: [u32; 4],
    span: [u32; 4],
    tables: [u32; 4],
    sizes: [u32; 4],
    middle: [f32; 4],
    ranks: [f32; 4],
    params: [f32; PARAMS],
    numbers: [f32; NUMBERS],
    palette: [[u32; 4]; 64],
}

pub fn drawn(fractal: &Fractal) -> bool {
    KINDS.iter().any(|entry| entry.kind == fractal.kind)
}

// What a kind worked out from its parameters, kept for the next view of the same fractal.
pub struct Recipe {
    entry: &'static Entry,
    values: Vec<f64>,
    prepared: Prepared,
    colors: Option<palette::Lut>,
}

impl Recipe {
    pub fn key(fractal: &Fractal) -> String {
        format!("{} {:?}", fractal.kind, fractal.params)
    }

    pub fn new(fractal: &Fractal) -> Option<Self> {
        let entry = KINDS.iter().find(|entry| entry.kind == fractal.kind)?;
        let spec = fractal::spec(&fractal.kind)?;
        let Draw::Density(make) = spec.draw else {
            return None;
        };
        let values = spec.values(fractal);
        let orbit = make(&values, &fractal.camera);
        let prepared = orbit.prepared();
        let fits = values.len() <= PARAMS
            && prepared.tables.len() <= TABLES
            && match entry.form {
                // An orbit that ends is drawn only as the states the core tracked of it: a walker that had to
                // walk to an age of it first would take steps without a bound that is worth the card.
                Form::Traced => {
                    (13..=13 + NUMBERS).contains(&prepared.numbers.len())
                        && prepared.tables.len() == 3
                        && if prepared.numbers[4] > 0.0 {
                            prepared.numbers[12] > 0.0 && !prepared.tables[2].is_empty()
                        } else {
                            steady(&*orbit)
                        }
                }
                Form::Bare(_) => steady(&*orbit),
            };
        fits.then(|| Self {
            entry,
            values,
            colors: orbit.colors(),
            prepared,
        })
    }

    // The steps of a tracked orbit, as the walkers of the card walk it.
    fn span(&self) -> u64 {
        match self.entry.form {
            Form::Traced => {
                let (span, every) = (self.prepared.numbers[4] as u64, self.prepared.numbers[12] as u64);
                span.min(self.prepared.tables[2].len() as u64 / 3 * every)
            }
            Form::Bare(_) => 0,
        }
    }

    // The processor walks each of its streams from the start of an orbit: with few points a long orbit is never
    // walked to its end, and the card stops where the processor does.
    fn stream(goal: u64) -> u64 {
        goal.div_ceil(STREAMS).max(1)
    }

    // The most steps `walkers` take to plot `goal` points: each takes the steps before its first point unseen.
    // What the card is asked for is held against it before anything is sent.
    pub fn work(&self, goal: u64, walkers: u32) -> u64 {
        let unseen = match self.entry.form {
            Form::Traced if self.prepared.numbers[12] > 0.0 => 0,
            Form::Traced => self.prepared.numbers[5] as u64,
            Form::Bare(burn) => u64::from(burn),
        };
        goal.saturating_add(u64::from(walkers).saturating_mul(unseen))
    }

    // The fractal as the shader reads it and the numbers of its tables, or nothing for a view too fine for 32-bit
    // numbers. `slot` is the first slot after the walkers.
    pub fn packed(
        &self,
        fractal: &Fractal,
        [width, height]: [u32; 2],
        margin: [u32; 2],
        goal: u64,
        slot: u32,
        finest: Option<f64>,
    ) -> Option<(Box<Packed>, Vec<f32>)> {
        let camera = &fractal.camera;
        let per_unit = camera.zoom * f64::from(width.min(height)) / 4.0;
        let [across, down] = [width + 2 * margin[0], height + 2 * margin[1]];
        let far = camera.center[0].abs().max(camera.center[1].abs()) + 0.5 * f64::from(across.max(down)) / per_unit;
        if per_unit * far > finest.unwrap_or(self.entry.finest) {
            return None;
        }
        let mut data: Box<Packed> = Box::new(bytemuck::Zeroable::zeroed());
        let numbers = &self.prepared.numbers;
        let mut extras: &[f64] = numbers;
        match self.entry.form {
            Form::Traced => {
                let fitted = u32::from(numbers[6] > 0.5) | u32::from(numbers[7] > 0.5) << 1;
                data.counts = [self.entry.code, numbers[0] as u32, fitted, numbers[5] as u32];
                data.middle = [numbers[1] as f32, numbers[2] as f32, (1.0 / numbers[3]) as f32, 0.0];
                data.ranks = [
                    numbers[8] as f32,
                    numbers[9] as f32,
                    numbers[10] as f32,
                    numbers[11] as f32,
                ];
                let every = numbers[12] as u32;
                let tracked = (self.prepared.tables[2].len() / 3) as u32;
                let stream = Self::stream(goal).min(u64::from(u32::MAX));
                data.span = [
                    self.span().min(u64::from(u32::MAX)) as u32,
                    every,
                    tracked,
                    stream as u32,
                ];
                extras = &numbers[13..];
            }
            Form::Bare(burn) => {
                data.counts = [self.entry.code, 3, 0, burn];
                data.middle = [0.0, 0.0, 1.0, 0.0];
            }
        }
        for (to, from) in data.numbers.iter_mut().zip(extras) {
            *to = *from as f32;
        }
        for (to, from) in data.params.iter_mut().zip(&self.values) {
            *to = *from as f32;
        }
        let mut tables = Vec::new();
        for (index, table) in self.prepared.tables.iter().enumerate() {
            data.tables[index] = tables.len() as u32;
            data.sizes[index] = table.len() as u32;
            tables.extend(table.iter().map(|value| *value as f32));
        }
        let colors = self.colors.unwrap_or_else(|| palette::lut(&fractal.palette));
        for (shade, [r, g, b]) in colors.into_iter().enumerate() {
            data.palette[shade / 4][shade % 4] = u32::from(r) | u32::from(g) << 8 | u32::from(b) << 16;
        }
        let angle = camera.rotation.to_radians();
        let up = if camera.flip_y { -per_unit } else { per_unit };
        data.view = [
            camera.center[0] as f32,
            camera.center[1] as f32,
            math::cos(angle) as f32,
            math::sin(angle) as f32,
        ];
        data.scale = [per_unit as f32, up as f32, across as f32, down as f32];
        data.extent = [across, down, slot, 0];
        Some((data, tables))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_orbit_kinds_are_packed_as_the_shader_reads_them() {
        assert_eq!(size_of::<Packed>(), 9 * 16 + 4 * PARAMS + 4 * NUMBERS + 1024);
        let mut codes: Vec<u32> = KINDS.iter().map(|entry| entry.code).collect();
        codes.dedup();
        assert_eq!(codes, [1, 2, 3, 4]);
        for entry in &KINDS {
            let scene = fractal::random_scene(entry.kind, 3).expect("the kind is listed");
            let shown = scene.layers[0].fractal().expect("the scene is a fractal");
            assert!(drawn(shown));
            let recipe = Recipe::new(shown).expect("the kind is one the graphics card draws");
            let (data, tables) = recipe
                .packed(shown, [640, 360], [0; 2], 1 << 20, 4096, None)
                .expect("the view is not deep");
            assert_eq!(data.counts[0], entry.code);
            assert_eq!(data.extent, [640, 360, 4096, 0]);
            assert_eq!(
                tables.len() as u32,
                data.tables[3]
                    .max(data.tables[2] + data.sizes[2])
                    .max(data.sizes.iter().sum())
            );
            assert!(recipe.work(1 << 20, 4096) <= (1 << 20) + 4096 * 32, "{}", entry.kind);

            let mut deep = shown.clone();
            deep.camera.zoom = 1.0e7;
            assert!(recipe.packed(&deep, [640, 360], [0; 2], 1 << 20, 4096, None).is_none());
        }
        // 32-bit numbers do not hold an orbit of the hopalong on its curve, the steps a walker of the Buddhabrot
        // takes for an orbit have no bound that is known before it walks, and the walkers of the Gumowski-Mira
        // map walk long orbits one step after another, which the processor does faster: they are the processor's.
        // The flows have no shader yet.
        let left: Vec<&str> = fractal::all()
            .filter(|spec| spec.family == fractal::Family::Orbit && !KINDS.iter().any(|entry| entry.kind == spec.name))
            .map(|spec| spec.name)
            .collect();
        assert_eq!(left, ["buddhabrot", "flow", "gumowski_mira", "hopalong"]);
    }

    #[test]
    fn the_work_of_the_walkers_is_known_before_they_walk() {
        let recipe = |name: &str| {
            let scene = fractal::preset(name).expect("the preset exists");
            Recipe::new(scene.layers[0].fractal().expect("the preset is a fractal")).expect("the card draws it")
        };
        let goal = 32 * 1920 * 1080;
        assert_eq!(recipe("clifford-classic").work(goal, 129_600), goal + 129_600 * 32);
        assert_eq!(recipe("de_jong-classic").work(goal, 129_600), goal + 129_600 * 32);
        assert_eq!(recipe("lorenz-butterfly").work(goal, 129_600), goal);
        assert_eq!(recipe("ikeda-classic").work(goal, 129_600), goal + 129_600 * 32);
        for name in ["gumowski_mira-feathers", "hopalong-classic", "buddhabrot-classic"] {
            let scene = fractal::preset(name).expect("the preset exists");
            assert!(
                Recipe::new(scene.layers[0].fractal().expect("a fractal")).is_none(),
                "{name}"
            );
        }
    }

    #[test]
    fn an_orbit_that_ends_on_a_cycle_is_left_to_the_processor() {
        for spec in fractal::all().filter(|spec| ["clifford", "de_jong", "ikeda"].contains(&spec.name)) {
            for preset in spec.presets {
                let scene = fractal::preset(&format!("{}-{}", spec.name, preset.name)).expect("the preset exists");
                assert!(
                    Recipe::new(scene.layers[0].fractal().expect("a fractal")).is_some(),
                    "{}",
                    preset.name
                );
            }
        }
        let scene = fractal::random_scene("de_jong", 7).expect("the kind is listed");
        assert!(Recipe::new(scene.layers[0].fractal().expect("a fractal")).is_none());
    }
}

use eframe::egui;
use std::io::Write as _;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, OnceLock};
use web_time::Instant;

static STARTED: OnceLock<Instant> = OnceLock::new();

pub fn started() {
    STARTED.get_or_init(Instant::now);
}

fn since_start() -> f32 {
    STARTED
        .get()
        .map_or(0.0, |start| start.elapsed().as_secs_f32() * 1000.0)
}

struct Sweep {
    at: f32,
    to: f32,
    step: f32,
    height: f32,
    every: u32,
}

// A test run's record: FRACTIQUE_TRACE=file gets a line a frame, FRACTIQUE_FRAMES=folder the frames a step asks
// for and the first FRACTIQUE_FIRST frames of the run, and FRACTIQUE_CLOCK=1 makes every frame a sixtieth of a
// second long, so a run repeats.
pub struct Probe {
    out: Option<std::io::BufWriter<std::fs::File>>,
    folder: Option<PathBuf>,
    pub fixed_clock: bool,
    // FRACTIQUE_VEIL: another strength of the blur over the picture, to compare.
    pub veil: Option<f32>,
    pub frame: u64,
    last: Option<Instant>,
    pub slept: f32,
    left: u32,
    saved: u32,
    // A frame asked for is the picture of the frame after the one that asked.
    asked: u32,
    wait: u32,
    chained: bool,
    sweep: Option<Sweep>,
    pub renders: u32,
    pub note: String,
    pending: Arc<AtomicU32>,
    stages: Vec<&'static str>,
}

impl Probe {
    pub fn new() -> Self {
        let out = std::env::var_os("FRACTIQUE_TRACE")
            .and_then(|path| std::fs::File::create(path).ok())
            .map(std::io::BufWriter::new);
        Self {
            out,
            folder: std::env::var_os("FRACTIQUE_FRAMES").map(PathBuf::from),
            fixed_clock: std::env::var_os("FRACTIQUE_CLOCK").is_some(),
            veil: std::env::var("FRACTIQUE_VEIL")
                .ok()
                .and_then(|share| share.parse().ok()),
            frame: 0,
            last: None,
            slept: 0.0,
            left: std::env::var("FRACTIQUE_FIRST")
                .ok()
                .and_then(|count| count.parse().ok())
                .unwrap_or(0),
            saved: 0,
            asked: 0,
            wait: 0,
            chained: false,
            sweep: None,
            renders: 0,
            note: String::new(),
            pending: Arc::default(),
            stages: Vec::new(),
        }
    }

    pub fn on(&self) -> bool {
        self.out.is_some()
    }

    pub fn stage(&mut self, name: &'static str) {
        if self.stages.contains(&name) {
            return;
        }
        self.stages.push(name);
        if let Some(out) = &mut self.out {
            let _ = writeln!(out, "start {name} {:.1}", since_start());
        }
    }

    pub fn recording(&self) -> bool {
        self.left > 0
    }

    pub fn pending(&self) -> bool {
        self.pending.load(Ordering::Relaxed) > 0
    }

    // frames:N records the next N frames; sweep:from:to:step:height[:every] records a window that is made wider
    // or narrower by a step every few frames.
    pub fn begin(&mut self, step: &str) -> bool {
        let numbers: Vec<f32> = step.split(':').skip(1).filter_map(|part| part.parse().ok()).collect();
        if step.starts_with("frames:")
            && let [count] = numbers[..]
        {
            self.left = count as u32;
        } else if step.starts_with("sweep:")
            && let [from, to, by, height, ..] = numbers[..]
        {
            let every = numbers.get(4).map_or(1, |every| (*every as u32).max(1));
            self.sweep = Some(Sweep {
                at: from,
                to,
                step: by.abs().max(1.0),
                height,
                every,
            });
            self.left = (((to - from).abs() / by.abs().max(1.0)).ceil() as u32 + 1) * every + 24;
        } else {
            return false;
        }
        self.chained = true;
        true
    }

    // While frames are recorded the steps after it are taken one a frame, skip:N waiting N frames, until rest.
    pub fn next(&mut self, steps: &mut Vec<String>) -> Option<String> {
        if !self.chained {
            return None;
        }
        if self.wait > 0 {
            self.wait -= 1;
            return None;
        }
        let step = steps.pop();
        match step.as_deref() {
            None | Some("rest") => {
                self.chained = false;
                None
            }
            Some(skip) if skip.starts_with("skip:") => {
                self.wait = skip[5..].parse().unwrap_or(0);
                None
            }
            _ => step,
        }
    }

    pub fn frame(&mut self, ctx: &egui::Context, work: f32, panes: [&[(egui::Rect, f32, bool)]; 2], fades: &[f32]) {
        self.frame += 1;
        let gap = self.last.map_or(0.0, |last| last.elapsed().as_secs_f32() * 1000.0);
        self.last = Some(Instant::now());
        if let Some(out) = &mut self.out {
            let screen = ctx.viewport_rect();
            let list = |panes: &[(egui::Rect, f32, bool)]| -> String {
                panes
                    .iter()
                    .enumerate()
                    .map(|(index, (rect, _, dark))| {
                        format!(
                            "{:.1},{:.1},{:.1},{:.1},{},{:.3}",
                            rect.min.x,
                            rect.min.y,
                            rect.max.x,
                            rect.max.y,
                            u8::from(*dark),
                            fades.get(index).copied().unwrap_or(1.0)
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(";")
            };
            let _ = writeln!(
                out,
                "frame {} t {:.1} gap {:.2} slept {:.2} work {:.2} size {:.0}x{:.0} renders {} shot {} drawn [{}] laid [{}] {}",
                self.frame,
                since_start(),
                gap,
                self.slept,
                work,
                screen.width(),
                screen.height(),
                self.renders,
                std::mem::take(&mut self.asked),
                list(panes[0]),
                list(panes[1]),
                self.note,
            );
            let _ = out.flush();
        }
        if self.left == 0 {
            return;
        }
        self.left -= 1;
        ctx.request_repaint();
        if let Some(sweep) = &mut self.sweep {
            let done = (sweep.at - sweep.to).abs() < 0.5;
            if !done && self.frame.is_multiple_of(u64::from(sweep.every)) {
                let by = sweep.step.min((sweep.to - sweep.at).abs());
                sweep.at += if sweep.to > sweep.at { by } else { -by };
                ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(sweep.at, sweep.height)));
            }
        }
        if self.left == 0 {
            self.sweep = None;
            self.chained = false;
        }
        let Some(folder) = &self.folder else {
            return;
        };
        self.saved += 1;
        self.asked = self.saved;
        let path = folder.join(format!("f{:04}.png", self.saved));
        let (pending, wake) = (self.pending.clone(), ctx.clone());
        pending.fetch_add(1, Ordering::Relaxed);
        let callback = egui::ScreenshotCallback::new(move |image: Arc<egui::ColorImage>| {
            std::thread::spawn(move || {
                let _ = crate::save_png(&path, image.as_raw(), image.size);
                pending.fetch_sub(1, Ordering::Relaxed);
                wake.request_repaint();
            });
        });
        ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(callback));
    }
}

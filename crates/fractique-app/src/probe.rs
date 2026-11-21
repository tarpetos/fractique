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
// A test run's window on a screen it never comes onto: FRACTIQUE_SCREEN=2560x1440 is that screen in points, and
// the window is maximized and made full screen by its size alone, as winit does it on Windows. 1920x1080+1046+42
// also says where the window that is neither stands on that screen; it stands in the middle otherwise.
#[cfg_attr(
    any(target_os = "android", target_os = "ios", target_arch = "wasm32"),
    allow(dead_code)
)]
pub struct Sham {
    screen: egui::Vec2,
    place: Option<egui::Pos2>,
    restored: egui::Vec2,
    // The size asked for last: until the window has it, its size is not the one to come back to.
    asked: Option<egui::Vec2>,
    pub maximized: bool,
    pub full: bool,
    kept: bool,
}

const TASKBAR: f32 = 48.0;

#[cfg_attr(
    any(target_os = "android", target_os = "ios", target_arch = "wasm32"),
    allow(dead_code)
)]
impl Sham {
    fn new() -> Option<Self> {
        let text = std::env::var("FRACTIQUE_SCREEN").ok()?;
        let mut parts = text.split('+');
        let (width, height) = parts.next()?.split_once('x')?;
        let mut corner = parts.filter_map(|part| part.parse::<f32>().ok());
        Some(Self {
            screen: egui::vec2(width.parse().ok()?, height.parse().ok()?),
            place: corner.next().zip(corner.next()).map(|(x, y)| egui::pos2(x, y)),
            restored: egui::Vec2::ZERO,
            asked: None,
            maximized: false,
            full: false,
            kept: false,
        })
    }

    pub fn origin(&self, window: egui::Vec2) -> egui::Pos2 {
        if self.full || self.maximized {
            egui::Pos2::ZERO
        } else {
            self.place.unwrap_or(((self.screen - window) * 0.5).to_pos2())
        }
    }

    fn keep(&mut self, window: egui::Vec2) {
        if !self.maximized && !self.full && self.asked.is_none_or(|asked| asked == window) {
            self.restored = window;
        }
    }

    fn size(&mut self) -> egui::Vec2 {
        let size = match (self.maximized, self.full) {
            (_, true) => self.screen,
            (true, false) => self.screen - egui::vec2(0.0, TASKBAR),
            (false, false) => self.restored,
        };
        self.asked = Some(size);
        size
    }

    pub fn maximize(&mut self, on: bool, window: egui::Vec2) -> egui::Vec2 {
        self.keep(window);
        self.maximized = on;
        self.size()
    }

    pub fn fullscreen(&mut self, on: bool, window: egui::Vec2) -> egui::Vec2 {
        self.keep(window);
        if on && !self.full {
            self.kept = self.maximized;
        } else if !on && self.full {
            self.maximized = self.kept;
        }
        self.full = on;
        self.size()
    }
}

pub struct Probe {
    out: Option<std::io::BufWriter<std::fs::File>>,
    folder: Option<PathBuf>,
    pub fixed_clock: bool,
    // FRACTIQUE_EVERY=3: of the frames a step records only every third is saved, so saving them does not slow
    // what is recorded.
    every: u64,
    // FRACTIQUE_VEIL: another strength of the blur over the picture, to compare.
    pub veil: Option<f32>,
    // FRACTIQUE_SCALE: the pixels a point, whatever the screen the window stands beside says.
    pub scale: Option<f32>,
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
    // Milliseconds of the frame's parts: taking pictures in, asking for one, the card's step.
    pub parts: [f32; 3],
    pub note: String,
    // How far an export is and what it does, for the frame's line; FRACTIQUE_CANCEL=writing (or rendering)
    // cancels one in that stage.
    pub export: String,
    #[cfg(not(target_arch = "wasm32"))]
    pub cancel: Option<String>,
    pending: Arc<AtomicU32>,
    stages: Vec<&'static str>,
    pub sham: Option<Sham>,
    pub pointer: Option<Pointer>,
}

impl Probe {
    pub fn new() -> Self {
        // FRACTIQUE_SHARE=0.5: the share of a frame a picture has, whatever the window needs.
        if let Some(share) = std::env::var("FRACTIQUE_SHARE")
            .ok()
            .and_then(|share| share.parse().ok())
        {
            crate::pace::fix(share);
        }
        let out = std::env::var_os("FRACTIQUE_TRACE")
            .and_then(|path| std::fs::File::create(path).ok())
            .map(std::io::BufWriter::new);
        Self {
            out,
            folder: std::env::var_os("FRACTIQUE_FRAMES").map(PathBuf::from),
            fixed_clock: std::env::var_os("FRACTIQUE_CLOCK").is_some(),
            every: std::env::var("FRACTIQUE_EVERY")
                .ok()
                .and_then(|every| every.parse().ok())
                .unwrap_or(1),
            veil: std::env::var("FRACTIQUE_VEIL")
                .ok()
                .and_then(|share| share.parse().ok()),
            scale: std::env::var("FRACTIQUE_SCALE")
                .ok()
                .and_then(|scale| scale.parse().ok()),
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
            parts: [0.0; 3],
            note: String::new(),
            export: String::new(),
            #[cfg(not(target_arch = "wasm32"))]
            cancel: std::env::var("FRACTIQUE_CANCEL").ok(),
            pending: Arc::default(),
            stages: Vec::new(),
            sham: Sham::new(),
            pointer: Pointer::new(),
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
        self.stage("recording");
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
                "frame {} t {:.1} gap {:.2} slept {:.2} work {:.2} size {:.0}x{:.0} renders {} shot {} drawn [{}] laid [{}] {}{}",
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
                std::mem::take(&mut self.export),
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
            self.stage("recorded");
        }
        let Some(folder) = self
            .folder
            .as_ref()
            .filter(|_| self.frame.is_multiple_of(self.every.max(1)))
        else {
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

struct Way {
    from: egui::Pos2,
    to: egui::Pos2,
    frames: u32,
    at: u32,
    press: bool,
}

const REST: u32 = 6;
const PRESS: u32 = 5;

// FRACTIQUE_POINTER=arrow or finger: a test run draws the pointer its steps lead, for a film of the app. The
// steps move:0.5:0.9 and click:0.5:0.9 lead it to a share of the window, a third number being the frames it takes.
pub struct Pointer {
    finger: bool,
    at: Option<egui::Pos2>,
    down: bool,
    way: Option<Way>,
}

impl Pointer {
    pub fn new() -> Option<Self> {
        let kind = std::env::var("FRACTIQUE_POINTER").ok()?;
        Some(Self {
            finger: kind == "finger",
            at: None,
            down: false,
            way: None,
        })
    }

    pub fn lead(&mut self, to: egui::Pos2, frames: u32, press: bool) {
        self.way = Some(Way {
            from: self.at.unwrap_or(to + egui::vec2(140.0, 110.0)),
            to,
            frames: frames.max(1),
            at: 0,
            press,
        });
    }

    pub fn events(&mut self, events: &mut Vec<egui::Event>) {
        let Some(way) = &mut self.way else {
            return;
        };
        way.at += 1;
        let share = (way.at as f32 / way.frames as f32).min(1.0);
        let pos = way.from + (way.to - way.from) * (share * share * (3.0 - 2.0 * share));
        events.push(egui::Event::PointerMoved(pos));
        let button = |pressed| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        let pressed_at = way.frames + REST;
        if way.press && way.at == pressed_at {
            events.push(button(true));
        } else if way.press && way.at == pressed_at + PRESS {
            events.push(button(false));
        }
        if way.at >= if way.press { pressed_at + PRESS } else { way.frames } {
            self.way = None;
        }
    }

    pub fn draw(&mut self, ctx: &egui::Context) {
        let (seen, down) = ctx.input(|input| (input.pointer.latest_pos(), input.pointer.any_down()));
        self.at = seen.or(self.at);
        self.down = down;
        let Some(at) = self.at else {
            return;
        };
        if self.way.is_some() {
            ctx.request_repaint();
        }
        let mut painter = ctx.layer_painter(egui::LayerId::new(
            egui::Order::Debug,
            egui::Id::unique("probe pointer"),
        ));
        painter.set_clip_rect(ctx.viewport_rect());
        if self.finger {
            let (radius, fill) = if down { (17.0, 120) } else { (21.0, 55) };
            painter.circle_filled(at, radius, egui::Color32::from_white_alpha(fill));
            painter.circle_stroke(at, radius, egui::Stroke::new(2.0, egui::Color32::from_white_alpha(210)));
            return;
        }
        let size = if down { 1.2 } else { 1.35 };
        let point = |x: f32, y: f32| at + egui::vec2(x, y) * size;
        let outline = vec![
            point(0.0, 0.0),
            point(0.0, 17.0),
            point(4.4, 13.2),
            point(7.6, 20.4),
            point(10.4, 19.2),
            point(7.3, 12.2),
            point(12.6, 12.2),
        ];
        let head = vec![point(0.0, 0.0), point(0.0, 17.0), point(12.6, 12.2)];
        let tail = vec![point(4.4, 13.2), point(7.6, 20.4), point(10.4, 19.2), point(7.3, 12.2)];
        let dark = egui::Stroke::new(1.6, egui::Color32::from_black_alpha(230));
        for part in [head, tail] {
            painter.add(egui::Shape::convex_polygon(
                part,
                egui::Color32::WHITE,
                egui::Stroke::NONE,
            ));
        }
        painter.add(egui::Shape::closed_line(outline, dark));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_test_window_is_maximized_and_made_full_screen_by_its_size() {
        let (screen, window) = (egui::vec2(1920.0, 1080.0), egui::vec2(1290.0, 690.0));
        let mut sham = Sham {
            screen,
            place: None,
            restored: egui::Vec2::ZERO,
            asked: None,
            maximized: false,
            full: false,
            kept: false,
        };
        let work = screen - egui::vec2(0.0, TASKBAR);
        assert_eq!(sham.maximize(true, window), work);
        assert_eq!(sham.origin(work), egui::Pos2::ZERO);
        // Full screen over a maximized window comes back to it maximized.
        assert_eq!(sham.fullscreen(true, work), screen);
        assert_eq!(sham.fullscreen(false, screen), work);
        assert_eq!(sham.maximize(false, work), window);
        assert_eq!(sham.fullscreen(true, window), screen);
        assert_eq!(sham.fullscreen(false, screen), window);
        assert_eq!(sham.origin(window), egui::pos2(315.0, 195.0));
        assert_eq!(sham.maximize(true, window), work);
        // Two steps sent in one frame: the window still has the size of before the first.
        assert_eq!(sham.maximize(false, work), window);
        sham.fullscreen(true, work);
        sham.fullscreen(false, screen);
        assert_eq!(sham.maximize(true, screen), work);
        assert_eq!(sham.maximize(false, work), window);
    }
}

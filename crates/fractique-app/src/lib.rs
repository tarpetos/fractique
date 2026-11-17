mod blur;
#[cfg(not(target_arch = "wasm32"))]
mod export;
mod glass;
mod i18n;
#[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
mod update;
#[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
mod updates;
mod widgets;

use std::path::PathBuf;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};

use web_time::Instant;

use eframe::egui;
use eframe::egui_wgpu::{self, wgpu};
use fractique_core::random::{mutate, random_scene};
use fractique_core::{Control, Image, RenderOptions, Scene, presets, render};

use glass::{Gfx, Optics};
use i18n::{Lang, Texts};
use widgets::{MEDIUM, SEMIBOLD, font};

const DRAFT_SIDE: f32 = 480.0;
const DRAFT_SAMPLES: u32 = 12;
const FULL_SIDE: f32 = if cfg!(target_arch = "wasm32") {
    900.0
} else if cfg!(any(target_os = "android", target_os = "ios")) {
    1600.0
} else {
    2560.0
};
const FULL_SAMPLES: u32 = if cfg!(target_arch = "wasm32") {
    14
} else if cfg!(any(target_os = "android", target_os = "ios")) {
    18
} else {
    32
};

struct Rendered {
    generation: u64,
    draft: bool,
    image: Image,
    soft: (Vec<u8>, u32, u32),
    seconds: f32,
}

#[derive(Clone, Copy, PartialEq)]
struct Adjust {
    brightness: f64,
    gamma: f64,
    glow: f64,
    zoom: f64,
}

const NEUTRAL: Adjust = Adjust {
    brightness: 1.0,
    gamma: 1.0,
    glow: 1.0,
    zoom: 1.0,
};

struct App {
    texts: Texts,
    base: Scene,
    title: String,
    is_random: bool,
    seed: u64,
    adjust: Adjust,
    optics: Optics,
    tuning: bool,
    inspector_open: Option<bool>,
    panes: Vec<(egui::Rect, f32)>,
    results: (mpsc::Sender<Rendered>, mpsc::Receiver<Rendered>),
    cancel: Arc<AtomicBool>,
    generation: u64,
    requested: Option<[u32; 2]>,
    rendering: bool,
    status: String,
    shown_frames: u32,
    screenshot: Option<PathBuf>,
    screenshot_done: Arc<AtomicBool>,
    last_frame: Instant,
    #[cfg(not(target_arch = "wasm32"))]
    export: export::Sheet,
    #[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
    updates: updates::Updates,
}

// Two cores stay free for the window: a render on every core makes it stutter.
#[cfg(not(target_arch = "wasm32"))]
fn render_pool() -> &'static rayon::ThreadPool {
    static POOL: OnceLock<rayon::ThreadPool> = OnceLock::new();
    POOL.get_or_init(|| {
        let cores = std::thread::available_parallelism().map_or(4, usize::from);
        let threads = cores.saturating_sub(2).max(1);
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .expect("the render threads start")
    })
}

fn fitted(size: [u32; 2], side: f32, samples: u32) -> RenderOptions {
    let shrink = (size[0].max(size[1]) as f32 / side).max(1.0);
    RenderOptions {
        width: ((size[0] as f32 / shrink) as u32).max(1),
        height: ((size[1] as f32 / shrink) as u32).max(1),
        samples_per_pixel: samples,
        supersample: 1,
    }
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>, screenshot: Option<PathBuf>) -> Self {
        let stored = |key: &str| cc.storage.and_then(|storage| storage.get_string(key));
        let lang = Lang::forced()
            .or_else(|| stored("language").map(|code| Lang::from_locale(&code)))
            .unwrap_or_else(Lang::system);
        let texts = Texts::new(lang);
        let glass = |key: &str, default: f32| stored(key).and_then(|text| text.parse().ok()).unwrap_or(default);
        let (opacity, blur) = (glass("glass_opacity", 0.4), glass("glass_blur", 0.7));
        widgets::install_fonts(&cc.egui_ctx, texts.lang);
        cc.egui_ctx.set_visuals(egui::Visuals::dark());
        if let Some(state) = &cc.wgpu_render_state {
            note(&format!("graphics: {:?}", state.adapter.get_info()));
            let gfx = Gfx::new(&state.device, state.target_format);
            state.renderer.write().callback_resources.insert(gfx);
        }
        let title = "aurora-spiral".to_owned();
        Self {
            texts,
            base: presets::get(&title).unwrap_or_else(|| random_scene(1)),
            title,
            is_random: false,
            seed: 0,
            adjust: NEUTRAL,
            optics: Optics {
                refraction: 38.0,
                blur,
                tint: opacity,
                rim: 1.0,
                radius: 30.0,
                bezel: 20.0,
            },
            tuning: false,
            inspector_open: None,
            panes: Vec::new(),
            results: mpsc::channel(),
            cancel: Arc::new(AtomicBool::new(false)),
            generation: 0,
            requested: None,
            rendering: false,
            status: String::new(),
            shown_frames: 0,
            screenshot,
            screenshot_done: Arc::new(AtomicBool::new(false)),
            last_frame: Instant::now(),
            #[cfg(not(target_arch = "wasm32"))]
            export: export::Sheet {
                open: std::env::var("FRACTIQUE_SHOW").is_ok_and(|what| what == "export"),
                ..Default::default()
            },
            #[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
            updates: updates::Updates::new(),
        }
    }

    fn scene(&self) -> Scene {
        let mut scene = self.base.clone();
        for layer in &mut scene.layers {
            let tone = &mut layer.flame.tone;
            tone.brightness *= self.adjust.brightness;
            tone.gamma *= self.adjust.gamma;
            tone.glow_strength *= self.adjust.glow;
            layer.flame.camera.zoom *= self.adjust.zoom;
        }
        scene
    }

    fn start_render(&mut self, ctx: &egui::Context, size: [u32; 2]) {
        self.cancel.store(true, Ordering::Relaxed);
        self.cancel = Arc::new(AtomicBool::new(false));
        self.generation += 1;
        self.requested = Some(size);
        self.rendering = true;

        let full = fitted(size, FULL_SIDE, FULL_SAMPLES);
        let mut passes = vec![(true, fitted(size, DRAFT_SIDE, DRAFT_SAMPLES)), (false, full)];
        // A browser page has one thread: a draft first would only make it wait twice.
        if passes[0].1.width == full.width || cfg!(target_arch = "wasm32") {
            passes.remove(0);
        }
        let (scene, cancel, generation) = (self.scene(), self.cancel.clone(), self.generation);
        let (sender, ctx) = (self.results.0.clone(), ctx.clone());
        let work = move || {
            for (draft, options) in passes {
                let started = Instant::now();
                let control = Control {
                    cancel: Some(&cancel),
                    progress: None,
                };
                #[cfg(not(target_arch = "wasm32"))]
                let result = render_pool().install(|| render(&scene, &options, &control));
                #[cfg(target_arch = "wasm32")]
                let result = render(&scene, &options, &control);
                let Ok(image) = result else {
                    return;
                };
                let radius = image.width.min(image.height) as f32 * 0.03;
                let soft = blur::soften(&image.rgba, image.width as usize, image.height as usize, radius);
                let seconds = started.elapsed().as_secs_f32();
                let done = Rendered {
                    generation,
                    draft,
                    image,
                    soft,
                    seconds,
                };
                if sender.send(done).is_err() {
                    return;
                }
                ctx.request_repaint();
            }
        };
        #[cfg(not(target_arch = "wasm32"))]
        std::thread::spawn(work);
        #[cfg(target_arch = "wasm32")]
        work();
    }

    fn receive(&mut self, frame: &eframe::Frame) {
        while let Ok(done) = self.results.1.try_recv() {
            if done.generation != self.generation {
                continue;
            }
            if let Some(state) = frame.wgpu_render_state()
                && let Some(gfx) = state.renderer.write().callback_resources.get_mut::<Gfx>()
            {
                let image = &done.image;
                let soft = (done.soft.0.as_slice(), done.soft.1, done.soft.2);
                gfx.set_images(
                    &state.device,
                    &state.queue,
                    (&image.rgba, image.width, image.height),
                    soft,
                );
            }
            if !done.draft {
                self.rendering = false;
                self.shown_frames = self.shown_frames.max(1);
                self.status = format!("{} × {}  ·  {:.2} s", done.image.width, done.image.height, done.seconds);
            }
        }
    }

    fn pick(&mut self, scene: Scene, title: String, is_random: bool) {
        self.base = scene;
        self.title = title;
        self.is_random = is_random;
        self.requested = None;
    }

    fn t(&self, key: &str) -> String {
        self.texts.get(key)
    }

    fn inspector(&mut self, ui: &mut egui::Ui, width: f32) {
        ui.set_width(width);
        ui.spacing_mut().item_spacing = egui::vec2(8.0, 2.0);
        ui.label(
            egui::RichText::new("Fractique")
                .font(font(30.0, SEMIBOLD))
                .color(egui::Color32::WHITE),
        );
        ui.label(
            egui::RichText::new(self.t("tagline"))
                .size(12.5)
                .color(egui::Color32::from_white_alpha(140)),
        );

        widgets::section(ui, &self.t("presets"));
        if self.is_random {
            widgets::row(ui, &self.title.clone(), true);
        }
        for name in presets::names() {
            if widgets::row(ui, name, self.title == name)
                && let Some(scene) = presets::get(name)
            {
                self.pick(scene, name.to_owned(), false);
            }
        }

        widgets::section(ui, &self.t("tone"));
        let before = self.adjust;
        widgets::slider(ui, &self.t("brightness"), &mut self.adjust.brightness, (0.3, 2.5), true);
        widgets::slider(ui, &self.t("gamma"), &mut self.adjust.gamma, (0.5, 1.8), false);
        widgets::slider(ui, &self.t("glow"), &mut self.adjust.glow, (0.0, 3.0), false);
        widgets::section(ui, &self.t("view"));
        widgets::slider(ui, &self.t("zoom"), &mut self.adjust.zoom, (0.4, 2.5), true);

        ui.add_space(8.0);
        if widgets::pill(ui, &self.t("reset"), false, 0.0) {
            self.adjust = NEUTRAL;
        }

        widgets::section(ui, &self.t("glass"));
        let mut opacity = f64::from(self.optics.tint);
        widgets::slider(ui, &self.t("opacity"), &mut opacity, (0.0, 1.0), false);
        self.optics.tint = opacity as f32;
        let mut blur = f64::from(self.optics.blur);
        widgets::slider(ui, &self.t("blur"), &mut blur, (0.0, 1.0), false);
        self.optics.blur = blur as f32;
        if self.adjust != before {
            self.requested = None;
        }

        #[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
        {
            widgets::section(ui, &self.t("scene"));
            ui.add_space(4.0);
            let (open, save) = (self.t("open"), self.t("save"));
            let half = (ui.available_width() - 8.0) * 0.5;
            let mut clicked = (false, false);
            ui.horizontal(|ui| {
                clicked.0 = widgets::pill(ui, &open, false, half);
                clicked.1 = widgets::pill(ui, &save, false, half);
            });
            if clicked.0 {
                self.open_scene();
            }
            if clicked.1 {
                self.save_scene();
            }
        }

        widgets::section(ui, &self.t("language"));
        let mut chosen = self.texts.lang;
        egui::ComboBox::from_id_salt("language")
            .width(ui.available_width())
            .selected_text(chosen.name())
            .show_ui(ui, |ui| {
                for lang in Lang::ALL {
                    ui.selectable_value(&mut chosen, lang, lang.name());
                }
            });
        if chosen != self.texts.lang {
            self.texts = Texts::new(chosen);
            widgets::install_fonts(ui.ctx(), chosen);
        }

        #[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
        {
            ui.add_space(10.0);
            let label = match self.updates.newer() {
                Some(version) => self.t("available").replace("{v}", version),
                None => format!("{} · v{}", self.t("updates"), update::current()),
            };
            if widgets::row(ui, &label, self.updates.newer().is_some()) {
                self.updates.show(ui.ctx());
            }
        }
    }

    fn toolbar(&mut self, ui: &mut egui::Ui, wide: bool, room: f32) {
        let gap = if wide { 8.0 } else { 6.0 };
        ui.spacing_mut().item_spacing = egui::vec2(gap, gap);
        ui.spacing_mut().button_padding.x = if wide { 18.0 } else { 12.0 };
        let keys: &[&str] = if cfg!(target_arch = "wasm32") {
            &["randomize", "mutate", "controls"]
        } else {
            &["randomize", "mutate", "export", "controls"]
        };
        let labels: Vec<String> = keys.iter().map(|key| self.t(key)).collect();
        let natural: f32 = labels.iter().map(|label| widgets::pill_width(ui, label)).sum();
        let one_row = natural + (labels.len() - 1) as f32 * gap <= room;
        let width = if one_row { 0.0 } else { (room - gap) * 0.5 };

        let mut clicked = None;
        let mut row = |ui: &mut egui::Ui, buttons: std::ops::Range<usize>| {
            ui.horizontal(|ui| {
                for index in buttons {
                    if widgets::pill(ui, &labels[index], index == 0, width) {
                        clicked = Some(keys[index]);
                    }
                }
            });
        };
        if one_row {
            row(ui, 0..labels.len());
        } else {
            row(ui, 0..2);
            row(ui, 2..labels.len());
        }

        match clicked {
            Some("randomize") => {
                self.seed += 1;
                let title = format!("{} {}", self.t("random"), self.seed);
                self.pick(random_scene(self.seed), title, true);
            }
            Some("mutate") => {
                self.seed += 1;
                let scene = mutate(&self.base, self.seed, 0.25);
                let (title, is_random) = (self.title.clone(), self.is_random);
                self.pick(scene, title, is_random);
            }
            #[cfg(not(target_arch = "wasm32"))]
            Some("export") => {
                self.export.open = !self.export.open;
                self.export.note.clear();
            }
            Some("controls") => self.inspector_open = Some(!self.inspector_open.unwrap_or(false)),
            _ => {}
        }
    }

    #[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
    fn open_scene(&mut self) {
        let picked = rfd::FileDialog::new()
            .add_filter("Fractique, flam3", &["json", "flame"])
            .pick_file();
        let Some(path) = picked else { return };
        let name = path
            .file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
            .unwrap_or_default();
        let is_flame = path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("flame"));
        let loaded = std::fs::read_to_string(&path)
            .map_err(|error| error.to_string())
            .and_then(|text| {
                if is_flame {
                    let mut flames = fractique_core::import::import_flames(&text).map_err(|error| error.to_string())?;
                    Ok(flames.swap_remove(0).scene)
                } else {
                    Scene::from_json(&text).map_err(|error| error.to_string())
                }
            });
        match loaded {
            Ok(scene) => {
                self.adjust = NEUTRAL;
                self.pick(scene, name, false);
            }
            Err(error) => self.status = error,
        }
    }

    #[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
    fn save_scene(&mut self) {
        let name = format!("{}.json", self.title.replace(' ', "-"));
        let Some(path) = rfd::FileDialog::new()
            .set_file_name(name)
            .add_filter("Fractique", &["json"])
            .save_file()
        else {
            return;
        };
        self.status = match std::fs::write(&path, self.scene().to_json() + "\n") {
            Ok(()) => self.t("saved"),
            Err(error) => format!("{}: {error}", self.t("failed")),
        };
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn export_sheet(&mut self, ui: &mut egui::Ui, window: [u32; 2]) {
        ui.set_width(250.0);
        ui.spacing_mut().item_spacing = egui::vec2(8.0, 2.0);
        ui.label(
            egui::RichText::new(self.t("export_title"))
                .font(font(17.0, SEMIBOLD))
                .color(egui::Color32::WHITE),
        );

        if let Some(job) = &self.export.job {
            let share = job.share();
            ui.add_space(14.0);
            let (bar, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 6.0), egui::Sense::hover());
            let done = egui::Rect::from_min_size(bar.min, egui::vec2(bar.width() * share, bar.height()));
            ui.painter()
                .rect_filled(bar, egui::CornerRadius::same(3), egui::Color32::from_white_alpha(45));
            ui.painter()
                .rect_filled(done, egui::CornerRadius::same(3), egui::Color32::from_white_alpha(225));
            ui.add_space(8.0);
            ui.label(format!("{} · {:.0}%", self.t("exporting"), share * 100.0));
            ui.add_space(12.0);
            if widgets::pill(ui, &self.t("cancel"), false, ui.available_width()) {
                job.cancel();
            }
            return;
        }

        widgets::section(ui, &self.t("size"));
        let own = format!("{} · {} × {}", self.t("size_window"), window[0], window[1]);
        if widgets::row(ui, &own, self.export.size == 0) {
            self.export.size = 0;
        }
        for (index, [width, height]) in export::SIZES.into_iter().enumerate() {
            if widgets::row(ui, &format!("{width} × {height}"), self.export.size == index + 1) {
                self.export.size = index + 1;
            }
        }
        widgets::section(ui, &self.t("quality"));
        for (index, (key, _, _)) in export::QUALITIES.into_iter().enumerate() {
            if widgets::row(ui, &self.t(key), self.export.quality == index) {
                self.export.quality = index;
            }
        }
        if !self.export.note.is_empty() {
            ui.add_space(8.0);
            ui.label(
                egui::RichText::new(&self.export.note)
                    .size(12.5)
                    .color(egui::Color32::from_white_alpha(190)),
            );
        }

        ui.add_space(12.0);
        let (cancel, save) = (self.t("cancel"), self.t("save_png"));
        let half = (ui.available_width() - 8.0) * 0.5;
        let mut clicked = (false, false);
        ui.horizontal(|ui| {
            clicked.0 = widgets::pill(ui, &cancel, false, half);
            clicked.1 = widgets::pill(ui, &save, true, half);
        });
        if clicked.0 {
            self.export.open = false;
        }
        if clicked.1 {
            let [width, height] = if self.export.size == 0 {
                window
            } else {
                export::SIZES[self.export.size - 1]
            };
            let (_, samples_per_pixel, supersample) = export::QUALITIES[self.export.quality];
            let options = RenderOptions {
                width,
                height,
                samples_per_pixel,
                supersample,
            };
            let name = format!("{}-{width}x{height}.png", self.title.replace(' ', "-"));
            if let Some(path) = export::target(&name) {
                self.export.job = Some(export::Job::start(
                    self.scene(),
                    options,
                    path,
                    ui.ctx().clone(),
                    render_pool(),
                ));
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn finish_export(&mut self) {
        let Some(outcome) = self.export.job.as_ref().and_then(export::Job::finished) else {
            return;
        };
        self.export.job = None;
        match outcome {
            export::Outcome::Saved(path) => {
                let name = path
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_default();
                self.export.note = format!("{}: {name}", self.t("saved"));
            }
            export::Outcome::Cancelled => self.export.note.clear(),
            export::Outcome::Failed(error) => self.export.note = format!("{}: {error}", self.t("failed")),
        }
    }

    fn tuning(&mut self, ui: &mut egui::Ui) {
        ui.set_width(220.0);
        ui.spacing_mut().item_spacing = egui::vec2(8.0, 2.0);
        ui.label(egui::RichText::new(self.t("glass_title")).font(font(15.0, SEMIBOLD)));
        let labels = ["refraction", "depth", "bezel"].map(|key| self.t(key));
        let optics = &mut self.optics;
        let edit = |ui: &mut egui::Ui, label: &str, value: &mut f32, range: (f64, f64)| {
            let mut wide = f64::from(*value);
            widgets::slider(ui, label, &mut wide, range, false);
            *value = wide as f32;
        };
        edit(ui, &labels[0], &mut optics.refraction, (-80.0, 80.0));
        edit(ui, &labels[1], &mut optics.rim, (0.0, 2.0));
        edit(ui, &labels[2], &mut optics.bezel, (4.0, 48.0));
    }

    fn take_screenshot(&mut self, ctx: &egui::Context) {
        if self.screenshot_done.load(Ordering::Relaxed) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }
        if self.shown_frames == 0 {
            return;
        }
        self.shown_frames += 1;
        ctx.request_repaint();
        if self.shown_frames == 6
            && let Some(path) = self.screenshot.clone()
        {
            let (done, ctx_done) = (self.screenshot_done.clone(), ctx.clone());
            let callback = egui::ScreenshotCallback::new(move |image: Arc<egui::ColorImage>| {
                if let Err(error) = save_png(&path, image.as_raw(), image.size) {
                    eprintln!("fractique: {}: {error}", path.display());
                }
                done.store(true, Ordering::Relaxed);
                ctx_done.request_repaint();
            });
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(callback));
        }
    }
}

fn save_png(path: &std::path::Path, rgba: &[u8], size: [usize; 2]) -> Result<(), String> {
    let file = std::fs::File::create(path).map_err(|e| e.to_string())?;
    let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), size[0] as u32, size[1] as u32);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
    writer.write_image_data(rgba).map_err(|e| e.to_string())?;
    writer.finish().map_err(|e| e.to_string())
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        // Every mouse move asks for a redraw; at a 240 Hz screen's pace that makes a graphics card whine.
        #[cfg(not(target_arch = "wasm32"))]
        {
            let pace = std::time::Duration::from_micros(16_600);
            let since = self.last_frame.elapsed();
            if since < pace {
                std::thread::sleep(pace - since);
            }
            self.last_frame = Instant::now();
        }
        self.receive(frame);
        #[cfg(not(target_arch = "wasm32"))]
        self.finish_export();
        #[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
        self.updates.settle(&ctx);
        if ctx.input(|input| input.key_pressed(egui::Key::F2)) {
            self.tuning = !self.tuning;
        }

        let screen = ctx.viewport_rect();
        let content = safe_content(&ctx);
        let scale = ctx.pixels_per_point();
        let size = [
            (screen.width() * scale).round() as u32,
            (screen.height() * scale).round() as u32,
        ];
        if size[0] > 0 && size[1] > 0 && self.requested != Some(size) {
            self.start_render(&ctx, size);
        }

        let painter = ctx.layer_painter(egui::LayerId::background()).with_clip_rect(screen);
        if std::env::var_os("FRACTIQUE_DEBUG_LAYOUT").is_some() {
            let ruler = ctx.layer_painter(egui::LayerId::debug()).with_clip_rect(screen);
            let mut y = 0.0;
            while y <= screen.bottom() {
                let color = egui::Color32::from_rgb(255, 60, 60);
                ruler.hline(screen.x_range(), y, egui::Stroke::new(1.0, color));
                ruler.text(
                    egui::pos2(6.0, y),
                    egui::Align2::LEFT_BOTTOM,
                    format!("{y}"),
                    egui::FontId::proportional(11.0),
                    color,
                );
                y += 100.0;
            }
            if self.shown_frames == 1 {
                eprintln!("layout: viewport {screen:?} content {content:?} scale {scale} pixels {size:?}");
                eprintln!("layout: window {:?}", ctx.input(|input| input.viewport().clone()));
                self.shown_frames = 2;
            }
        }
        let mut glass = glass::Frame::new(&painter, screen);
        for (rect, radius) in &self.panes {
            glass.pane(
                *rect,
                Optics {
                    radius: *radius,
                    ..self.optics
                },
            );
        }

        let wide = content.width() >= 700.0;
        let open = *self.inspector_open.get_or_insert(wide);
        #[cfg(not(target_arch = "wasm32"))]
        let open = open && (wide || !self.export.open);
        let mut panes = Vec::new();
        if open {
            let corner = content.min + egui::vec2(20.0, 20.0);
            let inspector = egui::Area::new(egui::Id::unique("inspector"))
                .fixed_pos(corner)
                .show(&ctx, |ui| {
                    egui::Frame::NONE.inner_margin(egui::Margin::same(22)).show(ui, |ui| {
                        let room = content.height() - 40.0 - 44.0 - if wide { 0.0 } else { 80.0 };
                        egui::ScrollArea::vertical()
                            .max_height(room)
                            .min_scrolled_height(room)
                            .show(ui, |ui| self.inspector(ui, (content.width() - 84.0).min(248.0)));
                    });
                });
            panes.push((inspector.response.rect, self.optics.radius));
        }

        let toolbar = egui::Area::new(egui::Id::unique("toolbar"))
            .fixed_pos(egui::pos2(content.center().x, content.bottom() - 20.0))
            .pivot(egui::Align2::CENTER_BOTTOM)
            .show(&ctx, |ui| {
                egui::Frame::NONE
                    .inner_margin(egui::Margin::same(9))
                    .show(ui, |ui| self.toolbar(ui, wide, content.width() - 58.0));
            });
        panes.push((toolbar.response.rect, (toolbar.response.rect.height() * 0.5).min(28.0)));

        if wide || !open {
            let status = egui::Area::new(egui::Id::unique("status"))
                .fixed_pos(egui::pos2(content.right() - 20.0, content.top() + 20.0))
                .pivot(egui::Align2::RIGHT_TOP)
                .show(&ctx, |ui| {
                    egui::Frame::NONE
                        .inner_margin(egui::Margin::symmetric(18, 11))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                let name = egui::RichText::new(&self.title)
                                    .font(font(13.5, MEDIUM))
                                    .color(egui::Color32::WHITE);
                                ui.add(egui::Label::new(name).extend());
                                let detail = if self.rendering {
                                    self.t("rendering")
                                } else {
                                    self.status.clone()
                                };
                                let detail = egui::RichText::new(detail)
                                    .size(13.0)
                                    .color(egui::Color32::from_white_alpha(170));
                                ui.add(egui::Label::new(detail).extend());
                            });
                        });
                });
            panes.push((status.response.rect, status.response.rect.height() * 0.5));
        }

        #[cfg(not(target_arch = "wasm32"))]
        if self.export.open {
            let sheet = egui::Area::new(egui::Id::unique("export"))
                .fixed_pos(content.center())
                .pivot(egui::Align2::CENTER_CENTER)
                .order(egui::Order::Foreground)
                .show(&ctx, |ui| {
                    egui::Frame::NONE
                        .inner_margin(egui::Margin::same(22))
                        .show(ui, |ui| self.export_sheet(ui, size));
                });
            panes.push((sheet.response.rect, self.optics.radius));
        }

        #[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
        if self.updates.open {
            let width = (content.width() - 84.0).min(600.0);
            let sheet = egui::Area::new(egui::Id::unique("updates"))
                .fixed_pos(content.center())
                .pivot(egui::Align2::CENTER_CENTER)
                .order(egui::Order::Foreground)
                .show(&ctx, |ui| {
                    egui::Frame::NONE
                        .inner_margin(egui::Margin::same(22))
                        .show(ui, |ui| self.updates.ui(ui, &self.texts, width));
                });
            panes.push((sheet.response.rect, self.optics.radius));
        }

        if self.tuning {
            let tuning = egui::Area::new(egui::Id::unique("tuning"))
                .fixed_pos(egui::pos2(content.right() - 20.0, content.bottom() - 20.0))
                .pivot(egui::Align2::RIGHT_BOTTOM)
                .show(&ctx, |ui| {
                    egui::Frame::NONE
                        .inner_margin(egui::Margin::same(20))
                        .show(ui, |ui| self.tuning(ui));
                });
            panes.push((tuning.response.rect, self.optics.radius));
        }

        // The panes are drawn under the widgets, so they use the places measured one frame earlier.
        if panes != self.panes {
            self.panes = panes;
            ctx.request_repaint();
        }
        if self.screenshot.is_some() {
            self.take_screenshot(&ctx);
        }
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        storage.set_string("language", self.texts.lang.code().to_owned());
        storage.set_string("glass_opacity", self.optics.tint.to_string());
        storage.set_string("glass_blur", self.optics.blur.to_string());
    }

    #[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
    fn on_exit(&mut self) {
        self.updates.on_exit();
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.012, 0.014, 0.045, 1.0]
    }
}

#[cfg(target_os = "android")]
static ANDROID: OnceLock<winit::platform::android::activity::AndroidApp> = OnceLock::new();

#[cfg(target_os = "android")]
fn own_folder() -> Option<PathBuf> {
    ANDROID.get().and_then(|app| app.external_data_path())
}

#[cfg(target_os = "ios")]
fn own_folder() -> Option<PathBuf> {
    std::env::var_os("HOME").map(|home| PathBuf::from(home).join("Documents"))
}

fn safe_content(ctx: &egui::Context) -> egui::Rect {
    // egui knows the safe area on iOS; on Android the activity reports it, in pixels.
    #[cfg(target_os = "android")]
    if let Some(app) = ANDROID.get() {
        let (free, scale) = (app.content_rect(), ctx.pixels_per_point());
        let rect = egui::Rect::from_min_max(
            egui::pos2(free.left as f32 / scale, free.top as f32 / scale),
            egui::pos2(free.right as f32 / scale, free.bottom as f32 / scale),
        );
        if rect.width() > 1.0 && rect.height() > 1.0 {
            return rect.intersect(ctx.viewport_rect());
        }
    }
    ctx.content_rect()
}

pub fn window_icon() -> egui::IconData {
    let decoder = png::Decoder::new(std::io::Cursor::new(
        include_bytes!("../../../assets/icon/icon-256.png").as_slice(),
    ));
    let mut reader = decoder.read_info().expect("the icon is a PNG");
    let mut rgba = vec![0; reader.output_buffer_size().expect("the icon fits in memory")];
    let info = reader.next_frame(&mut rgba).expect("the icon decodes");
    rgba.truncate(info.buffer_size());
    egui::IconData {
        rgba,
        width: info.width,
        height: info.height,
    }
}

#[cfg(not(target_arch = "wasm32"))]
struct Problems;

#[cfg(not(target_arch = "wasm32"))]
impl log::Log for Problems {
    fn enabled(&self, metadata: &log::Metadata<'_>) -> bool {
        metadata.level() <= log::Level::Warn
    }

    fn log(&self, record: &log::Record<'_>) {
        if self.enabled(record.metadata()) {
            let line = format!("{}: {}: {}", record.level(), record.target(), record.args());
            eprintln!("{line}");
            note(&line);
        }
    }

    fn flush(&self) {}
}

fn offered_limits(wgpu_options: &mut egui_wgpu::WgpuConfiguration) {
    if let egui_wgpu::WgpuSetup::CreateNew(setup) = &mut wgpu_options.wgpu_setup {
        // egui asks every GPU for fixed limits; an older phone or the iOS Simulator offers less and refuses to start.
        setup.device_descriptor = Arc::new(|adapter| wgpu::DeviceDescriptor {
            label: Some("fractique device"),
            required_limits: adapter.limits(),
            ..Default::default()
        });
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn run(mut options: eframe::NativeOptions, screenshot: Option<PathBuf>) -> eframe::Result {
    // eframe reports why it cannot start through the log crate and then exits with code 0.
    if log::set_logger(&Problems).is_ok() {
        log::set_max_level(log::LevelFilter::Warn);
    }
    offered_limits(&mut options.wgpu_options);
    eframe::run_native(
        "Fractique",
        options,
        Box::new(move |cc| Ok(Box::new(App::new(cc, screenshot)))),
    )
}

#[cfg(target_arch = "wasm32")]
pub async fn run_web(canvas: eframe::web_sys::HtmlCanvasElement) -> Result<(), eframe::wasm_bindgen::JsValue> {
    let _ = eframe::WebLogger::init(log::LevelFilter::Warn);
    let mut options = eframe::WebOptions::default();
    offered_limits(&mut options.wgpu_options);
    let runner = eframe::WebRunner::new();
    runner
        .start(canvas, options, Box::new(|cc| Ok(Box::new(App::new(cc, None)))))
        .await
}

#[cfg(target_os = "android")]
static LOG: OnceLock<PathBuf> = OnceLock::new();

#[cfg(target_os = "android")]
fn note(text: &str) {
    use std::io::Write;
    if let Some(path) = LOG.get()
        && let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(path)
    {
        let _ = writeln!(file, "{text}");
    }
}

#[cfg(not(target_os = "android"))]
fn note(_text: &str) {}

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
fn android_main(app: winit::platform::android::activity::AndroidApp) {
    use winit::platform::android::activity::AndroidApp;
    let _ = ANDROID.set(app.clone());
    // A phone shows no console: what stops the app is left in a file its owner can copy off the phone.
    if let Some(folder) = app.external_data_path() {
        let _ = std::fs::create_dir_all(&folder);
        let path = folder.join("fractique-log.txt");
        let _ = std::fs::remove_file(&path);
        let _ = LOG.set(path);
    }
    let sdk = AndroidApp::sdk_version();
    note(&format!("started, Android SDK {sdk}"));
    std::panic::set_hook(Box::new(|info| note(&format!("panic: {info}"))));
    // Vulkan drivers of phones older than Android 11 crash inside the driver; OpenGL ES works on them.
    if sdk < 30 {
        // SAFETY: no other thread exists yet.
        unsafe { std::env::set_var("WGPU_BACKEND", "gl") };
        note("using OpenGL ES");
    }
    let options = eframe::NativeOptions {
        android_app: Some(app),
        ..Default::default()
    };
    match run(options, None) {
        Ok(()) => note("closed"),
        Err(error) => note(&format!("could not start: {error}")),
    }
}

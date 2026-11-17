mod blur;
mod glass;
mod widgets;

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock, mpsc};
use std::time::Instant;

use eframe::egui;
use eframe::egui_wgpu::{self, wgpu};
use fractique_core::random::{mutate, random_scene};
use fractique_core::{Control, Image, RenderOptions, Scene, presets, render};

use glass::{Gfx, Optics};
use widgets::{MEDIUM, SEMIBOLD, font};

const DRAFT_SIDE: f32 = 480.0;
const DRAFT_SAMPLES: u32 = 12;
const FULL_SIDE: f32 = if cfg!(any(target_os = "android", target_os = "ios")) {
    1600.0
} else {
    2560.0
};
const FULL_SAMPLES: u32 = if cfg!(any(target_os = "android", target_os = "ios")) {
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
    base: Scene,
    title: String,
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
}

// Two cores stay free for the window: a render on every core makes it stutter.
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
        widgets::install_fonts(&cc.egui_ctx);
        cc.egui_ctx.set_visuals(egui::Visuals::dark());
        if let Some(state) = &cc.wgpu_render_state {
            note(&format!("graphics: {:?}", state.adapter.get_info()));
            let gfx = Gfx::new(&state.device, state.target_format);
            state.renderer.write().callback_resources.insert(gfx);
        }
        let title = "aurora-spiral".to_owned();
        Self {
            base: presets::get(&title).unwrap_or_else(|| random_scene(1)),
            title,
            seed: 0,
            adjust: NEUTRAL,
            optics: Optics {
                refraction: 38.0,
                blur: 0.7,
                tint: 0.4,
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
        if passes[0].1.width == full.width {
            passes.remove(0);
        }
        let (scene, cancel, generation) = (self.scene(), self.cancel.clone(), self.generation);
        let (sender, ctx) = (self.results.0.clone(), ctx.clone());
        std::thread::spawn(move || {
            for (draft, options) in passes {
                let started = Instant::now();
                let control = Control {
                    cancel: Some(&cancel),
                    progress: None,
                };
                let Ok(image) = render_pool().install(|| render(&scene, &options, &control)) else {
                    return;
                };
                let radius = image.width.min(image.height) as f32 * 0.03;
                let soft = blur::soften(&image.rgba, image.width as usize, image.height as usize, radius);
                let seconds = started.elapsed().as_secs_f32();
                if sender
                    .send(Rendered {
                        generation,
                        draft,
                        image,
                        soft,
                        seconds,
                    })
                    .is_err()
                {
                    return;
                }
                ctx.request_repaint();
            }
        });
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

    fn pick(&mut self, scene: Scene, title: String) {
        self.base = scene;
        self.title = title;
        self.requested = None;
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
            egui::RichText::new("fractal wallpapers")
                .size(12.5)
                .color(egui::Color32::from_white_alpha(140)),
        );

        widgets::section(ui, "PRESETS");
        if self.title.starts_with("random") {
            widgets::row(ui, &self.title.clone(), true);
        }
        for name in presets::names() {
            if widgets::row(ui, name, self.title == name)
                && let Some(scene) = presets::get(name)
            {
                self.pick(scene, name.to_owned());
            }
        }

        widgets::section(ui, "TONE");
        let before = self.adjust;
        widgets::slider(ui, "Brightness", &mut self.adjust.brightness, (0.3, 2.5), true);
        widgets::slider(ui, "Gamma", &mut self.adjust.gamma, (0.5, 1.8), false);
        widgets::slider(ui, "Glow", &mut self.adjust.glow, (0.0, 3.0), false);
        widgets::section(ui, "VIEW");
        widgets::slider(ui, "Zoom", &mut self.adjust.zoom, (0.4, 2.5), true);

        widgets::section(ui, "GLASS");
        let mut opacity = f64::from(self.optics.tint);
        widgets::slider(ui, "Opacity", &mut opacity, (0.0, 1.0), false);
        self.optics.tint = opacity as f32;
        let mut blur = f64::from(self.optics.blur);
        widgets::slider(ui, "Blur", &mut blur, (0.0, 1.0), false);
        self.optics.blur = blur as f32;
        if self.adjust != before {
            self.requested = None;
        }
    }

    fn toolbar(&mut self, ui: &mut egui::Ui, wide: bool) {
        ui.spacing_mut().item_spacing.x = if wide { 8.0 } else { 5.0 };
        ui.spacing_mut().button_padding.x = if wide { 18.0 } else { 12.0 };
        ui.horizontal(|ui| {
            if widgets::pill(ui, "Randomize", true) {
                self.seed += 1;
                self.pick(random_scene(self.seed), format!("random {}", self.seed));
            }
            if widgets::pill(ui, "Mutate", false) {
                self.seed += 1;
                let scene = mutate(&self.base, self.seed, 0.25);
                let title = self.title.clone();
                self.pick(scene, title);
            }
            if widgets::pill(ui, "Controls", false) {
                self.inspector_open = Some(!self.inspector_open.unwrap_or(false));
            }
            if widgets::pill(ui, "Reset", false) {
                self.adjust = NEUTRAL;
                self.requested = None;
            }
        });
    }

    fn tuning(&mut self, ui: &mut egui::Ui) {
        ui.set_width(220.0);
        ui.spacing_mut().item_spacing = egui::vec2(8.0, 2.0);
        ui.label(egui::RichText::new("Glass").font(font(15.0, SEMIBOLD)));
        let optics = &mut self.optics;
        let edit = |ui: &mut egui::Ui, label: &str, value: &mut f32, range: (f64, f64)| {
            let mut wide = f64::from(*value);
            widgets::slider(ui, label, &mut wide, range, false);
            *value = wide as f32;
        };
        edit(ui, "Refraction", &mut optics.refraction, (-80.0, 80.0));
        edit(ui, "Depth", &mut optics.rim, (0.0, 2.0));
        edit(ui, "Bezel", &mut optics.bezel, (4.0, 48.0));
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
        self.receive(frame);
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

        let painter = ctx.layer_painter(egui::LayerId::background());
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
                    .show(ui, |ui| self.toolbar(ui, wide));
            });
        panes.push((toolbar.response.rect, toolbar.response.rect.height() * 0.5));

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
                                    "rendering".to_owned()
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

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.012, 0.014, 0.045, 1.0]
    }
}

#[cfg(target_os = "android")]
static ANDROID: OnceLock<winit::platform::android::activity::AndroidApp> = OnceLock::new();

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

struct Problems;

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

pub fn run(mut options: eframe::NativeOptions, screenshot: Option<PathBuf>) -> eframe::Result {
    // eframe reports why it cannot start through the log crate and then exits with code 0.
    if log::set_logger(&Problems).is_ok() {
        log::set_max_level(log::LevelFilter::Warn);
    }
    if let egui_wgpu::WgpuSetup::CreateNew(setup) = &mut options.wgpu_options.wgpu_setup {
        // egui asks every GPU for 8192-pixel textures; older phones offer 4096 and refuse to start.
        setup.device_descriptor = Arc::new(|adapter| {
            let base = if adapter.get_info().backend == wgpu::Backend::Gl {
                wgpu::Limits::downlevel_webgl2_defaults()
            } else {
                wgpu::Limits::default()
            };
            wgpu::DeviceDescriptor {
                label: Some("fractique device"),
                required_limits: wgpu::Limits {
                    max_texture_dimension_2d: adapter.limits().max_texture_dimension_2d.min(8192),
                    ..base
                },
                ..Default::default()
            }
        });
    }
    eframe::run_native(
        "Fractique",
        options,
        Box::new(move |cc| Ok(Box::new(App::new(cc, screenshot)))),
    )
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

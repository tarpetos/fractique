mod blur;
mod glass;

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::time::Instant;

use eframe::egui;
use fractique_core::random::random_scene;
use fractique_core::{Control, Image, RenderOptions, Scene, presets, render};

use glass::{Gfx, Optics};

const PREVIEW_SAMPLES: u32 = 30;
// Above this the interactive preview is rendered smaller and stretched: it keeps the sliders responsive.
const PREVIEW_SIDE: u32 = 2048;

struct Rendered {
    generation: u64,
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

struct App {
    base: Scene,
    title: String,
    seed: u64,
    adjust: Adjust,
    optics: Optics,
    panes: Vec<egui::Rect>,
    results: (mpsc::Sender<Rendered>, mpsc::Receiver<Rendered>),
    cancel: Arc<AtomicBool>,
    generation: u64,
    requested: Option<([u32; 2], Adjust)>,
    rendering: bool,
    status: String,
    shown_frames: u32,
    screenshot: Option<PathBuf>,
    screenshot_done: Arc<AtomicBool>,
}

fn style(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.override_text_color = Some(egui::Color32::from_white_alpha(235));
    visuals.slider_trailing_fill = true;
    visuals.selection.bg_fill = egui::Color32::from_rgb(70, 205, 190);
    visuals.window_fill = egui::Color32::from_rgba_unmultiplied(22, 26, 44, 240);
    visuals.window_stroke = egui::Stroke::new(1.0, egui::Color32::from_white_alpha(40));
    visuals.extreme_bg_color = egui::Color32::from_black_alpha(70);
    for (widget, alpha) in [
        (&mut visuals.widgets.inactive, 30),
        (&mut visuals.widgets.hovered, 55),
        (&mut visuals.widgets.active, 80),
        (&mut visuals.widgets.open, 55),
    ] {
        widget.bg_fill = egui::Color32::from_white_alpha(alpha);
        widget.weak_bg_fill = egui::Color32::from_white_alpha(alpha);
        widget.bg_stroke = egui::Stroke::NONE;
        widget.corner_radius = egui::CornerRadius::same(10);
        widget.fg_stroke = egui::Stroke::new(1.5, egui::Color32::from_white_alpha(240));
    }
    ctx.set_visuals(visuals);
    ctx.global_style_mut(|style| {
        style.spacing.item_spacing = egui::vec2(10.0, 10.0);
        style.spacing.button_padding = egui::vec2(12.0, 7.0);
        style.spacing.slider_width = 150.0;
        style.spacing.interact_size.y = 26.0;
    });
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>, screenshot: Option<PathBuf>) -> Self {
        style(&cc.egui_ctx);
        if let Some(state) = &cc.wgpu_render_state {
            let gfx = Gfx::new(&state.device, state.target_format);
            state.renderer.write().callback_resources.insert(gfx);
        }
        let title = "aurora-spiral".to_owned();
        Self {
            base: presets::get(&title).unwrap_or_else(|| random_scene(1)),
            title,
            seed: 0,
            adjust: Adjust {
                brightness: 1.0,
                gamma: 1.0,
                glow: 1.0,
                zoom: 1.0,
            },
            optics: Optics {
                refraction: 44.0,
                blur: 0.5,
                tint: 0.3,
                rim: 1.0,
                radius: 28.0,
                bezel: 26.0,
            },
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
        self.requested = Some((size, self.adjust));
        self.rendering = true;

        let shrink = (size[0].max(size[1]) as f32 / PREVIEW_SIDE as f32).max(1.0);
        let options = RenderOptions {
            width: ((size[0] as f32 / shrink) as u32).max(1),
            height: ((size[1] as f32 / shrink) as u32).max(1),
            samples_per_pixel: PREVIEW_SAMPLES,
            supersample: 1,
        };
        let (scene, cancel, generation) = (self.scene(), self.cancel.clone(), self.generation);
        let (sender, ctx) = (self.results.0.clone(), ctx.clone());
        std::thread::spawn(move || {
            let started = Instant::now();
            let control = Control {
                cancel: Some(&cancel),
                progress: None,
            };
            if let Ok(image) = render(&scene, &options, &control) {
                let radius = image.width.min(image.height) as f32 * 0.03;
                let soft = blur::soften(&image.rgba, image.width as usize, image.height as usize, radius);
                let seconds = started.elapsed().as_secs_f32();
                if sender
                    .send(Rendered {
                        generation,
                        image,
                        soft,
                        seconds,
                    })
                    .is_ok()
                {
                    ctx.request_repaint();
                }
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
                gfx.set_images(
                    &state.device,
                    &state.queue,
                    (&image.rgba, image.width, image.height),
                    (&done.soft.0, done.soft.1, done.soft.2),
                );
                self.shown_frames = 1;
            }
            self.rendering = false;
            self.status = format!("{} × {} · {:.2} s", done.image.width, done.image.height, done.seconds);
        }
    }

    fn controls(&mut self, ui: &mut egui::Ui) {
        ui.set_width(270.0);
        ui.label(egui::RichText::new("frcq.").size(34.0).strong());
        ui.label(
            egui::RichText::new("fractique · fractal flames")
                .size(12.0)
                .color(egui::Color32::from_white_alpha(150)),
        );
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            egui::ComboBox::from_id_salt("preset")
                .width(160.0)
                .selected_text(self.title.clone())
                .show_ui(ui, |ui| {
                    for name in presets::names() {
                        if ui.selectable_label(self.title == name, name).clicked()
                            && let Some(scene) = presets::get(name)
                        {
                            self.base = scene;
                            self.title = name.to_owned();
                            self.requested = None;
                        }
                    }
                });
            if ui.button("Randomize").clicked() {
                self.seed += 1;
                self.base = random_scene(self.seed);
                self.title = format!("random {}", self.seed);
                self.requested = None;
            }
        });

        ui.add_space(2.0);
        let adjust = &mut self.adjust;
        ui.add(egui::Slider::new(&mut adjust.brightness, 0.3..=2.5).text("Brightness"));
        ui.add(egui::Slider::new(&mut adjust.gamma, 0.5..=1.8).text("Gamma"));
        ui.add(egui::Slider::new(&mut adjust.glow, 0.0..=3.0).text("Glow"));
        ui.add(
            egui::Slider::new(&mut adjust.zoom, 0.4..=2.5)
                .logarithmic(true)
                .text("Zoom"),
        );
        if ui.button("Reset").clicked() {
            *adjust = Adjust {
                brightness: 1.0,
                gamma: 1.0,
                glow: 1.0,
                zoom: 1.0,
            };
        }
    }

    fn glass_controls(&mut self, ui: &mut egui::Ui) {
        ui.set_width(250.0);
        ui.label(egui::RichText::new("Glass").size(16.0).strong());
        let optics = &mut self.optics;
        ui.add(egui::Slider::new(&mut optics.refraction, -60.0..=60.0).text("Refraction"));
        ui.add(egui::Slider::new(&mut optics.blur, 0.0..=1.0).text("Blur"));
        ui.add(egui::Slider::new(&mut optics.tint, 0.0..=1.0).text("Tint"));
        ui.add(egui::Slider::new(&mut optics.rim, 0.0..=2.0).text("Rim light"));
        ui.add(egui::Slider::new(&mut optics.radius, 6.0..=48.0).text("Radius"));
        ui.add(egui::Slider::new(&mut optics.bezel, 4.0..=48.0).text("Bezel"));
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
        // A few frames after the flame arrives, so the panes have their final places.
        if self.shown_frames == 6
            && let Some(path) = self.screenshot.clone()
        {
            let (done, ctx_done) = (self.screenshot_done.clone(), ctx.clone());
            let callback = egui::ScreenshotCallback::new(move |image: Arc<egui::ColorImage>| {
                if let Err(error) = save_png(&path, image.as_raw(), image.size) {
                    eprintln!("frcq: {}: {error}", path.display());
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

        let screen = ctx.content_rect();
        let scale = ctx.pixels_per_point();
        let size = [
            (screen.width() * scale).round() as u32,
            (screen.height() * scale).round() as u32,
        ];
        if size[0] > 0 && size[1] > 0 && self.requested != Some((size, self.adjust)) {
            self.start_render(&ctx, size);
        }

        let painter = ctx.layer_painter(egui::LayerId::background());
        let mut glass = glass::Frame::new(&painter, screen);
        for pane in &self.panes {
            glass.pane(*pane, self.optics);
        }

        let margin = egui::Margin::same(20);
        let mut panes = Vec::new();
        let controls = egui::Area::new(egui::Id::unique("controls"))
            .fixed_pos(egui::pos2(28.0, 28.0))
            .show(&ctx, |ui| {
                egui::Frame::NONE.inner_margin(margin).show(ui, |ui| self.controls(ui));
            });
        panes.push(controls.response.rect);

        let tuning = egui::Area::new(egui::Id::unique("glass"))
            .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-28.0, -28.0))
            .show(&ctx, |ui| {
                egui::Frame::NONE
                    .inner_margin(margin)
                    .show(ui, |ui| self.glass_controls(ui));
            });
        panes.push(tuning.response.rect);

        let status = if self.rendering {
            "rendering…".to_owned()
        } else {
            self.status.clone()
        };
        let pill = egui::Area::new(egui::Id::unique("status"))
            .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-28.0, 28.0))
            .show(&ctx, |ui| {
                egui::Frame::NONE
                    .inner_margin(egui::Margin::symmetric(18, 10))
                    .show(ui, |ui| {
                        ui.add(egui::Label::new(format!("{} · {status}", self.title)).extend());
                    });
            });
        panes.push(pill.response.rect);

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

fn main() -> eframe::Result {
    let mut args = std::env::args().skip(1);
    let mut screenshot = None;
    while let Some(arg) = args.next() {
        if arg == "--screenshot" {
            screenshot = args.next().map(PathBuf::from);
        }
    }
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("frcq.")
            .with_inner_size([1280.0, 800.0]),
        ..Default::default()
    };
    eframe::run_native(
        "frcq.",
        options,
        Box::new(move |cc| Ok(Box::new(App::new(cc, screenshot)))),
    )
}

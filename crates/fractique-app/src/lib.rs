mod blur;
#[cfg(not(target_arch = "wasm32"))]
mod export;
mod glass;
mod i18n;
#[cfg(target_os = "ios")]
mod ios;
mod phone;
#[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
mod update;
#[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
mod updates;
mod widgets;

use std::collections::HashMap;
use std::path::PathBuf;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, mpsc};

use web_time::Instant;

use eframe::egui;
use eframe::egui_wgpu::{self, wgpu};
use fractique_core::random::{mutate, random_scene};
use fractique_core::scene::{Blend, MAX_LAYERS};
use fractique_core::wallpaper::{Layout, Material, Request, chosen as wallpaper_choice, random_wallpaper};
use fractique_core::{Control, Developed, Image, RenderError, RenderOptions, Scene, compose, presets, render_layer};

use glass::{Gfx, Optics};
use i18n::{Lang, Texts};
use widgets::{MEDIUM, SEMIBOLD, font};

const DRAFT_SIDE: f32 = 480.0;
const DRAFT_SAMPLES: u32 = 12;
const MIDDLE_SIDE: f32 = 1280.0;
const MIDDLE_SAMPLES: u32 = 16;
const FULL_SIDE: f32 = if cfg!(target_arch = "wasm32") {
    900.0
} else if cfg!(any(target_os = "android", target_os = "ios")) {
    1600.0
} else {
    4096.0
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
    view: View,
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

const SIZES: [(&str, [u32; 2]); 21] = [
    ("HD", [1280, 720]),
    ("Full HD", [1920, 1080]),
    ("QHD", [2560, 1440]),
    ("Ultrawide", [3440, 1440]),
    ("4K", [3840, 2160]),
    ("Android HD+", [720, 1600]),
    ("Android HD+ 19.5:9", [720, 1560]),
    ("Android Full HD", [1080, 1920]),
    ("Android Full HD+", [1080, 2400]),
    ("Galaxy S", [1080, 2340]),
    ("Pixel", [1080, 2424]),
    ("Xiaomi 1.5K", [1220, 2712]),
    ("Pixel Pro", [1280, 2856]),
    ("Pixel Pro XL", [1344, 2992]),
    ("Galaxy S Ultra", [1440, 3120]),
    ("Android QHD+", [1440, 3200]),
    ("iPhone", [1179, 2556]),
    ("iPhone Pro Max", [1290, 2796]),
    ("iPad", [1640, 2360]),
    ("iPad Pro", [2048, 2732]),
    ("Android tablet", [1600, 2560]),
];
const LARGEST_SIDE: u32 = 16384;

fn size_list(
    ui: &mut egui::Ui,
    texts: &Texts,
    own: &str,
    base: [u32; 2],
    chosen: &mut Option<[u32; 2]>,
    custom: &mut [String; 2],
    height: f32,
) -> bool {
    let mut picked = false;
    if widgets::row(ui, own, chosen.is_none()) {
        *chosen = None;
        picked = true;
    }
    let listed = SIZES.iter().any(|(_, size)| Some(*size) == *chosen);
    let mut bar = egui::style::ScrollStyle::floating();
    bar.bar_width = 5.0;
    ui.spacing_mut().scroll = bar;
    egui::ScrollArea::vertical().max_height(height).show(ui, |ui| {
        for (name, size) in SIZES {
            let label = format!("{} × {}  ·  {name}", size[0], size[1]);
            if widgets::row(ui, &label, *chosen == Some(size)) {
                *chosen = Some(size);
                picked = true;
            }
        }
    });
    if custom[0].is_empty() {
        let [width, height] = chosen.unwrap_or(base);
        *custom = [width.to_string(), height.to_string()];
    }
    ui.add_space(6.0);
    widgets::row(ui, &texts.get("size_custom"), chosen.is_some() && !listed);
    ui.add_space(4.0);
    let mut entered = false;
    let (line, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 36.0), egui::Sense::hover());
    let mut left = line.left();
    for (side, text) in custom.iter_mut().enumerate() {
        let rect = egui::Rect::from_min_size(egui::pos2(left, line.top()), egui::vec2(84.0, line.height()));
        ui.painter()
            .rect_filled(rect, egui::CornerRadius::same(10), egui::Color32::from_white_alpha(30));
        let field = egui::TextEdit::singleline(text)
            .frame(egui::Frame::NONE)
            .font(font(14.0, MEDIUM))
            .text_color(egui::Color32::WHITE)
            .align(egui::Align2::LEFT_CENTER)
            .desired_width(60.0)
            .char_limit(5);
        let typed = ui.put(rect.shrink2(egui::vec2(12.0, 4.0)), field);
        entered |= typed.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter));
        text.retain(|c| c.is_ascii_digit());
        left = rect.right();
        if side == 0 {
            ui.painter().text(
                egui::pos2(left + 9.0, line.center().y),
                egui::Align2::CENTER_CENTER,
                "×",
                egui::FontId::proportional(14.0),
                egui::Color32::from_white_alpha(170),
            );
            left += 18.0;
        }
    }
    let button = egui::Rect::from_min_max(egui::pos2(left + 8.0, line.top()), line.right_bottom());
    ui.scope_builder(egui::UiBuilder::new().max_rect(button), |ui| {
        entered |= widgets::pill(ui, &texts.get("apply"), false, button.width());
    });
    if entered {
        match parsed_size(&format!("{}x{}", custom[0], custom[1])) {
            Some(size) => {
                *chosen = Some(size);
                picked = true;
            }
            None => *custom = Default::default(),
        }
    }
    picked
}

#[derive(Clone, Copy, PartialEq)]
enum Sheet {
    Presets,
    Sizes,
}

#[derive(Clone, Copy, PartialEq)]
enum Listing {
    Presets,
    Layouts,
    Materials,
}

fn shown(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for word in name.split('-') {
        if !out.is_empty() {
            out.push(' ');
        }
        let mut letters = word.chars();
        if let Some(first) = letters.next() {
            out.extend(first.to_uppercase());
            out.push_str(letters.as_str());
        }
    }
    out
}

fn named(mut scene: Scene, title: &str) -> Scene {
    let several = scene.layers.len() > 1;
    for (index, layer) in scene.layers.iter_mut().enumerate() {
        layer.name = if several {
            format!("{title} {}", index + 1)
        } else {
            title.to_owned()
        };
    }
    scene
}

fn parsed_size(text: &str) -> Option<[u32; 2]> {
    let (width, height) = text.split_once('x')?;
    let side = |text: &str| {
        text.trim()
            .parse::<u32>()
            .ok()
            .filter(|side| (16..=LARGEST_SIDE).contains(side))
    };
    Some([side(width)?, side(height)?])
}

#[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
const BAR: f32 = 44.0;
#[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
const BAR_GAP: f32 = 10.0;

// What the picture shows: the point `right` and `down` of the scene's own middle, in shorter sides of the picture,
// magnified `zoom` times.
#[derive(Clone, Copy, PartialEq)]
struct View {
    right: f64,
    down: f64,
    zoom: f64,
}

const WHOLE: View = View {
    right: 0.0,
    down: 0.0,
    zoom: 1.0,
};

// Past this the numbers of the renderer run out of digits.
// The app's own scene files. Inside is the scene as JSON, so every other Fractique tool reads them too.
#[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
const SCENE_FILE: &str = "frcq";
#[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
static OPEN_FIRST: OnceLock<PathBuf> = OnceLock::new();

#[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
pub fn open_at_start(path: PathBuf) {
    let _ = OPEN_FIRST.set(path);
}

const MAX_MAGNIFICATION: f64 = 1e10;
const MIN_MAGNIFICATION: f64 = 1e-4;
// A new picture is drawn once the view has stood still this long.
const SETTLE: f32 = 0.18;

impl View {
    fn moved(&mut self, by: [f64; 2]) {
        self.right = (self.right - by[0] / self.zoom).clamp(-1e6, 1e6);
        self.down = (self.down - by[1] / self.zoom).clamp(-1e6, 1e6);
    }

    fn magnified(&mut self, factor: f64, around: [f64; 2]) {
        let held = [self.right + around[0] / self.zoom, self.down + around[1] / self.zoom];
        self.zoom = (self.zoom * factor).clamp(MIN_MAGNIFICATION, MAX_MAGNIFICATION);
        self.right = held[0] - around[0] / self.zoom;
        self.down = held[1] - around[1] / self.zoom;
        self.moved([0.0; 2]);
    }

    // Where this view is in a picture drawn for the view `shown`: the picture point at its middle, and how many
    // times larger it shows the picture.
    fn within(&self, shown: &View, size: [u32; 2]) -> [f32; 3] {
        let shorter = f64::from(size[0].min(size[1]));
        let across = (self.right - shown.right) * shown.zoom * shorter / f64::from(size[0]);
        let along = (self.down - shown.down) * shown.zoom * shorter / f64::from(size[1]);
        [
            (0.5 + across) as f32,
            (0.5 + along) as f32,
            (self.zoom / shown.zoom) as f32,
        ]
    }

    fn label(&self) -> String {
        match self.zoom {
            zoom if zoom >= 1e6 => format!("{zoom:.1e}"),
            zoom if zoom >= 100.0 => format!("{zoom:.0}"),
            zoom if zoom >= 1.0 => format!("{zoom:.1}"),
            zoom => format!("{zoom:.3}"),
        }
    }
}

// A part of the screen, from its middle, in shorter sides of the picture.
fn sides(of_screen: egui::Vec2, size: [u32; 2], stretch: f32) -> [f64; 2] {
    let of_picture = if stretch > 1.0 {
        egui::vec2(of_screen.x, of_screen.y * stretch)
    } else {
        egui::vec2(of_screen.x / stretch, of_screen.y)
    };
    let shorter = f64::from(size[0].min(size[1]));
    [
        f64::from(of_picture.x) * f64::from(size[0]) / shorter,
        f64::from(of_picture.y) * f64::from(size[1]) / shorter,
    ]
}

const NEUTRAL: Adjust = Adjust {
    brightness: 1.0,
    gamma: 1.0,
    glow: 1.0,
    zoom: 1.0,
};

#[derive(Default)]
struct Wall {
    on: bool,
    live: bool,
    layout: Option<Layout>,
    material: Option<Material>,
    held: Option<(Layout, Material)>,
    dark: bool,
    hue: Option<f64>,
}

impl Wall {
    fn request(&self, size: [u32; 2]) -> Request {
        Request {
            layout: self.layout.or(self.held.map(|held| held.0)),
            material: self.material.or(self.held.map(|held| held.1)),
            ratio: f64::from(size[1].max(1)) / f64::from(size[0].max(1)),
            dark: self.dark,
            hue: self.hue,
        }
    }
}

struct App {
    texts: Texts,
    base: Scene,
    wall: Wall,
    phone: phone::Phone,
    title: String,
    is_random: bool,
    seed: u64,
    adjust: Adjust,
    view: View,
    shown: View,
    drawn: Option<View>,
    touched: Instant,
    bare: bool,
    optics: Optics,
    tuning: bool,
    inspector_open: Option<bool>,
    panes: Vec<(egui::Rect, f32, bool)>,
    toolbar_size: egui::Vec2,
    languages_open: bool,
    picture: Option<[u32; 2]>,
    custom: [String; 2],
    sizes_open: bool,
    presets_open: bool,
    listing: Listing,
    adding_layer: bool,
    layer: usize,
    #[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
    maximize_again: bool,
    #[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
    full_pending: bool,
    #[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
    maximize_pending: bool,
    #[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
    probe: u32,
    language_anchor: egui::Pos2,
    results: (mpsc::Sender<Rendered>, mpsc::Receiver<Rendered>),
    kept: Layers,
    cancel: Arc<AtomicBool>,
    generation: u64,
    requested: Option<[u32; 2]>,
    rendering: bool,
    status: String,
    shown_frames: u32,
    screenshot: Option<PathBuf>,
    screenshot_done: Arc<AtomicBool>,
    saved: String,
    light: f32,
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

struct Kept {
    layer: Arc<Developed>,
    used: u64,
    home: bool,
}

type Layers = Arc<Mutex<HashMap<String, Kept>>>;

const KEPT_BYTES: usize = if cfg!(any(target_os = "android", target_os = "ios", target_arch = "wasm32")) {
    160 << 20
} else {
    1200 << 20
};

// A layer's pixels depend only on its flame, its place in the scene, the seed and the size, so a layer that did
// not change is taken from an earlier render. The layers of the whole view are the last to be dropped: Reset view
// comes back to them.
fn rendered(
    scene: &Scene,
    options: &RenderOptions,
    control: &Control<'_>,
    kept: &Layers,
    home: bool,
) -> Result<Image, RenderError> {
    static USES: AtomicU64 = AtomicU64::new(0);
    let size = format!(
        "{} {} {} {} ",
        options.width, options.height, options.samples_per_pixel, options.supersample
    );
    let (mut keys, mut layers) = (Vec::new(), Vec::new());
    for (index, layer) in scene.layers.iter().enumerate() {
        let flame = serde_json::to_string(&layer.flame).unwrap_or_default();
        let key = format!("{size}{} {index} {flame}", scene.seed);
        let used = USES.fetch_add(1, Ordering::Relaxed);
        let found = kept
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get_mut(&key)
            .map(|entry| {
                entry.used = used;
                entry.home |= home;
                entry.layer.clone()
            });
        let developed = match found {
            Some(developed) => developed,
            None => {
                let layer = Arc::new(render_layer(scene, index, options, control)?);
                let entry = Kept {
                    layer: layer.clone(),
                    used,
                    home,
                };
                kept.lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .insert(key.clone(), entry);
                layer
            }
        };
        keys.push(key);
        layers.push(developed);
    }
    {
        let mut kept = kept.lock().unwrap_or_else(|e| e.into_inner());
        let mut bytes: usize = kept.values().map(|entry| entry.layer.bytes()).sum();
        let mut spare: Vec<(bool, u64, String)> = kept
            .iter()
            .filter(|(key, _)| !keys.contains(key))
            .map(|(key, entry)| (entry.home, entry.used, key.clone()))
            .collect();
        spare.sort();
        for (_, _, key) in spare {
            if bytes <= KEPT_BYTES {
                break;
            }
            if let Some(entry) = kept.remove(&key) {
                bytes -= entry.layer.bytes();
            }
        }
    }
    compose(scene, &layers.iter().map(|layer| &**layer).collect::<Vec<_>>(), options)
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
        #[cfg(target_os = "ios")]
        join_scene(cc);
        let stored = |key: &str| cc.storage.and_then(|storage| storage.get_string(key));
        let lang = Lang::forced()
            .or_else(|| stored("language").map(|code| Lang::from_locale(&code)))
            .unwrap_or_else(Lang::system);
        let texts = Texts::new(lang);
        let glass = |key: &str, default: f32| stored(key).and_then(|text| text.parse().ok()).unwrap_or(default);
        let (opacity, blur) = (glass("glass_opacity", 0.4), glass("glass_blur", 0.7));
        widgets::install_fonts(&cc.egui_ctx, texts.lang);
        cc.egui_ctx.set_visuals(egui::Visuals::dark());
        // The window comes back where and as large as it was, on the same screen, but never in full screen.
        #[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
        cc.egui_ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(false));
        if let Some(state) = &cc.wgpu_render_state {
            note(&format!("graphics: {:?}", state.adapter.get_info()));
            let gfx = Gfx::new(&state.device, state.target_format);
            state.renderer.write().callback_resources.insert(gfx);
        }
        let restore = screenshot.is_none() || std::env::var_os("FRACTIQUE_RESTORE").is_some();
        let session = if restore { stored("session") } else { None };
        let title = "aurora-spiral".to_owned();
        let mut app = Self {
            texts,
            base: named(presets::get(&title).unwrap_or_else(|| random_scene(1)), &shown(&title)),
            wall: Wall {
                on: std::env::var("FRACTIQUE_SHOW").is_ok_and(|what| what.contains("wall") || what == "layouts"),
                live: std::env::var("FRACTIQUE_SHOW").is_ok_and(|what| what.contains("wall") || what == "layouts"),
                ..Default::default()
            },
            phone: phone::Phone::new(
                stored("hints").and_then(|text| text.parse().ok()).unwrap_or(0),
                stored("kept"),
            ),
            title,
            is_random: false,
            seed: 0,
            adjust: NEUTRAL,
            view: WHOLE,
            shown: WHOLE,
            drawn: None,
            touched: Instant::now(),
            bare: false,
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
            toolbar_size: egui::Vec2::ZERO,
            languages_open: std::env::var("FRACTIQUE_SHOW").is_ok_and(|what| what == "languages"),
            language_anchor: egui::Pos2::ZERO,
            picture: stored("picture").and_then(|text| parsed_size(&text)),
            custom: Default::default(),
            sizes_open: std::env::var("FRACTIQUE_SHOW").is_ok_and(|what| what == "sizes"),
            presets_open: std::env::var("FRACTIQUE_SHOW").is_ok_and(|what| what == "presets" || what == "layouts"),
            listing: if std::env::var("FRACTIQUE_SHOW").is_ok_and(|what| what == "layouts") {
                Listing::Layouts
            } else {
                Listing::Presets
            },
            adding_layer: false,
            layer: 0,
            #[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
            maximize_again: false,
            #[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
            full_pending: false,
            #[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
            maximize_pending: false,
            #[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
            probe: 0,
            results: mpsc::channel(),
            kept: Layers::default(),
            cancel: Arc::new(AtomicBool::new(false)),
            generation: 0,
            requested: None,
            rendering: false,
            status: String::new(),
            shown_frames: 0,
            screenshot,
            screenshot_done: Arc::new(AtomicBool::new(false)),
            saved: String::new(),
            light: 0.0,
            last_frame: Instant::now(),
            #[cfg(not(target_arch = "wasm32"))]
            export: export::Sheet {
                open: std::env::var("FRACTIQUE_SHOW").is_ok_and(|what| what.starts_with("export")),
                ..Default::default()
            },
            #[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
            updates: updates::Updates::new(),
        };
        if let Some(session) = session {
            let _ = app.restore(&session);
        }
        #[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
        if let Some(path) = OPEN_FIRST.get() {
            app.load_scene(path);
        }
        app
    }

    fn scene(&self) -> Scene {
        let mut scene = self.base.clone();
        for layer in &mut scene.layers {
            let tone = &mut layer.flame.tone;
            tone.brightness *= self.adjust.brightness;
            tone.gamma *= self.adjust.gamma;
            tone.glow_strength *= self.adjust.glow;
            layer.flame.camera.zoom *= self.adjust.zoom;
            layer.flame.camera = layer
                .flame
                .camera
                .looking_at(self.view.right, self.view.down, self.view.zoom);
        }
        scene
    }

    fn scene_at(&self, size: [u32; 2]) -> Scene {
        if self.wall.live {
            random_wallpaper(self.seed, &self.wall.request(size))
        } else {
            self.scene()
        }
    }

    fn new_wallpaper(&mut self) {
        self.seed += 1;
        self.title = format!("{} {}", self.t("wallpaper"), self.seed);
        self.is_random = true;
        self.wall.live = true;
        self.requested = None;
        self.view = WHOLE;
    }

    fn start_render(&mut self, ctx: &egui::Context, size: [u32; 2]) {
        self.cancel.store(true, Ordering::Relaxed);
        self.cancel = Arc::new(AtomicBool::new(false));
        self.generation += 1;
        self.requested = Some(size);
        self.drawn = Some(self.view);
        self.rendering = true;

        let full = fitted(size, FULL_SIDE, FULL_SAMPLES);
        let mut passes = vec![(true, fitted(size, DRAFT_SIDE, DRAFT_SAMPLES)), (false, full)];
        // A deep view takes several times longer to draw: a picture in between shortens the wait for a sharp one.
        let between = fitted(size, MIDDLE_SIDE, MIDDLE_SAMPLES);
        if self.view.zoom > 4.0 && full.width.max(full.height) >= 2 * between.width.max(between.height) {
            passes.insert(1, (true, between));
        }
        // A browser page has one thread: a draft first would only make it wait twice.
        if passes[0].1.width == full.width || cfg!(target_arch = "wasm32") {
            passes.remove(0);
        }
        let (scene, cancel, generation) = (self.scene_at(size), self.cancel.clone(), self.generation);
        let view = self.view;
        let home = view == WHOLE;
        let (sender, ctx, kept) = (self.results.0.clone(), ctx.clone(), self.kept.clone());
        let work = move || {
            for (draft, options) in passes {
                let started = Instant::now();
                let control = Control {
                    cancel: Some(&cancel),
                    progress: None,
                };
                #[cfg(not(target_arch = "wasm32"))]
                let result = render_pool().install(|| rendered(&scene, &options, &control, &kept, home));
                #[cfg(target_arch = "wasm32")]
                let result = rendered(&scene, &options, &control, &kept, home);
                let Ok(image) = result else {
                    return;
                };
                let radius = image.width.min(image.height) as f32 * 0.03;
                let soft = blur::soften(&image.rgba, image.width as usize, image.height as usize, radius);
                let seconds = started.elapsed().as_secs_f32();
                let done = Rendered {
                    generation,
                    draft,
                    view,
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
            self.shown = done.view;
            let pixels = done.image.rgba.chunks_exact(4).step_by(97);
            let (sum, count) = pixels.fold((0u64, 0u64), |(sum, count), pixel| {
                (
                    sum + u64::from(pixel[0]) * 2 + u64::from(pixel[1]) * 5 + u64::from(pixel[2]),
                    count + 8,
                )
            });
            self.light = sum as f32 / (count.max(1) as f32 * 255.0);
            if !done.draft {
                self.rendering = false;
                self.shown_frames = self.shown_frames.max(1);
                self.status = format!("{} × {}  ·  {:.2} s", done.image.width, done.image.height, done.seconds);
                if done.view.zoom != 1.0 {
                    self.status += &format!("  ·  {}×", done.view.label());
                }
            }
        }
    }

    fn session(&self) -> String {
        let name = |layout: Layout| {
            Layout::ALL
                .iter()
                .find(|(_, each)| *each == layout)
                .map(|(name, _)| *name)
        };
        let stuff = |material: Material| {
            Material::ALL
                .iter()
                .find(|(_, each)| *each == material)
                .map(|(name, _)| *name)
        };
        let optics = [self.optics.refraction, self.optics.rim, self.optics.bezel];
        let session = serde_json::json!({
            "scene": serde_json::to_value(&self.base).unwrap_or_default(),
            "title": self.title,
            "random": self.is_random,
            "seed": self.seed,
            "adjust": [self.adjust.brightness, self.adjust.gamma, self.adjust.glow, self.adjust.zoom],
            "view": [self.view.right, self.view.down, self.view.zoom],
            "layer": self.layer,
            "inspector": self.inspector_open,
            "optics": optics,
            "wall": {
                "on": self.wall.on,
                "live": self.wall.live,
                "layout": self.wall.layout.and_then(name),
                "material": self.wall.material.and_then(stuff),
                "held": self.wall.held.and_then(|(layout, material)| Some([name(layout)?, stuff(material)?])),
                "dark": self.wall.dark,
                "hue": self.wall.hue,
            },
        });
        session.to_string()
    }

    fn restore(&mut self, text: &str) -> Option<()> {
        let session: serde_json::Value = serde_json::from_str(text).ok()?;
        let scene: Scene = serde_json::from_value(session.get("scene")?.clone()).ok()?;
        scene.validate().ok()?;
        let numbers = |key: &str| -> Option<Vec<f64>> {
            session
                .get(key)?
                .as_array()?
                .iter()
                .map(serde_json::Value::as_f64)
                .collect()
        };
        self.base = scene;
        self.title = session.get("title")?.as_str()?.to_owned();
        self.is_random = session.get("random")?.as_bool()?;
        self.seed = session.get("seed")?.as_u64()?;
        if let Some(&[brightness, gamma, glow, zoom]) = numbers("adjust").as_deref() {
            self.adjust = Adjust {
                brightness,
                gamma,
                glow,
                zoom,
            };
        }
        if let Some(&[right, down, zoom]) = numbers("view").as_deref()
            && (MIN_MAGNIFICATION..=MAX_MAGNIFICATION).contains(&zoom)
        {
            self.view = View { right, down, zoom };
        }
        self.layer = session.get("layer").and_then(serde_json::Value::as_u64).unwrap_or(0) as usize;
        self.inspector_open = session.get("inspector").and_then(serde_json::Value::as_bool);
        if let Some(&[refraction, rim, bezel]) = numbers("optics").as_deref() {
            self.optics.refraction = refraction as f32;
            self.optics.rim = rim as f32;
            self.optics.bezel = bezel as f32;
        }
        let wall = session.get("wall")?;
        let word = |key: &str| wall.get(key).and_then(serde_json::Value::as_str);
        let held = wall.get("held").and_then(serde_json::Value::as_array);
        self.wall = Wall {
            on: wall.get("on")?.as_bool()?,
            live: wall.get("live")?.as_bool()?,
            layout: word("layout").and_then(Layout::parse),
            material: word("material").and_then(Material::parse),
            hue: wall.get("hue").and_then(serde_json::Value::as_f64),
            held: held.and_then(|held| {
                Some((
                    Layout::parse(held.first()?.as_str()?)?,
                    Material::parse(held.get(1)?.as_str()?)?,
                ))
            }),
            dark: wall.get("dark")?.as_bool()?,
        };
        Some(())
    }

    fn open_sheet(&mut self, which: Sheet) {
        let (presets, sizes) = (self.presets_open, self.sizes_open);
        self.presets_open = which == Sheet::Presets && !presets;
        self.listing = Listing::Presets;
        self.adding_layer = false;
        self.sizes_open = which == Sheet::Sizes && !sizes;
        self.languages_open = false;
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.export.open = false;
        }
        #[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
        {
            self.updates.open = false;
        }
    }

    fn preset_sheet(&mut self, ui: &mut egui::Ui, room: f32) {
        ui.set_width(260.0);
        ui.spacing_mut().item_spacing = egui::vec2(8.0, 2.0);
        if self.listing != Listing::Presets {
            self.wall_sheet(ui, room);
            return;
        }
        let adding = self.adding_layer;
        widgets::section(ui, &self.t(if adding { "layers" } else { "presets" }));
        let mut bar = egui::style::ScrollStyle::floating();
        bar.bar_width = 5.0;
        ui.spacing_mut().scroll = bar;
        let mut chosen = None;
        egui::ScrollArea::vertical()
            .max_height((room - 110.0).max(90.0))
            .show(ui, |ui| {
                if adding && widgets::row(ui, &shown(&self.t("random")), false) {
                    self.seed += 1;
                    let title = format!("{} {}", shown(&self.t("random")), self.seed);
                    chosen = Some((random_scene(self.seed), title));
                }
                for name in presets::names() {
                    let current = !adding && !self.is_random && self.title == name;
                    if widgets::row(ui, &shown(name), current)
                        && let Some(scene) = presets::get(name)
                    {
                        chosen = Some((named(scene, &shown(name)), name.to_owned()));
                    }
                }
            });
        let Some((scene, title)) = chosen else { return };
        self.presets_open = false;
        if adding {
            if let Some(mut layer) = scene.layers.into_iter().next()
                && self.base.layers.len() < MAX_LAYERS
            {
                layer.blend = Blend::Screen;
                layer.name = shown(&title);
                self.base.layers.push(layer);
                self.layer = self.base.layers.len() - 1;
                self.requested = None;
            }
        } else {
            self.pick(scene, title, false);
        }
    }

    fn size_sheet(&mut self, ui: &mut egui::Ui, window: [u32; 2], room: f32) {
        ui.set_width(300.0);
        ui.spacing_mut().item_spacing = egui::vec2(8.0, 2.0);
        widgets::section(ui, &self.t("size"));
        let own = format!("{} · {} × {}", self.t("size_window"), window[0], window[1]);
        let height = (room - 230.0).clamp(34.0, 420.0);
        if size_list(
            ui,
            &self.texts,
            &own,
            window,
            &mut self.picture,
            &mut self.custom,
            height,
        ) {
            self.sizes_open = false;
        }
    }

    fn reset_button(&mut self, ctx: &egui::Context, at: egui::Pos2) -> egui::Rect {
        let area = egui::Area::new(egui::Id::unique("reset_view"))
            .fixed_pos(at)
            .pivot(egui::Align2::CENTER_BOTTOM)
            .show(ctx, |ui| {
                let ink = egui::Color32::WHITE;
                let galley = ui
                    .painter()
                    .layout_no_wrap(self.t("reset_view"), font(14.0, MEDIUM), ink);
                let (rect, response) =
                    ui.allocate_exact_size(egui::vec2(galley.size().x + 40.0, 42.0), egui::Sense::click());
                if response.hovered() {
                    ui.painter()
                        .rect_filled(rect, egui::CornerRadius::same(21), egui::Color32::from_white_alpha(22));
                }
                ui.painter().galley(
                    egui::pos2(rect.left() + 20.0, rect.center().y - galley.size().y * 0.5),
                    galley,
                    ink,
                );
                if response.clicked() {
                    self.reset_view();
                }
            });
        area.response.rect
    }

    fn reset_view(&mut self) {
        self.view = WHOLE;
        self.touched = Instant::now();
    }

    fn explore(&mut self, ui: &mut egui::Ui, screen: egui::Rect) {
        let picture = ui.interact(screen, egui::Id::unique("picture"), egui::Sense::click_and_drag());
        let scale = ui.ctx().pixels_per_point();
        let window = [
            (screen.width() * scale).round() as u32,
            (screen.height() * scale).round() as u32,
        ];
        let size = self.picture.unwrap_or(window);
        if size[0] == 0 || size[1] == 0 {
            return;
        }
        let stretch = (size[0] as f32 / size[1] as f32) / (screen.width() / screen.height()).max(0.001);
        let before = self.view;
        let swiping = self.phone.on && self.wall.live;
        if picture.dragged() {
            if swiping {
                self.phone.swipe += picture.drag_delta();
            } else {
                self.view
                    .moved(sides(picture.drag_delta() / screen.size(), size, stretch));
            }
        }
        if picture.drag_stopped() {
            let by = std::mem::take(&mut self.phone.swipe);
            if swiping && by.x.abs() > phone::SWIPE && by.x.abs() > by.y.abs() * 1.4 {
                self.step_wallpaper(by.x < 0.0);
            }
        }
        let held = ui.input(|input| {
            input
                .pointer
                .press_start_time()
                .is_some_and(|start| input.time - start > phone::HOLD)
        });
        // A finger that moved, or a second finger, is a drag or a pinch for the rest of that press, not a hold.
        let down = picture.is_pointer_button_down_on();
        let moved = ui.input(|input| {
            let strayed = input
                .pointer
                .press_origin()
                .zip(input.pointer.interact_pos())
                .is_some_and(|(from, to)| (to - from).length() > 10.0);
            strayed || input.multi_touch().is_some()
        });
        self.phone.strayed = down && (self.phone.strayed || moved);
        let holding = self.phone.on && down && held && !self.phone.strayed;
        if holding != self.phone.preview {
            self.phone.preview = holding;
            self.phone.previewed |= holding;
            ui.ctx().request_repaint();
        }
        if self.phone.on && picture.is_pointer_button_down_on() && !held {
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(60));
        }
        if picture.clicked() && std::mem::take(&mut self.phone.previewed) {
        } else if picture.clicked() && (self.phone.sheet || self.phone.tune) {
            self.phone.sheet = false;
            self.phone.tune = false;
        } else if picture.clicked() && (self.languages_open || self.sizes_open || self.presets_open) {
            self.languages_open = false;
            self.sizes_open = false;
            self.presets_open = false;
        } else if picture.double_clicked() {
            self.view = WHOLE;
        } else if picture.clicked() && (self.bare || self.phone.on) {
            self.bare = !self.bare;
        }
        if picture.contains_pointer() {
            let (wheel, pinch, pointer) = ui.input(|input| {
                (
                    input.smooth_scroll_delta.y,
                    input.zoom_delta(),
                    input.pointer.hover_pos(),
                )
            });
            let factor = pinch * (1.0 + wheel * 0.004).clamp(0.5, 2.0);
            if factor != 1.0 {
                let around = pointer.map_or(egui::Vec2::ZERO, |at| (at - screen.center()) / screen.size());
                self.view.magnified(f64::from(factor), sides(around, size, stretch));
            }
        }
        if self.view != before {
            self.touched = Instant::now();
        }
    }

    #[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
    fn window_keys(&mut self, ctx: &egui::Context) {
        let (hide, full, leave, is_full) = ctx.input(|input| {
            (
                input.key_pressed(egui::Key::H) && !ctx.egui_wants_keyboard_input(),
                input.key_pressed(egui::Key::F11),
                input.key_pressed(egui::Key::Escape),
                input.viewport().fullscreen.unwrap_or(false),
            )
        });
        if hide {
            self.bare = !self.bare;
        }
        if full {
            self.full_screen(ctx, !is_full);
        }
        if leave {
            if self.languages_open || self.sizes_open || self.presets_open {
                self.languages_open = false;
                self.sizes_open = false;
                self.presets_open = false;
            } else if self.bare {
                self.bare = false;
            } else if is_full {
                self.full_screen(ctx, false);
            }
        }
        if self.full_pending {
            if ctx.input(|input| input.viewport().maximized.unwrap_or(false)) {
                ctx.request_repaint();
            } else {
                self.full_screen(ctx, true);
            }
        }
        if self.maximize_pending {
            if is_full {
                ctx.request_repaint();
            } else {
                self.maximize_pending = false;
                ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(true));
            }
        }
        self.probe_full_screen(ctx);
        self.resize_edges(ctx, is_full);
    }

    #[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
    fn full_screen(&mut self, ctx: &egui::Context, on: bool) {
        // A maximized window keeps its maximized size in full screen and leaves a black strip where the taskbar was,
        // so it leaves the maximized state first and goes full screen once the system has done that.
        if on && ctx.input(|input| input.viewport().maximized.unwrap_or(false)) {
            self.maximize_again = true;
            self.full_pending = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(false));
            ctx.request_repaint();
            return;
        }
        self.full_pending = false;
        ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(on));
        if !on && std::mem::take(&mut self.maximize_again) {
            self.maximize_pending = true;
            ctx.request_repaint();
        }
    }

    #[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
    fn toggle_maximized(&mut self, ctx: &egui::Context) {
        let (maximized, is_full) = ctx.input(|input| {
            (
                input.viewport().maximized.unwrap_or(false),
                input.viewport().fullscreen.unwrap_or(false),
            )
        });
        // Maximizing a window that is in full screen leaves it over the taskbar with a black strip in its place.
        if is_full {
            self.maximize_again = true;
            self.full_screen(ctx, false);
        } else {
            ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
        }
    }

    #[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
    fn probe_full_screen(&mut self, ctx: &egui::Context) {
        let Some(report) = std::env::var_os("FRACTIQUE_DEBUG_FULL") else {
            return;
        };
        self.probe += 1;
        ctx.request_repaint();
        match self.probe {
            20 => ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(true)),
            90 => self.full_screen(ctx, true),
            150 => self.toggle_maximized(ctx),
            91..240 => {
                let line = format!(
                    "{} {:?} {:?}\n",
                    self.probe,
                    ctx.viewport_rect().size() * ctx.pixels_per_point(),
                    ctx.input(|input| {
                        let window = input.viewport();
                        (
                            window.maximized,
                            window.fullscreen,
                            window.inner_rect.map(|rect| rect.size()),
                            window.outer_rect.map(|rect| rect.size()),
                        )
                    }),
                );
                use std::io::Write;
                if self.probe.is_multiple_of(10)
                    && let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(&report)
                {
                    let _ = file.write_all(line.as_bytes());
                }
            }
            240 => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
            _ => {}
        }
    }

    #[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
    fn resize_edges(&self, ctx: &egui::Context, is_full: bool) {
        use egui::{CursorIcon, ResizeDirection};
        let maximized = ctx.input(|input| input.viewport().maximized.unwrap_or(false));
        let Some(at) = ctx.input(|input| input.pointer.hover_pos()) else {
            return;
        };
        if is_full || maximized {
            return;
        }
        let screen = ctx.viewport_rect();
        let grip = 6.0;
        let (left, right) = (at.x < screen.left() + grip, at.x > screen.right() - grip);
        let (top, bottom) = (at.y < screen.top() + grip, at.y > screen.bottom() - grip);
        let edge = match (left, right, top, bottom) {
            (true, _, true, _) => (ResizeDirection::NorthWest, CursorIcon::ResizeNwSe),
            (_, true, true, _) => (ResizeDirection::NorthEast, CursorIcon::ResizeNeSw),
            (true, _, _, true) => (ResizeDirection::SouthWest, CursorIcon::ResizeNeSw),
            (_, true, _, true) => (ResizeDirection::SouthEast, CursorIcon::ResizeNwSe),
            (true, ..) => (ResizeDirection::West, CursorIcon::ResizeHorizontal),
            (_, true, ..) => (ResizeDirection::East, CursorIcon::ResizeHorizontal),
            (_, _, true, _) => (ResizeDirection::North, CursorIcon::ResizeVertical),
            (_, _, _, true) => (ResizeDirection::South, CursorIcon::ResizeVertical),
            _ => return,
        };
        ctx.set_cursor_icon(edge.1);
        if ctx.input(|input| input.pointer.primary_pressed()) {
            ctx.send_viewport_cmd(egui::ViewportCommand::BeginResize(edge.0));
        }
    }

    #[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
    fn title_bar(&mut self, ctx: &egui::Context, screen: egui::Rect) -> egui::Rect {
        let bar = egui::Rect::from_min_max(
            screen.min + egui::vec2(BAR_GAP, BAR_GAP),
            egui::pos2(screen.right() - BAR_GAP, screen.top() + BAR_GAP + BAR),
        );
        let is_full = ctx.input(|input| input.viewport().fullscreen.unwrap_or(false));
        egui::Area::new(egui::Id::unique("title bar"))
            .fixed_pos(bar.min)
            .show(ctx, |ui| {
                let (rect, grip) = ui.allocate_exact_size(bar.size(), egui::Sense::click_and_drag());
                if grip.double_clicked() {
                    self.toggle_maximized(ctx);
                } else if grip.drag_started_by(egui::PointerButton::Primary) {
                    ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
                }
                let painter = ui.painter().clone();
                let name = painter.text(
                    rect.left_center() + egui::vec2(20.0, 0.0),
                    egui::Align2::LEFT_CENTER,
                    "Fractique",
                    font(14.5, SEMIBOLD),
                    egui::Color32::WHITE,
                );

                let newer = self.updates.newer().map(str::to_owned);
                let (label, ink) = match &newer {
                    Some(version) => (format!("v{version} ↑"), egui::Color32::from_rgb(14, 18, 34)),
                    None => (format!("v{}", update::current()), egui::Color32::from_white_alpha(235)),
                };
                let galley = painter.layout_no_wrap(label, font(12.5, MEDIUM), ink);
                let chip = egui::Rect::from_min_size(
                    egui::pos2(name.right() + 12.0, rect.center().y - 13.0),
                    egui::vec2(galley.size().x + 22.0, 26.0),
                );
                let version = ui.interact(chip, egui::Id::unique("title bar version"), egui::Sense::click());
                let lift = if version.hovered() { 22 } else { 0 };
                let fill = if newer.is_some() { 225 } else { 30 + lift };
                painter.rect_filled(
                    chip,
                    egui::CornerRadius::same(13),
                    egui::Color32::from_white_alpha(fill),
                );
                painter.galley(chip.center() - galley.size() * 0.5, galley, ink);
                if version.clicked() {
                    self.languages_open = false;
                    self.sizes_open = false;
                    self.presets_open = false;
                    self.export.open = false;
                    self.updates.show(ctx);
                }

                let ink = egui::Stroke::new(1.4, egui::Color32::from_white_alpha(235));
                let mut slot = egui::Rect::from_center_size(
                    egui::pos2(rect.right() - 7.0 - 15.0, rect.center().y),
                    egui::Vec2::splat(30.0),
                );
                for button in ["close", "maximize", "minimize", "full", "hide"] {
                    let pressed = ui.interact(slot, egui::Id::unique(("title bar", button)), egui::Sense::click());
                    let fill = match (pressed.hovered(), button) {
                        (true, "close") => egui::Color32::from_rgb(214, 58, 44),
                        (true, _) => egui::Color32::from_white_alpha(56),
                        _ => egui::Color32::from_white_alpha(30),
                    };
                    painter.circle_filled(slot.center(), 15.0, fill);
                    let (c, r) = (slot.center(), 4.5);
                    let line = |from: egui::Vec2, to: egui::Vec2| {
                        painter.line_segment([c + from, c + to], ink);
                    };
                    match button {
                        "close" => {
                            line(egui::vec2(-r, -r), egui::vec2(r, r));
                            line(egui::vec2(-r, r), egui::vec2(r, -r));
                        }
                        "maximize" => {
                            let square = egui::Rect::from_center_size(c, egui::Vec2::splat(2.0 * r));
                            painter.rect_stroke(square, egui::CornerRadius::same(2), ink, egui::StrokeKind::Middle);
                        }
                        "minimize" => line(egui::vec2(-r, 0.0), egui::vec2(r, 0.0)),
                        "full" => {
                            for (x, y) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
                                let corner = egui::vec2(x * (r + 0.5), y * (r + 0.5));
                                line(corner, corner - egui::vec2(x * 3.5, 0.0));
                                line(corner, corner - egui::vec2(0.0, y * 3.5));
                            }
                        }
                        _ => {
                            let pane = egui::Rect::from_center_size(c, egui::vec2(2.0 * r + 3.0, 2.0 * r - 1.0));
                            painter.rect_stroke(pane, egui::CornerRadius::same(3), ink, egui::StrokeKind::Middle);
                            line(egui::vec2(-r - 2.5, r + 1.5), egui::vec2(r + 2.5, -r - 1.5));
                        }
                    }
                    if pressed.clicked() {
                        match button {
                            "close" => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
                            "maximize" => self.toggle_maximized(ctx),
                            "minimize" => ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true)),
                            "full" => self.full_screen(ctx, !is_full),
                            _ => self.bare = true,
                        }
                    }
                    slot = slot.translate(egui::vec2(-36.0, 0.0));
                }

                let ink = egui::Color32::from_white_alpha(235);
                let galley = painter.layout_no_wrap(self.texts.lang.name().to_owned(), font(12.5, MEDIUM), ink);
                let chip = egui::Rect::from_min_max(
                    egui::pos2(slot.right() - galley.size().x - 40.0, rect.center().y - 15.0),
                    egui::pos2(slot.right(), rect.center().y + 15.0),
                );
                let language = ui.interact(chip, egui::Id::unique("title bar language"), egui::Sense::click());
                let lift = if language.hovered() || self.languages_open {
                    26
                } else {
                    0
                };
                painter.rect_filled(
                    chip,
                    egui::CornerRadius::same(15),
                    egui::Color32::from_white_alpha(30 + lift),
                );
                painter.galley(
                    egui::pos2(chip.left() + 14.0, chip.center().y - galley.size().y * 0.5),
                    galley,
                    ink,
                );
                let mark = egui::pos2(chip.right() - 15.0, chip.center().y);
                let tip = if self.languages_open { -2.0 } else { 2.0 };
                let stroke = egui::Stroke::new(1.4, ink);
                painter.line_segment([mark + egui::vec2(-4.0, -tip), mark + egui::vec2(0.0, tip)], stroke);
                painter.line_segment([mark + egui::vec2(4.0, -tip), mark + egui::vec2(0.0, tip)], stroke);
                self.language_anchor = chip.right_bottom();
                if language.clicked() {
                    self.languages_open = !self.languages_open;
                    self.sizes_open = false;
                    self.presets_open = false;
                    self.export.open = false;
                    self.updates.open = false;
                }
            });
        bar
    }

    fn pick(&mut self, scene: Scene, title: String, is_random: bool) {
        self.base = scene;
        self.title = title;
        self.is_random = is_random;
        self.wall.live = false;
        self.requested = None;
        self.view = WHOLE;
        self.layer = 0;
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

        #[cfg(any(target_os = "android", target_os = "ios", target_arch = "wasm32"))]
        {
            widgets::section(ui, &self.t("language"));
            if widgets::row(ui, self.texts.lang.name(), true) {
                self.languages_open = !self.languages_open;
                self.sizes_open = false;
                self.presets_open = false;
            }
        }

        ui.add_space(12.0);
        let (flames, wallpapers) = (self.t("flames"), self.t("wallpapers"));
        let mode = widgets::segments(ui, &[flames, wallpapers], usize::from(self.wall.on)).map(|index| index == 1);
        match mode {
            Some(true) if !self.wall.on => {
                self.wall.on = true;
                self.new_wallpaper();
            }
            Some(false) if self.wall.on => {
                self.wall.on = false;
                if self.wall.live {
                    let name = "aurora-spiral";
                    if let Some(scene) = presets::get(name) {
                        self.pick(named(scene, &shown(name)), name.to_owned(), false);
                    }
                }
            }
            _ => {}
        }

        if self.wall.on {
            self.wallpaper_controls(ui);
        } else {
            widgets::section(ui, &self.t("presets"));
            if widgets::dropdown(ui, &shown(&self.title)) {
                self.open_sheet(Sheet::Presets);
            }
        }

        widgets::section(ui, &self.t("size"));
        let label = match self.picture {
            Some([width, height]) => format!("{width} × {height}"),
            None => self.t("size_window"),
        };
        if widgets::dropdown(ui, &label) {
            self.open_sheet(Sheet::Sizes);
        }

        let flames = !self.wall.live && !self.base.layers.is_empty();
        if flames {
            self.layers(ui);
        }

        let before = self.adjust;
        if flames {
            widgets::section(ui, &self.t("tone"));
            widgets::slider(ui, &self.t("brightness"), &mut self.adjust.brightness, (0.3, 2.5), true);
            widgets::slider(ui, &self.t("gamma"), &mut self.adjust.gamma, (0.5, 1.8), false);
            widgets::slider(ui, &self.t("glow"), &mut self.adjust.glow, (0.0, 3.0), false);
            widgets::section(ui, &self.t("view"));
            widgets::slider(ui, &self.t("zoom"), &mut self.adjust.zoom, (0.4, 2.5), true);

            ui.add_space(8.0);
            if widgets::pill(ui, &self.t("reset"), false, ui.available_width()) {
                self.adjust = NEUTRAL;
            }
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
    }

    fn layers(&mut self, ui: &mut egui::Ui) {
        widgets::section(ui, &self.t("layers"));
        ui.add_space(2.0);
        let before = self.base.layers.clone();
        let count = self.base.layers.len();
        self.layer = self.layer.min(count - 1);
        let blends = ["blend_add", "blend_screen", "blend_over"].map(|key| self.t(key));
        let (opacity, word) = (self.t("opacity"), self.t("layer"));
        let (chosen, mut pick, mut act) = (self.layer, None, None);
        let layers = &mut self.base.layers;

        egui::Frame::NONE
            .fill(egui::Color32::from_white_alpha(10))
            .corner_radius(14)
            .inner_margin(egui::Margin::same(6))
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 4.0;
                for (index, layer) in layers.iter_mut().enumerate() {
                    let label = if layer.name.is_empty() {
                        format!("{word} {}", index + 1)
                    } else {
                        layer.name.clone()
                    };
                    if index != chosen {
                        if widgets::row(ui, &label, false) {
                            pick = Some(index);
                        }
                        continue;
                    }
                    let card = egui::Margin {
                        left: 12,
                        right: 8,
                        top: 6,
                        bottom: 8,
                    };
                    egui::Frame::NONE
                        .fill(egui::Color32::from_white_alpha(26))
                        .corner_radius(10)
                        .inner_margin(card)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                let name = egui::RichText::new(&label)
                                    .font(font(14.0, MEDIUM))
                                    .color(egui::Color32::WHITE);
                                ui.add(egui::Label::new(name).truncate());
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    ui.spacing_mut().item_spacing.x = 4.0;
                                    for (kind, enabled) in
                                        [("remove", count > 1), ("down", index + 1 < count), ("up", index > 0)]
                                    {
                                        if widgets::round(ui, kind, enabled) {
                                            act = Some(kind);
                                        }
                                    }
                                });
                            });
                            ui.add_space(6.0);
                            ui.scope(|ui| {
                                ui.set_width(ui.available_width() - 4.0);
                                let current = [Blend::Add, Blend::Screen, Blend::Over]
                                    .iter()
                                    .position(|blend| *blend == layer.blend)
                                    .unwrap_or(0);
                                if let Some(index) = widgets::segments(ui, &blends, current) {
                                    layer.blend = [Blend::Add, Blend::Screen, Blend::Over][index];
                                }
                                widgets::slider(ui, &opacity, &mut layer.opacity, (0.0, 1.0), false);
                            });
                        });
                }
                if widgets::add_row(ui, count < MAX_LAYERS) {
                    act = Some("add");
                }
            });

        if let Some(index) = pick {
            self.layer = index;
        }
        let at = self.layer;
        match act {
            Some("add") => {
                self.open_sheet(Sheet::Presets);
                self.presets_open = true;
                self.adding_layer = true;
            }
            Some("remove") => {
                self.base.layers.remove(at);
                self.layer = at.min(count - 2);
            }
            Some("up") => {
                self.base.layers.swap(at, at - 1);
                self.layer = at - 1;
            }
            Some("down") => {
                self.base.layers.swap(at, at + 1);
                self.layer = at + 1;
            }
            _ => {}
        }
        if self.base.layers != before {
            self.requested = None;
        }
    }

    fn wallpaper_controls(&mut self, ui: &mut egui::Ui) {
        let any = self.t("any");
        let layout = self
            .wall
            .layout
            .and_then(|layout| Layout::ALL.iter().find(|(_, each)| *each == layout));
        let material = self
            .wall
            .material
            .and_then(|material| Material::ALL.iter().find(|(_, each)| *each == material));

        widgets::section(ui, &self.t("layout"));
        let label = layout.map_or(any.clone(), |(name, _)| self.t(&format!("l_{name}")));
        if widgets::dropdown(ui, &label) {
            self.open_listing(Listing::Layouts);
        }

        widgets::section(ui, &self.t("material"));
        let label = material.map_or(any, |(name, _)| self.t(&format!("m_{name}")));
        if widgets::dropdown(ui, &label) {
            self.open_listing(Listing::Materials);
        }

        ui.add_space(12.0);
        let before = self.wall.dark;
        let tones = [self.t("light"), self.t("dark")];
        if let Some(index) = widgets::segments(ui, &tones, usize::from(self.wall.dark)) {
            self.wall.dark = index == 1;
        }
        if before != self.wall.dark {
            self.wall_changed(true);
        }
    }

    fn open_listing(&mut self, listing: Listing) {
        let again = self.presets_open && self.listing == listing;
        self.open_sheet(Sheet::Presets);
        self.presets_open = !again;
        self.listing = listing;
    }

    fn wall_changed(&mut self, same_shapes: bool) {
        if !same_shapes {
            self.wall.held = None;
        }
        if self.wall.live {
            self.requested = None;
        } else {
            self.new_wallpaper();
        }
    }

    fn wall_sheet(&mut self, ui: &mut egui::Ui, room: f32) {
        let layouts = self.listing == Listing::Layouts;
        widgets::section(ui, &self.t(if layouts { "layout" } else { "material" }));
        let mut bar = egui::style::ScrollStyle::floating();
        bar.bar_width = 5.0;
        ui.spacing_mut().scroll = bar;
        let before = (self.wall.layout, self.wall.material);
        let mut picked = false;
        egui::ScrollArea::vertical()
            .max_height((room - 110.0).max(90.0))
            .show(ui, |ui| {
                if layouts {
                    if widgets::row(ui, &self.t("any"), self.wall.layout.is_none()) {
                        self.wall.layout = None;
                        picked = true;
                    }
                    for (name, layout) in Layout::ALL {
                        if widgets::row(ui, &self.t(&format!("l_{name}")), self.wall.layout == Some(layout)) {
                            self.wall.layout = Some(layout);
                            picked = true;
                        }
                    }
                } else {
                    if widgets::row(ui, &self.t("any"), self.wall.material.is_none()) {
                        self.wall.material = None;
                        picked = true;
                    }
                    for (name, material) in Material::ALL {
                        if widgets::row(ui, &self.t(&format!("m_{name}")), self.wall.material == Some(material)) {
                            self.wall.material = Some(material);
                            picked = true;
                        }
                    }
                }
            });
        if picked {
            self.presets_open = false;
            if before != (self.wall.layout, self.wall.material) {
                self.wall_changed(false);
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
                    if widgets::pill(ui, &labels[index], false, width) {
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

        if let Some(key) = clicked {
            self.act(key);
        }
    }

    fn act(&mut self, key: &str) {
        match Some(key) {
            Some("randomize") if self.wall.on => {
                self.wall.held = None;
                self.new_wallpaper();
            }
            Some("mutate") if self.wall.live => {
                let size = self.requested.unwrap_or([1, 2]);
                self.wall.held = Some(wallpaper_choice(self.seed, &self.wall.request(size)));
                self.new_wallpaper();
            }
            Some("randomize") if self.base.layers.is_empty() => {
                self.seed += 1;
                let title = format!("{} {}", shown(&self.t("random")), self.seed);
                self.pick(named(random_scene(self.seed), &title), title.clone(), true);
            }
            Some("mutate") if self.base.layers.is_empty() => {}
            Some("randomize") => {
                self.seed += 1;
                let title = format!("{} {}", shown(&self.t("random")), self.seed);
                let at = self.layer.min(self.base.layers.len() - 1);
                let fresh = random_scene(self.seed).layers.swap_remove(0);
                self.base.layers[at].flame = fresh.flame;
                self.base.layers[at].name = title.clone();
                if self.base.layers.len() == 1 {
                    self.title = title;
                    self.is_random = true;
                }
                self.view = WHOLE;
                self.requested = None;
            }
            Some("mutate") => {
                self.seed += 1;
                let at = self.layer.min(self.base.layers.len() - 1);
                let mut alone = self.base.clone();
                alone.layers = vec![self.base.layers[at].clone()];
                if let Some(changed) = mutate(&alone, self.seed, 0.25).layers.pop() {
                    self.base.layers[at].flame = changed.flame;
                }
                self.requested = None;
            }
            #[cfg(not(target_arch = "wasm32"))]
            Some("export") => {
                self.presets_open = false;
                self.sizes_open = false;
                self.languages_open = false;
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
            .add_filter("Fractique, flam3", &[SCENE_FILE, "json", "flame"])
            .add_filter("Fractique", &[SCENE_FILE, "json"])
            .add_filter("flam3", &["flame"])
            .pick_file();
        if let Some(path) = picked {
            self.load_scene(&path);
        }
    }

    #[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
    fn load_scene(&mut self, path: &std::path::Path) {
        let name = path
            .file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
            .unwrap_or_default();
        let is_flame = path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("flame"));
        let loaded = std::fs::read_to_string(path)
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
        let name = format!("{}.{SCENE_FILE}", self.title.replace(' ', "-"));
        let Some(path) = rfd::FileDialog::new()
            .set_file_name(name)
            .add_filter("Fractique", &[SCENE_FILE])
            .add_filter("JSON", &["json"])
            .save_file()
        else {
            return;
        };
        let scene = self.scene_at(self.requested.unwrap_or([1, 2]));
        self.status = match std::fs::write(&path, scene.to_json() + "\n") {
            Ok(()) => self.t("saved"),
            Err(error) => format!("{}: {error}", self.t("failed")),
        };
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn export_sheet(&mut self, ui: &mut egui::Ui, window: [u32; 2], room: f32) {
        ui.set_width(300.0);
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

        let shown = self.picture.unwrap_or(window);
        widgets::section(ui, &self.t("size"));
        let own = format!("{} × {}", shown[0], shown[1]);
        let height = (room - 400.0).clamp(34.0, 230.0);
        size_list(
            ui,
            &self.texts,
            &own,
            shown,
            &mut self.export.size,
            &mut self.export.custom,
            height,
        );
        let [width, height] = self.export.size.unwrap_or(shown);
        widgets::section(ui, &self.t("quality"));
        ui.add_space(2.0);
        let qualities = export::QUALITIES.map(|(key, _, _)| self.t(key));
        if let Some(index) = widgets::segments(ui, &qualities, self.export.quality) {
            self.export.quality = index;
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
        let save = self.t(if cfg!(target_os = "ios") {
            "save_photos"
        } else {
            "save_png"
        });
        let cancel = self.t("cancel");
        let half = (ui.available_width() - 8.0) * 0.5;
        let mut clicked = (false, false);
        ui.horizontal(|ui| {
            clicked.0 = widgets::pill(ui, &cancel, false, half);
            clicked.1 = widgets::pill(ui, &save, true, half);
        });
        self.export.share = false;
        if cfg!(target_os = "ios") {
            ui.add_space(8.0);
            if widgets::pill(ui, &self.t("share"), false, ui.available_width()) {
                self.export.share = true;
                clicked.1 = true;
            }
        }
        if clicked.0 {
            self.export.open = false;
        }
        if clicked.1 {
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
                    self.scene_at([width, height]),
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
                #[cfg(target_os = "ios")]
                {
                    tick(true);
                    if self.export.share {
                        ios::share(&path);
                    } else if ios::save_to_photos(&path) {
                        self.export.note = self.t("saved");
                    }
                    self.export.open = false;
                }
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

fn tick(strong: bool) {
    #[cfg(target_os = "ios")]
    ios::tick(strong);
    #[cfg(not(target_os = "ios"))]
    let _ = strong;
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
        self.explore(ui, screen);
        #[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
        let content = {
            self.window_keys(&ctx);
            let mut content = safe_content(&ctx);
            if !self.bare {
                content.min.y += BAR + BAR_GAP;
            }
            content
        };
        #[cfg(any(target_os = "android", target_os = "ios", target_arch = "wasm32"))]
        let content = {
            let mut content = safe_content(&ctx);
            // The on-screen keyboard covers the lower part of a phone's screen and does not report its size.
            if cfg!(any(target_os = "android", target_os = "ios")) && ctx.egui_wants_keyboard_input() {
                content.max.y = content.min.y + content.height() * 0.56;
            }
            content
        };
        let scale = ctx.pixels_per_point();
        let size = [
            (screen.width() * scale).round() as u32,
            (screen.height() * scale).round() as u32,
        ];
        let window = size;
        let size = self.picture.unwrap_or(window);
        if window[0] > 0 && window[1] > 0 {
            if self.requested != Some(size) {
                self.start_render(&ctx, size);
            } else if self.drawn != Some(self.view) {
                let still = self.touched.elapsed().as_secs_f32();
                if still >= SETTLE {
                    self.start_render(&ctx, size);
                } else {
                    ctx.request_repaint_after(std::time::Duration::from_secs_f32(SETTLE - still));
                }
            }
        }
        let stretch = (size[0] as f32 / size[1] as f32) / (screen.width() / screen.height()).max(0.001);

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
        let mut glass = glass::Frame::new(&painter, screen, {
            let [across, along, larger] = self.view.within(&self.shown, size);
            [across, along, larger, stretch]
        });
        for (rect, radius, dark) in &self.panes {
            let mut optics = Optics {
                radius: *radius,
                ..self.optics
            };
            if *dark {
                optics.tint = optics.tint.max(0.82);
                optics.blur = 1.0;
            }
            if self.phone.clear.contains(rect) {
                optics.tint *= 0.25;
                optics.blur *= 0.5;
            } else if self.light > 0.3 {
                // The texts are white: over a light picture the glass has to be dark enough to read them.
                optics.tint = optics.tint.max((0.3 + 0.75 * self.light).min(0.92));
            }
            glass.pane(*rect, optics);
        }

        if self.bare || self.phone.preview {
            if self.phone.preview {
                self.clock(&ctx, content);
            }
            let mut panes = Vec::new();
            if self.view != WHOLE {
                let rect = self.reset_button(&ctx, egui::pos2(content.center().x, content.bottom() - 20.0));
                panes.push((rect, rect.height() * 0.5, false));
            }
            if panes != self.panes {
                self.panes = panes;
                ctx.request_repaint();
            }
            if self.screenshot.is_some() {
                self.take_screenshot(&ctx);
            }
            return;
        }

        let wide = content.width() >= 700.0;
        let open = *self.inspector_open.get_or_insert(wide);
        // A sheet's glass is drawn under every text, so a panel it would cover hides while the sheet is open.
        #[cfg(not(target_arch = "wasm32"))]
        let sheet = self.export.open || self.sizes_open || self.presets_open;
        #[cfg(target_arch = "wasm32")]
        let sheet = self.sizes_open || self.presets_open;
        #[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
        let sheet = sheet || self.updates.open;
        let sheet = sheet
            || (self.languages_open && cfg!(any(target_os = "android", target_os = "ios", target_arch = "wasm32")));
        let open = open && !(sheet && content.width() < 1320.0);
        let column = (content.width() - 84.0).min(248.0);
        let beside = content.left() + 20.0 + column + 44.0 + 16.0;
        let fits = wide && open && content.center().x - self.toolbar_size.x * 0.5 >= beside;
        let lifted = if self.view != WHOLE { 56.0 } else { 0.0 };
        let under = if wide && (fits || !open) {
            0.0
        } else {
            self.toolbar_size.y.max(60.0) + 12.0 + lifted
        };
        let mut panes = Vec::new();
        if self.phone.on {
            let covered = sheet || self.languages_open;
            self.phone_home(&ctx, content, size, &mut panes, covered);
        } else {
            #[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
            panes.push((self.title_bar(&ctx, screen), BAR * 0.5, false));
            if open {
                let corner = content.min + egui::vec2(20.0, 20.0);
                let inspector = egui::Area::new(egui::Id::unique("inspector"))
                    .fixed_pos(corner)
                    .show(&ctx, |ui| {
                        let margin = egui::Margin {
                            left: 22,
                            right: 8,
                            top: 22,
                            bottom: 22,
                        };
                        egui::Frame::NONE.inner_margin(margin).show(ui, |ui| {
                            let room = content.height() - 40.0 - 44.0 - under;
                            let mut bar = egui::style::ScrollStyle::floating();
                            bar.bar_width = 5.0;
                            bar.floating_allocated_width = 0.0;
                            ui.spacing_mut().scroll = bar;
                            egui::ScrollArea::vertical()
                                .max_height(room)
                                .min_scrolled_height(room)
                                .show(ui, |ui| {
                                    let gutter = egui::Margin {
                                        right: 14,
                                        ..Default::default()
                                    };
                                    egui::Frame::NONE
                                        .inner_margin(gutter)
                                        .show(ui, |ui| self.inspector(ui, column));
                                });
                        });
                    });
                panes.push((inspector.response.rect, self.optics.radius, false));
            }

            let covered = sheet || self.languages_open;
            if !covered {
                let toolbar = egui::Area::new(egui::Id::unique("toolbar"))
                    .fixed_pos(egui::pos2(content.center().x, content.bottom() - 20.0))
                    .pivot(egui::Align2::CENTER_BOTTOM)
                    .show(&ctx, |ui| {
                        egui::Frame::NONE.inner_margin(egui::Margin::same(9)).show(ui, |ui| {
                            let detail = if self.rendering {
                                self.t("rendering")
                            } else {
                                self.status.clone()
                            };
                            let name = ui.painter().layout_no_wrap(
                                shown(&self.title),
                                font(13.5, MEDIUM),
                                egui::Color32::WHITE,
                            );
                            let detail = ui.painter().layout_no_wrap(
                                detail,
                                egui::FontId::proportional(13.0),
                                egui::Color32::from_white_alpha(170),
                            );
                            let width = (self.toolbar_size.x - 18.0).max(name.size().x + 10.0 + detail.size().x);
                            let (line, _) = ui.allocate_exact_size(egui::vec2(width, 24.0), egui::Sense::hover());
                            let left = line.center().x - (name.size().x + 10.0 + detail.size().x) * 0.5;
                            let after = left + name.size().x + 10.0;
                            ui.painter().galley(
                                egui::pos2(left, line.center().y - name.size().y * 0.5),
                                name,
                                egui::Color32::WHITE,
                            );
                            ui.painter().galley(
                                egui::pos2(after, line.center().y - detail.size().y * 0.5),
                                detail,
                                egui::Color32::WHITE,
                            );
                            ui.add_space(5.0);
                            self.toolbar(ui, wide, content.width() - 58.0);
                        });
                    });
                self.toolbar_size = toolbar.response.rect.size();
                panes.push((
                    toolbar.response.rect,
                    (toolbar.response.rect.height() * 0.5).min(28.0),
                    false,
                ));
            }

            if self.view != WHOLE && !covered {
                let above = content.bottom() - 20.0 - self.toolbar_size.y - 10.0;
                let rect = self.reset_button(&ctx, egui::pos2(content.center().x, above));
                panes.push((rect, rect.height() * 0.5, false));
            }
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
                        .show(ui, |ui| self.export_sheet(ui, window, content.height()));
                });
            panes.push((sheet.response.rect, self.optics.radius, true));
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
            panes.push((sheet.response.rect, self.optics.radius, true));
        }

        if self.presets_open {
            let sheet = egui::Area::new(egui::Id::unique("presets"))
                .fixed_pos(content.center())
                .pivot(egui::Align2::CENTER_CENTER)
                .order(egui::Order::Foreground)
                .show(&ctx, |ui| {
                    egui::Frame::NONE
                        .inner_margin(egui::Margin::same(22))
                        .show(ui, |ui| self.preset_sheet(ui, content.height()));
                });
            panes.push((sheet.response.rect, self.optics.radius, true));
        }

        if self.sizes_open {
            let sheet = egui::Area::new(egui::Id::unique("sizes"))
                .fixed_pos(content.center())
                .pivot(egui::Align2::CENTER_CENTER)
                .order(egui::Order::Foreground)
                .show(&ctx, |ui| {
                    egui::Frame::NONE
                        .inner_margin(egui::Margin::same(22))
                        .show(ui, |ui| self.size_sheet(ui, window, content.height()));
                });
            panes.push((sheet.response.rect, self.optics.radius, true));
        }

        if self.languages_open {
            let (at, pivot) = if cfg!(any(target_os = "android", target_os = "ios", target_arch = "wasm32")) {
                (content.center(), egui::Align2::CENTER_CENTER)
            } else {
                (self.language_anchor + egui::vec2(0.0, 12.0), egui::Align2::RIGHT_TOP)
            };
            let mut chosen = self.texts.lang;
            let menu = egui::Area::new(egui::Id::unique("languages"))
                .fixed_pos(at)
                .pivot(pivot)
                .order(egui::Order::Foreground)
                .show(&ctx, |ui| {
                    egui::Frame::NONE.inner_margin(egui::Margin::same(10)).show(ui, |ui| {
                        ui.set_width(196.0);
                        ui.spacing_mut().item_spacing = egui::vec2(8.0, 2.0);
                        let mut bar = egui::style::ScrollStyle::floating();
                        bar.bar_width = 5.0;
                        ui.spacing_mut().scroll = bar;
                        egui::ScrollArea::vertical()
                            .max_height(content.height() - 110.0)
                            .show(ui, |ui| {
                                for lang in Lang::ALL {
                                    if widgets::row(ui, lang.name(), lang == chosen) {
                                        chosen = lang;
                                        self.languages_open = false;
                                    }
                                }
                            });
                    });
                });
            if chosen != self.texts.lang {
                self.texts = Texts::new(chosen);
                widgets::install_fonts(&ctx, chosen);
            }
            panes.push((menu.response.rect, 22.0, true));
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
            panes.push((tuning.response.rect, self.optics.radius, false));
        }

        // The panes are drawn under the widgets, so they use the places measured one frame earlier.
        if panes != self.panes {
            self.panes = panes;
            ctx.request_repaint();
        }
        if self.screenshot.is_some() {
            self.take_screenshot(&ctx);
        }

        // A phone ends an app without telling it, so the session is written when it changes, not at exit.
        if self.screenshot.is_none()
            && !ctx.input(|input| input.pointer.any_down())
            && let Some(storage) = frame.storage_mut()
        {
            let settings = format!(
                "{} {} {} {} {} {:?} {}",
                self.phone.hints,
                self.phone.kept.len(),
                self.texts.lang.code(),
                self.optics.tint,
                self.optics.blur,
                self.picture,
                self.session()
            );
            if settings == self.saved {
                return;
            }
            self.saved = settings;
            eframe::App::save(self, storage);
            storage.flush();
        }
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        storage.set_string("language", self.texts.lang.code().to_owned());
        storage.set_string("glass_opacity", self.optics.tint.to_string());
        storage.set_string("glass_blur", self.optics.blur.to_string());
        let picture = self
            .picture
            .map_or("auto".to_owned(), |[width, height]| format!("{width}x{height}"));
        storage.set_string("picture", picture);
        storage.set_string("session", self.session());
        storage.set_string("hints", self.phone.hints.to_string());
        storage.set_string("kept", self.phone.kept_text());
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
    // eframe finds no settings folder of its own on a phone.
    #[cfg(any(target_os = "android", target_os = "ios"))]
    if options.persistence_path.is_none()
        && let Some(folder) = own_folder()
    {
        let _ = std::fs::create_dir_all(&folder);
        options.persistence_path = Some(folder.join("settings.ron"));
    }
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

// iOS 27 refuses to start an app without a scene; winit's window does not join one by itself.
#[cfg(target_os = "ios")]
fn join_scene(cc: &eframe::CreationContext<'_>) {
    use objc2::runtime::AnyObject;
    use objc2::{class, msg_send};
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};

    let Ok(handle) = cc.window_handle() else { return };
    let RawWindowHandle::UiKit(handle) = handle.as_raw() else {
        return;
    };
    let view = handle.ui_view.as_ptr().cast::<AnyObject>();
    unsafe {
        let window: *mut AnyObject = msg_send![view, window];
        if window.is_null() {
            return;
        }
        let joined: *mut AnyObject = msg_send![window, windowScene];
        if !joined.is_null() {
            return;
        }
        let app: *mut AnyObject = msg_send![class!(UIApplication), sharedApplication];
        let scenes: *mut AnyObject = msg_send![app, connectedScenes];
        let scene: *mut AnyObject = msg_send![scenes, anyObject];
        if scene.is_null() {
            return;
        }
        let _: () = msg_send![window, setWindowScene: scene];
        let _: () = msg_send![window, makeKeyAndVisible];
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kept_layers_give_the_same_picture() {
        let scene = presets::get("solar-mandala").expect("the preset exists");
        let options = RenderOptions {
            width: 96,
            height: 64,
            samples_per_pixel: 8,
            supersample: 1,
        };
        let control = Control {
            cancel: None,
            progress: None,
        };
        let kept = Layers::default();
        let whole = fractique_core::render(&scene, &options, &control).expect("it renders");
        let first = rendered(&scene, &options, &control, &kept, true).expect("it renders");
        let again = rendered(&scene, &options, &control, &kept, true).expect("it renders");
        assert_eq!(first.rgba, whole.rgba);
        assert_eq!(again.rgba, whole.rgba);
        assert_eq!(kept.lock().expect("no thread panicked").len(), scene.layers.len());

        let mut nearer = scene.clone();
        for layer in &mut nearer.layers {
            layer.flame.camera.zoom *= 2.0;
        }
        rendered(&nearer, &options, &control, &kept, false).expect("it renders");
        let both = kept.lock().expect("no thread panicked");
        assert_eq!(both.len(), 2 * scene.layers.len());
        assert_eq!(both.values().filter(|entry| entry.home).count(), scene.layers.len());
    }
}

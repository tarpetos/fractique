#[cfg(target_os = "android")]
mod android;
#[cfg(any(windows, test))]
mod association;
mod blur;
mod chaos;
#[cfg(not(target_arch = "wasm32"))]
mod export;
mod glass;
mod i18n;
#[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
mod instance;
#[cfg(target_os = "ios")]
mod ios;
mod orbit;
mod phone;
mod preview;
#[cfg(any(feature = "store", test))]
mod store;
#[cfg(all(feature = "store", any(target_os = "ios", target_os = "macos")))]
mod storekit;
#[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
mod update;
#[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
mod updates;
mod widgets;

use std::collections::HashMap;
use std::path::PathBuf;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, mpsc};

use web_time::Instant;

use eframe::egui;
use eframe::egui_wgpu::{self, wgpu};
use fractique_core::palette;
use fractique_core::random::{Generator, mutate, random_scene, random_scene_of};
use fractique_core::scene::{Blend, Layer, MAX_LAYERS};
use fractique_core::wallpaper::{Layout, Material, Request, chosen as wallpaper_choice, random_wallpaper};
use fractique_core::{
    Control, Developed, Image, RenderError, RenderOptions, Scene, Tone, compose, develop, fractal, presets,
    render_density, render_layer,
};

use chaos::{Chaos, Step};
use glass::{Gfx, Optics};
use i18n::{Lang, Texts};
use preview::{Preview, Source};
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

#[derive(Clone)]
struct Held {
    size: [u32; 2],
    options: RenderOptions,
    keys: Vec<String>,
    sources: Vec<Arc<Source>>,
}

struct Rendered {
    generation: u64,
    draft: bool,
    view: View,
    zoom: f64,
    held: Option<Held>,
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

const SIZES: &[(&str, [u32; 2], Family)] = &[
    ("HD", [1280, 720], Family::Screen),
    ("Full HD", [1920, 1080], Family::Screen),
    ("QHD", [2560, 1440], Family::Screen),
    ("Ultrawide", [3440, 1440], Family::Screen),
    ("4K", [3840, 2160], Family::Screen),
    ("Android HD+", [720, 1600], Family::Android),
    ("Android HD+ 19.5:9", [720, 1560], Family::Android),
    ("Android Full HD", [1080, 1920], Family::Android),
    ("Android Full HD+", [1080, 2400], Family::Android),
    ("Galaxy S", [1080, 2340], Family::Android),
    ("Pixel", [1080, 2424], Family::Android),
    ("Xiaomi 1.5K", [1220, 2712], Family::Android),
    ("Pixel Pro", [1280, 2856], Family::Android),
    ("Pixel Pro XL", [1344, 2992], Family::Android),
    ("Galaxy S Ultra", [1440, 3120], Family::Android),
    ("Android QHD+", [1440, 3200], Family::Android),
    ("Android tablet", [1600, 2560], Family::Android),
    // Native pixels from Apple's Tech Specs pages (support.apple.com), read in November 2026: docs/usage.md.
    ("iPhone SE", [750, 1334], Family::IPhone),
    ("iPhone 11", [828, 1792], Family::IPhone),
    ("iPhone mini", [1080, 2340], Family::IPhone),
    ("iPhone 11 Pro", [1125, 2436], Family::IPhone),
    ("iPhone 11 Pro Max", [1242, 2688], Family::IPhone),
    ("iPhone 12–14, 17e", [1170, 2532], Family::IPhone),
    ("iPhone 14 Plus", [1284, 2778], Family::IPhone),
    ("iPhone 15, 16", [1179, 2556], Family::IPhone),
    ("iPhone 15–16 Plus", [1290, 2796], Family::IPhone),
    ("iPhone 17, 18 Pro", [1206, 2622], Family::IPhone),
    ("iPhone Air", [1260, 2736], Family::IPhone),
    ("iPhone 18 Pro Max", [1320, 2868], Family::IPhone),
    ("iPhone Duo 5.4″", [1398, 2034], Family::IPhone),
    ("iPhone Duo 7.6″", [1878, 2670], Family::IPhone),
    ("iPad mini", [1488, 2266], Family::IPad),
    ("iPad mini (A20 Pro)", [1512, 2316], Family::IPad),
    ("iPad 10.2″", [1620, 2160], Family::IPad),
    ("iPad, iPad Air 11″", [1640, 2360], Family::IPad),
    ("iPad Pro 10.5″", [1668, 2224], Family::IPad),
    ("iPad Pro 11″ 2018–22", [1668, 2388], Family::IPad),
    ("iPad Pro 11″", [1668, 2420], Family::IPad),
    ("iPad Air 13″", [2048, 2732], Family::IPad),
    ("iPad Pro 13″", [2064, 2752], Family::IPad),
    ("MacBook Neo", [2408, 1506], Family::Mac),
    ("MacBook Air 13″ M1", [2560, 1600], Family::Mac),
    ("MacBook Air 13″", [2560, 1664], Family::Mac),
    ("MacBook Air 15″", [2880, 1864], Family::Mac),
    ("MacBook Pro 14″", [3024, 1964], Family::Mac),
    ("MacBook 14″ OLED", [3024, 1986], Family::Mac),
    ("MacBook Pro 16″", [3456, 2234], Family::Mac),
    ("MacBook 16″ OLED", [3472, 2266], Family::Mac),
    ("iMac 24″", [4480, 2520], Family::Mac),
    ("Studio Display, 5K", [5120, 2880], Family::Mac),
    ("Pro Display XDR", [6016, 3384], Family::Mac),
    ("Watch 40 mm", [324, 394], Family::Watch),
    ("Watch 41 mm", [352, 430], Family::Watch),
    ("Watch 42 mm", [374, 446], Family::Watch),
    ("Watch 44 mm", [368, 448], Family::Watch),
    ("Watch 45 mm", [396, 484], Family::Watch),
    ("Watch 46 mm", [416, 496], Family::Watch),
    ("Watch Ultra", [410, 502], Family::Watch),
    ("Watch Ultra 3", [422, 514], Family::Watch),
];

#[derive(Clone, Copy, PartialEq, Debug)]
enum Family {
    Screen,
    IPhone,
    IPad,
    Mac,
    Watch,
    Android,
}

impl Family {
    const ALL: [Family; 6] = [
        Family::Screen,
        Family::IPhone,
        Family::IPad,
        Family::Mac,
        Family::Watch,
        Family::Android,
    ];

    fn label(self, texts: &Texts) -> String {
        match self {
            Family::Screen => texts.get("size_screens"),
            Family::IPhone => "iPhone".to_owned(),
            Family::IPad => "iPad".to_owned(),
            Family::Mac => "Mac".to_owned(),
            Family::Watch => "Watch".to_owned(),
            Family::Android => "Android".to_owned(),
        }
    }
}

fn offered(family: Family, store: bool) -> bool {
    family != Family::Android || !store
}

fn sizes() -> impl Iterator<Item = (&'static str, [u32; 2])> {
    SIZES
        .iter()
        .filter(|(_, _, family)| offered(*family, cfg!(feature = "store")))
        .map(|(name, size, _)| (*name, *size))
}
const LARGEST_SIDE: u32 = 16384;

fn size_list(
    ui: &mut egui::Ui,
    texts: &Texts,
    own: &str,
    base: [u32; 2],
    chosen: &mut Option<[u32; 2]>,
    custom: &mut [String; 2],
    list: List<'_>,
) -> Pick {
    let (height, locks, locked) = (list.height, list, list.locked);
    let mut picked = Pick::None;
    if widgets::locked_row(ui, own, chosen.is_none(), locked(None)) {
        *chosen = None;
        picked = Pick::Chosen;
    }
    let listed = chosen.is_some_and(is_listed);
    let groups: Vec<Family> = Family::ALL
        .into_iter()
        .filter(|family| offered(*family, cfg!(feature = "store")))
        .collect();
    let remembered = egui::Id::unique("size_group");
    let of_chosen = || {
        let size = (*chosen)?;
        let family = SIZES.iter().find(|(_, listed, _)| *listed == size)?.2;
        groups.iter().position(|group| *group == family)
    };
    let mut group = ui
        .data(|data| data.get_temp::<usize>(remembered))
        .or_else(of_chosen)
        .unwrap_or(0)
        .min(groups.len() - 1);
    ui.add_space(4.0);
    for (row, part) in groups.chunks(3).enumerate() {
        let labels: Vec<String> = part.iter().map(|family| family.label(texts)).collect();
        let at = group
            .checked_sub(row * 3)
            .filter(|at| *at < part.len())
            .unwrap_or(usize::MAX);
        if let Some(at) = widgets::segments(ui, &labels, at) {
            group = row * 3 + at;
        }
        ui.add_space(2.0);
    }
    ui.data_mut(|data| data.insert_temp(remembered, group));
    let family = groups[group];
    let height = (height - 76.0).max(64.0);
    let mut bar = egui::style::ScrollStyle::floating();
    bar.bar_width = 5.0;
    ui.spacing_mut().scroll = bar;
    let scroll = egui::ScrollArea::vertical()
        .id_salt(("sizes", group))
        .max_height(height)
        .min_scrolled_height(height);
    scroll.show(ui, |ui| {
        for (name, size, _) in SIZES.iter().filter(|(_, _, of)| *of == family) {
            let (name, size) = (*name, *size);
            let label = format!("{} × {}  ·  {name}", size[0], size[1]);
            if widgets::locked_row(ui, &label, *chosen == Some(size), locked(Some(size))) {
                *chosen = Some(size);
                picked = Pick::Chosen;
            }
        }
    });
    if custom[0].is_empty() {
        let [width, height] = chosen.unwrap_or(base);
        *custom = [width.to_string(), height.to_string()];
    }
    ui.add_space(6.0);
    let entry = parsed_size(&format!("{}x{}", custom[0], custom[1]));
    let mut entered = widgets::locked_row(
        ui,
        &texts.get("size_custom"),
        chosen.is_some() && !listed,
        locks.custom || entry.is_some_and(|size| locked(Some(size))),
    ) && locks.custom;
    ui.add_space(4.0);
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
            size if locks.custom => picked = Pick::Custom(size),
            Some(size) => {
                *chosen = Some(size);
                picked = Pick::Chosen;
            }
            None => *custom = Default::default(),
        }
    }
    picked
}

fn is_listed(size: [u32; 2]) -> bool {
    sizes().any(|(_, listed)| listed == size)
}

#[derive(Clone, Copy)]
struct List<'a> {
    height: f32,
    locked: &'a dyn Fn(Option<[u32; 2]>) -> bool,
    custom: bool,
}

#[derive(Clone, Copy, PartialEq)]
enum Pick {
    None,
    Chosen,
    // A custom size was asked for where it needs the purchase.
    Custom(Option<[u32; 2]>),
}

#[derive(Clone, Copy, PartialEq)]
enum Sheet {
    Presets,
    Sizes,
}

#[cfg(feature = "store")]
#[derive(Clone, Copy, PartialEq)]
enum Wanted {
    Nothing,
    Export { size: [u32; 2], share: bool },
    ExportSize(Option<[u32; 2]>),
    Picture([u32; 2]),
}

#[derive(Clone, Copy, PartialEq)]
enum Listing {
    Presets,
    Kinds,
    Layouts,
    Materials,
    Recent,
}

fn shown(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for word in name.split(['-', '_']) {
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

fn file_name(title: &str) -> String {
    let mut out = String::with_capacity(title.len());
    for word in title.split(|letter: char| !letter.is_alphanumeric()) {
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
// The app's own files: see `document`.
#[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
const SCENE_FILE: &str = "frcq";
#[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
static OPEN_FIRST: OnceLock<PathBuf> = OnceLock::new();

#[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
pub fn open_at_start(path: PathBuf) {
    let _ = OPEN_FIRST.set(path);
}

#[cfg(windows)]
pub fn register_scene_files() {
    association::register();
}

#[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
pub fn first_instance(open: Option<&std::path::Path>) -> bool {
    instance::first(open)
}

// The date and the time as the lock screen shows them; where the system is not asked, the time of Apple's pictures.
fn lock_screen_texts() -> (String, String) {
    #[cfg(target_os = "ios")]
    {
        let (date, time) = ios::clock();
        let digits: String = time
            .chars()
            .take_while(|c| c.is_ascii_digit() || matches!(c, ':' | '.'))
            .collect();
        (date, if digits.is_empty() { time } else { digits })
    }
    #[cfg(not(target_os = "ios"))]
    (String::new(), "9:41".to_owned())
}

const GLASS: (f32, f32) = (0.4, 0.7);
const MAX_MAGNIFICATION: f64 = 1e10;
const MIN_MAGNIFICATION: f64 = 1e-4;
// A new picture is drawn once the view has stood still this long.
const SETTLE: f32 = 0.18;

impl View {
    fn moved(&mut self, by: [f64; 2]) {
        self.right = (self.right - by[0] / self.zoom).clamp(-1e6, 1e6);
        self.down = (self.down - by[1] / self.zoom).clamp(-1e6, 1e6);
    }

    fn magnified(&mut self, factor: f64, around: [f64; 2], most: f64) {
        let held = [self.right + around[0] / self.zoom, self.down + around[1] / self.zoom];
        self.zoom = (self.zoom * factor).clamp(MIN_MAGNIFICATION, most);
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

    // The same view of a camera that magnifies `by` times more.
    fn scaled(&self, by: f64) -> View {
        View {
            right: self.right / by,
            down: self.down / by,
            zoom: self.zoom * by,
        }
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

fn shows(what: &str) -> bool {
    std::env::var("FRACTIQUE_SHOW").is_ok_and(|shown| shown == what)
}

// Whether the graphics card is used. It is where a trial has shown it clearly faster than the processor.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Verdict {
    Unknown,
    // With the samples a second of the processor in the trial: what a run of the card is held against.
    Fast(Option<f64>),
    Slow,
}

// The trial: one flame drawn by the graphics card, then rendered by the processor. The processor's part counts
// when no other render ran beside it.
struct Trial {
    card: Option<f32>,
    processor: Option<(mpsc::Receiver<f32>, Arc<AtomicBool>)>,
    runs: u8,
    best: f32,
}

// The card's best of this many runs counts: one run is a few hundredths of a second, and anything else the
// machine does at that moment would decide it.
const TRIAL_RUNS: u8 = 3;
// A run this long is not noise, and is not repeated; a card that needs longer than the second number is slow.
const TRIAL_QUICK: f32 = 0.25;
const TRIAL_LONGEST: f32 = 3.0;

// A smaller picture is over before the card's fixed costs stop counting: a phone's card then looks slower than it is.
const TRIAL_SIZE: [u32; 2] = [1280, 720];
// The card is used where it ends the trial at least this many times sooner than the processor.
// Measured on an Adreno 740, which is 1.7 times as fast at 800 × 450 and clearly quicker to use.
const CLEARLY: f32 = 1.3;
// A run of the card may take as long as the processor is expected to, and at least this many seconds.
const PATIENCE: f64 = 0.3;

fn trial_options() -> RenderOptions {
    RenderOptions {
        width: TRIAL_SIZE[0],
        height: TRIAL_SIZE[1],
        samples_per_pixel: FULL_SAMPLES,
        supersample: 1,
    }
}

// A test run that keeps the picture of the graphics card: all of its samples, or those of its first batch.
fn chaos_only() -> bool {
    shows("gpu") || shows("gpu-first")
}

// A test run that shows the controls scrolled down by FRACTIQUE_SCROLL points: the graphics card's switch is
// near their end.
fn scrolled(area: egui::ScrollArea) -> egui::ScrollArea {
    match std::env::var("FRACTIQUE_SCROLL").ok().and_then(|by| by.parse().ok()) {
        Some(by) => area.vertical_scroll_offset(by),
        None => area,
    }
}

fn timed() -> bool {
    std::env::var_os("FRACTIQUE_DEBUG_TIMES").is_some()
}

impl Adjust {
    // A test run's tone: FRACTIQUE_ADJUST=brightness,gamma,glow,zoom. With FRACTIQUE_SHOW=preview it is set once
    // the picture is rendered and stays the graphics card's.
    fn asked() -> Option<Adjust> {
        let text = std::env::var("FRACTIQUE_ADJUST").ok()?;
        let numbers: Vec<f64> = text.split(',').filter_map(|number| number.parse().ok()).collect();
        let &[brightness, gamma, glow, zoom] = numbers.as_slice() else {
            return None;
        };
        Some(Adjust {
            brightness,
            gamma,
            glow,
            zoom,
        })
    }
}

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

// What a tab showed when the other one was chosen, to come back to as it was.
struct Rest {
    base: Scene,
    title: String,
    is_random: bool,
    seed: u64,
    view: View,
    layer: usize,
    held: Option<(Layout, Material)>,
    picture: Option<(Rendered, [u32; 2])>,
}

struct App {
    texts: Texts,
    base: Scene,
    wall: Wall,
    rest: [Option<Rest>; 3],
    fractals: bool,
    kind: String,
    latest: Option<Rendered>,
    palette: Option<usize>,
    phone: phone::Phone,
    title: String,
    is_random: bool,
    seed: u64,
    generator: Generator,
    adjust: Adjust,
    view: View,
    shown: View,
    shown_zoom: f64,
    drawn: Option<(View, f64)>,
    touched: Instant,
    gpu: bool,
    preview: Option<Preview>,
    card: String,
    card_key: String,
    verdict: Verdict,
    measured_slow: bool,
    trial: Option<Trial>,
    held: Option<Held>,
    previewing: bool,
    chaos: Option<Chaos>,
    chaos_tried: bool,
    compiling: bool,
    chaotic: bool,
    chaos_at: (View, f64),
    quick: (Option<f32>, Option<f32>),
    look: String,
    asked: Instant,
    measured: Arc<AtomicU32>,
    #[cfg(not(target_arch = "wasm32"))]
    instant: Option<(export::Later, [u32; 2])>,
    #[cfg(not(target_arch = "wasm32"))]
    export_at: Option<PathBuf>,
    #[cfg(not(target_arch = "wasm32"))]
    export_asked: Instant,
    frozen: bool,
    bare: bool,
    optics: Optics,
    tuning: bool,
    inspector_open: Option<bool>,
    panes: Vec<(egui::Rect, f32, bool)>,
    toolbar_size: egui::Vec2,
    languages_open: bool,
    picture: Option<[u32; 2]>,
    custom: [String; 2],
    // A text of a fractal while it is typed, by its layer and name: what is typed need not be valid yet.
    typed: Option<(String, String)>,
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
    #[cfg(target_os = "android")]
    keeping: Option<(PathBuf, bool, u8)>,
    #[cfg(target_os = "android")]
    waiting: Option<(PathBuf, bool)>,
    #[cfg(target_os = "android")]
    focused: bool,
    #[cfg(target_os = "android")]
    asking: Option<([u32; 2], bool)>,
    last: Option<(u64, Arc<Image>)>,
    last_frame: Instant,
    #[cfg(not(target_arch = "wasm32"))]
    export: export::Sheet,
    #[cfg(feature = "store")]
    store: Option<store::Store>,
    #[cfg(feature = "store")]
    pro: Option<Wanted>,
    #[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
    updates: updates::Updates,
    #[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
    arrivals: Option<mpsc::Receiver<Option<PathBuf>>>,
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

struct Kept<T> {
    item: Arc<T>,
    used: u64,
    home: bool,
}

#[derive(Default)]
struct Store {
    layers: HashMap<String, Kept<Developed>>,
    sources: HashMap<String, Kept<Source>>,
}

type Layers = Arc<Mutex<Store>>;
type Sources = (Vec<String>, Vec<Arc<Source>>);

const KEPT_BYTES: usize = if cfg!(any(target_os = "android", target_os = "ios", target_arch = "wasm32")) {
    160 << 20
} else {
    1200 << 20
};

fn size_key(options: &RenderOptions) -> String {
    format!(
        "{} {} {} {} ",
        options.width, options.height, options.samples_per_pixel, options.supersample
    )
}

// What a layer draws, without its name, its blend and its opacity.
fn content_key(layer: &Layer) -> String {
    let bare = Layer {
        name: String::new(),
        blend: Blend::Add,
        opacity: 1.0,
        content: layer.content.clone(),
    };
    serde_json::to_string(&bare).unwrap_or_default()
}

// What a layer's density depends on: its flame without the tone, but with the smoothing.
fn source_key(scene: &Scene, index: usize, size: &str) -> String {
    let mut layer = scene.layers[index].clone();
    *layer.tone_mut() = Tone {
        smoothing: layer.tone().smoothing,
        ..Tone::default()
    };
    format!("{size}{} {index} {}", scene.seed, content_key(&layer))
}

// The graphics card previews the layers the processor rendered when all of them are flames.
fn all_flames(scene: &Scene) -> bool {
    scene.sheets.is_none() && scene.layers.iter().all(|layer| layer.flame().is_some())
}

// The graphics card plays the chaos game for flames and walks the orbits of the kinds it has a shader for; a scene
// with any other layer is the processor's.
fn all_densities(scene: &Scene) -> bool {
    let drawn = |layer: &Layer| layer.fractal().is_none_or(orbit::drawn);
    scene.sheets.is_none() && scene.layers.iter().all(drawn)
}

// A layer's pixels depend only on its flame, its place in the scene, the seed and the size, so a layer that did
// not change is taken from an earlier render. The layers of the whole view are the last to be dropped: Reset view
// comes back to them. With `sources` the densities are kept too: a layer whose tone alone changed is developed
// from its density, and the graphics card previews from them.
fn rendered(
    scene: &Scene,
    options: &RenderOptions,
    control: &Control<'_>,
    kept: &Layers,
    home: bool,
    sources: bool,
) -> Result<(Image, Option<Sources>), RenderError> {
    static USES: AtomicU64 = AtomicU64::new(0);
    let size = size_key(options);
    let (mut keys, mut layers) = (Vec::new(), Vec::new());
    let (mut source_keys, mut held) = (Vec::new(), Vec::new());
    for (index, layer) in scene.layers.iter().enumerate() {
        let key = format!("{size}{} {index} {}", scene.seed, content_key(layer));
        let used = USES.fetch_add(1, Ordering::Relaxed);
        let found = kept
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .layers
            .get_mut(&key)
            .map(|entry| {
                entry.used = used;
                entry.home |= home;
                entry.item.clone()
            });
        let source_key = sources.then(|| source_key(scene, index, &size));
        let mut source = source_key.as_ref().and_then(|key| {
            let mut kept = kept.lock().unwrap_or_else(|e| e.into_inner());
            kept.sources.get_mut(key).map(|entry| {
                entry.used = used;
                entry.home |= home;
                entry.item.clone()
            })
        });
        let developed = match found {
            Some(developed) => developed,
            None => {
                let layer = match &source_key {
                    None => render_layer(scene, index, options, control)?,
                    Some(source_key) => {
                        let made = match source.take() {
                            Some(made) => made,
                            None => {
                                let made = Arc::new(Source::new(render_density(scene, index, options, control)?));
                                let entry = Kept {
                                    item: made.clone(),
                                    used,
                                    home,
                                };
                                kept.lock()
                                    .unwrap_or_else(|e| e.into_inner())
                                    .sources
                                    .insert(source_key.clone(), entry);
                                made
                            }
                        };
                        let layer = develop(scene, index, &made.density)?;
                        source = Some(made);
                        layer
                    }
                };
                let layer = Arc::new(layer);
                let entry = Kept {
                    item: layer.clone(),
                    used,
                    home,
                };
                kept.lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .layers
                    .insert(key.clone(), entry);
                layer
            }
        };
        keys.push(key);
        layers.push(developed);
        source_keys.extend(source_key);
        held.extend(source);
    }
    {
        let mut kept = kept.lock().unwrap_or_else(|e| e.into_inner());
        let mut bytes: usize = kept.layers.values().map(|entry| entry.item.bytes()).sum::<usize>()
            + kept.sources.values().map(|entry| entry.item.bytes()).sum::<usize>();
        // A density makes its layer again without the chaos game, so of two equally old the layer goes first.
        let mut spare: Vec<(bool, u64, bool, String)> = kept
            .layers
            .iter()
            .filter(|(key, _)| !keys.contains(key))
            .map(|(key, entry)| (entry.home, entry.used, false, key.clone()))
            .chain(
                kept.sources
                    .iter()
                    .filter(|(key, _)| !source_keys.contains(key))
                    .map(|(key, entry)| (entry.home, entry.used, true, key.clone())),
            )
            .collect();
        spare.sort();
        for (_, _, is_source, key) in spare {
            if bytes <= KEPT_BYTES {
                break;
            }
            bytes -= if is_source {
                kept.sources.remove(&key).map_or(0, |entry| entry.item.bytes())
            } else {
                kept.layers.remove(&key).map_or(0, |entry| entry.item.bytes())
            };
        }
    }
    let image = compose(scene, &layers.iter().map(|layer| &**layer).collect::<Vec<_>>(), options)?;
    let held = (sources && held.len() == scene.layers.len() && !held.is_empty()).then_some((source_keys, held));
    Ok((image, held))
}

// A scroll area left to itself starts two rows tall and can stay so.
fn list_height(rows: usize, room: f32) -> f32 {
    (rows as f32 * 32.0 - 2.0).clamp(30.0, (room - 110.0).max(90.0))
}

fn chaos_keys(scene: &Scene, options: &RenderOptions) -> Vec<String> {
    let size = size_key(options);
    (0..scene.layers.len())
        .map(|index| source_key(scene, index, &size))
        .collect()
}

// What a picture drawn from the layers' levels depends on besides the levels.
fn look_key(scene: &Scene) -> String {
    let layers: Vec<_> = scene
        .layers
        .iter()
        .map(|layer| (layer.blend, layer.opacity, layer.tone()))
        .collect();
    format!("{:?} {layers:?}", scene.background)
}

// How light a picture is, from every `step`th of its pixels: what picks dark ink and darker glass over it.
fn lightness(rgba: &[u8], step: usize) -> f32 {
    let pixels = rgba.chunks_exact(4).step_by(step);
    let (sum, count) = pixels.fold((0u64, 0u64), |(sum, count), pixel| {
        (
            sum + u64::from(pixel[0]) * 2 + u64::from(pixel[1]) * 5 + u64::from(pixel[2]),
            count + 8,
        )
    });
    sum as f32 / (count.max(1) as f32 * 255.0)
}

// The verdict kept for a graphics card, if it is this card's and this version's. Only a fast one is believed:
// a trial run at a bad moment must not leave the card out for good.
// Not "slow": that was kept by trials that asked a card to be twice as fast on a smaller picture.
const SLOW: &str = "slow2";

fn verdict_of(text: &str, card: &str) -> Verdict {
    match text.strip_prefix(card).and_then(|rest| rest.strip_prefix('|')) {
        Some(SLOW) => Verdict::Slow,
        Some(rest) => match rest.strip_prefix("fast|").and_then(|rate| rate.parse().ok()) {
            Some(rate) => Verdict::Fast(Some(rate)),
            None => Verdict::Unknown,
        },
        None => Verdict::Unknown,
    }
}

fn status_of(size: [u32; 2], seconds: f32, view: View) -> String {
    let mut status = format!("{} × {}  ·  {seconds:.2} s", size[0], size[1]);
    if view.zoom != 1.0 {
        status += &format!("  ·  {}×", view.label());
    }
    status
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
        let (opacity, blur) = (glass("glass_opacity", GLASS.0), glass("glass_blur", GLASS.1));
        widgets::install_fonts(&cc.egui_ctx, texts.lang);
        cc.egui_ctx.set_visuals(egui::Visuals::dark());
        // The window comes back where and as large as it was, on the same screen, but never in full screen.
        #[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
        cc.egui_ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(false));
        if let Some(state) = &cc.wgpu_render_state {
            note(&format!("graphics: {:?}", state.adapter.get_info()));
            if timed() {
                let card = state.adapter.get_info();
                eprintln!("times: {} on {:?}", card.name, card.backend);
            }
            let gfx = Gfx::new(&state.device, state.target_format);
            state.renderer.write().callback_resources.insert(gfx);
        }
        // The card accelerates only where it can play the chaos game; elsewhere the app is the processor's alone. A
        // browser page renders on the thread that draws, so there the card's picture would wait for it.
        // FRACTIQUE_GPU makes a test run a device that cannot (none), one whose card is slow (slow), one that is
        // not asked (fast) or one that is asked anew (measure).
        let forced = std::env::var("FRACTIQUE_GPU").ok();
        let able = cfg!(not(target_arch = "wasm32")) && forced.as_deref() != Some("none");
        let mut preview = cc
            .wgpu_render_state
            .as_ref()
            .filter(|state| able && Chaos::able(&state.adapter, &state.device))
            .and_then(|state| Preview::new(&state.adapter, &state.device, &state.queue, &cc.egui_ctx));
        let info = cc.wgpu_render_state.as_ref().map(|state| state.adapter.get_info());
        let card = info.as_ref().map_or_else(String::new, |card| {
            let api = match card.backend {
                wgpu::Backend::Vulkan => "Vulkan",
                wgpu::Backend::Metal => "Metal",
                wgpu::Backend::Dx12 => "Direct3D 12",
                wgpu::Backend::Gl => "OpenGL",
                wgpu::Backend::BrowserWebGpu => "WebGPU",
                wgpu::Backend::Noop => "",
            };
            format!("{} · {api}", card.name)
        });
        let card_key = info.as_ref().map_or_else(String::new, |card| {
            format!("{}|{}|{}", card.name, card.driver_info, env!("CARGO_PKG_VERSION"))
        });
        let verdict = match forced.as_deref() {
            _ if preview.is_none() => Verdict::Slow,
            Some("fast") => Verdict::Fast(None),
            Some("slow" | "measure") => Verdict::Unknown,
            _ => stored("card").map_or(Verdict::Unknown, |text| verdict_of(&text, &card_key)),
        };
        // A card of its own, not one that shares the processor's chip, is faster without asking.
        let discrete = info
            .as_ref()
            .is_some_and(|card| card.device_type == wgpu::DeviceType::DiscreteGpu);
        let verdict = if discrete && forced.is_none() && preview.is_some() {
            Verdict::Fast(None)
        } else {
            verdict
        };
        if verdict == Verdict::Slow {
            preview = None;
        }
        let gpu = match forced.as_deref() {
            Some(choice) => choice != "off",
            None => stored("gpu").is_none_or(|choice| choice != "off"),
        };
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
            rest: [None, None, None],
            fractals: false,
            kind: "mandelbrot".to_owned(),
            latest: None,
            palette: None,
            phone: phone::Phone::new(stored("kept"), stored("recent")),
            title,
            is_random: false,
            seed: 0,
            generator: Generator::First,
            adjust: NEUTRAL,
            view: WHOLE,
            shown: WHOLE,
            shown_zoom: 1.0,
            drawn: None,
            touched: Instant::now(),
            gpu,
            preview,
            card,
            card_key,
            verdict,
            measured_slow: false,
            trial: None,
            held: None,
            previewing: false,
            chaos: None,
            chaos_tried: false,
            compiling: false,
            chaotic: false,
            chaos_at: (WHOLE, 1.0),
            quick: (None, None),
            look: String::new(),
            asked: Instant::now(),
            measured: Arc::new(AtomicU32::new(u32::MAX)),
            #[cfg(not(target_arch = "wasm32"))]
            instant: None,
            #[cfg(not(target_arch = "wasm32"))]
            export_at: std::env::var_os("FRACTIQUE_EXPORT").map(PathBuf::from),
            #[cfg(not(target_arch = "wasm32"))]
            export_asked: Instant::now(),
            frozen: false,
            bare: chaos_only() || shows("picture"),
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
            picture: std::env::var("FRACTIQUE_PICTURE")
                .ok()
                .or_else(|| stored("picture"))
                .and_then(|text| parsed_size(&text)),
            custom: Default::default(),
            typed: None,
            sizes_open: std::env::var("FRACTIQUE_SHOW").is_ok_and(|what| what == "sizes"),
            presets_open: ["presets", "layouts", "kinds"].into_iter().any(shows),
            listing: if shows("layouts") {
                Listing::Layouts
            } else if shows("kinds") {
                Listing::Kinds
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
            #[cfg(target_os = "android")]
            keeping: None,
            #[cfg(target_os = "android")]
            waiting: None,
            #[cfg(target_os = "android")]
            focused: true,
            #[cfg(target_os = "android")]
            asking: None,
            last: None,
            last_frame: Instant::now(),
            #[cfg(not(target_arch = "wasm32"))]
            export: export::Sheet {
                open: std::env::var("FRACTIQUE_SHOW").is_ok_and(|what| what.starts_with("export")),
                ..Default::default()
            },
            #[cfg(feature = "store")]
            store: store::open(&cc.egui_ctx),
            #[cfg(feature = "store")]
            pro: std::env::var("FRACTIQUE_SHOW")
                .is_ok_and(|what| what.starts_with("pro"))
                .then_some(Wanted::Nothing),
            #[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
            updates: updates::Updates::new(),
            #[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
            arrivals: instance::watch(&cc.egui_ctx),
        };
        if let Some(session) = session {
            let _ = app.restore(&session);
        }
        // A test run in the Fractals tab: FRACTIQUE_SHOW=fractals (also kinds, fractals-phone, fractals-phone-sheet),
        // with FRACTIQUE_FRACTAL=julia for a kind other than the first.
        if std::env::var("FRACTIQUE_SHOW").is_ok_and(|what| what.contains("fractals") || what == "kinds") {
            app.wall.on = false;
            app.fractals = true;
            let kind = std::env::var("FRACTIQUE_FRACTAL").unwrap_or_else(|_| app.kind.clone());
            app.show_kind(&kind);
        }
        #[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
        if let Some(path) = OPEN_FIRST.get() {
            app.load_scene(path);
        }
        if !shows("preview")
            && let Some(adjust) = Adjust::asked()
        {
            app.adjust = adjust;
        }
        app
    }

    // The scene in the colors chosen for flames, if any were.
    fn dressed(&self) -> Scene {
        let mut scene = self.base.clone();
        if let Some(palette) = self.palette.and_then(|at| palette::named(palette::NAMES[at])) {
            for layer in &mut scene.layers {
                *layer.palette_mut() = palette.clone();
            }
        }
        scene
    }

    fn tab(&self) -> usize {
        match (self.wall.on, self.fractals) {
            (true, _) => 2,
            (false, true) => 1,
            (false, false) => 0,
        }
    }

    // A fractal drawn pixel by pixel runs out of digits where its own zoom times the view's passes the limit.
    fn deepest(&self) -> f64 {
        let own = self
            .base
            .layers
            .iter()
            .filter(|layer| layer.fractal().is_some() && !self.wall.live)
            .map(|layer| layer.camera().zoom)
            .fold(1.0, f64::max);
        MAX_MAGNIFICATION / own
    }

    fn kind_name(&self, kind: &str) -> String {
        self.t(&format!("f_{kind}"))
    }

    fn show_kind(&mut self, kind: &str) {
        let Some(spec) = fractal::spec(kind) else {
            return;
        };
        self.kind = kind.to_owned();
        if let Some(preset) = spec.presets.first() {
            self.show_preset(&format!("{kind}-{}", preset.name));
        }
    }

    fn show_preset(&mut self, name: &str) {
        if let Some(scene) = presets::get(name).or_else(|| fractal::preset(name)) {
            self.pick(named(scene, &shown(name)), name.to_owned(), false);
        }
    }

    // The tab a scene belongs to, and the kind of its first fractal.
    fn follow(&mut self, scene: &Scene) {
        let kind = scene
            .layers
            .first()
            .and_then(|layer| layer.fractal())
            .map(|shown| &shown.kind);
        self.fractals = kind.is_some();
        if let Some(kind) = kind {
            self.kind = kind.clone();
        }
    }

    // The tab that is left keeps what it showed, picture and all, and the one that is chosen shows what it had.
    fn set_mode(&mut self, tab: usize) {
        let (from, walls) = (self.tab(), tab == 2);
        if tab == from {
            return;
        }
        if self.wall.live == self.wall.on {
            let size = self.requested;
            let settled = !self.rendering && self.drawn == Some((self.view, self.adjust.zoom));
            let held = size
                .filter(|_| self.wall.live)
                .map(|size| wallpaper_choice(self.seed, &self.wall.request(size)));
            self.rest[from] = Some(Rest {
                base: self.base.clone(),
                title: self.title.clone(),
                is_random: self.is_random,
                seed: self.seed,
                view: self.view,
                layer: self.layer,
                held,
                picture: self.latest.take().filter(|_| settled).zip(size),
            });
        }
        self.wall.on = walls;
        self.fractals = tab == 1;
        let Some(rest) = self.rest[tab].take() else {
            match tab {
                2 => self.new_wallpaper(),
                1 => self.show_kind(&self.kind.clone()),
                _ => self.show_preset("aurora-spiral"),
            }
            return;
        };
        if !walls {
            self.base = rest.base;
        }
        self.wall.live = walls;
        self.wall.held = rest.held.or(self.wall.held);
        (self.title, self.is_random, self.seed) = (rest.title, rest.is_random, rest.seed);
        (self.view, self.layer) = (rest.view, rest.layer);
        self.cancel.store(true, Ordering::Relaxed);
        self.cancel = Arc::new(AtomicBool::new(false));
        self.generation += 1;
        match rest.picture {
            Some((mut picture, size)) if Some(size) == self.requested => {
                picture.generation = self.generation;
                self.drawn = Some((rest.view, self.adjust.zoom));
                self.rendering = false;
                let _ = self.results.0.send(picture);
            }
            _ => self.requested = None,
        }
    }

    fn scene(&self) -> Scene {
        self.scene_in(self.view, self.adjust.zoom)
    }

    fn scene_in(&self, view: View, zoom: f64) -> Scene {
        let mut scene = self.dressed();
        for layer in &mut scene.layers {
            let tone = layer.tone_mut();
            tone.brightness *= self.adjust.brightness;
            tone.gamma *= self.adjust.gamma;
            tone.glow_strength *= self.adjust.glow;
            layer.camera_mut().zoom *= zoom;
            *layer.camera_mut() = layer.camera().looking_at(view.right, view.down, view.zoom);
        }
        scene
    }

    fn card_used(&self) -> bool {
        matches!(self.verdict, Verdict::Fast(_)) && self.preview.as_ref().is_some_and(Preview::works)
    }

    fn gpu_on(&self) -> bool {
        self.gpu && self.card_used()
    }

    // The graphics card's own picture of the scene as it is now is on the screen. It follows the tone, the
    // opacities and the blends by itself, so nothing has to be rendered.
    fn card_shows(&self, size: [u32; 2]) -> bool {
        if !self.chaotic || !self.gpu_on() || self.wall.live {
            return false;
        }
        let keys = chaos_keys(&self.scene(), &fitted(size, FULL_SIDE, FULL_SAMPLES));
        self.chaos.as_ref().is_some_and(|chaos| chaos.keys() == Some(&keys))
    }

    // False when the layers on the graphics card are not this scene's: then the processor has to render.
    fn preview_now(&mut self, ctx: &egui::Context, frame: &eframe::Frame, size: [u32; 2]) -> bool {
        if !self.gpu_on() || self.wall.live {
            return false;
        }
        let scene = self.scene_in(self.shown, self.shown_zoom);
        if !all_flames(&scene) || scene.validate().is_err() {
            return false;
        }
        let (Some(preview), Some(held)) = (&mut self.preview, &self.held) else {
            return false;
        };
        if held.size != size {
            return false;
        }
        let prefix = size_key(&held.options);
        let keys = (0..scene.layers.len()).map(|index| source_key(&scene, index, &prefix));
        if !keys.eq(held.keys.iter().cloned()) || !preview.draw(&scene, &held.sources, ctx) {
            return false;
        }
        let Some(state) = frame.wgpu_render_state() else {
            return false;
        };
        let mut renderer = state.renderer.write();
        let (Some(gfx), Some((picture, soft))) = (renderer.callback_resources.get_mut::<Gfx>(), preview.views()) else {
            return false;
        };
        gfx.show(&state.device, picture, soft);
        self.previewing = true;
        true
    }

    // The render waits until the controls rest, as it does for a moved view.
    fn hold_render(&mut self, ctx: &egui::Context, size: [u32; 2]) {
        self.cancel.store(true, Ordering::Relaxed);
        self.cancel = Arc::new(AtomicBool::new(false));
        self.generation += 1;
        self.requested = Some(size);
        self.drawn = None;
        self.touched = Instant::now();
        self.rendering = true;
        ctx.request_repaint_after(std::time::Duration::from_secs_f32(SETTLE));
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
        self.drawn = Some((self.view, self.adjust.zoom));
        self.rendering = true;
        self.asked = Instant::now();

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
        // The preview on the screen is already sharper than a draft of the same view.
        if self.previewing
            && (self.view, self.adjust.zoom) == (self.shown, self.shown_zoom)
            && let Some(held) = &self.held
        {
            passes.retain(|(draft, options)| !draft || options.width >= held.options.width);
        }
        let (scene, cancel, generation) = (self.scene_at(size), self.cancel.clone(), self.generation);
        // The graphics card's picture is the screen's: the processor renders only what the card does not draw.
        if self.begin_chaos(ctx, &scene, &full) {
            self.rendering = !self.chaos.as_ref().is_some_and(Chaos::finished);
            return;
        }
        // The shaders are not compiled yet: a draft stands in, and the render begins again when they are.
        let soon = self.gpu_on() && all_densities(&scene) && self.chaos.as_ref().is_some_and(Chaos::pending);
        if soon && chaos_only() {
            return;
        }
        if soon && passes.iter().any(|(draft, _)| *draft) {
            passes.retain(|(draft, _)| *draft);
        }
        if timed() {
            eprintln!("times: the processor renders {} passes", passes.len());
        }
        let (view, zoom) = (self.view, self.adjust.zoom);
        let sources = self.gpu_on() && all_flames(&scene);
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
                let result = render_pool().install(|| rendered(&scene, &options, &control, &kept, home, sources));
                #[cfg(target_arch = "wasm32")]
                let result = rendered(&scene, &options, &control, &kept, home, sources);
                let Ok((image, held)) = result else {
                    return;
                };
                let held = held.map(|(keys, sources)| Held {
                    size,
                    options,
                    keys,
                    sources,
                });
                let radius = image.width.min(image.height) as f32 * 0.03;
                let soft = blur::soften(&image.rgba, image.width as usize, image.height as usize, radius);
                let seconds = started.elapsed().as_secs_f32();
                let done = Rendered {
                    generation,
                    draft,
                    view,
                    zoom,
                    held,
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

    fn receive(&mut self, ctx: &egui::Context, frame: &eframe::Frame) {
        if (self.previewing || self.chaotic) && !self.gpu_on() {
            self.previewing = false;
            self.chaotic = false;
            self.requested = None;
        }
        while let Ok(done) = self.results.1.try_recv() {
            if done.generation != self.generation {
                continue;
            }
            self.previewing = false;
            self.chaotic = false;
            if let Some(chaos) = &mut self.chaos {
                chaos.end();
            }
            if timed() {
                let pass = if done.draft { "a draft" } else { "the picture" };
                eprintln!(
                    "times: {pass} of {} × {} from the processor after {:.3} s",
                    done.image.width,
                    done.image.height,
                    self.asked.elapsed().as_secs_f32()
                );
            }
            self.held = done.held.clone().filter(|held| {
                self.gpu_on()
                    && self
                        .preview
                        .as_ref()
                        .is_some_and(|preview| preview.take(&held.sources, ctx))
            });
            self.shown_zoom = done.zoom;
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
            self.light = lightness(&done.image.rgba, 97);
            self.last = (!done.draft).then(|| (done.generation, Arc::new(done.image.clone())));
            if !done.draft {
                self.remember(self.requested.unwrap_or([done.image.width, done.image.height]));
            }
            if !done.draft {
                self.rendering = false;
                self.shown_frames = self.shown_frames.max(1);
                self.status = status_of([done.image.width, done.image.height], done.seconds, done.view);
                if shows("preview")
                    && !self.frozen
                    && let Some(adjust) = Adjust::asked()
                {
                    self.adjust = adjust;
                    self.requested = None;
                    self.frozen = true;
                }
                self.latest = Some(done);
            }
        }
    }

    fn begin_chaos(&mut self, ctx: &egui::Context, scene: &Scene, full: &RenderOptions) -> bool {
        let drawn = self.gpu_on() && !self.wall.live && all_densities(scene);
        let Some(chaos) = &mut self.chaos else {
            return false;
        };
        let keys = chaos_keys(scene, full);
        if !drawn {
            if self.trial.is_none() {
                chaos.end();
            }
            return false;
        }
        // A card that turns out slower on this picture than the processor was in the trial gives it up.
        let limit = match self.verdict {
            Verdict::Fast(Some(rate)) => {
                let samples = scene.layers.len() as f64
                    * f64::from(full.width)
                    * f64::from(full.height)
                    * f64::from(full.samples_per_pixel);
                Some((samples / rate).max(PATIENCE) as f32)
            }
            _ => None,
        };
        let goes_on = chaos.keys() == Some(&keys);
        let wanted = RenderOptions {
            samples_per_pixel: if shows("gpu-first") {
                chaos::FIRST_SAMPLES
            } else {
                full.samples_per_pixel
            },
            ..*full
        };
        if !chaos.begin(scene, &wanted, &keys, self.asked, limit, ctx) {
            if timed() && !chaos.pending() {
                eprintln!("times: the graphics card does not draw this scene");
            }
            return false;
        }
        if !goes_on {
            self.quick = (None, None);
            self.chaos_at = (self.view, self.adjust.zoom);
        }
        true
    }

    // Begins the trial of a graphics card nothing is known about yet, once the screen's picture is rendered.
    fn try_card(&mut self, ctx: &egui::Context, frame: &eframe::Frame) {
        if self.verdict != Verdict::Unknown || self.trial.is_some() || self.rendering || self.requested.is_none() {
            return;
        }
        if !self.chaos_tried {
            self.chaos_tried = true;
            self.chaos = frame
                .wgpu_render_state()
                .and_then(|state| Chaos::new(&state.adapter, &state.device, &state.queue, ctx));
            self.compiling = self.chaos.as_ref().is_some_and(Chaos::pending);
        }
        let Some(chaos) = &mut self.chaos else {
            self.decide(None);
            return;
        };
        if chaos.pending() {
            return;
        }
        let Some(scene) = presets::get("aurora-spiral") else {
            self.decide(None);
            return;
        };
        if chaos.begin(
            &scene,
            &trial_options(),
            &["trial".to_owned()],
            Instant::now(),
            None,
            ctx,
        ) {
            self.trial = Some(Trial {
                card: None,
                processor: None,
                runs: 0,
                best: f32::INFINITY,
            });
        } else {
            self.decide(None);
        }
    }

    fn trial_step(&mut self, chaos: &mut Chaos, ctx: &egui::Context) {
        let Some(trial) = &mut self.trial else {
            return;
        };
        let Some(card) = trial.card else {
            // The owner asked for a picture: the trial gives way and begins again when the app rests.
            if self.rendering {
                chaos.end();
                self.trial = None;
            } else if chaos.advance(ctx) == Step::Refused {
                self.decide(None);
            } else if chaos.spent() > TRIAL_LONGEST {
                chaos.end();
                self.measured_slow = true;
                self.decide(None);
            } else if chaos.finished() {
                let slowed = std::env::var("FRACTIQUE_GPU").is_ok_and(|choice| choice == "slow");
                trial.best = trial.best.min(chaos.spent() * if slowed { 1.0e4 } else { 1.0 });
                trial.runs += 1;
                chaos.end();
                let again = trial.runs < TRIAL_RUNS
                    && trial.best < TRIAL_QUICK
                    && presets::get("aurora-spiral").is_some_and(|scene| {
                        chaos.begin(
                            &scene,
                            &trial_options(),
                            &["trial".to_owned()],
                            Instant::now(),
                            None,
                            ctx,
                        )
                    });
                if !again {
                    trial.card = Some(trial.best);
                }
                ctx.request_repaint();
            }
            return;
        };
        match &mut trial.processor {
            None if self.rendering => {}
            None => {
                let (sender, done) = mpsc::channel();
                let ctx = ctx.clone();
                let stop = Arc::new(AtomicBool::new(false));
                let stopped = stop.clone();
                let work = move || {
                    let Some(scene) = presets::get("aurora-spiral") else {
                        return;
                    };
                    let started = Instant::now();
                    let control = Control {
                        cancel: Some(&stopped),
                        progress: None,
                    };
                    if fractique_core::render(&scene, &trial_options(), &control).is_ok() {
                        let _ = sender.send(started.elapsed().as_secs_f32());
                    }
                    ctx.request_repaint();
                };
                #[cfg(not(target_arch = "wasm32"))]
                std::thread::spawn(move || render_pool().install(work));
                #[cfg(target_arch = "wasm32")]
                work();
                trial.processor = Some((done, stop));
            }
            // A render of the owner's beside it would make the processor look slow: the trial's render stops
            // and is made again when the app rests.
            Some((_, stop)) if self.rendering => {
                stop.store(true, Ordering::Relaxed);
                trial.processor = None;
            }
            Some((done, _)) => match done.try_recv() {
                Ok(seconds) => {
                    self.measured_slow = true;
                    self.decide(Some((card, seconds)));
                }
                Err(mpsc::TryRecvError::Empty) => {}
                Err(mpsc::TryRecvError::Disconnected) => self.decide(None),
            },
        }
    }

    // The trial's seconds on the card and on the processor, or nothing when the card could not run it.
    fn decide(&mut self, seconds: Option<(f32, f32)>) {
        self.trial = None;
        self.verdict = match seconds {
            Some((card, processor)) if card * CLEARLY <= processor => {
                let options = trial_options();
                let samples = f64::from(options.width * options.height * options.samples_per_pixel);
                Verdict::Fast(Some(samples / f64::from(processor.max(1e-6))))
            }
            _ => Verdict::Slow,
        };
        if timed() {
            let (card, processor) = seconds.unwrap_or((f32::NAN, f32::NAN));
            let used = if self.verdict == Verdict::Slow {
                "left out"
            } else {
                "used"
            };
            eprintln!(
                "times: the trial took {card:.3} s on the graphics card, {processor:.3} s on the processor: {used}"
            );
        }
        if self.verdict == Verdict::Slow {
            (self.preview, self.chaos, self.held) = (None, None, None);
        }
    }

    // The chaos game on the graphics card: takes in the batch that has finished, sends the next and shows the
    // picture, with the tone as it is now.
    fn chaos_frame(&mut self, ctx: &egui::Context, frame: &eframe::Frame) {
        let Some(mut chaos) = self.chaos.take() else {
            return;
        };
        let pending = chaos.pending();
        // The shaders are compiled, or refused: a render that began without them begins again.
        let flames = self.wall.live || all_densities(&self.base);
        if self.compiling && !pending && self.rendering && !self.chaotic && self.gpu_on() && flames {
            self.requested = None;
        }
        self.compiling = pending;
        if self.trial.is_some() {
            self.trial_step(&mut chaos, ctx);
        } else if chaos.keys().is_some() {
            self.chaos_step(&mut chaos, ctx, frame);
        }
        self.chaos = Some(chaos);
        let measured = self.measured.swap(u32::MAX, Ordering::Relaxed);
        if measured != u32::MAX && self.chaotic {
            self.light = f32::from_bits(measured);
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            if self.chaotic
                && let Some(size) = self.requested
                && let Some(path) = self.export_at.take()
            {
                self.export_to(ctx, size, path);
            }
            self.hand_over(ctx);
        }
    }

    // An Instant export of the graphics card's picture: it waits for the last of the samples, then the picture
    // is copied back from the card and saved. A picture the card gave up meanwhile is rendered by the processor.
    #[cfg(not(target_arch = "wasm32"))]
    fn hand_over(&mut self, ctx: &egui::Context) {
        let Some((later, size)) = self.instant.take() else {
            return;
        };
        if later.cancelled() {
            return;
        }
        if !self.requested.is_some_and(|shown| self.card_shows(shown)) {
            let options = RenderOptions {
                width: size[0],
                height: size[1],
                samples_per_pixel: FULL_SAMPLES,
                supersample: 1,
            };
            let job = export::Job::start(self.scene_at(size), options, later.path(), ctx.clone(), render_pool());
            self.export.job = Some(job);
            return;
        }
        let (Some(preview), Some(chaos)) = (&self.preview, &self.chaos) else {
            return;
        };
        if !chaos.finished() {
            self.instant = Some((later, size));
            return;
        }
        let saved = preview.read(false, ctx, move |pixels| match pixels {
            Some((rgba, size)) => later.save(rgba, size),
            None => later.fail(),
        });
        if let Some(index) = saved {
            chaos.watch(index);
        }
    }

    fn chaos_step(&mut self, chaos: &mut Chaos, ctx: &egui::Context, frame: &eframe::Frame) {
        let step = chaos.advance(ctx);
        if step == Step::Refused {
            if timed() {
                eprintln!("times: the graphics card gave the scene up");
            }
            self.requested = None;
            return;
        }
        let times = chaos.times();
        let whole = times.1.is_some() && self.quick.1.is_none();
        if times != self.quick {
            if let (Some((size, _)), Some(first)) = (chaos.levels(), times.0) {
                self.status = status_of(size, times.1.unwrap_or(first), self.chaos_at.0);
                if timed() && self.quick.0.is_none() {
                    eprintln!(
                        "times: the first picture of {} × {} from the graphics card after {first:.3} s",
                        size[0], size[1]
                    );
                }
            }
            if timed()
                && let Some(full) = times.1
            {
                let batches: Vec<String> = chaos
                    .batches()
                    .iter()
                    .map(|(took, steps)| format!("{:.0}/{:.1}", took * 1000.0, *steps as f64 / 1e6))
                    .collect();
                eprintln!("times: all samples from the graphics card after {full:.3} s");
                eprintln!("times: its batches, ms/million steps: {}", batches.join(" "));
            }
            self.quick = times;
        }
        self.chaos_draw(chaos, step == Step::Drawn, ctx, frame);
        if whole && self.chaotic {
            self.rendering = false;
            self.shown_frames = self.shown_frames.max(1);
            if let Some(size) = self.requested {
                self.remember(size);
            }
            if shows("preview")
                && !self.frozen
                && let Some(adjust) = Adjust::asked()
            {
                self.adjust = adjust;
                self.requested = None;
                self.frozen = true;
            }
        }
    }

    // Draws the picture from the graphics card's levels when they are new or the tone has changed, and has its
    // lightness measured.
    fn chaos_draw(&mut self, chaos: &Chaos, fresh: bool, ctx: &egui::Context, frame: &eframe::Frame) {
        if !fresh && !self.chaotic {
            return;
        }
        let scene = self.scene_in(self.chaos_at.0, self.chaos_at.1);
        let look = look_key(&scene);
        if !fresh && look == self.look {
            return;
        }
        let (Some(preview), Some((size, levels))) = (&mut self.preview, chaos.levels()) else {
            return;
        };
        if !preview.draw_levels(&scene, size, &levels, ctx) {
            return;
        }
        let Some(state) = frame.wgpu_render_state() else {
            return;
        };
        let mut renderer = state.renderer.write();
        let (Some(gfx), Some((picture, soft))) = (renderer.callback_resources.get_mut::<Gfx>(), preview.views()) else {
            return;
        };
        gfx.show(&state.device, picture, soft);
        let (measured, wake) = (self.measured.clone(), ctx.clone());
        let read = preview.read(true, ctx, move |pixels| {
            if let Some((rgba, _)) = pixels {
                measured.store(lightness(&rgba, 1).to_bits(), Ordering::Relaxed);
                wake.request_repaint();
            }
        });
        if let Some(index) = read {
            chaos.watch(index);
        }
        self.chaotic = true;
        self.previewing = false;
        self.look = look;
        (self.shown, self.shown_zoom) = self.chaos_at;
        (self.latest, self.last) = (None, None);
    }

    fn detail(&self) -> String {
        if self.rendering && !self.chaotic {
            self.t("rendering")
        } else {
            self.status.clone()
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
            "generator": self.generator.number(),
            "fractals": self.fractals,
            "kind": self.kind,
            "adjust": [self.adjust.brightness, self.adjust.gamma, self.adjust.glow, self.adjust.zoom],
            "view": [self.view.right, self.view.down, self.view.zoom],
            "layer": self.layer,
            "palette": self.palette.map(|at| palette::NAMES[at]),
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
        self.generator = generator_of(&session);
        self.follow(&self.base.clone());
        if let Some(fractals) = session.get("fractals").and_then(serde_json::Value::as_bool) {
            self.fractals = fractals;
        }
        if let Some(kind) = session.get("kind").and_then(serde_json::Value::as_str)
            && fractal::spec(kind).is_some()
        {
            self.kind = kind.to_owned();
        }
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
        self.palette = session
            .get("palette")
            .and_then(serde_json::Value::as_str)
            .and_then(|name| palette::NAMES.iter().position(|each| *each == name));
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
        #[cfg(feature = "store")]
        {
            self.pro = None;
        }
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
        if self.listing == Listing::Recent {
            self.recent_sheet(ui, room);
            return;
        }
        if self.listing == Listing::Kinds {
            self.kind_sheet(ui, room);
            return;
        }
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
        // A flame can lie over a fractal: the list of layers to add holds the presets of both.
        let mut names = if self.fractals { self.kind_presets() } else { Vec::new() };
        if !self.fractals || adding {
            names.extend(presets::names().into_iter().map(str::to_owned));
        }
        let tall = list_height(names.len() + usize::from(adding), room);
        egui::ScrollArea::vertical()
            .max_height(tall)
            .min_scrolled_height(tall)
            .show(ui, |ui| {
                if adding && widgets::row(ui, &shown(&self.t("random")), false) {
                    chosen = Some(self.fresh());
                }
                for name in &names {
                    let current = !adding && !self.is_random && self.title == *name;
                    if widgets::row(ui, &shown(name), current)
                        && let Some(scene) = presets::get(name).or_else(|| fractal::preset(name))
                    {
                        chosen = Some((named(scene, &shown(name)), name.clone()));
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

    fn kind_presets(&self) -> Vec<String> {
        fractal::spec(&self.kind)
            .map(|spec| {
                spec.presets
                    .iter()
                    .map(|preset| format!("{}-{}", spec.name, preset.name))
                    .collect()
            })
            .unwrap_or_default()
    }

    // The random scene of the next seed, of the kind the tab makes, and its title.
    fn fresh(&mut self) -> (Scene, String) {
        if self.fractals {
            self.seed += 1;
            let title = format!("{} {}", self.kind_name(&self.kind), self.seed);
            if let Some(scene) = fractal::random_scene(&self.kind, self.seed) {
                return (named(scene, &title), title);
            }
        }
        let title = self.next_random();
        (named(random_scene_of(self.generator, self.seed), &title), title)
    }

    fn kind_sheet(&mut self, ui: &mut egui::Ui, room: f32) {
        widgets::section(ui, &self.t("fractal_kind"));
        let mut bar = egui::style::ScrollStyle::floating();
        bar.bar_width = 5.0;
        ui.spacing_mut().scroll = bar;
        let mut chosen = None;
        let tall = list_height(fractal::all().count(), room);
        egui::ScrollArea::vertical()
            .max_height(tall)
            .min_scrolled_height(tall)
            .show(ui, |ui| {
                for spec in fractal::all() {
                    if widgets::row(ui, &self.kind_name(spec.name), self.kind == spec.name) {
                        chosen = Some(spec.name);
                    }
                }
            });
        if let Some(kind) = chosen {
            self.presets_open = false;
            self.show_kind(kind);
        }
    }

    // The controls of the chosen layer's fractal: a slider for each number of its kind, a field for each text.
    fn fractal_controls(&mut self, ui: &mut egui::Ui) {
        let at = self.layer.min(self.base.layers.len().saturating_sub(1));
        let Some((spec, values, texts)) =
            self.base
                .layers
                .get(at)
                .and_then(|layer| layer.fractal())
                .and_then(|shown| {
                    let spec = fractal::spec(&shown.kind)?;
                    let texts: Vec<String> = spec
                        .texts()
                        .iter()
                        .map(|text| spec.text(shown, text.name).to_owned())
                        .collect();
                    Some((spec, spec.values(shown), texts))
                })
        else {
            return;
        };
        widgets::section(ui, &self.t("parameters"));
        let mut changed = Vec::new();
        for (param, before) in spec.params.iter().zip(values) {
            let mut value = before;
            let label = self.t(&format!("p_{}", param.name));
            // A whole parameter whose every value has a name, `c_<parameter>_<value>` in the texts, is a choice.
            let names: Vec<String> = (param.min as i64..=param.max as i64)
                .map(|value| format!("c_{}_{value}", param.name))
                .map(|key| (self.t(&key), key))
                .take_while(|(name, key)| param.whole && name != key)
                .map(|(name, _)| name)
                .collect();
            if names.len() as f64 == param.max - param.min + 1.0 {
                if let Some(index) = widgets::choice(ui, &label, &names, (before - param.min) as usize) {
                    value = param.min + index as f64;
                }
            } else {
                let digits = if param.whole { 0 } else { 3 };
                widgets::slider_to(ui, &label, &mut value, (param.min, param.max), param.log, digits);
                if param.whole {
                    value = value.round();
                }
            }
            if value != before {
                changed.push((param.name, value.clamp(param.min, param.max)));
            }
        }
        let mut written = Vec::new();
        for (text, before) in spec.texts().iter().zip(texts) {
            let key = format!("{at} {}", text.name);
            let mut value = match &self.typed {
                Some((of, typed)) if *of == key => typed.clone(),
                _ => before.clone(),
            };
            let label = self.t(&format!("p_{}", text.name));
            let valid = (text.check)(&value).is_ok();
            let field = widgets::text_field(ui, &label, &mut value, fractique_core::scene::MAX_TEXT, valid);
            if field.changed() {
                if (text.check)(&value).is_ok() && value != before {
                    written.push((text.name, value.clone()));
                }
                self.typed = Some((key, value));
            } else if field.lost_focus() && self.typed.as_ref().is_some_and(|(of, _)| *of == key) {
                self.typed = None;
            }
        }
        if (!changed.is_empty() || !written.is_empty())
            && let Some(shown) = self.base.layers[at].fractal_mut()
        {
            shown
                .params
                .extend(changed.into_iter().map(|(name, value)| (name.to_owned(), value)));
            shown
                .text
                .extend(written.into_iter().map(|(name, value)| (name.to_owned(), value)));
            self.requested = None;
        }
    }

    fn recent_sheet(&mut self, ui: &mut egui::Ui, room: f32) {
        widgets::section(ui, &self.t("recent"));
        let mut bar = egui::style::ScrollStyle::floating();
        bar.bar_width = 5.0;
        ui.spacing_mut().scroll = bar;
        let mut chosen = None;
        let tall = list_height(self.phone.recent.len(), room);
        egui::ScrollArea::vertical()
            .max_height(tall)
            .min_scrolled_height(tall)
            .show(ui, |ui| {
                for (index, piece) in self.phone.recent.iter().enumerate() {
                    if widgets::row(ui, &shown(&piece.title), index == 0) {
                        chosen = Some(piece.clone());
                    }
                }
            });
        if let Some(piece) = chosen {
            self.presets_open = false;
            self.show_piece(&piece);
        }
    }

    fn size_sheet(&mut self, ui: &mut egui::Ui, window: [u32; 2], room: f32) {
        ui.set_width(300.0);
        ui.spacing_mut().item_spacing = egui::vec2(8.0, 2.0);
        widgets::section(ui, &self.t("size"));
        let own = format!("{} · {} × {}", self.t("size_window"), window[0], window[1]);
        let height = (room - 230.0).clamp(34.0, 420.0);
        let locked = self.lock();
        #[cfg(feature = "store")]
        let before = self.picture;
        let custom = self.pro_missing();
        let locks = |size: Option<[u32; 2]>| size.is_some_and(&locked);
        let list = |height: f32| List {
            height,
            locked: &locks,
            custom,
        };
        let picked = size_list(
            ui,
            &self.texts,
            &own,
            window,
            &mut self.picture,
            &mut self.custom,
            list(height),
        );
        if picked == Pick::Chosen {
            self.sizes_open = false;
            #[cfg(feature = "store")]
            if let Some(size) = self.picture
                && Some(size) != before
                && locked(size)
            {
                self.picture = before;
                self.want(Wanted::Picture(size));
            }
        }
        #[cfg(feature = "store")]
        if let Pick::Custom(size) = picked {
            self.want(size.map_or(Wanted::Nothing, Wanted::Picture));
        }
    }

    fn pro_missing(&self) -> bool {
        #[cfg(feature = "store")]
        return self.store.as_ref().is_some_and(|store| !store.owned());
        #[cfg(not(feature = "store"))]
        false
    }

    fn lock(&self) -> impl Fn([u32; 2]) -> bool + use<> {
        #[cfg(feature = "store")]
        let locks = self.pro_missing();
        #[cfg(feature = "store")]
        return move |size| locks && store::is_large(size);
        #[cfg(not(feature = "store"))]
        |_| false
    }

    fn pro_open(&self) -> bool {
        #[cfg(feature = "store")]
        return self.pro.is_some();
        #[cfg(not(feature = "store"))]
        false
    }

    #[cfg(feature = "store")]
    fn want(&mut self, wanted: Wanted) {
        self.open_sheet(Sheet::Sizes);
        self.sizes_open = false;
        self.pro = Some(wanted);
        if let Some(store) = &mut self.store {
            store.wake();
        }
    }

    #[cfg(feature = "store")]
    fn settle_store(&mut self, ctx: &egui::Context) {
        let Some(store) = &mut self.store else {
            return;
        };
        if !store.settle() {
            return;
        }
        tick(true);
        match self.pro.take() {
            Some(Wanted::Export { size, share }) => {
                self.export.open = true;
                self.export.note.clear();
                self.export.share = share;
                self.start_export(ctx, size);
            }
            Some(Wanted::ExportSize(size)) => {
                self.export.open = true;
                if size.is_some() {
                    self.export.size = size;
                }
            }
            Some(Wanted::Picture(size)) => self.picture = Some(size),
            Some(Wanted::Nothing) => self.pro = Some(Wanted::Nothing),
            None => {}
        }
    }

    #[cfg(feature = "store")]
    fn pro_sheet(&mut self, ui: &mut egui::Ui, width: f32) {
        use store::{Problem, State};

        ui.set_width(width);
        ui.spacing_mut().item_spacing = egui::vec2(8.0, 2.0);
        ui.label(
            egui::RichText::new(self.t("pro_title"))
                .font(font(17.0, SEMIBOLD))
                .color(egui::Color32::WHITE),
        );
        let Some(store) = &self.store else {
            return;
        };
        let state = store.state().clone();
        let price = store.price().map(|price| {
            let known = ui.fonts_mut(|fonts| fonts.has_glyphs(&font(14.0, MEDIUM), &price.shown));
            if known && !price.shown.is_empty() {
                price.shown.clone()
            } else {
                price.plain.clone()
            }
        });
        let soft = egui::Color32::from_white_alpha(200);
        ui.add_space(8.0);
        ui.label(
            egui::RichText::new(self.t(if state == State::Owned {
                "pro_owned"
            } else {
                "pro_about"
            }))
            .size(13.5)
            .color(soft),
        );
        ui.add_space(14.0);
        let full = ui.available_width();
        if state == State::Owned {
            if widgets::pill(ui, &self.t("close"), true, full) {
                self.pro = None;
            }
            return;
        }
        let words = match &state {
            State::Loading => self.t("pro_loading"),
            State::Buying => self.t("pro_waiting"),
            State::Failed(Problem::Told(words)) if !words.is_empty() => words.clone(),
            State::Failed(Problem::Told(_)) => self.t("pro_failed"),
            State::Failed(Problem::NoProduct) => self.t("pro_no_product"),
            State::Failed(Problem::NothingToRestore) => self.t("pro_nothing"),
            State::Failed(Problem::NotAllowed) => self.t("pro_not_allowed"),
            State::Failed(Problem::Deferred) => self.t("pro_deferred"),
            State::Ready | State::Owned => String::new(),
        };
        if !words.is_empty() {
            ui.label(egui::RichText::new(words).size(12.5).color(egui::Color32::WHITE));
            ui.add_space(12.0);
        }
        let (mut buy, mut restore) = (false, false);
        if matches!(state, State::Ready | State::Failed(_)) {
            let label = match &price {
                Some(price) => self.t("pro_buy").replace("{price}", price),
                None => self.t("pro_retry"),
            };
            buy = widgets::pill(ui, &label, true, full);
            ui.add_space(8.0);
        }
        if state != State::Buying {
            restore = widgets::pill(ui, &self.t("pro_restore"), false, full);
            ui.add_space(8.0);
        }
        if widgets::pill(ui, &self.t("cancel"), false, full)
            && let Some(Wanted::Export { .. } | Wanted::ExportSize(_)) = self.pro.take()
        {
            self.export.open = true;
        }
        if let Some(store) = &mut self.store
            && (buy || restore)
        {
            if buy {
                store.buy();
            } else {
                store.restore();
            }
            ui.ctx().request_repaint();
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

    fn sheet_open(&self) -> bool {
        let lists = self.sizes_open || self.presets_open || self.languages_open || self.pro_open();
        #[cfg(not(target_arch = "wasm32"))]
        let lists = lists || self.export.open;
        #[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
        let lists = lists || self.updates.open;
        lists
    }

    fn explore(&mut self, ui: &mut egui::Ui, screen: egui::Rect) {
        let picture = ui.interact(screen, egui::Id::unique("picture"), egui::Sense::click_and_drag());
        let blocked = self.sheet_open();
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
        if picture.dragged() && !blocked {
            if swiping {
                self.phone.swipe += picture.drag_delta();
            } else {
                self.view
                    .moved(sides(picture.drag_delta() / screen.size(), size, stretch));
            }
        }
        if picture.drag_stopped() {
            let by = std::mem::take(&mut self.phone.swipe);
            if swiping && !blocked && by.x.abs() > phone::SWIPE && by.x.abs() > by.y.abs() * 1.4 {
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
        // A finger that rests is a long press to egui, which then stops counting the press as the picture's.
        let touching = ui.input(|input| input.pointer.any_down());
        self.phone.pressing = touching && (self.phone.pressing || picture.is_pointer_button_down_on());
        let down = self.phone.pressing;
        let moved = ui.input(|input| {
            let strayed = input
                .pointer
                .press_origin()
                .zip(input.pointer.interact_pos())
                .is_some_and(|(from, to)| (to - from).length() > 10.0);
            strayed || input.multi_touch().is_some()
        });
        self.phone.strayed = down && (self.phone.strayed || moved);
        let holding = self.phone.on && down && held && !self.phone.strayed && !blocked;
        if holding != self.phone.preview {
            self.phone.preview = holding;
            self.phone.previewed |= holding;
            ui.ctx().request_repaint();
        }
        if self.phone.on && down && !held {
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
        } else if blocked {
        } else if picture.double_clicked() {
            self.view = WHOLE;
        } else if picture.clicked() && (self.bare || self.phone.on) {
            self.bare = !self.bare;
        }
        if picture.contains_pointer() && !blocked {
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
                let most = self.deepest();
                self.view
                    .magnified(f64::from(factor), sides(around, size, stretch), most);
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

    fn next_random(&mut self) -> String {
        self.seed += 1;
        self.generator = Generator::LATEST;
        format!("{} {}", shown(&self.t("random")), self.seed)
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
        self.inspector_head(ui);
        self.inspector_scene(ui);
        self.inspector_look(ui);
    }

    fn inspector_head(&mut self, ui: &mut egui::Ui) {
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
    }

    fn inspector_scene(&mut self, ui: &mut egui::Ui) {
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
        let tabs = ["flames", "fractals", "shapes"].map(|key| self.t(key));
        if let Some(tab) = widgets::segments(ui, &tabs, self.tab()) {
            self.set_mode(tab);
        }

        if self.wall.on {
            self.wallpaper_controls(ui);
        } else {
            if self.fractals {
                widgets::section(ui, &self.t("fractal_kind"));
                if widgets::dropdown(ui, &self.kind_name(&self.kind)) {
                    self.open_listing(Listing::Kinds);
                }
            }
            widgets::section(ui, &self.t("presets"));
            if widgets::dropdown(ui, &shown(&self.title)) {
                self.open_sheet(Sheet::Presets);
            }
        }
        if self.phone.recent.len() > 1 {
            widgets::section(ui, &self.t("recent"));
            if widgets::dropdown(ui, &shown(&self.phone.recent[1].title)) {
                self.open_listing(Listing::Recent);
            }
        }
        widgets::section(ui, &self.t("color"));
        ui.add_space(6.0);
        self.colors(ui, 16.0);

        widgets::section(ui, &self.t("size"));
        let label = match self.picture {
            Some([width, height]) => format!("{width} × {height}"),
            None => self.t("size_window"),
        };
        if widgets::dropdown(ui, &label) {
            self.open_sheet(Sheet::Sizes);
        }

        if !self.wall.live && !self.base.layers.is_empty() {
            self.layers(ui);
        }
    }

    fn inspector_look(&mut self, ui: &mut egui::Ui) {
        let flames = !self.wall.live && !self.base.layers.is_empty();
        let before = self.adjust;
        if flames {
            self.fractal_controls(ui);
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
        ui.add_space(8.0);
        if widgets::pill(ui, &self.t("reset"), false, ui.available_width()) {
            (self.optics.tint, self.optics.blur) = GLASS;
        }
        if self.adjust != before {
            let zoomed = Adjust {
                zoom: self.adjust.zoom,
                ..before
            };
            if self.adjust == zoomed && self.gpu_on() {
                self.touched = Instant::now();
            } else {
                self.requested = None;
            }
        }

        if self.card_used() {
            widgets::section(ui, &self.t("graphics"));
            if widgets::toggle(ui, &self.t("gpu"), &mut self.gpu) {
                self.held = None;
                self.requested = None;
                self.chaotic = false;
                if let Some(chaos) = &mut self.chaos {
                    chaos.end();
                }
                let mut kept = self.kept.lock().unwrap_or_else(|e| e.into_inner());
                if self.gpu {
                    kept.layers.clear();
                } else {
                    kept.sources.clear();
                }
            }
            if self.gpu {
                ui.add_space(4.0);
                let card = egui::RichText::new(&self.card)
                    .size(12.0)
                    .color(egui::Color32::from_white_alpha(110));
                ui.add(egui::Label::new(card).truncate());
                ui.add_space(2.0);
                ui.label(
                    egui::RichText::new(self.t("gpu_note"))
                        .size(12.0)
                        .color(egui::Color32::from_white_alpha(150)),
                );
            }
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
        let tall = list_height(
            1 + if layouts {
                Layout::ALL.len()
            } else {
                Material::ALL.len()
            },
            room,
        );
        egui::ScrollArea::vertical()
            .max_height(tall)
            .min_scrolled_height(tall)
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
                let (scene, title) = self.fresh();
                self.pick(scene, title, true);
            }
            Some("mutate") if self.base.layers.is_empty() => {}
            Some("randomize") => {
                let (mut scene, title) = self.fresh();
                let at = self.layer.min(self.base.layers.len() - 1);
                let fresh = scene.layers.swap_remove(0);
                self.base.layers[at].content = fresh.content;
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
                    self.base.layers[at].content = changed.content;
                }
                self.requested = None;
            }
            #[cfg(not(target_arch = "wasm32"))]
            Some("export") => {
                #[cfg(feature = "store")]
                {
                    self.pro = None;
                }
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
    fn arrive(&mut self, ctx: &egui::Context) {
        let Some(arrivals) = &self.arrivals else {
            return;
        };
        let arrived: Vec<Option<PathBuf>> = arrivals.try_iter().collect();
        for path in arrived {
            if let Some(path) = path {
                self.load_scene(&path);
            }
            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
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
        let read = std::fs::read(path)
            .map_err(|error| error.to_string())
            .and_then(|bytes| {
                if fractique_core::file::is_packed(&bytes) {
                    fractique_core::file::unpack(&bytes).map(|document| document.to_string())
                } else {
                    String::from_utf8(bytes).map_err(|error| error.to_string())
                }
            });
        let text = match read {
            Ok(text) => text,
            Err(error) => {
                self.status = error;
                return;
            }
        };
        if !is_flame && text.contains("\"fractique\"") {
            self.view = WHOLE;
            self.adjust = NEUTRAL;
            if self.restore(&text).is_some() {
                let size = serde_json::from_str::<serde_json::Value>(&text).ok();
                let side = |index: usize| {
                    size.as_ref()?
                        .get("picture")?
                        .get(index)?
                        .as_u64()
                        .map(|side| side as u32)
                };
                self.picture = side(0).zip(side(1)).map(|(width, height)| [width, height]);
                self.requested = None;
                self.drawn = None;
            } else {
                self.status = self.t("failed");
            }
            return;
        }
        let loaded = if is_flame {
            fractique_core::import::import_flames(&text)
                .map(|mut flames| flames.swap_remove(0).scene)
                .map_err(|error| error.to_string())
        } else {
            Scene::from_json(&text).map_err(|error| error.to_string())
        };
        match loaded {
            Ok(scene) => {
                self.adjust = NEUTRAL;
                self.wall.on = false;
                self.follow(&scene);
                self.pick(scene, name, false);
            }
            Err(error) => self.status = error,
        }
    }

    // The app's own file, packed by `fractique_core::file`: the session, so that it opens as it was saved, and
    // under "shown" the scene as it was on the screen, which is what the other tools render.
    #[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
    fn document(&self) -> Vec<u8> {
        let size = self.requested.unwrap_or([1, 2]);
        let mut document: serde_json::Value = serde_json::from_str(&self.session()).unwrap_or_default();
        if let Some(fields) = document.as_object_mut() {
            fields.insert("fractique".to_owned(), 1.into());
            fields.insert("picture".to_owned(), serde_json::json!(self.picture));
            fields.insert(
                "shown".to_owned(),
                serde_json::to_value(self.scene_at(size)).unwrap_or_default(),
            );
        }
        fractique_core::file::pack(&document)
    }

    #[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
    fn save_scene(&mut self) {
        let name = format!("{}.{SCENE_FILE}", file_name(&self.title));
        let Some(path) = rfd::FileDialog::new()
            .set_file_name(name)
            .add_filter("Fractique", &[SCENE_FILE])
            .add_filter("JSON", &["json"])
            .save_file()
        else {
            return;
        };
        let own = path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case(SCENE_FILE));
        let bytes = if own {
            self.document()
        } else {
            (self.scene_at(self.requested.unwrap_or([1, 2])).to_json() + "\n").into_bytes()
        };
        self.status = match std::fs::write(&path, bytes) {
            Ok(()) => self.t("saved"),
            Err(error) => format!("{}: {error}", self.t("failed")),
        };
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn export_sheet(&mut self, ui: &mut egui::Ui, window: [u32; 2], room: f32, wide: Option<f32>) {
        ui.set_width(wide.unwrap_or(300.0));
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
        let own = format!("{} × {}", shown[0], shown[1]);
        let size = self.t("size");
        let locked = self.lock();
        let custom = self.pro_missing();
        let locks = |size: Option<[u32; 2]>| locked(size.unwrap_or(shown));
        let list = |height: f32| List {
            height,
            locked: &locks,
            custom,
        };
        let mut picked = Pick::None;
        #[cfg(feature = "store")]
        let size_before = self.export.size;
        let clicked = if wide.is_some() {
            // A low, wide screen: the sizes at the left, the rest at the right.
            ui.spacing_mut().item_spacing.x = 24.0;
            let mut clicked = (false, false);
            ui.columns(2, |columns| {
                let ui = &mut columns[0];
                ui.spacing_mut().item_spacing = egui::vec2(8.0, 2.0);
                widgets::section(ui, &size);
                let height = (room - 250.0).clamp(34.0, 230.0);
                picked = size_list(
                    ui,
                    &self.texts,
                    &own,
                    shown,
                    &mut self.export.size,
                    &mut self.export.custom,
                    list(height),
                );
                columns[1].spacing_mut().item_spacing = egui::vec2(8.0, 2.0);
                clicked = self.export_choices(&mut columns[1]);
            });
            clicked
        } else {
            widgets::section(ui, &size);
            let height = (room - 400.0).clamp(34.0, 230.0);
            picked = size_list(
                ui,
                &self.texts,
                &own,
                shown,
                &mut self.export.size,
                &mut self.export.custom,
                list(height),
            );
            self.export_choices(ui)
        };
        let [width, height] = self.export.size.unwrap_or(shown);
        #[cfg(feature = "store")]
        if let Pick::Custom(size) = picked {
            self.want(Wanted::ExportSize(size));
            return;
        }
        #[cfg(feature = "store")]
        if picked == Pick::Chosen
            && self.export.size != size_before
            && self.pro_missing()
            && store::needs_pro([width, height], self.export.size.is_none_or(is_listed))
        {
            let asked = self.export.size;
            self.export.size = size_before;
            self.want(Wanted::ExportSize(asked));
            return;
        }
        #[cfg(not(feature = "store"))]
        let _ = picked;
        if clicked.0 {
            self.export.open = false;
        }
        if clicked.1 {
            #[cfg(feature = "store")]
            if self.pro_missing() && store::needs_pro([width, height], self.export.size.is_none_or(is_listed)) {
                self.want(Wanted::Export {
                    size: [width, height],
                    share: self.export.share,
                });
                return;
            }
            // The owner is asked before the picture is made, not after the wait for it.
            #[cfg(target_os = "android")]
            if !android::granted() {
                android::ask();
                self.asking = Some(([width, height], self.export.share));
                self.export.note = self.t("allow_storage");
                return;
            }
            self.start_export(ui.ctx(), [width, height]);
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn start_export(&mut self, ctx: &egui::Context, [width, height]: [u32; 2]) {
        let name = format!("{}-{width}x{height}.png", file_name(&self.title));
        if let Some(path) = export::target(&name) {
            self.export_to(ctx, [width, height], path);
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn export_to(&mut self, ctx: &egui::Context, [width, height]: [u32; 2], path: PathBuf) {
        let (_, samples, supersample) = export::QUALITIES[self.export.quality];
        self.export_asked = Instant::now();
        let as_shown = samples == export::AS_SHOWN && (self.shown, self.shown_zoom) == (self.view, self.adjust.zoom);
        let on_card = self.chaos.as_ref().and_then(Chaos::levels).map(|(size, _)| size);
        if as_shown && on_card == Some([width, height]) && self.requested.is_some_and(|size| self.card_shows(size)) {
            let (job, later) = export::Job::later(path, ctx.clone());
            self.export.job = Some(job);
            self.instant = Some((later, [width, height]));
            ctx.request_repaint();
            return;
        }
        if as_shown
            && !self.rendering
            && let Some((generation, image)) = &self.last
            && *generation == self.generation
            && [image.width, image.height] == [width, height]
        {
            self.export.job = Some(export::Job::save(image.clone(), path, ctx.clone()));
            return;
        }
        let options = RenderOptions {
            width,
            height,
            samples_per_pixel: if samples == export::AS_SHOWN {
                FULL_SAMPLES
            } else {
                samples
            },
            supersample,
        };
        self.export.job = Some(export::Job::start(
            self.scene_at([width, height]),
            options,
            path,
            ctx.clone(),
            render_pool(),
        ));
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn export_choices(&mut self, ui: &mut egui::Ui) -> (bool, bool) {
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
        let save = self.t(if cfg!(any(target_os = "android", target_os = "ios")) {
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
        if cfg!(any(target_os = "android", target_os = "ios")) {
            ui.add_space(8.0);
            if widgets::pill(ui, &self.t("share"), false, ui.available_width()) {
                self.export.share = true;
                clicked.1 = true;
            }
        }
        clicked
    }

    #[cfg(target_os = "android")]
    fn keep_on_android(&mut self, ctx: &egui::Context) {
        // The permission dialog takes the screen; the app has it back once the owner has answered.
        let focused = ctx.input(|input| input.focused);
        if focused
            && !self.focused
            && let Some((size, share)) = self.asking.take()
            && android::granted()
        {
            self.export.note.clear();
            self.export.share = share;
            self.start_export(ctx, size);
        }
        if focused
            && !self.focused
            && let Some((path, share)) = self.waiting.take()
            && android::granted()
        {
            self.export.open = false;
            self.keeping = Some((path, share, 3));
        }
        self.focused = focused;

        let Some((path, share, frames)) = self.keeping.take() else {
            return;
        };
        // The share menu covers the app and stops its frames: the sheet must be gone from the screen first.
        if frames > 0 {
            self.keeping = Some((path, share, frames - 1));
            ctx.request_repaint();
            return;
        }
        match android::keep(&path, share) {
            android::Kept::Done => self.export.note = self.t("saved"),
            android::Kept::Asked => {
                self.waiting = Some((path, share));
                self.export.note = self.t("allow_storage");
                self.export.open = true;
            }
            android::Kept::Failed => {}
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
                if timed() {
                    let seconds = self.export_asked.elapsed().as_secs_f32();
                    eprintln!("times: the export is saved after {seconds:.3} s");
                }
                #[cfg(target_os = "ios")]
                {
                    tick(true);
                    if self.export.share {
                        let from = self.phone.share_from;
                        ios::share(&path, [from.left(), from.top(), from.width(), from.height()]);
                    } else if ios::save_to_photos(&path) {
                        self.export.note = self.t("saved");
                    }
                    self.export.open = false;
                }
                #[cfg(target_os = "android")]
                {
                    self.export.open = false;
                    self.keeping = Some((path, self.export.share, 3));
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
        #[cfg(not(target_arch = "wasm32"))]
        if self.export.job.is_some() || self.export_at.is_some() {
            ctx.request_repaint();
            return;
        }
        if self.verdict == Verdict::Unknown {
            ctx.request_repaint();
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
        #[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
        self.arrive(&ctx);
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
        self.receive(&ctx, frame);
        #[cfg(not(target_arch = "wasm32"))]
        self.finish_export();
        #[cfg(feature = "store")]
        self.settle_store(&ctx);
        #[cfg(target_os = "android")]
        self.keep_on_android(&ctx);
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
        // An error of the graphics card ends its part for the session: the switch goes and the processor renders.
        let broken = self.preview.as_ref().is_some_and(|preview| !preview.works())
            || self.chaos.as_ref().is_some_and(|chaos| !chaos.works());
        if broken && self.verdict == Verdict::Unknown {
            self.decide(None);
        }
        if broken {
            (self.preview, self.chaos, self.held) = (None, None, None);
            self.kept.lock().unwrap_or_else(|e| e.into_inner()).sources.clear();
        }
        self.try_card(&ctx, frame);
        if self.gpu && !self.chaos_tried && self.card_used() {
            self.chaos_tried = true;
            self.chaos = frame
                .wgpu_render_state()
                .and_then(|state| Chaos::new(&state.adapter, &state.device, &state.queue, &ctx));
            self.compiling = self.chaos.as_ref().is_some_and(Chaos::pending);
        }
        if window[0] > 0 && window[1] > 0 {
            if self.requested != Some(size) {
                if self.card_shows(size) {
                    self.requested = Some(size);
                    self.drawn = Some((self.view, self.adjust.zoom));
                } else if self.preview_now(&ctx, frame, size) {
                    self.hold_render(&ctx, size);
                } else {
                    self.previewing = false;
                    self.start_render(&ctx, size);
                }
            } else if self.drawn != Some((self.view, self.adjust.zoom)) && !self.frozen {
                let still = self.touched.elapsed().as_secs_f32();
                if still >= SETTLE {
                    self.start_render(&ctx, size);
                } else {
                    ctx.request_repaint_after(std::time::Duration::from_secs_f32(SETTLE - still));
                }
            }
        }
        self.chaos_frame(&ctx, frame);
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
            let [across, along, larger] = if self.adjust.zoom == self.shown_zoom || !self.gpu_on() {
                self.view.within(&self.shown, size)
            } else {
                self.view
                    .scaled(self.adjust.zoom)
                    .within(&self.shown.scaled(self.shown_zoom), size)
            };
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
                optics.tint = if self.light > phone::LIGHT {
                    0.6
                } else {
                    optics.tint * 0.25
                };
                optics.blur *= 0.5;
            } else if self.light > 0.3 {
                // The texts are white: over a light picture the glass has to be dark enough to read them.
                optics.tint = optics.tint.max((0.3 + 0.75 * self.light).min(0.92));
            }
            glass.pane(*rect, optics);
        }

        if self.bare || self.phone.preview {
            let mut panes = Vec::new();
            if self.phone.preview {
                for rect in self.clock(&ctx, content, screen) {
                    panes.push((rect, rect.height() * 0.5, false));
                }
            }
            if self.view != WHOLE && !self.phone.preview {
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
            || self.pro_open()
            || (self.languages_open && cfg!(any(target_os = "android", target_os = "ios", target_arch = "wasm32")));
        #[cfg(not(target_arch = "wasm32"))]
        let open = open && !self.export.open && !self.pro_open();
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
                            scrolled(egui::ScrollArea::vertical())
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
                            let detail = self.detail();
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
                    egui::Frame::NONE.inner_margin(egui::Margin::same(22)).show(ui, |ui| {
                        let low = content.width() > content.height() && content.height() < 560.0;
                        let wide = low.then(|| (content.width() - 60.0).min(604.0));
                        self.export_sheet(ui, window, content.height(), wide);
                    });
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

        #[cfg(feature = "store")]
        if self.pro.is_some() {
            let width = (content.width() - 84.0).min(300.0);
            let sheet = egui::Area::new(egui::Id::unique("pro"))
                .fixed_pos(content.center())
                .pivot(egui::Align2::CENTER_CENTER)
                .order(egui::Order::Foreground)
                .show(&ctx, |ui| {
                    egui::Frame::NONE
                        .inner_margin(egui::Margin::same(22))
                        .show(ui, |ui| self.pro_sheet(ui, width));
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
                "{} {:?} {} {} {} {} {:?} {}",
                self.gpu,
                self.verdict,
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
        if std::env::var_os("FRACTIQUE_GPU").is_none() {
            storage.set_string("gpu", if self.gpu { "on" } else { "off" }.to_owned());
            let verdict = match self.verdict {
                Verdict::Fast(Some(rate)) => Some(format!("fast|{rate}")),
                Verdict::Slow if !self.card_key.is_empty() && self.measured_slow => Some(SLOW.to_owned()),
                _ => None,
            };
            if let Some(verdict) = verdict {
                storage.set_string("card", format!("{}|{verdict}", self.card_key));
            }
        }
        let picture = self
            .picture
            .map_or("auto".to_owned(), |[width, height]| format!("{width}x{height}"));
        storage.set_string("picture", picture);
        storage.set_string("session", self.session());
        storage.set_string("kept", self.phone.kept_text());
        storage.set_string("recent", self.phone.recent_text());
        #[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
        if let Some(path) = std::env::var_os("FRACTIQUE_DOCUMENT") {
            let _ = std::fs::write(path, self.document());
        }
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

#[cfg(target_os = "android")]
fn focus_changed(ctx: &egui::Context) -> bool {
    static FOCUSED: AtomicBool = AtomicBool::new(false);
    let focused = ctx.input(|input| input.focused);
    FOCUSED.swap(focused, Ordering::Relaxed) != focused
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
        // From Android 15 on a window reaches under the system bars and the activity's rectangle is the whole of
        // it: the bars and the camera cutout are asked for apart, once for each size of the window.
        static BARS: Mutex<([i32; 4], [i32; 4])> = Mutex::new(([0; 4], [0; 4]));
        let whole = [free.left, free.top, free.right, free.bottom];
        let bars = BARS.lock().map_or([0; 4], |mut bars| {
            if bars.0 != whole || focus_changed(ctx) {
                *bars = (whole, android::insets());
            }
            bars.1
        });
        let window = ctx.viewport_rect();
        let inside = egui::Rect::from_min_max(
            window.min + egui::vec2(bars[0] as f32, bars[1] as f32) / scale,
            window.max - egui::vec2(bars[2] as f32, bars[3] as f32) / scale,
        );
        if rect.width() > 1.0 && rect.height() > 1.0 {
            return rect.intersect(inside);
        }
        return inside;
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
        // A window that joins its scene late does not learn which way up the screen is, and the share sheet then
        // lays itself out for a screen on its side. Giving the window its view controller again tells it.
        let root: *mut AnyObject = msg_send![window, rootViewController];
        if !root.is_null() {
            let root: *mut AnyObject = msg_send![root, retain];
            let _: () = msg_send![window, setRootViewController: std::ptr::null_mut::<AnyObject>()];
            let _: () = msg_send![window, setRootViewController: root];
            let _: () = msg_send![root, release];
        }
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

// A session written before the second generator has no number: its random flames came from the first.
fn generator_of(session: &serde_json::Value) -> Generator {
    session
        .get("generator")
        .and_then(serde_json::Value::as_u64)
        .and_then(Generator::from_number)
        .unwrap_or(Generator::First)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_session_without_a_generator_is_from_the_first() {
        let of = |text: &str| generator_of(&serde_json::from_str(text).expect("it is JSON"));
        assert_eq!(of(r#"{"random": true, "seed": 12}"#), Generator::First);
        assert_eq!(of(r#"{"generator": 1}"#), Generator::First);
        assert_eq!(of(r#"{"generator": 2}"#), Generator::Second);
        assert_eq!(of(r#"{"generator": 9}"#), Generator::First);
        assert_eq!(Generator::LATEST, Generator::Second);
    }

    #[test]
    fn the_store_builds_offer_screens_and_apple_devices() {
        let of = |store: bool| SIZES.iter().filter(move |(_, _, family)| offered(*family, store));
        assert!(of(true).all(|(_, _, family)| *family != Family::Android));
        assert!(of(true).any(|(_, size, _)| store::is_large(*size)));
        assert!(of(true).any(|(_, size, _)| !store::is_large(*size)));
        assert!(of(false).any(|(_, _, family)| *family == Family::Android));
        for group in Family::ALL {
            let listed: Vec<[u32; 2]> = SIZES
                .iter()
                .filter(|(_, _, family)| *family == group)
                .map(|(_, size, _)| *size)
                .collect();
            assert!(!listed.is_empty(), "{group:?} is empty");
            for (at, size) in listed.iter().enumerate() {
                assert!(!listed[..at].contains(size), "{size:?} is listed twice");
                assert!(parsed_size(&format!("{}x{}", size[0], size[1])).is_some());
            }
        }
    }

    #[test]
    fn a_verdict_is_kept_for_its_card_and_version() {
        let card = "Adreno 610|V@0502.0|0.1.0";
        assert_eq!(verdict_of("Adreno 610|V@0502.0|0.1.0|slow", card), Verdict::Unknown);
        assert_eq!(verdict_of("Adreno 610|V@0502.0|0.1.0|slow2", card), Verdict::Slow);
        assert_eq!(
            verdict_of("Adreno 610|V@0502.0|0.1.0|fast|1500000.5", card),
            Verdict::Fast(Some(1_500_000.5))
        );
        assert_eq!(verdict_of("Adreno 610|V@0502.0|0.0.9|slow", card), Verdict::Unknown);
        assert_eq!(verdict_of("Mali-G52|r1|0.1.0|fast|9", card), Verdict::Unknown);
        assert_eq!(verdict_of("Adreno 610|V@0502.0|0.1.0|fast|", card), Verdict::Unknown);
    }

    #[test]
    fn file_names_are_words_joined() {
        assert_eq!(file_name("aurora-spiral"), "AuroraSpiral");
        assert_eq!(file_name("Random 12"), "Random12");
        assert_eq!(file_name("my scene.v2"), "MySceneV2");
    }

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
        let (first, _) = rendered(&scene, &options, &control, &kept, true, false).expect("it renders");
        let (again, _) = rendered(&scene, &options, &control, &kept, true, false).expect("it renders");
        assert_eq!(first.rgba, whole.rgba);
        assert_eq!(again.rgba, whole.rgba);
        assert_eq!(
            kept.lock().expect("no thread panicked").layers.len(),
            scene.layers.len()
        );

        let mut nearer = scene.clone();
        for layer in &mut nearer.layers {
            layer.camera_mut().zoom *= 2.0;
        }
        rendered(&nearer, &options, &control, &kept, false, false).expect("it renders");
        let both = kept.lock().expect("no thread panicked");
        assert_eq!(both.layers.len(), 2 * scene.layers.len());
        assert_eq!(
            both.layers.values().filter(|entry| entry.home).count(),
            scene.layers.len()
        );
        assert!(both.sources.is_empty());
    }

    #[test]
    fn a_fractal_is_kept_as_a_layer_and_left_to_the_processor() {
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
        let flame = presets::get("aurora-spiral").expect("the preset exists");
        assert!(all_flames(&flame));
        for spec in fractal::all() {
            let mut scene = fractal::random_scene(spec.name, 4).expect("the kind is listed");
            assert!(!all_flames(&scene), "{}", spec.name);
            scene.layers.extend(flame.layers.clone());
            assert!(!all_flames(&scene), "{}", spec.name);
            let kept = Layers::default();
            let whole = fractique_core::render(&scene, &options, &control).expect("it renders");
            let (first, held) = rendered(&scene, &options, &control, &kept, true, false).expect("it renders");
            assert_eq!(first.rgba, whole.rgba, "{}", spec.name);
            assert!(held.is_none());

            let stop = AtomicBool::new(true);
            let stopped = Control {
                cancel: Some(&stop),
                progress: None,
            };
            let mut dimmer = scene.clone();
            dimmer.layers[1].opacity = 0.5;
            let (again, _) = rendered(&dimmer, &options, &stopped, &kept, true, false).expect("nothing is rendered");
            let expected = fractique_core::render(&dimmer, &options, &control).expect("it renders");
            assert_eq!(again.rgba, expected.rgba, "{}", spec.name);
        }
    }

    #[test]
    fn a_zoom_is_a_scaled_view() {
        let camera = presets::get("aurora-spiral").expect("the preset exists").layers[0]
            .camera()
            .clone();
        let size = [300, 200];
        let (shown, shown_zoom) = (
            View {
                right: 0.2,
                down: -0.1,
                zoom: 3.0,
            },
            0.8,
        );
        let (view, zoom) = (
            View {
                right: -0.3,
                down: 0.25,
                zoom: 5.0,
            },
            1.7,
        );
        let through = |view: View, zoom: f64| {
            let mut camera = camera.clone();
            camera.zoom *= zoom;
            camera.looking_at(view.right, view.down, view.zoom)
        };
        let (drawn, wanted) = (through(shown, shown_zoom), through(view, zoom));
        let angle = drawn.rotation.to_radians();
        let (cos, sin) = (fractique_core::math::cos(angle), fractique_core::math::sin(angle));
        let (dx, dy) = (wanted.center[0] - drawn.center[0], wanted.center[1] - drawn.center[1]);
        let up = if drawn.flip_y { -1.0 } else { 1.0 };
        let across = (dx * cos + dy * sin) * drawn.zoom / 4.0 * 200.0 / 300.0;
        let along = -(dy * cos - dx * sin) * up * drawn.zoom / 4.0;
        let [x, y, larger] = view.scaled(zoom).within(&shown.scaled(shown_zoom), size);
        assert!((f64::from(x) - 0.5 - across).abs() < 1e-5, "{x} {across}");
        assert!((f64::from(y) - 0.5 - along).abs() < 1e-5, "{y} {along}");
        assert!((f64::from(larger) - wanted.zoom / drawn.zoom).abs() < 1e-5);
    }

    #[test]
    fn kept_densities_give_the_same_picture() {
        let mut scene = presets::get("solar-mandala").expect("the preset exists");
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
        let (first, held) = rendered(&scene, &options, &control, &kept, true, true).expect("it renders");
        assert_eq!(
            first.rgba,
            fractique_core::render(&scene, &options, &control).unwrap().rgba
        );
        let (keys, sources) = held.expect("every layer has its density");
        assert_eq!(sources.len(), scene.layers.len());

        for layer in &mut scene.layers {
            let tone = layer.tone_mut();
            (tone.brightness, tone.gamma, tone.glow_strength) = (tone.brightness * 1.7, tone.gamma * 0.8, 0.6);
            tone.glow_radius = 0.02;
            layer.opacity *= 0.5;
        }
        let stop = AtomicBool::new(true);
        let stopped = Control {
            cancel: Some(&stop),
            progress: None,
        };
        let (toned, held) = rendered(&scene, &options, &stopped, &kept, true, true).expect("nothing is rendered");
        assert_eq!(
            toned.rgba,
            fractique_core::render(&scene, &options, &control).unwrap().rgba
        );
        let (again, same) = held.expect("every layer has its density");
        assert_eq!(again, keys);
        assert!(sources.iter().zip(&same).all(|(a, b)| Arc::ptr_eq(a, b)));
    }
}

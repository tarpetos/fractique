#[cfg(target_os = "android")]
mod android;
#[cfg(any(windows, test))]
mod association;
mod blur;
mod chaos;
mod colors;
mod deep;
#[cfg(target_os = "linux")]
mod desktop;
#[cfg(not(target_arch = "wasm32"))]
mod export;
mod field;
mod glass;
mod i18n;
#[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
mod instance;
#[cfg(target_os = "ios")]
mod ios;
mod motion;
mod orbit;
mod pace;
mod phone;
mod preview;
mod probe;
mod sheets;
#[cfg(any(feature = "store", test))]
mod store;
#[cfg(all(feature = "store", any(target_os = "ios", target_os = "macos")))]
mod storekit;
#[cfg(target_os = "linux")]
mod tiling;
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
// The least a picture of half the side is worth showing before the whole one: clearly more than a draft.
const SOONER_SIDE: f32 = 640.0;
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

// An Instant export larger than the picture on the screen, drawn on the graphics card a tile at a time: the
// tile on its way, and the scene of each.
#[cfg(not(target_arch = "wasm32"))]
struct Tiling {
    tiles: export::Tiles,
    scene: Scene,
    size: [u32; 2],
    cells: Vec<[u32; 4]>,
    apron: u32,
    next: usize,
}

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
    // The graphics card draws the picture: this is only the first, small one of it.
    aside: bool,
    view: View,
    zoom: f64,
    held: Option<Held>,
    image: Image,
    soft: (Vec<u8>, u32, u32),
    speed: Option<(String, bool, f64)>,
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
    ("8K", [7680, 4320], Family::Screen),
    ("16K", [15360, 8640], Family::Screen),
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
    // The newest first: the list shows five at a time, and those are the ones people hold.
    ("iPhone Duo 7.6″", [1878, 2670], Family::IPhone),
    ("iPhone Duo 5.4″", [1398, 2034], Family::IPhone),
    ("iPhone 18 Pro Max", [1320, 2868], Family::IPhone),
    ("iPhone Air", [1260, 2736], Family::IPhone),
    ("iPhone 17, 18 Pro", [1206, 2622], Family::IPhone),
    ("iPhone 15–16 Plus", [1290, 2796], Family::IPhone),
    ("iPhone 15, 16", [1179, 2556], Family::IPhone),
    ("iPhone 14 Plus", [1284, 2778], Family::IPhone),
    ("iPhone 12–14, 17e", [1170, 2532], Family::IPhone),
    ("iPhone 11 Pro Max", [1242, 2688], Family::IPhone),
    ("iPhone 11 Pro", [1125, 2436], Family::IPhone),
    ("iPhone mini", [1080, 2340], Family::IPhone),
    ("iPhone 11", [828, 1792], Family::IPhone),
    ("iPhone SE", [750, 1334], Family::IPhone),
    ("iPad Pro 13″", [2064, 2752], Family::IPad),
    ("iPad Pro 11″", [1668, 2420], Family::IPad),
    ("iPad Air 13″", [2048, 2732], Family::IPad),
    ("iPad, iPad Air 11″", [1640, 2360], Family::IPad),
    ("iPad mini (A20 Pro)", [1512, 2316], Family::IPad),
    ("iPad mini", [1488, 2266], Family::IPad),
    ("iPad Pro 11″ 2018–22", [1668, 2388], Family::IPad),
    ("iPad Pro 10.5″", [1668, 2224], Family::IPad),
    ("iPad 10.2″", [1620, 2160], Family::IPad),
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
    let height = (height - 76.0).max(32.0);
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
    // Only while nothing is being typed: a field emptied on the way to another number must stay empty.
    if custom.iter().all(String::is_empty) && ui.memory(|memory| memory.focused().is_none()) {
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

// Every size of the lists is free; one typed in needs the purchase. `None` is the size of the screen itself.
#[cfg(feature = "store")]
fn typed(chosen: Option<[u32; 2]>) -> bool {
    chosen.is_some_and(|size| !is_listed(size))
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

fn is_seed(text: &str) -> bool {
    !text.is_empty() && text.chars().all(|letter| letter.is_ascii_digit())
}

// A random picture's title is its word in the language it was made in and its seed.
fn seeded_words() -> &'static [(&'static str, String)] {
    static WORDS: std::sync::OnceLock<Vec<(&'static str, String)>> = std::sync::OnceLock::new();
    WORDS.get_or_init(|| {
        Lang::ALL
            .into_iter()
            .flat_map(|lang| {
                let texts = Texts::new(lang);
                ["random", "wallpaper"].map(|key| (key, file_name(&texts.get(key))))
            })
            .collect()
    })
}

// A title as the app shows it. A fractal's is the name of a preset or a kind and a seed, spelled as the files
// spell it or as an older build showed it: its words are given in the language, the seed is left out, and the file
// names stay as they are.
fn displayed(texts: &i18n::Texts, title: &str) -> String {
    let joined = file_name(title);
    if let Some(name) = presets::names().into_iter().find(|name| file_name(name) == joined) {
        let key = format!("n_flame_{name}");
        let text = texts.get(&key);
        return if text == key { shown(name) } else { text };
    }
    for spec in fractal::all() {
        let Some(rest) = joined.strip_prefix(&file_name(spec.name)) else {
            continue;
        };
        let kind = texts.get(&format!("f_{}", spec.name));
        if let Some(preset) = spec.presets.iter().find(|preset| file_name(preset.name) == rest) {
            return format!("{kind} {}", preset_name(texts, spec.name, preset.name));
        }
        if is_seed(rest) {
            return kind;
        }
    }
    let word = |(_, word): &&(&str, String)| joined.strip_prefix(word.as_str()).is_some_and(is_seed);
    if let Some((key, _)) = seeded_words().iter().find(word) {
        return shown(&texts.get(key));
    }
    shown(title)
}

fn preset_name(texts: &i18n::Texts, kind: &str, preset: &str) -> String {
    let key = format!("n_{kind}_{preset}");
    let text = texts.get(&key);
    if text == key { shown(preset) } else { text }
}

// A layer's name as the app gave it in one language, given in another: the name of a preset, of a kind or of a
// random picture, alone or with the layer's number after it. A name from elsewhere (a scene file) has none.
fn renamed(name: &str, from: &i18n::Texts, to: &i18n::Texts, titles: &[String]) -> Option<String> {
    let known = |name: &str| {
        let title = |title: &str| (displayed(from, title) == name).then(|| displayed(to, title));
        let flame = || presets::names().into_iter().find_map(title);
        let kind = || {
            fractal::all().find_map(|spec| {
                let preset = |preset: &fractal::Preset| title(&format!("{}-{}", spec.name, preset.name));
                title(&format!("{} 1", spec.name)).or_else(|| spec.presets.iter().find_map(preset))
            })
        };
        let word = || {
            ["random", "wallpaper"]
                .into_iter()
                .find_map(|key| title(&format!("{} 1", from.get(key))))
        };
        titles
            .iter()
            .find_map(|known| title(known))
            .or_else(flame)
            .or_else(kind)
            .or_else(word)
    };
    if name.is_empty() {
        return None;
    }
    known(name).or_else(|| {
        let (head, number) = name.rsplit_once(' ')?;
        is_seed(number)
            .then(|| known(head))
            .flatten()
            .map(|head| format!("{head} {number}"))
    })
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
#[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
const BAR_EDGE: f32 = 14.0;

// What the picture shows: the point `right` and `down` of the scene's own middle, in shorter sides of the picture,
// magnified `zoom` times.
#[derive(Clone, Copy, PartialEq, Debug)]
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

pub use probe::started;

#[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
pub fn open_at_start(path: PathBuf) {
    let _ = OPEN_FIRST.set(path);
}

#[cfg(target_os = "linux")]
pub use desktop::{app_id, integrate};

pub fn tiled() -> bool {
    #[cfg(target_os = "linux")]
    return tiling::tiled();
    #[cfg(not(target_os = "linux"))]
    false
}

#[cfg(windows)]
pub fn register_scene_files() {
    association::register();
}

#[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
pub fn first_instance(open: Option<&std::path::Path>) -> bool {
    instance::first(open)
}

// What a start from a terminal says when Fractique is already running, in the language the app was left in.
#[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
pub fn already_running(with_file: bool) -> String {
    let left = || {
        let settings = std::fs::read_to_string(eframe::storage_dir("Fractique")?.join("app.ron")).ok()?;
        let after = settings.split("\"language\":").nth(1)?;
        Some(Lang::from_locale(after.split('"').nth(1)?))
    };
    let lang = Lang::forced().or_else(left).unwrap_or_else(Lang::system);
    Texts::new(lang).get(if with_file {
        "already_running_file"
    } else {
        "already_running"
    })
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

// A picture ready for the screen, which takes it when its turn comes (`motion::turn`).
struct Offer {
    flame: wgpu::TextureView,
    soft: wgpu::TextureView,
    at: Picture,
    light: Option<f32>,
    // The textures are the ones the card goes on drawing into.
    card: bool,
    pixels: [u32; 2],
    generation: u64,
}

// A sheet on its way in or out: the name its height is kept under, the point it hangs from and how far it is shown.
struct Leaf {
    // It is open, not on its way out: what it reaches gives way to it.
    live: bool,
    height: &'static str,
    at: egui::Pos2,
    pivot: egui::Align2,
    shown: f32,
}

// A picture on the screen: the view it was drawn for, and the size of the window it was drawn for.
#[derive(Clone, Copy)]
struct Picture {
    view: View,
    zoom: f64,
    size: [u32; 2],
    // How many times wider and higher than the view the picture is: it is drawn with a margin around the view.
    wide: [f64; 2],
}

// How a picture laid over the window lies there when it is so many times wider and higher than its view.
fn spread([across, along, larger, stretch]: [f32; 4], wide: [f64; 2]) -> [f32; 4] {
    if wide == [1.0; 2] {
        return [across, along, larger, stretch];
    }
    let [wider, higher] = wide.map(|side| side as f32);
    let (over, down) = if stretch > 1.0 {
        (larger * wider, larger * higher / stretch)
    } else {
        (larger * wider * stretch, larger * higher)
    };
    let at = [0.5 + (across - 0.5) / wider, 0.5 + (along - 0.5) / higher];
    if over > down {
        [at[0], at[1], over, over / down]
    } else {
        [at[0], at[1], down, over / down]
    }
}

// How a picture drawn for a window of one size lies in a window of another, before it is drawn again: its
// aspect over the screen's, and how many times larger it is shown. A picture of a chosen size is fitted whole;
// one that follows the window keeps the scale the new picture will have, which goes by the shorter side.
fn laid(drawn: [u32; 2], window: [u32; 2], fixed: bool, screen: egui::Vec2) -> (f32, f32) {
    let [width, height] = drawn.map(|side| side.max(1) as f32);
    if fixed || drawn == window {
        return ((width / height) / (screen.x / screen.y).max(0.001), 1.0);
    }
    let [new_width, new_height] = window.map(|side| side.max(1) as f32);
    let scale = width.min(height) / new_width.min(new_height);
    let (across, along) = (new_width * scale / width, new_height * scale / height);
    let stretch = along / across;
    (stretch, if stretch > 1.0 { 1.0 / across } else { 1.0 / along })
}

// How the picture the screen holds lies in the window for a view: the transform from its own view to that one,
// whole, however far apart the two are.
fn lain(eye: (View, f64), shown: &Picture, card: bool, window: [u32; 2], fixed: bool, screen: egui::Vec2) -> [f32; 4] {
    let [across, along, larger] = stand_in(eye, shown, card);
    let (stretch, closer) = laid(shown.size, window, fixed, screen);
    spread([across, along, larger * closer, stretch], shown.wide)
}

fn stand_in((view, zoom): (View, f64), shown: &Picture, card: bool) -> [f32; 3] {
    let [across, along, larger] = if zoom == shown.zoom || !card {
        view.within(&shown.view, shown.size)
    } else {
        view.scaled(zoom).within(&shown.view.scaled(shown.zoom), shown.size)
    };
    [across, along, if larger.is_finite() { larger } else { 1.0 }]
}

// A wallpaper of a chosen size fills a window of another shape: it is shown up to this many times larger than
// fitted whole, and what is over is cut. Past that its soft continuation is beside it.
const FILL_MOST: f32 = 1.25;

// How many times larger than fitted whole a picture that fills the window is shown, and the shares of it the
// window then shows across and down. `stretch` is the picture's aspect over the window's.
fn filled(stretch: f32) -> (f32, [f64; 2]) {
    let cover = stretch.max(1.0 / stretch.max(1e-6));
    let fill = cover.min(FILL_MOST);
    let (cut, left) = (1.0 / f64::from(fill), f64::from(cover / fill));
    (fill, if stretch > 1.0 { [cut, left] } else { [left, cut] })
}

// The pixels on each side of a picture of a chosen size that a window of another shape shows beside it: the
// card draws them with the picture, up to half of its side.
fn strips(picture: [u32; 2], window: [u32; 2]) -> [u32; 2] {
    let [width, height] = picture.map(f64::from);
    let [across, down] = window.map(|side| f64::from(side.max(1)));
    let beside = |seen: f64, side: f64| {
        let each = (seen - side) / 2.0;
        if each < 0.5 {
            0
        } else {
            (each.ceil() as u32 + 1).min(side as u32 / 2)
        }
    };
    [
        beside(height * across / down, width),
        beside(width * down / across, height),
    ]
}

const GLASS: (f32, f32) = (0.4, 0.7);
const MAX_MAGNIFICATION: f64 = 1e10;
const MIN_MAGNIFICATION: f64 = 1e-4;
// The smallest a view shows a picture that is drawn anew for it, a flame or a fractal of any kind, as a share of
// the picture's own framing.
const FLOOR: f64 = 0.2;
// A moved view the card draws in less than this many seconds is shown as it is drawn, without a blur.
const BRISK: f32 = 0.1;
// A moved view the card draws in less than this many seconds, half a frame, is drawn whole for every frame: it
// needs no draft.
const INSTANT: f32 = 0.5 * pace::FRAME;
// A picture that is only magnified is shown at most this many times larger.
const IMAGE_CEILING: f64 = 8.0;
// A flame is magnified as far as some view of it still holds this share of its light: a deep view is drawn from
// particles that descend from the few inside it, and below about a hundred lines of descent two renders of one
// view are two pictures (docs/performance.md, "Where a view stops being a picture").
const ENOUGH: f64 = 1.0 / 512.0;
const FLAME_DEEPEST: (f64, f64) = (10.0, 300.0);
const DEPTH_STEPS: i32 = 20;
const DEPTH_SIZE: [u32; 2] = [480, 270];
const DEPTH_SAMPLES: u32 = 8;

// Where the grain of an attractor's samples takes over, measured at 3840 × 2160 in its brightest place.
fn orbit_deepest(kind: &str) -> Option<f64> {
    match kind {
        "buddhabrot" => Some(10.0),
        "clifford" | "de_jong" | "hopalong" => Some(30.0),
        "gumowski_mira" => Some(50.0),
        "lorenz" => Some(100.0),
        _ => None,
    }
}

// The deepest zoom at which some view of the whole picture still holds `ENOUGH` of a layer's light, from the
// levels of a small density of it. tone.rs: a level is ln(1 + light) / 8.
fn flame_deepest(levels: &[f32], [width, height]: [usize; 2]) -> f64 {
    let light = |cell: &[f32]| fractique_core::math::exp(f64::from(cell[3]) * 8.0) - 1.0;
    let across = width + 1;
    let mut sums = vec![0.0; across * (height + 1)];
    for (at, cell) in levels.chunks_exact(4).take(width * height).enumerate() {
        let (x, y) = (at % width, at / width);
        sums[(y + 1) * across + x + 1] =
            light(cell) + sums[y * across + x + 1] + sums[(y + 1) * across + x] - sums[y * across + x];
    }
    let all = sums[across * (height + 1) - 1];
    if all <= 0.0 {
        return FLAME_DEEPEST.0;
    }
    let held = |zoom: f64| {
        let (wide, high) = (width as f64 / zoom, height as f64 / zoom);
        let (cells_wide, cells_high) = (
            (wide.ceil() as usize).clamp(1, width),
            (high.ceil() as usize).clamp(1, height),
        );
        let mut most = 0.0f64;
        for y in 0..=height - cells_high {
            for x in 0..=width - cells_wide {
                let (right, below) = (x + cells_wide, y + cells_high);
                let inside = sums[below * across + right] - sums[y * across + right] - sums[below * across + x]
                    + sums[y * across + x];
                most = most.max(inside);
            }
        }
        most / all * (wide * high) / (cells_wide * cells_high) as f64
    };
    let (least, most) = FLAME_DEEPEST;
    let at = |step: i32| least * fractique_core::math::pow(most / least, f64::from(step) / f64::from(DEPTH_STEPS));
    let deepest = (1..=DEPTH_STEPS).take_while(|step| held(at(*step)) >= ENOUGH).last();
    deepest.map_or(least, at)
}

// How deep the flames of the picture on the screen go: worked out once a picture, beside the window's thread.
struct Depth {
    key: String,
    ceiling: f64,
    asked: Option<mpsc::Receiver<(String, f64)>>,
}
// A new picture is drawn once the view has stood still this long.
const SETTLE: f32 = 0.18;

impl View {
    // A view of a test run's FRACTIQUE_THEN: zoom, or zoom:right:down.
    fn asked(text: &str) -> Option<View> {
        let numbers: Vec<f64> = text.split(':').filter_map(|number| number.parse().ok()).collect();
        match *numbers.as_slice() {
            [zoom] => Some(View { zoom, ..WHOLE }),
            [zoom, right, down] => Some(View { right, down, zoom }),
            _ => None,
        }
    }

    fn moved(&mut self, by: [f64; 2]) {
        self.right = (self.right - by[0] / self.zoom).clamp(-1e6, 1e6);
        self.down = (self.down - by[1] / self.zoom).clamp(-1e6, 1e6);
    }

    // The limits of a view. It is never smaller than `least` times the whole picture, and it lies inside the
    // picture's territory, which is the same at every scale; a side of the view that is longer than the
    // territory's has it in its middle. `covering` is for a picture that is only ever shown as it is, larger (a
    // wallpaper): the share of it the window shows across and down when the view is the whole picture, 1, or less
    // on the side where it fills a window of another shape and is cut.
    fn limited(&mut self, size: [u32; 2], least: f64, covering: Option<[f64; 2]>) {
        self.zoom = self.zoom.max(least);
        let (territory, held) = (territory(size, least, covering), self.held(size, covering));
        let free = [0, 1].map(|side| (territory[side] - held[side]).max(0.0));
        self.right = self.right.clamp(-free[0], free[0]);
        self.down = self.down.clamp(-free[1], free[1]);
    }

    // Half of what the view shows across and down, in shorter sides of the picture.
    fn held(&self, size: [u32; 2], covering: Option<[f64; 2]>) -> [f64; 2] {
        let shorter = f64::from(size[0].min(size[1]).max(1));
        let shares = covering.unwrap_or([1.0; 2]);
        [0, 1].map(|side| 0.5 * shares[side] * f64::from(size[side]) / shorter / self.zoom)
    }

    // The wheel, a pinch, the slider or a test run's step: the view made larger or smaller around a point, which
    // stays where it is unless the territory's edge is in the way of a view that grows.
    fn zoomed(&mut self, factor: f64, around: [f64; 2], size: [u32; 2], reach: (f64, f64), covering: Option<[f64; 2]>) {
        self.magnified(factor, around, reach.0, reach.1);
        self.limited(size, reach.0, covering);
    }

    fn magnified(&mut self, factor: f64, around: [f64; 2], least: f64, most: f64) {
        let held = [self.right + around[0] / self.zoom, self.down + around[1] / self.zoom];
        self.zoom = (self.zoom * factor).clamp(least, most);
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
}

// Half of the sides of a picture's territory, in shorter sides of the picture: all that a view of it may show,
// whatever its scale. A wallpaper's is the wallpaper. Any other picture's is what its smallest view shows moved
// as far as the picture's framing stays inside that view: nine framings across and nine down.
fn territory(size: [u32; 2], least: f64, covering: Option<[f64; 2]>) -> [f64; 2] {
    let shorter = f64::from(size[0].min(size[1]).max(1));
    let half = if covering.is_some() { 0.5 } else { 1.0 / least - 0.5 };
    size.map(|side| half * f64::from(side) / shorter)
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

// A test run with a list open, alone or over something else: kinds, phone-tune-kinds, phone-tune-sizes.
fn opens(what: &str) -> bool {
    std::env::var("FRACTIQUE_SHOW").is_ok_and(|shown| shown.split('-').any(|part| part == what))
}

// A test run with the list of recent pictures open: recent, fractals-recent, wall-recent.
fn shows_recent() -> bool {
    std::env::var("FRACTIQUE_SHOW").is_ok_and(|shown| shown.contains("recent"))
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

const TRIAL: &str = "trial";

// Whose the card's step of a frame is.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Turn {
    Trial,
    Draft,
    Picture,
    Nobody,
}

// By the run that is on the card, not by whether a trial is on: a picture asked for during the trial is drawn by
// the card too, and nothing else would take its units in.
fn turn(keys: Option<&[String]>, draft: Option<&[String]>) -> Turn {
    match keys {
        Some(keys) if keys == [TRIAL] => Turn::Trial,
        Some(keys) if draft == Some(keys) => Turn::Draft,
        Some(_) => Turn::Picture,
        None => Turn::Nobody,
    }
}

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
    choices: colors::Choices,
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
    choices: colors::Choices,
    phone: phone::Phone,
    title: String,
    is_random: bool,
    seed: u64,
    generator: Generator,
    adjust: Adjust,
    view: View,
    shown: View,
    shown_zoom: f64,
    shown_wide: [f64; 2],
    // The picture on the screen is a finished one, not the first, small one of a picture on its way.
    shown_whole: bool,
    // A test run's margin around the view, as a share of its sides, in place of the one the app works out.
    margin_asked: Option<f64>,
    // The seconds the card took for the view alone of the picture it drew last; whether the picture on the card
    // has been drawn again with a margin, or left without one, and whether the screen has that one.
    took: Option<f32>,
    widened: bool,
    wide_drawn: bool,
    // The finished picture of a view the picture on the screen still holds is not shown before it has its
    // margin: the seconds it took, which is the time it is shown with.
    deferred: Option<f32>,
    drawn: Option<(View, f64)>,
    // The view of the picture that is rendered or on the screen, and whether the screen shows it magnified
    // because the view itself would take too long to render.
    aimed: (View, f64),
    magnified: bool,
    given_up: bool,
    // The picture awaited is the card's alone: the processor renders its small draft at the most.
    carded: bool,
    speeds: pace::Speeds,
    // A render that was stopped because it would have taken too long: its generation, and the speed it showed.
    late: Late,
    too_long: Option<(View, f64)>,
    touched: Instant,
    gpu: bool,
    gpu_chosen: bool,
    gpu_warned: bool,
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
    // The card draws the whole scene pixel by pixel: a moved view is drawn again at once, without a rest.
    swift: bool,
    // The card draws the picture within a frame: a moved view needs no draft.
    at_once: bool,
    // The card went silent over the picture on its way and was asked again.
    stalled: bool,
    // The draft of a view that moves: how small it is drawn, the run that draws it, the view it is of, the view
    // it is drawn for with its margin, whether it has been shown, and the view of the draft on the screen.
    draft: pace::Draft,
    draft_keys: Option<Vec<String>>,
    draft_of: (View, f64),
    draft_view: View,
    draft_landed: bool,
    draft_shown: Option<(View, f64)>,
    depth: Depth,
    draft_note: ([u32; 2], f32),
    scale_note: (f64, f64, f32),
    // The card gave a draft of this scene up: its views wait for the picture as they did.
    draft_off: bool,
    wheeled: Option<(f64, u32)>,
    chaos_at: (View, f64),
    quick: (Option<f32>, Option<f32>),
    look: String,
    asked: Instant,
    measured: Arc<AtomicU32>,
    #[cfg(not(target_arch = "wasm32"))]
    instant: Option<(export::Later, [u32; 2], Option<[f32; 3]>)>,
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
    fades: Vec<f32>,
    stage: motion::Stage,
    under: f32,
    // Where the sheets stand, how far each is shown and whether it is open or on its way out.
    covers: Vec<(egui::Rect, f32, bool)>,
    covering: Vec<(egui::Rect, f32, bool)>,
    seats: Vec<(&'static str, egui::Rect)>,
    wide: Option<bool>,
    veil: motion::Veil,
    fresh: motion::Motion,
    fading: bool,
    veiled: bool,
    resizing: bool,
    let_go: bool,
    brisk: bool,
    began: Instant,
    begun: bool,
    waited: Instant,
    counting: bool,
    last_size: [u32; 2],
    used: Instant,
    turned: Option<Instant>,
    moved_only: bool,
    pictures: u32,
    offers: motion::Offers<Offer>,
    shown_of: (u64, bool),
    scene_from: u64,
    handing: bool,
    live_card: bool,
    veil_level: f32,
    shown_eye: Option<(View, f64)>,
    past_eye: Option<(View, f64)>,
    last_eye: (View, f64),
    past: Option<Picture>,
    shown_size: Option<[u32; 2]>,
    aimed_size: [u32; 2],
    fit_asked: [u32; 2],
    drawn_with: [u32; 2],
    #[cfg(not(target_arch = "wasm32"))]
    tiling: Option<Tiling>,
    stood: motion::Rest<[u32; 2]>,
    // The window in pixels, since when it has that shape, and what a picture of a chosen size was last drawn
    // with beside it for that shape.
    window: [u32; 2],
    shape_stood: motion::Rest<[u32; 2]>,
    strips_asked: [u32; 2],
    // The shares of a wallpaper the window shows across and down when the view is the whole of it.
    seen: Option<[f64; 2]>,
    leaf_top: f32,
    toolbar_size: egui::Vec2,
    languages_open: bool,
    picture: Option<[u32; 2]>,
    then: Vec<View>,
    steps: Vec<String>,
    tap: Option<(egui::Pos2, u8)>,
    dragged: Option<(egui::Pos2, egui::Pos2, u32, u32)>,
    escape: bool,
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
    // The window's size, its place on the screen and the frame its panes stood in, a frame ago.
    #[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
    window_was: Option<(egui::Rect, egui::Pos2, egui::Rect)>,
    // When a button of the app asked the system for another size of the window.
    #[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
    leaping: Option<f64>,
    #[cfg(windows)]
    handle: Option<isize>,
    leap: Option<motion::Leap>,
    landed: bool,
    leap_from: egui::Rect,
    lay_was: motion::Lay,
    reset_home: Option<egui::Pos2>,
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
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    last_frame: Instant,
    trace: probe::Probe,
    #[cfg(not(target_arch = "wasm32"))]
    export: export::Sheet,
    #[cfg(feature = "store")]
    store: Option<store::Store>,
    #[cfg(feature = "store")]
    pro: Option<Wanted>,
    #[cfg(feature = "store")]
    saves: store::Saves,
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
type Late = Arc<Mutex<Option<(u64, Option<(String, f64)>)>>>;
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

// The kinds the processor alone draws and takes long over: marked in the list, left out on a phone.
const HEAVY: [&str; 9] = [
    "buddhabrot",
    "hopalong",
    "gumowski_mira",
    "mandelbulb",
    "mandelbox",
    "menger_sponge",
    "kleinian",
    "lyapunov",
    "l_system",
];

fn heavy(kind: &str) -> bool {
    HEAVY.contains(&kind)
}

// The graphics card draws the scene by itself: as densities, or all of it pixel by pixel, or as a wallpaper.
fn on_card(scene: &Scene) -> bool {
    sheets::drawn(scene) || all_densities(scene) || (scene.sheets.is_none() && scene.layers.iter().all(field::drawn))
}

// What the layers of a scene cost the processor at this size.
fn parts<'a>(scene: &'a Scene, options: &RenderOptions) -> Vec<pace::Part<'a>> {
    let pixels = f64::from(options.width) * f64::from(options.height);
    let samples = pixels * f64::from(options.samples_per_pixel);
    let part = |layer: &'a Layer| {
        let Some(shown) = layer.fractal() else {
            return pace::Part {
                kind: pace::FLAME,
                work: samples,
                stepped: false,
            };
        };
        let spec = fractal::spec(&shown.kind);
        let stepped = !spec.is_some_and(fractal::Spec::is_density);
        let iterations = || {
            let param = spec?.params.iter().find(|param| param.name == "iterations")?;
            Some(shown.params.get(param.name).copied().unwrap_or(param.default))
        };
        let steps = || field::steps_a_pixel(shown, options).or_else(iterations).unwrap_or(1.0);
        pace::Part {
            kind: &shown.kind,
            work: if stepped { pixels * steps() } else { samples },
            stepped,
        }
    };
    scene.layers.iter().map(part).collect()
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
) -> Result<(Image, Option<Sources>, bool), RenderError> {
    static USES: AtomicU64 = AtomicU64::new(0);
    let size = size_key(options);
    let mut fresh = true;
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
        fresh &= found.is_none() && source.is_none();
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
    Ok((image, held, fresh))
}

// A scroll area left to itself starts two rows tall and can stay so.
// On a phone a list stands in the middle and grows until the phone's outer margin is left above and below it; on a
// computer it has the window's height to itself, as it had.
fn list_place(content: egui::Rect, phone: bool) -> (egui::Pos2, egui::Align2, f32) {
    let high = content.height();
    let room = if phone {
        high - 2.0 * widgets::OUTER + LIST_SPARE
    } else {
        high
    };
    (content.center(), egui::Align2::CENTER_CENTER, room)
}

// What `list_height` leaves of its room beyond the rows, less what a list's heading and padding take.
const LIST_SPARE: f32 = 110.0 - LIST_FRAME;
const LIST_FRAME: f32 = 2.0 * widgets::PAD + widgets::CLOSE + 6.0;

// How wide a sheet's content is. On a phone every sheet is as wide as the screen leaves inside the outer margin and
// the padding, up to a width that reads well on a phone on its side; on a computer each has its own.
fn sheet_width(screen: f32, natural: f32, phone: bool) -> f32 {
    if phone {
        (screen - 2.0 * (widgets::OUTER + widgets::PAD)).min(natural.max(300.0))
    } else {
        natural
    }
}

fn list_height(rows: usize, room: f32) -> f32 {
    (rows as f32 * 32.0 - 2.0).clamp(30.0, (room - 110.0).max(90.0))
}

fn chaos_keys(scene: &Scene, options: &RenderOptions, margin: [u32; 2]) -> Vec<String> {
    let mut size = size_key(options);
    if margin != [0; 2] {
        size += &format!("+{}+{} ", margin[0], margin[1]);
    }
    if sheets::drawn(scene) {
        return vec![size + &sheets::key(scene)];
    }
    (0..scene.layers.len())
        .map(|index| source_key(scene, index, &size) + &field::key(&scene.layers[index]))
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

// The verdict kept for a graphics card, if it is this card's and this version's.
// Not "slow": that was kept by trials that asked a card to be twice as fast on a smaller picture. Not "slow2":
// that was kept by trials whose units were nearly all the way to a phone's chip and back.
const SLOW: &str = "slow3";

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

// How many times larger than its own framing a picture is shown, as the detail line and the controls say it: the
// one number of a view's scale, the wheel's and the controls' together.
fn scale_label(scale: f64) -> String {
    match scale {
        zoom if zoom >= 999_999.5 => format!("{zoom:.1e}"),
        zoom if zoom >= 99.95 => format!("{zoom:.0}"),
        zoom if zoom >= 9.995 => format!("{zoom:.1}"),
        zoom => format!("{zoom:.2}"),
    }
}

fn status_of(size: [u32; 2], seconds: f32, scale: f64) -> String {
    let mut status = format!("{} × {}  ·  {seconds:.2} s", size[0], size[1]);
    if (scale - 1.0).abs() > 1e-9 {
        status += &format!("  ·  {}×", scale_label(scale));
    }
    status
}

// The pixels a picture of a chosen size is drawn with for the screen: as many as it has in the window, where it
// lies whole, or cut by as much as it is shown larger to fill the window, and never more than it has. What the
// card and the memory hold goes by the window, whatever size is chosen.
fn on_screen(size: [u32; 2], window: [u32; 2], fill: f32) -> [u32; 2] {
    if window[0] == 0 || window[1] == 0 {
        return size;
    }
    let [across, down] = [0, 1].map(|side| size[side] as f32 / window[side] as f32);
    let shrink = (across.max(down) / fill.max(1.0)).max(1.0);
    size.map(|side| ((side as f32 / shrink).round() as u32).max(1))
}

// How many times finer a side the picture on the screen is drawn when it stands for a larger one: the pixels of
// that one a pixel of the screen covers, at most FINEST a side. The processor draws the screen's picture plain.
const FINEST: u32 = 4;

fn finer(size: [u32; 2], drawn: [u32; 2]) -> u32 {
    let times = size[0] as f32 / drawn[0].max(1) as f32;
    (times.round() as u32).clamp(1, FINEST)
}

fn plain(options: &RenderOptions) -> RenderOptions {
    RenderOptions {
        supersample: 1,
        ..*options
    }
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
        let mut probe = probe::Probe::new();
        probe.stage("app");
        if screenshot.is_none() {
            probe.sham = None;
        }
        let stored = |key: &str| cc.storage.and_then(|storage| storage.get_string(key));
        let lang = Lang::forced()
            .or_else(|| stored("language").map(|code| Lang::from_locale(&code)))
            .unwrap_or_else(Lang::system);
        let texts = Texts::new(lang);
        let glass = |key: &str, default: f32| stored(key).and_then(|text| text.parse().ok()).unwrap_or(default);
        let (opacity, blur) = (glass("glass_opacity", GLASS.0), glass("glass_blur", GLASS.1));
        probe.stage("texts");
        widgets::install_fonts(&cc.egui_ctx, texts.lang);
        probe.stage("fonts");
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
            let gfx = Gfx::new(&state.device, &state.queue, state.target_format);
            state.renderer.write().callback_resources.insert(gfx);
        }
        probe.stage("glass");
        // The card accelerates only where it can play the chaos game; elsewhere the app is the processor's alone. A
        // browser page renders on the thread that draws, so there the card's picture would wait for it.
        // FRACTIQUE_GPU makes a test run a device that cannot (none), one whose card is slow (slow), one that is
        // not asked (fast) or one that is asked anew (measure).
        let forced = std::env::var("FRACTIQUE_GPU").ok();
        let able = cfg!(not(target_arch = "wasm32")) && forced.as_deref() != Some("none");
        let preview = cc
            .wgpu_render_state
            .as_ref()
            .filter(|state| able && Chaos::able(&state.adapter, &state.device))
            .and_then(|state| Preview::new(&state.adapter, &state.device, &state.queue, &cc.egui_ctx));
        probe.stage("preview");
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
        let gpu_chosen = forced.is_some() || stored("gpu_by").is_some();
        let gpu = match forced.as_deref() {
            Some(choice) => choice != "off",
            None if gpu_chosen => stored("gpu").is_none_or(|choice| choice != "off"),
            None => verdict != Verdict::Slow,
        };
        let restore = screenshot.is_none() || std::env::var_os("FRACTIQUE_RESTORE").is_some();
        let session = if restore { stored("session") } else { None };
        let title = "aurora-spiral".to_owned();
        let first = displayed(&texts, &title);
        let walls = opens("layouts") || opens("materials");
        let mut app = Self {
            texts,
            base: named(presets::get(&title).unwrap_or_else(|| random_scene(1)), &first),
            wall: Wall {
                on: std::env::var("FRACTIQUE_SHOW").is_ok_and(|what| what.contains("wall")) || walls,
                live: std::env::var("FRACTIQUE_SHOW").is_ok_and(|what| what.contains("wall")) || walls,
                ..Default::default()
            },
            rest: [None, None, None],
            fractals: false,
            kind: "mandelbrot".to_owned(),
            latest: None,
            choices: colors::Choices::default(),
            phone: phone::Phone::new(stored("kept"), stored("recent")),
            title,
            is_random: false,
            seed: 0,
            generator: Generator::First,
            adjust: NEUTRAL,
            view: WHOLE,
            shown: WHOLE,
            shown_zoom: 1.0,
            shown_wide: [1.0; 2],
            shown_whole: false,
            margin_asked: std::env::var("FRACTIQUE_MARGIN")
                .ok()
                .and_then(|share| share.parse().ok()),
            took: None,
            widened: false,
            wide_drawn: false,
            deferred: None,
            drawn: None,
            aimed: (WHOLE, 1.0),
            magnified: false,
            given_up: false,
            carded: false,
            late: Arc::default(),
            too_long: None,
            speeds: {
                let mut speeds = pace::Speeds::default();
                if let Verdict::Fast(Some(rate)) = verdict {
                    speeds.seen(pace::FLAME, false, rate);
                }
                speeds
            },
            touched: Instant::now(),
            gpu,
            gpu_chosen,
            gpu_warned: false,
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
            swift: false,
            at_once: false,
            stalled: false,
            draft: pace::Draft::new(),
            draft_keys: None,
            draft_of: (WHOLE, 1.0),
            draft_view: WHOLE,
            draft_landed: false,
            draft_shown: None,
            depth: Depth {
                key: String::new(),
                ceiling: FLAME_DEEPEST.1,
                asked: None,
            },
            draft_note: ([0; 2], 0.0),
            scale_note: (0.0, 0.0, 0.0),
            draft_off: false,
            wheeled: None,
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
            tuning: opens("tuning"),
            inspector_open: None,
            panes: Vec::new(),
            fades: Vec::new(),
            stage: motion::Stage::default(),
            under: 0.0,
            covers: Vec::new(),
            covering: Vec::new(),
            seats: Vec::new(),
            wide: None,
            veil: motion::Veil::new(),
            fresh: motion::Motion::at(1.0),
            fading: false,
            veiled: false,
            resizing: false,
            let_go: false,
            brisk: false,
            began: Instant::now(),
            begun: false,
            waited: Instant::now(),
            counting: false,
            last_size: [0; 2],
            used: Instant::now(),
            turned: None,
            moved_only: false,
            pictures: 0,
            offers: motion::Offers::new(),
            shown_of: (0, true),
            scene_from: 0,
            handing: false,
            live_card: false,
            veil_level: 0.0,
            shown_eye: None,
            past_eye: None,
            last_eye: (WHOLE, 1.0),
            past: None,
            shown_size: None,
            aimed_size: [0; 2],
            fit_asked: [0; 2],
            drawn_with: [0; 2],
            #[cfg(not(target_arch = "wasm32"))]
            tiling: None,
            stood: motion::Rest::new(),
            window: [0; 2],
            shape_stood: motion::Rest::new(),
            strips_asked: [0; 2],
            seen: None,
            leaf_top: 0.0,
            toolbar_size: egui::Vec2::ZERO,
            languages_open: opens("languages"),
            language_anchor: egui::Pos2::ZERO,
            picture: std::env::var("FRACTIQUE_PICTURE")
                .ok()
                .or_else(|| stored("picture"))
                .and_then(|text| parsed_size(&text)),
            then: std::env::var("FRACTIQUE_THEN").map_or_else(
                |_| Vec::new(),
                |views| views.split(',').rev().filter_map(View::asked).collect(),
            ),
            steps: std::env::var("FRACTIQUE_STEPS").map_or_else(
                |_| Vec::new(),
                |steps| steps.split(',').rev().map(str::to_owned).collect(),
            ),
            tap: None,
            dragged: None,
            escape: false,
            custom: Default::default(),
            typed: None,
            sizes_open: opens("sizes"),
            presets_open: ["presets", "layouts", "materials", "kinds"].into_iter().any(opens) || shows_recent(),
            listing: if opens("layouts") {
                Listing::Layouts
            } else if opens("materials") {
                Listing::Materials
            } else if opens("kinds") {
                Listing::Kinds
            } else if shows_recent() {
                Listing::Recent
            } else {
                Listing::Presets
            },
            adding_layer: false,
            layer: 0,
            #[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
            maximize_again: false,
            #[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
            window_was: None,
            #[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
            leaping: None,
            #[cfg(windows)]
            handle: None,
            leap: None,
            landed: false,
            leap_from: egui::Rect::ZERO,
            lay_was: [0.5, 0.5, 1.0, 1.0],
            reset_home: None,
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
            trace: probe,
            #[cfg(not(target_arch = "wasm32"))]
            export: export::Sheet {
                open: std::env::var("FRACTIQUE_SHOW").is_ok_and(|what| what.starts_with("export")),
                quality: std::env::var("FRACTIQUE_QUALITY")
                    .ok()
                    .and_then(|index| index.parse().ok())
                    .filter(|index| *index < export::QUALITIES.len())
                    .unwrap_or(0),
                size: std::env::var("FRACTIQUE_EXPORT_SIZE")
                    .ok()
                    .and_then(|text| parsed_size(&text)),
                ..Default::default()
            },
            #[cfg(feature = "store")]
            store: store::open(&cc.egui_ctx),
            #[cfg(feature = "store")]
            saves: stored("saves").map_or_else(store::Saves::default, |text| store::Saves::parse(&text)),
            #[cfg(feature = "store")]
            pro: std::env::var("FRACTIQUE_SHOW")
                .is_ok_and(|what| what.starts_with("pro"))
                .then_some(Wanted::Nothing),
            #[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
            updates: updates::Updates::new(),
            #[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
            arrivals: instance::watch(&cc.egui_ctx),
        };
        app.trace.stage("made");
        if let Some(session) = session {
            let _ = app.restore(&session);
            app.view = WHOLE;
        }
        // A test run in the Fractals tab: FRACTIQUE_SHOW=fractals (also kinds, fractals-phone, fractals-phone-sheet),
        // with FRACTIQUE_FRACTAL=julia for a kind other than the first.
        if std::env::var("FRACTIQUE_SHOW").is_ok_and(|what| what.contains("fractals")) || opens("kinds") {
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
        app.trace.stage("ready");
        app
    }

    // The scene with every layer in the colors chosen for it, if any were.
    fn dressed(&self) -> Scene {
        let mut scene = self.base.clone();
        self.choices.dress(&mut scene);
        scene
    }

    fn chosen_layer(&self) -> usize {
        self.layer.min(self.base.layers.len().saturating_sub(1))
    }

    fn choice(&self) -> Option<usize> {
        self.choices.of(self.chosen_layer())
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

    fn display(&self, title: &str) -> String {
        displayed(&self.texts, title)
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
            self.pick(named(scene, &self.display(name)), name.to_owned(), false);
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
                choices: self.choices.clone(),
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
            self.choices = rest.choices;
        }
        self.wall.live = walls;
        self.wall.held = rest.held.or(self.wall.held);
        (self.title, self.is_random, self.seed) = (rest.title, rest.is_random, rest.seed);
        (self.view, self.layer) = (rest.view, rest.layer);
        self.phone.origin = phone::Origin::default();
        self.cancel.store(true, Ordering::Relaxed);
        self.cancel = Arc::new(AtomicBool::new(false));
        self.generation += 1;
        self.scene_from = self.generation;
        self.offers.begin(self.generation, true);
        match rest.picture {
            Some((mut picture, size)) if Some(size) == self.requested => {
                picture.generation = self.generation;
                self.aimed_size = size;
                self.drawn = Some((rest.view, self.adjust.zoom));
                self.aimed = (picture.view, picture.zoom);
                self.magnified = picture.view != rest.view;
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

    // How the picture of this size is drawn for the screen: a chosen size is a canvas, and the screen shows it
    // with the window's pixels.
    // A canvas larger than the screen shows it is drawn as that picture scaled down: so many times finer a side.
    fn full(&self, size: [u32; 2]) -> RenderOptions {
        let drawn = fitted(self.fit(size), FULL_SIDE, FULL_SAMPLES);
        let finer = if self.picture.is_some() && !self.covering() {
            finer(size, [drawn.width, drawn.height])
        } else {
            1
        };
        RenderOptions {
            supersample: finer,
            ..drawn
        }
    }

    fn fit(&self, size: [u32; 2]) -> [u32; 2] {
        if self.picture.is_none() {
            return size;
        }
        let [width, height] = self.window.map(|side| side.max(1) as f32);
        let stretch = (size[0] as f32 / size[1].max(1) as f32) / (width / height);
        let fill = if self.covering() { filled(stretch).0 } else { 1.0 };
        on_screen(size, self.window, fill)
    }

    fn card_used(&self) -> bool {
        self.preview.as_ref().is_some_and(Preview::works)
    }

    // Until the trial has told, a switch nobody has set leaves the pictures to the processor.
    fn gpu_on(&self) -> bool {
        self.gpu && self.card_used() && (self.gpu_chosen || self.verdict != Verdict::Unknown)
    }

    // The graphics card's own picture of the scene as it is now is on the screen. It follows the tone, the
    // opacities and the blends by itself, so nothing has to be rendered.
    fn card_shows(&self, size: [u32; 2]) -> bool {
        if !self.chaotic || !self.gpu_on() {
            return false;
        }
        let full = self.full(size);
        let keys = chaos_keys(&self.scene_at(size), &full, self.strips_asked);
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
        let turn = !self.live_card && !self.fresh.moving(self.stage.now);
        gfx.show(&state.device, picture, soft, turn);
        drop(renderer);
        if turn {
            self.turn_picture(motion::PANEL);
        }
        self.live_card = true;
        self.pictures += 1;
        self.previewing = true;
        true
    }

    // The render waits until the controls rest, as it does for a moved view.
    fn hold_render(&mut self, ctx: &egui::Context, size: [u32; 2]) {
        self.cancel.store(true, Ordering::Relaxed);
        self.cancel = Arc::new(AtomicBool::new(false));
        self.generation += 1;
        self.offers.begin(self.generation, false);
        self.requested = Some(size);
        self.drawn = None;
        self.touched = Instant::now();
        self.rendering = true;
        self.carded = false;
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
        self.begin();
    }

    // Whether the picture of the view is expected soon enough to be rendered: the graphics card draws it, or the
    // processor has shown the speed for it. The whole view is rendered however long it takes, and a wallpaper has
    // no other view.
    fn fast(&mut self, size: [u32; 2]) -> bool {
        self.fast_at(self.view, size)
    }

    fn fast_at(&mut self, view: View, size: [u32; 2]) -> bool {
        let zoom = self.adjust.zoom;
        if view == WHOLE || self.wall.live || sheets::drawn(&self.base) || self.too_long == Some((view, zoom)) {
            return view == WHOLE;
        }
        let full = self.full(size);
        let scene = self.scene_in(view, zoom);
        let deep = view.zoom * zoom >= pace::DEEP_FROM;
        let card = self.gpu_on() && !self.given_up && on_card(&scene);
        if card
            && self
                .chaos
                .as_mut()
                .is_some_and(|chaos| chaos.draws(&scene, &full, deep))
        {
            return true;
        }
        view.zoom * zoom <= 1.0 || self.speeds.seconds(&parts(&scene, &plain(&full)), deep) <= pace::LONGEST
    }

    // Renders the view, or leaves a view that would take too long to the picture the app has, magnified: the one
    // on the screen while it covers the view, the whole view's otherwise. `changed`: the picture itself is not
    // the scene's any more.
    fn show_view(&mut self, ctx: &egui::Context, size: [u32; 2], changed: bool) {
        if self.fast(size) {
            self.magnified = false;
            self.start_render(ctx, size);
            return;
        }
        let asked = (self.view, self.adjust.zoom);
        let (picture, zoom) = self.aimed;
        self.magnified = true;
        if !changed && zoom == asked.1 && (picture == WHOLE || pace::covers(asked.0.within(&picture, size))) {
            self.drawn = Some(asked);
            if !self.rendering {
                self.shown_frames = self.shown_frames.max(1);
            }
        } else {
            self.view = WHOLE;
            self.start_render(ctx, size);
            self.view = asked.0;
            self.drawn = Some(asked);
            if !self.rendering {
                self.shown_frames = self.shown_frames.max(1);
            }
        }
        if timed() {
            let (shown, label) = (scale_label(self.aimed.0.zoom), scale_label(asked.0.zoom));
            eprintln!("times: the view at {label}× is the picture of {shown}×, magnified");
        }
    }

    fn start_render(&mut self, ctx: &egui::Context, size: [u32; 2]) {
        self.cancel.store(true, Ordering::Relaxed);
        self.cancel = Arc::new(AtomicBool::new(false));
        self.generation += 1;
        if !self.moved_only {
            self.scene_from = self.generation;
        }
        self.offers.begin(self.generation, !self.moved_only);
        self.requested = Some(size);
        self.drawn = Some((self.view, self.adjust.zoom));
        self.aimed = (self.view, self.adjust.zoom);
        self.rendering = true;
        self.asked = Instant::now();
        self.aimed_size = size;
        self.fit_asked = self.fit(size);
        self.trace.renders += 1;
        self.draft_keys = None;

        let finest = self.full(size);
        let full = plain(&finest);
        let mut passes = vec![(true, fitted(size, DRAFT_SIDE, DRAFT_SAMPLES)), (false, full)];
        // A deep view takes several times longer to draw: a picture in between shortens the wait for a sharp one.
        let between = fitted(size, MIDDLE_SIDE, MIDDLE_SAMPLES);
        if self.view.zoom > 4.0 && full.width.max(full.height) >= 2 * between.width.max(between.height) {
            passes.insert(1, (true, between));
        }
        // A browser page has one thread: a draft first would only make it wait twice.
        if passes[0].1.width >= full.width || cfg!(target_arch = "wasm32") {
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
        // A kind the processor alone draws takes seconds at the window's size: a picture of half the side ends the
        // wait four times sooner, and the whole one takes its place when it is there.
        let slow = scene
            .layers
            .iter()
            .any(|layer| layer.fractal().is_some_and(|shown| heavy(&shown.kind)));
        let sooner = RenderOptions {
            width: full.width / 2,
            height: full.height / 2,
            ..full
        };
        if slow
            && sooner.width.max(sooner.height) as f32 >= SOONER_SIDE
            && passes.last().is_some_and(|(draft, _)| !draft)
        {
            passes.insert(passes.len() - 1, (false, sooner));
        }
        let mut aside = false;
        self.carded = self.begin_chaos(ctx, &scene, &finest);
        // The graphics card's picture is the screen's: the processor renders only what the card does not draw.
        if self.carded {
            self.rendering = !self.chaos.as_ref().is_some_and(Chaos::finished);
            // The first picture of a new scene is the processor's small one: it is there in a frame or two
            // whatever the size of the picture the card draws.
            passes.retain(|(draft, _)| *draft);
            passes.truncate(1);
            if !self.rendering || self.moved_only || passes.is_empty() {
                return;
            }
            aside = true;
        }
        // The shaders are not compiled yet: a draft stands in, and the render begins again when they are.
        let soon = !aside && self.gpu_on() && on_card(&scene) && self.chaos.as_ref().is_some_and(Chaos::pending);
        // A shader asked for here can be there before this frame looks at the card: the render then begins again.
        self.compiling |= soon;
        if soon && chaos_only() {
            return;
        }
        if soon && passes.iter().any(|(draft, _)| *draft) {
            passes.retain(|(draft, _)| *draft);
        }
        if timed() && !aside {
            eprintln!("times: the processor renders {} passes", passes.len());
        }
        let (view, zoom) = (self.view, self.adjust.zoom);
        let sources = !aside && self.gpu_on() && all_flames(&scene);
        let home = !aside && view == WHOLE;
        let deep = view.zoom * zoom >= pace::DEEP_FROM;
        let measured = Some(parts(&scene, &full))
            .filter(|parts| !self.wall.live && parts.windows(2).all(|pair| pair[0].kind == pair[1].kind))
            .and_then(|parts| {
                Some((
                    parts.first()?.kind.to_owned(),
                    parts.iter().map(|part| part.work).sum::<f64>(),
                ))
            });
        let (sender, ctx, kept) = (self.results.0.clone(), ctx.clone(), self.kept.clone());
        let (late, layers) = (self.late.clone(), scene.layers.len().max(1) as f64);
        let work = move || {
            for (draft, options) in passes {
                let whole = options.width >= full.width;
                let started = Instant::now();
                // A layer that will clearly take too long is given up: the picture the app has is magnified instead.
                let begun = Mutex::new((Instant::now(), 0.0));
                let watch = |share: f64| {
                    let mut begun = begun.lock().unwrap_or_else(|e| e.into_inner());
                    if share < begun.1 {
                        begun.0 = Instant::now();
                    }
                    begun.1 = share;
                    let spent = begun.0.elapsed().as_secs_f64();
                    if spent > pace::PROOF && spent > share * pace::LONGEST && !cancel.swap(true, Ordering::Relaxed) {
                        let shown = measured.as_ref().filter(|_| deep);
                        let speed = shown.map(|(kind, work)| (kind.clone(), work / layers * share / spent));
                        *late.lock().unwrap_or_else(|e| e.into_inner()) = Some((generation, speed));
                    }
                };
                let watch: &(dyn Fn(f64) + Sync) = &watch;
                let control = Control {
                    cancel: Some(&cancel),
                    progress: (!draft && !home).then_some(watch),
                };
                #[cfg(not(target_arch = "wasm32"))]
                let result = render_pool().install(|| rendered(&scene, &options, &control, &kept, home, sources));
                #[cfg(target_arch = "wasm32")]
                let result = rendered(&scene, &options, &control, &kept, home, sources);
                let Ok((image, held, fresh)) = result else {
                    if timed() {
                        let spent = started.elapsed().as_secs_f32();
                        eprintln!("times: a render that was dropped stopped after {spent:.3} s");
                    }
                    ctx.request_repaint();
                    return;
                };
                let spent = started.elapsed().as_secs_f64().max(1e-6);
                let speed = measured
                    .as_ref()
                    .filter(|_| fresh && !draft && whole)
                    .map(|(kind, work)| (kind.clone(), deep, work / spent));
                let held = held.map(|(keys, sources)| Held {
                    size,
                    options,
                    keys,
                    sources,
                });
                let radius = image.width.min(image.height) as f32 * 0.03;
                let soft = blur::soften(&image.rgba, image.width as usize, image.height as usize, radius);
                let done = Rendered {
                    generation,
                    draft,
                    aside,
                    view,
                    zoom,
                    held,
                    image,
                    soft,
                    speed,
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

    // Whether a first, small picture of the one awaited would be shown: of a new scene, once while the blur stands.
    // A view that moved and a window that changed size have the picture on the screen for that.
    fn takes_first(&self) -> bool {
        !self.moved_only && self.offers.wants_first()
    }

    fn offer(&mut self, offer: Offer, last: bool) {
        self.offers.offer(offer.generation, offer, last);
    }

    fn present(&mut self, offer: Offer, last: bool, frame: &eframe::Frame) {
        let Some(state) = frame.wgpu_render_state() else {
            return;
        };
        // One time a picture: from the click, the step of the wheel or the edge it answers to the finished
        // picture. A first picture has none.
        if last {
            let scale = self.scale_of(offer.at.view.zoom, offer.at.zoom);
            let took = self.deferred.take().unwrap_or(self.began.elapsed().as_secs_f32());
            self.status = status_of(self.picture.map_or(offer.pixels, |_| offer.at.size), took, scale);
        }
        // A first picture comes in the time the blur does: it is seen under the whole blur only.
        let turned = self.turn_picture(if last { motion::PANEL } else { motion::GIVE });
        self.draft_shown = None;
        if turned {
            self.past_eye = self.shown_eye.take();
        } else {
            self.shown_eye = None;
        }
        self.shown_of = (offer.generation, last);
        if let Some(gfx) = state.renderer.write().callback_resources.get_mut::<Gfx>() {
            gfx.show(&state.device, &offer.flame, &offer.soft, turned);
        }
        (self.shown, self.shown_zoom, self.shown_size) = (offer.at.view, offer.at.zoom, Some(offer.at.size));
        (self.shown_wide, self.shown_whole) = (offer.at.wide, last);
        if let Some(light) = offer.light {
            self.light = light;
        }
        self.live_card = offer.card;
        self.pictures += 1;
    }

    // Another picture takes the screen: the one shown fades into it, unless it is itself still fading in, when
    // the new one takes its place in that fade.
    fn turn_picture(&mut self, seconds: f32) -> bool {
        let now = self.stage.now;
        if self.fresh.moving(now) {
            return false;
        }
        self.past = Some(Picture {
            view: self.shown,
            zoom: self.shown_zoom,
            size: self.shown_size.unwrap_or(self.aimed_size),
            wide: self.shown_wide,
        });
        self.fresh = motion::Motion::at(0.0);
        self.fresh.aim(1.0, now, motion::span(seconds), 1.0);
        self.fading = true;
        true
    }

    fn receive(&mut self, ctx: &egui::Context, frame: &eframe::Frame) {
        if (self.previewing || self.chaotic) && !self.gpu_on() {
            self.previewing = false;
            self.chaotic = false;
            self.requested = None;
        }
        let late = self.late.lock().unwrap_or_else(|e| e.into_inner()).take();
        if let Some((generation, speed)) = late
            && generation == self.generation
        {
            if let Some((kind, rate)) = speed {
                self.speeds.seen(&kind, true, rate);
            }
            if timed() {
                eprintln!("times: the processor would take too long: the render is given up");
            }
            self.too_long = Some(self.aimed);
            self.requested = None;
        }
        while let Ok(done) = self.results.1.try_recv() {
            if done.generation != self.generation {
                continue;
            }
            if done.aside {
                if self.takes_first()
                    && let Some(state) = frame.wgpu_render_state()
                {
                    let image = &done.image;
                    let soft = (done.soft.0.as_slice(), done.soft.1, done.soft.2);
                    let picture = (image.rgba.as_slice(), image.width, image.height);
                    let (flame, soft) = Gfx::upload(&state.device, &state.queue, picture, soft);
                    let offer = Offer {
                        flame,
                        soft,
                        at: Picture {
                            view: done.view,
                            zoom: done.zoom,
                            size: self.aimed_size,
                            wide: [1.0; 2],
                        },
                        light: None,
                        card: false,
                        pixels: [image.width, image.height],
                        generation: done.generation,
                    };
                    self.offer(offer, false);
                    self.trace.stage("draft");
                }
                continue;
            }
            self.previewing = false;
            self.chaotic = false;
            if let Some((kind, deep, rate)) = &done.speed {
                self.speeds.seen(kind, *deep, *rate);
                if timed() {
                    eprintln!("times: the processor made {rate:.3e} a second of {kind}, deep: {deep}");
                }
            }
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
            if (!done.draft || self.takes_first())
                && let Some(state) = frame.wgpu_render_state()
            {
                let image = &done.image;
                self.drawn_with = [image.width, image.height];
                let soft = (done.soft.0.as_slice(), done.soft.1, done.soft.2);
                let picture = (image.rgba.as_slice(), image.width, image.height);
                let (flame, soft) = Gfx::upload(&state.device, &state.queue, picture, soft);
                let offer = Offer {
                    flame,
                    soft,
                    at: Picture {
                        view: done.view,
                        zoom: done.zoom,
                        size: self.aimed_size,
                        wide: [1.0; 2],
                    },
                    light: Some(lightness(&image.rgba, 97)),
                    card: false,
                    pixels: [image.width, image.height],
                    generation: done.generation,
                };
                self.offer(offer, !done.draft);
            }
            self.trace.stage(if done.draft { "draft" } else { "picture" });
            self.last = (!done.draft).then(|| (done.generation, Arc::new(done.image.clone())));
            if !done.draft {
                self.remember(self.requested.unwrap_or([done.image.width, done.image.height]));
            }
            if !done.draft {
                self.rendering = false;
                self.shown_frames = self.shown_frames.max(1);
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
        let drawn = self.gpu_on() && on_card(scene);
        self.swift = false;
        let margin = self.strips_of(scene, full);
        self.strips_asked = margin;
        let Some(chaos) = &mut self.chaos else {
            return false;
        };
        let keys = chaos_keys(scene, full, margin);
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
                    * f64::from(full.samples_per_pixel)
                    * f64::from(full.supersample * full.supersample);
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
        if !chaos.begin(scene, &wanted, margin, &keys, self.asked, limit, ctx) {
            if timed() && !chaos.pending() {
                eprintln!("times: the graphics card does not draw this scene");
            }
            return false;
        }
        if !goes_on {
            self.quick = (None, None);
            (self.widened, self.wide_drawn, self.deferred) = (false, false, None);
            self.chaos_at = (self.view, self.adjust.zoom);
        } else if self.wall.live && self.chaotic {
            self.chaos_at = (self.view, self.adjust.zoom);
            (self.shown, self.shown_zoom) = self.chaos_at;
            self.shown_size = Some(self.aimed_size);
            self.shown_wide = [1.0; 2];
        }
        self.swift = scene.layers.iter().all(field::drawn);
        true
    }

    // What a window of another shape shows beside a picture of a chosen size, in the picture's pixels on each
    // side: a scene the card draws pixel by pixel is drawn with that around it.
    fn strips_of(&self, scene: &Scene, full: &RenderOptions) -> [u32; 2] {
        let pixelwise = scene.sheets.is_none() && !scene.layers.is_empty() && scene.layers.iter().all(field::drawn);
        if self.picture.is_none() || !pixelwise {
            return [0; 2];
        }
        strips([full.width, full.height], self.window)
    }

    // The margins a picture of the scene may be drawn with, as shares of the view's sides, the widest first. A
    // picture made of samples (a flame, an attractor, a deep view) takes long enough for a moved view to wait for
    // it; one drawn pixel by pixel has a draft while it moves, and a wallpaper is one picture.
    fn margins(&self, scene: &Scene) -> Vec<f64> {
        if self.wall.live || sheets::drawn(scene) || scene.layers.iter().all(field::drawn) {
            return Vec::new();
        }
        match self.margin_asked {
            Some(share) => Some(share).into_iter().filter(|share| *share > 0.0).collect(),
            None => pace::margins(self.took).collect(),
        }
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
            [0; 2],
            &[TRIAL.to_owned()],
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
            let own = chaos.keys().is_some_and(|keys| keys == [TRIAL]);
            // The owner asked for a picture: the trial gives way and begins again when the app rests. The run on
            // the card may be that picture's by now, and is left alone.
            if self.rendering || !own {
                if own {
                    chaos.end();
                }
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
                            [0; 2],
                            &[TRIAL.to_owned()],
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
        if let Verdict::Fast(Some(rate)) = self.verdict {
            self.speeds.seen(pace::FLAME, false, rate);
        }
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
            if seconds.is_none() && !self.measured_slow {
                (self.preview, self.chaos, self.held) = (None, None, None);
            } else if !self.gpu_chosen {
                self.gpu = false;
            }
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
        let flames = self.wall.live || on_card(&self.base);
        // Whether the card showed the picture before this one says nothing of it: the draft of the processor
        // that stands in can come after the shaders, and the render would wait for a run that was never begun.
        if self.compiling && !pending && self.rendering && chaos.keys().is_none() && self.gpu_on() && flames {
            self.requested = None;
        }
        self.compiling = pending;
        let mut tried = false;
        if chaos.warming() {
            let size = self.requested.unwrap_or(self.aimed_size);
            let full = self.full(size);
            chaos.warm(ctx, [full.width, full.height]);
        } else {
            self.trial_step(&mut chaos, ctx);
            match turn(chaos.keys(), self.draft_keys.as_deref()) {
                Turn::Trial => tried = true,
                Turn::Draft => self.draft_step(&mut chaos, ctx, frame),
                Turn::Picture => self.chaos_step(&mut chaos, ctx, frame),
                Turn::Nobody => {}
            }
        }
        // A picture on its way whose card has gone silent is begun again, once, and is the processor's the
        // second time: the screen never waits for an answer that does not come.
        if self.rendering && !tried && chaos.stalled() {
            let again = !std::mem::replace(&mut self.stalled, true);
            log::warn!("the graphics card is silent: the picture is begun again (on the card: {again})");
            chaos.end();
            self.given_up |= !again;
            self.requested = None;
        }
        // A picture that is the card's alone while the card has no run: nothing would end the wait.
        if self.rendering && self.carded && chaos.keys().is_none() && !chaos.pending() {
            self.requested = None;
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
                self.export_to(ctx, self.export.size.unwrap_or(size), path);
            }
            self.hand_over(ctx);
            self.tile_step(ctx);
        }
    }

    // An Instant export of the graphics card's picture: it waits for the last of the samples, then the picture
    // is copied back from the card and saved. A picture the card gave up meanwhile is rendered by the processor.
    #[cfg(not(target_arch = "wasm32"))]
    fn hand_over(&mut self, ctx: &egui::Context) {
        let Some((later, size, cut)) = self.instant.take() else {
            return;
        };
        if later.cancelled() {
            return;
        }
        if cut.is_some() && !self.chaotic {
            later.fail();
            return;
        }
        if self.drafting() {
            self.instant = Some((later, size, cut));
            return;
        }
        if cut.is_none() && !self.requested.is_some_and(|shown| self.card_shows(shown)) {
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
            self.instant = Some((later, size, cut));
            return;
        }
        let margin = if self.wide_drawn || !chaos.widened() {
            chaos.margin()
        } else {
            [0; 2]
        };
        let saved = preview.read(false, margin, ctx, move |pixels| match (pixels, cut) {
            (Some((rgba, from)), Some(at)) => later.save(pace::magnified(&rgba, from, at, size), size),
            (Some((rgba, from)), None) if from == size => later.save(rgba, from),
            (Some((rgba, from)), None) => later.shrunk(rgba, from, size),
            (None, _) => later.fail(),
        });
        if let Some(index) = saved {
            chaos.watch(index);
        }
    }

    // Begins an Instant export larger than the picture on the screen, for a scene the card draws pixel by pixel
    // and whose tiles join without a seam: nothing of a tile reaches past it but the look at its neighbours.
    #[cfg(not(target_arch = "wasm32"))]
    fn tiles_suit(&self, scene: &Scene, size: [u32; 2]) -> Option<u32> {
        let tiled = scene.sheets.is_none() && !scene.layers.is_empty() && scene.layers.iter().all(field::drawn);
        // A glow reaches from one tile into the next: a tile is drawn with as much around it as the widest does.
        let shorter = f64::from(size[0].min(size[1]));
        let reach = |layer: &Layer| {
            let tone = layer.tone();
            if tone.glow_strength > 0.0 {
                4.0 * tone.glow_radius * shorter
            } else {
                0.0
            }
        };
        let apron = export::APRON + scene.layers.iter().map(reach).fold(0.0, f64::max).ceil() as u32;
        (tiled && self.gpu_on() && !self.given_up && !self.wall.live && apron <= export::TILE / 4).then_some(apron)
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn begin_tiles(&mut self, ctx: &egui::Context, size: [u32; 2], path: &std::path::Path) -> bool {
        let scene = self.scene_at(size);
        let Some(apron) = self.tiles_suit(&scene, size) else {
            return false;
        };
        let (across, cells) = export::tiles(size, export::TILE);
        if !self.tile_begun(ctx, &scene, size, cells[0], apron) {
            if timed() {
                eprintln!("times: the graphics card does not draw the export in tiles");
            }
            return false;
        }
        let (job, tiles) = export::Job::tiled(size, across, cells.clone(), path.to_owned(), ctx.clone());
        self.export.job = Some(job);
        self.tiling = Some(Tiling {
            tiles,
            scene,
            size,
            cells,
            apron,
            next: 0,
        });
        true
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn tile_begun(&mut self, ctx: &egui::Context, scene: &Scene, size: [u32; 2], cell: [u32; 4], apron: u32) -> bool {
        let options = RenderOptions {
            width: size[0],
            height: size[1],
            samples_per_pixel: FULL_SAMPLES,
            supersample: 1,
        };
        self.chaos
            .as_mut()
            .is_some_and(|chaos| chaos.begin_tile(scene, &options, cell, [apron; 2], ctx))
    }

    // Takes in the tile that is whole: it is drawn, copied back, handed to the file, and the next one begins. A
    // tile the card gives up leaves the whole export to the processor.
    #[cfg(not(target_arch = "wasm32"))]
    fn tile_step(&mut self, ctx: &egui::Context) {
        let Some(mut tiling) = self.tiling.take() else {
            return;
        };
        if tiling.tiles.cancelled() {
            if let Some(chaos) = &mut self.chaos {
                chaos.end_tile();
            }
            return;
        }
        let step = self.chaos.as_mut().map_or(Step::Refused, Chaos::advance_tile);
        if step == Step::Waiting {
            self.tiling = Some(tiling);
            return;
        }
        let mut sent = false;
        if step == Step::Drawn
            && let (Some(chaos), Some(preview)) = (&mut self.chaos, &mut self.preview)
            && let Some((size, levels)) = chaos.tile_levels()
        {
            // The glow is as wide as in the whole picture, whatever the tile's size.
            if preview.draw_tile(&tiling.scene, size, tiling.size, &levels, ctx) {
                let (to, index) = (tiling.tiles.sender(), tiling.next);
                let read = preview.read_tile([tiling.apron; 2], ctx, move |pixels| {
                    if let Some((rgba, _)) = pixels {
                        let _ = to.send((index, rgba));
                    }
                });
                if let Some(read) = read {
                    chaos.watch(read);
                    sent = true;
                }
            }
            chaos.end_tile();
        }
        tiling.next += 1;
        let more = tiling.next < tiling.cells.len();
        if sent && (!more || self.tile_begun(ctx, &tiling.scene, tiling.size, tiling.cells[tiling.next], tiling.apron))
        {
            if more {
                self.tiling = Some(tiling);
            }
            ctx.request_repaint();
            return;
        }
        if timed() {
            eprintln!("times: the graphics card gave a tile of the export up: the processor renders it");
        }
        let options = RenderOptions {
            width: tiling.size[0],
            height: tiling.size[1],
            samples_per_pixel: FULL_SAMPLES,
            supersample: 1,
        };
        let path = tiling.tiles.path();
        let scene = tiling.scene.clone();
        drop(tiling);
        self.export.job = Some(export::Job::start(scene, options, path, ctx.clone(), render_pool()));
    }

    // The card draws a moved view of the scene small while it moves: the whole scene is drawn pixel by pixel, and
    // the picture itself takes the card longer than a moment.
    fn drafts(&self) -> bool {
        self.swift && !self.at_once && !self.draft_off && !self.covering() && !self.given_up && self.gpu_on()
    }

    fn drafting(&self) -> bool {
        self.draft_keys.is_some() && self.chaos.as_ref().and_then(Chaos::keys) == self.draft_keys.as_deref()
    }

    // Begins the draft of the view as it is now: the same view with a margin around it, a few times smaller.
    fn begin_draft(&mut self, ctx: &egui::Context, size: [u32; 2]) -> bool {
        let eye = (self.view, self.adjust.zoom);
        let full = self.full(size);
        let whole = self.scene();
        let around = View {
            zoom: self.view.zoom / pace::DRAFT_MARGIN,
            ..self.view
        };
        let scene = self.scene_in(around, eye.1);
        let [width, height] = self.draft.size([full.width, full.height]);
        let options = RenderOptions {
            width,
            height,
            samples_per_pixel: pace::DRAFT_SAMPLES,
            supersample: 1,
        };
        // What the window shows beside the picture is beside the draft too, in the draft's pixels.
        let beside = self.strips_of(&whole, &full);
        let margin = [(beside[0], width, full.width), (beside[1], height, full.height)].map(|(strip, small, large)| {
            (f64::from(strip) * f64::from(small) / (f64::from(large.max(1)) * pace::DRAFT_MARGIN)).ceil() as u32
        });
        let Some(chaos) = &mut self.chaos else {
            return false;
        };
        let keys = chaos_keys(&scene, &options, margin);
        let begun = chaos.draws(&whole, &full, false)
            && chaos.begin(&scene, &options, margin, &keys, Instant::now(), None, ctx);
        if !begun {
            return false;
        }
        (self.draft_keys, self.draft_of, self.draft_view) = (Some(keys), eye, around);
        self.draft_landed = false;
        self.rendering = true;
        self.carded = false;
        true
    }

    // Takes in the batches of the draft, and puts it on the screen once it is whole: in the place of what was
    // there, with no fade, as the view moves under the hand.
    fn draft_step(&mut self, chaos: &mut Chaos, ctx: &egui::Context, frame: &eframe::Frame) {
        if chaos.advance(ctx) == Step::Refused {
            (self.draft_keys, self.draft_shown, self.draft_off) = (None, None, true);
            return;
        }
        if self.draft_landed || !chaos.finished() {
            return;
        }
        self.draft_landed = true;
        let size = self.requested.unwrap_or(self.aimed_size);
        let full = self.full(size);
        let spent = chaos.spent();
        self.draft
            .took(spent, pace::slice_of(true), full.width.min(full.height));
        let scene = self.scene_in(self.draft_view, self.draft_of.1);
        let (Some(preview), Some((pixels, levels))) = (&mut self.preview, chaos.levels()) else {
            return;
        };
        self.draft_note = (pixels, spent * 1000.0);
        if !preview.draw_draft(&scene, pixels, &levels, ctx) {
            return;
        }
        let margin = chaos.margin();
        let wide = [0, 1]
            .map(|side| f64::from(pixels[side]) / f64::from(pixels[side].saturating_sub(2 * margin[side]).max(1)));
        let Some(state) = frame.wgpu_render_state() else {
            return;
        };
        let mut renderer = state.renderer.write();
        let (Some(gfx), Some((picture, soft))) = (renderer.callback_resources.get_mut::<Gfx>(), preview.draft_views())
        else {
            return;
        };
        // A picture that was still fading in is cut short: the draft's textures may be the ones it fades from.
        if self.fresh.moving(self.stage.now) {
            self.fresh = motion::Motion::at(1.0);
            gfx.forget_past(&state.device);
        }
        gfx.show(&state.device, picture, soft, false);
        (self.shown, self.shown_zoom, self.shown_size) = (self.draft_view, self.draft_of.1, Some(size));
        (self.shown_wide, self.shown_whole) = (wide, false);
        self.shown_eye = None;
        self.live_card = false;
        self.draft_shown = Some(self.draft_of);
        self.pictures += 1;
    }

    fn chaos_step(&mut self, chaos: &mut Chaos, ctx: &egui::Context, frame: &eframe::Frame) {
        let step = chaos.advance(ctx);
        if step == Step::Refused {
            if timed() {
                eprintln!("times: the graphics card gave the scene up {}", chaos.why());
            }
            self.given_up = true;
            self.requested = None;
            return;
        }
        let times = chaos.times();
        let whole = times.1.is_some() && self.quick.1.is_none();
        if times != self.quick {
            if let (Some((size, _)), Some(first)) = (chaos.levels(), times.0)
                && timed()
                && self.quick.0.is_none()
            {
                {
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
                    .map(|(took, steps)| format!("{:.1}/{:.1}", took * 1000.0, *steps as f64 / 1e6))
                    .collect();
                eprintln!("times: all samples from the graphics card after {full:.3} s");
                match chaos.slowest() {
                    Some((slowest, count)) => eprintln!("times: its {count} batches, the longest {slowest:.3} s"),
                    None => {
                        let (count, _, longest, busy) = chaos.load();
                        let past = chaos.overrun() * 1000.0;
                        eprintln!(
                            "times: its {count} units, the longest {longest:.1} ms, the card busy {busy:.0} ms, \
                             the latest end {past:.1} ms past a slice"
                        );
                        eprintln!("times: its batches, ms/million steps: {}", batches.join(" "));
                    }
                }
            }
            self.quick = times;
        }
        if whole {
            // What the view alone costs the card decides the margin of the pictures after it.
            let ((size, _), margin) = (chaos.levels().unwrap_or_default(), chaos.margin());
            let cells = |size: [u32; 2]| size[0].max(1) as f32 * size[1].max(1) as f32;
            let inner = [
                size[0].saturating_sub(2 * margin[0]),
                size[1].saturating_sub(2 * margin[1]),
            ];
            self.took = times.1.map(|seconds| seconds * cells(inner) / cells(size));
            // A view of a kind drawn pixel by pixel is shown as it is drawn only where that takes no time to
            // speak of: a large picture is waited for as every other.
            self.brisk = times.1.is_some_and(|seconds| seconds < BRISK);
            self.at_once = times.1.is_some_and(|seconds| seconds < INSTANT);
        }
        // The picture on the screen still holds the view that moved: the new one is not shown before it has its
        // margin, so the view never stands without one.
        let size = self.requested.unwrap_or(self.aimed_size);
        if whole && self.moved_only && self.kept(size) && self.widen(chaos, ctx, true) {
            self.deferred = Some(self.began.elapsed().as_secs_f32());
        }
        let held = self.deferred.is_some();
        if !held {
            self.chaos_draw(chaos, step == Step::Drawn, whole, ctx, frame);
            if whole && self.chaotic {
                self.card_done();
            }
            self.widen(chaos, ctx, false);
        }
        let wide = chaos.widened() && !self.wide_drawn;
        if wide && timed() {
            let (count, _, longest, busy) = chaos.load();
            eprintln!(
                "times: the picture with a margin of {:?} after {:.3} s more: {count} units, the longest \
                 {longest:.1} ms, the card busy {busy:.0} ms, the latest end {:.1} ms past a slice",
                chaos.margin(),
                chaos.widened_in().unwrap_or(0.0),
                chaos.overrun() * 1000.0
            );
        }
        if held && (wide || !chaos.widening()) {
            // With its margin, or without it where the card gave the margin up.
            self.chaos_draw(chaos, true, true, ctx, frame);
            if self.chaotic {
                self.card_done();
            }
        } else if wide && self.live_card && !self.rendering {
            self.chaos_draw(chaos, true, false, ctx, frame);
        }
    }

    fn card_done(&mut self) {
        self.stalled = false;
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

    // The card draws the finished picture again with a margin around the view, behind the one on the screen:
    // nothing waits for it and nothing shows it until the view moves. It begins once the window rests, or at
    // once (`now`) for a picture that is held back until it has its margin. True when it has begun.
    fn widen(&mut self, chaos: &mut Chaos, ctx: &egui::Context, now: bool) -> bool {
        let rests = (now || (self.chaotic && !self.rendering)) && self.trial.is_none() && !self.frozen && !self.swift;
        let whole = chaos.finished() && chaos.margin() == [0; 2] && !chaos.widening();
        let whole = whole && self.drawn == Some(self.chaos_at);
        let Some(size) = self.requested.filter(|_| rests && whole && !self.widened) else {
            return false;
        };
        let used = self.used.elapsed().as_secs_f32();
        if !now && used < 0.3 {
            ctx.request_repaint_after(std::time::Duration::from_secs_f32(0.3 - used));
            return false;
        }
        self.widened = true;
        let scene = self.scene_in(self.chaos_at.0, self.chaos_at.1);
        let full = self.full(size);
        let px = |share: f64| [full.width, full.height].map(|side| (f64::from(side) * share).round() as u32);
        let mut margins = self.margins(&scene).into_iter().map(px);
        let begun = margins
            .find(|margin| chaos.holds(&scene, &full, *margin))
            .is_some_and(|margin| chaos.widen(&scene, &full, margin, ctx));
        if begun {
            ctx.request_repaint();
        }
        begun
    }

    // Draws the picture from the graphics card's levels when they are new or the tone has changed, and has its
    // lightness measured.
    fn chaos_draw(&mut self, chaos: &Chaos, fresh: bool, last: bool, ctx: &egui::Context, frame: &eframe::Frame) {
        if !fresh && !last && !self.chaotic {
            return;
        }
        // While a picture is awaited the screen stands still: of the card's batches only the first is drawn, for
        // the first picture of a new scene, and the last. The others would be seen changing behind the blur.
        // A moved view of a kind the card draws within a frame is shown as it is drawn, and so is a tone that
        // changes once the picture is whole.
        let shown_as_drawn = self.swift && self.moved_only && self.brisk && self.draft_shown.is_none();
        let staged = self.rendering && !shown_as_drawn;
        let first = staged && fresh && !last && self.takes_first();
        if staged && !last && !first {
            if fresh {
                (self.chaotic, self.previewing) = (true, false);
                (self.latest, self.last) = (None, None);
            }
            return;
        }
        let scene = match self.requested.filter(|_| self.wall.live) {
            Some(size) => self.scene_at(size),
            None => self.scene_in(self.chaos_at.0, self.chaos_at.1),
        };
        let look = look_key(&scene);
        if !fresh && !last && look == self.look {
            return;
        }
        let scale = self.scale_of(self.chaos_at.0.zoom, self.chaos_at.1);
        let (Some(preview), Some((size, levels))) = (&mut self.preview, chaos.levels()) else {
            return;
        };
        let margin = chaos.margin();
        let inner = [size[0] - 2 * margin[0], size[1] - 2 * margin[1]];
        self.drawn_with = size;
        let wide = [0, 1].map(|side| f64::from(size[side]) / f64::from(inner[side].max(1)));
        // The screen may show these very textures: it gets a copy of them before they are drawn into.
        let kept = (staged && self.live_card).then(|| preview.snapshot()).flatten();
        // A margin the window shows beside a picture of a chosen size comes in a fade: the picture as it is on the
        // screen fades into the same one with its margin.
        let eased = !staged && self.live_card && self.picture.is_some() && !self.fresh.moving(self.stage.now);
        let eased = eased && chaos.widened() && !self.wide_drawn;
        let before = eased.then(|| preview.snapshot()).flatten();
        // A picture drawn again with a margin keeps the view it had: that is drawn from its own levels.
        let inset = chaos.inset();
        let inset = inset.as_ref().map(|(_, levels)| levels.as_slice());
        if !preview.draw_wide(&scene, size, inner, &levels, inset, ctx) {
            return;
        }
        self.wide_drawn |= chaos.widened();
        let Some(state) = frame.wgpu_render_state() else {
            return;
        };
        let at = Picture {
            view: self.chaos_at.0,
            zoom: self.chaos_at.1,
            size: self.aimed_size,
            wide,
        };
        let mut offer = None;
        {
            let mut renderer = state.renderer.write();
            let (Some(gfx), Some((picture, soft))) = (renderer.callback_resources.get_mut::<Gfx>(), preview.views())
            else {
                return;
            };
            if let Some((flame, soft)) = &kept {
                gfx.show(&state.device, flame, soft, false);
                self.live_card = false;
            }
            if first {
                offer = preview.snapshot().map(|(flame, soft)| (flame, soft, false));
            } else if !self.live_card {
                offer = Some((picture.clone(), soft.clone(), true));
            } else {
                if let Some((flame, soft)) = &before {
                    gfx.show(&state.device, flame, soft, false);
                    self.past = Some(Picture {
                        view: self.shown,
                        zoom: self.shown_zoom,
                        size: self.shown_size.unwrap_or(self.aimed_size),
                        wide: self.shown_wide,
                    });
                    self.past_eye = None;
                    self.fresh = motion::Motion::at(0.0);
                    self.fresh.aim(1.0, self.stage.now, motion::span(motion::PANEL), 1.0);
                    self.fading = true;
                }
                gfx.show(&state.device, picture, soft, before.is_some());
                (self.shown, self.shown_zoom) = self.chaos_at;
                self.shown_size = Some(self.aimed_size);
                (self.shown_wide, self.shown_whole) = (wide, last || !staged);
                self.shown_of = (self.generation, true);
                self.draft_shown = None;
                self.pictures += 1;
                if last {
                    let took = self.deferred.take().unwrap_or(self.began.elapsed().as_secs_f32());
                    self.status = status_of(self.picture.map_or(inner, |_| self.aimed_size), took, scale);
                }
            }
        }
        self.trace.stage("card");
        if !first {
            let (measured, wake) = (self.measured.clone(), ctx.clone());
            let read = preview.read(true, margin, ctx, move |pixels| {
                if let Some((rgba, _)) = pixels {
                    measured.store(lightness(&rgba, 1).to_bits(), Ordering::Relaxed);
                    wake.request_repaint();
                }
            });
            if let Some(index) = read {
                chaos.watch(index);
            }
        }
        if let Some((flame, soft, card)) = offer {
            let offer = Offer {
                flame,
                soft,
                at,
                light: None,
                card,
                pixels: inner,
                generation: self.generation,
            };
            self.offer(offer, !first);
        }
        self.chaotic = true;
        self.previewing = false;
        self.look = look;
        (self.latest, self.last) = (None, None);
    }

    fn detail(&self) -> String {
        // Before the first tenth of a wait nothing counts: a picture that is there by then shows its time alone.
        let counted = pace::counted(self.began.elapsed().as_secs_f32()).unwrap_or(0.0);
        let waiting = self.counting && self.waited.elapsed().as_secs_f32() >= 0.1;
        if let Some(seconds) = waiting.then_some(counted) {
            let size = self.requested.unwrap_or(self.aimed_size);
            let full = self.full(size);
            status_of(self.picture.unwrap_or([full.width, full.height]), seconds, self.scale())
        } else if self.magnified && self.view.zoom > 1.0 {
            format!("{}  ·  {}×", self.t("magnified"), scale_label(self.scale()))
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
        let view = self.view.scaled(self.adjust.zoom);
        let session = serde_json::json!({
            "scene": serde_json::to_value(&self.base).unwrap_or_default(),
            "title": self.title,
            "random": self.is_random,
            "seed": self.seed,
            "generator": self.generator.number(),
            "fractals": self.fractals,
            "kind": self.kind,
            "adjust": [self.adjust.brightness, self.adjust.gamma, self.adjust.glow],
            "view": [view.right, view.down, view.zoom],
            "layer": self.layer,
            "colors": self.choices.saved(self.base.layers.len()),
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
        let (adjust, view) = saved_look(numbers("adjust").as_deref(), numbers("view").as_deref());
        if let Some(adjust) = adjust {
            self.adjust = adjust;
        }
        if let Some(view) = view {
            self.view = view;
        }
        self.layer = session.get("layer").and_then(serde_json::Value::as_u64).unwrap_or(0) as usize;
        self.choices = colors::Choices::restored(&session, self.base.layers.len());
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

    fn preset_sheet(&mut self, ui: &mut egui::Ui, room: f32, width: f32) {
        ui.set_width(width);
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
        widgets::heading(ui, &self.t(if adding { "layers" } else { "presets" }));
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
                    if widgets::row(ui, &self.display(name), current)
                        && let Some(scene) = presets::get(name).or_else(|| fractal::preset(name))
                    {
                        chosen = Some((named(scene, &self.display(name)), name.clone()));
                    }
                }
            });
        let Some((scene, title)) = chosen else { return };
        self.presets_open = false;
        if adding {
            let name = self.display(&title);
            if let Some(mut layer) = scene.layers.into_iter().next()
                && self.base.layers.len() < MAX_LAYERS
            {
                layer.blend = Blend::Screen;
                layer.name = name;
                self.base.layers.push(layer);
                self.layer = self.base.layers.len() - 1;
                self.choices.choose(self.layer, None);
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
            let title = format!("{} {}", self.kind, self.seed);
            if let Some(scene) = fractal::random_scene(&self.kind, self.seed) {
                return (named(scene, &self.display(&title)), title);
            }
        }
        let title = self.next_random();
        (
            named(random_scene_of(self.generator, self.seed), &self.display(&title)),
            title,
        )
    }

    fn kind_sheet(&mut self, ui: &mut egui::Ui, room: f32) {
        widgets::heading(ui, &self.t("fractal_kind"));
        let mut bar = egui::style::ScrollStyle::floating();
        bar.bar_width = 5.0;
        ui.spacing_mut().scroll = bar;
        let mut chosen = None;
        let hint = self.t("heavy_kind");
        // A phone is not offered the kinds that take its processor long, here as in the styles.
        let kinds: Vec<_> = fractal::all()
            .filter(|spec| !self.phone.on || !heavy(spec.name))
            .collect();
        let tall = list_height(kinds.len(), room);
        egui::ScrollArea::vertical()
            .max_height(tall)
            .min_scrolled_height(tall)
            .show(ui, |ui| {
                for spec in kinds {
                    let mark = heavy(spec.name).then_some(hint.as_str());
                    if widgets::marked_row(ui, &self.kind_name(spec.name), self.kind == spec.name, mark) {
                        chosen = Some(spec.name);
                    }
                }
            });
        if let Some(kind) = chosen {
            self.presets_open = false;
            if kind != self.kind {
                self.show_kind(kind);
            }
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
        widgets::heading(ui, &self.t("recent"));
        let mut bar = egui::style::ScrollStyle::floating();
        bar.bar_width = 5.0;
        ui.spacing_mut().scroll = bar;
        let mut chosen = None;
        let mine = self.recent_here();
        let current = self.phone.at[self.tab()];
        let (ctx, now) = (ui.ctx().clone(), phone::now());
        let side = widgets::PICTURE * ctx.pixels_per_point();
        let pixels = [side.x as usize, side.y as usize];
        let ratio = f64::from(widgets::PICTURE.y / widgets::PICTURE.x);
        let mut budget = 2;
        let each = widgets::PICTURE.y + 10.0;
        let tall = (mine.len() as f32 * each - 2.0).clamp(each - 2.0, (room - 110.0).max(2.0 * each));
        egui::ScrollArea::vertical()
            .max_height(tall)
            .min_scrolled_height(tall)
            .show(ui, |ui| {
                for (index, piece) in mine.iter().enumerate() {
                    let key = format!("r {}", piece.key());
                    let picture = self.thumb(&ctx, key, pixels, 40, &mut budget, || Some(piece.scene(ratio)));
                    let made = piece.origin.made.map(|made| phone::clock(made, now));
                    let label = self.label(&piece.title, piece.origin);
                    if widgets::picture_row(ui, &label, made.as_deref(), picture, index == current) {
                        chosen = Some(piece.clone());
                    }
                }
            });
        if let Some(piece) = chosen {
            self.presets_open = false;
            self.show_piece(&piece);
        }
    }

    fn size_sheet(&mut self, ui: &mut egui::Ui, window: [u32; 2], room: f32, width: f32) {
        ui.set_width(width);
        ui.spacing_mut().item_spacing = egui::vec2(8.0, 2.0);
        widgets::heading(ui, &self.t("size"));
        let own = format!("{} · {} × {}", self.t("size_window"), window[0], window[1]);
        let height = (room - 230.0).clamp(34.0, 420.0);
        let locked = self.lock();
        #[cfg(feature = "store")]
        let before = self.picture;
        let custom = self.pro_missing();
        let locks = |size: Option<[u32; 2]>| size.is_some_and(|size| locked(Some(size), size));
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
                && locked(Some(size), size)
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

    // Whether the size chosen in a list needs the purchase; `None` is the list's first row.
    fn lock(&self) -> impl Fn(Option<[u32; 2]>, [u32; 2]) -> bool + use<> {
        #[cfg(feature = "store")]
        let locks = self.pro_missing();
        #[cfg(feature = "store")]
        return move |chosen, _| locks && typed(chosen);
        #[cfg(not(feature = "store"))]
        |_, _| false
    }

    // The free saves left for today, for whoever has not bought.
    #[cfg(feature = "store")]
    fn saves_left(&self) -> Option<u32> {
        self.pro_missing().then(|| self.saves.left(&store::today()))
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
        widgets::title(ui, &self.t("pro_title"), 17.0);
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
        if state == State::Owned {
            return;
        }
        ui.add_space(14.0);
        let full = ui.available_width();
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

    #[cfg(feature = "store")]
    fn leave_pro(&mut self) {
        let owned = !self.pro_missing();
        if let Some(Wanted::Export { .. } | Wanted::ExportSize(_)) = self.pro.take()
            && !owned
        {
            self.export.open = true;
        }
    }

    // A click beside an open list closes it and goes no further: this lies over everything else and takes it.
    fn list_backdrop(&mut self, ctx: &egui::Context, screen: egui::Rect) {
        let behind = egui::Area::new(egui::Id::unique("list_backdrop"))
            .fixed_pos(screen.min)
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                ui.allocate_response(screen.size(), egui::Sense::click_and_drag())
            });
        if behind.inner.clicked() {
            self.languages_open = false;
            self.sizes_open = false;
            self.presets_open = false;
        }
    }

    // What the system's Back of a phone leaves, one thing at a press: a list, a sheet, the styles or the controls.
    fn set_language(&mut self, ctx: &egui::Context, lang: Lang) {
        let before = std::mem::replace(&mut self.texts, Texts::new(lang));
        widgets::install_fonts(ctx, lang);
        let mut titles = vec![self.title.clone()];
        titles.extend(
            self.phone
                .recent
                .iter()
                .chain(&self.phone.kept)
                .map(|piece| piece.title.clone()),
        );
        for layer in &mut self.base.layers {
            if let Some(name) = renamed(&layer.name, &before, &self.texts, &titles) {
                layer.name = name;
            }
        }
    }

    fn leave(&mut self) {
        if self.languages_open || self.sizes_open || self.presets_open {
            self.languages_open = false;
            self.sizes_open = false;
            self.presets_open = false;
            return;
        }
        #[cfg(feature = "store")]
        if self.pro.is_some() {
            self.leave_pro();
            return;
        }
        #[cfg(not(target_arch = "wasm32"))]
        if self.export.open && self.export.job.is_none() {
            self.export.open = false;
            return;
        }
        self.phone.sheet = false;
        self.phone.tune = false;
    }

    fn sheet(id: &'static str, at: egui::Pos2, pivot: egui::Align2, live: bool) -> egui::Area {
        egui::Area::new(egui::Id::unique(id))
            .fixed_pos(at)
            .pivot(pivot)
            .order(egui::Order::Foreground)
            .interactable(live)
            .fade_in(false)
    }

    pub(crate) fn seat(&mut self, key: &'static str, rect: egui::Rect) {
        match self.seats.iter_mut().find(|(name, _)| *name == key) {
            Some(seat) => seat.1 = rect,
            None => self.seats.push((key, rect)),
        }
    }

    fn seat_of(&self, key: &'static str) -> Option<egui::Rect> {
        self.seats.iter().find(|(name, _)| *name == key).map(|(_, seat)| *seat)
    }

    // Whether a pane gives way: a sheet that is open reaches the place the pane was last seen at.
    pub(crate) fn gives(&self, key: &'static str) -> bool {
        self.seats
            .iter()
            .find(|(name, _)| *name == key)
            .is_some_and(|(_, rect)| self.gives_at(key, *rect))
    }

    // It gives way to a sheet that is open, and stays away from one on its way out until that has mostly gone.
    pub(crate) fn gives_at(&self, key: &'static str, rect: egui::Rect) -> bool {
        let given = self.stage.aim_of(key) == Some(0.0);
        let open: Vec<egui::Rect> = (self.covers.iter())
            .filter(|(_, _, live)| *live)
            .map(|(sheet, ..)| *sheet)
            .collect();
        let leaving = |(sheet, shown, live): &(egui::Rect, f32, bool)| !live && *shown >= 0.5 && sheet.intersects(rect);
        motion::gives_way(rect, &open, widgets::OUTER, given) || self.covers.iter().any(leaving)
    }

    // A sheet's texts fade with it, and what a sheet that grows has not reached yet stays unseen.
    fn unfold(&mut self, ui: &mut egui::Ui, leaf: &Leaf) {
        ui.set_opacity(leaf.shown);
        self.leaf_top = ui.max_rect().top();
        if let Some(tall) = self.stage.peek(leaf.height) {
            let mut clip = ui.clip_rect();
            clip.max.y = clip.max.y.min(self.leaf_top + tall);
            ui.set_clip_rect(clip);
        }
    }

    // A sheet whose content changes height grows or shrinks to it around the point it hangs from; egui would
    // jump there, a frame late.
    fn leaf(
        &mut self,
        ctx: &egui::Context,
        panes: &mut phone::Panes,
        response: &egui::Response,
        leaf: &Leaf,
        radius: f32,
        dark: bool,
    ) {
        let size = response.rect.size();
        let stands = egui::pos2(response.rect.left(), leaf.at.y - leaf.pivot.y().to_factor() * size.y);
        self.covering
            .push((egui::Rect::from_min_size(stands, size), leaf.shown, leaf.live));
        let span = if leaf.shown == 0.0 { 0.0 } else { motion::SMALL };
        let tall = self.stage.glide(leaf.height, response.rect.height(), span);
        let top = leaf.at.y - leaf.pivot.y().to_factor() * tall + (1.0 - leaf.shown) * motion::SHIFT;
        let by = egui::vec2(0.0, top - self.leaf_top);
        ctx.set_transform_layer(response.layer_id, egui::emath::TSTransform::from_translation(by));
        let from = panes.len();
        let rect = egui::Rect::from_min_size(
            egui::pos2(response.rect.left(), top),
            egui::vec2(response.rect.width(), tall),
        );
        panes.push((rect, radius, dark));
        self.faded(panes, from, leaf.shown);
    }

    // A pane that comes and goes: its texts and its glass rise and fade as one.
    fn pane(
        &mut self,
        ctx: &egui::Context,
        panes: &mut phone::Panes,
        response: &egui::Response,
        radius: f32,
        dark: bool,
        shown: f32,
    ) {
        let by = egui::vec2(0.0, (1.0 - shown) * motion::SHIFT);
        ctx.set_transform_layer(response.layer_id, egui::emath::TSTransform::from_translation(by));
        let from = panes.len();
        panes.push((response.rect.translate(by), radius, dark));
        self.faded(panes, from, shown);
    }

    pub(crate) fn faded(&mut self, panes: &phone::Panes, from: usize, shown: f32) {
        self.fades.resize(from, 1.0);
        self.fades.resize(panes.len(), shown);
    }

    // Whether a picture is on its way that the screen does not show yet: one asked for, one being drawn, or one
    // that will be drawn when the view or the window has rested. A view that is only magnified has none coming.
    fn awaited(&mut self, size: [u32; 2]) -> bool {
        if self.resizing {
            return true;
        }
        // A finished picture whose tone the card follows by itself is not waited for: what the screen shows is
        // already the picture of the control that moves.
        let followed = self.latest.is_some() && self.previews(size);
        if self.requested != Some(size) {
            return !(followed || self.card_shows(size));
        }
        // The card draws a moved view of these within a frame or two.
        if (self.previewing && self.latest.is_some()) || (self.swift && self.moved_only && self.brisk) {
            return false;
        }
        if self.kept(size) {
            return false;
        }
        if self.rendering {
            return true;
        }
        let asked = (self.view, self.adjust.zoom);
        if self.drawn == Some(asked) || self.frozen {
            return false;
        }
        let (picture, zoom) = self.aimed;
        self.fast(size) || zoom != asked.1 || !(picture == WHOLE || pace::covers(asked.0.within(&picture, size)))
    }

    // The share of the view the picture on the screen does not cover: what is shown there is not the picture.
    fn beside(&self, size: [u32; 2]) -> f32 {
        let (view, zoom) = self.shown_eye.unwrap_or((self.view, self.adjust.zoom));
        let (shown, of) = (self.shown.scaled(self.shown_zoom), self.shown_size.unwrap_or(size));
        pace::beside(pace::widened(view.scaled(zoom).within(&shown, of), self.shown_wide))
    }

    // The finished picture on the screen holds all of the view as it is now, as sharp as it was drawn: the view
    // moved or shows more of the scene, inside the margin the picture was drawn with. Nothing is waited for then:
    // the picture of the new view takes its place when it is there.
    fn kept(&self, size: [u32; 2]) -> bool {
        if !self.chaotic || !self.shown_whole || self.shown_wide == [1.0; 2] || self.shown_size != Some(size) {
            return false;
        }
        // A scene drawn pixel by pixel has a draft while it moves.
        if self.swift {
            return false;
        }
        if self.shown_eye.is_some() || !(self.moved_only || !self.rendering) {
            return false;
        }
        let shown = self.shown.scaled(self.shown_zoom);
        let at = self.view.scaled(self.adjust.zoom).within(&shown, size);
        at[2] <= 1.001 && pace::covers(pace::widened(at, self.shown_wide))
    }

    // What `preview_now` asks before it draws: the layers on the card are this scene's.
    fn previews(&self, size: [u32; 2]) -> bool {
        if !self.gpu_on() || self.wall.live {
            return false;
        }
        let Some(held) = self.held.as_ref().filter(|held| held.size == size) else {
            return false;
        };
        let scene = self.scene_in(self.shown, self.shown_zoom);
        let prefix = size_key(&held.options);
        all_flames(&scene)
            && (0..scene.layers.len())
                .map(|index| source_key(&scene, index, &prefix))
                .eq(held.keys.iter().cloned())
    }

    // The picture as this frame shows it, decided when the frame's clicks have been taken in: the blur over it
    // begins in the frame that asks for a new picture.
    fn picture_now(
        &mut self,
        ctx: &egui::Context,
        frame: &eframe::Frame,
        size: [u32; 2],
        screen: egui::Rect,
    ) -> glass::Shown {
        let now = self.stage.now;
        // The time shown with a picture runs from the last thing that asked for it.
        let eye = (self.view, self.adjust.zoom);
        if self.requested.is_none() || eye != self.last_eye || size != self.last_size {
            self.began = Instant::now();
            self.begun = self.requested.is_none();
        }
        self.last_size = size;
        let awaited = self.awaited(size);
        // The picture of another scene has no place in the new one's view: it stays where it was last seen.
        let another = self.requested.is_none() || self.shown_of.0 < self.scene_from;
        if awaited && another && self.shown_eye.is_none() {
            self.shown_eye = Some(self.last_eye);
        }
        let offered = self.offers.waiting();
        let fading = self.fresh.moving(now);
        let turn = motion::turn(self.veil_level, awaited, fading, offered[0], offered[1]);
        if let Some(offer) = self.offers.take(turn) {
            self.handing = turn == motion::Turn::First;
            self.present(offer, !self.handing, frame);
        }
        if self.veil_level <= 0.0 && !awaited {
            self.offers.rest();
            self.handing = false;
        }
        // The blur stands until the finished picture has taken the screen, and while a first one fades in.
        let standing = awaited || self.offers.waiting()[1] || (self.handing && self.fresh.moving(now));
        let held = ctx.input(|input| input.pointer.any_down());
        let (veil, moving) = self.veil.step(standing, held, now);
        // While the picture is on its way the time counts, a tenth of a second at a step.
        if standing && !self.counting {
            self.waited = self.began;
        }
        self.counting = standing;
        if standing {
            let passed = self.began.elapsed().as_secs_f32();
            let next = (passed * 10.0).floor() / 10.0 + 0.1 - passed;
            ctx.request_repaint_after(std::time::Duration::from_secs_f32(next.max(0.005)));
        }
        self.veil_level = veil;
        let fresh = self.fresh.value(now);
        if moving || self.fresh.moving(now) {
            ctx.request_repaint();
        } else if std::mem::take(&mut self.fading)
            && let Some(state) = frame.wgpu_render_state()
            && let Some(gfx) = state.renderer.write().callback_resources.get_mut::<Gfx>()
        {
            gfx.forget_past(&state.device);
        }
        self.veiled = veil > 0.0;
        if self.trace.on() {
            let (batches, last, worst, spent) = self.chaos.as_ref().map_or((0, 0.0, 0.0, 0.0), Chaos::load);
            self.trace.note = format!(
                "veil {veil:.3} fresh {fresh:.3} awaited {} picture {} of {}{} asked {} rendering {} first {} view {:.5}:{:.5}:{:.5} \
                 draft {}x{} {:.2} beside {:.4} scale {:.4}..{:.4} knob {:.4} card {batches} {last:.2} {worst:.2} {spent:.1} slice {:.2} parts {:.1} {:.1} {:.1} detail [{}] seats {} covers {}",
                u8::from(awaited),
                self.pictures,
                self.shown_of.0,
                if self.shown_of.1 { "" } else { "d" },
                self.generation,
                u8::from(self.rendering),
                u8::from(self.handing),
                self.view.zoom,
                self.view.right,
                self.view.down,
                self.draft_note.0[0],
                self.draft_note.0[1],
                self.draft_note.1,
                self.beside(size),
                self.scale_note.0,
                self.scale_note.1,
                self.scale_note.2,
                pace::slice() * 1000.0,
                self.trace.parts[0],
                self.trace.parts[1],
                self.trace.parts[2],
                self.detail(),
                self.seats
                    .iter()
                    .map(|(key, seat)| {
                        let gave = u8::from(self.stage.aim_of(key) == Some(0.0));
                        format!(
                            "{key}:{gave}:{:.0},{:.0},{:.0},{:.0}",
                            seat.min.x, seat.min.y, seat.max.x, seat.max.y
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("|"),
                self.covers
                    .iter()
                    .filter(|(_, _, live)| *live)
                    .map(|(sheet, ..)| format!(
                        "{:.0},{:.0},{:.0},{:.0}",
                        sheet.min.x, sheet.min.y, sheet.max.x, sheet.max.y
                    ))
                    .collect::<Vec<_>>()
                    .join("|")
            );
        }
        let fixed = self.picture.is_some();
        let now_eye = (self.view, self.adjust.zoom);
        self.last_eye = now_eye;
        // A wallpaper of a chosen size fills the window; a picture of another kind after it glides back to its
        // frame.
        let stretch = (size[0] as f32 / size[1].max(1) as f32) / (screen.width() / screen.height()).max(0.001);
        let fills = if fixed && self.covering() { 1.0 } else { 0.0 };
        let fill = 1.0 + (filled(stretch).0 - 1.0) * self.stage.glide("fill", fills, motion::PANEL);
        let card = self.gpu_on();
        let lay = |shown: &Picture, eye: Option<(View, f64)>| {
            let [across, along, larger, stretch] =
                lain(eye.unwrap_or(now_eye), shown, card, size, fixed, screen.size());
            [across, along, larger * fill, stretch]
        };
        let shown = Picture {
            view: self.shown,
            zoom: self.shown_zoom,
            size: self.shown_size.unwrap_or(size),
            wide: self.shown_wide,
        };
        let mut laid = glass::Shown {
            view: lay(&shown, self.shown_eye),
            past: lay(&self.past.unwrap_or(shown), self.past_eye.or(self.shown_eye)),
            fresh,
            veil: veil * self.trace.veil.unwrap_or(motion::VEIL),
        };
        if let Some(leap) = &mut self.leap {
            leap.aim(self.lay_was, laid.view, self.leap_from, screen);
            laid.view = leap.lay(laid.view, now);
            laid.past = leap.lay(laid.past, now);
        }
        self.lay_was = laid.view;
        if self.trace.on() {
            use std::fmt::Write as _;
            let four = |lay: [f32; 4]| format!("{:.4},{:.4},{:.4},{:.4}", lay[0], lay[1], lay[2], lay[3]);
            let _ = write!(
                self.trace.note,
                " lay {} past {} px {}x{} scale {:.2} window {} bare {} wide {:.3} strips {}x{} drawn {}x{} held {:.5} stand {:.4}",
                four(laid.view),
                four(laid.past),
                size[0],
                size[1],
                ctx.pixels_per_point(),
                self.window_note(ctx),
                u8::from(self.bare),
                self.shown_wide[0],
                self.strips_asked[0],
                self.strips_asked[1],
                self.drawn_with[0],
                self.drawn_with[1],
                self.scale_of(shown.view.zoom, shown.zoom),
                stand_in(self.shown_eye.unwrap_or(now_eye), &shown, card)[2],
            );
        }
        laid
    }

    #[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
    fn window_note(&self, ctx: &egui::Context) -> String {
        let (maximized, full) = self.window_state(ctx);
        let at = self.window_origin(ctx);
        format!("{}{}:{:.0}:{:.0}", u8::from(maximized), u8::from(full), at.x, at.y)
    }

    #[cfg(any(target_os = "android", target_os = "ios", target_arch = "wasm32"))]
    fn window_note(&self, _ctx: &egui::Context) -> String {
        String::new()
    }

    // Where the window's picture begins on its screen, in points.
    #[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
    fn window_origin(&self, ctx: &egui::Context) -> egui::Pos2 {
        if let Some(sham) = &self.trace.sham {
            return sham.origin(ctx.viewport_rect().size());
        }
        ctx.input(|input| input.viewport().inner_rect.map_or(egui::Pos2::ZERO, |rect| rect.min))
    }

    fn glaze(
        &mut self,
        glass: &mut glass::Frame<'_>,
        panes: phone::Panes,
        ctx: &egui::Context,
        frame: &eframe::Frame,
        size: [u32; 2],
        screen: egui::Rect,
    ) {
        let shown = self.picture_now(ctx, frame, size, screen);
        self.fades.resize(panes.len(), 1.0);
        for ((rect, radius, dark), opacity) in panes.iter().zip(&self.fades) {
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
            glass.pane(*rect, optics, *opacity);
        }
        glass.finish(shown);
        self.panes = panes;
    }

    // Where the button that resets the view stands: above the toolbar, and at the window's foot without the
    // interface. It glides from the one to the other.
    pub(crate) fn reset_at(&mut self, home: Option<egui::Pos2>, content: egui::Rect) -> egui::Pos2 {
        let foot = egui::pos2(content.center().x, content.bottom() - 20.0);
        if home.is_some() {
            self.reset_home = home;
        }
        let home = self.reset_home.unwrap_or(foot);
        let up = self
            .stage
            .glide("reset_up", if self.bare { 0.0 } else { 1.0 }, motion::PANEL);
        foot + (home - foot) * up
    }

    pub(crate) fn reset_button(&mut self, ctx: &egui::Context, at: egui::Pos2, shown: f32, live: bool) -> egui::Rect {
        let area = egui::Area::new(egui::Id::unique("reset_view"))
            .fixed_pos(at)
            .pivot(egui::Align2::CENTER_BOTTOM)
            .constrain(false)
            .interactable(live)
            .fade_in(false)
            .show(ctx, |ui| {
                ui.set_opacity(shown);
                let ink = egui::Color32::WHITE;
                let galley = ui
                    .painter()
                    .layout_no_wrap(self.t("reset_view"), font(14.0, MEDIUM), ink);
                let (rect, response) =
                    ui.allocate_exact_size(egui::vec2(galley.size().x + 40.0, 42.0), egui::Sense::click());
                let fill = (22.0 * widgets::lit(ui, &response)) as u8;
                if fill > 0 {
                    ui.painter().rect_filled(
                        rect,
                        egui::CornerRadius::same(21),
                        egui::Color32::from_white_alpha(fill),
                    );
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

    // A wallpaper is one picture, shown larger at most: nothing is beside it to zoom out to.
    fn covering(&self) -> bool {
        self.wall.live || sheets::drawn(&self.base)
    }

    // How far out the view goes, the scale of the controls counted in: a wallpaper not at all, every other
    // picture to a fifth of its framing.
    fn least(&self) -> f64 {
        if self.covering() { 1.0 } else { FLOOR / self.adjust.zoom }
    }

    // How far in: to where the numbers of the kind run out, a flame and an attractor to where a view of them
    // stops being a picture, and no further into a picture that is only shown larger than where one of its
    // pixels covers eight of the screen's.
    fn most(&self) -> f64 {
        if self.covering() {
            return IMAGE_CEILING;
        }
        let of_layer = |layer: &Layer| match layer.fractal() {
            Some(shown) => orbit_deepest(&shown.kind),
            None => Some(self.depth.ceiling),
        };
        let pictured = self
            .base
            .layers
            .iter()
            .filter_map(of_layer)
            .fold(f64::INFINITY, f64::min);
        self.deepest().min(pictured) / self.adjust.zoom
    }

    // Asks for the depth of the flames of a new picture, and takes the answer when it is there.
    fn fathom(&mut self, ctx: &egui::Context) {
        if self.covering() || !self.base.layers.iter().any(|layer| layer.flame().is_some()) {
            return;
        }
        let key: String = (0..self.base.layers.len())
            .map(|index| source_key(&self.base, index, ""))
            .collect();
        if let Some((of, ceiling)) = self.depth.asked.as_ref().and_then(|asked| asked.try_recv().ok()) {
            self.depth.asked = None;
            if of == key {
                self.depth.ceiling = ceiling;
            }
        }
        if key == self.depth.key {
            return;
        }
        self.depth.key = key.clone();
        let (sender, receiver) = mpsc::channel();
        self.depth.asked = Some(receiver);
        let (scene, ctx) = (self.base.clone(), ctx.clone());
        let work = move || {
            let options = RenderOptions {
                width: DEPTH_SIZE[0],
                height: DEPTH_SIZE[1],
                samples_per_pixel: DEPTH_SAMPLES,
                supersample: 1,
            };
            let flames = (0..scene.layers.len()).filter(|index| scene.layers[*index].flame().is_some());
            let deepest = flames
                .filter_map(|index| render_density(&scene, index, &options, &Control::default()).ok())
                .map(|density| flame_deepest(&density.levels(), DEPTH_SIZE.map(|side| side as usize)))
                .fold(FLAME_DEEPEST.0, f64::max);
            let _ = sender.send((key, deepest));
            ctx.request_repaint();
        };
        #[cfg(not(target_arch = "wasm32"))]
        std::thread::spawn(move || render_pool().install(work));
        #[cfg(target_arch = "wasm32")]
        work();
    }

    // The scale of the view as it is shown and set everywhere: the wheel's and the controls' in one number. A
    // wallpaper has the wheel's alone.
    pub(crate) fn scale(&self) -> f64 {
        self.scale_of(self.view.zoom, self.adjust.zoom)
    }

    fn scale_of(&self, view: f64, controls: f64) -> f64 {
        if self.covering() { view } else { view * controls }
    }

    // The range the view's scale has for the picture as it is drawn now: the wheel's, the pinch's and the
    // slider's. Past the deepest view that is drawn, the picture the app has is shown larger, up to eight times;
    // a view is never pulled back out by this.
    fn reach(&mut self, size: [u32; 2]) -> (f64, f64) {
        let (least, most) = (self.least(), self.most());
        if self.covering() {
            return (least, most);
        }
        let here = self.view;
        let at = move |zoom: f64| View { zoom, ..here };
        if self.fast_at(at(most), size) {
            return (least, most);
        }
        // The deepest view that is drawn lies between the whole picture, which always is, and the ceiling.
        let (mut drawn, mut shown) = (1.0 / self.adjust.zoom, most);
        for _ in 0..12 {
            let middle = (drawn * shown).sqrt();
            if self.fast_at(at(middle), size) {
                drawn = middle;
            } else {
                shown = middle;
            }
        }
        let held = (self.aimed.0.zoom.max(1.0) * IMAGE_CEILING).max(self.view.zoom);
        (least, most.min(drawn.max(held)))
    }

    fn zoom_to(&mut self, zoom: f64, size: [u32; 2]) {
        self.zoom_by(zoom / self.view.zoom, [0.0; 2], size);
        if (self.view.zoom / zoom - 1.0).abs() < 1e-12 {
            self.view.zoom = zoom;
        }
    }

    // The slider of the view's scale: the wheel's quantity and the wheel's range. Its ends glide to those of
    // another picture and its knob to a scale set elsewhere, so neither jumps.
    fn scale_slider(&mut self, ui: &mut egui::Ui) {
        let size = self.requested.unwrap_or(self.aimed_size);
        let (least, most) = self.reach(size);
        let unit = self.scale_of(1.0, self.adjust.zoom);
        let (label, before) = (self.t("zoom"), self.scale());
        let shown = format!("{}×", scale_label(before));
        let stage = &mut self.stage;
        let mut glided = |key: &'static str, end: f64| {
            let aim = fractique_core::math::ln(end) as f32;
            let at = stage.glide(key, aim, motion::PANEL);
            if at == aim {
                end
            } else {
                fractique_core::math::exp(f64::from(at))
            }
        };
        let range = (glided("scale_least", least * unit), glided("scale_most", most * unit));
        let (mut scale, mut knob) = (before, 0.0);
        let changed = widgets::slider_as(ui, &label, &mut scale, range, true, &shown, |share, held| {
            knob = stage.glide("scale_knob", share, if held { 0.0 } else { motion::SMALL });
            knob
        });
        self.scale_note = (range.0, range.1, knob);
        if changed && size[0] > 0 && size[1] > 0 {
            let view = self.view;
            self.zoom_to(scale.clamp(least * unit, most * unit) / unit, size);
            if self.view != view {
                self.touched = Instant::now();
            }
        }
    }

    fn zoom_by(&mut self, factor: f64, around: [f64; 2], size: [u32; 2]) {
        let reach = self.reach(size);
        self.view.zoomed(factor, around, size, reach, self.seen);
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
        // A wallpaper of a chosen size fills the window and is cut where it is over: the hand moves it as large
        // as it is shown, and the view goes as far as what is cut.
        let (fill, seen) = match (self.covering(), self.picture) {
            (false, _) => (1.0, None),
            (true, None) => (1.0, Some([1.0; 2])),
            (true, Some(_)) => {
                let (fill, shares) = filled(stretch);
                (fill, Some(shares))
            }
        };
        self.seen = seen;
        let before = self.view;
        let swiping = self.phone.on && self.wall.live;
        if picture.dragged() && !blocked {
            if swiping {
                self.phone.swipe += picture.drag_delta();
            } else {
                self.view
                    .moved(sides(picture.drag_delta() / screen.size() / fill, size, stretch));
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
        } else if picture.clicked() && (self.languages_open || self.sizes_open || self.presets_open) {
            self.languages_open = false;
            self.sizes_open = false;
            self.presets_open = false;
        } else if picture.clicked() && (self.phone.sheet || self.phone.tune) {
            self.phone.sheet = false;
            self.phone.tune = false;
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
                let around = pointer.map_or(egui::Vec2::ZERO, |at| (at - screen.center()) / screen.size() / fill);
                self.zoom_by(f64::from(factor), sides(around, size, stretch), size);
            }
        }
        if let Some((by, left)) = self.wheeled.take() {
            self.zoom_by(by, [0.0; 2], size);
            self.wheeled = (left > 1).then_some((by, left - 1));
            ui.ctx().request_repaint();
        }
        self.view.zoom = self.view.zoom.min(self.most());
        self.view.limited(size, self.least(), seen);
        if self.view != before {
            self.touched = Instant::now();
        }
    }

    #[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
    fn window_keys(&mut self, ctx: &egui::Context) {
        let typing = ctx.egui_wants_keyboard_input();
        let is_full = self.window_state(ctx).1;
        let (hide, full, leave, back, forward) = ctx.input(|input| {
            let arrow = |key| input.modifiers.alt && input.key_pressed(key) && !typing;
            let side = |button| input.pointer.button_pressed(button);
            (
                input.key_pressed(egui::Key::H) && !typing,
                input.key_pressed(egui::Key::F11),
                input.key_pressed(egui::Key::Escape),
                arrow(egui::Key::ArrowLeft) || side(egui::PointerButton::Extra1),
                arrow(egui::Key::ArrowRight) || side(egui::PointerButton::Extra2),
            )
        });
        if back != forward {
            self.travel(back);
        }
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
        self.probe_full_screen(ctx);
        self.resize_edges(ctx, is_full);
    }

    // The size the system is asked for comes in one step. What is in the window then moves by itself (`stand`),
    // so the system's own animation of a window that is maximized or restored is off until that has ended: two
    // motions at once are read as a jump.
    #[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
    fn leap_asked(&mut self, ctx: &egui::Context) {
        // macOS moves a window to full screen in its own animation, through many sizes.
        if !motion::wanted() || cfg!(target_os = "macos") {
            return;
        }
        self.leaping = Some(ctx.input(|input| input.time));
        self.system_motion(false);
    }

    #[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
    fn system_motion(&self, on: bool) {
        #[cfg(windows)]
        if let Some(handle) = self.handle
            && self.trace.sham.is_none()
            && std::env::var_os("FRACTIQUE_SYSTEM_MOTION").is_none()
        {
            use windows_sys::Win32::Graphics::Dwm::{DWMWA_TRANSITIONS_FORCEDISABLED, DwmSetWindowAttribute};
            let off = i32::from(!on);
            // SAFETY: the handle is the app's own window, and the call reads one BOOL through the pointer.
            unsafe {
                DwmSetWindowAttribute(
                    handle as _,
                    DWMWA_TRANSITIONS_FORCEDISABLED as u32,
                    (&raw const off).cast(),
                    size_of::<i32>() as u32,
                );
            }
        }
        #[cfg(not(windows))]
        let _ = on;
    }

    // The frame the panes are laid out in. After a leap of the window it glides from where the window's frame
    // stood on the screen to the window as it is now; otherwise it is the window.
    #[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
    fn stand(&mut self, ctx: &egui::Context, screen: egui::Rect, now: f64) -> egui::Rect {
        let origin = self.window_origin(ctx);
        let asked = self.leaping.is_some_and(|since| now - since < 0.5);
        if let Some((was, at, stood)) = self.window_was
            && asked
            && was.size() != screen.size()
            && was.width() > 0.0
            && screen.width() > 0.0
        {
            let by = at - origin;
            self.leap = Some(motion::Leap::new(stood.translate(by), now));
            self.leap_from = was.translate(by);
            self.leaping = None;
        } else if self.leaping.is_some() && !asked {
            self.leaping = None;
            self.system_motion(true);
        }
        let stand = match self.leap {
            Some(leap) if leap.moving(now) => {
                ctx.request_repaint();
                leap.frame(screen, now)
            }
            Some(_) => {
                self.leap = None;
                self.landed = true;
                self.system_motion(true);
                screen
            }
            None => screen,
        };
        self.window_was = Some((screen, origin, stand));
        stand
    }

    #[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
    fn window_state(&self, ctx: &egui::Context) -> (bool, bool) {
        if let Some(sham) = &self.trace.sham {
            return (sham.maximized, sham.full);
        }
        ctx.input(|input| {
            (
                input.viewport().maximized.unwrap_or(false),
                input.viewport().fullscreen.unwrap_or(false),
            )
        })
    }

    #[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
    fn set_maximized(&mut self, ctx: &egui::Context, on: bool) {
        self.leap_asked(ctx);
        ctx.send_viewport_cmd(match &mut self.trace.sham {
            Some(sham) => egui::ViewportCommand::InnerSize(sham.maximize(on, ctx.viewport_rect().size())),
            None => egui::ViewportCommand::Maximized(on),
        });
    }

    #[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
    fn set_full(&mut self, ctx: &egui::Context, on: bool) {
        self.leap_asked(ctx);
        ctx.send_viewport_cmd(match &mut self.trace.sham {
            Some(sham) => egui::ViewportCommand::InnerSize(sham.fullscreen(on, ctx.viewport_rect().size())),
            None => egui::ViewportCommand::Fullscreen(on),
        });
    }

    #[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
    fn full_screen(&mut self, ctx: &egui::Context, on: bool) {
        // A maximized window goes to full screen as it is and comes back maximized, in one step each way: restored
        // on the way, it stood at its small size on the screen for a frame or three, with the desktop around it.
        #[cfg(windows)]
        motion::whole::set(on && self.trace.sham.is_none());
        self.set_full(ctx, on);
        if !on && std::mem::take(&mut self.maximize_again) {
            self.set_maximized(ctx, true);
        }
    }

    #[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
    fn toggle_maximized(&mut self, ctx: &egui::Context) {
        let (maximized, is_full) = self.window_state(ctx);
        // Full screen is left to the window as it was before it: maximized, or to be maximized now.
        if is_full {
            self.maximize_again = !maximized;
            self.full_screen(ctx, false);
        } else {
            self.set_maximized(ctx, !maximized);
        }
    }

    #[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
    fn probe_full_screen(&mut self, ctx: &egui::Context) {
        let Some(report) = std::env::var_os("FRACTIQUE_DEBUG_FULL") else {
            return;
        };
        // FRACTIQUE_DEBUG_HOLD: the same window, left alone: what drives it comes from outside.
        if std::env::var_os("FRACTIQUE_DEBUG_HOLD").is_some() {
            return;
        }
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
        let maximized = self.window_state(ctx).0;
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

    // The press of the title bar's eye, read before the panes are laid out: they begin to go in the frame of
    // the release, not in the one after it.
    #[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
    fn eye_pressed(&mut self, ctx: &egui::Context) {
        let released = ctx.input(|input| {
            input
                .pointer
                .primary_released()
                .then(|| input.pointer.interact_pos())
                .flatten()
        });
        let eye = ctx.read_response(egui::Id::unique(("title bar", "hide")));
        if let (Some(at), Some(eye)) = (released, eye)
            && eye.is_pointer_button_down_on()
            && eye.rect.contains(at)
        {
            self.bare = true;
        }
    }

    #[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
    fn title_bar(&mut self, ctx: &egui::Context, screen: egui::Rect, shown: f32) -> egui::Rect {
        let bar = egui::Rect::from_min_max(
            screen.min + egui::vec2(BAR_GAP, BAR_GAP),
            egui::pos2(screen.right() - BAR_GAP, screen.top() + BAR_GAP + BAR),
        );
        let is_full = self.window_state(ctx).1;
        egui::Area::new(egui::Id::unique("title bar"))
            .fixed_pos(bar.min)
            .constrain(false)
            .interactable(!self.bare)
            .fade_in(false)
            .show(ctx, |ui| {
                ui.set_opacity(shown);
                let (rect, grip) = ui.allocate_exact_size(bar.size(), egui::Sense::click_and_drag());
                if grip.double_clicked() {
                    self.toggle_maximized(ctx);
                } else if grip.drag_started_by(egui::PointerButton::Primary) {
                    ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
                    self.let_go = true;
                }
                let painter = ui.painter().clone();
                let name = painter.text(
                    rect.left_center() + egui::vec2(BAR_EDGE, 0.0),
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
                let lift = (22.0 * widgets::lit(ui, &version)) as u8;
                let fill = if newer.is_some() { 225 } else { 30 + lift };
                painter.rect_filled(
                    chip,
                    egui::CornerRadius::same(13),
                    egui::Color32::from_white_alpha(fill),
                );
                painter.galley(chip.center() - galley.size() * 0.5, galley, ink);
                // The App Store updates its own builds, and they may not point at a download outside it.
                if version.clicked() && !cfg!(feature = "store") {
                    self.languages_open = false;
                    self.sizes_open = false;
                    self.presets_open = false;
                    self.export.open = false;
                    self.updates.show(ctx);
                }

                let ink = egui::Stroke::new(1.4, egui::Color32::from_white_alpha(235));
                let mut slot = egui::Rect::from_center_size(
                    egui::pos2(rect.right() - BAR_EDGE - 15.0, rect.center().y),
                    egui::Vec2::splat(30.0),
                );
                for button in ["close", "maximize", "minimize", "full", "hide"] {
                    if tiled() && matches!(button, "maximize" | "minimize") {
                        continue;
                    }
                    let pressed = ui.interact(slot, egui::Id::unique(("title bar", button)), egui::Sense::click());
                    let lit = if button == "close" {
                        egui::Color32::from_rgb(214, 58, 44)
                    } else {
                        egui::Color32::from_white_alpha(56)
                    };
                    let fill = egui::Color32::from_white_alpha(30).lerp_to_gamma(lit, widgets::lit(ui, &pressed));
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
                            let lid = |up: f32| {
                                (0..=20).map(move |step| {
                                    let t = step as f32 / 10.0 - 1.0;
                                    c + egui::vec2(7.5 * t, -up * 4.8 * (1.0 - t * t))
                                })
                            };
                            let outline: Vec<egui::Pos2> = lid(1.0).chain(lid(-1.0).rev().skip(1)).take(40).collect();
                            painter.add(egui::Shape::closed_line(outline, ink));
                            painter.circle_stroke(c, 2.7, ink);
                            painter.circle_filled(c, 1.1, ink.color);
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
                let lit = language.hovered() || self.languages_open;
                let lift = (26.0 * widgets::glided(ui, language.id.with("lit"), lit)) as u8;
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
        self.begin();
        self.choices = colors::Choices::all(self.choice(), scene.layers.len());
        self.base = scene;
        self.title = title;
        self.is_random = is_random;
        self.wall.live = false;
        self.requested = None;
        self.given_up = false;
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

    fn gpu_turned(&mut self) {
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
        if self.phone.on || cfg!(any(target_os = "android", target_os = "ios", target_arch = "wasm32")) {
            widgets::section(ui, &self.t("language"));
            if widgets::row(ui, self.texts.lang.name(), true) {
                self.languages_open = !self.languages_open;
                self.sizes_open = false;
                self.presets_open = false;
            }
        }

        ui.add_space(widgets::SECTION);
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
            if widgets::dropdown(ui, &self.display(&self.title)) {
                self.open_sheet(Sheet::Presets);
            }
        }
        // Each tab has its own recent pictures: the one before the picture shown, or that picture when it is alone.
        let tab = self.tab();
        let mine: Vec<&phone::Piece> = self.phone.recent.iter().filter(|piece| piece.tab() == tab).collect();
        let at = self.phone.at[tab];
        let earlier = mine.get(at + 1).or(mine.get(at)).or(mine.first());
        if let Some(label) = earlier.map(|piece| self.label(&piece.title, piece.origin)) {
            widgets::section(ui, &self.t("recent"));
            if widgets::dropdown(ui, &label) {
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
            self.scale_slider(ui);

            ui.add_space(8.0);
            if widgets::pill(ui, &self.t("reset"), false, ui.available_width()) {
                self.adjust = NEUTRAL;
                self.reset_view();
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
            let (turned, pressed) = widgets::toggle_warned(ui, &self.t("gpu"), &mut self.gpu);
            if turned {
                self.gpu_chosen = true;
                self.gpu_turned();
            }
            self.gpu_warned ^= pressed;
            widgets::warning(ui, ui.scope_id().with("gpu weak"), &self.t("gpu_weak"), self.gpu_warned);
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
        let title = self.display(&self.title);
        let shades: Vec<palette::Lut> = self
            .base
            .layers
            .iter()
            .enumerate()
            .map(|(index, layer)| {
                let chosen = self.choices.of(index).and_then(|at| palette::NAMES.get(at));
                let worn = chosen.and_then(|name| palette::named(name));
                palette::lut(worn.as_ref().unwrap_or(layer.palette()))
            })
            .collect();
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
                    } else if count == 1
                        && layer
                            .name
                            .rsplit_once(' ')
                            .is_some_and(|(word, seed)| word == title && is_seed(seed))
                    {
                        title.clone()
                    } else {
                        layer.name.clone()
                    };
                    let dot = |painter: &egui::Painter, at: egui::Pos2, reach: f32| {
                        phone::palette_disc(painter, at, reach, &shades[index]);
                    };
                    if index != chosen {
                        if widgets::dot_row(ui, &label, &dot) {
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
                            let enabled = [count > 1, index + 1 < count, index > 0];
                            act = widgets::layer_head(ui, &label, enabled, &dot).act.or(act);
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
                self.choices.remove(at);
                self.layer = at.min(count - 2);
            }
            Some("up") => {
                self.base.layers.swap(at, at - 1);
                self.choices.swap(at, at - 1);
                self.layer = at - 1;
            }
            Some("down") => {
                self.base.layers.swap(at, at + 1);
                self.choices.swap(at, at + 1);
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
        widgets::heading(ui, &self.t(if layouts { "layout" } else { "material" }));
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

    // `wide`: 0 in a narrow window, 1 in a wide one; the buttons grow from one to the other.
    fn toolbar(&mut self, ui: &mut egui::Ui, wide: f32, room: f32) {
        let gap = egui::lerp(6.0..=8.0, wide);
        ui.spacing_mut().item_spacing = egui::vec2(gap, gap);
        ui.spacing_mut().button_padding.x = egui::lerp(12.0..=18.0, wide);
        let keys: &[&str] = if cfg!(target_arch = "wasm32") {
            &["randomize", "mutate", "controls"]
        } else {
            &["randomize", "mutate", "export", "controls"]
        };
        let labels: Vec<String> = keys.iter().map(|key| self.t(key)).collect();
        let natural: f32 = labels.iter().map(|label| widgets::pill_width(ui, label)).sum();
        let arrow = widgets::BACK + gap;
        let one_row = arrow + natural + (labels.len() - 1) as f32 * gap <= room;
        let hint = self.t("back");

        let (mut clicked, mut back) = (None, false);
        let mut row = |ui: &mut egui::Ui, buttons: std::ops::Range<usize>| {
            ui.horizontal(|ui| {
                let first = buttons.start == 0;
                if first {
                    back = widgets::back(ui, &hint);
                }
                let width = match (one_row, first) {
                    (true, _) => 0.0,
                    (false, true) => (room - arrow - gap) * 0.5,
                    (false, false) => (room - gap) * 0.5,
                };
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

        if back {
            self.travel(true);
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
                let settled = self.settle();
                let (size, origin) = (self.requested.unwrap_or([1, 2]), self.phone.origin);
                self.wall.held = Some(wallpaper_choice(self.seed, &self.wall.request(size)));
                self.new_wallpaper();
                if settled {
                    (self.phone.step, self.phone.origin) = (phone::Step::Mutated, origin);
                }
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
                self.base.layers[at].name = self.display(&title);
                if self.base.layers.len() == 1 {
                    self.title = title;
                    self.is_random = true;
                }
                self.view = WHOLE;
                self.requested = None;
                self.begin();
            }
            Some("mutate") => {
                self.settle();
                self.phone.step = phone::Step::Mutated;
                self.seed += 1;
                let at = self.layer.min(self.base.layers.len() - 1);
                let mut alone = self.base.clone();
                alone.layers = vec![self.base.layers[at].clone()];
                if let Some(changed) = mutate(&alone, self.seed, 0.25).layers.pop() {
                    self.base.layers[at].content = changed.content;
                }
                self.view = WHOLE;
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
                log::warn!("the file was not read: {error}");
                self.status = self.t("open_failed");
                return;
            }
        };
        if !is_flame && text.contains("\"fractique\"") {
            self.view = WHOLE;
            self.adjust = NEUTRAL;
            if self.restore(&text).is_some() {
                self.begin();
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
            Err(error) => {
                log::warn!("the file is no scene: {error}");
                self.status = self.t("open_failed");
            }
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
            Err(error) => {
                log::warn!("the scene was not saved: {error}");
                self.t("failed")
            }
        };
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn export_sheet(&mut self, ui: &mut egui::Ui, window: [u32; 2], room: f32, (width, wide): (f32, bool)) {
        ui.set_width(width);
        ui.spacing_mut().item_spacing = egui::vec2(8.0, 2.0);
        widgets::title(ui, &self.t("export_title"), 17.0);

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
            if job.timed() {
                ui.ctx().request_repaint_after(std::time::Duration::from_millis(100));
            }
            let stage = match job.stage() {
                export::Stage::Rendering => "rendering",
                export::Stage::Writing => "writing_file",
            };
            ui.label(format!("{} · {:.0}%", self.t(stage), share * 100.0));
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
        // The first row is the picture's size: a size chosen for the picture is asked for as that size is.
        let picture = self.picture;
        let locks = |size: Option<[u32; 2]>| locked(size.or(picture), shown);
        let list = |height: f32| List {
            height,
            locked: &locks,
            custom,
        };
        let mut picked = Pick::None;
        #[cfg(feature = "store")]
        let size_before = self.export.size;
        let clicked = if wide {
            // A low, wide screen: the sizes at the left, the rest at the right.
            ui.spacing_mut().item_spacing.x = 24.0;
            let mut clicked = false;
            ui.columns(2, |columns| {
                let ui = &mut columns[0];
                ui.spacing_mut().item_spacing = egui::vec2(8.0, 2.0);
                widgets::section(ui, &size);
                let height = (room - 2.0 * widgets::OUTER - 258.0).clamp(34.0, 230.0);
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
                clicked = self.export_choices(&mut columns[1], self.export.size.unwrap_or(shown));
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
            self.export_choices(ui, self.export.size.unwrap_or(shown))
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
            && typed(self.export.size.or(self.picture))
        {
            let wanted = self.export.size;
            self.export.size = size_before;
            self.want(Wanted::ExportSize(wanted));
            return;
        }
        #[cfg(not(feature = "store"))]
        let _ = picked;
        if clicked {
            #[cfg(feature = "store")]
            if self.pro_missing() && (typed(self.export.size.or(self.picture)) || self.saves_left() == Some(0)) {
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
        if self.magnified && self.view != WHOLE {
            self.export_magnified(ctx, [width, height], path);
            return;
        }
        let as_shown = samples == export::AS_SHOWN && (self.shown, self.shown_zoom) == (self.view, self.adjust.zoom);
        let on_card = self.chaos.as_ref().and_then(|chaos| {
            let ((size, _), margin) = (chaos.levels()?, chaos.margin());
            Some([size[0] - 2 * margin[0], size[1] - 2 * margin[1]])
        });
        if timed() {
            let shows = self.requested.is_some_and(|size| self.card_shows(size));
            eprintln!("times: an export of {width} × {height}, as shown {as_shown}, on the card {on_card:?} {shows}");
        }
        // The picture on the screen is the export, as it is or made smaller; a larger one is drawn in tiles.
        if as_shown
            && let Some(screen) = on_card
            && self.requested.is_some_and(|size| self.card_shows(size))
        {
            match export::way(screen, [width, height]) {
                export::Way::Shown | export::Way::Shrunk => {
                    let (job, later) = export::Job::later([width, height], path, ctx.clone());
                    self.export.job = Some(job);
                    self.instant = Some((later, [width, height], None));
                    ctx.request_repaint();
                    return;
                }
                export::Way::Larger if self.begin_tiles(ctx, [width, height], &path) => return,
                _ => {}
            }
        }
        if as_shown
            && !self.rendering
            && let Some((generation, image)) = &self.last
            && *generation == self.generation
        {
            let from = [image.width, image.height];
            match export::way(from, [width, height]) {
                export::Way::Shown => {
                    self.export.job = Some(export::Job::save(image.clone(), path, ctx.clone()));
                    return;
                }
                export::Way::Shrunk => {
                    let image = image.clone();
                    let make = move || {
                        Arc::new(Image {
                            width,
                            height,
                            rgba: pace::shrunk(&image.rgba, from, [width, height]),
                        })
                    };
                    self.export.job = Some(export::Job::made([width, height], make, path, ctx.clone()));
                    return;
                }
                _ => {}
            }
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

    // A view that is a magnified picture on the screen is exported as that: rendering it is what the app spares.
    #[cfg(not(target_arch = "wasm32"))]
    fn export_magnified(&mut self, ctx: &egui::Context, size: [u32; 2], path: PathBuf) {
        let at = self.view.within(&self.shown, self.requested.unwrap_or(size));
        if self.chaotic {
            let (job, later) = export::Job::later(size, path, ctx.clone());
            self.export.job = Some(job);
            self.instant = Some((later, size, Some(at)));
            ctx.request_repaint();
        } else if let Some((generation, image)) = &self.last
            && *generation == self.generation
        {
            let image = image.clone();
            let make = move || {
                Arc::new(Image {
                    width: size[0],
                    height: size[1],
                    rgba: pace::magnified(&image.rgba, [image.width, image.height], at, size),
                })
            };
            self.export.job = Some(export::Job::made(size, make, path, ctx.clone()));
        } else {
            self.export.note = self.t("rendering");
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn export_choices(&mut self, ui: &mut egui::Ui, size: [u32; 2]) -> bool {
        widgets::section(ui, &self.t("quality"));
        ui.add_space(2.0);
        let qualities = export::QUALITIES.map(|(key, _, _)| self.t(key));
        // What the renderer does not take at this size is not offered: the highest quality that it takes is.
        let off = export::QUALITIES.map(|(_, _, supersample)| !export::fits(size, supersample));
        if off[self.export.quality] {
            self.export.quality = off.iter().rposition(|off| !off).unwrap_or(0);
        }
        if let Some(index) = widgets::segments_of(ui, &qualities, self.export.quality, &off) {
            self.export.quality = index;
        }
        #[cfg(feature = "store")]
        if let Some(left) = self.saves_left() {
            let line = if left == 0 {
                self.t("saves_none")
            } else {
                self.t("saves_left").replace("{left}", &left.to_string())
            };
            ui.add_space(8.0);
            ui.label(
                egui::RichText::new(line)
                    .size(12.5)
                    .color(egui::Color32::from_white_alpha(190)),
            );
        }
        if !off.contains(&false) {
            ui.add_space(8.0);
            ui.label(
                egui::RichText::new(self.t("export_no_room"))
                    .size(12.5)
                    .color(egui::Color32::from_white_alpha(190)),
            );
            return false;
        }
        if off.contains(&true) {
            let fitting: Vec<&str> = qualities
                .iter()
                .zip(off)
                .filter(|(_, off)| !off)
                .map(|(name, _)| name.as_str())
                .collect();
            ui.add_space(8.0);
            ui.label(
                egui::RichText::new(format!("{} {}", self.t("q_only_here"), fitting.join(", ")))
                    .size(12.5)
                    .color(egui::Color32::from_white_alpha(190)),
            );
        }
        // An Instant export that is not instant says why, before the press.
        let drawn = self.full(self.requested.unwrap_or(self.aimed_size));
        let instant = export::QUALITIES[self.export.quality].1 == export::AS_SHOWN && !self.magnified;
        let quick = |app: &Self| {
            export::way([drawn.width, drawn.height], size) == export::Way::Larger
                && app.tiles_suit(&app.scene_at(size), size).is_some()
        };
        if instant
            && let Some(key) = export::slow([drawn.width, drawn.height], size)
            && !quick(self)
        {
            ui.add_space(8.0);
            ui.label(
                egui::RichText::new(self.t(key))
                    .size(12.5)
                    .color(egui::Color32::from_white_alpha(190)),
            );
        }
        if self.magnified && self.view != WHOLE {
            ui.add_space(8.0);
            ui.label(
                egui::RichText::new(self.t("export_magnified"))
                    .size(12.5)
                    .color(egui::Color32::from_white_alpha(190)),
            );
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
        let mut clicked = widgets::pill(ui, &save, true, ui.available_width());
        self.export.share = false;
        if cfg!(any(target_os = "android", target_os = "ios")) {
            ui.add_space(8.0);
            if widgets::pill(ui, &self.t("share"), false, ui.available_width()) {
                self.export.share = true;
                clicked = true;
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
        if let Some(job) = &self.export.job {
            let stage = format!("{:?}", job.stage()).to_lowercase();
            if self.trace.cancel.as_deref() == Some(&stage) {
                job.cancel();
            }
            self.trace.export = format!(" export {:.3} {stage}", job.share());
        }
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
                #[cfg(feature = "store")]
                if self.pro_missing() {
                    self.saves.count(&store::today());
                }
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
            export::Outcome::Cancelled => {
                self.trace.export = " export cancelled".to_owned();
                self.export.note.clear();
            }
            export::Outcome::Failed(why) => self.export.note = format!("{}: {}", self.t("failed"), self.t(why.key())),
        }
    }

    fn tuning(&mut self, ui: &mut egui::Ui) {
        ui.set_width(220.0);
        ui.spacing_mut().item_spacing = egui::vec2(8.0, 2.0);
        widgets::title(ui, &self.t("glass_title"), 15.0);
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
            if self.trace.pending() {
                ctx.request_repaint();
            } else {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            return;
        }
        #[cfg(not(target_arch = "wasm32"))]
        if self.export.job.is_some() || self.export_at.is_some() {
            ctx.request_repaint();
            return;
        }
        if self.verdict == Verdict::Unknown {
            // `trial:randomize` presses the button while the card is tried, `tried:randomize` while the processor is.
            let amid = self.trial.as_ref().map(|trial| match trial.card {
                Some(_) => "tried:",
                None => "trial:",
            });
            let next = self.steps.last().zip(amid);
            if let Some(step) = next.and_then(|(step, amid)| step.strip_prefix(amid)).map(str::to_owned) {
                self.steps.pop();
                self.step(ctx, &step);
            }
            ctx.request_repaint();
            return;
        }
        if self.trace.recording() {
            ctx.request_repaint();
            if let Some(step) = self.trace.next(&mut self.steps) {
                self.step(ctx, &step);
            }
            return;
        }
        // FRACTIQUE_THEN: the next view is shown when the picture of this one is whole, or the processor is long
        // at it.
        let tired = self.shown_frames == 0 && self.rendering && self.asked.elapsed().as_secs_f32() > 1.5;
        if self.shown_frames == 0 && !tired {
            if !self.then.is_empty() {
                ctx.request_repaint_after(std::time::Duration::from_millis(300));
            }
            return;
        }
        if (self.shown_frames == 5 || tired)
            && let Some(view) = self.then.pop()
        {
            self.view = view;
            self.touched = Instant::now();
            self.asked = Instant::now();
            self.shown_frames = 0;
            ctx.request_repaint();
            return;
        }
        if tired {
            return;
        }
        // FRACTIQUE_STEPS: the buttons a test run presses, each when the picture of the one before is whole
        // (randomize, mutate, back, forward), and tap:0.5:0.1, a tap at that share of the window's width and
        // height; wait lets a few frames pass after one.
        if self.shown_frames == 5
            && let Some(step) = self.steps.pop()
        {
            self.step(ctx, &step);
            return;
        }
        if self.phone.busy() || self.stage.moving() || self.fresh.moving(self.stage.now) || self.veiled {
            ctx.request_repaint();
            return;
        }
        self.shown_frames += 1;
        ctx.request_repaint();
        if self.shown_frames == 6
            && let Some(path) = self.screenshot.clone()
        {
            // A test run's panes go beside its picture: a script checks that no two of them overlap.
            let panes: String = self
                .panes
                .iter()
                .map(|(pane, radius, _)| {
                    format!("{} {} {} {} {radius}\n", pane.min.x, pane.min.y, pane.max.x, pane.max.y)
                })
                .collect();
            let _ = std::fs::write(path.with_extension("panes"), panes);
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

impl App {
    fn step(&mut self, ctx: &egui::Context, step: &str) {
        ctx.request_repaint();
        let step = ["trial:", "tried:"]
            .iter()
            .find_map(|amid| step.strip_prefix(amid))
            .unwrap_or(step);
        let numbers: Vec<f32> = step.split(':').skip(1).filter_map(|part| part.parse().ok()).collect();
        if self.trace.begin(step) {
            self.shown_frames = 1;
            return;
        }
        if step.starts_with("size:")
            && let [width, height] = numbers[..]
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(width, height)));
            self.shown_frames = 1;
            return;
        }
        if step.starts_with("zoom:")
            && let [by] = numbers[..]
        {
            let before = self.view;
            self.zoom_by(f64::from(by), [0.0; 2], self.requested.unwrap_or(self.aimed_size));
            // A step that ends at a limit changes nothing, and nothing is drawn for it.
            if self.view == before || (self.covering() && !self.rendering) {
                self.shown_frames = 1;
                return;
            }
            self.touched = Instant::now();
            self.asked = Instant::now();
            self.shown_frames = 0;
            return;
        }
        if let Some(place) = step.strip_prefix("view:")
            && let Some(view) = View::asked(place)
        {
            self.view = view;
            self.touched = Instant::now();
            self.asked = Instant::now();
            self.shown_frames = 0;
            return;
        }
        if step.starts_with("wheel:")
            && let [by, frames] = numbers[..]
        {
            self.wheeled = Some((f64::from(by), frames as u32));
            self.shown_frames = 1;
            return;
        }
        if step == "escape" {
            self.escape = true;
            self.shown_frames = 1;
            return;
        }
        if step == "gpu" {
            self.gpu = !self.gpu;
            self.gpu_chosen = true;
            self.gpu_turned();
            self.shown_frames = 1;
            return;
        }
        if step.starts_with("layer:")
            && let [layer] = numbers[..]
        {
            self.layer = (layer as usize).min(self.base.layers.len().saturating_sub(1));
            self.shown_frames = 1;
            return;
        }
        if let Some(name) = step.strip_prefix("palette:") {
            let choice = palette::NAMES.iter().position(|each| *each == name);
            self.choices.choose(self.chosen_layer(), choice);
            self.requested = None;
            self.asked = Instant::now();
            self.shown_frames = 0;
            return;
        }
        if let Some(name) = step.strip_prefix("preset:") {
            self.show_preset(name);
            self.asked = Instant::now();
            self.shown_frames = 0;
            return;
        }
        if step.starts_with("drag:")
            && let [x, y, to_x, to_y, frames] = numbers[..]
        {
            let screen = ctx.content_rect();
            let at = |x: f32, y: f32| screen.min + screen.size() * egui::vec2(x, y);
            self.dragged = Some((at(x, y), at(to_x, to_y), frames as u32, 0));
            self.shown_frames = 1;
            return;
        }
        if (step.starts_with("click:") || step.starts_with("move:"))
            && let Some(pointer) = &mut self.trace.pointer
            && let [x, y, ref frames @ ..] = numbers[..]
        {
            let screen = ctx.content_rect();
            let to = screen.min + screen.size() * egui::vec2(x, y);
            let frames = frames.first().map_or(30, |frames| *frames as u32);
            pointer.lead(to, frames, step.starts_with("click:"));
            self.shown_frames = 1;
            return;
        }
        if step.starts_with("tap:") || step == "wait" {
            if let [across, down] = numbers[..] {
                let screen = ctx.content_rect();
                self.tap = Some((screen.min + screen.size() * egui::vec2(across, down), 0));
            }
            self.shown_frames = 1;
            return;
        }
        #[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
        if matches!(step, "fullscreen" | "maximize" | "restore") {
            if step == "fullscreen" {
                let full = self.window_state(ctx).1;
                self.full_screen(ctx, !full);
            } else {
                self.toggle_maximized(ctx);
            }
            self.shown_frames = 1;
            return;
        }
        if matches!(step, "hide" | "show") {
            self.bare = step == "hide";
            self.shown_frames = 1;
            return;
        }
        if let Some(code) = step.strip_prefix("lang:") {
            self.set_language(ctx, Lang::from_locale(code));
            self.shown_frames = 1;
            return;
        }
        match step {
            "back" | "forward" => self.travel(step == "back"),
            key => self.act(key),
        }
        if matches!(step, "export" | "controls") {
            self.shown_frames = 1;
            return;
        }
        self.asked = Instant::now();
        self.shown_frames = 0;
    }
}

fn tick(strong: bool) {
    #[cfg(target_os = "ios")]
    ios::tick(strong);
    #[cfg(not(target_os = "ios"))]
    let _ = strong;
}

// One short tick of the phone for a press that has nothing to do; a computer and a browser stay silent.
fn refuse() {
    #[cfg(target_os = "ios")]
    ios::tick(false);
    #[cfg(target_os = "android")]
    android::refuse();
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
    fn raw_input_hook(&mut self, ctx: &egui::Context, input: &mut egui::RawInput) {
        if let Some(scale) = self.trace.scale {
            let native = input.viewport().native_pixels_per_point.unwrap_or(1.0);
            ctx.set_zoom_factor(scale / native);
        }
        if self.trace.fixed_clock {
            input.time = Some(self.trace.frame as f64 / 60.0);
            input.predicted_dt = 1.0 / 60.0;
        }
        if let Some(pointer) = &mut self.trace.pointer {
            pointer.events(&mut input.events);
        }
        if std::mem::take(&mut self.escape) {
            input.events.push(egui::Event::Key {
                key: egui::Key::Escape,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            });
        }
        // Linux takes the pointer for a window it drags and never tells of the button's release; Windows tells.
        if std::mem::take(&mut self.let_go) && !cfg!(windows) {
            let at = ctx.input(|input| input.pointer.latest_pos());
            input.events.push(pace::let_go(at.unwrap_or_default()));
        }
        if let Some((from, to, frames, at)) = self.dragged {
            let pos = from + (to - from) * (at as f32 / frames.max(1) as f32).min(1.0);
            let press = |pressed| egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            };
            input.events.push(egui::Event::PointerMoved(pos));
            if at == 1 {
                input.events.push(press(true));
            } else if at == frames + 2 {
                input.events.push(press(false));
            }
            self.dragged = (at < frames + 2).then_some((from, to, frames, at + 1));
            return;
        }
        let Some((pos, phase)) = self.tap else {
            return;
        };
        let press = |pressed| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        input.events.push(match phase {
            0 => egui::Event::PointerMoved(pos),
            1 => press(true),
            2 => press(false),
            _ => egui::Event::PointerGone,
        });
        self.tap = (phase < 3).then_some((pos, phase + 1));
    }

    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        self.trace.stage("frame");
        let began = Instant::now();
        self.trace.slept = 0.0;
        self.draw(ui, frame);
        if let Some(pointer) = &mut self.trace.pointer {
            pointer.draw(ui.ctx());
        }
        if self.trace.on() || self.trace.recording() || self.trace.fixed_clock {
            let work = began.elapsed().as_secs_f32() * 1000.0 - self.trace.slept;
            self.trace
                .frame(ui.ctx(), work, [&self.panes, &self.panes], &self.fades);
        }
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        storage.set_string("language", self.texts.lang.code().to_owned());
        storage.set_string("glass_opacity", self.optics.tint.to_string());
        storage.set_string("glass_blur", self.optics.blur.to_string());
        if std::env::var_os("FRACTIQUE_GPU").is_none() {
            storage.set_string("gpu", if self.gpu { "on" } else { "off" }.to_owned());
            if self.gpu_chosen {
                storage.set_string("gpu_by", "owner".to_owned());
            }
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
        #[cfg(feature = "store")]
        storage.set_string("saves", self.saves.text());
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

impl App {
    fn draw(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let now = ctx.input(|input| input.time);
        self.stage.begin(now);
        self.fades.clear();
        // The window comes first: while it is used, and for a moment after, a picture gets the smaller share
        // of the graphics card.
        let used = ctx.input(|input| pace::used(input.pointer.any_down(), &input.events)) || self.stage.moving();
        if used {
            self.used = Instant::now();
        }
        pace::attend(self.used.elapsed().as_secs_f32() < 0.3);
        let (held, turned) = ctx.input(|input| (input.pointer.any_down(), pace::turned(&input.events)));
        if turned {
            self.turned = Some(Instant::now());
        }
        pace::hold(held || self.turned.is_some_and(|at| at.elapsed().as_secs_f32() < 0.3));
        #[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
        self.arrive(&ctx);
        // Every mouse move asks for a redraw; at a 240 Hz screen's pace that makes a graphics card whine.
        #[cfg(not(target_arch = "wasm32"))]
        {
            let pace = std::time::Duration::from_micros(16_600);
            let since = self.last_frame.elapsed();
            if since < pace {
                std::thread::sleep(pace - since);
                self.trace.slept = (pace - since).as_secs_f32() * 1000.0;
            }
            self.last_frame = Instant::now();
        }
        let clock = Instant::now();
        self.receive(&ctx, frame);
        self.trace.parts[0] = clock.elapsed().as_secs_f32() * 1000.0;
        #[cfg(not(target_arch = "wasm32"))]
        self.finish_export();
        let own = pace::FRAME_ON_CARD.swap(0, Ordering::Relaxed);
        if own > 0 {
            pace::needs(own);
            self.trace.export.push_str(&format!(" own {:.2}", own as f32 * 1e-3));
        }
        #[cfg(feature = "store")]
        self.settle_store(&ctx);
        #[cfg(target_os = "android")]
        self.keep_on_android(&ctx);
        #[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
        self.updates.settle(&ctx);
        if ctx.input(|input| input.key_pressed(egui::Key::F2)) {
            self.tuning = !self.tuning;
        }
        if ctx.input(|input| input.key_pressed(egui::Key::BrowserBack)) {
            self.leave();
        }

        let screen = ctx.viewport_rect();
        self.fathom(&ctx);
        self.explore(ui, screen);
        // The interface goes and comes back as one: every pane is shown by its share of it.
        #[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
        {
            self.window_keys(&ctx);
            self.eye_pressed(&ctx);
        }
        let face = self.stage.prompt("interface", !self.bare, motion::PANEL);
        let aside = face.is_none() || self.phone.preview;
        if self.bare {
            self.languages_open = false;
            self.sizes_open = false;
            self.presets_open = false;
        }
        self.stage.dim = if aside { 0.0 } else { 1.0 - face.unwrap_or(1.0) };
        #[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
        let (stand, content, whole) = {
            #[cfg(windows)]
            if self.handle.is_none() {
                use eframe::egui_wgpu::wgpu::rwh::{HasWindowHandle as _, RawWindowHandle};
                self.handle = match frame.window_handle().map(|handle| handle.as_raw()) {
                    Ok(RawWindowHandle::Win32(window)) => Some(window.hwnd.get()),
                    _ => None,
                };
                if let Some(handle) = self.handle {
                    motion::whole::watch(handle);
                }
            }
            let stand = self.stand(&ctx, screen, now);
            let safe = safe_content(&ctx);
            let mut content =
                egui::Rect::from_min_max(stand.min + (safe.min - screen.min), stand.max - (screen.max - safe.max));
            let mut whole = safe;
            if face.is_some() && !self.phone.on {
                content.min.y += BAR + BAR_GAP;
                whole.min.y += BAR + BAR_GAP;
            }
            (stand, content, whole)
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
        // The layout a width decides goes by the window as it is, not by the frame that glides to it.
        #[cfg(any(target_os = "android", target_os = "ios", target_arch = "wasm32"))]
        let whole = content;
        let scale = ctx.pixels_per_point();
        let size = [
            (screen.width() * scale).round() as u32,
            (screen.height() * scale).round() as u32,
        ];
        let window = size;
        self.window = window;
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
        // A window whose edge is dragged keeps its picture, laid out for the new size, until the size has stood. A
        // picture of a chosen size is not drawn again for a window: nothing waits, blurs or counts then.
        let wait = self.stood.wait(size, now, motion::span(SETTLE));
        let resizing = wait > 0.0 && self.requested.is_some();
        if resizing {
            ctx.request_repaint_after(std::time::Duration::from_secs_f32(wait));
        }
        let reshaped = self.shape_stood.wait(window, now, motion::span(SETTLE));
        let clock = Instant::now();
        if window[0] > 0 && window[1] > 0 && !resizing {
            if self.requested != Some(size) {
                // A picture asked for after the frame's end, as a test run's step does, counts from here.
                if self.requested.is_none() && !std::mem::take(&mut self.begun) {
                    self.began = Instant::now();
                }
                self.moved_only = self.requested.is_some();
                self.draft_off = false;
                if self.card_shows(size) {
                    self.requested = Some(size);
                    self.drawn = Some((self.view, self.adjust.zoom));
                } else if self.latest.is_some() && self.preview_now(&ctx, frame, size) {
                    self.hold_render(&ctx, size);
                } else {
                    self.previewing = false;
                    self.show_view(&ctx, size, true);
                }
            } else if self.drawn != Some((self.view, self.adjust.zoom)) && !self.frozen && self.drafts() {
                self.moved_only = true;
                if self.drafting()
                    && let Some(mut chaos) = self.chaos.take()
                {
                    self.draft_step(&mut chaos, &ctx, frame);
                    self.chaos = Some(chaos);
                }
                let flying = self.drafting() && !self.draft_landed;
                let shown = self.draft_shown == Some((self.view, self.adjust.zoom));
                match pace::hand_over(flying, shown) {
                    pace::Next::Wait => {}
                    pace::Next::Draft if self.begin_draft(&ctx, size) => {}
                    pace::Next::Draft | pace::Next::Picture => self.show_view(&ctx, size, self.drawn.is_none()),
                }
            } else if self.drawn != Some((self.view, self.adjust.zoom)) && !self.frozen {
                let still = self.touched.elapsed().as_secs_f32();
                if still >= SETTLE || self.swift {
                    self.moved_only = true;
                    self.show_view(&ctx, size, self.drawn.is_none());
                } else {
                    ctx.request_repaint_after(std::time::Duration::from_secs_f32(SETTLE - still));
                }
            } else if self.picture.is_some() && !self.rendering && !self.frozen && self.fit(size) != self.fit_asked {
                // A window of another size shows a picture of a chosen size with more or fewer pixels: once the
                // size has stood the picture is drawn with those.
                if reshaped > 0.0 {
                    ctx.request_repaint_after(std::time::Duration::from_secs_f32(reshaped));
                } else {
                    self.fit_asked = self.fit(size);
                    self.moved_only = true;
                    self.began = Instant::now();
                    self.show_view(&ctx, size, false);
                }
            } else if self.swift && self.chaotic && !self.rendering && !self.frozen && self.picture.is_some() {
                // A window of another shape shows more or less beside a picture of a chosen size: once the shape
                // has stood the card draws the picture with that beside it.
                let full = self.full(size);
                let other = !self.covering() && strips([full.width, full.height], window) != self.strips_asked;
                if other && reshaped > 0.0 {
                    ctx.request_repaint_after(std::time::Duration::from_secs_f32(reshaped));
                } else if other && self.gpu_on() && self.fast(size) {
                    self.moved_only = true;
                    self.began = Instant::now();
                    self.show_view(&ctx, size, false);
                }
            }
        }
        self.trace.parts[1] = clock.elapsed().as_secs_f32() * 1000.0;
        let clock = Instant::now();
        self.chaos_frame(&ctx, frame);
        self.trace.parts[2] = clock.elapsed().as_secs_f32() * 1000.0;
        self.resizing = resizing;

        // A layer's painter stops at the safe area, and `with_clip_rect` can only narrow that: the picture goes under
        // the camera and the home bar, to the edges of the screen.
        let mut painter = ctx.layer_painter(egui::LayerId::background());
        painter.set_clip_rect(screen);
        if std::env::var_os("FRACTIQUE_DEBUG_LAYOUT").is_some() {
            let mut ruler = ctx.layer_painter(egui::LayerId::debug());
            ruler.set_clip_rect(screen);
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

        if aside {
            let mut panes = Vec::new();
            if self.phone.preview {
                for rect in self.clock(&ctx, content, screen) {
                    panes.push((rect, rect.height() * 0.5, false));
                }
            }
            let reset = self.view != WHOLE && !self.phone.preview;
            if let Some(shown) = self.stage.apart("reset", reset, motion::SMALL) {
                let from = panes.len();
                let at = self.reset_at(None, content);
                let rect = self.reset_button(&ctx, at, shown, reset);
                panes.push((rect, rect.height() * 0.5, false));
                self.faded(&panes, from, shown);
            }
            self.glaze(&mut glass, panes, &ctx, frame, size, screen);
            self.stage.keep_moving(&ctx);
            if self.screenshot.is_some() {
                self.take_screenshot(&ctx);
            }
            return;
        }

        let wide = motion::latch(self.wide, whole.width(), 700.0, motion::BAND);
        self.wide = Some(wide);
        let open = *self.inspector_open.get_or_insert(wide);
        // A sheet's glass is drawn under every text, so a pane the sheet reaches, at the place where it will stand,
        // gives way to it whole; one it does not reach stays as it is. What gave way comes back once the sheet has
        // mostly gone: the two are never both whole.
        if let Some(seat) = self.seat_of("inspector") {
            let corner = content.min + egui::vec2(20.0, 20.0);
            let tall = (content.height() - 40.0 - self.under).max(0.0);
            self.seat(
                "inspector",
                egui::Rect::from_min_size(corner, egui::vec2(seat.width(), tall)),
            );
        }
        let hidden = self.gives("inspector");
        let open = open && !hidden;

        let column = (whole.width() - 84.0).min(248.0);
        let beside = whole.left() + 20.0 + column + 44.0 + 16.0;
        let mut panes = Vec::new();
        // What does not rise by itself as it comes sinks by as much while the interface goes.
        let sink = egui::vec2(0.0, self.stage.dim * motion::SHIFT);
        let live = !self.bare;
        if self.phone.on {
            self.phone_home(&ctx, content, size, &mut panes);
        } else {
            #[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
            if let Some(shown) = self.stage.shown("title", true, motion::PANEL) {
                panes.push((self.title_bar(&ctx, stand.translate(sink), shown), BAR * 0.5, false));
                self.faded(&panes, 0, shown);
            }

            // Where the toolbar stands, or would: a pane that has given way is looked for where it would be now.
            let foot = egui::pos2(content.center().x, content.bottom() - 20.0);
            if self.toolbar_size != egui::Vec2::ZERO {
                let seat = egui::Align2::CENTER_BOTTOM.anchor_size(foot, self.toolbar_size);
                self.seat("toolbar", seat);
            }
            let bar_on = !self.gives("toolbar");
            if let Some(shown) = self.stage.shown("toolbar", bar_on, motion::GIVE) {
                let toolbar = egui::Area::new(egui::Id::unique("toolbar"))
                    .fixed_pos(egui::pos2(content.center().x, content.bottom() - 20.0) + sink)
                    .pivot(egui::Align2::CENTER_BOTTOM)
                    .constrain(false)
                    .interactable(bar_on && live)
                    .fade_in(false)
                    .show(&ctx, |ui| {
                        ui.set_opacity(shown);
                        egui::Frame::NONE.inner_margin(egui::Margin::same(9)).show(ui, |ui| {
                            let detail = self.detail();
                            let name = ui.painter().layout_no_wrap(
                                self.label(&self.title, self.phone.origin),
                                font(13.5, MEDIUM),
                                egui::Color32::WHITE,
                            );
                            let detail = ui.painter().layout_no_wrap(
                                detail,
                                egui::FontId::proportional(13.0),
                                egui::Color32::from_white_alpha(170),
                            );
                            // The digits of a time that counts are of different widths: the line is laid out
                            // for the widest, so nothing beside them moves.
                            let widest: String = self
                                .detail()
                                .chars()
                                .map(|c| if c.is_ascii_digit() { '0' } else { c })
                                .collect();
                            let widest = ui.painter().layout_no_wrap(
                                widest,
                                egui::FontId::proportional(13.0),
                                egui::Color32::WHITE,
                            );
                            let room = widest.size().x.max(detail.size().x);
                            let width = (self.toolbar_size.x - 18.0).max(name.size().x + 10.0 + room);
                            let (line, _) = ui.allocate_exact_size(egui::vec2(width, 24.0), egui::Sense::hover());
                            let left = line.center().x - (name.size().x + 10.0 + room) * 0.5;
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
                            let tight = egui::remap_clamp(whole.width(), 676.0..=724.0, 0.0..=1.0);
                            self.toolbar(ui, tight, whole.width() - 58.0);
                        });
                    });
                self.toolbar_size = toolbar.response.rect.size();
                let from = panes.len();
                panes.push((
                    toolbar.response.rect,
                    (toolbar.response.rect.height() * 0.5).min(28.0),
                    false,
                ));
                self.faded(&panes, from, shown);
            }

            // The panel grows shorter as the buttons come to stand under it, over the last points before they do.
            let apart = whole.center().x - self.toolbar_size.x * 0.5 - beside;
            let clear = if wide {
                (apart / motion::RAMP).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let lifted = if self.view != WHOLE { 56.0 } else { 0.0 };
            let lifted = self.stage.glide("lifted", lifted, motion::SMALL);
            let room = self.toolbar_size.y.max(60.0) + 12.0 + lifted;
            let landed = std::mem::take(&mut self.landed);
            let under = motion::under(
                &mut self.stage,
                (1.0 - clear) * room,
                self.leap.is_some().then(|| {
                    let near =
                        content.center().x - self.toolbar_size.x * 0.5 - (content.left() + beside - whole.left());
                    if content.width() < 700.0 {
                        room
                    } else {
                        (1.0 - (near / motion::RAMP).clamp(0.0, 1.0)) * room
                    }
                }),
                landed,
            );
            self.under = under;
            let span = if hidden { motion::GIVE } else { motion::PANEL };
            if let Some(shown) = self.stage.shown("inspector", open, span) {
                let corner = content.min + egui::vec2(20.0, 20.0);
                let inspector = egui::Area::new(egui::Id::unique("inspector"))
                    .fixed_pos(corner)
                    .constrain(false)
                    .interactable(open && live)
                    .fade_in(false)
                    .show(&ctx, |ui| {
                        ui.set_opacity(shown);
                        let pad = widgets::PAD as i8;
                        let margin = egui::Margin {
                            left: pad,
                            right: pad - 14,
                            top: pad,
                            bottom: pad,
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
                self.seat("inspector", inspector.response.rect);
                self.pane(&ctx, &mut panes, &inspector.response, self.optics.radius, false, shown);
            }

            if let Some(seat) = self.seat_of("reset") {
                let above = egui::pos2(content.center().x, content.bottom() - 20.0 - self.toolbar_size.y - 10.0);
                self.seat("reset", egui::Align2::CENTER_BOTTOM.anchor_size(above, seat.size()));
            }
            let reset = self.view != WHOLE && !self.gives("reset");
            if let Some(shown) = self.stage.apart("reset", reset, motion::SMALL) {
                let above = content.bottom() - 20.0 - self.toolbar_size.y - 10.0;
                let from = panes.len();
                let at = self.reset_at(Some(egui::pos2(content.center().x, above)), content);
                let rect = self.reset_button(&ctx, at, shown, reset && live);
                self.seat("reset", rect);
                panes.push((rect, rect.height() * 0.5, false));
                self.faded(&panes, from, shown);
            }
        }

        #[cfg(not(target_arch = "wasm32"))]
        if let Some(shown) = self.stage.shown("export", self.export.open, motion::PANEL) {
            let leaf = Leaf {
                live: self.export.open,
                height: "export_tall",
                at: content.center(),
                pivot: egui::Align2::CENTER_CENTER,
                shown,
            };
            let sheet = Self::sheet(
                "export",
                content.center(),
                egui::Align2::CENTER_CENTER,
                self.export.open && live,
            )
            .show(&ctx, |ui| {
                self.unfold(ui, &leaf);
                widgets::pane().show(ui, |ui| {
                    let low = content.width() > content.height() && content.height() < 560.0;
                    let width = sheet_width(content.width(), if low { 604.0 } else { 300.0 }, self.phone.on);
                    self.export_sheet(ui, window, content.height(), (width, low));
                    if widgets::close(ui, &self.t("close")) {
                        self.export.open = false;
                    }
                });
            });
            self.leaf(&ctx, &mut panes, &sheet.response, &leaf, self.optics.radius, true);
        }

        #[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
        if let Some(shown) = self.stage.shown("updates", self.updates.open, motion::PANEL) {
            let width = (content.width() - 84.0).min(600.0);
            let leaf = Leaf {
                live: self.updates.open,
                height: "updates_tall",
                at: content.center(),
                pivot: egui::Align2::CENTER_CENTER,
                shown,
            };
            let sheet = Self::sheet(
                "updates",
                content.center(),
                egui::Align2::CENTER_CENTER,
                self.updates.open && live,
            )
            .show(&ctx, |ui| {
                self.unfold(ui, &leaf);
                widgets::pane().show(ui, |ui| {
                    self.updates.ui(ui, &self.texts, width);
                    if widgets::close(ui, &self.t("close")) {
                        self.updates.open = false;
                    }
                });
            });
            self.leaf(&ctx, &mut panes, &sheet.response, &leaf, self.optics.radius, true);
        }

        #[cfg(feature = "store")]
        if let Some(shown) = self.stage.shown("pro", self.pro.is_some(), motion::PANEL) {
            let width = sheet_width(content.width(), 300.0, self.phone.on).min(content.width() - 84.0);
            let leaf = Leaf {
                live: self.pro.is_some(),
                height: "pro_tall",
                at: content.center(),
                pivot: egui::Align2::CENTER_CENTER,
                shown,
            };
            let sheet = Self::sheet(
                "pro",
                content.center(),
                egui::Align2::CENTER_CENTER,
                self.pro.is_some() && live,
            )
            .show(&ctx, |ui| {
                self.unfold(ui, &leaf);
                widgets::pane().show(ui, |ui| {
                    self.pro_sheet(ui, width);
                    if widgets::close(ui, &self.t("close")) {
                        self.leave_pro();
                    }
                });
            });
            self.leaf(&ctx, &mut panes, &sheet.response, &leaf, self.optics.radius, true);
        }

        if let Some(shown) = self.stage.shown("presets", self.presets_open, motion::PANEL) {
            let live = self.presets_open;
            let (at, pivot, room) = list_place(content, self.phone.on);
            if live {
                self.list_backdrop(&ctx, screen);
            }
            let natural = if self.listing == Listing::Recent { 380.0 } else { 260.0 };
            let width = sheet_width(content.width(), natural, self.phone.on);
            let leaf = Leaf {
                live,
                height: "presets_tall",
                at,
                pivot,
                shown,
            };
            let sheet = Self::sheet("presets", at, pivot, live).show(&ctx, |ui| {
                self.unfold(ui, &leaf);
                widgets::pane().show(ui, |ui| {
                    self.preset_sheet(ui, room, width);
                    if widgets::close(ui, &self.t("close")) {
                        self.presets_open = false;
                    }
                });
            });
            ctx.move_to_top(sheet.response.layer_id);
            self.leaf(&ctx, &mut panes, &sheet.response, &leaf, self.optics.radius, true);
        }

        if let Some(shown) = self.stage.shown("sizes", self.sizes_open, motion::PANEL) {
            let live = self.sizes_open;
            let (at, pivot, room) = list_place(content, self.phone.on);
            if live {
                self.list_backdrop(&ctx, screen);
            }
            let leaf = Leaf {
                live,
                height: "sizes_tall",
                at,
                pivot,
                shown,
            };
            let sheet = Self::sheet("sizes", at, pivot, live).show(&ctx, |ui| {
                self.unfold(ui, &leaf);
                widgets::pane().show(ui, |ui| {
                    let width = sheet_width(content.width(), 300.0, self.phone.on);
                    self.size_sheet(ui, window, room, width);
                    if widgets::close(ui, &self.t("close")) {
                        self.sizes_open = false;
                    }
                });
            });
            ctx.move_to_top(sheet.response.layer_id);
            self.leaf(&ctx, &mut panes, &sheet.response, &leaf, self.optics.radius, true);
        }

        if let Some(shown) = self.stage.shown("languages", self.languages_open, motion::PANEL) {
            let live = self.languages_open;
            // On a computer the list hangs from its button in the title bar and is left as any dropdown is: by a
            // row, the button, a click beside it or Escape. It has no close button of its own.
            let dropdown = !self.phone.on
                && cfg!(not(any(
                    target_os = "android",
                    target_os = "ios",
                    target_arch = "wasm32"
                )));
            let (at, pivot, room) = if dropdown {
                let at = self.language_anchor + egui::vec2(0.0, 12.0);
                (at, egui::Align2::RIGHT_TOP, content.height())
            } else {
                list_place(content, self.phone.on)
            };
            let mut chosen = self.texts.lang;
            if live {
                self.list_backdrop(&ctx, screen);
            }
            let leaf = Leaf {
                live,
                height: "languages_tall",
                at,
                pivot,
                shown,
            };
            let menu = Self::sheet("languages", at, pivot, live).show(&ctx, |ui| {
                self.unfold(ui, &leaf);
                widgets::pane().show(ui, |ui| {
                    ui.set_width(sheet_width(content.width(), 196.0, self.phone.on));
                    ui.spacing_mut().item_spacing = egui::vec2(8.0, 2.0);
                    if dropdown {
                        widgets::caption(ui, &self.t("language"));
                    } else {
                        widgets::heading(ui, &self.t("language"));
                    }
                    let mut bar = egui::style::ScrollStyle::floating();
                    bar.bar_width = 5.0;
                    ui.spacing_mut().scroll = bar;
                    let tall = list_height(Lang::ALL.len(), room);
                    egui::ScrollArea::vertical()
                        .max_height(tall)
                        .min_scrolled_height(tall)
                        .show(ui, |ui| {
                            for lang in Lang::ALL {
                                if widgets::row(ui, lang.name(), lang == chosen) {
                                    chosen = lang;
                                    self.languages_open = false;
                                }
                            }
                        });
                    if !dropdown && widgets::close(ui, &self.t("close")) {
                        self.languages_open = false;
                    }
                });
            });
            ctx.move_to_top(menu.response.layer_id);
            if chosen != self.texts.lang {
                self.set_language(&ctx, chosen);
            }
            self.leaf(&ctx, &mut panes, &menu.response, &leaf, self.optics.radius, true);
        }
        self.covers = std::mem::take(&mut self.covering);

        if let Some(seat) = self.seat_of("tuning") {
            let corner = egui::pos2(
                content.right() - 20.0,
                content.bottom() - 20.0 - (content.bottom() - 20.0 - seat.bottom()).max(0.0),
            );
            self.seat("tuning", egui::Align2::RIGHT_BOTTOM.anchor_size(corner, seat.size()));
        }
        let tuned = self.tuning && !self.gives("tuning");
        if let Some(shown) = self.stage.shown("tuning", tuned, motion::PANEL) {
            // In a narrow window the toolbar reaches under this panel's place: the panel stands above it.
            let wide = 220.0 + 2.0 * widgets::PAD;
            let reach = content.center().x + self.toolbar_size.x * 0.5 + 12.0;
            let lift = if !self.phone.on && reach > content.right() - 20.0 - wide {
                self.toolbar_size.y + 12.0
            } else {
                0.0
            };
            let lift = self.stage.glide("tuning_lift", lift, motion::SMALL);
            let tuning = egui::Area::new(egui::Id::unique("tuning"))
                .fixed_pos(egui::pos2(content.right() - 20.0, content.bottom() - 20.0 - lift))
                .pivot(egui::Align2::RIGHT_BOTTOM)
                .constrain(false)
                .interactable(tuned && live)
                .fade_in(false)
                .show(&ctx, |ui| {
                    ui.set_opacity(shown);
                    widgets::pane().show(ui, |ui| {
                        self.tuning(ui);
                        if widgets::close(ui, &self.t("close")) {
                            self.tuning = false;
                        }
                    });
                });
            self.seat("tuning", tuning.response.rect);
            self.pane(&ctx, &mut panes, &tuning.response, self.optics.radius, false, shown);
        }

        self.glaze(&mut glass, panes, &ctx, frame, size, screen);
        self.stage.keep_moving(&ctx);
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

// The tone and the view of a saved session. A session written while the controls had a scale of their own holds
// it as a fourth number of the tone: it was part of that picture's view, and is no setting of the app.
fn saved_look(adjust: Option<&[f64]>, view: Option<&[f64]>) -> (Option<Adjust>, Option<View>) {
    let old = match adjust {
        Some(&[.., scale]) if adjust.is_some_and(|numbers| numbers.len() == 4) => scale,
        _ => 1.0,
    };
    let adjust = match adjust {
        Some(&[brightness, gamma, glow, ..]) => Some(Adjust {
            brightness,
            gamma,
            glow,
            zoom: 1.0,
        }),
        _ => None,
    };
    let view = match view {
        Some(&[right, down, zoom]) if (MIN_MAGNIFICATION..=MAX_MAGNIFICATION).contains(&zoom) => {
            let old = if old.is_finite() && old > 0.0 { old } else { 1.0 };
            Some(View { right, down, zoom }.scaled(old))
        }
        _ => None,
    };
    (adjust, view)
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
    fn a_view_of_a_wallpaper_never_shows_what_is_beside_it() {
        let size = [1600, 1000];
        // Out of the whole picture: back to it, in its middle.
        let mut view = View {
            right: 0.3,
            down: -0.2,
            zoom: 0.174,
        };
        view.limited(size, 1.0, Some([1.0; 2]));
        assert_eq!(view, WHOLE);
        // Twice as large: half of the picture is seen, so the middle may move a quarter of it either way.
        let mut view = View {
            right: 5.0,
            down: -5.0,
            zoom: 2.0,
        };
        view.limited(size, 1.0, Some([1.0; 2]));
        assert!((view.right - 0.4).abs() < 1e-12 && (view.down + 0.25).abs() < 1e-12);
        let [across, along, larger] = view.within(&WHOLE, size);
        let half = 0.5 / larger;
        assert!(across + half <= 1.0 + 1e-6 && along - half >= -1e-6);
        // A view inside the picture is left as it is.
        let inside = View {
            right: 0.1,
            down: 0.05,
            zoom: 3.0,
        };
        let mut view = inside;
        view.limited(size, 1.0, Some([1.0; 2]));
        assert_eq!(view, inside);
        // The wheel stops at the whole picture and does not push the view aside.
        let mut view = WHOLE;
        view.magnified(0.5, [0.3, 0.2], 1.0, 1e10);
        view.limited(size, 1.0, Some([1.0; 2]));
        assert_eq!(view, WHOLE);
        // And at eight times the picture.
        view.magnified(1e6, [0.0; 2], 1.0, IMAGE_CEILING);
        assert_eq!(view.zoom, IMAGE_CEILING);
    }

    #[test]
    fn a_wallpaper_of_a_chosen_size_fills_a_window_of_another_shape() {
        // 16 to 9 in a maximized window of 1920 by 1032 points: 4.65 % larger, cut above and below.
        let stretch = (16.0 / 9.0) / (1920.0 / 1032.0);
        let (fill, [across, down]) = filled(stretch);
        assert!((fill - 1.0465).abs() < 1e-3);
        assert!((across - 1.0).abs() < 1e-6 && (down - 1.0 / 1.0465).abs() < 1e-3);
        // In a window less wide than the picture it is cut at its sides.
        let (fill, [across, down]) = filled(1.1);
        assert!((fill - 1.1).abs() < 1e-6 && (across - 1.0 / 1.1).abs() < 1e-6 && (down - 1.0).abs() < 1e-6);
        // A picture of another shape altogether is not cut by more than a fifth: it leaves the window's sides.
        let (fill, [across, down]) = filled(0.25);
        assert_eq!(fill, FILL_MOST);
        assert!(across > 1.0 && (down - 0.8).abs() < 1e-6);
        // The whole view moves over what is cut and no further, so all of the picture can be seen.
        let mut view = View {
            right: 1.0,
            down: 1.0,
            zoom: 1.0,
        };
        view.limited([3840, 2160], 1.0, Some(filled(stretch).1));
        assert_eq!(view.right, 0.0);
        assert!((view.down - 0.5 * (1.0 - 1.0 / 1.0465)).abs() < 1e-3);
        // Where it leaves the window's sides it does not move across until it is larger than the window.
        let mut view = View {
            right: 1.0,
            down: 0.0,
            zoom: 2.0,
        };
        view.limited([1000, 2000], 1.0, Some(filled(0.25).1));
        assert_eq!(view.right, 0.0);
    }

    #[test]
    fn a_picture_of_a_chosen_size_is_drawn_with_the_pixels_the_window_shows_it_with() {
        let window = [3840, 2160];
        assert_eq!(on_screen([15360, 8640], window, 1.0), window);
        assert_eq!(on_screen([7680, 4320], window, 1.0), window);
        assert_eq!(on_screen([3840, 2160], window, 1.0), window);
        // A smaller picture keeps its own pixels, and one of another shape lies whole in the window.
        assert_eq!(on_screen([1280, 720], window, 1.0), [1280, 720]);
        assert_eq!(on_screen([8640, 15360], window, 1.0), [1215, 2160]);
        assert_eq!(on_screen([10320, 4320], window, 1.0), [3840, 1607]);
        // Whatever is chosen, no side is longer than the window's.
        for size in [[15360, 8640], [16384, 16384], [16384, 100], [100, 16384], [5000, 3000]] {
            let drawn = on_screen(size, window, 1.0);
            assert!(
                drawn[0] <= window[0] && drawn[1] <= window[1],
                "{size:?} is drawn {drawn:?}"
            );
            assert!(drawn[0] <= size[0] && drawn[1] <= size[1]);
        }
        // A wallpaper that fills the window is shown larger and cut: it is drawn with as many pixels more.
        assert_eq!(on_screen([8640, 8640], window, 1.25), [2700, 2700]);
        assert_eq!(on_screen([2000, 2000], window, 1.25), [2000, 2000]);
        // Before the window has a size the picture has its own.
        assert_eq!(on_screen([7680, 4320], [0, 0], 1.0), [7680, 4320]);
        // The picture stands for the chosen one scaled down: so many of its pixels a side to one of the screen's.
        assert_eq!(finer([15360, 8640], window), 4);
        assert_eq!(finer([7680, 4320], window), 2);
        assert_eq!(finer([3840, 2160], window), 1);
        assert_eq!(finer([1280, 720], [1280, 720]), 1);
        assert_eq!(finer([16384, 16384], [1080, 1080]), FINEST);
        assert_eq!(finer([5760, 3240], window), 2);
    }

    #[test]
    fn what_a_window_shows_beside_a_picture_of_a_chosen_size_is_drawn_with_it() {
        // 3840 × 2160 in a maximized window of 2576 × 1385 pixels: 89 pixels a side, and one more.
        assert_eq!(strips([3840, 2160], [2576, 1385]), [90, 0]);
        assert_eq!(strips([3840, 2160], [1920, 1080]), [0, 0]);
        // A window less wide than the picture shows more above and below.
        assert_eq!(strips([3840, 2160], [1600, 1200]), [0, 361]);
        // At most half of the picture's side: a portrait picture in a wide window.
        assert_eq!(strips([1170, 2532], [1920, 1032]), [585, 0]);
        // A picture fitted to the largest side the app draws keeps its shape but for the rounding.
        assert_eq!(strips([4096, 2201], [1920, 1032]), [0, 0]);
    }

    #[test]
    fn a_view_of_a_fractal_stops_before_the_figure_is_a_speck_or_out_of_sight() {
        let size = [3840, 2160];
        // Mandelbox Cube, wheeled out to 0.009 times: it stops at a fifth of its framing.
        let mut view = WHOLE;
        for _ in 0..40 {
            view.zoomed(0.8, [0.0; 2], size, (FLOOR, 1e10), None);
        }
        assert_eq!(view.zoom, FLOOR);
        // With the scale of the controls at 0.5 the wheel has half as far to go.
        let mut scaled = View { zoom: 0.009, ..WHOLE };
        scaled.limited(size, FLOOR / 0.5, None);
        assert_eq!(scaled.zoom * 0.5, FLOOR);
        // At the floor the figure moves about and never leaves the view, on any side.
        let half = 3840.0 / 2160.0 / 2.0;
        for (right, down) in [(40.0, 0.0), (-40.0, 0.0), (0.0, 40.0), (0.0, -40.0)] {
            let mut view = View {
                right,
                down,
                zoom: FLOOR,
            };
            view.limited(size, FLOOR, None);
            let [across, along, larger] = view.within(&WHOLE, size);
            let seen = 0.5 / larger;
            assert!(across - seen <= 1e-6 && across + seen >= 1.0 - 1e-6);
            assert!(along - seen <= 1e-6 && along + seen >= 1.0 - 1e-6);
            assert!(view.right.abs() <= half * 4.0 + 1e-9 && view.down.abs() <= 2.0 + 1e-9);
        }
        // A view within its limits is not touched.
        let inside = View {
            right: -0.5,
            down: 0.3,
            zoom: 0.6,
        };
        let mut view = inside;
        view.limited(size, FLOOR, None);
        assert_eq!(view, inside);
    }

    // The pictures a view is limited for: the smallest and the largest scale and what a wallpaper covers.
    type Limits = (&'static str, (f64, f64), Option<[f64; 2]>);

    fn kinds_of_picture() -> Vec<Limits> {
        vec![
            ("a flame", (FLOOR, FLAME_DEEPEST.1), None),
            ("a shallow flame", (FLOOR, FLAME_DEEPEST.0), None),
            ("an attractor", (FLOOR, orbit_deepest("lorenz").unwrap_or(1.0)), None),
            ("a fractal drawn pixel by pixel", (FLOOR, MAX_MAGNIFICATION), None),
            (
                "a solid inside a magnified scene",
                (FLOOR, MAX_MAGNIFICATION / 40.0),
                None,
            ),
            (
                "a flame under controls that halve it",
                (FLOOR / 0.5, FLAME_DEEPEST.1 / 0.5),
                None,
            ),
            ("a wallpaper", (1.0, IMAGE_CEILING), Some([1.0; 2])),
            (
                "a wallpaper cut above and below",
                (1.0, IMAGE_CEILING),
                Some(filled(0.9).1),
            ),
            (
                "a wallpaper beside which the window shows more",
                (1.0, IMAGE_CEILING),
                Some(filled(0.25).1),
            ),
        ]
    }

    const SHAPES: [[u32; 2]; 4] = [[1920, 1080], [1080, 1920], [1000, 1000], [3440, 1440]];

    fn scales((least, most): (f64, f64)) -> Vec<f64> {
        let mut scales: Vec<f64> = (0..=24)
            .map(|step| least * fractique_core::math::pow(most / least, f64::from(step) / 24.0))
            .collect();
        scales.extend(
            [1.0, 1.001, 1.25, 2.0]
                .into_iter()
                .filter(|scale| (least..=most).contains(scale)),
        );
        scales.sort_by(f64::total_cmp);
        scales
    }

    // Places all over a territory and past it, as shares of its half sides.
    fn places() -> impl Iterator<Item = [f64; 2]> {
        let shares = [-1.5, -1.0, -0.999, -0.6, -0.2, 0.0, 0.1, 0.5, 0.97, 1.0, 3.0];
        shares
            .into_iter()
            .flat_map(move |across| shares.map(|down| [across, down]))
    }

    fn inside(view: &View, size: [u32; 2], least: f64, covering: Option<[f64; 2]>) -> bool {
        let (territory, held) = (territory(size, least, covering), view.held(size, covering));
        let lies = |at: f64, side: usize| {
            let slack = 1e-9 * territory[side];
            if held[side] > territory[side] {
                at.abs() <= slack
            } else {
                at.abs() + held[side] <= territory[side] + slack
            }
        };
        lies(view.right, 0) && lies(view.down, 1)
    }

    #[test]
    fn the_camera_goes_at_every_scale_where_it_goes_at_the_whole_picture() {
        for (name, (least, most), covering) in kinds_of_picture() {
            for size in SHAPES {
                let territory = territory(size, least, covering);
                let unit = 1.0_f64.clamp(least, most);
                for place in places() {
                    let mut whole = View {
                        right: place[0] * territory[0],
                        down: place[1] * territory[1],
                        zoom: unit,
                    };
                    whole.limited(size, least, covering);
                    assert!(inside(&whole, size, least, covering), "{name} {size:?} {place:?}");
                    for zoom in scales((least, most)).into_iter().filter(|zoom| *zoom >= unit) {
                        let asked = View { zoom, ..whole };
                        let mut view = asked;
                        view.limited(size, least, covering);
                        assert_eq!(view, asked, "{name} {size:?}: {zoom} times is not where 1 times is");
                    }
                }
                // The edge a view reaches is the territory's, whatever the scale.
                for zoom in scales((least, most)) {
                    let mut view = View {
                        right: 1e5,
                        down: -1e5,
                        zoom,
                    };
                    view.limited(size, least, covering);
                    let held = view.held(size, covering);
                    for (at, side) in [(view.right, 0), (-view.down, 1)] {
                        if held[side] <= territory[side] {
                            let edge = at + held[side];
                            assert!(
                                (edge - territory[side]).abs() <= 1e-9 * territory[side],
                                "{name} {zoom}"
                            );
                        } else {
                            assert_eq!(at, 0.0, "{name} {zoom}");
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn a_view_made_larger_keeps_the_point_it_is_made_larger_around() {
        for (name, reach, covering) in kinds_of_picture() {
            for size in SHAPES {
                let territory = territory(size, reach.0, covering);
                let fine = 1e-9 * territory[0].max(territory[1]);
                for scale in scales(reach) {
                    for place in places().step_by(3) {
                        let mut view = View {
                            right: place[0] * territory[0],
                            down: place[1] * territory[1],
                            zoom: scale,
                        };
                        view.limited(size, reach.0, covering);
                        let (from, held) = (view, view.held(size, covering));
                        for corner in [[0.0, 0.0], [0.9, -0.9], [-1.0, 1.0], [0.3, 1.0]] {
                            let mut view = from;
                            let around = [corner[0] * held[0] * view.zoom, corner[1] * held[1] * view.zoom];
                            let point =
                                |view: &View| [view.right + around[0] / view.zoom, view.down + around[1] / view.zoom];
                            for _ in 0..6 {
                                let (was, before) = (view, point(&view));
                                view.zoomed(1.9, around, size, reach, covering);
                                let now = point(&view);
                                // A side of the view that was longer than the territory's had it in its middle.
                                let fitted = was.held(size, covering);
                                for side in [0, 1] {
                                    assert!(
                                        fitted[side] > territory[side] || (now[side] - before[side]).abs() <= fine,
                                        "{name} {size:?}: the point {before:?} is at {now:?} at {} times",
                                        view.zoom
                                    );
                                }
                                assert!(inside(&view, size, reach.0, covering));
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn a_view_made_smaller_moves_no_further_than_the_territory_asks() {
        for (name, reach, covering) in kinds_of_picture() {
            for size in SHAPES {
                let territory = territory(size, reach.0, covering);
                for from in scales(reach) {
                    for place in places() {
                        let mut view = View {
                            right: place[0] * territory[0],
                            down: place[1] * territory[1],
                            zoom: from,
                        };
                        view.limited(size, reach.0, covering);
                        for around in [[0.0, 0.0], [0.4, -0.3]] {
                            let mut free = view;
                            free.magnified(0.7, around, reach.0, reach.1);
                            let mut after = view;
                            after.zoomed(0.7, around, size, reach, covering);
                            assert_eq!(after.zoom, free.zoom);
                            assert!(inside(&after, size, reach.0, covering), "{name} {size:?} {after:?}");
                            let held = after.held(size, covering);
                            for (side, at, asked) in [(0, after.right, free.right), (1, after.down, free.down)] {
                                let room = (territory[side] - held[side]).max(0.0);
                                let over = (asked.abs() - room).max(0.0);
                                assert!(
                                    ((at - asked).abs() - over).abs() <= 1e-9 * territory[side],
                                    "{name} {size:?}: moved from {asked} to {at}, {over} over"
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn a_view_moved_aside_and_made_larger_stays_where_it_was_moved() {
        let (size, reach) = ([1920, 1080], (FLOOR, FLAME_DEEPEST.1));
        // Half the scale, dragged right as far as it went before the territory: 0.8889 shorter sides.
        let mut view = View {
            right: 0.5 * (1.0 / 0.5 - 1.0) * 1920.0 / 1080.0,
            down: 0.0,
            zoom: 0.5,
        };
        let aside = view.right;
        while view.zoom < reach.1 {
            view.zoomed(1.25, [0.0; 2], size, reach, None);
            assert_eq!(view.right, aside, "at {} times", view.zoom);
        }
        // And back out: the camera moves only once the view would pass the territory's edge.
        let edge = territory(size, FLOOR, None)[0];
        while view.zoom > FLOOR {
            view.zoomed(0.8, [0.0; 2], size, reach, None);
            let held = view.held(size, None)[0];
            assert!(
                (view.right - aside.min(edge - held)).abs() < 1e-12,
                "at {} times",
                view.zoom
            );
        }
        assert!((view.right - aside).abs() < 1e-12);
    }

    #[test]
    fn a_kept_view_outside_the_territory_opens_at_its_edge() {
        let size = [1920, 1080];
        for (name, (least, most), covering) in kinds_of_picture() {
            let territory = territory(size, least, covering);
            for zoom in scales((least, most)) {
                let (_, kept) = saved_look(None, Some(&[40.0, -0.01, zoom]));
                let mut view = kept.expect("the view is kept");
                view.limited(size, least, covering);
                assert!(inside(&view, size, least, covering), "{name} {zoom}");
                let held = view.held(size, covering);
                assert!((view.right - (territory[0] - held[0]).max(0.0)).abs() <= 1e-9 * territory[0]);
                if held[1] + 0.01 <= territory[1] {
                    assert_eq!(view.down, -0.01, "{name} {zoom}");
                }
            }
            // A window of another shape has another territory, and the view comes inside that one.
            let mut view = View {
                right: territory[0],
                down: territory[1],
                zoom: most,
            };
            view.limited(size, least, covering);
            let upright = [1080, 1920];
            view.limited(upright, least, covering);
            assert!(inside(&view, upright, least, covering), "{name}");
        }
    }

    #[test]
    fn a_flame_goes_as_deep_as_a_view_holds_enough_of_it() {
        let [width, height] = DEPTH_SIZE.map(|side| side as usize);
        let level = |light: f64| (fractique_core::math::ln(1.0 + light) / 8.0) as f32;
        let grid = |light: &dyn Fn(usize, usize) -> f64| -> Vec<f32> {
            (0..width * height)
                .flat_map(|at| [0.0, 0.0, 0.0, level(light(at % width, at / width))])
                .collect()
        };
        let step = |step: i32| FLAME_DEEPEST.0 * fractique_core::math::pow(30.0, f64::from(step) / 20.0);
        // Light spread evenly: a view magnified z times holds 1 / z² of it, enough up to 22.6 times.
        let even = flame_deepest(&grid(&|_, _| 1.0), [width, height]);
        assert!((even - step(4)).abs() < 1e-9 && even < 22.63 && step(5) > 22.63);
        // All of it in one spot: every view of that spot holds enough.
        let spot = flame_deepest(
            &grid(&|x, y| if (x, y) == (200, 100) { 1e6 } else { 0.0 }),
            [width, height],
        );
        assert_eq!(spot, FLAME_DEEPEST.1);
        // A knot that holds a hundredth of the light in a 60th of the picture's side, on an even ground.
        let knot = |x: usize, y: usize| {
            if (200..208).contains(&x) && (100..104).contains(&y) {
                41.0
            } else {
                1.0
            }
        };
        let knotted = flame_deepest(&grid(&knot), [width, height]);
        assert!(knotted > even && knotted < FLAME_DEEPEST.1, "{knotted}");
        assert_eq!(flame_deepest(&grid(&|_, _| 0.0), [width, height]), FLAME_DEEPEST.0);
        assert_eq!(orbit_deepest("buddhabrot"), Some(10.0));
        assert_eq!(orbit_deepest("lorenz"), Some(100.0));
        assert_eq!(orbit_deepest("mandelbrot"), None);
    }

    #[test]
    fn a_view_has_one_scale() {
        // The detail line says the wheel's scale and the controls' as one number.
        assert_eq!(
            status_of([1280, 800], 0.5, 0.5 * 0.4),
            "1280 × 800  ·  0.50 s  ·  0.20×"
        );
        assert_eq!(status_of([1280, 800], 0.5, 1.0), "1280 × 800  ·  0.50 s");
        let labels = [
            (0.2, "0.20"),
            (2.0, "2.00"),
            (9.996, "10.0"),
            (13.72, "13.7"),
            (99.96, "100"),
            (1373.13, "1373"),
            (999_999.0, "999999"),
            (999_999.6, "1.0e6"),
            (9_523_809_523.81, "9.5e9"),
        ];
        for (scale, label) in labels {
            assert_eq!(scale_label(scale), label);
        }
    }

    #[test]
    fn a_picture_keeps_its_scale_while_the_window_changes_size() {
        let screen = |[width, height]: [u32; 2]| egui::vec2(width as f32, height as f32);
        let same = [1280, 800];
        assert_eq!(laid(same, same, false, screen(same)), (1.0, 1.0));
        // Narrower with the same shorter side: the picture is cut at its sides, not squeezed.
        let narrow = [1000, 800];
        let (stretch, closer) = laid(same, narrow, false, screen(narrow));
        assert!((stretch - 1.28).abs() < 1e-4 && (closer - 1.28).abs() < 1e-4);
        // Wider: as large as it was, with room beside it.
        let wide = [1600, 800];
        let (stretch, closer) = laid(same, wide, false, screen(wide));
        assert!((stretch - 0.8).abs() < 1e-4 && (closer - 1.0).abs() < 1e-4);
        // A shorter side half as long shows the picture half as large.
        let low = [1280, 400];
        let (stretch, closer) = laid(same, low, false, screen(low));
        assert!((stretch - 0.5).abs() < 1e-4 && (closer - 1.0).abs() < 1e-4);
        // A picture of a chosen size is fitted whole into any window.
        let (stretch, closer) = laid([1920, 1080], narrow, true, screen(narrow));
        assert!((stretch - (16.0 / 9.0) / 1.25).abs() < 1e-4 && closer == 1.0);
    }

    #[test]
    fn a_long_list_on_a_phone_leaves_the_same_edge_above_and_below() {
        for (wide, high) in [(360.0, 568.0), (360.0, 780.0), (430.0, 932.0), (780.0, 360.0)] {
            let content = egui::Rect::from_min_size(egui::pos2(0.0, 24.0), egui::vec2(wide, high));
            let (at, pivot, room) = list_place(content, true);
            assert_eq!((at, pivot), (content.center(), egui::Align2::CENTER_CENTER));
            let long = list_height(40, room) + LIST_FRAME;
            assert!((long - (high - 2.0 * widgets::OUTER)).abs() < 0.01, "{high}: {long}");
            let short = list_height(3, room) + LIST_FRAME;
            assert_eq!(short, 3.0 * 32.0 - 2.0 + LIST_FRAME);
        }
        for natural in [196.0, 260.0, 300.0, 380.0, 604.0] {
            let width = sheet_width(360.0, natural, true);
            assert_eq!(width + 2.0 * widgets::PAD + 2.0 * widgets::OUTER, 360.0, "{natural}");
            assert_eq!(sheet_width(1280.0, natural, false), natural);
        }
        assert_eq!(sheet_width(780.0, 260.0, true), 300.0);
        assert_eq!(sheet_width(780.0, 604.0, true), 604.0);
        let window = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 746.0));
        assert_eq!(
            list_place(window, false),
            (window.center(), egui::Align2::CENTER_CENTER, 746.0)
        );
    }

    #[test]
    fn a_layer_named_by_the_app_is_named_anew_in_another_language() {
        let (english, ukrainian) = (i18n::Texts::new(i18n::Lang::En), i18n::Texts::new(i18n::Lang::Uk));
        let titles = ["buddhabrot-classic".to_owned(), "Random 354".to_owned()];
        let again = |name: &str| renamed(name, &ukrainian, &english, &titles);
        assert_eq!(
            again(&displayed(&ukrainian, "buddhabrot-classic")),
            Some(displayed(&english, "buddhabrot-classic"))
        );
        assert_eq!(again(&displayed(&ukrainian, "Random 354")), Some("Random".to_owned()));
        assert_eq!(
            again(&displayed(&ukrainian, "aurora-spiral")),
            Some(displayed(&english, "aurora-spiral"))
        );
        let second = format!("{} 2", displayed(&ukrainian, "aurora-spiral"));
        assert_eq!(
            again(&second),
            Some(format!("{} 2", displayed(&english, "aurora-spiral")))
        );
        assert_eq!(again(&ukrainian.get("f_julia")), Some(english.get("f_julia")));
        assert_eq!(again("Мій шар"), None);
        assert_eq!(again(""), None);
        let back = renamed("Random", &english, &ukrainian, &titles);
        assert_eq!(back, Some(displayed(&ukrainian, "Random 354")));
    }

    #[test]
    fn a_scale_saved_by_the_old_slider_is_that_picture_s_view_and_no_setting() {
        let (adjust, view) = saved_look(Some(&[1.2, 0.9, 1.5, 0.4]), Some(&[0.3, -0.2, 2.0]));
        let adjust = adjust.expect("the tone is kept");
        assert_eq!((adjust.brightness, adjust.gamma, adjust.glow), (1.2, 0.9, 1.5));
        assert_eq!(adjust.zoom, 1.0);
        let view = view.expect("the view is kept");
        assert!((view.zoom - 0.8).abs() < 1e-12);
        assert!((view.right - 0.75).abs() < 1e-12 && (view.down + 0.5).abs() < 1e-12);
        let (adjust, view) = saved_look(Some(&[1.0, 1.0, 1.0]), Some(&[0.0, 0.0, 3.0]));
        assert_eq!(adjust.map(|adjust| adjust.zoom), Some(1.0));
        assert_eq!(view, Some(View { zoom: 3.0, ..WHOLE }));
        assert_eq!(saved_look(None, Some(&[0.0, 0.0, 1e40])).1, None);
        assert!(saved_look(Some(&[1.0]), None).0.is_none());
    }

    #[test]
    fn a_session_opens_at_its_whole_view_and_a_document_as_it_was_saved() {
        let old = serde_json::json!({ "adjust": [1.0, 1.0, 1.0, 0.4], "view": [0.0, 0.0, 1.0] });
        let numbers = |key: &str| -> Vec<f64> {
            old[key]
                .as_array()
                .expect("a list")
                .iter()
                .filter_map(serde_json::Value::as_f64)
                .collect()
        };
        let (adjust, view) = saved_look(Some(&numbers("adjust")), Some(&numbers("view")));
        // The picture of the document is the one that was saved: 0.4 of its framing.
        assert!((view.expect("a view").zoom - 0.4).abs() < 1e-12);
        // What the next picture is scaled by is 1 again.
        assert_eq!(adjust.expect("a tone").zoom, 1.0);
    }

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
    fn a_picture_asked_for_during_the_trial_has_the_card() {
        let (trial, picture, draft) = ([TRIAL.to_owned()], ["a flame".to_owned()], ["a draft".to_owned()]);
        assert_eq!(turn(Some(&trial), None), Turn::Trial);
        assert_eq!(turn(Some(&picture), None), Turn::Picture);
        assert_eq!(turn(Some(&picture), Some(&draft)), Turn::Picture);
        assert_eq!(turn(Some(&draft), Some(&draft)), Turn::Draft);
        assert_eq!(turn(None, None), Turn::Nobody);
    }

    #[test]
    fn a_verdict_is_kept_for_its_card_and_version() {
        let card = "Adreno 610|V@0502.0|0.1.0";
        assert_eq!(verdict_of("Adreno 610|V@0502.0|0.1.0|slow", card), Verdict::Unknown);
        assert_eq!(verdict_of("Adreno 610|V@0502.0|0.1.0|slow2", card), Verdict::Unknown);
        assert_eq!(verdict_of("Adreno 610|V@0502.0|0.1.0|slow3", card), Verdict::Slow);
        assert_eq!(
            verdict_of("Adreno 610|V@0502.0|0.1.0|fast|1500000.5", card),
            Verdict::Fast(Some(1_500_000.5))
        );
        assert_eq!(verdict_of("Adreno 610|V@0502.0|0.0.9|slow", card), Verdict::Unknown);
        assert_eq!(verdict_of("Mali-G52|r1|0.1.0|fast|9", card), Verdict::Unknown);
        assert_eq!(verdict_of("Adreno 610|V@0502.0|0.1.0|fast|", card), Verdict::Unknown);
    }

    #[test]
    fn a_fractal_is_named_in_the_language_and_its_file_in_english() {
        let english = i18n::Texts::new(i18n::Lang::En);
        let ukrainian = i18n::Texts::new(i18n::Lang::Uk);
        for spec in fractal::all() {
            for preset in spec.presets {
                let key = format!("n_{}_{}", spec.name, preset.name);
                assert_ne!(english.get(&key), key);
                let title = format!("{}-{}", spec.name, preset.name);
                assert_eq!(displayed(&ukrainian, &shown(&title)), displayed(&ukrainian, &title));
                assert_ne!(displayed(&ukrainian, &title), shown(&title));
            }
        }
        assert_eq!(displayed(&english, "mandelbox-station"), "Mandelbox Station");
        assert_eq!(file_name("mandelbox-station"), "MandelboxStation");
        assert_eq!(
            displayed(&ukrainian, "burning_ship 42"),
            ukrainian.get("f_burning_ship")
        );
        assert_eq!(file_name("burning_ship 42"), "BurningShip42");
        assert_eq!(
            displayed(&ukrainian, "Gumowski-Mira 7"),
            displayed(&ukrainian, "gumowski_mira 7")
        );
        for name in presets::names() {
            assert_eq!(displayed(&english, name), shown(name));
            assert_ne!(displayed(&ukrainian, name), shown(name), "{name}");
            assert_eq!(displayed(&ukrainian, &shown(name)), displayed(&ukrainian, name));
        }
        assert_eq!(file_name("aurora-spiral"), "AuroraSpiral");
        assert_eq!(displayed(&ukrainian, "my scene"), "My scene");
    }

    #[test]
    fn a_seed_is_kept_in_the_title_and_the_file_name_and_not_shown() {
        let english = i18n::Texts::new(i18n::Lang::En);
        let ukrainian = i18n::Texts::new(i18n::Lang::Uk);
        for (title, file) in [
            ("Random 12", "Random12"),
            ("Випадковий 354", "Випадковий354"),
            ("random 7", "Random7"),
        ] {
            assert_eq!(displayed(&ukrainian, title), "Випадковий");
            assert_eq!(displayed(&english, title), "Random");
            assert_eq!(file_name(title), file);
        }
        for (title, file) in [("Шпалери 40", "Шпалери40"), ("wallpaper 57", "Wallpaper57")] {
            assert_eq!(displayed(&ukrainian, title), "Шпалери");
            assert_eq!(displayed(&english, title), "Wallpaper");
            assert_eq!(file_name(title), file);
        }
        for title in ["Mandelbrot 42", "mandelbrot 42", "Mandelbrot42"] {
            assert_eq!(displayed(&ukrainian, title), ukrainian.get("f_mandelbrot"));
            assert_eq!(displayed(&english, title), "Mandelbrot");
            assert_eq!(file_name(title), "Mandelbrot42");
        }
        assert_eq!(displayed(&english, "Burning ship 42"), english.get("f_burning_ship"));
        assert_eq!(displayed(&english, "My scene 2"), "My scene 2");
        assert_eq!(displayed(&english, "Random"), "Random");
        for lang in i18n::Lang::ALL {
            let texts = i18n::Texts::new(lang);
            for key in ["random", "wallpaper"] {
                let word = shown(&texts.get(key));
                assert_eq!(
                    displayed(&english, &format!("{word} 9")),
                    shown(&english.get(key)),
                    "{word}"
                );
                assert_eq!(displayed(&texts, &format!("{word} 9")), word);
            }
        }
    }

    // The point of a held picture that glass.wgsl samples at a point of the window, in the shader's own 32 bits.
    fn sampled(lay: [f32; 4], at: [f32; 2]) -> [f32; 2] {
        let mut from_middle = [at[0] - 0.5, at[1] - 0.5];
        if lay[3] > 1.0 {
            from_middle[1] *= lay[3];
        } else {
            from_middle[0] /= lay[3];
        }
        [from_middle[0] / lay[2] + lay[0], from_middle[1] / lay[2] + lay[1]]
    }

    // The same point as the view itself has it there, in 64 bits.
    fn meant(eye: (View, f64), held: &Picture, at: [f32; 2]) -> [f64; 2] {
        let (view, shown) = (eye.0.scaled(eye.1), held.view.scaled(held.zoom));
        let shorter = f64::from(held.size[0].min(held.size[1]));
        let middle = [view.right - shown.right, view.down - shown.down];
        [0, 1].map(|side| {
            let across = f64::from(held.size[side]) / shorter;
            let scene = middle[side] + (f64::from(at[side]) - 0.5) * across / view.zoom;
            0.5 + scene * shown.zoom / across / held.wide[side]
        })
    }

    const CORNERS: [[f32; 2]; 5] = [[0.5, 0.5], [0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]];

    // How far the held picture is from where the view has it: in the picture's own sides, and in pixels of the
    // window.
    fn astray(eye: (View, f64), held: &Picture) -> (f64, f64) {
        let screen = egui::vec2(held.size[0] as f32, held.size[1] as f32);
        let lay = lain(eye, held, true, held.size, false, screen);
        let larger = eye.0.scaled(eye.1).zoom / held.view.scaled(held.zoom).zoom;
        let mut worst = (0.0, 0.0);
        for at in CORNERS {
            let (drawn, exact) = (sampled(lay, at), meant(eye, held, at));
            for side in 0..2 {
                let off = (f64::from(drawn[side]) - exact[side]).abs();
                let pixels = off * held.wide[side] * larger * f64::from(held.size[side]);
                worst = (
                    f64::max(worst.0, off / exact[side].abs().max(1.0)),
                    f64::max(worst.1, pixels),
                );
            }
        }
        worst
    }

    // A wheel turned fast and a hand that drags meanwhile: every view from where it starts to the limit.
    fn fast_zoom(from: View, by: f64, size: [u32; 2], reach: (f64, f64), covering: Option<[f64; 2]>) -> Vec<View> {
        let (mut view, mut views) = (from, vec![from]);
        let end = if by > 1.0 { reach.1 } else { reach.0 };
        for step in 0..2000 {
            if view.zoom == end {
                break;
            }
            view.zoomed(by, [0.21 / view.zoom, -0.13 / view.zoom], size, reach, covering);
            if step % 5 == 4 {
                view.moved([0.04, -0.03]);
                view.limited(size, reach.0, covering);
            }
            views.push(view);
        }
        assert_eq!(view.zoom, end);
        views
    }

    // One bit of the shader's numbers at a point of the picture, with room for the few operations on the way.
    const BITS: f64 = 8.0 * f32::EPSILON as f64;

    #[test]
    fn a_held_picture_stands_for_every_view_of_a_fast_zoom() {
        for (kind, reach, covering) in kinds_of_picture() {
            for size in SHAPES {
                for wide in [[1.0; 2], [1.5, 1.5], [1.25, 1.6]] {
                    let whole = View {
                        zoom: reach.0.max(1.0),
                        ..WHOLE
                    };
                    let held = Picture {
                        view: whole,
                        zoom: 1.0,
                        size,
                        wide,
                    };
                    let inward = fast_zoom(whole, 1.19, size, reach, covering);
                    let deepest = *inward.last().unwrap_or(&whole);
                    let back = Picture { view: deepest, ..held };
                    let outward = fast_zoom(deepest, 0.83, size, reach, covering);
                    for (held, views) in [(held, &inward), (back, &outward)] {
                        for view in views {
                            let eye = (*view, 1.0);
                            let larger = stand_in(eye, &held, true)[2];
                            let exact = (view.zoom / held.view.zoom) as f32;
                            assert_eq!(larger, exact, "{kind} at {}", view.zoom);
                            let (off, pixels) = astray(eye, &held);
                            if view.zoom >= held.view.zoom {
                                assert!(off <= BITS, "{kind} at {}: {off} of the picture", view.zoom);
                            }
                            if view.zoom <= held.view.zoom || reach.1 <= FLAME_DEEPEST.1 / 0.5 {
                                assert!(pixels < 1.0, "{kind} at {}: {pixels} pixels", view.zoom);
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn a_stand_in_goes_as_far_in_as_the_view() {
        let size = [3840, 2160];
        let held = Picture {
            view: View { zoom: 2.2, ..WHOLE },
            zoom: 1.0,
            size,
            wide: [1.5; 2],
        };
        let view = View {
            zoom: 300.0,
            right: 0.01,
            down: 0.02,
        };
        assert_eq!(stand_in((view, 1.0), &held, true)[2], (300.0 / 2.2) as f32);
        // The scale of the controls is the picture's too.
        let under = View { zoom: 150.0, ..view };
        assert_eq!(stand_in((under, 2.0), &held, true)[2], (300.0 / 2.2) as f32);
        // Past what 32 bits tell apart in the picture, the window is one point of it: the colour there.
        let deep = View {
            zoom: 1e10,
            right: 0.3,
            down: -0.2,
        };
        let whole = Picture {
            view: WHOLE,
            zoom: 1.0,
            size,
            wide: [1.0; 2],
        };
        let lay = lain((deep, 1.0), &whole, true, size, false, egui::vec2(3840.0, 2160.0));
        assert_eq!(lay[2], 1e10);
        assert_eq!(sampled(lay, [0.0, 0.0]), sampled(lay, [1.0, 1.0]));
        assert!(astray((deep, 1.0), &whole).0 <= BITS);
        let nowhere = Picture {
            view: View { zoom: 0.0, ..WHOLE },
            ..whole
        };
        assert_eq!(stand_in((deep, 1.0), &nowhere, false)[2], 1.0);
    }

    #[test]
    fn a_picture_that_arrives_during_a_zoom_takes_the_screen_where_the_one_before_stood() {
        for (kind, reach, covering) in kinds_of_picture() {
            let size = [3840, 2160];
            let whole = View {
                zoom: reach.0.max(1.0),
                ..WHOLE
            };
            let inward = fast_zoom(whole, 1.19, size, reach, covering);
            let mut views = inward.clone();
            views.extend(fast_zoom(*inward.last().unwrap_or(&whole), 0.83, size, reach, covering));
            let mut held = Picture {
                view: whole,
                zoom: 1.0,
                size,
                wide: [1.5; 2],
            };
            for (step, view) in views.iter().enumerate() {
                let eye = (*view, 1.0);
                // The picture of a view a few steps back, as one that was drawn meanwhile is.
                let arrived = Picture {
                    view: views[step.saturating_sub(3)],
                    zoom: 1.0,
                    size,
                    wide: [1.25; 2],
                };
                for at in CORNERS {
                    let lays = [held, arrived].map(|picture| {
                        let lay = lain(eye, &picture, true, size, false, egui::vec2(3840.0, 2160.0));
                        (sampled(lay, at), meant(eye, &picture, at))
                    });
                    for (drawn, exact) in lays {
                        for side in 0..2 {
                            let off = (f64::from(drawn[side]) - exact[side]).abs() / exact[side].abs().max(1.0);
                            assert!(off <= BITS, "{kind} at {}: {off}", view.zoom);
                        }
                    }
                }
                let (before, after) = (stand_in(eye, &held, true)[2], stand_in(eye, &arrived, true)[2]);
                let same = f64::from(before) * held.view.zoom / (f64::from(after) * arrived.view.zoom);
                assert!((same - 1.0).abs() < 1e-6, "{kind} at {}: {same}", view.zoom);
                if step % 11 == 10 {
                    held = arrived;
                }
            }
        }
    }

    #[test]
    fn a_picture_with_a_margin_lies_where_its_view_lies() {
        // glass.wgsl: the point of the picture at a point of the window, from the window's middle.
        let seen = |[across, along, larger, stretch]: [f32; 4], [x, y]: [f32; 2]| {
            let (x, y) = if stretch > 1.0 {
                (x, y * stretch)
            } else {
                (x / stretch, y)
            };
            [x / larger + across, y / larger + along]
        };
        for lay in [[0.5, 0.5, 1.0, 1.0], [0.62, 0.41, 0.7, 0.955], [0.3, 0.8, 2.5, 1.3]] {
            for wide in [[1.5, 1.5], [2.0, 2.0], [1.25, 1.2504], [1.2504, 1.25]] {
                let laid = spread(lay, wide);
                for at in [[0.0, 0.0], [0.5, -0.5], [-0.31, 0.22]] {
                    let (view, picture) = (seen(lay, at), seen(laid, at));
                    for side in 0..2 {
                        let expected = 0.5 + (view[side] - 0.5) / wide[side] as f32;
                        assert!((picture[side] - expected).abs() < 1e-5, "{lay:?} {wide:?} {at:?}");
                    }
                }
            }
        }
        assert_eq!(spread([0.4, 0.6, 3.0, 0.9], [1.0; 2]), [0.4, 0.6, 3.0, 0.9]);
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
        let (first, _, _) = rendered(&scene, &options, &control, &kept, true, false).expect("it renders");
        let (again, _, _) = rendered(&scene, &options, &control, &kept, true, false).expect("it renders");
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
    fn a_heavy_kind_is_a_kind_the_card_does_not_draw() {
        for kind in HEAVY {
            let scene = fractal::random_scene(kind, 4).unwrap_or_else(|| panic!("{kind} is not a kind"));
            assert!(!on_card(&scene), "{kind}");
        }
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
            let (first, held, _) = rendered(&scene, &options, &control, &kept, true, false).expect("it renders");
            assert_eq!(first.rgba, whole.rgba, "{}", spec.name);
            assert!(held.is_none());

            let stop = AtomicBool::new(true);
            let stopped = Control {
                cancel: Some(&stop),
                progress: None,
            };
            let mut dimmer = scene.clone();
            dimmer.layers[1].opacity = 0.5;
            let (again, _, _) = rendered(&dimmer, &options, &stopped, &kept, true, false).expect("nothing is rendered");
            let expected = fractique_core::render(&dimmer, &options, &control).expect("it renders");
            assert_eq!(again.rgba, expected.rgba, "{}", spec.name);
        }
    }

    #[test]
    fn the_work_of_a_picture_is_its_samples_or_its_steps() {
        let options = RenderOptions {
            width: 200,
            height: 100,
            samples_per_pixel: 32,
            supersample: 1,
        };
        let flame = presets::get("aurora-spiral").expect("a preset");
        let work = parts(&flame, &options);
        assert_eq!(
            (work[0].kind, work[0].work, work[0].stepped),
            (pace::FLAME, 640_000.0, false)
        );
        let set = fractal::preset("mandelbrot-whole").expect("a preset");
        let work = parts(&set, &options);
        let whole = 20_000.0 * 4.0 * 300.0;
        assert_eq!((work[0].kind, work[0].stepped), ("mandelbrot", true));
        assert!((whole..whole * 1.2).contains(&work[0].work), "{}", work[0].work);
        let mut deeper = set.clone();
        deeper.layers[0].camera_mut().zoom *= 1e4;
        assert!(parts(&deeper, &options)[0].work > 2.5 * whole);
        let solid = fractal::preset("mandelbulb-classic").expect("a preset");
        assert_eq!(parts(&solid, &options)[0].work, 20_000.0 * 10.0);
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
        let (first, held, _) = rendered(&scene, &options, &control, &kept, true, true).expect("it renders");
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
        let (toned, held, _) = rendered(&scene, &options, &stopped, &kept, true, true).expect("nothing is rendered");
        assert_eq!(
            toned.rgba,
            fractique_core::render(&scene, &options, &control).unwrap().rgba
        );
        let (again, same) = held.expect("every layer has its density");
        assert_eq!(again, keys);
        assert!(sources.iter().zip(&same).all(|(a, b)| Arc::ptr_eq(a, b)));
    }
}

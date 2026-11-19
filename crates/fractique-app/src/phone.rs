use std::collections::{HashMap, HashSet};
use std::sync::mpsc;

use eframe::egui;
use egui::{Align2, Color32, CornerRadius, Pos2, Rect, Sense, Stroke, vec2};
use fractique_core::wallpaper::{Layout, Material, Request, chosen, random_wallpaper};
use fractique_core::{Control, RenderOptions, Scene, fractal, palette, presets, render};
use serde_json::{Value, json};

use crate::widgets::{self, MEDIUM, SEMIBOLD, font};
use crate::{App, WHOLE, named, shown};

pub const SWIPE: f32 = 56.0;
const TILE_RATIO: f32 = 1.6;
const MOST_KEPT: usize = 48;
// The recent pictures a tab keeps: each of the three has a list of its own.
const MOST_RECENT: usize = 20;
// OKLCH hue in degrees, and the color the dot is drawn in.
const TINTS: [(f64, [u8; 3]); 8] = [
    (27.0, [216, 69, 47]),
    (55.0, [224, 138, 44]),
    (95.0, [217, 185, 46]),
    (148.0, [79, 168, 100]),
    (195.0, [47, 167, 168]),
    (258.0, [63, 127, 216]),
    (300.0, [138, 99, 210]),
    (350.0, [214, 92, 147]),
];
pub const HOLD: f64 = 0.5;
const SHEET_TOP: f32 = 0.3;
const SHEET_TALL: f32 = 0.06;

#[derive(Clone, PartialEq)]
pub enum Kind {
    Wall(u64, Layout, Material, bool, Option<f64>),
    Flame(Box<Scene>),
}

// A picture's place in the history: the pictures mutated from one another are a family, numbered as they were made.
#[derive(Clone, Copy, PartialEq, Default, Debug)]
pub struct Origin {
    pub family: u64,
    pub mutation: u32,
    pub made: Option<u64>,
}

#[derive(Clone, Copy, PartialEq, Default, Debug)]
pub enum Step {
    #[default]
    Adjusted,
    Fresh,
    Mutated,
    Picked,
    Visited,
}

#[derive(Clone, PartialEq)]
pub struct Piece {
    pub title: String,
    pub kind: Kind,
    pub origin: Origin,
}

impl Piece {
    pub fn tab(&self) -> usize {
        match &self.kind {
            Kind::Wall(..) => 2,
            Kind::Flame(scene) if scene.layers.iter().any(|layer| layer.fractal().is_some()) => 1,
            Kind::Flame(_) => 0,
        }
    }

    pub(crate) fn key(&self) -> String {
        use std::hash::{Hash, Hasher};
        match &self.kind {
            Kind::Wall(seed, layout, material, dark, hue) => {
                format!("w {seed} {layout:?} {material:?} {dark} {hue:?}")
            }
            Kind::Flame(scene) => {
                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                scene.to_json().hash(&mut hasher);
                format!("f {:016x}", hasher.finish())
            }
        }
    }

    pub(crate) fn scene(&self, ratio: f64) -> Scene {
        match &self.kind {
            Kind::Wall(seed, layout, material, dark, hue) => random_wallpaper(
                *seed,
                &Request {
                    layout: Some(*layout),
                    material: Some(*material),
                    ratio,
                    dark: *dark,
                    hue: *hue,
                },
            ),
            Kind::Flame(scene) => (**scene).clone(),
        }
    }

    fn to_value(&self) -> Value {
        let mut value = match &self.kind {
            Kind::Wall(seed, layout, material, dark, hue) => {
                let name = |all: &[(&'static str, bool)]| all.iter().find(|(_, is)| *is).map_or("", |(n, _)| n);
                json!({
                    "title": self.title,
                    "seed": seed,
                    "layout": name(&Layout::ALL.map(|(n, l)| (n, l == *layout))),
                    "material": name(&Material::ALL.map(|(n, m)| (n, m == *material))),
                    "dark": dark,
                    "hue": hue,
                })
            }
            Kind::Flame(scene) => json!({ "title": self.title, "scene": scene.to_json() }),
        };
        if self.origin != Origin::default() {
            value["family"] = json!(self.origin.family);
            value["mutation"] = json!(self.origin.mutation);
            value["made"] = json!(self.origin.made);
        }
        value
    }

    fn from_value(value: &Value) -> Option<Self> {
        let title = value.get("title")?.as_str()?.to_owned();
        let kind = match value.get("scene").and_then(Value::as_str) {
            Some(text) => Kind::Flame(Box::new(Scene::from_json(text).ok()?)),
            None => Kind::Wall(
                value.get("seed")?.as_u64()?,
                Layout::parse(value.get("layout")?.as_str()?)?,
                Material::parse(value.get("material")?.as_str()?)?,
                value.get("dark")?.as_bool()?,
                value.get("hue").and_then(Value::as_f64),
            ),
        };
        let number = |key: &str| value.get(key).and_then(Value::as_u64);
        let origin = Origin {
            family: number("family").unwrap_or(0),
            mutation: number("mutation")
                .and_then(|count| u32::try_from(count).ok())
                .unwrap_or(0),
            made: number("made"),
        };
        Some(Self { title, kind, origin })
    }
}

pub struct Phone {
    pub on: bool,
    pub sheet: bool,
    pub tune: bool,
    pub swipe: egui::Vec2,
    pub preview: bool,
    pub previewed: bool,
    pub strayed: bool,
    pub lift: f32,
    pub pull: f32,
    pub clear: Vec<Rect>,
    pub kept: Vec<Piece>,
    pub recent: Vec<Piece>,
    pub step: Step,
    pub origin: Origin,
    pub at: [usize; 3],
    thumbs: HashMap<String, egui::TextureHandle>,
    drawing: HashSet<String>,
    drawn: (mpsc::Sender<Thumb>, mpsc::Receiver<Thumb>),
    pub pressing: bool,
    pub share_from: Rect,
}

type Thumb = (String, [usize; 2], Vec<u8>);

// More at once would take the cores the picture itself is drawn with.
const DRAWN_AT_ONCE: usize = 2;

impl Phone {
    pub fn busy(&self) -> bool {
        !self.drawing.is_empty()
    }

    pub fn kept_text(&self) -> String {
        Value::Array(self.kept.iter().map(Piece::to_value).collect()).to_string()
    }

    pub fn recent_text(&self) -> String {
        Value::Array(self.recent.iter().map(Piece::to_value).collect()).to_string()
    }

    pub fn new(kept: Option<String>, recent: Option<String>) -> Self {
        let pieces = |text: Option<String>| -> Vec<Piece> {
            text.and_then(|text| serde_json::from_str::<Value>(&text).ok())
                .and_then(|value| value.as_array().cloned())
                .map(|list| list.iter().filter_map(Piece::from_value).collect())
                .unwrap_or_default()
        };
        let (kept, recent) = (pieces(kept), pieces(recent));
        let show = std::env::var("FRACTIQUE_SHOW").unwrap_or_default();
        Self {
            on: cfg!(any(target_os = "android", target_os = "ios")) || show.contains("phone"),
            sheet: show.contains("sheet"),
            tune: show.contains("tune"),
            swipe: egui::Vec2::ZERO,
            preview: false,
            previewed: false,
            strayed: false,
            lift: SHEET_TOP,
            pull: 0.0,
            clear: Vec::new(),
            kept,
            recent,
            step: Step::Adjusted,
            origin: Origin::default(),
            at: [0; 3],
            thumbs: HashMap::new(),
            drawing: HashSet::new(),
            drawn: mpsc::channel(),
            pressing: false,
            share_from: Rect::ZERO,
        }
    }
}

// One thread: a sheet of small pictures must not take the cores the window and the picture itself are drawn with.
#[cfg(not(target_arch = "wasm32"))]
fn thumb_pool() -> &'static rayon::ThreadPool {
    static POOL: std::sync::OnceLock<rayon::ThreadPool> = std::sync::OnceLock::new();
    POOL.get_or_init(|| {
        rayon::ThreadPoolBuilder::new()
            .num_threads(1)
            .build()
            .expect("the thread of the small pictures starts")
    })
}

// A small picture is kept as a file on a phone: drawing a sheet of them anew at every start takes a slow phone
// most of a minute.
fn thumb_file(key: &str, pixels: [usize; 2]) -> Option<std::path::PathBuf> {
    use std::hash::{Hash, Hasher};
    #[cfg(target_os = "android")]
    let folder = crate::own_folder().map(|folder| folder.join("cache"));
    #[cfg(target_os = "ios")]
    let folder = std::env::var_os("HOME").map(|home| std::path::PathBuf::from(home).join("Library/Caches"));
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    let folder: Option<std::path::PathBuf> = None;
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    key.hash(&mut hasher);
    let name = format!("{:016x}-{}x{}.png", hasher.finish(), pixels[0], pixels[1]);
    Some(folder?.join(concat!("thumbs-", env!("CARGO_PKG_VERSION"))).join(name))
}

fn stored_thumb(path: &std::path::Path, pixels: [usize; 2]) -> Option<Vec<u8>> {
    let mut reader = png::Decoder::new(std::io::BufReader::new(std::fs::File::open(path).ok()?))
        .read_info()
        .ok()?;
    let mut rgba = vec![0; reader.output_buffer_size()?];
    let frame = reader.next_frame(&mut rgba).ok()?;
    let fits = [frame.width as usize, frame.height as usize] == pixels
        && frame.color_type == png::ColorType::Rgba
        && frame.bit_depth == png::BitDepth::Eight;
    fits.then_some(rgba)
}

// Four across on an upright phone; more on one lying on its side, where four would be taller than the screen.
fn tile_width(room: f32, gap: f32) -> f32 {
    let across = (room / 104.0).floor().max(4.0);
    ((room - (across - 1.0) * gap) / across).floor()
}

#[derive(Clone, Copy)]
enum Icon {
    New,
    Back,
    Keep(bool),
    Mutate,
    Style,
    Save,
}

fn icon(painter: &egui::Painter, at: Pos2, which: Icon, ink: Color32, scale: f32) {
    let stroke = Stroke::new(1.6 * scale.max(0.7), ink);
    let v = |x: f32, y: f32| vec2(x, y) * scale;
    let capped = |from: Pos2, to: Pos2, width: f32| {
        painter.line_segment([from, to], Stroke::new(width, ink));
        painter.circle_filled(from, width * 0.5, ink);
        painter.circle_filled(to, width * 0.5, ink);
    };
    match which {
        Icon::New => {
            let width = 2.0 * scale.max(0.7);
            let (reach, across, bend) = (16.0, 16.0 * 0.866, 3.0);
            let corners = [
                (0.0, -reach),
                (across, -reach * 0.5),
                (across, reach * 0.5),
                (0.0, reach),
                (-across, reach * 0.5),
                (-across, -reach * 0.5),
            ]
            .map(|(x, y)| vec2(x, y));
            let mut outline = Vec::new();
            for (index, corner) in corners.into_iter().enumerate() {
                let from = corner + (corners[(index + 5) % 6] - corner).normalized() * bend;
                let to = corner + (corners[(index + 1) % 6] - corner).normalized() * bend;
                for step in 0..=4 {
                    let t = step as f32 / 4.0;
                    let point = from * (1.0 - t) * (1.0 - t) + corner * 2.0 * t * (1.0 - t) + to * t * t;
                    outline.push(at + point * scale);
                }
            }
            painter.add(egui::Shape::closed_line(outline, Stroke::new(width, ink)));
            for index in [1, 3, 5] {
                capped(at, at + corners[index] * (0.95 * scale), width);
            }
            let (left, right) = (vec2(-across * 0.5, reach * 0.25), vec2(across * 0.5, reach * 0.25));
            let (two, three) = (vec2(across * 0.22, reach * 0.3), vec2(across * 0.25, -reach * 0.33));
            painter.circle_filled(at + v(0.0, -reach * 0.5), 1.7 * scale, ink);
            for pip in [left - two, left + two, right - three, right, right + three] {
                painter.circle_filled(at + pip * scale, 1.5 * scale, ink);
            }
        }
        Icon::Back => {
            painter.line_segment([at + v(-8.0, 0.0), at + v(8.0, 0.0)], stroke);
            painter.line_segment([at + v(-8.0, 0.0), at + v(-2.0, -6.0)], stroke);
            painter.line_segment([at + v(-8.0, 0.0), at + v(-2.0, 6.0)], stroke);
        }
        Icon::Keep(full) => {
            let outline: Vec<Pos2> = (0..40)
                .map(|step| {
                    let t = step as f32 / 40.0 * std::f32::consts::TAU;
                    let (sin, cos) = t.sin_cos();
                    let x = 16.0 * sin * sin * sin;
                    let y = 13.0 * cos - 5.0 * (2.0 * t).cos() - 2.0 * (3.0 * t).cos() - (4.0 * t).cos();
                    at + v(x, -y * 0.92 + 1.0) * 0.52
                })
                .collect();
            if full {
                let mut mesh = egui::Mesh::default();
                mesh.colored_vertex(at + v(0.0, 1.0), ink);
                for point in &outline {
                    mesh.colored_vertex(*point, ink);
                }
                for index in 0..outline.len() as u32 {
                    mesh.add_triangle(0, 1 + index, 1 + (index + 1) % outline.len() as u32);
                }
                painter.add(egui::Shape::mesh(mesh));
            }
            painter.add(egui::Shape::closed_line(outline, stroke));
        }
        Icon::Mutate => {
            let scale = scale * 0.8;
            let v = |x: f32, y: f32| vec2(x, y) * scale;
            capped(at + v(-12.0, 12.0), at + v(1.5, -1.5), 2.0 * scale.max(0.7));
            let (center, reach) = (at + v(6.5, -6.5), 7.0 * scale);
            let thin = reach * 0.26;
            let points = [
                (0.0, -reach),
                (thin, -thin),
                (reach, 0.0),
                (thin, thin),
                (0.0, reach),
                (-thin, thin),
                (-reach, 0.0),
                (-thin, -thin),
            ]
            .map(|(x, y)| center + vec2(x, y));
            painter.add(egui::Shape::closed_line(points.to_vec(), stroke));
            for (x, y, reach) in [(-7.0, -8.0, 2.6), (11.0, 7.0, 2.2)] {
                let width = 1.4 * scale.max(0.7);
                capped(at + v(x - reach, y), at + v(x + reach, y), width);
                capped(at + v(x, y - reach), at + v(x, y + reach), width);
            }
        }
        Icon::Style => {
            for (y, knob) in [(-6.0, 4.0), (0.0, -4.0), (6.0, 2.0)] {
                painter.line_segment([at + v(-9.0, y), at + v(9.0, y)], stroke);
                painter.circle_filled(at + v(knob, y), 2.8 * scale, ink);
            }
        }
        Icon::Save => {
            painter.line_segment([at + v(0.0, -9.0), at + v(0.0, 3.0)], stroke);
            painter.line_segment([at + v(-5.0, -2.0), at + v(0.0, 3.0)], stroke);
            painter.line_segment([at + v(5.0, -2.0), at + v(0.0, 3.0)], stroke);
            painter.line_segment([at + v(-8.0, 8.0), at + v(8.0, 8.0)], stroke);
        }
    }
}

// The name on one or two lines, and the mutation as a tag of its own: it is no part of the name.
fn placard_lines(title: &str, mutation: u32, word: &str, turkish: bool) -> (String, String, String) {
    let (first, second) = match title.rsplit_once(' ') {
        Some((head, tail)) if title.chars().count() > 9 => (head.to_owned(), tail.to_owned()),
        _ => (title.to_owned(), String::new()),
    };
    let tag = match mutation {
        0 => String::new(),
        count if turkish => format!("{} {count}", word.replace('i', "İ").to_uppercase()),
        count => format!("{} {count}", word.to_uppercase()),
    };
    (first, second, tag)
}

struct Rail {
    shutter: Pos2,
    large: f32,
    small: f32,
    fan: [Pos2; 3],
    column: [Pos2; 2],
    scale: f32,
}

// A phone on its side: the largest are about 440 points high, less their bars.
const LOW: f32 = 420.0;
const LARGE: f32 = 68.0;
const SMALL: f32 = 46.0;
const CLEAR: f32 = 8.0;

// How far from the large button's centre the three small ones around it stand, clear of it and of one another,
// and how far along the large button's own edge the first and the last of them are from its centre.
fn reach(scale: f32) -> [f32; 2] {
    let aside = (LARGE - SMALL) * 0.5 * scale;
    let mut away = (LARGE + SMALL) * 0.5 * scale + CLEAR;
    let between = |away: f32| {
        let turn = std::f32::consts::FRAC_PI_4 + (aside / away).asin();
        2.0 * away * (turn * 0.5).sin()
    };
    while between(away) < SMALL * scale + CLEAR {
        away += 0.25;
    }
    [away, (away * away - aside * aside).sqrt()]
}

fn rail(content: Rect, top: f32) -> Rail {
    let tall = |scale: f32, gap: f32, foot: f32| {
        foot + LARGE * scale * 0.5 + reach(scale)[1] + SMALL * scale * 2.5 + 2.0 * gap
    };
    let room = content.bottom() - top;
    let (full, tight) = (tall(1.0, 10.0, 20.0), tall(1.0, 4.0, 8.0));
    let (scale, gap, foot) = if room >= tight {
        let loose = ((room - tight) / (full - tight)).min(1.0);
        (1.0, 4.0 + 6.0 * loose, 8.0 + 12.0 * loose)
    } else {
        let (mut low, mut high) = (0.25, 1.0);
        for _ in 0..20 {
            let middle = (low + high) * 0.5;
            if tall(middle, 4.0, 8.0) > room {
                high = middle;
            } else {
                low = middle;
            }
        }
        (low, 4.0, 8.0)
    };
    let (large, small, [away, along]) = (LARGE * scale, SMALL * scale, reach(scale));
    let shutter = egui::pos2(
        content.right() - 14.0 - large * 0.5,
        content.bottom() - foot - large * 0.5,
    );
    let (aside, slant) = ((large - small) * 0.5, away * std::f32::consts::FRAC_1_SQRT_2);
    let fan = [vec2(aside, -along), vec2(-slant, -slant), vec2(-along, aside)].map(|by| shutter + by);
    let style = fan[0].y - small - gap;
    Rail {
        shutter,
        large,
        small,
        fan,
        column: [style, style - small - gap].map(|y| egui::pos2(fan[0].x, y)),
        scale,
    }
}

pub(crate) fn now() -> u64 {
    web_time::SystemTime::now()
        .duration_since(web_time::UNIX_EPOCH)
        .map_or(0, |since| since.as_secs())
}

pub(crate) fn clock(made: u64, now: u64) -> String {
    use chrono::TimeZone;
    let local = |seconds: u64| chrono::Local.timestamp_opt(i64::try_from(seconds).ok()?, 0).single();
    match (local(made), local(now)) {
        (Some(made), Some(now)) if made.date_naive() == now.date_naive() => made.format("%H:%M").to_string(),
        (Some(made), _) => made.format("%d.%m %H:%M").to_string(),
        _ => String::new(),
    }
}

// The place the piece has among the recent pictures of its tab afterwards; `at` is the place of the picture that
// was on the screen before it.
fn note(recent: &mut Vec<Piece>, mut piece: Piece, step: Step, at: usize, now: u64) -> usize {
    let (tab, key) = (piece.tab(), piece.key());
    let rows: Vec<usize> = (0..recent.len()).filter(|&index| recent[index].tab() == tab).collect();
    let twin = rows.iter().position(|&index| recent[index].key() == key);
    let before = rows.get(at).copied().filter(|&index| {
        matches!((&piece.kind, &recent[index].kind), (Kind::Flame(_), Kind::Flame(_)))
            && recent[index].title == piece.title
    });
    let family = now.max(recent.iter().map(|other| other.origin.family + 1).max().unwrap_or(0));
    match (step, twin, before) {
        (Step::Adjusted | Step::Visited, Some(twin), _) => return twin,
        // A flame that is being adjusted is one picture, not one for every move of a slider.
        (Step::Adjusted, None, Some(before)) => {
            piece.origin = recent[before].origin;
            recent[before] = piece;
            return at;
        }
        (Step::Mutated, ..) => {
            if piece.origin.family == 0 {
                piece.origin.family = family;
                if let Some(before) = before {
                    recent[before].origin.family = family;
                }
            }
            let last = recent
                .iter()
                .filter(|other| other.origin.family == piece.origin.family)
                .map(|other| other.origin.mutation)
                .fold(piece.origin.mutation, u32::max);
            piece.origin.mutation = last + 1;
            piece.origin.made = Some(now);
        }
        (Step::Picked | Step::Visited, ..) => {
            piece.origin.made.get_or_insert(now);
        }
        (Step::Adjusted, ..) if piece.origin.family != 0 && matches!(piece.kind, Kind::Wall(..)) => {
            piece.origin.made = Some(now);
        }
        (Step::Fresh | Step::Adjusted, ..) => {
            piece.origin = Origin {
                family,
                mutation: 0,
                made: Some(now),
            };
        }
    }
    // A new picture made from further back is followed by the picture that was on the screen and those before
    // it, so that Back goes on from there; the newer ones that were stepped over come after them.
    if at > 0 && at < rows.len() && !matches!(step, Step::Picked | Step::Visited) {
        let mut mine: Vec<Piece> = rows.iter().map(|&index| recent[index].clone()).collect();
        mine.rotate_left(at);
        for (&index, piece) in rows.iter().zip(mine) {
            recent[index] = piece;
        }
    }
    recent.retain(|other| other.key() != key);
    recent.insert(0, piece);
    let mut kept = 0;
    recent.retain(|other| {
        kept += usize::from(other.tab() == tab);
        other.tab() != tab || kept <= MOST_RECENT
    });
    0
}

// A disc filled with a gradient from its upper left to its lower right, with a soft rim: rings of triangles,
// each corner colored by its place along that line, and a last ring that fades out.
fn gradient_disc(painter: &egui::Painter, at: Pos2, reach: f32, shade: impl Fn(f32) -> Color32) {
    const RINGS: u32 = 6;
    const AROUND: u32 = 48;
    let feather = 1.0 / painter.ctx().pixels_per_point();
    let along = vec2(1.0, 1.0).normalized();
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(at, shade(0.5));
    for ring in 1..=RINGS + 1 {
        let (radius, alpha) = if ring > RINGS {
            (reach + feather * 0.5, 0.0)
        } else {
            ((reach - feather * 0.5) * ring as f32 / RINGS as f32, 1.0)
        };
        for step in 0..AROUND {
            let angle = step as f32 / AROUND as f32 * std::f32::consts::TAU;
            let offset = vec2(angle.cos(), angle.sin()) * radius;
            let share = (offset.dot(along) / reach * 0.5 + 0.5).clamp(0.0, 1.0);
            mesh.colored_vertex(at + offset, shade(share).gamma_multiply(alpha));
        }
    }
    for step in 0..AROUND {
        mesh.add_triangle(0, 1 + step, 1 + (step + 1) % AROUND);
    }
    for ring in 0..RINGS {
        let (inner, outer) = (1 + ring * AROUND, 1 + (ring + 1) * AROUND);
        for step in 0..AROUND {
            let next = (step + 1) % AROUND;
            mesh.add_triangle(inner + step, outer + step, outer + next);
            mesh.add_triangle(inner + step, outer + next, inner + next);
        }
    }
    painter.add(egui::Shape::mesh(mesh));
}

type Panes = Vec<(Rect, f32, bool)>;

pub(crate) const LIGHT: f32 = 0.55;

impl App {
    fn ink(&self) -> Color32 {
        if self.light > LIGHT {
            Color32::from_rgb(16, 16, 22)
        } else {
            Color32::WHITE
        }
    }

    pub(crate) fn step_wallpaper(&mut self, forward: bool) {
        if forward {
            self.wall.held = None;
            self.new_wallpaper();
        } else if self.seed > 1 {
            self.seed -= 2;
            self.new_wallpaper();
        }
    }

    fn piece(&self, size: [u32; 2]) -> Piece {
        if self.wall.live {
            let (layout, material) = self.current(size);
            Piece {
                title: shown(&self.title),
                kind: Kind::Wall(self.seed, layout, material, self.wall.dark, self.wall.hue),
                origin: self.phone.origin,
            }
        } else {
            Piece {
                title: shown(&self.title),
                kind: Kind::Flame(Box::new(self.dressed())),
                origin: self.phone.origin,
            }
        }
    }

    pub(crate) fn remember(&mut self, size: [u32; 2]) {
        let piece = self.piece(size);
        let tab = piece.tab();
        let step = std::mem::take(&mut self.phone.step);
        let at = note(&mut self.phone.recent, piece, step, self.phone.at[tab], now());
        self.phone.at[tab] = at;
        if let Some(found) = self.phone.recent.iter().filter(|other| other.tab() == tab).nth(at) {
            self.phone.origin = found.origin;
        }
    }

    // The picture on the screen takes its place among the recent ones before another one replaces it.
    pub(crate) fn settle(&mut self) -> bool {
        match self.requested {
            Some(size) => self.remember(size),
            None if !self.wall.live => self.remember([1, 2]),
            None => return false,
        }
        true
    }

    pub(crate) fn begin(&mut self) {
        self.phone.step = Step::Fresh;
        self.phone.origin = Origin::default();
    }

    pub(crate) fn recent_here(&self) -> Vec<Piece> {
        let tab = self.tab();
        let mine = self.phone.recent.iter().filter(|piece| piece.tab() == tab);
        mine.cloned().collect()
    }

    pub(crate) fn has_older(&self) -> bool {
        let tab = self.tab();
        self.phone.recent.iter().filter(|piece| piece.tab() == tab).count() > self.phone.at[tab] + 1
    }

    // One step through the recent pictures of the tab, which stay in their order.
    pub(crate) fn travel(&mut self, back: bool) {
        self.settle();
        let tab = self.tab();
        let at = self.phone.at[tab];
        let to = if back { at + 1 } else { at.wrapping_sub(1) };
        let Some(piece) = self.recent_here().into_iter().nth(to) else {
            return;
        };
        self.show_piece(&piece);
        self.phone.step = Step::Visited;
        self.phone.at[tab] = to;
    }

    pub(crate) fn label(&self, title: &str, origin: Origin) -> String {
        let name = self.display(title);
        match origin.mutation {
            0 => name,
            count => format!("{name} · {} {count}", self.t("mutation")),
        }
    }

    fn tile_label(&self, piece: &Piece) -> String {
        match piece.origin.mutation {
            0 => self.display(&piece.title),
            count => format!("{} {count}", self.t("mutation")),
        }
    }

    pub(crate) fn show_piece(&mut self, piece: &Piece) {
        match &piece.kind {
            Kind::Wall(seed, layout, material, dark, hue) => {
                self.wall.hue = *hue;
                self.wall.on = true;
                self.wall.layout = None;
                self.wall.material = None;
                self.wall.held = Some((*layout, *material));
                self.wall.dark = *dark;
                self.seed = seed.saturating_sub(1);
                self.new_wallpaper();
            }
            Kind::Flame(scene) => {
                self.wall.on = false;
                self.follow(scene);
                self.palette = None;
                self.pick((**scene).clone(), piece.title.clone(), false);
            }
        }
        self.phone.step = Step::Picked;
        self.phone.origin = piece.origin;
    }

    // The picture as the lock screen of an iPhone shows it: the date, the time, the torch and the camera.
    pub(crate) fn clock(&self, ctx: &egui::Context, content: Rect, screen: Rect) -> [Rect; 2] {
        let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Middle, egui::Id::unique("phone_clock")));
        let ink = self.ink();
        let (date, time) = crate::lock_screen_texts();
        let middle = content.center().x;
        let day = font(21.0, SEMIBOLD);
        if !date.is_empty() && ctx.fonts_mut(|fonts| fonts.has_glyphs(&day, &date)) {
            painter.text(
                egui::pos2(middle, content.top() + 40.0),
                Align2::CENTER_CENTER,
                date,
                day,
                ink,
            );
        }
        painter.text(
            egui::pos2(middle, content.top() + 112.0),
            Align2::CENTER_CENTER,
            time,
            font(104.0, SEMIBOLD),
            ink,
        );

        let stroke = Stroke::new(1.7, ink);
        let low = screen.bottom() - 84.0;
        let torch = egui::pos2(screen.left() + 70.0, low);
        painter.add(egui::Shape::closed_line(
            [
                (-6.0, -11.0),
                (6.0, -11.0),
                (6.0, -6.5),
                (3.2, -3.0),
                (3.2, 11.0),
                (-3.2, 11.0),
                (-3.2, -3.0),
                (-6.0, -6.5),
            ]
            .map(|(x, y)| torch + vec2(x, y))
            .to_vec(),
            stroke,
        ));
        painter.line_segment([torch + vec2(-6.0, -7.5), torch + vec2(6.0, -7.5)], stroke);
        painter.circle_filled(torch + vec2(0.0, 2.5), 1.4, ink);
        let camera = egui::pos2(screen.right() - 70.0, low);
        painter.rect_stroke(
            Rect::from_center_size(camera + vec2(0.0, 1.0), vec2(24.0, 16.0)),
            CornerRadius::same(4),
            stroke,
            egui::StrokeKind::Middle,
        );
        painter.line_segment([camera + vec2(-4.5, -7.0), camera + vec2(-2.5, -10.0)], stroke);
        painter.line_segment([camera + vec2(-2.5, -10.0), camera + vec2(2.5, -10.0)], stroke);
        painter.line_segment([camera + vec2(2.5, -10.0), camera + vec2(4.5, -7.0)], stroke);
        painter.circle_stroke(camera + vec2(0.0, 1.0), 4.4, stroke);
        [torch, camera].map(|at| Rect::from_center_size(at, vec2(52.0, 52.0)))
    }

    // The colors to choose from: a hue for the shapes, a palette for the flames, or the picture's own.
    pub(crate) fn colors(&mut self, ui: &mut egui::Ui, dot: f32) {
        let walls = self.wall.on;
        let before = (self.wall.hue, self.palette);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(3.0, 6.0);
            let cell = |ui: &mut egui::Ui, chosen: bool, paint: &dyn Fn(&egui::Painter, Pos2, f32)| {
                let (rect, response) = ui.allocate_exact_size(vec2(dot + 8.0, dot + 8.0), Sense::click());
                paint(ui.painter(), rect.center(), dot * 0.5);
                if chosen {
                    ui.painter()
                        .circle_stroke(rect.center(), dot * 0.5 + 3.0, Stroke::new(1.8, Color32::WHITE));
                }
                response.clicked()
            };
            let none = |painter: &egui::Painter, at: Pos2, reach: f32| {
                let faint = Stroke::new(1.4, Color32::from_white_alpha(190));
                painter.circle_stroke(at, reach - 1.0, faint);
                let slant = (reach - 1.0) * 0.7;
                painter.line_segment([at + vec2(-slant, slant), at + vec2(slant, -slant)], faint);
            };
            if walls {
                if cell(ui, self.wall.hue.is_none(), &none) {
                    self.wall.hue = None;
                }
                for (degrees, [r, g, b]) in TINTS {
                    let fill = move |painter: &egui::Painter, at: Pos2, reach: f32| {
                        painter.circle_filled(at, reach, Color32::from_rgb(r, g, b));
                    };
                    if cell(ui, self.wall.hue == Some(degrees), &fill) {
                        self.wall.hue = Some(degrees);
                    }
                }
            } else {
                if cell(ui, self.palette.is_none(), &none) {
                    self.palette = None;
                }
                for (index, name) in palette::NAMES.iter().enumerate() {
                    let Some(shades) = palette::named(name).map(|named| palette::lut(&named)) else {
                        continue;
                    };
                    let fill = move |painter: &egui::Painter, at: Pos2, reach: f32| {
                        gradient_disc(painter, at, reach, |share| {
                            let [r, g, b] = shades[(share * 255.0).round() as usize];
                            Color32::from_rgb(r, g, b)
                        });
                    };
                    if cell(ui, self.palette == Some(index), &fill) {
                        self.palette = Some(index);
                    }
                }
            }
        });
        if before != (self.wall.hue, self.palette) {
            if walls && !self.wall.live {
                self.new_wallpaper();
            } else {
                self.requested = None;
            }
        }
    }

    fn current(&self, size: [u32; 2]) -> (Layout, Material) {
        chosen(self.seed, &self.wall.request(size))
    }

    pub(crate) fn phone_home(
        &mut self,
        ctx: &egui::Context,
        content: Rect,
        size: [u32; 2],
        panes: &mut Panes,
        covered: bool,
    ) {
        // A list's glass is drawn under every text: what it would cover hides while the list is open.
        if covered {
            return;
        }
        if self.phone.tune {
            self.phone_tune(ctx, content, panes);
            return;
        }
        if self.phone.sheet {
            self.phone_sheet(ctx, content, size, panes);
            return;
        }
        let ink = self.ink();

        self.phone.clear.clear();
        let (on_picture, ink) = (ink, Color32::WHITE);
        let reverse = Color32::from_rgb(14, 18, 34);
        let labels = ["flames", "fractals", "shapes"].map(|key| self.t(key));
        let (lift, space) = if content.height() < LOW {
            (6.0, 6.0)
        } else {
            (10.0, 8.0)
        };
        let tab = self.tab();
        let mut mode = None;
        let pill = egui::Area::new(egui::Id::unique("phone_mode"))
            .fixed_pos(egui::pos2(content.center().x, content.top() + lift))
            .pivot(Align2::CENTER_TOP)
            .show(ctx, |ui| {
                egui::Frame::NONE.inner_margin(egui::Margin::same(4)).show(ui, |ui| {
                    ui.spacing_mut().item_spacing = vec2(2.0, 0.0);
                    ui.horizontal(|ui| {
                        for (index, label) in labels.iter().enumerate() {
                            let chosen = index == tab;
                            let color = if chosen { reverse } else { ink };
                            let galley = ui.painter().layout_no_wrap(label.clone(), font(13.5, MEDIUM), color);
                            let (rect, response) =
                                ui.allocate_exact_size(vec2(galley.size().x + 26.0, 36.0), Sense::click());
                            if chosen {
                                ui.painter()
                                    .rect_filled(rect, CornerRadius::same(18), ink.gamma_multiply(0.92));
                            }
                            ui.painter().galley(rect.center() - galley.size() * 0.5, galley, color);
                            if response.clicked() {
                                mode = Some(index);
                            }
                        }
                    });
                });
            });
        panes.push((pill.response.rect, pill.response.rect.height() * 0.5, false));
        self.phone.clear.push(pill.response.rect);
        if let Some(tab) = mode {
            self.set_mode(tab);
        }

        let strip = egui::Area::new(egui::Id::unique("phone_colors"))
            .fixed_pos(egui::pos2(content.center().x, pill.response.rect.bottom() + space))
            .pivot(Align2::CENTER_TOP)
            .show(ctx, |ui| {
                egui::Frame::NONE
                    .inner_margin(egui::Margin::symmetric(8, 3))
                    .show(ui, |ui| self.colors(ui, 20.0));
            });
        panes.push((strip.response.rect, strip.response.rect.height() * 0.5, false));
        self.phone.clear.push(strip.response.rect);
        let below = strip.response.rect.bottom() + 12.0;

        let similar = if self.wall.live { "similar" } else { "mutate" };
        let piece = self.piece(size);
        let is_kept = self.phone.kept.iter().any(|kept| kept.kind == piece.kind);
        let column = [(Icon::Style, "style"), (Icon::Save, "export")];
        let fan = [
            (Icon::Mutate, similar),
            (Icon::Keep(is_kept), "keep"),
            (Icon::Back, "back"),
        ];
        let taken = pill.response.rect.right().max(strip.response.rect.right()) + 8.0;
        let beside = rail(content, content.top() + 6.0);
        let free = taken <= beside.shutter.x - beside.small * 0.5;
        let place = if free {
            beside
        } else {
            rail(content, strip.response.rect.bottom() + 8.0)
        };
        let fan_left = place.fan[2].x - place.small * 0.5;
        // Where the name does not fit beside the buttons, as on a narrow upright phone, it stands above them.
        let (right, foot) = if fan_left - 12.0 - content.left() - 22.0 >= 170.0 {
            (fan_left - 12.0, content.bottom() - 24.0)
        } else {
            let above = place.fan[0].y - place.small * 0.5 - 12.0;
            (place.shutter.x - place.small * 0.5 - 12.0, above)
        };
        let mut pressed = None;
        let texts = &self.texts;
        let mut round = |id: egui::Id, at: Pos2, side: f32, which: Icon, key: &'static str| {
            let name = texts.get(key);
            let button = egui::Area::new(id)
                .fixed_pos(at)
                .pivot(Align2::CENTER_CENTER)
                .show(ctx, |ui| {
                    let (rect, response) = ui.allocate_exact_size(vec2(side, side), Sense::click());
                    if matches!(which, Icon::New) {
                        let held = response.is_pointer_button_down_on();
                        let reach = side * 0.5 - if held { 3.0 } else { 1.0 };
                        ui.painter().circle_stroke(rect.center(), reach, Stroke::new(1.6, ink));
                    }
                    icon(ui.painter(), rect.center(), which, ink, place.scale);
                    if response.on_hover_text(name).clicked() {
                        pressed = Some(key);
                    }
                });
            button.response.rect
        };
        let mut rects = Vec::new();
        for (index, (which, key)) in column.into_iter().enumerate() {
            let id = egui::Id::unique(("phone_rail", index));
            let rect = round(id, place.column[index], place.small, which, key);
            if key == "export" {
                self.phone.share_from = rect;
            }
            rects.push(rect);
        }
        for (index, (which, key)) in fan.into_iter().enumerate() {
            let id = egui::Id::unique(("phone_fan", index));
            rects.push(round(id, place.fan[index], place.small, which, key));
        }
        let shutter = egui::Id::unique("phone_shutter");
        rects.push(round(shutter, place.shutter, place.large, Icon::New, "randomize"));
        for rect in rects {
            panes.push((rect, rect.height() * 0.5, false));
            self.phone.clear.push(rect);
        }

        match pressed {
            Some("style") => {
                self.phone.sheet = true;
                self.phone.lift = SHEET_TOP;
                self.phone.pull = 0.0;
                // The sheet has to be laid out before a finger can land on it, or the picture under it takes the touch.
                ctx.request_repaint();
            }
            Some("similar") => self.act("mutate"),
            Some("back") if self.has_older() => self.travel(true),
            Some("back") => crate::refuse(),
            Some("keep") => {
                crate::tick(true);
                match self.phone.kept.iter().position(|kept| kept.kind == piece.kind) {
                    Some(at) => {
                        self.phone.kept.remove(at);
                    }
                    None => {
                        self.phone.kept.insert(0, piece);
                        self.phone.kept.truncate(MOST_KEPT);
                    }
                }
            }
            Some("randomize") => {
                crate::tick(false);
                self.act("randomize");
            }
            Some(key) => self.act(key),
            None => {}
        }

        if self.view.zoom != 1.0 {
            ctx.layer_painter(egui::LayerId::new(egui::Order::Middle, egui::Id::unique("phone_zoom")))
                .text(
                    egui::pos2(content.left() + 22.0, below),
                    Align2::LEFT_TOP,
                    format!("{}×", self.view.label()),
                    font(30.0, SEMIBOLD),
                    on_picture,
                );
        }

        self.placard(ctx, content, [right, foot], size, on_picture);

        if self.view != WHOLE {
            let rect = self.reset_button(ctx, egui::pos2(content.center().x, below + 44.0));
            panes.push((rect, rect.height() * 0.5, false));
        }
    }

    fn placard(&self, ctx: &egui::Context, content: Rect, [right, foot]: [f32; 2], size: [u32; 2], ink: Color32) {
        let painter = ctx.layer_painter(egui::LayerId::new(
            egui::Order::Middle,
            egui::Id::unique("phone_placard"),
        ));
        let soft = ink.gamma_multiply(0.72);
        let mut tag = String::new();
        let (first, second) = if self.wall.live {
            let (layout, material) = self.current(size);
            let layout = Layout::ALL
                .iter()
                .find(|(_, l)| *l == layout)
                .map_or("bands", |(n, _)| n);
            let material = Material::ALL
                .iter()
                .find(|(_, m)| *m == material)
                .map_or("relief", |(n, _)| n);
            (self.t(&format!("l_{layout}")), self.t(&format!("m_{material}")))
        } else {
            let turkish = self.texts.lang == crate::i18n::Lang::Tr;
            let lines = placard_lines(
                &self.display(&self.title),
                self.phone.origin.mutation,
                &self.t("mutation"),
                turkish,
            );
            tag = lines.2;
            (lines.0, lines.1)
        };
        let detail = self.detail();
        let left = content.left() + 22.0;
        let room = right - left;
        let mut y = foot;
        let mut line = |text: String, id: egui::FontId, color: Color32, gap: f32| {
            if text.is_empty() {
                return;
            }
            let mut id = id;
            let mut galley = painter.layout_no_wrap(text.clone(), id.clone(), color);
            if galley.size().x > room {
                id.size *= room / galley.size().x;
                galley = painter.layout_no_wrap(text, id, color);
            }
            y -= galley.size().y;
            painter.galley(egui::pos2(left, y), galley, color);
            y -= gap;
        };
        line(detail, egui::FontId::monospace(11.5), soft, 6.0);
        line(tag, font(11.0, SEMIBOLD), Color32::from_white_alpha(120), 4.0);
        line(second, font(34.0, MEDIUM), soft, -6.0);
        line(first, font(34.0, SEMIBOLD), ink, 4.0);
    }

    fn phone_tune(&mut self, ctx: &egui::Context, content: Rect, panes: &mut Panes) {
        let panel = content.shrink(14.0);
        // On its side a phone is wide and low: the controls go in two columns.
        let lying = content.width() > content.height();
        let mut done = false;
        egui::Area::new(egui::Id::unique("phone_tune"))
            .fixed_pos(panel.min)
            .show(ctx, |ui| {
                let margin = egui::Margin {
                    left: 22,
                    right: 8,
                    top: 16,
                    bottom: 14,
                };
                egui::Frame::NONE.inner_margin(margin).show(ui, |ui| {
                    let inner = panel.width() - 30.0;
                    ui.set_width(inner);
                    ui.spacing_mut().item_spacing = vec2(8.0, 2.0);
                    let top = ui.cursor().top();
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| self.inspector_head(ui));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                            ui.add_space(14.0);
                            let (spot, _) =
                                ui.allocate_exact_size(vec2(widgets::CLOSE, widgets::CLOSE), Sense::hover());
                            done = widgets::close_at(ui, spot.center(), &self.t("close"));
                        });
                    });
                    ui.add_space(4.0);
                    let room = panel.height() - 30.0 - (ui.cursor().top() - top);
                    let mut bar = egui::style::ScrollStyle::floating();
                    bar.bar_width = 5.0;
                    bar.floating_allocated_width = 0.0;
                    ui.spacing_mut().scroll = bar;
                    crate::scrolled(egui::ScrollArea::vertical())
                        .max_height(room)
                        .min_scrolled_height(room)
                        .show(ui, |ui| {
                            let gutter = egui::Margin {
                                right: 14,
                                ..Default::default()
                            };
                            egui::Frame::NONE.inner_margin(gutter).show(ui, |ui| {
                                if lying {
                                    ui.spacing_mut().item_spacing.x = 26.0;
                                    ui.columns(2, |columns| {
                                        columns[0].spacing_mut().item_spacing = vec2(8.0, 2.0);
                                        self.inspector_scene(&mut columns[0]);
                                        columns[1].spacing_mut().item_spacing = vec2(8.0, 2.0);
                                        self.inspector_look(&mut columns[1]);
                                    });
                                } else {
                                    self.inspector_scene(ui);
                                    self.inspector_look(ui);
                                }
                            });
                        });
                });
            });
        panes.push((panel, self.optics.radius, false));
        if done {
            self.phone.tune = false;
        }
    }

    pub(crate) fn thumb(
        &mut self,
        ctx: &egui::Context,
        key: String,
        pixels: [usize; 2],
        samples: u32,
        budget: &mut u32,
        scene: impl FnOnce() -> Option<Scene>,
    ) -> Option<egui::TextureId> {
        if let Some(found) = self.phone.thumbs.get(&key) {
            return Some(found.id());
        }
        while let Ok((key, pixels, rgba)) = self.phone.drawn.1.try_recv() {
            self.phone.drawing.remove(&key);
            if self.phone.thumbs.len() > 120 {
                self.phone.thumbs.clear();
            }
            let picture = egui::ColorImage::from_rgba_unmultiplied(pixels, &rgba);
            let texture = ctx.load_texture(key.clone(), picture, egui::TextureOptions::LINEAR);
            self.phone.thumbs.insert(key, texture);
        }
        if let Some(found) = self.phone.thumbs.get(&key) {
            return Some(found.id());
        }
        if *budget == 0 || self.phone.drawing.contains(&key) || self.phone.drawing.len() >= DRAWN_AT_ONCE {
            return None;
        }
        *budget -= 1;
        let options = RenderOptions {
            width: pixels[0] as u32,
            height: pixels[1] as u32,
            samples_per_pixel: samples,
            supersample: 1,
        };
        let scene = scene()?;
        self.phone.drawing.insert(key.clone());
        let (sender, ctx) = (self.phone.drawn.0.clone(), ctx.clone());
        let kept = thumb_file(&key, pixels);
        // A small picture drawn on the window's thread holds every frame up for as long as it takes.
        let work = move || {
            let rgba = kept
                .as_deref()
                .and_then(|path| stored_thumb(path, pixels))
                .unwrap_or_else(|| {
                    #[cfg(not(target_arch = "wasm32"))]
                    let image = thumb_pool().install(|| render(&scene, &options, &Control::default()));
                    #[cfg(target_arch = "wasm32")]
                    let image = render(&scene, &options, &Control::default());
                    match image {
                        Ok(image) => {
                            if let Some(path) = &kept {
                                let _ = path.parent().map(std::fs::create_dir_all);
                                let _ = crate::save_png(path, &image.rgba, pixels);
                            }
                            image.rgba
                        }
                        Err(_) => vec![0; pixels[0] * pixels[1] * 4],
                    }
                });
            let _ = sender.send((key, pixels, rgba));
            ctx.request_repaint();
        };
        #[cfg(not(target_arch = "wasm32"))]
        std::thread::spawn(work);
        #[cfg(target_arch = "wasm32")]
        work();
        None
    }

    fn tile(ui: &mut egui::Ui, side: [f32; 2], picture: Option<egui::TextureId>, label: &str, chosen: bool) -> bool {
        let (rect, response) = ui.allocate_exact_size(vec2(side[0], side[1] + 20.0), Sense::click());
        let frame = Rect::from_min_size(rect.min, vec2(side[0], side[1]));
        let round = CornerRadius::same(10);
        match picture {
            Some(id) => {
                egui::Image::from_texture(egui::load::SizedTexture::new(id, frame.size()))
                    .corner_radius(round)
                    .paint_at(ui, frame);
            }
            None => {
                ui.painter().rect_filled(frame, round, Color32::from_white_alpha(18));
            }
        }
        let edge = if chosen {
            Stroke::new(2.0, Color32::WHITE)
        } else {
            Stroke::new(0.6, Color32::from_white_alpha(70))
        };
        ui.painter().rect_stroke(frame, round, edge, egui::StrokeKind::Inside);
        let color = if chosen {
            Color32::WHITE
        } else {
            Color32::from_white_alpha(170)
        };
        let mut id = egui::FontId::proportional(11.0);
        let mut galley = ui.painter().layout_no_wrap(label.to_owned(), id.clone(), color);
        if galley.size().x > side[0] {
            id.size *= side[0] / galley.size().x;
            galley = ui.painter().layout_no_wrap(label.to_owned(), id, color);
        }
        ui.painter().galley(
            egui::pos2(frame.center().x - galley.size().x * 0.5, frame.bottom() + 4.0),
            galley,
            color,
        );
        response.clicked()
    }

    fn phone_sheet(&mut self, ctx: &egui::Context, content: Rect, size: [u32; 2], panes: &mut Panes) {
        let high = content.height();
        let top = content.top() + (high * self.phone.lift + self.phone.pull).clamp(high * SHEET_TALL, high * 0.92);
        let rect = Rect::from_min_max(egui::pos2(content.left() + 8.0, top), content.max - vec2(8.0, 8.0));
        let scale = ctx.pixels_per_point();
        let mut close = false;
        let mut tune = false;
        egui::Area::new(egui::Id::unique("phone_sheet"))
            .fixed_pos(rect.min)
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                ui.set_min_size(rect.size());
                ui.set_max_size(rect.size());
                egui::Frame::NONE
                    .inner_margin(egui::Margin {
                        left: 18,
                        right: 6,
                        top: 10,
                        bottom: 14,
                    })
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing = vec2(8.0, 2.0);
                        let (bar, grab) =
                            ui.allocate_exact_size(vec2(ui.available_width() - 12.0, 30.0), Sense::click_and_drag());
                        ui.painter().rect_filled(
                            Rect::from_center_size(bar.center(), vec2(40.0, 4.5)),
                            CornerRadius::same(2),
                            Color32::from_white_alpha(150),
                        );
                        close = grab.clicked();
                        if grab.dragged() {
                            self.phone.pull += grab.drag_delta().y;
                        }
                        if grab.drag_stopped() {
                            let at = self.phone.lift + std::mem::take(&mut self.phone.pull) / high;
                            if at > 0.55 {
                                close = true;
                            } else {
                                self.phone.lift = if at < 0.2 { SHEET_TALL } else { SHEET_TOP };
                            }
                        }
                        let mut scroll = egui::style::ScrollStyle::floating();
                        scroll.bar_width = 5.0;
                        scroll.floating_allocated_width = 0.0;
                        ui.spacing_mut().scroll = scroll;
                        egui::ScrollArea::vertical()
                            .max_height(rect.height() - 60.0)
                            .min_scrolled_height(rect.height() - 60.0)
                            .show(ui, |ui| {
                                egui::Frame::NONE
                                    .inner_margin(egui::Margin {
                                        right: 12,
                                        ..Default::default()
                                    })
                                    .show(ui, |ui| {
                                        let kept = self.phone.kept.clone();
                                        close |= self.sheet_row(ui, scale, "collection", &kept, None);
                                        let recent = self.recent_here();
                                        if recent.len() > 1 {
                                            let shown = Some(self.phone.at[self.tab()]);
                                            close |= self.sheet_row(ui, scale, "recent", &recent, shown);
                                        }
                                        widgets::section(ui, &self.t("color"));
                                        ui.add_space(6.0);
                                        self.colors(ui, 26.0);
                                        if self.wall.on {
                                            self.sheet_walls(ui, size, scale);
                                        } else if self.fractals {
                                            close |= self.sheet_fractals(ui, scale);
                                        } else {
                                            close |= self.sheet_flames(ui, scale);
                                        }
                                        ui.add_space(18.0);
                                        tune = widgets::pill(ui, &self.t("fine_tune"), false, ui.available_width());
                                    });
                            });
                    });
            });
        panes.push((rect, 28.0, true));
        if tune {
            self.phone.sheet = false;
            self.phone.tune = true;
        } else if close {
            self.phone.sheet = false;
        }
    }

    fn sheet_row(
        &mut self,
        ui: &mut egui::Ui,
        scale: f32,
        title: &str,
        pieces: &[Piece],
        shown: Option<usize>,
    ) -> bool {
        if pieces.is_empty() {
            return false;
        }
        let ctx = ui.ctx().clone();
        let wide = tile_width(ui.available_width(), 8.0);
        let side = [wide, (wide * TILE_RATIO).round()];
        let pixels = [(side[0] * scale) as usize, (side[1] * scale) as usize];
        let mut budget = 1;
        let mut picked = None;
        widgets::section(ui, &self.t(title));
        ui.add_space(4.0);
        egui::ScrollArea::horizontal()
            .id_salt(("phone_row", title))
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = vec2(8.0, 0.0);
                    for (index, piece) in pieces.iter().enumerate() {
                        let picture = self.thumb(&ctx, format!("k {}", piece.key()), pixels, 10, &mut budget, || {
                            Some(piece.scene(f64::from(TILE_RATIO)))
                        });
                        if Self::tile(ui, side, picture, &self.tile_label(piece), Some(index) == shown) {
                            picked = Some(piece.clone());
                        }
                    }
                });
            });
        match picked {
            Some(piece) => {
                self.show_piece(&piece);
                true
            }
            None => false,
        }
    }

    fn sheet_walls(&mut self, ui: &mut egui::Ui, size: [u32; 2], scale: f32) {
        let ctx = ui.ctx().clone();
        let before = (self.wall.layout, self.wall.material, self.wall.dark);
        let (now_layout, now_material) = self.current(size);
        let (seed, dark, hue) = (self.seed, self.wall.dark, self.wall.hue);
        let any = self.t("any");
        let gap = 8.0;
        let wide = tile_width(ui.available_width(), gap);
        let mut budget = 3;

        widgets::section(ui, &self.t("layout"));
        ui.add_space(4.0);
        let side = [wide, (wide * TILE_RATIO).round()];
        let pixels = [(side[0] * scale) as usize, (side[1] * scale) as usize];
        let mut layout = self.wall.layout;
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(gap, 10.0);
            if Self::tile(ui, side, None, &any, layout.is_none()) {
                layout = None;
            }
            for (name, value) in Layout::ALL {
                let picture = self.thumb(
                    &ctx,
                    format!("l {name} {now_material:?} {dark} {seed} {hue:?}"),
                    pixels,
                    1,
                    &mut budget,
                    || {
                        Some(random_wallpaper(
                            seed,
                            &Request {
                                layout: Some(value),
                                material: Some(now_material),
                                ratio: f64::from(TILE_RATIO),
                                dark,
                                hue,
                            },
                        ))
                    },
                );
                if Self::tile(ui, side, picture, &self.t(&format!("l_{name}")), layout == Some(value)) {
                    layout = Some(value);
                }
            }
        });
        self.wall.layout = layout;

        widgets::section(ui, &self.t("material"));
        ui.add_space(4.0);
        let side = [wide, wide];
        let pixels = [(wide * scale) as usize, (wide * scale) as usize];
        let mut material = self.wall.material;
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(gap, 10.0);
            if Self::tile(ui, side, None, &any, material.is_none()) {
                material = None;
            }
            for (name, value) in Material::ALL {
                let picture = self.thumb(
                    &ctx,
                    format!("m {name} {now_layout:?} {dark} {seed} {hue:?}"),
                    pixels,
                    1,
                    &mut budget,
                    || {
                        Some(random_wallpaper(
                            seed,
                            &Request {
                                layout: Some(now_layout),
                                material: Some(value),
                                ratio: 1.0,
                                dark,
                                hue,
                            },
                        ))
                    },
                );
                if Self::tile(
                    ui,
                    side,
                    picture,
                    &self.t(&format!("m_{name}")),
                    material == Some(value),
                ) {
                    material = Some(value);
                }
            }
        });
        self.wall.material = material;

        ui.add_space(16.0);
        let (light, shade) = (self.t("light"), self.t("dark"));
        let half = (ui.available_width() - 8.0) * 0.5;
        let mut is_dark = self.wall.dark;
        ui.horizontal(|ui| {
            if widgets::pill(ui, &light, !is_dark, half) {
                is_dark = false;
            }
            if widgets::pill(ui, &shade, is_dark, half) {
                is_dark = true;
            }
        });
        self.wall.dark = is_dark;

        if before != (self.wall.layout, self.wall.material, self.wall.dark) {
            if before.2 == self.wall.dark {
                self.wall.held = None;
            }
            if self.wall.live {
                self.requested = None;
            } else {
                self.new_wallpaper();
            }
        }
    }

    // The kinds of fractal, each with a small picture of its first preset, and the presets of the chosen one.
    fn sheet_fractals(&mut self, ui: &mut egui::Ui, scale: f32) -> bool {
        let ctx = ui.ctx().clone();
        let gap = 8.0;
        let wide = tile_width(ui.available_width(), gap);
        let side = [wide, (wide * TILE_RATIO).round()];
        let pixels = [(side[0] * scale) as usize, (side[1] * scale) as usize];
        let mut budget = 2;
        let colors = self.palette.and_then(|at| palette::named(palette::NAMES[at]));
        let pictured = |name: &str| {
            let mut scene = fractal::preset(name)?;
            if let Some(colors) = &colors {
                for layer in &mut scene.layers {
                    *layer.palette_mut() = colors.clone();
                }
            }
            Some(scene)
        };

        widgets::section(ui, &self.t("fractal_kind"));
        ui.add_space(4.0);
        let mut kind = None;
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(gap, 10.0);
            for spec in fractal::all().filter(|spec| !crate::heavy(spec.name)) {
                let first = spec
                    .presets
                    .first()
                    .map(|preset| format!("{}-{}", spec.name, preset.name));
                let key = format!("p {first:?} {:?}", self.palette);
                let picture = self.thumb(&ctx, key, pixels, 10, &mut budget, || pictured(&first?));
                if Self::tile(ui, side, picture, &self.kind_name(spec.name), self.kind == spec.name) {
                    kind = Some(spec.name);
                }
            }
        });
        if let Some(kind) = kind {
            self.show_kind(kind);
        }

        widgets::section(ui, &self.t("presets"));
        ui.add_space(4.0);
        let random_label = shown(&self.t("random"));
        let (mut random, mut picked) = (false, None);
        let prefix = self.kind.len() + 1;
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(gap, 10.0);
            random = Self::tile(ui, side, None, &random_label, self.is_random && !self.wall.live);
            for name in self.kind_presets() {
                let key = format!("p {:?} {:?}", Some(&name), self.palette);
                let picture = self.thumb(&ctx, key, pixels, 10, &mut budget, || pictured(&name));
                let current = !self.is_random && self.title == name;
                let label = crate::preset_name(&self.texts, &self.kind, &name[prefix..]);
                if Self::tile(ui, side, picture, &label, current) {
                    picked = Some(name);
                }
            }
        });
        if random {
            self.act("randomize");
            return true;
        }
        if let Some(name) = picked {
            self.show_preset(&name);
            return true;
        }
        false
    }

    fn sheet_flames(&mut self, ui: &mut egui::Ui, scale: f32) -> bool {
        let ctx = ui.ctx().clone();
        let gap = 8.0;
        let wide = tile_width(ui.available_width(), gap);
        let side = [wide, (wide * TILE_RATIO).round()];
        let pixels = [(side[0] * scale) as usize, (side[1] * scale) as usize];
        let mut budget = 1;
        let mut picked = None;
        let mut random = false;

        widgets::section(ui, &self.t("presets"));
        ui.add_space(4.0);
        let random_label = shown(&self.t("random"));
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(gap, 10.0);
            random = Self::tile(ui, side, None, &random_label, self.is_random && !self.wall.live);
            for name in presets::names() {
                let colors = self.palette.and_then(|at| palette::named(palette::NAMES[at]));
                let key = format!("p {name} {:?}", self.palette);
                let picture = self.thumb(&ctx, key, pixels, 10, &mut budget, || {
                    let mut scene = presets::get(name)?;
                    if let Some(colors) = colors {
                        for layer in &mut scene.layers {
                            *layer.palette_mut() = colors.clone();
                        }
                    }
                    Some(scene)
                });
                let current = !self.is_random && self.title == name;
                if Self::tile(ui, side, picture, &self.display(name), current) {
                    picked = Some(name);
                }
            }
        });
        if random {
            self.act("randomize");
            return true;
        }
        if let Some(name) = picked
            && let Some(scene) = presets::get(name)
        {
            self.pick(named(scene, &self.display(name)), name.to_owned(), false);
            return true;
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_mutation_is_a_tag_under_the_name_and_no_part_of_it() {
        let lines = |title: &str, mutation: u32| placard_lines(title, mutation, "мутація", false);
        let text = |a: &str, b: &str, c: &str| (a.to_owned(), b.to_owned(), c.to_owned());
        assert_eq!(lines("Випадковий", 0), text("Випадковий", "", ""));
        assert_eq!(lines("Випадковий", 1), text("Випадковий", "", "МУТАЦІЯ 1"));
        assert_eq!(lines("Спіраль аврори", 0), text("Спіраль", "аврори", ""));
        assert_eq!(lines("Спіраль аврори", 12), text("Спіраль", "аврори", "МУТАЦІЯ 12"));
        assert_eq!(placard_lines("Rastgele", 2, "değişim", true).2, "DEĞİŞİM 2");
        assert_eq!(placard_lines("Zufall", 2, "Mutation", false).2, "MUTATION 2");
    }

    #[test]
    fn the_buttons_stay_between_the_status_bar_and_the_lower_edge() {
        for (width, height, top) in [
            (753.0, 360.0, 30.0),
            (800.0, 360.0, 0.0),
            (640.0, 295.0, 24.0),
            (568.0, 320.0, 0.0),
        ]
        .into_iter()
        .chain((120..900).step_by(7).map(|height| (400.0, height as f32, 6.0)))
        {
            let content = Rect::from_min_max(egui::pos2(0.0, top), egui::pos2(width, height));
            let place = rail(content, content.top() + 6.0);
            let half = place.small * 0.5;
            let (right, bottom) = (place.shutter.x + place.large * 0.5, place.shutter.y + place.large * 0.5);
            assert!(bottom <= content.bottom() && right <= content.right(), "{height}");
            let away = (place.fan[1] - place.shutter).length();
            for (index, at) in place.fan.into_iter().enumerate() {
                assert!(((at - place.shutter).length() - away).abs() < 0.01, "{height}");
                assert!(away - half - place.large * 0.5 >= CLEAR - 0.01, "{height}: {away}");
                assert!(at.x - half > content.left(), "{height}");
                if index > 0 {
                    let between = (at - place.fan[index - 1]).length() - place.small;
                    assert!(between >= CLEAR - 0.01, "{height}: {between}");
                }
            }
            assert!((place.fan[2].y + half - bottom).abs() < 0.001, "{height}");
            let mut lowest = place.fan[0].y - half;
            for at in [place.fan[0]].into_iter().chain(place.column) {
                assert!((at.x + half - right).abs() < 0.001, "{height}");
            }
            for at in place.column {
                assert!(at.y + half < lowest, "{height}");
                lowest = at.y - half;
            }
            assert!(lowest >= content.top() - 0.01, "{height}: {lowest}");
            let tight = 8.0 + 34.0 + reach(1.0)[1] + 115.0 + 8.0;
            assert_eq!(place.scale == 1.0, content.height() - 6.0 >= tight, "{height}");
        }
    }

    #[test]
    fn a_tall_screen_has_the_buttons_at_their_full_size() {
        let place = rail(
            Rect::from_min_max(egui::pos2(0.0, 40.0), egui::pos2(360.0, 800.0)),
            150.0,
        );
        assert_eq!((place.large, place.small, place.scale), (68.0, 46.0, 1.0));
        assert_eq!(place.shutter, egui::pos2(312.0, 746.0));
        assert_eq!(reach(1.0)[0], 65.0);
        assert_eq!(place.fan[0].x, 323.0);
        assert!((place.fan[2].y - 757.0).abs() < 0.001);
        assert!((place.fan[0].y - place.column[0].y - 56.0).abs() < 0.001);
        assert!((place.column[0].y - place.column[1].y - 56.0).abs() < 0.001);
    }

    #[test]
    fn a_low_screen_closes_the_gaps_before_the_buttons_shrink() {
        let place = rail(Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(700.0, 240.0)), 0.0);
        assert_eq!(place.scale, 1.0);
        let gap = place.column[0].y - place.column[1].y - 46.0;
        assert!((4.0..10.0).contains(&gap), "{gap}");
        assert!(place.column[1].y - 23.0 >= -0.01);
    }

    fn wall(seed: u64) -> Piece {
        Piece {
            title: "Waves Steel".into(),
            kind: Kind::Wall(seed, Layout::ALL[0].1, Material::ALL[0].1, false, None),
            origin: Origin::default(),
        }
    }

    fn flame(title: &str, brightness: f64) -> Piece {
        let mut scene = fractique_core::presets::get("aurora-spiral").unwrap();
        scene.layers[0].tone_mut().brightness = brightness;
        Piece {
            title: title.into(),
            kind: Kind::Flame(Box::new(scene)),
            origin: Origin::default(),
        }
    }

    #[test]
    fn a_new_picture_after_going_back_is_followed_by_the_pictures_behind_the_one_shown() {
        let mut recent = Vec::new();
        for seed in 1..=4 {
            note(&mut recent, wall(seed), Step::Fresh, 0, 100);
        }
        assert!(kinds(&recent) == kinds(&[wall(4), wall(3), wall(2), wall(1)]));
        assert_eq!(note(&mut recent, wall(9), Step::Fresh, 2, 200), 0);
        assert!(kinds(&recent) == kinds(&[wall(9), wall(2), wall(1), wall(4), wall(3)]));
    }

    fn kinds(recent: &[Piece]) -> Vec<Kind> {
        recent.iter().map(|piece| piece.kind.clone()).collect()
    }

    #[test]
    fn recent_pictures_are_newest_first_without_repeats() {
        let mut recent = Vec::new();
        for piece in [wall(1), wall(2), flame("Aurora Spiral", 1.0), wall(1)] {
            note(&mut recent, piece, Step::Fresh, 0, 100);
        }
        assert_eq!(
            note(&mut recent, flame("Aurora Spiral", 1.4), Step::Adjusted, 0, 100),
            0
        );
        assert!(kinds(&recent) == kinds(&[wall(1), flame("Aurora Spiral", 1.4), wall(2)]));
        for seed in 10..60 {
            note(&mut recent, wall(seed), Step::Fresh, 0, 100);
        }
        assert_eq!(recent.len(), MOST_RECENT + 1);
        assert!(
            recent
                .iter()
                .any(|piece| piece.kind == flame("Aurora Spiral", 1.4).kind)
        );
        assert_eq!((wall(1).tab(), flame("Aurora Spiral", 1.4).tab()), (2, 0));
        assert!(recent[0].kind == wall(59).kind);
    }

    #[test]
    fn every_mutation_is_a_picture_of_its_own_and_an_adjustment_is_none() {
        let mut recent = Vec::new();
        note(&mut recent, flame("Random 7", 1.0), Step::Fresh, 0, 100);
        let mut origin = recent[0].origin;
        assert_eq!((origin.mutation, origin.made), (0, Some(100)));
        for count in 1..=5 {
            let mut piece = flame("Random 7", 1.0 + f64::from(count));
            piece.origin = origin;
            assert_eq!(note(&mut recent, piece, Step::Mutated, 0, 100 + u64::from(count)), 0);
            origin = recent[0].origin;
            assert_eq!((origin.mutation, origin.made), (count, Some(100 + u64::from(count))));
        }
        assert_eq!(recent.len(), 6);
        assert!(recent.iter().all(|piece| piece.origin.family == origin.family));
        let fifth_down = recent[4].clone();
        assert_eq!(fifth_down.origin.mutation, 1);

        let mut adjusted = flame("Random 7", 9.5);
        adjusted.origin = origin;
        assert_eq!(note(&mut recent, adjusted.clone(), Step::Adjusted, 0, 200), 0);
        assert_eq!(recent.len(), 6);
        assert!(recent[0].kind == adjusted.kind);
        assert_eq!(recent[0].origin, origin);

        assert_eq!(note(&mut recent, fifth_down.clone(), Step::Visited, 0, 300), 4);
        assert!(recent[4] == fifth_down);
        let mut tuned = flame("Random 7", 2.5);
        tuned.origin = fifth_down.origin;
        assert_eq!(note(&mut recent, tuned.clone(), Step::Adjusted, 4, 300), 4);
        assert!(recent[4].kind == tuned.kind && recent.len() == 6);

        let mut mutated = flame("Random 7", 7.7);
        mutated.origin = fifth_down.origin;
        assert_eq!(note(&mut recent, mutated, Step::Mutated, 4, 400), 0);
        assert_eq!((recent.len(), recent[0].origin.mutation), (7, 6));
        assert_eq!(note(&mut recent, tuned.clone(), Step::Picked, 0, 500), 0);
        assert_eq!(recent.len(), 7);
        assert!(recent[0] == tuned);

        note(&mut recent, flame("Random 8", 0.3), Step::Fresh, 0, 600);
        assert_eq!(recent[0].origin.mutation, 0);
        assert_ne!(recent[0].origin.family, origin.family);
    }

    #[test]
    fn a_list_of_an_older_build_loads_and_a_mutation_starts_its_family() {
        let old = Value::Array(vec![json!({
            "title": "Random 12",
            "scene": fractique_core::presets::get("aurora-spiral").unwrap().to_json(),
        })]);
        let mut phone = Phone::new(None, Some(old.to_string()));
        assert_eq!(phone.recent.len(), 1);
        assert_eq!(phone.recent[0].origin, Origin::default());
        assert_eq!(phone.recent_text(), old.to_string());

        note(&mut phone.recent, flame("Random 12", 3.0), Step::Mutated, 0, 100);
        assert_eq!(phone.recent[0].origin.mutation, 1);
        assert_eq!(phone.recent[1].origin.family, phone.recent[0].origin.family);
        let again = Phone::new(None, Some(phone.recent_text()));
        assert!(again.recent == phone.recent);
        let mut same = phone.recent[0].clone();
        same.origin = Origin::default();
        assert_eq!(same.key(), phone.recent[0].key());
    }

    #[test]
    fn the_time_of_another_day_has_its_date() {
        let day = 86_400;
        assert_eq!(clock(1000, 1000).len(), "14:32".len());
        assert_eq!(clock(1000, 1000 + 3 * day).len(), "18.11 14:32".len());
    }
}

use std::collections::{HashMap, HashSet};
use std::sync::mpsc;

use eframe::egui;
use egui::{Align2, Color32, CornerRadius, Pos2, Rect, Sense, Stroke, vec2};
use fractique_core::wallpaper::{Layout, Material, Request, chosen, random_wallpaper};
use fractique_core::{Control, RenderOptions, Scene, presets, render};
use serde_json::{Value, json};

use crate::widgets::{self, MEDIUM, SEMIBOLD, font};
use crate::{App, WHOLE, named, shown};

pub const SWIPE: f32 = 56.0;
const TILE_RATIO: f32 = 1.6;
const MOST_KEPT: usize = 48;
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

#[derive(Clone, PartialEq)]
pub struct Piece {
    pub title: String,
    pub kind: Kind,
}

impl Piece {
    fn key(&self) -> String {
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

    fn to_value(&self) -> Value {
        match &self.kind {
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
        }
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
        Some(Self { title, kind })
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
    thumbs: HashMap<String, egui::TextureHandle>,
    drawing: HashSet<String>,
    drawn: (mpsc::Sender<Thumb>, mpsc::Receiver<Thumb>),
    pub pressing: bool,
}

type Thumb = (String, [usize; 2], Vec<u8>);

// More at once would take the cores the picture itself is drawn with.
const DRAWN_AT_ONCE: usize = 2;

impl Phone {
    pub fn kept_text(&self) -> String {
        Value::Array(self.kept.iter().map(Piece::to_value).collect()).to_string()
    }

    pub fn new(kept: Option<String>) -> Self {
        let kept = kept
            .and_then(|text| serde_json::from_str::<Value>(&text).ok())
            .and_then(|value| value.as_array().cloned())
            .map(|list| list.iter().filter_map(Piece::from_value).collect())
            .unwrap_or_default();
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
            thumbs: HashMap::new(),
            drawing: HashSet::new(),
            drawn: mpsc::channel(),
            pressing: false,
        }
    }
}

// Four across on an upright phone; more on one lying on its side, where four would be taller than the screen.
fn tile_width(room: f32, gap: f32) -> f32 {
    let across = (room / 104.0).floor().max(4.0);
    ((room - (across - 1.0) * gap) / across).floor()
}

#[derive(Clone, Copy)]
enum Icon {
    Keep(bool),
    Similar,
    Style,
    Save,
}

fn icon(painter: &egui::Painter, at: Pos2, which: Icon, ink: Color32) {
    let stroke = Stroke::new(1.6, ink);
    match which {
        Icon::Keep(full) => {
            let outline: Vec<Pos2> = (0..40)
                .map(|step| {
                    let t = step as f32 / 40.0 * std::f32::consts::TAU;
                    let (sin, cos) = t.sin_cos();
                    let x = 16.0 * sin * sin * sin;
                    let y = 13.0 * cos - 5.0 * (2.0 * t).cos() - 2.0 * (3.0 * t).cos() - (4.0 * t).cos();
                    at + vec2(x, -y * 0.92 + 1.0) * 0.52
                })
                .collect();
            if full {
                let mut mesh = egui::Mesh::default();
                mesh.colored_vertex(at + vec2(0.0, 1.0), ink);
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
        Icon::Similar => {
            let star = |center: Pos2, reach: f32| {
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
            };
            star(at + vec2(-2.0, 2.0), 8.0);
            star(at + vec2(8.0, -8.0), 3.4);
        }
        Icon::Style => {
            for (y, knob) in [(-6.0, 4.0), (0.0, -4.0), (6.0, 2.0)] {
                painter.line_segment([at + vec2(-9.0, y), at + vec2(9.0, y)], stroke);
                painter.circle_filled(at + vec2(knob, y), 2.8, ink);
            }
        }
        Icon::Save => {
            painter.line_segment([at + vec2(0.0, -9.0), at + vec2(0.0, 3.0)], stroke);
            painter.line_segment([at + vec2(-5.0, -2.0), at + vec2(0.0, 3.0)], stroke);
            painter.line_segment([at + vec2(5.0, -2.0), at + vec2(0.0, 3.0)], stroke);
            painter.line_segment([at + vec2(-8.0, 8.0), at + vec2(8.0, 8.0)], stroke);
        }
    }
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

    fn set_mode(&mut self, walls: bool) {
        if walls && !self.wall.on {
            self.wall.on = true;
            self.new_wallpaper();
        } else if !walls && self.wall.on {
            self.wall.on = false;
            let name = "aurora-spiral";
            if self.wall.live
                && let Some(scene) = presets::get(name)
            {
                self.pick(named(scene, &shown(name)), name.to_owned(), false);
            }
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
                title: self.title.clone(),
                kind: Kind::Wall(self.seed, layout, material, self.wall.dark, self.wall.hue),
            }
        } else {
            Piece {
                title: shown(&self.title),
                kind: Kind::Flame(Box::new(self.base.clone())),
            }
        }
    }

    fn show_piece(&mut self, piece: &Piece) {
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
                self.pick((**scene).clone(), piece.title.clone(), false);
            }
        }
    }

    pub(crate) fn clock(&self, ctx: &egui::Context, content: Rect) {
        ctx.layer_painter(egui::LayerId::new(egui::Order::Middle, egui::Id::unique("phone_clock")))
            .text(
                egui::pos2(content.center().x, content.top() + content.height() * 0.17),
                Align2::CENTER_CENTER,
                "9:41",
                font(92.0, SEMIBOLD),
                self.ink(),
            );
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
        if self.phone.tune {
            self.phone_tune(ctx, content, panes);
            return;
        }
        if self.phone.sheet {
            self.phone_sheet(ctx, content, size, panes);
            return;
        }
        if covered {
            return;
        }
        let ink = self.ink();

        self.phone.clear.clear();
        let (on_picture, ink) = (ink, Color32::WHITE);
        let reverse = Color32::from_rgb(14, 18, 34);
        let labels = [self.t("flames"), self.t("wallpapers")];
        let walls = self.wall.on;
        let mut mode = None;
        let pill = egui::Area::new(egui::Id::unique("phone_mode"))
            .fixed_pos(egui::pos2(content.center().x, content.top() + 10.0))
            .pivot(Align2::CENTER_TOP)
            .show(ctx, |ui| {
                egui::Frame::NONE.inner_margin(egui::Margin::same(4)).show(ui, |ui| {
                    ui.spacing_mut().item_spacing = vec2(2.0, 0.0);
                    ui.horizontal(|ui| {
                        for (index, label) in labels.iter().enumerate() {
                            let chosen = (index == 1) == walls;
                            let color = if chosen { reverse } else { ink };
                            let galley = ui.painter().layout_no_wrap(label.clone(), font(13.5, MEDIUM), color);
                            let (rect, response) =
                                ui.allocate_exact_size(vec2(galley.size().x + 30.0, 36.0), Sense::click());
                            if chosen {
                                ui.painter()
                                    .rect_filled(rect, CornerRadius::same(18), ink.gamma_multiply(0.92));
                            }
                            ui.painter().galley(rect.center() - galley.size() * 0.5, galley, color);
                            if response.clicked() {
                                mode = Some(index == 1);
                            }
                        }
                    });
                });
            });
        panes.push((pill.response.rect, pill.response.rect.height() * 0.5, false));
        self.phone.clear.push(pill.response.rect);
        if let Some(walls) = mode {
            self.set_mode(walls);
        }

        let shutter = egui::pos2(content.right() - 52.0, content.bottom() - 54.0);
        let similar = if self.wall.live { "similar" } else { "mutate" };
        let piece = self.piece(size);
        let is_kept = self.phone.kept.iter().any(|kept| kept.kind == piece.kind);
        let rail = [
            (Icon::Save, "export"),
            (Icon::Style, "style"),
            (Icon::Similar, similar),
            (Icon::Keep(is_kept), "keep"),
        ];
        // Upright the labels would run into the scene's name; on its side a phone has the room for them.
        let hinted = content.width() > content.height();
        let mut pressed = None;
        for (index, (which, key)) in rail.into_iter().enumerate() {
            let at = shutter - vec2(0.0, 66.0 + 56.0 * index as f32);
            let button = egui::Area::new(egui::Id::unique(("phone_rail", index)))
                .fixed_pos(at)
                .pivot(Align2::CENTER_CENTER)
                .show(ctx, |ui| {
                    let (rect, response) = ui.allocate_exact_size(vec2(46.0, 46.0), Sense::click());
                    icon(ui.painter(), rect.center(), which, ink);
                    if response.clicked() {
                        pressed = Some(key);
                    }
                });
            let rect = button.response.rect;
            panes.push((rect, rect.height() * 0.5, false));
            self.phone.clear.push(rect);
            if hinted {
                ctx.layer_painter(egui::LayerId::new(egui::Order::Middle, egui::Id::unique("phone_hints")))
                    .text(
                        rect.left_center() - vec2(10.0, 0.0),
                        Align2::RIGHT_CENTER,
                        self.t(key),
                        font(13.0, MEDIUM),
                        on_picture,
                    );
            }
        }

        let button = egui::Area::new(egui::Id::unique("phone_shutter"))
            .fixed_pos(shutter)
            .pivot(Align2::CENTER_CENTER)
            .show(ctx, |ui| {
                let (rect, response) = ui.allocate_exact_size(vec2(68.0, 68.0), Sense::click());
                let at = rect.center();
                let held = response.is_pointer_button_down_on();
                ui.painter()
                    .circle_stroke(at, if held { 31.0 } else { 33.0 }, Stroke::new(1.6, ink));
                let stroke = Stroke::new(2.0, ink);
                for flip in [1.0, -1.0] {
                    let (from, to) = (at + vec2(-11.0, -7.0 * flip), at + vec2(11.0, 7.0 * flip));
                    ui.painter().line_segment([from, to], stroke);
                    ui.painter().line_segment([to, to + vec2(-6.5, 0.0)], stroke);
                    ui.painter().line_segment([to, to + vec2(-2.0, -6.0 * flip)], stroke);
                }
                if response.clicked() {
                    pressed = Some("randomize");
                }
            });
        panes.push((button.response.rect, 34.0, false));
        self.phone.clear.push(button.response.rect);

        match pressed {
            Some("style") => {
                self.phone.sheet = true;
                self.phone.lift = SHEET_TOP;
                self.phone.pull = 0.0;
            }
            Some("similar") => self.act("mutate"),
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

        if self.view != WHOLE && !self.wall.live {
            ctx.layer_painter(egui::LayerId::new(egui::Order::Middle, egui::Id::unique("phone_zoom")))
                .text(
                    egui::pos2(content.left() + 22.0, content.top() + 62.0),
                    Align2::LEFT_TOP,
                    format!("{}×", self.view.label()),
                    font(30.0, SEMIBOLD),
                    on_picture,
                );
        }

        self.placard(ctx, content, size, on_picture);

        if self.view != WHOLE {
            let rect = self.reset_button(ctx, egui::pos2(content.center().x, content.top() + 112.0));
            panes.push((rect, rect.height() * 0.5, false));
        }
    }

    fn placard(&self, ctx: &egui::Context, content: Rect, size: [u32; 2], ink: Color32) {
        let painter = ctx.layer_painter(egui::LayerId::new(
            egui::Order::Middle,
            egui::Id::unique("phone_placard"),
        ));
        let soft = ink.gamma_multiply(0.72);
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
            let title = shown(&self.title);
            match title.rsplit_once(' ') {
                Some((head, tail)) if title.chars().count() > 9 => (head.to_owned(), tail.to_owned()),
                _ => (title, String::new()),
            }
        };
        let detail = if self.rendering {
            self.t("rendering")
        } else {
            self.status.clone()
        };
        let room = content.width() - 22.0 - 110.0;
        let left = content.left() + 22.0;
        let mut y = content.bottom() - 24.0;
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
        line(detail, egui::FontId::monospace(11.5), soft, 8.0);
        line(second, font(34.0, MEDIUM), soft, -6.0);
        line(first, font(34.0, SEMIBOLD), ink, 4.0);
        if self.wall.live {
            line(format!("{:04}", self.seed), egui::FontId::monospace(12.5), soft, 0.0);
        }
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
                            done = widgets::round(ui, "remove", true);
                        });
                    });
                    ui.add_space(4.0);
                    let room = panel.height() - 30.0 - (ui.cursor().top() - top);
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

    fn thumb(
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
        // A small picture drawn on the window's thread holds every frame up for as long as it takes.
        let work = move || {
            #[cfg(not(target_arch = "wasm32"))]
            let image = crate::render_pool().install(|| render(&scene, &options, &Control::default()));
            #[cfg(target_arch = "wasm32")]
            let image = render(&scene, &options, &Control::default());
            let rgba = image
                .map(|image| image.rgba)
                .unwrap_or_else(|_| vec![0; pixels[0] * pixels[1] * 4]);
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
                                        close |= self.sheet_kept(ui, scale);
                                        if self.wall.on {
                                            self.sheet_walls(ui, size, scale);
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

    fn sheet_kept(&mut self, ui: &mut egui::Ui, scale: f32) -> bool {
        if self.phone.kept.is_empty() {
            return false;
        }
        let ctx = ui.ctx().clone();
        let wide = tile_width(ui.available_width(), 8.0);
        let side = [wide, (wide * TILE_RATIO).round()];
        let pixels = [(side[0] * scale) as usize, (side[1] * scale) as usize];
        let mut budget = 1;
        let mut picked = None;
        widgets::section(ui, &self.t("collection"));
        ui.add_space(4.0);
        let kept = self.phone.kept.clone();
        egui::ScrollArea::horizontal()
            .id_salt("phone_kept")
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = vec2(8.0, 0.0);
                    for piece in &kept {
                        let picture = self.thumb(&ctx, format!("k {}", piece.key()), pixels, 10, &mut budget, || {
                            Some(match &piece.kind {
                                Kind::Wall(seed, layout, material, dark, hue) => random_wallpaper(
                                    *seed,
                                    &Request {
                                        layout: Some(*layout),
                                        material: Some(*material),
                                        ratio: f64::from(TILE_RATIO),
                                        dark: *dark,
                                        hue: *hue,
                                    },
                                ),
                                Kind::Flame(scene) => (**scene).clone(),
                            })
                        });
                        if Self::tile(ui, side, picture, &piece.title, false) {
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
        let before = (self.wall.layout, self.wall.material, self.wall.dark, self.wall.hue);
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

        widgets::section(ui, &self.t("color"));
        ui.add_space(6.0);
        let mut tint = self.wall.hue;
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(7.0, 8.0);
            let dot = |ui: &mut egui::Ui, fill: Option<Color32>, chosen: bool| {
                let (rect, response) = ui.allocate_exact_size(vec2(34.0, 34.0), Sense::click());
                match fill {
                    Some(color) => {
                        ui.painter().circle_filled(rect.center(), 13.0, color);
                    }
                    None => {
                        let faint = Stroke::new(1.4, Color32::from_white_alpha(170));
                        ui.painter().circle_stroke(rect.center(), 12.0, faint);
                        ui.painter().line_segment(
                            [rect.center() + vec2(-8.5, 8.5), rect.center() + vec2(8.5, -8.5)],
                            faint,
                        );
                    }
                }
                if chosen {
                    ui.painter()
                        .circle_stroke(rect.center(), 16.0, Stroke::new(2.0, Color32::WHITE));
                }
                response.clicked()
            };
            if dot(ui, None, tint.is_none()) {
                tint = None;
            }
            for (degrees, [r, g, b]) in TINTS {
                if dot(ui, Some(Color32::from_rgb(r, g, b)), tint == Some(degrees)) {
                    tint = Some(degrees);
                }
            }
        });
        self.wall.hue = tint;

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

        if before != (self.wall.layout, self.wall.material, self.wall.dark, self.wall.hue) {
            if before.2 == self.wall.dark && before.3 == self.wall.hue {
                self.wall.held = None;
            }
            if self.wall.live {
                self.requested = None;
            } else {
                self.new_wallpaper();
            }
        }
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
                let picture = self.thumb(&ctx, format!("p {name}"), pixels, 10, &mut budget, || {
                    presets::get(name)
                });
                let current = !self.is_random && self.title == name;
                if Self::tile(ui, side, picture, &shown(name), current) {
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
            self.pick(named(scene, &shown(name)), name.to_owned(), false);
            return true;
        }
        false
    }
}

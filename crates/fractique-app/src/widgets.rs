use std::sync::Arc;

use fractique_core::math;

use crate::i18n::Lang;
use crate::motion;

use eframe::egui::{self, Align2, Color32, CornerRadius, FontFamily, FontId, Sense, Ui, vec2};

pub const MEDIUM: &str = "medium";
pub const SEMIBOLD: &str = "semibold";

pub fn install_fonts(ctx: &egui::Context, lang: Lang) {
    let mut fonts = egui::FontDefinitions::default();
    let mut add = |name: &str, bytes: &'static [u8]| {
        fonts
            .font_data
            .insert(name.to_owned(), Arc::new(egui::FontData::from_static(bytes)));
    };
    add("inter", include_bytes!("../assets/fonts/Inter-Regular.ttf"));
    add(MEDIUM, include_bytes!("../assets/fonts/Inter-Medium.ttf"));
    add(SEMIBOLD, include_bytes!("../assets/fonts/Inter-SemiBold.ttf"));
    add("japanese", include_bytes!("../assets/fonts/NotoSansJP-subset.ttf"));
    add("chinese", include_bytes!("../assets/fonts/NotoSansSC-subset.ttf"));
    add("korean", include_bytes!("../assets/fonts/NotoSansKR-subset.ttf"));
    add(
        "devanagari",
        include_bytes!("../assets/fonts/NotoSansDevanagari-subset.ttf"),
    );

    // Japanese, Chinese and Korean share characters that each draws its own way: the language's font goes first.
    let fallbacks: [&str; 4] = match lang {
        Lang::ZhHans => ["chinese", "japanese", "korean", "devanagari"],
        Lang::Ko => ["korean", "japanese", "chinese", "devanagari"],
        _ => ["japanese", "chinese", "korean", "devanagari"],
    };
    let rest = fonts
        .families
        .get(&FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();
    for (family, first) in [
        (FontFamily::Proportional, "inter"),
        (FontFamily::Name(MEDIUM.into()), MEDIUM),
        (FontFamily::Name(SEMIBOLD.into()), SEMIBOLD),
    ] {
        let mut names = vec![first.to_owned()];
        names.extend(fallbacks.iter().map(|name| (*name).to_owned()));
        names.extend(rest.iter().cloned());
        fonts.families.insert(family, names);
    }
    ctx.set_fonts(fonts);
}

pub fn font(size: f32, family: &str) -> FontId {
    FontId::new(size, FontFamily::Name(family.into()))
}

fn white(alpha: u8) -> Color32 {
    Color32::from_white_alpha(alpha)
}

// The one measure of every pane: its content stands this far from each of its four edges.
pub const PAD: f32 = 22.0;
// On a phone a pane stands this far from the screen's safe edges.
pub const OUTER: f32 = 12.0;
pub const SECTION: f32 = 14.0;
const CAPTION: f32 = 2.0;
const HEAD: f32 = 4.0;

// A state that is entered and left over a short time instead of at once: 0 off, 1 on.
pub fn glided(ui: &Ui, id: egui::Id, on: bool) -> f32 {
    ui.ctx().animate_bool_with_time(id, on, motion::span(motion::SMALL))
}

pub fn lit(ui: &Ui, response: &egui::Response) -> f32 {
    glided(ui, response.id.with("lit"), response.hovered())
}

fn lift(from: u8, by: f32, lit: f32) -> u8 {
    (f32::from(from) + by * lit).round().clamp(0.0, 255.0) as u8
}

pub fn pane() -> egui::Frame {
    egui::Frame::NONE.inner_margin(egui::Margin::same(PAD as i8))
}

pub fn section(ui: &mut Ui, title: &str) {
    ui.add_space(SECTION);
    ui.label(egui::RichText::new(title).font(font(11.0, SEMIBOLD)).color(white(120)));
    ui.add_space(CAPTION);
}

// A pane's first line is as high as the close button beside it, so the button fills the corner's padding exactly.
fn head(ui: &mut Ui, text: &str, font: FontId, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), CLOSE), Sense::hover());
    ui.painter()
        .text(rect.left_center(), egui::Align2::LEFT_CENTER, text, font, color);
}

pub fn title(ui: &mut Ui, text: &str, size: f32) {
    head(ui, text, font(size, SEMIBOLD), Color32::WHITE);
}

// A heading of a pane without a close button: a line of its own height under the pane's padding.
pub fn caption(ui: &mut Ui, title: &str) {
    ui.label(egui::RichText::new(title).font(font(11.0, SEMIBOLD)).color(white(120)));
    ui.add_space(HEAD);
}

pub fn heading(ui: &mut Ui, title: &str) {
    head(ui, title, font(11.0, SEMIBOLD), white(120));
    ui.add_space(HEAD);
}

pub fn row(ui: &mut Ui, text: &str, selected: bool) -> bool {
    locked_row(ui, text, selected, false)
}

fn lock(painter: &egui::Painter, at: egui::Pos2, color: Color32) {
    let shackle = egui::Rect::from_center_size(at - vec2(0.0, 3.0), vec2(6.0, 8.0));
    let round = CornerRadius {
        nw: 3,
        ne: 3,
        sw: 0,
        se: 0,
    };
    painter.rect_stroke(shackle, round, egui::Stroke::new(1.4, color), egui::StrokeKind::Middle);
    let body = egui::Rect::from_center_size(at + vec2(0.0, 2.5), vec2(11.0, 8.0));
    painter.rect_filled(body, CornerRadius::same(2), color);
}

pub fn locked_row(ui: &mut Ui, text: &str, selected: bool, locked: bool) -> bool {
    let (rect, response) = plain_row(ui, text, selected);
    if locked {
        lock(ui.painter(), rect.right_center() - vec2(16.0, 0.0), white(150));
    }
    response.clicked()
}

pub fn marked_row(ui: &mut Ui, text: &str, selected: bool, mark: Option<&str>) -> bool {
    let (rect, response) = plain_row(ui, text, selected);
    match mark {
        Some(hint) => {
            let at = rect.right_center() - vec2(16.0, 0.0);
            ui.painter().circle_filled(at, 3.5, Color32::from_rgb(255, 190, 90));
            response.on_hover_text(hint).clicked()
        }
        None => response.clicked(),
    }
}

fn plain_row(ui: &mut Ui, text: &str, selected: bool) -> (egui::Rect, egui::Response) {
    let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), 30.0), Sense::click());
    row_text(ui, rect, &response, text, selected, 12.0, rect.width() - 40.0);
    (rect, response)
}

fn row_text(ui: &Ui, rect: egui::Rect, response: &egui::Response, text: &str, selected: bool, indent: f32, room: f32) {
    let fill = if selected { 40 } else { lift(0, 18.0, lit(ui, response)) };
    if fill > 0 {
        ui.painter().rect_filled(rect, CornerRadius::same(9), white(fill));
    }
    let (font, color) = if selected {
        (font(14.0, MEDIUM), white(255))
    } else {
        (FontId::proportional(14.0), white(200))
    };
    let mut job = egui::text::LayoutJob::simple_singleline(text.to_owned(), font, color);
    job.wrap = egui::text::TextWrapping::truncate_at_width(room);
    let galley = ui.painter().layout_job(job);
    let at = rect.left_center() + vec2(indent, -galley.size().y * 0.5);
    ui.painter().galley(at, galley, color);
}

pub const PICTURE: egui::Vec2 = vec2(88.0, 55.0);

// A row with a small picture before its text and a dim note at its right edge.
pub fn picture_row(
    ui: &mut Ui,
    text: &str,
    note: Option<&str>,
    picture: Option<egui::TextureId>,
    selected: bool,
) -> bool {
    let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), PICTURE.y + 8.0), Sense::click());
    let note = note.map(|note| {
        ui.painter()
            .layout_no_wrap(note.to_owned(), FontId::proportional(12.0), white(120))
    });
    let noted = note.as_ref().map_or(0.0, |note| note.size().x + 10.0);
    let indent = PICTURE.x + 16.0;
    row_text(
        ui,
        rect,
        &response,
        text,
        selected,
        indent,
        rect.width() - indent - 12.0 - noted,
    );
    if let Some(note) = note {
        let at = rect.right_center() - vec2(12.0 + note.size().x, note.size().y * 0.5);
        ui.painter().galley(at, note, white(120));
    }
    let frame = egui::Rect::from_min_size(rect.min + vec2(4.0, 4.0), PICTURE);
    let round = CornerRadius::same(7);
    let arrived = glided(ui, response.id.with("picture"), picture.is_some());
    if arrived < 1.0 {
        ui.painter().rect_filled(frame, round, white(18));
    }
    if let Some(id) = picture {
        egui::Image::from_texture(egui::load::SizedTexture::new(id, frame.size()))
            .corner_radius(round)
            .tint(Color32::WHITE.gamma_multiply(arrived))
            .paint_at(ui, frame);
    }
    let edge = egui::Stroke::new(0.6, white(70));
    ui.painter().rect_stroke(frame, round, edge, egui::StrokeKind::Inside);
    response.clicked()
}

pub const BACK: f32 = 38.0;

pub fn back(ui: &mut Ui, hint: &str) -> bool {
    let (rect, response) = ui.allocate_exact_size(vec2(BACK, BACK), Sense::click());
    let fill = if response.is_pointer_button_down_on() {
        20
    } else {
        lift(30, 20.0, lit(ui, &response))
    };
    ui.painter().circle_filled(rect.center(), BACK * 0.5, white(fill));
    let ink = egui::Stroke::new(1.6, white(240));
    let tip = rect.center() - vec2(6.5, 0.0);
    for to in [vec2(13.0, 0.0), vec2(5.5, -5.5), vec2(5.5, 5.5)] {
        ui.painter().line_segment([tip, tip + to], ink);
    }
    response.on_hover_text(hint).clicked()
}

pub const CLOSE: f32 = 28.0;

pub fn line(ui: &Ui, size: f32) -> f32 {
    ui.fonts_mut(|fonts| fonts.row_height(&font(size, SEMIBOLD)))
}

pub fn close_spot(top: f32, line: f32, right: f32) -> egui::Pos2 {
    egui::pos2(right - CLOSE * 0.5, top + line * 0.5)
}

pub fn close(ui: &mut Ui, hint: &str) -> bool {
    let rect = ui.min_rect();
    close_at(ui, close_spot(rect.top(), CLOSE, rect.right()), hint)
}

pub fn close_at(ui: &mut Ui, center: egui::Pos2, hint: &str) -> bool {
    // A finger needs 44 points; the circle drawn is smaller.
    let touch = egui::Rect::from_center_size(center, egui::Vec2::splat(44.0));
    let response = ui.interact(touch, ui.scope_id().with("close"), Sense::click());
    let fill = if response.is_pointer_button_down_on() {
        14
    } else {
        lift(30, 20.0, lit(ui, &response))
    };
    ui.painter().circle_filled(center, CLOSE * 0.5, white(fill));
    let ink = egui::Stroke::new(1.5, white(240));
    for turn in [1.0, -1.0] {
        ui.painter()
            .line_segment([center + vec2(-4.0, -4.0 * turn), center + vec2(4.0, 4.0 * turn)], ink);
    }
    response.on_hover_text(hint).clicked()
}

fn round(ui: &mut Ui, kind: &str, enabled: bool) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(vec2(28.0, 28.0), Sense::click());
    let fill = lift(26, if enabled { 24.0 } else { 0.0 }, lit(ui, &response));
    ui.painter().circle_filled(rect.center(), 14.0, white(fill));
    let ink = egui::Stroke::new(1.4, white(if enabled { 235 } else { 70 }));
    let (c, r) = (rect.center(), 4.5);
    let line = |from: egui::Vec2, to: egui::Vec2| {
        ui.painter().line_segment([c + from, c + to], ink);
    };
    match kind {
        "add" => {
            line(vec2(-r, 0.0), vec2(r, 0.0));
            line(vec2(0.0, -r), vec2(0.0, r));
        }
        "remove" => {
            line(vec2(-r + 1.0, -r + 1.0), vec2(r - 1.0, r - 1.0));
            line(vec2(-r + 1.0, r - 1.0), vec2(r - 1.0, -r + 1.0));
        }
        "up" => {
            line(vec2(-r, 2.5), vec2(0.0, -2.5));
            line(vec2(r, 2.5), vec2(0.0, -2.5));
        }
        _ => {
            line(vec2(-r, -2.5), vec2(0.0, 2.5));
            line(vec2(r, -2.5), vec2(0.0, 2.5));
        }
    }
    response
}

pub const DOT: f32 = 16.0;
const DOT_GAP: f32 = 8.0;
const ROW_EDGE: f32 = 12.0;
// A name with less room than this, an ellipsis and a letter, is left out: it would lie over the buttons.
const NAME_LEAST: f32 = 24.0;

pub type Dot<'a> = &'a dyn Fn(&egui::Painter, egui::Pos2, f32);

// Along a row of the layers, from its left edge: the middle of the dot, where the name starts and its room.
pub fn dotted(width: f32) -> (f32, f32, f32) {
    let start = ROW_EDGE + DOT + DOT_GAP;
    (ROW_EDGE + DOT * 0.5, start, (width - start - ROW_EDGE).max(0.0))
}

pub fn dot_row(ui: &mut Ui, text: &str, dot: Dot) -> bool {
    let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), 30.0), Sense::click());
    let (middle, start, room) = dotted(rect.width());
    row_text(ui, rect, &response, text, false, start, room);
    dot(ui.painter(), rect.left_center() + vec2(middle, 0.0), DOT * 0.5);
    response.clicked()
}

pub struct Head {
    pub act: Option<&'static str>,
    pub dot: egui::Rect,
    pub name: egui::Rect,
    pub buttons: [egui::Rect; 3],
}

// The chosen layer's first line: the dot and the name from the left, the buttons from the right.
pub fn layer_head(ui: &mut Ui, text: &str, enabled: [bool; 3], dot: Dot) -> Head {
    let mut head = Head {
        act: None,
        dot: egui::Rect::NOTHING,
        name: egui::Rect::NOTHING,
        buttons: [egui::Rect::NOTHING; 3],
    };
    ui.horizontal(|ui| {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            for (at, kind) in ["remove", "down", "up"].into_iter().enumerate() {
                let response = round(ui, kind, enabled[at]);
                if enabled[at] && response.clicked() {
                    head.act = Some(kind);
                }
                head.buttons[at] = response.rect;
            }
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = DOT_GAP;
                let (rect, _) = ui.allocate_exact_size(vec2(DOT, DOT), Sense::hover());
                dot(ui.painter(), rect.center(), DOT * 0.5);
                head.dot = rect;
                let name = egui::RichText::new(text).font(font(14.0, MEDIUM)).color(Color32::WHITE);
                if ui.available_width() >= NAME_LEAST {
                    head.name = ui.add(egui::Label::new(name).truncate()).rect;
                }
            });
        });
    });
    head
}

pub fn dropdown(ui: &mut Ui, text: &str) -> bool {
    let mark = egui::pos2(
        ui.cursor().min.x + ui.available_width() - 16.0,
        ui.cursor().min.y + 15.0,
    );
    let clicked = row(ui, text, true);
    let ink = egui::Stroke::new(1.4, white(210));
    ui.painter()
        .line_segment([mark + vec2(-4.5, -2.5), mark + vec2(0.0, 2.5)], ink);
    ui.painter()
        .line_segment([mark + vec2(4.5, -2.5), mark + vec2(0.0, 2.5)], ink);
    clicked
}

pub fn segments(ui: &mut Ui, labels: &[String], chosen: usize) -> Option<usize> {
    segments_of(ui, labels, chosen, &[])
}

// The same with some of the segments not to be had: `off` says which, and those are dim and take no click.
pub fn segments_of(ui: &mut Ui, labels: &[String], chosen: usize, off: &[bool]) -> Option<usize> {
    let (track, _) = ui.allocate_exact_size(vec2(ui.available_width(), 32.0), Sense::hover());
    ui.painter().rect_filled(track, CornerRadius::same(10), white(18));
    let each = track.width() / labels.len() as f32;
    let widest = labels
        .iter()
        .map(|label| {
            let galley = ui
                .painter()
                .layout_no_wrap(label.clone(), font(13.5, MEDIUM), Color32::WHITE);
            galley.size().x
        })
        .fold(1.0, f32::max);
    let size = (13.5 * ((each - 14.0) / widest).min(1.0)).max(8.0);
    let mut picked = None;
    // The mark of the chosen one glides to the next instead of jumping there.
    let glide = ui.scope_id().with(("segments", labels.first()));
    let marked = ui
        .ctx()
        .animate_value_with_time(glide, chosen as f32, motion::span(motion::SMALL));
    let mark = egui::Rect::from_min_size(track.min + vec2(each * marked, 0.0), vec2(each, track.height()));
    ui.painter()
        .rect_filled(mark.shrink(2.0), CornerRadius::same(8), white(52));
    for (index, label) in labels.iter().enumerate() {
        let cell = egui::Rect::from_min_size(track.min + vec2(each * index as f32, 0.0), vec2(each, track.height()));
        // Two switches in one panel must not share ids: a click on one would count for the other.
        let response = ui.interact(
            cell,
            ui.scope_id().with(("segment", labels.first(), index)),
            Sense::click(),
        );
        let selected = index == chosen;
        if off.get(index).copied().unwrap_or(false) {
            let font = FontId::proportional(size);
            ui.painter()
                .text(cell.center(), Align2::CENTER_CENTER, label, font, white(70));
            continue;
        }
        let fill = lift(0, 16.0, lit(ui, &response));
        if !selected && fill > 0 {
            ui.painter()
                .rect_filled(cell.shrink(2.0), CornerRadius::same(8), white(fill));
        }
        let (font, color) = if selected {
            (font(size, MEDIUM), white(255))
        } else {
            (FontId::proportional(size), white(190))
        };
        ui.painter()
            .text(cell.center(), Align2::CENTER_CENTER, label, font, color);
        if response.clicked() {
            picked = Some(index);
        }
    }
    picked
}

// A choice among named values, under the name of what is chosen: a switch for up to three, a list for more.
pub fn choice(ui: &mut Ui, label: &str, names: &[String], chosen: usize) -> Option<usize> {
    ui.add_space(4.0);
    let (header, _) = ui.allocate_exact_size(vec2(ui.available_width(), 16.0), Sense::hover());
    ui.painter().text(
        header.left_center(),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(13.0),
        white(190),
    );
    ui.add_space(4.0);
    let picked = if names.len() <= 3 {
        segments(ui, names, chosen)
    } else {
        (0..names.len()).fold(None, |picked, index| {
            if row(ui, &names[index], index == chosen) {
                Some(index)
            } else {
                picked
            }
        })
    };
    ui.add_space(4.0);
    picked.filter(|index| *index != chosen)
}

pub fn add_row(ui: &mut Ui, enabled: bool) -> bool {
    let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), 30.0), Sense::click());
    let fill = lift(12, if enabled { 18.0 } else { 0.0 }, lit(ui, &response));
    ui.painter().rect_filled(rect, CornerRadius::same(9), white(fill));
    let ink = egui::Stroke::new(1.5, white(if enabled { 220 } else { 70 }));
    let c = rect.center();
    ui.painter().line_segment([c - vec2(5.0, 0.0), c + vec2(5.0, 0.0)], ink);
    ui.painter().line_segment([c - vec2(0.0, 5.0), c + vec2(0.0, 5.0)], ink);
    enabled && response.clicked()
}

fn fitted(ui: &Ui, text: &str, mut font: FontId, color: Color32, room: f32) -> Arc<egui::Galley> {
    let galley = ui.painter().layout_no_wrap(text.to_owned(), font.clone(), color);
    if galley.size().x <= room || room <= 0.0 {
        return galley;
    }
    font.size = (font.size * room / galley.size().x).max(8.0);
    ui.painter().layout_no_wrap(text.to_owned(), font, color)
}

pub fn pill_width(ui: &Ui, text: &str) -> f32 {
    let galley = ui
        .painter()
        .layout_no_wrap(text.to_owned(), font(14.0, MEDIUM), Color32::WHITE);
    galley.size().x + 2.0 * ui.spacing().button_padding.x
}

pub fn pill(ui: &mut Ui, text: &str, primary: bool, min_width: f32) -> bool {
    let color = if primary {
        Color32::from_rgb(14, 18, 34)
    } else {
        white(240)
    };
    let padding = ui.spacing().button_padding.x;
    let (galley, width) = if min_width > 0.0 {
        (
            fitted(ui, text, font(14.0, MEDIUM), color, min_width - 2.0 * padding.min(12.0)),
            min_width,
        )
    } else {
        let galley = ui.painter().layout_no_wrap(text.to_owned(), font(14.0, MEDIUM), color);
        let width = galley.size().x + 2.0 * padding;
        (galley, width)
    };
    let (rect, response) = ui.allocate_exact_size(vec2(width, 38.0), Sense::click());
    let by = if response.is_pointer_button_down_on() {
        -10.0
    } else {
        20.0 * lit(ui, &response)
    };
    let fill = white(lift(if primary { 225 } else { 30 }, by, 1.0));
    ui.painter().rect_filled(rect, CornerRadius::same(19), fill);
    ui.painter().galley(rect.center() - galley.size() * 0.5, galley, color);
    response.clicked()
}

pub fn toggle(ui: &mut Ui, label: &str, on: &mut bool) -> bool {
    let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), 30.0), Sense::click());
    if response.clicked() {
        *on = !*on;
    }
    let track = egui::Rect::from_center_size(egui::pos2(rect.right() - 19.0, rect.center().y), vec2(38.0, 22.0));
    let ink = white(if response.hovered() { 230 } else { 190 });
    let galley = fitted(
        ui,
        label,
        FontId::proportional(13.0),
        ink,
        track.left() - rect.left() - 10.0,
    );
    let painter = ui.painter();
    painter.galley(
        egui::pos2(rect.left(), rect.center().y - galley.size().y * 0.5),
        galley,
        ink,
    );
    let turned = glided(ui, response.id.with("on"), *on);
    painter.rect_filled(track, CornerRadius::same(11), white(lift(45, 170.0, turned)));
    let at = egui::lerp(track.left() + 11.0..=track.right() - 11.0, turned);
    let knob = Color32::WHITE.lerp_to_gamma(Color32::from_rgb(14, 18, 34), turned);
    painter.circle_filled(egui::pos2(at, track.center().y), 8.0, knob);
    response.clicked()
}

pub fn text_field(ui: &mut Ui, label: &str, value: &mut String, limit: usize, valid: bool) -> egui::Response {
    ui.add_space(4.0);
    let (header, _) = ui.allocate_exact_size(vec2(ui.available_width(), 16.0), Sense::hover());
    ui.painter().text(
        header.left_center(),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(13.0),
        white(190),
    );
    ui.add_space(2.0);
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 30.0), Sense::hover());
    ui.painter().rect_filled(rect, CornerRadius::same(8), white(26));
    let field = egui::TextEdit::singleline(value)
        .frame(egui::Frame::NONE)
        .font(font(13.0, MEDIUM))
        .text_color(if valid {
            Color32::WHITE
        } else {
            Color32::from_rgb(255, 150, 140)
        })
        .desired_width(rect.width() - 20.0)
        .char_limit(limit);
    ui.put(rect.shrink2(vec2(10.0, 4.0)), field)
}

pub fn slider(ui: &mut Ui, label: &str, value: &mut f64, range: (f64, f64), logarithmic: bool) -> bool {
    slider_to(ui, label, value, range, logarithmic, 2)
}

pub fn slider_to(
    ui: &mut Ui,
    label: &str,
    value: &mut f64,
    range: (f64, f64),
    logarithmic: bool,
    digits: usize,
) -> bool {
    let shown = format!("{value:.digits$}");
    slider_as(ui, label, value, range, logarithmic, &shown, |share, _| share)
}

// A slider that shows its value as the text given. `settle` gets where the knob belongs, as a share of the
// track, and whether the slider is held, and says where the knob is drawn.
pub fn slider_as(
    ui: &mut Ui,
    label: &str,
    value: &mut f64,
    range: (f64, f64),
    logarithmic: bool,
    shown: &str,
    settle: impl FnOnce(f32, bool) -> f32,
) -> bool {
    let (low, high) = range;
    let to_share = |v: f64| {
        if logarithmic {
            math::ln(v / low) / math::ln(high / low)
        } else {
            (v - low) / (high - low)
        }
    };
    let from_share = |t: f64| {
        if t <= 0.0 {
            low
        } else if t >= 1.0 {
            high
        } else if logarithmic {
            low * math::pow(high / low, t)
        } else {
            low + (high - low) * t
        }
    };

    ui.add_space(4.0);
    let (header, _) = ui.allocate_exact_size(vec2(ui.available_width(), 16.0), Sense::hover());
    ui.painter().text(
        header.left_center(),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(13.0),
        white(190),
    );
    ui.painter().text(
        header.right_center(),
        Align2::RIGHT_CENTER,
        shown,
        font(13.0, MEDIUM),
        white(240),
    );

    let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), 22.0), Sense::click_and_drag());
    let knob = 8.0;
    let (start, end) = (rect.left() + knob, rect.right() - knob);
    let mut changed = false;
    // A finger that lands on a slider while it scrolls the panel must not move the knob: the slider waits for
    // the finger's direction, and a vertical one scrolls.
    let mut sliding = true;
    if ui.input(|input| input.any_touches()) {
        let id = response.id.with("gesture");
        let (undecided, slide, scroll) = (0u8, 1u8, 2u8);
        let mut gesture = ui.data(|data| data.get_temp(id)).unwrap_or(undecided);
        if response.dragged() {
            if gesture == undecided {
                let moved = ui
                    .input(|input| input.pointer.press_origin().zip(input.pointer.interact_pos()))
                    .map_or(egui::Vec2::ZERO, |(from, to)| to - from);
                if moved.length() > 8.0 {
                    gesture = if moved.x.abs() > moved.y.abs() { slide } else { scroll };
                }
            }
            if gesture == scroll {
                ui.scroll_with_delta(vec2(0.0, response.drag_delta().y));
            }
        } else {
            gesture = undecided;
        }
        ui.data_mut(|data| data.insert_temp(id, gesture));
        sliding = gesture == slide || response.clicked();
    }
    if sliding && let Some(pointer) = response.interact_pointer_pos() {
        let share = f64::from(((pointer.x - start) / (end - start)).clamp(0.0, 1.0));
        let next = from_share(share);
        if next != *value {
            *value = next;
            changed = true;
        }
    }
    let share = settle(to_share(*value).clamp(0.0, 1.0) as f32, response.dragged());
    let at = start + (end - start) * share.clamp(0.0, 1.0);
    let middle = rect.center().y;
    let track =
        |from: f32, to: f32| egui::Rect::from_min_max(egui::pos2(from, middle - 2.0), egui::pos2(to, middle + 2.0));
    let painter = ui.painter();
    painter.rect_filled(track(rect.left(), rect.right()), CornerRadius::same(2), white(45));
    painter.rect_filled(track(rect.left(), at), CornerRadius::same(2), white(215));
    let held = response.hovered() || response.dragged();
    let grown = 1.5 * glided(ui, response.id.with("held"), held);
    painter.circle_filled(
        egui::pos2(at, middle + 1.5),
        knob + grown + 1.0,
        Color32::from_black_alpha(70),
    );
    painter.circle_filled(egui::pos2(at, middle), knob + grown, Color32::WHITE);
    changed
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inside(pane: egui::Rect, radius: f32, at: egui::Pos2) -> bool {
        let reach = CLOSE * 0.5;
        let core = pane.shrink(radius);
        let nearest = egui::pos2(
            at.x.clamp(core.left(), core.right()),
            at.y.clamp(core.top(), core.bottom()),
        );
        pane.shrink(reach).contains(at) && (at - nearest).length() + reach <= radius
    }

    #[test]
    fn the_close_button_fills_the_corner_of_the_padding() {
        for width in [120.0, 196.0, 260.0, 300.0, 604.0] {
            let content = egui::Rect::from_min_size(egui::pos2(40.0, 60.0), vec2(width, 200.0));
            let pane = content.expand(PAD);
            let at = close_spot(content.top(), CLOSE, content.right());
            let (above, beside) = (at.y - CLOSE * 0.5 - pane.top(), pane.right() - at.x - CLOSE * 0.5);
            assert_eq!((above, beside), (PAD, PAD));
            assert!(inside(pane, 30.0, at), "{width}");
        }
    }

    #[test]
    fn beside_a_taller_title_the_close_button_stands_on_its_line() {
        let content = egui::Rect::from_min_size(egui::pos2(34.0, 34.0), vec2(292.0, 700.0));
        let pane = content.expand(PAD);
        let at = close_spot(content.top(), 36.0, content.right());
        assert_eq!(at.y, content.top() + 18.0);
        assert_eq!(pane.right() - at.x - CLOSE * 0.5, PAD);
        assert!(inside(pane, 30.0, at));
    }

    #[test]
    fn a_layer_s_dot_its_name_and_its_buttons_never_overlap() {
        for width in [120.0, 180.0, 204.0, 224.0, 236.0] {
            let (middle, start, room) = dotted(width);
            assert!(middle - DOT * 0.5 >= ROW_EDGE && middle + DOT * 0.5 + DOT_GAP <= start);
            assert!(start + room <= width - ROW_EDGE, "{width}");
        }
        let names = || {
            let flames = fractique_core::presets::names().into_iter().map(str::to_owned);
            flames.chain(fractique_core::fractal::preset_names())
        };
        for lang in Lang::ALL {
            let texts = crate::i18n::Texts::new(lang);
            let longest = names()
                .map(|name| crate::displayed(&texts, &name))
                .max_by_key(|name| name.chars().count())
                .expect("there are presets");
            let ctx = egui::Context::default();
            install_fonts(&ctx, lang);
            for width in [120.0, 150.0, 180.0, 204.0] {
                for name in [longest.as_str(), "x"] {
                    let mut head = None;
                    for _ in 0..2 {
                        let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
                            ui.scope_builder(
                                egui::UiBuilder::new()
                                    .max_rect(egui::Rect::from_min_size(egui::pos2(10.0, 10.0), vec2(width, 40.0))),
                                |ui| {
                                    ui.set_width(width);
                                    let shown = layer_head(ui, name, [true; 3], &|_, _, _| ());
                                    head = Some((shown, ui.min_rect()));
                                },
                            );
                        });
                    }
                    let (head, row) = head.expect("the row was laid out");
                    let mut parts = vec![head.dot, head.name];
                    parts.extend(head.buttons);
                    for (at, one) in parts.iter().enumerate() {
                        let named = at != 1 || width >= 150.0;
                        assert!(one.width() > 0.0 || !named, "{} {width} {name}: {parts:?}", lang.code());
                        if one.width() <= 0.0 {
                            continue;
                        }
                        assert!(
                            row.expand(0.5).contains_rect(*one) && row.width() <= width + 0.5,
                            "{} {width} {name}: {one:?} in {row:?}",
                            lang.code()
                        );
                        for other in &parts[at + 1..] {
                            assert!(
                                !one.shrink(0.5).intersects(other.shrink(0.5)),
                                "{} {width} {name}: {one:?} over {other:?}",
                                lang.code()
                            );
                        }
                    }
                    assert!(head.name.width() <= 0.0 || head.name.left() - head.dot.right() >= DOT_GAP - 0.5);
                    assert_eq!(head.dot.size(), vec2(DOT, DOT));
                }
            }
        }
    }

    #[test]
    fn the_measures_of_a_pane_are_these() {
        assert_eq!(
            (PAD, OUTER, SECTION, CAPTION, HEAD, CLOSE),
            (22.0, 12.0, 14.0, 2.0, 4.0, 28.0)
        );
    }
}

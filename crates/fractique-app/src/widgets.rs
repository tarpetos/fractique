use std::sync::Arc;

use fractique_core::math;

use crate::i18n::Lang;

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

pub fn section(ui: &mut Ui, title: &str) {
    ui.add_space(14.0);
    ui.label(egui::RichText::new(title).font(font(11.0, SEMIBOLD)).color(white(120)));
    ui.add_space(2.0);
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
    let fill = if selected {
        40
    } else if response.hovered() {
        18
    } else {
        0
    };
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
    match picture {
        Some(id) => {
            egui::Image::from_texture(egui::load::SizedTexture::new(id, frame.size()))
                .corner_radius(round)
                .paint_at(ui, frame);
        }
        None => {
            ui.painter().rect_filled(frame, round, white(18));
        }
    }
    let edge = egui::Stroke::new(0.6, white(70));
    ui.painter().rect_stroke(frame, round, edge, egui::StrokeKind::Inside);
    response.clicked()
}

pub const BACK: f32 = 38.0;

pub fn back(ui: &mut Ui, hint: &str) -> bool {
    let (rect, response) = ui.allocate_exact_size(vec2(BACK, BACK), Sense::click());
    let lift = match (response.is_pointer_button_down_on(), response.hovered()) {
        (true, _) => 20,
        (false, true) => 50,
        _ => 30,
    };
    ui.painter().circle_filled(rect.center(), BACK * 0.5, white(lift));
    let ink = egui::Stroke::new(1.6, white(240));
    let tip = rect.center() - vec2(6.5, 0.0);
    for to in [vec2(13.0, 0.0), vec2(5.5, -5.5), vec2(5.5, 5.5)] {
        ui.painter().line_segment([tip, tip + to], ink);
    }
    response.on_hover_text(hint).clicked()
}

pub const CLOSE: f32 = 28.0;

pub fn heading(ui: &mut Ui, title: &str) {
    let around = (CLOSE - line(ui, 11.0)) * 0.5;
    ui.add_space(around);
    ui.label(egui::RichText::new(title).font(font(11.0, SEMIBOLD)).color(white(120)));
    ui.add_space(around + 4.0);
}

pub fn line(ui: &Ui, size: f32) -> f32 {
    ui.fonts_mut(|fonts| fonts.row_height(&font(size, SEMIBOLD)))
}

pub fn close_spot(top: f32, line: f32, right: f32) -> egui::Pos2 {
    egui::pos2(right - CLOSE * 0.5, top + line * 0.5)
}

pub fn close(ui: &mut Ui, line: f32, hint: &str) -> bool {
    let rect = ui.min_rect();
    close_at(ui, close_spot(rect.top(), line, rect.right()), hint)
}

pub fn close_at(ui: &mut Ui, center: egui::Pos2, hint: &str) -> bool {
    // A finger needs 44 points; the circle drawn is smaller.
    let touch = egui::Rect::from_center_size(center, egui::Vec2::splat(44.0));
    let response = ui.interact(touch, ui.scope_id().with("close"), Sense::click());
    let lift = match (response.is_pointer_button_down_on(), response.hovered()) {
        (true, _) => 14,
        (false, true) => 50,
        _ => 30,
    };
    ui.painter().circle_filled(center, CLOSE * 0.5, white(lift));
    let ink = egui::Stroke::new(1.5, white(240));
    for turn in [1.0, -1.0] {
        ui.painter()
            .line_segment([center + vec2(-4.0, -4.0 * turn), center + vec2(4.0, 4.0 * turn)], ink);
    }
    response.on_hover_text(hint).clicked()
}

pub fn round(ui: &mut Ui, kind: &str, enabled: bool) -> bool {
    let (rect, response) = ui.allocate_exact_size(vec2(28.0, 28.0), Sense::click());
    let lift = if enabled && response.hovered() { 24 } else { 0 };
    ui.painter().circle_filled(rect.center(), 14.0, white(26 + lift));
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
    enabled && response.clicked()
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
    for (index, label) in labels.iter().enumerate() {
        let cell = egui::Rect::from_min_size(track.min + vec2(each * index as f32, 0.0), vec2(each, track.height()));
        // Two switches in one panel must not share ids: a click on one would count for the other.
        let response = ui.interact(
            cell,
            ui.scope_id().with(("segment", labels.first(), index)),
            Sense::click(),
        );
        let selected = index == chosen;
        if selected {
            ui.painter()
                .rect_filled(cell.shrink(2.0), CornerRadius::same(8), white(52));
        } else if response.hovered() {
            ui.painter()
                .rect_filled(cell.shrink(2.0), CornerRadius::same(8), white(16));
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
    let lift = if enabled && response.hovered() { 18 } else { 0 };
    ui.painter().rect_filled(rect, CornerRadius::same(9), white(12 + lift));
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
    let lift = if response.is_pointer_button_down_on() {
        -10
    } else if response.hovered() {
        20
    } else {
        0
    };
    let fill = if primary {
        white((225 + lift).clamp(0, 255) as u8)
    } else {
        white((30 + lift).clamp(0, 255) as u8)
    };
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
    painter.rect_filled(track, CornerRadius::same(11), white(if *on { 215 } else { 45 }));
    let (at, knob) = if *on {
        (track.right() - 11.0, Color32::from_rgb(14, 18, 34))
    } else {
        (track.left() + 11.0, Color32::WHITE)
    };
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
    let (low, high) = range;
    let to_share = |v: f64| {
        if logarithmic {
            math::ln(v / low) / math::ln(high / low)
        } else {
            (v - low) / (high - low)
        }
    };
    let from_share = |t: f64| {
        if logarithmic {
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
    let shown = format!("{value:.digits$}");
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
    let at = start + (end - start) * to_share(*value).clamp(0.0, 1.0) as f32;
    let middle = rect.center().y;
    let track =
        |from: f32, to: f32| egui::Rect::from_min_max(egui::pos2(from, middle - 2.0), egui::pos2(to, middle + 2.0));
    let painter = ui.painter();
    painter.rect_filled(track(rect.left(), rect.right()), CornerRadius::same(2), white(45));
    painter.rect_filled(track(rect.left(), at), CornerRadius::same(2), white(215));
    let grown = if response.hovered() || response.dragged() {
        1.5
    } else {
        0.0
    };
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
    fn the_close_button_stands_on_the_title_line_in_the_title_margin() {
        for (margin, radius, line) in [
            (22.0, 30.0, 20.0),
            (20.0, 30.0, 18.0),
            (22.0, 30.0, 36.0),
            (18.0, 28.0, 13.0),
        ] {
            for width in [120.0, 196.0, 260.0, 300.0, 604.0] {
                let content = egui::Rect::from_min_size(egui::pos2(40.0, 60.0), vec2(width, 200.0));
                let pane = content.expand(margin);
                let at = close_spot(content.top(), line, content.right());
                assert_eq!(at.y, content.top() + line * 0.5);
                assert_eq!(pane.right() - (at.x + CLOSE * 0.5), content.left() - pane.left());
                assert!(inside(pane, radius, at), "{margin} {radius} {line} {width}");
            }
        }
    }

    #[test]
    fn over_a_list_the_gap_above_the_close_button_is_the_gap_beside_it() {
        for (margin, radius) in [(22.0, 30.0), (10.0, 22.0)] {
            let content = egui::Rect::from_min_size(egui::pos2(40.0, 60.0), vec2(196.0, 300.0));
            let pane = content.expand(margin);
            let at = close_spot(content.top(), CLOSE, content.right());
            let (above, beside) = (at.y - CLOSE * 0.5 - pane.top(), pane.right() - at.x - CLOSE * 0.5);
            assert_eq!((above, beside), (margin, margin));
            assert!(inside(pane, radius, at));
        }
    }
}

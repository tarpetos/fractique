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
    let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), 30.0), Sense::click());
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
    let (family, color) = if selected {
        (MEDIUM, white(255))
    } else {
        ("inter", white(200))
    };
    let font = if selected {
        font(14.0, family)
    } else {
        FontId::proportional(14.0)
    };
    ui.painter().text(
        rect.left_center() + vec2(12.0, 0.0),
        Align2::LEFT_CENTER,
        text,
        font,
        color,
    );
    response.clicked()
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

pub fn slider(ui: &mut Ui, label: &str, value: &mut f64, range: (f64, f64), logarithmic: bool) -> bool {
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
    let shown = format!("{value:.2}");
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
    if let Some(pointer) = response.interact_pointer_pos() {
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

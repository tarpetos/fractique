//! The app's own controls, drawn to sit on glass.

use eframe::egui::{self, Align2, Color32, CornerRadius, FontFamily, FontId, Sense, Ui, vec2};

pub const MEDIUM: &str = "medium";
pub const SEMIBOLD: &str = "semibold";

pub fn install_fonts(ctx: &egui::Context) {
    use egui::epaint::text::{FontInsert, FontPriority, InsertFontFamily};
    let fonts: [(&str, &'static [u8], FontFamily); 3] = [
        (
            "inter",
            include_bytes!("../assets/fonts/Inter-Regular.ttf"),
            FontFamily::Proportional,
        ),
        (
            MEDIUM,
            include_bytes!("../assets/fonts/Inter-Medium.ttf"),
            FontFamily::Name(MEDIUM.into()),
        ),
        (
            SEMIBOLD,
            include_bytes!("../assets/fonts/Inter-SemiBold.ttf"),
            FontFamily::Name(SEMIBOLD.into()),
        ),
    ];
    for (name, bytes, family) in fonts {
        let families = vec![InsertFontFamily {
            family,
            priority: FontPriority::Highest,
        }];
        ctx.add_font(FontInsert::new(name, egui::FontData::from_static(bytes), families));
    }
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

/// A full-width row of a list; true when it was clicked.
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

/// A capsule button; true when it was clicked.
pub fn pill(ui: &mut Ui, text: &str, primary: bool) -> bool {
    let color = if primary {
        Color32::from_rgb(14, 18, 34)
    } else {
        white(240)
    };
    let galley = ui.painter().layout_no_wrap(text.to_owned(), font(14.0, MEDIUM), color);
    let (rect, response) = ui.allocate_exact_size(vec2(galley.size().x + 36.0, 38.0), Sense::click());
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

/// A labelled slider: the name and the value above a slim track. True when the value changed.
pub fn slider(ui: &mut Ui, label: &str, value: &mut f64, range: (f64, f64), logarithmic: bool) -> bool {
    let (low, high) = range;
    let to_share = |v: f64| {
        if logarithmic {
            (v / low).ln() / (high / low).ln()
        } else {
            (v - low) / (high - low)
        }
    };
    let from_share = |t: f64| {
        if logarithmic {
            low * (high / low).powf(t)
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

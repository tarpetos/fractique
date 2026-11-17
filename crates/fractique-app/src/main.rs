// A release build is a window program: without this Windows opens a console beside it.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

use std::path::PathBuf;

use eframe::egui;

fn main() -> eframe::Result {
    let mut args = std::env::args().skip(1);
    let mut screenshot = None;
    while let Some(arg) = args.next() {
        if arg == "--screenshot" {
            screenshot = args.next().map(PathBuf::from);
        }
    }
    let mut viewport = egui::ViewportBuilder::default()
        .with_title("frcq.")
        .with_inner_size([1280.0, 800.0])
        .with_icon(fractique_app::window_icon());
    if screenshot.is_some() {
        viewport = viewport.with_visible(false).with_taskbar(false).with_active(false);
    }
    fractique_app::run(
        eframe::NativeOptions {
            viewport,
            ..Default::default()
        },
        screenshot,
    )
}

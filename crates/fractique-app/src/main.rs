// A release build is a window program: without this Windows opens a console beside it.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result {
    use eframe::egui;
    use std::path::PathBuf;

    let mut args = std::env::args().skip(1);
    let mut screenshot = None;
    while let Some(arg) = args.next() {
        if arg == "--screenshot" {
            screenshot = args.next().map(PathBuf::from);
        }
    }
    let mut viewport = egui::ViewportBuilder::default()
        .with_title("Fractique")
        .with_inner_size([1280.0, 800.0])
        .with_icon(fractique_app::window_icon());
    if screenshot.is_some() {
        // A screenshot run is a test: it draws into a window nobody sees.
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

#[cfg(target_arch = "wasm32")]
fn main() {
    use eframe::wasm_bindgen::JsCast as _;
    use eframe::web_sys;

    wasm_bindgen_futures::spawn_local(async {
        let canvas = web_sys::window()
            .and_then(|window| window.document())
            .and_then(|document| document.get_element_by_id("canvas"))
            .and_then(|element| element.dyn_into::<web_sys::HtmlCanvasElement>().ok())
            .expect("the page has a canvas with the id 'canvas'");
        if let Err(error) = fractique_app::run_web(canvas).await {
            web_sys::console::error_1(&error);
        }
    });
}

// A release build is a window program: without this Windows opens a console beside it.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result {
    use eframe::egui;
    use std::path::PathBuf;

    fractique_app::started();

    let mut args = std::env::args().skip(1);
    let mut screenshot = None;
    let mut size = [1280.0, 800.0];
    let mut open = None;
    while let Some(arg) = args.next() {
        if arg == "--screenshot" {
            screenshot = args.next().map(PathBuf::from);
        } else if arg == "--size"
            && let Some((width, height)) = args.next().as_deref().and_then(|text| text.split_once('x'))
            && let (Ok(width), Ok(height)) = (width.parse::<f32>(), height.parse::<f32>())
        {
            size = [width, height];
        } else if !arg.starts_with("--") {
            open = Some(std::path::absolute(&arg).unwrap_or_else(|_| PathBuf::from(arg)));
        }
    }
    #[cfg(not(target_os = "ios"))]
    {
        let test = screenshot.is_some() || std::env::var_os("FRACTIQUE_DEBUG_FULL").is_some();
        let many = cfg!(debug_assertions) || std::env::var_os("FRACTIQUE_MANY").is_some();
        if !test && !many && !fractique_app::first_instance(open.as_deref()) {
            return Ok(());
        }
        if let Some(path) = open {
            fractique_app::open_at_start(path);
        }
    }
    #[cfg(target_os = "ios")]
    let _ = open;
    #[cfg(windows)]
    if screenshot.is_none() {
        fractique_app::register_scene_files();
    }
    let mut viewport = egui::ViewportBuilder::default()
        .with_title("Fractique")
        .with_inner_size(size)
        .with_min_inner_size([640.0, 480.0])
        .with_decorations(false)
        .with_icon(fractique_app::window_icon());
    // FRACTIQUE_VISIBLE: a test run in a window on the screen, for what only a real window shows.
    if screenshot.is_some() && std::env::var_os("FRACTIQUE_VISIBLE").is_none() {
        // A screenshot run is a test: it draws into a window nobody sees. eframe shows every window after its
        // first frame and draws nothing into a hidden one, so the window stands far outside every screen.
        viewport = viewport
            .with_visible(false)
            .with_taskbar(false)
            .with_active(false)
            .with_position([-30000.0, -30000.0]);
    }
    let mut options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    if screenshot.is_some() || std::env::var_os("FRACTIQUE_DEBUG_FULL").is_some() {
        // A test run must not change the window and settings the owner's next launch restores.
        options.persist_window = false;
        options.persistence_path = Some(std::env::temp_dir().join("fractique-screenshot.ron"));
    }
    fractique_app::run(options, screenshot)
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

# Vendored crates

Copies of three crates from crates.io with four fixes, wired in through `[patch.crates-io]` in the root `Cargo.toml`.
`eframe` and `egui-wgpu` keep their own licenses (MIT OR Apache-2.0), `winit` its own (Apache-2.0), in their folders.

| Crate | Version | Why |
| --- | --- | --- |
| `eframe` | 0.37.0 | On iOS, egui lays the UI out for the whole screen (`Window::outer_size`), but eframe resized the drawing surface to the safe area (`Window::inner_size`). The UI was cut off at the bottom and stretched by the height of the notch and the home bar. The surface now follows `outer_size` on iOS. |
| `egui-wgpu` | 0.37.0 | The same, where the surface is first created. |
| `eframe` | 0.37.0 | On Windows, a window whose edge is dragged was painted twice for every size: once in the event of the new size, and again for the redraw eframe asked for at once and for the system's own `WM_PAINT`. With the app's frame pacing the edge was followed 30 times a second on a 60 Hz screen. A size that was just painted is not painted again unless the app asked for a frame (`native/run.rs`). |
| `winit` | 0.30.13 | On macOS, winit linked two private functions of the window server, `CGSMainConnectionID` and `CGSSetWindowBackgroundBlurRadius`, for the blur behind a window (`Window::set_blur`). The app never asks for it, but the Mac App Store rejects every binary that names a private function (guideline 2.5.1). They are not linked, and `set_blur` does nothing (`platform_impl/macos/ffi.rs`, `window_delegate.rs`). |
| `eframe` | 0.37.0 | With `persist_window` off, eframe did not save the window's place and size but still restored the ones an earlier run had saved. Under a tiling window manager, where the app keeps no window, the size of an old tile came back with every start. A window that is not kept is not restored either (`native/wgpu_integration.rs`, `native/epi_integration.rs`). |

Every changed line is marked `Fractique patch`. The first fix changes nothing outside iOS, the second nothing outside Windows, the third nothing where `persist_window` is on, the winit one nothing but the blur behind a Mac window, which the app does not use.

When a newer crate has its fix, delete its folder and its line in `[patch.crates-io]`. To move to a newer
version without it: copy the crate from `~/.cargo/registry/src/*/` again and apply the marked lines.

# Vendored crates

Copies of two crates from crates.io with three fixes, wired in through `[patch.crates-io]` in the root `Cargo.toml`.
Both keep their own licenses (MIT OR Apache-2.0), in their folders.

| Crate | Version | Why |
| --- | --- | --- |
| `eframe` | 0.37.0 | On iOS, egui lays the UI out for the whole screen (`Window::outer_size`), but eframe resized the drawing surface to the safe area (`Window::inner_size`). The UI was cut off at the bottom and stretched by the height of the notch and the home bar. The surface now follows `outer_size` on iOS. |
| `egui-wgpu` | 0.37.0 | The same, where the surface is first created. |
| `eframe` | 0.37.0 | On Windows, a window whose edge is dragged was painted twice for every size: once in the event of the new size, and again for the redraw eframe asked for at once and for the system's own `WM_PAINT`. With the app's frame pacing the edge was followed 30 times a second on a 60 Hz screen. A size that was just painted is not painted again unless the app asked for a frame (`native/run.rs`). |
| `eframe` | 0.37.0 | With `persist_window` off, eframe did not save the window's place and size but still restored the ones an earlier run had saved. Under a tiling window manager, where the app keeps no window, the size of an old tile came back with every start. A window that is not kept is not restored either (`native/wgpu_integration.rs`, `native/epi_integration.rs`). |

Every changed line is marked `Fractique patch`. The first fix changes nothing outside iOS, the second nothing outside Windows, the third nothing where `persist_window` is on.

When a newer `eframe` has the fix, delete this folder and the `[patch.crates-io]` section. To move to a newer
version without it: copy the two crates from `~/.cargo/registry/src/*/` again and apply the marked lines.

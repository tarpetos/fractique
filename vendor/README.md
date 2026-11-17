# Vendored crates

Copies of two crates from crates.io with one fix, wired in through `[patch.crates-io]` in the root `Cargo.toml`.
Both keep their own licenses (MIT OR Apache-2.0), in their folders.

| Crate | Version | Why |
| --- | --- | --- |
| `eframe` | 0.37.0 | On iOS, egui lays the UI out for the whole screen (`Window::outer_size`), but eframe resized the drawing surface to the safe area (`Window::inner_size`). The UI was cut off at the bottom and stretched by the height of the notch and the home bar. The surface now follows `outer_size` on iOS. |
| `egui-wgpu` | 0.37.0 | The same, where the surface is first created. |

Every changed line is marked `Fractique patch`. Nothing changes on other platforms.

When a newer `eframe` has the fix, delete this folder and the `[patch.crates-io]` section. To move to a newer
version without it: copy the two crates from `~/.cargo/registry/src/*/` again and apply the marked lines.

# Roadmap

The order is chosen so that every step is usable on its own and the picture quality is settled before any UI is
built on it.

| Step | What | State |
| --- | --- | --- |
| 1 | **Core:** scene format, renderer, presets, randomizer, exact repeatability | Done |
| 2 | **CLI:** render scenes to PNG | Done |
| 3 | **MCP server** over stdio: presets, random scenes, mutation, validation, `.flame` import, rendering with the image returned ([mcp.md](mcp.md)) | Done |
| 4 | **App:** one codebase for Windows, macOS, Linux, Android, iOS and the web, with its own glass title bar on a computer; presets, randomize, mutate, layers editing, tone, picture sizes, export with sizes and quality, glass, 15 languages | Done |
| 5 | **Any zoom:** views magnified up to ten thousand million times, in the renderer and in the app (drag, wheel, pinch) | Done |
| 6 | **Wallpapers:** sheets in the scene format, a wallpaper for every seed from 14 layouts and 6 materials, light and dark; in the command line tool and the app | Done |
| 7 | **Phones:** a screen of its own on Android and iOS, a collection of kept pictures, saving to Photos or the gallery, sharing | Done |
| 8 | **Scene files and session:** the packed `.frcq` file, read by the app and the command line tool; the session kept between launches | Done |
| 9 | **Installers and updates:** one program file for Windows, an AppImage, a `.deb`, a macOS app and an Android APK built by the release workflow; the updates window ([releasing.md](releasing.md)) | Built; no version has been released |
| 10 | **iOS distribution:** runs in the Simulator and on an iPhone from a Mac, uploads to TestFlight ([ios.md](ios.md)); the App Store | In progress |
| 11 | **Server:** HTTP API with a job queue and limits, web UI (the app compiled to WebAssembly), Docker image | Planned |

## App, next

What the app does not do yet:

- Export and scene files on the web; scene files on phones.
- A GPU preview while a slider moves.
- Choosing a wallpaper's color on a computer (a phone has it in its styles).

## Renderer, next

- Wallpapers through MCP; cancelling a render and progress reports through MCP; MCP over HTTP with the server.
- More variations (the ones that take the transform's coefficients: waves, popcorn, rings, fan; and more
  parametric ones).
- More presets, and named palettes in scene files.
- Rendering in tiles, to lower the memory of large supersampled images.
- Animation: interpolating between two scenes.

## Decisions made

| Question | Decision |
| --- | --- |
| UI framework for the app | `egui` with `eframe` on `wgpu`: Rust, and it reaches the five platforms and WebAssembly. `vendor/` carries the two crates with a fix for the iOS drawing surface |
| Installers | `cargo-packager` for Linux and macOS. Windows gets no installer: the app is one file that replaces itself |
| The app's own file | `.frcq`: packed JSON, not a binary encoding of the scene, which came out larger |

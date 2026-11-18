# Roadmap

The order is chosen so that every step is usable on its own and the picture quality is settled before any UI is
built on it.

| Step | What | State |
| --- | --- | --- |
| 1 | **Core:** scene format, renderer, presets, randomizer, exact repeatability | Done |
| 2 | **CLI:** render scenes to PNG | Done |
| 3 | **MCP server** over stdio: presets, random scenes, mutation, validation, `.flame` import, wallpapers, rendering with the image returned, progress and cancelling ([mcp.md](mcp.md)) | Done |
| 4 | **App:** one codebase for Windows, macOS, Linux, Android, iOS and the web, with its own glass title bar on a computer; presets, randomize, mutate, layers editing, tone, picture sizes, export with sizes and quality, glass, 15 languages | Done |
| 5 | **Any zoom:** views magnified up to ten thousand million times, in the renderer and in the app (drag, wheel, pinch) | Done |
| 6 | **Wallpapers:** sheets in the scene format, a wallpaper for every seed from 14 layouts and 6 materials, light and dark; in the command line tool and the app | Done |
| 7 | **Phones:** a screen of its own on Android and iOS, a collection of kept pictures, saving to Photos or the gallery, sharing | Done |
| 8 | **Scene files and session:** the packed `.frcq` file, read by the app, the command line tool and the MCP server, turned into JSON and back by `fractique convert`, opened by a double click in the file manager; one window at a time on a computer; the session kept between launches | Done |
| 9 | **Installers and updates:** one program file for Windows, an AppImage, a `.deb`, a macOS app and an Android APK built by the release workflow; the updates window ([releasing.md](releasing.md)) | Built; no version has been released |
| 10 | **iOS distribution:** runs in the Simulator and on an iPhone from a Mac, uploads to TestFlight ([ios.md](ios.md)); the App Store | In progress |
| 11 | **Server:** HTTP API with a job queue and limits, the web UI (the app compiled to WebAssembly) served beside it, Docker image ([server.md](server.md)) | Done |
| 12 | **MCP over HTTP:** the tools of the MCP server at `/mcp` of the HTTP server (Streamable HTTP), from one implementation, under the limits and the queue of the server and without access to its disk ([mcp.md](mcp.md#over-http)) | Done |
| 13 | **GPU preview:** while a tone slider, the zoom, an opacity or a blend moves, the graphics card draws the picture from the rendered layers; the processor's picture replaces it when the control rests ([architecture.md](architecture.md#the-preview-on-the-graphics-card)) | Done; seen running on Linux with a software graphics card only |
| 14 | **The chaos game on the graphics card:** the first picture of a flame comes from the graphics card, at full size in a fraction of a second, and grows finer until the processor's exact picture replaces it ([architecture.md](architecture.md#the-chaos-game-on-the-graphics-card)) | Done; measured on Windows with an NVIDIA RTX 4070 and run on Linux with a software graphics card. Not run on macOS, on a phone or on an integrated graphics card |

## App, next

What the app does not do yet:

- Export and scene files on the web; scene files on phones.
- The chaos game on the graphics card in a browser: the page renders on its one thread, so the exact render would
  have to wait for the graphics card's picture first.
- Choosing a wallpaper's color on a computer (a phone has it in its styles).
- Opening a `.frcq` file by a double click on macOS: the files are shown as Fractique's, and no more.

## Renderer, next

- The second random generator in the MCP server and the HTTP server: both still make random scenes with the first.
- More presets, and named palettes in scene files.
- Rendering in tiles, to lower the memory of large supersampled images.
- Animation: interpolating between two scenes.

## Decisions made

| Question | Decision |
| --- | --- |
| What the graphics card may draw | Pictures that the processor's replace: a preview while a control moves, and the first picture of a flame. The picture that stays on the screen and every export are the processor's, so a scene still gives the same pixels everywhere |
| UI framework for the app | `egui` with `eframe` on `wgpu`: Rust, and it reaches the five platforms and WebAssembly. `vendor/` carries the two crates with a fix for the iOS drawing surface |
| Installers | `cargo-packager` for Linux and macOS. Windows gets no installer: the app is one file that replaces itself |
| The app's own file | `.frcq`: packed JSON, not a binary encoding of the scene, which came out larger |

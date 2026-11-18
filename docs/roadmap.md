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
| 10 | **iOS distribution:** runs in the Simulator and on an iPhone from a Mac, uploads to TestFlight ([ios.md](ios.md)); the App Store, where large and custom export sizes need the one-time purchase Fractique Pro ([ios.md](ios.md#fractique-pro-the-purchase)) | In progress: the purchase is written and compiles for iOS, and has not been run against the App Store |
| 11 | **Server:** HTTP API with a job queue and limits, the web UI (the app compiled to WebAssembly) served beside it, Docker image ([server.md](server.md)) | Done |
| 12 | **MCP over HTTP:** the tools of the MCP server at `/mcp` of the HTTP server (Streamable HTTP), from one implementation, under the limits and the queue of the server and without access to its disk ([mcp.md](mcp.md#over-http)) | Done |
| 13 | **GPU preview:** while a tone slider, the zoom, an opacity or a blend moves, the graphics card draws the picture from the rendered layers; the processor's picture replaces it when the control rests ([architecture.md](architecture.md#the-preview-on-the-graphics-card)) | Done; seen running on Linux with a software graphics card only |
| 14 | **The chaos game on the graphics card:** with GPU acceleration on, a flame is drawn by the graphics card alone, at full size in a fraction of a second, and the processor renders only exact exports; the switch exists only where a trial shows the card clearly faster than the processor ([architecture.md](architecture.md#the-chaos-game-on-the-graphics-card)) | Done; measured on Windows with an NVIDIA RTX 4070 and run on Linux with a software graphics card. Not run on macOS, on a phone or on an integrated graphics card |

| 15 | **Fractals of other kinds:** a layer is a flame or a fractal of a kind from a registry, drawn per pixel or as a density; the Mandelbrot set, Julia sets and the Clifford attractor, with presets, a random picture for every seed and mutation; a third tab in the app, `fractique fractals` and `--fractal`, the MCP tools and the HTTP server ([architecture.md](architecture.md#fractals-of-other-kinds)) | The foundation and three kinds are done. Next, on the same registry: Burning Ship, Tricorn, Multibrot, Phoenix, Magnet; Newton, Nova; Buddhabrot, De Jong, Lorenz, Gumowski-Mira, Hopalong; Apollonian gasket, Kleinian groups, L-systems, Lyapunov; Mandelbulb, Mandelbox, Menger sponge |
| 16 | **Escape-time and root-finding kinds:** Burning Ship, Tricorn, Multibrot, Phoenix and Magnet beside the Mandelbrot and Julia sets, and Newton and Nova, the first two of the root-finding family; each with six presets, a random picture for every seed and a mutation ([scene-format.md](scene-format.md#escape-time-kinds)) | Done. The golden hashes were produced on Linux |
| 17 | **Orbit kinds:** the Buddhabrot, with the Nebulabrot coloring and the Anti-Buddhabrot, and the De Jong, Lorenz, Gumowski-Mira and Hopalong attractors beside the Clifford attractor, which makes the orbit family whole; each with six presets, a random picture for every seed and a mutation ([scene-format.md](scene-format.md#orbit-kinds)) | Done. The golden hashes were produced on Linux |
| 18 | **Solids:** the Mandelbulb, the Mandelbox and the Menger sponge, ray-marched by distance estimates on the processor with light, soft shadows, fog and a halo; an eye that turns around the solid and goes inside; presets, random pictures measured before they are kept, mutation ([scene-format.md](scene-format.md#solid-kinds), [architecture.md](architecture.md#the-ray-marched-kinds)) | Done; times measured on 3 threads only |
| 19 | **Geometric fractals** on the same registry: the Apollonian gasket, Kleinian limit sets (Grandma's and Maskit's recipes), L-systems with their rules as text in the scene, the Lyapunov fractal ([scene-format.md](scene-format.md#geometric-kinds)) | Done |

## App, next

What the app does not do yet:

- Export and scene files on the web; scene files on phones.
- The chaos game on the graphics card in a browser: the page renders on its one thread, so the exact render would
  have to wait for the graphics card's picture first.
- Choosing a wallpaper's color on a computer (a phone has it in its styles).
- More fractals of other kinds on the graphics card: it draws flames and the Clifford, De Jong and Lorenz
  attractors ([architecture.md](architecture.md#orbits-on-the-graphics-card)); a scene with any other kind in it
  is rendered by the processor alone, and the preview is for flames.
- The density estimation of the graphics card in bands, a few rows a submission: its work for a picture of
  3840 × 2160 is more than [one submission may do](architecture.md#the-work-of-one-submission), so such a picture
  is the processor's.
- Editing the colors of a fractal beyond the eight palettes, and typing a parameter in as a number.
- A build for the Mac App Store: the purchase code is shared with iOS, and no script signs, sandboxes and packs
  the app for it ([ios.md](ios.md#a-mac-app-store-build)).
- Opening a `.frcq` file by a double click on macOS: the files are shown as Fractique's, and no more.

## Renderer, next

- The second random generator in the MCP server and the HTTP server: both still make random scenes with the first.
- More presets, and named palettes in scene files.
- Views of a Mandelbrot or a Julia set past the digits of 64-bit numbers (perturbation), and deep views of the
  orbit kinds.
- Rendering in tiles, to lower the memory of large supersampled images.
- Solids: the graphics card (a distance estimate is a short shader), moving the eye by dragging the picture
  instead of the `turn` and `tilt` sliders, and more kinds on the shared marcher (quaternion Julia sets, other
  folded solids).
- Animation: interpolating between two scenes.

## Decisions made

| Question | Decision |
| --- | --- |
| What the graphics card may draw | The screen's picture of a flame or of an attractor it has a shader for, and its Instant export, where the card is clearly faster than the processor and the switch is on: a render beside it kept every core busy for seconds. Exports at Standard, High and Ultra, the command line tool, the MCP tools and the server are the processor's, so a scene still gives the same pixels everywhere |
| UI framework for the app | `egui` with `eframe` on `wgpu`: Rust, and it reaches the five platforms and WebAssembly. `vendor/` carries the two crates with a fix for the iOS drawing surface |
| Installers | `cargo-packager` for Linux and macOS. Windows gets no installer: the app is one file that replaces itself |
| The app's own file | `.frcq`: packed JSON, not a binary encoding of the scene, which came out larger |

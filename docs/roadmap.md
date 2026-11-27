# Roadmap

The order is chosen so that every step is usable on its own and the picture quality is settled before any UI is
built on it.

| Step | What | State |
| --- | --- | --- |
| 1 | **Core:** scene format, renderer, presets, randomizer, exact repeatability | Done |
| 2 | **CLI:** render scenes to PNG | Done |
| 3 | **MCP server** over stdio: presets, random scenes, mutation, validation, `.flame` import, wallpapers, rendering with the image returned, progress and cancelling ([mcp.md](mcp.md)) | Done |
| 4 | **App:** one codebase for Windows, macOS, Linux, Android, iOS and the web, with its own glass title bar on a computer; presets, randomize, mutate, layers editing, tone, picture sizes, export with sizes and quality, glass, 15 languages | Done |
| 5 | **Zoom:** magnified views in the renderer and in the app (drag, wheel, pinch), each kind as far as it is still a picture: a flame between 10 and 300 times, an attractor between 10 and 100, a wallpaper 8, the other kinds of fractal up to ten thousand million | Done |
| 6 | **Wallpapers:** sheets in the scene format, a wallpaper for every seed from 23 layouts and 11 materials (14 and 6 at first), light and dark; in the command line tool and the app | Done |
| 7 | **Phones:** a screen of its own on Android and iOS, a collection of kept pictures, saving to Photos or the gallery, sharing | Done |
| 8 | **Scene files and session:** the packed `.frcq` file, read by the app, the command line tool and the MCP server, turned into JSON and back by `fractique convert`, opened by a double click in the file manager; one window at a time on a computer; the session kept between launches | Done |
| 9 | **Installers and updates:** one program file for Windows, an AppImage, a `.deb`, a macOS app and an Android APK built by the release workflow, with the command line tool, the MCP server and the HTTP server inside the packages and as files of their own; the updates window of the app and the `update` command of the three other programs ([releasing.md](releasing.md)) | Done: released on GitHub from 1.0.0 to 1.0.3; since 1.0.5 a version tag builds every platform, iPhone, iPad and the Mac App Store included, and uploads, pushes and publishes nothing unless all of them succeed ([releasing.md](releasing.md)) |
| 10 | **iOS distribution:** runs in the Simulator and on an iPhone from a Mac, uploads to TestFlight ([ios.md](ios.md)); the App Store, where large and custom export sizes need the one-time purchase Fractique Pro ([ios.md](ios.md#fractique-pro-the-purchase)) | Done for iPhone and iPad: on the App Store since 1.0.4 (2026-11-25), with Fractique Pro; the Mac version is in review |
| 11 | **Server:** HTTP API with a job queue and limits, the web UI (the app compiled to WebAssembly) served beside it, a reference of the API at `/docs` with an OpenAPI description, a `Dockerfile` ([server.md](server.md)) | Done; the image is built from the `Dockerfile`, none is published |
| 12 | **MCP over HTTP:** the tools of the MCP server at `/mcp` of the HTTP server (Streamable HTTP), from one implementation, under the limits and the queue of the server and without access to its disk ([mcp.md](mcp.md#over-http)) | Done |
| 13 | **GPU preview:** while a tone slider, the zoom, an opacity or a blend moves, the graphics card draws the picture from the rendered layers; the processor's picture replaces it when the control rests ([architecture.md](architecture.md#the-preview-on-the-graphics-card)) | Done |
| 14 | **The chaos game on the graphics card:** with GPU acceleration on, a flame is drawn by the graphics card alone, at full size in a fraction of a second, and the processor renders only exact exports; the switch exists only where a trial shows the card clearly faster than the processor ([architecture.md](architecture.md#the-chaos-game-on-the-graphics-card)) | Done; measured on Windows with an NVIDIA RTX 4070 and a GeForce GTX 1650 and on Linux with the graphics chip of a processor, a Radeon Vega 3 ([performance.md](performance.md#the-picture-of-the-graphics-card)). Not run on macOS |
| 15 | **Fractals of other kinds:** a layer is a flame or a fractal of a kind from a registry, drawn per pixel or as a density; the Mandelbrot set, Julia sets and the Clifford attractor, with presets, a random picture for every seed and mutation; a third tab in the app, `fractique fractals` and `--fractal`, the MCP tools and the HTTP server ([architecture.md](architecture.md#fractals-of-other-kinds)) | Done; the other kinds on the same registry are rows 16 to 19 |
| 16 | **Escape-time and root-finding kinds:** Burning Ship, Tricorn, Multibrot, Phoenix and Magnet beside the Mandelbrot and Julia sets, and Newton and Nova, the first two of the root-finding family; each with six presets, a random picture for every seed and a mutation ([scene-format.md](scene-format.md#escape-time-kinds)) | Done. The golden hashes were produced on Linux |
| 17 | **Orbit kinds:** the Buddhabrot, with the Nebulabrot coloring and the Anti-Buddhabrot, and the De Jong, Lorenz, Gumowski-Mira and Hopalong attractors beside the Clifford attractor, which makes the orbit family whole; each with six presets, a random picture for every seed and a mutation ([scene-format.md](scene-format.md#orbit-kinds)) | Done. The golden hashes were produced on Linux |
| 18 | **Solids:** the Mandelbulb, the Mandelbox and the Menger sponge, ray-marched by distance estimates on the processor with light, soft shadows, fog and a halo; an eye that turns around the solid and goes inside; presets, random pictures measured before they are kept, mutation ([scene-format.md](scene-format.md#solid-kinds), [architecture.md](architecture.md#the-ray-marched-kinds)) | Done |
| 19 | **Geometric fractals** on the same registry: the Apollonian gasket, Kleinian limit sets (Grandma's and Maskit's recipes), L-systems with their rules as text in the scene, the Lyapunov fractal ([scene-format.md](scene-format.md#geometric-kinds)) | Done |
| 20 | **Escape-time and root-finding kinds on the graphics card:** the nine kinds are drawn pixel by pixel by the graphics card alone, in batches whose work is bounded before they are sent and that follow one another without waiting for the window ([architecture.md](architecture.md#fractals-drawn-pixel-by-pixel-on-the-graphics-card)) | Done; measured on Windows with a GeForce GTX 1650 and on Linux with a Radeon Vega 3 ([performance.md](performance.md#fractals-on-the-graphics-card)) |
| 21 | **Deep views on the graphics card:** a magnified view that holds little of a flame is drawn by the card from particles, as the renderer draws it, so zooming and moving a flame stays on the card ([architecture.md](architecture.md#deep-views-on-the-graphics-card)) | Done; measured on Windows with an NVIDIA RTX 4070 and run on Linux with a software graphics card ([performance.md](performance.md#deep-views-on-the-graphics-card)). Not run on another card |
| 22 | **Wallpapers on the graphics card:** with GPU acceleration on, a wallpaper is drawn by the graphics card alone, pixel for pixel as the processor draws it but for a step of one in a few numbers, in batches whose work is bounded before they are sent ([architecture.md](architecture.md#wallpapers-on-the-graphics-card)) | Done; measured on Windows with an NVIDIA RTX 4070 and run on Linux with a software graphics card ([performance.md](performance.md#wallpapers-on-the-graphics-card)). Not run on another card |
| 23 | **The window first:** the window keeps its pace while the graphics card draws: the chaos game is sent to the card in units of about 1.5 ms until the card's slice of the frame is over; every sheet, list and panel opens, closes and resizes as one move; a new picture comes in two stages, a still blurred one and the finished one, with a quick preview of every new scene under the blur ([architecture.md](architecture.md#motion)) | Done for flames, attractors and deep views; measured on Windows with an NVIDIA RTX 4070. The kinds drawn pixel by pixel and the wallpapers are not on the units yet |
| 24 | **Views with limits:** a view goes no further out than a fifth of the picture's framing and no deeper than its kind is a picture; a flame, an attractor and a deep view have a margin of the scene around the view; a fractal drawn pixel by pixel is drawn for every frame while it moves; a view that would take over 10 seconds is the picture the app has, magnified ([performance.md](performance.md#where-a-view-stops-being-a-picture)) | Done; a heavy flame can still take the card longer than 10 seconds in a deep view |
| 25 | **A chosen size as a canvas:** 8K and 16K in the list; the screen draws a chosen size with the window's pixels, as that picture scaled down, and shows the scene beside it instead of bars; an Instant export smaller than the picture on the screen is that picture made smaller, a larger one of a fractal drawn pixel by pixel is drawn on the card in tiles | Done; measured on Windows with an NVIDIA RTX 4070 |
| 26 | **Recent pictures and Back:** each tab keeps its last 20 pictures, with small pictures and every mutation as a picture of its own; **Back** steps through them; the phone's round buttons stand under the thumb | Done |
| 27 | **Four programs:** the MCP server as a program of its own, `fractique-mcp`, beside the app, `fractique` and `fractique-server`; each of the three with an icon on Windows and an `update` command ([releasing.md](releasing.md#how-the-other-programs-update)) | Done: released beside the app since 1.0.0 |
| 28 | **Colors of a layer:** a chosen palette belongs to the selected layer, each layer keeps its own through adding, removing and reordering, and a dot before a layer's name shows the colors it is drawn with | Done |
| 29 | **One territory at every zoom:** a picture can be moved within the same nine framings at every zoom, so zooming in never moves the camera; the blurred picture follows a fast zoom to the kind's ceiling | Done |
| 30 | **Faint parts of a deep view:** a view that holds less than an eighth of a flame raises its faint end and gives it a share of the points, so what a bright part in the frame kept dark is seen and smooth | Done |
| 31 | **A gallery:** a hundred flames, a hundred fractals, a hundred isometric fractals and a hundred wallpapers in `docs/gallery`, each with the `.frcq` file that renders it again; forty of them in two or three layers of different colors, eighty in the dark palettes of one hue | Done |
| 32 | **The tools from the AppImage:** its first word, `cli`, `mcp` or `server`, starts the program packed in it | Done |
| 33 | **Your own colors:** a gradient of one to eight colors for a layer, picked on a square and a bar of hues, and of one to eight colors for a wallpaper, kept in the session and the recent pictures | Done |
| 34 | **Phones as computers:** every kind of fractal on a phone, the isometric solids included | Done |
| 35 | **Full rows on a phone:** the Ikeda attractor (24 kinds, the card draws it too), a 30th flame preset and more presets of five kinds, so that every row of tiles ends full; **Add layer** first and **Random** as a few random pictures in turn; one's own colors in eight fixed places, with the picker and **Delete color** in one card, and the same picker for a wallpaper's colors | Done |

## App, next

What the app does not do yet:

- Export and scene files on the web; scene files on phones.
- The chaos game on the graphics card in a browser: the page renders on its one thread, so the exact render would
  have to wait for the graphics card's picture first.
- Choosing a wallpaper's color on a computer (a phone has it in its styles).
- More fractals of other kinds on the graphics card: it draws flames, the Clifford, De Jong, Ikeda and Lorenz
  attractors ([architecture.md](architecture.md#orbits-on-the-graphics-card)), the escape-time and
  root-finding kinds ([architecture.md](architecture.md#fractals-drawn-pixel-by-pixel-on-the-graphics-card)) and
  wallpapers; a
  scene with a geometric kind or a solid in it, or with a flame beside a kind drawn pixel by pixel, is rendered
  by the processor alone.
- The kinds drawn pixel by pixel and the wallpapers in the units of card work that the chaos game has: their
  work is still sent in batches sized beforehand.
- Deep views of a kind drawn pixel by pixel past the digits of the graphics card's 32-bit numbers: the processor
  draws them, slowly, and at a large size the view is the magnified picture instead.
- Deep views of a heavy flame within 10 seconds on the graphics card: Emerald Star takes an RTX 4070 13 s at
  100×.
- Instant exports of a flame, an attractor and a wallpaper larger than the picture on the screen on the graphics
  card: the processor renders them.
- A mark on the screen where the exported picture ends, for a picture of a chosen size in a window of another
  shape.
- A run on Android of what rows 23 to 30 changed.
- A deep view on the graphics card without the 0.1 to 0.5 s that giving the faint parts their points costs it.
- A device with a measured limit of its graphics card hands nearly every picture to the processor at once: the
  check judges a whole picture by its first small unit of work.
- Editing the colors of a fractal beyond the eight palettes, and typing a parameter in as a number.
- A purchase made on a Mac: `scripts/mac/appstore.sh` builds, sandboxes and uploads the Mac App Store's build
  ([ios.md](ios.md#a-mac-app-store-build)), and nobody has bought Fractique Pro in it yet.
- A purchase seen on a second device: the app asks StoreKit 2 at its start what the Apple Account owns
  (`ios/ProWatch.swift`), and nobody has yet bought on one device and opened the other.
- Opening a `.frcq` file by a double click on macOS: the files are shown as Fractique's, and no more.

## Renderer, next

- The second random generator in the MCP server and the HTTP server: both still make random scenes with the first.
- More presets, and named palettes in scene files.
- A published Docker image of the server: it is built from the `Dockerfile` by hand.
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
| Faint parts of a view between half and an eighth of a flame | They stay as the plain chaos game draws them, dark beside the flame until the view holds less than an eighth, where the deep view takes over. Starting the deep view at a quarter makes them smooth, but such a view takes the processor about twenty times as long: Emerald Star beside the flame at 2×, 1920 × 1080 Standard, 3.3 s plain and 72.9 s deep (measured 2026-11-22), and exports are the processor's. Raising the faint end of the plain chaos game instead costs nothing, but brightens the whole view (+41 % at 2× with a lift of at most 4) and shows its few points there as grain; the owner chose to leave it |
| What the graphics card may draw | The screen's picture of a flame or of a fractal it has a shader for, and its exports, where the card is clearly faster than the processor and the switch is on: a render beside it kept every core busy for seconds. Exports at Standard, High and Ultra are the card's too, as Ultra at 3840 × 2160 took a minute or more on the processor and 12 s on an RTX 4070; **Exact pictures** gives them to the processor. The command line tool, the MCP tools and the server are the processor's, so a scene still gives the same pixels everywhere there |
| UI framework for the app | `egui` with `eframe` on `wgpu`: Rust, and it reaches the five platforms and WebAssembly. `vendor/` carries the two crates with a fix for the iOS drawing surface |
| Installers | `cargo-packager` for Linux and macOS. Windows gets no installer: the app is one file that replaces itself |
| The app's own file | `.frcq`: packed JSON, not a binary encoding of the scene, which came out larger |

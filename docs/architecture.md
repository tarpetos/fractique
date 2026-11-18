# Architecture

Fractique is a Rust workspace. One crate, **`fractique-core`**, turns a scene into pixels and knows nothing else:
no files, no terminal, no network, no UI. Every other part is a thin wrapper around it, so a scene renders to the
same image wherever it is rendered from.

## Overview

```mermaid
flowchart LR
    SCENE[("Scene<br/>one JSON file,<br/>or the app's .frcq")]

    subgraph CORE["fractique-core (pure Rust, no I/O)"]
        VAL["Scene<br/>format, validation"]
        RND["Randomizer<br/>random scenes, mutation"]
        PRE["Presets<br/>compiled in"]
        WAL["Wallpapers<br/>sheets for a seed"]
        IMP["Import<br/>.flame files"]
        REN["Renderer<br/>chaos game, histogram, deep views"]
        TON["Tone<br/>density estimation, log, gamma, glow"]
        SHE["Sheets<br/>flat shapes, drawn exactly"]
        VAL --> REN --> TON
        VAL --> SHE
        RND --> VAL
        PRE --> VAL
        WAL --> VAL
        IMP --> VAL
    end

    CLI["fractique-cli<br/>the fractique command"]
    MCP["fractique mcp<br/>MCP server over stdio"]
    SRV["fractique-server<br/>HTTP API, web UI, Docker (planned)"]
    APP["fractique-app<br/>Windows, macOS, Linux, Android, iOS, web"]

    SCENE --> VAL
    TON -- "RGBA pixels" --> CLI
    SHE -- "under the layers" --> TON
    TON -- "PNG, inline" --> MCP
    TON -.-> SRV
    TON -- "RGBA pixels" --> APP
    CLI -- "PNG" --> OUT[("image file")]
    APP -- "PNG, .frcq" --> OUT
```

| Crate | State | What it is |
| --- | --- | --- |
| `fractique-core` | works | The renderer, the scene format, presets, the randomizer, the wallpapers, the `.frcq` packing |
| `fractique-cli` | works | The `fractique` command: renders scenes and wallpapers to PNG, converts `.flame` files, and runs as an MCP server (`fractique mcp`, see [mcp.md](mcp.md)): an AI agent edits a scene, renders a preview, looks, adjusts |
| `fractique-app` | works | The app, `fractique-gui`: one codebase for Windows, macOS, Linux, Android, iOS and the web ([The app](#the-app)) |
| `fractique-server` | planned | HTTP API with a job queue, the web UI, the Docker image |

`vendor/` holds `eframe` and `egui-wgpu` with the fix the iOS drawing surface needs; the workspace patches them
in.

### Why this shape

- **One renderer.** The app, the CLI, the MCP server and the HTTP server all call the same `render` function with
  the same scene, so they cannot disagree about what a scene looks like.
- **A scene is data.** Everything a render depends on is in one JSON document (`Scene`): the flames, the palettes,
  the camera, the tone settings, the sheets of a wallpaper and the seed. Presets, random scenes, wallpapers,
  mutations, files and API requests are all the same type. The format is described in
  [scene-format.md](scene-format.md).
- **No I/O in the core.** It compiles for every target unchanged (desktop, Android, iOS, WebAssembly) and is easy
  to test. `file.rs` turns a document into the bytes of a `.frcq` file and back; reading and writing the file is
  the caller's.
- **CPU rendering.** GPUs do not give the same bits from one vendor or driver to the next; the processor does.
  See [determinism.md](determinism.md).

## How a render works

```mermaid
flowchart TB
    S["Scene + options<br/>(size, samples per pixel, supersample)"] --> V["Validate"]
    V --> SH["Sheets, when the scene has them<br/>drawn onto the background"]
    V --> L{{"for each layer"}}
    L --> C["Compile the flame<br/>transforms, palette table, camera"]
    C --> W["64 streams, each with its own random generator<br/>and walker, run on all cores"]
    W -- "integer adds" --> H[("Histogram<br/>hits and color sums per cell")]
    H --> E["Density estimation<br/>cells with few hits are spread over neighbours"]
    E --> T["Log tone mapping, gamma, vibrancy, highlight"]
    T --> D["Average the supersample cells"]
    D --> G["Glow"]
    G --> B["Blend onto the canvas<br/>add, screen or over"]
    B --> L
    SH --> B
    B --> Q["Dither to 8 bits"]
    Q --> I["Image (RGBA)"]
```

| Step | Where | What happens |
| --- | --- | --- |
| Chaos game | `chaos.rs` | A walker starts at a random point and repeats: pick a transform by weight, apply its affine map, then the weighted sum of its variations, then the optional post map. Each step moves the walker's color toward the transform's color. Every point after the first 24 is plotted. |
| Deep views | `deep.rs` | When a view holds less than an eighth of the flame, the plain chaos game wastes its points outside it. Then the layer is drawn from 8192 particles: points of the flame inside the view, each with the seeds of the steps that made it. They are found window by window, every window half the size of the one before: particles that fall out are replaced by copies of those that stay, and each is moved about inside the window by giving it a new start and keeping its latest steps. The render moves the particles the same way and plots them. Brightness stays what it was where the view began to hold less than an eighth, so a deep view is neither burnt nor black. |
| Streams | `render.rs` | The samples of a layer (`samples per pixel × width × height`) are split over 64 streams. The number is fixed, so the result does not depend on how many cores a machine has. |
| Histogram | `render.rs` | Each plotted point adds 1 to its cell's hit count and its palette color to the cell's color sums. These are integer additions on atomics: the order in which threads add does not change the totals. |
| Density estimation | `tone.rs` | A cell with few hits is noise, so it is spread with a wide kernel; a cell with many hits stays sharp. This is what makes sparse wisps smooth without blurring the dense structure. |
| Tone mapping | `tone.rs` | Brightness follows the logarithm of the density, relative to the average density, so the same scene has the same brightness at any resolution, quality and zoom. Gamma, vibrancy and highlight shape it. |
| Glow | `tone.rs` | A Gaussian blur of the layer is added back. Its radius is a share of the image's shorter side. |
| Sheets | `sheets.rs` | A wallpaper's sheets are drawn onto the background before any layer. For every pixel and every sheet, bottom to top: the distance to the sheet's shapes gives its coverage (one pixel of soft edge), the rim along its edge and, moved toward the light, the shadow it casts on what is under it. Nothing is sampled, so the seed, the samples per pixel and the supersample do not change it. |
| Layers | `render.rs` | Each layer has its own flame, camera and tone settings and is blended onto the canvas in order. `render_layer` develops one layer and `compose` blends developed layers, so a caller can keep the layers that did not change. |
| Dither | `render.rs` | An 8×8 ordered dither removes the bands that 8 bits would show in faint gradients on a dark background. |

### Any resolution

The camera is not in pixels. At zoom 1 the shorter side of the image shows 4 world units, whatever the size, and a
wider image shows more of the world on its long side. The number of samples grows with the number of pixels, and
glow and smoothing are sized relative to the image. So one scene gives the same picture at 640×360 and at
3840×2160: the large one is sharper, not different.

### Any zoom

A view can be magnified until the 64-bit numbers of the renderer run out of digits, at about ten thousand million
times; the app stops there. A deep view costs five to ten times a plain one, because every point is sent through
the steps its particle keeps. Where the flame has nothing, a deep view is empty: the gaps between its threads are
wider than the threads. A flame whose transforms blur (`blur`, `gaussian_blur`) is smooth below the size of
its blur and shows no more detail there.

### Phones

A phone renders the same scene with the same code at a lower quality level: fewer samples per pixel and no
supersampling (`mobile`: 30 samples per pixel). Density estimation is what keeps that from looking noisy. See
[performance.md](performance.md) for the numbers.

## Wallpapers

`random_wallpaper(seed, request)` in `wallpaper.rs` builds a scene of sheets. The request names the proportion of
the image, light or dark, and optionally a layout, a material and a hue; the seed decides the rest. One generator
picks the layout and the material, a second places the shapes of that layout for the proportion, a third chooses
the colors and the light of the material. Colors are chosen in OKLCH, so equal steps look equal. `chosen` tells
which layout and material a seed gives, without building the scene.

## The randomizer

`random_scene(seed)` builds a flame of 2 to 4 transforms from a weighted list of variations, picks a palette and
sometimes a symmetry and a second layer. It then probes the flame with 30 000 points to place the camera, and
rejects flames that are a dot, dust or a filled square, trying up to 40 candidates per layer. All of it is driven by
the seed alone, so a seed is a scene. `mutate(scene, seed, strength)` nudges the transforms of an existing scene.

## The app

`fractique-app` is built with `egui` and `eframe` on `wgpu`. The graphics card only shows things: the picture
itself is always rendered by `fractique-core` on the processor, in a thread of its own.

| File | What |
| --- | --- |
| `lib.rs` | The window: the inspector, the toolbar, the title bar the app draws itself, the sheets with lists, the session, the render passes of the picture |
| `phone.rs` | The phone's screen: the round buttons, the placard, the styles with their small pictures, the collection |
| `export.rs` | An export: a render in a thread of its own, with progress and cancel |
| `update.rs`, `updates.rs` | The releases, the download and the install; the updates window ([releasing.md](releasing.md)) |
| `glass.rs`, `glass.wgsl`, `blur.rs` | The picture on the screen and the glass panes over it: each pane shows a blurred, refracted part of the picture |
| `widgets.rs` | The controls: rows, pills, sliders, switches; the fonts |
| `i18n.rs`, `locales/` | The texts, in 15 languages |
| `android.rs`, `ios.rs` | The gallery, Photos and sharing |
| `main.rs` | The start on a computer and in a browser; a file given at start |

How the picture reaches the screen:

| Step | What happens |
| --- | --- |
| A change | The scene, the size or the view changes. The render in progress is cancelled and a new one starts. A moved or zoomed view waits until it has rested for 0.18 s; until then the picture already on the screen is moved and scaled by the graphics card |
| Passes | A draft first (480 pixels on its longer side, 12 samples per pixel), then the full picture. A view magnified more than 4 times gets a pass in between (1280 pixels, 16 samples). The picture is shown after each |
| The full picture | At the chosen size, up to 4096 pixels a side with 32 samples per pixel on a computer, 1600 and 18 on a phone, 900 and 14 in a browser |
| Kept layers | A layer's pixels depend only on its flame, its place, the seed and the size. Developed layers are kept (up to 1200 MB on a computer, 160 MB on a phone and in a browser), so a change to one layer renders that layer only, and **Reset view** comes back without a render |
| Threads | Renders run on a pool that leaves two cores to the window. A browser page has one thread and renders there, without a draft |
| Events | A finished pass is sent over a channel and asks for a repaint. Nothing polls |

An export is a separate render at the size and quality chosen, with the scene as it is on the screen. The session
is written whenever it changes, not at exit: a phone ends an app without telling it.

## Limits

| | |
| --- | --- |
| Image side | 1 to 16 384 pixels |
| Supersample | 1 to 4 |
| Samples per pixel | 1 to 100 000 |
| Histogram cells (`width × height × supersample²`) | at most 2²⁸ |
| Layers per scene | 1 to 16, or none when the scene has sheets |
| Transforms per flame | 1 to 64 |
| Sheets per scene | 1 to 48 |
| Shapes per sheet | 1 to 32 |
| Magnification of a view in the app | 0.0001 to 10 000 000 000 |
| Unpacked `.frcq` document | at most 64 MB |

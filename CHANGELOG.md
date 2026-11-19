# Changelog

## Unreleased

* **Names of fractals in every language:** the names of the kinds, of their presets and of the flame presets are
  shown in the app's language, in all 15 (the kinds were kept in English for a while, the presets always were). The default
  name of an exported picture and of a scene file is made from the English name as before (`MandelboxStation`).

* **Heavy kinds:** the kinds that only the processor draws and that take it long (Buddhabrot, Hopalong,
  Gumowski-Mira, Mandelbulb, Mandelbox, Menger sponge, Kleinian, Lyapunov, L-system) carry an amber dot in the
  list of types on a computer, with a hint, and are left out of the tiles on a phone. A scene or a `.frcq` file
  with such a kind still opens and renders everywhere.

* **Fractals of other kinds:** a layer of a scene is now a flame or a fractal of another kind, and three kinds are
  here: the **Mandelbrot set** and **Julia sets**, drawn pixel by pixel with smooth colors, a thin dark rim along
  the set, colors that fade to dark away from it and anti-aliasing where the picture has edges, and the
  **Clifford attractor**, drawn as a density through the flame's histogram, tone mapping and glow. They use the
  palettes, the camera, the layers and the blend modes of flames, so a Mandelbrot set can lie under a flame, and a
  view goes as deep as 64-bit numbers reach (a zoom of about 10¹³ at 1080 pixels; the app stops at 10¹⁰). Each
  kind has six presets, a random picture for every seed that is checked to be worth looking at, and a mutation.
  * **App:** a third tab, **Fractals**, between **Flames** and **Shapes**: the type (a list on a computer, tiles
    with small pictures on a phone), the presets of the type, **Randomize** and **Mutate**, a slider for every
    parameter, and the palettes, layers, tone, view, recent pictures, collection, export, session and `.frcq`
    files as for flames. These kinds are rendered by the processor; GPU acceleration is for flames.
  * **Command line:** `fractique fractals` lists the kinds with their parameters and presets; `--fractal KIND`
    with `render --random SEED` and `randomize SEED` makes the random fractal of a seed; `--preset` and
    `preset` take the fractal presets (`mandelbrot-seahorse-valley`, `julia-dragon`, `clifford-wings`, ...).
  * **MCP and HTTP:** the tool `list_fractals`, `fractal` in `randomize_scene` and in `render_scene` with
    `random_seed`; `GET /fractals`, and `fractal` in `POST /random` and in `POST /render` with `random`.
  * **Scene format:** a layer holds `fractal` in place of `flame`
    ([docs/scene-format.md](docs/scene-format.md#fractal)). The version stays 1: every scene and `.frcq` file
    written before loads and renders to the same pixels, and the golden hashes of flames, wallpapers and random
    scenes are untouched. In `fractique-core`, `Layer::flame` became `Layer::content`, with `flame()`,
    `fractal()`, `camera()`, `tone()` and `palette()` to reach into either.
  * The kinds live in a registry, one file each ([docs/architecture.md](docs/architecture.md#how-to-add-a-fractal-type)).
    The golden hashes of the three kinds were produced on Linux.
  * **Seven more kinds**, each with six presets, a random picture for every seed and a mutation. Escape-time:
    **Burning Ship** (`burning_ship`), **Tricorn** (`tricorn`), **Multibrot** (`multibrot`, z^d + c with the
    power from 2 to 8; a power that is not whole shows the seams of its branch cut), **Phoenix** (`phoenix`, the
    Julia form and the map of its constants) and **Magnet** (`magnet`, both types, with the orbits that settle
    on 1 colored like those that escape). Root-finding, the first two of that family: **Newton** (`newton`,
    zⁿ − 1 with n from 2 to 12 and five named polynomials, a complex relaxation, the basins colored by root and
    shaded by a smooth count) and **Nova** (`nova`, Newton's step plus a constant, as the map of the constants
    and as its Julia form). In the app a parameter that is a choice (the form, the type, the polynomial) shows
    the names of its values in place of a slider. The Mandelbrot and Julia sets render as before.

* **Five more fractals, the rest of the orbit family:** the **Buddhabrot** and the **De Jong**, **Lorenz**,
  **Gumowski-Mira** and **Hopalong** attractors, drawn as densities like the Clifford attractor. Each has six
  presets, a random picture for every seed that is checked to be worth looking at, and a mutation
  ([docs/scene-format.md](docs/scene-format.md#orbit-kinds)).
  * **Buddhabrot:** the orbits that leave, or with `anti` those that stay; colored by the length of the orbit,
    by where it started, or as the Nebulabrot of three iteration limits in blue, green and red; `turn` and `tilt`
    turn the picture towards the plane of the starting points. Its starting points are taken only from the
    cells of the plane that can hold an orbit to draw, evenly, so the picture is the one blind sampling gives and
    most of the wasted orbits are never started.
  * **Lorenz:** integrated in fixed steps by the fourth-order Runge-Kutta method, turned and tipped by two
    angles; `duration` goes from a few separate lines to the density of the whole attractor.
  * **The attractors** are colored by the speed of the orbit, the direction of its step or the distance from the
    middle (the height, for Lorenz), each spread evenly over the palette. In the app these colorings, those of
    the Buddhabrot and its two kinds of orbits are choices by name.
  * In `fractique-core`, a kind drawn as a density may bring its own colors in place of the layer's palette
    (`Orbit::colors`); the Nebulabrot does. Nothing changes for the kinds that were there: their golden hashes
    are untouched. The golden hashes of the five kinds were produced on Linux.
* **Solids:** three kinds in three dimensions, `mandelbulb` (any power from 2 to 16), `mandelbox` (scale, fold
  limit, inner radius) and `menger_sponge` (the level, and a twist, a bend and a scale that fold it into other
  shapes), each ray-marched by a distance estimate on the processor, with a key light, soft shadows, shading of
  the pits, fog by depth, a halo and colors from orbit traps through the palette. The eye turns around the solid
  (`turn`, `tilt`), comes nearer or goes inside (`distance`) and has a `field_of_view`; the app's pan and zoom
  move and magnify the picture as a longer lens does. Six presets a kind, a random picture for every seed that is
  measured before it is kept, and a mutation ([docs/scene-format.md](docs/scene-format.md#solid-kinds)). A
  per-pixel kind can now learn the width of a pixel, prepare itself for a picture and say how much neighbouring
  pixels may differ before they are drawn again; the pictures of the other kinds are untouched. The golden hashes
  of the three kinds were produced on Linux.

* **Geometric fractals:** four kinds join the registry. The **Apollonian gasket** (`apollonian`), from any
  Descartes configuration, as shaded discs, rings or a glow, colored by generation, size or ancestor. **Kleinian
  limit sets** (`kleinian`), the two-generator groups of "Indra's Pearls" by Grandma's recipe and Maskit's, with
  the traces as parameters, drawn by a depth-first search of words that follows the view. **L-systems**
  (`l_system`): a turtle draws what an axiom and rewriting rules spell, with brackets for branches; the dragon
  curve, the Koch snowflake, the Sierpinski triangle, the Hilbert and Gosper curves, a plant and a bush are
  presets. The **Lyapunov fractal** (`lyapunov`) of the logistic map for any sequence of A and B. Each has six or
  seven presets, a random picture for every seed and a mutation.
  * **Scene format:** a fractal may hold `text` beside `params`, for parameters that are text: the axiom and the
    rules of an L-system, the sequence of a Lyapunov fractal
    ([docs/scene-format.md](docs/scene-format.md#geometric-kinds)). A field that may be left out: the version
    stays 1 and every scene written before loads and renders as it did.
  * **App:** a field under the sliders for every text of the chosen fractal; the style and the coloring of a
    gasket, what an L-system's colors follow and the recipe of a Kleinian group are choices with names.
  * **Command line, MCP, HTTP:** `fractique fractals`, `list_fractals` and `GET /fractals` list the texts of a
    kind with their defaults.
  * In `fractique-core`: `Draw::PixelText` and `Draw::DensityText` build a kind from the whole `Fractal`, with
    its `Text`s; `Orbit::gain`, 1 unless a kind says otherwise, scales a density, which lets a curve stay as
    bright at every zoom. No existing kind renders differently: every golden hash from before is untouched, and
    those of the four kinds were produced on Linux.

* **GPU acceleration:** the graphics card draws the escape-time kinds (Mandelbrot, Julia, Burning Ship, Tricorn,
  Multibrot, Phoenix, Magnet) and the root-finding kinds (Newton, Nova) too, pixel by pixel: first in blocks, then
  every pixel, then the edges from more samples, and a moved view is drawn again at once. What one submission may
  do is bounded before it is sent, as for flames: at most 100 000 000 steps of the kind's loop and what the card
  has shown it does in 0.1 s; a scene that could need more for 64 pixels is the processor's, as are views too deep
  for 32-bit numbers, scenes that mix these kinds with flames, the geometric kinds and the solids. On a GeForce
  GTX 1650 a picture of 1920×1080 takes 0.02 to 0.4 s where the processor beside it takes 0.5 to 2.1 s. Exports at
  Standard, High and Ultra, the command line, MCP and the server are the processor's as before: no golden hash
  changes ([docs/architecture.md](docs/architecture.md#fractals-drawn-pixel-by-pixel-on-the-graphics-card)).
* **GPU acceleration:** the graphics card draws three attractors beside flames: **Clifford**, **De Jong** and
  **Lorenz**, alone or in a scene with flames. The Gumowski-Mira map, the Hopalong and the Buddhabrot stay the
  processor's: the first is slower on a card than on the processor, 32-bit numbers leave the curve of the second,
  and the work of the third has no bound that is known before it is done
  ([docs/architecture.md](docs/architecture.md#orbits-on-the-graphics-card)).
* **GPU acceleration, safer:** what the graphics card is given in one go now has a hard bound that is known before
  it is sent: at most 4 steps a pixel and 4 194 304 steps for the walkers of all layers, and a density estimation
  over at most 17 179 869 184 cells, which leaves the card a picture of 3840×2160 with five layers. Before, a
  batch could grow to all the samples of a picture and was given 20 s. A batch that takes longer than 0.5 s now
  ends the card's part for the session. The price: a picture takes more batches, so the last samples come later
  than before
  ([docs/architecture.md](docs/architecture.md#the-work-of-one-submission)).
* **GPU acceleration:** the trial of a built-in graphics chip gives way when a picture is asked for and begins
  again when the app rests; its verdict is kept again, so a slow phone is tried once, not at every start. The
  first picture after a start is no longer handed to the processor because the shaders took time to load.
* **Phones:** the small pictures of the styles are drawn on one thread, so the window and the picture keep their
  cores, and are kept as files, so a sheet that was drawn once opens at once the next time.
* **GPU acceleration:** the trial that decides whether a built-in graphics chip is used draws a larger picture and
  asks for the card to be 1.3 times as fast, not twice: a phone with an Adreno 740 was left out at 1.7.
* **GPU acceleration:** a graphics card of its own (not one built into the processor) is used without a trial.
  Elsewhere the card's best of three trial runs counts, and a verdict that it is slow is not kept between launches,
  so one trial at a bad moment no longer hides the switch for good.
* **App:** with GPU acceleration on, the graphics card's picture of a flame is the screen's picture, and the
  processor no longer renders it a second time: a new flame at 3840×2160 took the processor 45 to 155 s of core
  time before and takes it 2 to 3 s now, the start of the app included, and the whole picture is there after 0.2
  to 0.3 s on an NVIDIA RTX 4070. Tone, opacity and blend follow at once. **Instant** export saves that picture,
  copied back from the card, once all of its samples are there; exports at Standard, High and Ultra are rendered
  by the processor and exact as before, and so are the command line tool, the MCP tools and the server. What the
  card does not draw (wallpapers, deep views) is rendered as before.
* **App:** the **Graphics** section is shown only where the graphics card helps. The app tries the card against the
  processor once, after its first picture, and remembers the answer for that card and version; where the card has
  no compute shaders or is not at least twice as fast, there is no switch and the app works on the processor
  alone, as it did before any acceleration. A card that turns out slow on one picture gives that picture to the
  processor at once, and an error of the card takes the switch away for the session. Under the switch are the
  card's name and the note, which now says that an Instant export can differ between devices too.

* **Sizes:** the size list is in groups with tabs over it: Screens, iPhone, iPad, Mac, Watch and Android. Every
  build has the full Apple list now (59 sizes in all), with the Apple Watch screens for a picture to use as a
  watch face; the App Store builds leave the Android group out.
* **App Store builds:** a locked size in the export list opens the Fractique Pro sheet when it is chosen, not at
  Save.
* **App Store builds (iPhone, iPad, Mac):** one purchase, **Fractique Pro**, paid once. Without it an export of
  2560 × 1440 pixels or more by pixel count (width × height of 3 686 400 or more) and a size typed into
  **Custom** are locked; smaller listed sizes, the picture on the screen at any size and every other feature stay
  free. Locked sizes show a small lock; choosing one, or **Save** or **Share** with one, opens a sheet with the
  App Store's price, **Buy**, **Restore purchases** and **Cancel**, and after the purchase the export goes on by
  itself. The size lists of these builds hold computer screens and 34 Apple screens (iPhone, iPad, MacBook, iMac,
  Studio Display, Pro Display XDR) instead of the Android ones. Windows, Linux, Android and the web are not
  changed: everything is free there and the builds hold no purchase code (the `store` feature,
  [docs/usage.md](docs/usage.md#free-and-fractique-pro), [docs/ios.md](docs/ios.md#fractique-pro-the-purchase)).

* **App:** with GPU acceleration on, the graphics card plays the chaos game itself, so a new flame is on the screen
  in a fraction of a second: at 3840×2160 the first picture comes after about 0.03 s and all of its samples after
  0.2 to 0.4 s on an NVIDIA RTX 4070, where the processor takes 4 to 12 s. The picture is at the full size from the
  start and grows finer with every frame. Wallpapers, views that hold little of the flame and graphics cards
  without compute shaders are left to the processor, and so is everything when the switch is off
  ([docs/architecture.md](docs/architecture.md#the-chaos-game-on-the-graphics-card)).

* **App:** while Export is open the controls are hidden, and while any sheet or list is open (export, presets,
  adding a layer, sizes, layouts, recent, languages, updates) the picture under it does not move, zoom or reset.
* **App:** under **GPU acceleration**, while it is on, a line says that the quick picture can differ slightly
  between devices and that exports are always exact.
* **Phones:** a finger that lands on a slider while it scrolls a panel no longer moves the slider: the slider
  follows a sideways move only, and a vertical one scrolls.
* **App:** GPU acceleration, a switch in the controls, on at first and kept between launches. While a tone slider,
  the zoom, a layer's opacity or its blend moves, the graphics card draws the picture from the layers already
  rendered, so it answers at once; when the control rests, the processor renders the picture and replaces the
  preview. A change to a layer's tone no longer runs the chaos game again. The rendered picture and every export
  are the processor's, as before; with the switch off the app renders as it did.
* **Renderer:** `render_density` and `develop` split `render_layer` in two, before and after the tone mapping,
  and give the same layer bit for bit. Existing scenes render as before.
* **App:** the last 24 pictures are remembered. **Recent** in the inspector lists them, and the styles of a phone
  show them as a row of small pictures; a click brings one back. The list is kept between launches.
* **MCP over HTTP:** `fractique-server` speaks MCP at `POST /mcp` (Streamable HTTP, revisions `2025-06-18` and
  `2025-03-26`) with the tools of `fractique mcp`, from the same code ([docs/mcp.md](docs/mcp.md#over-http)). A
  tool call is answered as an event stream with its progress; `notifications/cancelled`, the end of the session
  and a closed connection stop a render. Renders obey the limits of the server and wait in its queue. `file` and
  `save_to` do not exist there: a client on the network cannot read or write the disk of the server. On by
  default; `--mcp off` or `FRACTIQUE_MCP=off` turns it off, `--mcp-origin` names the web pages that may call it.
  `GET /limits` says whether it is on.
* **Export:** a fourth quality, **Instant**, now the first choice: it saves the picture that is on the screen at
  once instead of rendering it anew. Standard, High and Ultra render as before.
* **Android 9 and older:** storage access is asked for when Save is pressed, before the picture is made, and the
  export goes on by itself once it is allowed.
* **Scenes:** a number in a scene file is read as exactly the value written. Before, one could come out a last
  digit away, so a scene saved and read again could render differently from the one it was saved from. The two
  presets this touched, `ice-crystal` and `violet-whirl`, now carry the numbers they were always read as, so every
  preset renders the same pixels as before.
* **HTTP server:** `fractique-server` offers presets, random scenes, wallpapers and renders as PNG over HTTP
  ([docs/server.md](docs/server.md)). Renders wait in a bounded queue and a full one answers 429; the size, the
  samples and the time of a render are limited by settings; a client that disconnects cancels its render. It can
  serve the web build of the app beside the API.
* **Docker:** a `Dockerfile` and a `docker-compose.yml` build one small image with the server and the web app,
  run as a user without rights.
* **MCP server:** `list_wallpaper_styles` and `randomize_wallpaper` make wallpapers; `render_scene` also takes
  `wallpaper_seed` (with `layout`, `material`, `dark`), `file` (the path of a `.frcq` or JSON scene) and `frcq`
  (the bytes of a `.frcq` in base64). A call with a progress token gets `notifications/progress` while it renders,
  and `notifications/cancelled` stops a render, which then gets no answer. `ping` is answered during a render.
* **README translations:** the 14 translations name the buttons, the blend modes and the other terms as the app
  shows them in that language.
* **Command line tool:** `fractique convert` turns a `.frcq` file into its scene as JSON (`--document` for
  everything the file holds) and a JSON scene or document into a `.frcq`.

* **Variations:** 21 more, 49 in all: `waves`, `popcorn`, `rings` and `fan`, which take their shape from the
  transform's own `affine` numbers, and `blob`, `pdj`, `fan2`, `rings2`, `perspective`, `noise`, `juliascope`,
  `radial_blur`, `pie`, `ngon`, `rectangles`, `arch`, `square`, `rays`, `blade`, `secant2` and `twintrian`.
  `fractique variations` lists them with their parameters, and the `.flame` import reads them and their
  parameters. Existing scenes render as before, and `fractique randomize` gives the same scene for every seed: the
  randomizer does not use the new variations.
* **Randomizer:** a second random generator beside the first. It draws from all 49 variations, gives the
  parametric ones random parameters and skips more of the pictures that come out as specks, a dim haze or an even
  fog. `fractique randomize SEED --generator 2` and `fractique render --random SEED --generator 2` use it; without
  the option both commands give the scene they always gave. The app's Randomize makes its new flames with it, and a
  session or a `.frcq` file holds the number of the generator (`generator`; a file without it opens as before).
  The scenes of the first generator are now pinned by tests.
* **Presets:** `golden-filigree`, `frost-lotus`, `azure-turbine`, `orchid-pinwheel`, `lilac-mosaic`,
  `amber-lantern`, `emerald-plume` and `ember-thorns`, made with the new variations.
* **App (desktop):** one window at a time. A second start brings the open window to the front and passes it the
  file it was given. A debug build, or `FRACTIQUE_MANY=1`, still opens as many as asked.
* **Wallpapers:** a scene can hold `sheets`, a stack of flat shapes (rounded boxes, petals, rings and arcs, waves and
  ribbons) with lit edges, gradients, translucency and soft shadows, drawn under the flame layers.
  `fractique render --wallpaper SEED` makes one for every seed out of fourteen layouts (bands, discs, waves,
  ribbons, burst, arcs, tiles, bars, fan, slats, planes, steps, frames, lines), six materials (relief, paper,
  glass, glow, steel, carbon) and generated colors, each with a light and a dark version. Scenes without `sheets`
  render as before.
* **App:** a Flames and Wallpapers switch in the controls. In wallpaper mode the layout, the material and light or
  dark are chosen there; Randomize makes a new wallpaper and Mutate another of the same layout and material.
* **Phones:** the app has a screen of its own on a phone. The picture fills it, with its name and number as a
  placard. A round glass button makes a new one; four small ones keep it, make a similar one, open the styles and
  export. A swipe goes to the next or the previous wallpaper. The styles are a sheet of small live pictures of every
  layout, material and preset, with the kept pictures in a row above them and a row of colors for the wallpapers.
* **iPhone:** an exported picture goes to Photos or to the share sheet.
* **Android:** an exported picture goes to the gallery (`Pictures/Fractique`) or to other apps through Share.
* **Phones:** Fine-tune in the styles opens every control over the whole screen; the panels are laid out for a
  phone on its side too.
* **Scene files:** the app saves `.frcq` files, a packed format of its own, about a seventh of the size of the JSON:
  the scene with the tone, the view, the size and the wallpaper choices, so it opens as it was saved. The command
  line tool reads them. A file given to the app at start opens in it.
* **Session:** the app comes back to the scene, the view and the settings it was left with.
* **App:** a wallpaper's layout and material are chosen from lists, like a preset.
* **Wallpapers:** a wallpaper can be asked for in one color (`hue` of the request). The bands layout no longer
  makes an invalid shape on screens about 1.6 times as tall as wide.
* **App:** a double click on a `.frcq` file opens it in Fractique. The Windows program registers the file type for
  the current user when it starts; the `.deb` and the AppImage carry the type and the association. On macOS the type
  is declared, but a double click does not open the file yet.

## 0.1.0

First version: the renderer and the command line tool.

* **Renderer:** fractal flames with 28 variations, layers (add, screen, over), rotational and mirror symmetry,
  density estimation, glow, ordered dithering, any resolution up to 16384 pixels a side.
* **Any zoom:** a view magnified up to ten thousand million times is drawn sharp, from particles kept inside it; in
  the app, drag, wheel and pinch move the view and zoom in and out, and the picture is drawn anew when the view rests.
  A scene whose view holds less than an eighth of its flame renders differently from before this.
* **Exact repeatability:** the same scene and options give the same pixels on every platform and thread count.
* **Presets:** `aurora-spiral`, `emerald-star`, `emerald-burst`, `emerald-mandala`, `ember-vortex`, `azure-rings`, `golden-bloom`,
  `solar-mandala`, `jade-orb`, `violet-whirl`, `radiant-star`, `teal-lattice`, `amber-tree`, `silk-current`,
  `tide-curl`, `orchid-knot`, `frost-crown`, `green-comet`, `moss-nebula`, `lilac-cloud`, `ice-crystal`.
* **Randomizer and mutation:** a scene for every seed, and variants of a scene.
* **Command line tool:** `render`, `presets`, `preset`, `randomize`, `mutate`, `fit`, `import`, `variations`, `mcp`.
* **Import:** `.flame` files of flam3 and Apophysis convert to scenes.
* **MCP server:** `fractique mcp` lets an AI agent list presets, make and change scenes and render them.
* **App:** a window with the flame and glass panels over it, for Windows, macOS, Linux, Android, iOS and the web,
  in 15 languages.
* **Export, scenes, updates:** the app exports PNGs at any size, opens and saves scenes, checks GitHub for newer
  versions and installs any released version.
* **Downloads:** one program file for Windows (no installer), an AppImage and a .deb, a macOS app and an Android
  APK, built by a release workflow.
* **Setup:** `setup.sh` (Linux, macOS) and `setup.cmd` (Windows).

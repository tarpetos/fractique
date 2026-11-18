# Changelog

## Unreleased

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

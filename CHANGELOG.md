# Changelog

## Unreleased

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
* **App:** a window with the flame and glass panels over it, for Windows, Linux, Android, iOS and the web, in 15
  languages.
* **Export, scenes, updates:** the app exports PNGs at any size, opens and saves scenes, checks GitHub for newer
  versions and installs any released version.
* **Downloads:** one program file for Windows (no installer), an AppImage and a .deb, a macOS app and an Android
  APK, built by a release workflow.
* **Setup:** `setup.sh` (Linux, macOS) and `setup.cmd` (Windows).

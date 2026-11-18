# Changelog

## 0.1.0

First version: the renderer and the command line tool.

* **Renderer:** fractal flames with 28 variations, layers (add, screen, over), rotational and mirror symmetry,
  density estimation, glow, ordered dithering, any resolution up to 16384 pixels a side.
* **Exact repeatability:** the same scene and options give the same pixels on every platform and thread count.
* **Presets:** `aurora-spiral`, `emerald-star`, `emerald-burst`, `emerald-mandala`, `ember-vortex`, `azure-rings`, `golden-bloom`,
  `solar-mandala`, `jade-orb`.
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

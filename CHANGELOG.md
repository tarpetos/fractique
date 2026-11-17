# Changelog

## 0.1.0

First version: the renderer and the command line tool.

* **Renderer:** fractal flames with 28 variations, layers (add, screen, over), rotational and mirror symmetry,
  density estimation, glow, ordered dithering, any resolution up to 16384 pixels a side.
* **Exact repeatability:** the same scene and options give the same pixels on every platform and thread count.
* **Presets:** `aurora-spiral`, `emerald-star`, `emerald-mandala`, `ember-vortex`, `azure-rings`, `golden-bloom`,
  `solar-mandala`, `jade-orb`.
* **Randomizer and mutation:** a scene for every seed, and variants of a scene.
* **Command line tool:** `render`, `presets`, `preset`, `randomize`, `mutate`, `variations`.
* **Setup:** `setup.sh` (Linux, macOS) and `setup.cmd` (Windows).

<div align="center">
  <img src="assets/icon/icon-256.png" alt="Fractique" width="128" height="128">
  <h1>Fractique</h1>
  <p><strong>A fractal flame and wallpaper generator that gives the same picture from the same input, on every platform.</strong></p>
  <p><sub>
    <strong>English</strong> · <a href="docs/readme/README_UK.md">Українська</a> · <a href="docs/readme/README_RU.md">Русский</a> · <a href="docs/readme/README_DE.md">Deutsch</a> · <a href="docs/readme/README_FR.md">Français</a> · <a href="docs/readme/README_ES.md">Español</a> · <a href="docs/readme/README_IT.md">Italiano</a> · <a href="docs/readme/README_JA.md">日本語</a><br>
    <a href="docs/readme/README_CN.md">简体中文</a> · <a href="docs/readme/README_PT.md">Português</a> · <a href="docs/readme/README_KO.md">한국어</a> · <a href="docs/readme/README_HI.md">हिन्दी</a> · <a href="docs/readme/README_TR.md">Türkçe</a> · <a href="docs/readme/README_PL.md">Polski</a> · <a href="docs/readme/README_ID.md">Bahasa Indonesia</a>
  </sub></p>
</div>

Fractique makes two kinds of pictures. Fractal flames: the glowing, wispy kind of fractal made of thin curves and soft light. And wallpapers: flat sheets with lit edges and soft shadows, each in a light and a dark version. You pick a preset, roll a random picture or layer several flames over each other, and export it at any size, from a phone screen to a 4K monitor. A scene is one small file, and the same file always renders to exactly the same pixels, on Windows, macOS, Linux, Android and iOS alike.

<p align="center">
  <img src="docs/samples/aurora-spiral.png" alt="The aurora-spiral preset: teal and green spiral wisps on a dark background" width="880">
</p>

## What it does

- **Flames:** 28 variations, layers with blend modes, rotational and mirror symmetry, density estimation, glow, and 21 built-in presets.
- **Wallpapers:** one for every seed, out of 14 layouts and 6 materials, each in a light and a dark version, laid out for the size of the screen.
- **The app:** one codebase for Windows, macOS, Linux, Android, iOS and the web. The picture fills the window and glass panels float over it; on the desktop the app has a glass title bar of its own. In 15 languages.
- **Randomize and Mutate:** every seed is a picture; flames that would be a dot or a filled square are skipped. Mutate makes a variant of the one you like.
- **Layers:** add a flame from the presets or a random one, remove it, reorder, and set the blend (add, screen, over) and the opacity of each. Up to 16 layers.
- **Picture size:** the window's own, one of 21 screens (computers up to 4K, Android phones and tablets, iPhones and iPads), or any width and height from 16 to 16384 pixels.
- **Deep zoom:** drag, wheel and pinch move the view and magnify it up to ten thousand million times. The picture follows at once and is redrawn sharp when the view rests.
- **Phones:** a screen of its own. One round button makes a new picture, a swipe goes to the next wallpaper, a sheet of small live pictures chooses the style, and the pictures you keep form a collection.
- **Export:** a PNG at any size, in three quality levels, rendered in the background. On a phone it is saved to Photos or the gallery, or shared.
- **Project files:** a `.frcq` file keeps the scene with its view, tone and size, so it opens as it was saved ([docs/scene-format.md](docs/scene-format.md)). Plain `.json` scenes and `.flame` files of flam3 and Apophysis open too. Between launches the app comes back to the session it was left with.
- **Updates:** a window lists every released version with its notes and installs any of them. On Windows the app is one portable `.exe` that replaces itself.
- **Exact repeatability:** the same scene gives the same pixels on every machine, checked by tests on Linux, Windows and macOS ([docs/determinism.md](docs/determinism.md)).
- **The command line tool** (`fractique`): renders flames and wallpapers to PNG, makes random scenes and variants, converts `.flame` files.
- **MCP server** (`fractique mcp`): an AI agent makes, changes and renders scenes ([docs/mcp.md](docs/mcp.md)).

| | |
| --- | --- |
| ![emerald-star](docs/samples/emerald-star.png) | ![ember-vortex](docs/samples/ember-vortex.png) |
| `emerald-star` (two layers) | `ember-vortex` |
| ![golden-bloom](docs/samples/golden-bloom.png) | ![solar-mandala](docs/samples/solar-mandala.png) |
| `golden-bloom` | `solar-mandala` |

Wallpapers, light and dark, each with its layout and material:

| | | | |
| --- | --- | --- | --- |
| ![waves](docs/samples/wallpaper-waves-light.png) | ![burst](docs/samples/wallpaper-burst-light.png) | ![tiles](docs/samples/wallpaper-tiles-light.png) | ![fan](docs/samples/wallpaper-fan-light.png) |
| `waves`, `glass` | `burst`, `glow` | `tiles`, `relief` | `fan`, `glass` |
| ![discs](docs/samples/wallpaper-discs-dark.png) | ![bars](docs/samples/wallpaper-bars-dark.png) | ![arcs](docs/samples/wallpaper-arcs-dark.png) | ![lines](docs/samples/wallpaper-lines-dark.png) |
| `discs`, `paper` | `bars`, `paper` | `arcs`, `carbon` | `lines`, `paper` |

## Quick start

One command builds the app and the command line tool. It installs Rust when it is missing.

| System | Command |
| --- | --- |
| Linux, macOS | `./setup.sh` |
| Windows | `setup.cmd` |

Start the app:

```sh
./target/release/fractique-gui
```

Or render from the command line:

```sh
# a preset as a 4K picture
./target/release/fractique render --preset aurora-spiral --size 3840x2160 --quality high -o aurora.png

# the random flame of seed 42, for a phone screen
./target/release/fractique render --random 42 --size 1080x2400 --quality mobile -o phone.png

# the wallpaper of seed 7 for a phone, light and dark
./target/release/fractique render --wallpaper 7 --size 1290x2796 -o wallpaper.png
./target/release/fractique render --wallpaper 7 --dark --size 1290x2796 -o wallpaper-dark.png

# keep a random scene as a file, change it, render it
./target/release/fractique randomize 42 -o scene.json
./target/release/fractique render scene.json -o mine.png
```

On Windows the programs are `target\release\fractique-gui.exe` and `target\release\fractique.exe`. Everything the app and the commands do: [docs/usage.md](docs/usage.md).

## Documentation

| | |
| --- | --- |
| [docs/usage.md](docs/usage.md) | The app, the commands, the quality levels, how to edit a scene |
| [docs/scene-format.md](docs/scene-format.md) | Every field of a scene file, every variation, sheets, the `.frcq` file |
| [docs/mcp.md](docs/mcp.md) | The MCP server: connecting an AI agent, the tools |
| [docs/ios.md](docs/ios.md) | Building and running on an iPhone or in the iOS Simulator |
| [docs/releasing.md](docs/releasing.md) | How a release is made, how the app updates itself |
| [docs/architecture.md](docs/architecture.md) | How the parts fit together, how a render works |
| [docs/determinism.md](docs/determinism.md) | Why a render repeats exactly, and what is and is not promised |
| [docs/performance.md](docs/performance.md) | Measured render times and memory |
| [docs/roadmap.md](docs/roadmap.md) | What is done and what comes next |

## License

[PolyForm Noncommercial License 1.0.0](LICENSE.md): free for any noncommercial use. For commercial use, ask the author for a separate license. Third-party licenses are listed in [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md). Contributions: see [CONTRIBUTING.md](CONTRIBUTING.md).

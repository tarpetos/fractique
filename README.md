<div align="center">
  <img src="assets/icon/icon-256.png" alt="Fractique" width="128" height="128">
  <h1>Fractique</h1>
  <p><strong>A fractal flame generator that gives the same picture from the same input, on every platform.</strong></p>
  <p><sub>
    <strong>English</strong> · <a href="docs/readme/README_UK.md">Українська</a> · <a href="docs/readme/README_RU.md">Русский</a> · <a href="docs/readme/README_DE.md">Deutsch</a> · <a href="docs/readme/README_FR.md">Français</a> · <a href="docs/readme/README_ES.md">Español</a> · <a href="docs/readme/README_IT.md">Italiano</a> · <a href="docs/readme/README_JA.md">日本語</a><br>
    <a href="docs/readme/README_CN.md">简体中文</a> · <a href="docs/readme/README_PT.md">Português</a> · <a href="docs/readme/README_KO.md">한국어</a> · <a href="docs/readme/README_HI.md">हिन्दी</a> · <a href="docs/readme/README_TR.md">Türkçe</a> · <a href="docs/readme/README_PL.md">Polski</a> · <a href="docs/readme/README_ID.md">Bahasa Indonesia</a>
  </sub></p>
</div>

Fractique renders fractal flames: the glowing, wispy kind of fractal made of thin curves and soft light. You pick a preset, roll a random scene or layer several flames over each other, and render it at any resolution, from a phone screen to a 4K wallpaper. A scene is one small JSON file, and the same file always renders to exactly the same pixels, on Windows, macOS, Linux, Android and iOS alike.

<p align="center">
  <img src="docs/samples/aurora-spiral.png" alt="The aurora-spiral preset: teal and green spiral wisps on a dark background" width="880">
</p>

## What works today

- **The renderer** (`fractique-core`): fractal flames with 28 variations, layers with blend modes, rotational and mirror symmetry, density estimation, glow, any resolution.
- **The app:** one codebase for Windows, Linux, Android, iOS and the web, with glass panels over the flame, in 15 languages.
- **Presets:** 9 built-in scenes.
- **Randomizer:** every seed is a scene; scenes that would be a dot or a filled square are skipped.
- **Mutation:** a variant of a scene you like, with a strength from 0 to 1.
- **Exact repeatability:** the same scene gives the same pixels on every machine, checked by tests on Linux, Windows and macOS ([docs/determinism.md](docs/determinism.md)).
- **Import:** `.flame` files of flam3 and Apophysis convert to scenes.
- **The command line tool** (`fractique`): renders scenes to PNG.
- **MCP server** (`fractique mcp`): an AI agent makes, changes and renders scenes ([docs/mcp.md](docs/mcp.md)).
- **HTTP server** (`fractique-server`): an HTTP API with a render queue and limits, serves the web app, ships as a Docker image ([docs/server.md](docs/server.md)).

| | |
| --- | --- |
| ![emerald-star](docs/samples/emerald-star.png) | ![ember-vortex](docs/samples/ember-vortex.png) |
| `emerald-star` (two layers) | `ember-vortex` |

## Set up

One command builds everything. It installs Rust when it is missing.

| System | Command |
| --- | --- |
| Linux, macOS | `./setup.sh` |
| Windows | `setup.cmd` |

Then:

```sh
# a preset as a 4K wallpaper
./target/release/fractique render --preset aurora-spiral --size 3840x2160 --quality high -o aurora.png

# the random scene of seed 42, for a phone screen
./target/release/fractique render --random 42 --size 1080x2400 --quality mobile -o phone.png

# keep a random scene as a file, change it, render it
./target/release/fractique randomize 42 -o scene.json
./target/release/fractique render scene.json -o mine.png
```

On Windows the tool is `target\release\fractique.exe`. All commands and options: [docs/usage.md](docs/usage.md).

## Documentation

| | |
| --- | --- |
| [docs/usage.md](docs/usage.md) | The commands, the quality levels, how to edit a scene |
| [docs/scene-format.md](docs/scene-format.md) | Every field of a scene file, every variation |
| [docs/mcp.md](docs/mcp.md) | The MCP server: connecting an AI agent, the tools |
| [docs/server.md](docs/server.md) | The HTTP server: endpoints, limits, Docker |
| [docs/ios.md](docs/ios.md) | Building and running on an iPhone or in the iOS Simulator |
| [docs/releasing.md](docs/releasing.md) | How a release is made, how the app updates itself |
| [docs/architecture.md](docs/architecture.md) | How the parts fit together, how a render works |
| [docs/determinism.md](docs/determinism.md) | Why a render repeats exactly, and what is and is not promised |
| [docs/performance.md](docs/performance.md) | Measured render times and memory |
| [docs/roadmap.md](docs/roadmap.md) | What is done and what comes next |

## License

[PolyForm Noncommercial License 1.0.0](LICENSE.md): free for any noncommercial use. For commercial use, ask the author for a separate license. Third-party licenses are listed in [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md). Contributions: see [CONTRIBUTING.md](CONTRIBUTING.md).

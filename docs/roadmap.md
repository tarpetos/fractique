# Roadmap

The order is chosen so that every step is usable on its own and the picture quality is settled before any UI is
built on it.

| Step | What | State |
| --- | --- | --- |
| 1 | **Core:** scene format, renderer, presets, randomizer, exact repeatability | Done |
| 2 | **CLI:** render scenes to PNG | Done |
| 3 | **MCP server** over stdio: presets, random scenes, mutation, validation, `.flame` import, rendering with the image returned ([mcp.md](mcp.md)) | Done |
| 4 | **App:** one codebase for Windows, Linux, macOS, Android, iOS and the web; presets, randomize, mutate, tone, glass, 15 languages. Runs on Windows, Linux, Android, the iOS Simulator and in a browser. Still to do: layers, export, saving, GPU preview | In progress |
| 5 | **Server:** HTTP API with a job queue and limits, web UI (the app compiled to WebAssembly), Docker image | Planned |
| 6 | **Android app** | Planned |
| 7 | **iOS app** (built on GitHub's macOS runners; a device build needs an Apple developer account) | Planned |

## Renderer, next

- Cancelling a render and progress reports through MCP; MCP over HTTP with the server.
- More variations (the ones that take the transform's coefficients: waves, popcorn, rings, fan; and more
  parametric ones).
- More presets, and named palettes in scene files.
- Rendering in tiles, to lower the memory of large supersampled images.
- Progressive rendering for interactive previews.
- Animation: interpolating between two scenes.

## Open decisions

- **UI framework** for the app. It must be Rust and reach all five platforms plus WebAssembly. The candidates are
  checked for their Android and iOS state when step 4 starts.
- **Installers:** `cargo-packager` for Windows, macOS and Linux is the working assumption.

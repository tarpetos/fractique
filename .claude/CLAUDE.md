# Notes for coding agents

Fractique is a deterministic fractal flame generator in Rust. Read [docs/architecture.md](../docs/architecture.md)
before a change that crosses crates, and follow [CONTRIBUTING.md](../CONTRIBUTING.md).

## The one rule that must never break

The same scene and the same render options give the same pixels on every platform, every machine and every thread
count. The golden hashes in `crates/fractique-core/tests/determinism.rs` check it, and CI runs them on Linux, Windows
and macOS. In `fractique-core`:

- Transcendental math goes through `math.rs` (the pure-Rust `libm` crate). Never `f64::sin`, `powf`, `exp` and the
  like: Clippy refuses them (`clippy.toml`). `sqrt`, `floor`, `abs` and plain arithmetic are exact and allowed.
- Threads only ever add integers to the histogram. Never sum floats across threads; float work is per pixel, in a
  fixed order.
- All randomness comes from `rng.rs`, seeded from the scene. No `rand` crate, no clock, no `HashMap` iteration order
  in anything that reaches a pixel.
- A change that alters the output of existing scenes changes the golden hashes: that is a breaking change. Say so in
  the pull request and in `CHANGELOG.md`, and never update the hashes just to make a test pass.

## Names

The name is **Fractique** everywhere: the app's title, the README, the store listings, the repository, the crates
and the commands. The short mark `frcq.` was tried and is on hold: do not use it until the owner brings it back.

## Rules

- **Everything in English:** code, comments, docs, UI text, commit messages.
- **Comments:** almost none. No module headers, no doc comments that say what a function, a struct or a field is, no
  narration. One line, only for a workaround, an outside constraint or where a tuned number comes from. Explanations
  go in `docs/`.
- **The scene format is a contract:** `docs/scene-format.md` changes together with `scene.rs` and `variation.rs`.
  A scene file that loaded before must keep loading and rendering the same; otherwise bump `SCENE_VERSION`.
- **Texts of the app:** never a literal in the code. Every text is a key in `crates/fractique-app/locales/en.json`
  and in each of the other 14 locale files; a test fails when one lacks a key. The texts of the fractal kinds are in
  `locales/fractals/<family>.json`, all 15 languages in one file. The names of the kinds (`f_*`), of their presets
  (`n_<kind>_<preset>`), of the flame presets (`n_flame_<preset>`, in `flame.json`) and of the Mandelbrot and Julia forms
  are translated in every language; a default file name is
  made from the English name (`MandelboxStation`), never from the translated one. After adding texts in Japanese,
  Chinese, Korean or Hindi, run `tools/i18n/make_fonts.py`: the fonts for those hold only the characters in use.
- **README:** `README.md` has 14 translations in `docs/readme/`. A change to one goes into all of them.
- **Dependencies:** after a change to `Cargo.lock`, run `python3 tools/notices/make_notices.py`: it rewrites the table of
  `THIRD_PARTY_NOTICES.md` from what the lock file resolves to.
- **Layers:** `fractique-core` knows nothing about files, terminals, networks or UI. Everything else wraps it.
- **Event-driven:** no polling loops in the app, server or MCP crates; background work posts events.
- Commit and pull request titles: Conventional Commits, see "Commit messages" in CONTRIBUTING.md.
- No attribution lines in commits or pull requests (no "Generated with", no co-author or session trailers).

## Commands

```sh
cargo build --release                               # the CLI: target/release/fractique
cargo test --release                                # unit tests and golden hashes
cargo test --release -p fractique-core --no-default-features   # the same without threads
cargo fmt --all && cargo clippy --all-targets -- -D warnings
cargo clippy -p fractique-app --all-targets --features store -- -D warnings   # the App Store variant
cargo check --target aarch64-apple-ios -p fractique-app                       # the binary too, not only --lib
./target/release/fractique render --preset aurora-spiral --size 960x540 -o /tmp/check.png
```

Look at a render after a change to the renderer, the tone mapping or the randomizer: the tests prove that the output
is stable, not that it is good.

## Where things are

| Path | What |
| --- | --- |
| `crates/fractique-core/src/scene.rs` | The scene format and its validation |
| `crates/fractique-core/src/variation.rs` | The variations (the nonlinear functions) |
| `crates/fractique-core/src/chaos.rs` | The chaos game: one walker per stream |
| `crates/fractique-core/src/render.rs` | Streams, histogram, layers, dithering |
| `crates/fractique-core/src/deep.rs` | Deep views: the particles a magnified view is drawn from |
| `crates/fractique-core/src/tone.rs` | Density estimation, log tone mapping, glow |
| `crates/fractique-core/src/random.rs` | Random scenes, mutation, camera fitting |
| `crates/fractique-core/src/fractal/` | Fractals of other kinds: `mod.rs` registry, one file per kind in a folder per family (`escape`, `root`, `orbit`, `geometric`, `solid`); "How to add a fractal type" in `docs/architecture.md` |
| `crates/fractique-core/presets/` | Built-in scenes, compiled into the binary |
| `crates/fractique-core/src/import.rs` | The `.flame` import |
| `crates/fractique-cli/src/main.rs` | The `fractique` command |
| `crates/fractique-app/` | The app (desktop, Android, iOS, web): `lib.rs` UI, `glass.wgsl` shader, `preview.rs` + `preview.wgsl` GPU preview, `chaos.rs` + `chaos.wgsl` the chaos game on the graphics card, `field.rs` + `field*.wgsl` the fractals the card draws pixel by pixel, `store.rs` the App Store purchase, `i18n.rs` + `locales/` texts |
| `crates/fractique-app/src/update.rs`, `updates.rs` | Checking for, listing and installing releases; its window |
| `crates/fractique-update/` | The `update` command of `fractique`, `fractique-mcp` and `fractique-server`: `tools.json`, the signature, the swap, the refusals; `PUBKEY` for the app too; described in `docs/releasing.md` |
| `scripts/build/`, `.github/workflows/release.yml` (calls `release-*.yml`), `docs/releasing.md` | Installers and the release workflow |
| `.github/workflows/ci.yml` (calls `lint.yml`, `test.yml`) | The checks of every pull request |
| `vendor/` | `eframe` and `egui-wgpu` with the iOS surface fix |
| `crates/fractique-mcp/src/lib.rs` | The MCP tools, shared by both MCP servers; listed in `docs/mcp.md` |
| `crates/fractique-cli/src/mcp.rs` | The MCP server over stdio (`fractique mcp`) |
| `crates/fractique-server/` | The HTTP server (`fractique-server`): `lib.rs` API, queue and limits; `mcp.rs` MCP over HTTP at `/mcp`; `tests/api.rs`, `tests/mcp.rs`; described in `docs/server.md`. `Dockerfile`, `docker-compose.yml` at the root |
| `docs/roadmap.md` | What is done and what comes next |

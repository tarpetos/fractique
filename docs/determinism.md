# Determinism

**The promise:** the same scene, rendered with the same options (size, samples per pixel, supersample), gives the
same pixels, bit for bit, on every platform, every processor and every number of threads.

It matters because a scene is then a complete description of a picture. You can share a 2 kB JSON file instead of
an image, render a preview on a phone and the final image on a server, or let an AI agent work on a scene and know
that what it saw is what you get.

## What makes it hold

| Source of differences | What Fractique does |
| --- | --- |
| `sin`, `cos`, `exp`, `pow` differ between systems (each has its own math library) | All of them come from the pure-Rust `libm` crate through `math.rs`: the same code, the same bits everywhere. Clippy refuses the standard library's versions (`clippy.toml`). |
| Random numbers | An own generator (xoshiro256++, seeded through SplitMix64) written out in `rng.rs`. No dependency can change it. The scene's seed is its only input. |
| The number of cores | The samples are always split over 64 streams, each with its own generator derived from (seed, layer, stream). A machine with 4 cores runs the same 64 streams as one with 32, only fewer at a time. |
| The order threads finish in | Threads only add integers to the histogram. Integer addition gives the same total in any order. No floating-point sum ever crosses a thread. |
| Floating-point rounding | Only IEEE 754 operations that are exactly defined (`+ - * /`, `sqrt`), in a fixed order. No fused multiply-add, no fast-math. Rust does not reorder or fuse them on its own. |
| Graphics cards | Not used for rendering: drivers and vendors round differently. |
| Hash map order, the clock, the locale | None of them is used on the way to a pixel. |

## How it is checked

`crates/fractique-core/tests/determinism.rs` holds golden hashes: the hash of a few presets rendered at a small
size. CI runs them on Linux x64, Linux ARM64, Windows x64 and macOS on Apple Silicon. The same file checks that 1,
3 and all threads give the same image, and the test suite also runs with threads compiled out
(`--no-default-features`).

The command line tool prints the hash of every image it renders, so you can compare two machines yourself:

```sh
fractique render --preset aurora-spiral --size 192x108 --samples 20 --supersample 1
```

## What is not promised

- **Different options give a different image.** 1920×1080 and 3840×2160 show the same picture, but the pixels of
  one are not a scaled copy of the other. The same goes for a different quality level.
- **Across versions.** A new version may improve the renderer. When that changes what existing scenes render to, it
  is listed in [CHANGELOG.md](../CHANGELOG.md) as a breaking change. The `version` field of a scene exists so that
  old scenes can keep their old look when that matters.
- **The PNG file's bytes.** The pixels are the same; the compression is the PNG library's business.

## Rules for changes

1. Math through `math.rs`, randomness through `rng.rs`.
2. Threads add integers; floats stay per pixel.
3. A golden hash changes only on purpose, with a line in the changelog. Never to make a test pass.

# Determinism

**The promise:** the same scene, rendered with the same options (size, samples per pixel, supersample), gives the
same pixels, bit for bit, on every platform, every processor and every number of threads.

It matters because a scene is then a complete description of a picture. You can share a 2 kB JSON file instead of
an image, render a preview on a phone and the final image on a server, or let an AI agent work on a scene and know
that what it saw is what you get. It holds for wallpapers and for fractals of other kinds as it does for flames.

## What makes it hold

| Source of differences | What Fractique does |
| --- | --- |
| `sin`, `cos`, `exp`, `pow` differ between systems (each has its own math library) | All of them come from the pure-Rust `libm` crate through `math.rs`: the same code, the same bits everywhere. Clippy refuses the standard library's versions of these and of every other such function (the inverse and hyperbolic ones, the other logarithms and exponentials, `cbrt`, `hypot`), for `f64` and for `f32` (`clippy.toml`). |
| Random numbers | An own generator (xoshiro256++, seeded through SplitMix64) written out in `rng.rs`. No dependency can change it. The scene's seed is its only input. |
| The number of cores | The samples are always split over 64 streams, each with its own generator derived from (seed, layer, stream). A machine with 4 cores runs the same 64 streams as one with 32, only fewer at a time. |
| The order threads finish in | Threads only add integers to the histogram. Integer addition gives the same total in any order. No floating-point sum ever crosses a thread. |
| Floating-point rounding | Only IEEE 754 operations that are exactly defined (`+ - * /`, `sqrt`), in a fixed order. No fused multiply-add, no fast-math. Rust does not reorder or fuse them on its own. Clippy refuses `mul_add`, and `powi`, whose order of multiplications is not fixed. |
| Graphics cards | Never used by the renderer: drivers and vendors round differently. The app alone uses one, for its screen and its Instant export, where the card is clearly faster and GPU acceleration is on: what that makes is listed under [Which outputs are exact](#which-outputs-are-exact). |
| Wallpapers | The sheets are drawn pixel by pixel, each pixel from the scene alone, with the same `math.rs` functions; rows run on several threads, and no value crosses from one pixel to another. The generator that makes a wallpaper for a seed uses `rng.rs` and `math.rs` only. |
| Fractals drawn per pixel (the escape-time, root-finding and solid kinds, `apollonian`, `lyapunov`) | Every sample is computed from its point alone, with exact arithmetic and the `math.rs` functions, in a fixed order; rows run on several threads and no value crosses from one sample to another. Which pixels get more samples is decided from the finished first pass, which is the same on every machine. What a solid works out before its samples (how deep the rays of every 4 × 4 pixels are clear of it) comes from the picture's size and view alone. |
| Fractals drawn as a density (`clifford`) | The points of the orbits go through the same 64 streams and the same integer histogram as a flame's. |
| The other kinds of the `orbit` family | What a kind measures before a render (the ranks its colors go by, the size it is scaled to, the cells a `buddhabrot` takes its starting points from) is computed on one thread, in a fixed order, from the parameters alone, with a generator of its own. A `lorenz` is integrated with a fixed step. |
| Curves drawn as a density (`kleinian`, `l_system`) | The segments of the curve are found on one thread, in a fixed order, from the scene alone. Each of the 64 streams then spreads its points along them with its own generator, into the same integer histogram. |
| Deep views | The particles a magnified view is drawn from are moved in fixed groups of 128, each with its own generator, so the threads cannot change the result. |
| Hash map order, the clock, the locale | None of them is used on the way to a pixel. |

## How it is checked

`crates/fractique-core/tests/determinism.rs` holds golden hashes, each of a scene rendered at a small size: eleven
presets (one of them at two supersamples), one deep view, seven wallpapers, three random scenes of each of the two
random generators and a scene for each of the 21 variations added to the first 28. Every kind of fractal has the
golden hash of one of its presets in its own file under `src/fractal/`; those were produced on Linux x64 and have
not yet been compared on another system. CI runs them on Linux x64, Linux ARM64, Windows x64 and macOS on Apple
Silicon. The same file checks that the number of threads does not change the image, for a flame (1, 3 and all
threads), a deep view, a wallpaper, the 21 added variations and a random scene of every kind of fractal, and the
test suite also runs with threads compiled out (`--no-default-features`).

The command line tool prints the hash of every image it renders, so you can compare two machines yourself:

```sh
fractique render --preset aurora-spiral --size 192x108 --samples 20 --supersample 1
```

## Which outputs are exact

| Output | Exact, bit for bit? |
| --- | --- |
| The command line tool (`fractique render`) | Yes |
| The MCP tools, over stdio and over HTTP | Yes |
| The HTTP server | Yes |
| The app's export at Standard, High and Ultra | Yes: rendered by the processor, whatever the switch says. The one exception is a view the screen shows as a magnified picture, because drawing it would take over 10 seconds: its export saves what the screen shows |
| The app's screen and its Instant export with GPU acceleration off, or on a device that has no switch | Yes, for the size and the samples of the screen's picture. An Instant export of the same shape and smaller is that picture made smaller, every pixel the mean of those it covers; one that is larger or of another shape is rendered at its own size |
| The app's screen with GPU acceleration on | No, for what the graphics card draws: flames and their deep views, the `clifford`, `de_jong` and `lorenz` attractors, the escape-time and root-finding kinds, and wallpapers. For flames and attractors its random numbers are others, and 32-bit floats round differently from card to card: shapes, colors and brightness are the same; the grain is not. A wallpaper is the processor's but for one step of 255 in two to four of a hundred numbers. What the card leaves to the processor (the other kinds, a scene that mixes a per-pixel kind with a flame, a view too deep for 32-bit numbers) is exact |
| The app's Instant export with GPU acceleration on | No, where the card drew the screen's picture: the file is that picture, or that picture made smaller, or, for a larger size of a fractal the card draws pixel by pixel, drawn by the card in tiles. A flame, an attractor or a wallpaper of a larger size, and any picture of another shape, is rendered by the processor and is exact |

A scene file saved by the app holds the scene, not pixels: rendered by any of the exact outputs it gives the same
image everywhere, whatever drew it on the screen.

## What is not promised

- **Different options give a different image.** 1920×1080 and 3840×2160 show the same picture, but the pixels of
  one are not a scaled copy of the other. The same goes for a different quality level.
- **Across versions.** A new version may improve the renderer. When that changes what existing scenes render to, it
  is listed in [CHANGELOG.md](../CHANGELOG.md) as a breaking change. The `version` field of a scene exists so that
  old scenes can keep their old look when that matters.
- **The PNG file's bytes.** The pixels are the same; the compression is the PNG library's business.
- **The picture on the app's screen.** It has fewer samples and at most 4096 pixels a side (1600 on a phone, 900
  in a browser), a picture of a chosen size is drawn with the pixels it has in the window, and with GPU
  acceleration on it is the graphics card's. A view that would take longer than 10 seconds to draw is the picture
  the app has, magnified. An export at Standard, High or Ultra of a view that is drawn, and a render of the saved
  scene by the command line tool, are exact.
- **A wallpaper's seed across versions.** The wallpaper of a seed may change when its generator improves. A saved
  scene does not: it holds the shapes, not the seed that made them. The same goes for the random fractal of a
  seed (`--fractal`): its scene holds the view and the parameters. The random flame of a seed does not change
  either: the random generators are numbered, tests pin scenes of each, and a better one is added as a new number.
  It changed once: the second generator stopped drawing the variation `rectangles`, and the seeds that drew it give
  another scene since, a breaking change listed in the changelog.

## Rules for changes

1. Math through `math.rs`, randomness through `rng.rs`.
2. Threads add integers; floats stay per pixel.
3. A golden hash changes only on purpose, with a line in the changelog. Never to make a test pass.

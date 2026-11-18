# Performance

All numbers below were measured on one machine: **AMD Ryzen 7 5700X** (8 cores), 12 threads and 16 GB of memory
given to WSL2 on Windows 11, release build. They show what to expect, not a guarantee. The preset is
`aurora-spiral` (one layer); a scene with two layers takes about twice as long.

## Quality levels

| Level | Samples per pixel | Supersample | Meant for |
| --- | --- | --- | --- |
| `draft` | 8 | 1 | Previews while you change a scene |
| `mobile` | 30 | 1 | Phones |
| `standard` | 100 | 1 | The default |
| `high` | 300 | 2 | Wallpapers |
| `ultra` | 1000 | 2 | Prints; slow |

## Render times

| Level | Size | Time |
| --- | --- | --- |
| `draft` | 1920×1080 | 0.9 s |
| `mobile` | 1080×2400 | 1.4 s |
| `standard` | 1920×1080 | 2.2 s |
| `high` | 1920×1080 | 6.9 s |
| `high` | 3840×2160 | 30.4 s |

The time grows with `samples per pixel × width × height` and falls with the number of cores. A view magnified
until it holds less than an eighth of the flame costs five to ten times as much
([architecture.md](architecture.md#any-zoom)). Phones were not measured yet.

## Wallpapers

A wallpaper is drawn exactly, so the quality level and the samples do not change its time. It grows with the
number of pixels and with the number of sheets and shapes of the layout.

| Size | Time, about |
| --- | --- |
| 1290×2796 | 1 s |
| 3840×2160 | 1 to 2 s |

## In the app

The picture on the screen is rendered in passes, so that something shows at once
([architecture.md](architecture.md#the-app)).

| Pass | Longer side, at most | Samples per pixel |
| --- | --- | --- |
| Draft | 480 | 12 |
| In between, for a view magnified more than 4 times | 1280 | 16 |
| Full, on a computer | 4096 | 32 |
| Full, on a phone | 1600 | 18 |
| Full, in a browser (no draft) | 900 | 14 |

Renders use all cores but two, which stay with the window. The toolbar shows the time the full picture took, or
the time of the first picture when the graphics card drew it. An export is a separate render at `standard`,
`high` or `ultra`.

### The first picture on the graphics card

With GPU acceleration on, the graphics card plays the chaos game of a flame itself and shows it at the full size
([architecture.md](architecture.md#the-chaos-game-on-the-graphics-card)); the processor's picture replaces it
later. Measured in the app on Windows 11 with the same processor and an **NVIDIA GeForce RTX 4070** (Vulkan), at
the 32 samples per pixel of the full pass, in seconds from the request. `random-9` is seed 9 of the first random
generator, `random2-3` and `random2-11` are seeds of the second.

| Scene | Layers | Size | First picture | All samples | Processor |
| --- | --- | --- | --- | --- | --- |
| `aurora-spiral` | 1 | 1920×1080 | 0.02 s | 0.05 s | 0.69 s |
| `ember-vortex` | 1 | 1920×1080 | 0.02 s | 0.05 s | 1.22 s |
| `golden-filigree` | 1 | 1920×1080 | 0.02 s | 0.05 s | 0.95 s |
| `random2-3` | 1 | 1920×1080 | 0.02 s | 0.05 s | 1.01 s |
| `solar-mandala` | 2 | 1920×1080 | 0.02 s | 0.06 s | 2.28 s |
| `lilac-cloud` | 2 | 1920×1080 | 0.02 s | 0.12 s | 1.25 s |
| `frost-crown` | 2 | 1920×1080 | 0.02 s | 0.06 s | 1.77 s |
| `random-9` | 2 | 1920×1080 | 0.02 s | 0.06 s | 1.84 s |
| `random2-11` | 2 | 1920×1080 | 0.02 s | 0.06 s | 1.81 s |
| `aurora-spiral` | 1 | 3840×2160 | 0.03 s | 0.16 s | 4.36 s |
| `ember-vortex` | 1 | 3840×2160 | 0.03 s | 0.20 s | 6.79 s |
| `golden-filigree` | 1 | 3840×2160 | 0.03 s | 0.19 s | 5.30 s |
| `random2-3` | 1 | 3840×2160 | 0.04 s | 0.18 s | 5.62 s |
| `solar-mandala` | 2 | 3840×2160 | 0.07 s | 0.34 s | 11.64 s |
| `lilac-cloud` | 2 | 3840×2160 | 0.06 s | 0.35 s | 7.40 s |
| `frost-crown` | 2 | 3840×2160 | 0.05 s | 0.31 s | 9.07 s |
| `random-9` | 2 | 3840×2160 | 0.06 s | 0.39 s | 9.61 s |
| `random2-11` | 2 | 3840×2160 | 0.06 s | 0.27 s | 8.29 s |

"First picture" is when the frame after the first batch of samples begins: the picture is then on its way to the
screen. "All samples" is when the last batch is finished. "Processor" is the full pass of the same app with GPU
acceleration off, on 14 threads. Over the 29 presets and 20 seeds of each random generator at 1920×1080 the first
picture took 0.02 s every time, all samples 0.05 to 0.12 s (0.05 s in the middle) and the processor 0.62 to 2.28 s (1.04 s in
the middle). No other graphics card was measured.

## Memory

A render holds one histogram per layer at a time. Each cell (`width × height × supersample²` of them) takes 28
bytes in the histogram and about 20 more while the layer is developed.

| Size | Supersample | Peak, about |
| --- | --- | --- |
| 1080×2400 | 1 | 125 MB |
| 1920×1080 | 1 | 100 MB |
| 1920×1080 | 2 | 400 MB |
| 3840×2160 | 1 | 400 MB |
| 3840×2160 | 2 | 1.6 GB |

The memory column is computed from the cell sizes, not measured. Rendering in tiles to lower it is on the
[roadmap](roadmap.md). A wallpaper without flame layers has no histogram: it takes about 16 bytes a pixel while it
is drawn.

The app also keeps developed layers, so that a layer that did not change is not rendered again: 12 bytes a pixel
each, up to 1200 MB on a computer and 160 MB on a phone or in a browser; the least recently used go first, the
layers of the whole view last. With GPU acceleration on, the densities of the layers share that room, 16 bytes a
pixel each, and the graphics card holds 8 more for each density that has been on the screen, for as long as it is kept.

While the graphics card plays the chaos game it holds, for each layer, 16 bytes a pixel of histogram, 8 of levels
and up to 8 MB of walkers: about 200 MB a layer at 3840×2160. They are freed when the processor's picture arrives.

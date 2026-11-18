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

## Fractals of other kinds

Measured the same way, with the PNG written, on the presets named. A fractal drawn per pixel does not depend on
the samples per pixel as a flame does: the quality level only sets how many samples the pixels along edges get
(9 at `standard`, 36 at `high`), so its time follows the iterations and how much of the picture is edge.
`mandelbrot-whole` is mostly smooth, `mandelbrot-spiral` is a view magnified 60 000 times with edges everywhere.
`clifford` is a density like a flame and costs about as much.

| Preset | Level | 1920×1080 | 3840×2160 |
| --- | --- | --- | --- |
| `mandelbrot-whole` | `standard` | 0.6 s | 2.6 s |
| `mandelbrot-whole` | `high` | 0.7 s | 2.8 s |
| `mandelbrot-seahorse-valley` | `standard` | 1.1 s | 4.3 s |
| `mandelbrot-seahorse-valley` | `high` | 2.5 s | 10.5 s |
| `mandelbrot-spiral` | `standard` | 2.5 s | 5.6 s |
| `mandelbrot-spiral` | `high` | 5.1 s | 12.1 s |
| `julia-dragon` | `standard` | 0.7 s | 2.9 s |
| `julia-dragon` | `high` | 1.2 s | 4.7 s |
| `clifford-classic` | `standard` | 1.9 s | 9.2 s |
| `clifford-classic` | `high` | 6.6 s | 39.0 s |

At `mobile` and 1080×2400, `mandelbrot-seahorse-valley` took 1.3 s and `clifford-classic` 1.0 s. A Mandelbrot
view magnified 10¹⁰ times with 1500 iterations (9000 steps a point after the zoom) took 4.1 s at 1920×1080 and
`standard`. Phones were not measured, and in the app these kinds are always the processor's.

A per-pixel layer has no histogram: it holds about 32 bytes a pixel while it is drawn, and 64 while its glow is added.

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

Renders use all cores but two, which stay with the window. The toolbar shows the time the picture took. An export
is a separate render at `standard`, `high` or `ultra`.

### The picture of the graphics card

Where the graphics card is used ([architecture.md](architecture.md#when-the-graphics-card-is-used)) and GPU
acceleration is on, the card plays the chaos game of a flame and its picture is the screen's: the processor
renders nothing for it. Measured in the app on Windows 11 with the same processor and an **NVIDIA GeForce RTX
4070** (Vulkan), at the 32 samples per pixel of the full pass. `random2-3` is seed 3 of the second random
generator; `solar-mandala` has two layers, the others one.

| Scene | Size | Card: first picture | Card: whole picture | Processor: whole picture | Processor time, card | Processor time, processor |
| --- | --- | --- | --- | --- | --- | --- |
| `aurora-spiral` | 1920×1080 | 0.02 s | 0.05 s | 0.68 s | 1.3 s | 9.5 s |
| `random2-3` | 1920×1080 | 0.02 s | 0.05 s | 1.00 s | 1.3 s | 12.7 s |
| `solar-mandala` | 1920×1080 | 0.02 s | 0.05 s | 2.37 s | 1.6 s | 29.5 s |
| `aurora-spiral` | 3840×2160 | 0.03 s | 0.17 s | 3.55 s | 2.2 s | 44.5 s |
| `random2-3` | 3840×2160 | 0.04 s | 0.20 s | 5.51 s | 2.7 s | 70.1 s |
| `solar-mandala` | 3840×2160 | 0.07 s | 0.34 s | 12.62 s | 3.0 s | 155.0 s |

"First picture" is when the frame after the first batch of samples begins, "whole picture" when the last batch is
finished, both from the request. The two last columns are the processor time of the whole test run, all cores
added up: the app starts, shows the one scene, writes a screenshot and ends. With the card that is the start and
the screenshot; with the processor rendering the same picture it is the render too.

An **Instant** export of the card's picture, asked for while the first batch was on the screen, was saved 0.6 to
0.7 s later at 1920×1080 and 2.2 to 2.5 s later at 3840×2160; nearly all of that is writing the PNG. The file held
the same pixels as the screen.

The trial that decides whether the card is used took 0.009 s on this card and 0.095 s on the processor. No other
graphics card was measured, and no phone.

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
and up to 8 MB of walkers: about 200 MB a layer at 3840×2160. With the last batch the histogram and the walkers
are freed, and the levels, 66 MB a layer at that size, stay for as long as the picture is on the screen.

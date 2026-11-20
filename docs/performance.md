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
| `draft` | 1920×1080 | 1.0 s |
| `mobile` | 1080×2400 | 1.6 s |
| `standard` | 1920×1080 | 2.4 s |
| `high` | 1920×1080 | 7.5 s |
| `high` | 3840×2160 | 34.7 s |

The time grows with `samples per pixel × width × height` and falls with the number of cores. A view magnified
until it holds less than an eighth of the flame costs five to ten times as much
([architecture.md](architecture.md#any-zoom)). Phones were not measured yet.

## Fractals of other kinds

Measured with `fractique render`, with the PNG written, on the presets named and on all 12 threads. Every render
was run three times, one at a time, with nothing else rendering or compiling on the machine, and the fastest time is
the one given. A fractal drawn per pixel does not depend on the samples per
pixel as a flame does: the quality level only sets how many samples the pixels along edges get (9 at `standard`, 36
at `high`), so its time follows the iterations and how much of the picture is edge. `mandelbrot-whole` is mostly
smooth, `mandelbrot-spiral` is a view magnified 60 000 times with edges everywhere. `clifford` is a density like a
flame and costs about as much.

| Preset | Level | 1920×1080 | 3840×2160 |
| --- | --- | --- | --- |
| `mandelbrot-whole` | `standard` | 1.1 s | 4.5 s |
| `mandelbrot-whole` | `high` | 1.4 s | 5.0 s |
| `mandelbrot-seahorse-valley` | `standard` | 1.0 s | 4.7 s |
| `mandelbrot-seahorse-valley` | `high` | 2.5 s | 7.5 s |
| `mandelbrot-spiral` | `standard` | 1.3 s | 4.9 s |
| `mandelbrot-spiral` | `high` | 2.7 s | 9.9 s |
| `julia-dragon` | `standard` | 0.7 s | 2.9 s |
| `julia-dragon` | `high` | 1.2 s | 4.6 s |
| `clifford-classic` | `standard` | 1.8 s | 9.1 s |
| `clifford-classic` | `high` | 6.6 s | 40.2 s |

The other kinds of the `orbit` family are densities too and cost about what `clifford` does, here at `standard`
with `clifford-classic` given again as the yardstick:

| Preset | 1920×1080 | 3840×2160 |
| --- | --- | --- |
| `clifford-classic` | 1.8 s | 9.1 s |
| `de_jong-classic` | 2.2 s | 12.0 s |
| `gumowski_mira-feathers` | 1.3 s | 6.6 s |
| `hopalong-web` | 1.7 s | 10.7 s |
| `lorenz-butterfly` | 1.9 s | 9.3 s |
| `lorenz-veil` | 1.8 s | 10.4 s |
| `buddhabrot-classic` | 2.3 s | 13.7 s |
| `buddhabrot-nebula` | 2.2 s | 13.2 s |
| `buddhabrot-filigree` | 2.4 s | 13.3 s |
| `buddhabrot-anti` | 1.8 s | 10.2 s |

A `buddhabrot` plots as many points as any density, so its time does not grow with its iterations: what it adds
is the orbits it starts and does not draw. Dividing the plane into cells takes 2 to 8 ms before a render. Of the
starting points the cells leave, about one in 10 is drawn at 1000 iterations with `shortest` 10, and about one in 750 for
`buddhabrot-filigree` (20 000 iterations, `shortest` 2000), where blind sampling over the disc would draw one in
10 000. The random scene of a seed takes 0.03 to 0.15 s to make, a `buddhabrot` the longest.

At `mobile` and 1080×2400, `mandelbrot-seahorse-valley` took 1.0 s and `clifford-classic` 0.8 s. A Mandelbrot view
magnified 10¹⁰ times with 1500 iterations (9000 steps a point after the zoom) took 4.1 s at 1920×1080 and
`standard`, measured earlier and not again with the tables. Phones were not measured. What the graphics card takes
for these kinds in the app is [further down](#fractals-on-the-graphics-card).

A per-pixel layer has no histogram: it holds about 32 bytes a pixel while it is drawn, and 64 while its glow is added.

### The later escape-time kinds and the root-finding kinds

Measured at `standard`, the same way as the tables above; `mandelbrot-seahorse-valley` is given again for scale.
For each kind the first row is its whole picture and the others are magnified views, where most pixels lie along an
edge and are drawn from 9 samples.

| Preset | 1920×1080 | 3840×2160 |
| --- | --- | --- |
| `mandelbrot-seahorse-valley` (for scale) | 1.0 s | 4.7 s |
| `burning_ship-whole` | 0.5 s | 2.1 s |
| `burning_ship-ship` | 0.8 s | 3.2 s |
| `burning_ship-towers` | 1.1 s | 4.6 s |
| `tricorn-whole` | 0.6 s | 2.2 s |
| `tricorn-spiral` | 2.4 s | 9.5 s |
| `multibrot-cubic` | 0.6 s | 2.3 s |
| `multibrot-sunflower` (power 6) | 3.2 s | 12.6 s |
| `phoenix-ushiki` | 0.7 s | 2.7 s |
| `phoenix-feathers` | 2.6 s | 10.1 s |
| `magnet-first` | 0.9 s | 3.6 s |
| `magnet-pearls` | 1.8 s | 7.1 s |
| `newton-cubic` | 0.6 s | 2.2 s |
| `newton-swirl` | 0.8 s | 3.1 s |
| `nova-whole` | 1.0 s | 3.5 s |
| `nova-sapphire` | 2.1 s | 8.1 s |

A step of `magnet` costs two complex divisions and one of `multibrot` a multiplication for every unit of its
power, so they are slower than `mandelbrot` at the same depth; a power that is not whole costs a logarithm, a
sine and a cosine a step. `newton` measures the distance to every root at every step.

### Geometric kinds

Measured the same way as the tables above; `clifford-classic` and `mandelbrot-whole` are given again for scale.

| Preset | 1920×1080, `standard` | 3840×2160, `standard` |
| --- | --- | --- |
| `apollonian-classic` | 0.4 s | 1.5 s |
| `apollonian-gap` | 0.5 s | 1.8 s |
| `lyapunov-ribbons` (400 iterations) | 2.4 s | 7.3 s |
| `lyapunov-lattice` (800 iterations) | 5.1 s | 18.1 s |
| `l_system-dragon` | 1.4 s | 9.8 s |
| `l_system-plant` | 1.2 s | 5.2 s |
| `kleinian-pearls` | 1.3 s | 7.2 s |
| `kleinian-lace` | 3.5 s | 14.7 s |
| `mandelbrot-whole`, for scale | 1.1 s | 4.5 s |
| `clifford-classic`, for scale | 1.8 s | 9.1 s |

At `high` and 1920×1080: `apollonian-classic` 0.4 s, `lyapunov-ribbons` 3.6 s, `kleinian-pearls` 3.6 s,
`l_system-dragon` 2.6 s. At `mobile` and 1080×2400: 0.4 s, 1.4 s, 0.6 s and 0.6 s.

An Apollonian gasket is the cheapest picture of all: a point needs a few inversions. A Lyapunov fractal costs
its `warmup` and `iterations` at every sample, and its chaotic part is all edges, which are drawn from nine
samples a pixel at `standard`. The curves cost what a density costs, and more where they light more of the
picture (`kleinian-lace`). Before its points are drawn a curve is built on one thread: the search of a
`kleinian` preset takes under 0.2 s, an `l_system` of a million steps about as long, and the segments are held
in memory while the layer renders, 56 bytes each (at most 2 000 000 for a `kleinian`, 1 048 576 for an
`l_system`, which holds as much again while it is built: 110 MB at its largest).

### Solids

Measured the same way as the tables above.

| Preset | `standard` 1920×1080 | `standard` 3840×2160 | `high` 1920×1080 |
| --- | --- | --- | --- |
| `mandelbulb-classic` | 1.1 s | 5.1 s | 2.7 s |
| `mandelbulb-coral` | 4.2 s | 18.6 s | 13.8 s |
| `mandelbox-cube` | 1.5 s | 4.9 s | 2.7 s |
| `mandelbox-station` | 4.6 s | 20.1 s | 12.6 s |
| `menger_sponge-classic` | 0.6 s | 2.6 s | 1.4 s |
| `menger_sponge-courtyard` | 1.7 s | 6.4 s | 4.1 s |

A solid seen whole (`mandelbulb-classic`, `mandelbox-cube`, `menger_sponge-classic`) leaves much of the picture
empty and its rays reach it in a few steps. A view from near or inside (`mandelbulb-coral`, `mandelbox-station`)
fills the picture, most of it edge: about four times the work. At `mobile` and 1080×2400, `mandelbulb-classic` took
0.7 s and `mandelbox-station` 3.2 s. The app's draft, 480 pixels wide with one sample a pixel, took under 0.1 s for
each of these: a ray stops at a pixel's width from the solid, so a small picture walks fewer steps as well as fewer
rays.

Where the time goes, on `mandelbox-station`: a ray takes about 23 estimates to reach the solid after the look
ahead (65 without it), 9 for the normal and the pits and about 11 for the shadow; `shadow` 0 saves those. The
iterations of a kind multiply all of it.

## Wallpapers

A wallpaper is drawn exactly, so the quality level and the samples do not change its time. It grows with the
number of pixels and with the number of sheets and shapes of the layout.

| Size | Time, about |
| --- | --- |
| 1290×2796 | 1 s |
| 3840×2160 | 1 to 2 s |

In the app with GPU acceleration on, the graphics card draws the wallpaper
([below](#wallpapers-on-the-graphics-card)).

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

The trial that decides whether the card is used took 0.009 s on this card and 0.095 s on the processor. These
numbers are from before [the bounds of a submission](architecture.md#the-work-of-one-submission): with them a
whole picture takes more batches, and the card was not measured again.

With the bounds, on a laptop with Windows 11 and an **NVIDIA GeForce GTX 1650** (Vulkan), in a window nobody
sees, at 32 samples per pixel. `solar-mandala` has two layers, the others one. "Card at work" is the sum of the
batches, "longest batch" the one that took the card longest; between two batches the app waits for a frame, which
took about 0.1 s in these runs, so the whole picture comes that much later for every batch.

| Scene | Size | First picture | Batches | Card at work | Longest batch | Whole picture | Processor: whole picture |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `clifford-classic` | 1280×720 | 0.07 s | 10 | 0.13 s | 16 ms | 0.77 s | |
| `de_jong-classic` | 1280×720 | 0.08 s | 10 | 0.15 s | 18 ms | 0.79 s | |
| `lorenz-butterfly` | 1280×720 | 0.05 s | 10 | 0.07 s | 14 ms | 0.69 s | |
| `aurora-spiral` | 1280×720 | 0.03 s | 9 | 0.07 s | 16 ms | 0.58 s | |
| `solar-mandala` | 1280×720 | 0.04 s | 17 | 0.42 s | 26 ms | 1.83 s | |
| `clifford-classic` | 1920×1080 | 0.09 s | 18 | 0.46 s | 29 ms | 2.00 s | 2.38 s |
| `de_jong-classic` | 1920×1080 | 0.09 s | 18 | 0.48 s | 31 ms | 2.03 s | 3.05 s |
| `lorenz-butterfly` | 1920×1080 | 0.07 s | 18 | 0.30 s | 20 ms | 1.83 s | 2.26 s |
| `aurora-spiral` | 1920×1080 | 0.03 s | 17 | 0.29 s | 19 ms | 1.70 s | |
| `solar-mandala` | 1920×1080 | 0.05 s | 33 | 1.58 s | 51 ms | 4.79 s | |
| `clifford-classic` | 2560×1440 | 0.10 s | 31 | 1.06 s | 38 ms | 4.06 s | |
| `de_jong-classic` | 2560×1440 | 0.10 s | 31 | 1.08 s | 40 ms | 4.09 s | |
| `lorenz-butterfly` | 2560×1440 | 0.09 s | 30 | 0.88 s | 35 ms | 3.76 s | |
| `aurora-spiral` | 2560×1440 | 0.04 s | 30 | 0.99 s | 46 ms | 3.86 s | |
| `solar-mandala` | 2560×1440 | 0.08 s | 58 | 4.24 s | 76 ms | 10.26 s | |

These runs were made while a flame was held to the bounds of the attractors, which left a picture of 3840×2160 to
the processor (`aurora-spiral` took it 14.4 s on this laptop). A flame is no longer held to them and a picture of
that size is the card's again; with the bound lifted for a measurement this card needed up to 153 ms a batch for
one layer of that size and up to 282 ms for two. It has not been timed on this laptop since.

The same build on a Debian 13 box with the graphics chip of its processor, an **AMD Radeon Vega 3** (Vulkan,
RADV), far weaker: at 1920×1080 the longest batch took 120 ms for `clifford-classic`, 148 ms for
`de_jong-classic`, 100 ms for `lorenz-butterfly`, 190 ms for `aurora-spiral` and 301 ms for `solar-mandala`;
at 1280×720, 65, 78, 47, 73 and 110 ms. All stay under the 0.5 s after which the card is given up.

### Deep views on the graphics card

A magnified view that holds less than an eighth of the flame's points is drawn by the card from particles
([architecture.md](architecture.md#deep-views-on-the-graphics-card)); before, the card gave such a view up and
the processor drew it. Measured on Windows 11 with the **NVIDIA GeForce RTX 4070** (Vulkan) and the Ryzen 7 5700X
by the test `deep_views_on_the_graphics_card_beside_the_processor` of `chaos.rs`, without a window, at 32
samples per pixel. Each preset is magnified about a point of its picture that is lit as half of its lit pixels
are. "First picture" and "whole picture" are from the request, the plan of the view included; "processor" is
`fractique-core` rendering the same view on all 16 threads.

| Scene | Magnified | Size | Card: first picture | Card: whole picture | Processor |
| --- | --- | --- | --- | --- | --- |
| `aurora-spiral` | 3 | 3840×2160 | 0.27 s | 1.92 s | 36.5 s |
| `aurora-spiral` | 14 | 3840×2160 | 0.18 s | 1.85 s | 48.2 s |
| `aurora-spiral` | 100 | 3840×2160 | 0.32 s | 1.65 s | 42.0 s |
| `aurora-spiral` | 1000 | 3840×2160 | not drawn | | 46.2 s |
| `aurora-spiral` | 3 | 1920×1080 | 0.13 s | 0.46 s | 8.1 s |
| `aurora-spiral` | 14 | 1920×1080 | 0.14 s | 0.45 s | 10.1 s |
| `aurora-spiral` | 100 | 1920×1080 | 0.11 s | 0.33 s | 9.7 s |
| `aurora-spiral` | 1000 | 1920×1080 | 0.16 s | 0.39 s | 11.2 s |
| `solar-mandala` | 14 | 1920×1080 | 0.25 s | 0.54 s | 12.7 s |
| `solar-mandala` | 100 | 1920×1080 | 0.25 s | 0.54 s | 14.2 s |
| `solar-mandala` | 1000 | 1920×1080 | 0.33 s | 0.67 s | 26.2 s |
| `ember-vortex` | 14 | 1920×1080 | 0.07 s | 0.17 s | 14.2 s |
| `ember-vortex` | 100 | 1920×1080 | 0.08 s | 0.25 s | 17.6 s |
| `ember-vortex` | 1000 | 1920×1080 | not drawn | | 19.4 s |

"Not drawn": the view is too fine for 32-bit floats where it is (cells a unit times the distance from the
origin above 400 000), which is known before anything is sent; the processor renders it. `solar-mandala` and
`ember-vortex` magnified 3 times still hold more than an eighth of their points at that place and are drawn by
the plain chaos game (0.23 s and 0.02 s). The longest submission of these runs took 0.12 s; a step of the plan
took 1 to 36 ms.

In the app, in a window nobody sees, `aurora-spiral` at 3840×2160 about the point (-0.9, 0.56), one view after
another: magnified 14 times, the first picture came 0.17 s after the request and the whole picture after 1.53 s
(the processor: 33 to 36 s); moved by a hundredth of the view, 0.18 s and 1.54 s; magnified 100 times, 0.23 s
and 1.24 s; 300 times, 0.28 s and 1.53 s; and back to the whole flame, 0.08 s. No view went to the processor.

How far the card's picture is from the processor's, as the mean and the largest difference between the averages
of 8 × 8 blocks on the scale of 0 to 255, beside the mean difference between two renders of the processor whose
seeds differ. A deep view is drawn from a few thousand particles that share their latest steps, so two renders of
one view differ far more than two renders of a whole flame do.

| Scene | Magnified | Size | Card to processor: mean | Largest | Processor to processor, another seed: mean |
| --- | --- | --- | --- | --- | --- |
| `aurora-spiral` | 3 | 3840×2160 | 1.72 | 18.5 | 1.38 |
| `aurora-spiral` | 14 | 3840×2160 | 3.38 | 52.0 | 2.33 |
| `aurora-spiral` | 100 | 3840×2160 | 3.78 | 59.8 | 3.34 |
| `aurora-spiral` | 3 | 1920×1080 | 2.00 | 15.6 | 1.71 |
| `aurora-spiral` | 14 | 1920×1080 | 4.59 | 54.3 | 3.20 |
| `aurora-spiral` | 100 | 1920×1080 | 4.86 | 61.4 | 3.19 |
| `aurora-spiral` | 1000 | 1920×1080 | 2.33 | 28.3 | 2.15 |
| `solar-mandala` | 14 | 1920×1080 | 2.49 | 28.2 | 2.45 |
| `solar-mandala` | 100 | 1920×1080 | 6.94 | 110.5 | 7.86 |
| `solar-mandala` | 1000 | 1920×1080 | 32.33 | 230.9 | 38.35 |
| `ember-vortex` | 14 | 1920×1080 | 2.51 | 65.0 | 2.24 |
| `ember-vortex` | 100 | 1920×1080 | 6.61 | 77.0 | 4.95 |

The card's picture is as far from the processor's as the processor's is from its own with another seed, or a
little farther. Both are estimates: at 640×360 the plain chaos game, played on the card until as many points
were in the picture (0.5 to 4 s of it), was 10.3, 9.2 and 7.7 away from the card's deep view of `aurora-spiral`
magnified 14 and 100 times and of `ember-vortex` magnified 14 times, and 7.8, 12.4 and 5.8 away from the
processor's (`a_deep_view_beside_the_plain_chaos_game`): the plain chaos game shows more faint threads than
either.

### Fractals on the graphics card

The escape-time and the root-finding kinds are drawn pixel by pixel by the graphics card
([architecture.md](architecture.md#fractals-drawn-pixel-by-pixel-on-the-graphics-card)). Measured in the app, in
a window nobody sees, at the 32 samples per pixel of the full pass: the seconds from the request to the whole
picture, of the card and of the processor of the same machine. The first of each kind is its first preset, the
second one of its deeper or longer ones.

A laptop with Windows 11 and an **NVIDIA GeForce GTX 1650** (Vulkan):

| Scene | 640×400: card | Processor | 1280×720: card | Processor | 1920×1080: card | Processor |
| --- | --- | --- | --- | --- | --- | --- |
| `mandelbrot-whole` | 0.01 s | 0.05 s | 0.01 s | 0.27 s | 0.02 s | 0.49 s |
| `mandelbrot-seahorse-valley` | 0.02 s | 0.26 s | 0.04 s | 0.56 s | 0.09 s | 1.10 s |
| `julia-dragon` | 0.02 s | 0.10 s | 0.02 s | 0.35 s | 0.04 s | 0.71 s |
| `julia-siegel-disc` | 0.02 s | 0.27 s | 0.04 s | 0.60 s | 0.06 s | 1.28 s |
| `burning_ship-whole` | 0.02 s | 0.07 s | 0.02 s | 0.30 s | 0.03 s | 0.57 s |
| `burning_ship-cathedral` | 0.03 s | 0.32 s | 0.06 s | 0.74 s | 0.12 s | 1.46 s |
| `tricorn-whole` | 0.02 s | 0.07 s | 0.02 s | 0.29 s | 0.02 s | 0.51 s |
| `tricorn-coast` | 0.02 s | 0.27 s | 0.04 s | 0.56 s | 0.06 s | 1.10 s |
| `multibrot-cubic` | 0.01 s | 0.09 s | 0.02 s | 0.31 s | 0.03 s | 0.56 s |
| `multibrot-flower` | 0.02 s | 0.10 s | 0.04 s | 0.35 s | 0.06 s | 0.67 s |
| `phoenix-ushiki` | 0.02 s | 0.08 s | 0.02 s | 0.34 s | 0.03 s | 0.66 s |
| `phoenix-galaxy` | 0.03 s | 0.15 s | 0.06 s | 0.54 s | 0.08 s | 1.02 s |
| `magnet-first` | 0.02 s | 0.27 s | 0.03 s | 0.54 s | 0.04 s | 1.04 s |
| `magnet-pearls` | 0.03 s | 0.41 s | 0.06 s | 1.03 s | 0.10 s | 2.14 s |
| `newton-cubic` | 0.02 s | 0.07 s | 0.03 s | 0.28 s | 0.03 s | 0.49 s |
| `newton-swirl` | 0.03 s | 0.27 s | 0.06 s | 0.52 s | 0.11 s | 0.97 s |
| `nova-whole` | 0.03 s | 0.27 s | 0.04 s | 0.51 s | 0.08 s | 0.94 s |
| `nova-julia` | 0.08 s | 0.35 s | 0.24 s | 0.92 s | 0.38 s | 1.78 s |

The first picture came after 0.01 to 0.02 s. A picture took 6 to 696 batches (the most for `nova-julia` at
1920×1080, whose samples may take 5400 steps each), and the longest batch of all took 15 ms. The card's pictures
differ from the processor's by 0.3 to 0.4 of 255 on average and by 4 at most, in blocks of 8 × 8 pixels.

A Debian 13 box with the graphics chip of its processor, an **AMD Radeon Vega 3** (Vulkan, RADV), at 1920×1080.
The app there notices the processor's picture up to a second late, so the time the command line tool took for
the same picture is beside it:

| Scene | Card | Batches | Longest batch | Processor, in the app | Processor, `fractique render` |
| --- | --- | --- | --- | --- | --- |
| `mandelbrot-whole` | 0.16 s | 21 | 23 ms | 1.1 s | 1.3 s |
| `mandelbrot-seahorse-valley` | 0.46 s | 90 | 20 ms | 3.1 s | 2.4 s |
| `julia-dragon` | 0.19 s | 37 | 21 ms | 2.1 s | 1.8 s |
| `julia-siegel-disc` | 0.34 s | 62 | 21 ms | 3.5 s | 2.7 s |
| `burning_ship-whole` | 0.18 s | 32 | 23 ms | 2.1 s | 1.3 s |
| `burning_ship-cathedral` | 0.46 s | 193 | 22 ms | 3.1 s | 2.9 s |
| `tricorn-whole` | 0.17 s | 33 | 22 ms | 1.1 s | 1.3 s |
| `tricorn-coast` | 0.30 s | 91 | 19 ms | 2.1 s | 2.3 s |
| `multibrot-cubic` | 0.14 s | 31 | 19 ms | 2.1 s | 1.4 s |
| `multibrot-flower` | 0.28 s | 92 | 25 ms | 2.1 s | 1.6 s |
| `phoenix-ushiki` | 0.22 s | 29 | 22 ms | 2.1 s | 1.6 s |
| `phoenix-galaxy` | 0.42 s | 135 | 22 ms | 3.3 s | 2.5 s |
| `magnet-first` | 0.26 s | 42 | 27 ms | 3.1 s | 2.6 s |
| `magnet-pearls` | 0.61 s | 135 | 20 ms | 6.0 s | 5.0 s |
| `newton-cubic` | 0.15 s | 37 | 19 ms | 1.1 s | 1.3 s |
| `newton-swirl` | 0.41 s | 222 | 29 ms | 3.1 s | 2.5 s |
| `nova-whole` | 0.26 s | 121 | 18 ms | 3.1 s | 2.4 s |
| `nova-julia` | 1.16 s | 695 | 18 ms | 6.1 s | 4.1 s |

At 640×400 the card took 0.03 to 0.26 s there and at 1280×720 0.10 to 0.69 s; the longest batch of all took
49 ms, and the pictures differ from the processor's by 0.3 of 255 on average and by 4 at most. No other card
was measured.

### A view that moves

While a view of a kind drawn pixel by pixel is dragged or wheeled, the screen shows a small draft of the view
itself ([architecture.md](architecture.md#motion)). Measured on an **NVIDIA GeForce RTX 4070** (Vulkan), in a
window nobody sees, with a picture of 3840 × 2160, from the trace alone (no frames saved): a drag of 40 frames
sideways and 30 steps of the wheel outwards, both at twice the framing. "Not the picture" is the share of the view
that the picture on the screen does not cover, where the blurred continuation or the dark is shown.

| Scene, movement | Not the picture: before, mean and most | After | The draft, the card's time for one | A frame of the window: before | After | From the last move to the picture: before | After |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `burning_ship-cathedral`, drag | 20 %, 39 % | 0.03 %, 1 % in the first frame, then none | 1200 × 675, 2.6 ms | 0.6 ms, 1.0 at most | 0.9 ms, 1.2 at most | 0.12 s | 0.14 s |
| `burning_ship-cathedral`, wheel | 55 %, 84 % | 0.4 %, 6 % in one frame | 1200 × 675, 2.8 ms | 0.8 ms, 1.2 | 1.0 ms, 1.4 | 0.14 s | 0.13 s |
| `mandelbrot-whole`, drag | 20 %, 39 % | 0.03 %, 1 % in the first frame, then none | 1200 × 675 and 1600 × 900, 0.9 ms | 0.7 ms, 0.9 | 0.9 ms, 1.4 | 0.02 s | 0.03 s |
| `mandelbrot-whole`, wheel | 55 %, 84 % | 0.4 %, 6 % in one frame | 1200 × 675, 0.7 ms | 0.8 ms, 1.0 | 1.0 ms, 1.2 | 0.02 s | 0.03 s |
| `nova`, first preset, drag | 20 %, 39 % | 0.03 %, 1 % in the first frame, then none | 1200 × 675, then 800 × 450, 3.1 ms | 0.7 ms, 1.0 | 1.0 ms, 1.4 | 0.10 s | 0.14 s |

The first frame of a movement has no draft yet and shows the picture it had, moved by that one frame. The wheel
of the test run turns after the frame has decided what to draw, so its first draft is a frame later than a real
wheel's. The picture comes one frame later than before, the frame the draft of the resting view takes. Before,
the picture of a view that moved began anew with every frame and was never whole while the view moved, also for
the scenes the card draws in 0.02 s: at 3840 × 2160 no kind's first preset is whole within a frame (0.014 s for
`mandelbrot-whole` to 0.18 s for `nova`).

Flames, attractors and deep views are not drafted. Tried through the chaos game as it is, at 1, 4 and 16 samples
a pixel: a draft of `aurora-spiral` of 1200 × 675 took the card 0.23 to 0.28 s over 0.9 s of frames, one of
400 × 225 0.02 to 0.07 s over 0.1 s, since the walk, the look over the tiles and each strip of the levels are a
submission and a frame each; and it shows the bright core with the fine threads as specks, which is not the
picture. They keep the picture they had, moved: 10 % of the view is not the picture on average and 20 % at most
in a drag of a fifth of the window, 55 % and 84 % in the wheel run above, and the picture comes 0.8 to 1.0 s
after the last move for the flame on the card (0.7 s on the processor at 1600 × 1040, 1.7 s for `clifford`).

### Wallpapers on the graphics card

A wallpaper is drawn by the graphics card
([architecture.md](architecture.md#wallpapers-on-the-graphics-card)). Measured by the test
`how_fast_the_graphics_card_draws_wallpapers` of `sheets.rs`, without a window, on Windows 11 with an **NVIDIA
GeForce RTX 4070** (Vulkan) and a Ryzen 7 5700X: the seconds from the request to the whole picture, of the card
and of `fractique_core::render` on all threads, for seed 7 of six of the 14 layouts.

| Layout | 1920×1080: card | Processor | 3840×2160: card | Batches | Processor |
| --- | --- | --- | --- | --- | --- |
| `bands` | 0.001 s | 0.05 s | 0.002 s | 6 | 0.19 s |
| `waves` | 0.002 s | 0.22 s | 0.017 s | 24 | 0.88 s |
| `burst` | 0.003 s | 0.15 s | 0.02 to 0.06 s | 28 | 0.59 s |
| `tiles` | 0.001 s | 0.06 s | 0.013 s | 13 | 0.25 s |
| `frames` | 0.001 s | 0.13 s | 0.022 s | 14 | 0.59 s |
| `lines` | 0.002 s | 0.07 s | 0.022 s | 17 | 0.29 s |

Over all 14 layouts the whole picture of 1920 × 1080 took 1 to 3 ms in 1 to 8 batches and that of 3840 × 2160
2 to 62 ms in 6 to 29; the first picture of 3840 × 2160 was there after 1 to 10 ms, and the longest batch of
all took 10 ms. The heaviest scene the format allows (48 sheets of 32 waves of 4 swells) at 96 × 64 took 5
batches, the longest 2 ms. In the app, in a window nobody sees, the first picture of a wallpaper of
3840 × 2160 came 4 ms after the request and the whole one after 20 ms, in 18 batches; at 1920 × 1080 the whole
one came after 6 ms, where the processor took 0.24 s. No other card was measured.

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

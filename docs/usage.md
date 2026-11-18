# Using Fractique

Today Fractique is a command line tool, `fractique`. This guide covers every command, with examples. For the fields
of a scene file see [scene-format.md](scene-format.md); for render times see [performance.md](performance.md).

## Contents

1. [Install](#1-install)
2. [Render a preset](#2-render-a-preset)
3. [Size and quality](#3-size-and-quality)
4. [Random scenes](#4-random-scenes)
5. [Edit a scene](#5-edit-a-scene)
6. [Layers](#6-layers)
7. [Variants of a scene](#7-variants-of-a-scene)
8. [Flames from other programs](#8-flames-from-other-programs)
9. [Troubleshooting](#9-troubleshooting)

---

## 1. Install

You need the repository and one command. It installs Rust when it is missing, builds the tool and runs the tests.

| System | Command | Result |
| --- | --- | --- |
| Linux, macOS | `./setup.sh` | `target/release/fractique` |
| Windows | `setup.cmd` | `target\release\fractique.exe` |

`--skip-tests` leaves the tests out. On Windows the script also installs the Microsoft C++ Build Tools (several GB)
when they are missing: Rust needs them to link programs. On Linux and macOS it needs a C compiler and tells you how
to install one.

The examples below write `fractique` for the tool; use the path from the table, or copy the file to a folder on
your `PATH`.

## 2. Render a preset

```sh
fractique presets                                  # the names
fractique render --preset aurora-spiral -o aurora.png
```

Without `-o` the image is written to `fractique.png` in the current folder. While it renders, the tool shows the
progress; at the end it prints the size, the time and the image's hash:

```
aurora.png (1920x1080, 100 samples per pixel, 2.2 s, hash 1b440b3a4f4106cd)
```

The hash is the same on every machine for the same scene and options: that is how you check that two computers
rendered the same picture.

## 3. Size and quality

```sh
fractique render --preset emerald-star --size 3840x2160 --quality high -o star-4k.png
fractique render --preset emerald-star --size 1080x2400 --quality mobile -o star-phone.png
```

`--size` takes any `WIDTHxHEIGHT` up to 16384 a side. The picture is framed by its shorter side: a wider or taller
image shows more of the surroundings, it does not stretch or crop the middle.

| `--quality` | Samples per pixel | Supersample | Use |
| --- | --- | --- | --- |
| `draft` | 8 | 1 | A quick look |
| `mobile` | 30 | 1 | Phones |
| `standard` (default) | 100 | 1 | Everyday |
| `high` | 300 | 2 | Wallpapers |
| `ultra` | 1000 | 2 | Prints |

`--samples N` and `--supersample N` replace the level's numbers. More samples means less grain; supersample 2
renders at twice the size and averages, which sharpens thin lines and costs four times the memory.

## 4. Random scenes

Every number is a scene:

```sh
fractique render --random 42 -o r42.png
fractique render --random 43 -o r43.png
```

To keep one, write it to a file. The file is the scene; the number is no longer needed:

```sh
fractique randomize 42 -o scene.json
fractique render scene.json -o mine.png
```

A random scene of a seed may change between versions of Fractique when the randomizer improves. A saved scene file
does not.

## 5. Edit a scene

A scene is a JSON file ([scene-format.md](scene-format.md)). Start from a preset or a random scene:

```sh
fractique preset aurora-spiral > scene.json
```

Changes that are easy to try by hand:

| Change | Field |
| --- | --- |
| Move or zoom the view | `camera.center`, `camera.zoom`, `camera.rotation` |
| Brighter, softer, more glow | `tone.brightness`, `tone.gamma`, `tone.glow_radius`, `tone.glow_strength` |
| Other colors | `palette.stops` |
| A kaleidoscope | `symmetry`: `3`, `4`, `6`, or negative for a mirror |
| Another shape | a transform's `variations` (`fractique variations` lists them) or its `affine` numbers |
| The grain | `seed`, or `--seed N` on the command line |

Render at `--quality draft` while you experiment. A file with a mistake is refused with a message that names the
field, for example `layer 0, transform 1: color: must be between 0 and 1`.

## 6. Layers

A scene can hold up to 16 layers, each a flame of its own with its own camera, palette and tone, blended in order.
The `emerald-star` preset is two: a violet veil of rings under a green four-pointed star.

```sh
fractique preset emerald-star > star.json
```

To overlay two scenes, copy the entry in `layers` of one file into the `layers` list of the other. `blend` decides
how they meet (`add`, `screen`, `over`) and `opacity` how strong the layer is.

## 7. Variants of a scene

```sh
fractique mutate scene.json 1 --strength 0.2 -o variant1.json
fractique mutate scene.json 2 --strength 0.2 -o variant2.json
```

The number picks the variant; the same number always gives the same one. `--strength` goes from 0 (no change) to 1
(barely related); the default is 0.3. The camera is kept, so a strong mutation can move the flame partly out of
view: lower `camera.zoom` then.

## The app

`fractique-gui` is the same renderer with a window: the flame fills it and glass panels float over it.

| Where | What |
| --- | --- |
| Inspector (left; **Controls** in the toolbar shows and hides it) | Presets, tone, zoom, glass, scene |
| Title bar (desktop) | The version (opens the updates), the language, hide panels, minimize, maximize, close |
| **Randomize** | The next random scene |
| **Mutate** | A variant of the current scene |
| **Size** (inspector) | The picture's size: the window's own, one of 13 screens from phones to 4K, or any width and height from 16 to 16384 pixels. The picture is rendered in that shape and shown whole in the window |
| **Export** | A PNG at the picture's size, or at another size chosen in the same list there, and at the quality you choose; rendered in the background with a progress bar |
| **Scene: Open, Save** | Scenes as `.json`; **Open** also reads `.flame` files |
| Drag, wheel or pinch on the picture | Moves and magnifies the picture as it is, up to 12 times, without a new render; a double click shows all of it again |
| **H**, or the crossed pane in the title bar | Hides every panel; **H**, **Esc** or a click brings them back. On a phone a tap hides and shows them |
| **F11** | Full screen; **Esc** leaves it |
| The version next to the name in the title bar | Every released version with its notes; update, or install an older one |

On a phone the picture is saved into the app's own folder (`exports`), and the app has no updates window:
[releasing.md](releasing.md) says what updates itself and what does not. The language, the glass settings and the window (its place, size and screen) are kept between launches, in the
app's data folder: `%APPDATA%\Fractique` on Windows, `~/.local/share/fractique` on Linux,
`~/Library/Application Support/Fractique` on macOS. The window never reopens in full screen.

## 8. Flames from other programs

`.flame` files, the format of flam3, Apophysis and the programs built on them, convert to scenes:

```sh
fractique import pack.flame -o scene.json            # the first flame of the file
fractique import pack.flame --index 3 -o scene.json  # the fourth
fractique render scene.json -o imported.png
```

The transforms, their colors, the palette, the camera, the symmetry and the tone settings are read. What the file
uses and Fractique does not have is listed while it converts: variations outside the
[28 built in](scene-format.md#variation), xaos, and flam3's numbered palettes (a file that names one by number gets
the `aurora` palette). An imported flame shows the same structure as in its own program; brightness and color are
close, not identical, because the tone mapping differs.

## 9. Troubleshooting

| What you see | What to do |
| --- | --- |
| The image is black or nearly black | The flame is outside the view, or too faint. `fractique fit scene.json -o scene.json` moves every layer's camera to frame its flame; or lower `camera.zoom`, raise `tone.brightness`. |
| The image is grainy | Raise the quality, or `tone.smoothing` (up to 4). |
| Thin lines look jagged | Use `--quality high` or `--supersample 2`. |
| `the image is too large for this supersample` | Width × height × supersample² is limited to 2²⁸ cells. Lower the supersample. |
| The render uses too much memory | See the memory table in [performance.md](performance.md); lower the supersample. |
| `not a scene: unknown field ...` | A field name is misspelled, or it is in the wrong place. Compare with [scene-format.md](scene-format.md). |

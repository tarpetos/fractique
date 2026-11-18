# Using Fractique

Fractique is three programs over one renderer: the app, `fractique-gui`, the command line tool, `fractique`, and
the HTTP server, `fractique-server`. This guide covers the first two, with examples. For the fields of a scene
file see [scene-format.md](scene-format.md); for render times see [performance.md](performance.md); for the HTTP
server see [server.md](server.md); for AI agents, over stdio or HTTP, see [mcp.md](mcp.md).

## Contents

1. [Install](#1-install)
2. [The app](#2-the-app)
3. [Render a preset](#3-render-a-preset)
4. [Size and quality](#4-size-and-quality)
5. [Random scenes](#5-random-scenes)
6. [Wallpapers](#6-wallpapers)
7. [Edit a scene](#7-edit-a-scene)
8. [Layers](#8-layers)
9. [Variants of a scene](#9-variants-of-a-scene)
10. [Flames from other programs](#10-flames-from-other-programs)
11. [Files of the app](#11-files-of-the-app)
12. [Troubleshooting](#12-troubleshooting)
13. [The sample pictures](#13-the-sample-pictures)

---

## 1. Install

You need the repository and one command. It installs Rust when it is missing, builds both programs and runs the
tests.

| System | Command | Result |
| --- | --- | --- |
| Linux, macOS | `./setup.sh` | `target/release/fractique-gui`, `target/release/fractique` |
| Windows | `setup.cmd` | `target\release\fractique-gui.exe`, `target\release\fractique.exe` |

`--skip-tests` leaves the tests out. On Windows the script also installs the Microsoft C++ Build Tools (several GB)
when they are missing: Rust needs them to link programs. On Linux and macOS it needs a C compiler and tells you how
to install one.

A released version needs no build: [releasing.md](releasing.md) lists the files of a release, among them one
program file for Windows, an AppImage, a macOS app and an Android APK. The iPhone app is built on a Mac:
[ios.md](ios.md).

The examples below write `fractique` for the tool; use the path from the table, or copy the file to a folder on
your `PATH`.

## 2. The app

`fractique-gui` is the same renderer with a window: the picture fills it and glass panels float over it. It makes
flames and wallpapers; a switch in the controls goes from one to the other.

```sh
./target/release/fractique-gui
./target/release/fractique-gui scene.frcq      # opens that file; .json and .flame work too
```

### On a computer

| Where | What |
| --- | --- |
| Title bar | The app draws its own, as a glass capsule: the name, the version (opens the updates), the language, hide the panels, full screen, minimize, maximize, close. Dragging it moves the window, a double click maximizes it, the window's edges resize it |
| Inspector (left; **Controls** in the toolbar shows and hides it) | **Flames** or **Shapes**, then the preset or the wallpaper's choices, the colors, the size, the layers, the tone, the zoom, the glass, the scene |
| Toolbar (bottom) | The picture's name, its size, the time it took and the magnification; **Randomize**, **Mutate**, **Export**, **Controls** |
| **Presets** (inspector) | A list of the 29 built-in flames |
| **Recent** (inspector; in the styles on a phone) | The last 24 pictures that were on the screen, newest first: presets, random flames, opened files and wallpapers. A click brings one back. A flame that is being adjusted counts once. The list is kept between launches |
| **Randomize** | Flames: a new random flame in the chosen layer, made by the second random generator ([Random scenes](#5-random-scenes)). Wallpapers: a new wallpaper |
| **Mutate** | Flames: a variant of the chosen layer's flame. Wallpapers: another wallpaper of the same layout and material |
| **Layout**, **Material**, **Light** and **Dark** (inspector, wallpapers) | One of the 14 layouts and 6 materials of [Wallpapers](#6-wallpapers), or **Any**, which leaves the choice to each new wallpaper; and its light or dark version |
| **Size** (inspector) | The picture's size: **Auto** (the window's own, in its real pixels), one of 21 screens (computers up to 4K, Android phones and tablets, iPhones and iPads), or any width and height from 16 to 16384 pixels. The picture is rendered at that size, up to 4096 pixels a side on screen (1600 on a phone, 900 in a browser); a larger one is shown scaled down and exported in full. It is shown whole in the window. The App Store builds list computer screens and Apple devices, and a size typed in needs Fractique Pro there ([Free and Fractique Pro](#free-and-fractique-pro)) |
| **Layers** (inspector, flames) | The scene's flames, one over another, up to 16: **+** adds one from the presets or a random one, **×** removes, the arrows reorder; each has a blend (add, screen, over) and an opacity. Only the layer that changed is rendered again |
| **Tone**, **View** (inspector, flames) | Brightness, gamma and glow of the whole picture, and a zoom; **Reset** brings them back |
| **Graphics: GPU acceleration** (inspector; on at first) | A flame that is new on the screen (**Randomize**, **Mutate**, a preset, a moved view, another size) is first drawn by the graphics card, at the full size and in a fraction of a second; it grows finer over the next frames, and the processor's picture replaces it when it is rendered. The toolbar shows the time the first picture took. While a tone slider, the zoom, a layer's opacity or its blend moves, the graphics card draws the picture from the layers already rendered, so it answers at once; when the control rests the processor renders the picture and replaces the preview. The preview is close to the rendered picture, not equal to it: it has no dithering and its glow is blurred at a lower resolution. The graphics card's own picture has another grain than the processor's and can differ slightly from one device to the next. An export never uses either. Off, every change is rendered by the processor straight away, drafts first. The switch is not shown where the graphics card cannot draw a preview; in a browser, and where the graphics card has no compute shaders (older phones), it previews but leaves every flame to the processor |
| **Glass** (inspector) | Opacity and blur of the panels. **F2** opens the fine tuning of the glass: refraction, depth, bezel |
| Drag, wheel or pinch on the picture | Moves the view and zooms in and out, from 10,000 times smaller to 10,000,000,000 times larger. The picture follows at once and is drawn anew, sharp, when the view rests; the toolbar shows the magnification. **Export** and **Save** take the view as it is. A double click shows the whole scene again |
| **Reset view** (appears once the picture is moved or zoomed) | Shows the whole scene again, as a double click does |
| **Export** | A PNG at the picture's size, or at another size chosen in the same list there, in one of four qualities. **Instant** saves the picture that is on the screen, at once; the others render it anew in the background with a progress bar and **Cancel**. In the App Store builds a size with a lock needs Fractique Pro ([Free and Fractique Pro](#free-and-fractique-pro)) |
| **Scene: Open, Save** | **Save** writes a `.frcq` file, the app's own: the scene together with the tone, the view, the size and the flames or wallpapers choices, so it opens as it was saved, in a compact form that is not text. It can also write a plain `.json` scene, which is. **Open** reads `.frcq`, `.json` and `.flame`. A file given to the app when it starts (dropped on its icon, or "Open with") opens in it |
| **H**, or the crossed pane in the title bar | Hides every panel; **H**, **Esc** or a click brings them back |
| **F11**, or the four corners in the title bar | Full screen; **Esc** leaves it |
| Starting the app a second time | Brings the window that is already open to the front instead of opening another; a file given to the second start opens in the first |
| The version next to the name in the title bar | Every released version with its notes; update, or install an older one. It shows an arrow when a newer version is out |

The qualities of **Export**:

| Quality | Samples per pixel | Supersample | The same as |
| --- | --- | --- | --- |
| Instant | as on the screen: 32 on a computer, 18 on a phone | 1 | The picture as shown; rendered anew only when another size is chosen |
| Standard | 100 | 1 | `--quality standard` |
| High | 300 | 2 | `--quality high` |
| Ultra | 1000 | 2 | `--quality ultra` |

A wallpaper is drawn exactly, so its export is the same at every quality. The file name is offered from the
picture's name and size, for example `AuroraSpiral-1920x1080.png`.

### Opening a `.frcq` file from the file manager

A double click on a `.frcq` file opens it in Fractique, in the window that is already open if there is one:

- **Windows:** after the downloaded program has been started once. Move or rename the program and start it again,
  and the files follow it.
- **Linux:** after installing the `.deb`. The AppImage needs a tool that adds AppImages to the system
  (AppImageLauncher, `appimaged`); without one, choose Fractique under "Open with".
- **macOS:** the files are shown as Fractique's, but a double click does not open them yet: use **Open** in the app.

[releasing.md](releasing.md#files-of-the-app-frcq) says how each system is told.

### On a phone

On Android and iOS the picture fills the screen and the controls are a few round glass buttons.

| Where | What |
| --- | --- |
| **Flames**, **Shapes** (top) | What the app makes: flames, or wallpapers of flat lit shapes. Each keeps its picture while the other is shown |
| The row of colors under them | A hue for the shapes, a palette for the flames; the crossed circle gives the picture its own colors back |
| The large round button | A new picture: a random flame, or the next wallpaper |
| The four small buttons above it | **Keep** puts the picture into the collection, or takes it out (it holds 48). **Similar** makes another wallpaper of the same layout and material; for a flame it is **Mutate**, a variant of it. **Style** opens the styles. **Export** saves the picture. Their names stand beside them when the phone is on its side |
| The placard (bottom left) | A wallpaper's layout, material and number, or a flame's name; under it the size and the time |
| Swipe left or right (wallpapers) | The next or the previous wallpaper |
| Drag and pinch (flames) | Move the view and zoom, as on a computer; **Reset view** appears |
| Tap | Hides the buttons, and shows them again |
| Press and hold | Shows the picture as the lock screen of an iPhone, with the date, the time, the torch and the camera, for as long as the finger rests. On an iPhone the date and the time are the real ones |
| **Style** | A sheet that slides up, with small live pictures to choose from. First the **collection**, the pictures kept. For wallpapers: every layout, every material, a row of colors (any, or one of eight) and **Light** or **Dark**. For flames: every preset, and a random one. Its handle drags it taller or away |
| **Fine-tune** (at the end of the styles) | The controls of the computer's inspector, over the whole screen: language, size, layers, tone, glass, GPU acceleration. The cross closes them |
| **Export** | The size and the quality, then **Save to Photos** or **Share**. On an iPhone the picture goes to Photos or to the share sheet. On Android it goes to the gallery, into `Pictures/Fractique`, and **Share** then offers it to other apps. Android 9 and older ask for storage access when **Save** is pressed, before the picture is made; once allowed, the export goes on by itself |

A phone has no **Scene: Open, Save** and no updates window: [releasing.md](releasing.md) says what updates itself
and what does not. In a browser the app has no export, no scene files and no updates window.

### Free and Fractique Pro

Every feature of Fractique is free on every platform. The builds from the App Store (iPhone, iPad, Mac) have one
purchase, **Fractique Pro**: paid once, kept for good, restored on another device of the same Apple Account with
**Restore purchases**.

| Where | Free | Needs Fractique Pro |
| --- | --- | --- |
| Windows, Linux, Android, the web, and every build made without the `store` feature | Everything | Nothing: these builds hold no purchase |
| The App Store builds | Everything on the screen at any size; **Auto**; a listed size of fewer than 3 686 400 pixels (2560 × 1440), such as 1920 × 1080 or 1290 × 2796, as the picture's size and as an export | An export (**Save**, **Share**, at every quality, **Instant** too) of 3 686 400 pixels or more, whatever its shape; choosing such a size for the picture; a size typed into **Custom** |

The limit counts pixels, width times height: 1440 × 2560 and 1920 × 1920 are at it, 2560 × 1439 is under it. The
window's own size (**Auto**) is never refused on the screen; exporting it follows the same count, so a window of
2560 × 1600 pixels exports with Fractique Pro.

A size that needs the purchase has a small lock in the list. Choosing it, pressing **Save** or **Share** with it,
or pressing **Apply** under **Custom** opens the **Fractique Pro** sheet: the price as the App Store gives it,
**Buy**, **Restore purchases** and **Cancel**. After the purchase the locks are gone and what was asked for goes
on: the export starts, or the size is set. The purchase is remembered on the device, so the app is unlocked
without a network.

The sizes the App Store builds list, besides HD, Full HD, QHD, Ultrawide and 4K. They are the native pixels of
the screens, read from Apple's Tech Specs pages (`support.apple.com/en-us/<number>`) in November 2026; iPhones
and iPads upright, as Apple gives them:

| Name in the list | Pixels | Screens | Apple's page |
| --- | --- | --- | --- |
| iPhone SE | 750 × 1334 | SE (2nd and 3rd generation) | 111882, 111866 |
| iPhone 11 | 828 × 1792 | 11 | 111865 |
| iPhone mini | 1080 × 2340 | 12 mini, 13 mini | 111877, 111873 |
| iPhone 11 Pro | 1125 × 2436 | 11 Pro | 111879 |
| iPhone 11 Pro Max | 1242 × 2688 | 11 Pro Max | 111878 |
| iPhone 12–14, 17e | 1170 × 2532 | 12, 12 Pro, 13, 13 Pro, 14, 16e, 17e | 111876, 111872, 111850, 122208, 126470 |
| iPhone 14 Plus | 1284 × 2778 | 12 Pro Max, 13 Pro Max, 14 Plus | 111874, 111870, 111854 |
| iPhone 15, 16 | 1179 × 2556 | 14 Pro, 15, 15 Pro, 16 | 111849, 111831, 111829, 121029 |
| iPhone 15–16 Plus | 1290 × 2796 | 14 Pro Max, 15 Plus, 15 Pro Max, 16 Plus | 111846, 111830, 111828, 121030 |
| iPhone 17, 18 Pro | 1206 × 2622 | 16 Pro, 17, 17 Pro, 18 Pro | 121031, 125089, 125090, 148590 |
| iPhone Air | 1260 × 2736 | Air | 125092 |
| iPhone 18 Pro Max | 1320 × 2868 | 16 Pro Max, 17 Pro Max, 18 Pro Max | 121032, 125091, 148591 |
| iPhone Duo 5.4″ | 1398 × 2034 | Duo, the outer screen | 148588 |
| iPhone Duo 7.6″ | 1878 × 2670 | Duo, the inner screen | 148588 |
| iPad mini | 1488 × 2266 | mini (6th generation, A17 Pro) | 111886, 121456 |
| iPad mini (A20 Pro) | 1512 × 2316 | mini (A20 Pro) | 149482 |
| iPad 10.2″ | 1620 × 2160 | iPad (7th to 9th generation) | 111911, 118451, 111898 |
| iPad, iPad Air 11″ | 1640 × 2360 | iPad (10th generation, A16), Air (4th, 5th generation), Air 11-inch | 111840, 122240, 111887, 126471 |
| iPad Pro 10.5″ | 1668 × 2224 | Pro 10.5-inch, Air (3rd generation) | 111927, 111939 |
| iPad Pro 11″ 2018–22 | 1668 × 2388 | Pro 11-inch (1st to 4th generation) | 111974, 118452, 111897, 111842 |
| iPad Pro 11″ | 1668 × 2420 | Pro 11-inch (M4, M5) | 119892, 125406 |
| iPad Air 13″ | 2048 × 2732 | Air 13-inch, Pro 12.9-inch (3rd to 6th generation) | 126472, 111841 |
| iPad Pro 13″ | 2064 × 2752 | Pro 13-inch (M4, M5) | 119891, 125407 |
| MacBook Neo | 2408 × 1506 | MacBook Neo | 126322 |
| MacBook Air 13″ M1 | 2560 × 1600 | MacBook Air 13-inch to 2020, MacBook Pro 13-inch | 111883, 111869 |
| MacBook Air 13″ | 2560 × 1664 | MacBook Air 13-inch from 2022 | 111867, 126320 |
| MacBook Air 15″ | 2880 × 1864 | MacBook Air 15-inch | 111346, 126321 |
| MacBook Pro 14″ | 3024 × 1964 | MacBook Pro 14-inch | 111902, 148300 |
| MacBook 14″ OLED | 3024 × 1986 | MacBook Pro 14-inch PROLED XDR | 148301 |
| MacBook Pro 16″ | 3456 × 2234 | MacBook Pro 16-inch | 111901, 126319 |
| MacBook 16″ OLED | 3472 × 2266 | MacBook Pro 16-inch PROLED XDR | 148302 |
| iMac 24″ | 4480 × 2520 | iMac 24-inch | 111895, 148299 |
| Studio Display, 5K | 5120 × 2880 | Studio Display, Studio Display XDR, iMac 27-inch 5K | 111890, 126324, 126323, 111913 |
| Pro Display XDR | 6016 × 3384 | Pro Display XDR | 111892 |

The 10.5-inch iPads no longer get the current iPadOS and are listed because they are still in use. Apple Watch
faces are not in the list: a watch crops any photo to its face. The Android sizes are left out of the App Store
builds; every other build keeps its list of 21 screens.

### What the app keeps

The whole session is kept between launches: the scene that was open with its layers, the view, the tone, the size,
flames or wallpapers and their choices, the collection, the language, the glass, GPU acceleration and the window
(its place, size and screen). It is written when it changes, into the app's data folder: `%APPDATA%\Fractique\data` on Windows,
`~/.local/share/fractique` on Linux, `~/Library/Application Support/Fractique` on macOS, and the app's own folder
on a phone. The window never reopens in full screen.

## 3. Render a preset

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

## 4. Size and quality

```sh
fractique render --preset emerald-star --size 3840x2160 --quality high -o star-4k.png
fractique render --preset emerald-star --size 1080x2400 --quality mobile -o star-phone.png
```

`--size` takes any `WIDTHxHEIGHT` up to 16384 a side; without it the image is 1920x1080. The picture is framed by
its shorter side: a wider or taller image shows more of the surroundings, it does not stretch or crop the middle.

| `--quality` | Samples per pixel | Supersample | Use |
| --- | --- | --- | --- |
| `draft` | 8 | 1 | A quick look |
| `mobile` | 30 | 1 | Phones |
| `standard` (default) | 100 | 1 | Everyday |
| `high` | 300 | 2 | Wallpapers |
| `ultra` | 1000 | 2 | Prints |

`--samples N` and `--supersample N` replace the level's numbers. More samples means less grain; supersample 2
renders at twice the size and averages, which sharpens thin lines and costs four times the memory.

## 5. Random scenes

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

There are two random generators, and the same number is a different scene in each. `--generator 1`, the one
used when the option is left out, draws from the first 27 variations. `--generator 2` draws from all 49, gives the
parametric ones (`blob`, `pdj`, `ngon`, `juliascope` and the rest) random parameters, and skips more of the
pictures that come out as specks, a dim haze or an even fog:

```sh
fractique render --random 42 --generator 2 -o r42.png
fractique randomize 42 --generator 2 -o scene.json
```

The app's **Randomize** uses generator 2. A seed keeps its scene in each generator from one version of Fractique
to the next: a better randomizer comes as a new generator with a number of its own.

## 6. Wallpapers

A wallpaper is a scene made of flat sheets with lit edges and soft shadows instead of a flame. Every number is one:

```sh
fractique render --wallpaper 7 --size 1290x2796 -o wallpaper.png
fractique render --wallpaper 7 --dark --size 1290x2796 -o wallpaper-dark.png
```

The seed picks the layout, the material, the colors and the light. `--dark` gives the dark version of the same
shapes. To hold one of the choices and let the seed vary the rest:

```sh
fractique render --wallpaper 7 --layout waves --material glass --size 1290x2796 -o waves.png
```

| | Names |
| --- | --- |
| `--layout` | Round: `bands`, `discs`, `waves`, `ribbons`, `burst`, `arcs`, `tiles`, `bars`, `fan`. Straight: `slats`, `planes`, `steps`, `frames`, `lines` |
| `--material` | `relief` (one color, bright edges), `paper` (a color ramp, deep shadows), `glass` (translucent sheets that mix where they overlap), `glow` (dim sheets, wide bright edges), `steel` (graphite, navy, forest, oxblood or bronze, thin sharp edges), `carbon` (near black or near white with one accent color) |

The shapes are laid out for the size given, so render at the size of the screen the wallpaper is for: the same
seed gives other shapes for a screen of another proportion. To keep one as a scene file, and change it by hand:

```sh
fractique wallpaper 7 --layout waves --size 1290x2796 -o wallpaper.json
fractique render wallpaper.json --size 1290x2796 -o wallpaper.png
```

`fractique wallpaper` lays the shapes out for 1290x2796 when no `--size` is given. The quality, the samples and
`--seed` change nothing here: sheets are drawn exactly. The app can also ask for a wallpaper in one color; the
command line leaves the colors to the seed.

## 7. Edit a scene

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

Every command that takes a scene file also takes a `.frcq` file saved by the app, and uses the scene as it was on
the screen; `fractique convert` turns one into JSON to edit ([Files of the app](#11-files-of-the-app)):

```sh
fractique render AuroraSpiral.frcq --size 3840x2160 --quality high -o aurora.png
```

## 8. Layers

A scene can hold up to 16 layers, each a flame of its own with its own camera, palette and tone, blended in order.
The `emerald-star` preset is two: a violet veil of rings under a green four-pointed star.

```sh
fractique preset emerald-star > star.json
```

To overlay two scenes, copy the entry in `layers` of one file into the `layers` list of the other. `blend` decides
how they meet (`add`, `screen`, `over`) and `opacity` how strong the layer is. In the app the same is done in
**Layers**, without a file.

## 9. Variants of a scene

```sh
fractique mutate scene.json 1 --strength 0.2 -o variant1.json
fractique mutate scene.json 2 --strength 0.2 -o variant2.json
```

The number picks the variant; the same number always gives the same one. `--strength` goes from 0 (no change) to 1
(barely related); the default is 0.3. The camera is kept, so a strong mutation can move the flame partly out of
view: lower `camera.zoom` then, or run `fractique fit`. Only flames change: the sheets of a wallpaper stay as they
are.

## 10. Flames from other programs

`.flame` files, the format of flam3, Apophysis and the programs built on them, convert to scenes:

```sh
fractique import pack.flame -o scene.json            # the first flame of the file
fractique import pack.flame --index 3 -o scene.json  # the fourth
fractique render scene.json -o imported.png
```

The transforms, their colors, the palette, the camera, the symmetry and the tone settings are read. What the file
uses and Fractique does not have is listed while it converts: variations outside the
[49 built in](scene-format.md#variation), xaos, and flam3's numbered palettes (a file that names one by number gets
the `aurora` palette). An imported flame shows the same structure as in its own program; brightness and color are
close, not identical, because the tone mapping differs. The app opens a `.flame` file too, and takes its first
flame.

## 11. Files of the app

The app saves `.frcq` files: the scene with the app's session, packed, not text
([scene-format.md](scene-format.md#the-frcq-file)). Every command that reads a scene reads them too, and takes the scene as it
was on the screen:

```sh
fractique render mine.frcq --size 3840x2160 --quality high -o mine.png
fractique mutate mine.frcq 1 -o variant.json
fractique fit mine.frcq -o framed.json
```

`fractique convert` goes between the two forms; what the file holds decides the direction:

```sh
fractique convert mine.frcq -o scene.json                 # the scene the file shows, as JSON
fractique convert mine.frcq --document -o document.json   # everything in the file: the session and the scene
fractique convert scene.json -o scene.frcq                # a scene, packed for the app
fractique convert document.json -o mine.frcq              # a document, packed again as it is
```

Without `-o` the JSON is printed. A scene packed this way opens in the app as a plain scene: whole view, neutral
tone. A `.frcq` file is about a seventh of the same scene as JSON.

## 12. Troubleshooting

| What you see | What to do |
| --- | --- |
| The image is black or nearly black | The flame is outside the view, or too faint. `fractique fit scene.json -o scene.json` moves every layer's camera to frame its flame; or lower `camera.zoom`, raise `tone.brightness`. |
| The image is grainy | Raise the quality, or `tone.smoothing` (up to 4). |
| Thin lines look jagged | Use `--quality high` or `--supersample 2`. |
| `the image is too large for this supersample` | Width × height × supersample² is limited to 2²⁸ cells. Lower the supersample. |
| The render uses too much memory | See the memory table in [performance.md](performance.md); lower the supersample. |
| `not a scene: unknown field ...` | A field name is misspelled, or it is in the wrong place. Compare with [scene-format.md](scene-format.md). |
| `the file is of version ...` | The `.frcq` file was saved by a newer version of the app. Update, or save a `.json` scene there. |
| A deep view in the app stays empty | Where the flame has nothing, there is nothing to draw: the gaps between its threads are wider than the threads. **Reset view** goes back. |
| The Android app closes at start | It leaves `fractique-log.txt` in its own folder (`Android/data/app.fractique/files`): send that file with the report. |

## 13. The sample pictures

The pictures in `docs/samples/` and in the README are rendered by these commands, from the repository's root:

```sh
fractique render --preset aurora-spiral --size 960x540 --quality high -o docs/samples/aurora-spiral.png
fractique render --preset emerald-star --size 640x360 --quality high -o docs/samples/emerald-star.png
fractique render --preset ember-vortex --size 640x360 --quality high -o docs/samples/ember-vortex.png
fractique render --preset golden-bloom --size 640x360 --quality high -o docs/samples/golden-bloom.png
fractique render --preset solar-mandala --size 640x360 --quality high -o docs/samples/solar-mandala.png
fractique render --wallpaper 3 --layout waves --material glass --size 360x780 -o docs/samples/wallpaper-waves-light.png
fractique render --wallpaper 5 --layout burst --material glow --size 360x780 -o docs/samples/wallpaper-burst-light.png
fractique render --wallpaper 7 --layout tiles --material relief --size 360x780 -o docs/samples/wallpaper-tiles-light.png
fractique render --wallpaper 9 --layout fan --material glass --size 360x780 -o docs/samples/wallpaper-fan-light.png
fractique render --wallpaper 2 --layout discs --material paper --dark --size 360x780 -o docs/samples/wallpaper-discs-dark.png
fractique render --wallpaper 6 --layout arcs --material carbon --dark --size 360x780 -o docs/samples/wallpaper-arcs-dark.png
fractique render --wallpaper 14 --layout lines --material paper --dark --size 360x780 -o docs/samples/wallpaper-lines-dark.png
fractique render --wallpaper 8 --layout bars --material paper --dark --size 360x780 -o docs/samples/wallpaper-bars-dark.png
```

The files were then made smaller without changing a pixel (`optipng -o2`).

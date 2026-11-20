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
flames, fractals of other kinds and wallpapers; three tabs in the controls go from one to the other.

```sh
./target/release/fractique-gui
./target/release/fractique-gui scene.frcq      # opens that file; .json and .flame work too
```

### On a computer

| Where | What |
| --- | --- |
| Title bar | The app draws its own, as a glass capsule: the name, the version (opens the updates), the language, hide the panels, full screen, minimize, maximize, close. Dragging it moves the window, a double click maximizes it, the window's edges resize it |
| Motion | Every sheet, list and panel fades and rises in and out in under a fifth of a second, and what it covers gives way to it; a click acts at once, also while something still moves. While a window is resized the picture keeps its scale and the panels follow the edge; the picture is drawn anew when the size has stood for 0.18 s. Full screen, maximizing and restoring change the window in one step: the picture and the panels then go on from where they stood on the screen and glide to their new places as one, and the panels hidden by **H** fade out and in together while the picture stays as it is. From the moment a new picture is asked for there are two stages: one still, blurred picture (the one on the screen, or a first small picture of the new scene), then the finished one, which fades in as the blur lifts; a picture that is there at once is not blurred. Where Windows is set to show no animations, nothing is animated |
| The limits of a view | The wheel, a pinch and a drag stop at limits, without a jump and without drawing anything for the step that hits one. A wallpaper is not made smaller than the window at all and is moved only as far as it still covers the window; it is shown at most 8 times larger. A flame and a fractal of any kind are shown no smaller than a fifth of their own framing (0.2×) and cannot be dragged out of sight: a figure smaller than the window stays inside it whole, and of a figure larger than the window at most a tenth of the window shows what is beside it. The scale is one number: the one on the toolbar's line is the one the **Scale** slider of the controls shows and sets, and the wheel moves that slider. The slider spans what the view of the picture on the screen can take, from its smallest scale to its largest, logarithmically: its knob stands at an end exactly when the view is at that limit, and when another picture has another range the ends and the knob glide there. Both places write the scale the same way: two decimals below 10, one below 100, none above, and `9.5e9×` from a million on. **Reset** under it and **Reset view** both go back to the whole picture. Inwards a view of a fractal drawn pixel by pixel goes as far as the numbers of its kind reach (10 000 000 000× over the kind's own magnification). A flame goes as far as its densest place is still a picture, between 10× and 300× depending on the flame, and an attractor to 10× (Buddhabrot), 30× (Clifford, De Jong, Hopalong), 50× (Gumowski-Mira) or 100× (Lorenz): deeper, the picture would be specks or would not be the same picture twice; a view that would take too long to draw is the picture the app has, shown at most 8 times larger. Beside a picture that does not fill the window there is the window's dark background. A saved file or a **Recent** picture with a view outside these limits opens at the limit |
| Inspector (left; **Controls** in the toolbar shows and hides it) | **Flames**, **Fractals** or **Shapes**, then the preset, the fractal's type or the wallpaper's choices, the colors, the size, the layers, the fractal's parameters, the tone, the zoom, the glass, the scene |
| **Fractals** (inspector) | **Type** lists the 22 kinds of fractal, from the Mandelbrot set to the Menger sponge; the nine that only the processor draws and that take it long carry an amber dot. Choosing one shows its first preset; **Presets** lists the presets of that kind. **Parameters** has a slider for every parameter of the chosen layer's fractal: the iterations, the color density and shift, the dark edge and the fade of an escape-time fractal, the constant of a Julia set, the four numbers of an attractor ([scene-format.md](scene-format.md#fractal)). The colors, the size, the layers, the tone, the view, **Recent**, **Export** and **Scene** work as for flames. Each tab keeps its picture while another is shown |
| **Parameters**, the fields (inspector, fractals) | A kind with a parameter that is text has a field for it under the sliders: the axiom and the rules of an L-system, the sequence of letters of a Lyapunov fractal. The picture follows what is typed as soon as the kind can read it; until then the text is shown in red and the picture stays |
| Toolbar (bottom) | The picture's name, its size, the time it took and the magnification. The time is one number a picture: from the click, the last step of the wheel or the edge that asked for it to the finished picture on the screen, with whatever the app waited in between; while the picture is on its way it counts in tenths of a second and ends on that number. A view that is only magnified keeps the time of the picture it is magnified from; the **Back** arrow, **Randomize**, **Mutate**, **Export**, **Controls**. A random picture is named by its word alone (**Random**, the fractal's type, **Wallpaper**) and a mutation by its place in its family (**Random · mutation 2**); the seed is not shown, and the file name offered by **Export** and **Save** keeps it (`Random354`, `Mandelbrot42`) |
| **Presets** (inspector) | A list of the 29 built-in flames; in **Fractals**, the presets of the chosen type |
| **Recent** (inspector; in the styles on a phone) | The last 20 pictures of the tab that were on the screen, newest first, with the one on the screen marked: presets, random pictures, mutations and opened files. Each of the three tabs keeps its own list. On a computer every row has a small picture, the name and, at the right, the time the picture was made (`14:32`, or `18.11 14:32` for an earlier day); on a phone they are tiles. A click brings one back and puts it first. Every **Mutate** is a picture of its own: the pictures mutated from one another are a family, named after the first and numbered as they were made (**Random**, **Random · mutation 1**, **Random · mutation 2**), so the fifth row down is five mutations back. A flame that is being adjusted (tone, colors, layers, view) stays one picture. The list is kept between launches |
| **Back** (the arrow in the toolbar; **Alt+←** or the mouse's back button; the round button with an arrow at the left of the large one on a phone) | One step back through the recent pictures of the tab: the picture one older than the one on the screen. The list keeps its order and the marked row moves down, so another press goes one further back; **Alt+→** or the mouse's forward button goes forward again. A new picture made from there (**Randomize**, **Mutate**, a preset, a file) goes first as usual. The arrow always looks the same: pressed when the tab has no older picture, it changes nothing, and a phone answers with one short tick |
| **Randomize** | Flames: a new random flame in the chosen layer, made by the second random generator ([Random scenes](#5-random-scenes)). Fractals: a new random picture of the chosen type in the chosen layer. Wallpapers: a new wallpaper |
| **Mutate** | Flames: a variant of the chosen layer's flame. Fractals: a nearby view of the Mandelbrot set, a Julia set of a nearby constant, an attractor of nearby numbers. Wallpapers: another wallpaper of the same layout and material |
| **Layout**, **Material**, **Light** and **Dark** (inspector, wallpapers) | One of the 14 layouts and 6 materials of [Wallpapers](#6-wallpapers), or **Any**, which leaves the choice to each new wallpaper; and its light or dark version |
| **Size** (inspector) | The picture's size: **Auto** (the window's own, in its real pixels), one of 59 screens, or any width and height from 16 to 16384 pixels. The list is in groups, chosen by the tabs over it: **Screens** (computers up to 4K), **iPhone**, **iPad**, **Mac**, **Watch** and **Android**. The picture is rendered at that size, up to 4096 pixels a side on screen (1600 on a phone, 900 in a browser); a larger one is shown scaled down and exported in full. It is shown whole in the window. The App Store builds list computer screens and Apple devices, and a size typed in needs Fractique Pro there ([Free and Fractique Pro](#free-and-fractique-pro)) |
| **Layers** (inspector, flames and fractals) | The scene's flames and fractals, one over another, up to 16: **+** adds one from the presets or a random one (in **Fractals** the list holds the presets of the chosen type and the flames, so a flame can lie over a Mandelbrot set), **×** removes, the arrows reorder; each has a blend (add, screen, over) and an opacity. Only the layer that changed is rendered again |
| **Tone**, **View** (inspector, flames) | Brightness, gamma and glow of the whole picture, and a zoom; **Reset** brings them back |
| **Graphics: GPU acceleration** (inspector; on at first; only on some devices) | The section exists where the graphics card draws a flame clearly faster than the processor: a graphics card of its own is used as it is, and elsewhere the app tries both once, after its first picture, and remembers the answer for that card. On a device without such a card (many phones, a computer without a graphics card of its own, a browser) there is no switch and the processor renders everything. On, the graphics card draws every flame, the Clifford, De Jong and Lorenz attractors, the escape-time and root-finding kinds (Mandelbrot, Julia, Burning Ship, Tricorn, Multibrot, Phoenix, Magnet, Newton, Nova) and the wallpapers itself, at the full size and within a second or two, growing finer over a few frames; the processor does not render for the screen, so its cores stay free. Tone, opacity and blend follow at once. Under the switch are the card's name and a note: the picture on the screen and an **Instant** export come from the card, have another grain than the processor's render and can differ slightly from one device to the next, while an export at Standard, High or Ultra is always rendered exactly. A deep view of a flame is the card's too. A wallpaper is there in a few hundredths of a second, and its picture is the processor's but for a step of one in a few of its numbers. A scene with any other kind of fractal in a layer or with a flame beside a kind drawn pixel by pixel, a deep view of a scene with an attractor beside its flames, and whatever else the card does not draw are rendered by the processor as before. Off, every change is rendered by the processor straight away, drafts first |
| **Glass** (inspector) | Opacity and blur of the panels. **F2** opens the fine tuning of the glass: refraction, depth, bezel |
| Drag, wheel or pinch on the picture | Moves the view and zooms in and out, from 10,000 times smaller to 10,000,000,000 times larger (for a fractal, counted together with the zoom its scene already has). The picture follows at once and is drawn anew, sharp, when the view rests; the toolbar shows the magnification. A view whose picture would take more than 10 seconds on this device is not drawn anew: the picture stays as it is, magnified, and the status line says **magnified picture**. That happens to deep views of a flame without GPU acceleration at large sizes, to views deeper than the graphics card reaches, and to the heavy kinds at large sizes; a smaller picture size brings the sharp view back. A wallpaper is always magnified this way. **Export** and **Save** take the view as it is. A double click shows the whole scene again |
| **Reset view** (appears once the picture is moved or zoomed) | Shows the whole scene again, as a double click does |
| **Export** | A PNG at the picture's size, or at another size chosen in the same list there, in one of four qualities. **Instant** saves the picture that is on the screen, at once: with GPU acceleration on that is the graphics card's picture, which is not exact to the pixel; the others render it anew on the processor, exactly, in the background with a progress bar and **Cancel**. The bar covers the rendering and the writing of the file, says which of the two is going on, and is full when the file is on the disk; **Cancel** works during both and leaves no half-written file. The PNG is compressed fast: a file is written about 17 times sooner than with the usual compression (0.4 s instead of 7.1 s for 7680 × 4320) and is about 40 % larger, with the same pixels. A quality the renderer does not take at the chosen size is dim and cannot be chosen, and a line under the qualities names those that can: High and Ultra render at twice the size a side, which is held to 2²⁸ cells, so 15360 × 8640 is exported as Instant or Standard. When an export, a file to open or a file to save fails, the sheet or the toolbar's line says so in the app's language; the error itself goes to the log. A view that is a magnified picture on the screen is exported as that picture at every quality, and the sheet says so. In the App Store builds a size with a lock needs Fractique Pro ([Free and Fractique Pro](#free-and-fractique-pro)) |
| **Scene: Open, Save** | **Save** writes a `.frcq` file, the app's own: the scene together with the tone, the view, the size and the flames or wallpapers choices, so it opens as it was saved, in a compact form that is not text. It can also write a plain `.json` scene, which is. **Open** reads `.frcq`, `.json` and `.flame`. A file given to the app when it starts (dropped on its icon, or "Open with") opens in it |
| The round button with a cross | On every sheet, list and panel that opens over the picture: the export, the sizes, the presets, **Recent**, the fractal types, the layouts and materials, the languages, the updates, **Fractique Pro**. It stands in the pane's top right corner, as far from the top and the right edge as the title is from the left one, on the title's line. It closes what it sits on and changes nothing. A list closes also by a click anywhere beside it, which does nothing else (it does not press what lies under it, nor move the picture), by **Esc**, and by a click on the row that is already chosen. An export that is running keeps its **Cancel**, which stops the export |
| **H**, or the crossed pane in the title bar | Hides every panel; **H**, **Esc** or a click brings them back |
| **F11**, or the four corners in the title bar | Full screen; **Esc** leaves it |
| Starting the app a second time | Brings the window that is already open to the front instead of opening another; a file given to the second start opens in the first |
| The version next to the name in the title bar | Every released version with its notes; update, or install an older one. It shows an arrow when a newer version is out |

The qualities of **Export**:

| Quality | Samples per pixel | Supersample | The same as |
| --- | --- | --- | --- |
| Instant | as on the screen: 32 on a computer, 18 on a phone | 1 | The picture as shown, the graphics card's when GPU acceleration made it; rendered anew by the processor only when another size is chosen |
| Standard | 100 | 1 | `--quality standard` |
| High | 300 | 2 | `--quality high` |
| Ultra | 1000 | 2 | `--quality ultra` |

A wallpaper is drawn exactly, so its export is the same at Standard, High and Ultra; an Instant export of a
wallpaper the graphics card drew differs from them by one step of 255 in about one number of forty. The file name is offered from the
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
| **Flames**, **Fractals**, **Shapes** (top) | What the app makes: flames, fractals of other kinds, or wallpapers of flat lit shapes. Each keeps its picture while another is shown. On a low screen, a phone on its side, the switch and the row of colors stand closer to the top and to one another |
| The row of colors under them | A hue for the shapes, a palette for the flames and the fractals; the crossed circle gives the picture its own colors back |
| The large round button, with a die | A new picture: a random flame, a random fractal of the chosen type, or the next wallpaper |
| The three small buttons around it | They stand around the large button, under the thumb, all equally far from its centre. Above it, with its right edge on the large button's, the wand is **Similar**, which makes another wallpaper of the same layout and material; for a flame it is **Mutate**, a variant of it. On the diagonal the heart is **Keep**: it puts the picture into the collection, or takes it out (it holds 48). At its left, standing on the same line as the large button, the arrow is **Back**: it shows the picture before this one, as on a computer; when there is none the picture stays and the phone gives one short tick |
| The two small buttons above **Mutate** | **Style**, the sliders, opens the styles and, over it, **Export**, the arrow down, saves the picture. No button of this screen has its name written beside it |
| The buttons on a low screen | The right edge always stands between the status bar or the camera cutout (and the row of colors, where that reaches it) and the lower edge of the screen: the gaps between the buttons close first, then the buttons become smaller. Where the buttons leave the picture's name too little room at the bottom, it stands above them |
| Closing | **Fine-tune**, the export and every list (sizes, presets, **Recent**, fractal types, layouts, materials, languages) have the round button with a cross in their top right corner. The styles have none: they close by a tap on their handle, a drag of it downwards or a tap on the picture above. A list stands in the middle of the screen, as wide as the other sheets; a long one grows until a narrow edge of the picture is left above and below it, and scrolls. A tap beside a list closes it and does nothing else; so does a tap on the row already chosen. The system's **Back** of an Android phone closes what is open, one thing at a press: a list, then a sheet, then the styles or the controls |
| The placard (bottom left) | A wallpaper's layout and material, or a flame's name; under the name of a mutation a small dim tag says which (**MUTATION 2**); under that the size and the time |
| Swipe left or right (wallpapers) | The next or the previous wallpaper |
| Drag and pinch (flames) | Move the view and zoom, as on a computer; **Reset view** appears |
| Pinch (wallpapers) | Magnifies the wallpaper as a picture; a drag stays a swipe to the next one, and **Reset view** shows it whole again |
| Tap | Hides the buttons, and shows them again |
| Press and hold | Shows the picture as the lock screen of an iPhone, with the date, the time, the torch and the camera, for as long as the finger rests. On an iPhone the date and the time are the real ones |
| **Style** | A sheet that slides up, with small live pictures to choose from. First the **collection**, the pictures kept. For wallpapers: every layout, every material, a row of colors (any, or one of eight) and **Light** or **Dark**. For flames: every preset, and a random one. For fractals: every type but the nine that take the processor long, each with a small picture, then the presets of the chosen type and a random one. Its handle drags it taller or away |
| **Fine-tune** (at the end of the styles) | The controls of the computer's inspector, over the whole screen: language, size, layers, a fractal's parameters, tone, glass, GPU acceleration. A list opened from them (the language, the type, the presets, **Recent**, the size, a layout or a material) stands alone over the picture: the controls hide while it is open and come back when a row is chosen or the list is closed. The list of types leaves out the nine that take a phone's processor long, as the styles do; a picture of such a type that is on the screen stays, and **Recent** still brings one back. The cross closes the controls |
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
**Buy**, **Restore purchases** and the round button with a cross that closes it. After the purchase the locks are gone and what was asked for goes
on: the export starts, or the size is set. The purchase is remembered on the device, so the app is unlocked
without a network.

The Apple sizes of the list, in every build. They are the native pixels of
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

The 10.5-inch iPads no longer get the current iPadOS and are listed because they are still in use. The Apple
Watch sizes are the screens of the 40 to 46 mm cases and of the Ultra (324 × 394 to 422 × 514): a picture saved at
one goes to Photos and becomes a face in the Watch app. Those of the 41 and 45 mm cases (Series 7 to 9) are the
widely published figures, not read from a current Apple page. The Android group is left out of the App Store
builds.

### What the app keeps

The whole session is kept between launches: the scene that was open with its layers, the view, the tone, the size,
the tab and its choices, the recent pictures, the collection, the language, the glass, GPU acceleration and the
window (its place, size and screen). It is written when it changes, into the app's data folder:
`%APPDATA%\Fractique\data` on Windows, `~/.local/share/fractique` on Linux,
`~/Library/Application Support/Fractique` on macOS, and the app's own folder on a phone. The window never reopens
in full screen.

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
used when the option is left out, draws from the first 27 variations. `--generator 2` draws from 48 of the 49 (all but `rectangles`, which tiles a flame into squares), gives the
parametric ones (`blob`, `pdj`, `ngon`, `juliascope` and the rest) random parameters, and skips more of the
pictures that come out as specks, a dim haze or an even fog:

```sh
fractique render --random 42 --generator 2 -o r42.png
fractique randomize 42 --generator 2 -o scene.json
```

The app's **Randomize** uses generator 2. A seed keeps its scene in each generator from one version of Fractique
to the next: a better randomizer comes as a new generator with a number of its own.

### Fractals of other kinds

`fractique fractals` lists the kinds of fractal besides flames, with the parameters of each, their defaults and
ranges, and their presets:

```sh
fractique fractals
fractique render --preset mandelbrot-seahorse-valley -o seahorses.png
fractique preset julia-dragon > dragon.json
```

`--fractal KIND` makes the random scene of a seed a fractal of that kind:

```sh
fractique render --random 7 --fractal mandelbrot -o m7.png
fractique render --random 7 --fractal julia --size 1080x2400 --quality mobile -o j7.png
fractique randomize 7 --fractal clifford -o attractor.json
```

| Kind | The random scene of a seed |
| --- | --- |
| `mandelbrot` | A view along the edge of the set, magnified from a few times to about a million: a walk of one to nine steps from the whole set, each into one of the busiest parts of the view before it. Views that are more than 28 % set, flat or noise are skipped; the colors are spread over what the view shows and the iterations set for its depth |
| `julia` | The Julia set of a point on the edge of the Mandelbrot set, whole or a part of it, judged the same way |
| `burning_ship` | A view found as for `mandelbrot`, most often along the mast on the real axis, where the small ships are. Views that are stripes, noise or dark all over are skipped as well |
| `tricorn` | A view along the edge of the set, found and judged as for `burning_ship` |
| `multibrot` | A power of 3, 4, 5 or 6 and a view along the edge of its set |
| `phoenix` | A feedback, then most often the Julia form of a constant from the edge of the map, or a view of the map itself |
| `magnet` | The first or the second type and a view along the edge of its set, where the bubbles are |
| `newton` | zⁿ − 1 with n from 3 to 8 or one of the named polynomials, often with a relaxation that curls the borders, and a view of the roots or of a border between their basins. Views with too little or too much border, or too dark, are skipped |
| `nova` | A degree and a relaxation, then a view of the map of the constants or the Julia form of a constant from its edge |
| `clifford` | An attractor of four random numbers. Those that settle on a few points or a curve, a dot in a dust or an even fog are skipped, and the camera frames the rest |
| `de_jong` | An attractor of four random numbers between −3.2 and 3.2, colored by one of its three colorings, judged and framed the same way |
| `gumowski_mira` | `mu` between −0.95 and 0.6, `a` between 0.003 and 0.02, `b` between 0.01 and 0.2, with any of the colorings. Those that fall onto a few points are skipped, as are the even blobs of a map that barely loses area |
| `hopalong` | `a` and `c` between −12 and 12, `b` between 0.05 and 6, orbits of 30 000 to 600 000 steps, with any of the colorings; the camera frames the middle 92 % of the points, since the lace thins out far from its middle |
| `lorenz` | `sigma` between 4 and 20, `rho` between 25 and 120, `beta` between 0.5 and 4, seen from any side, followed for 60 to 1800 units of time. Systems that come to rest, single closed curves and views along the attractor's edge are skipped |
| `buddhabrot` | 300 to 9000 iterations, with the shortest orbits drawn 5 steps long or a 4th to a 40th of the iterations, any of the three colorings; a third of the pictures are turned towards the plane of the starting points, and one in seven is an Anti-Buddhabrot of 20 to 200 iterations |
| `apollonian` | A gasket of random first circles, as discs, rings or a glow: whole for half of the seeds, a view into its gaps for the other half. Views that one circle fills are skipped |
| `kleinian` | A group close to the edge of those that have a limit set to draw, found by walking the traces toward that edge; Grandma's recipe, or for about four seeds in ten Maskit's, whose limit set is a band |
| `l_system` | One of 26 curves and plants, often with its angle moved a little, iterated as far as its lines stay apart. Drawings that are a line or a blot are skipped |
| `lyapunov` | A random sequence of 2 to 10 letters and a view where both rates are under 4. Views that are flat, all chaos or noise are skipped |
| `mandelbulb` | A bulb of power 8, or of a whole power from 3 to 12, seen whole from a random side or from near through a longer lens, under a light at a random angle. A view is probed before it is kept: one that fills too little of the frame, is dark, flat, of one color or noise is skipped, up to 12 times, and the colors are spread over what the view shows |
| `mandelbox` | A box of a scale from 1.6 to 3.2 or from −2.6 to −1.5, mostly seen from inside its outer layers and sometimes whole, judged the same way, up to 16 times |
| `menger_sponge` | The sponge, whole or from near, in a little over half of the seeds folded by a twist, a bend or a scale other than 3; judged the same way, with rough surfaces allowed |

The random fractal of a seed may change from one version of Fractique to the next, as a wallpaper's does; a scene
kept in a file does not. `--generator` is for flames and is refused with `--fractal`.

`fractique mutate` and the app's **Mutate** change the geometric kinds this way: an `apollonian` gets a nearby view
and nearby first circles, a `kleinian` nearby traces (kept only while the limit set stays much the same) and a
camera that frames it again, an `l_system` a nearby angle, a `lyapunov` a nearby view and starting value; each
moves its colors along the palette.

A kind can have parameters that are text; `fractique fractals` lists them after the numbers. In a scene file they
are in `text`, beside `params` ([scene-format.md](scene-format.md#geometric-kinds)): the axiom and the rules of an
L-system, the sequence of a Lyapunov fractal. To draw an L-system of your own, take a preset and change its texts:

```sh
fractique preset l_system-koch-snowflake > curve.json    # then edit "axiom", "rules", "angle", "iterations"
fractique render curve.json -o curve.png
```

A fractal is drawn at the samples and the supersample of the quality level, in its own way
([scene-format.md](scene-format.md#fractal)): `--quality high` draws the edges of a Mandelbrot set from 36 samples
a pixel, `standard` from 9, `draft` from one.

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

A scene can hold up to 16 layers, each a flame or a fractal of another kind with its own camera, palette and tone,
blended in order.
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
view: lower `camera.zoom` then, or run `fractique fit`. A fractal of another kind changes its own way: a
`mandelbrot` moves, turns and zooms its view a little and shifts its colors, a `julia` gets a nearby constant, a
`clifford` nearby numbers and a camera that frames it again. `fractique fit` leaves a fractal's view as it is. The
sheets of a wallpaper stay as they are. The other attractors change as `clifford` does, a `lorenz` is also turned
a little, and a `buddhabrot` gets other limits and a small turn; none of them changes its coloring.
A solid is seen from another side, a little nearer or farther, under a light that moved and in shifted colors: a
`mandelbulb` may get the next power, a `mandelbox` a nearby scale and inner radius, a `menger_sponge` another
twist and bend.

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
fractique render --preset mandelbrot-spiral --size 640x360 --quality high -o docs/samples/fractal-mandelbrot.png
fractique render --preset julia-spirals --size 640x360 --quality high -o docs/samples/fractal-julia.png
fractique render --preset clifford-wings --size 640x360 --quality high -o docs/samples/fractal-clifford.png
fractique render --preset burning_ship-ship --size 640x360 --quality high -o docs/samples/fractal-burning_ship.png
fractique render --preset tricorn-island --size 640x360 --quality high -o docs/samples/fractal-tricorn.png
fractique render --preset multibrot-spirals --size 640x360 --quality high -o docs/samples/fractal-multibrot.png
fractique render --preset phoenix-galaxy --size 640x360 --quality high -o docs/samples/fractal-phoenix.png
fractique render --preset magnet-pearls --size 640x360 --quality high -o docs/samples/fractal-magnet.png
fractique render --preset newton-seven --size 640x360 --quality high -o docs/samples/fractal-newton.png
fractique render --preset nova-star --size 640x360 --quality high -o docs/samples/fractal-nova.png
fractique render --preset buddhabrot-nebula --size 640x360 --quality high -o docs/samples/fractal-buddhabrot.png
fractique render --preset de_jong-classic --size 640x360 --quality high -o docs/samples/fractal-de_jong.png
fractique render --preset lorenz-butterfly --size 640x360 --quality high -o docs/samples/fractal-lorenz.png
fractique render --preset gumowski_mira-feathers --size 640x360 --quality high -o docs/samples/fractal-gumowski_mira.png
fractique render --preset hopalong-web --size 640x360 --quality high -o docs/samples/fractal-hopalong.png
fractique render --preset apollonian-threefold --size 640x360 --quality high -o docs/samples/fractal-apollonian.png
fractique render --preset kleinian-cusp --size 640x360 --quality high -o docs/samples/fractal-kleinian.png
fractique render --preset l_system-plant --size 640x360 --quality high -o docs/samples/fractal-l_system.png
fractique render --preset lyapunov-feathers --size 640x360 --quality high -o docs/samples/fractal-lyapunov.png
fractique render --preset mandelbulb-crown --size 640x360 --quality high -o docs/samples/fractal-mandelbulb.png
fractique render --preset mandelbox-station --size 640x360 --quality high -o docs/samples/fractal-mandelbox.png
fractique render --preset menger_sponge-classic --size 640x360 --quality high -o docs/samples/fractal-menger-sponge.png
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

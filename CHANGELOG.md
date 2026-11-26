# Changelog

## 1.0.5

* **App, phones:** deleting one of two layers no longer leaves the picture without its buttons; the layer card opens for a picture of one layer too.
* **Fractals:** Mandelbox, Mandelbulb and Menger sponge have an isometric camera and can take their colors from the light; presets in that style.
* **Fractals:** a new kind, `flow`: nine smooth attractors in three dimensions (Thomas, Halvorsen, Chen, Aizawa and others) drawn as lines.
* **App:** a fractal only the processor draws is always finished, at any zoom, and its time is told once, for the whole picture.
* **App, phones:** a slider takes a finger anywhere on its track and a little above or below it, follows one that goes more sideways than up or down, and a tap puts its knob there.
* **App, phones:** the layers, then the colors, in two rows under the switch: three layers and five colors in sight, the rest scrolled by a finger or by the arrows at their ends.
* **Gallery:** a hundred isometric fractals, `docs/gallery/isometric/`.
* **Colors:** nine dark palettes of one hue after rezonanca.art, each with its own background: amethyst, cobalt, garnet, abyss, lagoon, frost, rosewood, bronze, forest.
* **App, phones:** Back brings a picture back with the colors and the layer chosen for it.
* **App:** the colors stand in even rows from edge to edge: on a phone the 18 in two rows of nine, not eight, eight and two.
* **App:** your own colors: a gradient of one to five for a layer of a flame or a fractal, picked on a square and a bar of hues, and of one to five hues for a wallpaper; **Your colors** under the palettes, a rainbow circle in the phone's row.

## 1.0.4

* **App Store only:** a release for iPhone and iPad, with no release on GitHub.
* **App Store builds, iPhone and iPad:** the free saves of the day are kept in the Keychain, so deleting the app and installing it again no longer gives three new ones.
* **App Store builds:** the App Store lists the 15 languages of the app, and Settings offers them as the app's language.

## 1.0.3

* **App, phones:** a row of layers under the colors chooses the layer the colors, presets and buttons change; its sliders open the layer's blend, opacity and order.
* **App, phones:** Fine-tune holds only what the screen and the styles do not: size, a fractal's parameters, tone, zoom.
* **App:** in a picture of several layers a preset or a fractal type replaces only the chosen layer, as Randomize does.
* **App:** Open offers `.flame` files only in Flames; a wallpaper is saved under its layout and material.
* **App:** an export of several layers at Ultra and 3840 × 2160 stays on the graphics card instead of going to the processor.
* **App:** a gear at the right of the toolbar, and on a phone above the export button, opens the settings: language, glass, GPU acceleration, version.
* **App:** with GPU acceleration on, exports at Standard, High and Ultra are drawn by the graphics card, Ultra at 3840 × 2160 in 12 s on an RTX 4070 instead of 109 s; **Exact pictures** keeps them on the processor.
* **App:** an Instant export larger than the picture on the screen is drawn by the graphics card too: 3840 × 2160 in 0.3 s on an RTX 4070 instead of 5 s.

## 1.0.2

* **App Store builds:** hiding the panels to see the picture alone needs Fractique Pro; the eye opens its sheet.
* **App:** after the graphics card gives up a deep view of a fractal drawn pixel by pixel, zooming back out draws the view on the card again, with its drafts while it moves.
* **App, Linux:** the window has round corners, as on Windows and macOS.
* **App, Linux:** `--x11` opens the app through XWayland, where its window keeps its place, has round corners and comes forward on a second start.
* **App, Linux:** a window closed in full screen opens maximized next time, not at its smaller size.
* **App:** a drag of the top panel no longer moves a window in full screen; on GNOME it left the picture smaller, with black around.
* **App:** a second start from a terminal says that Fractique is already running.
* **App, Linux:** an AppImage adds itself to the list of apps and shows its icon on the file; the window names its app, so the dock shows its icon; the `.deb` files Fractique under Graphics.

## 1.0.1

* **App:** the nine fractals only the processor draws (Mandelbox, Mandelbulb, Buddhabrot and the like) show a picture of half the side first, four times sooner, then the whole one.
* **App:** on a computer too, an export that the machine's memory cannot hold is refused instead of ending the app.
* **MCP:** a skill for agents, `skills/fractique/SKILL.md`: when to use the tools and in what order.
* **App:** after an update "Restart now" starts the new version; before, the app only closed.
* **App:** an updated file named after its version takes the new version's name (`Fractique_1.0.1_x86_64.AppImage`).
* **App:** in the updates window the install button stands at the top right; the progress of a download and the messages stand above the list of versions.

## 1.0.0

* **A phone no longer starts an export it has no memory for:** a picture takes about 80 bytes of memory for each
  pixel while it is made, so 15360 × 8640 needs some 11 GB and the app was closed by the system halfway. On a phone
  the export sheet now asks how much memory is left: a quality that would not fit is dim, and where none fits the
  sheet says so and offers no button.

* **Small sheets on a phone keep their glass:** when the sheet of Fractique Pro changed height, or the export sheet
  shrank to its progress bar, the buttons came back around it and its dark glass went, leaving the texts on the
  bare picture. The buttons now stay away under every sheet, and the glass has room for twice as many panes.

* **Fractique Pro unlocks by itself:** bought on an iPhone, an iPad or a Mac, it is there on the buyer's other
  devices when the app is opened, with no press on **Restore purchases**, which stays for the case the App Store
  is slow to tell.

* **App Store builds:** every listed size is free; without Fractique Pro three pictures a day are saved or shared. Pro lifts the limit and keeps the **Custom** sizes.
* **App:** the answer of "Check for updates" follows a change of language; a version's date is no longer cut off.

## 0.1.0

The first release: the app for Windows, Linux, macOS, Android, iOS and the web, the command line tool, the MCP
server and the HTTP server.

* **A Docker image with every release:** `docker run -p 8080:8080 ghcr.io/tarpetos/fractique-server` runs the HTTP
  server with the web app, on x86-64 and on ARM64. The `Release` workflow builds it, asks the server in it for an
  answer and pushes it under the version and under `latest`.

* **One download for every Mac:** the macOS app and the three other programs are universal, with the code for
  Apple Silicon and for Intel in one file.

* **A video of the app, made by a script:** `tools/video/make_video.py` films the app at 3840 × 2160 and 60 frames a
  second on a display nobody sees, and writes the video, its subtitles in the 15 languages, the thumbnail and the
  texts for YouTube. The README and its translations show its preview, which links to the video.

* **Sliders under a finger move only by their knob:** in the controls of a phone a finger that scrolled the list
  moved a slider it happened to cross, and the knob jumped to the finger. A slider now moves only when the finger
  lands on its knob and goes clearly sideways, and the knob goes as far as the finger does, from where it stood.
  A finger anywhere else scrolls the list, which now follows it exactly. A mouse works as before.

* **Tiling window managers on Linux:** under i3, sway, Hyprland and the like the top panel has no buttons to
  minimize and to maximize, which did nothing there; full screen stays. The window's place and size are no longer
  kept from one run to the next there: a window made to float stood on its old tile, at the tile's size, instead
  of in the middle of the screen. `FRACTIQUE_TILING=on` or `off` decides it by hand.

* **The blur leaves after the window is dragged, on Linux:** a window dragged by its top panel, to a border of the
  screen or out of a maximized or full-screen state, kept its picture blurred until the next click: the system
  never told the app that the button was released. The title and the close button of the top panel now stand
  equally far from its ends.

* **Layered pictures in the gallery:** the twenty weakest flames and the twenty weakest fractals of `docs/gallery`
  gave their numbers to scenes of two or three layers, each layer in a palette of its own: a bright figure over a
  wide veil, two figures of different symmetry, an attractor's silk over a Mandelbrot, Julia or Hopalong ground.
  The colors are in the layers of the scene, so the app and `fractique render` draw the same picture.

* **A click on a picture of the gallery downloads its file:** a picture in `docs/gallery` led to GitHub's page of
  the `.frcq` file, where the file had to be downloaded by hand. Now the picture is the download itself (the file's
  raw address, which a browser saves), and the name under it leads to the file in the repository.

### Changed

* **A magnified view shows its faint parts, and draws them clean (a breaking change for deep views):** a view that
  holds less than an eighth of a flame (a deep zoom, or a view moved off the figure) kept the average brightness
  it had where it began to hold that little, and the average is carried by whatever is brightest in the frame.
  So a faint part beside a bright corner stayed dark until the corner left the view, and then lit up at once: a
  flame of three layers, moved off the figure, was nearly black at 0.48× and 0.52× and showed its arcs at 0.57×.
  And nearly all the points of such a view landed on the bright part, so what little showed of the faint one was
  grain. Now the faint end of such a view is raised on top of the brightness it had: by nothing where the view
  begins to hold less than an eighth, by more as less of the flame is in it, with full brightness left where it
  was, so nothing burns and nothing that showed gets fainter. And the points are spread: a part of the view
  that holds far more than the rest is visited less and each of its points counts more, which leaves the picture
  what it was and gives the faint parts the points, at the same number of points and the same time. The arcs of
  that flame show from about 0.42×, smooth at the standard quality. A view that holds an eighth of the flame or
  more is drawn exactly as before, and so is every preset at its own framing. Deep views of flames are drawn
  differently from this version on: the same scene and view give other pixels than before, the same on every
  machine as ever. The golden hash of the one deep view in `tests/determinism.rs` is replaced for it; every
  other golden hash is as it was.

### Added

* **The AppImage runs the programs packed in it:** an AppImage starts one program, so the command line tool and
  the two servers inside it could only be reached by unpacking it. Its first word now chooses one: `cli`, `mcp` or
  `server`, with that program's own arguments after it (`Fractique_<version>_x86_64.AppImage server --bind
  0.0.0.0:8080`). Without such a word it opens the app as before, and the word means nothing outside an AppImage.

* **A dot in a layer's colors before its name:** every row of **Layers** starts with a round dot like those of
  the row of colors, in the palette the layer is drawn with: the one chosen for it, or the layer's own. It
  follows a choice, **Randomize**, **Mutate**, a preset and a move of the layer at once, so a scene of several
  colors shows which layer has which.

### Changed

* **The switch of GPU acceleration is on every device whose card is able:** the **Graphics** section was there
  only where a trial at the first start showed the card clearly faster than the processor, so a phone whose
  trial went wrong had no switch at all. It is now there wherever the card can do the work. A warning sign
  stands beside the switch, and a press on it opens the line that a weak chip can be slower with it than
  without. The trial only decides how the switch stands at first, and until it has told the processor renders;
  a switch you have set stays as you set it.

### Fixed

* **A phone's graphics chip is fast again:** since the card's work is cut into units of 1.5 ms so the window
  keeps its pace, a phone lost most of the card's speed: the way to the chip and back takes 1 to 5 ms there, a
  quarter of a millisecond on a computer's card, so the units were nearly all way. An Adreno 740 took 1.3 s for
  the trial's picture, which its processor renders in 0.5 s, and the app left the card out. Where that way is
  long, a unit is now thirty times as long as it, two frames at the most, while a hand is on the window (a
  finger or a button held, the wheel, a pinch, a key); while none is, a hundred times, a tenth of a second at
  the most, and the units follow one another without a pause: a phone's chip slows its clock in pauses. A tap
  that is over is no hand, so the picture it asked for is drawn at the full pace. Measured on that phone: the
  trial's picture in 0.34 s in units with pauses, 0.17 s without, against 0.52 s on its processor, so 3.1 times
  as fast instead of 1.5. A verdict against a card kept by an earlier version is measured again. A graphics
  card of its own works as before.

* **A card that won its trial keeps its pictures:** on a device whose card is tried against the processor (a
  phone, a laptop without a graphics card of its own) every flame was still handed to the processor after the
  card's second unit of work, so GPU acceleration was on and did nothing. The time the card would need was
  reckoned from those first two units, which are the steps that plot nothing and the way to the card: 0.03 s
  of work for 0.4 % of a picture reads as 8 s, against half of the processor's 6. Now it is reckoned from the
  units that are mostly work, and against all of the processor's time where the card works without a pause. On
  a laptop with a Radeon Vega 3 six presets at 1280 × 720 took 3.1 to 7.1 s with the switch on, as with it off,
  and take 0.36 to 0.58 s. A card that is slower than the processor on a picture still gives it up, within a
  few tenths of a second.

* **A phone on OpenGL no longer closes ten seconds after a first start:** on a phone older than Android 11, and
  on any card reached through OpenGL, the app tried the chaos game on the graphics card when it first rested.
  The driver compiled the shaders on a thread of their own, the window waited for the one OpenGL context, and
  after a second the app ended. Such a card is no longer tried: the processor renders, as it did there after a
  trial that the card lost. A phone that had its verdict kept from an earlier version never showed it.

* **The blurred picture follows a fast zoom all the way:** while a view is drawn, the picture on the screen
  stands for it under the blur, magnified as the view is, but never more than 8 times. A wheel turned on past
  that left it standing while the number went on: with a picture finished at 2.2× after the first turn, the
  picture stopped at 17.6× and the number went to 300×, and the finished picture then jumped to a place the
  blur had never shown. The first zoom after **Randomize**, **Mutate**, a preset or an opened file did not move
  the picture at all until its first new picture came. The picture the screen holds is now drawn through the
  whole transform from its own view to the view of the frame, place and magnification, to the ceiling of the
  kind and to its floor: under the wheel, a pinch, a drag and the **Zoom** slider, for flames, attractors, the
  fractals drawn pixel by pixel and the solids alike. A picture that arrives on the way takes the screen where
  the one before stood. Magnified far past its detail the picture is a smooth smear, which is what the blur
  shows; it is still one quad, and a frame takes no longer. Measured on an RTX 4070 with a flame of three
  layers at 3840 × 2160: 300× over the 2.2× picture is shown 135.9 times larger, where it was 8.

* **One territory for a picture, the same at every zoom:** how far a view of a flame or a fractal could be moved
  depended on its scale. Below 1.0× the figure could be moved about the window, less and less on the way in and
  not at all at 1.0×, so a view moved to the right at 0.5× was pulled back to the middle while it was made
  larger; above 1.0× the room grew again, and the edge a view could reach shrank with the scale, which pushed
  the point under the cursor aside. A picture now has one territory: everything its smallest view (0.2×) could
  show, nine framings across and nine down with the figure in the middle. The view goes anywhere inside it at
  every scale, so a place seen at 1.0× is seen at 100× too. Making a view larger never moves it; making it
  smaller moves it only at the territory's edge, as far as it must. A figure can now be moved out of the
  window; **Reset view** or a double click brings it back. The range of the zoom of each kind and the limits of
  a wallpaper, whose territory is the wallpaper, are as they were. A file or a **Recent** picture with a view
  outside the territory opens at its edge.

* **A palette is the chosen layer's, not the whole scene's:** the app kept one chosen palette and dressed every
  layer in it, so a scene of several flames or fractals could only be of one color. The row of colors now acts on
  the layer chosen in **Layers**, shows that layer's choice and follows the selection; the crossed circle gives
  that layer its own colors back. A choice stays with its layer when layers are added, removed or reordered, in
  each of the tabs, between launches and in a `.frcq` file, which holds the choices as `colors`, a name or
  `null` a layer. A session or a file written before, with its single `palette`, opens as the picture it was:
  every layer in that palette. A scene of one layer behaves as it did, except that **Flames** and **Fractals**
  each keep their own choice while the other is shown.

* **A gallery of flames, fractals and wallpapers, each with its file:** [docs/gallery](docs/gallery/README.md) shows a
  hundred flames chosen from several thousand random scenes and their mutations, a hundred fractals of other kinds
  (all 22 kinds) and a hundred wallpapers (all 14 layouts and 6 materials), in a folder each (`flames`, `fractals`, `wallpapers`) whose page shows its pictures. Each has
  a small picture and beside it a `.frcq` file that the app opens and `fractique render` draws again at any size
  and quality. `tools/gallery/make_gallery.py` makes the pictures and the pages from the files, and a test loads
  every one of them, so they keep loading. The captions under the
  pictures of the README and of its 14 translations are now the names the app shows (Emerald Star, Mandelbrot:
  Spiral) instead of the presets' identifiers.

* **A whole parameter of a fractal holds a whole number:** a scene with a fraction in a parameter that is whole,
  `"iterations": 300.5`, loaded as it was written and was drawn as 300, so the file, the scene saved again and
  the picture did not agree. The fraction is now dropped when the scene is read, the way the renderer read it:
  cut off, or to the nearer of the two for `form`, `type`, `recipe` and `anti`. Such a file still loads and
  is drawn as before, with two exceptions that no whole number could keep: the steps of an escape-time or a
  root-finding kind at a zoom above 1, where the fraction was multiplied in before it was cut (such a view gets up
  to six steps fewer), and a `nova` of form 0 with a fraction in `degree`, whose starting
  point took the fraction while its steps did not. There the graphics card of the app and the processor drew two
  pictures; now they draw one.

* **The command line tool and the two servers update themselves on command:** the app updated itself and left
  `fractique`, `fractique-mcp` and `fractique-server` beside it as old as they were. Now each has `update`: it
  asks GitHub for the newest release, downloads this program's file for this system, checks it against the
  release's signature with the key built into the program and puts it in the program's place; `update --check`
  only reports (exit code 0: this is the newest, 10: a newer one is out, 1: could not find out) and
  `update --version 0.2.0` installs a named version, an older one too. `--version` typed into a terminal adds a
  line when a newer release is out, within 1.5 seconds or not at all; in a script it prints the version alone
  and asks no one, and `FRACTIQUE_NO_UPDATE_CHECK=1` switches the look off. Nothing else contacts the network:
  not the MCP server while it serves, not the HTTP server while it runs. A program of the `.deb`, of the
  AppImage or of the macOS app, one in a folder it cannot write to and one built from the source are not
  replaced and say what to do instead. A release now holds `tools.json` beside `latest.json`, and the three
  programs are signed like the app; releases before this one have no `tools.json`, and `update` says so.
  `fractique-mcp` still serves when started without an argument.
  [docs/releasing.md](docs/releasing.md#how-the-other-programs-update) has the rules.

* **Icons for the command line tool, the MCP server and the HTTP server:** on Windows only the app had an icon.
  `fractique.exe`, `fractique-mcp.exe` and `fractique-server.exe` now carry the app's picture in a color of their own
  (green, violet, amber) with a sign beside the letter (a prompt, a star, a rack), and their properties say
  Fractique, what the program is and its version. The MCP server, over stdio and over HTTP, tells a client its icon and the
  address of the repository in the answer to `initialize`.

* **A reference of the HTTP API in the browser, at `/docs`:** `fractique-server` had no reference a person could
  open. `/docs` now shows every endpoint with its fields, their types, limits and defaults, the answers and the
  errors, and sends a request to the server from the page: a render comes back as the picture.
  `/openapi.json` is the same as an OpenAPI 3.1 description with the limits of the instance in it. The page
  (Scalar API Reference, MIT) is inside the program, so nothing is loaded from another address and it works
  without a network. `--docs off` or `FRACTIQUE_DOCS=off` takes both away; `GET /limits` says `"docs"`.

* **`fractique-mcp`, the MCP server as a program of its own:** it is the server that `fractique mcp` runs and
  nothing else, so a client's configuration is one path with no argument. It is in every release, inside the
  packages and as a download of its own; `fractique mcp` stays.

* **The HTTP server and the command line tool in every release:** a release held the app, and the command line
  tool only for Windows and inside the Linux and macOS packages; the HTTP server had to be built from the source.
  Now `fractique-server` is in the `.deb`, the AppImage and the macOS app beside `fractique`, and both are
  downloads of their own for Windows, Linux and macOS, for a machine that wants no app.
  [docs/mcp.md](docs/mcp.md#where-the-program-is) says where each is after installing.

* **Clicking through again: every press shows a quick preview of its scene:** since the blur with its two stages,
  **Randomize**, **Mutate**, a preset or a step back pressed while a picture was still on its way showed the old
  picture under the blur until a finished one landed, and each press began that wait again: of 10 presses 200 ms
  apart 6 showed nothing of their scene on the graphics card (flames, 3840 × 2160, RTX 4070) and 9 on the
  processor. Now every press shows a small picture of its own scene under the blur: the blur comes over the old
  picture first (0.08 s), then the small picture takes its place once and stands still, and the finished one
  fades in as the blur lifts. Under a blur that already stands the next press's small picture is on the screen
  17 to 103 ms after the press where the graphics card draws flames (51 ms at the median), 19 to 36 ms for an
  attractor, 51 to 88 ms on the processor, and about 0.1 s after the first press, which is the blur's rise. It is the processor's draft of 480 pixels a side, also where the card
  draws the picture, so a canvas of 15360 × 8640 shows it as fast as the window's own size does. What a press
  that was overtaken had on its way is dropped with the next press and never reaches the screen after it; the
  blur stays while the presses go on and lifts over the finished picture of the last. The finished picture, its
  time and the frames of the interface are as they were (16.9 ms a frame at the median during the presses).

* **The export's note no longer calls a picture of the chosen size "larger":** with 15360 × 8640 chosen as the
  picture's size, the export sheet said an export of 15360 × 8640 was larger than the picture on the screen. The
  line now says that at this size the picture is drawn again for the file, and it is not shown where the
  graphics card draws the file in tiles, which takes about a second.

* **An Instant export from the picture on the screen, smaller or in tiles:** **Instant** saved the picture on the
  screen only at that picture's very size; at any other it rendered the scene again on the processor. Now an
  export of the picture's shape and smaller is the screen's picture made smaller, every pixel the mean of those
  it covers, at once (1920 × 1080 from a window of 3840 × 2160: 0.11 s for the Burning Ship), and with a canvas
  larger than the window that is the finer picture. An export larger than the picture on the screen, of a
  fractal the graphics card draws pixel by pixel, is drawn on the card in tiles of at most 2048 pixels a side,
  each with what its glow and its edges need around it, copied back and written to the file a row of tiles at a
  time: the picture is never whole on the card. 15360 × 8640 of the Burning Ship is saved in 1.4 s where the
  processor took 39 s, with the card's memory as it is for the screen; across the 11 borders of its 40 tiles
  neighbouring columns and rows differ as those beside them do. A flame, an attractor and a wallpaper of a
  larger size are rendered by the processor as before. The sheet says before the press when Instant is not
  instant: the size has another shape than the picture on the screen, or is larger, so the picture is drawn
  again.

* **A chosen size is the picture, and the screen shows it with the window's pixels:** with 15360 × 8640 chosen the
  toolbar said "4096 × 2304": the picture on the screen was drawn at 4096 pixels a side, held on the graphics
  card at that size, and named by it. Now a chosen size is a canvas. The line under the picture names the size
  chosen. What is drawn for the screen has the pixels the picture has in the window (3840 × 2160 in a maximized
  window of that many pixels, whatever is chosen), so the graphics card holds the same for **Auto**, 4K, 8K and
  16K: measured on an RTX 4070 as the app's dedicated memory, 601 MB for the Burning Ship and for a wallpaper at
  every one of the four, 859 to 891 MB for a flame with its margin. A canvas larger than the window is drawn as
  that picture scaled down, not as a smaller picture: a fractal drawn pixel by pixel takes, where its pixels
  differ, a grid of samples as many times finer a side as the canvas is larger (up to 8 × 8 a pixel, the 2 × 2
  of a pixel of a 16K canvas times its 4 × 4 pixels), and a flame, an attractor and a deep view plot the samples
  of the canvas (16 times those of the window for 16K) into the window's histogram, with the kernels of the
  density estimation the canvas would have, for up to 8 seconds. In that window the Burning Ship takes 0.04 s at
  4K, 0.05 s at 8K and 0.27 s at 16K, and the first flame 0.32, 0.62 and 1.71 s. A window that changes size
  gets the picture again, with its new pixels, once the size has stood. A wallpaper and the processor's
  pictures are drawn plain. An export has the size chosen, as before.

* **8K and 16K among the screens:** the list of sizes has 7680 × 4320 and 15360 × 8640 under **Screens**, after
  4K. An export has the size chosen.

* **A size of one's own can be typed again:** emptying the width of **Own** to type another number put the old
  size back in both fields at once. The fields are now filled only while nothing is typed in them.

* **No bars beside a picture of a chosen size:** with the size fixed (3840 × 2160, say) and the window of another
  shape (a maximized window of 1920 × 1032 points is wider than 16 to 9), the picture, fitted whole, left a strip
  of the window's flat dark at each side with a hard edge against the picture, and the title bar and the panel
  showed them as dark blocks. At rest the strips were black; while the picture was moved, the fractal was drawn
  there. Now the strips hold the scene itself wherever the app has it, the same at rest and while it moves. A
  fractal the card draws pixel by pixel is drawn with what the window shows beside it (89 pixels a side of
  3840 × 2160 in that window, 4.7 % more pixels; the picture in the middle is the one drawn alone), and so is
  its draft while it moves; a window whose shape changes gets the picture again once it has stood. For a flame,
  an attractor and a deep view the margin around the view is that content, and it now comes in a fade instead
  of one frame. A wallpaper has nothing beside it, so it fills the window: up to 1.25 times larger than fitted,
  cut above and below (by 4.4 % of its height in that window) or at its sides, and the whole view can be moved
  over what is cut; what is seen is then slightly less than what is exported. Where none of that is to be had
  (the processor's pictures, a card without a margin, the time before the margin is drawn, a strip wider than
  half the picture) the picture is continued turned over at its edge, sharp where they meet, blurred from there
  and fading into the dark, so no edge and no bar stands in the window. Measured in a window of that shape with
  the interface hidden, before and after: the share of the strips that is the flat background 100 % and 0 for
  the Burning Ship and for a wallpaper (4.5 % of the window and 0), 100 % and 0.4 to 2.7 % for a flame without
  a margin, which is the flame's own dark; the step across the picture's edge, as the mean difference of
  neighbouring columns, 8.3 and 0.02 (Burning Ship at rest), 94 and 1.3 (after a drag), 463 and 0.6 (wallpaper),
  6 to 37 and 0.8 to 3.2 (flame without a margin), each as much as the columns around it differ. An export, a
  small picture and Recent are the picture of the chosen size, as before (an Instant export of a fractal
  differs from the one before in 2.5 % of its numbers by one step of 255: the glow is worked out over the
  wider picture); the detail line names that size. Not marked on the screen yet: where the exported picture
  ends.

* **A wallpaper no longer stays blurred after the start:** where the wallpapers' shader was compiled within
  the frame that asked for it, the render that waited for it never began again, and the first wallpaper stood
  under the blur until something else was done (2 of 25 starts in test runs on an RTX 4070; none of 12 now).

* **A flame, an attractor and a deep view have a margin around the view:** dragged sideways or made smaller, the
  picture ended where its view had ended, and two thirds of the window could be its blurred continuation until
  the new picture came. The card now draws the finished picture a second time, behind the one on the screen,
  with a margin of the scene around the view: a view that moves or shows more inside the margin is sharp at
  once, with no blur and nothing beside it; the picture of the new view is drawn behind and takes its place with
  a margin of its own. The time to a finished picture is what it was: the view alone is drawn first, as before,
  and the picture with the margin comes 0.05 to 0.4 s later on an RTX 4070 (1 s later for a deep view at
  3840 × 2160). The view in it is the picture it was, pixel for pixel. The margin is a half, three eighths, a
  quarter or an eighth of the view on every side: the widest the card draws within 3 s and holds in 512 MB (a
  quarter at 3840 × 2160, a half at 2560 × 1440 and below on that card), none on a card that is slow or short of
  memory. Measured in the same steps with and without it, at 3840 × 2160: a drag and a step out of a flame, a
  deep view, Clifford and Lorenz showed 84 to 160 and 46 to 62 blurred frames with 18 % and 36 % of the view
  beside the picture, and none with the margin. Past the margin, and for a view that magnifies the picture,
  nothing changes. Exports, small pictures and Recent hold the view alone
  ([performance.md](docs/performance.md#a-margin-around-the-view)).

* **A flame and an attractor are magnified only as far as they are a picture:** a flame at 1373× was clouds of
  separate dots, and the wheel went on to 10 000 000 000×. Measured in the app at 3840 × 2160 on an RTX 4070, each
  view drawn twice with another seed: a deep view of a flame is the same picture twice while it holds about
  1/500 of the flame's light; with less it is smooth threads that are other threads the next time, and with far
  less, dots. A flame now goes as deep as its densest view holds 1/512 of its light, between 10× and 300×, worked
  out once a picture beside the window's thread; an attractor to 10× (Buddhabrot), 30× (Clifford, De Jong,
  Hopalong), 50× (Gumowski-Mira) or 100× (Lorenz), where its grain takes over. The Zoom slider ends there, and
  a saved or a recent view beyond it opens at the ceiling. Fractals drawn pixel by pixel, the solids, Kleinian
  groups, L-systems and wallpapers keep their limits. Known and not changed here: in a median place of a flame the
  picture is already of the second kind from about 15×, and a heavy flame (Emerald Star) takes the card 13 s at
  100×, over the 10 seconds a view may take ([performance.md](docs/performance.md#where-a-view-stops-being-a-picture)).

* **A dragged edge is followed twice as often:** measured on the screen (a capture at 60 frames a second of a
  real drag of the window's right edge, 900 pixels a second and 2700). Every size of the window was painted
  twice, the second time for a redraw of the same size, and with the app's pacing of a frame the window's edge
  itself moved only 30 times a second, in the releases before as well (there the content was a frame behind
  on top, 20 pixels at the median). A size is painted once now: the edge and the content move 55 times a second,
  the content one frame behind the edge (15 pixels at 900 pixels a second), with a picture of a chosen size and
  with one that follows the window.

* **The button that hides the interface is an eye:** it was drawn as an oval with a slash and did not read as
  one. It is an eye with an iris and a pupil now, chosen by the owner from five variants drawn in the real title
  bar. It has one state: the title bar goes away with the interface.

* **A picture that never came:** after a preset of a kind whose shader was not compiled yet, the picture could
  stay blurred with its time counting for ever. The render that begins again once the shader is ready was held
  back when the card had drawn the picture before; it began again only if the processor's stand-in arrived
  first, which it does not on a loaded machine (2 of 24 runs with four windows and a build beside them). It
  now asks whether a run of the card exists for the picture. And where the card is not heard of for 5 s while a
  picture is on its way, the picture is begun again once and is the processor's the second time; a batch left
  on its way by the picture before is waited for a second at most.

* **The card works on a picture in small units until the frame's time is up:** a picture of the chaos game was
  sent to the card one batch a frame, sized to fill the card's slice of the frame by what the last batch took.
  The guess was wrong both ways: a batch ran 11 to 24 ms where 8.3 were allowed in half of the pictures of a
  live run, and the card was busy a third of a picture's time where it was allowed half. Now the work is cut
  into units of about 1.5 ms, the end of one sends the next, and none begins once the frame's slice is over: the
  slice is filled whatever the scene costs, and a frame is late by one unit at the most, by construction. The
  slice itself is no longer half a frame but what the window leaves: the device measures what the card needs
  for the window's own frame as it runs (0.4 to 2 ms on an RTX 4070) and gives a picture the rest less 3 ms,
  at most 0.8 of a frame, and half of that while the window is used. The shaders are run once at the start, on
  two small pictures, and a flame's unplotted steps are split over two units. Off screen on an RTX 4070 at
  3840 × 2160: a flame is finished after 0.15 to 0.36 s instead of 0.53 to 0.94 s, a deep zoom after 1.8 s
  instead of 3.8 s, a Clifford or Lorenz attractor after 0.21 to 0.69 s instead of 1.4 to 1.5 s, and the first
  picture comes after 0.02 to 0.09 s; the longest unit is 2 to 3 ms (5 for a stage of a deep view, which is one
  piece) and the latest end of one is 1 to 1.5 ms past its slice. The kinds drawn pixel by pixel and the
  wallpapers are sent as they were.

* **Leaving full screen no longer shows the desktop, and a picture that is waited for is never one flat colour:**
  captured from the screen at 60 frames a second with a magenta window behind the app. Full screen from a
  maximized window was made by restoring the window first and maximizing it again on the way back, because a
  maximized window without decorations keeps the taskbar's strip empty in full screen. On the way back the
  window stood on the screen at its small size, the desktop around it, for 9 to 11 frames in the releases
  before (the system animated it) and for 1 to 3 since the glide (a hard blink). The window now stays maximized
  through full screen and the app tells Windows that the whole window is its client area meanwhile: 0 frames
  with anything but the app in the window's place in 10 runs, no strip. Separately: a deep fractal that the
  processor draws in seconds, wheeled further while its picture was awaited, showed the last picture magnified
  with the view, up to 440 times in the owner's session: a few of its pixels under the blur, one flat colour.
  The picture that stands for a view is magnified 8 times at most.

* **The Zoom slider spans the range of the view:** the slider ended at 100× while the wheel went on, so its knob
  rested at the end and the number ran on (1373.13, and 9523809523.81 where the toolbar said 9.5e9×). The slider
  now spans exactly what the view of the picture on the screen can take, logarithmically, from its smallest scale
  to its largest: the knob is at an end if and only if the view is at that limit, and the wheel, a pinch and the
  slider are one quantity with one range. When another picture has another range (another kind, or the
  processor in the place of the graphics card) the ends glide there and the knob with them; a scale set elsewhere
  (the wheel, **Reset view**) moves the knob in 0.12 s, not in one frame. The slider and the toolbar's line write
  the scale the same way: two decimals below 10, one below 100, none above, and `9.5e9×` from a million on. A
  drag of the slider that ends at a limit starts no render, no blur and no count, as a step of the wheel does
  not. Off screen on an RTX 4070 at 2580 × 1380: a flame's slider ran from 0.20× to 2056× (where the card's
  32-bit numbers end for that picture) and to 8× that once the picture at 2056× stood; with the processor alone
  from 0.20× to 8×; a Burning Ship's to 9.5e9×, and after the Mandelbrot seahorse valley was picked the end
  glided to 3.8e7× in 0.18 s.

* **Full screen, maximizing and hiding the panels are one smooth move:** recorded frame by frame, off screen, on
  an RTX 4070 at 3800 × 2120 pixels. Going to full screen from a maximized window, the window was restored to
  its small size for one drawn frame and then made full screen, and the same on the way back: every pane and the
  picture went there and back, which is the jump that was seen. Both steps are now sent to the system in one
  frame, and no frame is drawn between them. Whenever the window became smaller, the panel stood over the title
  bar for one frame and came back down; the panes are no longer held inside the window by the size they had a
  frame ago. And everything was simply somewhere else in the first frame of the new size. Now what is in the
  window goes on from where it stood on the screen: the panes glide to their places and the picture grows or
  shrinks to its new framing in 0.18 s, as one, under the one blur, and the new picture comes once, in the one
  cross-fade. On Windows the system's own animation of a maximized window is off for that moment, so that only
  one thing moves. The eye in the title bar, **H** and a tap on a phone made every pane vanish and appear in a
  single frame; they fade and rise together now, a second press turns them back from where they are, and the
  picture does not move. Counted the same way before and after, over full screen, maximizing and both together
  from a window of 800 × 520 and of 1290 × 690: 4 to 8 panes or pictures that jumped at each change and up to 4
  that turned back, now none; 3 panes that popped at each press of the eye, now none. A frame takes 17 ms
  through all of it, as before. A real switch to full screen cannot be made in a test run; it was emulated by
  the window's size ([architecture.md](docs/architecture.md#motion)). The owner's own run then showed what that
  emulation had missed, and a run of the same steps in a real window on his screen (1920 × 1080 points, the
  picture fixed at 3840 × 2160) now shows none of it: with a picture of a chosen size, every change of the window
  raised the whole blur for a fifth of a second and showed a time of many seconds though nothing was drawn (the
  rest watched the window's size, not the picture's); on the way to and from a small window the panel and the
  toolbar crossed by up to 11 points for one to three frames (the panel now makes room first); and the panes
  began to fade one frame after the press of the eye (now in its frame).

* **No long first batch of a picture:** the first batch of a flame was sized by the best speed any flame had
  shown, so a flame that costs more a step began with a batch far over its slice of a frame: 35 to 119 ms in
  seven of 23 pictures of a live run, a stutter at the start of each. Every run now begins with the smallest
  batch there is and the next is sized by what that one took. Off screen, eight random flames at 3840 × 2160 one
  after another: the first batch takes 1 ms (5 to 13 before) and no batch more than 9 ms (8 to 13 before); a
  picture is finished after 0.37 to 0.51 s instead of 0.34 to 0.44 s, which is what staying inside the slice
  costs.

* **An export says what it is doing until the file is there, and is not offered where it cannot be made:** the
  bar of an export showed the render only: at 100 % the PNG was still to be written, on one thread with no
  sign of it, 7 s for 7680 × 4320 and about half a minute for 15360 × 8640. The bar now covers the rendering and
  the writing, names the stage, and is full when the file is on the disk; **Cancel** works while the file is
  written and removes what was written. The file is compressed fast: 0.4 s instead of 7.1 s at 7680 × 4320, a
  file about 40 % larger (45 MB instead of 32 MB), the same pixels. **High** and **Ultra** at 15360 × 8640 ended
  in "could not save: the image is too large for this supersample", an English sentence of the renderer in a
  sheet in any language: a quality the renderer does not take at the chosen size is now dim and cannot be
  chosen, with a line that names the qualities that can. No error text of the renderer, of a file or of the
  network is shown any more: an export, an opened or a saved file, the list of versions and a download that
  fail say so in the app's language, and the error goes to the log.

* **The graphics card's slice of a frame is filled, and a pointer that only moves is not use:** a live run showed
  where a picture's time went: one batch is sent a frame, and about half of the frames carried almost none. A
  batch that ran short was followed by one only 1.5 times larger, the strips of the density estimation began
  with 16 rows at every drawing, and the first batch of a known card was sized without the 24 unplotted steps
  of its walkers and took 34 ms. Now a batch that ran short is followed by one that aims at 0.8 of the slice,
  up to 8 times larger; the strips begin with what the last drawing of such a scene showed; and the first batch
  leaves room for its unplotted steps. The window counted as used, with the smaller slice of 4.2 ms, at any
  event of the mouse, so in nearly every frame; now a held button or finger, the wheel, a pinch, a key and a
  pane in motion count, and a pointer that only moves leaves the picture its 8.3 ms. A draft of a moved view
  that was made smaller after one slow frame comes back to its size after eight that fit, not thirty in a row.
  Off screen on an RTX 4070 the first picture of a flame at 3840 × 2160 comes after 0.25, 0.10 and 0.20 s
  instead of 0.36, 0.15 and 0.25 s and the finished one after 0.68, 0.34 and 0.81 s instead of 0.69, 0.41 and
  0.90 s; no batch takes longer than before (10 ms), and a frame of the window is 17.0 ms as it was. A scene
  with an attractor is as it was (1.3 to 1.5 s): its batches are held to 4 194 304 steps.

* **A fractal drawn pixel by pixel follows the hand:** a Burning Ship, a Mandelbrot or any other kind the
  graphics card draws pixel by pixel was drawn anew for a moved view only when the view stood still: while it was
  dragged or wheeled the screen showed the picture it had, moved, with an empty or blurred slab where the picture
  ended, up to 84 % of the window on the way out. Now the view itself is drawn for every frame, small, and shown
  at the window's size: soft, but whole. It starts at a quarter of the picture a side and is made smaller at once
  when the card needs more than its 4 ms of a frame for it, finer slowly when it has room, never smaller than
  180 pixels on its shorter side. When the view rests the picture is drawn and takes the draft's place in one
  cross-fade; the blur over a picture that takes the card longer than 0.1 s stands over the draft meanwhile, as
  it did over the moved picture. Measured on an RTX 4070 at 3840 × 2160: what is not the picture fell from 20 %
  of the view on average (39 % at most) in a drag to 1 % in its first frame and nothing after; a frame of the
  window takes 0.9 ms instead of 0.7; the picture comes one frame later. Flames, attractors, deep views and
  what the processor renders are as they were: a draft of them is specks and takes the chaos game a tenth of a
  second or more ([performance.md](docs/performance.md#a-view-that-moves)).

* **A new picture opens at 1.0×:** before the view had one scale, the **Zoom** slider of the controls kept a
  number of its own in the app's settings. The slider is the view's scale now and nothing sets that number any
  more, but it was still read back at the start and multiplied into every picture: with the old slider left at
  0.4, every preset, every random flame and every fractal opened at 0.4×, and a view zoomed out to its limit read
  0.200×. The number is no longer saved or read as a setting. The app starts at the whole picture, **Mutate**
  goes back to it as **Randomize** does, and a file saved with the old slider moved opens at the view it was
  saved with.

* **One scale, tighter bounds, no black slab beside a moved picture:** the toolbar's line showed the wheel's
  scale and the controls had a **Zoom** of their own, so one view read 0.5× in one place and 0.4 in the other.
  There is one number now: the line shows it, the slider shows and sets it and moves with the wheel, and the
  limits hold for it; **Reset** and **Reset view** both go back to the whole picture. A figure smaller than the
  window can no longer be parked half outside it: it stays inside whole, and of a figure larger than the window
  at most a tenth of the window shows what is beside it. While a picture stands moved or made smaller until the
  next one is drawn, what is beside it is its own blurred copy fading into the background instead of a flat dark
  slab with a hard edge. A moved view of a kind drawn pixel by pixel is blurred like any other when the card
  needs more than 0.1 s for it.

* **Views have limits:** a wallpaper could be wheeled out to a small rectangle in the middle of the window with
  its edge smeared around it, and a fractal to a speck of a few pixels, drawn anew for that. A wallpaper is no
  longer made smaller than the window, is moved only as far as it still covers it and is shown at most 8 times
  larger. A flame and a fractal of any kind are shown no smaller than a fifth of their framing (0.2×) and cannot
  be dragged out of sight; a view that is only the picture the app has, magnified, stops at 8 times that
  picture. A step of the wheel that ends at a limit draws nothing. A file or a **Recent** picture with a view
  outside the limits opens at the limit; the files themselves are read as before. Beside a picture that does
  not fill the window, as one of a chosen size, there is now the window's dark background instead of the
  picture's own edge, blurred.

* **A pane hides when a list reaches it, and only then:** which panes go under an open list or sheet was a
  table: the toolbar always, the panel under some lists below a width, never under the list of languages. In a
  narrow window that list lay over the panel and both were read at once; in a wide one the toolbar hid although
  nothing came near it. Now a pane gives way if and only if the open sheet, where it stands, comes nearer to it
  than 12 points, by the rectangles as they are at that size and in that language; it goes whole and comes back
  when the sheet closes. On a computer the list of languages has no close button: it is a dropdown, left by a
  row, by its button, by a click beside it or by Escape, and its heading has the padding of every pane.

* **One time a picture:** the toolbar showed the time of the first, small picture while the blur stood (0.34 s)
  and another when the picture was finished (2.20 s). It shows one: from the click, the last step of the wheel
  or the dragged edge that asked for the picture to the finished picture on the screen. While the picture is on
  its way the time counts in tenths of a second and ends on that number; the line keeps room for the widest
  digits, so nothing beside it moves. A view that is only magnified keeps the time of its picture.

* **The window keeps its pace while the graphics card draws:** a picture drawn by the card took the card for 40 ms
  at a time, so the window drew 25 frames a second and the card showed as fully used until the picture was done
  (on master as much as after the two stages). Now the card gets a slice of every frame, half of it, and a
  quarter while the window is used; one batch is sent a frame, sized by how long the last one took, cut at once
  when it overruns; and the density estimation, which followed every batch and nothing showed, is drawn twice a
  picture, in strips. Measured on an RTX 4070 during a zoom into a flame at 2560 × 1600 with a list opened and
  closed meanwhile: a frame took 39 ms (median) and 51 ms at worst, now 4.6 ms and 13.7 ms; the card was busy
  96 % of the time, now 20 %; the picture takes 2.3 s instead of 0.6 s. A card is told to leave a picture to the
  processor when its work, at the share it gets, would take longer than the processor.

* **Everything opens, closes and resizes smoothly:** every sheet, list and panel (the presets, **Recent**, the
  types, the sizes, the languages, the export, the updates, Fractique Pro, the inspector, the glass panel of
  **F2**, a phone's styles and controls) fades and rises in over 0.18 s and leaves the same way, glass and texts
  together; closed midway it turns back from where it is. Before, each appeared at once, and its glass one frame
  after its texts: for that frame the panes of what it replaced stood empty on the screen, and in a window that
  was resized the glass trailed its panel by a frame at every step. What a sheet covers gives way in 0.08 s and
  comes back when the sheet has mostly gone; a click acts at once, also while something moves. A sheet whose
  content changes height (the updates when the list of releases arrives) grows to it instead of jumping. The
  panel no longer loses 100 points of height at one width as the toolbar comes under it: it makes room over the
  last 48 points, and the toolbar's buttons change their padding between 676 and 724 points of width instead of
  at 700. On a phone the buttons no longer start a frame at the wrong place. A window that is resized keeps its
  picture, at the scale the new one will have, and draws anew once its size has stood for 0.18 s; before, every
  step of the drag started a render (161 for 161 steps) and a frame of the drag took 25 ms where the graphics
  card draws, now 9 ms. A new picture fades in over the one before it, the first over the dark window, and the
  window's panes fade in with it. From the frame that asks for a new picture (a click, a step of the wheel, a
  control that renders, an edge that is dragged) to the finished one the screen shows two stages and one change
  between them: one still picture under a blur (70 % of the blurred copy the panes already use, whole after
  0.08 s, at no cost of its own), then the finished picture, which fades in as the blur leaves over 0.18 s. The
  still picture is the one on the screen, or once, for a new scene, a first small picture of it that takes its
  place under the whole blur; drafts that follow and the graphics card's batches in between are no longer shown,
  so nothing moves behind the blur. A picture that is there at once, a view that is only magnified, a moved view
  of a kind the card draws within a frame and a tone the card follows by itself get no blur. A test run's window
  now stands outside every screen: eframe showed it after its first frame although it was asked to stay hidden.
  Controls light up
  under the pointer over 0.12 s, a switch slides, the mark of a chosen segment glides and a small picture fades
  in when it is drawn. Where Windows is set to show no animations, nothing is animated. Test runs:
  `FRACTIQUE_TRACE`, `FRACTIQUE_FRAMES`, `FRACTIQUE_FIRST`, `FRACTIQUE_CLOCK`, `FRACTIQUE_VEIL`,
  `FRACTIQUE_MOTION` and the steps `frames`, `sweep`, `size`, `zoom`, `drag`, `preset`, `gpu`, `skip`, `rest`
  ([architecture.md](docs/architecture.md#motion)).

* **One set of measures for every pane, lists in the middle of a phone's screen:** every sheet, list and panel
  now has the same padding on all four sides, 22 points, and the close button fills the corner of it: 22 points
  from the top and from the right edge, on the title's line. Before, the list of languages had 10, the glass
  panel of **F2** 20, the phone's controls 16 above and 14 below, the styles 18 and 14, and a title's button sat
  3 to 7 points higher than its text's padding. On a phone every sheet is as wide as every other (12 points from
  the screen's edges; before the lists stood 28 and 72 points in, the size sheet and the export 8, the controls
  14), and a list stands in the middle of the screen: a long one (types, presets) grows until 12 points are left
  above and below it and scrolls, a short one is as high as its rows. The lower-edge placement of the lists, with
  a fifth of the screen empty above them, is gone. The list of languages is as wide as the others on a phone and
  has their padding on a computer too. The phone's styles have no close button again: it lay over the small
  pictures once the sheet was scrolled, and the handle closes them. On a phone the list of types leaves out the
  nine that take its processor long, as the styles always did, and with them the orange dot nobody could read
  there; a picture of such a type that is on the screen stays, and **Recent** still brings one back. A layer
  named by the app (after a preset, a type or a random picture) is named anew when the language changes; before,
  the layers kept the names of the language they were made in while everything else changed. In a narrow window
  the glass panel of **F2** stood over the toolbar; it stands above it. The export on a phone held on its side
  reached the screen's edges; it leaves 16 points. Test runs: `FRACTIQUE_STEPS` takes `lang:en`;
  `FRACTIQUE_SHOW` takes `tuning`; the language row of the phone's controls shows when the phone's screen runs
  on a computer; a screenshot run writes its panes' rectangles beside the picture.

* **Close buttons on the title's line, lists that are easy to leave:** the round button with a cross of every
  sheet, list and panel stands on the line of its title or heading, as far from the pane's right edge as the
  title is from the left one, and never over the pane's outline or its round corner; before it sat higher and
  nearer the corner than the title, and on the narrow list of languages it reached past the corner. Over a list
  the space above the button is the space beside it. The phone's styles have the button again, beside their
  handle. On an upright phone a list (the language, the fractal's type, the presets, **Recent**, the size, a
  layout, a material) stands at the lower edge of the screen and leaves the upper fifth free to tap; before, the
  list of types filled the screen. A click or a tap beside a list closes it and does nothing else: it no longer
  presses what lies under the list or moves the picture. A tap on the row already chosen closes the list of
  types without drawing the picture anew. The system's **Back** of an Android phone closes what is open, one
  thing at a press: a list, a sheet, the styles or the controls. Test runs: `FRACTIQUE_STEPS` takes
  `tap:0.5:0.1`, a tap at that share of the window, and `wait`.

* **Lists over the phone's controls can be read:** on a phone, a list opened from **Fine-tune** (the language,
  the fractal's type, the presets, **Recent**, the size, a wallpaper's layout or material) was drawn over the
  controls without a background of its own: the rows of the list and the labels, dropdowns and sliders of the
  controls stood on top of one another and neither could be read. The glass of every pane is drawn under all
  texts, so a list cannot cover a panel that stays on the screen; the controls now hide while a list is open, as
  the computer's inspector does in a narrow window, and come back when it closes. A tap beside the list closes
  the list and leaves the controls open; before, it closed the controls and left the list. The list of recent
  pictures was wider than an upright phone's screen, its sides and its close button cut off: it is as wide as the
  screen allows. The list of languages shows as many rows as fit instead of about ten with the last one cut in
  half. The fault was there since the phone got a screen of its own (#9), on every graphics backend.

* **The phone's buttons fit every screen, and what opens over the picture closes the same way:** the round
  buttons of the phone's screen stand around the lower right corner, under the thumb. The large one that makes a
  new picture is in the corner, and three small ones stand around it, all equally far from its centre: **Mutate**
  (**Similar** for a wallpaper) above it, with its right edge on the large button's, **Keep** on the diagonal
  and **Back** at its left, standing on the same line. **Style** and, over it, **Export** stand straight above **Mutate** on the right edge.
  No button has its name written beside it any more. The right edge is laid out in the room between the status
  bar or the camera cutout and the lower edge of the screen: where it does not fit, the gaps between the buttons
  close first, then the buttons become smaller, so no button is under the status bar or off the screen, upright
  or on its side. Before, **Back** was the fifth button of a column of five and was cut off at the top on phones held on
  their side. The large button has a new drawing, a die seen from a corner, and **Mutate** a wand with a star.
  On a low screen the switch of the tabs and the row of colors stand closer to the top and to one another.
  **Back** is never dim any more, on a phone and in the computer's toolbar: pressed with no older picture in the
  tab it changes nothing, and a phone gives one short tick (Android through its vibrator, with the permission
  to vibrate, which is granted without a dialog; an iPhone a light impact).
  The placard shows a mutation as a small dim tag under the picture's name (**MUTATION 1**) instead of a second
  large line that read as part of the name, and shows it for names of two words too.
  Every sheet, list and panel without a handle now has the same round button with a cross in its top right corner, on a phone and
  on a computer: **Fine-tune**, the export, the sizes, the presets, **Recent**, the fractal types, the layouts
  and materials, the languages (which gained a heading), the updates window, the **Fractique Pro** sheet and the
  glass panel of **F2**. The **Cancel** buttons of the export and of the Fractique Pro sheet and the **Close**
  buttons of the updates window and of the sheet of an owned Fractique Pro are gone; **Cancel** stays where it
  stops an export that is running. A tap beside a sheet and **Esc** work as they did, and the phone's styles, a
  sheet with a handle, close by the handle as before and have no such button.

* **Recent pictures with pictures, mutations and Back:** a random picture is named by its word alone
  (**Random**, the fractal's type, **Wallpaper**) wherever the app shows it: the seed is no longer on the
  screen, also for titles kept by older versions, and the file name offered by an export or a save keeps it
  (`Random354`, `Mandelbrot42`). Every **Mutate** is now a recent picture of its own, so the fifth row down
  is five mutations back; before, a flame and all its mutations were one entry. The pictures mutated from one
  another are a family, numbered as they were made: **Random**, **Random · mutation 1**, **Random ·
  mutation 2**. Adjusting the picture on the screen (tone, colors, layers, view) still updates its one entry.
  On a computer every row of **Recent** has a small picture, drawn in the background, and the time the picture
  was made; a phone's tiles say **mutation 2** under a mutation. The new **Back** arrow in the toolbar (also
  **Alt+←** and the mouse's back button; a round button on a phone) shows the picture one older than the one on
  the screen without reordering the list, so pressing it again goes further back; **Alt+→** and the mouse's
  forward button go forward. The stored list is read by older versions and theirs by this one. The app now
  depends on `chrono` for the local time.

* **Random flames without squares:** the second random generator no longer draws the variation `rectangles`,
  which tiled a flame into a grid of squares in about one random scene in seven. A seed that drew it gives another
  scene now (a breaking change for those seeds); every other seed gives the scene it gave, and no golden hash
  changed. The variation itself is as it was, in scene files and in imported `.flame` files.

* **Wallpapers on the graphics card:** with GPU acceleration on, the wallpapers of the Shapes tab are drawn by
  the graphics card, as flames and fractals are, so the switch means the same in every tab. A new wallpaper, a
  swipe, another layout, material, color, light or dark is on the screen at once: on an RTX 4070 a wallpaper of
  1920 × 1080 takes 1 to 3 ms and one of 3840 × 2160 2 to 62 ms, where the processor takes 0.05 to 0.2 s and 0.2
  to 0.9 s ([docs/performance.md](docs/performance.md#wallpapers-on-the-graphics-card)). The picture is the
  processor's but for one step of 255 in two to four of a hundred numbers, in every layout: across the corners
  of square tiles, where the processor's own rounding decides how bright a line one pixel wide is, the card is
  told what the processor finds. An Instant export saves it; exports at Standard, High and Ultra, the command
  line tool, the MCP tools and the server render on the processor as before, `fractique-core` did not change
  and no golden hash changed.

* **No view waits longer than 10 seconds:** a moved or zoomed view is drawn anew only when its picture is
  expected within 10 seconds on what would draw it. Otherwise the app keeps the picture it has and shows it
  magnified, as the first versions did, and the status line says **magnified picture**; nothing renders and
  nothing waits. The graphics card counts as fast for every view it draws. For the processor the app keeps the
  best speed its renders have shown in the session, for whole and for deep views of each kind, and a render
  that turns out longer than that is stopped after about a second. On a Ryzen 7 5700X without the card a deep
  view of a flame is drawn at 1280 × 720 (6 s) and at 1920 × 1080 while it fits (9 s), and magnified from
  2.83 times at 2560 × 1440 and 3840 × 2160, where it took 36 to 48 s; with an RTX 4070 views are sharp as far
  as the card reaches. The whole view is always rendered. An export of a magnified view saves what the screen
  shows, at every quality, and the sheet says so.

* **Wallpapers zoom as pictures:** a wheel or a pinch on a wallpaper magnifies its picture and keeps it so.
  Before, the wallpaper came back whole a moment later while the toolbar still showed the magnification.

* **Deep views on the graphics card:** with GPU acceleration on, a magnified view of a flame is drawn by the
  graphics card at any depth its 32-bit numbers reach, as the whole flame is. Before, the card gave up a view
  that held less than an eighth of the flame's points and the processor drew it: 33 to 48 s for a picture of
  3840 × 2160 on a Ryzen 7 5700X. The card now draws it from particles, as the renderer does: on an RTX 4070 the
  first picture of that size is there after 0.2 to 0.3 s and the whole one after 1.2 to 1.9 s
  ([docs/performance.md](docs/performance.md#deep-views-on-the-graphics-card)). The picture differs from the
  processor's as two of the processor's renders with different seeds differ. Exports at Standard, High and
  Ultra, the command line tool, the MCP tools and the server render on the processor as before, and no golden
  hash changed.

* **A picture the graphics card gives up costs that picture only:** a batch that took longer than its time
  ended the card's part until the app was started again, and that time is measured on the processor, which is
  late itself while it renders at full load. Now the picture is given to the processor and the next one is the
  card's again; only a submission that does not end, or an error of the card, ends its part for the session.

* **New icon:** a white lowercase f whose hook winds into a spiral, over a Buddhabrot in blue-grey on near-black,
  on every platform: the window, the Windows program, the macOS and Linux packages, the browser tab, Android and
  iOS. Below 64 pixels the letter is drawn heavier and simpler. On iOS the same icon serves the light and the dark
  appearance, dimmed in the dark one.

* **Recent pictures of each tab:** Flames, Fractals and Shapes each keep their own recent pictures, 20 of them,
  and the list a tab opens holds only its own, with the picture on screen marked.

* **Names of fractals in every language:** the names of the kinds, of their presets and of the flame presets are
  shown in the app's language, in all 15 (the kinds were kept in English for a while, the presets always were). The default
  name of an exported picture and of a scene file is made from the English name as before (`MandelboxStation`).

* **Heavy kinds:** the kinds that only the processor draws and that take it long (Buddhabrot, Hopalong,
  Gumowski-Mira, Mandelbulb, Mandelbox, Menger sponge, Kleinian, Lyapunov, L-system) carry an amber dot in the
  list of types on a computer, with a hint, and are left out of the tiles on a phone. A scene or a `.frcq` file
  with such a kind still opens and renders everywhere.

* **Fractals of other kinds:** a layer of a scene is now a flame or a fractal of another kind, and three kinds are
  here: the **Mandelbrot set** and **Julia sets**, drawn pixel by pixel with smooth colors, a thin dark rim along
  the set, colors that fade to dark away from it and anti-aliasing where the picture has edges, and the
  **Clifford attractor**, drawn as a density through the flame's histogram, tone mapping and glow. They use the
  palettes, the camera, the layers and the blend modes of flames, so a Mandelbrot set can lie under a flame, and a
  view goes as deep as 64-bit numbers reach (a zoom of about 10¹³ at 1080 pixels; the app stops at 10¹⁰). Each
  kind has six presets, a random picture for every seed that is checked to be worth looking at, and a mutation.
  * **App:** a third tab, **Fractals**, between **Flames** and **Shapes**: the type (a list on a computer, tiles
    with small pictures on a phone), the presets of the type, **Randomize** and **Mutate**, a slider for every
    parameter, and the palettes, layers, tone, view, recent pictures, collection, export, session and `.frcq`
    files as for flames. Which of the kinds the graphics card draws is in the entries on GPU acceleration below.
  * **Command line:** `fractique fractals` lists the kinds with their parameters and presets; `--fractal KIND`
    with `render --random SEED` and `randomize SEED` makes the random fractal of a seed; `--preset` and
    `preset` take the fractal presets (`mandelbrot-seahorse-valley`, `julia-dragon`, `clifford-wings`, ...).
  * **MCP and HTTP:** the tool `list_fractals`, `fractal` in `randomize_scene` and in `render_scene` with
    `random_seed`; `GET /fractals`, and `fractal` in `POST /random` and in `POST /render` with `random`.
  * **Scene format:** a layer holds `fractal` in place of `flame`
    ([docs/scene-format.md](docs/scene-format.md#fractal)). The version stays 1: every scene and `.frcq` file
    written before loads and renders to the same pixels, and the golden hashes of flames, wallpapers and random
    scenes are untouched. In `fractique-core`, `Layer::flame` became `Layer::content`, with `flame()`,
    `fractal()`, `camera()`, `tone()` and `palette()` to reach into either.
  * The kinds live in a registry, one file each ([docs/architecture.md](docs/architecture.md#how-to-add-a-fractal-type)).
    The golden hashes of the three kinds were produced on Linux.
  * **Seven more kinds**, each with six presets, a random picture for every seed and a mutation. Escape-time:
    **Burning Ship** (`burning_ship`), **Tricorn** (`tricorn`), **Multibrot** (`multibrot`, z^d + c with the
    power from 2 to 8; a power that is not whole shows the seams of its branch cut), **Phoenix** (`phoenix`, the
    Julia form and the map of its constants) and **Magnet** (`magnet`, both types, with the orbits that settle
    on 1 colored like those that escape). Root-finding, the first two of that family: **Newton** (`newton`,
    zⁿ − 1 with n from 2 to 12 and five named polynomials, a complex relaxation, the basins colored by root and
    shaded by a smooth count) and **Nova** (`nova`, Newton's step plus a constant, as the map of the constants
    and as its Julia form). In the app a parameter that is a choice (the form, the type, the polynomial) shows
    the names of its values in place of a slider. The Mandelbrot and Julia sets render as before.

* **Five more fractals, the rest of the orbit family:** the **Buddhabrot** and the **De Jong**, **Lorenz**,
  **Gumowski-Mira** and **Hopalong** attractors, drawn as densities like the Clifford attractor. Each has six
  presets, a random picture for every seed that is checked to be worth looking at, and a mutation
  ([docs/scene-format.md](docs/scene-format.md#orbit-kinds)).
  * **Buddhabrot:** the orbits that leave, or with `anti` those that stay; colored by the length of the orbit,
    by where it started, or as the Nebulabrot of three iteration limits in blue, green and red; `turn` and `tilt`
    turn the picture towards the plane of the starting points. Its starting points are taken only from the
    cells of the plane that can hold an orbit to draw, evenly, so the picture is the one blind sampling gives and
    most of the wasted orbits are never started.
  * **Lorenz:** integrated in fixed steps by the fourth-order Runge-Kutta method, turned and tipped by two
    angles; `duration` goes from a few separate lines to the density of the whole attractor.
  * **The attractors** are colored by the speed of the orbit, the direction of its step or the distance from the
    middle (the height, for Lorenz), each spread evenly over the palette. In the app these colorings, those of
    the Buddhabrot and its two kinds of orbits are choices by name.
  * In `fractique-core`, a kind drawn as a density may bring its own colors in place of the layer's palette
    (`Orbit::colors`); the Nebulabrot does. Nothing changes for the kinds that were there: their golden hashes
    are untouched. The golden hashes of the five kinds were produced on Linux.
* **Solids:** three kinds in three dimensions, `mandelbulb` (any power from 2 to 16), `mandelbox` (scale, fold
  limit, inner radius) and `menger_sponge` (the level, and a twist, a bend and a scale that fold it into other
  shapes), each ray-marched by a distance estimate on the processor, with a key light, soft shadows, shading of
  the pits, fog by depth, a halo and colors from orbit traps through the palette. The eye turns around the solid
  (`turn`, `tilt`), comes nearer or goes inside (`distance`) and has a `field_of_view`; the app's pan and zoom
  move and magnify the picture as a longer lens does. Six presets a kind, a random picture for every seed that is
  measured before it is kept, and a mutation ([docs/scene-format.md](docs/scene-format.md#solid-kinds)). A
  per-pixel kind can now learn the width of a pixel, prepare itself for a picture and say how much neighbouring
  pixels may differ before they are drawn again; the pictures of the other kinds are untouched. The golden hashes
  of the three kinds were produced on Linux.

* **Geometric fractals:** four kinds join the registry. The **Apollonian gasket** (`apollonian`), from any
  Descartes configuration, as shaded discs, rings or a glow, colored by generation, size or ancestor. **Kleinian
  limit sets** (`kleinian`), the two-generator groups of "Indra's Pearls" by Grandma's recipe and Maskit's, with
  the traces as parameters, drawn by a depth-first search of words that follows the view. **L-systems**
  (`l_system`): a turtle draws what an axiom and rewriting rules spell, with brackets for branches; the dragon
  curve, the Koch snowflake, the Sierpinski triangle, the Hilbert and Gosper curves, a plant and a bush are
  presets. The **Lyapunov fractal** (`lyapunov`) of the logistic map for any sequence of A and B. Each has six or
  seven presets, a random picture for every seed and a mutation.
  * **Scene format:** a fractal may hold `text` beside `params`, for parameters that are text: the axiom and the
    rules of an L-system, the sequence of a Lyapunov fractal
    ([docs/scene-format.md](docs/scene-format.md#geometric-kinds)). A field that may be left out: the version
    stays 1 and every scene written before loads and renders as it did.
  * **App:** a field under the sliders for every text of the chosen fractal; the style and the coloring of a
    gasket, what an L-system's colors follow and the recipe of a Kleinian group are choices with names.
  * **Command line, MCP, HTTP:** `fractique fractals`, `list_fractals` and `GET /fractals` list the texts of a
    kind with their defaults.
  * In `fractique-core`: `Draw::PixelText` and `Draw::DensityText` build a kind from the whole `Fractal`, with
    its `Text`s; `Orbit::gain`, 1 unless a kind says otherwise, scales a density, which lets a curve stay as
    bright at every zoom. No existing kind renders differently: every golden hash from before is untouched, and
    those of the four kinds were produced on Linux.

* **GPU acceleration:** the graphics card draws the escape-time kinds (Mandelbrot, Julia, Burning Ship, Tricorn,
  Multibrot, Phoenix, Magnet) and the root-finding kinds (Newton, Nova) too, pixel by pixel: first in blocks, then
  every pixel, then the edges from more samples, and a moved view is drawn again at once. What one submission may
  do is bounded before it is sent, as for flames: at most 100 000 000 steps of the kind's loop and what the card
  has shown it does in 0.1 s; a scene that could need more for 64 pixels is the processor's, as are views too deep
  for 32-bit numbers, scenes that mix these kinds with flames, the geometric kinds and the solids. On a GeForce
  GTX 1650 a picture of 1920×1080 takes 0.02 to 0.4 s where the processor beside it takes 0.5 to 2.1 s. Exports at
  Standard, High and Ultra, the command line, MCP and the server are the processor's as before: no golden hash
  changes ([docs/architecture.md](docs/architecture.md#fractals-drawn-pixel-by-pixel-on-the-graphics-card)).
* **GPU acceleration:** the graphics card draws three attractors beside flames: **Clifford**, **De Jong** and
  **Lorenz**, alone or in a scene with flames. The Gumowski-Mira map, the Hopalong and the Buddhabrot stay the
  processor's: the first is slower on a card than on the processor, 32-bit numbers leave the curve of the second,
  and the work of the third has no bound that is known before it is done
  ([docs/architecture.md](docs/architecture.md#orbits-on-the-graphics-card)).
* **GPU acceleration, safer for the new kinds:** what the graphics card is given in one go for a scene with an
  attractor in it has a hard bound that is known before it is sent: at most 4 steps a pixel and 4 194 304 steps
  for the walkers of all layers, and a density estimation over at most 17 179 869 184 cells; a batch that takes
  longer than 0.5 s gives its picture to the processor. A scene of flames alone is drawn by the card as it
  always was: its batches grow with the card's pace and it is given 20 s
  ([docs/architecture.md](docs/architecture.md#the-work-of-one-submission)).
* **GPU acceleration:** the trial of a built-in graphics chip gives way when a picture is asked for and begins
  again when the app rests; its verdict is kept again, so a slow phone is tried once, not at every start. The
  first picture after a start is no longer handed to the processor because the shaders took time to load.
* **Phones:** the small pictures of the styles are drawn on one thread, so the window and the picture keep their
  cores, and are kept as files, so a sheet that was drawn once opens at once the next time.
* **GPU acceleration:** the trial that decides whether a built-in graphics chip is used draws a larger picture and
  asks for the card to be 1.3 times as fast, not twice: a phone with an Adreno 740 was left out at 1.7.
* **GPU acceleration:** a graphics card of its own (not one built into the processor) is used without a trial.
  Elsewhere the card's best of three trial runs counts, and a verdict that it is slow is not kept between launches,
  so one trial at a bad moment no longer hides the switch for good.
* **App:** with GPU acceleration on, the graphics card's picture of a flame is the screen's picture, and the
  processor no longer renders it a second time: a new flame at 3840×2160 took the processor 45 to 155 s of core
  time before and takes it 2 to 3 s now, the start of the app included, and the whole picture is there after 0.2
  to 0.3 s on an NVIDIA RTX 4070. Tone, opacity and blend follow at once. **Instant** export saves that picture,
  copied back from the card, once all of its samples are there; exports at Standard, High and Ultra are rendered
  by the processor and exact as before, and so are the command line tool, the MCP tools and the server. What the
  card does not draw (wallpapers) is rendered as before.
* **App:** the **Graphics** section is shown only where the graphics card helps. The app tries the card against the
  processor once, after its first picture, and remembers the answer for that card and version; where the card has
  no compute shaders or is not at least 1.3 times as fast, there is no switch and the app works on the processor
  alone, as it did before any acceleration. A card that turns out slow on one picture gives that picture to the
  processor at once, and an error of the card takes the switch away for the session. Under the switch are the
  card's name and the note, which now says that an Instant export can differ between devices too.

* **Sizes:** the size list is in groups with tabs over it: Screens, iPhone, iPad, Mac, Watch and Android. Every
  build has the full Apple list now (59 sizes in all), with the Apple Watch screens for a picture to use as a
  watch face; the App Store builds leave the Android group out.
* **App Store builds:** a locked size in the export list opens the Fractique Pro sheet when it is chosen, not at
  Save.
* **App Store builds (iPhone, iPad, Mac):** one purchase, **Fractique Pro**, paid once. Without it an export of
  2560 × 1440 pixels or more by pixel count (width × height of 3 686 400 or more) and a size typed into
  **Custom** are locked; smaller listed sizes, the picture on the screen at any size and every other feature stay
  free. Locked sizes show a small lock; choosing one, or **Save** or **Share** with one, opens a sheet with the
  App Store's price, **Buy**, **Restore purchases** and **Cancel**, and after the purchase the export goes on by
  itself. The size lists of these builds hold computer screens and 34 Apple screens (iPhone, iPad, MacBook, iMac,
  Studio Display, Pro Display XDR) instead of the Android ones. Windows, Linux, Android and the web are not
  changed: everything is free there and the builds hold no purchase code (the `store` feature,
  [docs/usage.md](docs/usage.md#free-and-fractique-pro), [docs/ios.md](docs/ios.md#fractique-pro-the-purchase)).

* **App:** with GPU acceleration on, the graphics card plays the chaos game itself, so a new flame is on the screen
  in a fraction of a second: at 3840×2160 the first picture comes after about 0.03 s and all of its samples after
  0.2 to 0.4 s on an NVIDIA RTX 4070, where the processor takes 4 to 12 s. The picture is at the full size from the
  start and grows finer with every frame. Wallpapers and graphics cards
  without compute shaders are left to the processor, and so is everything when the switch is off
  ([docs/architecture.md](docs/architecture.md#the-chaos-game-on-the-graphics-card)).

* **App:** while Export is open the controls are hidden, and while any sheet or list is open (export, presets,
  adding a layer, sizes, layouts, recent, languages, updates) the picture under it does not move, zoom or reset.
* **App:** under **GPU acceleration**, while it is on, a line says that the quick picture can differ slightly
  between devices and that exports are always exact.
* **Phones:** a finger that lands on a slider while it scrolls a panel no longer moves the slider: the slider
  follows a sideways move only, and a vertical one scrolls.
* **App:** GPU acceleration, a switch in the controls, on at first and kept between launches. While a tone slider,
  the zoom, a layer's opacity or its blend moves, the graphics card draws the picture from the layers already
  rendered, so it answers at once. A change to a layer's tone no longer runs the chaos game again. Exports at
  Standard, High and Ultra are the processor's, as before; with the switch off the app renders as it did.
* **Renderer:** `render_density` and `develop` split `render_layer` in two, before and after the tone mapping,
  and give the same layer bit for bit. Existing scenes render as before.
* **App:** the last pictures are remembered, 20 of each tab. **Recent** in the inspector lists them, and the
  styles of a phone show them as a row of small pictures; a click brings one back. The list is kept between
  launches.
* **MCP over HTTP:** `fractique-server` speaks MCP at `POST /mcp` (Streamable HTTP, revisions `2025-06-18` and
  `2025-03-26`) with the tools of `fractique mcp`, from the same code ([docs/mcp.md](docs/mcp.md#over-http)). A
  tool call is answered as an event stream with its progress; `notifications/cancelled`, the end of the session
  and a closed connection stop a render. Renders obey the limits of the server and wait in its queue. `file` and
  `save_to` do not exist there: a client on the network cannot read or write the disk of the server. On by
  default; `--mcp off` or `FRACTIQUE_MCP=off` turns it off, `--mcp-origin` names the web pages that may call it.
  `GET /limits` says whether it is on.
* **Export:** a fourth quality, **Instant**, now the first choice: it saves the picture that is on the screen at
  once instead of rendering it anew. Standard, High and Ultra render as before.
* **Android 9 and older:** storage access is asked for when Save is pressed, before the picture is made, and the
  export goes on by itself once it is allowed.
* **Scenes:** a number in a scene file is read as exactly the value written. Before, one could come out a last
  digit away, so a scene saved and read again could render differently from the one it was saved from. The two
  presets this touched, `ice-crystal` and `violet-whirl`, now carry the numbers they were always read as, so every
  preset renders the same pixels as before.
* **HTTP server:** `fractique-server` offers presets, random scenes, wallpapers and renders as PNG over HTTP
  ([docs/server.md](docs/server.md)). Renders wait in a bounded queue and a full one answers 429; the size, the
  samples and the time of a render are limited by settings; a client that disconnects cancels its render. It can
  serve the web build of the app beside the API.
* **Docker:** a `Dockerfile` and a `docker-compose.yml` build one small image with the server and the web app,
  run as a user without rights.
* **MCP server:** `list_wallpaper_styles` and `randomize_wallpaper` make wallpapers; `render_scene` also takes
  `wallpaper_seed` (with `layout`, `material`, `dark`), `file` (the path of a `.frcq` or JSON scene) and `frcq`
  (the bytes of a `.frcq` in base64). A call with a progress token gets `notifications/progress` while it renders,
  and `notifications/cancelled` stops a render, which then gets no answer. `ping` is answered during a render.
* **README translations:** the 14 translations name the buttons, the blend modes and the other terms as the app
  shows them in that language.
* **Command line tool:** `fractique convert` turns a `.frcq` file into its scene as JSON (`--document` for
  everything the file holds) and a JSON scene or document into a `.frcq`.

* **Variations:** 21 more, 49 in all: `waves`, `popcorn`, `rings` and `fan`, which take their shape from the
  transform's own `affine` numbers, and `blob`, `pdj`, `fan2`, `rings2`, `perspective`, `noise`, `juliascope`,
  `radial_blur`, `pie`, `ngon`, `rectangles`, `arch`, `square`, `rays`, `blade`, `secant2` and `twintrian`.
  `fractique variations` lists them with their parameters, and the `.flame` import reads them and their
  parameters. Existing scenes render as before, and `fractique randomize` gives the same scene for every seed: the
  randomizer does not use the new variations.
* **Randomizer:** a second random generator beside the first. It draws from all 49 variations, gives the
  parametric ones random parameters and skips more of the pictures that come out as specks, a dim haze or an even
  fog. `fractique randomize SEED --generator 2` and `fractique render --random SEED --generator 2` use it; without
  the option both commands give the scene they always gave. The app's Randomize makes its new flames with it, and a
  session or a `.frcq` file holds the number of the generator (`generator`; a file without it opens as before).
  The scenes of the first generator are now pinned by tests.
* **Presets:** `golden-filigree`, `frost-lotus`, `azure-turbine`, `orchid-pinwheel`, `lilac-mosaic`,
  `amber-lantern`, `emerald-plume` and `ember-thorns`, made with the new variations.
* **App (desktop):** one window at a time. A second start brings the open window to the front and passes it the
  file it was given. A debug build, or `FRACTIQUE_MANY=1`, still opens as many as asked.
* **Wallpapers:** a scene can hold `sheets`, a stack of flat shapes (rounded boxes, petals, rings and arcs, waves and
  ribbons) with lit edges, gradients, translucency and soft shadows, drawn under the flame layers.
  `fractique render --wallpaper SEED` makes one for every seed out of fourteen layouts (bands, discs, waves,
  ribbons, burst, arcs, tiles, bars, fan, slats, planes, steps, frames, lines), six materials (relief, paper,
  glass, glow, steel, carbon) and generated colors, each with a light and a dark version. Scenes without `sheets`
  render as before.
* **App:** a switch in the controls between flames and wallpapers, the tabs **Flames** and **Shapes**. In wallpaper
  mode the layout, the material and light or dark are chosen there; Randomize makes a new wallpaper and Mutate
  another of the same layout and material.
* **Phones:** the app has a screen of its own on a phone. The picture fills it, with its name and number as a
  placard. A round glass button makes a new one; four small ones keep it, make a similar one, open the styles and
  export. A swipe goes to the next or the previous wallpaper. The styles are a sheet of small live pictures of every
  layout, material and preset, with the kept pictures in a row above them and a row of colors for the wallpapers.
* **iPhone:** an exported picture goes to Photos or to the share sheet.
* **Android:** an exported picture goes to the gallery (`Pictures/Fractique`) or to other apps through Share.
* **Phones:** Fine-tune in the styles opens every control over the whole screen; the panels are laid out for a
  phone on its side too.
* **Scene files:** the app saves `.frcq` files, a packed format of its own, about a seventh of the size of the JSON:
  the scene with the tone, the view, the size and the wallpaper choices, so it opens as it was saved. The command
  line tool reads them. A file given to the app at start opens in it.
* **Session:** the app comes back to the scene, the view and the settings it was left with.
* **App:** a wallpaper's layout and material are chosen from lists, like a preset.
* **Wallpapers:** a wallpaper can be asked for in one color (`hue` of the request). The bands layout no longer
  makes an invalid shape on screens about 1.6 times as tall as wide.
* **App:** a double click on a `.frcq` file opens it in Fractique. The Windows program registers the file type for
  the current user when it starts; the `.deb` and the AppImage carry the type and the association. On macOS the type
  is declared, but a double click does not open the file yet.

* **Renderer:** fractal flames with 28 variations, layers (add, screen, over), rotational and mirror symmetry,
  density estimation, glow, ordered dithering, any resolution up to 16384 pixels a side.
* **Any zoom:** a view magnified up to ten thousand million times is drawn sharp, from particles kept inside it; in
  the app, drag, wheel and pinch move the view and zoom in and out, and the picture is drawn anew when the view rests.
  A scene whose view holds less than an eighth of its flame renders differently from before this.
* **Exact repeatability:** the same scene and options give the same pixels on every platform and thread count.
* **Presets:** `aurora-spiral`, `emerald-star`, `emerald-burst`, `emerald-mandala`, `ember-vortex`, `azure-rings`, `golden-bloom`,
  `solar-mandala`, `jade-orb`, `violet-whirl`, `radiant-star`, `teal-lattice`, `amber-tree`, `silk-current`,
  `tide-curl`, `orchid-knot`, `frost-crown`, `green-comet`, `moss-nebula`, `lilac-cloud`, `ice-crystal`.
* **Randomizer and mutation:** a scene for every seed, and variants of a scene.
* **Command line tool:** `render`, `presets`, `preset`, `randomize`, `mutate`, `fit`, `import`, `variations`, `mcp`.
* **Import:** `.flame` files of flam3 and Apophysis convert to scenes.
* **MCP server:** `fractique mcp` lets an AI agent list presets, make and change scenes and render them.
* **App:** a window with the flame and glass panels over it, for Windows, macOS, Linux, Android, iOS and the web,
  in 15 languages.
* **Export, scenes, updates:** the app exports PNGs at any size, opens and saves scenes, checks GitHub for newer
  versions and installs any released version.
* **Downloads:** one program file for Windows (no installer), an AppImage and a .deb, a macOS app and an Android
  APK, built by a release workflow.
* **Setup:** `setup.sh` (Linux, macOS) and `setup.cmd` (Windows).

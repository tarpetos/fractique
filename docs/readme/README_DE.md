<div align="center">
  <img src="../../assets/icon/icon-256.png" alt="Fractique" width="128" height="128">
  <h1>Fractique</h1>
  <p><strong>Ein Generator für fraktale Flammen und Hintergrundbilder, der aus derselben Eingabe auf jeder Plattform dasselbe Bild erzeugt.</strong></p>
  <p><sub>
    <a href="../../README.md">English</a> · <a href="README_UK.md">Українська</a> · <a href="README_RU.md">Русский</a> · <strong>Deutsch</strong> · <a href="README_FR.md">Français</a> · <a href="README_ES.md">Español</a> · <a href="README_IT.md">Italiano</a> · <a href="README_JA.md">日本語</a><br>
    <a href="README_CN.md">简体中文</a> · <a href="README_PT.md">Português</a> · <a href="README_KO.md">한국어</a> · <a href="README_HI.md">हिन्दी</a> · <a href="README_TR.md">Türkçe</a> · <a href="README_PL.md">Polski</a> · <a href="README_ID.md">Bahasa Indonesia</a>
  </sub></p>
</div>

Fractique erzeugt drei Arten von Bildern. Fraktale Flammen: leuchtende, zarte Fraktale aus feinen Kurven und weichem Licht. Klassische Fraktale: die Mandelbrot-Menge, Julia-Mengen und seltsame Attraktoren, mit weichen Farbverläufen und tiefem Zoom. Und Hintergrundbilder: flache Blätter mit beleuchteten Kanten und weichen Schatten, jedes in einer hellen und einer dunklen Fassung. Du wählst eine Vorlage, würfelst ein zufälliges Bild oder legst mehrere Flammen übereinander und exportierst das Ergebnis in jeder Größe, vom Handybildschirm bis zum 4K-Monitor. Eine Szene ist eine kleine Datei, und dieselbe Datei ergibt immer exakt dieselben Pixel, unter Windows, macOS, Linux, Android und iOS.

<p align="center">
  <img src="../samples/aurora-spiral.png" alt="Die Vorlage aurora-spiral: türkise und grüne Spiralschleier auf dunklem Hintergrund" width="880">
</p>

## Was Fractique kann

- **Flammen:** 49 Variationen, Ebenen mit Mischmodi, Dreh- und Spiegelsymmetrie, Dichteschätzung, Leuchten und 29 eingebaute Vorlagen.
- **Fraktale:** die Mandelbrot-Menge, Julia-Mengen und der Clifford-Attraktor, jeweils mit Vorlagen und einem Bild für jeden Seed; sie teilen Paletten, Ebenen und den tiefen Zoom mit den Flammen.
- **Körper:** Mandelbulb, Mandelbox und Menger-Schwamm in drei Dimensionen, mit Licht, Schatten und Nebel; man kann um sie herumgehen oder hineinfliegen.
- **Hintergrundbilder:** eines für jeden Seed, aus 14 Layouts und 6 Materialien, jedes in einer hellen und einer dunklen Fassung, passend zur Größe des Bildschirms angeordnet.
- **Die App:** eine Codebasis für Windows, macOS, Linux, Android, iOS und das Web. Das Bild füllt das Fenster, darüber schweben Glasflächen; auf dem Desktop hat die App eine eigene gläserne Titelleiste. In 15 Sprachen.
- **„Zufall“ und „Mutieren“:** jeder Seed ist ein Bild; Flammen, die nur ein Punkt oder ein gefülltes Quadrat wären, werden übersprungen. „Mutieren“ erzeugt eine Variante des Bildes, das dir gefällt.
- **Ebenen:** eine Flamme aus den Vorlagen oder eine zufällige hinzufügen, entfernen, umsortieren und für jede den Mischmodus („Summe“, „Screen“, „Darüber“) und die Deckkraft einstellen. Bis zu 16 Ebenen.
- **Bildgröße:** die des Fensters, einer von 59 Bildschirmen (Computer bis 4K, Android-Handys und -Tablets, iPhones und iPads) oder jede Breite und Höhe von 16 bis 16384 Pixeln.
- **Tiefer Zoom:** Ziehen, Mausrad und Zwei-Finger-Geste bewegen die Ansicht und vergrößern sie bis zu zehnmilliardenfach. Das Bild folgt sofort und wird scharf neu gezeichnet, sobald die Ansicht ruht.
- **Handys:** ein eigener Bildschirm. Ein runder Knopf erzeugt ein neues Bild, ein Wisch führt zum nächsten Hintergrundbild, ein Blatt mit kleinen Live-Bildern wählt den Stil, und die Bilder, die du behältst, bilden eine Sammlung.
- **Export:** ein PNG in jeder Größe: sofort, so wie es auf dem Bildschirm ist, oder im Hintergrund in drei höheren Qualitätsstufen gerendert. Auf dem Handy wird es in Fotos oder der Galerie gespeichert oder geteilt. Alle Funktionen sind auf jeder Plattform kostenlos; nur die Version aus dem App Store (iPhone, iPad, Mac) verlangt für den Export ab 2560 × 1440 Pixeln oder in einer eigenen Größe den einmaligen Kauf von Fractique Pro.
- **Projektdateien:** eine `.frcq`-Datei enthält die Szene samt Ansicht, Ton und Größe und öffnet sich so, wie sie gespeichert wurde ([docs/scene-format.md](../scene-format.md)). Einfache `.json`-Szenen und `.flame`-Dateien von flam3 und Apophysis lassen sich ebenfalls öffnen. Zwischen zwei Starts kehrt die App zu der Sitzung zurück, in der sie verlassen wurde.
- **Updates:** ein Fenster listet jede veröffentlichte Version mit ihren Hinweisen auf und installiert jede beliebige davon. Unter Windows ist die App eine einzige portable `.exe`, die sich selbst ersetzt.
- **Exakte Wiederholbarkeit:** dieselbe Szene ergibt auf jedem Rechner dieselben Pixel, geprüft durch Tests unter Linux, Windows und macOS ([docs/determinism.md](../determinism.md)).
- **Das Kommandozeilenwerkzeug** (`fractique`): rendert Flammen und Hintergrundbilder als PNG, erzeugt zufällige Szenen und Varianten, konvertiert `.flame`-Dateien.
- **MCP-Server** (`fractique mcp`): ein KI-Agent erstellt, ändert und rendert Szenen ([docs/mcp.md](../mcp.md)).
- **HTTP-Server** (`fractique-server`): eine HTTP-API mit Render-Warteschlange und Grenzen, MCP für KI-Agenten unter `/mcp`, liefert die Web-App aus, gibt es als Docker-Image ([docs/server.md](../server.md)).

| | |
| --- | --- |
| ![emerald-star](../samples/emerald-star.png) | ![ember-vortex](../samples/ember-vortex.png) |
| `emerald-star` (zwei Ebenen) | `ember-vortex` |
| ![golden-bloom](../samples/golden-bloom.png) | ![solar-mandala](../samples/solar-mandala.png) |
| `golden-bloom` | `solar-mandala` |

Fraktale anderer Art: die Mandelbrot-Menge, eine Julia-Menge und der Clifford-Attraktor:

| | | |
| --- | --- | --- |
| ![mandelbrot](../samples/fractal-mandelbrot.png) | ![julia](../samples/fractal-julia.png) | ![clifford](../samples/fractal-clifford.png) |
| `mandelbrot-spiral` | `julia-spirals` | `clifford-wings` |

Als Dichte gezeichnete Bahnen: das Buddhabrot und die Attraktoren von De Jong, Lorenz, Gumowski-Mira und Hopalong:

| | | |
| --- | --- | --- |
| ![buddhabrot](../samples/fractal-buddhabrot.png) | ![de_jong](../samples/fractal-de_jong.png) | ![lorenz](../samples/fractal-lorenz.png) |
| `buddhabrot-nebula` | `de_jong-classic` | `lorenz-butterfly` |
| ![gumowski_mira](../samples/fractal-gumowski_mira.png) | ![hopalong](../samples/fractal-hopalong.png) | |
| `gumowski_mira-feathers` | `hopalong-web` | |

Geometrische Fraktale: ein Apollonisches Netz, die Grenzmenge einer Kleinschen Gruppe, ein L-System und ein Ljapunow-Fraktal:

| | | | |
| --- | --- | --- | --- |
| ![apollonian](../samples/fractal-apollonian.png) | ![kleinian](../samples/fractal-kleinian.png) | ![l-system](../samples/fractal-l_system.png) | ![lyapunov](../samples/fractal-lyapunov.png) |
| `apollonian-threefold` | `kleinian-cusp` | `l_system-plant` | `lyapunov-feathers` |

Körper in drei Dimensionen: Mandelbulb, Mandelbox und Menger-Schwamm:

| | | |
| --- | --- | --- |
| ![mandelbulb](../samples/fractal-mandelbulb.png) | ![mandelbox](../samples/fractal-mandelbox.png) | ![menger sponge](../samples/fractal-menger-sponge.png) |
| `mandelbulb-crown` | `mandelbox-station` | `menger_sponge-classic` |

Hintergrundbilder, hell und dunkel, jedes mit seinem Layout und Material:

| | | | |
| --- | --- | --- | --- |
| ![waves](../samples/wallpaper-waves-light.png) | ![burst](../samples/wallpaper-burst-light.png) | ![tiles](../samples/wallpaper-tiles-light.png) | ![fan](../samples/wallpaper-fan-light.png) |
| `waves`, `glass` | `burst`, `glow` | `tiles`, `relief` | `fan`, `glass` |
| ![discs](../samples/wallpaper-discs-dark.png) | ![bars](../samples/wallpaper-bars-dark.png) | ![arcs](../samples/wallpaper-arcs-dark.png) | ![lines](../samples/wallpaper-lines-dark.png) |
| `discs`, `paper` | `bars`, `paper` | `arcs`, `carbon` | `lines`, `paper` |

## Schnellstart

Ein Befehl baut die App und das Kommandozeilenwerkzeug. Fehlt Rust, wird es installiert.

| System | Befehl |
| --- | --- |
| Linux, macOS | `./setup.sh` |
| Windows | `setup.cmd` |

Die App starten:

```sh
./target/release/fractique-gui
```

Oder von der Kommandozeile rendern:

```sh
# eine Vorlage als 4K-Bild
./target/release/fractique render --preset aurora-spiral --size 3840x2160 --quality high -o aurora.png

# die zufällige Flamme mit Seed 42 für einen Handybildschirm
./target/release/fractique render --random 42 --size 1080x2400 --quality mobile -o phone.png

# die zufällige Mandelbrot-Ansicht mit Seed 7
./target/release/fractique render --random 7 --fractal mandelbrot --size 1920x1080 -o mandelbrot.png

# das Hintergrundbild mit Seed 7 für ein Handy, hell und dunkel
./target/release/fractique render --wallpaper 7 --size 1290x2796 -o wallpaper.png
./target/release/fractique render --wallpaper 7 --dark --size 1290x2796 -o wallpaper-dark.png

# eine zufällige Szene als Datei behalten, ändern, rendern
./target/release/fractique randomize 42 -o scene.json
./target/release/fractique render scene.json -o mine.png
```

Unter Windows heißen die Programme `target\release\fractique-gui.exe` und `target\release\fractique.exe`. Alles, was die App und die Befehle können: [docs/usage.md](../usage.md).

## Dokumentation

Die Dokumentation ist auf Englisch.

| | |
| --- | --- |
| [docs/usage.md](../usage.md) | Die App, die Befehle, die Qualitätsstufen, wie man eine Szene bearbeitet |
| [docs/scene-format.md](../scene-format.md) | Jedes Feld einer Szenendatei, jede Variation, Blätter, die `.frcq`-Datei |
| [docs/mcp.md](../mcp.md) | Der MCP-Server: einen KI-Agenten anbinden, die Werkzeuge |
| [docs/server.md](../server.md) | Der HTTP-Server: Endpunkte, Grenzen, Docker |
| [docs/ios.md](../ios.md) | Bauen und Ausführen auf einem iPhone oder im iOS-Simulator |
| [docs/releasing.md](../releasing.md) | Wie ein Release entsteht und wie sich die App selbst aktualisiert |
| [docs/architecture.md](../architecture.md) | Wie die Teile zusammenpassen, wie ein Rendering abläuft |
| [docs/determinism.md](../determinism.md) | Warum sich ein Rendering exakt wiederholt, was zugesagt ist und was nicht |
| [docs/performance.md](../performance.md) | Gemessene Renderzeiten und Speicherbedarf |
| [docs/roadmap.md](../roadmap.md) | Was fertig ist und was als Nächstes kommt |

## Lizenz

[PolyForm Noncommercial License 1.0.0](../../LICENSE.md): kostenlos für jede nichtkommerzielle Nutzung. Für kommerzielle Nutzung frage den Autor nach einer gesonderten Lizenz. Die Lizenzen Dritter stehen in [THIRD_PARTY_NOTICES.md](../../THIRD_PARTY_NOTICES.md). Beiträge: siehe [CONTRIBUTING.md](../../CONTRIBUTING.md).

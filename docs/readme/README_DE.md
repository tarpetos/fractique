<div align="center">
  <img src="../../assets/icon/icon-256.png" alt="Fractique" width="128" height="128">
  <h1>Fractique</h1>
  <p><strong>Ein Generator für Fraktalflammen, der aus derselben Eingabe auf jeder Plattform dasselbe Bild erzeugt.</strong></p>
  <p><sub>
    <a href="../../README.md">English</a> · <a href="README_UK.md">Українська</a> · <a href="README_RU.md">Русский</a> · <strong>Deutsch</strong> · <a href="README_FR.md">Français</a> · <a href="README_ES.md">Español</a> · <a href="README_IT.md">Italiano</a> · <a href="README_JA.md">日本語</a><br>
    <a href="README_CN.md">简体中文</a> · <a href="README_PT.md">Português</a> · <a href="README_KO.md">한국어</a> · <a href="README_HI.md">हिन्दी</a> · <a href="README_TR.md">Türkçe</a> · <a href="README_PL.md">Polski</a> · <a href="README_ID.md">Bahasa Indonesia</a>
  </sub></p>
</div>

Fractique rendert Fraktalflammen: leuchtende, hauchzarte Fraktale aus feinen Kurven und weichem Licht. Du wählst eine Vorlage, würfelst eine Zufallsszene oder legst mehrere Flammen übereinander und renderst das Ergebnis in jeder Auflösung, vom Handybildschirm bis zum 4K-Hintergrundbild. Eine Szene ist eine kleine JSON-Datei, und dieselbe Datei ergibt immer exakt dieselben Pixel, unter Windows, macOS, Linux, Android und iOS.

<p align="center">
  <img src="../samples/aurora-spiral.png" alt="Die Vorlage aurora-spiral: türkisfarbene und grüne Spiralschleier auf dunklem Grund" width="880">
</p>

## Was heute funktioniert

- **Der Renderer** (`fractique-core`): Fraktalflammen mit 49 Variationen, Ebenen mit Mischmodi, Dreh- und Spiegelsymmetrie, Dichteschätzung, Leuchten, jede Auflösung.
- **Die App:** eine Codebasis für Windows, Linux, Android, iOS und das Web, mit Glasflächen über der Flamme, in 15 Sprachen.
- **Vorlagen:** 9 eingebaute Szenen.
- **Zufallsgenerator:** jeder Startwert ist eine Szene; Szenen, die nur ein Punkt oder ein gefülltes Quadrat wären, werden übersprungen.
- **Mutation:** eine Variante einer Szene, die dir gefällt, mit einer Stärke von 0 bis 1.
- **Exakte Wiederholbarkeit:** dieselbe Szene ergibt auf jedem Rechner dieselben Pixel, geprüft durch Tests unter Linux, Windows und macOS ([docs/determinism.md](../determinism.md)).
- **Import:** `.flame`-Dateien von flam3 und Apophysis werden in Szenen umgewandelt.
- **Das Kommandozeilenwerkzeug** (`fractique`): rendert Szenen als PNG.
- **MCP-Server** (`fractique mcp`): ein KI-Agent erstellt, ändert und rendert Szenen ([docs/mcp.md](../mcp.md)).
- **HTTP-Server** (`fractique-server`): eine HTTP-API mit Render-Warteschlange und Grenzen, liefert die Web-App aus, gibt es als Docker-Image ([docs/server.md](../server.md)).

| | |
| --- | --- |
| ![emerald-star](../samples/emerald-star.png) | ![ember-vortex](../samples/ember-vortex.png) |
| `emerald-star` (zwei Ebenen) | `ember-vortex` |

## Einrichtung

Ein Befehl baut alles. Fehlt Rust, wird es installiert.

| System | Befehl |
| --- | --- |
| Linux, macOS | `./setup.sh` |
| Windows | `setup.cmd` |

Danach:

```sh
# eine Vorlage als 4K-Hintergrundbild
./target/release/fractique render --preset aurora-spiral --size 3840x2160 --quality high -o aurora.png

# die Zufallsszene zum Startwert 42, für einen Handybildschirm
./target/release/fractique render --random 42 --size 1080x2400 --quality mobile -o phone.png

# eine Zufallsszene als Datei behalten, ändern, rendern
./target/release/fractique randomize 42 -o scene.json
./target/release/fractique render scene.json -o mine.png
```

Unter Windows liegt das Werkzeug in `target\release\fractique.exe`. Alle Befehle und Optionen: [docs/usage.md](../usage.md).

## Dokumentation

Die Dokumentation ist auf Englisch.

| | |
| --- | --- |
| [docs/usage.md](../usage.md) | Die Befehle, die Qualitätsstufen, wie man eine Szene bearbeitet |
| [docs/scene-format.md](../scene-format.md) | Jedes Feld einer Szenendatei, jede Variation |
| [docs/mcp.md](../mcp.md) | Der MCP-Server: einen KI-Agenten verbinden, die Werkzeuge |
| [docs/server.md](../server.md) | Der HTTP-Server: Endpunkte, Grenzen, Docker |
| [docs/ios.md](../ios.md) | Bauen und Ausführen auf einem iPhone oder im iOS-Simulator |
| [docs/architecture.md](../architecture.md) | Wie die Teile zusammenspielen, wie ein Rendervorgang abläuft |
| [docs/determinism.md](../determinism.md) | Warum sich ein Rendering exakt wiederholt und was zugesagt ist und was nicht |
| [docs/performance.md](../performance.md) | Gemessene Renderzeiten und Speicherbedarf |
| [docs/roadmap.md](../roadmap.md) | Was fertig ist und was als Nächstes kommt |

## Lizenz

[PolyForm Noncommercial License 1.0.0](../../LICENSE.md): kostenlos für jede nichtkommerzielle Nutzung. Für kommerzielle Nutzung bitte beim Autor eine gesonderte Lizenz anfragen. Lizenzen Dritter stehen in [THIRD_PARTY_NOTICES.md](../../THIRD_PARTY_NOTICES.md). Beiträge: siehe [CONTRIBUTING.md](../../CONTRIBUTING.md).

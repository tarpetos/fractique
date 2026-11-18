<div align="center">
  <img src="../../assets/icon/icon-256.png" alt="Fractique" width="128" height="128">
  <h1>Fractique</h1>
  <p><strong>Un generatore di fiamme frattali che dallo stesso input produce la stessa immagine, su ogni piattaforma.</strong></p>
  <p><sub>
    <a href="../../README.md">English</a> · <a href="README_UK.md">Українська</a> · <a href="README_RU.md">Русский</a> · <a href="README_DE.md">Deutsch</a> · <a href="README_FR.md">Français</a> · <a href="README_ES.md">Español</a> · <strong>Italiano</strong> · <a href="README_JA.md">日本語</a><br>
    <a href="README_CN.md">简体中文</a> · <a href="README_PT.md">Português</a> · <a href="README_KO.md">한국어</a> · <a href="README_HI.md">हिन्दी</a> · <a href="README_TR.md">Türkçe</a> · <a href="README_PL.md">Polski</a> · <a href="README_ID.md">Bahasa Indonesia</a>
  </sub></p>
</div>

Fractique disegna fiamme frattali: frattali luminosi e impalpabili fatti di curve sottili e luce morbida. Scegli un preset, generi una scena casuale o sovrapponi più fiamme, e ottieni l’immagine a qualsiasi risoluzione, dallo schermo di un telefono a uno sfondo 4K. Una scena è un piccolo file JSON, e lo stesso file produce sempre esattamente gli stessi pixel, su Windows, macOS, Linux, Android e iOS.

<p align="center">
  <img src="../samples/aurora-spiral.png" alt="Il preset aurora-spiral: volute a spirale turchesi e verdi su sfondo scuro" width="880">
</p>

## Cosa funziona oggi

- **Il renderer** (`fractique-core`): fiamme frattali con 49 variazioni, livelli con metodi di fusione, simmetria rotazionale e speculare, stima della densità, bagliore, qualsiasi risoluzione.
- **L’app:** un unico codice per Windows, Linux, Android, iOS e il web, con pannelli di vetro sopra la fiamma, in 15 lingue.
- **Preset:** 9 scene integrate.
- **Generatore casuale:** ogni seme è una scena; le scene che sarebbero un punto o un quadrato pieno vengono scartate.
- **Mutazione:** una variante di una scena che ti piace, con un’intensità da 0 a 1.
- **Ripetibilità esatta:** la stessa scena dà gli stessi pixel su ogni macchina, verificato da test su Linux, Windows e macOS ([docs/determinism.md](../determinism.md)).
- **Importazione:** i file `.flame` di flam3 e Apophysis vengono convertiti in scene.
- **Lo strumento da riga di comando** (`fractique`): renderizza le scene in PNG.
- **Server MCP** (`fractique mcp`): un agente IA crea, modifica e renderizza scene ([docs/mcp.md](../mcp.md)).

| | |
| --- | --- |
| ![emerald-star](../samples/emerald-star.png) | ![ember-vortex](../samples/ember-vortex.png) |
| `emerald-star` (due livelli) | `ember-vortex` |

## Installazione

Un solo comando compila tutto. Installa Rust se manca.

| Sistema | Comando |
| --- | --- |
| Linux, macOS | `./setup.sh` |
| Windows | `setup.cmd` |

Poi:

```sh
# un preset come sfondo 4K
./target/release/fractique render --preset aurora-spiral --size 3840x2160 --quality high -o aurora.png

# la scena casuale del seme 42, per lo schermo di un telefono
./target/release/fractique render --random 42 --size 1080x2400 --quality mobile -o phone.png

# salvare una scena casuale in un file, modificarla, renderizzarla
./target/release/fractique randomize 42 -o scene.json
./target/release/fractique render scene.json -o mine.png
```

Su Windows lo strumento è in `target\release\fractique.exe`. Tutti i comandi e le opzioni: [docs/usage.md](../usage.md).

## Documentazione

La documentazione è in inglese.

| | |
| --- | --- |
| [docs/usage.md](../usage.md) | I comandi, i livelli di qualità, come modificare una scena |
| [docs/scene-format.md](../scene-format.md) | Ogni campo di un file di scena, ogni variazione |
| [docs/mcp.md](../mcp.md) | Il server MCP: collegare un agente IA, gli strumenti |
| [docs/ios.md](../ios.md) | Compilare ed eseguire su un iPhone o nel simulatore iOS |
| [docs/architecture.md](../architecture.md) | Come si incastrano le parti, come funziona un render |
| [docs/determinism.md](../determinism.md) | Perché un render si ripete esattamente, cosa è garantito e cosa no |
| [docs/performance.md](../performance.md) | Tempi di render e memoria misurati |
| [docs/roadmap.md](../roadmap.md) | Cosa è fatto e cosa viene dopo |

## Licenza

[PolyForm Noncommercial License 1.0.0](../../LICENSE.md): gratuito per qualsiasi uso non commerciale. Per uso commerciale, chiedi all’autore una licenza separata. Le licenze di terze parti sono elencate in [THIRD_PARTY_NOTICES.md](../../THIRD_PARTY_NOTICES.md). Contributi: vedi [CONTRIBUTING.md](../../CONTRIBUTING.md).

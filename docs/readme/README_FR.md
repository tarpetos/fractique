<div align="center">
  <img src="../../assets/icon/icon-256.png" alt="Fractique" width="128" height="128">
  <h1>Fractique</h1>
  <p><strong>Un générateur de flammes fractales qui donne la même image à partir des mêmes données, sur toutes les plateformes.</strong></p>
  <p><sub>
    <a href="../../README.md">English</a> · <a href="README_UK.md">Українська</a> · <a href="README_RU.md">Русский</a> · <a href="README_DE.md">Deutsch</a> · <strong>Français</strong> · <a href="README_ES.md">Español</a> · <a href="README_IT.md">Italiano</a> · <a href="README_JA.md">日本語</a><br>
    <a href="README_CN.md">简体中文</a> · <a href="README_PT.md">Português</a> · <a href="README_KO.md">한국어</a> · <a href="README_HI.md">हिन्दी</a> · <a href="README_TR.md">Türkçe</a> · <a href="README_PL.md">Polski</a> · <a href="README_ID.md">Bahasa Indonesia</a>
  </sub></p>
</div>

Fractique dessine des flammes fractales : des fractales lumineuses et vaporeuses, faites de courbes fines et de lumière douce. Vous choisissez un préréglage, tirez une scène au hasard ou superposez plusieurs flammes, puis vous obtenez l’image dans n’importe quelle résolution, de l’écran d’un téléphone au fond d’écran 4K. Une scène est un petit fichier JSON, et le même fichier donne toujours exactement les mêmes pixels, sous Windows, macOS, Linux, Android et iOS.

<p align="center">
  <img src="../samples/aurora-spiral.png" alt="Le préréglage aurora-spiral : des volutes en spirale turquoise et vertes sur fond sombre" width="880">
</p>

## Ce qui fonctionne aujourd’hui

- **Le moteur de rendu** (`fractique-core`) : flammes fractales avec 28 variations, calques avec modes de fusion, symétrie de rotation et en miroir, estimation de densité, lueur, toute résolution.
- **L’application :** un seul code pour Windows, Linux, Android, iOS et le web, avec des panneaux de verre par-dessus la flamme, en 15 langues.
- **Préréglages :** 9 scènes intégrées.
- **Générateur aléatoire :** chaque graine est une scène ; les scènes qui ne seraient qu’un point ou un carré plein sont écartées.
- **Mutation :** une variante d’une scène qui vous plaît, avec une force de 0 à 1.
- **Répétabilité exacte :** la même scène donne les mêmes pixels sur toutes les machines, ce que vérifient des tests sous Linux, Windows et macOS ([docs/determinism.md](../determinism.md)).
- **Import :** les fichiers `.flame` de flam3 et d’Apophysis sont convertis en scènes.
- **L’outil en ligne de commande** (`fractique`) : rend les scènes en PNG.
- **Serveur MCP** (`fractique mcp`) : un agent IA crée, modifie et rend des scènes ([docs/mcp.md](../mcp.md)).

| | |
| --- | --- |
| ![emerald-star](../samples/emerald-star.png) | ![ember-vortex](../samples/ember-vortex.png) |
| `emerald-star` (deux calques) | `ember-vortex` |

## Installation

Une seule commande compile tout. Elle installe Rust s’il manque.

| Système | Commande |
| --- | --- |
| Linux, macOS | `./setup.sh` |
| Windows | `setup.cmd` |

Ensuite :

```sh
# un préréglage comme fond d’écran 4K
./target/release/fractique render --preset aurora-spiral --size 3840x2160 --quality high -o aurora.png

# la scène aléatoire de la graine 42, pour un écran de téléphone
./target/release/fractique render --random 42 --size 1080x2400 --quality mobile -o phone.png

# garder une scène aléatoire dans un fichier, la modifier, la rendre
./target/release/fractique randomize 42 -o scene.json
./target/release/fractique render scene.json -o mine.png
```

Sous Windows, l’outil se trouve dans `target\release\fractique.exe`. Toutes les commandes et options : [docs/usage.md](../usage.md).

## Documentation

La documentation est en anglais.

| | |
| --- | --- |
| [docs/usage.md](../usage.md) | Les commandes, les niveaux de qualité, comment modifier une scène |
| [docs/scene-format.md](../scene-format.md) | Chaque champ d’un fichier de scène, chaque variation |
| [docs/mcp.md](../mcp.md) | Le serveur MCP : connecter un agent IA, les outils |
| [docs/ios.md](../ios.md) | Compiler et lancer sur un iPhone ou dans le simulateur iOS |
| [docs/architecture.md](../architecture.md) | Comment les parties s’articulent, comment se déroule un rendu |
| [docs/determinism.md](../determinism.md) | Pourquoi un rendu se répète exactement, ce qui est promis et ce qui ne l’est pas |
| [docs/performance.md](../performance.md) | Temps de rendu et mémoire mesurés |
| [docs/roadmap.md](../roadmap.md) | Ce qui est fait et ce qui vient ensuite |

## Licence

[PolyForm Noncommercial License 1.0.0](../../LICENSE.md) : gratuit pour tout usage non commercial. Pour un usage commercial, demandez une licence distincte à l’auteur. Les licences tierces sont listées dans [THIRD_PARTY_NOTICES.md](../../THIRD_PARTY_NOTICES.md). Contributions : voir [CONTRIBUTING.md](../../CONTRIBUTING.md).

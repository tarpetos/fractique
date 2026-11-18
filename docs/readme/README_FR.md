<div align="center">
  <img src="../../assets/icon/icon-256.png" alt="Fractique" width="128" height="128">
  <h1>Fractique</h1>
  <p><strong>Un générateur de flammes fractales et de fonds d’écran qui donne la même image à partir de la même entrée, sur toutes les plateformes.</strong></p>
  <p><sub>
    <a href="../../README.md">English</a> · <a href="README_UK.md">Українська</a> · <a href="README_RU.md">Русский</a> · <a href="README_DE.md">Deutsch</a> · <strong>Français</strong> · <a href="README_ES.md">Español</a> · <a href="README_IT.md">Italiano</a> · <a href="README_JA.md">日本語</a><br>
    <a href="README_CN.md">简体中文</a> · <a href="README_PT.md">Português</a> · <a href="README_KO.md">한국어</a> · <a href="README_HI.md">हिन्दी</a> · <a href="README_TR.md">Türkçe</a> · <a href="README_PL.md">Polski</a> · <a href="README_ID.md">Bahasa Indonesia</a>
  </sub></p>
</div>

Fractique crée deux sortes d’images. Des flammes fractales : ces fractales lumineuses et vaporeuses faites de courbes fines et de lumière douce. Et des fonds d’écran : des feuilles plates aux bords éclairés et aux ombres douces, chacun en version claire et en version sombre. Vous choisissez un préréglage, tirez une image au hasard ou superposez plusieurs flammes, puis vous exportez le résultat à n’importe quelle taille, de l’écran d’un téléphone à un moniteur 4K. Une scène est un seul petit fichier, et le même fichier donne toujours exactement les mêmes pixels, sous Windows, macOS, Linux, Android et iOS.

<p align="center">
  <img src="../samples/aurora-spiral.png" alt="Le préréglage aurora-spiral : des volutes en spirale turquoise et vertes sur fond sombre" width="880">
</p>

## Ce que fait Fractique

- **Flammes :** 49 variations, des calques avec modes de fusion, symétrie de rotation et en miroir, estimation de densité, lueur, et 29 préréglages intégrés.
- **Fonds d’écran :** un pour chaque graine, parmi 14 dispositions et 6 matières, chacun en version claire et en version sombre, disposé pour la taille de l’écran.
- **L’application :** une seule base de code pour Windows, macOS, Linux, Android, iOS et le web. L’image remplit la fenêtre et des panneaux de verre flottent au-dessus ; sur ordinateur, l’application a sa propre barre de titre en verre. En 15 langues.
- **« Aléatoire » et « Muter » :** chaque graine est une image ; les flammes qui ne seraient qu’un point ou un carré plein sont écartées. « Muter » crée une variante de celle qui vous plaît.
- **Calques :** ajoutez une flamme tirée des préréglages ou au hasard, supprimez-la, changez l’ordre, et réglez pour chacune la fusion (« Somme », « Écran », « Dessus ») et l’opacité. Jusqu’à 16 calques.
- **Taille de l’image :** celle de la fenêtre, l’un des 21 écrans (ordinateurs jusqu’à la 4K, téléphones et tablettes Android, iPhone et iPad), ou toute largeur et hauteur de 16 à 16384 pixels.
- **Zoom profond :** le glisser, la molette et le pincement déplacent la vue et l’agrandissent jusqu’à dix milliards de fois. L’image suit aussitôt et est redessinée nette dès que la vue s’immobilise.
- **Téléphones :** un écran à part. Un bouton rond crée une nouvelle image, un balayage passe au fond d’écran suivant, une feuille de petites images vivantes choisit le style, et les images que vous gardez forment une collection.
- **Export :** un PNG de n’importe quelle taille : tout de suite, tel qu’il est à l’écran, ou rendu en arrière-plan en trois niveaux de qualité supérieurs. Sur un téléphone, il est enregistré dans Photos ou dans la galerie, ou partagé.
- **Fichiers de projet :** un fichier `.frcq` garde la scène avec sa vue, son ton et sa taille, si bien qu’elle s’ouvre telle qu’elle a été enregistrée ([docs/scene-format.md](../scene-format.md)). Les scènes `.json` simples et les fichiers `.flame` de flam3 et d’Apophysis s’ouvrent aussi. D’un lancement à l’autre, l’application retrouve la session où elle a été quittée.
- **Mises à jour :** une fenêtre liste chaque version publiée avec ses notes et installe n’importe laquelle. Sous Windows, l’application est un seul `.exe` portable qui se remplace lui-même.
- **Répétabilité exacte :** la même scène donne les mêmes pixels sur toutes les machines, ce que vérifient des tests sous Linux, Windows et macOS ([docs/determinism.md](../determinism.md)).
- **L’outil en ligne de commande** (`fractique`) : fait le rendu des flammes et des fonds d’écran en PNG, crée des scènes aléatoires et des variantes, convertit les fichiers `.flame`.
- **Serveur MCP** (`fractique mcp`) : un agent d’IA crée et modifie des scènes, et en fait le rendu ([docs/mcp.md](../mcp.md)).
- **Serveur HTTP** (`fractique-server`) : une API HTTP avec une file d’attente de rendus et des limites, MCP pour les agents d’IA sur `/mcp`, sert l’application web, livré comme image Docker ([docs/server.md](../server.md)).

| | |
| --- | --- |
| ![emerald-star](../samples/emerald-star.png) | ![ember-vortex](../samples/ember-vortex.png) |
| `emerald-star` (deux calques) | `ember-vortex` |
| ![golden-bloom](../samples/golden-bloom.png) | ![solar-mandala](../samples/solar-mandala.png) |
| `golden-bloom` | `solar-mandala` |

Des fonds d’écran, clairs et sombres, chacun avec sa disposition et sa matière :

| | | | |
| --- | --- | --- | --- |
| ![waves](../samples/wallpaper-waves-light.png) | ![burst](../samples/wallpaper-burst-light.png) | ![tiles](../samples/wallpaper-tiles-light.png) | ![fan](../samples/wallpaper-fan-light.png) |
| `waves`, `glass` | `burst`, `glow` | `tiles`, `relief` | `fan`, `glass` |
| ![discs](../samples/wallpaper-discs-dark.png) | ![bars](../samples/wallpaper-bars-dark.png) | ![arcs](../samples/wallpaper-arcs-dark.png) | ![lines](../samples/wallpaper-lines-dark.png) |
| `discs`, `paper` | `bars`, `paper` | `arcs`, `carbon` | `lines`, `paper` |

## Démarrage rapide

Une seule commande compile l’application et l’outil en ligne de commande. Elle installe Rust s’il est absent.

| Système | Commande |
| --- | --- |
| Linux, macOS | `./setup.sh` |
| Windows | `setup.cmd` |

Lancez l’application :

```sh
./target/release/fractique-gui
```

Ou faites un rendu en ligne de commande :

```sh
# un préréglage en image 4K
./target/release/fractique render --preset aurora-spiral --size 3840x2160 --quality high -o aurora.png

# la flamme aléatoire de graine 42, pour un écran de téléphone
./target/release/fractique render --random 42 --size 1080x2400 --quality mobile -o phone.png

# le fond d’écran de graine 7 pour un téléphone, clair et sombre
./target/release/fractique render --wallpaper 7 --size 1290x2796 -o wallpaper.png
./target/release/fractique render --wallpaper 7 --dark --size 1290x2796 -o wallpaper-dark.png

# garder une scène aléatoire dans un fichier, la modifier, en faire le rendu
./target/release/fractique randomize 42 -o scene.json
./target/release/fractique render scene.json -o mine.png
```

Sous Windows, les programmes sont `target\release\fractique-gui.exe` et `target\release\fractique.exe`. Tout ce que font l’application et les commandes : [docs/usage.md](../usage.md).

## Documentation

La documentation est en anglais.

| | |
| --- | --- |
| [docs/usage.md](../usage.md) | L’application, les commandes, les niveaux de qualité, comment modifier une scène |
| [docs/scene-format.md](../scene-format.md) | Chaque champ d’un fichier de scène, chaque variation, les feuilles, le fichier `.frcq` |
| [docs/mcp.md](../mcp.md) | Le serveur MCP : connecter un agent d’IA, les outils |
| [docs/server.md](../server.md) | Le serveur HTTP : points d’accès, limites, Docker |
| [docs/ios.md](../ios.md) | Compiler et lancer sur un iPhone ou dans le simulateur iOS |
| [docs/releasing.md](../releasing.md) | Comment une version est publiée et comment l’application se met à jour |
| [docs/architecture.md](../architecture.md) | Comment les parties s’assemblent, comment fonctionne un rendu |
| [docs/determinism.md](../determinism.md) | Pourquoi un rendu se répète exactement, ce qui est promis et ce qui ne l’est pas |
| [docs/performance.md](../performance.md) | Temps de rendu et mémoire mesurés |
| [docs/roadmap.md](../roadmap.md) | Ce qui est fait et ce qui vient ensuite |

## Licence

[PolyForm Noncommercial License 1.0.0](../../LICENSE.md) : gratuit pour tout usage non commercial. Pour un usage commercial, demandez une licence distincte à l’auteur. Les licences tierces sont listées dans [THIRD_PARTY_NOTICES.md](../../THIRD_PARTY_NOTICES.md). Contributions : voir [CONTRIBUTING.md](../../CONTRIBUTING.md).

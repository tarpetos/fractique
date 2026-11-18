<div align="center">
  <img src="../../assets/icon/icon-256.png" alt="Fractique" width="128" height="128">
  <h1>Fractique</h1>
  <p><strong>Un generador de llamas fractales y fondos de pantalla que produce la misma imagen a partir de la misma entrada, en todas las plataformas.</strong></p>
  <p><sub>
    <a href="../../README.md">English</a> · <a href="README_UK.md">Українська</a> · <a href="README_RU.md">Русский</a> · <a href="README_DE.md">Deutsch</a> · <a href="README_FR.md">Français</a> · <strong>Español</strong> · <a href="README_IT.md">Italiano</a> · <a href="README_JA.md">日本語</a><br>
    <a href="README_CN.md">简体中文</a> · <a href="README_PT.md">Português</a> · <a href="README_KO.md">한국어</a> · <a href="README_HI.md">हिन्दी</a> · <a href="README_TR.md">Türkçe</a> · <a href="README_PL.md">Polski</a> · <a href="README_ID.md">Bahasa Indonesia</a>
  </sub></p>
</div>

Fractique crea tres tipos de imágenes. Llamas fractales: esos fractales luminosos y etéreos hechos de curvas finas y luz suave. Fractales clásicos: el conjunto de Mandelbrot, los conjuntos de Julia y atractores extraños, con colores suaves y zoom profundo. Y fondos de pantalla: láminas planas con bordes iluminados y sombras suaves, cada uno en versión clara y oscura. Eliges un preajuste, generas una imagen al azar o superpones varias llamas, y exportas el resultado a cualquier tamaño, desde la pantalla de un teléfono hasta un monitor 4K. Una escena es un único archivo pequeño, y el mismo archivo siempre da exactamente los mismos píxeles, en Windows, macOS, Linux, Android e iOS.

<p align="center">
  <img src="../samples/aurora-spiral.png" alt="El preajuste aurora-spiral: volutas en espiral turquesas y verdes sobre un fondo oscuro" width="880">
</p>

## Qué hace Fractique

- **Llamas:** 49 variaciones, capas con modos de fusión, simetría rotacional y de espejo, estimación de densidad, resplandor y 29 preajustes integrados.
- **Fractales:** el conjunto de Mandelbrot, los conjuntos de Julia y el atractor de Clifford, cada uno con preajustes y una imagen para cada semilla; comparten las paletas, las capas y el zoom profundo de las llamas.
- **Fondos de pantalla:** uno para cada semilla, a partir de 14 diseños y 6 materiales, cada uno en versión clara y oscura, dispuesto para el tamaño de la pantalla.
- **La aplicación:** una sola base de código para Windows, macOS, Linux, Android, iOS y la web. La imagen llena la ventana y sobre ella flotan paneles de vidrio; en el escritorio la aplicación tiene su propia barra de título de vidrio. En 15 idiomas.
- **«Aleatorio» y «Mutar»:** cada semilla es una imagen; las llamas que serían un punto o un cuadrado relleno se descartan. «Mutar» crea una variante de la que te gusta.
- **Capas:** añade una llama de los preajustes o una aleatoria, quítala, cambia el orden y ajusta la fusión («Suma», «Trama», «Encima») y la opacidad de cada una. Hasta 16 capas.
- **Tamaño de la imagen:** el de la ventana, una de 59 pantallas (ordenadores hasta 4K, teléfonos y tabletas Android, iPhone y iPad) o cualquier ancho y alto de 16 a 16384 píxeles.
- **Zoom profundo:** arrastrar, la rueda y el pellizco mueven la vista y la amplían hasta diez mil millones de veces. La imagen sigue al instante y se vuelve a dibujar nítida cuando la vista se detiene.
- **Teléfonos:** una pantalla propia. Un botón redondo crea una imagen nueva, un deslizamiento pasa al siguiente fondo de pantalla, una hoja de pequeñas imágenes vivas elige el estilo y las imágenes que conservas forman una colección.
- **Exportación:** un PNG de cualquier tamaño: al instante, tal como está en pantalla, o renderizado en segundo plano en tres niveles de calidad superiores. En un teléfono se guarda en Fotos o en la galería, o se comparte. Todas las funciones son gratuitas en todas las plataformas; solo la versión del App Store (iPhone, iPad, Mac) pide una compra única, Fractique Pro, para exportar a 2560 × 1440 píxeles o más o en un tamaño personalizado.
- **Archivos de proyecto:** un archivo `.frcq` guarda la escena con su vista, tono y tamaño, así que se abre tal como se guardó ([docs/scene-format.md](../scene-format.md)). También se abren escenas `.json` sencillas y archivos `.flame` de flam3 y Apophysis. Entre un inicio y otro, la aplicación vuelve a la sesión en la que se dejó.
- **Actualizaciones:** una ventana muestra cada versión publicada con sus notas e instala cualquiera de ellas. En Windows la aplicación es un único `.exe` portátil que se reemplaza a sí mismo.
- **Repetibilidad exacta:** la misma escena da los mismos píxeles en cualquier máquina, comprobado con pruebas en Linux, Windows y macOS ([docs/determinism.md](../determinism.md)).
- **La herramienta de línea de comandos** (`fractique`): renderiza llamas y fondos de pantalla a PNG, crea escenas aleatorias y variantes, convierte archivos `.flame`.
- **Servidor MCP** (`fractique mcp`): un agente de IA crea, modifica y renderiza escenas ([docs/mcp.md](../mcp.md)).
- **Servidor HTTP** (`fractique-server`): una API HTTP con cola de renderizado y límites, MCP para agentes de IA en `/mcp`, sirve la aplicación web, se distribuye como imagen de Docker ([docs/server.md](../server.md)).

| | |
| --- | --- |
| ![emerald-star](../samples/emerald-star.png) | ![ember-vortex](../samples/ember-vortex.png) |
| `emerald-star` (dos capas) | `ember-vortex` |
| ![golden-bloom](../samples/golden-bloom.png) | ![solar-mandala](../samples/solar-mandala.png) |
| `golden-bloom` | `solar-mandala` |

Fractales de otros tipos: el conjunto de Mandelbrot, un conjunto de Julia y el atractor de Clifford:

| | | |
| --- | --- | --- |
| ![mandelbrot](../samples/fractal-mandelbrot.png) | ![julia](../samples/fractal-julia.png) | ![clifford](../samples/fractal-clifford.png) |
| `mandelbrot-spiral` | `julia-spirals` | `clifford-wings` |

Fondos de pantalla, claros y oscuros, cada uno con su diseño y su material:

| | | | |
| --- | --- | --- | --- |
| ![waves](../samples/wallpaper-waves-light.png) | ![burst](../samples/wallpaper-burst-light.png) | ![tiles](../samples/wallpaper-tiles-light.png) | ![fan](../samples/wallpaper-fan-light.png) |
| `waves`, `glass` | `burst`, `glow` | `tiles`, `relief` | `fan`, `glass` |
| ![discs](../samples/wallpaper-discs-dark.png) | ![bars](../samples/wallpaper-bars-dark.png) | ![arcs](../samples/wallpaper-arcs-dark.png) | ![lines](../samples/wallpaper-lines-dark.png) |
| `discs`, `paper` | `bars`, `paper` | `arcs`, `carbon` | `lines`, `paper` |

## Inicio rápido

Un solo comando compila la aplicación y la herramienta de línea de comandos. Instala Rust si falta.

| Sistema | Comando |
| --- | --- |
| Linux, macOS | `./setup.sh` |
| Windows | `setup.cmd` |

Inicia la aplicación:

```sh
./target/release/fractique-gui
```

O renderiza desde la línea de comandos:

```sh
# un preajuste como imagen 4K
./target/release/fractique render --preset aurora-spiral --size 3840x2160 --quality high -o aurora.png

# la llama aleatoria de la semilla 42, para la pantalla de un teléfono
./target/release/fractique render --random 42 --size 1080x2400 --quality mobile -o phone.png

# la vista aleatoria del conjunto de Mandelbrot de la semilla 7
./target/release/fractique render --random 7 --fractal mandelbrot --size 1920x1080 -o mandelbrot.png

# el fondo de pantalla de la semilla 7 para un teléfono, claro y oscuro
./target/release/fractique render --wallpaper 7 --size 1290x2796 -o wallpaper.png
./target/release/fractique render --wallpaper 7 --dark --size 1290x2796 -o wallpaper-dark.png

# guardar una escena aleatoria en un archivo, cambiarla, renderizarla
./target/release/fractique randomize 42 -o scene.json
./target/release/fractique render scene.json -o mine.png
```

En Windows los programas son `target\release\fractique-gui.exe` y `target\release\fractique.exe`. Todo lo que hacen la aplicación y los comandos: [docs/usage.md](../usage.md).

## Documentación

La documentación está en inglés.

| | |
| --- | --- |
| [docs/usage.md](../usage.md) | La aplicación, los comandos, los niveles de calidad, cómo editar una escena |
| [docs/scene-format.md](../scene-format.md) | Cada campo de un archivo de escena, cada variación, las láminas, el archivo `.frcq` |
| [docs/mcp.md](../mcp.md) | El servidor MCP: conectar un agente de IA, las herramientas |
| [docs/server.md](../server.md) | El servidor HTTP: endpoints, límites, Docker |
| [docs/ios.md](../ios.md) | Compilar y ejecutar en un iPhone o en el simulador de iOS |
| [docs/releasing.md](../releasing.md) | Cómo se publica una versión y cómo se actualiza la aplicación |
| [docs/architecture.md](../architecture.md) | Cómo encajan las partes, cómo funciona un renderizado |
| [docs/determinism.md](../determinism.md) | Por qué un renderizado se repite exactamente, qué se promete y qué no |
| [docs/performance.md](../performance.md) | Tiempos de renderizado y memoria medidos |
| [docs/roadmap.md](../roadmap.md) | Qué está hecho y qué viene después |

## Licencia

[PolyForm Noncommercial License 1.0.0](../../LICENSE.md): gratis para cualquier uso no comercial. Para uso comercial, pide al autor una licencia aparte. Las licencias de terceros figuran en [THIRD_PARTY_NOTICES.md](../../THIRD_PARTY_NOTICES.md). Contribuciones: consulta [CONTRIBUTING.md](../../CONTRIBUTING.md).

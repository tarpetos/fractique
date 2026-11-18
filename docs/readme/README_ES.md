<div align="center">
  <img src="../../assets/icon/icon-256.png" alt="Fractique" width="128" height="128">
  <h1>Fractique</h1>
  <p><strong>Un generador de llamas fractales que produce la misma imagen a partir de la misma entrada, en cualquier plataforma.</strong></p>
  <p><sub>
    <a href="../../README.md">English</a> · <a href="README_UK.md">Українська</a> · <a href="README_RU.md">Русский</a> · <a href="README_DE.md">Deutsch</a> · <a href="README_FR.md">Français</a> · <strong>Español</strong> · <a href="README_IT.md">Italiano</a> · <a href="README_JA.md">日本語</a><br>
    <a href="README_CN.md">简体中文</a> · <a href="README_PT.md">Português</a> · <a href="README_KO.md">한국어</a> · <a href="README_HI.md">हिन्दी</a> · <a href="README_TR.md">Türkçe</a> · <a href="README_PL.md">Polski</a> · <a href="README_ID.md">Bahasa Indonesia</a>
  </sub></p>
</div>

Fractique dibuja llamas fractales: fractales luminosos y etéreos hechos de curvas finas y luz suave. Eliges un preajuste, generas una escena al azar o superpones varias llamas, y obtienes la imagen en cualquier resolución, desde la pantalla de un teléfono hasta un fondo de pantalla 4K. Una escena es un pequeño archivo JSON, y el mismo archivo produce siempre exactamente los mismos píxeles en Windows, macOS, Linux, Android e iOS.

<p align="center">
  <img src="../samples/aurora-spiral.png" alt="El preajuste aurora-spiral: volutas en espiral turquesas y verdes sobre fondo oscuro" width="880">
</p>

## Qué funciona hoy

- **El motor de render** (`fractique-core`): llamas fractales con 49 variaciones, capas con modos de fusión, simetría rotacional y de espejo, estimación de densidad, resplandor, cualquier resolución.
- **La aplicación:** un solo código para Windows, Linux, Android, iOS y la web, con paneles de vidrio sobre la llama, en 15 idiomas.
- **Preajustes:** 9 escenas integradas.
- **Generador aleatorio:** cada semilla es una escena; las escenas que serían un punto o un cuadrado relleno se descartan.
- **Mutación:** una variante de una escena que te gusta, con una fuerza de 0 a 1.
- **Repetibilidad exacta:** la misma escena da los mismos píxeles en cualquier máquina, comprobado con pruebas en Linux, Windows y macOS ([docs/determinism.md](../determinism.md)).
- **Importación:** los archivos `.flame` de flam3 y Apophysis se convierten en escenas.
- **La herramienta de línea de comandos** (`fractique`): renderiza escenas a PNG.
- **Servidor MCP** (`fractique mcp`): un agente de IA crea, modifica y renderiza escenas ([docs/mcp.md](../mcp.md)).

| | |
| --- | --- |
| ![emerald-star](../samples/emerald-star.png) | ![ember-vortex](../samples/ember-vortex.png) |
| `emerald-star` (dos capas) | `ember-vortex` |

## Instalación

Un solo comando lo compila todo. Instala Rust si falta.

| Sistema | Comando |
| --- | --- |
| Linux, macOS | `./setup.sh` |
| Windows | `setup.cmd` |

Después:

```sh
# un preajuste como fondo de pantalla 4K
./target/release/fractique render --preset aurora-spiral --size 3840x2160 --quality high -o aurora.png

# la escena aleatoria de la semilla 42, para la pantalla de un teléfono
./target/release/fractique render --random 42 --size 1080x2400 --quality mobile -o phone.png

# guardar una escena aleatoria en un archivo, modificarla, renderizarla
./target/release/fractique randomize 42 -o scene.json
./target/release/fractique render scene.json -o mine.png
```

En Windows la herramienta está en `target\release\fractique.exe`. Todos los comandos y opciones: [docs/usage.md](../usage.md).

## Documentación

La documentación está en inglés.

| | |
| --- | --- |
| [docs/usage.md](../usage.md) | Los comandos, los niveles de calidad, cómo editar una escena |
| [docs/scene-format.md](../scene-format.md) | Cada campo de un archivo de escena, cada variación |
| [docs/mcp.md](../mcp.md) | El servidor MCP: conectar un agente de IA, las herramientas |
| [docs/ios.md](../ios.md) | Compilar y ejecutar en un iPhone o en el simulador de iOS |
| [docs/architecture.md](../architecture.md) | Cómo encajan las partes, cómo funciona un render |
| [docs/determinism.md](../determinism.md) | Por qué un render se repite con exactitud, y qué se promete y qué no |
| [docs/performance.md](../performance.md) | Tiempos de render y memoria medidos |
| [docs/roadmap.md](../roadmap.md) | Qué está hecho y qué viene después |

## Licencia

[PolyForm Noncommercial License 1.0.0](../../LICENSE.md): gratis para cualquier uso no comercial. Para uso comercial, pide al autor una licencia aparte. Las licencias de terceros figuran en [THIRD_PARTY_NOTICES.md](../../THIRD_PARTY_NOTICES.md). Contribuciones: consulta [CONTRIBUTING.md](../../CONTRIBUTING.md).

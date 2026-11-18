<div align="center">
  <img src="../../assets/icon/icon-256.png" alt="Fractique" width="128" height="128">
  <h1>Fractique</h1>
  <p><strong>Генератор фрактального пламени, который из одинаковых входных данных даёт одинаковое изображение на любой платформе.</strong></p>
  <p><sub>
    <a href="../../README.md">English</a> · <a href="README_UK.md">Українська</a> · <strong>Русский</strong> · <a href="README_DE.md">Deutsch</a> · <a href="README_FR.md">Français</a> · <a href="README_ES.md">Español</a> · <a href="README_IT.md">Italiano</a> · <a href="README_JA.md">日本語</a><br>
    <a href="README_CN.md">简体中文</a> · <a href="README_PT.md">Português</a> · <a href="README_KO.md">한국어</a> · <a href="README_HI.md">हिन्दी</a> · <a href="README_TR.md">Türkçe</a> · <a href="README_PL.md">Polski</a> · <a href="README_ID.md">Bahasa Indonesia</a>
  </sub></p>
</div>

Fractique рисует фрактальное пламя: светящиеся, лёгкие фракталы из тонких кривых и мягкого света. Вы выбираете пресет, генерируете случайную сцену или накладываете несколько пламён друг на друга и получаете изображение любого разрешения, от экрана телефона до обоев 4K. Сцена — это один небольшой файл JSON, и один и тот же файл всегда даёт в точности те же пиксели на Windows, macOS, Linux, Android и iOS.

<p align="center">
  <img src="../samples/aurora-spiral.png" alt="Пресет aurora-spiral: бирюзовые и зелёные спиральные пряди на тёмном фоне" width="880">
</p>

## Что работает уже сейчас

- **Рендерер** (`fractique-core`): фрактальное пламя с 49 вариациями, слои с режимами смешивания, поворотная и зеркальная симметрия, оценка плотности, свечение, любое разрешение.
- **Приложение:** одна кодовая база для Windows, Linux, Android, iOS и веба, со стеклянными панелями поверх пламени, на 15 языках.
- **Пресеты:** 9 встроенных сцен.
- **Рандомизатор:** каждое зерно — это сцена; сцены, которые были бы точкой или сплошным квадратом, пропускаются.
- **Мутация:** вариант понравившейся сцены с силой от 0 до 1.
- **Точная повторяемость:** одна и та же сцена даёт одни и те же пиксели на любой машине; это проверяют тесты на Linux, Windows и macOS ([docs/determinism.md](../determinism.md)).
- **Импорт:** файлы `.flame` из flam3 и Apophysis преобразуются в сцены.
- **Инструмент командной строки** (`fractique`): рендерит сцены в PNG.
- **Сервер MCP** (`fractique mcp`): ИИ-агент создаёт, изменяет и рендерит сцены ([docs/mcp.md](../mcp.md)).

| | |
| --- | --- |
| ![emerald-star](../samples/emerald-star.png) | ![ember-vortex](../samples/ember-vortex.png) |
| `emerald-star` (два слоя) | `ember-vortex` |

## Установка

Одна команда собирает всё. Если Rust не установлен, она его установит.

| Система | Команда |
| --- | --- |
| Linux, macOS | `./setup.sh` |
| Windows | `setup.cmd` |

Затем:

```sh
# пресет как обои 4K
./target/release/fractique render --preset aurora-spiral --size 3840x2160 --quality high -o aurora.png

# случайная сцена с зерном 42 для экрана телефона
./target/release/fractique render --random 42 --size 1080x2400 --quality mobile -o phone.png

# сохранить случайную сцену в файл, изменить её, отрендерить
./target/release/fractique randomize 42 -o scene.json
./target/release/fractique render scene.json -o mine.png
```

В Windows инструмент находится в `target\release\fractique.exe`. Все команды и параметры: [docs/usage.md](../usage.md).

## Документация

Документация написана на английском.

| | |
| --- | --- |
| [docs/usage.md](../usage.md) | Команды, уровни качества, как редактировать сцену |
| [docs/scene-format.md](../scene-format.md) | Каждое поле файла сцены, каждая вариация |
| [docs/mcp.md](../mcp.md) | Сервер MCP: подключение ИИ-агента, инструменты |
| [docs/ios.md](../ios.md) | Сборка и запуск на iPhone или в симуляторе iOS |
| [docs/architecture.md](../architecture.md) | Как части связаны между собой, как работает рендер |
| [docs/determinism.md](../determinism.md) | Почему рендер повторяется точно, что гарантируется, а что нет |
| [docs/performance.md](../performance.md) | Измеренное время рендера и объём памяти |
| [docs/roadmap.md](../roadmap.md) | Что сделано и что дальше |

## Лицензия

[PolyForm Noncommercial License 1.0.0](../../LICENSE.md): бесплатно для любого некоммерческого использования. Для коммерческого использования обратитесь к автору за отдельной лицензией. Лицензии сторонних компонентов перечислены в [THIRD_PARTY_NOTICES.md](../../THIRD_PARTY_NOTICES.md). Вклад в проект: см. [CONTRIBUTING.md](../../CONTRIBUTING.md).

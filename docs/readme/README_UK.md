<div align="center">
  <img src="../../assets/icon/icon-256.png" alt="Fractique" width="128" height="128">
  <h1>Fractique</h1>
  <p><strong>Генератор фрактального полум’я, який з однакових вхідних даних дає однакове зображення на будь-якій платформі.</strong></p>
  <p><sub>
    <a href="../../README.md">English</a> · <strong>Українська</strong> · <a href="README_RU.md">Русский</a> · <a href="README_DE.md">Deutsch</a> · <a href="README_FR.md">Français</a> · <a href="README_ES.md">Español</a> · <a href="README_IT.md">Italiano</a> · <a href="README_JA.md">日本語</a><br>
    <a href="README_CN.md">简体中文</a> · <a href="README_PT.md">Português</a> · <a href="README_KO.md">한국어</a> · <a href="README_HI.md">हिन्दी</a> · <a href="README_TR.md">Türkçe</a> · <a href="README_PL.md">Polski</a> · <a href="README_ID.md">Bahasa Indonesia</a>
  </sub></p>
</div>

Fractique малює фрактальне полум’я: сяйливі, легкі фрактали з тонких кривих і м’якого світла. Ви обираєте пресет, генеруєте випадкову сцену або накладаєте кілька полум’їв одне на одне й отримуєте зображення будь-якої роздільності, від екрана телефона до шпалер 4K. Сцена — це один невеликий файл JSON, і той самий файл завжди дає точно ті самі пікселі на Windows, macOS, Linux, Android та iOS.

<p align="center">
  <img src="../samples/aurora-spiral.png" alt="Пресет aurora-spiral: бірюзові й зелені спіральні пасма на темному тлі" width="880">
</p>

## Що працює вже зараз

- **Рендерер** (`fractique-core`): фрактальне полум’я з 28 варіаціями, шари з режимами змішування, поворотна й дзеркальна симетрія, оцінка щільності, сяйво, будь-яка роздільність.
- **Застосунок:** одна кодова база для Windows, Linux, Android, iOS та вебу, зі скляними панелями поверх полум’я, 15 мовами.
- **Пресети:** 9 вбудованих сцен.
- **Рандомізатор:** кожне зерно — це сцена; сцени, що були б крапкою або суцільним квадратом, пропускаються.
- **Мутація:** варіант сцени, яка вам сподобалась, із силою від 0 до 1.
- **Точна повторюваність:** та сама сцена дає ті самі пікселі на будь-якій машині; це перевіряють тести на Linux, Windows і macOS ([docs/determinism.md](../determinism.md)).
- **Імпорт:** файли `.flame` з flam3 та Apophysis перетворюються на сцени.
- **Інструмент командного рядка** (`fractique`): рендерить сцени у PNG.
- **Сервер MCP** (`fractique mcp`): ШІ-агент створює, змінює й рендерить сцени ([docs/mcp.md](../mcp.md)).
- **HTTP-сервер** (`fractique-server`): HTTP API з чергою рендерів і лімітами, роздає вебзастосунок, постачається як образ Docker ([docs/server.md](../server.md)).

| | |
| --- | --- |
| ![emerald-star](../samples/emerald-star.png) | ![ember-vortex](../samples/ember-vortex.png) |
| `emerald-star` (два шари) | `ember-vortex` |

## Встановлення

Одна команда збирає все. Якщо Rust не встановлено, вона його встановить.

| Система | Команда |
| --- | --- |
| Linux, macOS | `./setup.sh` |
| Windows | `setup.cmd` |

Далі:

```sh
# пресет як шпалери 4K
./target/release/fractique render --preset aurora-spiral --size 3840x2160 --quality high -o aurora.png

# випадкова сцена із зерном 42 для екрана телефона
./target/release/fractique render --random 42 --size 1080x2400 --quality mobile -o phone.png

# зберегти випадкову сцену у файл, змінити її, відрендерити
./target/release/fractique randomize 42 -o scene.json
./target/release/fractique render scene.json -o mine.png
```

У Windows інструмент лежить у `target\release\fractique.exe`. Усі команди й параметри: [docs/usage.md](../usage.md).

## Документація

Документація написана англійською.

| | |
| --- | --- |
| [docs/usage.md](../usage.md) | Команди, рівні якості, як редагувати сцену |
| [docs/scene-format.md](../scene-format.md) | Кожне поле файлу сцени, кожна варіація |
| [docs/mcp.md](../mcp.md) | Сервер MCP: підключення ШІ-агента, інструменти |
| [docs/server.md](../server.md) | HTTP-сервер: ендпоінти, ліміти, Docker |
| [docs/ios.md](../ios.md) | Збирання й запуск на iPhone або в симуляторі iOS |
| [docs/architecture.md](../architecture.md) | Як частини поєднані між собою, як працює рендер |
| [docs/determinism.md](../determinism.md) | Чому рендер повторюється точно, що гарантовано, а що ні |
| [docs/performance.md](../performance.md) | Виміряний час рендеру й обсяг пам’яті |
| [docs/roadmap.md](../roadmap.md) | Що зроблено і що далі |

## Ліцензія

[PolyForm Noncommercial License 1.0.0](../../LICENSE.md): безкоштовно для будь-якого некомерційного використання. Для комерційного використання зверніться до автора по окрему ліцензію. Ліцензії сторонніх компонентів перелічено в [THIRD_PARTY_NOTICES.md](../../THIRD_PARTY_NOTICES.md). Внески: див. [CONTRIBUTING.md](../../CONTRIBUTING.md).

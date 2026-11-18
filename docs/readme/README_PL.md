<div align="center">
  <img src="../../assets/icon/icon-256.png" alt="Fractique" width="128" height="128">
  <h1>Fractique</h1>
  <p><strong>Generator płomieni fraktalnych, który z tych samych danych wejściowych daje ten sam obraz na każdej platformie.</strong></p>
  <p><sub>
    <a href="../../README.md">English</a> · <a href="README_UK.md">Українська</a> · <a href="README_RU.md">Русский</a> · <a href="README_DE.md">Deutsch</a> · <a href="README_FR.md">Français</a> · <a href="README_ES.md">Español</a> · <a href="README_IT.md">Italiano</a> · <a href="README_JA.md">日本語</a><br>
    <a href="README_CN.md">简体中文</a> · <a href="README_PT.md">Português</a> · <a href="README_KO.md">한국어</a> · <a href="README_HI.md">हिन्दी</a> · <a href="README_TR.md">Türkçe</a> · <strong>Polski</strong> · <a href="README_ID.md">Bahasa Indonesia</a>
  </sub></p>
</div>

Fractique rysuje płomienie fraktalne: świetliste, zwiewne fraktale z cienkich krzywych i miękkiego światła. Wybierasz preset, losujesz scenę albo nakładasz na siebie kilka płomieni i renderujesz w dowolnej rozdzielczości, od ekranu telefonu po tapetę 4K. Scena to jeden mały plik JSON, a ten sam plik zawsze daje dokładnie te same piksele w systemach Windows, macOS, Linux, Android i iOS.

<p align="center">
  <img src="../samples/aurora-spiral.png" alt="Preset aurora-spiral: turkusowe i zielone spiralne smugi na ciemnym tle" width="880">
</p>

## Co działa już dziś

- **Renderer** (`fractique-core`): płomienie fraktalne z 49 wariacjami, warstwy z trybami mieszania, symetria obrotowa i lustrzana, estymacja gęstości, poświata, dowolna rozdzielczość.
- **Aplikacja:** jedna baza kodu dla systemów Windows, Linux, Android, iOS i dla przeglądarki, ze szklanymi panelami nad płomieniem, w 15 językach.
- **Presety:** 9 wbudowanych scen.
- **Generator losowy:** każde ziarno to scena; sceny, które byłyby punktem albo wypełnionym kwadratem, są pomijane.
- **Mutacja:** wariant sceny, która ci się podoba, o sile od 0 do 1.
- **Dokładna powtarzalność:** ta sama scena daje te same piksele na każdej maszynie, co sprawdzają testy w systemach Linux, Windows i macOS ([docs/determinism.md](../determinism.md)).
- **Import:** pliki `.flame` z flam3 i Apophysis są zamieniane na sceny.
- **Narzędzie wiersza poleceń** (`fractique`): renderuje sceny do PNG.
- **Serwer MCP** (`fractique mcp`): agent AI tworzy, zmienia i renderuje sceny ([docs/mcp.md](../mcp.md)).
- **Serwer HTTP** (`fractique-server`): API HTTP z kolejką renderowania i limitami, udostępnia aplikację webową, dostępny jako obraz Dockera ([docs/server.md](../server.md)).

| | |
| --- | --- |
| ![emerald-star](../samples/emerald-star.png) | ![ember-vortex](../samples/ember-vortex.png) |
| `emerald-star` (dwie warstwy) | `ember-vortex` |

## Instalacja

Jedno polecenie buduje wszystko. Jeśli brakuje Rusta, zostanie zainstalowany.

| System | Polecenie |
| --- | --- |
| Linux, macOS | `./setup.sh` |
| Windows | `setup.cmd` |

Następnie:

```sh
# preset jako tapeta 4K
./target/release/fractique render --preset aurora-spiral --size 3840x2160 --quality high -o aurora.png

# losowa scena dla ziarna 42, na ekran telefonu
./target/release/fractique render --random 42 --size 1080x2400 --quality mobile -o phone.png

# zapisz losową scenę do pliku, zmień ją, wyrenderuj
./target/release/fractique randomize 42 -o scene.json
./target/release/fractique render scene.json -o mine.png
```

W systemie Windows narzędzie znajduje się w `target\release\fractique.exe`. Wszystkie polecenia i opcje: [docs/usage.md](../usage.md).

## Dokumentacja

Dokumentacja jest po angielsku.

| | |
| --- | --- |
| [docs/usage.md](../usage.md) | Polecenia, poziomy jakości, jak edytować scenę |
| [docs/scene-format.md](../scene-format.md) | Każde pole pliku sceny, każda wariacja |
| [docs/mcp.md](../mcp.md) | Serwer MCP: podłączanie agenta AI, narzędzia |
| [docs/server.md](../server.md) | Serwer HTTP: punkty końcowe, limity, Docker |
| [docs/ios.md](../ios.md) | Budowanie i uruchamianie na iPhonie lub w symulatorze iOS |
| [docs/architecture.md](../architecture.md) | Jak części do siebie pasują, jak przebiega renderowanie |
| [docs/determinism.md](../determinism.md) | Dlaczego render powtarza się dokładnie, co jest gwarantowane, a co nie |
| [docs/performance.md](../performance.md) | Zmierzone czasy renderowania i zużycie pamięci |
| [docs/roadmap.md](../roadmap.md) | Co jest gotowe i co będzie dalej |

## Licencja

[PolyForm Noncommercial License 1.0.0](../../LICENSE.md): za darmo do każdego niekomercyjnego użytku. W sprawie użytku komercyjnego poproś autora o osobną licencję. Licencje stron trzecich są wymienione w [THIRD_PARTY_NOTICES.md](../../THIRD_PARTY_NOTICES.md). Współtworzenie: zobacz [CONTRIBUTING.md](../../CONTRIBUTING.md).

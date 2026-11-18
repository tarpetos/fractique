<div align="center">
  <img src="../../assets/icon/icon-256.png" alt="Fractique" width="128" height="128">
  <h1>Fractique</h1>
  <p><strong>Um gerador de chamas fractais e papéis de parede que produz a mesma imagem a partir da mesma entrada, em qualquer plataforma.</strong></p>
  <p><sub>
    <a href="../../README.md">English</a> · <a href="README_UK.md">Українська</a> · <a href="README_RU.md">Русский</a> · <a href="README_DE.md">Deutsch</a> · <a href="README_FR.md">Français</a> · <a href="README_ES.md">Español</a> · <a href="README_IT.md">Italiano</a> · <a href="README_JA.md">日本語</a><br>
    <a href="README_CN.md">简体中文</a> · <strong>Português</strong> · <a href="README_KO.md">한국어</a> · <a href="README_HI.md">हिन्दी</a> · <a href="README_TR.md">Türkçe</a> · <a href="README_PL.md">Polski</a> · <a href="README_ID.md">Bahasa Indonesia</a>
  </sub></p>
</div>

O Fractique cria dois tipos de imagem. Chamas fractais: fractais luminosos e etéreos feitos de curvas finas e luz suave. E papéis de parede: folhas planas com bordas iluminadas e sombras suaves, cada um em versão clara e escura. Você escolhe uma predefinição, gera uma imagem aleatória ou sobrepõe várias chamas e exporta o resultado em qualquer tamanho, da tela de um celular a um monitor 4K. Uma cena é um único arquivo pequeno, e o mesmo arquivo sempre gera exatamente os mesmos pixels, no Windows, macOS, Linux, Android e iOS.

<p align="center">
  <img src="../samples/aurora-spiral.png" alt="A predefinição aurora-spiral: fios em espiral turquesa e verdes sobre um fundo escuro" width="880">
</p>

## O que o Fractique faz

- **Chamas:** 49 variações, camadas com modos de mesclagem, simetria rotacional e espelhada, estimativa de densidade, brilho e 29 predefinições embutidas.
- **Papéis de parede:** um para cada semente, a partir de 14 layouts e 6 materiais, cada um em versão clara e escura, disposto para o tamanho da tela.
- **O aplicativo:** uma única base de código para Windows, macOS, Linux, Android, iOS e a web. A imagem preenche a janela e painéis de vidro flutuam sobre ela; no desktop o aplicativo tem sua própria barra de título de vidro. Em 15 idiomas.
- **Randomize e Mutate:** cada semente é uma imagem; chamas que seriam um ponto ou um quadrado preenchido são descartadas. Mutate cria uma variante daquela de que você gostou.
- **Camadas:** adicione uma chama das predefinições ou uma aleatória, remova, reordene e defina a mesclagem (add, screen, over) e a opacidade de cada uma. Até 16 camadas.
- **Tamanho da imagem:** o da própria janela, uma de 21 telas (computadores até 4K, celulares e tablets Android, iPhones e iPads) ou qualquer largura e altura de 16 a 16384 pixels.
- **Zoom profundo:** arrastar, a roda do mouse e a pinça movem a vista e a ampliam até dez bilhões de vezes. A imagem acompanha na hora e é redesenhada nítida quando a vista para.
- **Celulares:** uma tela própria. Um botão redondo cria uma nova imagem, um deslize passa para o próximo papel de parede, uma folha de pequenas imagens ao vivo escolhe o estilo, e as imagens que você guarda formam uma coleção.
- **Exportação:** um PNG de qualquer tamanho, em três níveis de qualidade, renderizado em segundo plano. No celular ele é salvo em Fotos ou na galeria, ou compartilhado.
- **Arquivos de projeto:** um arquivo `.frcq` guarda a cena com sua vista, tom e tamanho, então ela abre como foi salva ([docs/scene-format.md](../scene-format.md)). Cenas `.json` simples e arquivos `.flame` do flam3 e do Apophysis também abrem. Entre uma execução e outra, o aplicativo volta à sessão em que foi deixado.
- **Atualizações:** uma janela lista cada versão lançada com suas notas e instala qualquer uma delas. No Windows o aplicativo é um único `.exe` portátil que substitui a si mesmo.
- **Repetibilidade exata:** a mesma cena gera os mesmos pixels em qualquer máquina, o que é verificado por testes no Linux, Windows e macOS ([docs/determinism.md](../determinism.md)).
- **A ferramenta de linha de comando** (`fractique`): renderiza chamas e papéis de parede em PNG, cria cenas aleatórias e variantes, converte arquivos `.flame`.
- **Servidor MCP** (`fractique mcp`): um agente de IA cria, altera e renderiza cenas ([docs/mcp.md](../mcp.md)).
- **Servidor HTTP** (`fractique-server`): uma API HTTP com fila de renderização e limites, serve o aplicativo web, distribuído como imagem Docker ([docs/server.md](../server.md)).

| | |
| --- | --- |
| ![emerald-star](../samples/emerald-star.png) | ![ember-vortex](../samples/ember-vortex.png) |
| `emerald-star` (duas camadas) | `ember-vortex` |
| ![golden-bloom](../samples/golden-bloom.png) | ![solar-mandala](../samples/solar-mandala.png) |
| `golden-bloom` | `solar-mandala` |

Papéis de parede, claros e escuros, cada um com seu layout e material:

| | | | |
| --- | --- | --- | --- |
| ![waves](../samples/wallpaper-waves-light.png) | ![burst](../samples/wallpaper-burst-light.png) | ![tiles](../samples/wallpaper-tiles-light.png) | ![fan](../samples/wallpaper-fan-light.png) |
| `waves`, `glass` | `burst`, `glow` | `tiles`, `relief` | `fan`, `glass` |
| ![discs](../samples/wallpaper-discs-dark.png) | ![bars](../samples/wallpaper-bars-dark.png) | ![arcs](../samples/wallpaper-arcs-dark.png) | ![lines](../samples/wallpaper-lines-dark.png) |
| `discs`, `paper` | `bars`, `paper` | `arcs`, `carbon` | `lines`, `paper` |

## Início rápido

Um único comando compila o aplicativo e a ferramenta de linha de comando. Ele instala o Rust se estiver faltando.

| Sistema | Comando |
| --- | --- |
| Linux, macOS | `./setup.sh` |
| Windows | `setup.cmd` |

Inicie o aplicativo:

```sh
./target/release/fractique-gui
```

Ou renderize pela linha de comando:

```sh
# uma predefinição como imagem 4K
./target/release/fractique render --preset aurora-spiral --size 3840x2160 --quality high -o aurora.png

# a chama aleatória da semente 42, para a tela de um celular
./target/release/fractique render --random 42 --size 1080x2400 --quality mobile -o phone.png

# o papel de parede da semente 7 para um celular, claro e escuro
./target/release/fractique render --wallpaper 7 --size 1290x2796 -o wallpaper.png
./target/release/fractique render --wallpaper 7 --dark --size 1290x2796 -o wallpaper-dark.png

# guardar uma cena aleatória em um arquivo, alterá-la, renderizá-la
./target/release/fractique randomize 42 -o scene.json
./target/release/fractique render scene.json -o mine.png
```

No Windows os programas são `target\release\fractique-gui.exe` e `target\release\fractique.exe`. Tudo o que o aplicativo e os comandos fazem: [docs/usage.md](../usage.md).

## Documentação

A documentação está em inglês.

| | |
| --- | --- |
| [docs/usage.md](../usage.md) | O aplicativo, os comandos, os níveis de qualidade, como editar uma cena |
| [docs/scene-format.md](../scene-format.md) | Cada campo de um arquivo de cena, cada variação, as folhas, o arquivo `.frcq` |
| [docs/mcp.md](../mcp.md) | O servidor MCP: como conectar um agente de IA, as ferramentas |
| [docs/server.md](../server.md) | O servidor HTTP: endpoints, limites, Docker |
| [docs/ios.md](../ios.md) | Compilar e executar em um iPhone ou no simulador do iOS |
| [docs/releasing.md](../releasing.md) | Como uma versão é lançada e como o aplicativo se atualiza |
| [docs/architecture.md](../architecture.md) | Como as partes se encaixam, como funciona uma renderização |
| [docs/determinism.md](../determinism.md) | Por que uma renderização se repete exatamente, o que é prometido e o que não é |
| [docs/performance.md](../performance.md) | Tempos de renderização e memória medidos |
| [docs/roadmap.md](../roadmap.md) | O que está pronto e o que vem a seguir |

## Licença

[PolyForm Noncommercial License 1.0.0](../../LICENSE.md): gratuita para qualquer uso não comercial. Para uso comercial, peça ao autor uma licença separada. As licenças de terceiros estão listadas em [THIRD_PARTY_NOTICES.md](../../THIRD_PARTY_NOTICES.md). Contribuições: veja [CONTRIBUTING.md](../../CONTRIBUTING.md).

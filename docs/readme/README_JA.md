<div align="center">
  <img src="../../assets/icon/icon-256.png" alt="Fractique" width="128" height="128">
  <h1>Fractique</h1>
  <p><strong>同じ入力からは、どのプラットフォームでも同じ画像を生成する、フラクタルの炎と壁紙のジェネレーター。</strong></p>
  <p><sub>
    <a href="../../README.md">English</a> · <a href="README_UK.md">Українська</a> · <a href="README_RU.md">Русский</a> · <a href="README_DE.md">Deutsch</a> · <a href="README_FR.md">Français</a> · <a href="README_ES.md">Español</a> · <a href="README_IT.md">Italiano</a> · <strong>日本語</strong><br>
    <a href="README_CN.md">简体中文</a> · <a href="README_PT.md">Português</a> · <a href="README_KO.md">한국어</a> · <a href="README_HI.md">हिन्दी</a> · <a href="README_TR.md">Türkçe</a> · <a href="README_PL.md">Polski</a> · <a href="README_ID.md">Bahasa Indonesia</a>
  </sub></p>
</div>

Fractique は 2 種類の画像を作ります。ひとつはフラクタルの炎。細い曲線と柔らかな光でできた、輝く霞のようなフラクタルです。もうひとつは壁紙。縁が光り、柔らかな影を落とす平らなシートを重ねたもので、それぞれにライト版とダーク版があります。プリセットを選ぶ、ランダムな画像を生成する、複数の炎を重ねるなどして、スマートフォンの画面から 4K モニターまで、好きなサイズで書き出せます。シーンは小さなファイル 1 つで、同じファイルは Windows、macOS、Linux、Android、iOS のどれでも常にまったく同じピクセルになります。

<p align="center">
  <img src="../samples/aurora-spiral.png" alt="aurora-spiral プリセット：暗い背景に青緑と緑の渦巻く光の筋" width="880">
</p>

## できること

- **炎：** 49 種類のバリエーション、ブレンドモード付きレイヤー、回転対称と鏡像対称、密度推定、グロー、そして 29 種類の組み込みプリセット。
- **壁紙：** シードごとに 1 枚。14 種類のレイアウトと 6 種類の素材から作られ、それぞれにライト版とダーク版があり、画面のサイズに合わせて配置されます。
- **アプリ：** Windows、macOS、Linux、Android、iOS、Web を 1 つのコードベースで。画像がウィンドウいっぱいに広がり、その上にガラスのパネルが浮かびます。デスクトップでは独自のガラスのタイトルバーを備えます。15 言語に対応。
- **「ランダム」と「変異」：** すべてのシードが 1 枚の画像になります。点や塗りつぶされた四角形になる炎は除外されます。「変異」は気に入った画像の変種を作ります。
- **レイヤー：** プリセットまたはランダムな炎を追加し、削除し、並べ替え、それぞれのブレンド（加算、スクリーン、重ねる）と不透明度を設定できます。最大 16 レイヤー。
- **画像サイズ：** ウィンドウそのもののサイズ、21 種類の画面（4K までのコンピューター、Android のスマートフォンとタブレット、iPhone と iPad）のいずれか、または 16 から 16384 ピクセルの任意の幅と高さ。
- **ディープズーム：** ドラッグ、ホイール、ピンチで表示を動かし、最大 100 億倍まで拡大できます。画像はすぐに追従し、表示が止まると鮮明に描き直されます。
- **スマートフォン：** 専用の画面。丸いボタン 1 つで新しい画像を作り、スワイプで次の壁紙へ進み、小さなライブ画像が並ぶシートでスタイルを選び、キープした画像はコレクションになります。
- **書き出し：** 任意のサイズの PNG を 3 段階の品質で、バックグラウンドでレンダリング。スマートフォンでは「写真」やギャラリーに保存するか、共有できます。
- **プロジェクトファイル：** `.frcq` ファイルはシーンをその表示、トーン、サイズとともに保存するので、保存したときのまま開けます（[docs/scene-format.md](../scene-format.md)）。通常の `.json` シーンや、flam3 と Apophysis の `.flame` ファイルも開けます。アプリは次に起動したとき、前回のセッションに戻ります。
- **アップデート：** リリースされた各バージョンとそのノートをウィンドウに一覧表示し、どのバージョンでもインストールできます。Windows 版は、自分自身を置き換えるポータブルな `.exe` 1 つです。
- **完全な再現性：** 同じシーンはどのマシンでも同じピクセルになります。Linux、Windows、macOS のテストで確認しています（[docs/determinism.md](../determinism.md)）。
- **コマンドラインツール**（`fractique`）：炎と壁紙を PNG にレンダリングし、ランダムなシーンや変種を作り、`.flame` ファイルを変換します。
- **MCP サーバー**（`fractique mcp`）：AI エージェントがシーンを作成、変更、レンダリング（[docs/mcp.md](../mcp.md)）。
- **HTTP サーバー**（`fractique-server`）：レンダリングのキューと上限を備えた HTTP API。Web アプリも配信し、Docker イメージとして提供（[docs/server.md](../server.md)）。

| | |
| --- | --- |
| ![emerald-star](../samples/emerald-star.png) | ![ember-vortex](../samples/ember-vortex.png) |
| `emerald-star`（2 レイヤー） | `ember-vortex` |
| ![golden-bloom](../samples/golden-bloom.png) | ![solar-mandala](../samples/solar-mandala.png) |
| `golden-bloom` | `solar-mandala` |

ライトとダークの壁紙。それぞれのレイアウトと素材：

| | | | |
| --- | --- | --- | --- |
| ![waves](../samples/wallpaper-waves-light.png) | ![burst](../samples/wallpaper-burst-light.png) | ![tiles](../samples/wallpaper-tiles-light.png) | ![fan](../samples/wallpaper-fan-light.png) |
| `waves`, `glass` | `burst`, `glow` | `tiles`, `relief` | `fan`, `glass` |
| ![discs](../samples/wallpaper-discs-dark.png) | ![bars](../samples/wallpaper-bars-dark.png) | ![arcs](../samples/wallpaper-arcs-dark.png) | ![lines](../samples/wallpaper-lines-dark.png) |
| `discs`, `paper` | `bars`, `paper` | `arcs`, `carbon` | `lines`, `paper` |

## クイックスタート

コマンド 1 つでアプリとコマンドラインツールをビルドします。Rust がなければインストールします。

| システム | コマンド |
| --- | --- |
| Linux, macOS | `./setup.sh` |
| Windows | `setup.cmd` |

アプリを起動：

```sh
./target/release/fractique-gui
```

またはコマンドラインからレンダリング：

```sh
# プリセットを 4K 画像として
./target/release/fractique render --preset aurora-spiral --size 3840x2160 --quality high -o aurora.png

# シード 42 のランダムな炎をスマートフォン画面用に
./target/release/fractique render --random 42 --size 1080x2400 --quality mobile -o phone.png

# シード 7 の壁紙をスマートフォン用に、ライトとダークで
./target/release/fractique render --wallpaper 7 --size 1290x2796 -o wallpaper.png
./target/release/fractique render --wallpaper 7 --dark --size 1290x2796 -o wallpaper-dark.png

# ランダムシーンをファイルに保存し、変更して、レンダリング
./target/release/fractique randomize 42 -o scene.json
./target/release/fractique render scene.json -o mine.png
```

Windows ではプログラムは `target\release\fractique-gui.exe` と `target\release\fractique.exe` にあります。アプリとコマンドでできることのすべて：[docs/usage.md](../usage.md)。

## ドキュメント

ドキュメントは英語です。

| | |
| --- | --- |
| [docs/usage.md](../usage.md) | アプリ、コマンド、品質レベル、シーンの編集方法 |
| [docs/scene-format.md](../scene-format.md) | シーンファイルの全フィールド、全バリエーション、シート、`.frcq` ファイル |
| [docs/mcp.md](../mcp.md) | MCP サーバー：AI エージェントの接続、ツール |
| [docs/server.md](../server.md) | HTTP サーバー：エンドポイント、上限、Docker |
| [docs/ios.md](../ios.md) | iPhone または iOS シミュレーターでのビルドと実行 |
| [docs/releasing.md](../releasing.md) | リリースの作り方、アプリが自分を更新する仕組み |
| [docs/architecture.md](../architecture.md) | 各部分のつながり、レンダリングの仕組み |
| [docs/determinism.md](../determinism.md) | レンダリングが正確に再現される理由、保証されることとされないこと |
| [docs/performance.md](../performance.md) | 計測したレンダリング時間とメモリ |
| [docs/roadmap.md](../roadmap.md) | 完了したことと今後の予定 |

## ライセンス

[PolyForm Noncommercial License 1.0.0](../../LICENSE.md)：非商用利用は無料です。商用利用については、作者に別途ライセンスをお問い合わせください。サードパーティのライセンスは [THIRD_PARTY_NOTICES.md](../../THIRD_PARTY_NOTICES.md) に記載しています。コントリビューション：[CONTRIBUTING.md](../../CONTRIBUTING.md) を参照。

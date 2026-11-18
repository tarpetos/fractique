<div align="center">
  <img src="../../assets/icon/icon-256.png" alt="Fractique" width="128" height="128">
  <h1>Fractique</h1>
  <p><strong>同じ入力からは、どのプラットフォームでも同じ画像を生成するフラクタルフレームジェネレーター。</strong></p>
  <p><sub>
    <a href="../../README.md">English</a> · <a href="README_UK.md">Українська</a> · <a href="README_RU.md">Русский</a> · <a href="README_DE.md">Deutsch</a> · <a href="README_FR.md">Français</a> · <a href="README_ES.md">Español</a> · <a href="README_IT.md">Italiano</a> · <strong>日本語</strong><br>
    <a href="README_CN.md">简体中文</a> · <a href="README_PT.md">Português</a> · <a href="README_KO.md">한국어</a> · <a href="README_HI.md">हिन्दी</a> · <a href="README_TR.md">Türkçe</a> · <a href="README_PL.md">Polski</a> · <a href="README_ID.md">Bahasa Indonesia</a>
  </sub></p>
</div>

Fractique はフラクタルフレームを描画します。細い曲線と柔らかな光でできた、輝く霞のようなフラクタルです。プリセットを選ぶ、ランダムなシーンを生成する、複数のフレームを重ねるなどして、スマートフォンの画面から 4K 壁紙まで、好きな解像度でレンダリングできます。シーンは小さな JSON ファイル 1 つで、同じファイルは Windows、macOS、Linux、Android、iOS のどれでも常にまったく同じピクセルになります。

<p align="center">
  <img src="../samples/aurora-spiral.png" alt="aurora-spiral プリセット：暗い背景に青緑と緑の渦巻く光の筋" width="880">
</p>

## 現在できること

- **レンダラー**（`fractique-core`）：49 種類のバリエーションを持つフラクタルフレーム、ブレンドモード付きレイヤー、回転対称と鏡像対称、密度推定、グロー、任意の解像度。
- **アプリ：** Windows、Linux、Android、iOS、Web を 1 つのコードベースで。フレームの上にガラスのパネルが浮かび、15 言語に対応。
- **プリセット：** 組み込みシーン 9 種類。
- **ランダム生成：** すべてのシードが 1 つのシーンになります。点や塗りつぶされた四角形になるシーンは除外されます。
- **変異：** 気に入ったシーンの変種を、0 から 1 の強さで作成。
- **完全な再現性：** 同じシーンはどのマシンでも同じピクセルになります。Linux、Windows、macOS のテストで確認しています（[docs/determinism.md](../determinism.md)）。
- **インポート：** flam3 と Apophysis の `.flame` ファイルをシーンに変換。
- **コマンドラインツール**（`fractique`）：シーンを PNG にレンダリング。
- **MCP サーバー**（`fractique mcp`）：AI エージェントがシーンを作成、変更、レンダリング（[docs/mcp.md](../mcp.md)）。

| | |
| --- | --- |
| ![emerald-star](../samples/emerald-star.png) | ![ember-vortex](../samples/ember-vortex.png) |
| `emerald-star` (2 レイヤー) | `ember-vortex` |

## セットアップ

コマンド 1 つですべてをビルドします。Rust がなければインストールします。

| システム | コマンド |
| --- | --- |
| Linux, macOS | `./setup.sh` |
| Windows | `setup.cmd` |

次に：

```sh
# プリセットを 4K 壁紙として
./target/release/fractique render --preset aurora-spiral --size 3840x2160 --quality high -o aurora.png

# シード 42 のランダムシーンをスマートフォン画面用に
./target/release/fractique render --random 42 --size 1080x2400 --quality mobile -o phone.png

# ランダムシーンをファイルに保存し、変更して、レンダリング
./target/release/fractique randomize 42 -o scene.json
./target/release/fractique render scene.json -o mine.png
```

Windows ではツールは `target\release\fractique.exe` にあります。すべてのコマンドとオプション：[docs/usage.md](../usage.md)。

## ドキュメント

ドキュメントは英語です。

| | |
| --- | --- |
| [docs/usage.md](../usage.md) | コマンド、品質レベル、シーンの編集方法 |
| [docs/scene-format.md](../scene-format.md) | シーンファイルの全フィールド、全バリエーション |
| [docs/mcp.md](../mcp.md) | MCP サーバー：AI エージェントの接続、ツール |
| [docs/ios.md](../ios.md) | iPhone または iOS シミュレーターでのビルドと実行 |
| [docs/architecture.md](../architecture.md) | 各部分のつながり、レンダリングの仕組み |
| [docs/determinism.md](../determinism.md) | レンダリングが正確に再現される理由、保証されることとされないこと |
| [docs/performance.md](../performance.md) | 計測したレンダリング時間とメモリ |
| [docs/roadmap.md](../roadmap.md) | 完了したことと今後の予定 |

## ライセンス

[PolyForm Noncommercial License 1.0.0](../../LICENSE.md)：非商用利用は無料です。商用利用については、作者に別途ライセンスをお問い合わせください。サードパーティのライセンスは [THIRD_PARTY_NOTICES.md](../../THIRD_PARTY_NOTICES.md) に記載しています。コントリビューション：[CONTRIBUTING.md](../../CONTRIBUTING.md) を参照。

<div align="center">
  <img src="../../assets/icon/icon-256.png" alt="Fractique" width="128" height="128">
  <h1>Fractique</h1>
  <p><strong>一个分形火焰生成器：相同的输入在任何平台上都得到相同的图像。</strong></p>
  <p><sub>
    <a href="../../README.md">English</a> · <a href="README_UK.md">Українська</a> · <a href="README_RU.md">Русский</a> · <a href="README_DE.md">Deutsch</a> · <a href="README_FR.md">Français</a> · <a href="README_ES.md">Español</a> · <a href="README_IT.md">Italiano</a> · <a href="README_JA.md">日本語</a><br>
    <strong>简体中文</strong> · <a href="README_PT.md">Português</a> · <a href="README_KO.md">한국어</a> · <a href="README_HI.md">हिन्दी</a> · <a href="README_TR.md">Türkçe</a> · <a href="README_PL.md">Polski</a> · <a href="README_ID.md">Bahasa Indonesia</a>
  </sub></p>
</div>

Fractique 渲染分形火焰：由细线和柔光构成的发光、轻盈的分形。你可以选择预设、生成随机场景，或把多个火焰叠加在一起，并以任意分辨率渲染，从手机屏幕到 4K 壁纸。一个场景就是一个小小的 JSON 文件，同一个文件在 Windows、macOS、Linux、Android 和 iOS 上始终渲染出完全相同的像素。

<p align="center">
  <img src="../samples/aurora-spiral.png" alt="aurora-spiral 预设：深色背景上青绿色与绿色的螺旋光丝" width="880">
</p>

## 目前可用的功能

- **渲染器**（`fractique-core`）：支持 28 种变体的分形火焰、带混合模式的图层、旋转对称与镜像对称、密度估计、辉光、任意分辨率。
- **应用：** 一套代码支持 Windows、Linux、Android、iOS 和网页，火焰之上浮着玻璃面板，提供 15 种语言。
- **预设：** 9 个内置场景。
- **随机生成：** 每个种子都是一个场景；只会变成一个点或一块实心方形的场景会被跳过。
- **变异：** 为你喜欢的场景生成变体，强度从 0 到 1。
- **精确可重复：** 同一个场景在每台机器上都得到相同的像素，并由 Linux、Windows 和 macOS 上的测试验证（[docs/determinism.md](../determinism.md)）。
- **导入：** flam3 和 Apophysis 的 `.flame` 文件可转换为场景。
- **命令行工具**（`fractique`）：将场景渲染为 PNG。
- **MCP 服务器**（`fractique mcp`）：AI 智能体可以创建、修改并渲染场景（[docs/mcp.md](../mcp.md)）。

| | |
| --- | --- |
| ![emerald-star](../samples/emerald-star.png) | ![ember-vortex](../samples/ember-vortex.png) |
| `emerald-star` (两个图层) | `ember-vortex` |

## 安装

一条命令即可构建全部内容。如果没有 Rust，它会自动安装。

| 系统 | 命令 |
| --- | --- |
| Linux, macOS | `./setup.sh` |
| Windows | `setup.cmd` |

然后：

```sh
# 把预设渲染成 4K 壁纸
./target/release/fractique render --preset aurora-spiral --size 3840x2160 --quality high -o aurora.png

# 种子 42 的随机场景，适配手机屏幕
./target/release/fractique render --random 42 --size 1080x2400 --quality mobile -o phone.png

# 把随机场景保存为文件，修改后再渲染
./target/release/fractique randomize 42 -o scene.json
./target/release/fractique render scene.json -o mine.png
```

在 Windows 上，工具位于 `target\release\fractique.exe`。全部命令和选项：[docs/usage.md](../usage.md)。

## 文档

文档为英文。

| | |
| --- | --- |
| [docs/usage.md](../usage.md) | 命令、质量等级、如何编辑场景 |
| [docs/scene-format.md](../scene-format.md) | 场景文件的每个字段、每种变体 |
| [docs/mcp.md](../mcp.md) | MCP 服务器：连接 AI 智能体、可用工具 |
| [docs/ios.md](../ios.md) | 在 iPhone 或 iOS 模拟器上构建和运行 |
| [docs/architecture.md](../architecture.md) | 各部分如何配合，渲染如何进行 |
| [docs/determinism.md](../determinism.md) | 为什么渲染结果能精确重复，哪些有保证、哪些没有 |
| [docs/performance.md](../performance.md) | 实测的渲染时间和内存 |
| [docs/roadmap.md](../roadmap.md) | 已完成的内容和后续计划 |

## 许可证

[PolyForm Noncommercial License 1.0.0](../../LICENSE.md)：任何非商业用途均可免费使用。商业用途请向作者申请单独的许可证。第三方许可证列于 [THIRD_PARTY_NOTICES.md](../../THIRD_PARTY_NOTICES.md)。参与贡献：请见 [CONTRIBUTING.md](../../CONTRIBUTING.md)。

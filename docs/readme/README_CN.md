<div align="center">
  <img src="../../assets/icon/icon-256.png" alt="Fractique" width="128" height="128">
  <h1>Fractique</h1>
  <p><strong>一款分形火焰与壁纸生成器：相同的输入，在任何平台上都得到相同的图像。</strong></p>
  <p><sub>
    <a href="../../README.md">English</a> · <a href="README_UK.md">Українська</a> · <a href="README_RU.md">Русский</a> · <a href="README_DE.md">Deutsch</a> · <a href="README_FR.md">Français</a> · <a href="README_ES.md">Español</a> · <a href="README_IT.md">Italiano</a> · <a href="README_JA.md">日本語</a><br>
    <strong>简体中文</strong> · <a href="README_PT.md">Português</a> · <a href="README_KO.md">한국어</a> · <a href="README_HI.md">हिन्दी</a> · <a href="README_TR.md">Türkçe</a> · <a href="README_PL.md">Polski</a> · <a href="README_ID.md">Bahasa Indonesia</a>
  </sub></p>
</div>

Fractique 生成两类图像。一类是分形火焰：由纤细曲线和柔和光线构成的、发光而轻盈的分形。另一类是壁纸：边缘受光、带柔和阴影的扁平薄片，每张都有浅色和深色两个版本。你可以选择预设、生成随机图像，或把多个火焰叠加在一起，再以任意尺寸导出，从手机屏幕到 4K 显示器都可以。一个场景就是一个小文件，同一个文件在 Windows、macOS、Linux、Android 和 iOS 上始终渲染出完全相同的像素。

<p align="center">
  <img src="../samples/aurora-spiral.png" alt="aurora-spiral 预设：深色背景上青绿色与绿色的螺旋光丝" width="880">
</p>

## 功能

- **火焰：** 49 种变体、带混合模式的图层、旋转对称与镜像对称、密度估计、辉光，以及 29 个内置预设。
- **壁纸：** 每个种子对应一张，由 14 种布局和 6 种材质组合而成，每张都有浅色和深色版本，并按屏幕尺寸排布。
- **应用：** 一套代码支持 Windows、macOS、Linux、Android、iOS 和 Web。图像铺满窗口，玻璃面板浮在其上；在桌面端，应用有自己的玻璃标题栏。支持 15 种语言。
- **Randomize 与 Mutate：** 每个种子都是一幅图像；会变成一个点或一块实心方形的火焰会被跳过。Mutate 为你喜欢的图像生成变体。
- **图层：** 从预设中添加火焰或添加随机火焰，删除、调整顺序，并为每个图层设置混合方式（add、screen、over）和不透明度。最多 16 个图层。
- **图像尺寸：** 窗口自身的尺寸、21 种屏幕之一（最高 4K 的电脑、Android 手机和平板、iPhone 和 iPad），或 16 到 16384 像素之间的任意宽高。
- **深度缩放：** 拖动、滚轮和双指捏合可移动视图，并将其放大至一百亿倍。图像立即跟随，视图停下后重新绘制为清晰的画面。
- **手机：** 专属界面。一个圆形按钮生成新图像，滑动切换到下一张壁纸，一张由实时小图组成的面板用来选择风格，你保留的图像组成收藏。
- **导出：** 任意尺寸的 PNG，三档质量，在后台渲染。在手机上可保存到“照片”或相册，也可以分享。
- **项目文件：** `.frcq` 文件保存场景及其视图、色调和尺寸，因此打开时与保存时一致（[docs/scene-format.md](../scene-format.md)）。普通的 `.json` 场景以及 flam3 和 Apophysis 的 `.flame` 文件也能打开。再次启动时，应用会回到上次离开时的会话。
- **更新：** 一个窗口列出每个已发布版本及其说明，并可安装其中任意一个。在 Windows 上，应用是一个会自我替换的便携 `.exe`。
- **精确可重复：** 同一个场景在任何机器上都得到相同的像素，并由 Linux、Windows 和 macOS 上的测试验证（[docs/determinism.md](../determinism.md)）。
- **命令行工具**（`fractique`）：将火焰和壁纸渲染为 PNG，生成随机场景和变体，转换 `.flame` 文件。
- **MCP 服务器**（`fractique mcp`）：AI 智能体创建、修改并渲染场景（[docs/mcp.md](../mcp.md)）。
- **HTTP 服务器**（`fractique-server`）：带渲染队列和限制的 HTTP API，在 `/mcp` 为 AI 智能体提供 MCP，同时提供网页版应用，并以 Docker 镜像发布（[docs/server.md](../server.md)）。

| | |
| --- | --- |
| ![emerald-star](../samples/emerald-star.png) | ![ember-vortex](../samples/ember-vortex.png) |
| `emerald-star` (两个图层) | `ember-vortex` |
| ![golden-bloom](../samples/golden-bloom.png) | ![solar-mandala](../samples/solar-mandala.png) |
| `golden-bloom` | `solar-mandala` |

浅色和深色壁纸，各自的布局与材质：

| | | | |
| --- | --- | --- | --- |
| ![waves](../samples/wallpaper-waves-light.png) | ![burst](../samples/wallpaper-burst-light.png) | ![tiles](../samples/wallpaper-tiles-light.png) | ![fan](../samples/wallpaper-fan-light.png) |
| `waves`, `glass` | `burst`, `glow` | `tiles`, `relief` | `fan`, `glass` |
| ![discs](../samples/wallpaper-discs-dark.png) | ![bars](../samples/wallpaper-bars-dark.png) | ![arcs](../samples/wallpaper-arcs-dark.png) | ![lines](../samples/wallpaper-lines-dark.png) |
| `discs`, `paper` | `bars`, `paper` | `arcs`, `carbon` | `lines`, `paper` |

## 快速开始

一条命令即可构建应用和命令行工具。如果没有安装 Rust，它会自动安装。

| 系统 | 命令 |
| --- | --- |
| Linux, macOS | `./setup.sh` |
| Windows | `setup.cmd` |

启动应用：

```sh
./target/release/fractique-gui
```

或者从命令行渲染：

```sh
# 把预设渲染成 4K 图像
./target/release/fractique render --preset aurora-spiral --size 3840x2160 --quality high -o aurora.png

# 种子 42 的随机火焰，用于手机屏幕
./target/release/fractique render --random 42 --size 1080x2400 --quality mobile -o phone.png

# 种子 7 的手机壁纸，浅色和深色
./target/release/fractique render --wallpaper 7 --size 1290x2796 -o wallpaper.png
./target/release/fractique render --wallpaper 7 --dark --size 1290x2796 -o wallpaper-dark.png

# 把随机场景保存为文件，修改后再渲染
./target/release/fractique randomize 42 -o scene.json
./target/release/fractique render scene.json -o mine.png
```

在 Windows 上，程序位于 `target\release\fractique-gui.exe` 和 `target\release\fractique.exe`。应用和命令的全部功能：[docs/usage.md](../usage.md)。

## 文档

文档为英文。

| | |
| --- | --- |
| [docs/usage.md](../usage.md) | 应用、命令、质量等级、如何编辑场景 |
| [docs/scene-format.md](../scene-format.md) | 场景文件的每个字段、每种变体、薄片、`.frcq` 文件 |
| [docs/mcp.md](../mcp.md) | MCP 服务器：连接 AI 智能体、工具列表 |
| [docs/server.md](../server.md) | HTTP 服务器：接口、限制、Docker |
| [docs/ios.md](../ios.md) | 在 iPhone 或 iOS 模拟器上构建和运行 |
| [docs/releasing.md](../releasing.md) | 如何发布版本，应用如何自我更新 |
| [docs/architecture.md](../architecture.md) | 各部分如何配合，渲染如何工作 |
| [docs/determinism.md](../determinism.md) | 渲染为何能精确重复，哪些有保证、哪些没有 |
| [docs/performance.md](../performance.md) | 实测的渲染时间和内存 |
| [docs/roadmap.md](../roadmap.md) | 已完成的内容和后续计划 |

## 许可证

[PolyForm Noncommercial License 1.0.0](../../LICENSE.md)：任何非商业用途均可免费使用。商业用途请向作者申请单独的许可证。第三方许可证列于 [THIRD_PARTY_NOTICES.md](../../THIRD_PARTY_NOTICES.md)。参与贡献：参见 [CONTRIBUTING.md](../../CONTRIBUTING.md)。

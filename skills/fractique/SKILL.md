---
name: fractique
description: Make fractal flame pictures, classic fractals (Mandelbrot, Julia, Mandelbulb and 19 more) and flat-shape wallpapers with the Fractique MCP server, and save them as PNG up to 16K. Use when the user asks for a fractal, a fractal flame, generative art, or a wallpaper for a phone or a desktop.
license: The skill text is free to use. Fractique itself is under PolyForm Noncommercial 1.0.0.
---

# Fractique

Fractique is a free fractal generator. This skill tells you how to drive its MCP server: pick or randomize a scene,
look at a small render, change it, and save the final picture at the size the user wants.

The same scene renders to the same pixels on every machine, so a scene saved as JSON is a complete recipe of a
picture.

## Before the first use

The tools below come from the MCP server `fractique-mcp`. If they are not available, tell the user to install it:

1. Download Fractique from https://github.com/tarpetos/fractique/releases/latest (Windows, Linux, macOS).
2. Register the server with the agent. For Claude Code:

   ```sh
   claude mcp add fractique -- /full/path/to/fractique-mcp
   ```

   For a client that takes JSON:

   ```json
   { "mcpServers": { "fractique": { "command": "/full/path/to/fractique-mcp" } } }
   ```

Where the program is after each kind of install: https://github.com/tarpetos/fractique/blob/master/docs/mcp.md

Do not download or run anything yourself without the user's word.

## The tools

| Tool | What it gives |
| --- | --- |
| `list_presets`, `get_preset` | The built-in scenes, and one of them as JSON |
| `randomize_scene` | The random scene of a `seed`: a flame, or with `fractal` a fractal of that kind |
| `list_fractals` | The kinds of fractal besides flames, with their parameters and presets |
| `mutate_scene` | A variant of a scene; `strength` from 0 to 1 |
| `fit_camera` | The scene with its flames framed |
| `validate_scene` | Whether a scene is valid, or which field is wrong |
| `describe_scene_format`, `list_variations` | Every field of a scene and every variation, for editing by hand |
| `import_flame` | A `.flame` file from Apophysis or flam3 as a scene |
| `list_wallpaper_styles`, `randomize_wallpaper` | Wallpapers made of flat shapes: the layouts and materials, and the wallpaper of a seed |
| `render_scene` | The picture as PNG, from a scene, a preset, a seed or a file |

## How to make a picture

1. Start from `randomize_scene` with a seed, or from `get_preset`. For a classic fractal call `list_fractals`
   first and pass the kind as `fractal`.
2. Call `render_scene` with the defaults (640 × 360, quality `mobile`). It answers in under a second and returns
   the picture inline, so you can look at it.
3. If it is dull or empty, try other seeds before editing: three or four small renders cost little. Show the user
   the candidates and let them choose.
4. To change the chosen one, call `mutate_scene` with a new seed, or edit the JSON (camera, tone, palette,
   transforms) after reading `describe_scene_format`. Check an edited scene with `validate_scene`.
5. Render again and compare.
6. For the final picture call `render_scene` with the user's size, `quality: "high"` and `save_to` set to the
   path of a PNG file. Pictures above about one megapixel are not returned inline; they are only saved.
7. Save the scene's JSON beside the PNG, so the picture can be made again at another size.

## Sizes worth knowing

- Desktop: 1920 × 1080, 2560 × 1440, 3840 × 2160.
- Phone: 1290 × 2796 fits most; ask for the model if the user wants an exact fit.
- `width` and `height` go up to 16384. A large picture at `high` or `ultra` takes from seconds to minutes: say
  so before starting one.

## Wallpapers of flat shapes

`render_scene` takes `wallpaper_seed` with `layout`, `material` and `dark`, and lays the shapes out for the size
it renders. `list_wallpaper_styles` has the names. Render with the proportions of the screen it is for.

## Rules

- Always look at a small render before a large one.
- Never write a file anywhere but the path the user gave or agreed to.
- Say which seed and which preset a picture came from, so the user can ask for it again.
- The pictures are computed from formulas. Do not call them AI-generated.

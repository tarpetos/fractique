# MCP server

`fractique mcp` runs Fractique as an [MCP](https://modelcontextprotocol.io) server, so an AI agent (Claude Code,
Claude Desktop or any other MCP client) can make scenes, change them and look at the result. The agent works on the
same scene JSON as the command line tool, and what it sees is exactly what a later render gives:
the same scene always renders to the same pixels.

The server speaks MCP over stdin and stdout. It is the same program as the command line tool, so there is nothing
else to install: build it with `setup.sh` or `setup.cmd` ([usage.md](usage.md#1-install)).

## Connect a client

**Claude Code, inside this repository:** nothing to do. `.mcp.json` at the root registers the server; Claude Code
asks once whether to use it.

**Claude Code, anywhere:**

```sh
claude mcp add fractique -- /full/path/to/fractique mcp
```

**Claude Desktop** and other clients that take a JSON configuration:

```json
{
  "mcpServers": {
    "fractique": {
      "command": "/full/path/to/fractique",
      "args": ["mcp"]
    }
  }
}
```

On Windows the program is `fractique.exe`.

## Tools

| Tool | Arguments | Returns |
| --- | --- | --- |
| `list_presets` | none | The names of the built-in scenes |
| `get_preset` | `name` | That scene as JSON |
| `randomize_scene` | `seed` | The random scene of the seed, framed |
| `mutate_scene` | `scene`, `seed`, `strength` (0 to 1, default 0.3) | A variant of the scene |
| `fit_camera` | `scene` | The scene with every layer's camera framing its flame |
| `validate_scene` | `scene` | Whether it is valid, or the field that is wrong |
| `describe_scene_format` | none | [scene-format.md](scene-format.md): every field, every variation |
| `list_variations` | none | The variations with their parameters and defaults |
| `import_flame` | `xml`, `index` (default 0) | A `.flame` file (flam3, Apophysis) as a scene, with what was left out |
| `render_scene` | one of `scene`, `preset`, `random_seed`; `width` (640), `height` (360), `quality` (`mobile`), `save_to` | The image as PNG, and a line with the size, the time and the hash |

A `scene` argument is the scene as a JSON object; its JSON text is accepted too. `quality` is one of `draft`,
`mobile`, `standard`, `high`, `ultra`; `width` and `height` go up to 16384.

### Rendering

`render_scene` returns the image inline, so the agent sees it. The defaults (640×360, quality `mobile`) render in
well under a second and are enough to judge a scene. Images above about one megapixel are not returned inline: give
`save_to`, the path of a PNG file to write, and the tool answers with the text line only. `save_to` also works for
small images, which are then both returned and saved.

A usual session:

1. `randomize_scene` or `get_preset` for a starting point.
2. `render_scene` to look at it.
3. Change the scene: the camera, the tone, the palette, a transform (`describe_scene_format` explains the fields),
   or ask `mutate_scene` for a variant.
4. `render_scene` again, and so on.
5. `render_scene` with the final size, `quality: "high"` and `save_to`.

### Wallpapers

No tool makes a wallpaper yet. A scene with `sheets` is a scene like any other, so `validate_scene` and
`render_scene` take one: write it with `fractique wallpaper SEED -o wallpaper.json`
([usage.md](usage.md#6-wallpapers)) or by hand from `describe_scene_format`, and render it at the proportion it
was laid out for. `fit_camera` and
`mutate_scene` change flames only and leave the sheets as they are.

## What the server does not do

- It writes only the file named in `save_to` and reads no files: scenes travel in the tool arguments. The app's
  `.frcq` files are for the command line: `fractique render file.frcq`.
- It handles one request at a time, in the order they arrive. A render cannot be cancelled and reports no progress
  yet; keep previews small.
- There is no HTTP transport yet: it comes with the server ([roadmap.md](roadmap.md)).

# MCP server

`fractique mcp` runs Fractique as an [MCP](https://modelcontextprotocol.io) server, so an AI agent (Claude Code,
Claude Desktop or any other MCP client) can make scenes, change them and look at the result. The agent works on the
same scene JSON as the command line tool, and what it sees is exactly what a later render gives:
the same scene always renders to the same pixels.

The server speaks MCP over stdin and stdout. It is the same program as the command line tool, so there is nothing
else to install: build it with `setup.sh` or `setup.cmd` ([usage.md](usage.md#1-install)).

The HTTP server offers the same tools to a client on another machine, at `/mcp`: see [Over HTTP](#over-http).

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
| `get_preset` | `name` | That scene as JSON; the presets of the fractals (`mandelbrot-whole`, ...) are listed by `list_fractals` |
| `randomize_scene` | `seed`; `fractal` | The random scene of the seed, framed: a flame, or with `fractal` a fractal of that kind |
| `list_fractals` | none | The kinds of fractal besides flames, with their parameters, defaults, ranges, texts and presets |
| `list_wallpaper_styles` | none | The layouts and the materials a wallpaper can have |
| `randomize_wallpaper` | `seed`; `layout`, `material`, `dark` (false), `width` (1290), `height` (2796) | The wallpaper of the seed as a scene, laid out for those proportions |
| `mutate_scene` | `scene`, `seed`, `strength` (0 to 1, default 0.3) | A variant of the scene |
| `fit_camera` | `scene` | The scene with every flame layer's camera framing its flame; a fractal of another kind keeps its view |
| `validate_scene` | `scene` | Whether it is valid, or the field that is wrong |
| `describe_scene_format` | none | [scene-format.md](scene-format.md): every field, every variation |
| `list_variations` | none | The variations with their parameters and defaults |
| `import_flame` | `xml`, `index` (default 0) | A `.flame` file (flam3, Apophysis) as a scene, with what was left out |
| `render_scene` | one of `scene`, `preset`, `random_seed`, `wallpaper_seed`, `file`, `frcq`; `width` (640), `height` (360), `quality` (`mobile`), `save_to`; with `random_seed`: `fractal`; with `wallpaper_seed`: `layout`, `material`, `dark` | The image as PNG, and a line with the size, the time and the hash |

A `scene` argument is the scene as a JSON object; its JSON text is accepted too, and so is the document of a
`.frcq` file (what `fractique convert file.frcq --document` prints): the scene under its `shown` is used.
`quality` is one of `draft`, `mobile`, `standard`, `high`, `ultra`; `width` and `height` go up to 16384.

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

A wallpaper is a scene of flat sheets instead of a flame ([usage.md](usage.md#6-wallpapers)). `randomize_wallpaper`
makes the one of a seed; `layout` and `material` hold a choice (`list_wallpaper_styles` has the names) and the
seed picks what is left out. The shapes are laid out for the proportions of `width` and `height`, so render the
scene with the same proportions:

```json
{ "name": "randomize_wallpaper", "arguments": { "seed": 7, "layout": "waves", "material": "glass", "dark": true } }
```

To look at a wallpaper without carrying its scene around, `render_scene` takes the seed itself and lays the
shapes out for the size it renders:

```json
{ "name": "render_scene", "arguments": { "wallpaper_seed": 7, "layout": "waves", "width": 430, "height": 932 } }
```

A scene with `sheets` is a scene like any other, so `validate_scene` and `render_scene` take one, also one
written by hand from `describe_scene_format`. `fit_camera` and `mutate_scene` change the layers only and leave the
sheets as they are.

### Files of the app

`render_scene` reads a scene file with `file`, the path of a `.frcq` file of the app or of a JSON scene, and
renders what the app showed when the file was saved. A client that holds the file and not a path the server can
reach gives its bytes in base64 as `frcq`:

```json
{ "name": "render_scene", "arguments": { "file": "/home/me/Pictures/mine.frcq", "quality": "high", "save_to": "/home/me/Pictures/mine.png", "width": 1920, "height": 1080 } }
```

### Progress and cancelling

A `tools/call` request that carries a progress token gets `notifications/progress` while it renders, as the MCP
specification describes them ([progress](https://modelcontextprotocol.io/specification/2025-06-18/basic/utilities/progress),
[cancellation](https://modelcontextprotocol.io/specification/2025-06-18/basic/utilities/cancellation)):

```json
{ "jsonrpc": "2.0", "id": 7, "method": "tools/call", "params": { "_meta": { "progressToken": "render-7" }, "name": "render_scene", "arguments": { "preset": "aurora-spiral", "width": 3840, "height": 2160, "quality": "high", "save_to": "aurora.png" } } }
```

```json
{ "jsonrpc": "2.0", "method": "notifications/progress", "params": { "progressToken": "render-7", "progress": 12, "total": 100, "message": "rendering" } }
```

`progress` is the percent done, a whole number that only grows: at most one hundred notifications for a render,
the last of them before the answer. Without a token none are sent.

To stop a render the client sends `notifications/cancelled` with the id of the request:

```json
{ "jsonrpc": "2.0", "method": "notifications/cancelled", "params": { "requestId": 7, "reason": "the user closed the preview" } }
```

The render stops within a moment, and that request gets no answer and no further progress, as the specification
asks. A request that is still waiting for its turn is dropped the same way. A cancellation that names an unknown or
finished request is ignored. The flames of a scene stop at once; its sheets (a wallpaper) are drawn in one go, in
about a second at 4K, report no progress and are not interrupted, but a cancelled wallpaper is not answered either.

## Over HTTP

`fractique-server` ([server.md](server.md)) speaks MCP at `POST /mcp` with the
[Streamable HTTP transport](https://modelcontextprotocol.io/specification/2025-06-18/basic/transports#streamable-http).
The tools are the ones above, from the same code (`crates/fractique-mcp`), and a scene renders to the same pixels.
It is on unless the server is started with `--mcp off` or `FRACTIQUE_MCP=off`.

```sh
claude mcp add --transport http fractique http://127.0.0.1:8080/mcp
```

```json
{ "mcpServers": { "fractique": { "type": "http", "url": "http://127.0.0.1:8080/mcp" } } }
```

What differs from `fractique mcp`:

- **No files.** A client on the network must not read or write the disk of the server, so `render_scene` has no
  `file` and no `save_to` there: `tools/list` does not show them, and a call that gives one is refused before
  anything is opened. A scene goes in `scene`, a `.frcq` file in `frcq` (base64), a `.flame` file as text in `xml`
  of `import_flame`. No tool takes a path.
- **The limits of the server.** `width`, `height` and `quality` must fit `FRACTIQUE_MAX_SIDE`,
  `FRACTIQUE_MAX_PIXELS`, `FRACTIQUE_MAX_SAMPLES` and `FRACTIQUE_MAX_SUPERSAMPLE`
  ([server.md](server.md#limits)); a request is 1 MiB at most, which bounds a scene and a `.frcq`. An image above
  about one megapixel is not returned, and with no `save_to` it cannot be rendered there: `POST /render` gives a
  large PNG.
- **The queue of the server.** A `render_scene` call takes a place in the same queue as `POST /render` and waits
  for a free slot. When the queue is full the call fails at once, as a tool error to try again later. A render
  past `FRACTIQUE_TIMEOUT` is stopped and fails the same way. The other tools do not wait.
- **Calls run side by side.** Each request is answered on its own connection, so a client may call a tool while
  a render runs.
- **The revisions** are `2025-06-18` and `2025-03-26`; `2024-11-05` has another HTTP transport, and a client that
  asks for it is offered `2025-06-18`.

The transport itself:

| | |
| --- | --- |
| `initialize` | Starts a session: the answer carries `Mcp-Session-Id`, and every later request must send it. Without it the answer is 400, with an unknown one 404, and the client starts again |
| Requests | `Content-Type: application/json`, one JSON-RPC message; a batch is refused. `Accept` lists `application/json` and `text/event-stream` |
| `tools/call` | Answered as `text/event-stream`: one `message` event for every progress notification, then one for the answer, and the stream ends |
| Other requests | Answered as `application/json` |
| Notifications and responses | 202 with no body |
| `GET /mcp` | 405: the server has nothing to say outside an answer |
| `DELETE /mcp` | Ends the session and stops its renders: 204 |
| `MCP-Protocol-Version` | When sent, it must be one of the revisions above, or the answer is 400 |
| `Origin` | A request that names a web page as its origin is refused with 403 unless the page is of this machine (`localhost`, `127.0.0.1`, `[::1]`) or listed in `--mcp-origin` / `FRACTIQUE_MCP_ORIGINS` (comma separated, `*` for any). Programs that are not browsers send none and pass |

By hand:

```sh
curl -i http://127.0.0.1:8080/mcp -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18"}}'

curl -N http://127.0.0.1:8080/mcp -H 'Content-Type: application/json' -H 'Mcp-Session-Id: THE-ID-FROM-ABOVE' \
  -d '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"_meta":{"progressToken":"p"},"name":"render_scene","arguments":{"preset":"aurora-spiral"}}}'
```

```
event: message
data: {"jsonrpc":"2.0","method":"notifications/progress","params":{"message":"rendering","progress":1,"progressToken":"p","total":100}}

event: message
data: {"id":2,"jsonrpc":"2.0","result":{"content":[{"data":"iVBORw0KGgo...","mimeType":"image/png","type":"image"},{"text":"640x360, 30 samples per pixel, 0.1 s, hash ...","type":"text"}],"isError":false}}
```

Cancelling is `notifications/cancelled` with the id of the request, sent in a request of its own with the same
session. The render stops, or leaves the queue, and its stream ends without an answer. A cancellation from another
session does nothing. A client that closes the connection of a call gives it up as well: the server keeps no
stream to come back to, so the answer could not reach it, and the render would only hold a place in the queue.
The specification advises against reading a disconnect as a cancellation; here the place in the queue weighs more.

The server keeps up to 1024 sessions. When one more starts, the one unused for the longest time ends, an idle one
before one with a call in progress.

## What the server does not do

- Over stdio it writes only the file named in `save_to` and reads only the file named in `file`.
- Over stdio it runs one tool at a time, in the order the calls arrive; `ping`, `tools/list` and cancellations are
  answered while a tool runs.
- It speaks the protocol revisions with the `initialize` handshake (`2025-06-18`, `2025-03-26`, `2024-11-05`),
  not the later ones.
- Over HTTP it has no accounts, no stream opened with `GET` and no resuming of a broken stream: a call whose
  connection broke is made again.

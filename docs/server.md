# HTTP server

`fractique-server` puts Fractique behind an HTTP API: presets, random scenes, wallpapers and renders that come back
as PNG. It calls the same renderer as the command line tool and the app, so a request renders to the same pixels
there, here and in a container. When it is given the web build of the app it serves that too, so one address offers
both the API and the app in a browser. AI agents reach the same renderer through MCP at `/mcp`.

Renders go through a queue with hard limits, so an instance open to the internet cannot be made to work without
end. See [Limits](#limits) before you publish one.

## Contents

1. [Run it](#run-it)
2. [Endpoints](#endpoints)
3. [Errors](#errors)
4. [Limits](#limits)
5. [The queue](#the-queue)
6. [MCP](#mcp)
7. [The web app](#the-web-app)
8. [Docker](#docker)
9. [Publishing an instance](#publishing-an-instance)

## Run it

```sh
cargo build --release -p fractique-server
./target/release/fractique-server
```

It listens on `127.0.0.1:8080`. Every setting is an option and an environment variable; the option wins:

| Option | Environment variable | Default | What |
| --- | --- | --- | --- |
| `--bind` | `FRACTIQUE_BIND` | `127.0.0.1:8080` | Address and port to listen on |
| `--web` | `FRACTIQUE_WEB` | none | Folder with the web build of the app, served at `/` |
| `--jobs` | `FRACTIQUE_JOBS` | 1 | Renders that run at once |
| `--queue` | `FRACTIQUE_QUEUE` | 8 | Renders that may wait for a free slot |
| `--max-side` | `FRACTIQUE_MAX_SIDE` | 4096 | Largest width and height, in pixels |
| `--max-pixels` | `FRACTIQUE_MAX_PIXELS` | 8294400 | Largest `width × height` (3840×2160) |
| `--max-samples` | `FRACTIQUE_MAX_SAMPLES` | 1000 | Most samples per pixel |
| `--max-supersample` | `FRACTIQUE_MAX_SUPERSAMPLE` | 2 | Largest supersample |
| `--timeout` | `FRACTIQUE_TIMEOUT` | 120 | Seconds a render may take before it is stopped |
| `--mcp` | `FRACTIQUE_MCP` | `on` | MCP for AI agents at `/mcp`: `on` or `off` |
| `--mcp-origin` | `FRACTIQUE_MCP_ORIGINS` | none | Origins of web pages that may call `/mcp` besides the pages of this machine; the option once per origin, the variable with commas between them; `*` for any |

## Endpoints

Requests and answers are JSON, except the PNG that `/render` returns. A field that the server does not know is
refused, so a misspelt name is an error and not a silent default.

| Method and path | Body | Returns |
| --- | --- | --- |
| `GET /presets` | | `{"presets": ["aurora-spiral", ...]}` |
| `GET /presets/{name}` | | The preset as a scene; a preset of a fractal (`mandelbrot-whole`) by the name `GET /fractals` gives |
| `POST /random` | `{"seed", "fractal"?}` | The random scene of the seed: a flame, or a fractal of the kind named |
| `GET /fractals` | | `{"fractals": [...]}`: the kinds of fractal besides flames, each with its `name`, `family`, how it is `drawn`, its `params` (`name`, `default`, `min`, `max`, `whole`), its `texts` (`name`, `default`) and its `presets` |
| `POST /wallpaper` | `{"seed", "layout"?, "material"?, "dark"?, "hue"?, "width"?, "height"?}` | The wallpaper of the seed as a scene |
| `POST /render` | One source, a size and a quality, see below | `image/png` |
| `GET /limits` | | The limits of this instance, whether MCP is on (`"mcp"`) and the names of the qualities, layouts and materials |
| `GET /health` | | `{"status": "ok", "busy": 0}`: renders running and waiting |
| `POST /mcp`, `DELETE /mcp` | JSON-RPC | MCP for AI agents, see [MCP](#mcp) |

A scene is the JSON document described in [scene-format.md](scene-format.md). A seed is a whole number from 0 to
2⁶⁴ − 1.

### Presets

```sh
curl http://127.0.0.1:8080/presets
curl http://127.0.0.1:8080/presets/aurora-spiral -o aurora-spiral.json
```

### A random scene

```sh
curl -X POST http://127.0.0.1:8080/random -d '{"seed": 42}' -o scene.json
curl -X POST http://127.0.0.1:8080/random -d '{"seed": 42, "fractal": "julia"}' -o julia.json
curl http://127.0.0.1:8080/fractals
```

### A wallpaper

```sh
curl -X POST http://127.0.0.1:8080/wallpaper \
  -d '{"seed": 7, "layout": "waves", "material": "glass", "dark": true, "width": 1290, "height": 2796}' \
  -o wallpaper.json
```

`layout` and `material` are the names that `GET /limits` lists; without them the seed picks. `hue` (degrees)
asks for one color. The shapes are laid out for `width` × `height`, 1290×2796 when they are left out.

### Render

```sh
curl -X POST http://127.0.0.1:8080/render \
  -d '{"preset": "aurora-spiral", "width": 1920, "height": 1080, "quality": "high"}' -o aurora.png

curl -X POST http://127.0.0.1:8080/render \
  -d '{"random": 42, "width": 1080, "height": 2400, "quality": "mobile"}' -o phone.png

curl -X POST http://127.0.0.1:8080/render \
  -d '{"wallpaper": {"seed": 7, "layout": "waves", "dark": true}, "width": 1290, "height": 2796}' -o wallpaper.png

curl -X POST http://127.0.0.1:8080/render \
  -d "{\"scene\": $(cat scene.json), \"width\": 960, \"height\": 540, \"samples\": 200}" -o mine.png
```

| Field | | |
| --- | --- | --- |
| `scene`, `preset`, `random` or `wallpaper` | exactly one | A scene document, a preset's name, the seed of a random scene, or a wallpaper: the body of `/wallpaper`, laid out for the size of the render |
| `fractal` | optional, with `random` | The kind of the random scene, a name that `GET /fractals` lists (`mandelbrot`, `julia`, `clifford`, ...); a flame when left out |
| `width`, `height` | required | The size of the image in pixels |
| `quality` | optional | `draft`, `mobile`, `standard`, `high` or `ultra` ([usage.md](usage.md#4-size-and-quality)); `standard` when left out |
| `samples` | optional | Samples per pixel, instead of the quality's |
| `supersample` | optional | Supersample, instead of the quality's |
| `seed` | optional | Replaces the scene's seed |

The answer is the PNG. Its `x-fractique-hash` header is the hash of the pixels, the same number the command line
tool prints after a render.

A scene sent back to the server renders exactly as the server would render it from its seed or preset name when the
numbers in it are kept as they were written. A client that reads the scene into its own number type and writes it
out again may round the last digit of a coordinate; the render is then of that slightly different scene.

## Errors

An error is a status and `{"error": "what went wrong"}`.

| Status | When |
| --- | --- |
| 400 | The body is not valid, a name is unknown, the scene is not valid or the request is outside the limits |
| 404 | No preset of that name, or nothing at that path |
| 413 | The body is larger than 1 MiB |
| 429 | The queue is full; `Retry-After` says to try again in a second |
| 503 | The render took longer than the time limit and was stopped |

## Limits

| Limit | Setting | Default |
| --- | --- | --- |
| Width and height | `FRACTIQUE_MAX_SIDE` | 4096 pixels |
| `width × height` | `FRACTIQUE_MAX_PIXELS` | 8 294 400 (3840×2160) |
| Samples per pixel | `FRACTIQUE_MAX_SAMPLES` | 1000 (the `ultra` quality) |
| Supersample | `FRACTIQUE_MAX_SUPERSAMPLE` | 2 |
| Time of one render | `FRACTIQUE_TIMEOUT` | 120 seconds |
| Renders at once | `FRACTIQUE_JOBS` | 1 |
| Renders waiting | `FRACTIQUE_QUEUE` | 8 |
| Request body | fixed | 1 MiB |

A request outside a limit is refused before it takes a place in the queue. A quality whose samples or supersample
are above the limits is refused too: with `FRACTIQUE_MAX_SAMPLES=300`, `ultra` is not available. The limits of the
renderer itself ([architecture.md](architecture.md#limits)) hold whatever the settings say.

The time limit counts the render, not the wait in the queue. A render that reaches it is cancelled and its slot is
free again as soon as the renderer notices, which is within a moment while it is still collecting samples. The last
steps of a render (density estimation, tone mapping, glow, sheets) are not interrupted; the limit on pixels is what
bounds them.

Memory is what `FRACTIQUE_JOBS` multiplies: a render at 3840×2160 with supersample 2 needs about 1.6 GB
([performance.md](performance.md#memory)). One render already uses every core, so more jobs at once do not finish
sooner; they only let a small render pass beside a large one.

## The queue

`FRACTIQUE_JOBS` renders run at once and `FRACTIQUE_QUEUE` more wait for a slot, in the order they came. One more
is answered at once with 429. `GET /health` shows how many are running and waiting.

A client that closes its connection gives its place up: a waiting render leaves the queue, a running one is
cancelled. Scenes, presets and the other answers do not go through the queue.

## MCP

`/mcp` is the MCP server of the command line tool ([mcp.md](mcp.md)) over HTTP: the same tools, from the same
code, for an agent that is not on this machine.

```sh
claude mcp add --transport http fractique http://127.0.0.1:8080/mcp
```

It is on by default. It opens nothing that the API beside it does not: a render asked for through it obeys the
same limits, waits in the same queue (`GET /health` counts it) and is stopped by the same time limit, and the
arguments that read or write a file (`file`, `save_to`) do not exist there, so a client cannot reach the disk.
`--mcp off` or `FRACTIQUE_MCP=off` takes the path away: `/mcp` then answers 404 and `GET /limits` says
`"mcp": false`.

A full queue, a time limit reached and a request outside the limits are tool errors there (`"isError": true` with
the reason as text), not HTTP statuses: the HTTP status of `/mcp` speaks of the transport only. The sessions, the
headers, progress, cancelling and the check of `Origin` are described in [mcp.md](mcp.md#over-http).

## The web app

The app compiles to WebAssembly with [Trunk](https://trunkrs.dev):

```sh
rustup target add wasm32-unknown-unknown
cargo install trunk --locked
cd crates/fractique-app && trunk build --release
cd ../.. && ./target/release/fractique-server --web crates/fractique-app/dist
```

The folder is served at `/` beside the API. A file with a `.gz` copy next to it is sent compressed to browsers
that accept it. The app in the browser renders on the visitor's own machine; it does not call the API.

## Docker

The `Dockerfile` at the root builds the server and the web app and puts both into a small image that runs as a
user without rights:

```sh
docker build -t fractique-server .
docker run --rm -p 8080:8080 fractique-server
```

Then `http://localhost:8080` is the app and `http://localhost:8080/presets` the API. Settings are environment
variables:

```sh
docker run --rm -p 8080:8080 -e FRACTIQUE_MAX_SIDE=2048 -e FRACTIQUE_TIMEOUT=30 fractique-server
```

`docker-compose.yml` is the same with every limit written out, a read-only file system and no capabilities:

```sh
docker compose up --build
```

`--build-arg JOBS=4` limits how many cores the build uses. The first build takes a while: it compiles the app for
the browser.

## Publishing an instance

- The server speaks plain HTTP. Put it behind a reverse proxy that ends TLS (Caddy, nginx, Traefik).
- It has no accounts and no limit per address. The queue bounds how much it works, not who gets the work: limit
  requests per address in the proxy.
- Give the container a memory limit that fits `FRACTIQUE_JOBS` renders at the largest size you allow.
- The API sends no CORS headers, so a page on another address cannot call it from a browser.
- `/mcp` has no accounts either. Turn it off (`FRACTIQUE_MCP=off`) when no agent uses it, or let the proxy ask
  for a token on that path. A session id is a random number that tells sessions apart, not a password.

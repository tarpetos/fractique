use std::collections::BTreeSet;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

use fractique_core::random::random_scene;
use fractique_core::wallpaper::{Layout, Material, Request, random_wallpaper};
use fractique_core::{Control, Image, Quality, RenderOptions, Scene, presets, render};
use fractique_server::{Config, app};
use serde_json::{Value, json};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::watch;

struct Reply {
    status: u16,
    head: String,
    body: Vec<u8>,
}

impl Reply {
    fn json(&self) -> Value {
        serde_json::from_slice(&self.body).expect("the body is JSON")
    }

    fn header(&self, name: &str) -> Option<&str> {
        self.head.lines().find_map(|line| {
            let (key, value) = line.split_once(':')?;
            key.eq_ignore_ascii_case(name).then(|| value.trim())
        })
    }

    fn image(&self) -> Image {
        let mut reader = png::Decoder::new(std::io::Cursor::new(&self.body))
            .read_info()
            .expect("the body is a PNG");
        let mut rgba = vec![0; reader.output_buffer_size().expect("the PNG has a size")];
        let frame = reader.next_frame(&mut rgba).expect("the PNG decodes");
        assert_eq!(
            (frame.color_type, frame.bit_depth),
            (png::ColorType::Rgba, png::BitDepth::Eight)
        );
        Image {
            width: frame.width,
            height: frame.height,
            rgba,
        }
    }
}

struct Server {
    address: SocketAddr,
    busy: watch::Receiver<usize>,
}

impl Server {
    async fn start(config: Config) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("a free port");
        let address = listener.local_addr().expect("the port is known");
        let app = app(config);
        tokio::spawn(async move { axum::serve(listener, app.router).await });
        Self {
            address,
            busy: app.busy,
        }
    }

    async fn open(&self, method: &str, path: &str, body: Option<&Value>) -> TcpStream {
        let body = body.map(Value::to_string).unwrap_or_default();
        let request = format!(
            "{method} {path} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
            self.address,
            body.len()
        );
        let mut stream = TcpStream::connect(self.address).await.expect("the server listens");
        stream.write_all(request.as_bytes()).await.expect("the request is sent");
        stream
    }

    async fn send(&self, method: &str, path: &str, body: Option<&Value>) -> Reply {
        let mut stream = self.open(method, path, body).await;
        let mut bytes = Vec::new();
        stream.read_to_end(&mut bytes).await.expect("the reply is read");
        let split = bytes
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
            .expect("the reply has a head");
        let head = String::from_utf8_lossy(&bytes[..split]).into_owned();
        let status = head
            .split_whitespace()
            .nth(1)
            .and_then(|code| code.parse().ok())
            .expect("the reply has a status");
        Reply {
            status,
            head,
            body: bytes[split + 4..].to_vec(),
        }
    }

    async fn get(&self, path: &str) -> Reply {
        self.send("GET", path, None).await
    }

    async fn post(&self, path: &str, body: Value) -> Reply {
        self.send("POST", path, Some(&body)).await
    }

    async fn busy_is(&self, count: usize) {
        let mut busy = self.busy.clone();
        tokio::time::timeout(Duration::from_secs(60), busy.wait_for(|busy| *busy == count))
            .await
            .unwrap_or_else(|_| panic!("the server never had {count} renders"))
            .expect("the server is alive");
    }
}

fn core_render(scene: &Scene, options: &RenderOptions) -> Image {
    render(scene, options, &Control::default()).expect("the scene renders")
}

fn endless() -> Value {
    json!({ "preset": "aurora-spiral", "width": 1024, "height": 1024, "samples": 100_000 })
}

fn roomy(jobs: usize, queue: usize) -> Config {
    Config {
        jobs,
        queue,
        max_samples: 100_000,
        ..Config::default()
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn presets_are_listed_and_returned() {
    let server = Server::start(Config::default()).await;

    let list = server.get("/presets").await;
    assert_eq!(list.status, 200);
    assert_eq!(list.json()["presets"], json!(presets::names()));

    let one = server.get("/presets/ember-vortex").await;
    assert_eq!(one.status, 200);
    let scene: Scene = serde_json::from_slice(&one.body).expect("a scene");
    assert_eq!(Some(scene), presets::get("ember-vortex"));

    let missing = server.get("/presets/no-such").await;
    assert_eq!(missing.status, 404);
    assert!(missing.json()["error"].as_str().is_some_and(|e| e.contains("no-such")));
}

#[tokio::test(flavor = "multi_thread")]
async fn random_scenes_and_wallpapers_are_the_core_ones() {
    let server = Server::start(Config::default()).await;

    let random = server.post("/random", json!({ "seed": 42 })).await;
    assert_eq!(random.status, 200);
    assert_eq!(
        random.body,
        serde_json::to_vec(&random_scene(42)).expect("a scene is JSON")
    );

    let body = json!({ "seed": 7, "layout": "waves", "material": "glass", "dark": true, "width": 800, "height": 600 });
    let wallpaper = server.post("/wallpaper", body).await;
    assert_eq!(wallpaper.status, 200);
    let request = Request {
        layout: Some(Layout::Waves),
        material: Some(Material::Glass),
        ratio: 600.0 / 800.0,
        dark: true,
        hue: None,
    };
    assert_eq!(
        wallpaper.body,
        serde_json::to_vec(&random_wallpaper(7, &request)).expect("a scene is JSON")
    );

    let julia = server.post("/random", json!({ "seed": 42, "fractal": "julia" })).await;
    assert_eq!(julia.status, 200);
    let expected = fractique_core::fractal::random_scene("julia", 42).expect("a kind");
    assert_eq!(julia.body, serde_json::to_vec(&expected).expect("a scene is JSON"));
    assert_eq!(
        server
            .post("/random", json!({ "seed": 1, "fractal": "nothing" }))
            .await
            .status,
        400
    );
    let kinds = server.get("/fractals").await;
    assert_eq!(kinds.status, 200);
    let names: Vec<String> = kinds.json()["fractals"]
        .as_array()
        .expect("a list")
        .iter()
        .map(|kind| kind["name"].as_str().expect("a name").to_owned())
        .collect();
    assert_eq!(
        names,
        fractique_core::fractal::all().map(|spec| spec.name).collect::<Vec<_>>()
    );
    let preset = server.get("/presets/mandelbrot-whole").await;
    assert_eq!(preset.status, 200);

    assert_eq!(server.post("/random", json!({})).await.status, 400);
    assert_eq!(server.post("/random", json!({ "seed": -1 })).await.status, 400);
    let unknown = server
        .post("/wallpaper", json!({ "seed": 1, "layout": "spirals" }))
        .await;
    assert_eq!(unknown.status, 400);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_render_is_the_png_of_the_core_render() {
    let server = Server::start(Config::default()).await;
    let options = RenderOptions::new(96, 64, Quality::Draft);
    let size = json!({ "width": 96, "height": 64, "quality": "draft" });
    let with = |source: Value| {
        let mut body = size.clone();
        body.as_object_mut()
            .expect("an object")
            .extend(source.as_object().expect("an object").clone());
        body
    };

    let preset = presets::get("aurora-spiral").expect("a preset");
    let wallpaper = Request {
        layout: Some(Layout::Discs),
        material: None,
        ratio: 64.0 / 96.0,
        dark: false,
        hue: None,
    };
    let mut reseeded = preset.clone();
    reseeded.seed = 99;
    let sent = Scene::from_json(&random_scene(5).to_json()).expect("a scene");
    let cases = [
        (with(json!({ "preset": "aurora-spiral" })), preset.clone()),
        (with(json!({ "random": 42 })), random_scene(42)),
        (
            with(json!({ "random": 42, "fractal": "mandelbrot" })),
            fractique_core::fractal::random_scene("mandelbrot", 42).expect("a kind"),
        ),
        (
            with(json!({ "preset": "clifford-classic" })),
            fractique_core::fractal::preset("clifford-classic").expect("a preset"),
        ),
        (with(json!({ "scene": sent })), sent.clone()),
        (
            with(json!({ "wallpaper": { "seed": 3, "layout": "discs" } })),
            random_wallpaper(3, &wallpaper),
        ),
        (with(json!({ "preset": "aurora-spiral", "seed": 99 })), reseeded),
    ];
    for (body, scene) in cases {
        let reply = server.post("/render", body.clone()).await;
        assert_eq!(reply.status, 200, "{body}");
        assert_eq!(reply.header("content-type"), Some("image/png"));
        assert!(reply.body.starts_with(b"\x89PNG\r\n\x1a\n"));
        let expected = core_render(&scene, &options);
        assert_eq!(reply.image(), expected, "{body}");
        assert_eq!(
            reply.header("x-fractique-hash"),
            Some(format!("{:016x}", expected.hash()).as_str())
        );
    }

    let samples = json!({ "preset": "aurora-spiral", "width": 80, "height": 80, "samples": 12, "supersample": 2 });
    let reply = server.post("/render", samples).await;
    let options = RenderOptions {
        width: 80,
        height: 80,
        samples_per_pixel: 12,
        supersample: 2,
    };
    assert_eq!(reply.image(), core_render(&preset, &options));
}

#[tokio::test(flavor = "multi_thread")]
async fn requests_outside_the_limits_are_refused() {
    let config = Config {
        max_side: 256,
        max_pixels: 200 * 200,
        max_samples: 50,
        max_supersample: 1,
        ..Config::default()
    };
    let server = Server::start(config).await;
    let refused = [
        json!({ "preset": "aurora-spiral", "width": 257, "height": 10, "quality": "draft" }),
        json!({ "preset": "aurora-spiral", "width": 0, "height": 10, "quality": "draft" }),
        json!({ "preset": "aurora-spiral", "width": 256, "height": 200, "quality": "draft" }),
        json!({ "preset": "aurora-spiral", "width": 64, "height": 64, "samples": 51 }),
        json!({ "preset": "aurora-spiral", "width": 64, "height": 64, "quality": "standard" }),
        json!({ "preset": "aurora-spiral", "width": 64, "height": 64, "quality": "draft", "supersample": 2 }),
        json!({ "preset": "aurora-spiral", "width": 64, "height": 64, "quality": "best" }),
        json!({ "preset": "aurora-spiral", "random": 1, "width": 64, "height": 64, "quality": "draft" }),
        json!({ "width": 64, "height": 64, "quality": "draft" }),
        json!({ "scene": { "version": 1 }, "width": 64, "height": 64, "quality": "draft" }),
        json!({ "preset": "aurora-spiral", "width": 64, "height": 64, "quality": "draft", "extra": 1 }),
    ];
    for body in refused {
        let reply = server.post("/render", body.clone()).await;
        assert_eq!(reply.status, 400, "{body}");
        assert!(reply.json()["error"].is_string(), "{body}");
    }
    server.busy_is(0).await;

    let missing = json!({ "preset": "no-such", "width": 64, "height": 64, "quality": "draft" });
    assert_eq!(server.post("/render", missing).await.status, 404);

    let allowed = json!({ "preset": "aurora-spiral", "width": 200, "height": 200, "samples": 50 });
    assert_eq!(server.post("/render", allowed).await.status, 200);

    let limits = server.get("/limits").await.json();
    assert_eq!(limits["max_side"], 256);
    assert_eq!(limits["max_pixels"], 40_000);
    assert_eq!(limits["max_samples"], 50);

    let large = json!({ "seed": 1, "layout": "x".repeat(fractique_server::MAX_BODY_BYTES) });
    assert_eq!(server.post("/wallpaper", large).await.status, 413);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_full_queue_refuses_and_a_closed_connection_frees_its_place() {
    let server = Server::start(roomy(1, 1)).await;

    let running = server.open("POST", "/render", Some(&endless())).await;
    server.busy_is(1).await;
    let waiting = server.open("POST", "/render", Some(&endless())).await;
    server.busy_is(2).await;

    let refused = server.post("/render", endless()).await;
    assert_eq!(refused.status, 429);
    assert_eq!(refused.header("retry-after"), Some("1"));
    assert!(refused.json()["error"].is_string());
    assert_eq!(server.get("/health").await.json()["busy"], 2);
    assert_eq!(server.get("/presets").await.status, 200);

    drop(waiting);
    server.busy_is(1).await;
    drop(running);
    server.busy_is(0).await;

    let small = json!({ "preset": "aurora-spiral", "width": 64, "height": 64, "quality": "draft" });
    assert_eq!(server.post("/render", small).await.status, 200);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_render_past_the_time_limit_is_stopped() {
    let config = Config {
        timeout: Duration::from_millis(50),
        ..roomy(1, 0)
    };
    let server = Server::start(config).await;
    let reply = server.post("/render", endless()).await;
    assert_eq!(reply.status, 503);
    assert!(reply.json()["error"].is_string());
    server.busy_is(0).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn the_web_folder_is_served_beside_the_api() {
    let folder = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("web");
    std::fs::create_dir_all(&folder).expect("a folder");
    std::fs::write(folder.join("index.html"), "<title>Fractique</title>").expect("a page");
    std::fs::write(folder.join("app.js"), "let fractique;").expect("a script");

    let config = Config {
        web: Some(folder),
        ..Config::default()
    };
    let server = Server::start(config).await;
    let page = server.get("/").await;
    assert_eq!(page.status, 200);
    assert_eq!(page.body, b"<title>Fractique</title>");
    assert_eq!(page.header("content-type"), Some("text/html"));
    assert_eq!(server.get("/app.js").await.body, b"let fractique;");
    assert_eq!(server.get("/../Cargo.toml").await.status, 404);
    assert_eq!(server.get("/missing.js").await.status, 404);
    assert_eq!(server.get("/presets").await.json()["presets"], json!(presets::names()));

    let bare = Server::start(Config::default()).await;
    let nothing = bare.get("/").await;
    assert_eq!(nothing.status, 404);
    assert!(nothing.json()["error"].is_string());
}

#[tokio::test(flavor = "multi_thread")]
async fn the_description_and_the_router_have_the_same_routes() {
    for mcp in [true, false] {
        let config = Config {
            mcp,
            ..Config::default()
        };
        let routes: BTreeSet<(String, String)> = app(config.clone())
            .routes
            .into_iter()
            .filter(|(_, path)| !path.starts_with("/docs/"))
            .map(|(method, path)| (method.to_owned(), path.to_owned()))
            .collect();
        assert_eq!(routes.contains(&("post".to_owned(), "/mcp".to_owned())), mcp);

        let server = Server::start(config).await;
        let reply = server.get("/openapi.json").await;
        assert_eq!(reply.status, 200);
        assert_eq!(reply.header("content-type"), Some("application/json"));
        let spec = reply.json();
        assert_eq!(spec["openapi"], "3.1.0");
        assert_eq!(spec["info"]["version"], env!("CARGO_PKG_VERSION"));
        let described: BTreeSet<(String, String)> = spec["paths"]
            .as_object()
            .expect("paths")
            .iter()
            .flat_map(|(path, item)| {
                let methods = item.as_object().expect("a path item").keys();
                methods.map(move |method| (method.clone(), path.clone()))
            })
            .collect();
        assert_eq!(described, routes);
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn the_examples_of_the_description_are_answered() {
    let server = Server::start(Config::default()).await;
    let spec = server.get("/openapi.json").await.json();
    let mut sent = 0;
    for path in ["/random", "/wallpaper", "/render"] {
        let content = &spec["paths"][path]["post"]["requestBody"]["content"]["application/json"];
        for (name, example) in content["examples"].as_object().expect("examples") {
            let reply = server.post(path, example["value"].clone()).await;
            assert_eq!(reply.status, 200, "{path} {name}");
            let png = reply.body.starts_with(b"\x89PNG\r\n\x1a\n");
            assert_eq!(png, path == "/render", "{path} {name}");
            sent += 1;
        }
    }
    assert!(sent >= 7);

    let schema = &spec["components"]["schemas"]["RenderRequest"];
    let mut body = json!({ "preset": "aurora-spiral", "width": 64, "height": 64, "quality": "draft" });
    for (field, _) in schema["properties"].as_object().expect("fields") {
        if body.get(field).is_none() {
            body[field] = Value::Null;
        }
    }
    assert_eq!(
        server.post("/render", body).await.status,
        200,
        "a field of the description is unknown"
    );
    let limits = server.get("/limits").await.json();
    let listed = spec["components"]["schemas"]["Limits"]["properties"]
        .as_object()
        .expect("fields");
    assert_eq!(
        listed.keys().collect::<BTreeSet<_>>(),
        limits.as_object().expect("limits").keys().collect::<BTreeSet<_>>()
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn the_reference_page_is_served_by_the_server_itself() {
    let server = Server::start(Config::default()).await;
    let page = server.get("/docs").await;
    assert_eq!(page.status, 200);
    assert_eq!(page.header("content-type"), Some("text/html; charset=utf-8"));
    let policy = page.header("content-security-policy").expect("a policy");
    assert!(policy.contains("default-src 'none'") && policy.contains("connect-src 'self'"));
    let html = String::from_utf8(page.body).expect("text");
    assert!(html.contains("<title>Fractique API</title>"));
    assert!(!html.contains("http://") && !html.contains("https://"));

    let assets = [
        ("/docs/scalar.js", "text/javascript; charset=utf-8"),
        ("/docs/init.js", "text/javascript; charset=utf-8"),
        ("/docs/icon-32.png", "image/png"),
        ("/docs/icon-64.png", "image/png"),
        ("/docs/icon.png", "image/png"),
    ];
    for (path, kind) in assets {
        let reply = server.get(path).await;
        assert_eq!(reply.status, 200, "{path}");
        assert_eq!(reply.header("content-type"), Some(kind), "{path}");
        assert_eq!(reply.header("content-encoding"), None, "{path}");
        assert_eq!(
            reply.body.starts_with(b"\x89PNG\r\n\x1a\n"),
            kind == "image/png",
            "{path}"
        );
    }
    assert!(html.contains("/docs/scalar.js") && html.contains("/docs/init.js") && html.contains("/docs/icon-32.png"));
    let plain = server.get("/docs/scalar.js").await.body;
    let name = b"createApiReference";
    assert!(plain.len() > 1_000_000 && plain.windows(name.len()).any(|window| window == name));

    let request = format!(
        "GET /docs/scalar.js HTTP/1.1\r\nHost: {}\r\nConnection: close\r\nAccept-Encoding: gzip, br\r\n\r\n",
        server.address
    );
    let mut stream = TcpStream::connect(server.address).await.expect("the server listens");
    stream.write_all(request.as_bytes()).await.expect("the request is sent");
    let mut packed = Vec::new();
    stream.read_to_end(&mut packed).await.expect("the reply is read");
    let head = String::from_utf8_lossy(&packed[..400]).to_ascii_lowercase();
    assert!(head.contains("content-encoding: gzip") && head.contains("vary: accept-encoding"));
    assert!(packed.len() < plain.len() / 2);

    assert_eq!(server.get("/limits").await.json()["docs"], true);
}

#[tokio::test(flavor = "multi_thread")]
async fn docs_off_takes_the_reference_away() {
    let config = Config {
        docs: false,
        ..Config::default()
    };
    let server = Server::start(config).await;
    for path in [
        "/docs",
        "/openapi.json",
        "/docs/scalar.js",
        "/docs/init.js",
        "/docs/icon.png",
    ] {
        let reply = server.get(path).await;
        assert_eq!(reply.status, 404, "{path}");
        assert!(reply.json()["error"].is_string(), "{path}");
    }
    assert_eq!(server.get("/limits").await.json()["docs"], false);
    assert_eq!(server.get("/presets").await.status, 200);
}

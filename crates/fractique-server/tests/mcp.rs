use std::net::SocketAddr;
use std::time::Duration;

use fractique_core::random::random_scene;
use fractique_core::{Control, Quality, RenderOptions, Scene, presets, render};
use fractique_server::{Config, app};
use serde_json::{Value, json};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::watch;

const BOTH: (&str, &str) = ("Accept", "application/json, text/event-stream");

struct Reply {
    status: u16,
    head: String,
    body: Vec<u8>,
}

fn unchunked(mut rest: &[u8]) -> Vec<u8> {
    let mut body = Vec::new();
    while let Some(end) = rest.windows(2).position(|window| window == b"\r\n") {
        let size = usize::from_str_radix(String::from_utf8_lossy(&rest[..end]).trim(), 16).expect("a chunk size");
        let chunk = &rest[end + 2..];
        if size == 0 || chunk.len() < size {
            break;
        }
        body.extend_from_slice(&chunk[..size]);
        rest = &chunk[(size + 2).min(chunk.len())..];
    }
    body
}

impl Reply {
    fn of(bytes: &[u8]) -> Self {
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
        let mut reply = Self {
            status,
            head,
            body: bytes[split + 4..].to_vec(),
        };
        if reply.header("transfer-encoding") == Some("chunked") {
            reply.body = unchunked(&reply.body);
        }
        reply
    }

    fn header(&self, name: &str) -> Option<&str> {
        self.head.lines().find_map(|line| {
            let (key, value) = line.split_once(':')?;
            key.eq_ignore_ascii_case(name).then(|| value.trim())
        })
    }

    fn json(&self) -> Value {
        serde_json::from_slice(&self.body).expect("the body is JSON")
    }

    fn events(&self) -> Vec<Value> {
        assert_eq!(self.status, 200);
        assert_eq!(self.header("content-type"), Some("text/event-stream"));
        String::from_utf8_lossy(&self.body)
            .split("\n\n")
            .filter(|event| !event.is_empty())
            .map(|event| {
                let data = event.strip_prefix("event: message\ndata: ").expect("a message event");
                serde_json::from_str(data).expect("the data is a JSON message")
            })
            .collect()
    }

    fn result(&self) -> Value {
        let events = self.events();
        let answer = events.last().expect("the stream ends with the answer");
        assert!(answer.get("method").is_none(), "{answer}");
        answer["result"].clone()
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

    async fn open(&self, method: &str, path: &str, headers: &[(&str, &str)], body: &str) -> TcpStream {
        let headers: String = headers
            .iter()
            .map(|(name, value)| format!("{name}: {value}\r\n"))
            .collect();
        let request = format!(
            "{method} {path} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n{headers}Content-Length: {}\r\n\r\n{body}",
            self.address,
            body.len()
        );
        let mut stream = TcpStream::connect(self.address).await.expect("the server listens");
        stream.write_all(request.as_bytes()).await.expect("the request is sent");
        stream
    }

    async fn send(&self, method: &str, path: &str, headers: &[(&str, &str)], body: &str) -> Reply {
        let mut stream = self.open(method, path, headers, body).await;
        let mut bytes = Vec::new();
        stream.read_to_end(&mut bytes).await.expect("the reply is read");
        Reply::of(&bytes)
    }

    async fn busy_is(&self, count: usize) {
        let mut busy = self.busy.clone();
        tokio::time::timeout(Duration::from_secs(60), busy.wait_for(|busy| *busy == count))
            .await
            .unwrap_or_else(|_| panic!("the server never had {count} renders"))
            .expect("the server is alive");
    }

    async fn session(&self) -> Session<'_> {
        let hello = json!({
            "jsonrpc": "2.0", "id": 0, "method": "initialize",
            "params": { "protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": { "name": "test", "version": "0" } },
        });
        let json = ("Content-Type", "application/json");
        let reply = self.send("POST", "/mcp", &[json, BOTH], &hello.to_string()).await;
        assert_eq!(reply.status, 200, "{}", reply.head);
        assert_eq!(reply.json()["result"]["protocolVersion"], "2025-06-18");
        let session = Session {
            server: self,
            id: reply.header("mcp-session-id").expect("a session").to_owned(),
        };
        let ready = session
            .post(json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }))
            .await;
        assert_eq!((ready.status, ready.body.len()), (202, 0));
        session
    }
}

struct Session<'a> {
    server: &'a Server,
    id: String,
}

fn request(id: u64, token: Option<&str>, name: &str, arguments: Value) -> Value {
    let mut params = json!({ "name": name, "arguments": arguments });
    if let Some(token) = token {
        params["_meta"] = json!({ "progressToken": token });
    }
    json!({ "jsonrpc": "2.0", "id": id, "method": "tools/call", "params": params })
}

impl Session<'_> {
    fn headers(&self) -> [(&str, &str); 4] {
        [
            ("Content-Type", "application/json"),
            BOTH,
            ("Mcp-Session-Id", &self.id),
            ("MCP-Protocol-Version", "2025-06-18"),
        ]
    }

    async fn post(&self, message: Value) -> Reply {
        let body = message.to_string();
        self.server.send("POST", "/mcp", &self.headers(), &body).await
    }

    async fn open(&self, message: Value) -> TcpStream {
        let body = message.to_string();
        self.server.open("POST", "/mcp", &self.headers(), &body).await
    }

    async fn call(&self, name: &str, arguments: Value) -> Value {
        self.post(request(1, None, name, arguments)).await.result()
    }

    async fn cancel(&self, id: u64) {
        let note = json!({ "jsonrpc": "2.0", "method": "notifications/cancelled", "params": { "requestId": id } });
        assert_eq!(self.post(note).await.status, 202);
    }
}

fn text(result: &Value) -> &str {
    result["content"][0]["text"].as_str().expect("a text")
}

fn refused(result: &Value) -> &str {
    assert_eq!(result["isError"], true, "{result}");
    text(result)
}

fn endless() -> Value {
    json!({ "preset": "aurora-spiral", "width": 1024, "height": 1024, "quality": "ultra" })
}

async fn rest(stream: &mut TcpStream) -> Reply {
    let mut bytes = Vec::new();
    tokio::time::timeout(Duration::from_secs(60), stream.read_to_end(&mut bytes))
        .await
        .expect("the stream ends")
        .expect("the reply is read");
    Reply::of(&bytes)
}

#[tokio::test(flavor = "multi_thread")]
async fn a_session_starts_answers_and_ends() {
    let server = Server::start(Config::default()).await;
    let session = server.session().await;
    assert_eq!(session.id.len(), 32);
    assert_ne!(session.id, server.session().await.id);

    let ping = session
        .post(json!({ "jsonrpc": "2.0", "id": 1, "method": "ping" }))
        .await;
    assert_eq!(ping.status, 200);
    assert_eq!(ping.header("content-type"), Some("application/json"));
    assert_eq!(ping.json(), json!({ "jsonrpc": "2.0", "id": 1, "result": {} }));

    let unknown = session
        .post(json!({ "jsonrpc": "2.0", "id": "a", "method": "nope" }))
        .await;
    assert_eq!(unknown.status, 200);
    assert_eq!(unknown.json()["error"]["code"], -32601);
    let nameless = session
        .post(json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {} }))
        .await;
    assert_eq!(nameless.json()["error"]["code"], -32602);
    let answered = session.post(json!({ "jsonrpc": "2.0", "id": 9, "result": {} })).await;
    assert_eq!((answered.status, answered.body.len()), (202, 0));

    let old =
        json!({ "jsonrpc": "2.0", "id": 0, "method": "initialize", "params": { "protocolVersion": "2024-11-05" } });
    let json = ("Content-Type", "application/json");
    let agreed = server.send("POST", "/mcp", &[json], &old.to_string()).await;
    assert_eq!(agreed.json()["result"]["protocolVersion"], "2025-06-18");

    let listen = server.send("GET", "/mcp", &[("Accept", "text/event-stream")], "").await;
    assert_eq!(listen.status, 405);
    assert!(listen.header("allow").is_some_and(|allow| allow.contains("POST")));

    let headers = session.headers();
    assert_eq!(server.send("DELETE", "/mcp", &headers, "").await.status, 204);
    assert_eq!(server.send("DELETE", "/mcp", &headers, "").await.status, 404);
    let gone = session
        .post(json!({ "jsonrpc": "2.0", "id": 3, "method": "ping" }))
        .await;
    assert_eq!(gone.status, 404);
    assert!(gone.json()["error"]["message"].is_string());
}

#[tokio::test(flavor = "multi_thread")]
async fn requests_that_break_the_transport_are_refused() {
    let server = Server::start(Config {
        mcp_origins: vec!["https://fractique.example".into()],
        ..Config::default()
    })
    .await;
    let session = server.session().await;
    let ping = json!({ "jsonrpc": "2.0", "id": 1, "method": "ping" }).to_string();
    let json = ("Content-Type", "application/json");
    let id = ("Mcp-Session-Id", session.id.as_str());
    let status = async |headers: &[(&str, &str)], body: &str| server.send("POST", "/mcp", headers, body).await.status;

    assert_eq!(status(&[json, id], &ping).await, 200);
    assert_eq!(status(&[json], &ping).await, 400);
    assert_eq!(
        status(&[json, ("Mcp-Session-Id", "0123456789abcdef")], &ping).await,
        404
    );
    assert_eq!(status(&[id], &ping).await, 415);
    assert_eq!(status(&[("Content-Type", "text/plain"), id], &ping).await, 415);
    assert_eq!(
        status(&[json, id, ("MCP-Protocol-Version", "2024-11-05")], &ping).await,
        400
    );
    assert_eq!(
        status(&[json, id, ("MCP-Protocol-Version", "2025-03-26")], &ping).await,
        200
    );
    assert_eq!(status(&[json, id, ("Accept", "text/html")], &ping).await, 406);
    assert_eq!(status(&[json, id], "{").await, 400);
    assert_eq!(status(&[json, id], &format!("[{ping}]")).await, 400);
    assert_eq!(status(&[json, id], "{}").await, 400);

    assert_eq!(status(&[json, id, ("Origin", "http://evil.example")], &ping).await, 403);
    assert_eq!(
        status(&[json, id, ("Origin", "http://localhost:5173")], &ping).await,
        200
    );
    assert_eq!(
        status(&[json, id, ("Origin", "https://fractique.example")], &ping).await,
        200
    );
    let hello = json!({ "jsonrpc": "2.0", "id": 0, "method": "initialize", "params": {} }).to_string();
    assert_eq!(status(&[json, ("Origin", "http://evil.example")], &hello).await, 403);
    let evil = [id, ("Origin", "http://evil.example")];
    assert_eq!(server.send("DELETE", "/mcp", &evil, "").await.status, 403);

    let call = request(1, None, "list_presets", json!({})).to_string();
    assert_eq!(status(&[json, id, ("Accept", "application/json")], &call).await, 406);
    let large = request(
        1,
        None,
        "import_flame",
        json!({ "xml": "x".repeat(fractique_server::MAX_BODY_BYTES) }),
    );
    assert_eq!(status(&[json, id], &large.to_string()).await, 413);

    let off = Server::start(Config {
        mcp: false,
        ..Config::default()
    })
    .await;
    assert_eq!(off.send("POST", "/mcp", &[json], &hello).await.status, 404);
    let limits = off.send("GET", "/limits", &[], "").await.json();
    assert_eq!(limits["mcp"], false);
    assert_eq!(server.send("GET", "/limits", &[], "").await.json()["mcp"], true);
}

#[tokio::test(flavor = "multi_thread")]
async fn the_tools_are_those_of_the_command_line_server_without_files() {
    let server = Server::start(Config::default()).await;
    let session = server.session().await;

    let listed = session
        .post(json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/list" }))
        .await;
    let tools = listed.json()["result"]["tools"].as_array().expect("tools").clone();
    assert_eq!(tools.len(), 12);
    let described = tools
        .iter()
        .find(|tool| tool["name"] == "render_scene")
        .expect("render_scene");
    let properties = described["inputSchema"]["properties"].as_object().expect("properties");
    assert!(properties.contains_key("frcq") && properties.contains_key("scene"));
    assert!(!properties.contains_key("file") && !properties.contains_key("save_to"));
    assert_eq!(properties["width"]["maximum"], 4096);

    let names = session.call("list_presets", json!({})).await;
    assert_eq!(text(&names), presets::names().join("\n"));
    let random = session.call("randomize_scene", json!({ "seed": 42 })).await;
    assert_eq!(random["isError"], false);
    assert_eq!(text(&random), random_scene(42).to_json());
    let scene: Value = serde_json::from_str(text(&random)).expect("a scene");
    assert_eq!(
        session.call("validate_scene", json!({ "scene": scene })).await["isError"],
        false
    );
    assert_eq!(
        session.call("mutate_scene", json!({ "scene": scene, "seed": 1 })).await["isError"],
        false
    );
    assert_eq!(
        session.call("fit_camera", json!({ "scene": scene })).await["isError"],
        false
    );
    assert!(refused(&session.call("no_such_tool", json!({})).await).contains("no_such_tool"));

    let options = RenderOptions::new(96, 64, Quality::Draft);
    let size = json!({ "width": 96, "height": 64, "quality": "draft" });
    let with = |name: &str, value: Value| {
        let mut arguments = size.clone();
        arguments[name] = value;
        arguments
    };
    let sent = Scene::from_json(text(&random)).expect("a scene");
    let cases = [
        (
            with("preset", "aurora-spiral".into()),
            presets::get("aurora-spiral").expect("a preset"),
        ),
        (with("random_seed", 42.into()), random_scene(42)),
        (with("scene", scene.clone()), sent),
    ];
    for (arguments, scene) in cases {
        let result = session.call("render_scene", arguments.clone()).await;
        assert_eq!(result["isError"], false, "{result}");
        assert_eq!(result["content"][0]["mimeType"], "image/png");
        assert!(
            result["content"][0]["data"]
                .as_str()
                .is_some_and(|data| data.starts_with("iVBORw0KGgo"))
        );
        let expected = render(&scene, &options, &Control::default()).expect("the scene renders");
        let hash = format!("hash {:016x}", expected.hash());
        assert!(
            result["content"][1]["text"]
                .as_str()
                .is_some_and(|note| note.ends_with(&hash)),
            "{arguments}"
        );
    }
    server.busy_is(0).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn the_disk_of_the_server_is_out_of_reach() {
    let server = Server::start(Config::default()).await;
    let session = server.session().await;
    let folder = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR"));
    let scene = folder.join("mcp-readable.json");
    std::fs::write(&scene, presets::get("aurora-spiral").expect("a preset").to_json()).expect("a scene file");
    let picture = folder.join("mcp-written.png");
    let _ = std::fs::remove_file(&picture);
    let small = json!({ "width": 64, "height": 36, "quality": "draft" });
    let with = |pairs: &[(&str, &str)]| {
        let mut arguments = small.clone();
        for (name, value) in pairs {
            arguments[*name] = (*value).into();
        }
        arguments
    };

    let read = session
        .call("render_scene", with(&[("file", &scene.to_string_lossy())]))
        .await;
    assert!(refused(&read).contains("file is refused"), "{read}");
    assert_eq!(read["content"].as_array().map(Vec::len), Some(1));
    let secret = session.call("render_scene", with(&[("file", "/etc/passwd")])).await;
    assert_eq!(refused(&secret), refused(&read));
    let missing = session.call("render_scene", with(&[("file", "/no/such/file")])).await;
    assert_eq!(refused(&missing), refused(&read));

    let both = [("preset", "aurora-spiral"), ("save_to", &picture.to_string_lossy())];
    let written = session.call("render_scene", with(&both)).await;
    assert!(refused(&written).contains("save_to is refused"), "{written}");
    assert!(!picture.exists());

    let path = session
        .call("import_flame", json!({ "xml": scene.to_string_lossy() }))
        .await;
    assert_eq!(path["isError"], true);
    assert!(!text(&path).contains("aurora"));
    let as_scene = session
        .call("validate_scene", json!({ "scene": scene.to_string_lossy() }))
        .await;
    assert_eq!(as_scene["isError"], true);

    assert_eq!(
        session.call("render_scene", with(&[("preset", "aurora-spiral")])).await["isError"],
        false
    );
    server.busy_is(0).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn renders_outside_the_limits_are_refused() {
    let config = Config {
        max_side: 256,
        max_pixels: 200 * 200,
        max_samples: 50,
        max_supersample: 1,
        ..Config::default()
    };
    let server = Server::start(config).await;
    let session = server.session().await;
    let outside = [
        json!({ "preset": "aurora-spiral", "width": 257, "height": 10, "quality": "draft" }),
        json!({ "preset": "aurora-spiral", "width": 0, "height": 10, "quality": "draft" }),
        json!({ "preset": "aurora-spiral", "width": 256, "height": 200, "quality": "draft" }),
        json!({ "preset": "aurora-spiral", "width": 64, "height": 64, "quality": "standard" }),
        json!({ "preset": "aurora-spiral", "width": 64, "height": 64, "quality": "high" }),
        json!({ "preset": "aurora-spiral" }),
    ];
    for arguments in outside {
        let result = session.call("render_scene", arguments.clone()).await;
        assert!(refused(&result).contains("this server"), "{arguments}: {result}");
    }
    let allowed = json!({ "preset": "aurora-spiral", "width": 200, "height": 200, "quality": "mobile" });
    assert_eq!(session.call("render_scene", allowed).await["isError"], false);
    server.busy_is(0).await;

    let roomy = Server::start(Config::default()).await;
    let large = json!({ "preset": "aurora-spiral", "width": 1920, "height": 1080, "quality": "draft" });
    let result = roomy.session().await.call("render_scene", large).await;
    assert!(refused(&result).contains("too large to return inline"), "{result}");
    assert!(!refused(&result).contains("save_to"));
}

#[tokio::test(flavor = "multi_thread")]
async fn progress_comes_as_events_before_the_answer() {
    let server = Server::start(Config::default()).await;
    let session = server.session().await;
    let arguments = json!({ "preset": "emerald-star", "width": 64, "height": 36, "quality": "draft" });
    let reply = session
        .post(request(5, Some("render-5"), "render_scene", arguments.clone()))
        .await;
    assert_eq!(reply.header("cache-control"), Some("no-cache"));
    let events = reply.events();
    let (answer, notes) = events.split_last().expect("events");
    assert_eq!(answer["id"], 5);
    assert_eq!(answer["result"]["isError"], false);
    assert!(!notes.is_empty());
    let mut last = 0;
    for note in notes {
        assert_eq!(note["method"], "notifications/progress");
        assert!(note.get("id").is_none());
        assert_eq!(note["params"]["progressToken"], "render-5");
        assert_eq!(note["params"]["total"], 100);
        let progress = note["params"]["progress"].as_u64().expect("a percent");
        assert!(progress > last && progress <= 100, "{progress} after {last}");
        last = progress;
    }
    assert_eq!(last, 100);

    let silent = session.post(request(6, None, "render_scene", arguments)).await;
    assert_eq!(silent.events().len(), 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn renders_share_the_queue_and_leave_it_when_cancelled_or_disconnected() {
    let server = Server::start(Config {
        jobs: 1,
        queue: 1,
        max_samples: 100_000,
        ..Config::default()
    })
    .await;
    let session = server.session().await;
    let other = server.session().await;
    let json = [("Content-Type", "application/json")];
    let direct = json!({ "preset": "aurora-spiral", "width": 1024, "height": 1024, "samples": 100_000 }).to_string();

    let running = server.open("POST", "/render", &json, &direct).await;
    server.busy_is(1).await;
    let mut waiting = session.open(request(7, None, "render_scene", endless())).await;
    server.busy_is(2).await;

    let full = session.call("render_scene", endless()).await;
    assert!(refused(&full).contains("queue is full"), "{full}");
    assert_eq!(server.send("POST", "/render", &json, &direct).await.status, 429);
    assert_eq!(session.call("list_presets", json!({})).await["isError"], false);

    other.cancel(7).await;
    session.cancel(8).await;
    assert_eq!(*server.busy.borrow(), 2);
    session.cancel(7).await;
    server.busy_is(1).await;
    assert!(rest(&mut waiting).await.events().is_empty());

    let dropped = session.open(request(9, None, "render_scene", endless())).await;
    server.busy_is(2).await;
    drop(dropped);
    server.busy_is(1).await;
    drop(running);
    server.busy_is(0).await;

    let mut rendering = session.open(request(10, Some("t"), "render_scene", endless())).await;
    let mut seen = Vec::new();
    while !String::from_utf8_lossy(&seen).contains("notifications/progress") {
        let mut bytes = [0; 4096];
        let read = rendering.read(&mut bytes).await.expect("the stream is read");
        assert_ne!(read, 0, "the stream ended before any progress");
        seen.extend_from_slice(&bytes[..read]);
    }
    session.cancel(10).await;
    let mut later = Vec::new();
    rendering.read_to_end(&mut later).await.expect("the stream ends");
    assert!(!String::from_utf8_lossy(&later).contains("\"result\""));
    server.busy_is(0).await;

    let silent = session.open(request(11, None, "render_scene", endless())).await;
    server.busy_is(1).await;
    drop(silent);
    server.busy_is(0).await;

    let ended = session.open(request(12, None, "render_scene", endless())).await;
    server.busy_is(1).await;
    assert_eq!(server.send("DELETE", "/mcp", &session.headers(), "").await.status, 204);
    server.busy_is(0).await;
    drop(ended);

    let small = json!({ "preset": "aurora-spiral", "width": 64, "height": 64, "quality": "draft" });
    assert_eq!(other.call("render_scene", small).await["isError"], false);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_render_past_the_time_limit_is_stopped_and_answered() {
    let server = Server::start(Config {
        timeout: Duration::from_millis(50),
        queue: 0,
        ..Config::default()
    })
    .await;
    let session = server.session().await;
    let result = session.call("render_scene", endless()).await;
    assert!(refused(&result).contains("longer than"), "{result}");
    server.busy_is(0).await;
}

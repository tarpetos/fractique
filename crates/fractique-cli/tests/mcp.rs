use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::Duration;

use serde_json::{Value, json};

const PATIENCE: Duration = Duration::from_secs(120);

struct Client {
    server: Child,
    input: Option<ChildStdin>,
    messages: Receiver<Value>,
}

impl Client {
    fn start() -> Self {
        let mut server = Command::new(env!("CARGO_BIN_EXE_fractique"))
            .arg("mcp")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("the server starts");
        let input = server.stdin.take();
        let output = BufReader::new(server.stdout.take().expect("a pipe"));
        let (sender, messages) = mpsc::channel();
        thread::spawn(move || {
            for line in output.lines().map_while(Result::ok) {
                let message = serde_json::from_str(&line).expect("every line is a JSON message");
                if sender.send(message).is_err() {
                    break;
                }
            }
        });
        let mut client = Self {
            server,
            input,
            messages,
        };
        client.send(&json!({
            "jsonrpc": "2.0", "id": 0, "method": "initialize",
            "params": { "protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": { "name": "test", "version": "0" } },
        }));
        assert_eq!(client.next()["result"]["serverInfo"]["name"], "fractique");
        client.send(&json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }));
        client
    }

    fn send(&mut self, message: &Value) {
        let input = self.input.as_mut().expect("the input is open");
        writeln!(input, "{message}").expect("the server reads");
        input.flush().expect("the server reads");
    }

    fn call(&mut self, id: u64, token: Option<&str>, name: &str, arguments: Value) {
        let mut params = json!({ "name": name, "arguments": arguments });
        if let Some(token) = token {
            params["_meta"] = json!({ "progressToken": token });
        }
        self.send(&json!({ "jsonrpc": "2.0", "id": id, "method": "tools/call", "params": params }));
    }

    fn next(&mut self) -> Value {
        self.messages
            .recv_timeout(PATIENCE)
            .expect("the server answers in time")
    }

    fn rest(mut self) -> Vec<Value> {
        drop(self.input.take());
        let mut rest = Vec::new();
        while let Ok(message) = self.messages.recv_timeout(PATIENCE) {
            rest.push(message);
        }
        assert!(self.server.wait().expect("the server ends").success());
        rest
    }
}

fn text_of(answer: &Value) -> &str {
    assert_eq!(answer["result"]["isError"], false, "{answer}");
    answer["result"]["content"][0]["text"].as_str().expect("a text")
}

#[test]
fn the_wallpaper_tools_answer_over_stdio() {
    let mut client = Client::start();
    client.send(&json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/list" }));
    let listed = client.next();
    let names: Vec<&str> = listed["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|tool| tool["name"].as_str())
        .collect();
    assert!(names.contains(&"list_wallpaper_styles") && names.contains(&"randomize_wallpaper"));

    client.call(2, None, "list_wallpaper_styles", json!({}));
    let styles = client.next();
    assert!(text_of(&styles).contains("waves") && text_of(&styles).contains("glass"));

    client.call(
        3,
        None,
        "randomize_wallpaper",
        json!({ "seed": 7, "layout": "waves", "material": "glass", "dark": true }),
    );
    let made = client.next();
    let scene: Value = serde_json::from_str(text_of(&made)).unwrap();
    assert_eq!(scene["name"], "wallpaper-7");
    assert!(
        scene["sheets"]["stack"]
            .as_array()
            .is_some_and(|stack| !stack.is_empty())
    );

    client.call(
        4,
        None,
        "render_scene",
        json!({ "scene": scene, "width": 90, "height": 195 }),
    );
    assert_eq!(client.next()["result"]["content"][0]["mimeType"], "image/png");
    let by_seed = json!({ "wallpaper_seed": 7, "layout": "waves", "material": "glass", "dark": true, "width": 90, "height": 195 });
    client.call(5, None, "render_scene", by_seed.clone());
    let first = client.next();
    client.call(6, None, "render_scene", by_seed);
    let second = client.next();
    assert_eq!(first["result"]["content"][0]["mimeType"], "image/png");
    assert_eq!(
        first["result"]["content"][0]["data"],
        second["result"]["content"][0]["data"]
    );
    assert!(client.rest().is_empty());
}

#[test]
fn progress_arrives_while_a_render_runs() {
    let mut client = Client::start();
    client.call(
        1,
        Some("first"),
        "render_scene",
        json!({ "preset": "aurora-spiral", "width": 320, "height": 180, "quality": "standard" }),
    );
    let mut last = 0;
    let answer = loop {
        let message = client.next();
        if message.get("id").is_some() {
            break message;
        }
        assert_eq!(message["method"], "notifications/progress");
        assert_eq!(message["params"]["progressToken"], "first");
        assert_eq!(message["params"]["total"], 100);
        let progress = message["params"]["progress"].as_u64().unwrap();
        assert!(progress > last, "{progress} after {last}");
        last = progress;
    };
    assert_eq!(last, 100);
    assert_eq!(answer["id"], 1);
    assert_eq!(answer["result"]["content"][0]["mimeType"], "image/png");
    assert!(client.rest().is_empty(), "nothing follows the answer");
}

#[test]
fn a_cancelled_render_stops_early_and_is_not_answered() {
    let mut client = Client::start();
    client.call(
        1,
        Some("long"),
        "render_scene",
        json!({ "preset": "aurora-spiral", "width": 1024, "height": 1024, "quality": "ultra" }),
    );
    let first = client.next();
    assert_eq!(first["method"], "notifications/progress");
    client.send(&json!({ "jsonrpc": "2.0", "id": 2, "method": "ping" }));
    client.send(&json!({
        "jsonrpc": "2.0", "method": "notifications/cancelled",
        "params": { "requestId": 1, "reason": "the test has seen enough" },
    }));
    client.call(3, None, "list_presets", json!({}));

    let mut seen = first["params"]["progress"].as_u64().unwrap();
    let mut answered = Vec::new();
    while answered.len() < 2 {
        let message = client.next();
        match message.get("id").and_then(Value::as_u64) {
            Some(id) => answered.push(id),
            None => seen = message["params"]["progress"].as_u64().unwrap(),
        }
    }
    assert_eq!(
        answered,
        [2, 3],
        "a ping is answered during the render, the next call after it"
    );
    assert!(seen < 50, "the render went on to {seen}% after it was cancelled");
    assert!(client.rest().is_empty(), "a cancelled request gets no answer");
}

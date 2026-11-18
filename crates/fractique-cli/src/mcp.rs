use std::io::{self, BufRead, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;

use fractique_core::Control;
use fractique_mcp::{Host, answer, failure, handle, outcome, reporter, tool_call};
use serde_json::Value;

struct Call {
    id: Value,
    params: Value,
    cancelled: Arc<AtomicBool>,
}

struct Server<W> {
    out: Mutex<W>,
    calls: Mutex<Vec<(Value, Arc<AtomicBool>)>>,
}

impl<W: Write + Send> Server<W> {
    // A cancelled request gets no further message: the flag is set and read under the lock of the output.
    fn send(&self, cancelled: Option<&AtomicBool>, message: &Value) -> io::Result<()> {
        let mut out = self.out.lock().unwrap_or_else(PoisonError::into_inner);
        if cancelled.is_some_and(|flag| flag.load(Ordering::Relaxed)) {
            return Ok(());
        }
        writeln!(out, "{message}").and_then(|()| out.flush())
    }

    fn cancel(&self, id: &Value) {
        let calls = self.calls.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some((_, flag)) = calls.iter().find(|(call, _)| call == id) {
            let _out = self.out.lock().unwrap_or_else(PoisonError::into_inner);
            flag.store(true, Ordering::Relaxed);
        }
    }

    fn receive(&self, line: &str, queue: &mpsc::Sender<Call>) -> io::Result<()> {
        let message: Value = match serde_json::from_str(line) {
            Ok(message) => message,
            Err(e) => return self.send(None, &failure(Value::Null, -32700, format!("not JSON: {e}"))),
        };
        let id = message.get("id").cloned();
        let params = message.get("params").cloned().unwrap_or(Value::Null);
        let Some(method) = message.get("method").and_then(Value::as_str) else {
            if message.is_array() {
                self.send(None, &failure(Value::Null, -32600, "batches are not supported".into()))?;
            }
            return Ok(());
        };
        match (method, id) {
            ("notifications/cancelled", _) => {
                if let Some(id) = params.get("requestId") {
                    self.cancel(id);
                }
                Ok(())
            }
            ("tools/call", Some(id)) => {
                let cancelled = Arc::new(AtomicBool::new(false));
                self.calls
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .push((id.clone(), cancelled.clone()));
                let _ = queue.send(Call { id, params, cancelled });
                Ok(())
            }
            (_, None) => Ok(()),
            (_, Some(id)) => self.send(
                None,
                &match handle(method, &params, &Host::LOCAL) {
                    Ok(result) => answer(&id, result),
                    Err(e) => failure(id, e.code, e.message),
                },
            ),
        }
    }

    fn work(&self, request: &Call) -> io::Result<()> {
        let flag = &*request.cancelled;
        let call = match tool_call(&request.params) {
            Ok(call) => call,
            Err(e) => return self.send(Some(flag), &failure(request.id.clone(), e.code, e.message)),
        };
        let report = call.token.map(|token| {
            reporter(token, |message| {
                let _ = self.send(Some(flag), &message);
            })
        });
        let control = Control {
            cancel: Some(flag),
            progress: match &report {
                Some(report) => Some(report),
                None => None,
            },
        };
        let result = outcome(fractique_mcp::call(call.name, call.arguments, &Host::LOCAL, &control));
        self.send(Some(flag), &answer(&request.id, result))
    }

    fn run(&self, calls: Receiver<Call>) -> io::Result<()> {
        for call in calls {
            let done = if call.cancelled.load(Ordering::Relaxed) {
                Ok(())
            } else {
                self.work(&call)
            };
            self.calls
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .retain(|(_, flag)| !Arc::ptr_eq(flag, &call.cancelled));
            done?;
        }
        Ok(())
    }
}

fn serve_on(input: impl BufRead, output: impl Write + Send) -> Result<(), String> {
    let server = Server {
        out: Mutex::new(output),
        calls: Mutex::new(Vec::new()),
    };
    let (queue, calls) = mpsc::channel();
    thread::scope(|scope| {
        let worker = scope.spawn(|| server.run(calls));
        let read = input.lines().try_for_each(|line| {
            let line = line.map_err(|e| format!("stdin: {e}"))?;
            if line.trim().is_empty() {
                return Ok(());
            }
            server.receive(&line, &queue).map_err(|e| format!("stdout: {e}"))
        });
        drop(queue);
        let worked = worker.join().map_err(|_| "the worker stopped".to_owned())?;
        read.and(worked.map_err(|e| format!("stdout: {e}")))
    })
}

pub fn serve() -> Result<(), String> {
    serve_on(io::stdin().lock(), io::stdout())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use fractique_core::wallpaper::{Layout, Material, random_wallpaper};
    use fractique_core::{Quality, RenderOptions, Scene, file, presets, render};
    use fractique_mcp::{PROTOCOLS, base64, names, wallpaper_request};
    use serde_json::json;

    use super::*;

    fn exchange(lines: &[String]) -> Vec<Value> {
        let mut out = Vec::new();
        serve_on(lines.join("\n").as_bytes(), &mut out).expect("the server runs");
        String::from_utf8(out)
            .expect("text")
            .lines()
            .map(|line| serde_json::from_str(line).expect("a JSON message"))
            .collect()
    }

    fn answer(line: &str) -> Option<Value> {
        exchange(&[line.to_owned()]).pop()
    }

    fn ask(request: Value) -> Value {
        answer(&request.to_string()).expect("an answer")
    }

    fn call_tool(name: &str, arguments: Value) -> Value {
        ask(json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": { "name": name, "arguments": arguments } }))["result"].clone()
    }

    #[test]
    fn initialize_agrees_on_a_version() {
        let known = ask(
            json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": { "protocolVersion": "2024-11-05" } }),
        );
        assert_eq!(known["result"]["protocolVersion"], "2024-11-05");
        let unknown = ask(
            json!({ "jsonrpc": "2.0", "id": 2, "method": "initialize", "params": { "protocolVersion": "1999-01-01" } }),
        );
        assert_eq!(unknown["result"]["protocolVersion"], PROTOCOLS[0]);
        assert_eq!(unknown["id"], 2);
    }

    #[test]
    fn notifications_get_no_answer() {
        assert!(answer(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#).is_none());
        assert_eq!(
            ask(json!({ "jsonrpc": "2.0", "id": "a", "method": "nope" }))["error"]["code"],
            -32601
        );
        assert_eq!(answer("{").unwrap()["error"]["code"], -32700);
    }

    #[test]
    fn every_listed_tool_has_a_schema() {
        let listed = ask(json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/list" }));
        let tools = listed["result"]["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 12);
        for tool in tools {
            assert_eq!(tool["inputSchema"]["type"], "object", "{}", tool["name"]);
            let unknown = call_tool(tool["name"].as_str().unwrap(), json!({ "scene": 5 }));
            assert!(unknown["content"].is_array(), "{}", tool["name"]);
        }
    }

    #[test]
    fn a_scene_goes_round() {
        let scene_text = call_tool("randomize_scene", json!({ "seed": 3 }))["content"][0]["text"]
            .as_str()
            .unwrap()
            .to_owned();
        let scene: Value = serde_json::from_str(&scene_text).unwrap();
        assert_eq!(call_tool("validate_scene", json!({ "scene": scene }))["isError"], false);
        assert_eq!(
            call_tool("validate_scene", json!({ "scene": scene_text }))["isError"],
            false
        );
        assert_eq!(
            call_tool("validate_scene", json!({ "scene": { "version": 1 } }))["isError"],
            true
        );
        let mutated = call_tool("mutate_scene", json!({ "scene": scene, "seed": 1 }));
        assert_eq!(mutated["isError"], false);
        assert_ne!(mutated["content"][0]["text"], scene_text);
        assert_eq!(call_tool("fit_camera", json!({ "scene": scene }))["isError"], false);
    }

    #[test]
    fn render_returns_a_png() {
        let result = call_tool(
            "render_scene",
            json!({ "preset": "aurora-spiral", "width": 64, "height": 36, "quality": "draft" }),
        );
        assert_eq!(result["isError"], false, "{result}");
        assert_eq!(result["content"][0]["mimeType"], "image/png");
        assert!(
            result["content"][0]["data"]
                .as_str()
                .unwrap()
                .starts_with("iVBORw0KGgo")
        );
        assert!(result["content"][1]["text"].as_str().unwrap().contains("hash "));

        let both = call_tool("render_scene", json!({ "preset": "aurora-spiral", "random_seed": 1 }));
        assert_eq!(both["isError"], true);
        let large = call_tool(
            "render_scene",
            json!({ "preset": "aurora-spiral", "width": 4000, "height": 4000 }),
        );
        assert!(large["content"][0]["text"].as_str().unwrap().contains("save_to"));
    }

    #[test]
    fn wallpapers_are_listed_and_made() {
        let styles = call_tool("list_wallpaper_styles", json!({}));
        let listed = styles["content"][0]["text"].as_str().unwrap();
        for name in names(&Layout::ALL).into_iter().chain(names(&Material::ALL)) {
            assert!(listed.contains(name), "{name}");
        }

        let make = |arguments: Value| {
            let made = call_tool("randomize_wallpaper", arguments);
            assert_eq!(made["isError"], false, "{made}");
            Scene::from_json(made["content"][0]["text"].as_str().unwrap()).expect("a valid scene")
        };
        let plain = make(json!({ "seed": 7 }));
        assert_eq!(plain, make(json!({ "seed": 7 })));
        assert!(plain.sheets.is_some() && plain.layers.is_empty());
        let request = wallpaper_request(Some("waves".into()), Some("glass".into()), 800, 600, true).unwrap();
        assert_eq!(
            make(
                json!({ "seed": 7, "layout": "waves", "material": "glass", "dark": true, "width": 800, "height": 600 })
            ),
            Scene::from_json(&random_wallpaper(7, &request).to_json()).unwrap()
        );
        assert_ne!(plain, make(json!({ "seed": 7, "dark": true })));

        let unknown = call_tool("randomize_wallpaper", json!({ "seed": 7, "layout": "spirals" }));
        assert_eq!(unknown["isError"], true);
        assert!(unknown["content"][0]["text"].as_str().unwrap().contains("spirals"));
    }

    fn hash_of(result: &Value) -> String {
        assert_eq!(result["isError"], false, "{result}");
        let note = result["content"][1]["text"].as_str().unwrap();
        note.split("hash ").nth(1).unwrap()[..16].to_owned()
    }

    #[test]
    fn a_wallpaper_renders_from_its_seed() {
        let request = wallpaper_request(None, Some("paper".into()), 90, 160, true).unwrap();
        let options = RenderOptions::new(90, 160, Quality::Mobile);
        let expected = render(&random_wallpaper(11, &request), &options, &Control::default()).unwrap();
        let rendered = call_tool(
            "render_scene",
            json!({ "wallpaper_seed": 11, "material": "paper", "dark": true, "width": 90, "height": 160 }),
        );
        assert_eq!(hash_of(&rendered), format!("{:016x}", expected.hash()));
        let both = call_tool("render_scene", json!({ "wallpaper_seed": 11, "random_seed": 11 }));
        assert_eq!(both["isError"], true);
    }

    #[test]
    fn a_frcq_renders_as_its_scene() {
        let scene = presets::get("aurora-spiral").unwrap();
        let shown = serde_json::to_value(&scene).unwrap();
        let document = json!({ "fractique": 1, "scene": { "ignored": true }, "shown": shown });
        let packed = file::pack(&document);
        let path = std::env::temp_dir().join(format!("fractique-mcp-{}.frcq", std::process::id()));
        fs::write(&path, &packed).unwrap();

        let options = json!({ "width": 64, "height": 36, "quality": "draft" });
        let with = |name: &str, value: Value| {
            let mut arguments = options.clone();
            arguments[name] = value;
            call_tool("render_scene", arguments)
        };
        let expected = hash_of(&with("preset", "aurora-spiral".into()));
        assert_eq!(hash_of(&with("file", path.to_string_lossy().into())), expected);
        assert_eq!(hash_of(&with("frcq", base64(&packed).into())), expected);
        assert_eq!(hash_of(&with("scene", document.clone())), expected);
        assert_eq!(hash_of(&with("scene", document.to_string().into())), expected);
        fs::remove_file(&path).unwrap();

        assert_eq!(with("file", "/nowhere/missing.frcq".into())["isError"], true);
        assert_eq!(with("frcq", base64(b"FRCQ\x01nonsense").into())["isError"], true);
        assert_eq!(
            with("frcq", base64(shown.to_string().as_bytes()).into())["isError"],
            true
        );
    }

    #[test]
    fn progress_grows_and_ends_before_the_answer() {
        let request = json!({
            "jsonrpc": "2.0", "id": 5, "method": "tools/call",
            "params": {
                "_meta": { "progressToken": "render-5" },
                "name": "render_scene",
                "arguments": { "preset": "emerald-star", "width": 64, "height": 36, "quality": "draft" },
            },
        });
        let messages = exchange(&[request.to_string()]);
        let (answer, notes) = messages.split_last().unwrap();
        assert_eq!(answer["id"], 5);
        assert_eq!(answer["result"]["isError"], false);
        assert!(!notes.is_empty());
        let mut last = 0;
        for note in notes {
            assert_eq!(note["method"], "notifications/progress");
            assert!(note.get("id").is_none());
            assert_eq!(note["params"]["progressToken"], "render-5");
            assert_eq!(note["params"]["total"], 100);
            let progress = note["params"]["progress"].as_u64().unwrap();
            assert!(progress > last && progress <= 100, "{progress} after {last}");
            last = progress;
        }
        assert_eq!(last, 100);

        let mut silent = request;
        silent["params"]["_meta"] = json!({});
        assert_eq!(exchange(&[silent.to_string()]).len(), 1);
    }

    #[test]
    fn a_call_cancelled_before_it_starts_is_not_answered() {
        let render = |id: u64| {
            json!({
                "jsonrpc": "2.0", "id": id, "method": "tools/call",
                "params": { "name": "render_scene", "arguments": { "preset": "aurora-spiral", "quality": "standard" } },
            })
            .to_string()
        };
        let cancel = |id: Value| {
            json!({ "jsonrpc": "2.0", "method": "notifications/cancelled", "params": { "requestId": id, "reason": "test" } })
                .to_string()
        };
        let messages = exchange(&[render(1), render(2), cancel(2.into()), cancel("unknown".into())]);
        assert_eq!(messages.len(), 1, "{messages:?}");
        assert_eq!(messages[0]["id"], 1);
    }
}

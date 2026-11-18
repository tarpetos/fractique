use std::fs;
use std::io::{self, BufRead, Write};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;
use std::time::Instant;

use fractique_core::import::import_flames;
use fractique_core::random::{fit_camera, mutate, random_scene};
use fractique_core::wallpaper::{Layout, Material, random_wallpaper};
use fractique_core::{Control, Kind, Quality, RenderOptions, Scene, file, presets, render};
use serde_json::{Value, json};

const PROTOCOLS: [&str; 3] = ["2025-06-18", "2025-03-26", "2024-11-05"];
const INLINE_PIXELS: u64 = 1_048_576;
const WALLPAPER_SIZE: (u64, u64) = (1290, 2796);
const SCENE_FORMAT: &str = include_str!("../../../docs/scene-format.md");

struct RpcError {
    code: i64,
    message: String,
}

fn tool(name: &str, description: &str, properties: Value, required: &[&str]) -> Value {
    json!({
        "name": name,
        "description": description,
        "inputSchema": { "type": "object", "properties": properties, "required": required },
    })
}

fn tools() -> Value {
    let scene = json!({ "type": "object", "description": "A scene, as returned by the other tools. describe_scene_format explains every field." });
    let layout =
        json!({ "type": "string", "enum": names(&Layout::ALL), "description": "Left out, the seed picks one." });
    let material =
        json!({ "type": "string", "enum": names(&Material::ALL), "description": "Left out, the seed picks one." });
    let dark = json!({ "type": "boolean", "description": "The dark version of the same shapes. Default false." });
    let seed = json!({ "type": "integer", "minimum": 0, "description": "Any whole number. The same number always gives the same result." });
    json!([
        tool("list_presets", "Names of the built-in scenes.", json!({}), &[]),
        tool(
            "get_preset",
            "A built-in scene as JSON.",
            json!({ "name": { "type": "string" } }),
            &["name"]
        ),
        tool(
            "randomize_scene",
            "The random scene of a seed, with its camera already framing the flame.",
            json!({ "seed": seed }),
            &["seed"]
        ),
        tool(
            "list_wallpaper_styles",
            "The layouts and the materials a wallpaper can have.",
            json!({}),
            &[]
        ),
        tool(
            "randomize_wallpaper",
            "The wallpaper of a seed: a scene of flat sheets with lit edges and soft shadows instead of a flame. The shapes are laid out for the proportions of width and height, so render it with the same proportions.",
            json!({
                "seed": seed,
                "layout": layout,
                "material": material,
                "dark": dark,
                "width": { "type": "integer", "minimum": 1, "description": "Default 1290." },
                "height": { "type": "integer", "minimum": 1, "description": "Default 2796." },
            }),
            &["seed"],
        ),
        tool(
            "mutate_scene",
            "A variant of a scene: every transform is nudged. The camera is kept; call fit_camera afterwards if the flame moved out of view.",
            json!({ "scene": scene, "seed": seed, "strength": { "type": "number", "minimum": 0, "maximum": 1, "description": "0 changes nothing, 1 leaves it barely related. Default 0.3." } }),
            &["scene", "seed"],
        ),
        tool(
            "fit_camera",
            "The scene with every layer's camera moved to frame its flame.",
            json!({ "scene": scene }),
            &["scene"]
        ),
        tool(
            "validate_scene",
            "Checks a scene and names the field that is wrong.",
            json!({ "scene": scene }),
            &["scene"]
        ),
        tool(
            "describe_scene_format",
            "The reference of the scene format: every field, every variation and its formula.",
            json!({}),
            &[]
        ),
        tool(
            "list_variations",
            "The variations a transform can use, with their parameters and defaults.",
            json!({}),
            &[]
        ),
        tool(
            "import_flame",
            "Converts a .flame file (flam3, Apophysis XML) to a scene.",
            json!({ "xml": { "type": "string", "description": "The text of the file." }, "index": { "type": "integer", "minimum": 0, "description": "Which flame of the file, from 0. Default 0." } }),
            &["xml"],
        ),
        tool(
            "render_scene",
            "Renders a scene and returns the image, so you can look at it. Give exactly one of scene, preset, random_seed, wallpaper_seed, file or frcq. The same input always gives the same pixels; the returned hash identifies them. Images above about one megapixel are not returned inline: give save_to. A request with a progress token gets notifications/progress while it renders, and notifications/cancelled stops it.",
            json!({
                "scene": scene,
                "preset": { "type": "string", "description": "Name of a built-in scene." },
                "random_seed": seed,
                "wallpaper_seed": { "type": "integer", "minimum": 0, "description": "Renders the wallpaper of this seed, laid out for width and height. layout, material and dark choose its style." },
                "layout": layout,
                "material": material,
                "dark": dark,
                "file": { "type": "string", "description": "Path of a scene file to read: a .frcq of the app, or JSON." },
                "frcq": { "type": "string", "description": "The bytes of a .frcq file, in base64." },
                "width": { "type": "integer", "minimum": 1, "maximum": 16384, "description": "Default 640." },
                "height": { "type": "integer", "minimum": 1, "maximum": 16384, "description": "Default 360." },
                "quality": { "type": "string", "enum": ["draft", "mobile", "standard", "high", "ultra"], "description": "Default mobile: enough to judge a scene. Use high for a final image." },
                "save_to": { "type": "string", "description": "Path of a PNG file to write as well." },
            }),
            &[],
        ),
    ])
}

fn scene_of(arguments: &Value) -> Result<Scene, String> {
    let scene = match arguments.get("scene") {
        // Some clients hand an object argument over as its JSON text.
        Some(Value::String(text)) => return crate::scene_from_bytes(text.as_bytes()),
        Some(value) => {
            serde_json::from_value::<Scene>(crate::shown(value.clone())).map_err(|e| format!("not a scene: {e}"))?
        }
        None => return Err("the scene argument is missing".into()),
    };
    scene.validate().map_err(|e| e.to_string())?;
    Ok(scene)
}

fn whole(arguments: &Value, name: &str) -> Result<Option<u64>, String> {
    match arguments.get(name) {
        None | Some(Value::Null) => Ok(None),
        Some(value) => value
            .as_u64()
            .map(Some)
            .ok_or_else(|| format!("{name} must be a whole number, 0 or above")),
    }
}

fn word<'a>(arguments: &'a Value, name: &str) -> Result<Option<&'a str>, String> {
    match arguments.get(name) {
        None | Some(Value::Null) => Ok(None),
        Some(value) => value.as_str().map(Some).ok_or_else(|| format!("{name} must be a text")),
    }
}

fn names<T>(all: &[(&'static str, T)]) -> Vec<&'static str> {
    all.iter().map(|(name, _)| *name).collect()
}

fn wallpaper(arguments: &Value, seed: u64, width: u64, height: u64) -> Result<Scene, String> {
    let side = |name: &str, value: u64| u32::try_from(value).map_err(|_| format!("{name} is too large"));
    let request = crate::wallpaper_request(
        word(arguments, "layout")?.map(str::to_owned),
        word(arguments, "material")?.map(str::to_owned),
        side("width", width)?.max(1),
        side("height", height)?.max(1),
        arguments.get("dark").and_then(Value::as_bool).unwrap_or(false),
    )?;
    Ok(random_wallpaper(seed, &request))
}

fn text(content: impl Into<String>) -> Vec<Value> {
    vec![json!({ "type": "text", "text": content.into() })]
}

fn base64(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let word = u32::from(chunk[0]) << 16
            | u32::from(*chunk.get(1).unwrap_or(&0)) << 8
            | u32::from(*chunk.get(2).unwrap_or(&0));
        for (i, shift) in [18, 12, 6, 0].into_iter().enumerate() {
            out.push(if i <= chunk.len() {
                TABLE[(word >> shift & 63) as usize] as char
            } else {
                '='
            });
        }
    }
    out
}

fn unbase64(text: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(text.len() / 4 * 3);
    let (mut word, mut bits) = (0u32, 0);
    for byte in text.bytes().filter(|byte| !byte.is_ascii_whitespace() && *byte != b'=') {
        let six = match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52,
            b'+' | b'-' => 62,
            b'/' | b'_' => 63,
            _ => return None,
        };
        word = word << 6 | u32::from(six);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((word >> bits) as u8);
        }
    }
    Some(out)
}

const SOURCES: [&str; 6] = ["scene", "preset", "random_seed", "wallpaper_seed", "file", "frcq"];

fn render_scene(arguments: &Value, control: &Control<'_>) -> Result<Vec<Value>, String> {
    let side = |name: &str, default: u64| -> Result<u32, String> {
        u32::try_from(whole(arguments, name)?.unwrap_or(default)).map_err(|_| format!("{name} is too large"))
    };
    let (width, height) = (side("width", 640)?, side("height", 360)?);
    let given: Vec<&str> = SOURCES
        .into_iter()
        .filter(|name| arguments.get(name).is_some_and(|value| !value.is_null()))
        .collect();
    let scene = match given[..] {
        ["scene"] => scene_of(arguments)?,
        ["preset"] => {
            let name = word(arguments, "preset")?.unwrap_or_default();
            presets::get(name).ok_or_else(|| format!("no preset is called '{name}'"))?
        }
        ["random_seed"] => random_scene(whole(arguments, "random_seed")?.unwrap_or_default()),
        ["wallpaper_seed"] => wallpaper(
            arguments,
            whole(arguments, "wallpaper_seed")?.unwrap_or_default(),
            u64::from(width),
            u64::from(height),
        )?,
        ["file"] => crate::load(Path::new(word(arguments, "file")?.unwrap_or_default()))?,
        ["frcq"] => {
            let bytes = word(arguments, "frcq")?
                .and_then(unbase64)
                .ok_or("frcq must be the bytes of a .frcq file in base64")?;
            if !file::is_packed(&bytes) {
                return Err("frcq does not hold a .frcq file; a scene as JSON goes in scene".into());
            }
            crate::scene_from_bytes(&bytes)?
        }
        _ => return Err(format!("give exactly one of {}", SOURCES.join(", "))),
    };
    let quality = arguments.get("quality").and_then(Value::as_str).unwrap_or("mobile");
    let quality = Quality::parse(quality).ok_or_else(|| format!("no quality is called '{quality}'"))?;
    let save_to = arguments.get("save_to").and_then(Value::as_str);
    let inline = u64::from(width) * u64::from(height) <= INLINE_PIXELS;
    if !inline && save_to.is_none() {
        return Err(format!(
            "{width}x{height} is too large to return inline: give save_to, or render a smaller preview"
        ));
    }

    let options = RenderOptions::new(width, height, quality);
    let started = Instant::now();
    let image = render(&scene, &options, control).map_err(|e| e.to_string())?;
    let png = crate::encode_png(&image)?;
    let mut note = format!(
        "{width}x{height}, {} samples per pixel, {:.1} s, hash {:016x}",
        options.samples_per_pixel,
        started.elapsed().as_secs_f64(),
        image.hash()
    );
    if let Some(path) = save_to {
        fs::write(path, &png).map_err(|e| format!("{path}: {e}"))?;
        note.push_str(&format!(", saved to {path}"));
    }
    let mut content = Vec::new();
    if inline {
        content.push(json!({ "type": "image", "data": base64(&png), "mimeType": "image/png" }));
    }
    content.push(json!({ "type": "text", "text": note }));
    Ok(content)
}

fn call(name: &str, arguments: &Value, control: &Control<'_>) -> Result<Vec<Value>, String> {
    match name {
        "list_presets" => Ok(text(presets::names().join("\n"))),
        "get_preset" => {
            let name = arguments
                .get("name")
                .and_then(Value::as_str)
                .ok_or("the name argument is missing")?;
            let scene = presets::get(name)
                .ok_or_else(|| format!("no preset is called '{name}'; list_presets has the names"))?;
            Ok(text(scene.to_json()))
        }
        "randomize_scene" => {
            let seed = whole(arguments, "seed")?.ok_or("the seed argument is missing")?;
            Ok(text(random_scene(seed).to_json()))
        }
        "list_wallpaper_styles" => Ok(text(format!(
            "layouts: {}\nmaterials: {}",
            names(&Layout::ALL).join(", "),
            names(&Material::ALL).join(", ")
        ))),
        "randomize_wallpaper" => {
            let seed = whole(arguments, "seed")?.ok_or("the seed argument is missing")?;
            let width = whole(arguments, "width")?.unwrap_or(WALLPAPER_SIZE.0);
            let height = whole(arguments, "height")?.unwrap_or(WALLPAPER_SIZE.1);
            Ok(text(wallpaper(arguments, seed, width, height)?.to_json()))
        }
        "mutate_scene" => {
            let seed = whole(arguments, "seed")?.ok_or("the seed argument is missing")?;
            let strength = arguments.get("strength").and_then(Value::as_f64).unwrap_or(0.3);
            Ok(text(mutate(&scene_of(arguments)?, seed, strength).to_json()))
        }
        "fit_camera" => {
            let mut scene = scene_of(arguments)?;
            for (index, layer) in scene.layers.iter_mut().enumerate() {
                let (camera, _) = fit_camera(&layer.flame)
                    .ok_or_else(|| format!("layer {index}: the flame draws nothing to frame"))?;
                layer.flame.camera = camera;
            }
            Ok(text(scene.to_json()))
        }
        "validate_scene" => scene_of(arguments).map(|_| text("The scene is valid.")),
        "describe_scene_format" => Ok(text(SCENE_FORMAT)),
        "list_variations" => {
            let lines: Vec<String> = Kind::ALL
                .iter()
                .map(|kind| {
                    let params: Vec<String> = kind
                        .params()
                        .iter()
                        .map(|(name, default)| format!("{name}={default}"))
                        .collect();
                    format!("{} {}", kind.name(), params.join(" ")).trim_end().to_owned()
                })
                .collect();
            Ok(text(lines.join("\n")))
        }
        "import_flame" => {
            let xml = arguments
                .get("xml")
                .and_then(Value::as_str)
                .ok_or("the xml argument is missing")?;
            let index = whole(arguments, "index")?.unwrap_or(0) as usize;
            let mut flames = import_flames(xml).map_err(|e| e.to_string())?;
            let count = flames.len();
            if index >= count {
                return Err(format!(
                    "the file holds {count} flames; index goes from 0 to {}",
                    count - 1
                ));
            }
            let flame = flames.swap_remove(index);
            let mut content = text(flame.scene.to_json());
            let mut notes = flame.warnings;
            if count > 1 {
                notes.push(format!("flame {index} of {count} in the file"));
            }
            if !notes.is_empty() {
                content.push(json!({ "type": "text", "text": notes.join("\n") }));
            }
            Ok(content)
        }
        "render_scene" => render_scene(arguments, control),
        _ => Err(format!("no tool is called '{name}'")),
    }
}

fn handle(method: &str, params: &Value) -> Result<Value, RpcError> {
    match method {
        "initialize" => {
            let asked = params.get("protocolVersion").and_then(Value::as_str).unwrap_or("");
            let version = PROTOCOLS.iter().find(|v| **v == asked).unwrap_or(&PROTOCOLS[0]);
            Ok(json!({
                "protocolVersion": version,
                "capabilities": { "tools": {} },
                "serverInfo": { "name": "fractique", "title": "Fractique", "version": env!("CARGO_PKG_VERSION") },
                "instructions": "Fractique renders fractal flames and wallpapers from scenes (JSON). A usual loop: get_preset, randomize_scene or randomize_wallpaper, render_scene to look at it, change the scene (describe_scene_format explains the fields), render again. The same scene always renders to the same pixels.",
            }))
        }
        "ping" => Ok(json!({})),
        "tools/list" => Ok(json!({ "tools": tools() })),
        _ => Err(RpcError {
            code: -32601,
            message: format!("method '{method}' is not supported"),
        }),
    }
}

fn failure(id: Value, code: i64, message: String) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

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
                &match handle(method, &params) {
                    Ok(result) => json!({ "jsonrpc": "2.0", "id": id, "result": result }),
                    Err(e) => failure(id, e.code, e.message),
                },
            ),
        }
    }

    fn work(&self, request: &Call) -> io::Result<()> {
        let flag = &*request.cancelled;
        let Some(name) = request.params.get("name").and_then(Value::as_str) else {
            return self.send(
                Some(flag),
                &failure(request.id.clone(), -32602, "tools/call needs a name".into()),
            );
        };
        let empty = json!({});
        let arguments = request
            .params
            .get("arguments")
            .filter(|a| !a.is_null())
            .unwrap_or(&empty);
        let token = request
            .params
            .pointer("/_meta/progressToken")
            .filter(|token| token.is_string() || token.is_i64() || token.is_u64());
        // Progress must only grow and the render threads report in any order: the lock keeps the percents in order.
        let reported = Mutex::new(0u64);
        let report = |share: f64| {
            let percent = (share.clamp(0.0, 1.0) * 100.0) as u64;
            let mut last = reported.lock().unwrap_or_else(PoisonError::into_inner);
            if percent > *last {
                *last = percent;
                let _ = self.send(
                    Some(flag),
                    &json!({
                        "jsonrpc": "2.0",
                        "method": "notifications/progress",
                        "params": { "progressToken": token, "progress": percent, "total": 100, "message": "rendering" },
                    }),
                );
            }
        };
        let control = Control {
            cancel: Some(flag),
            progress: token.map(|_| &report as &(dyn Fn(f64) + Sync)),
        };
        let result = match call(name, arguments, &control) {
            Ok(content) => json!({ "content": content, "isError": false }),
            Err(message) => json!({ "content": text(message), "isError": true }),
        };
        self.send(
            Some(flag),
            &json!({ "jsonrpc": "2.0", "id": request.id, "result": result }),
        )
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
    fn base64_pads() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
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
    fn base64_comes_back() {
        let bytes: Vec<u8> = (0..=255).collect();
        for length in [0, 1, 2, 3, 254, 255, 256] {
            assert_eq!(unbase64(&base64(&bytes[..length])).as_deref(), Some(&bytes[..length]));
        }
        assert_eq!(unbase64("Zm9v\nYmFy").as_deref(), Some(&b"foobar"[..]));
        assert!(unbase64("Zm9v!").is_none());
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
        let request = crate::wallpaper_request(Some("waves".into()), Some("glass".into()), 800, 600, true).unwrap();
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
        let request = crate::wallpaper_request(None, Some("paper".into()), 90, 160, true).unwrap();
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

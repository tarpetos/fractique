//! MCP server over stdin and stdout: one JSON-RPC message per line, answered as it arrives.

use std::fs;
use std::io::{BufRead, Write};
use std::time::Instant;

use fractique_core::import::import_flames;
use fractique_core::random::{fit_camera, mutate, random_scene};
use fractique_core::{Control, Kind, Quality, RenderOptions, Scene, presets, render};
use serde_json::{Value, json};

const PROTOCOLS: [&str; 3] = ["2025-06-18", "2025-03-26", "2024-11-05"];
// Above this an image is too heavy to hand to a model inline: it is saved to a file instead.
const INLINE_PIXELS: u64 = 1_048_576;
const SCENE_FORMAT: &str = include_str!("../../../docs/scene-format.md");

struct RpcError {
    code: i64,
    message: String,
}

fn invalid(message: impl Into<String>) -> RpcError {
    RpcError {
        code: -32602,
        message: message.into(),
    }
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
            "Renders a scene and returns the image, so you can look at it. Give exactly one of scene, preset or random_seed. The same input always gives the same pixels; the returned hash identifies them. Images above about one megapixel are not returned inline: give save_to.",
            json!({
                "scene": scene,
                "preset": { "type": "string", "description": "Name of a built-in scene." },
                "random_seed": seed,
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
        Some(Value::String(text)) => return Scene::from_json(text).map_err(|e| e.to_string()),
        Some(value) => serde_json::from_value::<Scene>(value.clone()).map_err(|e| format!("not a scene: {e}"))?,
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

fn render_scene(arguments: &Value) -> Result<Vec<Value>, String> {
    let preset = arguments.get("preset").and_then(Value::as_str);
    let random_seed = whole(arguments, "random_seed")?;
    let given = usize::from(arguments.get("scene").is_some())
        + usize::from(preset.is_some())
        + usize::from(random_seed.is_some());
    if given != 1 {
        return Err("give exactly one of scene, preset or random_seed".into());
    }
    let scene = match (preset, random_seed) {
        (Some(name), _) => presets::get(name).ok_or_else(|| format!("no preset is called '{name}'"))?,
        (_, Some(seed)) => random_scene(seed),
        _ => scene_of(arguments)?,
    };
    let side = |name: &str, default: u64| -> Result<u32, String> {
        u32::try_from(whole(arguments, name)?.unwrap_or(default)).map_err(|_| format!("{name} is too large"))
    };
    let (width, height) = (side("width", 640)?, side("height", 360)?);
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
    let image = render(&scene, &options, &Control::default()).map_err(|e| e.to_string())?;
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

fn call(name: &str, arguments: &Value) -> Result<Vec<Value>, String> {
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
        "render_scene" => render_scene(arguments),
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
                "serverInfo": { "name": "fractique", "title": "frcq.", "version": env!("CARGO_PKG_VERSION") },
                "instructions": "Fractique renders fractal flames from scenes (JSON). A usual loop: get_preset or randomize_scene, render_scene to look at it, change the scene (describe_scene_format explains the fields), render again. The same scene always renders to the same pixels.",
            }))
        }
        "ping" => Ok(json!({})),
        "tools/list" => Ok(json!({ "tools": tools() })),
        "tools/call" => {
            let name = params
                .get("name")
                .and_then(Value::as_str)
                .ok_or_else(|| invalid("tools/call needs a name"))?;
            let empty = json!({});
            let arguments = params.get("arguments").filter(|a| !a.is_null()).unwrap_or(&empty);
            Ok(match call(name, arguments) {
                Ok(content) => json!({ "content": content, "isError": false }),
                Err(message) => json!({ "content": text(message), "isError": true }),
            })
        }
        _ => Err(RpcError {
            code: -32601,
            message: format!("method '{method}' is not supported"),
        }),
    }
}

fn answer(line: &str) -> Option<Value> {
    let failure = |id: Value, code: i64, message: String| json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } });
    let message: Value = match serde_json::from_str(line) {
        Ok(message) => message,
        Err(e) => return Some(failure(Value::Null, -32700, format!("not JSON: {e}"))),
    };
    let id = message.get("id").cloned();
    let Some(method) = message.get("method").and_then(Value::as_str) else {
        // A response to something this server never asked, or not a request at all.
        return message
            .is_array()
            .then(|| failure(Value::Null, -32600, "batches are not supported".into()));
    };
    let result = handle(method, message.get("params").unwrap_or(&Value::Null));
    // A notification has no id and gets no answer.
    let id = id?;
    Some(match result {
        Ok(result) => json!({ "jsonrpc": "2.0", "id": id, "result": result }),
        Err(e) => failure(id, e.code, e.message),
    })
}

pub fn serve() -> Result<(), String> {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout().lock();
    for line in stdin.lock().lines() {
        let line = line.map_err(|e| format!("stdin: {e}"))?;
        if line.trim().is_empty() {
            continue;
        }
        if let Some(response) = answer(&line) {
            writeln!(stdout, "{response}")
                .and_then(|()| stdout.flush())
                .map_err(|e| format!("stdout: {e}"))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(tools.len(), 10);
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
}

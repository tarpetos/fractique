use std::fs;
use std::path::Path;
use std::sync::{Mutex, PoisonError};
use std::time::Instant;

use fractique_core::import::import_flames;
use fractique_core::random::{fit_camera, mutate, random_scene};
use fractique_core::wallpaper::{Layout, Material, Request, random_wallpaper};
use fractique_core::{Control, Image, Kind, Quality, RenderOptions, Scene, file, presets, render};
use serde_json::{Value, json};

pub const PROTOCOLS: [&str; 3] = ["2025-06-18", "2025-03-26", "2024-11-05"];
const INLINE_PIXELS: u64 = 1_048_576;
const WALLPAPER_SIZE: (u64, u64) = (1290, 2796);
const SCENE_FORMAT: &str = include_str!("../../../docs/scene-format.md");
const SOURCES: [&str; 6] = ["scene", "preset", "random_seed", "wallpaper_seed", "file", "frcq"];
const FILES: [&str; 2] = ["file", "save_to"];

#[derive(Clone, Debug)]
pub struct Limits {
    pub max_side: u32,
    pub max_pixels: u64,
    pub max_samples: u32,
    pub max_supersample: u32,
}

impl Limits {
    pub fn check(&self, options: &RenderOptions) -> Result<(), String> {
        let side = |v: u32| (1..=self.max_side).contains(&v);
        if !side(options.width) || !side(options.height) {
            return Err(format!(
                "width and height must be between 1 and {} on this server",
                self.max_side
            ));
        }
        if u64::from(options.width) * u64::from(options.height) > self.max_pixels {
            return Err(format!(
                "{}x{} is more than the {} pixels this server renders",
                options.width, options.height, self.max_pixels
            ));
        }
        if !(1..=self.max_samples).contains(&options.samples_per_pixel) {
            return Err(format!(
                "samples per pixel must be between 1 and {} on this server",
                self.max_samples
            ));
        }
        if !(1..=self.max_supersample).contains(&options.supersample) {
            return Err(format!(
                "supersample must be between 1 and {} on this server",
                self.max_supersample
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct Host {
    pub protocols: &'static [&'static str],
    pub files: bool,
    pub limits: Option<Limits>,
}

impl Host {
    pub const LOCAL: Self = Self {
        protocols: &PROTOCOLS,
        files: true,
        limits: None,
    };
}

pub struct RpcError {
    pub code: i64,
    pub message: String,
}

// A .frcq file of the app holds its session; the scene as it was on the screen is under "shown".
pub fn shown(mut document: Value) -> Value {
    match document.get_mut("shown").map(Value::take) {
        Some(scene) => scene,
        None => document,
    }
}

pub fn scene_from_bytes(bytes: &[u8]) -> Result<Scene, String> {
    let document = if file::is_packed(bytes) {
        Some(file::unpack(bytes)?)
    } else {
        serde_json::from_slice::<Value>(bytes)
            .ok()
            .filter(|document| document.get("fractique").is_some())
    };
    match document {
        Some(document) => Scene::from_json(&shown(document).to_string()),
        None => Scene::from_json(&String::from_utf8_lossy(bytes)),
    }
    .map_err(|e| e.to_string())
}

pub fn load(path: &Path) -> Result<Scene, String> {
    let fail = |e: String| format!("{}: {e}", path.display());
    scene_from_bytes(&fs::read(path).map_err(|e| fail(e.to_string()))?).map_err(fail)
}

pub fn encode_png(image: &Image) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    let mut encoder = png::Encoder::new(&mut bytes, image.width, image.height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
    writer.write_image_data(&image.rgba).map_err(|e| e.to_string())?;
    writer.finish().map_err(|e| e.to_string())?;
    Ok(bytes)
}

pub fn wallpaper_request(
    layout: Option<String>,
    material: Option<String>,
    width: u32,
    height: u32,
    dark: bool,
) -> Result<Request, String> {
    Ok(Request {
        layout: layout
            .map(|name| Layout::parse(&name).ok_or_else(|| format!("no layout is called '{name}'")))
            .transpose()?,
        material: material
            .map(|name| Material::parse(&name).ok_or_else(|| format!("no material is called '{name}'")))
            .transpose()?,
        ratio: f64::from(height) / f64::from(width),
        dark,
        hue: None,
    })
}

fn sources(host: &Host) -> Vec<&'static str> {
    SOURCES
        .into_iter()
        .filter(|name| host.files || !FILES.contains(name))
        .collect()
}

fn one_of(names: &[&str]) -> String {
    match names.split_last() {
        Some((last, rest)) if !rest.is_empty() => format!("{} or {last}", rest.join(", ")),
        _ => names.concat(),
    }
}

fn tool(name: &str, description: &str, properties: Value, required: &[&str]) -> Value {
    json!({
        "name": name,
        "description": description,
        "inputSchema": { "type": "object", "properties": properties, "required": required },
    })
}

pub fn tools(host: &Host) -> Value {
    let scene = json!({ "type": "object", "description": "A scene, as returned by the other tools. describe_scene_format explains every field." });
    let layout =
        json!({ "type": "string", "enum": names(&Layout::ALL), "description": "Left out, the seed picks one." });
    let material =
        json!({ "type": "string", "enum": names(&Material::ALL), "description": "Left out, the seed picks one." });
    let dark = json!({ "type": "boolean", "description": "The dark version of the same shapes. Default false." });
    let seed = json!({ "type": "integer", "minimum": 0, "description": "Any whole number. The same number always gives the same result." });
    let side = host.limits.as_ref().map_or(16384, |limits| limits.max_side.min(16384));
    let mut render = json!({
            "scene": scene,
            "preset": { "type": "string", "description": "Name of a built-in scene." },
            "random_seed": seed,
            "wallpaper_seed": { "type": "integer", "minimum": 0, "description": "Renders the wallpaper of this seed, laid out for width and height. layout, material and dark choose its style." },
            "layout": layout,
            "material": material,
            "dark": dark,
            "file": { "type": "string", "description": "Path of a scene file to read: a .frcq of the app, or JSON." },
            "frcq": { "type": "string", "description": "The bytes of a .frcq file, in base64." },
            "width": { "type": "integer", "minimum": 1, "maximum": side, "description": "Default 640." },
            "height": { "type": "integer", "minimum": 1, "maximum": side, "description": "Default 360." },
            "quality": { "type": "string", "enum": ["draft", "mobile", "standard", "high", "ultra"], "description": "Default mobile: enough to judge a scene. Use high for a final image." },
            "save_to": { "type": "string", "description": "Path of a PNG file to write as well." },
    });
    let description = format!(
        "Renders a scene and returns the image, so you can look at it. Give exactly one of {}. The same input always gives the same pixels; the returned hash identifies them. Images above about one megapixel are not returned inline{}. A request with a progress token gets notifications/progress while it renders, and notifications/cancelled stops it.",
        one_of(&sources(host)),
        if host.files { ": give save_to" } else { "" },
    );
    if !host.files
        && let Some(properties) = render.as_object_mut()
    {
        FILES.iter().for_each(|name| {
            properties.remove(*name);
        });
    }
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
        tool("render_scene", &description, render, &[]),
    ])
}

fn scene_of(arguments: &Value) -> Result<Scene, String> {
    let scene = match arguments.get("scene") {
        // Some clients hand an object argument over as its JSON text.
        Some(Value::String(text)) => return scene_from_bytes(text.as_bytes()),
        Some(value) => {
            serde_json::from_value::<Scene>(shown(value.clone())).map_err(|e| format!("not a scene: {e}"))?
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

pub fn names<T>(all: &[(&'static str, T)]) -> Vec<&'static str> {
    all.iter().map(|(name, _)| *name).collect()
}

fn wallpaper(arguments: &Value, seed: u64, width: u64, height: u64) -> Result<Scene, String> {
    let side = |name: &str, value: u64| u32::try_from(value).map_err(|_| format!("{name} is too large"));
    let request = wallpaper_request(
        word(arguments, "layout")?.map(str::to_owned),
        word(arguments, "material")?.map(str::to_owned),
        side("width", width)?.max(1),
        side("height", height)?.max(1),
        arguments.get("dark").and_then(Value::as_bool).unwrap_or(false),
    )?;
    Ok(random_wallpaper(seed, &request))
}

pub fn text(content: impl Into<String>) -> Vec<Value> {
    vec![json!({ "type": "text", "text": content.into() })]
}

pub fn base64(bytes: &[u8]) -> String {
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

pub fn unbase64(text: &str) -> Option<Vec<u8>> {
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

pub struct Render {
    scene: Scene,
    options: RenderOptions,
    save_to: Option<String>,
    inline: bool,
}

pub enum Step {
    Done(Vec<Value>),
    Render(Box<Render>),
}

fn given(arguments: &Value, name: &str) -> bool {
    arguments.get(name).is_some_and(|value| !value.is_null())
}

fn plan(arguments: &Value, host: &Host) -> Result<Render, String> {
    if let Some(name) = FILES.into_iter().find(|name| !host.files && given(arguments, name)) {
        return Err(format!(
            "{name} is refused: this server does not read or write files for a client"
        ));
    }
    let side = |name: &str, default: u64| -> Result<u32, String> {
        u32::try_from(whole(arguments, name)?.unwrap_or(default)).map_err(|_| format!("{name} is too large"))
    };
    let (width, height) = (side("width", 640)?, side("height", 360)?);
    let sources = sources(host);
    let chosen: Vec<&str> = sources.iter().copied().filter(|name| given(arguments, name)).collect();
    let scene = match chosen[..] {
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
        ["file"] => load(Path::new(word(arguments, "file")?.unwrap_or_default()))?,
        ["frcq"] => {
            let bytes = word(arguments, "frcq")?
                .and_then(unbase64)
                .ok_or("frcq must be the bytes of a .frcq file in base64")?;
            if !file::is_packed(&bytes) {
                return Err("frcq does not hold a .frcq file; a scene as JSON goes in scene".into());
            }
            scene_from_bytes(&bytes)?
        }
        _ => return Err(format!("give exactly one of {}", sources.join(", "))),
    };
    let quality = arguments.get("quality").and_then(Value::as_str).unwrap_or("mobile");
    let quality = Quality::parse(quality).ok_or_else(|| format!("no quality is called '{quality}'"))?;
    let options = RenderOptions::new(width, height, quality);
    if let Some(limits) = &host.limits {
        limits.check(&options)?;
    }
    let save_to = arguments.get("save_to").and_then(Value::as_str).map(str::to_owned);
    let inline = u64::from(width) * u64::from(height) <= INLINE_PIXELS;
    if !inline && save_to.is_none() {
        return Err(format!(
            "{width}x{height} is too large to return inline: {}render a smaller preview",
            if host.files { "give save_to, or " } else { "" }
        ));
    }
    Ok(Render {
        scene,
        options,
        save_to,
        inline,
    })
}

impl Render {
    pub fn run(&self, control: &Control<'_>) -> Result<Vec<Value>, String> {
        let RenderOptions { width, height, .. } = self.options;
        let started = Instant::now();
        let image = render(&self.scene, &self.options, control).map_err(|e| e.to_string())?;
        let png = encode_png(&image)?;
        let mut note = format!(
            "{width}x{height}, {} samples per pixel, {:.1} s, hash {:016x}",
            self.options.samples_per_pixel,
            started.elapsed().as_secs_f64(),
            image.hash()
        );
        if let Some(path) = &self.save_to {
            fs::write(path, &png).map_err(|e| format!("{path}: {e}"))?;
            note.push_str(&format!(", saved to {path}"));
        }
        let mut content = Vec::new();
        if self.inline {
            content.push(json!({ "type": "image", "data": base64(&png), "mimeType": "image/png" }));
        }
        content.push(json!({ "type": "text", "text": note }));
        Ok(content)
    }
}

pub fn prepare(name: &str, arguments: &Value, host: &Host) -> Result<Step, String> {
    let content = match name {
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
        "render_scene" => return plan(arguments, host).map(|render| Step::Render(Box::new(render))),
        _ => Err(format!("no tool is called '{name}'")),
    };
    content.map(Step::Done)
}

pub fn call(name: &str, arguments: &Value, host: &Host, control: &Control<'_>) -> Result<Vec<Value>, String> {
    match prepare(name, arguments, host)? {
        Step::Done(content) => Ok(content),
        Step::Render(render) => render.run(control),
    }
}

pub struct ToolCall<'a> {
    pub name: &'a str,
    pub arguments: &'a Value,
    pub token: Option<&'a Value>,
}

pub fn tool_call(params: &Value) -> Result<ToolCall<'_>, RpcError> {
    static EMPTY: Value = Value::Null;
    let name = params.get("name").and_then(Value::as_str).ok_or_else(|| RpcError {
        code: -32602,
        message: "tools/call needs a name".into(),
    })?;
    Ok(ToolCall {
        name,
        arguments: params.get("arguments").unwrap_or(&EMPTY),
        token: params
            .pointer("/_meta/progressToken")
            .filter(|token| token.is_string() || token.is_i64() || token.is_u64()),
    })
}

// Progress must only grow and the render threads report in any order: the lock keeps the percents in order.
pub fn reporter<'a>(token: &'a Value, send: impl Fn(Value) + Sync + 'a) -> impl Fn(f64) + Sync + 'a {
    let reported = Mutex::new(0u64);
    move |share: f64| {
        let percent = (share.clamp(0.0, 1.0) * 100.0) as u64;
        let mut last = reported.lock().unwrap_or_else(PoisonError::into_inner);
        if percent > *last {
            *last = percent;
            send(json!({
                "jsonrpc": "2.0",
                "method": "notifications/progress",
                "params": { "progressToken": token, "progress": percent, "total": 100, "message": "rendering" },
            }));
        }
    }
}

pub fn outcome(result: Result<Vec<Value>, String>) -> Value {
    match result {
        Ok(content) => json!({ "content": content, "isError": false }),
        Err(message) => json!({ "content": text(message), "isError": true }),
    }
}

pub fn answer(id: &Value, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

pub fn handle(method: &str, params: &Value, host: &Host) -> Result<Value, RpcError> {
    match method {
        "initialize" => {
            let asked = params.get("protocolVersion").and_then(Value::as_str).unwrap_or("");
            let version = host
                .protocols
                .iter()
                .find(|v| **v == asked)
                .unwrap_or(&host.protocols[0]);
            Ok(json!({
                "protocolVersion": version,
                "capabilities": { "tools": {} },
                "serverInfo": { "name": "fractique", "title": "Fractique", "version": env!("CARGO_PKG_VERSION") },
                "instructions": "Fractique renders fractal flames and wallpapers from scenes (JSON). A usual loop: get_preset, randomize_scene or randomize_wallpaper, render_scene to look at it, change the scene (describe_scene_format explains the fields), render again. The same scene always renders to the same pixels.",
            }))
        }
        "ping" => Ok(json!({})),
        "tools/list" => Ok(json!({ "tools": tools(host) })),
        _ => Err(RpcError {
            code: -32601,
            message: format!("method '{method}' is not supported"),
        }),
    }
}

pub fn failure(id: Value, code: i64, message: String) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

#[cfg(test)]
mod tests {
    use super::*;

    const REMOTE: Host = Host {
        protocols: &["2025-06-18"],
        files: false,
        limits: Some(Limits {
            max_side: 256,
            max_pixels: 200 * 200,
            max_samples: 50,
            max_supersample: 1,
        }),
    };

    #[test]
    fn base64_pads() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
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
    fn a_host_without_files_neither_lists_nor_takes_them() {
        let render = |host: &Host| {
            let tools = tools(host);
            let found = tools
                .as_array()
                .unwrap()
                .iter()
                .find(|tool| tool["name"] == "render_scene");
            found.unwrap().clone()
        };
        let local = render(&Host::LOCAL);
        let remote = render(&REMOTE);
        for name in FILES {
            assert!(local["inputSchema"]["properties"][name].is_object(), "{name}");
            assert!(remote["inputSchema"]["properties"].get(name).is_none(), "{name}");
            assert!(local["description"].as_str().unwrap().contains(name), "{name}");
            assert!(!remote["description"].as_str().unwrap().contains(name), "{name}");
        }
        assert_eq!(remote["inputSchema"]["properties"]["width"]["maximum"], 256);
        assert_eq!(local["inputSchema"]["properties"]["width"]["maximum"], 16384);

        let small = json!({ "preset": "aurora-spiral", "width": 64, "height": 36, "quality": "draft" });
        for (name, value) in [("file", "/etc/hostname"), ("save_to", "/tmp/fractique-refused.png")] {
            let mut arguments = small.clone();
            arguments[name] = value.into();
            let refused = prepare("render_scene", &arguments, &REMOTE).err().unwrap();
            assert!(refused.contains(name) && refused.contains("refused"), "{refused}");
            let alone = prepare("render_scene", &json!({ name: value }), &REMOTE).err().unwrap();
            assert!(alone.contains("refused"), "{alone}");
        }
        assert!(matches!(prepare("render_scene", &small, &REMOTE), Ok(Step::Render(_))));
    }

    #[test]
    fn a_host_with_limits_refuses_what_is_outside_them() {
        let refused = [
            json!({ "preset": "aurora-spiral", "width": 257, "height": 10, "quality": "draft" }),
            json!({ "preset": "aurora-spiral", "width": 0, "height": 10, "quality": "draft" }),
            json!({ "preset": "aurora-spiral", "width": 256, "height": 200, "quality": "draft" }),
            json!({ "preset": "aurora-spiral", "width": 64, "height": 64, "quality": "standard" }),
            json!({ "preset": "aurora-spiral", "width": 64, "height": 64, "quality": "high" }),
        ];
        for arguments in refused {
            let message = prepare("render_scene", &arguments, &REMOTE).err().unwrap();
            assert!(
                message.contains("on this server") || message.contains("this server renders"),
                "{message}"
            );
            assert!(
                matches!(prepare("render_scene", &arguments, &Host::LOCAL), Ok(Step::Render(_))),
                "{arguments}"
            );
        }
    }
}

use std::collections::BTreeSet;

use roxmltree::{Document, Node};

use crate::math::pow;
use crate::palette;
use crate::scene::{Blend, Camera, Content, Flame, Layer, Palette, SCENE_VERSION, Scene, Stop, Tone, Transform};
use crate::variation::{Kind, Variation};

pub struct Imported {
    pub scene: Scene,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImportError(pub String);

impl core::fmt::Display for ImportError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ImportError {}

const SETTINGS: [&str; 14] = [
    "weight",
    "color",
    "symmetry",
    "color_speed",
    "opacity",
    "coefs",
    "post",
    "animate",
    "name",
    "chaos",
    "plotmode",
    "var_color",
    "active",
    "enabled",
];

fn numbers(node: Node, name: &str) -> Result<Option<Vec<f64>>, ImportError> {
    let Some(text) = node.attribute(name) else {
        return Ok(None);
    };
    let parsed: Result<Vec<f64>, _> = text.split_whitespace().map(str::parse).collect();
    parsed
        .map(Some)
        .map_err(|_| ImportError(format!("{name}=\"{text}\" is not a list of numbers")))
}

fn number(node: Node, name: &str, default: f64) -> Result<f64, ImportError> {
    Ok(numbers(node, name)?.and_then(|v| v.first().copied()).unwrap_or(default))
}

fn is(node: Node, tag: &str) -> bool {
    node.is_element() && node.tag_name().name().eq_ignore_ascii_case(tag)
}

// The file lists a, d, b, e, c, f.
fn coefs(node: Node, name: &str) -> Result<Option<[f64; 6]>, ImportError> {
    match numbers(node, name)? {
        None => Ok(None),
        Some(v) if v.len() == 6 => Ok(Some([v[0], v[2], v[4], v[1], v[3], v[5]])),
        Some(_) => Err(ImportError(format!("{name} needs 6 numbers"))),
    }
}

fn transform(
    node: Node,
    unknown: &mut BTreeSet<String>,
    warnings: &mut Vec<String>,
) -> Result<Option<Transform>, ImportError> {
    let mut variations = Vec::new();
    let mut known: Vec<String> = SETTINGS.iter().map(|s| (*s).to_owned()).collect();
    for kind in Kind::ALL {
        let name = kind.name();
        known.push(name.clone());
        let mut params = Vec::new();
        for (param, default) in kind.params() {
            let attribute = format!("{name}_{param}");
            params.push(number(node, &attribute, *default)?);
            known.push(attribute);
        }
        let weight = number(node, &name, 0.0)?;
        if weight != 0.0 {
            variations.push(Variation { kind, weight, params });
        }
    }
    for attribute in node.attributes() {
        if !known.iter().any(|k| k == attribute.name()) {
            unknown.insert(attribute.name().to_owned());
        }
    }
    if node.attribute("chaos").is_some() {
        warnings.push("xaos (the chaos attribute) is not supported: transforms follow each other freely".into());
    }
    if variations.is_empty() {
        return Ok(None);
    }
    let speed = match numbers(node, "color_speed")? {
        Some(v) if !v.is_empty() => v[0],
        _ => (1.0 - number(node, "symmetry", 0.0)?) * 0.5,
    };
    let hidden = node
        .attribute("plotmode")
        .is_some_and(|m| m.eq_ignore_ascii_case("off"));
    Ok(Some(Transform {
        weight: number(node, "weight", 1.0)?.max(0.0),
        color: number(node, "color", 0.0)?.clamp(0.0, 1.0),
        color_speed: speed.clamp(0.0, 1.0),
        opacity: if hidden {
            0.0
        } else {
            number(node, "opacity", 1.0)?.clamp(0.0, 1.0)
        },
        affine: coefs(node, "coefs")?.unwrap_or([1.0, 0.0, 0.0, 0.0, 1.0, 0.0]),
        post: coefs(node, "post")?,
        variations,
    }))
}

fn hex_colors(text: &str, digits: usize, skip: usize) -> Vec<[f64; 3]> {
    let compact: Vec<u8> = text.bytes().filter(u8::is_ascii_hexdigit).collect();
    compact
        .chunks_exact(digits)
        .filter_map(|chunk| {
            let channel = |i: usize| {
                let pair = core::str::from_utf8(&chunk[skip + i * 2..skip + i * 2 + 2]).ok()?;
                u8::from_str_radix(pair, 16).ok().map(|v| f64::from(v) / 255.0)
            };
            Some([channel(0)?, channel(1)?, channel(2)?])
        })
        .collect()
}

fn read_palette(flame: Node) -> Result<Option<Palette>, ImportError> {
    let mut colors: Vec<[f64; 3]> = Vec::new();
    for child in flame.children() {
        if is(child, "color") {
            if let Some(rgb) = numbers(child, "rgb")?.filter(|v| v.len() >= 3) {
                colors.push([rgb[0], rgb[1], rgb[2]].map(|c| (c / 255.0).clamp(0.0, 1.0)));
            }
        } else if is(child, "palette") {
            let alpha = child
                .attribute("format")
                .is_some_and(|f| f.eq_ignore_ascii_case("RGBA"));
            colors = hex_colors(child.text().unwrap_or(""), if alpha { 8 } else { 6 }, 0);
        } else if is(child, "colors") {
            colors = hex_colors(child.attribute("data").unwrap_or(""), 8, 2);
        }
    }
    if colors.is_empty() {
        return Ok(None);
    }
    colors.truncate(256);
    let last = (colors.len() - 1).max(1) as f64;
    let stops = colors
        .into_iter()
        .enumerate()
        .map(|(i, color)| Stop {
            pos: i as f64 / last,
            color,
        })
        .collect();
    Ok(Some(Palette { stops }))
}

fn import_one(node: Node, index: usize) -> Result<Imported, ImportError> {
    let mut warnings = Vec::new();
    let mut unknown = BTreeSet::new();
    let name = node
        .attribute("name")
        .map_or_else(|| format!("flame-{index}"), str::to_owned);

    let mut transforms = Vec::new();
    let mut final_transform = None;
    let mut symmetry = 1;
    for child in node.children() {
        if is(child, "xform") {
            match transform(child, &mut unknown, &mut warnings)? {
                Some(t) => transforms.push(t),
                None => warnings.push(format!(
                    "transform {} has no supported variation and was left out",
                    transforms.len()
                )),
            }
        } else if is(child, "finalxform") {
            final_transform = transform(child, &mut unknown, &mut warnings)?;
        } else if is(child, "symmetry") {
            symmetry = number(child, "kind", 1.0)? as i32;
        }
    }
    if !unknown.is_empty() {
        let names: Vec<String> = unknown.into_iter().collect();
        warnings.push(format!("not supported and ignored: {}", names.join(", ")));
    }

    let palette = match read_palette(node)? {
        Some(palette) => palette,
        None => {
            warnings.push(
                "the file has no colors of its own (flam3's numbered palettes are not included): using 'aurora'".into(),
            );
            palette::named("aurora").ok_or_else(|| ImportError("no fallback palette".into()))?
        }
    };

    let size = numbers(node, "size")?.filter(|v| v.len() == 2 && v[0] > 0.0 && v[1] > 0.0);
    let zoom = match (size, numbers(node, "scale")?) {
        (Some(size), Some(scale)) if !scale.is_empty() && scale[0] > 0.0 => {
            scale[0] * pow(2.0, number(node, "zoom", 0.0)?) * 4.0 / size[0].min(size[1])
        }
        _ => 1.0,
    };
    let center = numbers(node, "center")?
        .filter(|v| v.len() == 2)
        .map_or([0.0, 0.0], |v| [v[0], v[1]]);
    let background = numbers(node, "background")?
        .filter(|v| v.len() == 3)
        .map_or([0.0; 3], |v| [v[0], v[1], v[2]]);

    let flame = Flame {
        camera: Camera {
            center,
            zoom,
            rotation: number(node, "rotate", 0.0)?,
            flip_y: true,
        },
        tone: Tone {
            // flam3's default brightness is 4, where this renderer's is 1.
            brightness: (number(node, "brightness", 4.0)? / 4.0).max(0.0),
            gamma: number(node, "gamma", 4.0)?.max(0.1),
            vibrancy: number(node, "vibrancy", 1.0)?.clamp(0.0, 1.0),
            ..Tone::default()
        },
        palette,
        symmetry: if symmetry == 0 { 1 } else { symmetry.clamp(-64, 64) },
        transforms,
        final_transform,
    };
    let scene = Scene {
        version: SCENE_VERSION,
        name: name.clone(),
        seed: 0,
        background: background.map(|c| c.clamp(0.0, 1.0)),
        sheets: None,
        layers: vec![Layer {
            name: String::new(),
            blend: Blend::Add,
            opacity: 1.0,
            content: Content::Flame(flame),
        }],
    };
    scene.validate().map_err(|e| ImportError(format!("{name}: {e}")))?;
    Ok(Imported { scene, warnings })
}

pub fn import_flames(xml: &str) -> Result<Vec<Imported>, ImportError> {
    let wrapped;
    let document = match Document::parse(xml) {
        Ok(document) => document,
        Err(first) => {
            let body = xml
                .trim_start()
                .strip_prefix("<?xml")
                .and_then(|rest| rest.split_once("?>"))
                .map_or(xml, |(_, rest)| rest);
            wrapped = format!("<flames>{body}</flames>");
            Document::parse(&wrapped).map_err(|_| ImportError(format!("not a flame file: {first}")))?
        }
    };
    let flames: Vec<Node> = document.descendants().filter(|n| is(*n, "flame")).collect();
    if flames.is_empty() {
        return Err(ImportError("the file has no <flame>".into()));
    }
    flames
        .into_iter()
        .enumerate()
        .map(|(index, node)| import_one(node, index))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const FILE: &str = r#"<flames name="pack">
<flame name="one" size="800 600" center="0.5 -0.25" scale="150" zoom="1" rotate="30" background="0 0 0.1" brightness="8" gamma="3" vibrancy="0.5">
   <xform weight="0.5" color="0.25" symmetry="-1" linear="1" julian="0.5" julian_power="3" julian_dist="-1" coefs="1 2 3 4 5 6" post="0.5 0 0 0.5 1 1"/>
   <xform weight="2" color="1" spherical="1" waves2="0.3" opacity="0.5" coefs="1 0 0 1 0 0"/>
   <xform weight="1" waves2="1"/>
   <finalxform color="0" eyefish="1" coefs="1 0 0 1 0 0"/>
   <symmetry kind="-3"/>
   <color index="0" rgb="255 0 0"/>
   <color index="1" rgb="0 0 255"/>
</flame>
<flame name="two"><xform spherical="1"/><palette count="2" format="RGB">FF0000 00FF00</palette></flame>
</flames>"#;

    #[test]
    fn reads_a_flame_pack() {
        let flames = import_flames(FILE).unwrap();
        assert_eq!(flames.len(), 2);
        let scene = &flames[0].scene;
        let flame = scene.layers[0].flame().unwrap();
        assert_eq!(scene.name, "one");
        assert_eq!(scene.background, [0.0, 0.0, 0.1]);
        assert_eq!(flame.camera.center, [0.5, -0.25]);
        assert_eq!(flame.camera.zoom, 150.0 * 2.0 * 4.0 / 600.0);
        assert!(flame.camera.flip_y);
        assert_eq!(
            (flame.tone.brightness, flame.tone.gamma, flame.tone.vibrancy),
            (2.0, 3.0, 0.5)
        );
        assert_eq!(flame.symmetry, -3);
        assert_eq!(flame.transforms.len(), 2);

        let first = &flame.transforms[0];
        assert_eq!(first.affine, [1.0, 3.0, 5.0, 2.0, 4.0, 6.0]);
        assert_eq!(first.post, Some([0.5, 0.0, 1.0, 0.0, 0.5, 1.0]));
        assert_eq!(first.color_speed, 1.0);
        assert_eq!(first.variations.len(), 2);
        assert_eq!(
            first.variations[1],
            Variation::with_params(Kind::Julian, 0.5, &[3.0, -1.0])
        );
        assert_eq!(flame.transforms[1].opacity, 0.5);
        assert!(flame.final_transform.is_some());
        assert_eq!(flame.palette.stops.len(), 2);
        assert_eq!(flame.palette.stops[1].color, [0.0, 0.0, 1.0]);

        let warnings = flames[0].warnings.join("\n");
        assert!(warnings.contains("waves2"), "{warnings}");
        assert!(warnings.contains("left out"), "{warnings}");
        assert_eq!(flames[1].scene.layers[0].palette().stops[1].color, [0.0, 1.0, 0.0]);
    }

    #[test]
    fn reads_the_parameters_of_every_variation_that_has_them() {
        let xml = r#"<flame><xform waves="0.5" popcorn="0.25" blob="1" blob_low="0.1" blob_high="0.9" blob_waves="7"
            pdj="1" pdj_a="1" pdj_b="2" pdj_c="3" pdj_d="4" fan2="1" fan2_x="0.2" fan2_y="0.4" rings2="1"
            rings2_val="0.3" perspective="1" perspective_angle="0.25" perspective_dist="3" juliascope="1"
            juliascope_power="-3" juliascope_dist="2" radial_blur="1" radial_blur_angle="0.75" pie="1"
            pie_slices="8" pie_rotation="0.5" pie_thickness="0.25" ngon="1" ngon_sides="6" ngon_power="3"
            ngon_circle="0.5" ngon_corners="2" rectangles="1" rectangles_x="0.3" rectangles_y="0.6" secant2="1"
            twintrian="1" coefs="1 0 0 1 0 0"/></flame>"#;
        let flames = import_flames(xml).unwrap();
        assert!(
            flames[0].warnings.iter().all(|w| !w.contains("ignored")),
            "{:?}",
            flames[0].warnings
        );
        let variations = &flames[0].scene.layers[0].flame().unwrap().transforms[0].variations;
        let expected = [
            Variation::new(Kind::Waves, 0.5),
            Variation::new(Kind::Popcorn, 0.25),
            Variation::with_params(Kind::Blob, 1.0, &[0.1, 0.9, 7.0]),
            Variation::with_params(Kind::Pdj, 1.0, &[1.0, 2.0, 3.0, 4.0]),
            Variation::with_params(Kind::Fan2, 1.0, &[0.2, 0.4]),
            Variation::with_params(Kind::Rings2, 1.0, &[0.3]),
            Variation::with_params(Kind::Perspective, 1.0, &[0.25, 3.0]),
            Variation::with_params(Kind::Juliascope, 1.0, &[-3.0, 2.0]),
            Variation::with_params(Kind::RadialBlur, 1.0, &[0.75]),
            Variation::with_params(Kind::Pie, 1.0, &[8.0, 0.5, 0.25]),
            Variation::with_params(Kind::Ngon, 1.0, &[6.0, 3.0, 0.5, 2.0]),
            Variation::with_params(Kind::Rectangles, 1.0, &[0.3, 0.6]),
            Variation::new(Kind::Secant2, 1.0),
            Variation::new(Kind::Twintrian, 1.0),
        ];
        assert_eq!(variations.as_slice(), expected.as_slice());
    }

    #[test]
    fn reads_flames_without_a_root() {
        let flames = import_flames(
            "<?xml version=\"1.0\"?><flame><xform linear=\"1\"/></flame><flame><xform linear=\"1\"/></flame>",
        );
        assert_eq!(flames.unwrap().len(), 2);
    }

    #[test]
    fn refuses_what_is_not_a_flame() {
        assert!(import_flames("hello").is_err());
        assert!(import_flames("<a/>").is_err());
        assert!(import_flames("<flame><xform waves2=\"1\"/></flame>").is_err());
    }
}

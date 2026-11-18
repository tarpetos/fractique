use serde::{Deserialize, Serialize};

use crate::palette;
use crate::variation::{Kind, Variation};

pub const SCENE_VERSION: u32 = 1;
pub const MAX_LAYERS: usize = 16;
pub const MAX_TRANSFORMS: usize = 64;
pub const MAX_SHEETS: usize = 48;
pub const MAX_SHAPES: usize = 32;
pub const MAX_SWELLS: usize = 4;

fn one() -> f64 {
    1.0
}
fn half() -> f64 {
    0.5
}
fn one_i() -> i32 {
    1
}
fn identity() -> [f64; 6] {
    [1.0, 0.0, 0.0, 0.0, 1.0, 0.0]
}
fn is_zero(v: &f64) -> bool {
    *v == 0.0
}
fn is_false(v: &bool) -> bool {
    !*v
}
fn is_one(v: &f64) -> bool {
    *v == 1.0
}
fn is_one_i(v: &i32) -> bool {
    *v == 1
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Scene {
    pub version: u32,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub name: String,
    pub seed: u64,
    #[serde(default)]
    pub background: [f64; 3],
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sheets: Option<Sheets>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub layers: Vec<Layer>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Sheets {
    #[serde(default = "Sheets::default_light")]
    pub light: f64,
    #[serde(default = "Sheets::default_rim")]
    pub rim: f64,
    #[serde(default = "Sheets::default_rim_width")]
    pub rim_width: f64,
    #[serde(default = "Sheets::default_bevel")]
    pub bevel: f64,
    #[serde(default = "Sheets::default_shadow")]
    pub shadow: f64,
    #[serde(default = "Sheets::default_shadow_offset")]
    pub shadow_offset: f64,
    #[serde(default = "Sheets::default_shadow_blur")]
    pub shadow_blur: f64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub gradient: f64,
    pub stack: Vec<Sheet>,
}

impl Sheets {
    fn default_light() -> f64 {
        135.0
    }
    fn default_rim() -> f64 {
        0.5
    }
    fn default_rim_width() -> f64 {
        0.006
    }
    fn default_bevel() -> f64 {
        0.12
    }
    fn default_shadow() -> f64 {
        0.3
    }
    fn default_shadow_offset() -> f64 {
        0.03
    }
    fn default_shadow_blur() -> f64 {
        0.12
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Sheet {
    pub color: [f64; 3],
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color_to: Option<[f64; 3]>,
    #[serde(default = "one", skip_serializing_if = "is_one")]
    pub opacity: f64,
    pub shapes: Vec<Shape>,
}

fn full_turn() -> f64 {
    360.0
}
fn is_full_turn(v: &f64) -> bool {
    *v == 360.0
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Shape {
    Box {
        center: [f64; 2],
        half: [f64; 2],
        #[serde(default)]
        corners: [f64; 4],
        #[serde(default, skip_serializing_if = "is_zero")]
        rotation: f64,
    },
    Petal {
        base: [f64; 2],
        length: f64,
        width: f64,
        #[serde(default, skip_serializing_if = "is_zero")]
        rotation: f64,
    },
    Ring {
        center: [f64; 2],
        radius: f64,
        thickness: f64,
        #[serde(default = "full_turn", skip_serializing_if = "is_full_turn")]
        span: f64,
        #[serde(default, skip_serializing_if = "is_zero")]
        rotation: f64,
    },
    Wave {
        level: f64,
        swells: Vec<Swell>,
        #[serde(default, skip_serializing_if = "is_zero")]
        thickness: f64,
        #[serde(default, skip_serializing_if = "is_zero")]
        rotation: f64,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Swell {
    pub height: f64,
    pub length: f64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub shift: f64,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum Blend {
    #[default]
    Add,
    Screen,
    Over,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Layer {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub name: String,
    #[serde(default)]
    pub blend: Blend,
    #[serde(default = "one", skip_serializing_if = "is_one")]
    pub opacity: f64,
    pub flame: Flame,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Flame {
    #[serde(default)]
    pub camera: Camera,
    #[serde(default)]
    pub tone: Tone,
    pub palette: Palette,
    #[serde(default = "one_i", skip_serializing_if = "is_one_i")]
    pub symmetry: i32,
    pub transforms: Vec<Transform>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub final_transform: Option<Transform>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Camera {
    #[serde(default)]
    pub center: [f64; 2],
    #[serde(default = "one")]
    pub zoom: f64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub rotation: f64,
    #[serde(default, skip_serializing_if = "is_false")]
    pub flip_y: bool,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            center: [0.0, 0.0],
            zoom: 1.0,
            rotation: 0.0,
            flip_y: false,
        }
    }
}

impl Camera {
    // The camera that looks at the point `right` and `down` of this one's middle, in shorter sides of the image,
    // and magnifies it `by` times.
    pub fn looking_at(&self, right: f64, down: f64, by: f64) -> Camera {
        let angle = self.rotation.to_radians();
        let (cos, sin) = (crate::math::cos(angle), crate::math::sin(angle));
        let across = right * 4.0 / self.zoom;
        let up = if self.flip_y { down } else { -down } * 4.0 / self.zoom;
        Camera {
            center: [
                self.center[0] + across * cos - up * sin,
                self.center[1] + across * sin + up * cos,
            ],
            zoom: self.zoom * by,
            ..self.clone()
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Tone {
    #[serde(default = "one")]
    pub brightness: f64,
    #[serde(default = "Tone::default_gamma")]
    pub gamma: f64,
    #[serde(default = "one")]
    pub vibrancy: f64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub highlight: f64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub glow_radius: f64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub glow_strength: f64,
    #[serde(default = "one", skip_serializing_if = "is_one")]
    pub smoothing: f64,
}

impl Tone {
    fn default_gamma() -> f64 {
        2.5
    }
}

impl Default for Tone {
    fn default() -> Self {
        Self {
            brightness: 1.0,
            gamma: 2.5,
            vibrancy: 1.0,
            highlight: 0.0,
            glow_radius: 0.0,
            glow_strength: 0.0,
            smoothing: 1.0,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Stop {
    pub pos: f64,
    pub color: [f64; 3],
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Palette {
    pub stops: Vec<Stop>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Transform {
    #[serde(default = "one")]
    pub weight: f64,
    #[serde(default)]
    pub color: f64,
    #[serde(default = "half")]
    pub color_speed: f64,
    #[serde(default = "one", skip_serializing_if = "is_one")]
    pub opacity: f64,
    #[serde(default = "identity")]
    pub affine: [f64; 6],
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub post: Option<[f64; 6]>,
    pub variations: Vec<Variation>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SceneError(pub String);

impl core::fmt::Display for SceneError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for SceneError {}

fn fail<T>(message: String) -> Result<T, SceneError> {
    Err(SceneError(message))
}

fn finite(values: &[f64], what: &str) -> Result<(), SceneError> {
    if values.iter().all(|v| v.is_finite()) {
        Ok(())
    } else {
        fail(format!("{what}: not a finite number"))
    }
}

fn unit(value: f64, what: &str) -> Result<(), SceneError> {
    if (0.0..=1.0).contains(&value) {
        Ok(())
    } else {
        fail(format!("{what}: must be between 0 and 1"))
    }
}

impl Transform {
    fn validate(&self, what: &str) -> Result<(), SceneError> {
        finite(&[self.weight, self.color, self.color_speed, self.opacity], what)?;
        finite(&self.affine, what)?;
        if let Some(post) = &self.post {
            finite(post, what)?;
        }
        if self.weight < 0.0 {
            return fail(format!("{what}: weight must not be negative"));
        }
        unit(self.color, &format!("{what}: color"))?;
        unit(self.color_speed, &format!("{what}: color_speed"))?;
        unit(self.opacity, &format!("{what}: opacity"))?;
        if self.variations.is_empty() {
            return fail(format!("{what}: needs at least one variation"));
        }
        for v in &self.variations {
            let name = v.kind.name();
            finite(&[v.weight], &format!("{what}: {name}"))?;
            finite(&v.params, &format!("{what}: {name}"))?;
            if v.params.len() > v.kind.params().len() {
                return fail(format!("{what}: {name} takes {} parameters", v.kind.params().len()));
            }
            if v.kind == Kind::Julian && v.kind.resolve(&v.params)[0] == 0.0 {
                return fail(format!("{what}: julian power must not be 0"));
            }
            if matches!(v.kind, Kind::Juliascope | Kind::Pie | Kind::Ngon) && v.kind.resolve(&v.params)[0] == 0.0 {
                return fail(format!("{what}: {name} {} must not be 0", v.kind.params()[0].0));
            }
        }
        Ok(())
    }
}

impl Flame {
    fn validate(&self, what: &str) -> Result<(), SceneError> {
        finite(&self.camera.center, what)?;
        finite(&[self.camera.zoom, self.camera.rotation], what)?;
        if self.camera.zoom <= 0.0 {
            return fail(format!("{what}: camera zoom must be above 0"));
        }
        let t = &self.tone;
        finite(
            &[
                t.brightness,
                t.gamma,
                t.vibrancy,
                t.highlight,
                t.glow_radius,
                t.glow_strength,
                t.smoothing,
            ],
            what,
        )?;
        if t.gamma <= 0.0 {
            return fail(format!("{what}: gamma must be above 0"));
        }
        if t.brightness < 0.0 || t.highlight < 0.0 || t.glow_strength < 0.0 {
            return fail(format!(
                "{what}: brightness, highlight and glow_strength must not be negative"
            ));
        }
        unit(t.vibrancy, &format!("{what}: vibrancy"))?;
        if !(0.0..=4.0).contains(&t.smoothing) {
            return fail(format!("{what}: smoothing must be between 0 and 4"));
        }
        if !(0.0..=0.25).contains(&t.glow_radius) {
            return fail(format!("{what}: glow_radius must be between 0 and 0.25"));
        }
        palette::validate(&self.palette).map_err(|e| SceneError(format!("{what}: {e}")))?;
        if self.symmetry == 0 || self.symmetry.unsigned_abs() > 64 {
            return fail(format!("{what}: symmetry must be between 1 and 64, or -1 and -64"));
        }
        if self.transforms.is_empty() || self.transforms.len() > MAX_TRANSFORMS {
            return fail(format!("{what}: needs 1 to {MAX_TRANSFORMS} transforms"));
        }
        for (i, t) in self.transforms.iter().enumerate() {
            t.validate(&format!("{what}, transform {i}"))?;
        }
        if self.transforms.iter().map(|t| t.weight).sum::<f64>() <= 0.0 {
            return fail(format!("{what}: the transform weights add up to 0"));
        }
        if let Some(t) = &self.final_transform {
            t.validate(&format!("{what}, final transform"))?;
        }
        Ok(())
    }
}

impl Shape {
    fn validate(&self, what: &str) -> Result<(), SceneError> {
        match self {
            Shape::Box {
                center,
                half,
                corners,
                rotation,
            } => {
                finite(center, what)?;
                finite(half, what)?;
                finite(corners, what)?;
                finite(&[*rotation], what)?;
                let [hx, hy] = *half;
                if hx <= 0.0 || hy <= 0.0 {
                    return fail(format!("{what}: half must be above 0"));
                }
                let [tl, tr, br, bl] = *corners;
                if corners.iter().any(|r| *r < 0.0) {
                    return fail(format!("{what}: corners must not be negative"));
                }
                let (wide, high) = (2.0 * hx, 2.0 * hy);
                if tl + tr > wide || bl + br > wide || tl + bl > high || tr + br > high {
                    return fail(format!("{what}: two corners on one side are larger than the side"));
                }
            }
            Shape::Petal {
                base,
                length,
                width,
                rotation,
            } => {
                finite(base, what)?;
                finite(&[*length, *width, *rotation], what)?;
                if *width <= 0.0 || *length < *width {
                    return fail(format!("{what}: width must be above 0 and length at least the width"));
                }
            }
            Shape::Ring {
                center,
                radius,
                thickness,
                span,
                rotation,
            } => {
                finite(center, what)?;
                finite(&[*radius, *thickness, *span, *rotation], what)?;
                if *radius <= 0.0 || *thickness <= 0.0 {
                    return fail(format!("{what}: radius and thickness must be above 0"));
                }
                if !(*span > 0.0 && *span <= 360.0) {
                    return fail(format!("{what}: span must be above 0 and at most 360"));
                }
            }
            Shape::Wave {
                level,
                swells,
                thickness,
                rotation,
            } => {
                finite(&[*level, *thickness, *rotation], what)?;
                if *thickness < 0.0 {
                    return fail(format!("{what}: thickness must not be negative"));
                }
                if swells.len() > MAX_SWELLS {
                    return fail(format!("{what}: takes up to {MAX_SWELLS} swells"));
                }
                for swell in swells {
                    finite(&[swell.height, swell.length, swell.shift], what)?;
                    if swell.length <= 0.0 {
                        return fail(format!("{what}: a swell's length must be above 0"));
                    }
                }
            }
        }
        Ok(())
    }
}

impl Sheets {
    fn validate(&self) -> Result<(), SceneError> {
        let what = "sheets";
        finite(
            &[
                self.light,
                self.rim,
                self.rim_width,
                self.bevel,
                self.shadow,
                self.shadow_offset,
                self.shadow_blur,
                self.gradient,
            ],
            what,
        )?;
        unit(self.shadow, "sheets: shadow")?;
        if !(0.0..=4.0).contains(&self.rim) || !(-1.0..=1.0).contains(&self.bevel) {
            return fail("sheets: rim must be between 0 and 4, bevel between -1 and 1".into());
        }
        if !(-1.0..=1.0).contains(&self.gradient) {
            return fail("sheets: gradient must be between -1 and 1".into());
        }
        let width = |v: f64| v > 0.0 && v <= 2.0;
        if !width(self.rim_width) || !width(self.shadow_blur) {
            return fail("sheets: rim_width and shadow_blur must be above 0 and at most 2".into());
        }
        if !(0.0..=1.0).contains(&self.shadow_offset) {
            return fail("sheets: shadow_offset must be between 0 and 1".into());
        }
        if self.stack.is_empty() || self.stack.len() > MAX_SHEETS {
            return fail(format!("sheets: needs 1 to {MAX_SHEETS} sheets"));
        }
        for (i, sheet) in self.stack.iter().enumerate() {
            finite(&sheet.color, &format!("sheet {i}"))?;
            finite(&[sheet.opacity], &format!("sheet {i}"))?;
            unit(sheet.opacity, &format!("sheet {i}: opacity"))?;
            for c in sheet.color.into_iter().chain(sheet.color_to.into_iter().flatten()) {
                if !(0.0..=1.0).contains(&c) {
                    return fail(format!("sheet {i}: colors must be between 0 and 1"));
                }
            }
            if sheet.shapes.is_empty() || sheet.shapes.len() > MAX_SHAPES {
                return fail(format!("sheet {i}: needs 1 to {MAX_SHAPES} shapes"));
            }
            for (j, shape) in sheet.shapes.iter().enumerate() {
                shape.validate(&format!("sheet {i}, shape {j}"))?;
            }
        }
        Ok(())
    }
}

impl Scene {
    pub fn validate(&self) -> Result<(), SceneError> {
        if self.version != SCENE_VERSION {
            return fail(format!(
                "scene version {} is not supported (this build reads {SCENE_VERSION})",
                self.version
            ));
        }
        finite(&self.background, "background")?;
        for c in self.background {
            unit(c, "background")?;
        }
        if let Some(sheets) = &self.sheets {
            sheets.validate()?;
        }
        if (self.layers.is_empty() && self.sheets.is_none()) || self.layers.len() > MAX_LAYERS {
            return fail(format!("a scene needs 1 to {MAX_LAYERS} layers, or sheets"));
        }
        for (i, layer) in self.layers.iter().enumerate() {
            finite(&[layer.opacity], &format!("layer {i}"))?;
            unit(layer.opacity, &format!("layer {i}: opacity"))?;
            layer.flame.validate(&format!("layer {i}"))?;
        }
        Ok(())
    }

    pub fn from_json(text: &str) -> Result<Self, SceneError> {
        let scene: Scene = serde_json::from_str(text).map_err(|e| SceneError(format!("not a scene: {e}")))?;
        scene.validate()?;
        Ok(scene)
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }
}

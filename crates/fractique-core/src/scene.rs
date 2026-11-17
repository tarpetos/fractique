use serde::{Deserialize, Serialize};

use crate::palette;
use crate::variation::{Kind, Variation};

pub const SCENE_VERSION: u32 = 1;
pub const MAX_LAYERS: usize = 16;
pub const MAX_TRANSFORMS: usize = 64;

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

/// Everything a render depends on. The same scene and the same render options always give the same pixels.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Scene {
    pub version: u32,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub name: String,
    pub seed: u64,
    /// sRGB, 0 to 1.
    #[serde(default)]
    pub background: [f64; 3],
    pub layers: Vec<Layer>,
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
    /// Rotational copies of the flame around the origin; a negative number also mirrors it.
    #[serde(default = "one_i", skip_serializing_if = "is_one_i")]
    pub symmetry: i32,
    pub transforms: Vec<Transform>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub final_transform: Option<Transform>,
}

/// At zoom 1 the shorter side of the image shows 4 world units, whatever the resolution.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Camera {
    #[serde(default)]
    pub center: [f64; 2],
    #[serde(default = "one")]
    pub zoom: f64,
    /// Degrees, counter-clockwise.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub rotation: f64,
    /// Draws the world's y axis downwards, as flam3 and Apophysis do.
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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Tone {
    #[serde(default = "one")]
    pub brightness: f64,
    #[serde(default = "Tone::default_gamma")]
    pub gamma: f64,
    /// 1 keeps the palette's hue in dim areas, 0 applies gamma to each channel.
    #[serde(default = "one")]
    pub vibrancy: f64,
    /// How fast the densest areas burn out to white.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub highlight: f64,
    /// Radius of the glow as a share of the image's shorter side.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub glow_radius: f64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub glow_strength: f64,
    /// Density estimation: 0 leaves every hit in its own cell, 1 is the standard amount.
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
    /// sRGB, 0 to 1.
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
    /// Position in the palette, 0 to 1.
    #[serde(default)]
    pub color: f64,
    #[serde(default = "half")]
    pub color_speed: f64,
    #[serde(default = "one", skip_serializing_if = "is_one")]
    pub opacity: f64,
    /// [a, b, c, d, e, f]: x' = a x + b y + c, y' = d x + e y + f.
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
        if self.layers.is_empty() || self.layers.len() > MAX_LAYERS {
            return fail(format!("a scene needs 1 to {MAX_LAYERS} layers"));
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

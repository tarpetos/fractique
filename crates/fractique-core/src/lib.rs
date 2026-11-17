//! Deterministic fractal flame renderer: one [`Scene`] always gives the same pixels, on every platform.

pub mod import;
pub mod math;
pub mod palette;
pub mod presets;
pub mod random;
pub mod render;
pub mod rng;
pub mod scene;
pub mod variation;

mod chaos;
mod tone;

pub use render::{Control, Image, Quality, RenderError, RenderOptions, render};
pub use scene::{Blend, Camera, Flame, Layer, Palette, SCENE_VERSION, Scene, SceneError, Stop, Tone, Transform};
pub use variation::{Kind, Variation};

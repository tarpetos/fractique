use std::collections::BTreeMap;

use crate::palette;
use crate::rng::Rng;
use crate::scene::{Blend, Camera, Content, Fractal, Layer, SCENE_VERSION, Scene, Tone};

pub(crate) mod density;
pub(crate) mod pixel;

mod escape;
mod geometric;
mod orbit;
mod root;
mod solid;

// One line per kind in its family's `mod.rs`: the module of the kind and its place in the family's list.
macro_rules! family {
    ($($kind:ident),* $(,)?) => {
        $(mod $kind;)*
        pub(super) const KINDS: &[&super::Spec] = &[$(&$kind::SPEC),*];
    };
}
use family;

const FAMILIES: [&[&Spec]; 5] = [escape::KINDS, root::KINDS, orbit::KINDS, geometric::KINDS, solid::KINDS];

const BACKGROUND: [f64; 3] = [0.008, 0.01, 0.03];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Family {
    Escape,
    Root,
    Orbit,
    Geometric,
    Solid,
}

impl Family {
    pub const ALL: [Family; 5] = [
        Family::Escape,
        Family::Root,
        Family::Orbit,
        Family::Geometric,
        Family::Solid,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Family::Escape => "escape-time",
            Family::Root => "root-finding",
            Family::Orbit => "orbit",
            Family::Geometric => "geometric",
            Family::Solid => "solid",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Param {
    pub name: &'static str,
    pub default: f64,
    pub min: f64,
    pub max: f64,
    pub whole: bool,
    pub log: bool,
}

impl Param {
    pub const fn real(name: &'static str, default: f64, min: f64, max: f64) -> Self {
        Self {
            name,
            default,
            min,
            max,
            whole: false,
            log: false,
        }
    }

    pub const fn whole(name: &'static str, default: f64, min: f64, max: f64) -> Self {
        Self {
            whole: true,
            ..Self::real(name, default, min, max)
        }
    }

    pub const fn log(self) -> Self {
        Self { log: true, ..self }
    }
}

pub struct Sample {
    pub at: f64,
    pub light: f64,
}

/// A kind drawn pixel by pixel. `sample` is called for points of the world in any order and on any thread:
/// it must depend on the point alone. `None` is a point the layer leaves clear.
pub trait Field: Sync {
    fn sample(&self, x: f64, y: f64) -> Option<Sample>;
}

/// A kind drawn as the density of the points of orbits. Each of the 64 streams of a render gets one walk.
pub trait Orbit: Sync {
    fn walk(&self, rng: &mut Rng) -> Box<dyn Walk + '_>;

    /// The colors of the places a walk plots, for a kind that brings its own in place of the layer's palette.
    fn colors(&self) -> Option<palette::Lut> {
        None
    }
}

pub trait Walk {
    /// Plots exactly `count` points: x, y and a place on the palette from 0 to 1.
    fn run(&mut self, rng: &mut Rng, count: u64, plot: &mut dyn FnMut(f64, f64, f64));
}

pub enum Draw {
    Pixel(fn(&[f64], &Camera) -> Box<dyn Field>),
    Density(fn(&[f64], &Camera) -> Box<dyn Orbit>),
}

pub struct Preset {
    pub name: &'static str,
    pub palette: &'static str,
    pub center: [f64; 2],
    pub zoom: f64,
    pub params: &'static [(&'static str, f64)],
}

pub struct Spec {
    pub name: &'static str,
    pub family: Family,
    pub params: &'static [Param],
    pub tone: Tone,
    pub draw: Draw,
    pub random: fn(u64) -> Fractal,
    pub mutate: fn(&Fractal, &mut Rng, f64) -> Fractal,
    pub presets: &'static [Preset],
}

impl Spec {
    pub fn fractal(&self, colors: &str, camera: Camera, params: &[(&str, f64)]) -> Fractal {
        Fractal {
            kind: self.name.to_owned(),
            camera,
            tone: self.tone.clone(),
            palette: palette::named(colors).unwrap_or_else(|| palette::named("aurora").unwrap()),
            params: params
                .iter()
                .map(|(name, value)| ((*name).to_owned(), *value))
                .collect::<BTreeMap<_, _>>(),
        }
    }

    pub fn values(&self, fractal: &Fractal) -> Vec<f64> {
        self.params
            .iter()
            .map(|param| fractal.params.get(param.name).copied().unwrap_or(param.default))
            .collect()
    }

    pub fn is_density(&self) -> bool {
        matches!(self.draw, Draw::Density(_))
    }

    fn preset(&self, preset: &Preset) -> Scene {
        let camera = Camera {
            center: preset.center,
            zoom: preset.zoom,
            ..Camera::default()
        };
        scene_of(
            self.fractal(preset.palette, camera, preset.params),
            format!("{}-{}", self.name, preset.name),
            0,
        )
    }
}

pub fn all() -> impl Iterator<Item = &'static Spec> {
    FAMILIES.into_iter().flatten().copied()
}

pub fn spec(kind: &str) -> Option<&'static Spec> {
    all().find(|spec| spec.name == kind)
}

pub fn scene_of(fractal: Fractal, name: String, seed: u64) -> Scene {
    Scene {
        version: SCENE_VERSION,
        name,
        seed,
        background: BACKGROUND,
        sheets: None,
        layers: vec![Layer {
            name: fractal.kind.clone(),
            blend: Blend::Add,
            opacity: 1.0,
            content: Content::Fractal(fractal),
        }],
    }
}

pub fn random_scene(kind: &str, seed: u64) -> Option<Scene> {
    let spec = spec(kind)?;
    Some(scene_of((spec.random)(seed), format!("{kind}-{seed}"), seed))
}

pub fn preset_names() -> Vec<String> {
    all()
        .flat_map(|spec| {
            spec.presets
                .iter()
                .map(move |preset| format!("{}-{}", spec.name, preset.name))
        })
        .collect()
}

pub fn preset(name: &str) -> Option<Scene> {
    all().find_map(|spec| {
        let rest = name.strip_prefix(spec.name)?.strip_prefix('-')?;
        let preset = spec.presets.iter().find(|preset| preset.name == rest)?;
        Some(spec.preset(preset))
    })
}

// A stream of random numbers of a kind's own: the kinds never share one.
pub(crate) fn stream(kind: &str, seed: u64, index: u64) -> Rng {
    let domain = kind.bytes().fold(0xCBF2_9CE4_8422_2325u64, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x0000_0100_0000_01B3)
    });
    Rng::derive(seed, domain | 1 << 32, index)
}

pub(crate) fn random_palette(rng: &mut Rng) -> &'static str {
    palette::NAMES[rng.below(palette::NAMES.len() as u64) as usize]
}

#[cfg(test)]
pub(crate) fn golden(scene: &Scene) -> u64 {
    let options = crate::RenderOptions {
        width: 192,
        height: 108,
        samples_per_pixel: 20,
        supersample: 2,
    };
    crate::render(scene, &options, &crate::Control::default())
        .expect("the scene renders")
        .hash()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_kinds_are_listed_in_order_and_once() {
        for family in FAMILIES {
            let names: Vec<_> = family.iter().map(|spec| spec.name).collect();
            let mut sorted = names.clone();
            sorted.sort_unstable();
            sorted.dedup();
            assert_eq!(names, sorted);
        }
        let mut names: Vec<_> = all().map(|spec| spec.name).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), all().count());
        for (family, kinds) in Family::ALL.into_iter().zip(FAMILIES) {
            assert!(kinds.iter().all(|spec| spec.family == family));
        }
    }

    #[test]
    fn every_kind_has_sound_parameters_presets_and_random_scenes() {
        for spec in all() {
            assert!(spec.name.bytes().all(|b| b.is_ascii_lowercase() || b == b'_'));
            for param in spec.params {
                assert!(
                    param.min <= param.default && param.default <= param.max,
                    "{}",
                    param.name
                );
                assert!(!param.log || param.min > 0.0, "{}", param.name);
            }
            assert!(!spec.presets.is_empty(), "{} has no preset", spec.name);
            for preset in spec.presets {
                let name = format!("{}-{}", spec.name, preset.name);
                let scene = super::preset(&name).expect("the preset is found by its name");
                scene.validate().unwrap_or_else(|e| panic!("{name}: {e}"));
                assert!(palette::named(preset.palette).is_some(), "{name}");
                assert!(crate::presets::get(&name).is_none(), "{name} is a flame's name too");
            }
            for seed in [0, 1, 7, u64::MAX] {
                let scene = random_scene(spec.name, seed).expect("the kind is listed");
                scene.validate().unwrap_or_else(|e| panic!("{} {seed}: {e}", spec.name));
                assert_eq!(scene, random_scene(spec.name, seed).unwrap());
                let mutated = crate::random::mutate(&scene, 3, 0.5);
                mutated
                    .validate()
                    .unwrap_or_else(|e| panic!("{} {seed}: {e}", spec.name));
                assert_ne!(mutated.layers, scene.layers, "{} {seed}", spec.name);
                assert_eq!(mutated, crate::random::mutate(&scene, 3, 0.5));
            }
        }
    }

    #[test]
    fn a_scene_of_a_fractal_comes_back_from_json() {
        for spec in all() {
            let scene = random_scene(spec.name, 5).unwrap();
            assert_eq!(Scene::from_json(&scene.to_json()).unwrap(), scene);
        }
        let bare = r#"{"version": 1, "seed": 1, "layers": [{"fractal": {"kind": "mandelbrot",
            "palette": {"stops": [{"pos": 0, "color": [1, 1, 1]}]}}}]}"#;
        Scene::from_json(bare).unwrap();
        for wrong in [
            bare.replace("mandelbrot", "nothing"),
            bare.replace(r#""kind""#, r#""params": {"speed": 1}, "kind""#),
            bare.replace(r#""kind""#, r#""params": {"iterations": 0}, "kind""#),
            bare.replace(r#"{"fractal""#, r#"{"flame": null, "extra": 1, "fractal""#),
            bare.replace(r#"{"fractal": {"#, r#"{"other": {"#),
        ] {
            assert!(Scene::from_json(&wrong).is_err(), "{wrong}");
        }
    }
}

use fractique_core::wallpaper::{Hues, lightness_chroma, toned};
use fractique_core::{Palette, Scene, Stop, fractal, palette};

use crate::widgets;

pub const MOST_OWN: usize = 8;

#[derive(Clone, Debug, PartialEq)]
pub enum Choice {
    Swatch(usize),
    // Colors of one's own, one to eight, in sRGB from 0 to 1, in the order of the gradient.
    Own(Vec<[f64; 3]>),
}

impl Choice {
    pub fn palette(&self) -> Option<Palette> {
        match self {
            Choice::Swatch(at) => palette::SWATCHES.get(*at).and_then(|name| palette::named(name)),
            Choice::Own(colors) => own(colors),
        }
    }

    pub fn swatch(&self) -> Option<usize> {
        match self {
            Choice::Swatch(at) => Some(*at),
            Choice::Own(_) => None,
        }
    }

    fn saved(&self) -> serde_json::Value {
        match self {
            Choice::Swatch(at) => palette::SWATCHES
                .get(*at)
                .map_or(serde_json::Value::Null, |name| (*name).into()),
            Choice::Own(colors) => colors.iter().map(|color| hex(*color)).collect(),
        }
    }

    fn restored(value: &serde_json::Value) -> Option<Self> {
        match value {
            serde_json::Value::String(name) => palette::SWATCHES
                .iter()
                .position(|each| each == name)
                .map(Choice::Swatch),
            serde_json::Value::Array(colors) => {
                let colors: Vec<[f64; 3]> = colors
                    .iter()
                    .filter_map(|c| c.as_str().and_then(parse_hex))
                    .take(MOST_OWN)
                    .collect();
                (!colors.is_empty()).then_some(Choice::Own(colors))
            }
            _ => None,
        }
    }
}

// One's own colors of a wallpaper as its hues: each color's hue, and how much lighter and stronger it is than the
// full color of that hue, which is what the material is given.
pub fn wall_hues(colors: &[[f64; 3]]) -> Option<Hues> {
    let full = |color: [f64; 3]| widgets::hsv_to_rgb([widgets::rgb_to_hsv(color)[0], 1.0, 1.0]);
    let hues: Vec<f64> = colors
        .iter()
        .map(|color| f64::from(widgets::rgb_to_hsv(*color)[0]) * 360.0)
        .collect();
    let tones: Vec<[f64; 2]> = colors
        .iter()
        .map(|color| {
            let ([lightness, chroma, _], [plain, strong, _]) =
                (lightness_chroma(*color), lightness_chroma(full(*color)));
            let round = |value: f64| (value * 1000.0).round() / 1000.0;
            [
                round(lightness - plain),
                if strong > 1e-6 { round(chroma / strong) } else { 1.0 },
            ]
        })
        .collect();
    Hues::toned(&hues, &tones)
}

// The colors of a wallpaper's hues, as one's own colors show them.
pub fn wall_colors(hues: &Hues) -> Vec<[f64; 3]> {
    hues.list()
        .iter()
        .zip(hues.tones())
        .map(|(hue, tone)| toned(widgets::hsv_to_rgb([(*hue / 360.0) as f32, 1.0, 1.0]), *tone))
        .collect()
}

pub fn hex(color: [f64; 3]) -> String {
    let [r, g, b] = color.map(|c| (c.clamp(0.0, 1.0) * 255.0).round() as u8);
    format!("#{r:02x}{g:02x}{b:02x}")
}

pub fn parse_hex(text: &str) -> Option<[f64; 3]> {
    let digits = text.strip_prefix('#')?;
    if digits.len() != 6 {
        return None;
    }
    let channel = |at: usize| {
        u8::from_str_radix(digits.get(at..at + 2)?, 16)
            .ok()
            .map(|v| f64::from(v) / 255.0)
    };
    Some([channel(0)?, channel(2)?, channel(4)?])
}

// A palette of one's own colors: several are spread evenly from the first to the last; one goes from its own dark
// through itself to its own light, so a flame in it still has depth.
pub fn own(colors: &[[f64; 3]]) -> Option<Palette> {
    let mix = |color: [f64; 3], toward: f64, by: f64| color.map(|c| (c + (toward - c) * by).clamp(0.0, 1.0));
    let stop = |pos: f64, color: [f64; 3]| Stop {
        pos,
        color: color.map(|c| c.clamp(0.0, 1.0)),
    };
    match colors {
        [] => None,
        [one] => Some(Palette {
            stops: vec![
                stop(0.0, mix(*one, 0.0, 0.6)),
                stop(0.55, *one),
                stop(1.0, mix(*one, 1.0, 0.45)),
            ],
        }),
        many => Some(Palette {
            stops: many
                .iter()
                .enumerate()
                .map(|(at, color)| stop(at as f64 / (many.len() - 1) as f64, *color))
                .collect(),
        }),
    }
}

// One's own colors for a solid lit in the colors of the light, which reads its palette as an ink from no light to
// full light: a dark of the first color first, then the colors, then a pale of the last for the brightest light.
pub fn own_lit(colors: &[[f64; 3]]) -> Option<Palette> {
    let (first, last) = (*colors.first()?, *colors.last()?);
    let mix = |color: [f64; 3], toward: f64, by: f64| color.map(|c| (c + (toward - c) * by).clamp(0.0, 1.0));
    let stop = |pos: f64, color: [f64; 3]| Stop {
        pos,
        color: color.map(|c| c.clamp(0.0, 1.0)),
    };
    let mut stops = vec![stop(0.0, mix(first, 0.0, 0.92)), stop(0.18, mix(first, 0.0, 0.6))];
    let spread = (colors.len() - 1).max(1) as f64;
    stops.extend(colors.iter().enumerate().map(|(at, color)| {
        let pos = if colors.len() == 1 {
            0.6
        } else {
            0.4 + 0.45 * at as f64 / spread
        };
        stop(pos, *color)
    }));
    stops.push(stop(1.0, mix(last, 1.0, 0.6)));
    Some(Palette { stops })
}

// A layer whose palette is read as the light falling on it.
fn lit(layer: &fractique_core::Layer) -> bool {
    layer
        .fractal()
        .and_then(|shown| shown.params.get("light_color"))
        .is_some_and(|light| *light > 0.0)
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Choices(Vec<Option<Choice>>);

impl Choices {
    pub fn all(choice: Option<Choice>, layers: usize) -> Self {
        Self(vec![choice; layers])
    }

    pub fn of(&self, layer: usize) -> Option<Choice> {
        self.0.get(layer).cloned().flatten()
    }

    pub fn choose(&mut self, layer: usize, choice: Option<Choice>) {
        if self.0.len() <= layer {
            self.0.resize(layer + 1, None);
        }
        self.0[layer] = choice;
    }

    pub fn remove(&mut self, layer: usize) {
        if layer < self.0.len() {
            self.0.remove(layer);
        }
    }

    pub fn swap(&mut self, one: usize, other: usize) {
        let (first, second) = (self.of(one), self.of(other));
        self.choose(one, second);
        self.choose(other, first);
    }

    pub fn dress(&self, scene: &mut Scene) {
        for (index, (layer, choice)) in scene.layers.iter_mut().zip(&self.0).enumerate() {
            let Some(choice) = choice else {
                continue;
            };
            let palette = match choice {
                Choice::Own(colors) if lit(layer) => own_lit(colors),
                _ => choice.palette(),
            };
            let dark = palette.as_ref().map(|palette| palette.stops[0].color);
            if let Some(palette) = palette {
                *layer.palette_mut() = palette;
            }
            // A palette of one hue brings its background, which the bottom layer covers but where it is clear;
            // another palette, one's own colors too, takes such a background back to the dark one.
            if index == 0 {
                let name = choice.swatch().and_then(|at| palette::SWATCHES.get(at));
                // A lit solid in one's own colors stands on the darkest of them, as one of a palette of one hue does.
                let own = matches!(choice, Choice::Own(_))
                    .then_some(dark)
                    .flatten()
                    .filter(|_| lit(layer));
                match name.and_then(|name| palette::backdrop(name)).or(own) {
                    Some(background) => {
                        scene.background = background;
                        layer.blend = fractique_core::Blend::Over;
                    }
                    None if palette::is_backdrop(scene.background) => scene.background = fractal::BACKGROUND,
                    None => {}
                }
            }
        }
    }

    pub fn saved(&self, layers: usize) -> serde_json::Value {
        (0..layers)
            .map(|layer| self.of(layer).map_or(serde_json::Value::Null, |choice| choice.saved()))
            .collect()
    }

    // A session of before the choices were a layer's has one "palette", which every layer wore.
    pub fn restored(session: &serde_json::Value, layers: usize) -> Self {
        match session.get("colors").and_then(serde_json::Value::as_array) {
            Some(saved) => Self(
                (0..layers)
                    .map(|layer| saved.get(layer).and_then(Choice::restored))
                    .collect(),
            ),
            None => Self::all(session.get("palette").and_then(Choice::restored), layers),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fractique_core::{fractal, presets};

    fn two_layers() -> Scene {
        let mut scene = fractal::preset("julia-dragon").expect("a preset");
        scene
            .layers
            .push(fractal::preset("newton-cubic").expect("a preset").layers.remove(0));
        scene
    }

    fn at(name: &str) -> Option<Choice> {
        palette::SWATCHES
            .iter()
            .position(|each| *each == name)
            .map(Choice::Swatch)
    }

    fn dressed(choices: &Choices, scene: &Scene) -> Scene {
        let mut scene = scene.clone();
        choices.dress(&mut scene);
        scene
    }

    #[test]
    fn a_palette_chosen_for_one_layer_leaves_the_others_their_own() {
        let scene = two_layers();
        let mut choices = Choices::all(None, 2);
        assert_eq!(dressed(&choices, &scene), scene);
        choices.choose(1, at("ember"));
        let shown = dressed(&choices, &scene);
        assert_eq!(shown.layers[0], scene.layers[0]);
        assert_eq!(shown.layers[1].palette(), &palette::named("ember").expect("a palette"));
        assert_eq!((choices.of(0), choices.of(1)), (None, at("ember")));
        choices.choose(0, at("ice"));
        let shown = dressed(&choices, &scene);
        assert_eq!(shown.layers[0].palette(), &palette::named("ice").expect("a palette"));
        assert_eq!(shown.layers[1].palette(), &palette::named("ember").expect("a palette"));
        choices.choose(1, None);
        assert_eq!(dressed(&choices, &scene).layers[1], scene.layers[1]);
    }

    #[test]
    fn a_palette_of_one_hue_brings_its_background_and_another_takes_it_away() {
        let scene = fractal::preset("mandelbox-blossom").expect("a preset");
        assert_eq!(Some(scene.background), palette::backdrop("rosewood"));
        let mut choices = Choices::all(at("amethyst"), 1);
        assert_eq!(
            Some(dressed(&choices, &scene).background),
            palette::backdrop("amethyst")
        );
        choices.choose(0, at("orchid"));
        assert_eq!(dressed(&choices, &scene).background, fractal::BACKGROUND);
        let own = fractal::preset("julia-dragon").expect("a preset");
        choices.choose(0, at("ember"));
        assert_eq!(dressed(&choices, &own).background, own.background);
    }

    #[test]
    fn a_scene_of_one_layer_wears_its_choice_as_it_did() {
        let scene = presets::get("aurora-spiral").expect("a preset");
        let mut whole = scene.clone();
        let gold = palette::named("gold").expect("a palette");
        for layer in &mut whole.layers {
            *layer.palette_mut() = gold.clone();
        }
        let mut choices = Choices::all(None, scene.layers.len());
        choices.choose(0, at("gold"));
        assert_eq!(dressed(&choices, &scene), whole);
        assert_eq!(dressed(&Choices::all(at("gold"), scene.layers.len()), &scene), whole);
    }

    #[test]
    fn a_choice_stays_with_its_layer_when_layers_are_moved_removed_and_added() {
        let mut choices = Choices::all(None, 3);
        choices.choose(0, at("ember"));
        choices.choose(2, at("ice"));
        choices.swap(0, 1);
        assert_eq!([0, 1, 2].map(|layer| choices.of(layer)), [None, at("ember"), at("ice")]);
        choices.swap(2, 1);
        assert_eq!([0, 1, 2].map(|layer| choices.of(layer)), [None, at("ice"), at("ember")]);
        choices.remove(1);
        assert_eq!([0, 1, 2].map(|layer| choices.of(layer)), [None, at("ember"), None]);
        choices.choose(2, None);
        choices.remove(0);
        assert_eq!([0, 1].map(|layer| choices.of(layer)), [at("ember"), None]);
        let mut short = Choices::default();
        short.swap(0, 1);
        short.remove(4);
        assert_eq!(short.of(7), None);
    }

    #[test]
    fn the_choices_of_a_session_come_back_each_to_its_layer() {
        let mut choices = Choices::all(None, 3);
        choices.choose(1, at("violet"));
        choices.choose(2, at("gold"));
        let session = serde_json::json!({ "colors": choices.saved(3) });
        assert_eq!(session["colors"], serde_json::json!([null, "violet", "gold"]));
        let text = session.to_string();
        let read: serde_json::Value = serde_json::from_str(&text).expect("it is JSON");
        assert_eq!(Choices::restored(&read, 3), choices);
        let fewer = Choices::restored(&read, 2);
        assert_eq!((fewer.of(1), fewer.of(2)), (at("violet"), None));
        let unknown = serde_json::json!({ "colors": ["mauve", 3, "ice"] });
        assert_eq!(Choices::restored(&unknown, 3), {
            let mut only = Choices::all(None, 3);
            only.choose(2, at("ice"));
            only
        });
    }

    #[test]
    fn a_session_with_one_palette_gives_the_scene_it_gave() {
        let scene = two_layers();
        let ember = palette::named("ember").expect("a palette");
        let mut before = scene.clone();
        for layer in &mut before.layers {
            *layer.palette_mut() = ember.clone();
        }
        let old = serde_json::json!({ "palette": "ember", "layer": 1 });
        assert_eq!(dressed(&Choices::restored(&old, 2), &scene), before);
        for none in [serde_json::json!({ "palette": null }), serde_json::json!({})] {
            assert_eq!(dressed(&Choices::restored(&none, 2), &scene), scene);
        }
        let both = serde_json::json!({ "palette": "ember", "colors": [null, "ice"] });
        assert_eq!(Choices::restored(&both, 2).of(0), None);
    }

    #[test]
    fn a_lit_solid_reads_one_s_own_colors_from_dark_to_light() {
        let (red, blue) = ([0.9, 0.3, 0.3], [0.3, 0.4, 0.9]);
        let lit = own_lit(&[red, blue]).expect("a palette");
        let stops: Vec<f64> = lit.stops.iter().map(|s| (s.pos * 100.0).round() / 100.0).collect();
        assert_eq!(stops, [0.0, 0.18, 0.4, 0.85, 1.0]);
        assert!(lit.stops[0].color.iter().all(|c| *c < 0.1));
        assert!(lit.stops[4].color.iter().all(|c| *c > 0.6));
        let scene = fractal::preset("mandelbox-plaza").expect("a preset");
        let mut choices = Choices::all(None, 1);
        choices.choose(0, Some(Choice::Own(vec![red, blue])));
        let shown = dressed(&choices, &scene);
        assert_eq!(shown.layers[0].palette(), &lit);
        assert_eq!(shown.background, lit.stops[0].color);
        // A fractal that is not lit wears them as they are.
        let flat = fractal::preset("julia-dragon").expect("a preset");
        assert_eq!(
            dressed(&choices, &flat).layers[0].palette(),
            &own(&[red, blue]).expect("a palette")
        );
    }

    #[test]
    fn colors_of_one_s_own_make_a_gradient_and_come_back_from_a_session() {
        let (red, blue) = ([0.9, 0.1, 0.1], [0.1, 0.2, 0.9]);
        let two = own(&[red, blue]).expect("a palette");
        assert_eq!(two.stops.iter().map(|s| s.pos).collect::<Vec<_>>(), [0.0, 1.0]);
        assert_eq!((two.stops[0].color, two.stops[1].color), (red, blue));
        let one = own(&[red]).expect("a palette");
        assert_eq!(one.stops.len(), 3);
        assert_eq!(one.stops[1].color, red);
        assert!(one.stops[0].color[0] < red[0] && one.stops[2].color[1] > red[1]);
        assert_eq!(own(&[]), None);

        let scene = two_layers();
        let mut choices = Choices::all(None, 2);
        choices.choose(1, Some(Choice::Own(vec![red, blue])));
        assert_eq!(dressed(&choices, &scene).layers[1].palette(), &two);
        let session = serde_json::json!({ "colors": choices.saved(2) });
        assert_eq!(session["colors"], serde_json::json!([null, ["#e61a1a", "#1a33e6"]]));
        let back = Choices::restored(&session, 2);
        let Some(Choice::Own(colors)) = back.of(1) else {
            panic!("the colors come back");
        };
        assert_eq!(
            colors.iter().map(|c| hex(*c)).collect::<Vec<_>>(),
            ["#e61a1a", "#1a33e6"]
        );
        let odd = serde_json::json!({ "colors": [["#12", "nope", "#00ff00"], []] });
        assert_eq!(
            Choices::restored(&odd, 2).of(0),
            Some(Choice::Own(vec![[0.0, 1.0, 0.0]]))
        );
        assert_eq!(Choices::restored(&odd, 2).of(1), None);
    }
}

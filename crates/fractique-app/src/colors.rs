use fractique_core::{Scene, fractal, palette};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Choices(Vec<Option<usize>>);

impl Choices {
    pub fn all(choice: Option<usize>, layers: usize) -> Self {
        Self(vec![choice; layers])
    }

    pub fn of(&self, layer: usize) -> Option<usize> {
        self.0.get(layer).copied().flatten()
    }

    pub fn choose(&mut self, layer: usize, choice: Option<usize>) {
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
            let name = choice.and_then(|at| palette::SWATCHES.get(at));
            if let Some(palette) = name.and_then(|name| palette::named(name)) {
                *layer.palette_mut() = palette;
            }
            // A palette of one hue brings its background, which the bottom layer covers but where it is clear;
            // another palette takes such a background back to the dark one.
            if index == 0 && name.is_some() {
                match name.and_then(|name| palette::backdrop(name)) {
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
            .map(|layer| self.of(layer).and_then(|at| palette::SWATCHES.get(at)).copied())
            .collect()
    }

    // A session of before the choices were a layer's has one "palette", which every layer wore.
    pub fn restored(session: &serde_json::Value, layers: usize) -> Self {
        let chosen = |name: &serde_json::Value| {
            let name = name.as_str()?;
            palette::SWATCHES.iter().position(|each| *each == name)
        };
        match session.get("colors").and_then(serde_json::Value::as_array) {
            Some(names) => Self((0..layers).map(|layer| names.get(layer).and_then(chosen)).collect()),
            None => Self::all(session.get("palette").and_then(chosen), layers),
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

    fn at(name: &str) -> Option<usize> {
        palette::SWATCHES.iter().position(|each| *each == name)
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
}

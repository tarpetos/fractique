use crate::scene::Scene;

const PRESETS: &[(&str, &str)] = &[
    ("aurora-spiral", include_str!("../presets/aurora-spiral.json")),
    ("emerald-star", include_str!("../presets/emerald-star.json")),
    ("emerald-mandala", include_str!("../presets/emerald-mandala.json")),
    ("ember-vortex", include_str!("../presets/ember-vortex.json")),
    ("azure-rings", include_str!("../presets/azure-rings.json")),
    ("golden-bloom", include_str!("../presets/golden-bloom.json")),
    ("solar-mandala", include_str!("../presets/solar-mandala.json")),
    ("jade-orb", include_str!("../presets/jade-orb.json")),
];

pub fn names() -> Vec<&'static str> {
    PRESETS.iter().map(|(name, _)| *name).collect()
}

pub fn get(name: &str) -> Option<Scene> {
    let (_, json) = PRESETS.iter().find(|(n, _)| *n == name)?;
    Scene::from_json(json).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_preset_loads() {
        for (name, json) in PRESETS {
            let scene = Scene::from_json(json).unwrap_or_else(|e| panic!("{name}: {e}"));
            assert_eq!(scene.name, *name);
        }
    }
}

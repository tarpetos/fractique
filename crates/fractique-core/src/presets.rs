use crate::scene::Scene;

const PRESETS: &[(&str, &str)] = &[
    ("aurora-spiral", include_str!("../presets/aurora-spiral.json")),
    ("emerald-star", include_str!("../presets/emerald-star.json")),
    ("emerald-burst", include_str!("../presets/emerald-burst.json")),
    ("emerald-mandala", include_str!("../presets/emerald-mandala.json")),
    ("ember-vortex", include_str!("../presets/ember-vortex.json")),
    ("azure-rings", include_str!("../presets/azure-rings.json")),
    ("golden-bloom", include_str!("../presets/golden-bloom.json")),
    ("solar-mandala", include_str!("../presets/solar-mandala.json")),
    ("jade-orb", include_str!("../presets/jade-orb.json")),
    ("violet-whirl", include_str!("../presets/violet-whirl.json")),
    ("radiant-star", include_str!("../presets/radiant-star.json")),
    ("teal-lattice", include_str!("../presets/teal-lattice.json")),
    ("amber-tree", include_str!("../presets/amber-tree.json")),
    ("silk-current", include_str!("../presets/silk-current.json")),
    ("tide-curl", include_str!("../presets/tide-curl.json")),
    ("orchid-knot", include_str!("../presets/orchid-knot.json")),
    ("frost-crown", include_str!("../presets/frost-crown.json")),
    ("green-comet", include_str!("../presets/green-comet.json")),
    ("moss-nebula", include_str!("../presets/moss-nebula.json")),
    ("lilac-cloud", include_str!("../presets/lilac-cloud.json")),
    ("ice-crystal", include_str!("../presets/ice-crystal.json")),
    ("golden-filigree", include_str!("../presets/golden-filigree.json")),
    ("frost-lotus", include_str!("../presets/frost-lotus.json")),
    ("azure-turbine", include_str!("../presets/azure-turbine.json")),
    ("orchid-pinwheel", include_str!("../presets/orchid-pinwheel.json")),
    ("ember-thorns", include_str!("../presets/ember-thorns.json")),
    ("lilac-mosaic", include_str!("../presets/lilac-mosaic.json")),
    ("amber-lantern", include_str!("../presets/amber-lantern.json")),
    ("emerald-plume", include_str!("../presets/emerald-plume.json")),
    ("lime-orbits", include_str!("../presets/lime-orbits.json")),
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

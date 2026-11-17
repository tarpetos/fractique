use std::sync::atomic::AtomicBool;

use fractique_core::random::random_scene;
use fractique_core::{Control, RenderError, RenderOptions, presets, render};

fn small(supersample: u32) -> RenderOptions {
    RenderOptions {
        width: 192,
        height: 108,
        samples_per_pixel: 20,
        supersample,
    }
}

// The promise of the project: these hashes are the same on every platform and every machine. A change here
// means existing scenes render differently: it needs a new scene version or a line in the changelog.
const GOLDEN: [(&str, u32, u64); 4] = [
    ("aurora-spiral", 1, 0x0bc0_6113_dc33_6c2f),
    ("aurora-spiral", 2, 0xe16d_5352_1408_8d99),
    ("emerald-star", 1, 0xf6b9_a86c_906a_153b),
    ("ember-vortex", 1, 0xcb35_68ab_cb43_98e8),
];

#[test]
fn presets_render_to_their_golden_hashes() {
    for (name, supersample, expected) in GOLDEN {
        let scene = presets::get(name).expect(name);
        let image = render(&scene, &small(supersample), &Control::default()).unwrap();
        assert_eq!(
            image.hash(),
            expected,
            "{name} at supersample {supersample}: got {:#018x}",
            image.hash()
        );
    }
}

#[test]
fn every_preset_renders() {
    for name in presets::names() {
        let scene = presets::get(name).expect(name);
        let options = RenderOptions {
            width: 64,
            height: 36,
            samples_per_pixel: 8,
            supersample: 1,
        };
        let image = render(&scene, &options, &Control::default()).unwrap();
        assert!(
            image.rgba.chunks_exact(4).any(|p| p[0] > 40 || p[1] > 40 || p[2] > 40),
            "{name} is black"
        );
    }
}

#[test]
fn a_render_repeats_exactly() {
    let scene = random_scene(7);
    let first = render(&scene, &small(1), &Control::default()).unwrap();
    assert_eq!(first, render(&scene, &small(1), &Control::default()).unwrap());
}

#[cfg(feature = "parallel")]
#[test]
fn the_thread_count_does_not_change_the_image() {
    let scene = random_scene(11);
    let all_cores = render(&scene, &small(2), &Control::default()).unwrap();
    for threads in [1, 3] {
        let pool = rayon::ThreadPoolBuilder::new().num_threads(threads).build().unwrap();
        let image = pool.install(|| render(&scene, &small(2), &Control::default()).unwrap());
        assert_eq!(image.hash(), all_cores.hash(), "{threads} threads");
    }
}

#[test]
fn the_seed_changes_the_image() {
    let mut scene = random_scene(7);
    let first = render(&scene, &small(1), &Control::default()).unwrap();
    scene.seed += 1;
    assert_ne!(
        first.hash(),
        render(&scene, &small(1), &Control::default()).unwrap().hash()
    );
}

#[test]
fn a_cancelled_render_stops() {
    let cancel = AtomicBool::new(true);
    let result = render(
        &random_scene(7),
        &small(1),
        &Control {
            cancel: Some(&cancel),
            progress: None,
        },
    );
    assert_eq!(result, Err(RenderError::Cancelled));
}

#[test]
fn bad_options_are_refused() {
    let options = RenderOptions {
        width: 0,
        height: 10,
        samples_per_pixel: 10,
        supersample: 1,
    };
    assert!(matches!(
        render(&random_scene(7), &options, &Control::default()),
        Err(RenderError::Options(_))
    ));
}

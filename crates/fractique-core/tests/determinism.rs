use std::sync::atomic::AtomicBool;

use fractique_core::random::random_scene;
use fractique_core::wallpaper::{Layout, Material, Request, random_wallpaper};
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
    ("emerald-star", 1, 0x383c_eab3_e607_ea04),
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

fn deep() -> fractique_core::Scene {
    let mut scene = presets::get("aurora-spiral").unwrap();
    let camera = &mut scene.layers[0].flame.camera;
    *camera = camera.looking_at(-0.1, -0.31, 40_000.0);
    scene
}

// A view that holds little of the flame is drawn another way (`deep.rs`); it keeps the same promise.
#[test]
fn a_deep_view_renders_to_its_golden_hash() {
    let image = render(&deep(), &small(1), &Control::default()).unwrap();
    assert!(
        image.rgba.chunks_exact(4).any(|p| p[0] > 40 || p[1] > 40 || p[2] > 40),
        "the deep view is black"
    );
    assert_eq!(image.hash(), 0x3322_9bcf_e53e_2be4, "got {:#018x}", image.hash());
}

#[cfg(feature = "parallel")]
#[test]
fn the_thread_count_does_not_change_a_deep_view() {
    let all_cores = render(&deep(), &small(1), &Control::default()).unwrap();
    let pool = rayon::ThreadPoolBuilder::new().num_threads(1).build().unwrap();
    let image = pool.install(|| render(&deep(), &small(1), &Control::default()).unwrap());
    assert_eq!(image.hash(), all_cores.hash());
}

// The picture point that is `right` and `down` of the middle becomes the middle.
#[test]
fn a_camera_looks_at_a_point_of_its_picture() {
    let mut scene = presets::get("ember-vortex").unwrap();
    let options = RenderOptions {
        width: 96,
        height: 64,
        samples_per_pixel: 60,
        supersample: 1,
    };
    let whole = render(&scene, &options, &Control::default()).unwrap();
    for layer in &mut scene.layers {
        layer.flame.camera = layer.flame.camera.looking_at(0.25, -0.125, 1.0);
    }
    let moved = render(&scene, &options, &Control::default()).unwrap();
    let lit = |image: &fractique_core::Image, x: u32, y: u32| {
        let at = ((y * image.width + x) * 4) as usize;
        image.rgba[at..at + 3].iter().map(|c| u32::from(*c)).sum::<u32>()
    };
    let (mut same, mut all) = (0, 0);
    for y in 8..56 {
        for x in 0..72 {
            all += 1;
            let (a, b) = (lit(&whole, x + 16, y - 8), lit(&moved, x, y));
            same += u32::from(a.abs_diff(b) <= 48);
        }
    }
    assert!(same * 10 >= all * 9, "{same} of {all} pixels moved with the camera");
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

fn wallpaper(seed: u64, layout: Layout, material: Material, dark: bool) -> fractique_core::Scene {
    random_wallpaper(
        seed,
        &Request {
            layout: Some(layout),
            material: Some(material),
            ratio: 108.0 / 192.0,
            dark,
        },
    )
}

const GOLDEN_WALLPAPERS: [(Layout, Material, bool, u64); 7] = [
    (Layout::Bands, Material::Relief, false, 0xf459_fe72_6982_fa7d),
    (Layout::Waves, Material::Paper, true, 0x7013_c4e2_9e79_68fa),
    (Layout::Burst, Material::Glass, false, 0x62bd_9197_547f_f914),
    (Layout::Arcs, Material::Glow, true, 0xd08f_47a2_d1a9_27f2),
    (Layout::Fan, Material::Paper, false, 0xd24d_8226_02b9_466c),
    (Layout::Planes, Material::Steel, true, 0xc7ce_9776_48d8_be7d),
    (Layout::Lines, Material::Carbon, true, 0xe392_54ac_a3cd_447a),
];

#[test]
fn wallpapers_render_to_their_golden_hashes() {
    for (layout, material, dark, expected) in GOLDEN_WALLPAPERS {
        let image = render(&wallpaper(3, layout, material, dark), &small(1), &Control::default()).unwrap();
        assert_eq!(
            image.hash(),
            expected,
            "{layout:?}, {material:?}, dark {dark}: got {:#018x}",
            image.hash()
        );
    }
}

#[test]
fn every_wallpaper_layout_renders() {
    for (name, layout) in Layout::ALL {
        for (_, material) in Material::ALL {
            let image = render(&wallpaper(5, layout, material, false), &small(1), &Control::default()).unwrap();
            let first = &image.rgba[..4];
            assert!(image.rgba.chunks_exact(4).any(|p| p != first), "{name} is flat");
        }
    }
}

#[cfg(feature = "parallel")]
#[test]
fn the_thread_count_does_not_change_a_wallpaper() {
    let scene = wallpaper(11, Layout::Ribbons, Material::Glass, false);
    let all_cores = render(&scene, &small(1), &Control::default()).unwrap();
    let pool = rayon::ThreadPoolBuilder::new().num_threads(1).build().unwrap();
    let image = pool.install(|| render(&scene, &small(1), &Control::default()).unwrap());
    assert_eq!(image.hash(), all_cores.hash());
}

#[test]
fn sheets_lie_under_the_flame_layers() {
    let mut scene = random_scene(7);
    let plain = render(&scene, &small(1), &Control::default()).unwrap();
    scene.sheets = wallpaper(7, Layout::Discs, Material::Paper, true).sheets;
    let over = render(&scene, &small(1), &Control::default()).unwrap();
    assert_ne!(plain.hash(), over.hash());
}

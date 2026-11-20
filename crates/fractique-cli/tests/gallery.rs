use std::fs;
use std::path::{Path, PathBuf};

use fractique_core::{Scene, file};
use fractique_mcp::load;
use serde_json::{Value, json};

#[derive(Clone, Copy, PartialEq, Debug)]
enum Section {
    Flames,
    Fractals,
    Wallpapers,
}

const AT_LEAST: [(Section, usize); 3] = [
    (Section::Flames, 100),
    (Section::Fractals, 12),
    (Section::Wallpapers, 12),
];

fn section(path: &Path) -> Section {
    match path.file_name().unwrap().to_string_lossy().chars().next() {
        Some('F') => Section::Fractals,
        Some('W') => Section::Wallpapers,
        _ => Section::Flames,
    }
}

fn gallery() -> Vec<PathBuf> {
    let folder = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/gallery");
    let mut files: Vec<PathBuf> = fs::read_dir(&folder)
        .expect("docs/gallery")
        .map(|entry| entry.expect("an entry").path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "frcq"))
        .collect();
    files.sort();
    files
}

#[test]
fn every_file_of_the_gallery_loads_as_a_valid_scene_of_its_section() {
    let files = gallery();
    for (which, least) in AT_LEAST {
        let found = files.iter().filter(|path| section(path) == which).count();
        assert!(found >= least, "the gallery lost files: {found} {which:?}");
    }
    for path in &files {
        let at = path.display();
        let scene = load(path).unwrap_or_else(|error| panic!("{error}"));
        assert_eq!(path.file_stem().unwrap().to_string_lossy()[4..], scene.name, "{at}");
        match section(path) {
            Section::Flames => assert!(scene.layers.iter().all(|layer| layer.flame().is_some()), "{at}"),
            Section::Fractals => assert!(scene.layers.iter().any(|layer| layer.flame().is_none()), "{at}"),
            Section::Wallpapers => assert!(scene.sheets.is_some(), "{at}"),
        }
    }
}

#[test]
fn every_file_of_the_gallery_is_a_document_the_app_opens() {
    for path in gallery() {
        let at = path.display();
        let document = file::unpack(&fs::read(&path).unwrap()).unwrap_or_else(|error| panic!("{at}: {error}"));
        assert_eq!(document["fractique"], 1, "{at}");
        for key in ["scene", "shown"] {
            Scene::from_json(&document[key].to_string()).unwrap_or_else(|error| panic!("{at}: {key}: {error}"));
        }
        assert!(
            document["title"].as_str().is_some_and(|title| !title.is_empty()),
            "{at}"
        );
        assert!(document["random"].is_boolean(), "{at}");
        assert!(document["seed"].is_u64(), "{at}");
        assert_eq!(document["picture"], json!([3840, 2160]), "{at}");
        let wall = &document["wall"];
        let shapes = section(&path) == Section::Wallpapers;
        for key in ["on", "live"] {
            assert_eq!(wall[key], Value::Bool(shapes), "{at}: {key}");
        }
        assert!(wall["dark"].is_boolean(), "{at}");
        match section(&path) {
            Section::Flames => {}
            Section::Fractals => {
                assert_eq!(document["fractals"], true, "{at}");
                assert_eq!(
                    document["kind"], document["shown"]["layers"][0]["fractal"]["kind"],
                    "{at}"
                );
            }
            Section::Wallpapers => {
                for key in ["layout", "material"] {
                    assert!(wall[key].is_string(), "{at}: {key}");
                }
            }
        }
    }
}

#[test]
fn every_file_of_the_gallery_has_its_picture_and_its_place_on_the_page() {
    let page = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/gallery.md")).unwrap();
    for path in gallery() {
        assert!(path.with_extension("webp").is_file(), "{}", path.display());
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        assert!(page.contains(&format!("(gallery/{name})")), "{name}");
    }
}

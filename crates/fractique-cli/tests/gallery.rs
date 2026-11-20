use std::fs;
use std::path::{Path, PathBuf};

use fractique_core::{Scene, file};
use fractique_mcp::load;
use serde_json::Value;

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
fn every_flame_of_the_gallery_loads_as_a_valid_scene() {
    let files = gallery();
    assert!(files.len() >= 100, "the gallery lost files: {}", files.len());
    for path in &files {
        let scene = load(path).unwrap_or_else(|error| panic!("{error}"));
        assert!(
            scene.layers.iter().all(|layer| layer.flame().is_some()),
            "{}",
            path.display()
        );
        assert_eq!(
            path.file_stem().unwrap().to_string_lossy()[4..],
            scene.name,
            "{}",
            path.display()
        );
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
        assert_eq!(document["random"], false, "{at}");
        assert!(document["seed"].is_u64(), "{at}");
        assert_eq!(document["picture"], serde_json::json!([3840, 2160]), "{at}");
        for key in ["on", "live", "dark"] {
            assert_eq!(document["wall"][key], Value::Bool(false), "{at}: {key}");
        }
    }
}

#[test]
fn every_flame_of_the_gallery_has_its_picture_and_its_place_on_the_page() {
    let page = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/gallery.md")).unwrap();
    for path in gallery() {
        assert!(path.with_extension("webp").is_file(), "{}", path.display());
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        assert!(page.contains(&format!("(gallery/{name})")), "{name}");
    }
}

use std::fs;
use std::path::{Path, PathBuf};

use fractique_core::{Scene, file};
use fractique_mcp::load;
use serde_json::{Value, json};

const AT_LEAST: [(&str, usize); 3] = [("flames", 100), ("fractals", 100), ("wallpapers", 100)];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/gallery")
}

fn files_of(folder: &str) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = fs::read_dir(root().join(folder))
        .unwrap_or_else(|error| panic!("docs/gallery/{folder}: {error}"))
        .map(|entry| entry.expect("an entry").path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "frcq"))
        .collect();
    files.sort();
    files
}

fn gallery() -> Vec<(&'static str, PathBuf)> {
    AT_LEAST
        .iter()
        .flat_map(|(folder, _)| files_of(folder).into_iter().map(move |path| (*folder, path)))
        .collect()
}

fn targets(page: &str) -> Vec<&str> {
    page.split("](")
        .skip(1)
        .filter_map(|rest| rest.split(')').next())
        .collect()
}

fn downloads() -> String {
    format!("{}/raw/master/", env!("CARGO_PKG_REPOSITORY"))
}

// A relative link is followed from its page; a download link must be this repository's raw address of a file.
fn led_to(page: &Path, target: &str) -> Option<PathBuf> {
    let file = target.split('#').next().unwrap_or(target);
    if !file.contains("://") {
        return Some(page.parent()?.join(file));
    }
    let inside = file.strip_prefix(&downloads())?;
    Some(Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(inside))
}

#[test]
fn every_file_of_the_gallery_loads_as_a_valid_scene_of_its_folder() {
    for (folder, least) in AT_LEAST {
        let found = files_of(folder).len();
        assert!(found >= least, "the gallery lost files: {found} in {folder}");
    }
    for (folder, path) in gallery() {
        let at = path.display();
        let scene = load(&path).unwrap_or_else(|error| panic!("{error}"));
        let stem = path.file_stem().unwrap().to_string_lossy().into_owned();
        assert_eq!(
            stem.split_once('-').map(|(_, name)| name),
            Some(scene.name.as_str()),
            "{at}"
        );
        match folder {
            "flames" => assert!(scene.layers.iter().all(|layer| layer.flame().is_some()), "{at}"),
            "fractals" => assert!(scene.layers.iter().any(|layer| layer.flame().is_none()), "{at}"),
            _ => assert!(scene.sheets.is_some(), "{at}"),
        }
    }
}

#[test]
fn every_file_of_the_gallery_is_a_document_the_app_opens() {
    for (folder, path) in gallery() {
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
        for key in ["on", "live"] {
            assert_eq!(wall[key], Value::Bool(folder == "wallpapers"), "{at}: {key}");
        }
        assert!(wall["dark"].is_boolean(), "{at}");
        match folder {
            "fractals" => {
                assert_eq!(document["fractals"], true, "{at}");
                assert_eq!(
                    document["kind"], document["shown"]["layers"][0]["fractal"]["kind"],
                    "{at}"
                );
            }
            "wallpapers" => {
                for key in ["layout", "material"] {
                    assert!(wall[key].is_string(), "{at}: {key}");
                }
            }
            _ => {}
        }
    }
}

#[test]
fn every_file_of_the_gallery_has_its_picture_its_download_and_its_place_on_the_page_of_its_folder() {
    for (folder, _) in AT_LEAST {
        let page = fs::read_to_string(root().join(folder).join("README.md")).unwrap();
        for path in files_of(folder) {
            assert!(path.with_extension("webp").is_file(), "{}", path.display());
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            let picture = name.replace(".frcq", ".webp");
            let download = format!("{}docs/gallery/{folder}/{name}", downloads());
            assert!(
                page.contains(&format!("]({picture})]({download})")),
                "{name}: the picture"
            );
            assert!(page.contains(&format!("]({name})")), "{name}: the name");
        }
    }
}

#[test]
fn every_link_of_the_pages_of_the_gallery_leads_to_a_file() {
    let front = root().join("README.md");
    let text = fs::read_to_string(&front).unwrap();
    for (folder, _) in AT_LEAST {
        assert!(text.contains(&format!("({folder}/README.md)")), "{folder}");
    }
    let pages = AT_LEAST.iter().map(|(folder, _)| root().join(folder).join("README.md"));
    for page in std::iter::once(front).chain(pages) {
        let text = fs::read_to_string(&page).unwrap();
        let found = targets(&text);
        assert!(!found.is_empty(), "{}", page.display());
        for target in found {
            let file = led_to(&page, target);
            assert!(file.is_some_and(|file| file.is_file()), "{}: {target}", page.display());
        }
    }
}

#[test]
fn a_download_link_to_a_wrong_folder_or_name_leads_nowhere() {
    let page = root().join("README.md");
    let right = format!("{}docs/gallery/flames/001-phoenix-flare.frcq", downloads());
    assert!(led_to(&page, &right).is_some_and(|file| file.is_file()));
    for wrong in [
        right.replace("/flames/", "/fractals/"),
        right.replace("001-", "000-"),
        right.replace("/raw/master/", "/raw/main/"),
        right.replace("/fractique/", "/another/"),
    ] {
        assert!(!led_to(&page, &wrong).is_some_and(|file| file.is_file()), "{wrong}");
    }
}

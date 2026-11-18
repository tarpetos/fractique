use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

fn fractique(arguments: &[&str]) -> (bool, Vec<u8>, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_fractique"))
        .args(arguments)
        .output()
        .expect("the command runs");
    (
        output.status.success(),
        output.stdout,
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

fn done(arguments: &[&str]) -> Vec<u8> {
    let (ok, stdout, stderr) = fractique(arguments);
    assert!(ok, "{arguments:?}: {stderr}");
    stdout
}

fn folder(name: &str) -> PathBuf {
    let folder = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = fs::remove_dir_all(&folder);
    fs::create_dir_all(&folder).expect("a folder for the test");
    folder
}

fn hash_of(scene: &str, picture: &str) -> String {
    let (ok, _, stderr) = fractique(&["render", scene, "--size", "64x36", "--quality", "draft", "-o", picture]);
    assert!(ok, "{stderr}");
    stderr.split("hash ").nth(1).expect("a hash")[..16].to_owned()
}

#[test]
fn a_scene_goes_to_frcq_and_back() {
    let folder = folder("convert-round-trip");
    let at = |name: &str| folder.join(name).to_string_lossy().into_owned();
    let (json, frcq, back, picture) = (at("scene.json"), at("scene.frcq"), at("back.json"), at("picture.png"));
    fs::write(&json, done(&["preset", "emerald-star"])).unwrap();
    done(&["convert", &json, "-o", &frcq]);
    let packed = fs::read(&frcq).unwrap();
    assert!(packed.starts_with(b"FRCQ"));
    assert!(packed.len() * 2 < fs::metadata(&json).unwrap().len() as usize);

    done(&["convert", &frcq, "-o", &back]);
    assert_eq!(fs::read(&back).unwrap(), fs::read(&json).unwrap());
    assert_eq!(done(&["convert", &frcq]), fs::read(&json).unwrap());
    assert_eq!(hash_of(&frcq, &picture), hash_of(&json, &picture));
}

#[test]
fn random_scenes_and_wallpapers_render_the_same_after_the_round_trip() {
    let folder = folder("convert-random");
    let at = |name: &str| folder.join(name).to_string_lossy().into_owned();
    let (json, frcq, back, picture) = (at("scene.json"), at("scene.frcq"), at("back.json"), at("picture.png"));
    for source in [["randomize", "42"], ["wallpaper", "7"]] {
        fs::write(&json, done(&source)).unwrap();
        done(&["convert", &json, "-o", &frcq]);
        done(&["convert", &frcq, "-o", &back]);
        let expected = hash_of(&json, &picture);
        assert_eq!(hash_of(&frcq, &picture), expected, "{source:?}");
        assert_eq!(hash_of(&back, &picture), expected, "{source:?}");
    }
}

#[test]
fn a_document_goes_to_json_and_back() {
    let folder = folder("convert-document");
    let at = |name: &str| folder.join(name).to_string_lossy().into_owned();
    let (json, frcq, document, again) = (
        at("scene.json"),
        at("scene.frcq"),
        at("document.json"),
        at("again.frcq"),
    );
    fs::write(&json, done(&["preset", "aurora-spiral"])).unwrap();
    done(&["convert", &json, "-o", &frcq]);

    done(&["convert", &frcq, "--document", "-o", &document]);
    let unpacked: Value = serde_json::from_slice(&fs::read(&document).unwrap()).unwrap();
    let scene: Value = serde_json::from_slice(&fs::read(&json).unwrap()).unwrap();
    assert_eq!(unpacked["fractique"], 1);
    assert_eq!(unpacked["shown"], scene);
    assert_eq!(unpacked["scene"], scene);
    assert_eq!(unpacked["title"], "aurora-spiral");

    done(&["convert", &document, "-o", &again]);
    assert_eq!(fs::read(&again).unwrap(), fs::read(&frcq).unwrap());
}

#[test]
fn every_command_that_reads_a_scene_reads_a_frcq() {
    let folder = folder("convert-commands");
    let at = |name: &str| folder.join(name).to_string_lossy().into_owned();
    let (json, frcq, picture) = (at("scene.json"), at("scene.frcq"), at("picture.png"));
    fs::write(&json, done(&["randomize", "5"])).unwrap();
    done(&["convert", &json, "-o", &frcq]);

    assert_eq!(hash_of(&frcq, &picture), hash_of(&json, &picture));
    assert_eq!(done(&["mutate", &frcq, "3"]), done(&["mutate", &json, "3"]));
    assert_eq!(done(&["fit", &frcq]), done(&["fit", &json]));
}

#[test]
fn what_cannot_be_converted_is_refused() {
    let folder = folder("convert-refused");
    let at = |name: &str| folder.join(name).to_string_lossy().into_owned();
    let (json, damaged, wrong) = (at("scene.json"), at("damaged.frcq"), at("wrong.json"));
    fs::write(&json, done(&["preset", "aurora-spiral"])).unwrap();
    fs::write(&damaged, b"FRCQ\x01nonsense").unwrap();
    fs::write(&wrong, br#"{ "version": 1 }"#).unwrap();

    let refused = |arguments: &[&str], part: &str| {
        let (ok, _, stderr) = fractique(arguments);
        assert!(!ok && stderr.contains(part), "{arguments:?}: {stderr}");
    };
    refused(&["convert", &json], "-o FILE.frcq");
    refused(&["convert", &json, "--document", "-o", &at("out.frcq")], "--document");
    refused(&["convert", &damaged], "damaged");
    refused(&["convert", &wrong, "-o", &at("out.frcq")], "wrong.json");
    refused(&["convert", &at("missing.frcq")], "missing.frcq");
}

#[test]
fn the_generator_is_chosen_by_its_number_and_the_first_is_the_default() {
    let first = done(&["randomize", "42"]);
    assert_eq!(done(&["randomize", "42", "--generator", "1"]), first);
    let second = done(&["randomize", "42", "--generator", "2"]);
    assert_ne!(second, first);
    assert_eq!(done(&["randomize", "42", "--generator", "2"]), second);

    let folder = folder("generator");
    let at = |name: &str| folder.join(name).to_string_lossy().into_owned();
    let (scene, picture) = (at("scene.json"), at("picture.png"));
    let rendered = |arguments: &[&str]| {
        let mut all = vec!["render", "--size", "64x36", "--quality", "draft", "-o", &picture];
        all.extend_from_slice(arguments);
        let (ok, _, stderr) = fractique(&all);
        assert!(ok, "{stderr}");
        stderr.split("hash ").nth(1).expect("a hash")[..16].to_owned()
    };
    fs::write(&scene, &first).unwrap();
    assert_eq!(rendered(&["--random", "42"]), hash_of(&scene, &picture));
    fs::write(&scene, &second).unwrap();
    assert_eq!(
        rendered(&["--random", "42", "--generator", "2"]),
        hash_of(&scene, &picture)
    );

    let (ok, _, stderr) = fractique(&["randomize", "42", "--generator", "3"]);
    assert!(!ok && stderr.contains("no generator has the number 3"), "{stderr}");
    assert!(
        !fractique(&[
            "render",
            "--preset",
            "aurora-spiral",
            "--generator",
            "2",
            "-o",
            &picture
        ])
        .0
    );
}

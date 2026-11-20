use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use fractique_update::{CLI, Failure, Fetch, Https, Options, Outcome, Setup, hint, run};

const KEY: &str = include_str!("fixtures/release.pub");
const OTHER_KEY: &str = include_str!("fixtures/other.pub");
const ARCHIVE_NAME: &str = "fractique-cli_9.9.9_x86_64-linux.tar.gz";
const ARCHIVE: &[u8] = include_bytes!("fixtures/fractique-cli_9.9.9_x86_64-linux.tar.gz");
const ARCHIVE_SIGNATURE: &str = include_str!("fixtures/fractique-cli_9.9.9_x86_64-linux.tar.gz.sig");
const BARE_NAME: &str = "fractique-cli_9.9.9_x64.exe";
const BARE: &[u8] = include_bytes!("fixtures/fractique-cli_9.9.9_x64.exe");
const BARE_SIGNATURE: &str = include_str!("fixtures/fractique-cli_9.9.9_x64.exe.sig");
const MCP_NAME: &str = "fractique-mcp_9.9.9_x86_64-linux.tar.gz";
const MCP_ARCHIVE: &[u8] = include_bytes!("fixtures/fractique-mcp_9.9.9_x86_64-linux.tar.gz");
const MCP_SIGNATURE: &str = include_str!("fixtures/fractique-mcp_9.9.9_x86_64-linux.tar.gz.sig");
const NEW: &[u8] = b"#!/bin/sh\necho \"fractique 9.9.9\"\n";
const OLD: &[u8] = b"#!/bin/sh\necho \"fractique 0.1.0\"\n";

type Pages = Arc<Mutex<HashMap<String, Vec<u8>>>>;

struct Site {
    releases: String,
    pages: Pages,
    asked: Arc<Mutex<Vec<String>>>,
}

impl Site {
    // Answers like GitHub: /latest/download/<file> redirects to the newest release's file.
    fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("a port");
        let releases = format!("http://{}", listener.local_addr().expect("an address"));
        let pages = Pages::default();
        let asked = Arc::new(Mutex::new(Vec::new()));
        let (held, seen) = (Arc::clone(&pages), Arc::clone(&asked));
        thread::spawn(move || {
            for stream in listener.incoming().map_while(Result::ok) {
                let mut lines = BufReader::new(&stream).lines().map_while(Result::ok);
                let Some(request) = lines.next() else { continue };
                lines.find(|line| line.is_empty());
                let path = request.split(' ').nth(1).unwrap_or_default().to_owned();
                seen.lock().expect("a lock").push(path.clone());
                let body = held.lock().expect("a lock").get(&path).cloned();
                let head = match (path.strip_prefix("/latest/download/"), &body) {
                    (Some(name), _) => format!("302 Found\r\nLocation: /download/v9.9.9/{name}\r\nContent-Length: 0"),
                    (None, Some(body)) => format!("200 OK\r\nContent-Length: {}", body.len()),
                    (None, None) => "404 Not Found\r\nContent-Length: 0".to_owned(),
                };
                let mut stream = &stream;
                let _ = write!(stream, "HTTP/1.1 {head}\r\nConnection: close\r\n\r\n")
                    .and_then(|()| stream.write_all(&body.unwrap_or_default()));
            }
        });
        Self { releases, pages, asked }
    }

    fn put(&self, name: &str, bytes: &[u8]) {
        let path = format!("/download/v9.9.9/{name}");
        self.pages.lock().expect("a lock").insert(path, bytes.to_vec());
    }

    fn list(&self, entries: &[(&str, &str, &str, &str)]) {
        let programs: Vec<String> = entries
            .iter()
            .map(|(program, platform, name, signature)| {
                format!(
                    r#""{program}": {{ "{platform}": {{ "url": "{}/download/v9.9.9/{name}", "signature": "{signature}" }} }}"#,
                    self.releases
                )
            })
            .collect();
        let tools = format!(
            r#"{{ "version": "9.9.9", "pub_date": "2026-11-20T12:00:00Z", "programs": {{ {} }} }}"#,
            programs.join(", ")
        );
        self.put("tools.json", tools.as_bytes());
    }

    fn release() -> Self {
        let site = Self::start();
        site.put(ARCHIVE_NAME, ARCHIVE);
        site.list(&[("fractique-cli", "linux-x86_64", ARCHIVE_NAME, ARCHIVE_SIGNATURE)]);
        site
    }

    fn asked(&self) -> Vec<String> {
        self.asked.lock().expect("a lock").clone()
    }
}

struct Copy {
    folder: tempfile::TempDir,
    exe: PathBuf,
}

impl Copy {
    fn named(name: &str) -> Self {
        let folder = tempfile::tempdir().expect("a folder");
        let exe = folder.path().join(name);
        fs::write(&exe, OLD).expect("a file");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&exe, fs::Permissions::from_mode(0o755)).expect("a mode");
        }
        Self { folder, exe }
    }

    fn untouched(&self) -> bool {
        fs::read(&self.exe).expect("a file") == OLD && fs::read_dir(self.folder.path()).expect("a folder").count() == 1
    }

    #[cfg(unix)]
    fn says(&self) -> String {
        let output = std::process::Command::new(&self.exe)
            .output()
            .expect("the program runs");
        String::from_utf8(output.stdout).expect("text")
    }
}

fn setup(releases: &str, exe: &Path, current: &str) -> Setup {
    Setup {
        program: CLI,
        current: current.into(),
        release: true,
        platform: Some("linux-x86_64".into()),
        exe: exe.to_owned(),
        appdir: None,
        pubkey: KEY.into(),
        releases: releases.to_owned(),
        fetch: Arc::new(Https),
    }
}

const CHECK: Options = Options {
    check: true,
    version: None,
};
const UPDATE: Options = Options {
    check: false,
    version: None,
};

fn named(version: &str) -> Options {
    Options {
        check: false,
        version: Some(version.to_owned()),
    }
}

#[test]
fn check_says_that_a_newer_release_is_out_and_replaces_nothing() {
    let (site, copy) = (Site::release(), Copy::named("fractique"));
    let report = run(&setup(&site.releases, &copy.exe, "0.1.0"), &CHECK);
    assert_eq!(report.outcome, Outcome::Newer, "{}", report.text);
    assert_eq!(report.outcome.code(), 10);
    assert_eq!(
        report.text,
        "fractique 9.9.9 is out; this is 0.1.0.\nInstall it with: fractique update"
    );
    assert!(copy.untouched());
    assert_eq!(
        site.asked(),
        ["/latest/download/tools.json", "/download/v9.9.9/tools.json"]
    );
}

#[test]
fn check_says_when_this_is_the_newest() {
    let (site, copy) = (Site::release(), Copy::named("fractique"));
    let same = run(&setup(&site.releases, &copy.exe, "9.9.9"), &CHECK);
    assert_eq!((same.outcome, same.outcome.code()), (Outcome::Current, 0));
    assert_eq!(same.text, "fractique 9.9.9 is the newest version.");
    let ahead = run(&setup(&site.releases, &copy.exe, "10.0.0"), &CHECK);
    assert_eq!(ahead.outcome, Outcome::Current);
    assert_eq!(ahead.text, "fractique 10.0.0 is newer than the newest release, 9.9.9.");
    let again = run(&setup(&site.releases, &copy.exe, "9.9.9"), &UPDATE);
    assert_eq!(again.outcome, Outcome::Current);
    assert!(copy.untouched());
}

#[test]
fn check_cannot_find_out_without_an_answer() {
    let copy = Copy::named("fractique");
    let closed = TcpListener::bind("127.0.0.1:0").expect("a port");
    let releases = format!("http://{}", closed.local_addr().expect("an address"));
    drop(closed);
    let report = run(&setup(&releases, &copy.exe, "0.1.0"), &CHECK);
    assert_eq!(
        (report.outcome, report.outcome.code()),
        (Outcome::Failed, 1),
        "{}",
        report.text
    );
    assert!(copy.untouched());
}

#[test]
fn update_replaces_the_program_with_the_one_in_the_archive() {
    let (site, copy) = (Site::release(), Copy::named("fractique"));
    let report = run(&setup(&site.releases, &copy.exe, "0.1.0"), &UPDATE);
    assert_eq!(
        (report.outcome, report.outcome.code()),
        (Outcome::Installed, 0),
        "{}",
        report.text
    );
    assert!(
        report.text.starts_with("fractique 0.1.0 is now 9.9.9: "),
        "{}",
        report.text
    );
    assert!(report.text.contains("until it is restarted"), "{}", report.text);
    assert_eq!(fs::read(&copy.exe).expect("a file"), NEW);
    assert_eq!(fs::read_dir(copy.folder.path()).expect("a folder").count(), 1);
    assert_eq!(
        site.asked(),
        [
            "/latest/download/tools.json",
            "/download/v9.9.9/tools.json",
            "/download/v9.9.9/fractique-cli_9.9.9_x86_64-linux.tar.gz"
        ]
    );
    #[cfg(unix)]
    assert_eq!(copy.says(), "fractique 9.9.9\n");
}

#[test]
fn update_replaces_the_program_with_a_bare_file_as_on_windows() {
    let (site, copy) = (Site::start(), Copy::named("fractique-cli_0.1.0_x64.exe"));
    site.put(BARE_NAME, BARE);
    site.list(&[("fractique-cli", "windows-x86_64", BARE_NAME, BARE_SIGNATURE)]);
    let mut windows = setup(&site.releases, &copy.exe, "0.1.0");
    windows.platform = Some("windows-x86_64".into());
    let report = run(&windows, &UPDATE);
    assert_eq!(report.outcome, Outcome::Installed, "{}", report.text);
    assert_eq!(fs::read(&copy.exe).expect("a file"), BARE);
    assert_eq!(fs::read_dir(copy.folder.path()).expect("a folder").count(), 1);
}

#[test]
fn a_named_version_is_installed_an_older_one_too() {
    let (site, copy) = (Site::release(), Copy::named("fractique"));
    let same = run(&setup(&site.releases, &copy.exe, "9.9.9"), &named("9.9.9"));
    assert_eq!(same.outcome, Outcome::Current);
    assert_eq!(same.text, "fractique 9.9.9 is the version you have.");
    assert!(copy.untouched());
    let older = run(&setup(&site.releases, &copy.exe, "10.0.0"), &named("v9.9.9"));
    assert_eq!(older.outcome, Outcome::Installed, "{}", older.text);
    assert_eq!(fs::read(&copy.exe).expect("a file"), NEW);
    assert_eq!(site.asked()[0], "/download/v9.9.9/tools.json");
    assert!(site.asked().iter().all(|path| !path.starts_with("/latest")));
}

#[test]
fn a_release_without_tools_json_is_named_as_such() {
    let (site, copy) = (Site::start(), Copy::named("fractique"));
    site.put(ARCHIVE_NAME, ARCHIVE);
    for options in [CHECK, UPDATE] {
        let report = run(&setup(&site.releases, &copy.exe, "0.1.0"), &options);
        assert_eq!((report.outcome, report.outcome.code()), (Outcome::Failed, 1));
        assert!(
            report.text.starts_with("the newest release holds no tools.json"),
            "{}",
            report.text
        );
        assert!(
            report.text.ends_with(&format!("{}/latest", site.releases)),
            "{}",
            report.text
        );
    }
    let report = run(&setup(&site.releases, &copy.exe, "0.1.0"), &named("0.0.9"));
    assert_eq!(report.outcome, Outcome::Failed);
    assert!(
        report.text.starts_with("release 0.0.9 holds no tools.json"),
        "{}",
        report.text
    );
    let report = run(&setup(&site.releases, &copy.exe, "0.1.0"), &named("the newest"));
    assert_eq!(report.outcome, Outcome::Failed);
    assert!(copy.untouched());
}

#[test]
fn a_download_that_is_not_what_was_signed_replaces_nothing() {
    let copy = Copy::named("fractique");
    let mut changed = ARCHIVE.to_vec();
    changed[40] ^= 1;
    let truncated = &ARCHIVE[..ARCHIVE.len() - 9];
    for bytes in [&changed[..], truncated, MCP_ARCHIVE, b""] {
        let site = Site::release();
        site.put(ARCHIVE_NAME, bytes);
        let report = run(&setup(&site.releases, &copy.exe, "0.1.0"), &UPDATE);
        assert_eq!(report.outcome, Outcome::Failed, "{}", report.text);
        assert!(report.text.ends_with("nothing was replaced"), "{}", report.text);
        assert!(copy.untouched());
    }
}

#[test]
fn a_signature_of_another_key_or_another_file_replaces_nothing() {
    let (site, copy) = (Site::release(), Copy::named("fractique"));
    let mut other = setup(&site.releases, &copy.exe, "0.1.0");
    other.pubkey = OTHER_KEY.into();
    assert_eq!(run(&other, &UPDATE).outcome, Outcome::Failed);
    assert!(copy.untouched());

    site.put(MCP_NAME, MCP_ARCHIVE);
    let wrong = [
        ("fractique-cli", "linux-x86_64", ARCHIVE_NAME, MCP_SIGNATURE),
        ("fractique-cli", "linux-x86_64", ARCHIVE_NAME, ""),
        ("fractique-cli", "linux-x86_64", MCP_NAME, MCP_SIGNATURE),
        (
            "fractique-cli",
            "linux-x86_64",
            "missing_9.9.9.tar.gz",
            ARCHIVE_SIGNATURE,
        ),
        (
            "fractique-cli",
            "linux-x86_64",
            "fractique-cli_9.9.9_missing.tar.gz",
            ARCHIVE_SIGNATURE,
        ),
    ];
    for entry in wrong {
        site.list(&[entry]);
        let report = run(&setup(&site.releases, &copy.exe, "0.1.0"), &UPDATE);
        assert_eq!(report.outcome, Outcome::Failed, "{}", report.text);
        assert!(report.text.ends_with("nothing was replaced"), "{}", report.text);
        assert!(copy.untouched());
    }
}

#[test]
fn a_release_without_this_program_for_this_system_replaces_nothing() {
    let (site, copy) = (Site::release(), Copy::named("fractique"));
    let mut arm = setup(&site.releases, &copy.exe, "0.1.0");
    arm.platform = Some("linux-aarch64".into());
    let report = run(&arm, &UPDATE);
    assert_eq!((report.outcome, report.outcome.code()), (Outcome::Refused, 3));
    assert_eq!(
        report.text,
        format!(
            "release 9.9.9 holds no fractique-cli for linux-aarch64.\nDownloads: {}/tag/v9.9.9",
            site.releases
        )
    );
    arm.platform = None;
    assert_eq!(run(&arm, &UPDATE).outcome, Outcome::Refused);
    let mut server = setup(&site.releases, &copy.exe, "0.1.0");
    server.program = fractique_update::SERVER;
    assert_eq!(run(&server, &UPDATE).outcome, Outcome::Refused);
    assert!(copy.untouched());
}

#[test]
fn a_copy_that_is_not_replaced_says_why_without_asking_anyone() {
    let (site, copy) = (Site::release(), Copy::named("fractique"));
    let mut built = setup(&site.releases, &copy.exe, "0.1.0");
    built.release = false;
    let report = run(&built, &UPDATE);
    assert_eq!((report.outcome, report.outcome.code()), (Outcome::Refused, 3));
    assert!(
        report.text.starts_with("this fractique was built from the source"),
        "{}",
        report.text
    );
    assert!(
        report.text.ends_with(&format!("Downloads: {}/latest", site.releases)),
        "{}",
        report.text
    );
    let report = run(&built, &named("9.9.9"));
    assert!(
        report
            .text
            .ends_with(&format!("Downloads: {}/tag/v9.9.9", site.releases)),
        "{}",
        report.text
    );
    assert!(site.asked().is_empty());

    let report = run(&built, &CHECK);
    assert_eq!(report.outcome, Outcome::Newer);
    assert!(
        report
            .text
            .starts_with("fractique 9.9.9 is out; this is 0.1.0.\nBut this fractique was built from the source"),
        "{}",
        report.text
    );
    assert!(copy.untouched());
}

struct Slow;

impl Fetch for Slow {
    fn get(&self, _: &str, _: Duration) -> Result<Vec<u8>, Failure> {
        thread::sleep(Duration::from_secs(5));
        Err(Failure::Missing)
    }
}

#[test]
fn the_hint_names_a_newer_release_on_a_terminal_only() {
    let (site, copy) = (Site::release(), Copy::named("fractique"));
    let limit = Duration::from_secs(20);
    let old = setup(&site.releases, &copy.exe, "0.1.0");
    assert_eq!(hint(&old, false, false, limit), None);
    assert_eq!(hint(&old, true, true, limit), None);
    let mut built = setup(&site.releases, &copy.exe, "0.1.0");
    built.release = false;
    assert_eq!(hint(&built, true, false, limit), None);
    assert!(site.asked().is_empty());

    assert_eq!(
        hint(&old, true, false, limit).as_deref(),
        Some("9.9.9 is out: fractique update")
    );
    assert_eq!(
        hint(&setup(&site.releases, &copy.exe, "9.9.9"), true, false, limit),
        None
    );
    assert_eq!(
        hint(&setup(&site.releases, &copy.exe, "10.0.0"), true, false, limit),
        None
    );
    assert!(copy.untouched());
}

#[test]
fn the_hint_is_silent_when_nothing_answers_in_time() {
    let copy = Copy::named("fractique");
    let without = Site::start();
    assert_eq!(
        hint(
            &setup(&without.releases, &copy.exe, "0.1.0"),
            true,
            false,
            Duration::from_secs(20)
        ),
        None
    );
    without.put("tools.json", b"<html>");
    assert_eq!(
        hint(
            &setup(&without.releases, &copy.exe, "0.1.0"),
            true,
            false,
            Duration::from_secs(20)
        ),
        None
    );

    let mut slow = setup(&without.releases, &copy.exe, "0.1.0");
    slow.fetch = Arc::new(Slow);
    let asked = Instant::now();
    assert_eq!(hint(&slow, true, false, Duration::from_millis(200)), None);
    assert!(asked.elapsed() < Duration::from_secs(3));
}

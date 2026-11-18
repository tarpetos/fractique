use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use cargo_packager_updater::semver::Version;
use cargo_packager_updater::{Config, Update, UpdaterBuilder, WindowsConfig, WindowsUpdateInstallMode, reqwest};

const REPOSITORY: &str = "tarpetos/fractique";
/// The first release that carries signed installers and a latest.json.
pub const OLDEST: &str = "0.1.0";
/// Public half of the release workflow's signing key (`cargo packager signer generate`).
const PUBKEY: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IEY2OUVDRDQxQjEzNjk3NEEKUldSS2x6YXhRYzJlOXM3dGJST3B0QUxtek5mVzI5ZjVRTERiendRYkIvTEFIalFnUDk1bmE3L28K";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Auto,
    Manual,
    Never,
}

pub fn is_release() -> bool {
    option_env!("FRACTIQUE_RELEASE").is_some()
}

pub fn kind() -> Kind {
    if !is_release() {
        return Kind::Never;
    }
    if cfg!(target_os = "linux") {
        return if std::env::var_os("APPIMAGE").is_some() {
            Kind::Auto
        } else {
            Kind::Manual
        };
    }
    if cfg!(target_os = "macos") {
        return if app_bundle().is_some() {
            Kind::Auto
        } else {
            Kind::Manual
        };
    }
    if own_folder_is_writable() {
        Kind::Auto
    } else {
        Kind::Manual
    }
}

// On Windows the app is one file that replaces itself, which it cannot do in a folder it may not write to.
fn own_folder_is_writable() -> bool {
    let Ok(exe) = std::env::current_exe() else {
        return false;
    };
    let probe = exe.with_extension("probe");
    let writable = std::fs::write(&probe, b"").is_ok();
    let _ = std::fs::remove_file(probe);
    writable
}

pub fn tidy() {
    if let Ok(exe) = std::env::current_exe() {
        let _ = std::fs::remove_file(exe.with_extension("old"));
        let _ = std::fs::remove_file(exe.with_extension("new"));
    }
}

#[cfg(windows)]
fn replace_self(bytes: &[u8]) -> Result<PathBuf, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let (new, old) = (exe.with_extension("new"), exe.with_extension("old"));
    std::fs::write(&new, bytes).map_err(|e| e.to_string())?;
    let _ = std::fs::remove_file(&old);
    // Windows refuses to overwrite a running program but lets it be renamed.
    std::fs::rename(&exe, &old).map_err(|e| e.to_string())?;
    if let Err(error) = std::fs::rename(&new, &exe) {
        let _ = std::fs::rename(&old, &exe);
        return Err(error.to_string());
    }
    Ok(exe)
}

fn app_bundle() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?.canonicalize().ok()?;
    exe.ancestors()
        .find(|folder| folder.extension().is_some_and(|e| e == "app"))
        .map(PathBuf::from)
}

pub fn release_page(version: &str) -> String {
    format!("https://github.com/{REPOSITORY}/releases/tag/v{version}")
}

fn manifest(version: Option<&str>) -> String {
    match version {
        Some(version) => format!("https://github.com/{REPOSITORY}/releases/download/v{version}/latest.json"),
        None => format!("https://github.com/{REPOSITORY}/releases/latest/download/latest.json"),
    }
}

fn config(version: Option<&str>) -> Result<Config, String> {
    let windows = WindowsConfig {
        install_mode: Some(WindowsUpdateInstallMode::Passive),
        installer_args: None,
    };
    Ok(Config {
        endpoints: vec![manifest(version).parse().map_err(|e| format!("{e}"))?],
        pubkey: PUBKEY.into(),
        windows: Some(windows),
    })
}

pub fn current() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

fn current_version() -> Result<Version, String> {
    current().parse().map_err(|e| format!("{e}"))
}

pub fn compare(version: &str) -> std::cmp::Ordering {
    match (Version::parse(version), current_version()) {
        (Ok(version), Ok(current)) => version.cmp(&current),
        _ => std::cmp::Ordering::Less,
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Release {
    pub version: String,
    pub date: String,
    pub notes: String,
}

#[derive(serde::Deserialize)]
struct ApiRelease {
    tag_name: String,
    #[serde(default)]
    body: Option<String>,
    #[serde(default)]
    published_at: Option<String>,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
}

pub fn releases() -> Result<Vec<Release>, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(30))
        .user_agent(concat!("fractique/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| e.to_string())?;
    let response = client
        .get(format!(
            "https://api.github.com/repos/{REPOSITORY}/releases?per_page=100"
        ))
        .header("Accept", "application/vnd.github+json")
        .send()
        .map_err(|e| e.to_string())?;
    let status = response.status();
    if status.as_u16() == 403 || status.as_u16() == 429 {
        return Err("GitHub is limiting requests from this network for now.".into());
    }
    if !status.is_success() {
        return Err(format!("GitHub answered {status}."));
    }
    let list: Vec<ApiRelease> = response.json().map_err(|e| e.to_string())?;
    Ok(listed(list))
}

fn listed(list: Vec<ApiRelease>) -> Vec<Release> {
    let Ok(oldest) = Version::parse(OLDEST) else {
        return Vec::new();
    };
    let mut out: Vec<(Version, Release)> = list
        .into_iter()
        .filter(|release| !release.draft && !release.prerelease)
        .filter_map(|release| {
            let version = Version::parse(release.tag_name.trim_start_matches('v')).ok()?;
            if version < oldest || !version.pre.is_empty() {
                return None;
            }
            let date = release
                .published_at
                .as_deref()
                .and_then(|d| d.get(..10))
                .unwrap_or_default()
                .to_owned();
            let notes = release.body.unwrap_or_default();
            Some((
                version.clone(),
                Release {
                    version: version.to_string(),
                    date,
                    notes,
                },
            ))
        })
        .collect();
    out.sort_by(|a, b| b.0.cmp(&a.0));
    out.dedup_by(|a, b| a.0 == b.0);
    out.into_iter().map(|(_, release)| release).collect()
}

pub struct Found {
    pub version: String,
    update: Update,
}

pub fn check() -> Result<Option<Found>, String> {
    let builder = UpdaterBuilder::new(current_version()?, config(None)?).timeout(Duration::from_secs(30));
    let update = builder
        .build()
        .map_err(|e| e.to_string())?
        .check()
        .map_err(|e| e.to_string())?;
    Ok(update.map(|update| Found {
        version: update.version.clone(),
        update,
    }))
}

pub fn fetch(version: &str) -> Result<Found, String> {
    let builder = UpdaterBuilder::new(current_version()?, config(Some(version))?)
        .timeout(Duration::from_secs(30))
        .version_comparator(|current, release| release.version != current);
    let update = builder
        .build()
        .map_err(|e| e.to_string())?
        .check()
        .map_err(|e| e.to_string())?;
    let update = update.ok_or_else(|| format!("{version} is the version you have."))?;
    Ok(Found {
        version: update.version.clone(),
        update,
    })
}

static READY: Mutex<Option<(Update, Vec<u8>)>> = Mutex::new(None);

pub fn download(found: &Found, progress: impl Fn(u64, Option<u64>)) -> Result<(), String> {
    let got = AtomicU64::new(0);
    let bytes = found
        .update
        .download_extended(
            |chunk, total| progress(got.fetch_add(chunk as u64, Ordering::Relaxed) + chunk as u64, total),
            || {},
        )
        .map_err(|e| e.to_string())?;
    *READY.lock().unwrap_or_else(|e| e.into_inner()) = Some((found.update.clone(), bytes));
    Ok(())
}

pub fn discard_pending() {
    READY.lock().unwrap_or_else(|e| e.into_inner()).take();
}

pub fn install_pending(relaunch: bool) {
    let Some((update, bytes)) = READY.lock().unwrap_or_else(|e| e.into_inner()).take() else {
        return;
    };
    #[cfg(windows)]
    {
        let _ = update;
        match replace_self(&bytes) {
            Ok(exe) if relaunch => {
                if let Err(error) = std::process::Command::new(exe).spawn() {
                    log::error!("the updated app could not be started: {error}");
                }
            }
            Ok(_) => {}
            Err(error) => log::error!("the update could not be installed: {error}"),
        }
    }
    #[cfg(not(windows))]
    install_bundle(&update, bytes, relaunch);
}

#[cfg(not(windows))]
fn install_bundle(update: &Update, bytes: Vec<u8>, relaunch: bool) {
    if let Err(error) = update.install(bytes) {
        log::error!("the update could not be installed: {error}");
        return;
    }
    if !relaunch {
        return;
    }
    let started = if cfg!(target_os = "macos") {
        app_bundle().map(|app| std::process::Command::new("open").arg("-n").arg(app).spawn())
    } else {
        std::env::var_os("APPIMAGE").map(|image| std::process::Command::new(image).spawn())
    };
    if let Some(Err(error)) = started {
        log::error!("the updated app could not be started: {error}");
    }
}

#[derive(Debug, PartialEq)]
pub enum Block {
    Heading(String),
    Item(String),
    Text(String),
}

pub fn blocks(notes: &str) -> Vec<Block> {
    let plain = |text: &str| text.replace("**", "").replace('`', "");
    let mut out: Vec<Block> = Vec::new();
    for line in notes.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            out.push(Block::Text(String::new()));
        } else if let Some(heading) = trimmed.strip_prefix('#') {
            out.push(Block::Heading(plain(heading.trim_start_matches('#').trim())));
        } else if let Some(item) = trimmed.strip_prefix("* ").or_else(|| trimmed.strip_prefix("- ")) {
            out.push(Block::Item(plain(item)));
        } else {
            match out.last_mut() {
                Some(Block::Item(text) | Block::Text(text)) if !text.is_empty() => {
                    text.push(' ');
                    text.push_str(&plain(trimmed));
                }
                _ => out.push(Block::Text(plain(trimmed))),
            }
        }
    }
    out.retain(|block| *block != Block::Text(String::new()));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn api(tag: &str, prerelease: bool) -> ApiRelease {
        ApiRelease {
            tag_name: tag.into(),
            body: Some(format!("notes {tag}")),
            published_at: Some("2026-11-14T16:45:45Z".into()),
            draft: false,
            prerelease,
        }
    }

    #[test]
    fn lists_installable_releases_newest_first() {
        let all = vec![
            api("v0.2.0", false),
            api("v0.0.9", false),
            api("v0.3.0", true),
            api("v0.10.0", false),
            api("nonsense", false),
        ];
        let got = listed(all);
        let versions: Vec<_> = got.iter().map(|release| release.version.as_str()).collect();
        assert_eq!(versions, ["0.10.0", "0.2.0"]);
        assert_eq!(got[1].date, "2026-11-14");
        assert_eq!(got[1].notes, "notes v0.2.0");
    }

    #[test]
    fn compares_with_this_copy() {
        assert_eq!(compare("999.0.0"), std::cmp::Ordering::Greater);
        assert_eq!(compare(current()), std::cmp::Ordering::Equal);
        assert_eq!(compare("0.0.1"), std::cmp::Ordering::Less);
        assert_eq!(compare("not a version"), std::cmp::Ordering::Less);
    }

    #[test]
    fn notes_become_headings_bullets_and_paragraphs() {
        let notes = "First version.\n\n* **Renderer:** flames with `28` variations,\n  layers.\n* **Fixes:**\n\n### Later\nA closing line\nthat wraps.";
        assert_eq!(
            blocks(notes),
            [
                Block::Text("First version.".into()),
                Block::Item("Renderer: flames with 28 variations, layers.".into()),
                Block::Item("Fixes:".into()),
                Block::Heading("Later".into()),
                Block::Text("A closing line that wraps.".into()),
            ]
        );
    }
}

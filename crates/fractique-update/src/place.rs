use std::fs;
use std::path::{Component, Path};

#[derive(Clone, Debug, PartialEq)]
pub enum Refusal {
    Development,
    AppImage,
    Bundle,
    Package(Option<String>),
    ReadOnly,
}

impl Refusal {
    pub fn says(&self, binary: &str, exe: &Path) -> String {
        let folder = exe.parent().unwrap_or(exe).display();
        match self {
            Self::Development => format!(
                "this {binary} was built from the source and is not a release, so it is not replaced. \
                 Build the version you want from the source: git pull, then setup.sh or cargo build --release."
            ),
            Self::AppImage => format!(
                "this {binary} is inside the AppImage, which the app updates as a whole: \
                 open Fractique and update it there. For a {binary} on its own, take its .tar.gz."
            ),
            Self::Bundle => format!(
                "this {binary} is inside the macOS app, which the app updates as a whole: \
                 open Fractique and update it there. For a {binary} on its own, take its .tar.gz."
            ),
            Self::Package(Some(package)) => format!(
                "this {binary} belongs to the package {package}, so it is not replaced: update the package. \
                 For Fractique that is the new .deb, installed over the old one."
            ),
            Self::Package(None) => format!(
                "this {binary} is in {folder}, where the system's packages are, so it is not replaced: \
                 update the package it came with."
            ),
            Self::ReadOnly => format!(
                "{folder} cannot be written to, so this {binary} is not replaced. Run the update with \
                 the rights to write there, or keep the program in a folder of your own."
            ),
        }
    }
}

const DPKG_LISTS: &str = "/var/lib/dpkg/info";
const SYSTEM_FOLDERS: [&str; 6] = ["/usr", "/bin", "/sbin", "/nix", "/snap", "/gnu"];

pub fn standing(exe: &Path, release: bool, appdir: Option<&Path>) -> Result<(), Refusal> {
    standing_with(exe, release, appdir, Path::new(DPKG_LISTS))
}

fn standing_with(exe: &Path, release: bool, appdir: Option<&Path>, dpkg_lists: &Path) -> Result<(), Refusal> {
    if !release {
        return Err(Refusal::Development);
    }
    if in_appimage(exe, appdir) {
        return Err(Refusal::AppImage);
    }
    if in_app_bundle(exe) {
        return Err(Refusal::Bundle);
    }
    if let Some(package) = package_of(exe, dpkg_lists) {
        return Err(Refusal::Package(Some(package)));
    }
    if in_system_folder(exe) {
        return Err(Refusal::Package(None));
    }
    if !exe.parent().is_some_and(writable) {
        return Err(Refusal::ReadOnly);
    }
    Ok(())
}

fn in_appimage(exe: &Path, appdir: Option<&Path>) -> bool {
    let mounted = |part: Component| part.as_os_str().to_string_lossy().starts_with(".mount_");
    appdir.is_some_and(|folder| exe.starts_with(folder)) || exe.components().any(mounted)
}

fn in_app_bundle(exe: &Path) -> bool {
    let parts: Vec<&Path> = exe.components().map(|part| Path::new(part.as_os_str())).collect();
    parts
        .windows(2)
        .any(|pair| pair[0].extension().is_some_and(|e| e == "app") && pair[1] == Path::new("Contents"))
}

fn in_system_folder(exe: &Path) -> bool {
    !exe.starts_with("/usr/local") && SYSTEM_FOLDERS.iter().any(|folder| exe.starts_with(folder))
}

fn package_of(exe: &Path, dpkg_lists: &Path) -> Option<String> {
    let wanted = exe.to_str()?;
    fs::read_dir(dpkg_lists).ok()?.flatten().find_map(|entry| {
        let list = entry.path();
        if list.extension().is_none_or(|e| e != "list") {
            return None;
        }
        let owns = fs::read_to_string(&list).ok()?.lines().any(|line| line == wanted);
        let package = list.file_stem()?.to_str()?.split(':').next()?.to_owned();
        owns.then_some(package)
    })
}

fn writable(folder: &Path) -> bool {
    let probe = folder.join(format!(".fractique-update-{}.probe", std::process::id()));
    let made = fs::OpenOptions::new().write(true).create_new(true).open(&probe).is_ok();
    let _ = fs::remove_file(probe);
    made
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nowhere() -> &'static Path {
        Path::new("/nonexistent/dpkg")
    }

    fn program_in(folder: &Path) -> std::path::PathBuf {
        let exe = folder.join("fractique");
        fs::write(&exe, b"program").expect("a file");
        exe
    }

    #[test]
    fn a_release_in_a_folder_of_ones_own_may_be_replaced() {
        let folder = tempfile::tempdir().expect("a folder");
        let exe = program_in(folder.path());
        assert_eq!(standing_with(&exe, true, None, nowhere()), Ok(()));
        assert_eq!(
            standing_with(&exe, true, Some(Path::new("/tmp/elsewhere")), nowhere()),
            Ok(())
        );
        assert_eq!(fs::read_dir(folder.path()).expect("a folder").count(), 1);
    }

    #[test]
    fn a_build_from_the_source_is_not_replaced() {
        let folder = tempfile::tempdir().expect("a folder");
        let exe = program_in(folder.path());
        assert_eq!(standing_with(&exe, false, None, nowhere()), Err(Refusal::Development));
    }

    #[test]
    fn a_program_inside_the_appimage_is_not_replaced() {
        let folder = tempfile::tempdir().expect("a folder");
        let bin = folder.path().join("usr/bin");
        fs::create_dir_all(&bin).expect("a folder");
        let exe = program_in(&bin);
        assert_eq!(
            standing_with(&exe, true, Some(folder.path()), nowhere()),
            Err(Refusal::AppImage)
        );
        let mounted = Path::new("/tmp/.mount_FractiAbCdEf/usr/bin/fractique-server");
        assert_eq!(standing_with(mounted, true, None, nowhere()), Err(Refusal::AppImage));
    }

    #[test]
    fn a_program_inside_the_macos_app_is_not_replaced() {
        let folder = tempfile::tempdir().expect("a folder");
        let inside = folder.path().join("Fractique.app/Contents/MacOS");
        fs::create_dir_all(&inside).expect("a folder");
        let exe = program_in(&inside);
        assert_eq!(standing_with(&exe, true, None, nowhere()), Err(Refusal::Bundle));
        let installed = Path::new("/Applications/Fractique.app/Contents/MacOS/fractique-mcp");
        assert_eq!(standing_with(installed, true, None, nowhere()), Err(Refusal::Bundle));
        let beside = folder.path().join("tools.app");
        fs::create_dir_all(&beside).expect("a folder");
        assert_eq!(standing_with(&program_in(&beside), true, None, nowhere()), Ok(()));
    }

    #[cfg(unix)]
    #[test]
    fn a_program_of_a_package_is_not_replaced() {
        let folder = tempfile::tempdir().expect("a folder");
        let lists = folder.path().join("info");
        fs::create_dir_all(&lists).expect("a folder");
        let exe = program_in(folder.path());
        fs::write(lists.join("other.list"), "/.\n/usr/bin/other\n").expect("a list");
        fs::write(lists.join("fractique.md5sums"), format!("{}\n", exe.display())).expect("a file");
        assert_eq!(standing_with(&exe, true, None, &lists), Ok(()));
        fs::write(lists.join("fractique:amd64.list"), format!("/.\n{}\n", exe.display())).expect("a list");
        assert_eq!(
            standing_with(&exe, true, None, &lists),
            Err(Refusal::Package(Some("fractique".into())))
        );
    }

    #[test]
    fn a_program_in_a_folder_of_the_system_is_not_replaced() {
        for path in [
            "/usr/bin/fractique",
            "/bin/fractique",
            "/usr/lib/fractique/fractique-server",
        ] {
            assert_eq!(
                standing_with(Path::new(path), true, None, nowhere()),
                Err(Refusal::Package(None)),
                "{path}"
            );
        }
        assert_eq!(
            standing_with(Path::new("/usr/local/nonexistent/fractique"), true, None, nowhere()),
            Err(Refusal::ReadOnly)
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_program_in_a_folder_that_cannot_be_written_to_is_not_replaced() {
        use std::os::unix::fs::PermissionsExt;
        let folder = tempfile::tempdir().expect("a folder");
        let exe = program_in(folder.path());
        fs::set_permissions(folder.path(), fs::Permissions::from_mode(0o555)).expect("a mode");
        let found = standing_with(&exe, true, None, nowhere());
        fs::set_permissions(folder.path(), fs::Permissions::from_mode(0o755)).expect("a mode");
        // root writes anywhere, so there the folder counts as writable.
        if fs::metadata(&exe).is_ok_and(|file| std::os::unix::fs::MetadataExt::uid(&file) == 0) {
            assert_eq!(found, Ok(()));
        } else {
            assert_eq!(found, Err(Refusal::ReadOnly));
        }
    }

    #[test]
    fn every_refusal_says_what_to_do() {
        let exe = Path::new("/opt/tools/fractique-server");
        let all = [
            Refusal::Development,
            Refusal::AppImage,
            Refusal::Bundle,
            Refusal::Package(Some("fractique".into())),
            Refusal::Package(None),
            Refusal::ReadOnly,
        ];
        for refusal in all {
            let text = refusal.says("fractique-server", exe);
            assert!(text.contains("fractique-server"), "{text}");
            assert!(!text.contains("  "), "{text}");
        }
        assert!(
            Refusal::ReadOnly
                .says("fractique", exe)
                .starts_with("/opt/tools cannot")
        );
    }
}

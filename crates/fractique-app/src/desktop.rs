use std::path::{Path, PathBuf};

// The id the window gives the system, which finds the app's entry by it: its icon in the dock and the list of
// apps, and the window it brings forward when the entry is clicked again. The .deb installs fractique-gui.desktop;
// an AppImage writes an entry of its own, under another name, so that one never hides the other.
pub fn app_id() -> &'static str {
    if std::env::var_os("APPIMAGE").is_some() {
        APPIMAGE_ID
    } else {
        "fractique-gui"
    }
}

const APPIMAGE_ID: &str = "fractique-appimage";
const ICON: &[u8] = include_bytes!("../../../assets/icon/icon-256.png");

// An AppImage is one file that nothing installs: at each start it puts its icon and an entry for the list of apps
// in the user's folders, pointing at where it is now, and shows the icon on the file itself in the file manager.
pub fn integrate() {
    let (Some(image), Some(data)) = (std::env::var_os("APPIMAGE").map(PathBuf::from), data_home()) else {
        return;
    };
    // Before the window opens: the system matches a window to an entry when it appears.
    if let Err(error) = add_entry(&image, &data) {
        log::warn!("the AppImage was not added to the list of apps: {error}");
        return;
    }
    // Also before the window: Files shows the icon at once instead of a moment after the start.
    show_icon(&image, &data);
}

fn data_home() -> Option<PathBuf> {
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| std::env::home_dir().map(|home| home.join(".local").join("share")))
}

fn icon_path(data: &Path) -> PathBuf {
    data.join("icons/hicolor/256x256/apps")
        .join(format!("{APPIMAGE_ID}.png"))
}

fn add_entry(image: &Path, data: &Path) -> std::io::Result<()> {
    write_if_changed(&icon_path(data), ICON)?;
    let entry = data.join("applications").join(format!("{APPIMAGE_ID}.desktop"));
    write_if_changed(&entry, desktop_entry(image).as_bytes())
}

// The file manager shows a file's own icon from GIO's metadata; where gio is missing the file keeps its icon.
fn show_icon(image: &Path, data: &Path) {
    let uri = format!("file://{}", icon_path(data).display());
    let shown = std::process::Command::new("gio")
        .args(["info", "-a", "metadata::custom-icon"])
        .arg(image)
        .output();
    if shown.is_ok_and(|shown| !String::from_utf8_lossy(&shown.stdout).contains(&uri)) {
        let _ = std::process::Command::new("gio")
            .args(["set", "-t", "string"])
            .arg(image)
            .args(["metadata::custom-icon", &uri])
            .status();
    }
}

fn write_if_changed(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if std::fs::read(path).is_ok_and(|old| old == bytes) {
        return Ok(());
    }
    if let Some(folder) = path.parent() {
        std::fs::create_dir_all(folder)?;
    }
    std::fs::write(path, bytes)
}

fn desktop_entry(image: &Path) -> String {
    // The quoting of an Exec key: the path in double quotes, with these four characters escaped.
    let mut quoted = String::new();
    for c in image.to_string_lossy().chars() {
        if matches!(c, '"' | '`' | '$' | '\\') {
            quoted.push('\\');
        }
        quoted.push(c);
    }
    format!(
        "[Desktop Entry]\nType=Application\nName=Fractique\nComment=Fractal flames, classic fractals and wallpapers\n\
         Exec=\"{quoted}\" %F\nIcon={APPIMAGE_ID}\nTerminal=false\nCategories=Graphics;\n\
         MimeType=application/x-fractique;\nStartupWMClass={APPIMAGE_ID}\n"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_appimage_adds_its_icon_and_its_entry() {
        let folder = std::env::temp_dir().join(format!("fractique-desktop-{}", std::process::id()));
        let image = folder.join("my apps").join("Fractique_1.0.1_x86_64.AppImage");
        std::fs::create_dir_all(image.parent().unwrap()).unwrap();
        std::fs::write(&image, b"").unwrap();
        let data = folder.join("share");
        add_entry(&image, &data).unwrap();
        show_icon(&image, &data);
        let entry = std::fs::read_to_string(data.join("applications/fractique-appimage.desktop")).unwrap();
        assert!(entry.contains(&format!("Exec=\"{}\" %F", image.display())));
        assert!(entry.contains("StartupWMClass=fractique-appimage"));
        assert_eq!(
            std::fs::read(data.join("icons/hicolor/256x256/apps/fractique-appimage.png")).unwrap(),
            ICON
        );
        // Where GIO keeps metadata, the file shows the icon.
        if let Ok(shown) = std::process::Command::new("gio")
            .args(["info", "-a", "metadata::custom-icon"])
            .arg(&image)
            .output()
            && shown.status.success()
        {
            assert!(String::from_utf8_lossy(&shown.stdout).contains("fractique-appimage.png"));
        }
        // Run again: nothing to change.
        add_entry(&image, &data).unwrap();
        show_icon(&image, &data);
        std::fs::remove_dir_all(&folder).unwrap();
    }

    #[test]
    fn an_exec_path_is_quoted() {
        let entry = desktop_entry(Path::new("/home/a\"b/$x/Fractique.AppImage"));
        assert!(entry.contains("Exec=\"/home/a\\\"b/\\$x/Fractique.AppImage\" %F"));
    }
}

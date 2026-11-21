use std::sync::OnceLock;

const TILING: [&str; 14] = [
    "i3",
    "sway",
    "hyprland",
    "bspwm",
    "dwm",
    "dwl",
    "xmonad",
    "river",
    "niri",
    "herbstluftwm",
    "leftwm",
    "spectrwm",
    "qtile",
    "ratpoison",
];
const SOCKETS: [&str; 4] = ["SWAYSOCK", "I3SOCK", "HYPRLAND_INSTANCE_SIGNATURE", "NIRI_SOCKET"];
const SESSIONS: [&str; 3] = ["XDG_CURRENT_DESKTOP", "XDG_SESSION_DESKTOP", "DESKTOP_SESSION"];

pub fn tiled() -> bool {
    static TILED: OnceLock<bool> = OnceLock::new();
    *TILED.get_or_init(|| match std::env::var("FRACTIQUE_TILING").as_deref() {
        Ok("on") => true,
        Ok("off") => false,
        _ => found(),
    })
}

fn found() -> bool {
    let said = SESSIONS
        .iter()
        .any(|key| std::env::var(key).is_ok_and(|value| names(&value)));
    let socket = SOCKETS.iter().any(|key| std::env::var_os(key).is_some());
    // Under Wayland the X server is the compositor's own and names no window manager of the session.
    let wayland = std::env::var_os("WAYLAND_DISPLAY").is_some() || std::env::var_os("WAYLAND_SOCKET").is_some();
    said || socket || (!wayland && manager().is_some_and(|name| names(&name)))
}

fn names(value: &str) -> bool {
    value.split(':').any(|part| {
        let name = part.rsplit('/').next().unwrap_or(part).trim().to_ascii_lowercase();
        TILING.contains(&name.as_str())
    })
}

fn manager() -> Option<String> {
    use x11rb::connection::Connection as _;
    use x11rb::protocol::xproto::{AtomEnum, ConnectionExt as _};

    let (x, screen) = x11rb::connect(None).ok()?;
    let root = x.setup().roots.get(screen)?.root;
    let atom = |name: &[u8]| Some(x.intern_atom(false, name).ok()?.reply().ok()?.atom);
    let check = atom(b"_NET_SUPPORTING_WM_CHECK")?;
    let owner = x
        .get_property(false, root, check, AtomEnum::WINDOW, 0, 1)
        .ok()?
        .reply()
        .ok()?
        .value32()?
        .next()?;
    let name = x
        .get_property(false, owner, atom(b"_NET_WM_NAME")?, AtomEnum::ANY, 0, 64)
        .ok()?
        .reply()
        .ok()?;
    String::from_utf8(name.value).ok()
}

#[cfg(test)]
mod tests {
    use super::names;

    #[test]
    fn a_tiling_window_manager_is_known_by_its_name() {
        for name in [
            "i3",
            "sway",
            "Hyprland",
            "ubuntu:sway",
            "/usr/share/xsessions/i3",
            " bspwm ",
        ] {
            assert!(names(name), "{name}");
        }
        for name in ["GNOME", "ubuntu:GNOME", "KDE", "Mutter", "GNOME Shell", "", "i3status"] {
            assert!(!names(name), "{name}");
        }
    }
}

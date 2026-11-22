use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use x11rb::connection::{Connection as _, RequestConnection as _};
use x11rb::protocol::shape::{self, ConnectionExt as _};
use x11rb::protocol::xproto::{ClientMessageEvent, ClipOrdering, ConnectionExt as _, EventMask, Rectangle, Window};
use x11rb::rust_connection::RustConnection;

fn window_of(handles: &impl HasWindowHandle) -> Option<Window> {
    match handles.window_handle().ok()?.as_raw() {
        RawWindowHandle::Xlib(window) => Window::try_from(window.window).ok(),
        RawWindowHandle::Xcb(window) => Some(window.window.get()),
        _ => None,
    }
}

// A window without the system's frame has square corners on Linux; Windows and macOS round them themselves.
pub struct Corners {
    connection: RustConnection,
    window: Window,
    shaped: Option<(u32, u32, u32)>,
}

impl Corners {
    pub fn of(handles: &impl HasWindowHandle) -> Option<Self> {
        let window = window_of(handles)?;
        let (connection, _) = x11rb::connect(None).ok()?;
        connection.extension_information(shape::X11_EXTENSION_NAME).ok()??;
        Some(Self {
            connection,
            window,
            shaped: None,
        })
    }

    pub fn set(&mut self, width: u32, height: u32, radius: u32) {
        let radius = radius.min(width / 2).min(height / 2);
        if self.shaped == Some((width, height, radius)) || width == 0 || height == 0 {
            return;
        }
        self.shaped = Some((width, height, radius));
        let rectangles = rounded(width, height, radius);
        let _ = self.connection.shape_rectangles(
            shape::SO::SET,
            shape::SK::BOUNDING,
            ClipOrdering::UNSORTED,
            self.window,
            0,
            0,
            &rectangles,
        );
        let _ = self.connection.flush();
    }
}

fn rounded(width: u32, height: u32, radius: u32) -> Vec<Rectangle> {
    let clamp = |value: u32| u16::try_from(value).unwrap_or(u16::MAX);
    let mut rectangles = Vec::with_capacity(2 * radius as usize + 1);
    let r = f64::from(radius);
    for row in 0..radius {
        let dy = r - f64::from(row) - 0.5;
        let inset = (r - (r * r - dy * dy).sqrt()).round() as u32;
        for y in [row, height - 1 - row] {
            rectangles.push(Rectangle {
                x: clamp(inset) as i16,
                y: clamp(y) as i16,
                width: clamp(width - 2 * inset),
                height: 1,
            });
        }
    }
    rectangles.push(Rectangle {
        x: 0,
        y: clamp(radius) as i16,
        width: clamp(width),
        height: clamp(height - 2 * radius),
    });
    rectangles
}

// What a dock does when its icon is clicked: GNOME brings forward a window a pager asks for.
pub struct Raise {
    connection: RustConnection,
    root: Window,
    window: Window,
}

impl Raise {
    pub fn of(handles: &impl HasWindowHandle) -> Option<Self> {
        let window = window_of(handles)?;
        let (connection, screen) = x11rb::connect(None).ok()?;
        let root = connection.setup().roots.get(screen)?.root;
        Some(Self {
            connection,
            root,
            window,
        })
    }

    pub fn activate(&self) -> bool {
        let Some(active) = self
            .connection
            .intern_atom(false, b"_NET_ACTIVE_WINDOW")
            .ok()
            .and_then(|cookie| Some(cookie.reply().ok()?.atom))
        else {
            return false;
        };
        let event = ClientMessageEvent::new(32, self.window, active, [2, 0, 0, 0, 0]);
        self.connection
            .send_event(
                false,
                self.root,
                EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY,
                event,
            )
            .is_ok()
            && self.connection.flush().is_ok()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn rounded_corners_cover_the_window_but_the_corners() {
        let rectangles = super::rounded(100, 60, 10);
        let area: u32 = rectangles
            .iter()
            .map(|r| u32::from(r.width) * u32::from(r.height))
            .sum();
        assert!(area < 100 * 60 && area > 100 * 60 - 100);
        assert_eq!(rectangles.last().unwrap().height, 40);
        assert!(rectangles.iter().all(|r| r.x as u32 + u32::from(r.width) <= 100));
        assert_eq!(super::rounded(100, 60, 0).len(), 1);
    }
}

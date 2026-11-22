use raw_window_handle::{HasDisplayHandle, HasWindowHandle, RawDisplayHandle, RawWindowHandle};
use wayland_client::backend::{Backend, ObjectId};
use wayland_client::globals::{GlobalListContents, registry_queue_init};
use wayland_client::protocol::{wl_registry, wl_surface::WlSurface};
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle};
use wayland_protocols::xdg::activation::v1::client::xdg_activation_v1::{self, XdgActivationV1};

struct Raise;

impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for Raise {
    fn event(
        _: &mut Self,
        _: &wl_registry::WlRegistry,
        _: wl_registry::Event,
        _: &GlobalListContents,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<XdgActivationV1, ()> for Raise {
    fn event(
        _: &mut Self,
        _: &XdgActivationV1,
        _: xdg_activation_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

pub fn on_wayland(frame: &eframe::Frame) -> bool {
    frame
        .display_handle()
        .is_ok_and(|display| matches!(display.as_raw(), RawDisplayHandle::Wayland(_)))
}

// Wayland brings a window to the front only with a token of what the user just did; winit has no call that takes
// one for a window that is already open, so the window's surface is activated here on winit's own connection.
pub fn activate(frame: &eframe::Frame, token: &str) -> bool {
    let (Ok(display), Ok(window)) = (frame.display_handle(), frame.window_handle()) else {
        return false;
    };
    let (RawDisplayHandle::Wayland(display), RawWindowHandle::Wayland(window)) = (display.as_raw(), window.as_raw())
    else {
        return false;
    };
    // The display is winit's: a connection made from it leaves it open when it goes.
    let backend = unsafe { Backend::from_foreign_display(display.display.as_ptr().cast()) };
    let connection = Connection::from_backend(backend);
    let Ok((globals, mut queue)) = registry_queue_init::<Raise>(&connection) else {
        return false;
    };
    let Ok(activation) = globals.bind::<XdgActivationV1, _, _>(&queue.handle(), 1..=1, ()) else {
        return false;
    };
    let Ok(id) = (unsafe { ObjectId::from_ptr(WlSurface::interface(), window.surface.as_ptr().cast()) }) else {
        return false;
    };
    let Ok(surface) = WlSurface::from_id(&connection, id) else {
        return false;
    };
    activation.activate(token.to_owned(), &surface);
    activation.destroy();
    queue.roundtrip(&mut Raise).is_ok()
}

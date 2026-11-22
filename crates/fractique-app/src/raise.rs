use std::ffi::c_void;
use std::ptr::NonNull;

use raw_window_handle::{HasDisplayHandle, HasWindowHandle, RawDisplayHandle, RawWindowHandle};
use wayland_client::backend::{Backend, ObjectId};
use wayland_client::globals::{GlobalListContents, registry_queue_init};
use wayland_client::protocol::{wl_registry, wl_surface::WlSurface};
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle};
use wayland_protocols::xdg::activation::v1::client::xdg_activation_token_v1::{self, XdgActivationTokenV1};
use wayland_protocols::xdg::activation::v1::client::xdg_activation_v1::{self, XdgActivationV1};

#[derive(Default)]
struct Raise {
    own: Option<String>,
}

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

impl Dispatch<XdgActivationTokenV1, ()> for Raise {
    fn event(
        raise: &mut Self,
        _: &XdgActivationTokenV1,
        event: xdg_activation_token_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let xdg_activation_token_v1::Event::Done { token } = event {
            raise.own = Some(token);
        }
    }
}

// The window's Wayland surface, taken when the app starts. A second start raises the window from the thread that
// hears it: the app's frames stop while its window is covered, and the second start would wait for one.
pub struct Surface {
    display: NonNull<c_void>,
    surface: NonNull<c_void>,
}

// libwayland's objects may be used from any thread, and the window lives as long as the app.
unsafe impl Send for Surface {}

impl Surface {
    pub fn of(handles: &(impl HasDisplayHandle + HasWindowHandle)) -> Option<Self> {
        let (RawDisplayHandle::Wayland(display), RawWindowHandle::Wayland(window)) = (
            handles.display_handle().ok()?.as_raw(),
            handles.window_handle().ok()?.as_raw(),
        ) else {
            return None;
        };
        Some(Self {
            display: display.display,
            surface: window.surface,
        })
    }

    // Wayland brings a window forward only with a token of what the user just did, which the dock and the file
    // manager hand to what they start. Without one, a start from a terminal, a token of the app's own is used:
    // the system then only says that the app wants attention.
    pub fn activate(&self, token: Option<&str>) -> bool {
        // The display is winit's: a connection made from it leaves it open when it goes.
        let backend = unsafe { Backend::from_foreign_display(self.display.as_ptr().cast()) };
        let connection = Connection::from_backend(backend);
        let Ok((globals, mut queue)) = registry_queue_init::<Raise>(&connection) else {
            return false;
        };
        let handle = queue.handle();
        let Ok(activation) = globals.bind::<XdgActivationV1, _, _>(&handle, 1..=1, ()) else {
            return false;
        };
        let Ok(id) = (unsafe { ObjectId::from_ptr(WlSurface::interface(), self.surface.as_ptr().cast()) }) else {
            return false;
        };
        let Ok(surface) = WlSurface::from_id(&connection, id) else {
            return false;
        };
        let mut raise = Raise::default();
        let token = match token {
            Some(token) => token.to_owned(),
            None => {
                let own = activation.get_activation_token(&handle, ());
                own.set_surface(&surface);
                own.commit();
                let _ = queue.roundtrip(&mut raise);
                own.destroy();
                let Some(token) = raise.own.take() else {
                    return false;
                };
                token
            }
        };
        activation.activate(token, &surface);
        activation.destroy();
        queue.roundtrip(&mut raise).is_ok()
    }
}

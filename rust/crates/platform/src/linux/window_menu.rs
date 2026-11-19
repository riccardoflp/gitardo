//! The menu of the window on X11, which `winit` opens on Wayland (`xdg_toplevel.show_window_menu`) but not on X11: the
//! window manager is asked for it with the message `_GTK_SHOW_WINDOW_MENU` to the root window, as GTK asks it, on the
//! connection of Xlib `winit` opened the window on. Xlib is reached through `x11-dl`, which loads `libX11` when the
//! program runs, as `winit` does, so that nothing of X11 is linked and Gitardo starts where only Wayland is.

use core::ffi::{c_int, c_long, c_ulong, c_void};
use core::ptr::NonNull;

use winit::dpi::PhysicalPosition;
use winit::raw_window_handle::{HasDisplayHandle, HasWindowHandle, RawDisplayHandle, RawWindowHandle};
use x11_dl::xlib;

/// The pointer the X server keeps for itself, the one `_GTK_SHOW_WINDOW_MENU` names: its number in `XInput` 2.
const VIRTUAL_CORE_POINTER: c_long = 2;

/// A place on the screen, in physical pixels from its top left corner, as the root window of X11 counts them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ScreenPlace {
    pub(crate) x: i32,
    pub(crate) y: i32,
}

/// A window `winit` opened on X11, with the connection of Xlib it is on.
pub(crate) struct X11Window {
    display: NonNull<c_void>,
    screen: c_int,
    window: c_ulong,
    inside: PhysicalPosition<i32>,
}

impl X11Window {
    /// The window, when `winit` opened it on X11; none on Wayland, where `winit` asks for the menu itself.
    pub(crate) fn of(window: &winit::window::Window) -> Option<X11Window> {
        let display = window.display_handle().ok()?.as_raw();
        let handle = window.window_handle().ok()?.as_raw();
        let (RawDisplayHandle::Xlib(display), RawWindowHandle::Xlib(handle)) = (display, handle) else {
            return None;
        };
        Some(X11Window {
            display: display.display?,
            screen: display.screen,
            window: handle.window,
            inside: window.inner_position().ok()?,
        })
    }

    /// Asks the window manager to open the menu of the window at a place of its inside, in physical pixels. A window
    /// manager that has no such menu ignores the message, and the window stays as it is.
    pub(crate) fn show_window_menu(&self, place: PhysicalPosition<f64>) {
        let Ok(library) = xlib::Xlib::open() else {
            // `winit` opened the window through the same library, so it is there; without it nothing can be asked.
            return;
        };
        let display = self.display.as_ptr().cast::<xlib::Display>();
        let message = window_menu_message(screen_place(self.inside, place));
        // SAFETY: `display` is the connection `winit` opened, alive while its window is, which the caller holds; the
        // name is a C string that outlives the call.
        let kind = unsafe { (library.XInternAtom)(display, c"_GTK_SHOW_WINDOW_MENU".as_ptr(), xlib::False) };
        // SAFETY: as above, and `screen` is the number of the screen `winit` gave with that connection.
        let root = unsafe { (library.XRootWindow)(display, self.screen) };
        let mut event = xlib::XEvent {
            client_message: xlib::XClientMessageEvent {
                type_: xlib::ClientMessage,
                serial: 0,
                send_event: xlib::True,
                display,
                window: self.window,
                message_type: kind,
                format: 32,
                data: message.into(),
            },
        };
        let mask = xlib::SubstructureRedirectMask | xlib::SubstructureNotifyMask;
        // SAFETY: `display` is alive as above; the press that asked for the menu holds the pointer for this program,
        // which is let go first so that the window manager can take it for its menu, as GTK does; `event` is a whole
        // client message alive for the call, which copies it.
        unsafe {
            let _ = (library.XUngrabPointer)(display, xlib::CurrentTime);
            let _ = (library.XSendEvent)(display, root, xlib::False, mask, &raw mut event);
            let _ = (library.XFlush)(display);
        }
    }
}

/// The place on the screen of a place of the inside of a window, whose inside starts at `inside` on the screen.
pub(crate) fn screen_place(inside: PhysicalPosition<i32>, place: PhysicalPosition<f64>) -> ScreenPlace {
    let place = place.cast::<i32>();
    ScreenPlace {
        x: inside.x.saturating_add(place.x),
        y: inside.y.saturating_add(place.y),
    }
}

/// The five numbers of the message `_GTK_SHOW_WINDOW_MENU`: the pointer, then the place on the screen; the last two
/// are not used.
pub(crate) fn window_menu_message(place: ScreenPlace) -> [c_long; 5] {
    [VIRTUAL_CORE_POINTER, c_long::from(place.x), c_long::from(place.y), 0, 0]
}

#[cfg(test)]
mod tests {
    #![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

    use super::*;

    #[test]
    fn window_title_bar__on_x11_the_menu_of_the_window_is_asked_of_the_window_manager_where_the_pointer_is() {
        let inside = PhysicalPosition::new(300, 120);
        let place = screen_place(inside, PhysicalPosition::new(41.6, 12.2));
        assert_eq!(place, ScreenPlace { x: 342, y: 132 });
        assert_eq!(window_menu_message(place), [VIRTUAL_CORE_POINTER, 342, 132, 0, 0]);
        // A window whose inside starts off the left of the screen still gives the place of the pointer on it.
        let place = screen_place(PhysicalPosition::new(-50, 0), PhysicalPosition::new(70.0, 5.0));
        assert_eq!(place, ScreenPlace { x: 20, y: 5 });
    }
}

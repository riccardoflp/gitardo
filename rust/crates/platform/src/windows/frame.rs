//! The procedure of a window that draws its own title bar (decision D58), put in front of the one of `winit` with
//! `SetWindowSubclass`. `winit` already lays the inside of a window without decorations over its whole frame
//! (`WM_NCCALCSIZE`), within the screen's work area while it is maximized, and keeps the styles of a window with a
//! frame, so that the system still gives it its shadow, its rounded corners, Aero Snap and its keys. This procedure
//! adds what the system asks of the places of the window (`WM_NCHITTEST`), from the function the program gave: the
//! caption moves the window, maximizes it with a double click and opens its menu with a right click, and the edges
//! resize it, all done by the system. The maximize button is said to be one (`HTMAXBUTTON`), which shows Snap Layouts
//! on Windows 11; the pointer over it then reaches the window as the system's and not as `winit`'s, so it is handed to
//! the program as input. `Alt+Space` opens the system's menu of the window, which `winit` keeps from the system. The
//! start of a move or a resize (`WM_ENTERSIZEMOVE`) is handed to the program too, which closes its menus then.

use std::cell::{Cell, RefCell};
use std::sync::Arc;

use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
use winit::window::Window as LibraryWindow;

use super::comctl32;
use super::user32::{self, HWND, LPARAM, LRESULT, WPARAM};
use crate::error::PlatformError;
use crate::frame::{FramePart, FrameParts, ResizeEdge};
use crate::geometry::{LogicalSize, PhysicalSize, Point, Scale};
use crate::input::{InputEvent, Modifiers, PointerButton, PointerButtonEvent, PointerMove};
use crate::mailbox::Mailbox;
use crate::translate;

#[cfg(test)]
mod tests;

/// The number this procedure is put in front of a window with, among the procedures of other libraries.
const SUBCLASS: usize = 0x4769_7461;

/// The procedure of a window that draws its own title bar, in front of the window's until dropped.
pub(crate) struct FrameProcedure {
    /// What the procedure keeps, at an address that does not move while the system calls it.
    hook: Box<FrameHook>,
}

/// What the procedure keeps.
struct FrameHook {
    /// The window.
    window: HWND,
    /// What each place of the window is, once the program said it.
    parts: RefCell<Option<FrameParts>>,
    /// Where the input it hands to the program goes.
    inbox: Arc<Mailbox<InputEvent>>,
    /// Whether the pointer is over the maximize button, which the system takes as its own.
    is_over_maximize: Cell<bool>,
    /// Whether the main button went down on the maximize button and is still held.
    is_pressed_on_maximize: Cell<bool>,
    /// Whether the procedure was taken away already, when the window was destroyed.
    is_removed: Cell<bool>,
}

impl FrameProcedure {
    /// Puts the procedure in front of the one of a window of `winit`.
    ///
    /// # Errors
    ///
    /// `PlatformError::Unsupported` when the window is not one of Win32; `PlatformError::System` when the system
    /// refused.
    pub(crate) fn install(
        window: &LibraryWindow,
        inbox: Arc<Mailbox<InputEvent>>,
    ) -> Result<FrameProcedure, PlatformError> {
        let handle = handle_of(window).ok_or(PlatformError::Unsupported {
            what: "draw the title bar of a window that is not of Win32",
        })?;
        let hook = Box::new(FrameHook {
            window: handle,
            parts: RefCell::new(None),
            inbox,
            is_over_maximize: Cell::new(false),
            is_pressed_on_maximize: Cell::new(false),
            is_removed: Cell::new(false),
        });
        let data = (&raw const *hook).expose_provenance();
        // SAFETY: the window is a window of this thread, alive; `procedure` has the signature the system calls; `data`
        // is the address of the boxed hook, which the procedure reads and which lives until `FrameProcedure` is
        // dropped, which takes the procedure away first.
        let result = unsafe { comctl32::SetWindowSubclass(hook.window, procedure, SUBCLASS, data) };
        if result == 0 {
            return Err(PlatformError::system(
                "SetWindowSubclass",
                &std::io::Error::last_os_error(),
            ));
        }
        Ok(FrameProcedure { hook })
    }

    /// Says what each place of the window is, from now on.
    pub(crate) fn set_parts(&self, parts: FrameParts) {
        if let Ok(mut kept) = self.hook.parts.try_borrow_mut() {
            *kept = Some(parts);
        }
    }
}

impl Drop for FrameProcedure {
    fn drop(&mut self) {
        self.hook.remove();
    }
}

/// The procedure, which the system calls with every message of the window before the procedures behind it.
unsafe extern "system" fn procedure(
    window: HWND,
    message: u32,
    first: WPARAM,
    second: LPARAM,
    _subclass: usize,
    data: usize,
) -> LRESULT {
    let hook: *const FrameHook = std::ptr::with_exposed_provenance(data);
    // SAFETY: `data` is the address of the hook that `FrameProcedure::install` boxed and gave with this procedure. The
    // procedure is taken away before the box is dropped, and when the window is destroyed, so the hook is alive
    // whenever the system calls it; only shared references to it are made.
    let hook = unsafe { &*hook };
    if let Some(answer) = hook.answer(message, first, second) {
        return answer;
    }
    // SAFETY: the message is handed on unchanged to the procedures behind this one, as the system asks of it.
    unsafe { comctl32::DefSubclassProc(window, message, first, second) }
}

impl FrameHook {
    /// What the procedure answers to a message; none hands it on to the procedures behind it.
    fn answer(&self, message: u32, first: WPARAM, second: LPARAM) -> Option<LRESULT> {
        let is_on_maximize = first == to_parameter(user32::HTMAXBUTTON);
        match message {
            user32::WM_NCHITTEST => self
                .part_at(second)
                .map(|part| LRESULT::try_from(hit_code(part)).unwrap_or_default()),
            user32::WM_NCMOUSEMOVE => {
                self.pointer_moved(is_on_maximize, second);
                None
            }
            user32::WM_NCMOUSELEAVE => {
                self.pointer_left();
                None
            }
            user32::WM_NCLBUTTONDOWN | user32::WM_NCLBUTTONDBLCLK if is_on_maximize => {
                self.is_pressed_on_maximize.set(true);
                self.post_button(second, InputEvent::PointerPressed);
                Some(0)
            }
            user32::WM_NCLBUTTONUP if self.is_pressed_on_maximize.replace(false) => {
                self.post_button(second, InputEvent::PointerReleased);
                is_on_maximize.then_some(0)
            }
            user32::WM_SYSCHAR if opens_window_menu(first) => {
                // SAFETY: the message is the system's, for this window, which the system's own procedure answers
                // with the menu of the window.
                Some(unsafe { user32::DefWindowProcW(self.window, message, first, second) })
            }
            user32::WM_ENTERSIZEMOVE => {
                self.post(InputEvent::MoveStarted);
                None
            }
            user32::WM_NCDESTROY => {
                self.remove();
                None
            }
            _ => None,
        }
    }

    /// What the place of the window at a point of the screen is, once the program said.
    fn part_at(&self, second: LPARAM) -> Option<FramePart> {
        let parts = self.parts.try_borrow().ok()?;
        let parts = parts.as_ref()?;
        let (place, size) = self.place(second)?;
        Some(parts(place, size))
    }

    /// The pointer moved over a place that is not the inside of the window: over the maximize button it is handed to
    /// the program, which draws the button under it; off it, the program is told it left.
    fn pointer_moved(&self, is_on_maximize: bool, second: LPARAM) {
        if is_on_maximize {
            self.is_over_maximize.set(true);
            self.track_leaving();
            if let Some((position, _)) = self.place(second) {
                self.post(InputEvent::PointerMoved(PointerMove {
                    position,
                    modifiers: Modifiers::NONE,
                }));
            }
        } else {
            self.pointer_left();
        }
    }

    /// The pointer left the maximize button, if it was over it.
    fn pointer_left(&self) {
        if self.is_over_maximize.replace(false) {
            self.post(InputEvent::PointerLeft);
        }
    }

    /// Asks the system to say when the pointer leaves the places of the window that are not its inside.
    fn track_leaving(&self) {
        let mut asked = user32::TRACKMOUSEEVENT {
            cbSize: u32::try_from(size_of::<user32::TRACKMOUSEEVENT>()).unwrap_or_default(),
            dwFlags: user32::TME_LEAVE | user32::TME_NONCLIENT,
            hwndTrack: self.window,
            dwHoverTime: 0,
        };
        // SAFETY: `asked` is a TRACKMOUSEEVENT with its size filled in, alive for the call, about this window.
        let _ = unsafe { user32::TrackMouseEvent(&raw mut asked) };
    }

    /// Hands the program a press or a release of the main button on the maximize button.
    fn post_button(&self, second: LPARAM, event: fn(PointerButtonEvent) -> InputEvent) {
        if let Some((position, _)) = self.place(second) {
            self.post(event(PointerButtonEvent {
                button: PointerButton::Primary,
                position,
                modifiers: Modifiers::NONE,
                click_count: 1,
            }));
        }
    }

    /// Hands input to the program; input for a window closing is not wanted.
    fn post(&self, event: InputEvent) {
        let _ = self.inbox.post(event);
    }

    /// A point of the screen a message gives as a place in the window, and the size of the window, in logical pixels.
    fn place(&self, second: LPARAM) -> Option<(Point, LogicalSize)> {
        let (x, y) = screen_point(second);
        let mut point = user32::POINT { x, y };
        // SAFETY: the window is alive while its procedure is called; `point` is alive for the call, which changes it.
        if unsafe { user32::ScreenToClient(self.window, &raw mut point) } == 0 {
            return None;
        }
        let mut inside = user32::RECT::default();
        // SAFETY: as above, for `inside`, which the call writes.
        if unsafe { user32::GetClientRect(self.window, &raw mut inside) } == 0 {
            return None;
        }
        // SAFETY: the window is alive while its procedure is called.
        let dpi = unsafe { user32::GetDpiForWindow(self.window) };
        let scale = Scale::from_dpi(if dpi == 0 { user32::USER_DEFAULT_SCREEN_DPI } else { dpi });
        let size = PhysicalSize {
            width: u32::try_from(inside.right - inside.left).unwrap_or_default(),
            height: u32::try_from(inside.bottom - inside.top).unwrap_or_default(),
        };
        let place = translate::logical_point(f64::from(point.x), f64::from(point.y), scale);
        Some((place, scale.to_logical(size)))
    }

    /// Takes the procedure away from the window, once.
    fn remove(&self) {
        if self.is_removed.replace(true) {
            return;
        }
        // SAFETY: the window is alive (this runs before `winit` destroys it, or while it is being destroyed), on its
        // thread, and the procedure and its number are those it was put in front of it with.
        let _ = unsafe { comctl32::RemoveWindowSubclass(self.window, procedure, SUBCLASS) };
    }
}

/// Whether a window is as it is restored to: neither maximized nor minimized. The styles say it at once, before the
/// window has its new size, while `winit` knows of a maximized window only once it is told its size.
pub(crate) fn is_restored(window: &LibraryWindow) -> bool {
    let Some(handle) = handle_of(window) else {
        return !window.is_maximized() && window.is_minimized() != Some(true);
    };
    // SAFETY: the window is alive, since `window` is borrowed; both calls only read its styles.
    unsafe { user32::IsZoomed(handle) == 0 && user32::IsIconic(handle) == 0 }
}

/// The handle of a window of Win32.
fn handle_of(window: &LibraryWindow) -> Option<HWND> {
    let handle = window.window_handle().ok()?;
    let RawWindowHandle::Win32(handle) = handle.as_raw() else {
        return None;
    };
    Some(std::ptr::with_exposed_provenance_mut(handle.hwnd.get().cast_unsigned()))
}

/// What the system is told a place of the window is. The two buttons the program takes the pointer on itself are the
/// inside of the window; only the maximize button is said to be one, for Snap Layouts.
fn hit_code(part: FramePart) -> u32 {
    match part {
        FramePart::Client | FramePart::Minimize | FramePart::Close => user32::HTCLIENT,
        FramePart::Caption => user32::HTCAPTION,
        FramePart::Maximize => user32::HTMAXBUTTON,
        FramePart::Edge(edge) => match edge {
            ResizeEdge::Left => user32::HTLEFT,
            ResizeEdge::Right => user32::HTRIGHT,
            ResizeEdge::Top => user32::HTTOP,
            ResizeEdge::Bottom => user32::HTBOTTOM,
            ResizeEdge::TopLeft => user32::HTTOPLEFT,
            ResizeEdge::TopRight => user32::HTTOPRIGHT,
            ResizeEdge::BottomLeft => user32::HTBOTTOMLEFT,
            ResizeEdge::BottomRight => user32::HTBOTTOMRIGHT,
        },
    }
}

/// The point of the screen a message of the pointer gives in its second parameter: two signed halves of 16 bits.
fn screen_point(second: LPARAM) -> (i32, i32) {
    let half = |shift: u32| {
        i32::from(
            u16::try_from((second >> shift) & 0xFFFF)
                .unwrap_or_default()
                .cast_signed(),
        )
    };
    (half(0), half(16))
}

/// Whether a character typed with Alt held opens the system's menu of the window: the space, as `Alt+Space`.
fn opens_window_menu(character: WPARAM) -> bool {
    character == usize::from(b' ')
}

/// A constant of the system as the first parameter of a message.
fn to_parameter(value: u32) -> WPARAM {
    usize::try_from(value).unwrap_or(usize::MAX)
}

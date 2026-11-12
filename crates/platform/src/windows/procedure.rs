//! The window procedure of Gitardo's windows: what each message of the system becomes, for the window and for the
//! program's handler.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;

use super::handle::{WindowHandle, outer_rect};
use super::keys::Utf16Joiner;
use super::mailbox::Mailbox;
use super::pointer::PointerState;
use super::present::RetainedFrame;
use super::types::{HWND, LPARAM, LRESULT, RECT, WPARAM};
use super::user32::{self, CREATESTRUCTW, MINMAXINFO};
use super::{cursor, keyboard, low_word, pointer, present};
use crate::dispatch::WindowSink;
use crate::event::{Cursor, WindowId};
use crate::geometry::{LogicalSize, PhysicalSize, Scale};
use crate::input::InputEvent;

/// The message that tells a window that synthetic input is waiting in its mailbox.
pub(crate) const WM_SYNTHETIC_INPUT: u32 = user32::WM_APP + 2;

/// The longest name of a setting read from `WM_SETTINGCHANGE`; the names looked for are shorter.
const LONGEST_SETTING_NAME: usize = 64;

/// What a window keeps for its window procedure. The `Window` owns it and destroys the window before dropping it.
pub(crate) struct WindowState {
    /// The window's name in its loop.
    pub(crate) id: WindowId,
    /// The loop the window belongs to.
    pub(crate) sink: Rc<dyn WindowSink>,
    /// The dots per inch of the screen the window is on.
    pub(crate) dpi: Cell<u32>,
    /// The size of the inside of the window, as last reported.
    pub(crate) size: Cell<PhysicalSize>,
    /// The smallest size of the inside of the window.
    pub(crate) minimum_size: LogicalSize,
    /// The shape of the pointer over the inside of the window.
    pub(crate) cursor: Cell<Cursor>,
    /// The last frame shown, to paint again what the system asks for.
    pub(crate) frame: RefCell<RetainedFrame>,
    /// The buttons held and the clicks counted.
    pub(crate) pointer: RefCell<PointerState>,
    /// The first half of a character typed in two halves.
    pub(crate) keyboard: RefCell<Utf16Joiner>,
    /// The synthetic input waiting for the window.
    pub(crate) synthetic: Arc<Mailbox<InputEvent>>,
}

impl WindowState {
    /// Hands an event of this window to the loop.
    pub(crate) fn push(&self, event: InputEvent) {
        self.sink.push_input(self.id, event);
    }

    /// The scale of the screen the window is on.
    pub(crate) fn scale(&self) -> Scale {
        Scale::from_dpi(self.dpi.get())
    }
}

/// A message for a window, with its parameters.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Message {
    /// Which message.
    pub(crate) id: u32,
    /// Its first parameter.
    pub(crate) wparam: WPARAM,
    /// Its second parameter.
    pub(crate) lparam: LPARAM,
}

/// Keeps the pointer given to `CreateWindowExW` with the window, when `WM_NCCREATE` brings it.
pub(crate) fn keep_state_on_create(window: WindowHandle, message: u32, lparam: LPARAM) {
    if message != user32::WM_NCCREATE {
        return;
    }
    let creation = std::ptr::with_exposed_provenance::<CREATESTRUCTW>(lparam.cast_unsigned());
    // SAFETY: with WM_NCCREATE, the system passes a pointer to the CREATESTRUCTW of the call that creates the window,
    // valid while the message is handled.
    if let Some(creation) = unsafe { creation.as_ref() } {
        window.set_user_data(creation.lpCreateParams.cast_const());
    }
}

/// The window procedure of Gitardo's windows.
pub(crate) extern "system" fn window_procedure(window: HWND, id: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let handle = WindowHandle::from_raw(window);
    keep_state_on_create(handle, id, lparam);
    let state = handle.user_data().cast::<WindowState>();
    // SAFETY: the pointer is null, or the `WindowState` that `Window::open` passed to CreateWindowExW. The `Window`
    // owns it and destroys this window before dropping it, and the pointer is cleared when the window is destroyed.
    let Some(state) = (unsafe { state.as_ref() }) else {
        return handle.default_procedure(id, wparam, lparam);
    };
    if id == user32::WM_NCDESTROY {
        handle.set_user_data(std::ptr::null());
        return handle.default_procedure(id, wparam, lparam);
    }
    let message = Message { id, wparam, lparam };
    window_message(state, handle, message)
        .or_else(|| pointer::pointer_message(state, handle, message))
        .or_else(|| keyboard::keyboard_message(state, handle, message))
        .unwrap_or_else(|| handle.default_procedure(id, wparam, lparam))
}

/// What a message about the window itself does; `None` for the messages the system handles alone.
fn window_message(state: &WindowState, window: WindowHandle, message: Message) -> Option<LRESULT> {
    match message.id {
        user32::WM_SIZE => resized(state, message),
        user32::WM_DPICHANGED => scale_changed(state, window, message),
        user32::WM_SETFOCUS => state.push(InputEvent::FocusGained),
        user32::WM_KILLFOCUS => {
            state.keyboard.borrow_mut().clear();
            state.push(InputEvent::FocusLost);
        }
        user32::WM_CLOSE => state.push(InputEvent::CloseRequested),
        user32::WM_PAINT => present::paint(state, window),
        WM_SYNTHETIC_INPUT => state
            .synthetic
            .take_all()
            .into_iter()
            .for_each(|event| state.push(event)),
        user32::WM_GETMINMAXINFO => return minimum_size(state, message),
        user32::WM_ERASEBKGND => return Some(1),
        user32::WM_SETCURSOR => return cursor::cursor_message(state, message),
        user32::WM_SETTINGCHANGE | user32::WM_ENTERSIZEMOVE | user32::WM_EXITSIZEMOVE => {
            system_changed(state, message);
            return None;
        }
        // Every other message is the system's to handle.
        _ => return None,
    }
    Some(0)
}

/// What the program learns from a message the system also handles: the theme changed, or the system entered or left
/// its own loop of moving or resizing the window.
fn system_changed(state: &WindowState, message: Message) {
    match message.id {
        user32::WM_SETTINGCHANGE if is_theme_setting(message) => state.sink.push_theme_changed(),
        user32::WM_ENTERSIZEMOVE => state.sink.enter_modal(),
        user32::WM_EXITSIZEMOVE => state.sink.leave_modal(),
        // Other settings are of no interest to Gitardo.
        _ => {}
    }
}

/// `WM_SIZE`: the inside of the window has a new size, which needs a new frame. A minimized window keeps its size.
fn resized(state: &WindowState, message: Message) {
    if message.wparam == user32::SIZE_MINIMIZED {
        return;
    }
    let bits = message.lparam.cast_unsigned();
    let size = PhysicalSize {
        width: u32::from(low_word(bits)),
        height: u32::from(low_word(bits >> 16)),
    };
    if size != state.size.get() {
        state.size.set(size);
        state.sink.request_redraw(state.id);
        state.push(InputEvent::Resized(size));
    }
}

/// `WM_DPICHANGED`: the window is on a screen of another scale, and takes the rectangle the system suggests for it.
fn scale_changed(state: &WindowState, window: WindowHandle, message: Message) {
    let dpi = u32::from(low_word(message.wparam));
    state.dpi.set(dpi);
    state.push(InputEvent::ScaleChanged(Scale::from_dpi(dpi)));
    let suggested = std::ptr::with_exposed_provenance::<RECT>(message.lparam.cast_unsigned());
    // SAFETY: with WM_DPICHANGED, the system passes a pointer to the suggested RECT, valid while the message is
    // handled.
    if let Some(rect) = unsafe { suggested.as_ref() } {
        // When the system refuses, the window keeps its size in physical pixels, which is still usable.
        let _ = window.place(*rect);
    }
}

/// `WM_GETMINMAXINFO`: the smallest size the person can resize the window to.
fn minimum_size(state: &WindowState, message: Message) -> Option<LRESULT> {
    let inside = state.scale().to_physical(state.minimum_size);
    let outer = outer_rect(inside, state.dpi.get()).ok()?;
    let limits = std::ptr::with_exposed_provenance_mut::<MINMAXINFO>(message.lparam.cast_unsigned());
    // SAFETY: with WM_GETMINMAXINFO, the system passes a pointer to a MINMAXINFO the window may change, valid while
    // the message is handled; nothing else refers to it meanwhile.
    let limits = unsafe { limits.as_mut() }?;
    limits.ptMinTrackSize.x = outer.right - outer.left;
    limits.ptMinTrackSize.y = outer.bottom - outer.top;
    Some(0)
}

/// Whether a `WM_SETTINGCHANGE` says the theme changed: between dark and light, or high contrast.
fn is_theme_setting(message: Message) -> bool {
    message.wparam == user32::SPI_SETHIGHCONTRAST
        || setting_name(message.lparam).as_deref() == Some("ImmersiveColorSet")
}

/// The name of the setting a `WM_SETTINGCHANGE` is about, when it names one.
fn setting_name(lparam: LPARAM) -> Option<String> {
    let start = std::ptr::with_exposed_provenance::<u16>(lparam.cast_unsigned());
    if start.is_null() {
        return None;
    }
    let mut name = Vec::new();
    while name.len() < LONGEST_SETTING_NAME {
        // SAFETY: with WM_SETTINGCHANGE the documentation says the parameter is null or a string ended by 0. Units
        // are read one at a time up to that 0, never past it.
        let unit = unsafe { start.add(name.len()).read_unaligned() };
        if unit == 0 {
            return Some(String::from_utf16_lossy(&name));
        }
        name.push(unit);
    }
    None
}

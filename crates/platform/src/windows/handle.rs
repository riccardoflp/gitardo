//! Windows of the system by their handle: creating and destroying them, and the calls that ask or change their
//! place, size, text and look.
//!
//! Every call here takes a window handle, which the system checks: a handle that names no window makes the call fail
//! with an error, never touch memory. That is what makes these calls safe to wrap with a plain handle.

use core::ffi::c_void;
use std::marker::PhantomData;

use super::types::{BOOL, FALSE, HWND, LPARAM, LRESULT, RECT, WPARAM};
use super::user32::{self, WNDCLASSEXW, WNDPROC};
use super::{check, dwmapi, kernel32, last_error};
use crate::convert::{to_signed, to_unsigned};
use crate::error::PlatformError;
use crate::geometry::{PhysicalSize, ScreenPosition};
use crate::theme::Appearance;

/// The style of Gitardo's windows: a title bar, borders that resize, and the buttons to minimize, maximize and close.
pub(crate) const WINDOW_STYLE: u32 = user32::WS_OVERLAPPEDWINDOW;

/// A window of the system, by its handle. It owns nothing: `OwnedWindow` destroys the window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct WindowHandle(HWND);

impl WindowHandle {
    /// The window a message is for.
    pub(crate) fn from_raw(window: HWND) -> WindowHandle {
        WindowHandle(window)
    }

    /// The handle, for a call that takes it.
    pub(crate) fn raw(self) -> HWND {
        self.0
    }

    /// The pointer kept with the window, or null.
    pub(crate) fn user_data(self) -> *const c_void {
        // SAFETY: the handle is checked by the system; GWLP_USERDATA is a slot every window has.
        let value = unsafe { user32::GetWindowLongPtrW(self.0, user32::GWLP_USERDATA) };
        std::ptr::with_exposed_provenance(value.cast_unsigned())
    }

    /// Keeps a pointer with the window, for its window procedure.
    pub(crate) fn set_user_data(self, pointer: *const c_void) {
        let value = pointer.expose_provenance().cast_signed();
        // SAFETY: the handle is checked by the system; GWLP_USERDATA is a slot every window has. The previous value
        // returned is of no use.
        let _ = unsafe { user32::SetWindowLongPtrW(self.0, user32::GWLP_USERDATA, value) };
    }

    /// What the system does with a message the window does not handle itself.
    pub(crate) fn default_procedure(self, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        // SAFETY: called from the window procedure with the parameters the system gave it, unchanged.
        unsafe { user32::DefWindowProcW(self.0, message, wparam, lparam) }
    }

    /// The size of the inside of the window, in physical pixels.
    pub(crate) fn client_size(self) -> Result<PhysicalSize, PlatformError> {
        let mut rect = RECT::default();
        // SAFETY: `rect` is a RECT the call writes into, alive for the call.
        check("GetClientRect", unsafe { user32::GetClientRect(self.0, &raw mut rect) })?;
        Ok(PhysicalSize {
            width: to_unsigned(rect.right - rect.left),
            height: to_unsigned(rect.bottom - rect.top),
        })
    }

    /// Where the top left corner of the window's frame is on the screen.
    pub(crate) fn position(self) -> Result<ScreenPosition, PlatformError> {
        let mut rect = RECT::default();
        // SAFETY: `rect` is a RECT the call writes into, alive for the call.
        check("GetWindowRect", unsafe { user32::GetWindowRect(self.0, &raw mut rect) })?;
        Ok(ScreenPosition {
            x: rect.left,
            y: rect.top,
        })
    }

    /// The dots per inch of the screen the window is on.
    pub(crate) fn dpi(self) -> u32 {
        // SAFETY: the handle is checked by the system, which returns 0 for one that is not a window.
        unsafe { user32::GetDpiForWindow(self.0) }
    }

    /// Changes the text of the title bar.
    pub(crate) fn set_text(self, text: &str) -> Result<(), PlatformError> {
        let text = super::wide(text);
        // SAFETY: `text` is a string ended by 0, alive for the call.
        check("SetWindowTextW", unsafe {
            user32::SetWindowTextW(self.0, text.as_ptr())
        })
    }

    /// Moves and resizes the window's frame to a rectangle of the screen.
    pub(crate) fn place(self, rect: RECT) -> Result<(), PlatformError> {
        let flags = user32::SWP_NOZORDER | user32::SWP_NOACTIVATE;
        let (width, height) = (rect.right - rect.left, rect.bottom - rect.top);
        // SAFETY: only integers are passed.
        let result = unsafe { user32::SetWindowPos(self.0, 0, rect.left, rect.top, width, height, flags) };
        check("SetWindowPos", result)
    }

    /// Resizes the window so that its inside has this size, keeping where it is.
    pub(crate) fn resize_inside(self, size: PhysicalSize) -> Result<(), PlatformError> {
        let outer = outer_rect(size, self.dpi())?;
        let flags = user32::SWP_NOMOVE | user32::SWP_NOZORDER | user32::SWP_NOACTIVATE;
        let (width, height) = (outer.right - outer.left, outer.bottom - outer.top);
        // SAFETY: only integers are passed.
        let result = unsafe { user32::SetWindowPos(self.0, 0, 0, 0, width, height, flags) };
        check("SetWindowPos", result)
    }

    /// Moves the window's frame, keeping its size.
    pub(crate) fn move_to(self, position: ScreenPosition) -> Result<(), PlatformError> {
        let flags = user32::SWP_NOSIZE | user32::SWP_NOZORDER | user32::SWP_NOACTIVATE;
        // SAFETY: only integers are passed.
        let result = unsafe { user32::SetWindowPos(self.0, 0, position.x, position.y, 0, 0, flags) };
        check("SetWindowPos", result)
    }

    /// Shows the window.
    pub(crate) fn show(self) {
        // SAFETY: only integers are passed. The result says whether the window was visible before, not a failure.
        let _ = unsafe { user32::ShowWindow(self.0, user32::SW_SHOW) };
    }

    /// Hides the window.
    pub(crate) fn hide(self) {
        // SAFETY: only integers are passed. The result says whether the window was visible before, not a failure.
        let _ = unsafe { user32::ShowWindow(self.0, user32::SW_HIDE) };
    }

    /// Makes the title bar and the borders dark or light.
    pub(crate) fn set_title_bar(self, appearance: Appearance) -> Result<(), PlatformError> {
        let is_dark: BOOL = match appearance {
            Appearance::Light => FALSE,
            Appearance::Dark => 1,
        };
        let size = u32::try_from(size_of::<BOOL>()).unwrap_or_default();
        let value = (&raw const is_dark).cast::<c_void>();
        let attribute = dwmapi::DWMWA_USE_IMMERSIVE_DARK_MODE;
        // SAFETY: `value` points to a BOOL alive for the call, and `size` is its size, as the attribute asks.
        let result = unsafe { dwmapi::DwmSetWindowAttribute(self.0, attribute, value, size) };
        if result < 0 {
            return Err(PlatformError::System {
                call: "DwmSetWindowAttribute",
                code: result.cast_unsigned(),
            });
        }
        Ok(())
    }

    /// Posts a message to the window's queue, for its window procedure.
    pub(crate) fn post(self, message: u32) -> Result<(), PlatformError> {
        // SAFETY: only integers are passed; posting to a window of another thread is allowed.
        check("PostMessageW", unsafe { user32::PostMessageW(self.0, message, 0, 0) })
    }
}

/// The rectangle of a window's frame whose inside has this size, on a screen of so many dots per inch.
pub(crate) fn outer_rect(size: PhysicalSize, dpi: u32) -> Result<RECT, PlatformError> {
    let mut rect = RECT {
        left: 0,
        top: 0,
        right: to_signed(size.width),
        bottom: to_signed(size.height),
    };
    // SAFETY: `rect` is a RECT the call reads and writes, alive for the call.
    let result = unsafe { user32::AdjustWindowRectExForDpi(&raw mut rect, WINDOW_STYLE, FALSE, 0, dpi) };
    check("AdjustWindowRectExForDpi", result)?;
    Ok(rect)
}

/// Registers a class of windows with its window procedure, once for the whole program; registering it again is not
/// an error.
pub(crate) fn register_class(name: &[u16], procedure: WNDPROC) -> Result<(), PlatformError> {
    let class = WNDCLASSEXW {
        cbSize: u32::try_from(size_of::<WNDCLASSEXW>()).unwrap_or_default(),
        style: 0,
        lpfnWndProc: Some(procedure),
        cbClsExtra: 0,
        cbWndExtra: 0,
        hInstance: module(),
        hIcon: 0,
        hCursor: 0,
        hbrBackground: 0,
        lpszMenuName: std::ptr::null(),
        lpszClassName: name.as_ptr(),
        hIconSm: 0,
    };
    // SAFETY: `class` is filled in as the documentation asks, and `name` is a string ended by 0 alive for the call;
    // the system copies the name.
    if unsafe { user32::RegisterClassExW(&raw const class) } == 0 {
        let error = last_error("RegisterClassExW");
        if error
            != (PlatformError::System {
                call: "RegisterClassExW",
                code: kernel32::ERROR_CLASS_ALREADY_EXISTS,
            })
        {
            return Err(error);
        }
    }
    Ok(())
}

/// The handle of the program's own module.
fn module() -> isize {
    // SAFETY: a null name asks for the module of the program, which needs no release.
    unsafe { kernel32::GetModuleHandleW(std::ptr::null()) }
}

/// What a new window is.
pub(crate) struct WindowRequest<'a> {
    /// The class, registered before, as a string ended by 0.
    pub(crate) class: &'a [u16],
    /// The title, as a string ended by 0.
    pub(crate) title: &'a [u16],
    /// Its style.
    pub(crate) style: u32,
    /// Its parent: `HWND_MESSAGE` for a window that only receives messages, 0 for none.
    pub(crate) parent: HWND,
    /// The pointer its window procedure gets with `WM_NCCREATE`.
    pub(crate) state: *const c_void,
}

/// A window this program created, destroyed when dropped. It belongs to the thread that created it.
pub(crate) struct OwnedWindow {
    handle: WindowHandle,
    thread_bound: PhantomData<*const ()>,
}

impl OwnedWindow {
    /// Creates a window, hidden, at the place and size the system picks.
    pub(crate) fn create(request: &WindowRequest<'_>) -> Result<OwnedWindow, PlatformError> {
        let default = user32::CW_USEDEFAULT;
        // SAFETY: the class and the title are strings ended by 0, alive for the call; `state` is passed through to
        // the window procedure, which knows what it points to.
        let window = unsafe {
            user32::CreateWindowExW(
                0,
                request.class.as_ptr(),
                request.title.as_ptr(),
                request.style,
                default,
                default,
                default,
                default,
                request.parent,
                0,
                module(),
                request.state,
            )
        };
        if window == 0 {
            return Err(last_error("CreateWindowExW"));
        }
        Ok(OwnedWindow {
            handle: WindowHandle(window),
            thread_bound: PhantomData,
        })
    }

    /// Its handle.
    pub(crate) fn handle(&self) -> WindowHandle {
        self.handle
    }
}

impl Drop for OwnedWindow {
    fn drop(&mut self) {
        // SAFETY: the window was created by this thread, which is the one dropping it since `OwnedWindow` is not
        // `Send`. A failure leaves a window the system destroys with the thread; nothing better can be done here.
        let _ = unsafe { user32::DestroyWindow(self.handle.0) };
    }
}

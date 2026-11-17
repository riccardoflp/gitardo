//! The functions, types and constants of `user32.dll` that Gitardo Desktop asks for itself, with the names the Windows
//! documentation gives them: the settings of the system that the libraries of the window do not read, and what the
//! procedure of a window that draws its own title bar answers and asks.

#![allow(
    clippy::upper_case_acronyms,
    reason = "the names are those of the Windows documentation"
)]
#![allow(
    non_snake_case,
    reason = "the names of the fields are those of the Windows documentation"
)]

use core::ffi::c_void;

/// A boolean of the system: 0 is false, anything else is true.
pub(crate) type BOOL = i32;

/// A window of the system.
pub(crate) type HWND = *mut c_void;
/// The first parameter of a message.
pub(crate) type WPARAM = usize;
/// The second parameter of a message.
pub(crate) type LPARAM = isize;
/// What a window procedure answers.
pub(crate) type LRESULT = isize;

/// A point, in pixels of the screen or of the inside of a window.
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub(crate) struct POINT {
    pub(crate) x: i32,
    pub(crate) y: i32,
}

/// A rectangle, in pixels.
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub(crate) struct RECT {
    pub(crate) left: i32,
    pub(crate) top: i32,
    pub(crate) right: i32,
    pub(crate) bottom: i32,
}

/// What `TrackMouseEvent` is asked to say about the pointer.
#[repr(C)]
pub(crate) struct TRACKMOUSEEVENT {
    pub(crate) cbSize: u32,
    pub(crate) dwFlags: u32,
    pub(crate) hwndTrack: HWND,
    pub(crate) dwHoverTime: u32,
}

/// The high contrast setting.
#[repr(C)]
pub(crate) struct HIGHCONTRASTW {
    pub(crate) cbSize: u32,
    pub(crate) dwFlags: u32,
    pub(crate) lpszDefaultScheme: *mut u16,
}

#[link(name = "user32")]
unsafe extern "system" {
    pub(crate) fn DefWindowProcW(hWnd: HWND, Msg: u32, wParam: WPARAM, lParam: LPARAM) -> LRESULT;
    pub(crate) fn GetClientRect(hWnd: HWND, lpRect: *mut RECT) -> BOOL;
    pub(crate) fn GetDoubleClickTime() -> u32;
    pub(crate) fn GetDpiForWindow(hwnd: HWND) -> u32;
    pub(crate) fn GetSystemMetricsForDpi(index: i32, dpi: u32) -> i32;
    pub(crate) fn IsIconic(hWnd: HWND) -> BOOL;
    pub(crate) fn IsZoomed(hWnd: HWND) -> BOOL;
    pub(crate) fn ScreenToClient(hWnd: HWND, lpPoint: *mut POINT) -> BOOL;
    pub(crate) fn SystemParametersInfoW(action: u32, parameter: u32, value: *mut c_void, update: u32) -> BOOL;
    pub(crate) fn TrackMouseEvent(lpEventTrack: *mut TRACKMOUSEEVENT) -> BOOL;
}

pub(crate) const SM_CXDOUBLECLK: i32 = 36;
pub(crate) const SM_CYDOUBLECLK: i32 = 37;
pub(crate) const SPI_GETWHEELSCROLLLINES: u32 = 0x0068;
pub(crate) const SPI_GETWHEELSCROLLCHARS: u32 = 0x006C;
pub(crate) const SPI_GETHIGHCONTRAST: u32 = 0x0042;
pub(crate) const HCF_HIGHCONTRASTON: u32 = 0x0000_0001;
/// What the wheel settings say when a notch should scroll a whole page.
pub(crate) const WHEEL_PAGESCROLL: u32 = u32::MAX;

/// The window is being destroyed: the last message it gets.
pub(crate) const WM_NCDESTROY: u32 = 0x0082;
/// What is the place of the window under the pointer?
pub(crate) const WM_NCHITTEST: u32 = 0x0084;
/// The pointer moved over a place of the window that is not its inside.
pub(crate) const WM_NCMOUSEMOVE: u32 = 0x00A0;
/// The main button went down over a place of the window that is not its inside.
pub(crate) const WM_NCLBUTTONDOWN: u32 = 0x00A1;
/// The main button went up over a place of the window that is not its inside.
pub(crate) const WM_NCLBUTTONUP: u32 = 0x00A2;
/// The main button went down a second time, soon, over a place of the window that is not its inside.
pub(crate) const WM_NCLBUTTONDBLCLK: u32 = 0x00A3;
/// A character typed with Alt held.
pub(crate) const WM_SYSCHAR: u32 = 0x0106;
/// The window starts to be moved or resized, by the pointer or by the keys of its menu.
pub(crate) const WM_ENTERSIZEMOVE: u32 = 0x0231;
/// The pointer left the places of the window that are not its inside.
pub(crate) const WM_NCMOUSELEAVE: u32 = 0x02A2;

/// The inside of the window.
pub(crate) const HTCLIENT: u32 = 1;
/// The title bar.
pub(crate) const HTCAPTION: u32 = 2;
/// The maximize button.
pub(crate) const HTMAXBUTTON: u32 = 9;
/// The left edge.
pub(crate) const HTLEFT: u32 = 10;
/// The right edge.
pub(crate) const HTRIGHT: u32 = 11;
/// The top edge.
pub(crate) const HTTOP: u32 = 12;
/// The top left corner.
pub(crate) const HTTOPLEFT: u32 = 13;
/// The top right corner.
pub(crate) const HTTOPRIGHT: u32 = 14;
/// The bottom edge.
pub(crate) const HTBOTTOM: u32 = 15;
/// The bottom left corner.
pub(crate) const HTBOTTOMLEFT: u32 = 16;
/// The bottom right corner.
pub(crate) const HTBOTTOMRIGHT: u32 = 17;

/// Says when the pointer leaves.
pub(crate) const TME_LEAVE: u32 = 0x0000_0002;
/// About the places of the window that are not its inside.
pub(crate) const TME_NONCLIENT: u32 = 0x0000_0010;
/// The scale of a window at 100 %.
pub(crate) const USER_DEFAULT_SCREEN_DPI: u32 = 96;

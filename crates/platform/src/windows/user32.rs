//! The functions, types and constants of `user32.dll` that Gitardo Desktop uses: windows, messages, input, the
//! cursor and the clipboard, with the names the Windows documentation gives them.

#![allow(
    clippy::upper_case_acronyms,
    reason = "the names are those of the Windows documentation"
)]
#![allow(
    non_snake_case,
    reason = "the names of the fields are those of the Windows documentation"
)]

use core::ffi::c_void;

use super::types::{
    BOOL, HANDLE, HBRUSH, HCURSOR, HDC, HICON, HINSTANCE, HMENU, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM,
};

/// The function that receives the messages of a window.
pub(crate) type WNDPROC = unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM) -> LRESULT;

/// What a class of windows is.
#[repr(C)]
pub(crate) struct WNDCLASSEXW {
    pub(crate) cbSize: u32,
    pub(crate) style: u32,
    pub(crate) lpfnWndProc: Option<WNDPROC>,
    pub(crate) cbClsExtra: i32,
    pub(crate) cbWndExtra: i32,
    pub(crate) hInstance: HINSTANCE,
    pub(crate) hIcon: HICON,
    pub(crate) hCursor: HCURSOR,
    pub(crate) hbrBackground: HBRUSH,
    pub(crate) lpszMenuName: *const u16,
    pub(crate) lpszClassName: *const u16,
    pub(crate) hIconSm: HICON,
}

/// A message from the queue of the thread.
#[repr(C)]
#[derive(Default)]
pub(crate) struct MSG {
    pub(crate) hwnd: HWND,
    pub(crate) message: u32,
    pub(crate) wParam: WPARAM,
    pub(crate) lParam: LPARAM,
    pub(crate) time: u32,
    pub(crate) pt: POINT,
    pub(crate) lPrivate: u32,
}

/// What `BeginPaint` says about the painting it starts.
#[repr(C)]
#[derive(Default)]
pub(crate) struct PAINTSTRUCT {
    pub(crate) hdc: HDC,
    pub(crate) fErase: BOOL,
    pub(crate) rcPaint: RECT,
    pub(crate) fRestore: BOOL,
    pub(crate) fIncUpdate: BOOL,
    pub(crate) rgbReserved: [u8; 32],
}

/// What `TrackMouseEvent` is asked to report.
#[repr(C)]
pub(crate) struct TRACKMOUSEEVENT {
    pub(crate) cbSize: u32,
    pub(crate) dwFlags: u32,
    pub(crate) hwndTrack: HWND,
    pub(crate) dwHoverTime: u32,
}

/// The sizes a window may take, which `WM_GETMINMAXINFO` lets the window change.
#[repr(C)]
pub(crate) struct MINMAXINFO {
    pub(crate) ptReserved: POINT,
    pub(crate) ptMaxSize: POINT,
    pub(crate) ptMaxPosition: POINT,
    pub(crate) ptMinTrackSize: POINT,
    pub(crate) ptMaxTrackSize: POINT,
}

/// What `CreateWindowExW` was given, as `WM_NCCREATE` passes it on.
#[repr(C)]
pub(crate) struct CREATESTRUCTW {
    pub(crate) lpCreateParams: *mut c_void,
    pub(crate) hInstance: HINSTANCE,
    pub(crate) hMenu: HMENU,
    pub(crate) hwndParent: HWND,
    pub(crate) cy: i32,
    pub(crate) cx: i32,
    pub(crate) y: i32,
    pub(crate) x: i32,
    pub(crate) style: i32,
    pub(crate) lpszName: *const u16,
    pub(crate) lpszClass: *const u16,
    pub(crate) dwExStyle: u32,
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
    pub(crate) fn RegisterClassExW(class: *const WNDCLASSEXW) -> u16;
    pub(crate) fn CreateWindowExW(
        extended_style: u32,
        class_name: *const u16,
        window_name: *const u16,
        style: u32,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        parent: HWND,
        menu: HMENU,
        instance: HINSTANCE,
        parameter: *const c_void,
    ) -> HWND;
    pub(crate) fn DestroyWindow(window: HWND) -> BOOL;
    pub(crate) fn DefWindowProcW(window: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT;
    pub(crate) fn GetWindowLongPtrW(window: HWND, index: i32) -> isize;
    pub(crate) fn SetWindowLongPtrW(window: HWND, index: i32, value: isize) -> isize;
    pub(crate) fn PeekMessageW(message: *mut MSG, window: HWND, first: u32, last: u32, remove: u32) -> BOOL;
    pub(crate) fn WaitMessage() -> BOOL;
    pub(crate) fn TranslateMessage(message: *const MSG) -> BOOL;
    pub(crate) fn DispatchMessageW(message: *const MSG) -> LRESULT;
    pub(crate) fn PostMessageW(window: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> BOOL;
    pub(crate) fn SetTimer(window: HWND, id: usize, milliseconds: u32, function: *const c_void) -> usize;
    pub(crate) fn KillTimer(window: HWND, id: usize) -> BOOL;
    pub(crate) fn SetWindowTextW(window: HWND, text: *const u16) -> BOOL;
    pub(crate) fn GetClientRect(window: HWND, rect: *mut RECT) -> BOOL;
    pub(crate) fn GetWindowRect(window: HWND, rect: *mut RECT) -> BOOL;
    pub(crate) fn SetWindowPos(window: HWND, after: HWND, x: i32, y: i32, width: i32, height: i32, flags: u32) -> BOOL;
    pub(crate) fn ShowWindow(window: HWND, command: i32) -> BOOL;
    pub(crate) fn GetDpiForWindow(window: HWND) -> u32;
    pub(crate) fn SetProcessDpiAwarenessContext(context: isize) -> BOOL;
    pub(crate) fn AdjustWindowRectExForDpi(rect: *mut RECT, style: u32, menu: BOOL, extended: u32, dpi: u32) -> BOOL;
    pub(crate) fn BeginPaint(window: HWND, paint: *mut PAINTSTRUCT) -> HDC;
    pub(crate) fn EndPaint(window: HWND, paint: *const PAINTSTRUCT) -> BOOL;
    pub(crate) fn InvalidateRect(window: HWND, rect: *const RECT, erase: BOOL) -> BOOL;
    pub(crate) fn UpdateWindow(window: HWND) -> BOOL;
    pub(crate) fn SetCapture(window: HWND) -> HWND;
    pub(crate) fn ReleaseCapture() -> BOOL;
    pub(crate) fn TrackMouseEvent(track: *mut TRACKMOUSEEVENT) -> BOOL;
    pub(crate) fn GetKeyState(virtual_key: i32) -> i16;
    pub(crate) fn MapVirtualKeyW(code: u32, map_type: u32) -> u32;
    pub(crate) fn GetMessageTime() -> i32;
    pub(crate) fn GetDoubleClickTime() -> u32;
    pub(crate) fn GetSystemMetricsForDpi(index: i32, dpi: u32) -> i32;
    pub(crate) fn SystemParametersInfoW(action: u32, parameter: u32, value: *mut c_void, update: u32) -> BOOL;
    pub(crate) fn ScreenToClient(window: HWND, point: *mut POINT) -> BOOL;
    pub(crate) fn LoadCursorW(instance: HINSTANCE, name: *const u16) -> HCURSOR;
    pub(crate) fn SetCursor(cursor: HCURSOR) -> HCURSOR;
    pub(crate) fn OpenClipboard(owner: HWND) -> BOOL;
    pub(crate) fn CloseClipboard() -> BOOL;
    pub(crate) fn EmptyClipboard() -> BOOL;
    pub(crate) fn GetClipboardData(format: u32) -> HANDLE;
    pub(crate) fn SetClipboardData(format: u32, data: HANDLE) -> HANDLE;
    pub(crate) fn IsClipboardFormatAvailable(format: u32) -> BOOL;
    pub(crate) fn CountClipboardFormats() -> i32;
}

// Messages.
pub(crate) const WM_SIZE: u32 = 0x0005;
pub(crate) const WM_SETFOCUS: u32 = 0x0007;
pub(crate) const WM_KILLFOCUS: u32 = 0x0008;
pub(crate) const WM_PAINT: u32 = 0x000F;
pub(crate) const WM_CLOSE: u32 = 0x0010;
pub(crate) const WM_QUIT: u32 = 0x0012;
pub(crate) const WM_ERASEBKGND: u32 = 0x0014;
pub(crate) const WM_SETTINGCHANGE: u32 = 0x001A;
pub(crate) const WM_SETCURSOR: u32 = 0x0020;
pub(crate) const WM_GETMINMAXINFO: u32 = 0x0024;
pub(crate) const WM_NCCREATE: u32 = 0x0081;
pub(crate) const WM_NCDESTROY: u32 = 0x0082;
pub(crate) const WM_KEYDOWN: u32 = 0x0100;
pub(crate) const WM_KEYUP: u32 = 0x0101;
pub(crate) const WM_CHAR: u32 = 0x0102;
pub(crate) const WM_SYSKEYDOWN: u32 = 0x0104;
pub(crate) const WM_SYSKEYUP: u32 = 0x0105;
pub(crate) const WM_SYSCHAR: u32 = 0x0106;
pub(crate) const WM_IME_COMPOSITION: u32 = 0x010F;
pub(crate) const WM_TIMER: u32 = 0x0113;
pub(crate) const WM_MOUSEMOVE: u32 = 0x0200;
pub(crate) const WM_LBUTTONDOWN: u32 = 0x0201;
pub(crate) const WM_LBUTTONUP: u32 = 0x0202;
pub(crate) const WM_RBUTTONDOWN: u32 = 0x0204;
pub(crate) const WM_RBUTTONUP: u32 = 0x0205;
pub(crate) const WM_MBUTTONDOWN: u32 = 0x0207;
pub(crate) const WM_MBUTTONUP: u32 = 0x0208;
pub(crate) const WM_MOUSEWHEEL: u32 = 0x020A;
pub(crate) const WM_XBUTTONDOWN: u32 = 0x020B;
pub(crate) const WM_XBUTTONUP: u32 = 0x020C;
pub(crate) const WM_MOUSEHWHEEL: u32 = 0x020E;
pub(crate) const WM_CAPTURECHANGED: u32 = 0x0215;
pub(crate) const WM_ENTERSIZEMOVE: u32 = 0x0231;
pub(crate) const WM_EXITSIZEMOVE: u32 = 0x0232;
pub(crate) const WM_MOUSELEAVE: u32 = 0x02A3;
pub(crate) const WM_DPICHANGED: u32 = 0x02E0;
pub(crate) const WM_APP: u32 = 0x8000;

// Window styles, positions and commands.
pub(crate) const WS_OVERLAPPEDWINDOW: u32 = 0x00CF_0000;
pub(crate) const CW_USEDEFAULT: i32 = i32::MIN;
pub(crate) const HWND_MESSAGE: HWND = -3;
pub(crate) const GWLP_USERDATA: i32 = -21;
pub(crate) const SWP_NOSIZE: u32 = 0x0001;
pub(crate) const SWP_NOMOVE: u32 = 0x0002;
pub(crate) const SWP_NOZORDER: u32 = 0x0004;
pub(crate) const SWP_NOACTIVATE: u32 = 0x0010;
pub(crate) const SW_HIDE: i32 = 0;
pub(crate) const SW_SHOW: i32 = 5;
pub(crate) const PM_REMOVE: u32 = 0x0001;
pub(crate) const DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2: isize = -4;
pub(crate) const SIZE_MINIMIZED: usize = 1;
pub(crate) const HTCLIENT: u16 = 1;

// Input.
pub(crate) const TME_LEAVE: u32 = 0x0000_0002;
pub(crate) const MAPVK_VK_TO_CHAR: u32 = 2;
pub(crate) const XBUTTON1: u16 = 0x0001;
pub(crate) const WHEEL_DELTA: i16 = 120;
pub(crate) const SM_CXDOUBLECLK: i32 = 36;
pub(crate) const SM_CYDOUBLECLK: i32 = 37;
pub(crate) const SPI_GETWHEELSCROLLLINES: u32 = 0x0068;
pub(crate) const SPI_GETWHEELSCROLLCHARS: u32 = 0x006C;
pub(crate) const SPI_GETHIGHCONTRAST: u32 = 0x0042;
pub(crate) const SPI_SETHIGHCONTRAST: usize = 0x0043;
pub(crate) const HCF_HIGHCONTRASTON: u32 = 0x0000_0001;

// Virtual keys.
pub(crate) const VK_BACK: u16 = 0x08;
pub(crate) const VK_TAB: u16 = 0x09;
pub(crate) const VK_RETURN: u16 = 0x0D;
pub(crate) const VK_SHIFT: u16 = 0x10;
pub(crate) const VK_CONTROL: u16 = 0x11;
pub(crate) const VK_MENU: u16 = 0x12;
pub(crate) const VK_PAUSE: u16 = 0x13;
pub(crate) const VK_CAPITAL: u16 = 0x14;
pub(crate) const VK_ESCAPE: u16 = 0x1B;
pub(crate) const VK_SPACE: u16 = 0x20;
pub(crate) const VK_PRIOR: u16 = 0x21;
pub(crate) const VK_NEXT: u16 = 0x22;
pub(crate) const VK_END: u16 = 0x23;
pub(crate) const VK_HOME: u16 = 0x24;
pub(crate) const VK_LEFT: u16 = 0x25;
pub(crate) const VK_UP: u16 = 0x26;
pub(crate) const VK_RIGHT: u16 = 0x27;
pub(crate) const VK_DOWN: u16 = 0x28;
pub(crate) const VK_SNAPSHOT: u16 = 0x2C;
pub(crate) const VK_INSERT: u16 = 0x2D;
pub(crate) const VK_DELETE: u16 = 0x2E;
pub(crate) const VK_LWIN: u16 = 0x5B;
pub(crate) const VK_RWIN: u16 = 0x5C;
pub(crate) const VK_APPS: u16 = 0x5D;
pub(crate) const VK_NUMPAD0: u16 = 0x60;
pub(crate) const VK_NUMPAD9: u16 = 0x69;
pub(crate) const VK_MULTIPLY: u16 = 0x6A;
pub(crate) const VK_ADD: u16 = 0x6B;
pub(crate) const VK_SUBTRACT: u16 = 0x6D;
pub(crate) const VK_DECIMAL: u16 = 0x6E;
pub(crate) const VK_DIVIDE: u16 = 0x6F;
pub(crate) const VK_F1: u16 = 0x70;
pub(crate) const VK_F4: u16 = 0x73;
pub(crate) const VK_F24: u16 = 0x87;
pub(crate) const VK_NUMLOCK: u16 = 0x90;
pub(crate) const VK_SCROLL: u16 = 0x91;

// Cursors.
pub(crate) const IDC_ARROW: u16 = 32512;
pub(crate) const IDC_IBEAM: u16 = 32513;
pub(crate) const IDC_SIZEWE: u16 = 32644;
pub(crate) const IDC_SIZENS: u16 = 32645;
pub(crate) const IDC_SIZEALL: u16 = 32646;
pub(crate) const IDC_NO: u16 = 32648;
pub(crate) const IDC_HAND: u16 = 32649;

// The clipboard.
pub(crate) const CF_UNICODETEXT: u32 = 13;

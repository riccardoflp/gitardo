//! The functions and constants of `dwmapi.dll` that Gitardo Desktop uses: the dark title bar, with the names the
//! Windows documentation gives them.

use core::ffi::c_void;

use super::types::HWND;

#[link(name = "dwmapi")]
unsafe extern "system" {
    pub(crate) fn DwmSetWindowAttribute(window: HWND, attribute: u32, value: *const c_void, size: u32) -> i32;
}

/// Whether the title bar and the borders of a window are dark; Windows 10 version 1809 and later.
pub(crate) const DWMWA_USE_IMMERSIVE_DARK_MODE: u32 = 20;

//! The functions and constants of `shell32.dll` that Gitardo Desktop asks for itself, with the names the Windows
//! documentation gives them: opening an address in the browser of the system.

#![allow(
    clippy::upper_case_acronyms,
    reason = "the names are those of the Windows documentation"
)]
#![allow(
    non_snake_case,
    reason = "the names of the parameters are those of the Windows documentation"
)]

use core::ffi::c_void;

/// Shows the window of what is opened as it normally shows.
pub(crate) const SW_SHOWNORMAL: i32 = 1;

#[link(name = "shell32")]
unsafe extern "system" {
    pub(crate) fn ShellExecuteW(
        hwnd: *mut c_void,
        lpOperation: *const u16,
        lpFile: *const u16,
        lpParameters: *const u16,
        lpDirectory: *const u16,
        nShowCmd: i32,
    ) -> *mut c_void;
}

//! The functions and types of `comctl32.dll` that Gitardo Desktop asks for itself, with the names the Windows
//! documentation gives them: a procedure of its own in front of the one of a window, which the window that draws its
//! own title bar needs to answer what the system asks of its frame.

#![allow(
    clippy::upper_case_acronyms,
    reason = "the names are those of the Windows documentation"
)]
#![allow(
    non_snake_case,
    reason = "the names of the parameters are those of the Windows documentation"
)]

use super::user32::{BOOL, HWND, LPARAM, LRESULT, WPARAM};

/// A procedure put in front of the one of a window, with the number it was given and the data it keeps.
pub(crate) type SUBCLASSPROC = unsafe extern "system" fn(
    hWnd: HWND,
    uMsg: u32,
    wParam: WPARAM,
    lParam: LPARAM,
    uIdSubclass: usize,
    dwRefData: usize,
) -> LRESULT;

#[link(name = "comctl32")]
unsafe extern "system" {
    pub(crate) fn SetWindowSubclass(
        hWnd: HWND,
        pfnSubclass: SUBCLASSPROC,
        uIdSubclass: usize,
        dwRefData: usize,
    ) -> BOOL;
    pub(crate) fn RemoveWindowSubclass(hWnd: HWND, pfnSubclass: SUBCLASSPROC, uIdSubclass: usize) -> BOOL;
    pub(crate) fn DefSubclassProc(hWnd: HWND, uMsg: u32, wParam: WPARAM, lParam: LPARAM) -> LRESULT;
}

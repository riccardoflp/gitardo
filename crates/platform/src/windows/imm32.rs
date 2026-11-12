//! The functions, types and constants of `imm32.dll` that Gitardo Desktop uses: the text an input method commits
//! and where its window goes, with the names the Windows documentation gives them.

#![allow(
    clippy::upper_case_acronyms,
    reason = "the names are those of the Windows documentation"
)]
#![allow(
    non_snake_case,
    reason = "the names of the fields are those of the Windows documentation"
)]

use core::ffi::c_void;

use super::types::{BOOL, HWND, POINT, RECT};

/// A handle to the input context of a window.
pub(crate) type HIMC = isize;

/// Where the window of the composition goes.
#[repr(C)]
pub(crate) struct COMPOSITIONFORM {
    pub(crate) dwStyle: u32,
    pub(crate) ptCurrentPos: POINT,
    pub(crate) rcArea: RECT,
}

#[link(name = "imm32")]
unsafe extern "system" {
    pub(crate) fn ImmGetContext(window: HWND) -> HIMC;
    pub(crate) fn ImmReleaseContext(window: HWND, context: HIMC) -> BOOL;
    pub(crate) fn ImmGetCompositionStringW(context: HIMC, index: u32, buffer: *mut c_void, bytes: u32) -> i32;
    pub(crate) fn ImmSetCompositionWindow(context: HIMC, form: *const COMPOSITIONFORM) -> BOOL;
}

/// The text the input method committed.
pub(crate) const GCS_RESULTSTR: u32 = 0x0800;
/// The composition window goes at a point.
pub(crate) const CFS_POINT: u32 = 0x0002;

//! The functions and constants of `kernel32.dll` that Gitardo Desktop uses: the program's module, error codes and
//! global memory for the clipboard, with the names the Windows documentation gives them.

use core::ffi::c_void;

use super::types::{BOOL, HGLOBAL, HINSTANCE};

#[link(name = "kernel32")]
unsafe extern "system" {
    pub(crate) fn GetModuleHandleW(name: *const u16) -> HINSTANCE;
    pub(crate) fn GetLastError() -> u32;
    pub(crate) fn GlobalAlloc(flags: u32, bytes: usize) -> HGLOBAL;
    pub(crate) fn GlobalLock(memory: HGLOBAL) -> *mut c_void;
    pub(crate) fn GlobalUnlock(memory: HGLOBAL) -> BOOL;
    pub(crate) fn GlobalFree(memory: HGLOBAL) -> HGLOBAL;
    pub(crate) fn GlobalSize(memory: HGLOBAL) -> usize;
}

pub(crate) const GMEM_MOVEABLE: u32 = 0x0002;
pub(crate) const ERROR_FILE_NOT_FOUND: u32 = 2;
pub(crate) const ERROR_CLASS_ALREADY_EXISTS: u32 = 1410;

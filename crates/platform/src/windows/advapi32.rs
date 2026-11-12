//! The functions and constants of `advapi32.dll` that Gitardo Desktop uses: reading the registry, with the names the
//! Windows documentation gives them.

#![allow(
    clippy::upper_case_acronyms,
    reason = "the names are those of the Windows documentation"
)]

use core::ffi::c_void;

/// A handle to a key of the registry.
pub(crate) type HKEY = isize;

#[link(name = "advapi32")]
unsafe extern "system" {
    pub(crate) fn RegGetValueW(
        key: HKEY,
        subkey: *const u16,
        value: *const u16,
        flags: u32,
        value_type: *mut u32,
        data: *mut c_void,
        bytes: *mut u32,
    ) -> i32;
}

/// The settings of the user signed in: `(HKEY)(ULONG_PTR)((LONG)0x80000001)`, extended with its sign.
pub(crate) const HKEY_CURRENT_USER: HKEY = -0x7FFF_FFFF;
/// Only a value of type `REG_DWORD` is read.
pub(crate) const RRF_RT_REG_DWORD: u32 = 0x0000_0010;

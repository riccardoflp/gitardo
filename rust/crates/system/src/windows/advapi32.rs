//! The functions, types and constants of `advapi32.dll` that Gitardo asks for itself, with the names the
//! Windows documentation gives them: the credential store.

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

/// A credential of the kind any program may keep for itself.
pub(crate) const CRED_TYPE_GENERIC: u32 = 1;
/// A credential kept for this user on this computer, across sessions, and not roamed to other computers.
pub(crate) const CRED_PERSIST_LOCAL_MACHINE: u32 = 2;
/// What `GetLastError` says when no credential has the name asked for.
pub(crate) const ERROR_NOT_FOUND: i32 = 1168;

/// A time of the file system: two halves of a count of 100-nanosecond intervals.
#[repr(C)]
pub(crate) struct FILETIME {
    pub(crate) dwLowDateTime: u32,
    pub(crate) dwHighDateTime: u32,
}

/// A credential, as the credential store reads and writes it.
#[repr(C)]
pub(crate) struct CREDENTIALW {
    pub(crate) Flags: u32,
    pub(crate) Type: u32,
    pub(crate) TargetName: *mut u16,
    pub(crate) Comment: *mut u16,
    pub(crate) LastWritten: FILETIME,
    pub(crate) CredentialBlobSize: u32,
    pub(crate) CredentialBlob: *mut u8,
    pub(crate) Persist: u32,
    pub(crate) AttributeCount: u32,
    pub(crate) Attributes: *mut c_void,
    pub(crate) TargetAlias: *mut u16,
    pub(crate) UserName: *mut u16,
}

#[link(name = "advapi32")]
unsafe extern "system" {
    pub(crate) fn CredReadW(TargetName: *const u16, Type: u32, Flags: u32, Credential: *mut *mut CREDENTIALW) -> BOOL;
    pub(crate) fn CredWriteW(Credential: *const CREDENTIALW, Flags: u32) -> BOOL;
    pub(crate) fn CredDeleteW(TargetName: *const u16, Type: u32, Flags: u32) -> BOOL;
    pub(crate) fn CredFree(Buffer: *mut c_void);
}

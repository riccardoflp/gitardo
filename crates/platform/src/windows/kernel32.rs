//! The functions, types and constants of `kernel32.dll` that Gitardo Desktop asks for itself, with the names the
//! Windows documentation gives them: watching folders for changes.

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
/// A handle of the system.
pub(crate) type HANDLE = *mut c_void;

/// What an asynchronous call of the system and its completion share.
#[repr(C)]
pub(crate) struct OVERLAPPED {
    pub(crate) Internal: usize,
    pub(crate) InternalHigh: usize,
    pub(crate) Offset: u32,
    pub(crate) OffsetHigh: u32,
    pub(crate) hEvent: HANDLE,
}

#[link(name = "kernel32")]
unsafe extern "system" {
    pub(crate) fn CreateFileW(
        lpFileName: *const u16,
        dwDesiredAccess: u32,
        dwShareMode: u32,
        lpSecurityAttributes: *mut c_void,
        dwCreationDisposition: u32,
        dwFlagsAndAttributes: u32,
        hTemplateFile: HANDLE,
    ) -> HANDLE;
    pub(crate) fn CreateEventW(
        lpEventAttributes: *mut c_void,
        bManualReset: BOOL,
        bInitialState: BOOL,
        lpName: *const u16,
    ) -> HANDLE;
    pub(crate) fn ReadDirectoryChangesW(
        hDirectory: HANDLE,
        lpBuffer: *mut c_void,
        nBufferLength: u32,
        bWatchSubtree: BOOL,
        dwNotifyFilter: u32,
        lpBytesReturned: *mut u32,
        lpOverlapped: *mut OVERLAPPED,
        lpCompletionRoutine: *mut c_void,
    ) -> BOOL;
    pub(crate) fn GetOverlappedResultEx(
        hFile: HANDLE,
        lpOverlapped: *mut OVERLAPPED,
        lpNumberOfBytesTransferred: *mut u32,
        dwMilliseconds: u32,
        bAlertable: BOOL,
    ) -> BOOL;
    pub(crate) fn GetOverlappedResult(
        hFile: HANDLE,
        lpOverlapped: *mut OVERLAPPED,
        lpNumberOfBytesTransferred: *mut u32,
        bWait: BOOL,
    ) -> BOOL;
    pub(crate) fn CancelIoEx(hFile: HANDLE, lpOverlapped: *mut OVERLAPPED) -> BOOL;
    pub(crate) fn CloseHandle(hObject: HANDLE) -> BOOL;
    pub(crate) fn GetLastError() -> u32;
}

pub(crate) const FILE_LIST_DIRECTORY: u32 = 0x0000_0001;
pub(crate) const FILE_SHARE_READ: u32 = 0x0000_0001;
pub(crate) const FILE_SHARE_WRITE: u32 = 0x0000_0002;
pub(crate) const FILE_SHARE_DELETE: u32 = 0x0000_0004;
pub(crate) const OPEN_EXISTING: u32 = 3;
pub(crate) const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x0200_0000;
pub(crate) const FILE_FLAG_OVERLAPPED: u32 = 0x4000_0000;
pub(crate) const FILE_NOTIFY_CHANGE_FILE_NAME: u32 = 0x0000_0001;
pub(crate) const FILE_NOTIFY_CHANGE_DIR_NAME: u32 = 0x0000_0002;
pub(crate) const FILE_NOTIFY_CHANGE_SIZE: u32 = 0x0000_0008;
pub(crate) const FILE_NOTIFY_CHANGE_LAST_WRITE: u32 = 0x0000_0010;
pub(crate) const FILE_NOTIFY_CHANGE_CREATION: u32 = 0x0000_0040;
pub(crate) const WAIT_TIMEOUT: u32 = 258;
pub(crate) const ERROR_IO_INCOMPLETE: u32 = 996;
pub(crate) const ERROR_NOTIFY_ENUM_DIR: u32 = 1022;

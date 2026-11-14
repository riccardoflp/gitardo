//! The functions, types and constants of `kernel32.dll` that Gitardo asks for itself, with the names the
//! Windows documentation gives them: watching folders for changes, making junctions, and the local time.

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
pub type BOOL = i32;
/// A handle of the system.
pub type HANDLE = *mut c_void;

/// What an asynchronous call of the system and its completion share.
#[repr(C)]
pub struct OVERLAPPED {
    pub Internal: usize,
    pub InternalHigh: usize,
    pub Offset: u32,
    pub OffsetHigh: u32,
    pub hEvent: HANDLE,
}

/// A moment, in 100-nanosecond intervals since 1601, in two halves.
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct FILETIME {
    pub dwLowDateTime: u32,
    pub dwHighDateTime: u32,
}

#[link(name = "kernel32")]
unsafe extern "system" {
    pub fn GetSystemTimeAsFileTime(lpSystemTimeAsFileTime: *mut FILETIME);
    pub fn FileTimeToLocalFileTime(lpFileTime: *const FILETIME, lpLocalFileTime: *mut FILETIME) -> BOOL;
    pub fn CreateFileW(
        lpFileName: *const u16,
        dwDesiredAccess: u32,
        dwShareMode: u32,
        lpSecurityAttributes: *mut c_void,
        dwCreationDisposition: u32,
        dwFlagsAndAttributes: u32,
        hTemplateFile: HANDLE,
    ) -> HANDLE;
    pub fn CreateEventW(
        lpEventAttributes: *mut c_void,
        bManualReset: BOOL,
        bInitialState: BOOL,
        lpName: *const u16,
    ) -> HANDLE;
    pub fn ReadDirectoryChangesW(
        hDirectory: HANDLE,
        lpBuffer: *mut c_void,
        nBufferLength: u32,
        bWatchSubtree: BOOL,
        dwNotifyFilter: u32,
        lpBytesReturned: *mut u32,
        lpOverlapped: *mut OVERLAPPED,
        lpCompletionRoutine: *mut c_void,
    ) -> BOOL;
    pub fn GetOverlappedResultEx(
        hFile: HANDLE,
        lpOverlapped: *mut OVERLAPPED,
        lpNumberOfBytesTransferred: *mut u32,
        dwMilliseconds: u32,
        bAlertable: BOOL,
    ) -> BOOL;
    pub fn GetOverlappedResult(
        hFile: HANDLE,
        lpOverlapped: *mut OVERLAPPED,
        lpNumberOfBytesTransferred: *mut u32,
        bWait: BOOL,
    ) -> BOOL;
    pub fn CancelIoEx(hFile: HANDLE, lpOverlapped: *mut OVERLAPPED) -> BOOL;
    pub fn CloseHandle(hObject: HANDLE) -> BOOL;
    pub fn DeviceIoControl(
        hDevice: HANDLE,
        dwIoControlCode: u32,
        lpInBuffer: *mut c_void,
        nInBufferSize: u32,
        lpOutBuffer: *mut c_void,
        nOutBufferSize: u32,
        lpBytesReturned: *mut u32,
        lpOverlapped: *mut OVERLAPPED,
    ) -> BOOL;
    pub fn GetLastError() -> u32;
}

pub const FILE_LIST_DIRECTORY: u32 = 0x0000_0001;
pub const GENERIC_WRITE: u32 = 0x4000_0000;
pub const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;
pub const FSCTL_SET_REPARSE_POINT: u32 = 0x0009_00A4;
pub const IO_REPARSE_TAG_MOUNT_POINT: u32 = 0xA000_0003;
pub const FILE_SHARE_READ: u32 = 0x0000_0001;
pub const FILE_SHARE_WRITE: u32 = 0x0000_0002;
pub const FILE_SHARE_DELETE: u32 = 0x0000_0004;
pub const OPEN_EXISTING: u32 = 3;
pub const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x0200_0000;
pub const FILE_FLAG_OVERLAPPED: u32 = 0x4000_0000;
pub const FILE_NOTIFY_CHANGE_FILE_NAME: u32 = 0x0000_0001;
pub const FILE_NOTIFY_CHANGE_DIR_NAME: u32 = 0x0000_0002;
pub const FILE_NOTIFY_CHANGE_SIZE: u32 = 0x0000_0008;
pub const FILE_NOTIFY_CHANGE_LAST_WRITE: u32 = 0x0000_0010;
pub const FILE_NOTIFY_CHANGE_CREATION: u32 = 0x0000_0040;
pub const WAIT_TIMEOUT: u32 = 258;
pub const ERROR_IO_INCOMPLETE: u32 = 996;
pub const ERROR_NOTIFY_ENUM_DIR: u32 = 1022;

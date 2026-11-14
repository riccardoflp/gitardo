//! The functions, types and constants of the C library that Gitardo Desktop asks for itself, with the names its manual
//! pages give them: watching folders with inotify, waiting for it with a timeout, and the local time. The C library is linked by the
//! standard library already; nothing more is linked.
//!
//! The values of the flags are those of `x86_64`, `aarch64` and `riscv64`, the systems Gitardo is built for.

#![allow(
    non_camel_case_types,
    reason = "the names of the types are those of the manual pages"
)]

use core::ffi::{c_char, c_int, c_long, c_short, c_ulong};

/// What `poll` waits for on one file, and what it found.
#[repr(C)]
pub(crate) struct pollfd {
    pub(crate) fd: c_int,
    pub(crate) events: c_short,
    pub(crate) revents: c_short,
}

/// How many files `poll` waits on.
pub(crate) type nfds_t = c_ulong;

/// A moment, in seconds since 1970 in UTC.
pub(crate) type time_t = i64;

/// A moment broken into the fields of the local time, as `localtime_r` fills it. Gitardo reads only `tm_gmtoff`.
#[repr(C)]
#[allow(
    dead_code,
    reason = "the C library fills every field, and Gitardo reads only the offset"
)]
#[allow(
    clippy::struct_field_names,
    reason = "the names of the fields are those of the manual pages"
)]
pub(crate) struct tm {
    pub(crate) tm_sec: c_int,
    pub(crate) tm_min: c_int,
    pub(crate) tm_hour: c_int,
    pub(crate) tm_mday: c_int,
    pub(crate) tm_mon: c_int,
    pub(crate) tm_year: c_int,
    pub(crate) tm_wday: c_int,
    pub(crate) tm_yday: c_int,
    pub(crate) tm_isdst: c_int,
    /// How far the local time is from UTC, in seconds east of it.
    pub(crate) tm_gmtoff: c_long,
    pub(crate) tm_zone: *const c_char,
}

unsafe extern "C" {
    pub(crate) fn inotify_init1(flags: c_int) -> c_int;
    pub(crate) fn inotify_add_watch(fd: c_int, pathname: *const c_char, mask: u32) -> c_int;
    pub(crate) fn inotify_rm_watch(fd: c_int, wd: c_int) -> c_int;
    pub(crate) fn poll(fds: *mut pollfd, nfds: nfds_t, timeout: c_int) -> c_int;
    pub(crate) fn localtime_r(timep: *const time_t, result: *mut tm) -> *mut tm;
}

/// `O_NONBLOCK`: reading when nothing is there says so at once instead of waiting.
pub(crate) const IN_NONBLOCK: c_int = 0o4000;
/// `O_CLOEXEC`: the programs Gitardo starts do not inherit the file.
pub(crate) const IN_CLOEXEC: c_int = 0o2_000_000;

pub(crate) const POLLIN: c_short = 0x0001;

pub(crate) const IN_MODIFY: u32 = 0x0000_0002;
pub(crate) const IN_CLOSE_WRITE: u32 = 0x0000_0008;
pub(crate) const IN_MOVED_FROM: u32 = 0x0000_0040;
pub(crate) const IN_MOVED_TO: u32 = 0x0000_0080;
pub(crate) const IN_CREATE: u32 = 0x0000_0100;
pub(crate) const IN_DELETE: u32 = 0x0000_0200;
pub(crate) const IN_DELETE_SELF: u32 = 0x0000_0400;
pub(crate) const IN_MOVE_SELF: u32 = 0x0000_0800;
pub(crate) const IN_Q_OVERFLOW: u32 = 0x0000_4000;
pub(crate) const IN_IGNORED: u32 = 0x0000_8000;
pub(crate) const IN_ONLYDIR: u32 = 0x0100_0000;
pub(crate) const IN_DONT_FOLLOW: u32 = 0x0200_0000;
pub(crate) const IN_EXCL_UNLINK: u32 = 0x0400_0000;
pub(crate) const IN_ISDIR: u32 = 0x4000_0000;

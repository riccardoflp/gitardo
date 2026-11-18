//! The functions and constants of the C library of Linux and macOS that Gitardo asks for itself, with the names
//! their manual gives them: the signal that ends the processes of a group, and the name of the side of a
//! pseudo-terminal its program writes to.

use std::ffi::c_char;

unsafe extern "C" {
    /// Sends a signal to a process, or, for a negative number, to every process of the group of that number.
    pub(crate) fn kill(pid: i32, sig: i32) -> i32;
    /// The name of the side of a pseudo-terminal its program writes to, from the side the window reads, in memory of
    /// the library that the next call overwrites; null when the system cannot name it.
    pub(crate) fn ptsname(fd: i32) -> *mut c_char;
    /// Gives the side of a pseudo-terminal its program writes to to the user of this process.
    #[cfg(test)]
    pub(crate) fn grantpt(fd: i32) -> i32;
    /// Lets the side of a pseudo-terminal its program writes to be opened.
    #[cfg(test)]
    pub(crate) fn unlockpt(fd: i32) -> i32;
}

/// The signal that ends a process, which it cannot ignore.
pub(crate) const SIGKILL: i32 = 9;

/// Opening a terminal does not make it the controlling terminal of this process.
#[cfg(target_os = "linux")]
pub(crate) const O_NOCTTY: i32 = 0o400;
/// Opening a terminal does not make it the controlling terminal of this process.
#[cfg(target_os = "macos")]
pub(crate) const O_NOCTTY: i32 = 0x2_0000;
/// Opening a terminal does not make it the controlling terminal of this process, as the systems of BSD number it.
#[cfg(not(any(target_os = "linux", target_os = "macos")))]
pub(crate) const O_NOCTTY: i32 = 0x8000;

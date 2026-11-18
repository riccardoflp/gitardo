//! Linux and macOS: the process group of a task, ended with a signal asked of the C library, and the side of a
//! pseudo-terminal its program writes to, opened by its name.
//!
//! `libc.rs` declares what is used of the library; `end_group` and `open_program_side` wrap the calls in safe
//! functions at once.

mod libc;

use std::ffi::{CStr, OsStr};
use std::fs::{File, OpenOptions};
use std::os::fd::AsRawFd;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::OpenOptionsExt;
use std::sync::{Mutex, PoisonError};

use crate::error::SystemError;

/// One call of `ptsname` at a time in this process: each answers in the same memory of the library.
static NAMING: Mutex<()> = Mutex::new(());

/// Opens the side of a pseudo-terminal its program writes to, by the name the system gives it from the side the window
/// reads, without making it the controlling terminal of this process.
pub(crate) fn open_program_side(window_side: &File) -> Result<File, SystemError> {
    let name = {
        let _one_at_a_time = NAMING.lock().unwrap_or_else(PoisonError::into_inner);
        // SAFETY: the call takes a number and touches no memory of this process; what it answers is the library's.
        let answered = unsafe { libc::ptsname(window_side.as_raw_fd()) };
        if answered.is_null() {
            return Err(SystemError::system("ptsname", &std::io::Error::last_os_error()));
        }
        // SAFETY: the pointer is not null and is to a string the library ended with a zero, which no other call
        // overwrites while the lock is held; it is copied before the lock is let go.
        unsafe { CStr::from_ptr(answered) }.to_owned()
    };
    OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(libc::O_NOCTTY)
        .open(OsStr::from_bytes(name.to_bytes()))
        .map_err(|error| SystemError::system("open", &error))
}

/// Lets the side of a new pseudo-terminal its program writes to be opened, as starting a program in it does.
#[cfg(test)]
pub(crate) fn unlock_program_side(window_side: &File) -> Result<(), SystemError> {
    let fd = window_side.as_raw_fd();
    // SAFETY: the calls take a number and touch no memory of this process.
    let answered = unsafe { (libc::grantpt(fd), libc::unlockpt(fd)) };
    if answered == (0, 0) {
        Ok(())
    } else {
        Err(SystemError::system("unlockpt", &std::io::Error::last_os_error()))
    }
}

/// Ends every process of a group, by the number of the process that leads it; a group with none left ends nothing.
pub(crate) fn end_group(leader: u32) -> Result<(), SystemError> {
    let Some(group) = i32::try_from(leader).ok().filter(|group| *group > 1) else {
        return Err(SystemError::Unsupported {
            what: "end the processes of a group with such a number",
        });
    };
    // SAFETY: the call takes two numbers and touches no memory of this process; a negative number names the group.
    let sent = unsafe { libc::kill(-group, libc::SIGKILL) };
    if sent == 0 {
        return Ok(());
    }
    let error = std::io::Error::last_os_error();
    // No process is left in the group: there is nothing to end.
    if error.raw_os_error() == Some(3) {
        return Ok(());
    }
    Err(SystemError::system("kill", &error))
}

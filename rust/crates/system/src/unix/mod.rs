//! Linux and macOS: the process group of a task, ended with a signal asked of the C library.
//!
//! `libc.rs` declares what is used of the library; `end_group` wraps the call in a safe function at once.

mod libc;

use crate::error::SystemError;

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

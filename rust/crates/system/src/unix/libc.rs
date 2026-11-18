//! The functions and constants of the C library of Linux and macOS that Gitardo asks for itself, with the names
//! their manual gives them: the signal that ends the processes of a group.

unsafe extern "C" {
    /// Sends a signal to a process, or, for a negative number, to every process of the group of that number.
    pub(crate) fn kill(pid: i32, sig: i32) -> i32;
}

/// The signal that ends a process, which it cannot ignore.
pub(crate) const SIGKILL: i32 = 9;

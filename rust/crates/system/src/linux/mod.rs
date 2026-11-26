//! Linux: the secrets, kept in a file only the user may read until Gitardo speaks to the Secret Service, and the
//! folders watched, asked of inotify in the C library.
//!
//! `libc.rs` declares what is used of the C library, with the names of its manual pages; `watch.rs` wraps it in safe
//! functions at once.

mod credentials;
mod libc;
mod watch;

pub(crate) use credentials::{delete_secret, read_secret, write_secret};
pub(crate) use watch::DirectoryWatch;

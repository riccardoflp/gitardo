//! Linux, on X11 and Wayland alike: what the libraries of the window do not read, asked of `gsettings`, the folders
//! watched, asked of inotify in the C library, and the local time, asked of the C library. The secrets are kept by the
//! crate `system`.
//!
//! `libc.rs` declares what is used of the C library, with the names of its manual pages; `settings.rs`, `watch.rs` and
//! `clock.rs` wrap it, and the programs they run, in safe functions at once.

mod clock;
mod libc;
mod settings;
mod watch;

pub(crate) use clock::utc_offset;
pub(crate) use settings::{double_click_time, is_high_contrast};
pub(crate) use watch::DirectoryWatch;

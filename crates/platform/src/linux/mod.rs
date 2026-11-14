//! Linux, on X11 and Wayland alike: what the libraries of the window do not read, asked of `gsettings`, the folders
//! watched, asked of inotify in the C library, the secrets, kept in a file only the user may read until Gitardo speaks
//! to the Secret Service, and addresses opened with `xdg-open`.
//!
//! `libc.rs` declares what is used of the C library, with the names of its manual pages; `settings.rs`, `watch.rs`,
//! `credentials.rs` and `shell.rs` wrap it, and the programs they run, in safe functions at once.

mod credentials;
mod libc;
mod settings;
mod shell;
mod watch;

pub(crate) use credentials::{delete_secret, read_secret, write_secret};
pub(crate) use settings::{double_click_time, is_high_contrast};
pub(crate) use shell::open_url;
pub(crate) use watch::DirectoryWatch;

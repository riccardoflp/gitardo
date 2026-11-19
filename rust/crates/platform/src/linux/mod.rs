//! Linux, on X11 and Wayland alike: what the libraries of the window do not read, asked of `gsettings`, the folders
//! watched, asked of inotify in the C library, and the local time, asked of the C library; on X11 alone, the menu of
//! the window, asked of the window manager through Xlib. The secrets are kept by the crate `system`.
//!
//! `libc.rs` declares what is used of the C library, with the names of its manual pages; `settings.rs`, `watch.rs` and
//! `clock.rs` wrap it, and the programs they run, in safe functions at once. `window_menu.rs` uses the bindings of Xlib
//! of `x11-dl`, which `winit` uses too.

mod clock;
mod libc;
mod settings;
mod watch;
mod window_menu;

pub(crate) use clock::utc_offset;
pub(crate) use settings::{double_click_time, is_high_contrast};
pub(crate) use watch::DirectoryWatch;
pub(crate) use window_menu::X11Window;

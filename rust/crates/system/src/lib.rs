//! The operating system without a window, as Gitardo's engine needs it: secrets kept in the credential store of the
//! system, links to folders for the worktrees it sets up, and the processes a task started, to end together. It links
//! no library of the window, so that the engine, and `gitardo-engine` with it, can run where there is none
//! (`docs/transition/01-engine.md`).
//!
//! What it asks of the system is declared by hand, one file per system library, and is the reason it may use `unsafe`
//! (rule R5.1). `platform` uses the same bindings for the window's needs: `kernel32` on Windows, the objects of
//! `CoreFoundation` on macOS. They are public for it alone; everything else sees the safe functions.

#![allow(unsafe_code)]
// On a system without a part of its own, what only the parts of Windows, Linux and macOS use is unused.
#![cfg_attr(
    not(any(windows, target_os = "linux", target_os = "macos")),
    allow(dead_code, unused_imports)
)]

mod credentials;
mod error;
mod links;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
pub mod macos;
mod process_tree;
#[cfg(unix)]
mod unix;
#[cfg(windows)]
pub mod windows;

pub use credentials::{delete_secret, read_secret, write_secret};
pub use error::SystemError;
pub use links::link_folder;
pub use process_tree::ProcessTree;

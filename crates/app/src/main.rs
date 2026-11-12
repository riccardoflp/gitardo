//! `gitardo`: the window of Gitardo Desktop. `gitardo <folder>...` opens the window on a workspace of those folders;
//! with none, it opens on the Welcome page.
//!
//! The widgets can be tried in the demo window, `gitardo-demo`.

#![forbid(unsafe_code)]
#![windows_subsystem = "windows"]

use std::path::PathBuf;
use std::process::ExitCode;

use app::desktop::{StartOptions, run};
use app::settings::configuration_folder;

/// Opens the window and runs until it is closed. A failure ends the program with a code of 1: the window has no
/// console to say more in.
fn main() -> ExitCode {
    let folders = std::env::args_os().skip(1).map(PathBuf::from).map(absolute).collect();
    let options = StartOptions {
        folders,
        configuration: configuration_folder(),
        visibility: platform::Visibility::Shown,
    };
    match run(&options) {
        Ok(()) => ExitCode::SUCCESS,
        Err(_) => ExitCode::FAILURE,
    }
}

/// A folder of the command line from the current folder, so that the workspace names the same folders later.
fn absolute(folder: PathBuf) -> PathBuf {
    std::path::absolute(&folder).unwrap_or(folder)
}

//! The chores and checks of the Gitardo Desktop project, run with `cargo xtask <command>`.

#![forbid(unsafe_code)]

mod catalog;
mod check;
mod error;
mod files;
mod problem;
mod source;
mod toml;
mod workspace;

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// What `cargo xtask` with no command, or with one it does not know, says.
const USAGE: &str = "Usage: cargo xtask <command>

Commands:
  check    rustfmt, clippy, the tests, and the rules of docs/03-code-rules.md that a program can check";

fn main() -> ExitCode {
    let command = std::env::args().nth(1);
    let outcome = match command.as_deref() {
        Some("check") => check::run(&workspace_root()),
        _ => writeln!(std::io::stderr().lock(), "{USAGE}")
            .map(|()| false)
            .map_err(error::Error::Output),
    };
    match outcome {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(error) => {
            // Nothing is left to tell the user when even the error cannot be written.
            let _ = writeln!(std::io::stderr().lock(), "{error}");
            ExitCode::FAILURE
        }
    }
}

/// The root of the workspace: the folder above this crate's own.
fn workspace_root() -> PathBuf {
    let manifest_folder = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest_folder.parent().unwrap_or(manifest_folder).to_path_buf()
}

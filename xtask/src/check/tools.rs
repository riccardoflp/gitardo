//! Rules R11, R12.1 and R12.2: the formatting of rustfmt, the lints of clippy with warnings denied, and the tests.

use std::io::Write;
use std::path::Path;
use std::process::Command;

use crate::error::Error;

/// The tools that run, in order, each with the arguments of `cargo` and what it checks.
const TOOLS: [(&[&str], &str); 3] = [
    (&["fmt", "--all", "--", "--check"], "the formatting of rustfmt"),
    (
        &[
            "clippy",
            "--workspace",
            "--all-targets",
            "--quiet",
            "--",
            "-D",
            "warnings",
        ],
        "the lints of clippy",
    ),
    (&["test", "--workspace", "--quiet"], "the tests"),
];

/// Runs every tool in the workspace at `root`, showing what it prints, and returns whether all passed.
pub fn run(root: &Path) -> Result<bool, Error> {
    // Cargo says which cargo runs this program, so that the tools are those of the same toolchain.
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let mut all_passed = true;
    for (arguments, what) in TOOLS {
        let command = format!("cargo {}", arguments.join(" "));
        writeln!(std::io::stdout().lock(), "Checking {what}: {command}")?;
        let status = Command::new(&cargo)
            .args(arguments)
            .current_dir(root)
            .status()
            .map_err(|source| Error::Tool {
                command: command.clone(),
                source,
            })?;
        if !status.success() {
            writeln!(std::io::stdout().lock(), "Failed: {what}.")?;
            all_passed = false;
        }
    }
    Ok(all_passed)
}

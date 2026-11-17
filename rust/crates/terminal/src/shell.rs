//! The shell a terminal runs: the one of `terminal.shellPath`, or the system's own.

use std::collections::HashMap;
use std::path::Path;

/// A shell: the program and what it is given.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Shell {
    /// The program, by its path or by a name looked for on the `PATH`.
    pub program: String,
    /// Its arguments.
    pub args: Vec<String>,
}

impl Shell {
    /// Its name, as the list of the terminals says it: the name of its program without its extension, `pwsh`.
    #[must_use]
    pub fn name(&self) -> String {
        let program = Path::new(&self.program);
        program
            .file_stem()
            .map_or_else(|| self.program.clone(), |stem| stem.to_string_lossy().into_owned())
    }
}

/// The shell of `terminal.shellPath`: the program it names, or the system's own when it is empty.
#[must_use]
pub fn shell_of(setting: &str, path: Option<&std::ffi::OsStr>) -> Shell {
    let setting = setting.trim();
    if setting.is_empty() {
        default_shell(path)
    } else {
        Shell {
            program: setting.to_owned(),
            args: Vec::new(),
        }
    }
}

/// The system's own shell: on Windows PowerShell 7 when `pwsh.exe` is on the `PATH` given, else Windows PowerShell; on
/// Linux and macOS the shell of `SHELL`, a login shell on macOS as its terminal starts it, else `/bin/sh`.
#[must_use]
pub fn default_shell(path: Option<&std::ffi::OsStr>) -> Shell {
    if cfg!(windows) {
        let has_pwsh =
            path.is_some_and(|path| std::env::split_paths(path).any(|folder| folder.join("pwsh.exe").is_file()));
        let program = if has_pwsh { "pwsh.exe" } else { "powershell.exe" };
        return Shell {
            program: program.to_owned(),
            args: vec![String::from("-NoLogo")],
        };
    }
    let program = std::env::var("SHELL")
        .ok()
        .filter(|shell| !shell.is_empty())
        .unwrap_or_else(|| String::from("/bin/sh"));
    let args = if cfg!(target_os = "macos") {
        vec![String::from("-l")]
    } else {
        Vec::new()
    };
    Shell { program, args }
}

/// What the shell finds in its environment besides the window's: a terminal of 256 colors that takes 24-bit colors too.
pub(crate) fn environment() -> HashMap<String, String> {
    HashMap::from([
        (String::from("TERM"), String::from("xterm-256color")),
        (String::from("COLORTERM"), String::from("truecolor")),
    ])
}

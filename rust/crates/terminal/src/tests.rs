//! The terminal driven through a real shell: Windows PowerShell on Windows, `sh` elsewhere, without a profile, in a
//! throwaway repository.

use std::path::Path;
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use model::terminal::{CellColor, Screen, Selection};
use testkit::Repository;

use super::{GridSize, Shell, StartOptions, Terminal, TerminalEvent};

/// How long a shell has to do what a test waits for.
const PATIENCE: Duration = Duration::from_secs(30);

/// The shell of the tests, without what a profile would print.
fn test_shell() -> Shell {
    if cfg!(windows) {
        Shell {
            program: String::from("powershell.exe"),
            args: vec![String::from("-NoLogo"), String::from("-NoProfile")],
        }
    } else {
        Shell {
            program: String::from("/bin/sh"),
            args: Vec::new(),
        }
    }
}

/// A terminal started for a test, and what it tells.
struct Running {
    terminal: Terminal,
    told: Receiver<TerminalEvent>,
}

impl Running {
    /// Starts the shell of the tests in a folder.
    fn start(folder: &Path, size: GridSize) -> Running {
        let (sender, told) = mpsc::channel();
        let sender = Mutex::new(sender);
        let tell: super::Tell = Arc::new(move |event| {
            let _sent = sender.lock().unwrap().send(event);
        });
        let options = StartOptions {
            folder: folder.to_path_buf(),
            shell: test_shell(),
            size,
            scrollback: 1_000,
            environment: Vec::new(),
        };
        Running {
            terminal: Terminal::start(options, tell).unwrap(),
            told,
        }
    }

    /// Types a line, ended by `Enter`.
    fn type_line(&self, line: &str) {
        self.terminal.write(format!("{line}\r").into_bytes());
    }

    /// Waits until the screen is as `wanted` says, and gives it.
    fn wait_for(&self, what: &str, wanted: impl Fn(&Screen) -> bool) -> Screen {
        let started = Instant::now();
        loop {
            self.terminal.take_wake();
            let screen = self.terminal.screen();
            if wanted(&screen) {
                return screen;
            }
            assert!(
                started.elapsed() < PATIENCE,
                "{what} did not come within 30 s:\n{}",
                screen.text()
            );
            let _told = self.told.recv_timeout(Duration::from_millis(100));
        }
    }

    /// Waits until the shell ended, and gives its exit code.
    fn wait_for_end(&self) -> Option<i32> {
        let started = Instant::now();
        while started.elapsed() < PATIENCE {
            if let Ok(TerminalEvent::Ended(code)) = self.told.recv_timeout(Duration::from_millis(100)) {
                return Some(code).flatten().or(Some(-1));
            }
        }
        panic!("the shell did not end within 30 s:\n{}", self.terminal.screen().text());
    }
}

/// A size wide enough for any path of a temporary folder.
const WIDE: GridSize = GridSize { columns: 200, rows: 30 };

#[test]
fn the_shell_starts_in_the_folder_given_and_prints_what_is_typed() {
    let repository = Repository::new();
    let running = Running::start(repository.path(), WIDE);
    running.type_line("pwd");
    let folder = repository.path().file_name().unwrap().to_string_lossy().into_owned();
    let parent = repository
        .path()
        .parent()
        .unwrap()
        .file_name()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    let wanted = format!("{parent}{}{folder}", std::path::MAIN_SEPARATOR);
    let _screen = running.wait_for("the folder", |screen| {
        screen.text().lines().any(|line| line.trim_end().ends_with(&wanted))
    });
}

#[test]
fn colors_printed_are_cells_of_those_colors() {
    let repository = Repository::new();
    let running = Running::start(repository.path(), WIDE);
    running.type_line("git --version");
    let _version = running.wait_for("git's version", |screen| screen.text().contains("git version"));
    if cfg!(windows) {
        running.type_line("Write-Host -ForegroundColor Red 'red text'");
    } else {
        running.type_line(r"printf '\033[31mred text\033[0m\n'");
    }
    let screen = running.wait_for("the red text", |screen| {
        screen
            .rows
            .iter()
            .flatten()
            .any(|span| span.text.trim() == "red text" && span.style.foreground != CellColor::Default)
    });
    let red = screen
        .rows
        .iter()
        .flatten()
        .find(|span| span.text.trim() == "red text")
        .unwrap();
    assert!(
        matches!(
            red.style.foreground,
            CellColor::Indexed(1 | 9) | CellColor::Rgb(_, _, _)
        ),
        "{:?}",
        red.style
    );
}

#[test]
fn the_end_of_the_shell_is_told_with_its_exit_code() {
    let repository = Repository::new();
    let running = Running::start(repository.path(), WIDE);
    running.type_line("exit 3");
    assert_eq!(running.wait_for_end(), Some(3));
}

#[test]
fn a_new_size_is_told_to_the_shell() {
    let repository = Repository::new();
    let mut running = Running::start(repository.path(), GridSize { columns: 100, rows: 20 });
    let ask = if cfg!(windows) {
        "'cols=' + [Console]::WindowWidth"
    } else {
        // `stty` reads the size of its input, which is still the terminal inside `$(…)`; the `tput` of macOS reads that
        // of its output, there a pipe, and says 80.
        "echo cols=$(stty size | cut -d' ' -f2)"
    };
    running.type_line(ask);
    let _wide = running.wait_for("100 columns", |screen| screen.text().contains("cols=100"));
    running.terminal.resize(GridSize { columns: 60, rows: 20 });
    running.type_line(ask);
    let screen = running.wait_for("60 columns", |screen| screen.text().contains("cols=60"));
    assert_eq!(screen.columns, 60);
}

#[test]
fn a_word_selected_with_a_double_click_is_the_text_selected() {
    let repository = Repository::new();
    let running = Running::start(repository.path(), WIDE);
    running.type_line("echo selectable");
    let screen = running.wait_for("the word", |screen| {
        screen.text().lines().any(|line| line.trim() == "selectable")
    });
    let lines: Vec<String> = screen.text().lines().map(str::to_owned).collect();
    let row = lines.iter().position(|line| line.trim() == "selectable").unwrap();
    let column = lines[row].find("selectable").unwrap() + 3;
    running.terminal.select(Some(Selection {
        from: (row, column),
        to: (row, column),
        is_word: true,
    }));
    let screen = running.terminal.screen();
    assert_eq!(screen.selected_text.as_deref(), Some("selectable"));
    assert!(
        screen.selected.iter().any(|(selected_row, _, _)| *selected_row == row),
        "{:?}",
        screen.selected
    );
    running.terminal.select(None);
    assert_eq!(running.terminal.screen().selected_text, None);
}

#[test]
fn what_its_start_asks_for_is_in_its_environment_and_its_last_lines_are_read_without_colors() {
    let repository = Repository::new();
    let (sender, told) = mpsc::channel();
    let sender = Mutex::new(sender);
    let tell: super::Tell = Arc::new(move |event| {
        let _sent = sender.lock().unwrap().send(event);
    });
    let options = StartOptions {
        folder: repository.path().to_path_buf(),
        shell: test_shell(),
        size: WIDE,
        scrollback: 1_000,
        environment: vec![(String::from("GITARDO_ASKED"), String::from("asked for"))],
    };
    let running = Running {
        terminal: Terminal::start(options, tell).unwrap(),
        told,
    };
    assert!(running.terminal.process().is_some_and(|process| process > 0));
    if cfg!(windows) {
        running.type_line("Write-Host -ForegroundColor Red \"it is $env:GITARDO_ASKED\"");
    } else {
        running.type_line(r#"printf '\033[31mit is %s\033[0m\n' "$GITARDO_ASKED""#);
    }
    let _screen = running.wait_for("the variable", |screen| {
        screen.text().lines().any(|line| line.trim() == "it is asked for")
    });
    let lines = running.terminal.last_lines(5);
    assert!(lines.len() <= 5);
    assert!(lines.iter().any(|line| line.trim() == "it is asked for"), "{lines:?}");
}

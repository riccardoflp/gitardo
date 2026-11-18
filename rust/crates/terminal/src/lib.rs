//! The terminals of Gitardo Desktop (`terminal.view`, D53): a shell started in a folder through a pseudo-terminal of
//! the system (`ConPTY` on Windows), with `TERM=xterm-256color`, what it prints parsed into a grid with its scrollback on
//! a thread of its own, and the screen read from that grid as plain values for the window to draw.
//!
//! The library that does it, `alacritty_terminal`, stays inside this crate (rule R1.6): nothing of it is in what this
//! crate gives.

#![forbid(unsafe_code)]

mod screen;
mod shell;

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};

use alacritty_terminal::event::{Event, EventListener, WindowSize};
use alacritty_terminal::event_loop::{EventLoop, EventLoopSender, Msg};
use alacritty_terminal::grid::{Dimensions, Scroll};
use alacritty_terminal::index::{Column, Line, Point, Side};
use alacritty_terminal::selection::{Selection as GridSelection, SelectionType};
use alacritty_terminal::sync::FairMutex;
use alacritty_terminal::term::{Config, Term, viewport_to_point};
use alacritty_terminal::tty;
use model::terminal::Selection;

pub use shell::{Shell, default_shell, shell_of};

/// What can go wrong with a terminal.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    /// The pseudo-terminal or the shell in it could not start, with what the system said.
    NotStarted {
        /// The shell.
        shell: String,
        /// What the system said.
        reason: String,
    },
}

impl std::fmt::Display for Error {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::NotStarted { shell, reason } => write!(formatter, "{shell} could not start: {reason}"),
        }
    }
}

impl std::error::Error for Error {}

/// The size of a terminal, in cells.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GridSize {
    /// How many columns.
    pub columns: u16,
    /// How many rows.
    pub rows: u16,
}

impl GridSize {
    /// The size, at least two columns and one row.
    fn at_least_one(self) -> GridSize {
        GridSize {
            columns: self.columns.max(2),
            rows: self.rows.max(1),
        }
    }

    /// The size, as the pseudo-terminal is told it.
    fn window_size(self) -> WindowSize {
        WindowSize {
            num_lines: self.rows,
            num_cols: self.columns,
            cell_width: 1,
            cell_height: 1,
        }
    }
}

/// The size of a grid, as the library asks for it.
struct Dimension {
    /// The size.
    size: GridSize,
}

impl Dimensions for Dimension {
    fn total_lines(&self) -> usize {
        usize::from(self.size.rows)
    }

    fn screen_lines(&self) -> usize {
        usize::from(self.size.rows)
    }

    fn columns(&self) -> usize {
        usize::from(self.size.columns)
    }
}

/// What a terminal tells the window, from its own thread.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TerminalEvent {
    /// It printed something, for the window to draw it again: told once until [`Terminal::take_wake`] is called.
    Printed,
    /// Its shell ended, with its exit code; none when the system gave none, as for a shell killed by a signal.
    Ended(Option<i32>),
}

/// How a terminal starts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StartOptions {
    /// The folder the shell starts in.
    pub folder: PathBuf,
    /// The shell.
    pub shell: Shell,
    /// Its size.
    pub size: GridSize,
    /// How many lines it keeps above the screen.
    pub scrollback: usize,
    /// What the shell finds in its environment over the window's own, such as the `options.env` of a task.
    pub environment: Vec<(String, String)>,
}

/// Where a terminal tells the window what happens.
pub type Tell = Arc<dyn Fn(TerminalEvent) + Send + Sync>;

/// What the thread of a terminal tells: the window, once for each print until it drew it, and the pseudo-terminal the
/// answers the shell asks for, such as where the cursor is.
#[derive(Clone)]
struct Listener {
    /// Where the window is told.
    tell: Tell,
    /// Whether the window was told of a print it has not drawn yet.
    is_woken: Arc<AtomicBool>,
    /// Where the answers go, once the thread is made.
    sender: Arc<OnceLock<EventLoopSender>>,
}

impl EventListener for Listener {
    fn send_event(&self, event: Event) {
        match event {
            Event::Wakeup => {
                if !self.is_woken.swap(true, Ordering::AcqRel) {
                    (self.tell)(TerminalEvent::Printed);
                }
            }
            Event::ChildExit(status) => (self.tell)(TerminalEvent::Ended(status.code())),
            Event::PtyWrite(text) => {
                if let Some(sender) = self.sender.get() {
                    // A terminal that is ending takes no answer.
                    let _sent = sender.send(Msg::Input(text.into_bytes().into()));
                }
            }
            // The rest is what a window of its own would do (a title, a bell, the clipboard), which the panel does not.
            _ => {}
        }
    }
}

/// A shell running in a pseudo-terminal, and the grid of what it printed.
pub struct Terminal {
    /// The grid, which the thread fills.
    term: Arc<FairMutex<Term<Listener>>>,
    /// Where what is typed, a new size and the end go.
    sender: EventLoopSender,
    /// Whether the window was told of a print it has not drawn yet.
    is_woken: Arc<AtomicBool>,
    /// The size it has.
    size: GridSize,
    /// The shell it runs.
    shell: Shell,
    /// The number of the process of its shell.
    process: Option<u32>,
    /// The side of its pseudo-terminal the shell writes to, held open as long as the terminal is, on Linux and macOS:
    /// what a program printed right before it ended is then still read, where a read that found the program gone
    /// would drop it. Windows has no such side.
    _program_side: Option<std::fs::File>,
}

impl Terminal {
    /// Starts a shell in a folder, in a pseudo-terminal of a size, on a thread that reads what it prints and tells
    /// `tell`.
    ///
    /// # Errors
    ///
    /// `Error::NotStarted` when the system could not make the pseudo-terminal or start the shell.
    pub fn start(options: StartOptions, tell: Tell) -> Result<Terminal, Error> {
        let size = options.size.at_least_one();
        let not_started = |reason: String| Error::NotStarted {
            shell: options.shell.name(),
            reason,
        };
        let pty_options = tty::Options {
            shell: Some(tty::Shell::new(
                options.shell.program.clone(),
                options.shell.args.clone(),
            )),
            working_directory: Some(options.folder.clone()),
            drain_on_exit: true,
            env: shell::environment(&options.environment),
            #[cfg(windows)]
            escape_args: true,
        };
        let pty = tty::new(&pty_options, size.window_size(), 0).map_err(|error| not_started(error.to_string()))?;
        #[cfg(windows)]
        let process = pty.child_watcher().pid().map(std::num::NonZeroU32::get);
        #[cfg(not(windows))]
        let process = Some(pty.child().id());
        // A terminal that cannot hold that side open still runs: what is printed right before the end may be lost.
        #[cfg(not(windows))]
        let program_side = system::open_program_side(pty.file()).ok();
        #[cfg(windows)]
        let program_side = None;
        let is_woken = Arc::new(AtomicBool::new(false));
        let listener = Listener {
            tell,
            is_woken: Arc::clone(&is_woken),
            sender: Arc::new(OnceLock::new()),
        };
        let config = Config {
            scrolling_history: options.scrollback,
            ..Config::default()
        };
        let term = Arc::new(FairMutex::new(Term::new(config, &Dimension { size }, listener.clone())));
        let event_loop = EventLoop::new(Arc::clone(&term), listener.clone(), pty, true, false)
            .map_err(|error| not_started(error.to_string()))?;
        let sender = event_loop.channel();
        let _set = listener.sender.set(event_loop.channel());
        // The thread ends when the shell ends or when the terminal is dropped; nothing waits for it.
        let _thread = event_loop.spawn();
        Ok(Terminal {
            term,
            sender,
            is_woken,
            size,
            shell: options.shell,
            process,
            _program_side: program_side,
        })
    }

    /// The number the system gave the process of its shell, when it said it: the process leads a group of its own on
    /// Linux and macOS, as the pseudo-terminal starts it.
    #[must_use]
    pub fn process(&self) -> Option<u32> {
        self.process
    }

    /// The last lines it printed, down to the one of the cursor, as many as a count at most, without their colors: a
    /// line longer than the terminal is wide is one line.
    #[must_use]
    pub fn last_lines(&self, count: usize) -> Vec<String> {
        let term = self.term.lock();
        let last = term.grid().cursor.point.line;
        let count = i32::try_from(count.saturating_sub(1)).unwrap_or(i32::MAX);
        let first = Line(last.0.saturating_sub(count)).max(term.topmost_line());
        let text = term.bounds_to_string(Point::new(first, Column(0)), Point::new(last, term.last_column()));
        text.lines().map(str::to_owned).collect()
    }

    /// The shell it runs.
    #[must_use]
    pub fn shell(&self) -> &Shell {
        &self.shell
    }

    /// Lets it tell the window again the next time it prints: the window is about to draw what it printed.
    pub fn take_wake(&self) {
        self.is_woken.store(false, Ordering::Release);
    }

    /// Gives the shell what was typed or pasted, and shows the bottom of what it printed.
    pub fn write(&self, bytes: Vec<u8>) {
        self.term.lock().scroll_display(Scroll::Bottom);
        // A shell that ended reads nothing more.
        let _sent = self.sender.send(Msg::Input(bytes.into()));
    }

    /// Tells the shell its new size, when it changed.
    pub fn resize(&mut self, size: GridSize) {
        let size = size.at_least_one();
        if size == self.size {
            return;
        }
        self.size = size;
        self.term.lock().resize(Dimension { size });
        // A shell that ended has no size to be told.
        let _sent = self.sender.send(Msg::Resize(size.window_size()));
    }

    /// Scrolls what is shown by some lines, up into what it printed before for a positive number.
    pub fn scroll(&self, lines: i32) {
        self.term.lock().scroll_display(Scroll::Delta(lines));
    }

    /// Selects from a cell of the screen shown to another, or the word under a cell; none takes the selection away.
    pub fn select(&self, selected: Option<Selection>) {
        let mut term = self.term.lock();
        let Some(selected) = selected else {
            term.selection = None;
            return;
        };
        let offset = term.grid().display_offset();
        let point = |(row, column): (usize, usize)| viewport_to_point(offset, Point::new(row, Column(column)));
        let (from, to) = (point(selected.from), point(selected.to));
        let (kind, from_side, to_side) = match (selected.is_word, from <= to) {
            (true, _) => (SelectionType::Semantic, Side::Left, Side::Right),
            (false, true) => (SelectionType::Simple, Side::Left, Side::Right),
            (false, false) => (SelectionType::Simple, Side::Right, Side::Left),
        };
        let mut selection = GridSelection::new(kind, from, from_side);
        selection.update(to, to_side);
        term.selection = Some(selection);
    }

    /// What it shows now: the rows of the screen, the cursor, what is selected, the modes the keys are sent in.
    #[must_use]
    pub fn screen(&self) -> model::terminal::Screen {
        screen::of(&self.term.lock())
    }
}

impl Drop for Terminal {
    /// The shell ends with the terminal: its thread stops, and the pseudo-terminal it held ends the shell.
    fn drop(&mut self) {
        // A terminal whose shell ended has no thread left to stop.
        let _sent = self.sender.send(Msg::Shutdown);
    }
}

#[cfg(test)]
mod tests;

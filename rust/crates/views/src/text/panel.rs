//! The words of the bottom panel and its terminals (`panel.bottom`, `terminal.view`).

/// The tab of the terminals, and the button of the bar of views that shows and hides the panel.
pub const TERMINAL: &str = "Terminal";
/// The tab of what runs, and the button of the bar of views that shows and hides the panel at it
/// (`activity.what-is-running`).
pub const ACTIVITY: &str = "Activity";
/// The tab of the Command log (`log.show`).
pub const COMMAND_LOG: &str = "Command log";
/// The name of the panel, for a screen reader.
pub const PANEL: &str = "Bottom panel";
/// The button that hides the panel and stops nothing.
pub const HIDE_PANEL: &str = "Hide the panel";
/// The button that starts another terminal.
pub const NEW_TERMINAL: &str = "New terminal";
/// The button that ends the shell of the terminal shown and takes the terminal away.
pub const KILL_TERMINAL: &str = "Kill the terminal";
/// The terminal, as the screen reader says it.
pub const TERMINAL_AREA: &str = "Terminal";
/// What the Terminal tab says while no repository is shown, when a terminal cannot start.
pub const NO_REPOSITORY: &str = "Open a repository to start a terminal in its folder";
/// What the Terminal tab says when the shell could not start.
pub const NOT_STARTED: &str = "The shell could not start";

/// What a terminal says under what it printed once its shell ended: `The shell ended with exit code 0`, or without a
/// code when the system gave none.
#[must_use]
pub fn shell_ended(code: Option<i32>) -> String {
    match code {
        Some(code) => format!("The shell ended with exit code {code}"),
        None => String::from("The shell ended"),
    }
}

/// What it says under that: `Enter` starts another shell.
pub const PRESS_ENTER: &str = "Press Enter to start a new one";

/// The name of a terminal in the list of the terminals of the panel: its number and its shell, `1: pwsh`.
#[must_use]
pub fn terminal_name(number: usize, shell: &str) -> String {
    format!("{number}: {shell}")
}

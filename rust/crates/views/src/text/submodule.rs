//! The words of the Submodules list of the branch rail (`submodule.section`) and of the menu of a submodule.

/// **Open in new window** (`submodule.open-in-new-window`).
pub const OPEN_IN_NEW_WINDOW: &str = "Open in new window";
/// **Open in terminal** (`submodule.open-in-terminal`).
pub const OPEN_IN_TERMINAL: &str = "Open in terminal";
/// **Show in Gitardo** (`submodule.show-in-gitardo`).
pub const SHOW_IN_GITARDO: &str = "Show in Gitardo";
/// **Initialize** (`submodule.initialize`).
pub const INITIALIZE: &str = "Initialize";
/// **Synchronize URLs from .gitmodules** (`submodule.sync-urls`).
pub const SYNC_URLS: &str = "Synchronize URLs from .gitmodules";
/// **Copy path** (`submodule.copy-path`).
pub const COPY_PATH: &str = "Copy path";
/// **Copy URL** (`submodule.copy-url`).
pub const COPY_URL: &str = "Copy URL";
/// **Deinitialize…** (`submodule.deinitialize`).
pub const DEINITIALIZE: &str = "Deinitialize\u{2026}";
/// **Remove…** (`submodule.remove`).
pub const REMOVE: &str = "Remove\u{2026}";
/// The **+** on the title of the list, for its tooltip (`submodule.add`).
pub const ADD: &str = "Add submodule";
/// A submodule with no branch checked out.
pub const DETACHED: &str = "detached";
/// The pencil beside a submodule with files not committed, for the screen reader.
pub const UNCOMMITTED: &str = "Files not committed";

/// **Reveal in File Explorer** on Windows, **Reveal in Finder** on macOS, **Open containing folder** elsewhere
/// (`submodule.reveal`), by the name Rust gives the system.
#[must_use]
pub fn reveal(system: &str) -> &'static str {
    match system {
        "windows" => "Reveal in File Explorer",
        "macos" => "Reveal in Finder",
        _ => "Open containing folder",
    }
}

/// **Update to the recorded commit `<short>`** (`submodule.update-to-recorded`).
#[must_use]
pub fn update_to_recorded(short: &str) -> String {
    format!("Update to the recorded commit {short}")
}

/// **Stage `<short>` as the recorded commit** (`submodule.stage-recorded`).
#[must_use]
pub fn stage_recorded(short: &str) -> String {
    format!("Stage {short} as the recorded commit")
}

/// **Update to the latest of `<branch>`**, or **of its remote** when `.gitmodules` gives it no branch
/// (`submodule.update-to-latest`).
#[must_use]
pub fn update_to_latest(branch: Option<&str>) -> String {
    match branch {
        Some(branch) => format!("Update to the latest of {branch}"),
        None => String::from("Update to the latest of its remote"),
    }
}

/// **Checked out: `<branch or detached>` at `<short>`**, in the tooltip of a submodule.
#[must_use]
pub fn checked_out(branch: Option<&str>, short: Option<&str>) -> String {
    let branch = branch.unwrap_or(DETACHED);
    match short {
        Some(short) => format!("Checked out: {branch} at {short}"),
        None => format!("Checked out: {branch}"),
    }
}

/// **Staged as the recorded commit: `<short>`**, in the tooltip of a submodule with another commit staged for it.
#[must_use]
pub fn staged(short: &str) -> String {
    format!("Staged as the recorded commit: {short}")
}

/// How many files are not committed in a submodule, in its tooltip.
#[must_use]
pub fn uncommitted(files: usize) -> String {
    if files == 1 {
        String::from("1 file not committed")
    } else {
        format!("{files} files not committed")
    }
}

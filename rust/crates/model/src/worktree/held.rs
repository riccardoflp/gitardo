//! `worktree.recover-held-folder`: what is done when Windows holds the folder of a worktree that git moves or removes.

use std::path::PathBuf;

/// How many times a move is tried in all.
pub const MOVE_TRIES: usize = 20;
/// How long a move waits before it is tried again, in milliseconds.
pub const MOVE_WAIT: u64 = 200;
/// How many times what is left of a folder git forgot is deleted before it gives up.
pub const DELETE_TRIES: usize = 10;

/// What the log says before a move is tried again.
pub const TRIED_AGAIN: &str = "Windows holds the folder, so it is tried again.";
/// What the log says before what is left of a folder git forgot is deleted.
pub const DELETING_LEFT: &str =
    "Windows held the folder. It is no longer a worktree, and Gitardo deletes what is left of it.";

/// A git step on the folder of a worktree that failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HeldInput<'a> {
    /// The arguments of the step, after `git`.
    pub arguments: &'a [String],
    /// What git said.
    pub said: &'a str,
    /// How many times it has failed, this time included.
    pub failures: usize,
    /// Whether it runs on Windows, the only system that holds folders so.
    pub is_windows: bool,
    /// Whether git still lists the worktree removed, once that was read; none before.
    pub is_listed: Option<bool>,
}

/// What is done about a step that failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Recovery {
    /// It is run again after a wait, the log saying this first.
    TryAgain {
        /// How long to wait, in milliseconds.
        wait: u64,
        /// What the log says: the first line of git's message, then why it is tried again.
        note: String,
    },
    /// Whether git still lists the worktree is read, and this asked again with the answer.
    ReadListed,
    /// Git forgot the worktree: what is left of its folder is deleted, links and not what they lead to, and the plan
    /// goes on.
    DeleteLeft {
        /// The folder.
        folder: PathBuf,
        /// What the log says first.
        note: String,
    },
    /// The failure is the operation's.
    Fail,
}

/// What is done when `git worktree move` or `git worktree remove` fails: on Windows, a move failing because the folder
/// is held changed nothing and is tried again, up to 20 times, 200 ms apart; a removal failing to delete the folder,
/// once git no longer lists the worktree, leaves the rest of the folder to delete. `git submodule deinit` and `git rm`
/// of a submodule (`submodule.deinitialize`, `submodule.remove`) failing on a folder that is held are tried again the
/// same way: git checks and writes its index only once the folder is gone, so running them again is safe. Any other
/// failure, and any failure elsewhere, is the operation's.
///
/// Feature: worktree.recover-held-folder
#[must_use]
pub fn recover_held_folder(input: &HeldInput<'_>) -> Recovery {
    if !input.is_windows {
        return Recovery::Fail;
    }
    let all: Vec<&str> = input.arguments.iter().map(String::as_str).collect();
    // A submodule inside another is worked on with `-C <its parent>` first.
    let words = match all.as_slice() {
        ["-C", _, rest @ ..] => rest,
        words => words,
    };
    match words {
        ["submodule", "deinit", ..] | ["rm", ..] if is_held_submodule(input.said) && input.failures < MOVE_TRIES => {
            Recovery::TryAgain {
                wait: MOVE_WAIT,
                note: format!("{} {TRIED_AGAIN}", first_line(input.said)),
            }
        }
        ["worktree", "move", ..] if is_held_move(input.said) && input.failures < MOVE_TRIES => Recovery::TryAgain {
            wait: MOVE_WAIT,
            note: format!("{} {TRIED_AGAIN}", first_line(input.said)),
        },
        ["worktree", "remove", .., folder] if input.said.contains("failed to delete '") => match input.is_listed {
            None => Recovery::ReadListed,
            Some(false) => Recovery::DeleteLeft {
                folder: PathBuf::from(folder),
                note: DELETING_LEFT.to_owned(),
            },
            Some(true) => Recovery::Fail,
        },
        _ => Recovery::Fail,
    }
}

/// Whether git failed to empty or delete the folder of a submodule because another program holds it.
fn is_held_submodule(said: &str) -> bool {
    ["could not remove", "Permission denied", "Device or resource busy"]
        .iter()
        .any(|reason| said.contains(reason))
}

/// Whether git failed to move a folder because another program holds it.
fn is_held_move(said: &str) -> bool {
    said.contains("failed to move")
        && ["Permission denied", "Invalid argument", "Device or resource busy"]
            .iter()
            .any(|reason| said.contains(reason))
}

/// The first line of what git said, without the spaces around it.
fn first_line(said: &str) -> &str {
    said.lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or_default()
}

/// How long to wait before what is left of a folder is deleted again, after so many failures; none once it has been
/// tried enough. The waits grow: 100 ms, then 200 ms, and so on.
#[must_use]
pub fn delete_wait(failures: usize) -> Option<u64> {
    (failures < DELETE_TRIES).then(|| 100 * u64::try_from(failures).unwrap_or(u64::MAX))
}

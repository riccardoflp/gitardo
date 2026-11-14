//! Getting past the folder of a worktree that Windows holds for a moment while git moves or removes it, as
//! `worktree.recover-held-folder` decides: the move tried again, or what is left of a folder git forgot deleted.

use std::path::{Path, PathBuf};

use git::Git;
use git::read::worktree::{parse_worktrees, worktrees_arguments};
use model::contract::Step;
use model::worktree::{HeldInput, Recovery, delete_wait, recover_held_folder};

/// What the recovery does, given by the runner of the steps so that it can be tried without git or a disk.
pub(crate) struct Attempts<'a> {
    /// Whether it runs on Windows.
    pub(crate) is_windows: bool,
    /// Runs the step again: what git said when it fails.
    pub(crate) again: &'a mut dyn FnMut() -> Result<(), String>,
    /// Whether git still lists a worktree, by its folder.
    pub(crate) is_listed: &'a mut dyn FnMut(&Path) -> bool,
    /// Deletes what is left of a folder, links and not what they lead to.
    pub(crate) delete: &'a mut dyn FnMut(&Path) -> Result<(), String>,
    /// Waits so many milliseconds.
    pub(crate) wait: &'a mut dyn FnMut(u64),
    /// Writes a line to the log.
    pub(crate) note: &'a mut dyn FnMut(&Step),
}

/// Gets past a git step that failed, when the recovery says how: `Ok` once it did, or what git said last.
pub(crate) fn recover(arguments: &[String], said: String, attempts: &mut Attempts<'_>) -> Result<(), String> {
    let mut said = said;
    let mut failures = 1;
    let mut is_listed = None;
    loop {
        let input = HeldInput {
            arguments,
            said: &said,
            failures,
            is_windows: attempts.is_windows,
            is_listed,
        };
        match recover_held_folder(&input) {
            Recovery::Fail => return Err(said),
            Recovery::ReadListed => {
                let folder = arguments.last().map_or_default(PathBuf::from);
                is_listed = Some((attempts.is_listed)(&folder));
            }
            Recovery::TryAgain { wait, note } => {
                (attempts.note)(&Step::Warn { words: note });
                (attempts.wait)(wait);
                match (attempts.again)() {
                    Ok(()) => {
                        let words = format!("git {}", arguments.join(" "));
                        (attempts.note)(&Step::Warn { words });
                        return Ok(());
                    }
                    Err(again) => {
                        said = again;
                        failures += 1;
                    }
                }
            }
            Recovery::DeleteLeft { folder, note } => {
                (attempts.note)(&Step::Warn { words: note });
                (attempts.note)(&Step::Delete { path: folder.clone() });
                return delete_left(&folder, attempts);
            }
        }
    }
}

/// Deletes what is left of a folder git forgot, trying again with longer waits while Windows holds it.
fn delete_left(folder: &Path, attempts: &mut Attempts<'_>) -> Result<(), String> {
    let mut failures = 0;
    loop {
        match (attempts.delete)(folder) {
            Ok(()) => return Ok(()),
            Err(said) => {
                failures += 1;
                let Some(wait) = delete_wait(failures) else {
                    return Err(said);
                };
                (attempts.wait)(wait);
            }
        }
    }
}

/// Whether git still lists a worktree, by its folder. When git cannot tell, it is taken as listed: the failure is then
/// the operation's, and nothing is deleted.
pub(crate) fn is_listed(git: &Git, folder: &Path) -> bool {
    let Ok(output) = git.run_read(&worktrees_arguments()) else {
        return true;
    };
    let Ok(worktrees) = parse_worktrees(&output) else {
        return true;
    };
    let folders: Vec<PathBuf> = worktrees.into_iter().map(|worktree| worktree.folder).collect();
    model::worktree::is_listed(&folders, folder)
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests;

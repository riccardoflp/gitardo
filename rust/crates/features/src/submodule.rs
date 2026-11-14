//! Submodules: opening one, bringing it to the commit recorded for it or to the latest of its branch, recording the
//! commit it is on, initializing, deinitializing and removing it, copying its path or URL, adding one, and
//! synchronizing their URLs (`docs/features/13-submodules.md`).
//!
//! A submodule inside another is managed from that one: its commands run as `git -C <parent> …`, with its path from
//! there, so that the log shows the command as it can be typed again from the top of the repository.

mod add;
mod copy;
mod deinitialize;
mod initialize;
mod open;
mod remove;
mod stage_recorded;
mod sync_urls;
mod update_to_latest;
mod update_to_recorded;

pub use add::{BRANCH, FOLDER, URL, add};
pub use copy::{copy_path, copy_url};
pub use deinitialize::{LOCAL_MODIFICATIONS, deinitialize};
pub use initialize::initialize;
pub use open::{OpenCommands, System, open_in_new_window, open_in_terminal, reveal, show_in_gitardo};
pub use remove::{GIT_FOLDER, remove};
pub use stage_recorded::stage_recorded;
pub use sync_urls::sync_urls;
pub use update_to_latest::update_to_latest;
pub use update_to_recorded::update_to_recorded;

use std::ffi::OsString;
use std::path::PathBuf;

use model::contract::{Context, Outcome, Refusal, Step, Target, Text, Undo};
use model::submodule::{Submodule, SubmoduleStatus, status};

/// Why the operations that change only a submodule's own repository are not taken back by Undo.
const IN_ITS_REFLOG: &str = "Not taken back by Undo: the commit it had is in the reflog of the submodule.";

/// Why the operations that change nothing have nothing to take back.
const NOTHING_CHANGES: &str = "Nothing to take back: nothing changes.";

/// The submodule an operation was invoked on, or the refusal when it is not there.
fn target_submodule(context: &Context) -> Result<&Submodule, Box<Outcome>> {
    let Target::Submodule(folder) = &context.target else {
        return Err(refusal("Pick a submodule first."));
    };
    context
        .repository
        .submodules
        .iter()
        .find(|submodule| submodule.folder == *folder)
        .ok_or_else(|| refusal(&format!("{folder} is no longer a submodule here.")))
}

/// The submodule an operation was invoked on, refused when its folder has no repository, with the end of the
/// sentence that says so: `.`, or `: update it first.`
fn initialized<'c>(context: &'c Context, end: &str) -> Result<&'c Submodule, Box<Outcome>> {
    let submodule = target_submodule(context)?;
    if submodule.checkout.is_some() {
        Ok(submodule)
    } else {
        Err(refusal(&format!("{} is not initialized{end}", submodule.folder)))
    }
}

/// A refusal, boxed as the operations return it.
fn refusal(message: &str) -> Box<Outcome> {
    Box::new(Outcome::Refuse(Refusal::saying(message)))
}

/// Whether a submodule is ahead of the recorded commit, behind it, both, or does not have it.
fn is_apart(submodule: &Submodule) -> bool {
    matches!(
        status(submodule),
        SubmoduleStatus::Ahead { .. }
            | SubmoduleStatus::Behind { .. }
            | SubmoduleStatus::Both { .. }
            | SubmoduleStatus::Unknown { .. }
    )
}

/// The folder of a submodule on disk.
fn on_disk(context: &Context, submodule: &Submodule) -> PathBuf {
    submodule
        .folder
        .split('/')
        .fold(context.repository.folder.clone(), |path, part| path.join(part))
}

/// A git step about a submodule, run from the top of the repository: `git <words> -- <path>`, as
/// `git -C <parent> <words> -- <path>` with its path from its parent for a submodule inside another.
fn submodule_step(context: &Context, submodule: &Submodule, words: &[&str]) -> Step {
    let (parent, path) = from_parent(submodule);
    let mut arguments: Vec<OsString> = Vec::new();
    if let Some(parent) = parent {
        arguments.extend(["-C", parent].map(OsString::from));
    }
    arguments.extend(words.iter().map(OsString::from));
    arguments.push(OsString::from("--"));
    arguments.push(OsString::from(literal(path)));
    Step::Git {
        arguments,
        directory: context.repository.folder.clone(),
        input: None,
    }
}

/// The folder of the submodule that has a submodule, if any, and the path of the submodule from it.
fn from_parent(submodule: &Submodule) -> (Option<&str>, &str) {
    match &submodule.parent {
        Some(parent) => {
            let path = submodule
                .folder
                .strip_prefix(parent.as_str())
                .and_then(|rest| rest.strip_prefix('/'))
                .unwrap_or(&submodule.folder);
            (Some(parent.as_str()), path)
        }
        None => (None, &submodule.folder),
    }
}

/// A path as git takes it literally: `:(literal)<path>` when git could read it as a pattern or a magic word (rule R9.2).
fn literal(path: &str) -> String {
    if path.starts_with(':') || path.contains(['*', '?', '[', ']', '\\']) {
        format!(":(literal){path}")
    } else {
        path.to_owned()
    }
}

/// The undo of an operation that changes nothing.
fn nothing_to_undo() -> Undo {
    Undo::Impossible(Text::from(NOTHING_CHANGES))
}

#[cfg(test)]
pub(crate) mod testing {
    //! Repositories with submodules, for the tests of the operations.

    use model::contract::{Context, Target};
    use model::submodule::{Checkout, Distance, Submodule};

    use crate::testing::{commit, context, state};

    /// `lib`, recorded at `commit('1')`, initialized and on that commit on `main`.
    pub fn lib() -> Submodule {
        Submodule {
            name: String::from("lib"),
            folder: String::from("lib"),
            parent: None,
            url: Some(String::from("../lib.git")),
            branch: None,
            recorded: Some(commit('1')),
            staged: None,
            checkout: Some(Checkout {
                commit: Some(commit('1')),
                branch: Some(String::from("main")),
                uncommitted: 0,
                distance: Some(Distance::default()),
            }),
        }
    }

    /// A submodule apart from its recorded commit, at `commit('9')`, on a branch or detached.
    pub fn apart(mut submodule: Submodule, (ahead, behind): (u32, u32), branch: Option<&str>) -> Submodule {
        submodule.checkout = Some(Checkout {
            commit: Some(commit('9')),
            branch: branch.map(str::to_owned),
            uncommitted: 0,
            distance: Some(Distance { ahead, behind }),
        });
        submodule
    }

    /// `vendor/a/b`, a submodule of `vendor/a`, on its recorded commit.
    pub fn nested() -> Submodule {
        Submodule {
            name: String::from("b"),
            folder: String::from("vendor/a/b"),
            parent: Some(String::from("vendor/a")),
            ..lib()
        }
    }

    /// The context of an operation invoked on a submodule, the repository having it.
    pub fn on(submodule: Submodule) -> Context {
        let mut state = state();
        let folder = submodule.folder.clone();
        state.submodules.push(submodule);
        context(state, Target::Submodule(folder))
    }
}

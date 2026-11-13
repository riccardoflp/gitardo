//! What a new worktree gets as steps after `git worktree add`: the links and copies of what git ignores in the
//! checkout it comes from (`worktree-setup.link-and-copy-steps`), then the commands of `worktrees.setupCommands`
//! (`worktree-setup.command-steps`).

use std::path::Path;

use super::{SetupPreview, SetupSettings};
use crate::contract::Step;

/// Steps for a new worktree, with the note the plan says them with.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SetupSteps {
    /// The steps, in order.
    pub steps: Vec<Step>,
    /// What the plan says it will do with them, so that the preview of `commandPreview` says it along with the steps;
    /// none when there are no steps.
    pub note: Option<String>,
    /// What Gitardo says once the plan ran, when the steps leave something undone that the user should know.
    pub said: Option<String>,
}

/// What [`link_and_copy_steps`] looks at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LinkAndCopyInput<'a> {
    /// The patterns in effect.
    pub settings: &'a SetupSettings,
    /// The folder the views show, which the links point to and the copies come from.
    pub source: &'a Path,
    /// The folder of the worktree.
    pub worktree: &'a Path,
    /// What the patterns take from the source for that worktree, with what it has from its commit.
    pub take: &'a SetupPreview,
}

/// The links, then the copies, a worktree gets, and the note that says them. A folder that holds the worktree is not
/// linked nor copied, and what the worktree has from its commit is not copied: the log says so of each. With no
/// pattern in effect there are no steps and no note.
///
/// Feature: worktree-setup.link-and-copy-steps
#[must_use]
pub fn link_and_copy_steps(input: &LinkAndCopyInput<'_>) -> SetupSteps {
    let links = &input.settings.link_folders.values;
    let copies = &input.settings.copy_files.values;
    if links.is_empty() && copies.is_empty() {
        return SetupSteps::default();
    }
    let at = |root: &Path, path: &str| {
        path.split('/')
            .fold(root.to_path_buf(), |folder, part| folder.join(part))
    };
    let holds_worktree = |path: &str| input.worktree.starts_with(at(input.source, path));
    let mut steps = Vec::new();
    for folder in &input.take.linked {
        steps.push(if holds_worktree(folder) {
            Step::Warn {
                words: format!("{folder} holds the worktree, so it is not linked."),
            }
        } else {
            Step::Link {
                from: at(input.worktree, folder),
                to: at(input.source, folder),
            }
        });
    }
    for path in &input.take.copied {
        steps.push(if holds_worktree(path) {
            Step::Warn {
                words: format!("{path} holds the worktree, so it is not copied."),
            }
        } else {
            Step::Copy {
                from: at(input.source, path),
                to: at(input.worktree, path),
            }
        });
    }
    steps.extend(input.take.already.iter().map(|path| Step::Warn {
        words: format!("{path} is in the worktree already."),
    }));
    let source = input.source.display();
    let note = match (links.is_empty(), copies.is_empty()) {
        (false, false) => format!(
            "Then Gitardo will link the folders matching {}, and copy what matches {}, from {source}, where git \
             ignores them.",
            links.join(", "),
            copies.join(", ")
        ),
        (false, true) => format!(
            "Then Gitardo will link the folders matching {}, from {source}, where git ignores them.",
            links.join(", ")
        ),
        _ => format!(
            "Then Gitardo will copy what matches {}, from {source}, where git ignores them.",
            copies.join(", ")
        ),
    };
    SetupSteps {
        steps,
        note: Some(note),
        said: None,
    }
}

/// What [`command_steps`] looks at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandStepsInput<'a> {
    /// The commands of `worktrees.setupCommands` in effect.
    pub commands: &'a [String],
    /// The folder of the worktree, where they run.
    pub worktree: &'a Path,
    /// Whether the folder of the window is trusted: only then do they run.
    pub trust: Trust,
    /// Whether the worktree gets links or copies before them, which the note of the commands follows.
    pub after: NoteStart,
}

/// Whether the user has trusted the folder of the window, so that the commands it asks to run are run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Trust {
    /// Not trusted: nothing it asks to run is run.
    #[default]
    NotTrusted,
    /// Trusted.
    Trusted,
}

/// How the note of the commands starts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoteStart {
    /// After the note of the links and copies: **After that it will run…**.
    AfterLinks,
    /// On its own: **Then Gitardo will run…**.
    Alone,
}

/// The commands a new worktree runs once it is created, linked and copied, one `run` step each in the worktree; in a
/// folder that is not trusted none runs, and the log and Gitardo say why.
///
/// Feature: worktree-setup.command-steps
#[must_use]
pub fn command_steps(input: &CommandStepsInput<'_>) -> SetupSteps {
    if input.commands.is_empty() {
        return SetupSteps::default();
    }
    if input.trust == Trust::NotTrusted {
        return SetupSteps {
            steps: vec![Step::Warn {
                words: String::from("The commands of worktrees.setupCommands did not run: this folder is not trusted."),
            }],
            note: None,
            said: Some(format!(
                "The commands for new worktrees did not run in {}: this folder is not trusted.",
                input.worktree.display()
            )),
        };
    }
    let steps = input
        .commands
        .iter()
        .map(|command| Step::Run {
            command: command.clone(),
            directory: input.worktree.to_path_buf(),
        })
        .collect();
    let start = match input.after {
        NoteStart::AfterLinks => "After that it will run",
        NoteStart::Alone => "Then Gitardo will run",
    };
    SetupSteps {
        steps,
        note: Some(format!(
            "{start} {}, in Tasks, in the worktree.",
            input.commands.join(", then ")
        )),
        said: None,
    }
}

/// What Gitardo says once the links and copies are all done, when some failed: the step that failed, or how many.
#[must_use]
pub fn failed_setup_steps(failed: &[String]) -> Option<String> {
    match failed {
        [] => None,
        [one] => Some(format!("Gitardo: `{one}` failed: the log says why.")),
        many => Some(format!("{} folders or files failed: the log says why.", many.len())),
    }
}

#[cfg(test)]
mod tests;

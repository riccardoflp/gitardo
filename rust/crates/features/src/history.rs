//! What the operations that bring commits onto what is checked out share: the operation under way that stops them,
//! what is checked out in words, the commit or reference they bring, and what a prediction of conflicts makes of their
//! plan.

use model::CommitId;
use model::branch::{RefKind, RefNames, ref_argument};
use model::contract::{Context, Danger, Outcome, Refusal, Target, Text};
use model::state::Operation;

/// The refusal while a merge, a rebase, a cherry-pick, a revert or a patch is under way: *Finish or abort the
/// `<operation>` first.*
pub(crate) fn refuse_under_way(context: &Context) -> Option<Outcome> {
    let operation = context.repository.operation?;
    Some(Outcome::Refuse(Refusal::saying(&format!(
        "Finish or abort the {} first.",
        operation_name(operation)
    ))))
}

/// The name of an operation stopped half way, as a refusal says it.
pub(crate) fn operation_name(operation: Operation) -> &'static str {
    match operation {
        Operation::Merge => "merge",
        Operation::Rebase => "rebase",
        Operation::CherryPick => "cherry-pick",
        Operation::Revert => "revert",
        Operation::Patch => "patch",
    }
}

/// What is checked out, as the words of an operation say it: the branch, or `HEAD` when detached.
pub(crate) fn current(context: &Context) -> &str {
    context.repository.checked_out_branch().unwrap_or("HEAD")
}

/// The commit an operation was invoked on, or the refusal when it was invoked on something else.
pub(crate) fn target_commit(context: &Context) -> Result<&CommitId, Box<Outcome>> {
    match &context.target {
        Target::Commit(commit) => Ok(commit),
        _ => Err(Box::new(Outcome::Refuse(Refusal::saying("Choose a commit first.")))),
    }
}

/// A commit or a reference brought onto what is checked out: as git is given it, as the words say it, and the commit
/// it stands for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Brought {
    /// As git is given it: the full hash, or the name of a branch, in full when a tag or a remote branch has it too.
    pub(crate) argument: String,
    /// As the words say it: the short hash, or the short name.
    pub(crate) name: String,
    /// The commit.
    pub(crate) commit: CommitId,
}

/// The commit, the local branch or the remote branch an operation was invoked on, or the refusal when it is none of
/// them, or no longer there.
pub(crate) fn brought(context: &Context) -> Result<Brought, Box<Outcome>> {
    let repository = &context.repository;
    let names = RefNames::of(repository);
    let refuse = |message: String| Box::new(Outcome::Refuse(Refusal::saying(&message)));
    match &context.target {
        Target::Commit(commit) => Ok(Brought {
            argument: commit.to_string(),
            name: commit.short().to_owned(),
            commit: commit.clone(),
        }),
        Target::Branch(name) => {
            let branch = repository
                .branches
                .iter()
                .find(|branch| branch.name == *name)
                .ok_or_else(|| refuse(format!("There is no branch {name}.")))?;
            Ok(Brought {
                argument: ref_argument(RefKind::Branch, name, &names),
                name: name.clone(),
                commit: branch.commit.clone(),
            })
        }
        Target::RemoteBranch(name) => {
            let branch = repository
                .remote_branches
                .iter()
                .find(|branch| branch.name == *name)
                .ok_or_else(|| refuse(format!("There is no remote branch {name}.")))?;
            Ok(Brought {
                argument: ref_argument(RefKind::RemoteBranch, name, &names),
                name: name.clone(),
                commit: branch.commit.clone(),
            })
        }
        _ => Err(refuse(String::from("Choose a commit or a branch first."))),
    }
}

/// How many paths the sentence of a prediction lists before it says how many more there are.
const PATHS_SHOWN: usize = 15;

/// The danger of a plan that stops on the conflicts predicted: none when no conflict is expected or git cannot tell,
/// and a confirmation naming them otherwise, after the note given first (`merge.read-predicted-conflicts`).
pub(crate) fn conflicts_danger(note: Option<&str>, conflicts: Option<&[String]>) -> Danger {
    let paths = conflicts.unwrap_or_default();
    if note.is_none() && paths.is_empty() {
        return Danger::None;
    }
    let said = describe_conflicts(paths);
    Danger::Confirm(Text::from(match note {
        Some(note) => format!("{note}\n\n{said}"),
        None => said,
    }))
}

/// *No conflicts expected.*, or *This will stop with conflicts in N files:* and the paths one per line, at most
/// fifteen, then *…and N more*.
fn describe_conflicts(paths: &[String]) -> String {
    if paths.is_empty() {
        return String::from("No conflicts expected.");
    }
    let files = if paths.len() == 1 {
        String::from("1 file")
    } else {
        format!("{} files", paths.len())
    };
    let mut lines = vec![format!("This will stop with conflicts in {files}:")];
    lines.extend(paths.iter().take(PATHS_SHOWN).cloned());
    if paths.len() > PATHS_SHOWN {
        lines.push(format!("\u{2026}and {} more", paths.len() - PATHS_SHOWN));
    }
    lines.join("\n")
}

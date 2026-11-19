//! Rebasing: the branch checked out or another one onto a commit, the interactive rebase of the commits before `HEAD`,
//! and going on once it stopped to edit a commit (`docs/features/07-history-rewriting.md`). The squash, the split and
//! the fixup are interactive rebases too, and build their plans here.

mod branch_onto;
mod continue_edit;
mod interactive;
mod onto;
pub(crate) mod stack;

pub use branch_onto::{BRANCH, branch_onto, dropped};
pub use continue_edit::continue_edit;
pub use interactive::{TODO, interactive};
pub use onto::onto;

use std::ffi::OsString;
use std::path::PathBuf;

use model::CommitId;
use model::contract::{AnswerKey, Answers, Context, Danger, FollowUp, Outcome, Plan, Refusal, Step, Text, Undo};
use model::rebase::{LineToHead, TodoStep, build_todo_list};
use model::state::Operation;

/// The line of the commit a rewrite starts at, up to `HEAD`.
pub const LINE: AnswerKey = AnswerKey("line");

/// The remote branches that have the commit a rewrite changes.
pub const PUSHED: AnswerKey = AnswerKey("pushed");

/// The folder, in the git folder of the worktree, where an interactive rebase finds its todo list and the new
/// messages, from its start until it ends.
pub const FILES_FOLDER: &str = "gitardo-rebase";

/// The refusal while an operation is stopped half way, which a rewrite would get in the way of, or while a bisect is
/// under way, which would then test commits that are no longer there.
pub(crate) fn refuse_under_way(context: &Context) -> Option<Outcome> {
    let Some(operation) = context.repository.operation else {
        return context
            .repository
            .bisect
            .is_some()
            .then(|| refuse(crate::bisect::END_FIRST));
    };
    let name = match operation {
        Operation::Merge => "merge",
        Operation::Rebase => "rebase",
        Operation::CherryPick => "cherry-pick",
        Operation::Revert => "revert",
        Operation::Patch => "patch",
    };
    Some(refuse(&format!("Finish or abort the {name} first.")))
}

/// A refusal that offers nothing else.
pub(crate) fn refuse(message: &str) -> Outcome {
    Outcome::Refuse(Refusal::saying(message))
}

/// The branch checked out, as titles name it: `HEAD` when detached.
pub(crate) fn current_name(context: &Context) -> &str {
    context.repository.checked_out_branch().unwrap_or("HEAD")
}

/// Whether there are changes `--autostash` sets aside: staged, or not staged in tracked files.
pub(crate) fn has_changes_to_set_aside(context: &Context) -> bool {
    let working_tree = &context.repository.working_tree;
    !(working_tree.staged.is_empty() && working_tree.unstaged.is_empty())
}

/// The folder of the files of an interactive rebase in this worktree.
pub(crate) fn files_folder(context: &Context) -> PathBuf {
    context.repository.git_folder.join(FILES_FOLDER)
}

/// A rewrite of the commits before `HEAD` by an interactive rebase, as `rebase.interactive` runs it.
pub(crate) struct Rewrite<'a> {
    /// The steps, oldest first.
    pub steps: &'a [TodoStep<'a>],
    /// The commit the rebase starts after; none from the root.
    pub base: Option<&'a CommitId>,
    /// Whether what is not committed is set aside and put back (`--autostash`).
    pub sets_aside: bool,
    /// The title of the plan.
    pub title: String,
    /// What the confirmation says.
    pub note: String,
    /// What follows it, besides the files deleted once the rebase ended.
    pub after: Vec<FollowUp>,
}

/// The plan of an interactive rebase: the new messages and the todo list written to the folder of the files of the
/// rebase, then `git rebase -i` with git's editor of the todo list copying the prepared one over its own. The folder is
/// deleted once the rebase ended, and kept while it is stopped. The other branches on the commits made again move
/// with it by the `update-ref` lines of the list, as `rebase.update-refs` decides, which the confirmation says.
///
/// # Errors
///
/// The read to make first, of the version of git or of the branches of the stack; or the refusal of
/// `rebase.build-todo-list`.
pub(crate) fn rewrite_plan(context: &Context, answers: &Answers, rewrite: Rewrite<'_>) -> Result<Plan, Box<Outcome>> {
    let (decided, moving) = stack::decide_todo(context, answers, rewrite.base)?;
    let folder = files_folder(context);
    let written = folder.to_string_lossy().replace('\\', "/");
    let list =
        build_todo_list(rewrite.steps, &written, &moving).map_err(|refusal| Box::new(refuse(refusal.as_str())))?;
    let mut steps: Vec<Step> = list
        .files
        .into_iter()
        .map(|(name, contents)| Step::Write {
            path: folder.join(name),
            contents: contents.into_bytes(),
        })
        .collect();
    steps.push(Step::Write {
        path: folder.join("todo"),
        contents: list.text.into_bytes(),
    });
    let mut arguments: Vec<OsString> = vec![
        OsString::from("-c"),
        OsString::from(format!("sequence.editor=cp \"{written}/todo\"")),
        OsString::from("-c"),
        OsString::from("core.editor=true"),
        OsString::from("rebase"),
        OsString::from("-i"),
    ];
    if rewrite.sets_aside {
        arguments.push(OsString::from("--autostash"));
    }
    arguments.push(OsString::from(
        rewrite.base.map_or_else(|| String::from("--root"), ToString::to_string),
    ));
    steps.push(Step::git(context.repository.folder.clone(), arguments));
    let mut after = rewrite.after;
    after.push(FollowUp::DeleteWhenRebaseEnded(folder.clone()));
    after.extend(crate::range_diff::see_what_changed_here(context));
    let plan = Plan {
        title: Text::from(rewrite.title),
        steps,
        danger: Danger::Confirm(Text::from(rewrite.note)),
        undo: Undo::ByReflog,
        after,
        on_failure: vec![FollowUp::DeleteWhenRebaseEnded(folder)],
    };
    Ok(stack::with_todo_stack(plan, answers, &decided))
}

/// `git -c core.editor=true rebase --continue`, which keeps the messages git prepared, then the files of the rebase
/// deleted once it ended.
pub(crate) fn continue_plan(context: &Context, title: String, mut steps: Vec<Step>) -> Plan {
    steps.push(Step::git(
        context.repository.folder.clone(),
        ["-c", "core.editor=true", "rebase", "--continue"],
    ));
    let folder = files_folder(context);
    Plan {
        title: Text::from(title),
        steps,
        danger: Danger::None,
        undo: Undo::ByReflog,
        after: vec![FollowUp::DeleteWhenRebaseEnded(folder.clone())],
        on_failure: vec![FollowUp::DeleteWhenRebaseEnded(folder)],
    }
}

/// The note of a rewrite, with the note of `rebase.read-pushed` first when the commit is pushed, and the sentence about
/// the changes set aside last when there are some.
pub(crate) fn rewrite_note(context: &Context, pushed: Option<Text>, middle: &str, set_aside: &str) -> String {
    let mut parts: Vec<String> = pushed.into_iter().map(|note| note.to_string()).collect();
    parts.push(middle.to_owned());
    if has_changes_to_set_aside(context) {
        parts.push(set_aside.to_owned());
    }
    parts.join(" ")
}

/// The line of a commit read under [`LINE`], or the read to make first.
pub(crate) fn line<'a>(
    answers: &'a model::contract::Answers,
    commit: &CommitId,
) -> Result<&'a LineToHead, Box<Outcome>> {
    answers.line(LINE).ok_or_else(|| {
        Box::new(Outcome::Read(
            LINE,
            model::contract::Query::Line { commit: commit.clone() },
        ))
    })
}

/// The line read under [`LINE`] of the oldest of some commits: read for the first, then again for one it does not
/// have, which is older or not on the line, until it has them all or there is none. The read to make next otherwise.
pub(crate) fn line_holding<'a>(
    answers: &'a model::contract::Answers,
    commits: &[CommitId],
) -> Result<&'a LineToHead, Box<Outcome>> {
    let Some(first) = commits.first() else {
        return Err(Box::new(refuse("Choose the commits first.")));
    };
    let read = line(answers, first)?;
    let Some(line) = read.commits.as_deref() else {
        return Ok(read);
    };
    match commits
        .iter()
        .find(|commit| !line.iter().any(|known| known.id == **commit))
    {
        Some(older) => Err(Box::new(Outcome::Read(
            LINE,
            model::contract::Query::Line { commit: older.clone() },
        ))),
        None => Ok(read),
    }
}

/// The remote branches that have a commit, read under [`PUSHED`], or the read to make first.
pub(crate) fn pushed<'a>(
    answers: &'a model::contract::Answers,
    commit: &CommitId,
) -> Result<&'a [String], Box<Outcome>> {
    answers.names(PUSHED).ok_or_else(|| {
        Box::new(Outcome::Read(
            PUSHED,
            model::contract::Query::Pushed { commit: commit.clone() },
        ))
    })
}

#[cfg(test)]
pub(crate) mod testing;

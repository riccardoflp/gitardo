//! `split.commit`: takes the changes of a commit out of it into the working tree, unstaged.

use std::fmt::Write as _;

use model::CommitId;
use model::contract::{Answers, Context, Danger, FollowUp, Outcome, Plan, Step, Target, Text, Undo, UndoRecord};
use model::rebase::{LineCommit, TodoAction, TodoStep, commits_after, pushed_note, subject_of};

use crate::rebase::{Rewrite, line, pushed, refuse, refuse_under_way, rewrite_note, rewrite_plan};

/// Splits the commit it was invoked on. The commit checked out is reset away with `git reset --mixed HEAD~`, its
/// changes left unstaged; an older one is stopped at by an interactive rebase that marks it **Edit**, then reset away
/// the same way, with `gitardo-split` written in `rebase-merge` so that the banner and **Continue** know it is a split.
/// The message of the commit split goes to the commit box.
///
/// Feature: split.commit
#[must_use]
pub fn commit(context: &Context, answers: &Answers) -> Outcome {
    let Target::Commit(commit) = &context.target else {
        return refuse("Choose a commit first.");
    };
    if let Some(refusal) = refuse_under_way(context) {
        return refusal;
    }
    let read = match line(answers, commit) {
        Ok(read) => read,
        Err(outcome) => return *outcome,
    };
    match read.parents.len() {
        0 => {
            return refuse(&format!(
                "{} is the first commit, with nothing before it to take its changes back to: it cannot be split.",
                commit.short()
            ));
        }
        1 => {}
        _ => return refuse(&format!("{} is a merge: it cannot be split.", commit.short())),
    }
    let Some((split, after)) = read.commits.as_deref().and_then(<[LineCommit]>::split_first) else {
        return refuse(&format!(
            "{} is not on the line of commits before HEAD without merges: it cannot be split by a rebase from here.",
            commit.short()
        ));
    };
    let names = match pushed(answers, commit) {
        Ok(names) => names,
        Err(read) => return *read,
    };
    let pushed = pushed_note(commit, names);
    let filled = FollowUp::FillCommitMessage(split.message.trim().to_owned());
    if after.is_empty() {
        return Outcome::Run(split_last(context, split, pushed, filled));
    }
    match split_older(context, read.base(), (split, after), pushed, filled) {
        Ok(plan) => Outcome::Run(plan),
        Err(refusal) => refuse(refusal.as_str()),
    }
}

/// The split of an older commit: an interactive rebase that stops at it, marked **Edit**, then `git reset --mixed
/// HEAD~` and `gitardo-split` written in `rebase-merge`.
fn split_older(
    context: &Context,
    base: Option<&CommitId>,
    (split, after): (&LineCommit, &[LineCommit]),
    pushed: Option<Text>,
    filled: FollowUp,
) -> Result<Plan, Text> {
    let mut steps = vec![TodoStep {
        action: TodoAction::Edit,
        commit: split.id.as_str(),
        reword: None,
    }];
    steps.extend(after.iter().map(|later| TodoStep {
        action: TodoAction::Pick,
        commit: later.id.as_str(),
        reword: None,
    }));
    let mut middle = format!(
        "The rebase stops at {} with its changes taken out of it, unstaged: commit them in as many commits as you \
         like, then Continue, and ",
        split.id.short()
    );
    let _ = write!(middle, "{} made again on top.", commits_after(after.len(), "it"));
    let rewrite = Rewrite {
        steps: &steps,
        base,
        sets_aside: true,
        title: format!("Split {}", split.id.short()),
        note: rewrite_note(
            context,
            pushed,
            &middle,
            "Changes that are not committed are set aside until the end.",
        ),
        after: vec![filled],
    };
    let mut plan = rewrite_plan(context, rewrite)?;
    let folder = context.repository.folder.clone();
    plan.steps.push(Step::git(folder, ["reset", "--mixed", "HEAD~"]));
    plan.steps.push(Step::Write {
        path: context
            .repository
            .git_folder
            .join("rebase-merge")
            .join(super::SPLIT_FILE),
        contents: format!("{}\n", split.id).into_bytes(),
    });
    Ok(plan)
}

/// The split of the commit checked out: `git reset --mixed HEAD~`, which the journal takes back with
/// `git reset --mixed <commit>`, since the files it added are untracked now.
fn split_last(context: &Context, split: &LineCommit, pushed: Option<Text>, filled: FollowUp) -> Plan {
    let short = split.id.short();
    let middle = format!(
        "The changes of {short} go back into the working tree, unstaged, and the branch goes back to the commit \
         before it: commit them in as many commits as you like."
    );
    let note = rewrite_note(
        context,
        pushed,
        &middle,
        "The changes that are not committed stay, among them.",
    );
    Plan {
        title: Text::from(format!("Split {short}")),
        steps: vec![Step::git(
            context.repository.folder.clone(),
            ["reset", "--mixed", "HEAD~"],
        )],
        danger: Danger::Confirm(Text::from(note)),
        undo: Undo::Journal(UndoRecord::SplitLast {
            commit: split.id.clone(),
        }),
        after: vec![
            filled,
            FollowUp::Say(Text::from(format!(
                "The changes of \"{}\" are in the working tree, unstaged: commit them in as many commits as you like.",
                subject_of(&split.message)
            ))),
        ],
        on_failure: Vec::new(),
    }
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests;

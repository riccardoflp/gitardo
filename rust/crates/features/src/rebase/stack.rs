//! The other branches of a stack in the rewrites (`rebase.update-refs`): read what decides it, then put the option or
//! the branches of the `update-ref` lines, the confirmation and what is said after into the plan.

use model::CommitId;
use model::contract::{
    AnswerKey, Answers, Context, Danger, FollowUp, MovedBranch, Outcome, Plan, Query, Text, Undo, UndoRecord,
};
use model::rebase::{StackedBranch, UpdateRefs, UpdateRefsInput, update_refs};

/// The version of git, read once for each engine.
pub const GIT_VERSION: AnswerKey = AnswerKey("git-version");

/// The branches on the commits the rebase makes again.
pub const STACKED: AnswerKey = AnswerKey("stacked");

/// What the rebase from `base` of `tip` does with the other branches of the stack, once both are read; the read to
/// make first otherwise. `base` is none for a rebase from the root.
pub(crate) fn decide(
    context: &Context,
    answers: &Answers,
    (base, tip): (Option<&str>, &str),
) -> Result<UpdateRefs, Box<Outcome>> {
    let Some(version) = answers.git_version(GIT_VERSION) else {
        return Err(Box::new(Outcome::Read(GIT_VERSION, Query::GitVersion)));
    };
    let Some(stacked) = answers.stacked(STACKED) else {
        return Err(Box::new(Outcome::Read(
            STACKED,
            Query::StackedBranches {
                base: base.map(str::to_owned),
                tip: tip.to_owned(),
            },
        )));
    };
    Ok(update_refs(&UpdateRefsInput {
        version,
        setting: context.settings.rebase_update_refs,
        stacked,
        worktree: &context.repository.folder,
    }))
}

/// The tip of a rewrite of what is checked out: the branch, or the commit of a detached `HEAD`.
pub(crate) fn checked_out_tip(context: &Context) -> String {
    match (context.repository.checked_out_branch(), &context.repository.head.commit) {
        (Some(branch), _) => branch.to_owned(),
        (None, commit) => commit.as_ref().map(ToString::to_string).unwrap_or_default(),
    }
}

/// What a rewrite of the commits after `base` up to what is checked out, whose todo list is Gitardo's, does with the
/// other branches of the stack, and the branches that move, each with its commit, for its `update-ref` lines; the read
/// to make first otherwise.
pub(crate) fn decide_todo(
    context: &Context,
    answers: &Answers,
    base: Option<&CommitId>,
) -> Result<(UpdateRefs, Vec<StackedBranch>), Box<Outcome>> {
    let base = base.map(ToString::to_string);
    let decided = decide(context, answers, (base.as_deref(), &checked_out_tip(context)))?;
    let moving = moving(answers, &decided);
    Ok((decided, moving))
}

/// The branches that move, each with the commit it is on, as the branches of the stack were read.
fn moving(answers: &Answers, decided: &UpdateRefs) -> Vec<StackedBranch> {
    answers
        .stacked(STACKED)
        .map(|stacked| {
            stacked
                .branches
                .iter()
                .filter(|branch| decided.moving.contains(&branch.name))
                .cloned()
                .collect()
        })
        .unwrap_or_default()
}

/// Undo of a rewrite taken back by the reflog that moves other branches: the journal keeps each with the commit it
/// was on, so that one Undo puts them back too (`undo.moved-branches`).
fn with_moved_undo(mut plan: Plan, answers: &Answers, decided: &UpdateRefs) -> Plan {
    let moved: Vec<MovedBranch> = moving(answers, decided)
        .into_iter()
        .map(|branch| MovedBranch {
            name: branch.name,
            before: branch.commit,
            after: None,
        })
        .collect();
    if plan.undo == Undo::ByReflog && !moved.is_empty() {
        plan.undo = Undo::Journal(UndoRecord::Rewrite { moved });
    }
    plan
}

/// The plan with the branches of the stack: asked first when one stays on the old commits, with the note; and, once
/// it ran, what is said names those that moved: "Rebased `p3` onto `main`; `p1` and `p2` moved with it."
pub(crate) fn with_stack(
    mut plan: Plan,
    answers: &Answers,
    decided: &UpdateRefs,
    (branch, onto): (&str, &str),
) -> Plan {
    if let Some(note) = &decided.note
        && !decided.staying.is_empty()
        && matches!(plan.danger, Danger::None | Danger::Agreed)
    {
        plan.danger = Danger::Confirm(Text::from(format!("{}. {note}", plan.title)));
    }
    if let Some(names) = moved(decided) {
        plan.after.push(FollowUp::Say(Text::from(format!(
            "Rebased `{branch}` onto `{onto}`; {names} with it."
        ))));
    }
    with_moved_undo(plan, answers, decided)
}

/// The rewrite whose todo list is Gitardo's with the branches of the stack: its confirmation ends with the note, and,
/// once it ran, what is said names those that moved: "`p1` and `p2` moved with the rewrite."
pub(crate) fn with_todo_stack(mut plan: Plan, answers: &Answers, decided: &UpdateRefs) -> Plan {
    if let Some(note) = decided.note.as_deref().filter(|note| !note.is_empty()) {
        plan.danger = match plan.danger {
            Danger::Confirm(text) => Danger::Confirm(Text::from(format!("{text}\n\n{note}"))),
            Danger::Destructive(text) => Danger::Destructive(Text::from(format!("{text}\n\n{note}"))),
            Danger::None | Danger::Agreed => Danger::Confirm(Text::from(format!("{}. {note}", plan.title))),
        };
    }
    if let Some(names) = moved(decided) {
        plan.after
            .push(FollowUp::Say(Text::from(format!("{names} with the rewrite."))));
    }
    with_moved_undo(plan, answers, decided)
}

/// "`p1` moved", "`p1` and `p2` moved": the branches that move, none when no branch does.
fn moved(decided: &UpdateRefs) -> Option<String> {
    let names: Vec<String> = decided.moving.iter().map(|name| format!("`{name}`")).collect();
    match names.as_slice() {
        [] => None,
        [one] => Some(format!("{one} moved")),
        [rest @ .., last] => Some(format!("{} and {last} moved", rest.join(", "))),
    }
}

/// The answers of the reads with a recent git and no other branch on the commits made again, for the tests.
#[cfg(test)]
pub(crate) fn no_stack(answers: Answers) -> Answers {
    answers
        .with(
            GIT_VERSION,
            model::contract::Answer::GitVersion(Some(model::rebase::GitVersion {
                major: 2,
                minor: 53,
                patch: 0,
                said: String::from("2.53.0"),
            })),
        )
        .with(
            STACKED,
            model::contract::Answer::Stacked(model::rebase::StackedBranches::default()),
        )
}

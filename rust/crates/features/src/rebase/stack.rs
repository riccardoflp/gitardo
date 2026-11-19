//! The other branches of a stack in the rebases that let git write their todo list (`rebase.update-refs`): read what
//! decides it, then put the option, the confirmation and what is said after into the plan.

use model::contract::{AnswerKey, Answers, Context, Danger, FollowUp, Outcome, Plan, Query, Text};
use model::rebase::{UpdateRefs, UpdateRefsInput, update_refs};

/// The version of git, read once for each engine.
pub const GIT_VERSION: AnswerKey = AnswerKey("git-version");

/// The branches on the commits the rebase makes again.
pub const STACKED: AnswerKey = AnswerKey("stacked");

/// What the rebase from `base` of `tip` does with the other branches of the stack, once both are read; the read to
/// make first otherwise.
pub(crate) fn decide(
    context: &Context,
    answers: &Answers,
    (base, tip): (&str, &str),
) -> Result<UpdateRefs, Box<Outcome>> {
    let Some(version) = answers.git_version(GIT_VERSION) else {
        return Err(Box::new(Outcome::Read(GIT_VERSION, Query::GitVersion)));
    };
    let Some(stacked) = answers.stacked(STACKED) else {
        return Err(Box::new(Outcome::Read(
            STACKED,
            Query::StackedBranches {
                base: Some(base.to_owned()),
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

/// The plan with the branches of the stack: asked first when one stays on the old commits, with the note; and, once
/// it ran, what is said names those that moved: "Rebased `p3` onto `main`; `p1` and `p2` moved with it."
pub(crate) fn with_stack(mut plan: Plan, decided: &UpdateRefs, (branch, onto): (&str, &str)) -> Plan {
    if let Some(note) = &decided.note
        && !decided.staying.is_empty()
        && matches!(plan.danger, Danger::None | Danger::Agreed)
    {
        plan.danger = Danger::Confirm(Text::from(format!("{}. {note}", plan.title)));
    }
    if !decided.moving.is_empty() {
        let names: Vec<String> = decided.moving.iter().map(|name| format!("`{name}`")).collect();
        let names = match names.as_slice() {
            [one] => format!("{one} moved"),
            [rest @ .., last] => format!("{} and {last} moved", rest.join(", ")),
            [] => String::new(),
        };
        plan.after.push(FollowUp::Say(Text::from(format!(
            "Rebased `{branch}` onto `{onto}`; {names} with it."
        ))));
    }
    plan
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

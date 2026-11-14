//! Bisect: finding the commit that broke something, by marking commits good or bad until git names the first bad one
//! (`docs/features/11-history-and-blame.md`).

mod answer;
mod end;
mod mark;
mod start;

pub use answer::{ANSWER, answer};
pub use end::end;
pub use mark::mark;
pub use start::start;

use model::CommitId;
use model::bisect::Bisect;
use model::contract::{
    AnswerKey, Answers, Choice, Context, Danger, FollowUp, Outcome, Plan, Question, QuestionKind, Refusal, Step,
    Target, Text, Undo,
};
use model::state::Operation;

/// Whether a commit is marked good or bad: `good` or `bad`, as the entry of the menu chosen says.
pub const MARK: AnswerKey = AnswerKey("mark");

/// What Undo says while a bisect is under way.
pub const END_FIRST: &str = "End the bisect first.";

/// What is said when no bisect is under way.
const NO_BISECT: &str = "No bisect is in progress.";

/// The commit a feature was invoked on, or the refusal when it was not invoked on one.
fn target_commit(context: &Context) -> Result<&CommitId, Box<Outcome>> {
    match &context.target {
        Target::Commit(commit) => Ok(commit),
        _ => Err(Box::new(Outcome::Refuse(Refusal::saying("Pick a commit first.")))),
    }
}

/// The refusal while an operation is stopped half way: a bisect is started or marked only when none is.
pub(crate) fn refuse_while_operation(context: &Context) -> Option<Outcome> {
    let operation = context.repository.operation?;
    let name = match operation {
        Operation::Merge => "merge",
        Operation::Rebase => "rebase",
        Operation::CherryPick => "cherry-pick",
        Operation::Revert => "revert",
        Operation::Patch => "patch",
    };
    Some(Outcome::Refuse(Refusal::saying(&format!(
        "Finish or abort the {name} first."
    ))))
}

/// The bisect under way, or the refusal when there is none.
fn bisect_under_way(context: &Context) -> Result<&Bisect, Box<Outcome>> {
    context
        .repository
        .bisect
        .as_ref()
        .ok_or_else(|| Box::new(Outcome::Refuse(Refusal::saying(NO_BISECT))))
}

/// `good` or `bad`, as given before or picked.
fn good_or_bad(answers: &Answers, title: &str) -> Result<&'static str, Box<Outcome>> {
    match answers.choice(MARK) {
        Some("good") => Ok("good"),
        Some("bad") => Ok("bad"),
        _ => Err(Box::new(Outcome::Ask(Question {
            key: MARK,
            kind: QuestionKind::Pick {
                choices: ["bad", "good"]
                    .into_iter()
                    .map(|mark| Choice {
                        identifier: mark.to_owned(),
                        label: Text::from(format!("This commit is {mark}")),
                        description: None,
                    })
                    .collect(),
            },
            title: Text::from(title),
            detail: None,
            default: Some(model::contract::Answer::Choice(String::from("bad"))),
        }))),
    }
}

/// A plan of `git bisect` commands, which Undo does not take back while the bisect is under way.
fn bisect_plan(context: &Context, title: String, commands: &[&[&str]], after: Vec<FollowUp>) -> Plan {
    Plan {
        title: Text::from(title),
        steps: commands
            .iter()
            .map(|words| Step::git(context.repository.folder.clone(), words.iter().copied()))
            .collect(),
        danger: Danger::None,
        undo: Undo::Impossible(Text::from(END_FIRST)),
        after,
        on_failure: Vec::new(),
    }
}

#[cfg(test)]
pub(crate) mod testing {
    use model::CommitId;
    use model::bisect::Bisect;
    use model::contract::{Context, Target};

    use crate::testing::{commit, context, state};

    /// A repository on `main` at `commit('a')`, with a bisect under way when one is given.
    pub(crate) fn bisecting(bisect: Option<Bisect>, target: Target) -> Context {
        let mut state = state();
        state.bisect = bisect;
        context(state, target)
    }

    /// A bisect with a bad commit and good ones, and how many commits are left.
    pub(crate) fn marked(bad: Option<char>, good: &[char], remaining: Option<u32>) -> Bisect {
        Bisect {
            bad: bad.map(commit),
            good: good.iter().copied().map(commit).collect::<Vec<CommitId>>(),
            skipped: Vec::new(),
            remaining,
        }
    }
}

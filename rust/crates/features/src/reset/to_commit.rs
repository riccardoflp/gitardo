//! `reset.to-commit`: moves the branch checked out (or `HEAD` when detached) to a commit, keeping the changes staged,
//! unstaged, or not at all.

use model::contract::{
    AnswerKey, Answers, Choice, Context, Danger, FollowUp, Outcome, Plan, Question, QuestionKind, Step, Text, Undo,
};

use crate::history::{current, target_commit};

/// How the reset keeps the changes, picked: `soft`, `mixed` or `hard`.
pub const MODE: AnswerKey = AnswerKey("mode");

/// The three ways, by identifier, with their words.
const MODES: [(&str, &str, &str); 3] = [
    ("soft", "Soft", "Keep all changes, staged"),
    ("mixed", "Mixed", "Keep all changes, unstaged"),
    ("hard", "Hard", "Discard all changes"),
];

/// Asks how to keep the changes, with no default, then runs `git reset --soft|--mixed|--hard <hash>`. A hard reset is
/// always shown first with its warning, as it throws away the changes not committed.
///
/// Feature: reset.to-commit
#[must_use]
pub fn to_commit(context: &Context, answers: &Answers) -> Outcome {
    let commit = match target_commit(context) {
        Ok(commit) => commit,
        Err(outcome) => return *outcome,
    };
    let title = format!("Reset {} to {}", current(context), commit.short());
    let Some(mode) = answers
        .choice(MODE)
        .and_then(|picked| MODES.iter().find(|(identifier, _, _)| *identifier == picked))
    else {
        return Outcome::Ask(mode_question(title));
    };
    let (identifier, _, _) = mode;
    let danger = if *identifier == "hard" {
        Danger::Destructive(Text::from(
            "A hard reset throws away all uncommitted changes to tracked files.",
        ))
    } else {
        Danger::None
    };
    Outcome::Run(Plan {
        title: Text::from(title),
        steps: vec![Step::git(
            context.repository.folder.clone(),
            ["reset", &format!("--{identifier}"), commit.as_str()],
        )],
        danger,
        undo: Undo::ByReflog,
        after: vec![FollowUp::SelectHead],
        on_failure: Vec::new(),
    })
}

/// The pick of the three ways, titled as the plan is.
fn mode_question(title: String) -> Question {
    Question {
        key: MODE,
        kind: QuestionKind::Pick {
            choices: MODES
                .iter()
                .map(|(identifier, label, description)| Choice {
                    identifier: (*identifier).to_owned(),
                    label: Text::from(*label),
                    description: Some(Text::from(*description)),
                })
                .collect(),
        },
        title: Text::from(title),
        detail: None,
        default: None,
    }
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::{MODE, to_commit};
    use crate::testing::{commit, context, lines, plan, question, state};
    use model::contract::{Answer, Answers, Danger, Target};

    fn mode(picked: &str) -> Answers {
        Answers::new().with(MODE, Answer::Choice(picked.to_owned()))
    }

    #[test]
    fn reset_to_commit__mixed_runs_git_reset_mixed_with_the_full_hash() {
        let context = context(state(), Target::Commit(commit('b')));
        let asked = question(to_commit(&context, &Answers::new()));
        assert_eq!(asked.title.as_str(), "Reset main to bbbbbbb");
        assert_eq!(asked.default, None);
        let plan = plan(to_commit(&context, &mode("mixed")));
        assert_eq!(lines(&plan), [format!("git reset --mixed {}", commit('b'))]);
        assert_eq!(plan.danger, Danger::None);
    }

    #[test]
    fn reset_to_commit__hard_always_shows_the_command_and_the_warning_first() {
        let context = context(state(), Target::Commit(commit('b')));
        let plan = plan(to_commit(&context, &mode("hard")));
        assert_eq!(
            plan.danger,
            Danger::Destructive(model::contract::Text::from(
                "A hard reset throws away all uncommitted changes to tracked files."
            ))
        );
        assert!(engine_asks(&plan.danger));
    }

    /// Whether the engine shows the plan first with `commandPreview` off: a destructive plan always is.
    fn engine_asks(danger: &Danger) -> bool {
        matches!(danger, Danger::Destructive(_) | Danger::Confirm(_))
    }
}

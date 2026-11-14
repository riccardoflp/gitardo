//! `trust.ask`: whether to trust the folder of the repository, asked before running the commands it asks to run.

use std::path::PathBuf;

use model::contract::{Answer, AnswerKey, Answers, Choice, Context, Outcome, Question, QuestionKind, Text};
use model::trust::Trust;
use model::worktree_setup::SettingSource;

/// The key of the answer: whether to trust the folder.
pub const TRUST: AnswerKey = AnswerKey("trust-folder");
/// The choice that trusts the folder.
const TRUST_IT: &str = "trust";
/// The choice that leaves it not trusted, this time.
const NOT_NOW: &str = "not-now";

/// What a folder asks to run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrustAsked<'a> {
    /// The commands, in the order they would run.
    pub commands: &'a [String],
    /// The settings file they come from.
    pub source: SettingSource,
}

/// Whether what the folder of the repository asks to run may run: the trust the window gives when it trusts the
/// folder or there is nothing to run, otherwise what the user answered to **Run the commands for new worktrees?**,
/// asked first with **Not now** proposed.
///
/// # Errors
///
/// The question, while it is not answered.
///
/// Feature: trust.ask
pub fn ask(context: &Context, answers: &Answers, asked: &TrustAsked<'_>) -> Result<Trust, Box<Outcome>> {
    let trust = context.settings.trust;
    if trust == Trust::Trusted || asked.commands.is_empty() {
        return Ok(trust);
    }
    match answers.choice(TRUST) {
        Some(TRUST_IT) => Ok(Trust::Trusted),
        Some(_) => Ok(Trust::NotTrusted),
        None => Err(Box::new(Outcome::Ask(question(context, asked)))),
    }
}

/// The folder the user has just trusted in answer to [`ask`], which the operation's follow-ups add to the folders
/// trusted; none when it was trusted before, or not now.
#[must_use]
pub fn trusted_by_answer(context: &Context, answers: &Answers) -> Option<PathBuf> {
    (context.settings.trust == Trust::NotTrusted && answers.choice(TRUST) == Some(TRUST_IT))
        .then(|| context.repository.folder.clone())
}

/// **Run the commands for new worktrees?**, naming the folder, where the commands come from and each command.
fn question(context: &Context, asked: &TrustAsked<'_>) -> Question {
    let folder = context.repository.folder.display();
    let place = match asked.source {
        SettingSource::User => "your settings",
        // Only the two settings files set commands.
        SettingSource::Repository | SettingSource::VsCode | SettingSource::Nothing => "its .gitardo/settings.json",
    };
    let commands: Vec<String> = asked.commands.iter().map(|command| format!("`{command}`")).collect();
    let choices = vec![
        Choice {
            identifier: String::from(TRUST_IT),
            label: Text::from("Trust this folder and run them"),
            description: Some(Text::from(format!(
                "What `{folder}` and the folders in it ask to run is run from now on."
            ))),
        },
        Choice {
            identifier: String::from(NOT_NOW),
            label: Text::from("Not now"),
            description: Some(Text::from("Create the worktree without running them.")),
        },
    ];
    Question {
        key: TRUST,
        kind: QuestionKind::Pick { choices },
        title: Text::from("Run the commands for new worktrees?"),
        detail: Some(Text::from(format!(
            "`{folder}` is not trusted, and worktrees.setupCommands in {place} asks to run {}. Trust it only if you \
             trust who wrote them.",
            commands.join(", then ")
        ))),
        default: Some(Answer::Choice(String::from(NOT_NOW))),
    }
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use model::contract::{Answer, Answers, QuestionKind, Target};
    use model::trust::Trust;
    use model::worktree_setup::SettingSource;

    use super::{TRUST, TrustAsked, ask, trusted_by_answer};
    use crate::testing::{context, state};

    fn commands() -> Vec<String> {
        vec![String::from("npm ci"), String::from("npm run build")]
    }

    #[test]
    fn trust_ask__a_folder_not_trusted_is_asked_about_naming_the_commands_with_not_now_proposed() {
        let context = context(state(), Target::Head);
        let commands = commands();
        let asked = TrustAsked {
            commands: &commands,
            source: SettingSource::Repository,
        };
        let question = match ask(&context, &Answers::new(), &asked) {
            Err(outcome) => crate::testing::question(*outcome),
            other => panic!("expected a question, got {other:?}"),
        };
        assert_eq!(question.key, TRUST);
        assert_eq!(question.title.as_str(), "Run the commands for new worktrees?");
        assert_eq!(
            question.detail.as_ref().map(model::contract::Text::as_str),
            Some(
                "`repository` is not trusted, and worktrees.setupCommands in its .gitardo/settings.json asks to run \
                 `npm ci`, then `npm run build`. Trust it only if you trust who wrote them."
            )
        );
        let QuestionKind::Pick { choices } = &question.kind else {
            panic!("{question:?}");
        };
        let labels: Vec<&str> = choices.iter().map(|choice| choice.label.as_str()).collect();
        assert_eq!(labels, ["Trust this folder and run them", "Not now"]);
        assert_eq!(question.default, Some(Answer::Choice(String::from("not-now"))));
    }

    #[test]
    fn trust_ask__trusting_it_runs_them_and_the_folder_is_trusted_from_then_on() {
        let context = context(state(), Target::Head);
        let commands = commands();
        let asked = TrustAsked {
            commands: &commands,
            source: SettingSource::User,
        };
        let answers = Answers::new().with(TRUST, Answer::Choice(String::from("trust")));
        assert_eq!(ask(&context, &answers, &asked), Ok(Trust::Trusted));
        assert_eq!(
            trusted_by_answer(&context, &answers),
            Some(context.repository.folder.clone())
        );
    }

    #[test]
    fn trust_ask__not_now_runs_nothing_and_trusts_nothing() {
        let context = context(state(), Target::Head);
        let commands = commands();
        let asked = TrustAsked {
            commands: &commands,
            source: SettingSource::Repository,
        };
        let answers = Answers::new().with(TRUST, Answer::Choice(String::from("not-now")));
        assert_eq!(ask(&context, &answers, &asked), Ok(Trust::NotTrusted));
        assert_eq!(trusted_by_answer(&context, &answers), None);
    }

    #[test]
    fn trust_ask__a_trusted_folder_or_nothing_to_run_asks_nothing() {
        let mut trusted = context(state(), Target::Head);
        trusted.settings.trust = Trust::Trusted;
        let commands = commands();
        let asked = TrustAsked {
            commands: &commands,
            source: SettingSource::Repository,
        };
        assert_eq!(ask(&trusted, &Answers::new(), &asked), Ok(Trust::Trusted));
        let answers = Answers::new().with(TRUST, Answer::Choice(String::from("trust")));
        assert_eq!(trusted_by_answer(&trusted, &answers), None, "it was trusted before");
        let not_trusted = context(state(), Target::Head);
        let nothing = TrustAsked {
            commands: &[],
            source: SettingSource::Nothing,
        };
        assert_eq!(ask(&not_trusted, &Answers::new(), &nothing), Ok(Trust::NotTrusted));
    }
}

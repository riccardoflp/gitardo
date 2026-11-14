//! `trust.manage`: the folders trusted, where those unticked stop being trusted.

use std::path::PathBuf;

use model::contract::{
    Answer, AnswerKey, Answers, Choice, Context, Danger, FollowUp, Outcome, Plan, Question, QuestionKind, Refusal,
    Text, Undo,
};

/// The key of the answer: the folders still trusted, by their paths.
pub const TRUSTED: AnswerKey = AnswerKey("trusted-folders");

/// Lists the folders trusted, each ticked, and stops trusting those unticked; refuses when none is trusted.
///
/// Feature: trust.manage
#[must_use]
pub fn manage(context: &Context, answers: &Answers) -> Outcome {
    let trusted = &context.settings.trusted_folders;
    if trusted.is_empty() {
        return Outcome::Refuse(Refusal::saying("No folder is trusted."));
    }
    let Some(kept) = answers.choices(TRUSTED) else {
        return Outcome::Ask(question(trusted));
    };
    let forgotten: Vec<PathBuf> = trusted
        .iter()
        .filter(|folder| !kept.contains(&identifier(folder)))
        .cloned()
        .collect();
    let (title, said) = match forgotten.as_slice() {
        [] => (
            String::from("Keep the trusted folders"),
            String::from("The trusted folders are unchanged."),
        ),
        [folder] => (
            format!("Stop trusting `{}`", folder.display()),
            format!("`{}` is no longer trusted.", folder.display()),
        ),
        _ => (
            format!("Stop trusting {} folders", forgotten.len()),
            format!("{} folders are no longer trusted.", forgotten.len()),
        ),
    };
    let mut after = Vec::new();
    if !forgotten.is_empty() {
        after.push(FollowUp::ForgetTrustedFolders(forgotten));
    }
    after.push(FollowUp::Say(Text::from(said)));
    Outcome::Run(Plan {
        title: Text::from(title),
        steps: Vec::new(),
        danger: Danger::None,
        undo: Undo::Impossible(Text::from("Gitardo asks again before it runs what such a folder asks.")),
        after,
        on_failure: Vec::new(),
    })
}

/// A folder as a choice answers it.
fn identifier(folder: &std::path::Path) -> String {
    folder.to_string_lossy().into_owned()
}

/// **Trusted folders**: the folders trusted, each ticked.
fn question(trusted: &[PathBuf]) -> Question {
    let choices = trusted
        .iter()
        .map(|folder| Choice {
            identifier: identifier(folder),
            label: Text::from(folder.display().to_string()),
            description: None,
        })
        .collect();
    Question {
        key: TRUSTED,
        kind: QuestionKind::PickMany { choices },
        title: Text::from("Trusted folders"),
        detail: Some(Text::from(
            "What a trusted folder and the folders in it ask to run is run. Untick a folder to stop trusting it.",
        )),
        default: Some(Answer::Choices(
            trusted.iter().map(|folder| identifier(folder)).collect(),
        )),
    }
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use std::path::PathBuf;

    use model::contract::{Answer, Answers, FollowUp, Outcome, QuestionKind, Target, Text};

    use super::{TRUSTED, manage};
    use crate::testing::{context, plan, question, state};

    fn trusting(folders: &[&str]) -> model::contract::Context {
        let mut context = context(state(), Target::Head);
        context.settings.trusted_folders = folders.iter().map(PathBuf::from).collect();
        context
    }

    #[test]
    fn trust_manage__lists_the_folders_trusted_each_ticked() {
        let context = trusting(&["/work/app", "/work/tools"]);
        let asked = question(manage(&context, &Answers::new()));
        assert_eq!(asked.title.as_str(), "Trusted folders");
        let QuestionKind::PickMany { choices } = &asked.kind else {
            panic!("{asked:?}");
        };
        let listed: Vec<&str> = choices.iter().map(|choice| choice.identifier.as_str()).collect();
        assert_eq!(listed, ["/work/app", "/work/tools"]);
        assert_eq!(
            asked.default,
            Some(Answer::Choices(vec![
                String::from("/work/app"),
                String::from("/work/tools")
            ]))
        );
    }

    #[test]
    fn trust_manage__a_folder_unticked_is_no_longer_trusted_and_the_others_stay() {
        let context = trusting(&["/work/app", "/work/tools", "/work/site"]);
        let answers = Answers::new().with(TRUSTED, Answer::Choices(vec![String::from("/work/tools")]));
        let made = plan(manage(&context, &answers));
        assert_eq!(made.steps, []);
        assert_eq!(made.title.as_str(), "Stop trusting 2 folders");
        assert_eq!(
            made.after,
            [
                FollowUp::ForgetTrustedFolders(vec![PathBuf::from("/work/app"), PathBuf::from("/work/site")]),
                FollowUp::Say(Text::from("2 folders are no longer trusted.")),
            ]
        );
        let kept_all = Answers::new().with(
            TRUSTED,
            Answer::Choices(vec![
                String::from("/work/app"),
                String::from("/work/tools"),
                String::from("/work/site"),
            ]),
        );
        assert_eq!(
            plan(manage(&context, &kept_all)).after,
            [FollowUp::Say(Text::from("The trusted folders are unchanged."))]
        );
    }

    #[test]
    fn trust_manage__with_no_folder_trusted_it_says_so_and_asks_nothing() {
        let context = trusting(&[]);
        let Outcome::Refuse(refusal) = manage(&context, &Answers::new()) else {
            panic!("expected a refusal");
        };
        assert_eq!(refusal.message.as_str(), "No folder is trusted.");
    }
}

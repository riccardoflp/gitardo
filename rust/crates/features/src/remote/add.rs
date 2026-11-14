//! `remote.add`: adds a remote, by its name and its URL.

use model::contract::{
    Answer, AnswerKey, Answers, Context, Danger, Outcome, Plan, Question, QuestionKind, Step, Text, Undo, Validation,
};

/// The name typed for the remote.
pub const NAME: AnswerKey = AnswerKey("name");

/// The URL typed for a remote.
pub const URL: AnswerKey = AnswerKey("url");

/// Asks the name of the remote, `origin` to start from in a repository with none, then its URL, and adds it; nothing
/// is fetched until asked.
///
/// Feature: remote.add
#[must_use]
pub fn add(context: &Context, answers: &Answers) -> Outcome {
    let remotes = &context.repository.remotes;
    let Some(name) = answers.text(NAME) else {
        return Outcome::Ask(Question {
            key: NAME,
            kind: QuestionKind::Text {
                placeholder: None,
                validation: Validation::ReferenceName {
                    taken: remotes.clone(),
                    taken_message: Text::from("A remote with that name already exists"),
                },
            },
            title: Text::from("Add remote"),
            detail: Some(Text::from("Name of the remote")),
            default: remotes.is_empty().then(|| Answer::Text(String::from("origin"))),
        });
    };
    let Some(url) = answers.text(URL) else {
        return Outcome::Ask(url_question(&format!("Add remote `{name}`"), None));
    };
    let url = url.trim();
    Outcome::Run(Plan {
        title: Text::from(format!("Add remote {name}")),
        steps: vec![Step::git(
            context.repository.folder.clone(),
            ["remote", "add", name, url],
        )],
        danger: Danger::None,
        undo: Undo::Impossible(Text::from("Not taken back by Undo: removing the remote does.")),
        after: vec![model::contract::FollowUp::ReadPullRequests],
        on_failure: Vec::new(),
    })
}

/// The question of the URL of a remote, starting from `current` when it has one.
pub(super) fn url_question(title: &str, current: Option<&str>) -> Question {
    Question {
        key: URL,
        kind: QuestionKind::Text {
            placeholder: Some(Text::from("https://github.com/owner/repository.git")),
            validation: Validation::Url,
        },
        title: Text::from(title),
        detail: Some(Text::from("URL of the remote repository")),
        default: current.map(|url| Answer::Text(url.to_owned())),
    }
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::{NAME, URL, add};
    use crate::testing::{context, lines, plan, question, state};
    use model::contract::{Answer, Answers, QuestionKind, Target};

    #[test]
    fn remote_add__the_name_starts_from_origin_in_a_repository_with_no_remote() {
        let name = question(add(&context(state(), Target::Head), &Answers::new()));
        assert_eq!(name.title.as_str(), "Add remote");
        assert_eq!(name.default, Some(Answer::Text(String::from("origin"))));
        let mut with_one = state();
        with_one.remotes.push(String::from("origin"));
        let name = question(add(&context(with_one, Target::Head), &Answers::new()));
        assert_eq!(name.default, None);
        let QuestionKind::Text { validation, .. } = name.kind else {
            panic!("the name is typed")
        };
        assert_eq!(
            validation.problem("origin").map(|text| text.to_string()),
            Some(String::from("A remote with that name already exists"))
        );
    }

    #[test]
    fn remote_add__adds_the_remote_with_its_url() {
        let answers = Answers::new()
            .with(NAME, Answer::Text(String::from("origin")))
            .with(URL, Answer::Text(String::from(" https://github.com/owner/app.git ")));
        assert_eq!(
            lines(&plan(add(&context(state(), Target::Head), &answers))),
            ["git remote add origin https://github.com/owner/app.git"]
        );
    }

    #[test]
    fn remote_add__a_url_that_starts_with_a_dash_cannot_be_accepted() {
        let answers = Answers::new().with(NAME, Answer::Text(String::from("origin")));
        let url = question(add(&context(state(), Target::Head), &answers));
        assert_eq!(url.title.as_str(), "Add remote `origin`");
        let QuestionKind::Text { validation, .. } = url.kind else {
            panic!("the URL is typed")
        };
        assert_eq!(
            validation.problem("--upload-pack=x").map(|text| text.to_string()),
            Some(String::from("That is not a URL"))
        );
        assert_eq!(
            validation.problem("  ").map(|text| text.to_string()),
            Some(String::from("Enter a URL"))
        );
        assert_eq!(validation.problem("git@github.com:owner/app.git"), None);
    }
}

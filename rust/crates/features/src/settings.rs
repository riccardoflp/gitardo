//! The settings changed from the window (`docs/features/18-settings.md`): the theme, written in the user's settings
//! file with the rest of its text, comments included, kept.

use json::{Key, Value};
use model::contract::{
    Answer, AnswerKey, Answers, Choice, Context, Danger, Outcome, Plan, Query, Question, QuestionKind, Refusal, Step,
    Text, Undo,
};

/// The theme picked: one of [`THEMES`], by its value.
pub const THEME: AnswerKey = AnswerKey("theme");

/// The text of the user's settings file, read before it is written.
const USER_FILE: AnswerKey = AnswerKey("user-settings-file");

/// The five themes, by the value `theme` takes and the name the user reads.
pub const THEMES: [(&str, &str); 5] = [
    ("system", "Follow the system"),
    ("dark", "Dark"),
    ("light", "Light"),
    ("highContrastDark", "High contrast dark"),
    ("highContrastLight", "High contrast light"),
];

/// What is said when the system does not say where the user's settings are.
const NO_USER_FILE: &str = "The system does not say where your settings are, so the theme was not changed.";

/// Asks for the theme among the five of `theme`, then writes it in the user's settings file, keeping the rest of the
/// file as it is; the window takes it when the file is read again. Choosing again is what takes it back.
///
/// Feature: settings.theme
#[must_use]
pub fn theme(context: &Context, answers: &Answers) -> Outcome {
    let Some(picked) = answers.choice(THEME) else {
        return Outcome::Ask(theme_question());
    };
    let Some(&(value, name)) = THEMES.iter().find(|(value, _)| *value == picked) else {
        return Outcome::Refuse(Refusal::saying(&format!("{picked} is not a theme.")));
    };
    let Some(file) = context.settings.user_settings_file.clone() else {
        return Outcome::Refuse(Refusal::saying(NO_USER_FILE));
    };
    let Some(Answer::Files(texts)) = answers.get(USER_FILE) else {
        return Outcome::Read(USER_FILE, Query::FileTexts { paths: vec![file] });
    };
    let before = texts
        .first()
        .cloned()
        .flatten()
        .map_or_default(|bytes| String::from_utf8_lossy(&bytes).into_owned());
    let path = [Key::Member(String::from("theme"))];
    let after = match json::set_value(&before, &path, &Value::String(value.to_owned())) {
        Ok(after) => after,
        Err(error) => {
            let said = format!(
                "{} is not JSON with comments, so the theme was not changed: {error}",
                file.display()
            );
            return Outcome::Refuse(Refusal::saying(&said));
        }
    };
    Outcome::Run(Plan {
        title: Text::from(format!("Use the theme {name}")),
        steps: vec![Step::Write {
            path: file,
            contents: after.into_bytes(),
        }],
        danger: Danger::None,
        undo: Undo::Impossible(Text::from("Choose the theme again to change it back.")),
        after: Vec::new(),
        on_failure: Vec::new(),
    })
}

/// **Theme**: the five themes.
fn theme_question() -> Question {
    Question {
        key: THEME,
        kind: QuestionKind::Pick {
            choices: THEMES
                .iter()
                .map(|(value, name)| Choice {
                    identifier: (*value).to_owned(),
                    label: Text::from(*name),
                    description: None,
                })
                .collect(),
        },
        title: Text::from("Theme"),
        detail: None,
        default: None,
    }
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use std::path::PathBuf;

    use model::contract::{Answer, Answers, Outcome, Step, Target};

    use super::{THEME, USER_FILE, theme};
    use crate::testing::{context, plan, question, state};

    fn with_user_file() -> model::contract::Context {
        let mut context = context(state(), Target::Head);
        context.settings.user_settings_file = Some(PathBuf::from("user").join("settings.json"));
        context
    }

    #[test]
    fn settings_theme__light_is_written_in_the_users_file_keeping_its_comments() {
        let before = "{\n  // Mine.\n  \"theme\": \"dark\",\n  \"graph.pageSize\": 500\n}\n";
        let answers = Answers::new()
            .with(THEME, Answer::Choice(String::from("light")))
            .with(USER_FILE, Answer::Files(vec![Some(before.as_bytes().to_vec())]));
        let plan = plan(theme(&with_user_file(), &answers));
        let [Step::Write { path, contents }] = plan.steps.as_slice() else {
            panic!("one file written: {plan:?}")
        };
        assert_eq!(path, &PathBuf::from("user").join("settings.json"));
        assert_eq!(
            String::from_utf8_lossy(contents),
            before.replace("\"dark\"", "\"light\"")
        );
    }

    #[test]
    fn settings_theme__with_no_theme_given_it_asks_among_the_five() {
        let asked = question(theme(&with_user_file(), &Answers::new()));
        let model::contract::QuestionKind::Pick { choices } = asked.kind else {
            panic!("a pick")
        };
        assert_eq!(choices.len(), 5);
        let unknown = Answers::new().with(THEME, Answer::Choice(String::from("sepia")));
        assert!(matches!(theme(&with_user_file(), &unknown), Outcome::Refuse(_)));
    }
}

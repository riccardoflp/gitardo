//! `sync.pull-with`: pulls in one of the four ways, whatever the settings say **Pull** does.

use model::contract::{AnswerKey, Answers, Choice, Context, FeatureSettings, Outcome, Question, QuestionKind, Text};

use super::pull::pull_plan;

/// The way to pull, picked: one of [`WAYS`], by its identifier.
pub const WAY: AnswerKey = AnswerKey("way");

/// The four ways to pull, by identifier, with `--rebase` and `--autostash` as each has them, in the order the menu of
/// **Sync** lists them.
pub const WAYS: [(&str, bool, bool); 4] = [
    ("plain", false, false),
    ("rebase", true, false),
    ("autostash", false, true),
    ("rebase-autostash", true, true),
];

/// The way **Pull** pulls with these settings, by identifier: the one the menu marks **default**.
#[must_use]
pub fn default_way(settings: &FeatureSettings) -> &'static str {
    WAYS.iter()
        .find(|(_, rebase, autostash)| *rebase == settings.pull_rebase && *autostash == settings.pull_autostash)
        .map_or("plain", |(identifier, _, _)| identifier)
}

/// Pulls in the way picked, asking which when it was not given; refuses as **Pull** does.
///
/// Feature: sync.pull-with
#[must_use]
pub fn pull_with(context: &Context, answers: &Answers) -> Outcome {
    let Some(way) = answers.choice(WAY) else {
        return Outcome::Ask(way_question(&context.settings));
    };
    let (rebase, autostash) = WAYS
        .iter()
        .find(|(identifier, _, _)| *identifier == way)
        .map_or((false, false), |(_, rebase, autostash)| (*rebase, *autostash));
    pull_plan(context, rebase, autostash)
}

fn way_question(settings: &FeatureSettings) -> Question {
    let default = default_way(settings);
    let choices = WAYS
        .iter()
        .map(|(identifier, rebase, autostash)| Choice {
            identifier: (*identifier).to_owned(),
            label: Text::from(super::target::pull_arguments(*rebase, *autostash).join(" ")),
            description: (*identifier == default).then(|| Text::from("default")),
        })
        .collect();
    Question {
        key: WAY,
        kind: QuestionKind::Pick { choices },
        title: Text::from("Pull"),
        detail: None,
        default: Some(model::contract::Answer::Choice(default.to_owned())),
    }
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::{WAY, default_way, pull_with};
    use crate::testing::{context, lines, plan, question, state};
    use model::contract::{Answer, Answers, FeatureSettings, QuestionKind, Target};

    #[test]
    fn sync_pull_with__no_options_runs_git_pull_whatever_the_settings() {
        let mut state = state();
        state.remotes.push(String::from("origin"));
        let context = context(state, Target::Head);
        let answers = Answers::new().with(WAY, Answer::Choice(String::from("plain")));
        assert_eq!(lines(&plan(pull_with(&context, &answers))), ["git pull"]);
    }

    #[test]
    fn sync_pull_with__with_rebase_off_autostash_is_the_default() {
        let settings = FeatureSettings {
            pull_rebase: false,
            ..FeatureSettings::default()
        };
        assert_eq!(default_way(&settings), "autostash");
        let mut context = context(state(), Target::Head);
        context.settings = settings;
        let QuestionKind::Pick { choices } = question(pull_with(&context, &Answers::new())).kind else {
            panic!("the way is picked")
        };
        let marked: Vec<&str> = choices
            .iter()
            .filter(|choice| choice.description.is_some())
            .map(|choice| choice.identifier.as_str())
            .collect();
        assert_eq!(marked, ["autostash"]);
    }
}

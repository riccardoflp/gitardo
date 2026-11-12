//! `remote.manage`: lists the remotes with their URLs, to fetch, repoint, rename, copy or remove one, or to add another.

use model::contract::{
    AnswerKey, Answers, Choice, Context, Danger, FollowUp, Outcome, Plan, Query, Question, QuestionKind, Target, Text,
    Undo,
};
use model::remote::Remote;

use super::REMOTES;

/// The remote picked, by its name, or [`ADD`].
pub const REMOTE: AnswerKey = AnswerKey("remote");

/// What to do with the remote picked: `fetch`, `set-url`, `rename`, `copy` or `remove`.
pub const ACTION: AnswerKey = AnswerKey("action");

/// The choice of the first pick that adds a remote: a colon, which no remote name has.
const ADD: &str = ":add";

/// Picks a remote among those read, each with its URL, or **Add remote…**; then what to do with it; then asks what the
/// feature chosen asks and runs what it runs. **Copy URL** puts the URL on the clipboard and runs nothing.
///
/// Feature: remote.manage
#[must_use]
pub fn manage(context: &Context, answers: &Answers) -> Outcome {
    let Some(remotes) = answers.remotes(REMOTES) else {
        return Outcome::Read(REMOTES, Query::Remotes);
    };
    let Some(picked) = answers.choice(REMOTE) else {
        return Outcome::Ask(remote_question(context, remotes));
    };
    if picked == ADD {
        return super::add(context, answers);
    }
    let Some(remote) = remotes.iter().find(|remote| remote.name == picked) else {
        return Outcome::Refuse(model::contract::Refusal::saying(&format!(
            "The remote {picked} is no longer there."
        )));
    };
    let Some(action) = answers.choice(ACTION) else {
        return Outcome::Ask(action_question(remote));
    };
    let mut on_remote = context.clone();
    on_remote.target = Target::Remote(remote.name.clone());
    match action {
        "fetch" => super::fetch(&on_remote, answers),
        "set-url" => super::set_url(&on_remote, answers),
        "rename" => super::rename(&on_remote, answers),
        "remove" => super::remove(&on_remote, answers),
        _ => Outcome::Run(copy_url(remote)),
    }
}

fn choice(identifier: &str, label: &str, description: Option<String>) -> Choice {
    Choice {
        identifier: identifier.to_owned(),
        label: Text::from(label),
        description: description.map(Text::from),
    }
}

fn remote_question(context: &Context, remotes: &[Remote]) -> Question {
    let repository = context
        .repository
        .folder
        .file_name()
        .map_or_else(String::new, |name| name.to_string_lossy().into_owned());
    let mut choices: Vec<Choice> = remotes
        .iter()
        .map(|remote| choice(&remote.name, &remote.name, Some(remote.url.clone())))
        .collect();
    choices.push(choice(ADD, "Add remote\u{2026}", None));
    Question {
        key: REMOTE,
        kind: QuestionKind::Pick { choices },
        title: Text::from(format!("Remotes of `{repository}`")),
        detail: Some(Text::from("Pick a remote to fetch, rename, repoint or remove it")),
        default: None,
    }
}

fn action_question(remote: &Remote) -> Question {
    let name = &remote.name;
    let choices = vec![
        choice("fetch", "Fetch", Some(format!("git fetch --prune {name}"))),
        choice("set-url", "Change URL\u{2026}", None),
        choice("rename", "Rename\u{2026}", None),
        choice("copy", "Copy URL", None),
        choice(
            "remove",
            "Remove",
            Some(String::from("Deletes its remote-tracking branches here")),
        ),
    ];
    Question {
        key: ACTION,
        kind: QuestionKind::Pick { choices },
        title: Text::from(format!("{name}: {}", remote.url)),
        detail: None,
        default: None,
    }
}

/// Puts the URL of a remote on the clipboard: no step, so nothing is logged.
fn copy_url(remote: &Remote) -> Plan {
    Plan {
        title: Text::from(format!("Copy the URL of {}", remote.name)),
        steps: Vec::new(),
        danger: Danger::None,
        undo: Undo::Impossible(Text::from("Nothing changed.")),
        after: vec![
            FollowUp::CopyToClipboard(remote.url.clone()),
            FollowUp::Say(Text::from(format!("Copied the URL of {}.", remote.name))),
        ],
        on_failure: Vec::new(),
    }
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::{ACTION, REMOTE, manage};
    use crate::remote::REMOTES;
    use crate::testing::{context, lines, plan, question, state};
    use model::contract::{Answer, Answers, FollowUp, QuestionKind, Target};
    use model::remote::Remote;

    fn read() -> Answers {
        let remote = |name: &str, url: &str| Remote {
            name: name.to_owned(),
            url: url.to_owned(),
        };
        let remotes = vec![
            remote("origin", "https://github.com/owner/app.git"),
            remote("upstream", "https://example.com/team/app.git"),
        ];
        Answers::new().with(REMOTES, Answer::Remotes(remotes))
    }

    fn picked(remote: &str, action: &str) -> Answers {
        read()
            .with(REMOTE, Answer::Choice(remote.to_owned()))
            .with(ACTION, Answer::Choice(action.to_owned()))
    }

    #[test]
    fn remote_manage__lists_the_remotes_with_their_urls_and_add_remote_last() {
        let asked = question(manage(&context(state(), Target::Head), &read()));
        assert_eq!(asked.title.as_str(), "Remotes of `repository`");
        let QuestionKind::Pick { choices } = asked.kind else {
            panic!("a remote is picked")
        };
        let listed: Vec<(&str, Option<&str>)> = choices
            .iter()
            .map(|choice| {
                (
                    choice.label.as_str(),
                    choice.description.as_ref().map(model::contract::Text::as_str),
                )
            })
            .collect();
        assert_eq!(
            listed,
            [
                ("origin", Some("https://github.com/owner/app.git")),
                ("upstream", Some("https://example.com/team/app.git")),
                ("Add remote\u{2026}", None)
            ]
        );
    }

    #[test]
    fn remote_manage__copy_url_puts_it_on_the_clipboard_and_runs_nothing() {
        let plan = plan(manage(&context(state(), Target::Head), &picked("origin", "copy")));
        assert!(plan.steps.is_empty(), "nothing is logged");
        assert!(plan.after.contains(&FollowUp::CopyToClipboard(String::from(
            "https://github.com/owner/app.git"
        ))));
    }

    #[test]
    fn runs_the_feature_chosen_on_the_remote_picked() {
        let context = context(state(), Target::Head);
        assert_eq!(
            lines(&plan(manage(&context, &picked("upstream", "fetch")))),
            ["git fetch --prune upstream"]
        );
        assert_eq!(
            lines(&plan(manage(&context, &picked("upstream", "remove")))),
            ["git remote remove upstream"]
        );
    }
}

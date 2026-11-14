//! `remote.set-url`: changes the URL a remote is fetched from and pushed to.

use model::contract::{Answers, Context, Danger, Outcome, Plan, Step, Text, Undo};

use super::add::{URL, url_question};
use super::{target_remote, unchanged, url_of};

/// Asks the URL of the remote it was invoked on, starting from the one it has, and sets it, without the spaces around
/// it; nothing runs when it is unchanged.
///
/// Feature: remote.set-url
#[must_use]
pub fn set_url(context: &Context, answers: &Answers) -> Outcome {
    let name = match target_remote(context) {
        Ok(name) => name,
        Err(refusal) => return *refusal,
    };
    let current = match url_of(answers, name) {
        Ok(current) => current,
        Err(read) => return *read,
    };
    let Some(url) = answers.text(URL) else {
        return Outcome::Ask(url_question(&format!("Change the URL of `{name}`"), current));
    };
    let url = url.trim();
    if Some(url) == current {
        return Outcome::Run(unchanged(format!("The URL of {name} is unchanged.")));
    }
    Outcome::Run(Plan {
        title: Text::from(format!("Change the URL of {name}")),
        steps: vec![Step::git(
            context.repository.folder.clone(),
            ["remote", "set-url", name, url],
        )],
        danger: Danger::None,
        undo: Undo::Impossible(Text::from("Not taken back by Undo: changing it back does.")),
        after: vec![model::contract::FollowUp::ReadPullRequests],
        on_failure: Vec::new(),
    })
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::set_url;
    use crate::remote::{REMOTES, URL};
    use crate::testing::{context, lines, plan, question, state};
    use model::contract::{Answer, Answers, Outcome, Query, Target};
    use model::remote::Remote;

    fn read() -> Answers {
        let origin = Remote {
            name: String::from("origin"),
            url: String::from("https://example.com/app.git"),
        };
        Answers::new().with(REMOTES, Answer::Remotes(vec![origin]))
    }

    #[test]
    fn remote_set_url__starts_from_the_url_it_has_sets_the_new_one_and_runs_nothing_when_unchanged() {
        let context = context(state(), Target::Remote(String::from("origin")));
        assert!(matches!(
            set_url(&context, &Answers::new()),
            Outcome::Read(REMOTES, Query::Remotes)
        ));
        let asked = question(set_url(&context, &read()));
        assert_eq!(asked.title.as_str(), "Change the URL of `origin`");
        assert_eq!(
            asked.default,
            Some(Answer::Text(String::from("https://example.com/app.git")))
        );
        let answers = read().with(URL, Answer::Text(String::from("git@github.com:owner/app.git")));
        assert_eq!(
            lines(&plan(set_url(&context, &answers))),
            ["git remote set-url origin git@github.com:owner/app.git"]
        );
        let same = read().with(URL, Answer::Text(String::from("https://example.com/app.git ")));
        assert_eq!(plan(set_url(&context, &same)).steps, Vec::new());
    }
}

//! `remote.fetch`: fetches one remote, and forgets the branches deleted there.

use model::contract::{Answers, Context, Danger, FollowUp, Outcome, Plan, Step, Text, Undo};

use super::target_remote;

/// Fetches the remote it was invoked on with `git fetch --prune <name>`.
///
/// Feature: remote.fetch
#[must_use]
pub fn fetch(context: &Context, _answers: &Answers) -> Outcome {
    let name = match target_remote(context) {
        Ok(name) => name,
        Err(refusal) => return *refusal,
    };
    Outcome::Run(Plan {
        title: Text::from(format!("Fetch {name}")),
        steps: vec![Step::git(context.repository.folder.clone(), ["fetch", "--prune", name])],
        danger: Danger::None,
        undo: Undo::Impossible(Text::from(
            "A fetch only moves the remote-tracking branches, which Undo does not take back.",
        )),
        after: vec![FollowUp::ReadPullRequests],
        on_failure: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::fetch;
    use crate::testing::{context, lines, plan, state};
    use model::contract::{Answers, Target};

    #[test]
    fn fetches_one_remote_and_prunes() {
        let context = context(state(), Target::Remote(String::from("upstream")));
        assert_eq!(
            lines(&plan(fetch(&context, &Answers::new()))),
            ["git fetch --prune upstream"]
        );
    }
}

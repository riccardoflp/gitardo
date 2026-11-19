//! `sync.fetch`: brings what every remote has, and forgets the remote branches deleted there.

use model::contract::{Answers, Context, Danger, FollowUp, Outcome, Plan, Step, Text, Undo};

use super::target::no_remote;

/// Fetches every remote and prunes what was deleted there; refuses in a repository with no remote, offering to add
/// one.
///
/// Feature: sync.fetch
#[must_use]
pub fn fetch(context: &Context, _answers: &Answers) -> Outcome {
    if context.repository.remotes.is_empty() {
        return no_remote();
    }
    Outcome::Run(Plan {
        title: Text::from("Fetch"),
        steps: vec![Step::git(
            context.repository.folder.clone(),
            ["fetch", "--all", "--prune"],
        )],
        danger: Danger::None,
        undo: Undo::Impossible(Text::from(
            "A fetch only moves the remote-tracking branches, which Undo does not take back.",
        )),
        after: vec![FollowUp::ReadPullRequests, FollowUp::CheckForcePushed],
        on_failure: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::fetch;
    use crate::testing::{context, lines, plan, state};
    use model::contract::{Answers, FeatureId, Outcome, Target};

    #[test]
    fn fetches_every_remote_and_prunes() {
        let mut state = state();
        state.remotes.push(String::from("origin"));
        assert_eq!(
            lines(&plan(fetch(&context(state, Target::Head), &Answers::new()))),
            ["git fetch --all --prune"]
        );
    }

    #[test]
    fn refuses_without_a_remote_and_offers_to_add_one() {
        let Outcome::Refuse(refusal) = fetch(&context(state(), Target::Head), &Answers::new()) else {
            panic!("a repository with no remote is refused")
        };
        assert_eq!(refusal.message.as_str(), "This repository has no remote.");
        assert_eq!(
            refusal.offer.map(|(feature, label)| (feature, label.to_string())),
            Some((FeatureId("remote.add"), String::from("Add remote…")))
        );
    }
}

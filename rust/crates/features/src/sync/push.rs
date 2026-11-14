//! `sync.push`: sends the commits of the branch checked out to its upstream, or publishes it for its first push.

use model::contract::{Answers, Context, Danger, FollowUp, Outcome, Plan, Text, Undo};

use super::target::{PUSH_UNDO, PushTarget, push_target};

/// Pushes the branch checked out to its upstream, `<branch>:<upstream branch>` when their names differ; with no
/// upstream, to the remote of `remote.default` with `--set-upstream`. Refuses on a detached `HEAD` and in a repository
/// with no remote.
///
/// Feature: sync.push
#[must_use]
pub fn push(context: &Context, _answers: &Answers) -> Outcome {
    match push_target(context, "push") {
        Ok(target) => Outcome::Run(push_plan(context, &target)),
        Err(refusal) => *refusal,
    }
}

/// The plan of a push to a target, with no option.
pub(crate) fn push_plan(context: &Context, target: &PushTarget) -> Plan {
    Plan {
        title: Text::from(format!("Push {} to {}", target.branch, target.remote_branch())),
        steps: vec![target.step(context, None)],
        danger: Danger::None,
        undo: Undo::Impossible(Text::from(PUSH_UNDO)),
        after: vec![FollowUp::ReadPullRequests],
        on_failure: Vec::new(),
    }
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::push;
    use crate::testing::{branch, context, lines, plan, state, tag, tracking};
    use model::contract::{Answers, Outcome, Target};
    use model::state::RepositoryState;

    fn on(name: &str, upstream: Option<&str>) -> RepositoryState {
        let mut state = state();
        state.remotes = vec![String::from("upstream"), String::from("origin")];
        state.head.branch = Some(format!("refs/heads/{name}"));
        let mut checked_out = branch(name, 'a', true);
        checked_out.upstream = upstream.map(|upstream| tracking("origin", upstream));
        state.branches = vec![checked_out];
        state
    }

    fn pushed(state: RepositoryState) -> Vec<String> {
        lines(&plan(push(&context(state, Target::Head), &Answers::new())))
    }

    #[test]
    fn sync_push__a_branch_following_its_namesake_pushes_to_it() {
        assert_eq!(pushed(on("main", Some("main"))), ["git push origin main"]);
    }

    #[test]
    fn sync_push__a_branch_following_another_name_pushes_to_that_name() {
        assert_eq!(
            pushed(on("local", Some("remote-name"))),
            ["git push origin local:remote-name"]
        );
    }

    #[test]
    fn sync_push__a_branch_with_no_upstream_is_published_to_the_default_remote() {
        assert_eq!(pushed(on("feature", None)), ["git push --set-upstream origin feature"]);
    }

    #[test]
    fn sync_push__a_branch_with_a_tag_of_its_name_is_named_in_full() {
        let mut state = on("release", Some("release"));
        state.tags.push(tag("release", 'b'));
        assert_eq!(pushed(state), ["git push origin refs/heads/release"]);
    }

    #[test]
    fn sync_push__a_detached_head_is_refused_and_says_why() {
        let mut state = on("main", Some("main"));
        state.head.branch = None;
        let Outcome::Refuse(refusal) = push(&context(state, Target::Head), &Answers::new()) else {
            panic!("a detached HEAD is refused")
        };
        assert_eq!(
            refusal.message.as_str(),
            "Cannot push: HEAD is detached. Check out a branch first."
        );
    }
}

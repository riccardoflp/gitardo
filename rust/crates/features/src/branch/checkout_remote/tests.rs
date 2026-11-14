use super::{AS_IT_IS, COUNTS, REBASE, WAY, checkout_remote};
use crate::branch::submodules::none;
use crate::testing::{branch, commit, context, lines, plan, question, state};
use model::Timestamp;
use model::contract::{Answer, Answers, Context, FollowUp, Outcome, Query, QuestionKind, Target};
use model::state::{RemoteBranch, RepositoryState};

/// A repository with `origin/feature`, and a local `feature` when `local` says whether it is checked out.
fn repository(local: Option<bool>) -> RepositoryState {
    let mut state = state();
    state.remotes = vec![String::from("origin")];
    state.remote_branches.push(RemoteBranch {
        name: String::from("origin/feature"),
        remote: String::from("origin"),
        commit: commit('c'),
        date: Timestamp(0),
    });
    if let Some(is_checked_out) = local {
        if is_checked_out {
            state.branches = vec![branch("main", 'a', false)];
            state.head.branch = Some(String::from("refs/heads/feature"));
        }
        state.branches.push(branch("feature", 'b', is_checked_out));
    }
    state
}

fn on_remote(state: RepositoryState) -> Context {
    context(state, Target::RemoteBranch(String::from("origin/feature")))
}

/// The answers once the counts were read: the commits only `feature` has, then those only `origin/feature` has.
fn counted(own: u32, theirs: u32) -> Answers {
    none(Answers::new()).with(
        COUNTS,
        Answer::Counts {
            left: own,
            right: theirs,
        },
    )
}

#[test]
fn branch_checkout_remote__tracks_the_remote_branch_when_no_local_branch_has_its_name() {
    let plan = plan(checkout_remote(&on_remote(repository(None)), &none(Answers::new())));
    assert_eq!(lines(&plan), ["git checkout --track origin/feature"]);
}

#[test]
fn branch_checkout_remote__checks_out_and_fast_forwards_a_branch_that_is_only_behind() {
    let context = on_remote(repository(Some(false)));
    assert_eq!(
        checkout_remote(&context, &Answers::new()),
        Outcome::Read(
            COUNTS,
            Query::Counts {
                left: String::from("refs/heads/feature"),
                right: String::from("origin/feature")
            }
        )
    );
    let plan = plan(checkout_remote(&context, &counted(0, 2)));
    assert_eq!(
        lines(&plan),
        ["git checkout feature", "git merge --ff-only origin/feature"]
    );
}

#[test]
fn branch_checkout_remote__only_fast_forwards_the_branch_checked_out() {
    let plan = plan(checkout_remote(&on_remote(repository(Some(true))), &counted(0, 2)));
    assert_eq!(lines(&plan), ["git merge --ff-only origin/feature"]);
}

#[test]
fn branch_checkout_remote__asks_when_each_has_commits_of_its_own() {
    let context = on_remote(repository(Some(false)));
    let asked = question(checkout_remote(&context, &counted(1, 2)));
    assert_eq!(
        asked.title.as_str(),
        "feature has 1 commits that origin/feature has not, which has 2 commits of its own"
    );
    let QuestionKind::Pick { choices } = asked.kind else {
        panic!("the way is picked")
    };
    let labels: Vec<&str> = choices.iter().map(|choice| choice.label.as_str()).collect();
    assert_eq!(
        labels,
        [
            "Check out feature and rebase it onto origin/feature",
            "Check out feature as it is"
        ]
    );
    let as_it_is = counted(1, 2).with(WAY, Answer::Choice(AS_IT_IS.to_owned()));
    assert_eq!(
        lines(&plan(checkout_remote(&context, &as_it_is))),
        ["git checkout feature"]
    );
    let rebase = counted(1, 2).with(WAY, Answer::Choice(REBASE.to_owned()));
    assert_eq!(
        lines(&plan(checkout_remote(&context, &rebase))),
        ["git rebase --autostash origin/feature feature"]
    );
}

#[test]
fn branch_checkout_remote__as_it_is_runs_nothing_on_the_branch_checked_out() {
    let context = on_remote(repository(Some(true)));
    let as_it_is = counted(1, 2).with(WAY, Answer::Choice(AS_IT_IS.to_owned()));
    let plan = plan(checkout_remote(&context, &as_it_is));
    assert_eq!(plan.steps, Vec::new());
}

#[test]
fn branch_checkout_remote__says_so_when_the_branch_checked_out_has_everything() {
    let plan = plan(checkout_remote(&on_remote(repository(Some(true))), &counted(0, 0)));
    assert_eq!(plan.steps, Vec::new());
    assert_eq!(
        plan.after,
        [FollowUp::Say(model::contract::Text::from(
            "feature is checked out already, with everything origin/feature has."
        ))]
    );
}

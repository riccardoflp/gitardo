use super::{PUBLISH, sync};
use crate::testing::{branch, context, lines, modified, plan, question, state, tracking};
use model::contract::{Answer, Answers, Context, Outcome, Target};
use model::state::RepositoryState;

/// `main` following `origin/main`, ahead and behind it by so many commits.
fn on_main(ahead: u32, behind: u32) -> RepositoryState {
    let mut state = state();
    state.remotes.push(String::from("origin"));
    if let Some(main) = state.branches.first_mut() {
        let mut upstream = tracking("origin", "main");
        upstream.ahead = ahead;
        upstream.behind = behind;
        main.upstream = Some(upstream);
    }
    state
}

fn synced(context: &Context) -> Vec<String> {
    lines(&plan(sync(context, &Answers::new())))
}

#[test]
fn sync_sync__fetches_pulls_and_pushes_in_that_order() {
    assert_eq!(
        synced(&context(on_main(1, 2), Target::Head)),
        [
            "git fetch --all --prune",
            "git pull --rebase --autostash",
            "git push origin main"
        ]
    );
}

#[test]
fn sync_sync__with_nothing_to_push_has_no_push() {
    assert_eq!(
        synced(&context(on_main(0, 2), Target::Head)),
        ["git fetch --all --prune", "git pull --rebase --autostash"]
    );
}

#[test]
fn sync_sync__runs_at_once_and_is_shown_first_only_when_command_preview_asks() {
    let plan = plan(sync(&context(on_main(1, 0), Target::Head), &Answers::new()));
    assert_eq!(plan.danger, model::contract::Danger::None);
}

#[test]
fn sync_sync__a_branch_with_no_upstream_asks_to_publish_it_and_only_pushes() {
    let mut state = on_main(0, 0);
    state.head.branch = Some(String::from("refs/heads/feature"));
    state.branches.push(branch("feature", 'a', true));
    let context = context(state, Target::Head);
    let publish = question(sync(&context, &Answers::new()));
    assert_eq!(
        publish.title.as_str(),
        "`feature` has no upstream. Publish it to `origin`?"
    );
    let answers = Answers::new().with(PUBLISH, Answer::Confirmed);
    assert_eq!(
        lines(&plan(sync(&context, &answers))),
        ["git push --set-upstream origin feature"]
    );
}

#[test]
fn sync_sync__refuses_to_rebase_over_changes_without_autostash() {
    let mut state = on_main(0, 1);
    state.working_tree.unstaged.push(modified("a.txt"));
    let mut context = context(state, Target::Head);
    context.settings.pull_autostash = false;
    let Outcome::Refuse(refusal) = sync(&context, &Answers::new()) else {
        panic!("the sync is refused")
    };
    assert_eq!(
        refusal.message.as_str(),
        "Commit or stash the changes first: the pull cannot rebase over them."
    );
}

#[test]
fn sync_sync__a_detached_head_is_refused_and_says_why() {
    let mut state = on_main(0, 0);
    state.head.branch = None;
    let Outcome::Refuse(refusal) = sync(&context(state, Target::Head), &Answers::new()) else {
        panic!("a detached HEAD is refused")
    };
    assert_eq!(
        refusal.message.as_str(),
        "Cannot sync: HEAD is detached. Check out a branch first."
    );
}

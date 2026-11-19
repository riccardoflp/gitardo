use model::contract::{Answer, Answers, FollowUp, Outcome, Query, QuestionKind, Target, Text, Undo, Validation};
use model::hosting::{PullList, PullListEntry};
use model::remote::Remote;
use model::state::RepositoryState;

use super::{LIST, NAME, NUMBER, PICK, REMOTES, TYPE_NUMBER, checkout_pull_request};
use crate::branch::submodules::none;
use crate::testing::{branch, context, lines, plan, question, state};

/// Answers with `origin` at a URL, and neither side of a checkout with submodules.
fn origin(url: &str) -> Answers {
    let origin = Remote {
        name: String::from("origin"),
        url: url.to_owned(),
    };
    none(Answers::new().with(REMOTES, Answer::Remotes(vec![origin])))
}

fn on_github() -> Answers {
    origin("https://github.com/owner/app.git")
}

/// #4 from the fork `contributor/app`, branch `fix-parser`, and #3 from the branch `feature` of the repository.
fn listed() -> Answer {
    let entry = |number: u64, branch: &str, fork: Option<&str>| PullListEntry {
        number,
        title: format!("Change {number}"),
        author: String::from("ada"),
        branch: branch.to_owned(),
        fork: fork.map(str::to_owned),
    };
    Answer::PullList(PullList::Listed(vec![
        entry(4, "fix-parser", Some("contributor/app")),
        entry(3, "feature", None),
    ]))
}

fn run(state: RepositoryState, answers: &Answers) -> Outcome {
    checkout_pull_request(&context(state, Target::Head), answers)
}

#[test]
fn hosting_checkout_pull_request__on_github_4_from_a_fork_proposes_owner_and_branch_and_fetches_its_reference() {
    assert_eq!(run(state(), &Answers::new()), Outcome::Read(REMOTES, Query::Remotes));
    assert!(matches!(
        run(state(), &on_github()),
        Outcome::Read(LIST, Query::PullRequestList { .. })
    ));
    let pick = question(run(state(), &on_github().with(LIST, listed())));
    let QuestionKind::Search { choices } = &pick.kind else {
        panic!("a search: {pick:?}");
    };
    let labels: Vec<_> = choices.iter().map(|choice| choice.label.as_str()).collect();
    assert_eq!(labels, ["#4 Change 4", "#3 Change 3", "Type its number\u{2026}"]);
    assert_eq!(
        choices.first().and_then(|choice| choice.description.clone()),
        Some(Text::from("ada \u{b7} contributor:fix-parser"))
    );
    let picked = on_github()
        .with(LIST, listed())
        .with(PICK, Answer::Choice(String::from("4")));
    let name = question(run(state(), &picked));
    assert_eq!(name.title.as_str(), "Check out #4 as");
    assert_eq!(name.default, Some(Answer::Text(String::from("contributor/fix-parser"))));
    let named = picked.with(NAME, Answer::Text(String::from("contributor/fix-parser")));
    let plan = plan(run(state(), &named));
    assert_eq!(
        lines(&plan),
        [
            "git fetch origin refs/pull/4/head:refs/heads/contributor/fix-parser",
            "git checkout contributor/fix-parser"
        ]
    );
    assert_eq!(
        plan.after.first(),
        Some(&FollowUp::SelectHead),
        "the tip of #4 is selected"
    );
    assert!(plan.after.contains(&FollowUp::Say(Text::from(
        "Checked out #4 as contributor/fix-parser. It has no upstream: a push would make a branch \
         contributor/fix-parser on origin."
    ))));
}

#[test]
fn hosting_checkout_pull_request__on_gitlab_7_typed_fetches_its_merge_request_reference() {
    let on_gitlab = origin("git@gitlab.com:group/app.git").with(LIST, Answer::PullList(PullList::Listed(Vec::new())));
    let number = question(run(state(), &on_gitlab));
    assert_eq!(number.title.as_str(), "Check out a merge request");
    assert!(matches!(
        number.kind,
        QuestionKind::Text {
            validation: Validation::PullRequestNumber,
            ..
        }
    ));
    let typed = on_gitlab.with(NUMBER, Answer::Text(String::from("!7")));
    assert_eq!(
        question(run(state(), &typed)).default,
        Some(Answer::Text(String::from("merge-request/7")))
    );
    let plan = plan(run(
        state(),
        &typed.with(NAME, Answer::Text(String::from("merge-request/7"))),
    ));
    assert_eq!(
        lines(&plan).first().map(String::as_str),
        Some("git fetch origin refs/merge-requests/7/head:refs/heads/merge-request/7")
    );
    assert_eq!(plan.title.as_str(), "Check out merge request !7");
    assert_eq!(
        Validation::PullRequestNumber.problem("seven"),
        Some(Text::from("Type the number of a pull request"))
    );
    assert_eq!(Validation::PullRequestNumber.problem("#7"), None);
}

#[test]
fn hosting_checkout_pull_request__3_from_a_branch_of_the_repository_fetches_it_and_checks_out_its_remote_branch() {
    let picked = on_github()
        .with(LIST, listed())
        .with(PICK, Answer::Choice(String::from("3")));
    let plan = plan(run(state(), &picked));
    assert_eq!(
        lines(&plan),
        ["git fetch origin feature", "git checkout --track origin/feature"]
    );
    let from_the_menu = context(state(), Target::RemoteBranch(String::from("origin/feature")));
    let given = on_github().with(NUMBER, Answer::Text(String::from("3")));
    let from_the_menu = checkout_pull_request(&from_the_menu, &given);
    assert_eq!(lines(&crate::testing::plan(from_the_menu)), lines(&plan));
}

#[test]
fn hosting_checkout_pull_request__4_again_brings_the_branch_up_to_the_pull_request() {
    let mut again = state();
    again.branches.push(branch("contributor/fix-parser", 'b', false));
    let picked = on_github()
        .with(LIST, listed())
        .with(PICK, Answer::Choice(String::from("4")));
    let name = question(run(again.clone(), &picked));
    let QuestionKind::Text { validation, .. } = &name.kind else {
        panic!("a box: {name:?}");
    };
    assert_eq!(
        validation.problem("contributor/fix-parser"),
        None,
        "a name a branch has is taken"
    );
    assert_eq!(
        validation.hint("contributor/fix-parser"),
        Some(Text::from(
            "contributor/fix-parser exists: it is brought up to the pull request if it can be fast-forwarded."
        ))
    );
    let named = picked.with(NAME, Answer::Text(String::from("contributor/fix-parser")));
    assert_eq!(
        lines(&plan(run(again, &named))),
        [
            "git fetch origin refs/pull/4/head:refs/heads/contributor/fix-parser",
            "git checkout contributor/fix-parser"
        ]
    );
    let mut on_it = state();
    on_it.branches = vec![branch("main", 'a', false), branch("contributor/fix-parser", 'b', true)];
    assert_eq!(
        lines(&plan(run(on_it, &named))),
        ["git pull --ff-only --no-rebase origin refs/pull/4/head"]
    );
}

#[test]
fn hosting_checkout_pull_request__undo_right_after_goes_back_by_the_reflog() {
    let picked = on_github()
        .with(LIST, listed())
        .with(PICK, Answer::Choice(String::from("3")));
    assert_eq!(plan(run(state(), &picked)).undo, Undo::ByReflog);
}

#[test]
fn hosting_checkout_pull_request__on_bitbucket_and_while_a_merge_is_under_way_it_is_refused() {
    let Outcome::Refuse(refusal) = run(state(), &origin("https://bitbucket.org/owner/app.git")) else {
        panic!("refused on Bitbucket");
    };
    assert_eq!(
        refusal.message.as_str(),
        "The main remote of this repository is not on GitHub, GitLab, Gitea or Forgejo."
    );
    let mut merging = state();
    merging.operation = Some(model::state::Operation::Merge);
    let Outcome::Refuse(refusal) = run(merging, &on_github()) else {
        panic!("refused while a merge is under way");
    };
    assert_eq!(refusal.message.as_str(), "Finish or abort the merge first.");
    let picked = on_github().with(PICK, Answer::Choice(TYPE_NUMBER.to_owned()));
    assert_eq!(question(run(state(), &picked)).key, NUMBER);
}

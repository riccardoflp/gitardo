use model::contract::{Answer, Answers, FollowUp, Outcome, Query, QuestionKind, Target, Text, Validation};
use model::github::{HelperAsked, Token};
use model::remote::Remote;

use super::{FOUND, REMOTES, TOKEN, load_pull_requests};
use crate::testing::{context, plan, question, state};

fn remote(name: &str, url: &str) -> Remote {
    Remote {
        name: name.to_owned(),
        url: url.to_owned(),
    }
}

fn on_github() -> Answers {
    Answers::new().with(
        REMOTES,
        Answer::Remotes(vec![remote("origin", "git@github.com:owner/app.git")]),
    )
}

fn load(answers: &Answers) -> Outcome {
    load_pull_requests(&context(state(), Target::Head), answers)
}

#[test]
fn github_load_pull_requests__the_token_is_read_allowing_git_credential_fill_only_once_chosen() {
    assert_eq!(load(&Answers::new()), Outcome::Read(REMOTES, Query::Remotes));
    assert_eq!(
        load(&on_github()),
        Outcome::Read(
            FOUND,
            Query::GitHubToken {
                helper: HelperAsked::Allowed
            }
        )
    );
}

#[test]
fn github_load_pull_requests__a_token_found_asks_nothing_keeps_it_and_reads_at_once() {
    let found = Token(String::from("from-helper"));
    let plan = plan(load(&on_github().with(FOUND, Answer::Token(Some(found.clone())))));
    assert!(plan.steps.is_empty(), "nothing changes in the repository");
    assert_eq!(
        plan.after,
        vec![FollowUp::KeepGitHubToken(found), FollowUp::ReadPullRequests]
    );
}

#[test]
fn github_load_pull_requests__with_no_token_found_it_asks_for_one_in_a_box_that_hides_it() {
    let asked = question(load(&on_github().with(FOUND, Answer::Token(None))));
    assert_eq!(asked.key, TOKEN);
    assert_eq!(asked.title, Text::from("GitHub token"));
    assert_eq!(
        asked.detail,
        Some(Text::from("A token that can read the pull requests of owner/app"))
    );
    let QuestionKind::Secret { validation } = asked.kind else {
        panic!("a secret is asked");
    };
    assert_eq!(validation, Validation::Required(Text::from("Type a token")));
    assert!(validation.problem("  ").is_some(), "it cannot be empty");
}

#[test]
fn github_load_pull_requests__the_token_typed_is_kept_and_the_pull_requests_read() {
    let answers = on_github()
        .with(FOUND, Answer::Token(None))
        .with(TOKEN, Answer::Text(String::from(" typed-token ")));
    let plan = plan(load(&answers));
    assert_eq!(
        plan.after,
        vec![
            FollowUp::KeepGitHubToken(Token(String::from("typed-token"))),
            FollowUp::ReadPullRequests
        ]
    );
}

#[test]
fn github_load_pull_requests__a_main_remote_on_another_host_is_refused_saying_why() {
    let answers = Answers::new().with(
        REMOTES,
        Answer::Remotes(vec![
            remote("origin", "https://gitlab.com/owner/app.git"),
            remote("upstream", "https://github.com/owner/app.git"),
        ]),
    );
    let Outcome::Refuse(refusal) = load(&answers) else {
        panic!("refused");
    };
    assert_eq!(
        refusal.message,
        Text::from("The main remote of this repository is not on github.com.")
    );
}

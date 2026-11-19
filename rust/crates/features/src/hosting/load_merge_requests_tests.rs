use model::contract::{Answer, Answers, FollowUp, Outcome, Query, QuestionKind, Target, Text};
use model::github::{HelperAsked, Token};
use model::remote::Remote;

use super::{FOUND, REMOTES, TOKEN, load_merge_requests};
use crate::testing::{context, plan, question, state};

fn origin(url: &str) -> Answers {
    let origin = Remote {
        name: String::from("origin"),
        url: url.to_owned(),
    };
    Answers::new().with(REMOTES, Answer::Remotes(vec![origin]))
}

fn on_gitlab() -> Answers {
    origin("git@gitlab.com:group/app.git")
}

fn load(answers: &Answers) -> Outcome {
    load_merge_requests(&context(state(), Target::Head), answers)
}

#[test]
fn hosting_load_merge_requests__the_token_is_read_allowing_git_credential_fill_only_once_chosen() {
    assert_eq!(load(&Answers::new()), Outcome::Read(REMOTES, Query::Remotes));
    assert_eq!(
        load(&on_gitlab()),
        Outcome::Read(
            FOUND,
            Query::HostingToken {
                host: String::from("gitlab.com"),
                helper: HelperAsked::Allowed
            }
        )
    );
    let found = Token(String::from("glpat-from-helper"));
    let plan = plan(load(&on_gitlab().with(FOUND, Answer::Token(Some(found.clone())))));
    assert!(plan.steps.is_empty(), "nothing changes in the repository");
    assert_eq!(
        plan.after,
        vec![
            FollowUp::KeepHostingToken(String::from("gitlab.com"), found),
            FollowUp::ReadPullRequests
        ]
    );
}

#[test]
fn hosting_load_merge_requests__with_no_token_found_it_asks_for_one_and_backing_out_keeps_nothing() {
    let none = on_gitlab().with(FOUND, Answer::Token(None));
    let asked = question(load(&none));
    assert_eq!(asked.key, TOKEN);
    assert_eq!(asked.title, Text::from("GitLab token"));
    assert_eq!(
        asked.detail,
        Some(Text::from(
            "A personal access token of gitlab.com with the scope read_api"
        ))
    );
    assert!(
        matches!(asked.kind, QuestionKind::Secret { .. }),
        "it hides what is typed"
    );
    let typed = plan(load(&none.with(TOKEN, Answer::Text(String::from(" glpat-typed ")))));
    assert_eq!(
        typed.after.first(),
        Some(&FollowUp::KeepHostingToken(
            String::from("gitlab.com"),
            Token(String::from("glpat-typed"))
        ))
    );
}

#[test]
fn hosting_load_merge_requests__in_a_repository_on_github_it_is_refused() {
    for url in ["https://github.com/owner/app.git", "https://bitbucket.org/team/app.git"] {
        assert!(matches!(load(&origin(url)), Outcome::Refuse(_)), "{url}");
    }
}

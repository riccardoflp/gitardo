//! A pull request checked out with real git (`hosting.checkout-pull-request`): the remote is a bare repository beside
//! the test's, that keeps the commits of pull request #4 under `refs/pull/4/head` as GitHub does, and the remotes are
//! given to the feature as if `origin` were on github.com. Nothing is asked of GitHub, and the credential store is in
//! memory.

use std::sync::Arc;

use engine::{Asker, CommandLog, Engine, EngineError, Finished, FixedClock, LogLevel, MemorySecrets};
use git::Git;
use git::read::repo::{StateRequest, read_state};
use model::Timestamp;
use model::contract::{Answer, Answers, Context, FeatureId, FeatureSettings, Plan, Question, Target};
use model::hosting::{PullList, PullListEntry};
use model::remote::Remote as RemoteOfModel;
use testkit::{Remote, Repository};

/// Answers each question with the answer it starts from, and runs every plan.
struct Defaults;

impl Asker for Defaults {
    fn ask(&mut self, question: &Question) -> Option<Answer> {
        question.default.clone()
    }

    fn confirm(&mut self, _plan: &Plan) -> bool {
        true
    }
}

/// A repository with `main` pushed to its bare `origin`, and #4 from a fork under `refs/pull/4/head` there.
fn repository() -> (Repository, Remote) {
    let mut repository = Repository::new();
    let _ = repository.commit("Add a", &[("a.txt", "a\n")]);
    let remote = Remote::new(&repository);
    let _ = repository.git(&["checkout", "--quiet", "-b", "fork-work"]);
    let _ = repository.commit("Fix the parser", &[("parser.txt", "fixed\n")]);
    let _ = repository.git(&["push", "--quiet", "origin", "fork-work:refs/pull/4/head"]);
    let _ = repository.git(&["checkout", "--quiet", "main"]);
    (repository, remote)
}

/// Checks out a pull request by its number, with `origin` taken for `owner/app` on github.com and #4 listed as coming
/// from the fork `contributor/app`.
fn check_out(repository: &Repository, number: &str) -> Result<Finished, EngineError> {
    let git = Git::new(repository.path());
    let mut engine = Engine::new(
        git.clone(),
        CommandLog::new(LogLevel::Debug),
        Box::new(FixedClock(Timestamp(0))),
    );
    engine.keep_secrets_in(Box::new(MemorySecrets::default()));
    let request = StateRequest {
        folder: repository.path().to_path_buf(),
    };
    let Ok(state) = read_state(&git, &request) else {
        panic!("the state of the repository is read");
    };
    let context = Context {
        repository: Arc::new(state),
        target: Target::Head,
        settings: FeatureSettings::default(),
        now: Timestamp(0),
    };
    let origin = RemoteOfModel {
        name: String::from("origin"),
        url: String::from("https://github.com/owner/app.git"),
    };
    let fork = PullListEntry {
        number: 4,
        title: String::from("Fix the parser"),
        author: String::from("contributor"),
        branch: String::from("fix-parser"),
        fork: Some(String::from("contributor/app")),
    };
    let answers = Answers::new()
        .with(features::hosting::REMOTES, Answer::Remotes(vec![origin]))
        .with(features::hosting::LIST, Answer::PullList(PullList::Listed(vec![fork])))
        .with(features::hosting::NUMBER, Answer::Text(number.to_owned()));
    let feature = FeatureId("hosting.checkout-pull-request");
    engine.run_feature_from(
        feature,
        features::hosting::checkout_pull_request,
        (&context, answers),
        &mut Defaults,
    )
}

#[test]
fn hosting_checkout_pull_request__4_from_a_fork_is_fetched_by_its_reference_and_again_brought_up_to_its_new_commits() {
    let (mut repository, _remote) = repository();
    let tip = repository.git(&["rev-parse", "fork-work"]);
    let Ok(Finished::Ran { steps, .. }) = check_out(&repository, "4") else {
        panic!("the plan ran");
    };
    assert!(
        steps
            .iter()
            .any(|line| line == "git fetch origin refs/pull/4/head:refs/heads/contributor/fix-parser")
    );
    assert_eq!(
        repository.git(&["branch", "--show-current"]).trim(),
        "contributor/fix-parser"
    );
    assert_eq!(repository.git(&["rev-parse", "HEAD"]), tip);
    let _ = repository.git(&["checkout", "--quiet", "fork-work"]);
    let newer = repository.commit("Fix the parser again", &[("parser.txt", "fixed again\n")]);
    let _ = repository.git(&["push", "--quiet", "origin", "fork-work:refs/pull/4/head"]);
    let _ = repository.git(&["checkout", "--quiet", "main"]);
    assert!(matches!(check_out(&repository, "#4"), Ok(Finished::Ran { .. })));
    assert_eq!(
        repository.git(&["rev-parse", "contributor/fix-parser"]).trim(),
        newer.trim()
    );
}

#[test]
fn hosting_checkout_pull_request__a_number_with_no_pull_request_says_so_and_no_branch_is_made() {
    let (repository, _remote) = repository();
    let Err(EngineError::Step(failure)) = check_out(&repository, "99") else {
        panic!("the fetch fails");
    };
    // A step speaks the user's language, so only the name of the ref, which git says in every language, is looked for.
    assert!(failure.said.contains("refs/pull/99/head"), "{}", failure.said);
    assert_eq!(failure.plan, "Check out pull request #99");
    assert!(
        repository.git(&["branch", "--list", "pull/99"]).trim().is_empty(),
        "no branch is made"
    );
    assert_eq!(repository.git(&["branch", "--show-current"]).trim(), "main");
}

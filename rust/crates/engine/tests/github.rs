//! The GitHub token as the engine reads it for a feature (`github.token`): from its credential store, or from git's
//! credential helper, with what the log says of it. The helper is the test's own, in a throwaway repository, and the
//! store is in memory: neither the user's helper nor the user's store is touched.

#![cfg(test)]
#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

use std::fs;
use std::sync::Arc;

use engine::SecretStore as _;
use engine::{Asker, CommandLog, Engine, FixedClock, GITHUB_TOKEN, LogKind, LogLevel, MemorySecrets};
use git::Git;
use git::read::repo::{StateRequest, read_state};
use model::Timestamp;
use model::contract::{
    Answer, AnswerKey, Answers, Context, Danger, FeatureId, FeatureSettings, FollowUp, Outcome, Plan, Query, Question,
    Target, Text, Undo,
};
use model::github::HelperAsked;
use testkit::Repository;

const TOKEN: AnswerKey = AnswerKey("token");

/// What the helper of the test prints for `get`.
const HELPER: &str = "!f() { echo username=ada; echo password=token-from-helper; }; f";

struct RunsAll;

impl Asker for RunsAll {
    fn ask(&mut self, _question: &Question) -> Option<Answer> {
        None
    }

    fn confirm(&mut self, _plan: &Plan) -> bool {
        true
    }
}

/// A feature that reads the token, allowed to ask git's helper, and keeps the token it found.
fn read_and_keep(_context: &Context, answers: &Answers) -> Outcome {
    match answers.get(TOKEN) {
        Some(Answer::Token(token)) => Outcome::Run(Plan {
            title: Text::from("Keep the token"),
            steps: Vec::new(),
            danger: Danger::None,
            undo: Undo::Impossible(Text::from("A test plan.")),
            after: token.iter().cloned().map(FollowUp::KeepGitHubToken).collect(),
            on_failure: Vec::new(),
        }),
        _ => Outcome::Read(
            TOKEN,
            Query::GitHubToken {
                helper: HelperAsked::Allowed,
            },
        ),
    }
}

/// A repository whose only credential helper is the test's, and an engine for it that reads no configuration of the
/// system or of the user, logs at the debug level and keeps secrets in memory.
fn setup() -> (Repository, Engine, MemorySecrets, Context) {
    let repository = Repository::new();
    let _ = repository.git(&["config", "--add", "credential.helper", ""]);
    let _ = repository.git(&["config", "--add", "credential.helper", HELPER]);
    let global = repository.path().parent().unwrap().join("empty.gitconfig");
    fs::write(&global, "").unwrap();
    let git = Git::new(repository.path())
        .with_variable("GIT_CONFIG_NOSYSTEM", "1")
        .with_variable("GIT_CONFIG_GLOBAL", global.as_os_str());
    let mut engine = Engine::new(
        git.clone(),
        CommandLog::new(LogLevel::Debug),
        Box::new(FixedClock(Timestamp(0))),
    );
    let secrets = MemorySecrets::default();
    engine.keep_secrets_in(Box::new(secrets.clone()));
    let request = StateRequest {
        folder: repository.path().to_path_buf(),
    };
    let context = Context {
        repository: Arc::new(read_state(&git, &request).unwrap()),
        target: Target::Head,
        settings: FeatureSettings::default(),
        now: Timestamp(0),
    };
    (repository, engine, secrets, context)
}

fn lines(engine: &Engine) -> Vec<(LogKind, String)> {
    engine
        .log()
        .entries()
        .map(|entry| (entry.kind.clone(), entry.line.clone()))
        .collect()
}

#[test]
fn github_token__the_command_log_at_debug_shows_git_credential_fill_never_what_it_printed() {
    let (_repository, mut engine, secrets, context) = setup();
    let _ran = engine
        .run_feature(FeatureId("github.token"), read_and_keep, &context, &mut RunsAll)
        .unwrap();
    let logged = lines(&engine);
    assert_eq!(logged.len(), 1, "{logged:?}");
    assert!(matches!(logged[0].0, LogKind::Read(_)));
    assert_eq!(logged[0].1, "git credential fill");
    assert!(logged.iter().all(|(_, line)| !line.contains("token-from-helper")));
    assert_eq!(
        secrets.kept().get(GITHUB_TOKEN).map(String::as_str),
        Some("token-from-helper"),
        "the token found is kept"
    );
}

#[test]
fn github_token__a_token_kept_in_the_store_is_read_without_git() {
    let (_repository, mut engine, secrets, context) = setup();
    secrets.write(GITHUB_TOKEN, "kept-token").unwrap();
    let _ran = engine
        .run_feature(FeatureId("github.token"), read_and_keep, &context, &mut RunsAll)
        .unwrap();
    assert_eq!(lines(&engine), []);
    assert_eq!(secrets.kept().get(GITHUB_TOKEN).map(String::as_str), Some("kept-token"));
}

//! What the tests of the operations share: contexts made from a few facts.

use std::path::PathBuf;
use std::sync::Arc;

use model::contract::{Context, FeatureSettings, Outcome, Plan, Question, Target};
use model::state::{Branch, ChangeKind, FileChange, Head, RepositoryState, Tag};
use model::{CommitId, Timestamp};

/// A commit id made of one digit repeated: `commit('a')` is `aaaa…`.
pub fn commit(digit: char) -> CommitId {
    CommitId::parse(&digit.to_string().repeat(40)).unwrap()
}

/// A repository in `repository/` on `main` at `commit('a')`, with nothing else.
pub fn state() -> RepositoryState {
    RepositoryState {
        folder: PathBuf::from("repository"),
        git_folder: PathBuf::from("repository/.git"),
        head: Head {
            commit: Some(commit('a')),
            branch: Some(String::from("refs/heads/main")),
        },
        branches: vec![branch("main", 'a', true)],
        ..RepositoryState::default()
    }
}

/// A local branch.
pub fn branch(name: &str, digit: char, is_checked_out: bool) -> Branch {
    Branch {
        name: name.to_owned(),
        commit: commit(digit),
        upstream: None,
        is_checked_out,
        date: Timestamp(0),
    }
}

/// A tag.
pub fn tag(name: &str, digit: char) -> Tag {
    Tag {
        name: name.to_owned(),
        commit: commit(digit),
        date: Timestamp(0),
    }
}

/// A file modified.
pub fn modified(path: &str) -> FileChange {
    FileChange {
        path: path.to_owned(),
        original_path: None,
        kind: ChangeKind::Modified,
    }
}

/// The context of a feature invoked on `target` in `state`, with the default settings.
pub fn context(state: RepositoryState, target: Target) -> Context {
    Context {
        repository: Arc::new(state),
        target,
        settings: FeatureSettings::default(),
        now: Timestamp(0),
    }
}

/// The question of an outcome that asks one.
pub fn question(outcome: Outcome) -> Question {
    match outcome {
        Outcome::Ask(question) => question,
        other => panic!("expected a question, got {other:?}"),
    }
}

/// The plan of an outcome that runs one.
pub fn plan(outcome: Outcome) -> Plan {
    match outcome {
        Outcome::Run(plan) => plan,
        other => panic!("expected a plan, got {other:?}"),
    }
}

/// The lines of the steps of a plan, as the log writes them.
pub fn lines(plan: &Plan) -> Vec<String> {
    plan.steps.iter().map(model::contract::Step::log_line).collect()
}

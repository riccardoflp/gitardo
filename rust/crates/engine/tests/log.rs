//! The command log of a plan run in a real repository (`log.record`): each step as it ran, the reads only at the debug
//! level, and what git said under a step that failed, with nothing after it.

#![cfg(test)]
#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

use std::sync::Arc;

use engine::{Asker, CommandLog, Engine, FixedClock, LogKind, LogLevel};
use git::Git;
use git::read::repo::{StateRequest, read_state};
use model::Timestamp;
use model::contract::{
    Answer, AnswerKey, Answers, Context, Danger, FeatureId, FeatureSettings, Outcome, Plan, Query, Question, Refusal,
    Step, Target, Text, Undo,
};
use testkit::Repository;

/// The key of the read of the reflog.
const REFLOG: AnswerKey = AnswerKey("reflog");

/// An asker that answers nothing and confirms every plan.
struct RunsAll;

impl Asker for RunsAll {
    fn ask(&mut self, _question: &Question) -> Option<Answer> {
        None
    }

    fn confirm(&mut self, _plan: &Plan) -> bool {
        true
    }
}

/// A plan of some steps.
fn plan(steps: Vec<Step>) -> Plan {
    Plan {
        title: Text::from("Tag and branch"),
        steps,
        danger: Danger::None,
        undo: Undo::Impossible(Text::from("A test plan.")),
        after: Vec::new(),
        on_failure: Vec::new(),
    }
}

/// An engine for a repository, whose log records what `level` says.
fn engine(repository: &Repository, level: LogLevel) -> Engine {
    Engine::new(
        Git::new(repository.path()),
        CommandLog::new(level),
        Box::new(FixedClock(Timestamp(0))),
    )
}

/// The lines of the log, each with its kind.
fn lines(engine: &Engine) -> Vec<(LogKind, String)> {
    engine
        .log()
        .entries()
        .map(|entry| (entry.kind.clone(), entry.line.clone()))
        .collect()
}

/// A feature that reads the reflog of `HEAD`, then refuses with nothing to do.
fn read_reflog(_context: &Context, answers: &Answers) -> Outcome {
    if answers.get(REFLOG).is_some() {
        Outcome::Refuse(Refusal::saying("Nothing to do"))
    } else {
        Outcome::Read(
            REFLOG,
            Query::Reflog {
                reference: String::from("HEAD"),
                count: 5,
            },
        )
    }
}

#[test]
fn log_record__every_command_that_changes_the_repository_is_logged_exactly_as_it_ran() {
    let mut repository = Repository::new();
    let _first = repository.commit("Add a", &[("a.txt", "a")]);
    let folder = repository.path().to_path_buf();
    let mut engine = engine(&repository, LogLevel::Info);
    let steps = vec![
        Step::git(folder.clone(), ["tag", "-a", "v1.0", "-m", "First release"]),
        Step::git(folder.clone(), ["branch", "fix/login"]),
    ];
    let ran = engine.run_plan(&plan(steps)).unwrap();
    assert_eq!(ran.len(), 2);
    assert_eq!(
        lines(&engine),
        [
            (LogKind::Step, String::from("git tag -a v1.0 -m \"First release\"")),
            (LogKind::Step, String::from("git branch fix/login")),
        ]
    );
    assert_eq!(repository.git(&["tag", "--list"]).trim(), "v1.0");
}

#[test]
fn log_record__reads_are_logged_only_at_debug() {
    let mut repository = Repository::new();
    let _first = repository.commit("Add a", &[("a.txt", "a")]);
    let request = StateRequest {
        folder: repository.path().to_path_buf(),
    };
    let context = Context {
        repository: Arc::new(read_state(&Git::new(repository.path()), &request).unwrap()),
        target: Target::Head,
        settings: FeatureSettings::default(),
        now: Timestamp(0),
    };
    let mut info = engine(&repository, LogLevel::Info);
    let _refused = info
        .run_feature(FeatureId("read"), read_reflog, &context, &mut RunsAll)
        .unwrap();
    assert_eq!(lines(&info), []);
    let mut debug = engine(&repository, LogLevel::Debug);
    let _refused = debug
        .run_feature(FeatureId("read"), read_reflog, &context, &mut RunsAll)
        .unwrap();
    let logged = lines(&debug);
    assert_eq!(logged.len(), 1, "{logged:?}");
    assert!(matches!(logged[0].0, LogKind::Read(_)), "{logged:?}");
}

#[test]
fn log_record__a_step_that_fails_has_its_error_under_it_and_the_steps_after_it_are_not_run() {
    let mut repository = Repository::new();
    let _first = repository.commit("Add a", &[("a.txt", "a")]);
    let folder = repository.path().to_path_buf();
    let mut engine = engine(&repository, LogLevel::Info);
    let steps = vec![
        Step::git(folder.clone(), ["checkout", "no-such-branch"]),
        Step::Write {
            path: folder.join("after.txt"),
            contents: b"written".to_vec(),
        },
    ];
    let failure = engine.run_plan(&plan(steps)).unwrap_err();
    assert_eq!(failure.ran, Vec::<String>::new());
    let logged = lines(&engine);
    assert_eq!(logged.len(), 2, "{logged:?}");
    assert_eq!(logged[0], (LogKind::Step, String::from("git checkout no-such-branch")));
    assert_eq!(logged[1].0, LogKind::Failure);
    assert!(logged[1].1.contains("no-such-branch"), "{logged:?}");
    assert!(!folder.join("after.txt").exists());
}

#[test]
fn log_record__what_git_printed_under_a_step_that_succeeded_is_kept_and_the_input_beside_the_step() {
    let mut repository = Repository::new();
    let _first = repository.commit("Add a", &[("a.txt", "a")]);
    let folder = repository.path().to_path_buf();
    std::fs::write(folder.join("b.txt"), "b").unwrap();
    let mut engine = engine(&repository, LogLevel::Info);
    let steps = vec![
        Step::git(folder.clone(), ["add", "--", "b.txt"]),
        Step::Git {
            arguments: ["commit", "--quiet", "-F", "-"].into_iter().map(Into::into).collect(),
            directory: folder.clone(),
            input: Some(b"Add b\n\nWith its body\n".to_vec()),
        },
        Step::git(folder.clone(), ["log", "-1", "--format=%s"]),
    ];
    let _ran = engine.run_plan(&plan(steps)).unwrap();
    assert_eq!(
        lines(&engine),
        [
            (LogKind::Step, String::from("git add -- b.txt")),
            (LogKind::Step, String::from("git commit --quiet -F -")),
            (LogKind::Input, String::from("  Add b")),
            (LogKind::Input, String::from("  ")),
            (LogKind::Input, String::from("  With its body")),
            (LogKind::Step, String::from("git log -1 --format=%s")),
            (LogKind::Output, String::from("Add b")),
        ]
    );
}

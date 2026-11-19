//! The progress of a step that asks git for it (`--progress`): the last line git printed on its standard error is the
//! progress of the activity of the plan while the step runs (`activity.track`, `activity.describe-task`).

#![cfg(test)]
#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

use std::thread;
use std::time::{Duration, Instant};

use engine::{Asker, CommandLog, Engine, FixedClock, LogLevel};
use git::Git;
use model::Timestamp;
use model::contract::{
    Answer, Answers, Context, Danger, FeatureId, FeatureSettings, Outcome, Plan, Question, Step, Target, Text, Undo,
};
use model::state::RepositoryState;
use testkit::Repository;

/// Runs every plan and answers no question.
struct RunsAll;

impl Asker for RunsAll {
    fn ask(&mut self, _question: &Question) -> Option<Answer> {
        None
    }

    fn confirm(&mut self, _plan: &Plan) -> bool {
        true
    }
}

/// A plan of one git step that prints a line of progress, then another over it, then waits: an alias of the shell
/// that git runs, given `--progress` as a step that asks git for its progress is.
fn printing_progress(context: &Context, _answers: &Answers) -> Outcome {
    let alias = "alias.slow=!printf 'Receiving objects:  12%% (321/2671)\\r' >&2; sleep 1; \
                 printf 'Receiving objects:  45%% (1203/2671)\\r' >&2; sleep 2; true";
    Outcome::Run(Plan {
        title: Text::from("Clone lib"),
        steps: vec![Step::git(
            context.repository.folder.clone(),
            ["-c", alias, "slow", "--progress"],
        )],
        danger: Danger::None,
        undo: Undo::Impossible(Text::from("A test plan.")),
        after: Vec::new(),
        on_failure: Vec::new(),
    })
}

#[test]
fn activity_track__a_plan_whose_git_step_asks_for_its_progress_says_the_last_line_git_printed_while_it_runs() {
    let repository = Repository::new();
    let folder = repository.path().to_path_buf();
    let context = Context {
        repository: std::sync::Arc::new(RepositoryState {
            folder: folder.clone(),
            ..RepositoryState::default()
        }),
        target: Target::Head,
        settings: FeatureSettings::default(),
        now: Timestamp(0),
    };
    let (sender, receiver) = std::sync::mpsc::channel();
    let running = thread::spawn(move || {
        let mut engine = Engine::new(
            Git::new(&folder),
            CommandLog::new(LogLevel::Info),
            Box::new(FixedClock(Timestamp(0))),
        );
        sender.send(engine.activities().clone()).unwrap();
        engine
            .run_feature(FeatureId("repo.clone"), printing_progress, &context, &mut RunsAll)
            .is_ok()
    });
    let activities = receiver.recv().unwrap();
    let started = Instant::now();
    let mut seen = Vec::new();
    while started.elapsed() < Duration::from_secs(10) {
        thread::sleep(Duration::from_millis(50));
        let running = activities.running();
        let Some(entry) = running.first() else {
            if seen.is_empty() {
                continue;
            }
            break;
        };
        if let Some(progress) = &entry.progress
            && seen.last() != Some(progress)
        {
            seen.push(progress.clone());
        }
    }
    assert!(running.join().unwrap(), "the plan ends well");
    assert_eq!(
        seen,
        [
            "Receiving objects:  12% (321/2671)",
            "Receiving objects:  45% (1203/2671)"
        ]
    );
}

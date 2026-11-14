//! The journal of Undo kept by the engine as plans run on a real repository, and dropped once `HEAD` moves from a
//! terminal.

#![cfg(test)]
#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

use std::sync::Arc;

use engine::{Asker, CommandLog, Engine, Finished, FixedClock, LogLevel};
use git::Git;
use git::read::repo::{StateRequest, read_state};
use model::contract::{
    Answer, AnswerKey, Answers, Context, Danger, FeatureId, FeatureSettings, Outcome, Plan, Query, Question, Refusal,
    Step, Target, Text, Undo, UndoRecord,
};
use model::state::RepositoryState;
use model::{CommitId, Timestamp};
use testkit::Repository;

const JOURNAL: AnswerKey = AnswerKey("journal");

/// Answers nothing and runs every plan.
struct RunsAll;

impl Asker for RunsAll {
    fn ask(&mut self, _question: &Question) -> Option<Answer> {
        None
    }

    fn confirm(&mut self, _plan: &Plan) -> bool {
        true
    }
}

/// Deletes the branch `old`, leaving what Undo needs in the journal.
fn delete_old(context: &Context, _answers: &Answers) -> Outcome {
    let Some(branch) = context.repository.branches.iter().find(|branch| branch.name == "old") else {
        return Outcome::Refuse(Refusal::saying("No branch old"));
    };
    Outcome::Run(Plan {
        title: Text::from("Delete branch old"),
        steps: vec![Step::git(context.repository.folder.clone(), ["branch", "-D", "old"])],
        danger: Danger::None,
        undo: Undo::Journal(UndoRecord::BranchDeleted {
            name: String::from("old"),
            commit: branch.commit.clone(),
        }),
        after: Vec::new(),
        on_failure: Vec::new(),
    })
}

/// Says, as a refusal, the title of the newest entry of the journal, or that there is none.
fn tell_journal(_context: &Context, answers: &Answers) -> Outcome {
    match answers.journal(JOURNAL) {
        None => Outcome::Read(JOURNAL, Query::Journal),
        Some(entry) => Outcome::Refuse(Refusal::saying(entry.map_or("none", |entry| entry.title.as_str()))),
    }
}

fn state(repository: &Repository) -> Arc<RepositoryState> {
    let request = StateRequest {
        folder: repository.path().to_path_buf(),
    };
    Arc::new(read_state(&Git::new(repository.path()), &request).unwrap())
}

fn context(repository: &Repository) -> Context {
    Context {
        repository: state(repository),
        target: Target::Head,
        settings: FeatureSettings::default(),
        now: Timestamp(0),
    }
}

fn journal_says(engine: &mut Engine, repository: &Repository) -> String {
    let finished = engine
        .run_feature(FeatureId("tell"), tell_journal, &context(repository), &mut RunsAll)
        .unwrap();
    let Finished::Refused(refusal) = finished else {
        panic!("the journal is told")
    };
    refusal.message.as_str().to_owned()
}

#[test]
fn undo_journal__keeps_a_deleted_branch_until_head_moves_from_a_terminal() {
    let mut repository = Repository::new();
    let _first = repository.commit("Add a", &[("a.txt", "a")]);
    repository.branch("old");
    let mut engine = Engine::new(
        Git::new(repository.path()),
        CommandLog::new(LogLevel::Info),
        Box::new(FixedClock(Timestamp(0))),
    );
    let finished = engine
        .run_feature(
            FeatureId("branch.delete"),
            delete_old,
            &context(&repository),
            &mut RunsAll,
        )
        .unwrap();
    let Finished::Ran { state_after, .. } = finished else {
        panic!("the branch is deleted")
    };
    assert!(state_after.is_some_and(|state| state.branches.iter().all(|branch| branch.name != "old")));
    let entry = engine.journal().newest().map(|entry| entry.record.clone());
    let Some(UndoRecord::BranchDeleted { name, commit }) = entry else {
        panic!("the deletion is in the journal")
    };
    assert_eq!(name, "old");
    assert_eq!(
        Some(commit),
        CommitId::parse(repository.git(&["rev-parse", "HEAD"]).trim_end())
    );
    assert_eq!(journal_says(&mut engine, &repository), "Delete branch old");

    let _terminal = repository.commit("Add b", &[("b.txt", "b")]);
    assert_eq!(journal_says(&mut engine, &repository), "none");
}

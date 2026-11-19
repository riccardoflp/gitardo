use std::path::PathBuf;

use model::CommitId;
use model::range_diff::{Line, Pair, PairKind, RangeDiff, RangeDiffTarget, Ranges};
use views::range_diff::{RangeDiffIntent, RangeDiffRead};

use super::{open, page_state, pairs_read, reduce};
use crate::persisted::Persisted;
use crate::settings::Settings;
use crate::store::message::JobKind;
use crate::store::{Effect, State};

/// A commit named by a digit.
fn commit(digit: char) -> CommitId {
    CommitId::parse(&digit.to_string().repeat(40)).unwrap()
}

/// What the rebase of `feature` onto `main` changed.
fn target() -> RangeDiffTarget {
    let line = |to: &str| Line {
        from: String::from("main"),
        to: to.to_owned(),
    };
    RangeDiffTarget {
        title: String::from("What the rebase of feature changed"),
        branch: String::from("feature"),
        ranges: Ranges {
            arguments: ["main", "old", "feature"].map(String::from).to_vec(),
            before: line("old"),
            now: line("feature"),
        },
    }
}

/// `B` reworded from `2…` into `6…`.
fn pairs() -> RangeDiff {
    RangeDiff {
        pairs: vec![Pair {
            kind: PairKind::Changed,
            before: Some(commit('2')),
            now: Some(commit('6')),
            subject: String::from("Add B, better"),
            diff: vec![String::from("-    Add B"), String::from("+    Add B, better")],
        }],
        before_count: 1,
        now_count: 1,
    }
}

#[test]
fn range_diff_page__a_click_on_the_short_hash_of_b_now_opens_the_page_of_that_commit() {
    let mut state = State::new(Persisted::default(), Settings::default(), None);
    let folder = PathBuf::from("work");
    let effects = open(&mut state, folder.clone(), target());
    let Some(Effect::Start(job)) = effects.iter().find(|effect| matches!(effect, Effect::Start(_))) else {
        panic!("reads the pairs: {effects:?}")
    };
    assert!(matches!(&job.kind, JobKind::ReadRangeDiff { request, .. } if request.ranges == target().ranges));
    assert_eq!(page_state(&state).map(|page| page.read), Some(RangeDiffRead::Comparing));
    let _read = pairs_read(&mut state, job.id, Ok(pairs()));
    let _opened = reduce(&mut state, RangeDiffIntent::Toggle(0));
    assert_eq!(page_state(&state).map(|page| page.opened.to_vec()), Some(vec![0]));
    let effects = reduce(&mut state, RangeDiffIntent::OpenCommit(commit('6')));
    let opens = effects.iter().any(|effect| {
        matches!(effect, Effect::Start(job) if matches!(&job.kind, JobKind::ReadCommitPage { commit: read, .. } if *read == commit('6')))
    });
    assert!(opens, "opens the page of the commit now: {effects:?}");
}

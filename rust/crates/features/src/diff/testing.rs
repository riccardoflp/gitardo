//! What the tests of the operations on a change share: a change of a page, and the versions of its file as read.

use std::ops::Range;

use model::contract::{Answer, Answers, Context, Outcome, Target};
use model::diff::{DiffChange, DiffSide, FileVersions};

use super::VERSIONS;
use crate::testing::{context, state};

/// A change of `src/app.rs` between two sides, at some lines of each, with the text a page showed there.
pub fn change(
    sides: (DiffSide, DiffSide),
    left: (Range<usize>, &[&str]),
    right: (Range<usize>, &[&str]),
) -> DiffChange {
    let owned = |lines: &[&str]| lines.iter().map(|line| (*line).to_owned()).collect();
    DiffChange {
        path: String::from("src/app.rs"),
        left: sides.0,
        right: sides.1,
        left_lines: left.0,
        right_lines: right.0,
        left_text: owned(left.1),
        right_text: owned(right.1),
    }
}

/// The versions of the file as bytes: in `HEAD` (or the commit asked for), in the index and on disk.
pub fn versions_of(commit: Option<&[u8]>, index: Option<&[u8]>, working_tree: Option<&[u8]>) -> Answers {
    let bytes = |text: Option<&[u8]>| text.map(<[u8]>::to_vec);
    Answers::new().with(
        VERSIONS,
        Answer::FileVersions(FileVersions {
            commit: bytes(commit),
            index: bytes(index),
            working_tree: bytes(working_tree),
        }),
    )
}

/// The versions of the file as text.
pub fn versions(commit: Option<&str>, index: Option<&str>, working_tree: Option<&str>) -> Answers {
    versions_of(
        commit.map(str::as_bytes),
        index.map(str::as_bytes),
        working_tree.map(str::as_bytes),
    )
}

/// The context of an operation invoked on a change, in the repository of the tests.
pub fn on(change: DiffChange) -> Context {
    context(state(), Target::DiffChange(Box::new(change)))
}

/// The sentence of an outcome that refuses.
pub fn refusal(outcome: Outcome) -> String {
    match outcome {
        Outcome::Refuse(refusal) => refusal.message.as_str().to_owned(),
        other => panic!("expected a refusal, got {other:?}"),
    }
}

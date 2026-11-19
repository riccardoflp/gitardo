//! The reads of the details of a commit, of its lines, of comparisons, of the two versions of a file, of a file at a
//! revision, of what a stash holds and of patches, against throwaway repositories and on golden files.
//!
//! The acceptance lines about the UI are tested with the views; each other line has a test here named after its card.

// The helpers of these tests fail as the tests do, loudly: clippy allows them what it allows tests.
#![cfg(test)]
#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

#[path = "details_and_diffs/awkward.rs"]
mod awkward;
#[path = "details_and_diffs/compare.rs"]
mod compare;
#[path = "details_and_diffs/details.rs"]
mod details;
#[path = "details_and_diffs/files.rs"]
mod files;
#[path = "details_and_diffs/golden.rs"]
mod golden;
#[path = "details_and_diffs/patch.rs"]
mod patch;
#[path = "details_and_diffs/signature.rs"]
mod signature;

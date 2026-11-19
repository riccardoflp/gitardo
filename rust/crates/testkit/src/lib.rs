//! What the tests of Gitardo Desktop share: throwaway repositories, golden files, a seeded random source and PNG images.
//!
//! - [`Repository`] is an empty git repository in a folder of its own under the temporary folder, with a fixed
//!   identity and fixed dates, deleted when it is dropped; [`Remote`] is a bare repository beside it.
//! - [`assert_golden`] compares what a test produced with a file recorded before, and records it again when
//!   `GITARDO_GOLDEN_RECORD=1` is set.
//! - [`Random`] is a seeded random source that says its seed when a test fails, so that `GITARDO_TEST_SEED` can run it
//!   again with the same numbers.
//! - [`png`] writes and reads the PNG images of the view snapshots, and draws the differences between two of them.
//!
//! This crate is only ever a `dev-dependency`: it is never shipped. That is why it may run `git` itself with
//! `std::process` and make and delete folders with `std::fs`, which only `crates/git` and `crates/engine` do in the
//! program: it sets up what a test needs, it is not part of what the test checks.

#![forbid(unsafe_code)]
// This crate runs only inside tests, where a step that fails must fail the test at once and loudly: a panic with a
// message is the right way to say it, as it is in the tests themselves.
#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

mod environment;
mod error;
mod folder;
mod golden;
pub mod png;
mod random;
mod remote;
mod repository;

pub use environment::FIRST_COMMIT_TIME;
pub use error::Error;
pub use golden::assert_golden;
pub use random::Random;
pub use remote::Remote;
pub use repository::{CONFLICT_BRANCH, CONFLICT_FILE, Repository, SigningKey};

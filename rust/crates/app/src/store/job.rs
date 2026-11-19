//! A job for a worker, and its number.

use super::message::JobKind;

/// The number of a job, unique in its window.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct JobId(pub u64);

/// A job for a worker.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Job {
    /// Its number, which its result carries back.
    pub id: JobId,
    /// What it does.
    pub kind: JobKind,
}

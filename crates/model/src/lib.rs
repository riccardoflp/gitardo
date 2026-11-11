//! The data of Gitardo Desktop and what is derived from it: commits, references, lanes, file trees, names and messages;
//! and the types every feature speaks in, its context, its questions and its plans (decision D14).

#![forbid(unsafe_code)]

pub mod branch;
pub mod contract;
mod identifier;
pub mod state;

pub use identifier::{CommitId, Timestamp};

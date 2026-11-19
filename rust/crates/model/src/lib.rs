//! The data of Gitardo Desktop and what is derived from it: commits, references, lanes, file trees, names and messages;
//! and the types every feature speaks in, its context, its questions and its plans (decision D14).

#![forbid(unsafe_code)]

pub mod activity;
pub mod ai;
pub mod bisect;
pub mod blame;
pub mod branch;
pub mod changes;
pub mod commit;
pub mod commit_page;
pub mod compare;
pub mod conflicts;
pub mod contract;
pub mod details;
pub mod diff;
pub mod drag;
pub mod file_history;
pub mod file_search;
pub mod file_view;
pub mod filter;
pub mod github;
pub mod graph;
pub mod help;
mod identifier;
pub mod markdown;
pub mod merge_view;
pub mod patch;
pub mod rebase;
pub mod reflog;
pub mod refs;
pub mod remote;
pub mod repo;
pub mod review;
pub mod search;
pub mod squash;
pub mod stash;
pub mod state;
pub mod submodule;
pub mod tasks;
pub mod terminal;
pub mod trail;
pub mod trust;
pub mod worktree;
pub mod worktree_setup;

pub use identifier::{CommitId, Timestamp};

//! Git itself: running it, the reads and their parsers, and finding repositories.
//!
//! This is the one crate that runs git (rule R1.4). A read is split into the arguments it gives git and the parser of
//! what git prints, both pure, so that the parsers are tested on output recorded from real git (rule R2.5).

#![forbid(unsafe_code)]

mod error;
mod exec;
pub mod read;

pub use error::GitError;
pub use exec::{CancelToken, Git, Output, command_line};

//! Windows: the credential store, asked of `advapi32.dll`, and the junctions made for links to folders and the job
//! objects that hold the processes of a task, asked of `kernel32.dll`, from Windows 10 version 1809.
//!
//! `kernel32.rs` and `advapi32.rs` declare what is used of each library, with the documentation's names;
//! `credentials.rs`, `junction.rs` and `job.rs` wrap those calls in safe functions at once. `platform` uses `kernel32` too, for
//! the folders it watches and the local time.

mod advapi32;
mod credentials;
mod job;
mod junction;
#[allow(
    missing_docs,
    reason = "the bindings are for platform, with the names of the Windows documentation, which says what each is"
)]
pub mod kernel32;

pub(crate) use credentials::{delete_secret, read_secret, write_secret};
pub(crate) use job::Job;
pub(crate) use junction::make_junction;

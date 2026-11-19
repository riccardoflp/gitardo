//! The pull requests of the hosting services other than GitHub: loading the merge requests of a GitLab for the first
//! time, with the token that reads them, and checking out a pull request in a local branch
//! (`docs/features/15-github.md`).

mod checkout_pull_request;
mod load_merge_requests;

pub use checkout_pull_request::{LIST, NAME, NUMBER, PICK, TYPE_NUMBER, checkout_pull_request};
pub use load_merge_requests::{FOUND, REMOTES, TOKEN, load_merge_requests};

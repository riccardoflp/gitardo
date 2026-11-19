//! The pull requests of the hosting services other than GitHub: loading the merge requests of a GitLab for the first
//! time, with the token that reads them, and checking out a pull request in a local branch
//! (`docs/features/15-github.md`).

mod load_merge_requests;

pub use load_merge_requests::{FOUND, REMOTES, TOKEN, load_merge_requests};

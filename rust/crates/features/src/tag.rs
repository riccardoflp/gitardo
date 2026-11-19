//! Tags: creating, deleting here and on a remote, checking out and pushing (`docs/features/06-branches-tags-remotes.md`).

mod checkout;
mod create;
mod delete;
mod delete_remote;
mod push;

pub use checkout::checkout;
pub use create::create;
pub use delete::delete;
pub use delete_remote::{HERE, REMOTE, delete_remote};
pub use push::push;

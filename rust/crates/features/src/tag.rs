//! Tags: creating, deleting, checking out and pushing (`docs/features/06-branches-tags-remotes.md`).

mod checkout;
mod create;
mod delete;
mod push;

pub use checkout::checkout;
pub use create::create;
pub use delete::delete;
pub use push::push;

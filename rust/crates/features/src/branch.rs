//! Branches: creating, checking out, renaming, deleting, fast-forwarding and cleaning up
//! (`docs/features/06-branches-tags-remotes.md`).

pub(crate) mod checkout;
mod checkout_detached;
mod checkout_remote;
mod checkout_remote_and_rebase;
mod clean_up;
mod create;
mod delete;
mod fast_forward;
mod rename;
pub(crate) mod submodules;
pub(crate) mod target;

pub use checkout::checkout;
pub(crate) use checkout::checkout_plan;
pub use checkout_detached::checkout_detached;
pub use checkout_remote::checkout_remote;
pub use checkout_remote_and_rebase::checkout_remote_and_rebase;
pub use clean_up::clean_up;
pub use create::create;
pub use delete::delete;
pub use fast_forward::fast_forward;
pub use rename::rename;

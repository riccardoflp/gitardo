//! Fetching, pulling and pushing (`docs/features/06-branches-tags-remotes.md`).

mod deepen;
mod fetch;
mod forced;
mod pull;
mod pull_with;
mod push;
mod push_force;
mod push_force_with_lease;
#[allow(
    clippy::module_inception,
    reason = "`sync.sync` is the action of its card, named as rule R2.1 says"
)]
mod sync;
mod target;

pub use deepen::{COUNT, deepen, unshallow};
pub use fetch::fetch;
pub use forced::OVERWRITTEN;
pub use pull::pull;
pub use pull_with::{WAY, WAYS, default_way, pull_with};
pub use push::push;
pub use push_force::push_force;
pub use push_force_with_lease::push_force_with_lease;
pub use sync::{PUBLISH, sync};
pub(crate) use target::PUSH_UNDO;

//! The changes not committed: staging and unstaging, one file or all of them, and discarding
//! (`docs/features/05-changes-and-commit.md`).

mod discard;
mod discard_all;
mod files;
pub mod ignore;
mod save;
pub mod skip;
mod stage;
mod stage_all;
mod stop_tracking;
mod unstage;
mod unstage_all;

pub use discard::discard;
pub use discard_all::discard_all;
pub use ignore::ignore;
pub use save::save;
pub use skip::{skip, unskip};
pub use stage::stage;
pub use stage_all::stage_all;
pub use stop_tracking::stop_tracking;
pub use unstage::unstage;
pub use unstage_all::unstage_all;

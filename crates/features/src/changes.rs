//! The changes not committed: staging and unstaging, one file or all of them, and discarding
//! (`docs/features/05-changes-and-commit.md`).

mod discard;
mod discard_all;
mod files;
mod stage;
mod stage_all;
mod unstage;
mod unstage_all;

pub use discard::discard;
pub use discard_all::discard_all;
pub use stage::stage;
pub use stage_all::stage_all;
pub use unstage::unstage;
pub use unstage_all::unstage_all;

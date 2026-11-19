//! Repositories made from the window: cloned and created (`docs/features/01-repositories.md`).

mod clone;
mod init;

pub use clone::{
    BRANCH as CLONE_BRANCH, DEPTH, NAME, OPTIONS, OTHER_BRANCH, PARENT, SHALLOW, START_FOLDER, SUBMODULES, URL, clone,
    is_on_disk,
};
pub use init::{BRANCH as INIT_BRANCH, FOLDER, INSIDE, init};

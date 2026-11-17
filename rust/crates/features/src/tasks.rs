//! The tasks of a repository (`docs/features/19-tasks.md`): those the worktree shown defines, read from its
//! `.vscode/tasks.json` and its `package.json` files, and the groups the user makes of them, kept in a file of the
//! common git folder.

pub mod edit_groups;
pub mod groups_file;
mod packages;
mod read;
#[cfg(test)]
mod testing;
mod variables;
mod workspace;

pub use edit_groups::edit_groups;
pub use read::read_tasks;

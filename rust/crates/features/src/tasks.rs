//! The tasks of a repository (`docs/features/19-tasks.md`): those the worktree shown defines, read from its
//! `.vscode/tasks.json` and its `package.json` files.

mod packages;
mod read;
#[cfg(test)]
mod testing;
mod variables;
mod workspace;

pub use read::read_tasks;

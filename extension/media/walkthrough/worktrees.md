# Worktrees

A worktree is another folder checked out from the same repository, on a branch of its own: a place to work on something else, or to let an agent work, without touching the folder you are in.

- **Create worktree…** on a branch puts that branch in a new worktree; on a commit it makes a new branch there.
- New worktrees go next to the repository, in `<repository>.worktrees`, unless `gitardo.worktrees.folder` says otherwise.
- **Choose What New Worktrees Get…** links or copies what git ignores, such as `node_modules` or `.env`, into each new worktree.
- The Worktrees section beside the graph and in the Branches view says how each one stands; a double click opens it in a new window.

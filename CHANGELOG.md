# Changelog

## Unreleased

- The documents to build Gitardo Desktop from: the clean room, the product, the architecture, the code rules, the
  contract of a feature, the platforms, testing, the roadmap, the decisions, and the catalog of every feature.
- The workspace of crates, with the lints of the code rules, and `cargo xtask check`, which refuses a package from
  outside the workspace, an edge between crates the architecture does not draw, a card and its code that do not name
  each other, I/O in a pure crate, `unsafe` outside `platform`, a file over 500 lines and CR line endings, then runs
  rustfmt, clippy and the tests.
- `testkit`, for the tests: throwaway repositories, golden files, a seeded random source, and PNG images.
- `json`: JSON and JSON with comments, parsed, written, and edited in place keeping the comments and the order.
- `gitardo-cli`: the state of a repository, and the first features run from a terminal through the engine: create a
  branch, commit, fetch, and undo the last commit, checkout, rebase or reset. A feature asks for what it needs with
  `<key>=<answer>`, and a plan that is shown first runs with `--yes`.
- Every read of milestone M1, each with golden tests on awkward repositories: a page of the graph and where a commit is
  in it, the commits of what is checked out and which are not pushed, the details and the lines of a commit, the two
  sides, files and lines of a comparison, a file at a revision, the working tree, the identity, the remotes, what a
  force push would overwrite, the stashes and what one changes, the worktrees and their links to folders, what git
  ignores, the submodules, the files in conflict and their sides, the conflicts a merge would have, a search of the
  history, blame, the names a file had, the reflog, a bisect, and where a rebase stopped.
- What is derived from them in `model`: the lanes of the graph, its labels, folds, dates and traced history; the tree
  of the files of a commit and the lines of its changes; the order of references; where a range of lines was.
- Undo keeps a journal of what the reflog does not see: a branch deleted, a stash dropped, conflicts resolved, a squash
  merge, a series of commits made at once.
- A plan can be cancelled while it runs: the git process or the command of the step running is killed, and no step
  after it starts.
- A window on Windows, drawn by the program: per-monitor scale, keys, the input method, the pointer, the clipboard,
  timers, a dark or light title bar, and the theme and fonts of the system.
- Drawing in software: shapes with anti-aliasing, TrueType fonts read and rasterized by the program with kerning and
  fallback fonts, icons of its own, and five themes (the system's, dark, light, high contrast dark and light).
- The toolkit: labels, buttons, check boxes, radio buttons, toggles, badges, progress, input boxes, scroll areas,
  lists that draw only the rows in sight, splitters, tooltips, menus with submenus, dialogs, confirmations, a quick
  pick with fuzzy filtering, find in page and drag and drop; only what changed is drawn again.
- `gitardo-demo`: every widget in a window, with the themes to choose from.

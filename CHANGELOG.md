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
- Remotes, sync and stashes from `gitardo-cli`: pull as the settings `pull.rebase` and `pull.autostash` say or in any
  of its four ways, push, force push with a lease or without, listing the commits it overwrites, sync, which fetches,
  pulls and pushes; add, remove, rename, repoint and fetch a remote, delete a branch from its remote; stash all changes,
  the staged ones or some files, apply, pop, drop or make a branch of a stash, each apply and pop saying first what it
  would stop on, and a stash dropped is brought back by Undo. A pull that leaves its autostash says so.
- **Copy hash** and **Copy message** in the menu of a commit in the timeline, and `commits.copy-hash` and
  `commits.copy-message` from `gitardo-cli`: the full hash, or the whole message as it was written.
- In the window: **Sync** in the header with the commits to pull and to push, and its menu with **Fetch**, **Pull**
  and its four ways, the default one marked, **Push** and the two forced pushes, none of which can be clicked while an
  operation runs; **Manage remotes…** in **…** and in Quick search; **Apply**, **Pop**, **Create branch from
  stash…** and **Drop** on a stash in the branch rail and in the timeline; **Delete from remote** on a remote branch;
  **Copy URL** puts the URL of a remote on the clipboard.
- Every read of milestone M1, each with golden tests on awkward repositories: a page of the graph and where a commit is
  in it, the details and the lines of a commit, the two sides, files and lines of a comparison, a file at a revision,
  the working tree, the identity, the remotes, what a force push would overwrite, the stashes and what one changes, the
  worktrees and their links to folders, what git ignores, the submodules, the files in conflict and their sides, the
  conflicts a merge would have, a search of the history, blame, the names a file had, the reflog, a bisect, and where a
  rebase stopped.
- What is derived from them in `model`: the lanes of the graph, its labels, folds, dates and traced history; the tree
  of the files of a commit and the lines of its changes; the order of references; where a range of lines was.
- Undo keeps a journal of what the reflog does not see: a branch deleted, a stash dropped, conflicts resolved, a squash
  merge, a series of commits made at once.
- A plan can be cancelled while it runs: the git process or the command of the step running is killed, and no step
  after it starts.
- A window on Windows, Linux and macOS through `winit`: per-monitor scale, keys, the input method, the pointer, the
  clipboard, the system's dialogs to choose a folder or a file, timers, a dark or light title bar, and the theme and
  high contrast of the system. The frame drawn in memory is shown with `softbuffer`, and what is on screen is told to
  screen readers through AccessKit.
- Drawing in software: shapes with anti-aliasing through `tiny-skia`, text through `cosmic-text` (shaping,
  right-to-left, fallback to the fonts of the system, emoji), Inter and JetBrains Mono that come with the program, icons
  of its own, and five themes (the system's, dark, light, high contrast dark and light) in the warm colors of the
  Timeline.
- The toolkit: labels, buttons and pills, check boxes, radio buttons, toggles, badges, progress, input boxes, scroll
  areas, lists that draw only the rows in sight, cards, splitters, tooltips, menus with submenus, dialogs,
  confirmations, a quick pick with fuzzy filtering, find in page and drag and drop; only what changed is drawn again,
  and the tree of what is on screen is built for the screen reader.
- `gitardo-demo`: the window of the Timeline with made-up data: the header with Quick search and Sync, the branch rail,
  the timeline with its draft and a million commits, the inspector and the Activity drawer, with the themes to choose
  from.
- The branch rail: favorites, the branches checked out last, the local branches with how far each is from its
  upstream, one list for each remote, and the stashes, worktrees and tags kept aside; a filter box from 15 names on,
  the order by name or by date, the menus of the references, and a branch's history brought out under the pointer.
- The branch filter: the button before Quick search and its list with the presets, the box and a tick on every
  reference, the kinds of references in **…**, **Show only this history**, **Show this history too** and **Hide this
  history** on the references of the rail, the bar over the timeline narrowed, and the line at the top of the rail.
  Narrowing reads only the first page of the history shown, and the choice is kept for each repository.
- The search box at the top of the timeline: the commits loaded found as you type, by message, author or the start
  of a hash; the whole history searched by message, author or change on `Enter`, the commits found kept in color and
  marked beside the scroll bar, the others dimmed, with their count; `Enter`, `F3` and the arrows step through them,
  loading the history as far back as they are; `Ctrl+F` puts the cursor in the box. The funnel shows only the commits
  found, as a list, until **Show all commits**, and stays pressed between runs. A hash that is not loaded is gone to
  with `Enter`, wherever it is: the history is loaded up to it, what narrows the timeline is taken away, and a commit
  on no branch or tag is shown with its history alone.
- **Go to commit…** in the **…** menu of the header: any name git takes for a commit, checked as you type.
- Quick search in the header (`Ctrl+K`): the commands, the branches, the remote branches, the tags and the commits
  loaded, filtered as you type; a command that is not built yet says so and does nothing.
- The inspector shows the row selected in the timeline: the title of its message and the rest unwrapped, cut at four
  lines with **Show more**, who made it and when, its hash, which a click copies, its parents, which a click selects,
  and its files as a list or as a tree whose directories fold, the first 300 with **Show all**; a stash shows the
  untracked files it took apart from the others. The layout chosen is kept between runs.
- The page of a commit, opened by a double click or `Enter` on a row of the timeline, by the button of the inspector or
  by **Open Commit as a Page…** in Quick search, which takes any name git turns into a commit, and never twice for the
  same commit: its whole message with its labels, who made it, its hash and parents, its files
  with the lines each gained and lost, and with **Show the changes** the diff of each file, the long ones folded at
  first, under headers that stay at the top, with find in page over the lines unfolded and **Show in the timeline**, which
  shows all branches again when the branches shown leave the commit out.
- `gitardo` watches the repository shown on Windows and reads it again on its own: once after a burst of changes,
  never two reads at once, never for the files git and its `core.fsmonitor` daemon write during a read, and not while
  the window is minimized. **Refresh** during a read is followed by one more read. With `readOnStartup` off, the
  window opens with the repository not read until **Read the repository** is clicked.
- The Activity drawer lists what runs, with why each read runs and how many reads in a row it is, and what ran a
  moment ago. **Show the commands Gitardo runs** shows the Command log beside it, each line with its time; the lines
  selected are copied with `Ctrl+C`. At `log.level` `debug` the log has each read with how long it took.
- The Diff page: two versions of a file side by side, kept level with blank rows, each change joined across the gutter
  and the words that changed marked, with a mark for each change beside the scroll bar. A click on a file in the
  inspector or the page of a commit, **Open all changes in one Diff page**, **Open in the Diff page**, an untracked
  file of a stash, and **Open changes** on a file of the Changes page open it: the index against the file on disk
  while some of the file is not staged, the last commit against the index once all of it is. With
  `changes.openOnClick` set to `diff`, a click on a file of the Changes page does the same. They open it never twice
  for the same diff. `F7` and `Shift+F7` go from change to change and, between `HEAD`, the index and the working tree,
  on to the next changed file. Long runs of unchanged lines fold
  (`diff.collapseUnchanged`), the line between the two sides is dragged, and changes already staged are dimmed. A binary
  file or a diff of more than 5 seconds is refused, and a file of more than 100,000 lines asks first.
- The settings page (**Settings** in **…**, or `Ctrl+,`) lists every setting with its description, its value, where
  the value comes from and why a value was not taken. A switch or a choice changes it in the user's file or the
  repository's, keeping the rest of the file. **Theme…** in **…** picks the theme. **Show Command Log** and **Open
  Settings** in Quick search open the log and the page.
- The Welcome page ticks **Open a commit, compare two branches** once the page of a commit is opened, and **Welcome**
  in Quick search shows it again with the steps done ticked.
- A row of the timeline keeps its message whole when it can: the labels take the room the message leaves, and those
  that do not fit are counted, `+2`, with their names in the tooltip of the count.
- The menu of a label a branch shares with its upstream ends with **Copy origin/feature**, which copies the name of the
  upstream.
- Quick search keeps its room in the header while something runs: the activity indicator takes only what is left,
  and shows its icon alone where its words would be cut short.
- Operations run from the window: **Branch** and **Undo** in **…**, **Fetch** in the menu of **Sync**, and **Create
  branch…**, **Fetch** and **Undo Last Operation** in Quick search. Their questions are dialogs of the window; a plan
  that can lose work, or every plan with `commandPreview` `all`, shows its commands first, and **Run** of one that
  cannot be undone is not pressed by `Enter`. What ran is in the Command log, and what it did is said over the
  timeline with **Undo**.
- Branches and tags: check out a branch, a remote branch (tracked, fast-forwarded, or rebased onto it when each has
  commits of its own), a commit or a tag on a detached `HEAD`; rename and delete a branch, fast-forward one that is not
  checked out, and clean up the branches merged or whose upstream is gone with their worktrees; create, delete, check
  out and push a tag. Each is in the menu of its branch, remote branch or tag in the branch rail and on its label in
  the timeline, and a commit's menu has **Create branch here…**, **Create tag here…** and **Check out commit
  (detached)**. A double click on a branch checks it out, or does what `branches.openIn` says; **Clean up branches…**
  is in **…** and in Quick search; Undo brings back a branch deleted at its commit.
- `gitardo-cli` stages and unstages a file or all changes, discards the changes of a file or all of them after a warning
  that says what is lost, amends the last commit, and switches the identity of a repository or goes back to the global
  one. A file whose name is also a pattern, such as `pages/[id].tsx`, is named to git as itself.
- The Changes page, from **Review** in the draft or **Show changes** in Quick search: the commit box with its message,
  kept for each repository between runs, **Commit** or **Amend** with **Amend last commit**, and `Ctrl+Enter`; the
  identity commits are made with, which a click switches; the files in conflict apart; and one list of the changed
  files, as a list or a tree, each with a box ticked, half ticked or empty as much of it is staged, the box on the
  header for every file, **Discard changes** and **Discard all changes**, and **Stash this file…**, **Stash all
  changes…** and **Stash only the staged changes…**. The draft at the top of the timeline
  shows the message and commits it, and its menu stages, unstages and discards all changes. **Switch identity…** is
  in **…** and in Quick search.

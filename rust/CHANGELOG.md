# Changelog

## Unreleased

- The stashes are read without an error when the last one goes while git reads them, as when it is dropped or a
  pull takes back the changes it set aside: there are none.
- **Skip** of a step of a rebase, a cherry-pick, a revert or a patch asks first while files are staged, changed or in
  conflict: *What was resolved in this step is thrown away.*
- **Take the current side** and **Take the incoming side** warn, rather than ask, when the file has lines neither side
  nor the base has, typed in it since git stopped: they are lost too.
- **Resolve automatically** asks one question, also with `commandPreview` set to `all`: **Resolve** and **Review** are
  not confirmed a second time.
- With `commandPreview` set to `all`, writing the resolutions accepted on the review asks first and reads the files
  again once it is answered, so that a file saved while the question was open is not written over.
- **Go to commit** and the other places a name of a commit is typed find nothing for a place past the end of a
  reflog, such as `stash@{3}` with two stashes, instead of failing.
- **Undo** keeps up to 50 of Gitardo's operations, newest last, for as long as nothing moves `HEAD` from outside
  Gitardo, and undoing again takes back the one before. Right after an operation it cannot take back, a push, a fetch,
  a tag, a stash or a worktree, it says so instead of taking back the one before it; staging, saving or discarding
  changes leaves nothing to take back, and Undo goes past them.
- **Undo** refuses while a merge, a rebase, a cherry-pick, a revert or a patch is under way, *Finish or abort the merge
  first.*, but for a squash merge or conflicts resolved, which it takes back from where they stopped.
- **Undo** deletes a branch created without checking it out.
- **Unstage all changes** no longer ends a merge, a rebase, a cherry-pick, a revert or a patch that waits to be
  committed with no conflict left: it unstages the staged files by name, as it does while files are in conflict.
- The window is made of islands: the header, the branch rail, the timeline, the inspector and the page shown are each
  a rounded card on a darker ground, with a gap between them that is dragged to resize them.
- A bar of views on the left edge of the window goes to **Timeline**, **Changes**, with the number of changed files,
  **Files** and **Compare**, and at its foot opens **Activity** and **Settings**; the place shown is marked.
- A status bar at the foot of the window says what is running, which opens the Activity drawer, and the identity
  commits are made with, which a click switches.
- **Search** in the header opens Quick search, as `Ctrl+K` does, in place of the box of text; the branch filter and the
  search box are at the top of the timeline.
- **Clean up branches** removes a worktree as **Remove worktree** does: on Windows its links to folders outside it are
  unlinked first, so that `git worktree remove` no longer empties the folders they lead to, and the folders it leaves
  empty are deleted.
- While a bisect is under way, squash, split, fixup, the interactive rebase and cherry-picking several commits are not
  offered, and squash, split and fixup are refused with **End the bisect first.**
- When `git branch -d` keeps a branch because it is not fully merged, **Delete anyway** offers `git branch -D`.
- Creating a branch and checking it out, and creating a branch from a stash, update the submodules afterwards when
  `submodules.updateOnCheckout` asks for it, as a checkout does; the new branch checked out is selected in the
  timeline.
- Undo of a checkout updates the submodules afterwards when `submodules.updateOnCheckout` asks for it, as the checkout
  does.
- Before the commands of `worktrees.setupCommands` run in a folder that is not trusted, Gitardo asks **Run the
  commands for new worktrees?**, naming the folder, where the commands come from and each of them. **Trust this folder
  and run them** runs them and trusts the folder, and every folder in it, from then on; **Not now** creates the
  worktree without running them, and asks again the next time. The folders trusted are kept in `state.json`, and
  **Trusted folders…** in Quick search lists them and stops trusting those unticked.
- The commands of `worktrees.setupCommands` run in **Tasks**, in the Activity drawer, which opens on them: one after
  the other, each named after its worktree, with what they print shown as it comes, a box that answers what the one
  running asks, and **Stop**. Other operations do not wait for them; opening the new worktree, or the offer to open it,
  waits for the last of them, and when one fails Gitardo says which, and how many did not run after it, and offers to
  open the worktree all the same.
- **Manage remotes…** finds a remote by its name or its URL as they are typed.
- The top of the branch rail names the branch checked out, with **Create branch**, at `HEAD`, and **Compare**.
- **Create branch** with `branches.format` set asks first whether to use the format; with it, the inputs of
  `branches.inputs` are asked in order, each with the value given last in this workspace, a `pattern` is checked while
  typing, `${base}` at a commit is picked among the branches that have it, `${date}` is today in local time, and the
  name made is shown to change or keep.
- Typing in the filter box of the branch rail, or in the box of the branch filter, lists the matches of every list
  together, the best first: `main`, then `origin/main`, then `maintenance`.
- With `branches.sortOrder` not set, the branches, the remote branches and the tags are listed by date when Git's own
  `branch.sort` is `-committerdate` or `committerdate`, and by name otherwise.
- **Edit** on a card of the review of the conflicts opens a box with the lines proposed, or the two sides one after the
  other when nothing is; **Use these lines** or `Ctrl+Enter` makes them the choice of the conflict, **Cancel** or
  `Escape` closes it.
- **Open all changes in one Diff page** on the Changes page opens one Diff page titled **Changes** with every changed
  file, each between the last commit and the file on disk, the changes already staged drawn dimmed.
- The Changes page leaves out the files with edits not saved that git ignores, that are in a submodule or in another
  worktree inside the repository, and those outside it; git is asked with `git check-ignore` when a file becomes or
  stops being unsaved, and when the repository is read again.
- The working tree side of a Diff page is edited where it is shown: a click places the caret, a drag, a double click
  and a triple click select, the keys of the editor move and type, `Ctrl+C`, `Ctrl+X` and `Ctrl+V` use the clipboard,
  and `Ctrl+Z` takes back what was typed. The lines typed show at once, the diff is computed again a moment after
  typing stops, and `Ctrl+S` saves the file. The text is the one the File page of the file shows, so an edit in one
  shows in the other. **Revert this change** is such an edit, saved at once, and `Ctrl+Z` brings the change back.
- `F4`, or **Jump to source** in the menu of a side, opens the File page of the file at the line of the caret; on a
  removed line of the left side, at the line above it.
- Linux, on X11 and Wayland: the window, high contrast and the time of a double click read from the desktop's
  settings, folders watched with inotify, addresses opened with `xdg-open`, and the GitHub token and the key of the AI
  provider kept in a file of the configuration folder that only the user may read, until Gitardo speaks to the Secret
  Service. `${date}` in the names of new branches is in the local time there too.
- macOS, written but not yet run on a Mac: high contrast and the time of a double click read from `AppKit`, folders
  watched with `FSEvents`, addresses opened with `open`, the GitHub token and the key of the AI provider kept in the
  keychain, and `${date}` in the names of new branches in the local time.
- The **…** menu of the header is in the groups of its card, with **Compare branches, tags or commits…** and
  **Switch identity…** in a group of its own.
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
- The Stashes list in **Kept aside** of the branch rail: its title counts the stashes and has a button that stashes all
  changes; each stash says how long ago it was made, has **Apply**, **Pop** and **Drop…** under the pointer when no
  operation runs, and unfolds to the files it changed, then the untracked files it took, a click opening one in the
  Diff page; with no stash, it says how to make one. **Show Stashes** in Quick search opens it, and so does the button
  beside the warning of a pull that left its autostash in the stashes.
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
- The gutter of the Diff page has buttons on each change: **Stage this change** and **Unstage this change** give the
  patch of that change alone to `git apply --cached`, and **Revert this change** writes back what the left side has
  there; a change of a file edited since the page was drawn is refused. The bar of a page of one file has **Stage
  file**, **Unstage file** and **Discard changes**, which asks first; a page whose untracked file is discarded closes.
  The page shows what the repository then has without being opened again.
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
- `gitardo-cli` merges a commit, a branch or a remote branch into the branch checked out, as git's configuration says
  or in one of four ways (no fast-forward, fast-forward only, squash), and takes a squash merge back; cherry-picks one
  commit, or several selected ones oldest first as one operation that Undo takes back whole, and gets past a commit
  that changes nothing with a warning in the log; reverts a commit; resets the branch to a commit, soft, mixed or hard;
  and edits the message of the last commit, or of an older one with an interactive rebase. A merge or a cherry-pick
  predicted to conflict says which files first; `cherryPick.recordOrigin` adds `-x`.
- In the window, the menu of a commit has **Merge into `<branch>`**, **Merge into `<branch>`…** to choose how,
  **Cherry-pick `<hash>`**, **Revert `<hash>`**, **Reset `<branch>` to here…** and **Edit message…**; the menus of a
  branch and of a remote branch have the two merges. `Ctrl`+click (`Cmd`+click on macOS) and `Shift`+click select
  several commits of the timeline; a bar over the commits counts them, with **Cherry-pick onto `<branch>`** and **Clear
  selection**, and their menu has **Cherry-pick N commits onto `<branch>`** and **Copy hashes**. After a squash merge,
  an empty message of the draft starts from the one git prepared, and Undo takes the squash merge back.
- Rebase and rewriting from `gitardo-cli`: rebase the current branch onto a commit or a branch, or another branch onto
  a commit; an interactive rebase of a todo list that picks, rewords, edits, squashes, fixes up, drops and reorders the
  commits up to `HEAD`; squash several commits into one, or a commit into the one before it, under a subject asked for;
  split a commit into its changes, unstaged, and continue the split; add the changes to an earlier commit as a fixup;
  and continue a rebase stopped to edit a commit. Each asks first, saying when a commit is pushed already, sets aside
  what is not committed where it can, and is taken back by Undo. `gitardo-cli show` says where a rebase stopped.
- Rebase and rewriting in the window: **Rebase `<branch>` onto this** in the menu of a commit, a branch and a remote
  branch; **Squash into the commit before it…**, **Split this commit…**, **Add staged changes to this commit (fixup)**
  or **Add all changes to this commit (fixup)**, and **Interactive rebase of N commits…** or **Edit this commit
  (interactive rebase)…** in the menu of a commit; **Squash into one** in the bar of a selection of commits that
  follow one another up to `HEAD`, and **Squash N commits into one…** in its menu. The bar of a selection also has
  **Copy as patch**, and its menu **Compare these two commits**, **Copy as patch** and **Save as patch…**. The interactive rebase page lists the commits up to `HEAD`, newest
  first, to reorder with **Move up**, **Move down** or by dragging, and to pick, reword, edit, squash, fix up or drop,
  says what keeps it from starting, and starts the rebase. While a rebase is stopped to edit or split a commit, a banner
  above the timeline says so, with **Continue**. After a squash the commit it made is selected, and after a split the
  commit box holds the message of the commit split.
- A bisect from the window and from `gitardo-cli`: **Start bisect: this commit is bad** or **…good** and, while one is
  under way, **Bisect: mark as good** or **…bad** in the menu of a commit; a banner above the commits that says what the
  bisect needs next, with **Good**, **Bad** and **Skip** for the commit checked out and **End**; the labels `good`,
  `bad` and `skipped` on the commits marked; and a message that names the first bad commit once it is found, with
  **Show in the timeline** and **End bisect**. Undo refuses while a bisect is under way.
- Patches, from the window and from `gitardo-cli`: **Copy as patch** and **Save as patch…** in the menu of a commit
  that is not a merge, one patch of `git format-patch` for one commit or several, oldest first; **Apply patch…** in
  **…** and in Quick search, from a file or from the clipboard, as commits with `git am --3way` or to the working tree
  only with `git apply --3way`, checked first: a patch that does not apply says why and changes nothing, one that will
  stop on conflicts names the files and asks first, and Undo takes back all the commits of a patch at once.
- Comparisons and the reflog from `gitardo-cli`: choose two branches, tags, commits or the working tree to compare,
  or a comparison opened before; compare a commit, branch or tag with another or with what is checked out, and two
  commits with each other. **Compare with…** and **Compare with `<current branch>`** are in the menus of commits,
  branches, remote branches and tags, and **Compare with `<short hash>`, selected before** in the menu of a commit
  while another is selected. **Reflog** in **…** and **Browse reflog** in Quick search list where `HEAD` has been, to
  show an entry in the timeline, make a branch there or check it out.
- The comparison page: the two sides, each a button that puts another in its place and its short hash that opens its
  commit, the arrows that swap them, how far apart they are and what merging would do, **Since they diverged** or
  **Everything that differs**, a box that narrows it to a file or a folder, the commits only in each side, those
  cherry-picked marked **also in**, and the files that differ, each opening in the Diff page, with an arrow that takes
  the version of the second side; **Open in the Diff page** and **Copy as patch**. A comparison with the working tree
  is read again with the repository, and one with a branch when the branch moves. **Compare with `<current branch>`**
  and **Compare with `<short hash>`, selected before** show the files that differ between the two commits in the
  inspector, with **Swap** and **Open as a page**.
- An operation stopped on conflicts: a banner over the timeline and the Changes page says which operation is in
  progress and how many files have conflicts, with **Continue** (once no file is left in conflict), **Skip** (not for a
  merge) and **Abort**, which asks first; it is also the banner of a rebase stopped to edit or split a commit, whose
  **Abort** now works. A merge, a cherry-pick or a stash pop that stops on conflicts says so and what to do next, with
  **Show log**. Each file in conflict on the Changes page has **Mark as resolved** and the two buttons that take the
  current or the incoming side for the whole file, after asking, Undo marking its conflicts again; a click on its name
  opens it in the Diff page, the base beside the file when one side deleted it. With the experimental setting
  `conflicts.autoResolve` on, **Resolve … automatically** writes the conflicts that need no choice in the files after
  saying what they are, stages the files left with none, offers **Continue** when none is left, and Undo marks the
  conflicts again. `gitardo-cli` continues, skips, aborts and marks a file resolved.
- The Merge page: a file in conflict in three columns, the current side, the result and the incoming side, labelled
  with what each side is, kept level and scrolled together; each change joined to the result by a band in its color,
  green for the current side, blue for the incoming side, grey for both, red for a conflict, with the words that
  changed marked, git's markers marked, and a conflict resolved by hand drawn faintly. The buttons of a band put a
  side, or both, in place of its region; the button with two ticks accepts every change that needs no choice; with
  `conflicts.autoResolve` on, **✦** and the sparkle accept what the rules propose; each is one edit that `Ctrl+Z` takes
  back and `Ctrl+Y` puts again. The arrows, `F7` and `Shift+F7` go from region to region and round, beside how many
  conflicts are left; the clock shows the base in a fourth column, and the next page opens the same way. **Mark as
  resolved** writes the result and stages it, asking first while git's markers are left, and closes the page.
- The review of the resolutions of the conflicts, with `conflicts.autoResolve` switched on: **Review the resolutions
  of the conflicts** in Quick search shows every conflict as a card, with what the rules propose, where each of its
  lines comes from, what looks wrong in it and what each side changed, or in three columns joined by bands; the files
  and the filters beside them; a side, or both, chosen instead; the conflicts accepted one by one, by file or all at
  once; the result shown in the Diff page; and **Write N resolutions**, which writes them as one operation, stages
  the files left with no conflict and says what it did.
- The File page: a file of the working tree, shown and edited in Gitardo's own editor, or a file at a revision, read
  only and titled with its short hash. It opens from a click on a file in Changes when `changes.openOnClick` is `file`,
  from **Open file** on a file in Changes, and from **Open the file at this revision** on a file of a commit; opening a
  file again shows its page at the line asked. Typing, a selection, `Ctrl+Z` and `Ctrl+Y` by words, **Cut**, **Copy**
  and `Ctrl+V`, with the line endings of the file kept; a dot in the trail while there are edits not saved, which the
  Diff page of the file shows too; a page with edits not saved asks before closing, with **Save**, **Don't save** and
  **Cancel**; a file changed in another program is read again when the page has no edits.
- **Save** with `Ctrl+S` on the File page, and **Save** on a file in Changes, write the text as it is shown, asking
  first when the file changed on disk since it was read.
- **Find** with `Ctrl+F` on the File page: a text or a regular expression, matching case and whole words or not, every
  match marked, `Enter`, `F3` and `Shift+F3` going from one to the next, and where the current one is among them; **Go
  to line** with `Ctrl+G`.
- **Open in external editor** on the File page, with the command of the setting `editor.external`, where `${file}` and
  `${line}` are the file and the line of the cursor.
- The code on the File and Diff pages in the colors of its language, the whole text colored at once, so that a line in
  a comment of several lines is a comment; changing the theme colors it again.
- The Submodules list in **Kept aside** of the branch rail, while the repository has submodules: each indented under the
  one that has it, with where it stands against its recorded commit, the branch it has checked out and a pencil with
  its files not committed, a tooltip that says all of it, **+** to add one, a double click that opens it in a new
  window, and a menu to open it in a terminal or the file manager, initialize it, update it to the recorded commit or
  to the latest of its branch, stage its commit as the recorded one, synchronize the URLs, copy its path or URL,
  deinitialize it and remove it; the list follows commits, checkouts and edits made in a submodule elsewhere.
- **Add submodule…** in **…** while the repository has none, and in Quick search; adding one clones it with its own
  submodules. Cloning a submodule from a folder on disk, which git refuses unless allowed, is offered again allowed for
  those commands only, and **Deinitialize** of a submodule with changes offers `--force`.
- A checkout updates the submodules to the commits it records, as `submodules.updateOnCheckout` says, asking first when
  a submodule has commits on a detached HEAD that no branch would keep.
- The submodule operations from `gitardo-cli`, with `--target submodule:<folder>`.
- The review of the conflicts also opens from **Resolve … automatically** in the banner of an operation that stopped,
  from the wand of a file in conflict on the Changes page, for that file alone, and from **Review** in the question of
  **Resolve conflicts automatically**.
- From the review of the conflicts: **Conflict at line N** opens the file at that line, **Resolve by hand** opens the
  Merge page at that conflict, a file under **Also in conflict** opens where it can be resolved, **Keep it, with these
  changes** and **Delete it** take a side of a file deleted on one side, and **Undo** and **Continue the merge** in the
  report put the conflicts back and go on with the merge. A file both sides added opens in the File page, with git's
  markers in it.
- On the Merge page a click on a line puts the cursor on it, and `F4` or **Jump to source** in its context menu opens
  the file in the File page at the line of the result it stands for. Closing a Merge page with choices not saved
  shows them in the File page, not saved, so that they are not lost.
- **Save all** on the Changes page writes every file with edits not saved in a File page.
- What new worktrees get of what git ignores, from `gitardo-cli`: link a folder into new worktrees, copy a file or a
  folder into them, and stop either, saved in the settings where the list is set, comments kept, with what became of
  each path and why nothing changed when nothing did; save or remove the settings of a place; bring ignored files into
  a worktree, or into all the other worktrees, without touching what they have, one warning at the end saying what
  failed. Links to folders are junctions on Windows.
- The Files page, from **Show files** in Quick search: the tree of the files of the working tree, folders folded until
  clicked, a click on a file opening it in the File page, and in the context menu of a file **Open file**, **Show file
  history** and **Compare with Branch, Tag or Commit…**, of a folder **Show folder history** and **Compare with…**.
- The history of a file under every name it had, of the lines selected in the File page (`git log -L`) and of a
  folder, each narrowing the timeline with a bar that says whose history it is, and **Show all commits** to take the
  narrowing away. They run from the Files page, from the context menu of the File page, from Quick search, and the
  history of a file from the clock beside a file of the inspector.
- **Compare with Branch, Tag or Commit…** on a file or a folder of the Files page opens the comparison of that side
  with the **Working tree**, narrowed to the file or the folder.
- The blame in the File page: who last changed the line under the cursor, how long ago and the subject of the commit,
  dimmed at its end, with a hover that shows the commit in the timeline; **Show file blame** in the title puts the date
  and author of each line in a column before the text, the bar of each line in the color of its age, the lines of the
  commit of the cursor brought out with a mark on the scroll bar for each run of them, the lines typed said *Not
  committed yet* at once, and a hover with **Show in the timeline**, **Open it as a page** and **Blame the file as it
  was before**. The funnel ignores whitespace and moved lines, the pin keeps the blame in every file, both written in
  the user's settings; `Alt+B` opens the commit of the line, `Shift+Alt+B` shows it in the timeline, and each has an
  entry in the context menu of the editor and in Quick search, as do **Toggle inline blame** and **Blame file before
  commit of current line**.
- Worktrees in the window: the Worktrees list in **Kept aside** of the branch rail shows how each one stands, its
  branch with `↑n ↓m`, `missing` when its folder is gone, and a pencil with the count of its files not committed; its
  menu opens a worktree in a new window, in the external editor, in a terminal or the file manager, shows it in this
  window or its commit in the timeline, renames it, copies its path, removes it with or without its branch, and prunes
  the missing ones. **Create worktree…** from the list, from Quick search, from a branch and from a commit asks for a
  name with where its folder goes said as it is typed, then offers to open it and, the first time, **Choose What It
  Gets…**; **Switch worktree…** lists them to open one.
- A worktree shown in this window, or a submodule with **Show in Gitardo**, has a banner above the timeline and the
  Changes page with **Back to** the repository of the window; what is done there is done in it.
- A branch another worktree has shows the worktree's icon; its double click and the first entries of its menu open the
  worktree, and it can be neither checked out nor deleted here. With `branches.openIn` set to `worktree`, a branch
  opens in a worktree of its own, made the first time.
- Removing a worktree takes out its links to folders outside it first, on Windows, so that what they point to keeps
  its files, and leaves no empty folder behind; when git refuses for modified files, **Remove anyway** removes it
  with `--force`. A folder Windows holds for a moment is tried again.
- The worktree operations from `gitardo-cli`, with `--target worktree:<folder>`.
- **Show in the timeline** and a click on a tag, a stash or a worktree in the branch rail load the history up to the
  commit and take away what narrows the timeline when it leaves the commit out, as **Go to commit…** does; a merge
  folded over the commit unfolds. The bar of the history of a file says the branches it is narrowed to, `· in the
  local branches`. Several commits selected bring out no history. A narrow window shows **Sync** as its icon and its
  counts.
- **Open file** in the bar of a Diff page whose right side is the working tree opens the File page of the file.
- **Edit message…** is not in the menu of a commit while a bisect is under way, and says *End the bisect first.*
- **Edit in settings.json** on the settings page opens the settings file chosen in a File page, making the
  repository's `.gitardo/settings.json` with `{}` when it is not there.
- **Switch Worktree…** has a box over the worktrees: typing part of a label, of a branch or of a folder keeps the
  worktrees it finds.
- The theme picked in **Theme…** is written in the user's settings file by an operation, which the Command log shows.
- A cherry-pick of several commits that stopped on a conflict and is continued in Gitardo is taken back whole by one
  **Undo**.
- **Choose the model that resolves conflicts** in Quick search: the models the AI provider of `ai.provider` lists, and
  how much the model thinks, written to `ai.model` and `ai.effort` in the user's settings file. When the provider wants
  a key and none is kept, it is asked, and kept in the credential store of the system once the choice is written.
- **Resolve with AI** in the banner, in the message of an operation that stopped, on a file in conflict in the Changes
  page (the sparkle, for that file alone) and **Ask the AI about the rest (N)** in the review page: each conflict the
  rules leave is asked about, one per question and two at a time, its card showing the AI reading, then the lines of
  its answer as they come, then the proposal with the model's name, or why it failed with **Ask again**. A bar counts
  the answers, with **Stop**. The answers are kept by file and conflict until the window is closed, also when the page
  is closed or the conflicts restored, and are dropped once their conflict is edited. The head of the page has the
  button with the model's name, which chooses another.
- The sparkle beside the message of the commit box drafts the message with the AI provider: from the staged changes
  when something is staged, otherwise from every change with the new files named, in the style of the last 10
  subjects. The answer takes the place of the text of the box; while it is written, the Activity drawer says
  **drafting commit message** and the sparkle becomes a button that stops it, leaving the box as it was.
- A box that asks for a token hides what is typed in it: a dot stands for each character, and nothing is copied from
  it.
- **Load pull requests from GitHub** in **…**, when the main remote is on GitHub, and **Sign in to GitHub** in Quick
  search: the token is the one kept, or the one git's credential helper knows for `github.com`, or one asked for, and
  it is kept in the credential store of the system. From then on the open pull requests are read again after a fetch,
  a pull or a push, and after a read of the repository when the last request is more than two minutes old, also after
  a restart. A token GitHub refuses is taken out of the store, so that the next load asks for one again.
- Once the pull requests are loaded, the label of a branch that has an open one ends with its number, with the icon of
  a pull request or of a draft, in the color of its checks; its tooltip names it, and a click opens it in the browser
  without selecting the commit. The branch rail shows the same badge and says the pull request in the tooltip of the
  branch. The menu of a branch has **Open pull request #12**, or **Create pull request…** when it has none, and the
  menu of a remote branch **Open pull request #12**. After the first push of a branch, the window says "Pushed
  `<branch>`." with **Create pull request**.
- A commit of the timeline, a branch label on it, or a branch or a remote branch of the branch rail can be dragged
  onto a row of the timeline: the rows that take it are outlined as the pointer passes over them, the others show
  that nothing can be dropped there, and dropping it opens a menu at the pointer to cherry-pick the commit, merge the
  branch or rebase onto it, or rebase or reset the branch there.
- With the new setting `editor.wordWrap` set to `on`, the File page wraps a line longer than the page between its
  words, or inside a word longer than the page; its number and its blame stay on its first row, the other rows start in
  the column of the text, and a click or a selection follows the rows.
- In the Columns of the review of the conflicts, a long line wraps between its words rather than in the middle of one,
  and the screen reader is told each line and its number.
- **Compare with…** in the context menu of a file of the inspector, of the page of a commit (in its list and on the
  header of its diff), of the comparison page and of the comparison in the inspector compares that file with a branch,
  a tag, a commit or the working tree, in the Diff page.
- Comparing a file or a folder of another repository of the workspace with a branch, a tag or a commit shows that
  repository first.
- The **New Worktrees** page, from Quick search (**Choose What New Worktrees Get…**), from **Choose what new worktrees
  get…** in the **…** of the Worktrees list and from **Choose What It Gets…** after the first worktree is created: what
  git ignores here in sections with its size, what is proposed for each and **Link**, **Copy** or **Nothing**, a name
  in several places as one line, the link and copy patterns with what each takes, the commands to run, and where they
  are saved, with **Bring into the other worktrees**, **Remove from…** and **Edit in settings file**.
- **History of the active file** in **…** shows the history of the file of the File page in front. **Fold all
  merges** and **Unfold all merges** leave **…** while what a search finds is shown as a list.
- **Open on GitHub** in the comparison page, when the main remote is on GitHub and neither side is the working tree,
  opens the same comparison on github.com: a branch by the name of its upstream there, a tag by its name, anything
  else by its full hash, with `..` for **Everything that differs**.
- The Merge page proposes what the AI answered in the review page for a conflict still marked: its **✦** names the
  model, how sure it is, why, and how many lines neither side has, and the sparkle says how many of the proposals are
  the AI's.
- A row of the Worktrees list shows the open pull request of its branch, in the color of its checks, and its tooltip
  names it.
- **Delete from remote** names the branch `refs/heads/<branch>` when a tag has its name, so that the remote no longer
  refuses the push as ambiguous and the tag stays.
- **Undo** of a branch deleted in Gitardo asks with a confirmation, not a warning: making the branch again loses
  nothing.
- **Edit message…** on an older commit removes the files of its rebase from the git folder once the rebase ends, also
  when it stopped on a conflict in a commit made again after it; they were left behind before.
- **Apply patch** with a patch of one commit says **Apply `<name>` as 1 commit**, as it says **as 2 commits** for two.
- Creating a worktree with `branches.format` set asks for the inputs of the format, each with the value given last,
  after what the new branch starts from, and keeps them for the next time; at a commit `${base}` is picked among the
  branches at it, its short hash and the branches that have it, and from a detached `HEAD` it is its short hash.
- Renaming a worktree deletes the folders it leaves empty before renaming its branch, so they go even when renaming
  the branch fails.
- **Open in worktree** on a remote branch, and a worktree on a new branch made from a branch with the format, give
  git the full name of the branch when a tag or another branch has its short name.
- On Windows, deinitializing or removing a submodule whose folder is held is tried again, as moving or removing a
  worktree is.
- Adding a submodule in a folder whose name git could read as a pattern updates that folder only.
- Removing a submodule whose clone cannot be deleted afterwards says so in the log and is not a failure.
- What git said under a step that Gitardo got past, as a cherry-pick that changes nothing is skipped, is a warning
  of the log, not an error.
- A comparison narrowed to a file or a folder says how far apart the whole sides are, and what merging them would do,
  as one that is not narrowed does; from the working tree in **Since they diverged**, its files are those the other
  side changed since the two diverged.

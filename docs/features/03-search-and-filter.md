# Search and filter

Finding commits and narrowing the timeline. The search box at the top of the timeline finds commits among those loaded
or in the whole history, by message, author or what they changed, steps through them and goes to a commit by its hash;
**Go to commit…** goes to anything git takes for a commit. The branch filter chooses which branches, remote branches and
tags the timeline shows, from presets, from ticks in a list, or from the context menu of a reference, and remembers the
choice for each repository. The **Search** page of Gitardo Desktop, the last cards of this page, finds files, folders
and lines of text in the files of the worktree shown
([D73](../08-decisions.md#d73--search-a-page-that-finds-files-folders-and-text-in-the-worktree-shown)); in the
extension, Search Here and VS Code's own search do it, and those cards are *Host only* there.

### `search.box` — The search box

The box at the top of the timeline searches in one of five ways, chosen from the list next to it, and steps through what
it finds.

- **Kind**: View
- **Function**: `views::search::box_view(state: &SearchBox, ui) -> Vec<Intent>`
- **Entry points**: the search box at the top of the timeline, also in the narrow layout of the window.
- **Uses**: `search.loaded`, `search.history`, `search.step`, `search.list-mode`, `search.go-to-hash`.

- The list before the box chooses the mode; each says in its tooltip what it runs, and the box has a placeholder of its
  own:

  | Mode | Placeholder | Tooltip |
  | --- | --- | --- |
  | **Loaded** | Find by message, author or hash | Highlight matches among the loaded commits. Enter on a hash goes to that commit, however far back it is |
  | **Message** | Text in the message, then Enter | git log --grep: search every commit message |
  | **Author** | Author name or email, then Enter | git log --author: search every commit author |
  | **Changes** | Text added or removed, then Enter | git log -S: commits that add or remove this text |
  | **Changes (regex)** | Regex on changed lines, then Enter | git log -G: commits whose changed lines match this POSIX extended regular expression ([0-9], not \d) |

- **Loaded** is the mode when the window opens. With **Loaded**, what is typed is searched at once (`search.loaded`);
  with the other modes, when `Enter` is pressed (`search.history`). Typing in another mode, or choosing another mode,
  drops what was found. Going to **Loaded** searches what is in the box among the loaded commits.
- While git looks, a spinning icon. Then the count: `14`, or `2/14` when the selected commit is the second of fourteen,
  with `+` when the search stopped counting (`search.history`), and the tooltip **Only the first matches were
  counted**. With matches, two buttons step through them: up, **Previous match (Shift+Enter, Shift+F3)**, and down,
  **Next match (Enter, F3)**; a click on them leaves the keyboard in the box.
- The funnel at the end, there for every mode but **Loaded**, shows only what is found, as a list (`search.list-mode`).
- With **Loaded** and a hash that is not among the loaded commits, `Enter ↵` takes the place of the count
  (`search.go-to-hash`).
- The commits found stay where they are in the timeline and the others are dimmed; they have marks beside the scroll bar
  (`graph.scroll-marks`).

| Key | Does |
| --- | --- |
| `Enter`, `F3` or the arrow down | Selects the next match, and starts again from the first after the last (`search.step`) |
| `Shift+Enter`, `Shift+F3` or the arrow up | Selects the previous one |
| `Ctrl+F` (`Cmd+F` on macOS) | Puts the cursor in the box and selects its text, from anywhere in the timeline: there the key is the box's, not Find in page |
| `Escape` | Empties the box |

`F3` and `Shift+F3` work wherever the keyboard is in the timeline; `Enter` and `Shift+Enter` in the box.

Acceptance:

- [ ] The list has the five modes in that order, **Loaded** chosen when the window opens.
- [ ] `Ctrl+F` with the keyboard on the list of commits puts the cursor in the box with its text selected.
- [ ] `Escape` in the box empties it and the timeline is no longer dimmed.
- [ ] Choosing **Message** after a search of the loaded commits drops the highlight until `Enter`.
- [ ] With matches, the count says `<n>`, and `<i>/<n>` once one of them is selected.

### `search.loaded` — Find among the loaded commits

Message or author among the commits already loaded, highlighted as you type; or the start of a hash, from its fourth
character on. Nothing runs.

- **Kind**: Derivation
- **Function**: `model::search::loaded(rows: &[Row], text: &str) -> Vec<Oid>`
- **Entry points**: typing in the search box with **Loaded** chosen.
- **Uses**: by `search.box`.

- The text, with its spaces at both ends taken away and without regard to case, is found in the subject of a commit
  or in the name of its author.
- Text of four characters or more that are all hexadecimal digits also finds the commits whose hash starts with it:
  "add" and "bed" are words before they are digits, and somewhere in forty digits they are bound to turn up.
- The draft of the uncommitted changes is not a commit to find. Stashes in the timeline are found like commits.
- Empty text finds nothing and dims nothing.
- The result is in the order of the rows.

Acceptance:

- [ ] `PARSER` finds `Fix the parser` and a commit by an author named `Parser Bot`.
- [ ] `3f2a` finds the commit `3f2a1c9…`; `3f2` finds only commits with `3f2` in their message or author.
- [ ] `add` does not find a commit by its hash `add1…`, only by message or author; `add1` does.
- [ ] The text `Uncommitted` does not find the draft of the uncommitted changes.

### `search.read-matches` — Search the whole history

The commits of the history shown that a search of their messages, their authors or their changes finds, newest first.

- **Kind**: Read
- **Function**: `git::read::search::read_matches(git, request: &MatchesRequest) -> Result<Matches, GitError>`
- **Reads**: `git log --format=%H --max-count=5001 <search> <selection>`, where `<selection>` is that of the timeline
  (`filter.log-arguments`, with `--first-parent` when `graph.firstParent` is on, and the history of a file or some
  lines when the timeline shows it), and `<search>`:

  | Mode | Arguments | Finds |
  | --- | --- | --- |
  | **Message** | `--fixed-strings --regexp-ignore-case --grep=<text>` | Commits whose message contains the text, ignoring case |
  | **Author** | `--fixed-strings --regexp-ignore-case --author=<text>` | Commits whose author name or email contains the text, ignoring case |
  | **Changes** | `-S<text>` | Commits that add or remove the text |
  | **Changes (regex)** | `-G<text>` | Commits with a changed line that matches the POSIX extended regular expression |

  The same arguments, without `--format` and the count, narrow the timeline itself when the search is shown as a list
  (`search.list-mode`).
- **Refuses**: git refusing the search, as for an invalid regular expression, is an error with git's message, not a
  search that found nothing.
- **Uses**: `filter.log-arguments`.

Of the 5,001 hashes asked for, 5,000 are kept, and one more says there are more.

Acceptance:

- [ ] **Message** with `fix` finds `Fix the parser` and `Prefix…`, but not a commit with `f.x`: the text is not a
      pattern.
- [ ] **Author** with `@example.com` finds every commit of an author with such an email.
- [ ] **Changes** with `parseLog` finds the commits that add or remove that text, not those that only move a line with
      it within a file.
- [ ] **Changes (regex)** with `[0-9]{3}` finds a commit that changed a line with `404`.
- [ ] **Changes (regex)** with `(` is refused with git's error.
- [ ] A search with 6,000 matches returns 5,000 and says there are more.
- [ ] With the timeline narrowed to `main`, a commit only on `feature` is not found.

### `search.history` — Find in the whole history

All modes but **Loaded** search the whole history that is shown when you press `Enter`: every branch, remote branch and
tag, or what a filter leaves of them.

- **Kind**: View
- **Function**: `views::search::history_view(state: &HistorySearch, ui) -> Vec<Intent>`
- **Entry points**: `Enter` in the search box with **Message**, **Author**, **Changes** or **Changes (regex)** chosen.
- **Reads**: `search.read-matches`.
- **Uses**: `search.step`, `graph.show-in-graph`, `activity.describe-task`.

- `Enter` with text asks git; `Enter` again on the same search, once it has found, steps to the next match
  (`search.step`), and while git still looks does nothing more.
- When the commits found come, the first of them is selected and shown, going back as far in the history as it is
  (`graph.show-in-graph`), unless the commit selected is one of them.
- The commits found stay where they are in the timeline and the others are dimmed. The box counts them, stopping at
  5,000 with a `+`.
- What a search of the whole history found is found again each time the commits listed change, so that a commit made
  or fetched meanwhile, or the history of another filter, is counted too.
- The two that look inside the changes read every commit and can take a while on a large repository; the box has its
  spinning icon until git is done.
- A search git refuses says "Gitardo: the search failed." with the first line of git's message, counts 0, and is not
  asked again when the commits change.

Acceptance:

- [ ] **Message** `fix`, `Enter`: the commits with `fix` in their message stay in color and the first is selected.
- [ ] `Enter` again selects the second.
- [ ] A commit made with `fix` in its message while the search is there is counted once the timeline is read again.
- [ ] A match 3,000 commits back is selected after the history is loaded up to it.
- [ ] **Changes (regex)** with `(` says "Gitardo: the search failed." and leaves the timeline undimmed.

### `search.step` — Step through the matches

Selects the next match, or the previous one, going round at the ends.

- **Kind**: Derivation
- **Function**: `model::search::step(matches: &[Oid], selected: Option<Oid>, by: Direction) -> Option<Oid>`
- **Entry points**: `Enter`, `F3` and the arrow down; `Shift+Enter`, `Shift+F3` and the arrow up (`search.box`).
- **Uses**: `graph.show-in-graph`.

The matches are in the order of the history. With the selected commit among them, the next is the one after it, and
after the last the first; the previous is the one before it, and before the first the last. With the selected commit
not among them, the next is the first and the previous the last. With no match there is none. The match chosen is
gone to as `graph.show-in-graph` goes to a commit: a match further back than the history is loaded is loaded, and one
inside a folded merge unfolds it.

Acceptance:

- [ ] Of 14 matches with the 14th selected, next selects the 1st.
- [ ] With the 1st selected, previous selects the 14th.
- [ ] With a commit selected that is no match, previous selects the 14th.

### `search.list-mode` — Only the commits found, as a list

The funnel in the box shows the same search the other way: only the commits found, as a list, until you press it again
or choose **Show all commits**.

- **Kind**: View
- **Function**: `views::search::list_mode_view(state: &ListMode, ui) -> Vec<Intent>`
- **Entry points**: the funnel at the end of the search box, **Show only the commits found, as a list, instead of
  highlighting them in the timeline**, pressed while it is on.
- **Offered when**: the mode is not **Loaded**.
- **Reads**: `graph.read-commits` with the arguments of the search.
- **Uses**: `search.read-matches`, `filter.bar`, `graph.lanes`.

- While it is pressed, `Enter` narrows the timeline to the commits the search finds instead of highlighting them, a page
  at a time. They are not each other's history: they are drawn with no lines between them, with no draft,
  no stashes and no folds.
- The bar above the commits says `Commits whose message contains <text>`, `Commits by authors matching <text>`,
  `Commits that add or remove <text>` or `Commits whose changed lines match <text>`, with the branches when the timeline
  is narrowed to some, and the count, and has **Show all commits** (`filter.bar`).
- Pressing it with text in the box turns what is highlighted into the list; pressing it again turns the list back into
  a search highlighted in the timeline, or, with nothing in the box, shows all commits again.
- Whether it is pressed stays the way it is left, after a page took the place of the timeline and after the window is
  closed.
- While the list of a search in the changes is read, the search box says **Searching…**.

Acceptance:

- [ ] With the funnel pressed, **Author** `alice`, `Enter`: only Alice's commits are listed, with no lines, under a
      bar saying `Commits by authors matching alice`.
- [ ] **Show all commits** brings back the whole timeline.
- [ ] The funnel is still pressed after the window is opened again.
- [ ] The funnel is not there with **Loaded**.

### `search.go-to-hash` — Go to a commit by its hash

Type the hash of a commit, or its first four characters or more, with **Loaded** chosen; if it is not among the loaded
commits, `Enter` asks git for it wherever it is.

- **Kind**: View
- **Function**: `views::search::go_to_hash_view(state: &SearchBox, ui) -> Vec<Intent>`
- **Entry points**: the search box, with **Loaded** chosen.
- **Offered when**: the text, its spaces taken away, is 4 to 64 hexadecimal digits in any case, and no loaded commit
  matches it. A commit among the loaded ones is highlighted like any other match instead (`search.loaded`).
- **Reads**: `search.read-revision` with the text in lower case.
- **Refuses**: none or several commits start with it: "Gitardo: "abcd" is not a commit of this repository, or is the
  start of several.", with the text typed in place of `abcd`.
- **Uses**: `graph.show-in-graph`.

The box shows `Enter ↵`, with the tooltip **Press Enter to go to the commit with this hash, wherever it is in the
history**, in the place of the count. `Enter` goes to the commit as `graph.show-in-graph` does:

- further back than the history is loaded: the history is loaded up to it;
- left out by a filter in use, such as the history of a file, or by the branches shown: the filter is taken away, then
  the branches shown;
- on no branch or tag, such as a commit lost to a reset or a rebase: the timeline shows the history of that commit
  alone, until **Show all commits**.

Acceptance:

- [ ] Typing the first eight characters of a commit 4,000 commits back shows `Enter ↵`; `Enter` loads the history up
      to it and selects it.
- [ ] Typing the hash of a commit lost to `git reset --hard HEAD~1` and pressing `Enter` shows its history alone, with
      the bar `Only the history of <short hash> · on no branch or tag`.
- [ ] `ABCDEF12` is looked for as `abcdef12`.
- [ ] Three characters show no `Enter ↵`.
- [ ] A prefix shared by two commits is refused with that message.

### `search.read-revision` — The commit a name stands for

The commit that a hash, a branch, a tag or anything else git understands stands for.

- **Kind**: Read
- **Function**: `git::read::search::read_revision(git, request: &RevisionRequest) -> Result<Option<Oid>, GitError>`
- **Reads**: `git rev-parse --verify --quiet <revision>^{commit}`.
- **Refuses**: nothing is asked of git for an empty revision or one that starts with `-`, which git would take for an
  option: there is no commit.
- **Uses**: by `search.go-to-hash`, `search.go-to-commit`, `commit-page.open`, `compare.choose`.

Git answers with the full hash of the commit; with nothing for a name it does not know, for an abbreviated hash that
starts more than one object, and for a name of something that is not a commit and does not lead to one. For a place
in a reflog past its end, such as `stash@{3}` with two stashes, git fails rather than saying nothing: that is no commit
either.

Acceptance:

- [ ] `HEAD~3`, `v1.0^`, `main@{yesterday}` and a full hash each give their commit.
- [ ] An annotated tag gives the commit it points at.
- [ ] `--all` gives none and runs nothing.
- [ ] The hash of a blob gives none.
- [ ] `stash@{1}` with one stash and `HEAD@{99}` give none, not an error.

### `search.go-to-commit` — Go to commit…

Asks for anything git takes for a commit, and opens the timeline at it.

- **Kind**: Operation
- **Function**: `features::search::go_to_commit(context, answers) -> Outcome`
- **Entry points**: **Go to Commit…** in Quick search; **Go to commit…** in the **…** menu of the header
  (`graph.more-menu`), also in the narrow layout.
- **Asks**: **Go to commit**, in Gitardo's input box: "A commit hash, or anything else git understands: HEAD~3, v1.0^,
  main@{yesterday}". While typing, what git does not know is refused with "Git does not know a commit by this name in
  this repository." (each value checked with `search.read-revision`). No default.
- **Reads**: `search.read-revision`.
- **Plan**: no step: nothing changes in the repository.
- **Danger**: None.
- **Undo**: nothing to undo.
- **After**: the commit is selected in the timeline, as `graph.show-in-graph` goes to it.
- **Refuses**: with no repository: nothing is asked.
- **Uses**: `graph.show-in-graph`.

Acceptance:

- [ ] `main@{yesterday}` selects the commit `main` was at yesterday.
- [ ] Typing `nope` shows "Git does not know a commit by this name in this repository." and `Enter` does nothing.
- [ ] `HEAD~3` with the timeline narrowed to the history of a file that `HEAD~3` did not change shows all commits again
      and selects it.

### `filter.picker` — Choose the branches shown

The timeline shows every branch, remote branch and tag until you tell it which ones. The button before the search box
says what is shown and opens the list to choose from.

- **Kind**: View
- **Function**: `views::filter::picker_view(state: &BranchPicker, ui) -> Vec<Intent>`
- **Entry points**: the button of the branches shown, at the top of the timeline before the search box, with the icon
  of three lines that get shorter; in the narrow layout, the icon alone.
- **Uses**: `filter.preset`, `filter.tick`, `filter.rank-names`, `filter.label`; by `refs.list-filter`.

- The button says what is shown (`filter.label`), the first name and `+2` apart when several are ticked, so that the
  count stays when the name is cut; its tooltip is `Branches shown in the timeline: <what>`. It is pressed in while the
  timeline is narrowed.
- A click opens the list under it, as tall as the room there is, and puts the keyboard in its box; another click on
  the button closes it, and so do a click outside it, the window losing the focus, and the window being resized.
- At the top, **All**, **Current**, **Upstream**, **Local** and **Remotes** (`filter.preset`), the one in use marked,
  each with a tooltip:

  | Preset | Tooltip |
  | --- | --- |
  | **All** | Every branch, remote branch and tag |
  | **Current** | Only what is checked out, whichever branch that is as you switch |
  | **Upstream** | What is checked out and its upstream, whichever branch that is as you switch: what is ahead of it and what is behind |
  | **Local** | Only the local branches, and what is checked out |
  | **Remotes** | Only the remote branches, and what is checked out |

- Then the box **Filter branches and tags**, which leaves in the list the names that contain what is typed, in one list
  without the headings of the groups, the ones that match best first (`filter.rank-names`). `Escape` empties it, wherever the keyboard is in the list, and closes
  the list when it is empty.
- Then every local branch, remote branch and tag with a tick, under **Local**, **Remotes** and **Tags**, in the order of
  `refs.sort`, the branch checked out marked; the ticks are those of what is shown now (`filter.tick`). The timeline
  changes as they are ticked; a second tick does not wait for the timeline to show the first. With no name left it says
  **No branch or tag has that in its name**, and in a repository with none **No branches or tags yet**.
- Nothing at the bottom since D51: the branch rail has a filter of its own (`refs.list-filter`), which shows this
  list over the rail, titled **Branches listed**, with three check boxes of kinds at its bottom.

Acceptance:

- [ ] Clicking the button opens the list with the keyboard in its box; clicking it again closes it.
- [ ] Ticking `feature` from **All** shows only the history of `feature`; the button says `feature` and is pressed in.
- [ ] Ticking `fix` too makes the button say `feature` and `+1`.
- [ ] Typing `main` lists `main` before `origin/main` before `maintenance`.
- [ ] `Escape` with text empties the box; `Escape` again closes the list.

### `filter.label` — What the branches shown are called

The words for the branches the timeline shows, on the button of the filter and on the line at the top of the branch
rail.

- **Kind**: Derivation
- **Function**: `model::filter::label(shown: &BranchesShown) -> String`
- **Uses**: by `filter.picker` and `refs.scope-line`.

| Shown | Label |
| --- | --- |
| Everything | `All branches` |
| **Current** | `Current branch` |
| **Upstream** | `Current and upstream` |
| The local branches | `Local branches` |
| The remote branches | `Remote branches` |
| The tags | `Tags` |
| The local and the remote branches | `Branches` |
| The local branches and the tags | `Local branches and tags` |
| The remote branches and the tags | `Remote branches and tags` |
| References ticked one by one | the first by its short name, `main`, and `main +2` with two more |

Acceptance:

- [ ] Each row of the table gives its label.
- [ ] `refs/remotes/origin/main` ticked alone is labeled `origin/main`.

### `filter.preset` — All, Current, Upstream, Local, Remotes

Choose with one click: everything, only what is checked out, what is checked out and its upstream, only the local
branches, or only the remote ones.

- **Kind**: Derivation
- **Function**: `model::filter::preset(preset: Preset) -> Option<BranchesShown>`
- **Entry points**: the presets at the top of the branch filter (`filter.picker`).
- **Uses**: `filter.log-arguments`.

| Preset | The timeline shows |
| --- | --- |
| **All** | everything: nothing narrows it |
| **Current** | what is checked out |
| **Upstream** | what is checked out and the upstream of its branch |
| **Local** | the local branches, and what is checked out |
| **Remotes** | the remote branches, and what is checked out |

**Current** and **Upstream** follow you as you switch branch: they name no branch, and are worked out at each read.
**Upstream** shows at a glance what the branch has that its remote has not, and what there is to pull. What is checked
out is in the timeline whichever preset is chosen.

Acceptance:

- [ ] **Current** on `main`, then checking out `feature`, shows the history of `feature`.
- [ ] **Upstream** on `main` tracking `origin/main` shows the commits of both, and those of no other branch.
- [ ] **Upstream** on a branch with no upstream, or detached, shows what is checked out alone.
- [ ] **Local** while detached at a commit no local branch has still lists that commit.

### `filter.tick` — Tick the branches to show

Under the presets, every local branch, remote branch and tag has a tick: the timeline shows the ones ticked.

- **Kind**: Derivation
- **Function**: `model::filter::tick(shown: Option<&BranchesShown>, reference: &str, references: &References) -> Option<BranchesShown>`
- **Entry points**: the ticks of the branch filter (`filter.picker`).
- **Uses**: `filter.log-arguments`.

- The references ticked are those the choice in use stands for: with **All**, none; with **Local**, **Remotes** or a
  kind switched off in the menu of the view, every reference of the kinds shown; with **Current**, the branch checked
  out, none when detached; with **Upstream**, that branch and its upstream; with references ticked one by one, those.
- Ticking a reference adds it to them, and ticking it again takes it out. From a preset, a tick added or taken away
  starts from the references it stands for, and the result is references ticked one by one.
- Taking out the last one shows everything again.
- References are kept by their full names (`refs/heads/main`, `refs/remotes/origin/main`, `refs/tags/v1.0`), so that a
  branch and a tag of the same name are two.

Acceptance:

- [ ] From **All**, ticking `main` shows `main` alone.
- [ ] From **Local** with `main` and `feature`, unticking `feature` shows `main` alone, ticked one by one.
- [ ] From **Upstream** on `main`, ticking `v1.0` shows `main`, `origin/main` and `v1.0`.
- [ ] Unticking the only reference ticked shows everything.

### `filter.rank-names` — Which names match best

The names that contain what is typed, the ones that match best first, as both the branch filter and the branch
rail list them.

- **Kind**: Derivation
- **Function**: `model::filter::rank_names(names: &[String], typed: &str) -> Vec<usize>`
- **Uses**: by `filter.picker` and `refs.filter`.

Without regard to case, a name matches when it contains what is typed, and ranks, from the best:

1. the whole name: `main`;
2. its last part after a slash: `origin/main`;
3. its start: `maintenance`;
4. the start of one of its parts, split at `/`, `.`, `_` and `-`: `release/main-2`;
5. anywhere: `domain`.

A name that does not contain it is left out. Among names that match as well, their order stays. Nothing typed leaves
every name in its order.

Acceptance:

- [ ] For `main`, the names `domain`, `release/main-2`, `maintenance`, `origin/main`, `main` are listed `main`,
      `origin/main`, `maintenance`, `release/main-2`, `domain`.
- [ ] For `MAIN`, the same.
- [ ] For `fix`, `feature/x` is left out.
- [ ] `a/main` and `b/main` keep their order.

### `filter.list-too` — The branches the timeline leaves out, dimmed in the branch rail

The branch rail lists what its own filter lists, whatever the timeline shows; the branches the timeline leaves out are
dimmed there, and one of them can be added to the timeline from its row.

- **Kind**: View
- **Function**: `views::filter::list_too_view(state: &ListToo, ui) -> Vec<Intent>`: the eye at the end of a dimmed row
- **Entry points**: the rows of the branch rail while the timeline is narrowed.
- **Uses**: `filter.show-too`, `refs.list-filter`.
- **Extension**: the branches the graph leaves out are dimmed in the references, say **Not shown in the graph**, and
  have the eye that adds them to it. **Filter the list of branches too** and the switch above the references are gone,
  and `gitardo.branches.followGraphFilter` is not read; `RepoState.branchesFollowScope` and its message are retired.
  Tested by `test/e2e/ui.mjs`, step "the branches list the favorites and the ones checked out last first, all of them whatever
  the graph is narrowed to, what their own filter chooses, and can be put away; the status bar opens the graph".

While the timeline is narrowed, a branch, a remote branch or a tag of the rail that the timeline leaves out is dimmed,
says **Not shown in the timeline** in its tooltip, and has an eye at the end of its row, under the pointer, that adds
its history to the timeline (`filter.show-too`). The timeline's branch filter has no **Filter the list of branches
too**: what the rail lists is chosen in its own filter (`refs.list-filter`), and `branches.followGraphFilter` is not
read ([D51](../08-decisions.md#d51--the-branch-rail-has-a-filter-of-its-own)).

Acceptance:

- [ ] Narrowed to `main`: the Local section lists every branch, all but `main` dimmed.
- [ ] The eye on `feature` adds `feature` to the timeline.
- [ ] With the pointer moved from the dimmed row of `feature` onto its eye and resting there ten frames, each frame
      draws the eye and the row lit, none differing from the one before; a click then adds `feature` to the timeline.
- [ ] The branch filter of the timeline has no **Filter the list of branches too**.

### `filter.kinds` — Show local branches, remote branches, tags

Switch each kind of reference on and off in the timeline, the same choice the list makes.

- **Kind**: Derivation
- **Function**: `model::filter::kinds(shown: Option<&BranchesShown>, kind: ReferenceKind) -> Option<BranchesShown>`
- **Entry points**: **Show local branches**, **Show remote branches** and **Show tags** in the menu of the view of the
  header (`graph.view-menu`), each checked while that kind is shown.
- **Offered when**: always, but greyed out with the hint **Chosen in the filter** while references are ticked one by
  one, or **Current** or **Upstream** is chosen; the one kind left on is greyed out too: one of them is always on.

The kinds shown are switched one at a time. All three on is everything (**All**); local branches alone is **Local**,
remote branches alone **Remotes**; the other pairs and single kinds show those kinds. What is checked out is in the
timeline whichever kinds are shown.

Acceptance:

- [ ] From **All**, unchecking **Show tags** shows the local and the remote branches, and the button says `Branches`.
- [ ] Unchecking **Show remote branches** then is the same as **Local**.
- [ ] With only the tags shown, **Show tags** cannot be unchecked.
- [ ] With `main` ticked alone, the three entries are greyed out with **Chosen in the filter**.

### `filter.show-only` — Show only this history

Narrows the timeline to what a branch, remote branch or tag can reach.

- **Kind**: Derivation
- **Function**: `model::filter::show_only(reference: &str) -> BranchesShown`
- **Entry points**: **Show only this history** in the context menu of a branch, remote branch or tag: on its label in
  the timeline and in the branch rail; a click on a branch or a remote branch in the branch rail (`refs.tree`);
  **Show the commits** on the Welcome page, on the branch checked out (`welcome.page`).
- **Offered when**: the reference is not already the only one shown (then the menu has **Show all branches** alone).
- **Uses**: `filter.log-arguments`, `graph.read-commits`, `graph.unpushed`.

The timeline shows that reference alone, ticked one by one, whatever was shown before. What is checked out is not
added: narrowed to a branch that is not checked out, the draft of the uncommitted changes is not there
(`graph.uncommitted-row`). Narrowed to the branch checked out, the timeline is the commits of what is checked out: read
a page at a time by `graph.read-commits` like any other history, with the search and the filter of the timeline, and
the commits not pushed yet ringed (`graph.unpushed`); how far the branch is from its upstream, what is to pull
included, is said by the branch in the header and by **Sync** (`graph.toolbar`).

Acceptance:

- [ ] **Show only this history** on `v1.0` lists the commits `v1.0` reaches and no others.
- [ ] On `feature` while `main` is checked out, the commit of `main` is not listed unless `feature` reaches it.
- [ ] On the reference already shown alone, the menu offers **Show all branches** instead.

### `filter.show-too` — Show this history too

Adds a branch, remote branch or tag to the ones shown, when the timeline is narrowed already.

- **Kind**: Derivation
- **Function**: `model::filter::show_too(shown: &BranchesShown, reference: &str, references: &References) -> BranchesShown`
- **Entry points**: **Show this history too** in the context menu of a branch, remote branch or tag (on its label in
  the timeline, in the branch rail); the eye on a dimmed reference of the branch rail (`filter.list-too`).
- **Offered when**: the timeline is narrowed and the reference is not among those shown.
- **Uses**: `filter.tick`.

It is a tick added (`filter.tick`): from a preset, the references it stands for and this one, ticked one by one.

Acceptance:

- [ ] Narrowed to `main`, **Show this history too** on `feature` shows both, and the button says `main` and `+1`.
- [ ] Narrowed to **Current** on `main`, it shows `main` and `feature`, no longer following the branch checked out.
- [ ] With the timeline not narrowed, the entry is not offered.

### `filter.hide` — Hide this history

Takes a branch, remote branch or tag out of the ones shown.

- **Kind**: Derivation
- **Function**: `model::filter::hide(shown: &BranchesShown, reference: &str, references: &References) -> BranchesShown`
- **Entry points**: **Hide this history** in the context menu of a branch, remote branch or tag (on its label in the
  timeline, in the branch rail).
- **Offered when**: the timeline is narrowed, the reference is among those shown and is not the only one.
- **Uses**: `filter.tick`.

It is a tick taken away (`filter.tick`).

Acceptance:

- [ ] Narrowed to `main` and `feature`, **Hide this history** on `feature` shows `main` alone.
- [ ] Narrowed to **Local**, hiding `fix` shows every other local branch, ticked one by one.
- [ ] On the only reference shown, the entry is not offered.

### `filter.show-all` — Show all branches

Shows everything again.

- **Kind**: Derivation
- **Function**: `model::filter::show_all() -> Option<BranchesShown>`
- **Entry points**: **Show all branches** in the context menu of a branch, remote branch or tag while the timeline is
  narrowed; **Show all branches** on the bar above the commits (`filter.bar`); **Show all** on the line at the top of
  the branch rail (`refs.scope-line`); a second click on the branch the timeline is narrowed to in the branch rail
  (`refs.tree`); **All** in the branch filter.
- **Offered when**: the timeline is narrowed to some branches.

Acceptance:

- [ ] **Show all branches** on the bar shows every branch, remote branch and tag, and the bar goes.
- [ ] The button of the branch filter is no longer pressed in.

### `filter.bar` — The bar of what the timeline is narrowed to

While the timeline is narrowed, a bar above the commits says to what, with the way to undo it.

- **Kind**: View
- **Function**: `views::filter::bar_view(state: &NarrowedBar, ui) -> Vec<Intent>`
- **Entry points**: above the commits of the timeline, while the timeline is narrowed to some branches, to a search
  shown as a list, to the history of a file, a folder or some lines, or to the history of a commit on no branch.
- **Uses**: `filter.show-all`, `search.list-mode`, `file-history.file`, `graph.show-in-graph`.

- What it says, from what was narrowed last:
  - a search shown as a list: `Commits whose message contains <text>` (`by authors matching`, `that add or remove`,
    `whose changed lines match`);
  - the history of a file: `History of <path>`, with `· earlier <old names>` when it was renamed; of a folder: `History
    of the folder <path>`; of lines: see `file-history.`;
  - the history of a commit on no branch or tag: `Only the history of <short hash> · on no branch or tag`;
  - branches alone: `Only the history of` and what is shown: **the current branch**, **the current branch** and **its
    upstream**, **the local branches** and **the tags**, or the names of up to three references ticked, `main, feature
    and 2 more`.
- A search or a file history inside some branches adds `· in` and the branches; not the history of a commit on no
  branch, nor that of lines, which is followed from one commit.
- Then the number of commits listed, without the draft, with `+` while there are more: `· 1000+ commits`,
  `· 1 commit`.
- One thing at a time, what was narrowed last first: **Show all commits** takes away the search or the history of a
  file or a commit; with only branches left, **Show all branches** (`filter.show-all`).
- While the timeline is narrowed to some branches, the branch filter's button is pressed in.

Acceptance:

- [ ] Narrowed to `main`, `feature`, `fix`, `v1.0` and `v2.0`, the bar says `Only the history of main, feature, fix and
      2 more`.
- [ ] Narrowed to **Upstream**, it says `Only the history of the current branch and its upstream`.
- [ ] Showing the history of a file while narrowed to **Local** says `History of <path> · in the local branches`, and
      **Show all commits** leaves `Only the history of the local branches`.
- [ ] With 1,000 commits loaded and more, it says `· 1000+ commits`.

### `filter.log-arguments` — Which history the timeline lists

The arguments of `git log` that say which commits the timeline lists, from the branches shown, in which order:
everything but the format, the count, and what a search or the history of a file adds.

- **Kind**: Derivation
- **Function**: `model::filter::log_arguments(shown: Option<&BranchesShown>, head: Option<Oid>, upstream: Option<&str>) -> Vec<OsString>`
- **Uses**: by `graph.read-commits`, `graph.read-position`, `search.read-matches`.

Always `--date-order` first, then:

| Shown | Arguments |
| --- | --- |
| Everything | `--branches --remotes --tags HEAD` |
| Some kinds | `--branches`, `--remotes`, `--tags` for each kind shown, then `HEAD` |
| **Current** | `HEAD` |
| **Upstream** | `HEAD` and the full name of the upstream, from `git rev-parse --symbolic-full-name @{upstream}`, when that gives a name starting with `refs/` |
| References ticked one by one | `--ignore-missing` and their full names, so that a name is never taken for an option, and one deleted since it was chosen is passed over; only names under `refs/heads/`, `refs/remotes/` and `refs/tags/` |

`HEAD` is left out before the first commit. `refs/stash` is never listed: the stashes have their own place
(`graph.place-stashes`). What narrows the timeline further works inside the branches shown: the history of a file is the
one it has on them, and a search finds only their commits. Following some lines (`file-history.`) and the history of a
commit on no branch take one commit instead, whatever the branches shown.

Acceptance:

- [ ] Shown everything: `--date-order --branches --remotes --tags HEAD`.
- [ ] Local branches and tags: `--date-order --branches --tags HEAD`.
- [ ] **Upstream** on `main` tracking `origin/main`: `--date-order HEAD refs/remotes/origin/main`.
- [ ] `main` and `v1.0` ticked: `--date-order --ignore-missing refs/heads/main refs/tags/v1.0`.
- [ ] In a repository with no commits, shown everything: `--date-order --branches --remotes --tags`.

### `filter.remember` — The branches shown, remembered per repository

The choice is one for the timeline, wide or narrow, and is remembered for each repository of the workspace. A
branch that is deleted is forgotten.

- **Kind**: Derivation
- **Function**: `model::filter::remember(stored: &StoredValue, references: &References) -> Option<BranchesShown>`
- **Uses**: by `repo.select` and `graph.read-commits`.

- The branches shown are kept for each repository, by its folder, in Gitardo's storage, and come back when the
  repository is shown again and after the window is closed. Choosing another repository shows its own.
- At each read, the choice is checked against the repository: references ticked that no longer exist are left out,
  and with none left, everything is shown and nothing is kept for it. What is kept is the choice as it was read.
- At most 200 references are kept ticked one by one: each is an argument of `git log`, and a command line has an end.
- What is kept in a shape this version does not know shows everything.

Acceptance:

- [ ] Narrowing to `feature`, choosing another repository and coming back shows `feature` alone again.
- [ ] After `git branch -D feature` in a terminal, a timeline narrowed to `feature` and `main` shows `main` alone, and
      that is what is kept.
- [ ] Narrowed to `feature` alone, deleting `feature` shows everything.
- [ ] The choice is there after the window is closed and opened again on the workspace.

### `file-search.page` — The Search page

Finds files, folders and lines of text in the files of the worktree shown, in one page: what is looked for, where, the
results, and the preview of the one under the keyboard
([D73](../08-decisions.md#d73--search-a-page-that-finds-files-folders-and-text-in-the-worktree-shown)).

- **Kind**: View
- **Function**: `views::file_search::page_view(state: &SearchPageState, ui) -> Vec<Intent>`
- **Entry points**: **Search in files** in the bar of views (`graph.view-bar`); `Ctrl+Shift+F` (`Cmd+Shift+F` on
  macOS);
  **Search in files…** in Quick search; **Search here** in the context menu of a file or a folder of the Files page
  (`file-search.open`).
- **Reads**: `file-search.read-names` when the page opens and each time the repository is read while it is open;
  `file-search.read-text` as it is typed.
- **Uses**: `file-search.pathspecs`, `file-search.rank-names`, `file-search.list-results`, `file-search.preview`,
  `file-search.open`, `file-search.remember`, `file-view.page`, `ui.page-trail`, `graph.resize-panes`.
- **Extension**: Search Here, for those who have it, and VS Code's own search: nothing to build.

- At the top, the box of what is looked for, which has the keyboard when the page opens, and at its right three
  switches for the text: **Match case** (`Alt+C`), **Whole word** (`Alt+W`) and **Regular expression** (`Alt+R`); in a
  page too narrow for them beside the box, they are in a row under it. Names are matched as `file-search.rank-names`
  says, whatever the switches.
- Under it, the modes: **All**, the folders, the files and the lines of text in sections; **Text**; **Files**;
  **Folders**. `Alt+1` to `Alt+4` choose them. The keys of the modes and of the switches work from the box too: `Alt`
  with a key types nothing in a box. On macOS they are with `Ctrl` too, `Ctrl+Option+C`, since `Option` with a key
  types a character there. At their right, the scopes: **Repository**, the worktree shown;
  **Changes**, the files that differ from the last commit, the untracked ones too; and, once **Search here** named
  one, the folder or the file it named, by its path, with **×** that takes it away.
- Then **Include** and **Exclude**, patterns separated by commas (`file-search.pathspecs`), and two switches:
  **.gitignore**, on at first, off to search what git ignores too; **Hidden**, off at first, on to search the files
  and the folders whose name starts with a dot.
- The results are on the left (`file-search.list-results`), found as it is typed: the names at once, from the list of
  the files read when the page opened; the lines of text as git finds them. The search of the text running is stopped
  and another started when what is typed, a switch, the mode, the scope or a filter changes. Under the results a line
  counts them: `12 lines in 3 files`, and `5,000+ lines in 240 files` when there are more lines than are listed; in
  **Files** and **Folders**, `3 files`, and `2,000 of 5,282 files` when more match than are listed. When the
  repository is read again the results stay as they are until those of the new read are whole.
- On the right, the preview of the result under the keyboard (`file-search.preview`). The line between the two is
  dragged (`graph.resize-panes`). In the narrow layout the results take the page, with no preview.
- The first result is under the keyboard as soon as there is one. `↑` and `↓` go to the result before and after,
  from the box too, and `Page Up` and `Page Down` ten further; the headings are passed over. A click brings a result
  under the keyboard. `Enter`, or a double click, opens the result: the File page of a file, at the line of a
  line of text, with **Search** before it in its title row to come back (`ui.page-trail`); a folder is shown in the
  tree of the Files page, opened.
- While the files are listed the results say **Reading the files…**; with nothing typed in **Text**, **Type to search
  the text of the files.**; with nothing found, **No results**. A regular expression git refuses is said under the
  box, in git's words; a read that failed says what git said in place of the results.
- Each repository tab has its own search, kept while another is shown.

Acceptance:

- [ ] `usock` typed in **Files** lists `src/hooks/useSocket.ts`, and the box had the keyboard without a click.
- [ ] `socket` typed in **Text** lists the lines that hold it under their files, each with its line number, and the
      line under the results counts them.
- [ ] In **All**, the folders, the files and the lines found are under **Folders**, **Files** and **Text**.
- [ ] `Alt+2` chooses **Text**, and `Alt+C` turns **Match case** on: `socket` no longer finds a line with `Socket`
      alone.
- [ ] With **Changes** chosen, a line of a file that does not differ from the last commit is not listed, and a line
      of an untracked file is.
- [ ] `*.rs` in **Include** leaves the lines of the Rust files alone, and `docs/` in **Exclude** takes out those
      under a folder `docs`.
- [ ] With **.gitignore** off a line of `node_modules/pkg/index.js`, which git ignores, is found; with **Hidden** on,
      a line of `.github/workflows/ci.yml`.
- [ ] The first result is under the keyboard, `↓` from the box goes to the next, passing over a heading, and `Enter`
      on a line of text opens the File page at that line, with **Search** in its title row.
- [ ] `(` typed with **Regular expression** on says what git said under the box, and lists no line.
- [ ] A search that ends after another was started is thrown away: only the lines of what is typed are listed.

### `file-search.read-names` — The files of the worktree, for the search

The paths of the files the search lists by name and takes its folders from: those git tracks and those it does not
track, without those it ignores unless they are asked for, narrowed by pathspecs.

- **Kind**: Read
- **Function**: `git::read::file_search::read_names(git: &Git, request: &NamesRequest) -> Result<Vec<String>, GitError>`
- **Reads**: `git ls-files -z -t --full-name --cached --others --deleted --exclude-standard -- <pathspecs>`, run in
  the top folder of the worktree, or in the folder **Search here** named, whose files alone it then lists; without
  `--exclude-standard` when what git ignores is searched too. The pathspecs are those of `file-search.pathspecs`.
- **Uses**: `file-search.pathspecs`.

- Each path is from the top of the worktree, with `/`, once, in order. A file in conflict, listed once for each of
  its stages, is kept once.
- A file git tracks that is not in the working tree any more, which git lists with the tag `R` beside its tag `H`, is
  left out: it has no text to search and none to show.
- A submodule is one path, the folder git tracks for it.

Acceptance:

- [ ] The files git tracks and the untracked ones are listed, each once and in order, and
      `node_modules/pkg/index.js`, which git ignores, is not.
- [ ] Asked for what git ignores too, `node_modules/pkg/index.js` is listed.
- [ ] A file git tracks that was deleted from the working tree is not listed.
- [ ] Run in the folder `src`, only the files under it are listed, `src/ui/list.rs` by its path from the top.
- [ ] With the pathspecs of **Hidden** off, `.hidden` and `.github/workflows/ci.yml` are not listed.
- [ ] `docs/a b/é.md` is listed as it is written.

### `file-search.pathspecs` — What narrows a search to some files

**Include**, **Exclude** and **Hidden**, as the pathspecs git is given, so that git itself says which files are
listed and searched.

- **Kind**: Derivation
- **Function**: `model::file_search::pathspecs(filters: &Filters) -> Vec<String>`
- **Uses**: by `file-search.read-names` and `file-search.read-text`.

**Include** and **Exclude** hold patterns separated by commas, as in Search Here:

| Pattern | Finds | In **Include** | In **Exclude** |
| --- | --- | --- | --- |
| `src` | A file or a folder named `src`, anywhere | `:(glob)**/src`, `:(glob)**/src/**` | `:(exclude,glob)**/src`, `:(exclude,glob)**/src/**` |
| `./src`, `/src`, `src/app` | A path from the top of what is searched | `:(glob)src/app`, `:(glob)src/app/**` | `:(exclude,glob)src/app`, `:(exclude,glob)src/app/**` |
| `dist/` | Folders only, by name; `./dist/` from the top | `:(glob)**/dist/**` | `:(exclude,glob)**/dist/**` |
| `**/*.test.ts`, `*.{js,ts}` | A glob | as a name or a path, one for each alternative in braces | the same, excluded |

- A comma inside braces separates alternatives, not patterns; git has no braces in its globs, so each alternative is
  a pattern of its own. Spaces around a pattern are dropped, `\` is read as `/`, and a pattern that is empty, `.` or
  `./` is left out.
- A pattern with a `/` in it, other than at its end, is a path from the top, as one that starts with `./` or `/` is.
- With **Hidden** off, `:(exclude,glob)**/.*` and `:(exclude,glob)**/.*/**` are added: no file and no folder whose
  name starts with a dot.
- The top is the top of the worktree in **Repository** and **Changes**, and the folder **Search here** named when it
  named one, since git is run there (Q808).
- Git matches them: `*` and `?` stop at `/`, `**` crosses folders, and the case of letters matters.

Acceptance:

- [ ] `src` in **Include** gives `:(glob)**/src` and `:(glob)**/src/**`.
- [ ] `./src` and `src/app` give `:(glob)src`, `:(glob)src/**`, `:(glob)src/app` and `:(glob)src/app/**`.
- [ ] `dist/` in **Exclude** gives `:(exclude,glob)**/dist/**` alone.
- [ ] `*.{js,ts}, docs` gives the pathspecs of `*.js`, of `*.ts` and of `docs`: the comma in the braces separates no
      pattern.
- [ ] With **Hidden** off the two pathspecs of the names with a dot are added, and with it on none is.
- [ ] `src\ui` is read as `src/ui`, and ` , ` and `./` give nothing.

### `file-search.rank-names` — Which files and folders match best

The names that match what is typed, the best first, with the letters that matched, for the files and the folders of
the search.

- **Kind**: Derivation
- **Function**: `model::file_search::rank_names(names: &[String], request: &RankRequest) -> RankedNames`
- **Uses**: by `file-search.list-results`.

- What is typed is cut at its spaces into terms. A name is kept when every term matches it, whatever the case of the
  letters.
- A term matches the last part of the path, the name of the file or of the folder, when its letters are there
  together, or else in order with others between them: `usock` matches `useSocket.ts`. Failing that it matches the
  whole path, the same two ways. A term with a `/` in it is matched against the whole path only.
- In **All**, where the names share the list with the lines of text, a term whose letters are found only apart along
  the whole path does not match: too much would.
- From the best: a term found in the name before one found in the path alone; letters together before letters apart;
  a letter that starts a word, after `/`, `.`, `_`, `-` or a space, or a capital after a small letter, before one
  inside a word. When one term is typed, a name that is all of it, with or without its extension, comes first, then
  one that starts with it. Of two that match as well, the shorter path comes first, then the one before in the order
  of the names.
- At most as many names as asked for are kept, the best, and how many matched in all is said.
- With nothing typed, the names are kept in their order, as many as asked for.
- The folders of the search are those of its files: every folder that holds one of them, each once, in order.

Acceptance:

- [ ] `usock` finds `src/hooks/useSocket.ts` and gives the letters `u`, `S`, `o`, `c`, `k` of its name to mark.
- [ ] `use sock` finds it too, and `use zzz` finds nothing.
- [ ] For `list`, `src/ui/list.rs` comes before `src/ui/list_view.rs`, then `src/playlist.rs`, then
      `src/list/mod.rs`, found in its path alone.
- [ ] `LIST` ranks them the same.
- [ ] `ui/list` finds `src/ui/list.rs` and not `src/list/ui.rs`.
- [ ] `rail` finds `src/render/ai/list.rs`, its letters apart along the path, in **Files**, and not in **All**.
- [ ] Of 30 names that match with 20 asked for, the 20 best are given and 30 are counted.
- [ ] With nothing typed, the names come in their order.

### `file-search.read-text` — Search the text of the files

The lines of the files that hold what is looked for, as git finds them, each with every match in it.

- **Kind**: Read
- **Function**: `git::read::file_search::read_text(git: &Git, request: &TextRequest, take: &mut dyn FnMut(Vec<TextMatch>) -> Taken) -> Result<TextEnd, GitError>`
- **Reads**: `git grep --color=always --full-name -n -z -I --untracked <how> -e <text> -- <pathspecs>`, run in the top
  folder of the worktree or in the folder **Search here** named, with the color of a match set for that command to
  one of Gitardo's own and every other color to none (`-c color.grep.matchSelected=…`), so that git marks every match
  of a line, where its other forms say only the first. `<how>`:

  | What is asked | Arguments |
  | --- | --- |
  | Plain text | `-F` |
  | **Regular expression** | `-P`, or `-E` when git says it was built without PCRE |
  | **Match case** off | `-i` |
  | **Whole word** | `-w` |
  | **.gitignore** off | `--no-exclude-standard` |

- **Refuses**: a regular expression git cannot read is an error with git's words, not a search that found nothing.
- **Uses**: `file-search.pathspecs`.

- What git prints is read as it comes, a part at a time, and the lines of each part are handed on at once. The read
  stops git once it has handed on as many lines as asked for, 5,000 for the page, and says there are more.
- Each line comes with the path of its file from the top of the worktree, its number from 1, its text without the
  white space at its start, and where each match is in that text. A line longer than 240 characters is cut to 240,
  starting 60 before its first match when that is further in.
- The pathspecs are those of `file-search.pathspecs`. Files named one by one, as for **Changes**, are given as
  `:(top,literal)<path>`, in as many runs of git as their paths ask: at most 24,000 characters of them in each, for
  the length a command line has on Windows.
- Git searches the files it tracks and those it does not track, as they are in the working tree, the ignored ones
  too when asked. A file with a NUL byte is not searched (`-I`), and a submodule is not entered (Q809).
- The match is told from the text by the escape sequence of its color, which no other part of the output has (Q816).

Acceptance:

- [ ] `socket` finds the lines that hold it in the files git tracks and in the untracked ones, whatever the case of
      its letters, and none of `node_modules/pkg/index.js`, which git ignores.
- [ ] With **Match case**, `Socket` finds `useSocket` and not `socket`.
- [ ] With **Whole word**, `sock` does not find `socket`.
- [ ] `a.b` finds `a.b` and not `axb`; as a regular expression it finds both.
- [ ] `let a = useSocket(); let b = useSocket();` is given once, with its two matches.
- [ ] In `héllo socket` the match is the bytes of `socket`.
- [ ] `(` as a regular expression is an error that says what git said.
- [ ] A text no file holds gives no line and no error.
- [ ] With ten lines that match and three asked for, three are given and the read says there are more.
- [ ] Asked for what git ignores too, a line of `node_modules/pkg/index.js` is found.
- [ ] Given `src/a.rs` alone to search, a line of `src/b.rs` is not found.
- [ ] A line of 1,000 characters with its match at the 500th is given as 240 characters, the match marked in them.
- [ ] A file with a NUL byte is not searched.
- [ ] A line that comes in two parts of what git prints is given once, whole.

### `file-search.list-results` — The results, as the list shows them

What was found, as the rows of the list of the Search page, for the mode chosen.

- **Kind**: Derivation
- **Function**: `model::file_search::list_results(found: &Found, mode: Mode) -> Vec<ResultRow>`
- **Uses**: `file-search.rank-names`; by `file-search.page`.

- **Files** and **Folders**: a row for each name kept, the best first (`file-search.rank-names`), at most 2,000.
- **Text**: for each file with lines found, in the order git gave them, a row of the file that counts its lines, then
  a row for each line, with its number and its text.
- **All**: **Folders**, with at most 50 folders, then **Files**, with at most 100 files, then **Text**, with the
  lines as in **Text**, each under a heading; a section with nothing in it has no heading.
- With nothing typed, **Files**, **Folders** and **All** list the names in their order, and **Text** nothing.

Acceptance:

- [ ] In **Files**, the rows are the files kept, in their order.
- [ ] In **Text**, the two lines of `src/a.rs` are under a row of `src/a.rs` that counts 2, then the line of
      `src/b.rs` under its own.
- [ ] In **All**, **Folders**, **Files** and **Text** head their rows, in that order.
- [ ] In **All** with no folder found there is no **Folders**.
- [ ] In **All**, of 80 folders and 300 files kept, 50 and 100 are listed.

### `file-search.preview` — The preview of a result

The file of the result under the keyboard, beside the results, with the line found lit.

- **Kind**: View
- **Function**: `views::file_search::preview_view(state: &PreviewState, ui) -> Vec<Intent>`
- **Entry points**: the right side of the Search page.
- **Uses**: `file-view.highlight`, `file-view.page`, `file-view.file-icons`.
- **Extension**: the preview of Search Here: nothing to build.

- The file is read from the working tree when a result comes under the keyboard, and drawn as the File page draws a
  file: the font of code, the numbers of the lines, the colors of its code (`file-view.highlight`). Above it, its
  path. Until it is read, the preview before stays.
- For a line of text, that line is brought to the middle and lit across the preview, and every match found in the
  file is marked, the one of the line outlined.
- A file found by its name is shown from its first line, the matches found in it marked when the text was searched
  too.
- A folder shows what it holds, its folders first, then its files, each with its icon, from the files listed.
- The preview is read only, and nothing is selected in it. A double click on one of its lines opens the File page at
  that line, as `Enter` in the results does for the line found.
- A file with a NUL byte says **This file is not text.** A file larger than 1 MB is not read: **This file is too
  large to preview. Press Enter to open it.** A file that cannot be read says what the system said.

Acceptance:

- [ ] With a line of text under the keyboard, the preview shows its file with that line lit, and every match found in
      the file marked.
- [ ] `↓` to a line of another file shows that file.
- [ ] A file found by its name is shown from its first line.
- [ ] A folder shows what it holds, its folders first.
- [ ] A double click on a line of the preview opens the File page at that line.
- [ ] A file with a NUL byte says **This file is not text.**
- [ ] A file larger than 1 MB is not read, and the preview says so.

### `file-search.open` — Search from anywhere

What the Search page looks for and where, when it is asked for from a place of the window: the text selected there,
the file or the folder named.

- **Kind**: Derivation
- **Function**: `model::file_search::open_with(search: &Search, asked: &Asked) -> Search`
- **Entry points**: **Search** in the bar of views; `Ctrl+Shift+F` (`Cmd+Shift+F` on macOS) anywhere in the window;
  **Search in files…** in Quick search; **Search here** in the context menu of a file, of a folder and of the top
  row of the tree of the Files page.
- **Uses**: by `file-search.page`.

- From the bar of views, the key or Quick search, the page shows as it was left. Text selected in the File page in
  front, or in the column of the Diff page in front that has the keyboard, becomes what is looked for when it is not
  empty and is on one line; otherwise what was typed stays.
- **Search here** on a folder makes it a scope, after **Changes**, named by its path, and chooses it; on a file, the
  scope is that file alone; on the top row, **Repository** is chosen. There is one such scope: the next **Search
  here** takes its place. What is typed, the mode and the filters stay.
- The box takes the keyboard each time.

Acceptance:

- [ ] Asked for with `parse_log` selected in the File page in front, the box holds `parse_log`.
- [ ] A selection of two lines, or none, leaves what the box held.
- [ ] **Search here** on `src/ui` adds the scope `src/ui` and chooses it, and what is typed stays.
- [ ] **Search here** on `src/app.rs` makes that file the scope.
- [ ] **Search here** on the top row chooses **Repository**.
- [ ] **Search here** on `docs`, after `src/ui`, puts `docs` in its place.
- [ ] `Ctrl+Shift+F` shows the page from the timeline, and **Search in files…** of Quick search does too.

### `file-search.remember` — The search, remembered for each repository

What is searched comes back when the page is opened again in the same repository, after the window was closed too.

- **Kind**: Derivation
- **Function**: `app::persisted::file_search::keep_search(list: &mut Vec<RepositorySearch>, folder: &Path, search: &Search)`
- **Uses**: by `file-search.page`.

- What is typed, the three switches of the text, the mode, the scope (**Repository**, **Changes**, or the one
  **Search here** named, with its path), **Include**, **Exclude**, **.gitignore** and **Hidden** are kept for each
  repository, by its folder, under `fileSearches` in Gitardo's `state.json`, for the 20 repositories searched last
  (Q817).
- A search that is as a first one is, with nothing typed and nothing changed, is not written.
- What is kept in a shape this version does not know is left out, and the page starts as a first one.

Acceptance:

- [ ] `socket`, the mode **Text**, **Match case**, `*.rs` in **Include** and the scope `src/ui` read back as they
      were kept.
- [ ] A search as a first one is, is not written.
- [ ] Of 21 repositories searched, the 20 searched last are kept.
- [ ] After the window is closed and opened again on the workspace, the page shows what was typed.

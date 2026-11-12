# Compare

How two branches, tags, commits, or one of them and the working tree, differ: how far apart they are, whether merging
them would conflict, the commits only in each, and the files that differ, with their changes. A comparison opens in a
page of its own, one per pair; the comparison of a commit with what is checked out, or of two commits of the timeline,
shows first in the inspector, in place of the commit. A file or a folder is compared the same way from the Files page
and the File page.

### `compare.resolve-revision` — Which commit a revision is

Turns what the user types or chooses, a hash, a branch, a tag, `HEAD~3`, `v1.0^` or `main@{yesterday}`, into the commit
it stands for, or says there is none.

- **Kind**: Read
- **Function**: `git::read::compare::resolve_revision(git: &Git, request: &RevisionRequest) -> Result<Option<Hash>, GitError>`
- **Reads**: `git rev-parse --verify --quiet <revision>^{commit}`.

Acceptance:

- [ ] `HEAD~3`, `v1.0^`, `main@{yesterday}` and a short hash return the full hash of their commit.
- [ ] A tag returns the commit it points to, also an annotated one.
- [ ] A name git does not know returns none, and no error.
- [ ] An empty text, or one that starts with `-`, returns none without running git: it would be read as an option.

### `compare.side-choices` — What a side can be chosen from

The list every question about a side of a comparison shows, in Gitardo's own picker.

- **Kind**: Derivation
- **Function**: `model::compare::side_choices(input: &SideChoicesInput) -> Vec<PickItem>`

Rules:

- **Input**: the branches, remote branches and tags of the repository, `HEAD` and the branch checked out, the side
  chosen already if any, whether the working tree may be offered, and the comparisons opened before, when they are to
  be offered. **Output**: the items of the picker, in this order:
  1. under **Opened before**, the comparisons opened before, each as `<first> ↔ <second>` with the path it was narrowed
     to, when they are offered;
  2. **Enter a hash or a revision…**, described **such as 1a2b3c4 or HEAD~3**, which is always shown whatever is typed
     to filter the list, and asks for the text (see `compare.choose`);
  3. what is checked out, by the name of its branch or `HEAD`, described with its short hash and **checked out**, when
     there is a commit;
  4. **Working tree**, described **with the changes not committed**, when it may be offered and is not the side chosen
     already;
  5. under **Branches**, the local branches but the one checked out; under **Remote branches**, the remote branches;
     under **Tags**, the tags; each with its short hash, in the order of `refs.sort`.
- An item that is the side chosen already, the same name at the same commit, is left out: comparing something with
  itself shows nothing.
- Typing filters the items by their names and their descriptions.

Acceptance:

- [ ] The second question of a comparison does not offer the side chosen first.
- [ ] **Working tree** is not offered when the other side is the working tree.
- [ ] Typing a short hash finds the branch whose commit has it.
- [ ] Without a commit (a new repository), the item of what is checked out is not there.
- [ ] **Opened before** lists the latest comparison first.

### `compare.recent` — The comparisons opened before

Each repository remembers the comparisons opened last, so that one can be opened again as it was.

- **Kind**: Derivation
- **Function**: `model::compare::recent(input: &RecentInput) -> Vec<RecentComparison>`

Rules:

- **Input**: the list remembered for the repository and the comparison just opened, swapped or narrowed. **Output**:
  the new list, at most 8, the latest first.
- An entry has the two sides, each by its name and the commit it was at, and the path it was narrowed to.
- An entry with the same two names, in either order, and the same path as the new one is taken out: the two ways of a
  comparison are one, and the last is kept.
- Opening an entry again opens the comparison of its two names where they are now: a side named by a hash, or by the
  start of its own hash, stays that commit; the **Working tree** stays the working tree; any other name is resolved
  again (`compare.resolve-revision`), and stays at the commit remembered when it no longer resolves.
- The list is kept per repository, across restarts, in the state Gitardo keeps for the workspace.

Acceptance:

- [ ] Opening a ninth comparison drops the oldest of eight.
- [ ] Opening `main ↔ feature` after `feature ↔ main` leaves one entry, `main ↔ feature`, first.
- [ ] The same pair narrowed to `src` and not narrowed are two entries.
- [ ] An entry of `feature` opened again after `feature` moved compares where `feature` is now.

### `compare.choose` — Compare branches, tags or commits

Asks for the two sides of a comparison, one after the other, and opens it.

- **Kind**: Operation
- **Function**: `features::compare::choose(context, answers) -> Outcome`
- **Entry points**: **Compare Branches, Tags or Commits…** in Quick search; **Compare** at the top of the branch
  rail; **Compare branches, tags or commits…** in the **…** menu of the header.
- **Asks**:
  1. **Compare**, with the placeholder **The branch, tag or commit to start from, or a comparison opened before**: the
     choices of `compare.side-choices`, with **Opened before** and the **Working tree**. Choosing a comparison opened
     before opens it (`compare.recent`) and asks nothing more.
  2. **Compare `<first>` with**, with the placeholder **The branch, tag or commit to compare it with**: the same
     choices, without the first side and without **Opened before**.
  3. After **Enter a hash or a revision…**, an input box with the title of the question and the prompt **A commit hash,
     or anything else git understands: HEAD~3, v1.0^, main@{yesterday}**. What git does not know is refused while
     typing: **Git does not know a commit by this name in this repository.** A hash typed is named by its first seven
     characters, anything else by what was typed.
- **Reads**: `compare.resolve-revision`, while typing and again once the text is accepted.
- **Plan**: no step; it changes nothing.
- **Danger**: None
- **Undo**: nothing to undo: it changes nothing.
- **After**: opens the comparison page of the two (`compare.page`), or shows the page they have, and remembers the
  comparison (`compare.recent`).
- **Refuses**: a text accepted before its check came back, which git does not know: **"`<text>`" is not a commit of
  this repository.**
- **Uses**: `compare.side-choices`, `compare.recent`, `compare.page`.

Acceptance:

- [ ] Choosing `main` then `feature` opens the page `main ↔ feature`.
- [ ] Choosing **Working tree** as the second side compares the files as they are with the first.
- [ ] Typing `HEAD~3` as a side opens a comparison whose side is named `HEAD~3`.
- [ ] Typing a name git does not know cannot be accepted.
- [ ] Choosing an entry of **Opened before** opens that comparison, narrowed as it was, without a second question.
- [ ] Escape at either question opens nothing.

### `compare.follow-sides` — Followed sides and fixed ones

A side named by a branch, a tag or a revision follows it as it moves; a side given by its hash stays that commit.

- **Kind**: Derivation
- **Function**: `model::compare::follow_sides(input: &SideInput) -> CompareSide`

Rules:

- **Input**: a side as chosen, its name and the commit it was at, and what its name resolves to now. **Output**: the
  side as the page shows it: its name, its commit, whether it is followed, and whether it is gone.
- A side is followed when its name is not the start of its own hash and git resolves the name to that very commit when
  it is chosen; otherwise it is fixed. A name that git reads as another commit than the one chosen, such as a tag that
  has the name of a branch, stays the commit chosen.
- The **Working tree** is followed as the files change.
- Each time the comparison is shown again, while the window shows its repository, a followed side is resolved again: it
  moves to where its name is now. When the name no longer resolves, as for a branch deleted, the side stays at its last
  commit and is marked gone: the page says **gone** beside it.
- The tooltip of a side says which it is: **The comparison follows `<name>` as it moves**, **This commit**, **The files
  as they are now, with the changes not committed**, or **`<name>` no longer stands for a commit: the comparison stays
  where it was**, each followed by **. Choose another branch, tag or commit**.
- Two comparisons are the same page when they are of the same repository, their sides have the same names, both followed
  or both fixed and, when fixed, at the same commits, and they are narrowed to the same path.

Acceptance:

- [ ] A comparison of `main` and `feature` shows the new commits of `feature` once it moves, with no action.
- [ ] A comparison of a commit given by its hash shows the same thing after its branch moves.
- [ ] A branch deleted while its comparison is open leaves the side at its last commit, marked **gone**.
- [ ] Asking for `main ↔ feature` twice shows one page; asking for `feature ↔ main` opens another.

### `compare.initial-mode` — How two sides are compared at first

Two branches or tags are compared as a pull request shows them, a commit or the working tree directly.

- **Kind**: Derivation
- **Function**: `model::compare::initial_mode(input: &(CompareSide, CompareSide)) -> CompareMode`

Rules:

- **Input**: the two sides. **Output**: **Since they diverged** or **Everything that differs**.
- **Since they diverged** when both sides are followed (`compare.follow-sides`) and neither is the working tree;
  **Everything that differs** otherwise.
- Choosing another side in the page sets the mode again by this rule; swapping the sides keeps the mode.
- When the two histories never meet, **Since they diverged** cannot be chosen, and the comparison is direct whatever the
  mode says.

Acceptance:

- [ ] `main` and `feature` open with **Since they diverged**.
- [ ] `main` and a commit given by its hash open with **Everything that differs**.
- [ ] A comparison with the **Working tree** opens with **Everything that differs**.

### `compare.read-sides` — Read a comparison

Everything the comparison page shows of two sides, except the lines of the files.

- **Kind**: Read
- **Function**: `git::read::compare::read_sides(git: &Git, request: &CompareRequest) -> Result<CompareView, GitError>`
- **Reads**, with `<ours>` the first side and `<theirs>` the second, the working tree standing for `HEAD` in both,
  `<since>` the merge base in **Since they diverged** when there is one and the first side otherwise, and
  `-- :(literal)<path>` at the end of each command when the comparison is narrowed, a bare `--` when it is not:
  - `git merge-base <ours> <theirs>`: where they diverged, none when their histories never meet;
  - `git rev-list --left-right --count <theirs>...<ours> --`: how many commits each has that the other has not;
  - the files: `git diff --name-status -z -M <since> <second>`; when the second side is the working tree,
    `git diff --name-status -z -M <since>`; when the first is, `git diff --name-status -z -M -R <second>`;
  - the commits: `git log --left-right --max-count=2000 --format=<side mark, hash, author name, author date, subject,
    separated by %x1f, each record ended by %x1e> <ours>...<theirs>`;
  - when the two sides have 2,000 commits or fewer between them: `git rev-list --cherry-mark <ours>...<theirs>`; a
    commit it marks with `=` has its change on the other side too, and is **also in** that side;
  - when neither side is the working tree and each has commits the other has not:
    `git merge-tree --write-tree --no-messages --name-only -z <ours> <theirs>`, whose records after the first are the
    files that would conflict; an exit code other than 0 or 1, as with a git older than 2.40 or unrelated histories,
    means it cannot be told.
- **Refuses**: two sides that are both the working tree, or that are not commits of the repository: **These are not
  commits of this repository.**

Acceptance:

- [ ] Two branches three and two commits apart from their merge base return 3 and 2, and that merge base.
- [ ] In **Since they diverged**, the files are those the second changed since the merge base.
- [ ] In **Everything that differs**, the files are those that differ between the two.
- [ ] With the working tree as the second side, a file changed and not committed is among the files.
- [ ] A commit cherry-picked from one side to the other is marked **also in**.
- [ ] With more than 2,000 commits between them, no commit is marked and the newest 2,000 are returned.
- [ ] Narrowed to `src/app.rs`, only that file and the commits that change it are returned.
- [ ] A file named `pages/[id].tsx` narrows to that file only.

### `compare.read-patch` — Read the lines of a comparison

What changed in each file of a comparison, for its page, and the same as a patch to copy.

- **Kind**: Read
- **Function**: `git::read::compare::read_patch(git: &Git, request: &ComparePatchRequest) -> Result<ComparePatch, GitError>`
- **Reads**, with `<since>`, `<second>` and the path as in `compare.read-sides`, the working tree on either side given
  the same way:
  - the lines: `git diff --no-color --no-ext-diff --no-textconv --submodule=short -M <since> <second> -- [<path>]`;
  - the patch to copy: the same with `--binary`.
- **Uses**: `commit-page.diff-lines`, with the same cap of 20,000 lines.

Acceptance:

- [ ] The files come back in the order and number of the files of `compare.read-sides`.
- [ ] The patch to copy includes binary files, so that `git apply` takes it whole.

### `compare.page` — The comparison page

Two sides compared on a page: how far apart they are, what merging would do, the commits only in each and the files that
differ, with their changes.

- **Kind**: View
- **Function**: `views::compare::page(state: &CompareState, ui: &mut Ui) -> Vec<Intent>`
- **Entry points**: `compare.choose`, `compare.with`, `compare.path-with`, **Open as a page** of
  `compare.in-place`, an entry of **Opened before**.
- **Reads**: `compare.read-sides`, then `compare.read-patch`. A comparison with the working tree is read again whenever
  the repository is; the others when a followed side moves.
- **Uses**: `compare.follow-sides`, `compare.initial-mode`, `compare.summary`, `compare.change-side`, `compare.swap`,
  `compare.set-mode`, `compare.narrow-to-path`, `compare.commits`, `compare.take-file`, `compare.copy-patch`,
  `compare.open-on-github`, `details.compare-file-with`, `details.file-tree`, `details.show-all-files`,
  `commit-page.initial-folds`, `commit-page.open`, the Diff page (`diff.page`).

What it shows, from the top:

- **Comparison**, and the two sides, each a button with its icon (branch, remote branch, tag, commit or files) and name
  that chooses another in its place (`compare.change-side`), followed by its short hash when its name is not that hash,
  which opens the commit in its page, and **gone** when it is; between them, the arrows that swap them (`compare.swap`);
- what `compare.summary` says of how far apart they are and where they diverged, and what merging would do;
- the box that narrows the comparison to a file or a folder (`compare.narrow-to-path`);
- **Since they diverged** and **Everything that differs** (`compare.set-mode`), then, when files differ, **Open in the
  Diff page**, which opens all of them one after the other, **Copy as patch** (`compare.copy-patch`), and, when the
  repository is on GitHub, neither side is the working tree and the two are not the same commit, **Open on GitHub**
  (`compare.open-on-github`);
- **Commits** (`compare.commits`);
- the files that differ and their changes, as the page of a commit has them (`commit-page.view`): tree or list, lines
  gained and lost, **Show the changes**, folds, the cap of 20,000 lines and the click that opens a file in the Diff page,
  between the two sides. Each file also has, when the second side is not the working tree, an arrow that takes the
  version of the second side (`compare.take-file`), and in its context menu **Compare with…**, which compares the file
  as the second side has it, or, for a file the second side deleted, as it was where the comparison starts.

The page is titled `<first> ↔ <second>`, followed by ` · <last part of the path>` when it is narrowed. While the page is
compared again after the user changed something in it, it shows that it is waiting; **Comparing…** shows until the first
answer. `Ctrl+F` (`Cmd+F` on macOS) finds a word in the page. Each pair has one page (`compare.follow-sides`). The page
stays of its repository when the window is pointed at another.

Acceptance:

- [ ] The comparison of `main` and `feature` shows both names, their short hashes, how far apart they are, their commits
      and their files.
- [ ] A click on the short hash of a side opens the page of that commit.
- [ ] Editing a file of the working tree updates a comparison with the **Working tree** without any action.
- [ ] **Open in the Diff page** opens all the files listed in one Diff page.
- [ ] **Open on GitHub** is not there for a repository that is not on GitHub, nor when a side is the working tree.
- [ ] A click on a file opens its Diff page between the two sides as the mode says.
- [ ] Closing the page while it is being compared drops the answer without error.

### `compare.summary` — How far apart, and what merging would do

The sentences under the sides of a comparison.

- **Kind**: Derivation
- **Function**: `model::compare::summary(input: &CompareView) -> CompareSummary`

Rules:

- **Input**: the two sides, how many commits each has that the other has not (`ahead` for the second, `behind` for the
  first), the merge base, and the files that would conflict, if known. **Output**: a sentence of distance, a sentence of
  where they diverged, and a sentence of merging.
- Distance, for two commits: **`<second>` is N commits ahead of `<first>` and M commits behind.**, or **… is N commits
  ahead of `<first>`.**, or **… is M commits behind `<first>`, with none of its own.**; **The two are the same
  commit.** when they are. `1 commit` is singular.
- With the working tree on one side, the subject is the commit checked out against the other side: **The commit checked
  out is N commits ahead of `<other>`…**, or **The commit checked out is `<other>`: only the changes not committed
  differ.**
- Where they diverged: **They diverged at `<short hash>`.**, the hash opening that commit, when the merge base is
  neither side and neither is the working tree; **Their histories never meet.** when there is no merge base; nothing
  otherwise.
- Merging the second into the first, only for two commits where the second has commits of its own: **Merging
  `<second>` into `<first>` would fast-forward it.** when the first has none of its own; **… would not conflict.**
  when `git merge-tree` found no conflict; **… would conflict in N files:** followed by the first 10 files and **and K
  more**; nothing when it cannot be told.

Acceptance:

- [ ] `feature` with 3 commits of its own and `main` with none says `feature` is 3 commits ahead and merging would
      fast-forward `main`.
- [ ] Two sides with commits of their own and a conflict in 12 files name 10 of them and **and 2 more**.
- [ ] A comparison with the working tree at `HEAD` says only the changes not committed differ, and nothing of merging.
- [ ] Unrelated histories say **Their histories never meet.**

### `compare.change-side` — Choose another side

Puts another branch, tag, commit or the working tree in place of one side, in the same page.

- **Kind**: Operation
- **Function**: `features::compare::change_side(context, answers) -> Outcome`
- **Entry points**: a click on either side at the top of the comparison page.
- **Asks**: **Compare with `<second>`** for the first side, or **Compare `<first>` with** for the second, with the
  placeholder **The branch, tag or commit to put in place of `<side>`**: the choices of `compare.side-choices` of the
  repository of the page, the other side left out, and **Working tree** unless the other side is the working tree.
- **Plan**: no step; it changes nothing.
- **Danger**: None
- **Undo**: nothing to undo: it changes nothing.
- **After**: the page compares the new pair, in the mode `compare.initial-mode` gives, with the path it had; the
  comparison is remembered (`compare.recent`).
- **Uses**: `compare.side-choices`, `compare.follow-sides`, `compare.initial-mode`.

Acceptance:

- [ ] Choosing `release` in place of `main` compares `release` with the second side in the same page, retitled.
- [ ] The choices are those of the page's repository, also when the window shows another.
- [ ] Escape leaves the page as it was.

### `compare.swap` — Swap the two sides

Compares in the other direction: what the second has against the first becomes what the first has against the second.

- **Kind**: View
- **Function**: `views::compare::swap(state: &CompareState, ui: &mut Ui) -> Vec<Intent>`
- **Entry points**: the arrows between the two sides of the comparison page (**Compare in the other direction**).
- **Uses**: `compare.recent`.

Acceptance:

- [ ] Swapping `main ↔ feature` shows `feature ↔ main`, with the counts of commits exchanged and the files changed the
      other way.
- [ ] The mode and the path are kept.
- [ ] The swapped comparison is the latest under **Opened before**, in place of the other way.

### `compare.set-mode` — Since they diverged, or everything that differs

Chooses which changes are listed: what the second changed since it left the first, as a pull request shows it, or all
that differs between the two, what the first changed meanwhile included.

- **Kind**: View
- **Function**: `views::compare::set_mode(state: &CompareState, ui: &mut Ui) -> Vec<Intent>`
- **Entry points**: the two buttons of the comparison page: **Since they diverged** (`git diff <first>...<second>`) and
  **Everything that differs** (`git diff <first> <second>`), whose tooltips say these commands with the names of the
  sides.
- **Offered when**: **Since they diverged** is disabled, with the tooltip **Their histories never meet**, when the
  sides have no merge base.

Acceptance:

- [ ] **Since they diverged** lists only the files the second side changed since the merge base.
- [ ] **Everything that differs** also lists the files the first side changed since then.
- [ ] The button of the mode shown is pressed in.

### `compare.narrow-to-path` — Narrow a comparison to a file or a folder

Shows only the files of a file or a folder, and only the commits that change it.

- **Kind**: View
- **Function**: `views::compare::narrow_to_path(state: &CompareState, ui: &mut Ui) -> Vec<Intent>`
- **Entry points**: the box under the summary of the comparison page (**Only the changes of this file or folder**);
  `compare.path-with`, which opens a comparison narrowed already.
- **Uses**: `compare.recent`.

Rules of the path typed: spaces around it are dropped, `\` becomes `/`, a leading `./` or `/` and a trailing `/` are
dropped; what is left, unless empty or `.`, is the path from the top of the repository. `Enter` narrows, `Escape` puts
back the path in use, and **×** (**Show the changes of every file**) shows everything again.

Acceptance:

- [ ] Typing `src` and `Enter` lists only the files under `src` and the commits that change them, and the title of the
      page ends with ` · src`.
- [ ] The count of a side's commits then reads **N of M change `src`**.
- [ ] `./src/` narrows to `src`.
- [ ] **×** shows every file and commit again.

### `compare.commits` — The commits only in each side

Lists the commits that tell the two sides apart: the ones only in the second, then the ones only in the first.

- **Kind**: View
- **Function**: `views::compare::commits(state: &CompareState, ui: &mut Ui) -> Vec<Intent>`
- **Entry points**: **Commits** in the comparison page, shown when either side has commits of its own.
- **Reads**: `compare.read-sides`.
- **Uses**: `commit-page.open`, `details.show-all-files` (the same cap, for commits).

What it shows: a group **Only in `<second>`**, then **Only in `<first>`**, each folding, with its count, or **N of M
change `<path>`** when narrowed, followed by **, K also in `<other>`** when some are. Each commit, newest first, has its
short hash, subject, author and how long ago; a click opens its page. A commit whose change the other side has in a
commit of its own, cherry-picked or rebased, is greyed out and says **also in `<other>`**, with the tooltip **`<other>`
has the same change in a commit of its own: it was cherry-picked, or rebased**. A group lists 300 commits and **Show all
N commits…**. When the commits were too many to be matched, the heading says **Too many to look for the same changes on
the other side**, and a group with more commits than were read says **The newest N are listed.**

Acceptance:

- [ ] A commit only in the second side is under **Only in `<second>`**, with its author and date.
- [ ] A commit cherry-picked from the first side onto the second is greyed out with **also in `<first>`**, and counted
      in the heading.
- [ ] With 2,500 commits between the two, the heading says they were too many to match, 2,000 are read, and the note
      says the newest are listed.
- [ ] A click on a commit opens its page.

### `compare.take-file` — Take the version of the second side

Makes a file in the working tree and the index what the second side of the comparison has: its version of the file, or
no file when it has none.

- **Kind**: Operation
- **Function**: `features::compare::take_file(context, answers) -> Outcome`
- **Entry points**: the arrow on the row of a file in the comparison page: **Take the version of `<second>` into the
  working tree**, or, for a file the second side deleted, **Delete it from the working tree, as `<second>` has no such
  file**.
- **Offered when**: the second side is not the working tree.
- **Plan**:
  - a file the second side has: `git checkout <commit of the second side> -- <file>`, titled **Take `<file>` from
    `<second>`**, with the note **The file in the working tree and the index becomes the one of `<second>`.**;
  - a file it does not have: `git rm -f --ignore-unmatch -- <file>`, titled **Delete `<file>`, as in `<second>`**, with
    the note **`<second>` has no such file, so it is deleted from the working tree and the index.**;
  - when the file has changes that are not committed, or when that cannot be told because the window shows another
    repository, the note ends with **Its changes that are not committed are lost.**
  - The file is given to git as `:(literal)<file>` when its name has `[`, `]`, `*`, `?` or `\`, or starts with `:`.
- **Danger**: Confirm, because it replaces the file in the working tree and the index; Destructive when the file has
  changes that are not committed, because they are lost.
- **Undo**: impossible: no reference moves, and the changes that were not committed are kept nowhere.
- **After**: the repository is read again, and the comparison with it.

Acceptance:

- [ ] Taking `src/app.rs` from `feature` runs `git checkout <hash of feature> -- src/app.rs` after asking, and the file
      is staged with the contents of `feature`.
- [ ] Taking a file `feature` deleted runs `git rm -f --ignore-unmatch -- <file>`.
- [ ] A file with uncommitted changes is asked about with the warning that they are lost.
- [ ] The arrow is not on the files of a comparison whose second side is the working tree.

### `compare.copy-patch` — Copy a comparison as a patch

Copies the changes of the comparison, as listed, as a patch that `git apply` takes, binary files included.

- **Kind**: Operation
- **Function**: `features::compare::copy_patch(context, answers) -> Outcome`
- **Entry points**: **Copy as patch** in the comparison page (**Copy the changes as a patch that git apply takes**).
- **Offered when**: files differ.
- **Reads**: `compare.read-patch`, the patch to copy, with the mode and the path of the page.
- **Plan**: no step; it changes nothing in the repository.
- **Danger**: None
- **Undo**: nothing to undo: it changes nothing.
- **After**: puts the patch on the clipboard and says **Copied the changes of N files as a patch.**; with nothing to
  copy, says **There are no changes to copy.**

Acceptance:

- [ ] The patch copied, applied with `git apply` on the first side, gives the files of the second side, in **Everything
      that differs**.
- [ ] A binary file changed is in the patch.
- [ ] Narrowed to a folder, only its files are in the patch.
- [ ] The message counts the files of the patch.

### `compare.open-on-github` — Open the comparison on GitHub

Opens the same comparison on github.com, each side by the name it has there.

- **Kind**: Operation
- **Function**: `features::compare::open_on_github(context, answers) -> Outcome`
- **Entry points**: **Open on GitHub** in the comparison page (**Compare the two on GitHub, once they are pushed**).
- **Offered when**: the main remote of the repository is on github.com, neither side is the working tree, and the two
  are not the same commit.
- **Plan**: no step; it changes nothing.
- **Danger**: None
- **Undo**: nothing to undo: it changes nothing.
- **After**: opens `https://github.com/<owner>/<repository>/compare/<first>...<second>` in the browser of the system,
  with `..` in place of `...` in **Everything that differs**; each part of a name between `/` is encoded for a URL.

How each side is named there: a followed local branch by the branch of its upstream when that upstream is on the GitHub
remote and not gone, else by its commit; a followed remote branch of the GitHub remote by its name without the remote;
a followed tag by its name; anything else, a fixed side or a side marked gone, by its full hash, which GitHub knows once
it is pushed.

Acceptance:

- [ ] `main ↔ feature`, both tracking `origin` on GitHub, opens `…/compare/main...feature`.
- [ ] A local branch whose upstream is `origin/feat-x` is named `feat-x`.
- [ ] A side given by its hash is named by its full hash.
- [ ] In **Everything that differs** the URL has `..`.

### `compare.path-with` — Compare a file or a folder with a branch, tag or commit

Compares a file or a folder as it is in the working tree with the branch, tag or commit the user chooses, in a
comparison page narrowed to it.

- **Kind**: Operation
- **Function**: `features::compare::path_with(context, answers) -> Outcome`
- **Entry points**: **Compare with Branch, Tag or Commit…** in the context menu of a file or a folder of the Files page,
  in the context menu and the title of the File page; Quick search, for the file of the File page shown.
- **Asks**: **Compare `<file or folder name>` with**, with the placeholder **The branch, tag or commit to compare the
  working tree with**: the choices of `compare.side-choices`, without **Working tree**.
- **Plan**: no step; it changes nothing.
- **Danger**: None
- **Undo**: nothing to undo: it changes nothing.
- **After**: points the window at the repository of the file when it is another (`repo.select`), and opens the
  comparison of the side chosen with the **Working tree**, narrowed to the file or folder (`compare.narrow-to-path`);
  the top folder of a repository is the whole repository.
- **Refuses**: a file outside every repository of the workspace, or no file at all: **Choose a file or a folder of this
  repository to compare.**

Acceptance:

- [ ] On `src/app.rs` in the Files page, choosing `main` opens `main ↔ Working tree · app.rs`, listing only that file.
- [ ] On a folder, the page lists only the files under it.
- [ ] On a file of another repository of the workspace, the window switches to it first.

### `compare.with` — Compare with…

Compares a commit, branch or tag with anything else, asking only for the other side.

- **Kind**: Operation
- **Function**: `features::compare::with(context, answers) -> Outcome`
- **Entry points**: **Compare with…** in the context menu of a commit in the timeline, of a local branch, of a remote
  branch and of a tag, in the timeline and the branch rail.
- **Asks**: **Compare `<name>` with**, with the placeholder **The branch, tag or commit to compare it with**: the choices
  of `compare.side-choices`, the side itself left out, with **Working tree**.
- **Plan**: no step; it changes nothing.
- **Danger**: None
- **Undo**: nothing to undo: it changes nothing.
- **After**: opens the comparison page of the two, the one the menu was opened on first.
- **Uses**: `compare.side-choices`, `compare.page`.

Acceptance:

- [ ] **Compare with…** on `feature`, choosing `main`, opens `feature ↔ main`.
- [ ] On a commit, the side is named by its short hash and is fixed.
- [ ] `feature` itself is not among the choices.

### `compare.with-current` — Compare with the current branch

Compares a commit, branch or tag with what is checked out, at once, in the inspector.

- **Kind**: Operation
- **Function**: `features::compare::with_current(context, answers) -> Outcome`
- **Entry points**: **Compare with `<current branch>`** (or **Compare with HEAD** when no branch is checked out) in the
  context menu of a commit, a local branch, a remote branch or a tag, in the timeline and the branch rail.
- **Offered when**: there is a commit checked out and the item is not at that commit.
- **Plan**: no step; it changes nothing.
- **Danger**: None
- **Undo**: nothing to undo: it changes nothing.
- **After**: shows the in-place comparison (`compare.in-place`) of the item, first, with what is checked out, second,
  in the inspector, whether it was asked from the timeline or from the branch rail.

Acceptance:

- [ ] On `feature`, with `main` checked out, the inspector shows the files that differ from `feature` to
      `main`.
- [ ] The entry is not on the commit checked out, nor on a branch at that commit.

### `compare.selected-before` — Compare two commits of the timeline

Compares the commit selected before with the one right-clicked.

- **Kind**: Operation
- **Function**: `features::compare::selected_before(context, answers) -> Outcome`
- **Entry points**: **Compare with `<short hash>`, selected before** in the context menu of a commit in the timeline;
  **Compare these two commits** in the bar and the context menu of a selection of two commits (`selection.bar`).
- **Offered when**: a commit was selected, the commit right-clicked is another, and the one selected is not the
  draft.
- **Plan**: no step; it changes nothing.
- **Danger**: None
- **Undo**: nothing to undo: it changes nothing.
- **After**: shows the in-place comparison (`compare.in-place`) from the commit selected before to the one clicked,
  each named by its short hash.

Acceptance:

- [ ] Selecting `a1b2c3d`, then right-clicking `e4f5a6b`, offers **Compare with a1b2c3d, selected before**, which lists
      the files that differ from `a1b2c3d` to `e4f5a6b` in the inspector.
- [ ] The entry is not offered when nothing was selected, or on the commit selected itself.

### `compare.read-files` — Read the files that differ between two commits

The files the in-place comparison lists.

- **Kind**: Read
- **Function**: `git::read::compare::read_files(git: &Git, request: &FilesRequest) -> Result<Comparison, GitError>`
- **Reads**: `git diff --name-status -z -M <from> <to>`.

Acceptance:

- [ ] The files are those that differ between the two commits, renames with both names.
- [ ] Two sides that are not full hashes return nothing, without running git.

### `compare.in-place` — The comparison in the inspector

The files that differ between two commits, shown in the inspector in place of the card of a commit, with the way to
swap them and to open the whole comparison as a page.

- **Kind**: View
- **Function**: `views::compare::in_place(state: &InPlaceState, ui: &mut Ui) -> Vec<Intent>`
- **Entry points**: `compare.with-current`, `compare.selected-before`.
- **Reads**: `compare.read-files`.
- **Uses**: `details.show-all-files`, `details.compare-file-with`, `compare.page`, the Diff page (`diff.page`).

What it shows: **Comparison**, a button that closes it and gives the inspector back to the commit selected, **From** and
**To**, each with its name when it is not its short hash and its short hash, which goes to that commit in the timeline
(**Show this commit**); **Swap** (**Compare in the other direction**), which exchanges the two; **Open as a page**
(**How far apart the two are, and every change, on a page**), which opens the same two in the comparison page; **N
changed files**, with, for more than one, a button that opens all of them in one Diff page titled `<from> ↔ <to>`; then
the files, a click on one opening its Diff page from the first to the second. **Loading…** shows until the files are
read.

Acceptance:

- [ ] **Swap** shows the files changed from the second to the first, the names exchanged.
- [ ] **Open as a page** opens the comparison page of the same two sides, named as here.
- [ ] A click on a file opens its Diff page between the two commits.
- [ ] Closing it shows the card of the commit selected in the inspector again.
- [ ] Asked from the branch rail, the comparison shows in the inspector too.

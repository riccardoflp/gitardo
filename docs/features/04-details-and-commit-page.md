# Details, commits and the page of a commit

How a commit is read. The details beside the graph summarize the commit selected: its message, who made it, and the
files it changed. The **Commits** pane of the side bar lists the commits of what is checked out, apart from the graph,
and says which are pushed. The page of a commit, in a tab of its own, shows it whole with what it changed in each file.
The derivations that shape a message and a list of files for all three are here too.

### `details.read-commit` — Read the details of a commit

Everything the details, the Commits pane, the Stashes pane and the page of a commit show of one commit: its message,
author, committer, dates, parents and the files it changed, and, for a stash, the untracked files it took.

- **Kind**: Read
- **Function**: `git::read::details::read_commit(git: &Git, request: &CommitRequest) -> Result<CommitDetail, GitError>`
- **Reads**:
  - `git show --no-patch --format=<hash, parents, author name, author email, author date, committer name, committer
    email, committer date, raw message, separated by %x1f> <hash>`;
  - the files, against the first parent: `git diff --name-status -z -M <first parent> <hash>`; for a commit with no
    parent, `git diff-tree --root --no-commit-id -r --name-status -z <hash>`;
  - for a commit with three parents (a stash), `git show --no-patch --format=%P%x1f%s <third parent>`; when its subject
    starts with `untracked files on `, the files of that commit with
    `git diff-tree --root --no-commit-id -r --name-status -z <third parent>`.
- **Uses**: `details.message-title` to split the raw message into subject and body.

Acceptance:

- [ ] The files of a commit with one parent are those `git diff --name-status -M` gives against that parent, renames
      with both names.
- [ ] The files of a merge are those it changed against its first parent only.
- [ ] The first commit of a repository lists every file it added.
- [ ] A stash made with `--include-untracked` returns its untracked files apart from the others; a stash made without
      untracked files, or whose untracked commit is empty, returns none.
- [ ] A hash that is not a commit of the repository returns no detail rather than an error the user sees.
- [ ] The uncommitted changes, asked for as a commit, return the subject **Uncommitted changes**, no author, and the
      files of the working tree and the index together.

### `details.show` — Details of the selected commit

The summary of the commit selected in the graph, beside the commits or under them when the Graph tab is narrow: its
message, who made it and when, its hash and parents, and the files it changed, each one click away from its diff.

- **Kind**: View
- **Function**: `views::details::show(state: &DetailsState, ui: &mut Ui) -> Vec<Intent>`
- **Entry points**: selecting a commit, a stash or the row of the uncommitted changes in the Graph tab; the same in the
  narrow layout of the Graph tab, where the details are under the commits.
- **Reads**: `details.read-commit`.
- **Uses**: `details.message-title`, `details.message-unwrap`, `details.show-more`, `details.file-tree`,
  `details.show-all-files`, `details.compare-file-with`, `details.stash`, `commit-page.open`, `message.edit`,
  `file-history.file`, the Diff page (`diff.page`), `compare.in-place` (which takes the place of the details).

What it shows, from the top:

- a button that closes the details, one beside it that opens the commit in its tab (**Open in a tab, with its changes**;
  not on the uncommitted changes), and the pencil of `message.edit` where that is offered;
- the title of the message in bold and the rest under it, as `details.message-title` and `details.message-unwrap` make
  them, cut by `details.show-more`;
- **Author** (the email in the tooltip), **Date**, **Committer** with its date only when the committer or the date
  differs from the author's, **Commit** with the short hash, which a click copies whole (**Copy full hash**), and
  **Parent** or **Parents**, each short hash going to that commit in the graph;
- the count, **N changed files**, followed by **against the first parent** on a merge;
- the two buttons that list the files one after the other, each with its directory beside its name (**One after the
  other, each with its directory**), or as a tree of directories (**Under their directories**); the choice is
  remembered and starts as the list;
- with more than one file, a button that opens the diffs of all of them one after the other in one Diff page (**Open
  all changes in one Diff page**), titled with the short hash and subject;
- the files, each with its status, its icon from the built-in file icons, its name and, on its row, a button that shows
  its history (**Show the history of this file**). A click opens its diff in the Diff page, against the first parent.
  Its context menu has **Compare with…**.

On the row of the uncommitted changes the details have no author, hash or parents. Each file has a box to tick, the
buttons of `stash.push-file` and of discarding its changes, and the buttons above the files act on the ticked files, or
on all of them when none is ticked (see `stash.push-file` and `changes.discard`).

Acceptance:

- [ ] Selecting a commit shows its title, author, date, short hash and parents, and the count of its files.
- [ ] The committer is shown only when its name or date differs from the author's.
- [ ] A click on the hash puts the full hash on the clipboard.
- [ ] A click on a parent selects that commit in the graph.
- [ ] A merge says **against the first parent** after the count.
- [ ] A click on a file opens the Diff page of that file between the first parent and the commit.
- [ ] The button next to the count opens all the files in one Diff page; it is not there with a single file.
- [ ] The layout chosen, list or tree, is the one the details have after the window is opened again.
- [ ] The button beside the close button opens the page of the commit (`commit-page.open`).
- [ ] While loading, the details say **Loading…**.

### `details.message-title` — The title of a message

A commit message whose first paragraph is long, often a message written as one paragraph with no title, would fill the
details with bold text: its title becomes its first sentence, or its first words, and the rest goes on under it.

- **Kind**: Derivation
- **Function**: `text::details::message_title(input: &RawMessage) -> ShownMessage`
- **Uses**: `details.message-unwrap` for the text under the title.

Rules:

- **Input**: the raw message of a commit. **Output**: a subject, a body, and from them a title and a text.
- The subject is the first paragraph, as git takes it, but only as far as its first line goes once the paragraph is
  unwrapped (`details.message-unwrap`): a list written right under the first line is not part of the subject, and goes
  to the body. The body is everything else, trimmed.
- A subject of at most 100 characters is the title as it is, and the text is the body unwrapped.
- A longer subject is cut after its first sentence: a `.`, `!` or `?`, possibly followed by a closing quote, parenthesis
  or bracket, then white space and a capital letter or a digit. The first such end that is at least 20 characters from
  the start is taken, if it comes no later than 120 characters; the rest of the subject starts the text, followed by
  the body after an empty line.
- When no sentence ends there, the subject is cut at the last space before the 80th character: the title ends with `…`,
  and the text starts with `…` and the rest. With no such space, the subject is the title whole.
- The commit itself is never changed: this is how the message is shown.

Examples:

- `Fix the parser` is the title, with nothing under it.
- A subject of 300 characters that starts `Read the configuration once at startup. Then …` has the title `Read the
  configuration once at startup.` and the text `Then …`.
- A subject of 150 characters with no full stop has a title of its words up to 80 characters, ending with `…`.

Acceptance:

- [ ] A subject of 100 characters or fewer is the title unchanged.
- [ ] A one-paragraph message of several sentences gets its first sentence as the title, and the others open the text.
- [ ] A first sentence shorter than 20 characters is not taken: the next end of sentence is.
- [ ] A sentence ending after 120 characters is not taken: the title is cut at a space with `…` instead.
- [ ] A list right under the first line goes to the body, not into the title.
- [ ] A dot followed by a digit or a lower-case letter with no space between, as in `v1.2` or `e.g. this`, does not
      end a sentence.

### `details.message-unwrap` — Lines of a message put back together

Messages are written to fit in 72 columns; shown in a narrower or wider place they would break twice a line. The lines
that were cut to fit are joined again, so that the text wraps at the width of the details or of the page.

- **Kind**: Derivation
- **Function**: `text::details::unwrap_message(input: &str) -> String`

Rules:

- **Input**: the body of a message. **Output**: the same body with some line breaks replaced by a space; no other
  character changes.
- The trailers are the last paragraph when every line of it is `Key: value` (a key of letters, digits and dashes):
  `Co-Authored-By: …`, `Signed-off-by: …`. They stay as written.
- The width of the message is its longest line of prose (a line with at least two words, before the trailers), up to 72.
  When that line is shorter than 50 characters the message was not wrapped, and is returned as it is.
- A line is joined to the line before it only when the line before can take more and the first word of this line would
  have taken the line before past the width: that is, the line before was cut.
- Never joined, and never joined to: lines inside a fenced block (between lines starting with ```` ``` ```` or `~~~`)
  and the fences themselves, empty lines, the trailers, quotations (`>`), headings (`#`), a line that is a single
  address (`scheme://…`), and a line that is one word of 30 characters or more.
- An item of a list (`-`, `*`, `+`, `•`, or a number followed by `.` or `)`) starts a new line; the lines that go on
  after it, indented or not, are joined to it as above. Elsewhere an indented line is laid out on purpose and stays.

Examples:

- Two lines of 70 and 30 characters, where the first word of the second would not have fitted on the first, become one
  line.
- A line of 40 characters followed by another line stays two lines: the first ended early.
- `- first item` followed by `- second item` stays two items.

Acceptance:

- [ ] A paragraph wrapped at 72 columns becomes one line.
- [ ] A message whose longest line is shorter than 50 characters is unchanged.
- [ ] A line that ends early is not joined to the next.
- [ ] Lists, indented lines, quotations, headings, fenced blocks, addresses and trailers are unchanged.
- [ ] The continuation of a list item, wrapped and indented to the item's text, is joined to the item.
- [ ] No character other than the line breaks joined changes.

### `details.show-more` — Show more of a long message

A long message in the details shows its first lines only, so that the files stay in sight under it; **Show more**
shows it whole, in place.

- **Kind**: View
- **Function**: `views::details::show_more(state: &MessageState, ui: &mut Ui) -> Vec<Intent>`
- **Entry points**: the link under the text of the message in the details.
- **Offered when**: the text under the title is longer than four lines at the width of the details.

Acceptance:

- [ ] Under the title the text shows four lines at most, and **Show more** is there only when more is hidden.
- [ ] **Show more** shows the whole text in place, without a scroll bar of its own, and becomes **Show less**.
- [ ] Making the details wider or narrower shows or hides **Show more** as the text fits or not.
- [ ] Selecting another commit shows its message cut again.

### `details.file-tree` — Files as a tree of directories

The files of a commit, of a comparison or of the changes, laid out under their directories, which fold.

- **Kind**: Derivation
- **Function**: `model::details::file_tree(input: &[FilePath]) -> Vec<TreeNode>`

Rules:

- **Input**: the paths of the files, with `/` between names. **Output**: a tree of directories and files; each file of
  the input is in it exactly once, and each directory once.
- In each directory the directories come first, then the files, each group by name, without regard to case; names that
  differ only in case are ordered by their exact characters.
- A directory that holds nothing but one other directory is one row named with both, as `src/git`, and so on down the
  chain. The top of the tree is never joined so.
- A directory folds and unfolds; the folds are kept while the same commit is shown, and forgotten for another one.

Examples:

- `src/git/ops.rs`, `src/git/repo.rs`, `README.md` give `src/git` with `ops.rs` and `repo.rs` under it, then
  `README.md`.
- `a/b.txt`, `a/c/d.txt` give `a` with `c` (holding `d.txt`) before `b.txt`.

Acceptance:

- [ ] Directories are listed before files at each level.
- [ ] A chain of directories with a single directory in each is one row.
- [ ] Every input path appears once, and the count of files is unchanged.
- [ ] Folding a directory hides its files until it is unfolded, and the fold is forgotten when another commit is shown.

### `details.show-all-files` — The first 300 files, and Show all

A commit can change thousands of files, and a row for each makes the view slow: a list of more than 300 files shows the
first 300 and a link for the rest.

- **Kind**: View
- **Function**: `views::details::show_all_files(state: &CappedList, ui: &mut Ui) -> Vec<Intent>`
- **Entry points**: the end of the files in the details, in the page of a commit, in the comparison page and in the
  in-place comparison, and of the Changes pane.

Acceptance:

- [ ] A list of 301 files shows 300 and **Show all 301 files…**.
- [ ] A list of 300 files shows them all, with no link.
- [ ] **Show all** shows every file of that list; another commit shown in its place starts again from 300.
- [ ] The count and the buttons of the header act on the whole list, not only on the files shown.
- [ ] In the tree layout the tree is made of the files shown.

### `details.compare-file-with` — Compare a file with a branch, tag or commit

Compares one file, as a commit, a comparison or the working tree has it, with the branch, tag, commit or working tree
the user chooses, in the Diff page.

- **Kind**: Operation
- **Function**: `features::details::compare_file_with(context, answers) -> Outcome`
- **Entry points**: **Compare with…** in the context menu of a file in the details, in the page of a commit (in the
  list and on the header of its diff), in the comparison page and in the in-place comparison.
- **Asks**: the other side, titled **Compare `<file name>` of `<side>` with**, with the choices of `compare.side-choices`,
  the side the file is taken from left out and **Working tree** offered unless that side is the working tree.
- **Plan**: no step; it changes nothing.
- **Danger**: None
- **Undo**: nothing to undo: it changes nothing.
- **After**: opens the Diff page of the file between the two, titled `<file name> (<left> ↔ <right>)`. The working tree
  is always on the right, where its file can be edited; otherwise the side chosen is on the left and the file's own on
  the right.
- **Uses**: `compare.side-choices`, `compare.resolve-revision`, the Diff page (`diff.page`).

Which version of the file is compared:

| The file is listed in | Its side |
| --- | --- |
| A commit | The commit, named by its short hash, or by the name the list gave it |
| A commit that deleted it | The first parent of that commit |
| A comparison | The second side of the comparison |
| A comparison where the second side deleted it | The side the comparison starts from |
| The uncommitted changes | The working tree |
| The uncommitted changes, deleted | `HEAD`, named by the branch checked out |

Acceptance:

- [ ] On a file of a commit, choosing `main` opens the Diff page with the file of `main` on the left and the file of the
      commit on the right.
- [ ] Choosing **Working tree** puts the file on disk on the right.
- [ ] A file the commit deleted is compared as it was in the parent.
- [ ] The side the file is taken from is not among the choices.
- [ ] Escape in the picker opens nothing.

### `details.stash` — Details of a stash

A stash selected shows what it holds, as a commit does, including the untracked files it took, which git keeps apart
from the rest.

- **Kind**: View
- **Function**: `views::details::stash(state: &DetailsState, ui: &mut Ui) -> Vec<Intent>`
- **Entry points**: selecting a stash in the graph or in the Stashes section of the references; opening its page.
- **Reads**: `details.read-commit`.

Acceptance:

- [ ] A stash shows its message, author, date and hash like a commit, and the files it changed against the commit it was
      made on.
- [ ] A stash made with untracked files shows them under **N untracked files**, apart from the others.
- [ ] A click on an untracked file opens the Diff page of the file against nothing: all its lines added.
- [ ] A stash made without untracked files shows no such section.
- [ ] The page of a stash (`commit-page.view`) shows the untracked files the same way.

### `commits.read-log` — Read the commits of what is checked out

The commits of `HEAD`, newest first, and which of them are not pushed yet, for the Commits pane.

- **Kind**: Read
- **Function**: `git::read::commits::read_log(git: &Git, request: &LogRequest) -> Result<BranchLog, GitError>`
- **Reads**:
  - `git log -z --format=<hash, parents, author name, author email, author date, subject> --max-count=<limit + 1> HEAD
    --`: one more than asked, to know whether there are more;
  - `git rev-parse --symbolic-full-name @{upstream}`, which has no answer without an upstream or when its branch is gone;
  - `git remote`;
  - unless there is no commit, or neither an upstream nor a remote:
    `git rev-list --max-count=<limit> HEAD --not <upstream> --`, or `--not --remotes` without an upstream.
- The limit asked for is at least 1 and at most 10,000.

Acceptance:

- [ ] Before the first commit it returns no commits and no error.
- [ ] With an upstream, the unpushed commits are those of `HEAD` not in the upstream.
- [ ] Without an upstream, the unpushed commits are those in no remote branch.
- [ ] In a repository with no remote, no commit is marked either way.
- [ ] Asking for 100 of a history of 150 returns 100 and says there are more.

### `commits.push-state` — What is pushed and what is to pull

What the line above the commits of the Commits pane says, and the mark on each commit.

- **Kind**: Derivation
- **Function**: `model::commits::push_state(input: &BranchLogAndBranch) -> PushState`

Rules:

- **Input**: the result of `commits.read-log`, the branch checked out, its upstream and how far behind it is.
  **Output**: the line above the list, and one mark per commit.
- No remote at all: the line says **No remote to push to**, and no commit has a mark.
- With an upstream: **N commits to push to `<upstream>`**, or **Everything is pushed to `<upstream>`**, followed by
  **, M commits to pull** when the branch is behind it.
- Without an upstream: **N commits on no remote branch: `<branch>` has no upstream**, or **Everything is on a remote
  branch: `<branch>` has no upstream**; when nothing is checked out, the reason is **no branch is checked out**.
- `1 commit` is singular. When every commit listed is unpushed and there are more, the count has a `+`: **100+
  commits to push**.
- A commit not pushed has a cloud with an arrow, in color, titled **Not pushed to `<upstream>`** or **On no remote
  branch**. A commit pushed has a plain cloud, titled **Pushed: it is in `<upstream>`**, or **in a remote branch**.
- With no commit, the pane says **No commits yet.** instead of the line.

Acceptance:

- [ ] Two commits ahead of `origin/main` and three behind give **2 commits to push to origin/main, 3 commits to pull**.
- [ ] A branch level with its upstream gives **Everything is pushed to origin/main**.
- [ ] A branch with no upstream, in a repository with a remote, marks its commits that no remote branch has.
- [ ] A repository with no remote says **No remote to push to** and marks nothing.

### `commits.view` — The Commits pane

The commits of what is checked out, newest first, and nothing else: apart from the graph, so that what the graph is
narrowed to, searched for or scrolled to changes nothing here.

- **Kind**: View
- **Function**: `views::commits::view(state: &CommitsState, ui: &mut Ui) -> Vec<Intent>`
- **Entry points**: the **Commits** pane of the side bar; command palette: **Show Commits in the Side Bar**, which
  unfolds and shows it.
- **Reads**: `commits.read-log`, first with a limit of 100; `details.read-commit` for a commit unfolded.
- **Uses**: `commits.push-state`, `commit-page.open`, `commit-page.show-in-graph`, `branch.checkout`, `branch.create`,
  `tag.create`, `revert.commit`, `reset.to-commit`, `commits.copy-hash`, `commits.copy-message`, `sync.fetch`, `sync.pull`,
  `sync.push`, `undo.last`, the Diff page (`diff.page`).

What it shows:

- the header names the branch checked out, and the repository when the workspace has several; it has fetch, pull and
  push, and under **…** the other ways to pull and push, and undo;
- the line of `commits.push-state`, then the commits, each with its mark and subject; its tooltip has the subject, the
  short hash, the author and the full date;
- a click on a commit unfolds the files it added, changed, deleted or renamed (**No files** when there are none,
  **Loading…** meanwhile), and a click on a file opens its diff against the first parent in the Diff page;
- on each commit, a button that opens its page (**Open in a tab, with its changes**) and one that shows it in
  the graph (**Show in the graph**);
- its context menu: **Open in a tab**, **Show in the graph**, **Check out commit (detached)** (not on the
  commit checked out), **Create branch here…**, **Create tag here…**, **Revert `<short hash>`**, **Reset `<branch>` to
  here…** (not on the commit checked out), **Copy hash**, **Copy message**;
- **Load more commits…** at the end, while there are more, reads 100 more.

The commits are read again when what is checked out, its upstream, or where the remote branches are changes, and from
the first 100 again when another repository is shown.

Acceptance:

- [ ] The pane lists the first 100 commits of `HEAD`, and **Load more commits…** lists 200.
- [ ] Narrowing the graph to another branch, or searching in it, does not change the pane.
- [ ] Checking out another branch lists its commits, and the header names it.
- [ ] A commit pushed after the list was read loses its arrow once the remote branch moves.
- [ ] Unfolding a commit lists its files, and a click on one opens its Diff page.
- [ ] The context menu of the commit checked out has no **Check out commit (detached)** and no **Reset … to here…**.
- [ ] Unfolded commits that are still listed stay unfolded after the list is read again.

### `commits.copy-hash` — Copy the hash of a commit

Puts the full hash of a commit on the clipboard.

- **Kind**: Operation
- **Function**: `features::commits::copy_hash(context, answers) -> Outcome`
- **Entry points**: **Copy hash** in the context menu of a commit in the Commits pane; a click on the hash in the
  details and in the page of a commit.
- **Plan**: no steps.
- **Danger**: None
- **Undo**: Nothing to take back: nothing changes.
- **After**: the hash is on the clipboard, and a short message says so.

Acceptance:

- [ ] The clipboard holds the 40 characters of the hash (64 in a repository with SHA-256 objects).

### `commits.copy-message` — Copy the message of a commit

Puts the whole message of a commit on the clipboard, as it was written.

- **Kind**: Operation
- **Function**: `features::commits::copy_message(context, answers) -> Outcome`
- **Entry points**: **Copy message** in the context menu of a commit in the Commits pane.
- **Reads**: `details.read-commit`, for the message in full.
- **Plan**: no steps.
- **Danger**: None
- **Undo**: Nothing to take back: nothing changes.
- **After**: the message is on the clipboard.

Acceptance:

- [ ] The clipboard holds the subject, the empty line and the body exactly as `git log -1 --format=%B` prints them,
      without the trailing newline.
- [ ] The lines are as written, not as the details re-wrap them (`details.message-unwrap`).

### `commit-page.open` — Open a commit in a tab

Opens a commit whole, in a page of its own, or shows the page it has.

- **Kind**: Operation
- **Function**: `features::commit_page::open(context, answers) -> Outcome`
- **Entry points**: double-click on a commit in the Graph tab, in its narrow layout or in the Commits pane, or `Enter`
  on the commit selected; the button on its row in the Commits pane; the button beside the one that closes the details;
  **Open in a tab** in the context menu of a commit; a parent in the page of a commit, a side, a commit or the
  merge base in the comparison page; command palette: **Open Commit in a Tab…**.
- **Offered when**: on a commit or a stash; not on the row of the uncommitted changes. A double click on the label of a
  branch checks the branch out instead (`branch.checkout`).
- **Asks**: from the command palette only, **Open commit**: a hash, or anything else git turns into a commit, in an
  input box with the prompt **A commit hash, or anything else git understands: HEAD~3, v1.0^, main@{yesterday}**; what
  git does not know is refused while typing with **Git does not know a commit by this name in this repository.**
- **Reads**: `compare.resolve-revision` for what is typed; `details.read-commit`.
- **Plan**: no step; it changes nothing.
- **Danger**: None
- **Undo**: nothing to undo: it changes nothing.
- **After**: opens the page (`commit-page.view`) in a new tab titled with the short hash and the subject, cut after 32
  characters with `…`; or shows the tab that commit of that repository has already.
- **Refuses**: a hash that is not a commit of the repository: **`<short hash>` is not a commit of this repository.**

Acceptance:

- [ ] Double-clicking a commit opens a tab titled `3f2a1c9 Fix the parser`.
- [ ] Opening the same commit again shows the tab it has; there is never a second tab for it.
- [ ] The same hash in two repositories of the workspace opens two tabs.
- [ ] Typing `HEAD~3` in the palette command opens the commit three before `HEAD`.
- [ ] Typing a name git does not know is refused in the box, and nothing opens.
- [ ] A subject longer than 32 characters is cut with `…` in the title of the tab.

### `commit-page.read-patch` — Read the lines a commit changed

What a commit changed in each of its files, as lines, for its page; the same lines serve the comparison page.

- **Kind**: Read
- **Function**: `git::read::commit_page::read_patch(git: &Git, request: &PatchRequest) -> Result<Vec<FilePatch>, GitError>`
- **Reads**: `git diff --no-color --no-ext-diff --no-textconv --submodule=short -M <first parent> <hash>`; for a commit
  with no parent, `git diff-tree --root --no-commit-id -r -p --no-color --no-ext-diff --no-textconv --submodule=short
  <hash>`.
- **Uses**: `commit-page.diff-lines` to split the output into files, hunks and counts.

Acceptance:

- [ ] The files come back in the order and number `details.read-commit` lists them.
- [ ] A file whose type changed (a file that became a link) is one file, made of the two parts git prints for it.
- [ ] When the files of the output do not match the files listed, the result says the lines cannot be shown, rather
      than showing them against the wrong files.
- [ ] No external diff driver or text conversion of the user's configuration is used.

### `commit-page.diff-lines` — The lines of each diff, and how many are sent

Turns the output of a diff into files with their hunks, the lines each gained and lost, and the cap of 20,000 lines a
page is given.

- **Kind**: Derivation
- **Function**: `model::commit_page::diff_lines(input: &DiffOutput) -> Vec<FilePatch>`

Rules:

- **Input**: the files listed, the output of the diff, and a limit of lines, 20,000. **Output**: for each file, its
  hunks, the count of lines added (`+`) and removed (`-`), and whether it is binary.
- The size of a file is the lines of its hunks plus one for each hunk header. Files are taken in order; a file whose
  size still fits under the limit, added to the files taken before it, gets its hunks, and a file that does not fit
  gets none, with its counts still given. A smaller file after it may still fit.
- Each line of a hunk is numbered as it is before and after, from the hunk header `@@ -a,b +c,d @@`: a removed line has
  only a number before, an added line only a number after, a line of context both. The note `\ No newline at end of
  file` has no number.

Examples:

- Three files of 15,000, 8,000 and 3,000 lines: the first and the third have their lines, the second says it is too
  long to show with the others.

Acceptance:

- [ ] A file of 3 hunks with 10 lines each counts 33 lines toward the limit.
- [ ] The counts of a file left without lines are still its added and removed lines.
- [ ] The line numbers before and after follow the hunk headers.
- [ ] A binary file has no hunks and says it is binary.

### `commit-page.view` — The page of a commit

A commit whole, in a tab: its full message, labels, author, committer, dates, hash and parents, the files it changed with
the lines each gained and lost, and, on demand, what changed in each, in the colors of the code.

- **Kind**: View
- **Function**: `views::commit_page::view(state: &CommitPageState, ui: &mut Ui) -> Vec<Intent>`
- **Entry points**: `commit-page.open`.
- **Reads**: `details.read-commit`, `commit-page.read-patch`.
- **Uses**: `details.message-title`, `details.message-unwrap`, `details.file-tree`, `details.show-all-files`,
  `details.compare-file-with`, `details.stash`, `commit-page.diff-lines`, `commit-page.initial-folds`,
  `commit-page.show-in-graph`, the Diff page (`diff.page`), the colors of code of the Diff page.

What it shows, from the top:

- the title of the message, the labels of the commit (its branches, remote branches and tags, as the graph draws them,
  while the other views show its repository), and the rest of the message;
- **Author** with name and `<email>`, **Date**, **Committer** with name, `<email>` and date only when it differs from the
  author, **Commit** with the full hash, which a click copies (**Copy the hash**), **Parent** or **Parents**, each of
  which opens its own page;
- **Show in the graph** and, when the commit changed files, **Open in the Diff page**, which opens all of its changes
  one after the other in one Diff page;
- the files, **N changed files** with the lines gained and lost by all of them (`+120 −45`), **against the first
  parent** on a merge, as a tree of directories or as a list: the two buttons beside the count choose, and the choice is
  remembered for the pages opened later, the tree to start with. Each file has its status, icon, path (`old → new` for a
  rename), its lines gained and lost or **binary**, and an eye that shows its changes in the page and goes to them
  (**Show its changes in this page**). A click opens it in the Diff page. Its context menu, in the list and on the header
  of its diff, has **Compare with…**;
- for a stash, its untracked files apart (`details.stash`);
- **Changes**, with **Show the changes**, which puts the diff of every file listed in the page, under the list, and
  **Hide the changes**, which takes them away; the changes are not in the page when it opens. While they are, **Fold
  all** and **Unfold all** fold or unfold every diff;
- each diff under a header that stays at the top while its lines scroll: the arrow that folds it, the status, icon and
  path (a click on the path folds it too), the lines gained and lost, and a button at the end that opens it in the Diff
  page. Its lines have the number each has before and after and their code in the colors of the theme. A binary file
  says **Binary file.**, a file past the cap **Too long to show with the others.**, each with **Open it in the Diff
  page**; a file with no change to its text says **No change to its text.**, or **Renamed, with no change to its
  text.**;
- once the page is scrolled down more than 400 pixels, a round button in its corner takes it back to the top (**Back to
  the top**).

`Ctrl+F` (`Cmd+F` on macOS) finds a word in the page with Gitardo's find in page.

A page stays of the repository its commit is in, also after the other views are pointed at another; it is not read again
when the repository changes, as a commit does not change.

Acceptance:

- [ ] The page shows the message in full, unwrapped as `details.message-unwrap` says, with no **Show more**.
- [ ] A click on the hash copies the full hash; a click on a parent opens the page of the parent.
- [ ] The labels are those of the commit in the graph, the branch checked out filled in.
- [ ] **Show the changes** puts a diff under the list for every file shown, and **Hide the changes** removes them.
- [ ] The eye on a file shows the changes and scrolls to that file's diff, unfolded.
- [ ] A file whose diff is folded at first unfolds with its arrow, and **Unfold all** unfolds every diff.
- [ ] A merge shows its changes against its first parent and says so after the count.
- [ ] A file past the 20,000-line cap says **Too long to show with the others.** with a link to the Diff page.
- [ ] When the lines cannot be matched to the files, the page says **Git lists these changes in a way that cannot be
      shown here.** with a link that opens them all in the Diff page.
- [ ] A commit that is no longer in the repository shows **This commit is not in the repository any more.**
- [ ] `Ctrl+F` finds text in the diffs that are unfolded.
- [ ] The layout chosen in one page is the one the next page opens with.

### `commit-page.initial-folds` — Which diffs start folded

A page of many or long diffs would take long to show and be hard to read: the long ones start folded.

- **Kind**: Derivation
- **Function**: `model::commit_page::initial_folds(input: &[FilePatch]) -> FoldSet`

Rules:

- **Input**: the files with their lines, in the order of the page. **Output**: which diffs are unfolded when the
  changes are shown.
- The size of a diff is its lines plus one for each hunk header.
- In order, a diff is unfolded when it is at most 800 lines and the diffs unfolded before it, with it, make at most 5,000
  lines; otherwise it is folded, and its lines do not count.
- A diff the user folded or unfolded stays as the user left it, whatever these rules say.
- A file with no lines (binary, or past the cap of `commit-page.diff-lines`) is unfolded, showing its note.

Examples:

- Diffs of 900, 100 and 200 lines: the first is folded, the other two unfolded.
- Twelve diffs of 450 lines each: the first eleven make 4,950 lines and are unfolded; the twelfth is folded.

Acceptance:

- [ ] A diff of 801 lines starts folded; one of 800 starts unfolded when it fits under 5,000.
- [ ] Once 5,000 lines are unfolded, a diff that would go past them starts folded, and a smaller one after it may still
      be unfolded.
- [ ] Folding a short diff by hand keeps it folded while the page is open.

### `commit-page.show-in-graph` — Show a commit in the graph

Brings the Graph tab to a commit, from a page or a pane that is apart from the graph.

- **Kind**: View
- **Function**: `views::commit_page::show_in_graph(state: &CommitPageState, ui: &mut Ui) -> Vec<Intent>`
- **Entry points**: **Show in the graph** in the page of a commit; the button and the context menu entry of a commit in
  the Commits pane.
- **Uses**: `repo.select` when the page is of another repository than the one the views show; `graph.show-in-graph`,
  which loads the commit when it is further back than the history loaded and shows all branches when a filter leaves it
  out.

Acceptance:

- [ ] **Show in the graph** in the page of a commit selects that commit in the Graph tab and brings the tab to the front.
- [ ] From the page of a commit of another repository, the views are pointed back at that repository first.
- [ ] A commit further back than the history loaded is loaded and selected.

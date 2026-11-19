# Details, commits and the page of a commit

How a commit is read. The inspector, beside the timeline, summarizes the commit selected: its message, who made it,
and the files it changed. Copying the hash or the message of a commit (`commits.`) is here too. The page of a commit
shows it whole with what it changed in each file. The commits of what is checked out are the timeline narrowed to the
branch checked out (`filter.show-only`), with what is not pushed ringed (`graph.unpushed`).
The derivations that shape a message and a list of files for the inspector and the page are here too.

### `details.read-commit` — Read the details of a commit

Everything the inspector, the Stashes list and the page of a commit show of one commit: its message,
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

### `details.read-signature` — Read the signature of a commit

Whether a commit is signed, and what git says of its signature: good, bad, made by a key it does not trust or cannot
check, and by whom ([D79](../08-decisions.md#d79--the-signature-of-a-commit-shown-and-the-options-of-the-commit-box-proposed)).

- **Kind**: Read
- **Function**: `git::read::details::read_signature(git: &Git, request: &SignatureRequest) -> Result<Signature, GitError>`
- **Reads**:
  - `git show --no-patch --format=%G?%x1f%GS%x1f%GK%x1f%GF%x1f%GT <hash>`: what git says of the signature, with the
    signer, the key, its fingerprint and how far it is trusted. Git checks the signature with the program its
    configuration names (`gpg`, `gpgsm` or `ssh-keygen`), which is why it is read apart from `details.read-commit`, for
    the commit shown alone and after its details (Q1090).
  - When git says `N`, no signature: `git cat-file commit <hash>`, for whether the commit has a `gpgsig` or
    `gpgsig-sha256` header all the same. Git says `N` of a commit signed with SSH while `gpg.ssh.allowedSignersFile`
    is not set; such a commit is signed, and git cannot check it here.
- **Uses**: by `details.signature`.
- **Extension**: Planned: `read/details.read-signature` of the engine, for the details of the graph and the tab of a
  commit.

What it gives, from git's letter: **Good** (`G`); **Good, from a key whose validity is not known** (`U`); **Good,
expired** (`X`); **Good, made by an expired key** (`Y`); **Good, made by a revoked key** (`R`); **Bad** (`B`);
**Cannot be checked** (`E`), as when the key is missing; **Signed, not checked** for a `gpgsig` header with `N`; and
**Not signed**. The kind of signature, OpenPGP, X.509 or SSH, is the first line of the header (`-----BEGIN PGP
SIGNATURE-----`, `-----BEGIN SIGNED MESSAGE-----`, `-----BEGIN SSH SIGNATURE-----`).

Acceptance:

- [ ] A commit made without `-S` is **Not signed**, and `git cat-file` ran once.
- [ ] A commit signed with SSH, with `gpg.ssh.allowedSignersFile` naming a file that lists its key, is **Good**, with
      the signer's email and the key's fingerprint.
- [ ] The same commit, with `gpg.ssh.allowedSignersFile` not set, is **Signed, not checked**, of the kind SSH.
- [ ] A signed commit whose message was changed in its object afterwards is **Bad**.
- [ ] A commit signed with OpenPGP by a key not in the keyring is **Cannot be checked**, with the key's identifier.

### `details.show` — Details of the selected commit

The summary of the commit selected in the timeline, in the inspector beside the commits or over them when the window is
narrow: its message, who made it and when, its hash and parents, and the files it changed, each one click away from its
diff.

- **Kind**: View
- **Function**: `views::details::show(state: &DetailsState, ui: &mut Ui) -> Vec<Intent>`
- **Entry points**: selecting a commit, a stash or the draft in the timeline; the same in the narrow layout of the
  window, where the inspector opens over the commits.
- **Reads**: `details.read-commit`.
- **Uses**: `details.message-title`, `details.message-unwrap`, `details.show-more`, `details.file-tree`,
  `details.show-all-files`, `details.compare-file-with`, `details.stash`, `commit-page.open`, `message.edit`,
  `note.show`, `file-history.file`, the Diff page (`diff.page`), `compare.in-place` (which takes the place of the commit in the
  inspector).

What it shows, from the top:

- a button that closes the inspector, one beside it that opens the commit as a page (**Open as a page, with its
  changes**; not on the draft), and the pencil of `message.edit` where that is offered;
- the title of the message in bold and the rest under it, as `details.message-title` and `details.message-unwrap` make
  them, cut by `details.show-more`;
- the note of the commit, when it has one, in the block **Note** (`note.show`);
- **Author** (the email in the tooltip), **Date**, **Committer** with its date only when the committer or the date
  differs from the author's, **Commit** with the short hash, which a click copies whole (**Copy full hash**), and
  **Parent** or **Parents**, each short hash going to that commit in the timeline;
- the count, **N changed files**, followed by **against the first parent** on a merge;
- the two buttons that list the files one after the other, each with its directory beside its name (**One after the
  other, each with its directory**), or as a tree of directories (**Under their directories**); the choice is
  remembered and starts as the list;
- with more than one file, a button that opens the diffs of all of them one after the other in one Diff page (**Open
  all changes in one Diff page**), titled with the short hash and subject;
- the files, each with its status, its icon from Symbols (`file-view.file-icons`), its name and, on its row, a button that shows
  its history (**Show the history of this file**), there while the pointer is on the row or on the button. A click
  opens its diff in the Diff page, against the first parent.
  Its context menu has **Compare with…**.

On the draft the inspector has no author, hash or parents. Each file has a box to tick, the
buttons of `stash.push-file` and of discarding its changes, and the buttons above the files act on the ticked files, or
on all of them when none is ticked (see `stash.push-file` and `changes.discard`).

Acceptance:

- [ ] Selecting a commit shows its title, author, date, short hash and parents, and the count of its files.
- [ ] The committer is shown only when its name or date differs from the author's.
- [ ] A click on the hash puts the full hash on the clipboard.
- [ ] A click on a parent selects that commit in the timeline.
- [ ] A merge says **against the first parent** after the count.
- [ ] A click on a file opens the Diff page of that file between the first parent and the commit.
- [ ] The button next to the count opens all the files in one Diff page; it is not there with a single file.
- [ ] The layout chosen, list or tree, is the one the inspector has after the window is opened again.
- [ ] The button beside the close button opens the page of the commit (`commit-page.open`).
- [ ] While loading, the inspector says **Loading…**.
- [ ] With the pointer moved from the row of a file onto the button of its history and resting there ten frames, each
      frame draws the button and the row lit, none differing from the one before; a click then asks for the history
      of that file.

### `details.signature` — The signature of a commit

Says, in the inspector and in the tab of a commit, whether the commit is signed and whether the signature holds
([D79](../08-decisions.md#d79--the-signature-of-a-commit-shown-and-the-options-of-the-commit-box-proposed)).

- **Kind**: View
- **Function**: `views::details::signature_view(state: &SignatureState, ui) -> Vec<Intent>`
- **Entry points**: selecting a commit in the timeline; opening a commit as a page (`commit-page.view`).
- **Reads**: `details.read-signature`, once the details of the commit are shown, so that they never wait for it.
- **Uses**: `details.read-signature`, `details.show`, `commit-page.view`.
- **Extension**: Planned: the same line in the details beside the graph and in the tab of a commit, read with
  `read/details.read-signature`.

- Under **Commit**, a line **Signature**, with a shield icon in the color of what git said, and its words: **Good**,
  in the color of an addition, with the signer, `Good, Ada <ada@example.com>`; the three kinds of good signature that
  git doubts, in the color of a warning, with what it doubts, `Good, expired key`; **Bad**, in the color of a removal;
  **Signed, not checked** and **Cannot be checked**, muted, with why when git said it. Its tooltip has the kind of
  signature, the key, its fingerprint and the trust git gave it.
- **Not signed** is said by no line: most commits are not signed, and a line on each would say nothing (Q1091).
- While the signature is read, the line says **Checking the signature…**; the stashes and the draft have none.
- Nothing is drawn in the timeline for the signature (Q1092).

Acceptance:

- [ ] A commit signed with a good signature shows **Signature** `Good, Ada <ada@example.com>`, in the color of an
      addition, and its tooltip has the fingerprint of the key.
- [ ] A commit not signed has no line **Signature**.
- [ ] A bad signature shows **Bad**, in the color of a removal.
- [ ] Selecting a commit shows its message and files before its signature is read.
- [ ] Going through ten commits with `↓` reads the signature of the one selected last, the reads before it cancelled.
- [ ] The tab of a commit signed shows the same line.

### `details.message-title` — The title of a message

A commit message whose first paragraph is long, often a message written as one paragraph with no title, would fill the
inspector with bold text: its title becomes its first sentence, or its first words, and the rest goes on under it.

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
that were cut to fit are joined again, so that the text wraps at the width of the inspector or of the page.

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

A long message in the inspector shows its first lines only, so that the files stay in sight under it; **Show more**
shows it whole, in place.

- **Kind**: View
- **Function**: `views::details::show_more(state: &MessageState, ui: &mut Ui) -> Vec<Intent>`
- **Entry points**: the link under the text of the message in the inspector.
- **Offered when**: the text under the title is longer than four lines at the width of the inspector.

Acceptance:

- [ ] Under the title the text shows four lines at most, and **Show more** is there only when more is hidden.
- [ ] **Show more** shows the whole text in place, without a scroll bar of its own, and becomes **Show less**.
- [ ] Making the inspector wider or narrower shows or hides **Show more** as the text fits or not.
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

A commit can change thousands of files, and a row for each makes the list slow: a list of more than 300 files shows the
first 300 and a link for the rest.

- **Kind**: View
- **Function**: `views::details::show_all_files(state: &CappedList, ui: &mut Ui) -> Vec<Intent>`
- **Entry points**: the end of the files in the inspector, in the page of a commit, in the comparison page and in the
  in-place comparison, and of the Changes page.

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
- **Entry points**: **Compare with…** in the context menu of a file in the inspector, in the page of a commit (in the
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
- **Entry points**: selecting a stash in the timeline or in the Stashes list of the branch rail; opening its page.
- **Reads**: `details.read-commit`.

Acceptance:

- [ ] A stash shows its message, author, date and hash like a commit, and the files it changed against the commit it was
      made on.
- [ ] A stash made with untracked files shows them under **N untracked files**, apart from the others.
- [ ] A click on an untracked file opens the Diff page of the file against nothing: all its lines added.
- [ ] A stash made without untracked files shows no such section.
- [ ] The page of a stash (`commit-page.view`) shows the untracked files the same way.

### `commits.copy-hash` — Copy the hash of a commit

Puts the full hash of a commit on the clipboard.

- **Kind**: Operation
- **Function**: `features::commits::copy_hash(context, answers) -> Outcome`
- **Entry points**: **Copy hash** in the context menu of a commit in the timeline, narrowed or not; a click on the hash
  in the inspector and in the page of a commit.
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
- **Entry points**: **Copy message** in the context menu of a commit in the timeline, narrowed or not.
- **Reads**: `details.read-commit`, for the message in full.
- **Plan**: no steps.
- **Danger**: None
- **Undo**: Nothing to take back: nothing changes.
- **After**: the message is on the clipboard.

Acceptance:

- [ ] The clipboard holds the subject, the empty line and the body exactly as `git log -1 --format=%B` prints them,
      without the trailing newline.
- [ ] The lines are as written, not as the inspector re-wraps them (`details.message-unwrap`).

### `commit-page.open` — Open a commit as a page

Opens a commit whole, in a page of its own, or shows the page it has.

- **Kind**: Operation
- **Function**: `features::commit_page::open(context, answers) -> Outcome`
- **Entry points**: double-click on a commit in the timeline, narrowed or not, or in its narrow layout, or `Enter` on
  the commit selected; the button beside the one that closes the inspector; **Open as a page** in the context menu of a commit; a parent in the
  page of a commit, a side, a commit or the merge base in the comparison page; Quick search: **Open Commit as a
  Page…**.
- **Offered when**: on a commit or a stash; not on the draft. A double click on the label of a
  branch checks the branch out instead (`branch.checkout`).
- **Asks**: from Quick search only, **Open commit**: a hash, or anything else git turns into a commit, in an
  input box with the prompt **A commit hash, or anything else git understands: HEAD~3, v1.0^, main@{yesterday}**; what
  git does not know is refused while typing with **Git does not know a commit by this name in this repository.**
- **Reads**: `compare.resolve-revision` for what is typed; `details.read-commit`.
- **Plan**: no step; it changes nothing.
- **Danger**: None
- **Undo**: nothing to undo: it changes nothing.
- **After**: opens the page (`commit-page.view`), titled in its trail with the short hash and the subject, cut after 32
  characters with `…`; or shows the page that commit of that repository has already.
- **Refuses**: a hash that is not a commit of the repository: **`<short hash>` is not a commit of this repository.**

Acceptance:

- [ ] Double-clicking a commit opens a page titled `3f2a1c9 Fix the parser`.
- [ ] Opening the same commit again shows the page it has; there is never a second page for it.
- [ ] The same hash in two repositories of the workspace opens two pages, one in the tab of each.
- [ ] Typing `HEAD~3` in the command of Quick search opens the commit three before `HEAD`.
- [ ] Typing a name git does not know is refused in the box, and nothing opens.
- [ ] A subject longer than 32 characters is cut with `…` in the title of the page.

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

A commit whole, on a page: its full message, labels, author, committer, dates, hash and parents, the files it changed
with the lines each gained and lost, and, on demand, what changed in each, in the colors of the code.

- **Kind**: View
- **Function**: `views::commit_page::view(state: &CommitPageState, ui: &mut Ui) -> Vec<Intent>`
- **Entry points**: `commit-page.open`.
- **Reads**: `details.read-commit`, `commit-page.read-patch`.
- **Uses**: `details.message-title`, `details.message-unwrap`, `details.file-tree`, `details.show-all-files`,
  `details.compare-file-with`, `details.stash`, `commit-page.diff-lines`, `commit-page.initial-folds`,
  `commit-page.show-in-graph`, `note.show`, the Diff page (`diff.page`), the colors of code of the Diff page.

What it shows, from the top:

- the title of the message, the labels of the commit (its branches, remote branches and tags, as the timeline draws
  them, while the window shows its repository), and the rest of the message;
- the note of the commit, when it has one, whole, in the block **Note** (`note.show`);
- **Author** with name and `<email>`, **Date**, **Committer** with name, `<email>` and date only when it differs from the
  author, **Commit** with the full hash, which a click copies (**Copy the hash**), **Parent** or **Parents**, each of
  which opens its own page;
- **Show in the timeline** and, when the commit changed files, **Open in the Diff page**, which opens all of its changes
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

A page stays of the repository its commit is in, also after the window is pointed at another; it is not read again
when the repository changes, as a commit does not change.

Acceptance:

- [ ] The page shows the message in full, unwrapped as `details.message-unwrap` says, with no **Show more**.
- [ ] A click on the hash copies the full hash; a click on a parent opens the page of the parent.
- [ ] The labels are those of the commit in the timeline, the branch checked out filled in.
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

### `commit-page.images` — The images of a commit in its page

The two versions of an image that a commit changed, drawn in its page and in a comparison in place of **Binary
file.** ([D74](../08-decisions.md#d74--images-are-drawn-where-their-files-are-shown)).

- **Kind**: View
- **Function**: `views::commit_page::images_view(state, ui) -> Vec<Intent>`
- **Entry points**: **Show the changes** in the page of a commit (`commit-page.view`) and in a comparison
  (`compare.`), for a file that is an image on either side.
- **Reads**: `diff.read-sides`, each side read as an image by `file-view.read-image`, when the diff of the file is
  shown.
- **Uses**: `commit-page.view`, `file-view.read-image`, `diff.image`.
- **Extension**: the page of a commit and the tab of a comparison, Gitardo's webview, with the two versions as images
  of `data:` URIs from the bytes of `read/diff.read-sides`.

Rules:

- The diff of a file that is an image on either side shows, under its header, the version before and the version after
  side by side, each at most 240 pixels high and half the width of the page, never larger than it is, on the ground of
  squares of `file-view.image`, **Empty** where a side has none; under each its size in pixels and its weight.
- A click on either picture opens the file in the Diff page (`diff.image`).
- The two are read when the diff is shown, unfolded, not when the page opens; a diff folded reads nothing. Until they
  are read, the place they take is kept, so that the page does not jump.
- An SVG shows its lines, as any text file, with **Show the images** on its header, which shows its pictures in their
  place, and **Show the lines** back ([Q846](../08-decisions.md#open-questions)).
- An image that cannot be read says why in its place; a file that is not an image still says **Binary file.**
- In the header of the file, **binary** stays where the count of lines would be.
- An image counts as one line for the folds of `commit-page.initial-folds`, and is unfolded at first as a binary file
  is.
- Extension: the webview draws the two as `<img>` of `data:` URIs, which its policy of content allows for images
  alone; in an `<img>` nothing of an SVG runs and nothing is fetched. A side over 10 MB is not sent: *Too large to show
  here.* with **Open it in the Diff page**.

Acceptance:

- [ ] The page of a commit that changed `media/logo.png` shows, with the changes shown, its two versions side by side
      in place of **Binary file.**
- [ ] A commit that added a PNG shows **Empty** on the left and the picture on the right.
- [ ] A folded diff of an image reads nothing until it is unfolded.
- [ ] A click on the picture opens the Diff page of the file at that commit.
- [ ] An SVG shows its lines; **Show the images** shows its two pictures.
- [ ] A comparison of two branches that differ in a PNG shows its two versions.
- [ ] Extension: the page of a commit that changed a PNG shows its two versions in the webview; one of 12 MB says *Too
      large to show here.*

### `commit-page.show-in-graph` — Show a commit in the timeline

Brings the timeline to a commit, from the page of that commit.

- **Kind**: View
- **Function**: `views::commit_page::show_in_graph(state: &CommitPageState, ui: &mut Ui) -> Vec<Intent>`
- **Entry points**: **Show in the timeline** in the page of a commit.
- **Uses**: `repo.switch-tab`, and `repo.select` in the extension, when the page is of another repository than the one
  shown; `graph.show-in-graph`, which loads the commit when it is further back than the history loaded and shows all
  branches when a filter leaves it out.

Acceptance:

- [ ] **Show in the timeline** in the page of a commit selects that commit in the timeline, shown in place of the page.
- [ ] From the page of a commit shown again in the tab of its repository, the commit is selected in the timeline of
      that repository.
- [ ] Extension: from the page of a commit of another repository, the repository of the page is chosen again first.
- [ ] A commit further back than the history loaded is loaded and selected.

## Notes

A note is text attached to a commit after it was made, without changing it: a review, a link to a build, a word for
later. Git keeps the notes in a reference of their own, `refs/notes/commits` unless `core.notesRef` names another, which
is not fetched nor pushed with the branches. Here they are shown in the inspector and in the page of a commit, written,
removed, and fetched and pushed on demand
([D85](../08-decisions.md#d85--notes-in-the-details-of-a-commit-archives-of-a-commit-and-the-upkeep-of-a-repository-from-its-row-proposed)).

### `note.read` — Read the notes of commits

Which commits have a note, and the text of the note of one commit.

- **Kind**: Read
- **Function**: `git::read::note::read(git: &Git, request: &NoteRequest) -> Result<Notes, GitError>`
- **Reads**: `git notes get-ref`, the reference of notes git uses (Q1210), and
  `git rev-parse --verify --quiet <reference>^{commit}`, the commit it is at; then, when it moved since the read
  before, `git notes --ref=<reference> list`, a line `<id of the note> <id of the commit>` for each commit with a note.
  The text of the note of the commit shown in the inspector or in a page is read with its details
  (`details.read-commit`), `git notes show <hash>`, as it was written, rather than `git cat-file blob` of the id the
  list gives, since the details are read without the list in hand, in the window and for the extension alike (Q1457).
  With no reference of notes, there are none.
- **Uses**: `repo.read-state`, with which the list is read, again only when the reference of notes moved
  (`repo.watch-changes`); by `note.show`.
- **Extension**: read by the engine with the details of a commit: planned.

Acceptance:

- [x] With a note on `3f2a1c9`, the list has `3f2a1c9`, and the text read is the note as written, lines and all.
- [x] A repository with no `refs/notes/commits` reads no note and no error.
- [x] With `core.notesRef` set to `refs/notes/review`, the notes of that reference are read.
- [x] A read after a change to a file of the working tree only does not run `git notes list` again.

### `note.show` — The note of a commit, in the inspector and in its page

The note of a commit under its message, wherever the commit is read.

- **Kind**: View
- **Function**: `views::details::note_view(state: &NoteState, ui: &mut Ui) -> Vec<Intent>`
- **Entry points**: drawn by the inspector (`details.show`) and by the page of a commit (`commit-page.view`) for a
  commit that has a note.
- **Reads**: `note.read`.
- **Uses**: `details.show`, `commit-page.view`, `note.edit`, `note.remove`, `ui.menu-icons`.
- **Extension**: planned: the same block under the message in the details beside the graph (`Detail.tsx`) and in the
  tab of a commit (`CommitApp.tsx`).

- Under the message, a block titled **Note**, with its text as it was written, lines kept as they are and not joined as
  those of a message are (Q1212), in the font of the message; in the inspector, a note of more than 12 lines is cut
  with **Show more**, as a message is (`details.show-more`).
- On the title of the block, the pencil of **Edit note…** (`note.edit`) and **…** with **Remove note** (`note.remove`).
- A commit with no note shows nothing; **Add note…** is in its context menu in the timeline. The rows of the timeline
  have no mark for a commit with a note (Q1211).

Acceptance:

- [x] Selecting a commit with the note `Reviewed.` shows **Note** and `Reviewed.` under its message in the inspector,
      and in its page.
- [x] A commit with no note shows no **Note**.
- [x] A note of three lines keeps its three lines.
- [x] The block matches its snapshots in each theme.

### `note.edit` — Add or edit the note of a commit

Writes the note of a commit, a new one or in place of the one it had.

- **Kind**: Operation
- **Function**: `features::note::edit(context, answers) -> Outcome`
- **Entry points**: **Add note…**, or **Edit note…** for a commit that has one, in the context menu of a commit in the
  timeline; the pencil of the block **Note** (`note.show`).
- **Offered when**: on a commit; not on the draft nor on a stash.
- **Asks**: **Note of `<short hash>`**, a text of several lines, its note as default when it has one.
- **Reads**: `note.read`, for the note it has.
- **Plan**: `git notes add -f -F - <hash>`, with the text on its standard input. Git takes away the empty lines at the
  start and at the end, as it does for a message.
- **Danger**: None.
- **Undo**: Undo puts the reference of notes back where it was, from its reflog, which git keeps for `refs/notes/` as
  for branches: `git update-ref <reference> <before> <after>`, or `git update-ref -d <reference> <after>` when it was
  the first note (Q1215).
- **After**: the block **Note** shows the new text.
- **Refuses**: an empty text: **A note cannot be empty: remove it instead.**; the same text as the note has: nothing
  runs.
- **Uses**: `note.read`, `note.show`, `undo.last`.
- **Extension**: planned: **Add Note…** and **Edit Note…** in the context menu of a commit in the graph, the text asked
  in an input box of VS Code, sending `feature/run` of `note.edit` to the engine.

Acceptance:

- [ ] **Add note…** on `3f2a1c9` with `Reviewed.` runs `git notes add -f -F - <hash>` with `Reviewed.` on its input,
      and `git notes show <hash>` prints `Reviewed.` after.
- [ ] **Edit note…** proposes the note it has, and the text changed replaces it.
- [ ] **Undo** right after it gives the commit its note of before, or none.
- [ ] An empty text is refused, and nothing runs.

### `note.remove` — Remove the note of a commit

Takes the note of a commit away.

- **Kind**: Operation
- **Function**: `features::note::remove(context, answers) -> Outcome`
- **Entry points**: **Remove note** in the **…** of the block **Note** and in the context menu of a commit that has a
  note in the timeline.
- **Offered when**: the commit has a note.
- **Plan**: `git notes remove <hash>`.
- **Danger**: None.
- **Undo**: as `note.edit`, from the reflog of the reference of notes.
- **After**: the block **Note** is gone.
- **Uses**: `note.read`, `note.show`, `undo.last`.
- **Extension**: planned: **Remove Note** in the context menu of a commit with a note in the graph and on the block of
  the note, sending `feature/run` of `note.remove` to the engine.

Acceptance:

- [ ] On a commit with a note it runs `git notes remove <hash>`, and the inspector shows no **Note** after.
- [ ] **Undo** right after it gives the note back.

### `note.fetch` — Fetch the notes of a remote

Brings the notes of a remote into the reference of notes of the repository, which `git fetch` does not do by itself.

- **Kind**: Operation
- **Function**: `features::note::fetch(context, answers) -> Outcome`
- **Entry points**: **Fetch notes** in the context menu of a remote in the branch rail; Quick search, **Fetch
  Notes…**, which asks the remote when there are several.
- **Offered when**: the repository has a remote.
- **Plan**: `git fetch <remote> <reference>:<reference>`, the reference of notes git uses (`refs/notes/commits`
  unless `core.notesRef` names another), which moves it only forward. When git refuses because each side has notes the
  other has not, Gitardo offers **Merge the notes**:
  `git fetch <remote> +<reference>:refs/notes/remotes/<remote>/<name>`, `<name>` being the reference without
  `refs/notes/`, then
  `git notes merge --strategy=cat_sort_uniq refs/notes/remotes/<remote>/<name>`, which puts the two notes of a commit
  one after the other, without a line twice (Q1214).
- **Danger**: None.
- **Undo**: as `note.edit`, from the reflog of the reference of notes.
- **After**: says how many commits have a note now; the inspector shows the notes fetched.
- **Refuses**: a remote with no notes: **`<remote>` has no notes.**
- **Uses**: `note.read`, `remote.read-remotes`, `undo.last`.
- **Extension**: planned: **Fetch Notes** in the context menu of a remote among the references and in the Branches
  view, sending `feature/run` of `note.fetch` to the engine.

Acceptance:

- [ ] From a remote whose notes are ahead, it runs `git fetch origin refs/notes/commits:refs/notes/commits`, and the
      notes fetched show in the inspector.
- [ ] With a note added on each side, git's refusal is followed by **Merge the notes**, after which a commit noted on
      both sides shows both notes.
- [ ] From a remote with no notes, it says so, and nothing changes.

### `note.push` — Push the notes to a remote

Sends the reference of notes to a remote, which `git push` does not do by itself.

- **Kind**: Operation
- **Function**: `features::note::push(context, answers) -> Outcome`
- **Entry points**: **Push notes** in the context menu of a remote in the branch rail; Quick search, **Push Notes…**.
- **Offered when**: the repository has a reference of notes and a remote.
- **Plan**: `git push <remote> <reference>`, the reference of notes git uses.
- **Danger**: None.
- **Undo**: Not taken back by Undo: the notes are on the remote.
- **Refuses**: when the remote refuses because it has notes this repository has not: **`<remote>` has notes you do not
  have: fetch them first.** with **Fetch notes** (`note.fetch`).
- **Uses**: `note.fetch`, `remote.read-remotes`.
- **Extension**: planned: **Push Notes** in the context menu of a remote, sending `feature/run` of `note.push` to the
  engine.

Acceptance:

- [ ] It runs `git push origin refs/notes/commits`, and a clone that fetches the notes then has them.
- [ ] When the remote has a note this repository has not, the refusal offers **Fetch notes**.

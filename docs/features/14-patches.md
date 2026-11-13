# Patches

Commits taken out of one repository and made again in another, or on another branch, as text: copied or saved as the
patch `git format-patch` makes, and applied from a file or from the clipboard, as commits with `git am` or as changes to
the working tree with `git apply`. A patch is checked before anything is touched. The patch of a comparison, which
`git apply` takes, is `compare.copy-patch`.

### `patch.read-format-patch` — Read commits as one patch

One or more commits as one patch, oldest first, each with its author, date and message, as `git am` takes it.

- **Kind**: Read
- **Function**: `git::read::patch::read_format_patch(git: &Git, request: &FormatPatchRequest) -> Result<String, GitError>`
- **Reads**: `git format-patch --stdout --no-color --no-ext-diff --no-textconv --submodule=short --max-count=<N>
  --no-walk=unsorted <hashes, newest first> --`. Git writes commits given this way in the reverse of the order it is
  given them, so the newest is given first for the patch to start with the oldest; the count keeps a single commit from
  being read as where to start from up to `HEAD`.
- **Uses**: `patch.count-commits`, to check that git made a patch of every commit.

Acceptance:

- [ ] Three commits selected in any order give one patch whose commits are in the order of the history, oldest first.
- [ ] One commit gives a patch of that commit only, not of the commits after it.
- [ ] Each commit of the patch starts with `From <full hash>` and has its author, date and message.
- [ ] A merge commit gives no patch of its own: the patch has fewer commits than were asked for.
- [ ] No external diff driver or text conversion of the user's configuration is used.

### `patch.count-commits` — How many commits a patch has

Tells a patch made by `git format-patch`, which can be applied as commits, from any other patch, and counts its commits.

- **Kind**: Derivation
- **Function**: `text::patch::count_commits(input: &str) -> usize`

Rules:

- **Input**: the text of a patch. **Output**: the number of its commits, 0 when it was not made by `git format-patch`.
- A patch made by `git format-patch` starts, after any blank space, with a line `From <hash> `, the hash of 40 to 64
  lower-case hexadecimal characters. Its commits are its lines that start so.
- Any other text has no commits, whatever lines it has further down.

Examples:

- A patch of `git format-patch -3 --stdout` counts 3.
- A patch of `git diff` counts 0.
- A text that starts with `From someone` counts 0.

Acceptance:

- [ ] A patch of five commits made by `git format-patch --stdout` counts 5.
- [ ] A patch with SHA-256 hashes of 64 characters is counted the same.
- [ ] The output of `git diff` counts 0, also when a line of it starts with `From `.

### `patch.copy` — Copy as patch

Copies one commit, or several selected, as one patch, for `git am` to make them again in another repository or on
another branch.

- **Kind**: Operation
- **Function**: `features::patch::copy(context, answers) -> Outcome`
- **Entry points**: **Copy as patch** in the context menu of a commit in the timeline; in the bar above the commits of
  the timeline and in the context menu of several selected commits (`selection.bar`).
- **Offered when**: every commit chosen is a commit with at most one parent: not a merge, not a stash, not the draft.
- **Reads**: `patch.read-format-patch`, with the commits in the order of the history, whatever the order they were
  clicked in.
- **Plan**: no step; it changes nothing in the repository.
- **Danger**: None
- **Undo**: nothing to undo: it changes nothing.
- **After**: puts the patch on the clipboard and says **Copied `<short hash>` as a patch.**, or **Copied N commits as a
  patch.**
- **Refuses**: when git made fewer commits than were chosen: **Git makes no patch of a merge commit.**, followed by
  **: leave the merges out of the selection** for several.
- **Uses**: `patch.read-format-patch`, `patch.count-commits`.

Acceptance:

- [ ] **Copy as patch** on a commit puts on the clipboard the text of `git format-patch --stdout` for that commit.
- [ ] With three commits selected newest first, the patch has the oldest first.
- [ ] The entry is not in the context menu of a merge, nor offered for a selection that holds one.
- [ ] `git am` of the patch copied, in another clone at the parent of the oldest commit, makes commits with the same
      authors, dates, messages and changes.

### `patch.save` — Save as patch…

Saves the same patch as **Copy as patch** to a file the user chooses.

- **Kind**: Operation
- **Function**: `features::patch::save(context, answers) -> Outcome`
- **Entry points**: **Save as patch…** in the context menu of a commit in the timeline and in the context menu of
  several selected commits.
- **Offered when**: as `patch.copy`.
- **Asks**: where to save it, in the system's save dialog titled **Save `<short hash>` as a patch** or **Save N commits
  as a patch**, filtered to `.patch`, starting in the top folder of the repository with the name `<short hash>.patch`,
  or `<short hash of the oldest>-<short hash of the newest>.patch` for several.
- **Reads**: `patch.read-format-patch`.
- **Plan**: `write <path chosen>`, with the patch.
- **Danger**: None
- **Undo**: impossible: the file saved is outside what Undo takes back; delete it to take it back.
- **Refuses**: as `patch.copy`.
- **Uses**: `patch.read-format-patch`, `patch.count-commits`.

Acceptance:

- [ ] **Save as patch…** on `3f2a1c9` proposes `<repository>/3f2a1c9.patch`, and saving writes the patch there.
- [ ] Two commits selected propose `<oldest>-<newest>.patch`.
- [ ] The log has a `write <path>` line for the file.
- [ ] Cancelling the dialog writes nothing.

### `patch.check` — Check a patch before applying it

What applying a patch with `--3way` would do, without doing it: the files it would leave in conflict, or what keeps it
from applying at all.

- **Kind**: Read
- **Function**: `git::read::patch::check(git: &Git, request: &PatchCheckRequest) -> Result<PatchCheck, GitError>`
- **Reads**: `git apply --check --3way <file>`, or `git apply --check --3way` with the patch on standard input when it
  is not in a file. A patch of several commits is checked as a whole, each change on top of the ones before it. The
  exit code is part of the answer:
  - 0: it applies; each line `Applied patch to '<file>' with conflicts.` names a file it will leave in conflict;
  - otherwise it does not apply, and the reasons are the lines of its output that start with `error:` or `fatal:`,
    without that word, each once; when there is none, git's whole error output, or **git apply exited with code N**.

Acceptance:

- [ ] A patch that applies cleanly returns no conflicts and no errors.
- [ ] A patch whose context has changed but that `--3way` can merge with conflicts names those files.
- [ ] A patch of a file that is not there returns git's reason, and nothing in the working tree or the index changes.
- [ ] The check runs as a read: it is logged at the debug level and never asks.

### `patch.apply` — Apply a patch

Applies a patch from a file or from the clipboard: as commits, each made again on top of the current branch with its
author and message, or to the working tree only, its changes staged and nothing committed.

- **Kind**: Operation
- **Function**: `features::patch::apply(context, answers) -> Outcome`
- **Entry points**: Quick search: **Apply Patch…**; **Apply patch…** in the **…** menu of the header.
- **Asks**:
  1. **Apply patch**: **From a file…** or **From the clipboard**.
  2. From a file: the system's open dialog titled **Apply patch**, starting in the top folder of the repository, with
     the filters **Patches** (`.patch`, `.diff`, `.mbox`, `.eml`) and **All files**, its button **Choose**.
  3. Only for a patch made by `git format-patch` (`patch.count-commits` above 0), **Apply `<name>`**, where the name is
     the file's name or **the patch in the clipboard**:
     - **As commits**, described `git am --3way`: **Makes its commit again on top of `<branch>`, with the authors and
       messages**, or **its N commits**; the default;
     - **To the working tree only**, described `git apply --3way`: **Applies the changes to the files and stages them,
       without committing**.
     Any other patch is applied to the working tree only, without this question.
- **Reads**: the file chosen, or the clipboard; `patch.check`.
- **Plan**:
  - from a file: `git am --3way <file>` or `git apply --3way <file>`; the patch is given to git as it is;
  - from the clipboard: `write <temporary file>` with the text of the clipboard, in the temporary folder of the system
    with a name that cannot collide (`gitardo-<unique>.patch`), then `git am --3way <temporary file>` or
    `git apply --3way <temporary file>`, then `delete <temporary file>`, which runs whether git succeeded or not.
  - Titled **Apply `<name>` as N commits**, with the note **The N commits will be made on top of `<branch>`, with the
    authors and messages the patch has.**, or **Apply `<name>` to the working tree**, with the note **The changes will
    be applied to the files and staged: nothing is committed.**
  - The note goes on with what `patch.check` found: **No conflicts expected.**, or **This will stop with conflicts in N
    files:** and the first 15 files, then **…and K more**.
- **Danger**: None when it applies cleanly; Confirm, because it will stop with files in conflict, when `patch.check`
  names some.
- **Undo**: as commits, Undo right after takes back all of its commits at once with `git reset --keep <commit before
  the patch>`, also when it stopped on a conflict and was continued in Gitardo (`conflicts.continue-operation`); continued from a
  terminal, or after anything else, Undo goes back one commit at a time by the reflog. To the working tree only,
  nothing moves a reference: Undo does not take it back.
- **After**: the repository is read again. When `git am` stops on a conflict, the banner of the operation shows
  (`patch.am-stopped`).
- **Refuses**:
  - a text with no line starting with `diff --git ` or `--- ` followed by a name: **`<file name>` has no patch in
    it.**, or **There is no patch in the clipboard.**;
  - as commits while a merge, rebase, cherry-pick, revert or am is under way: **Finish or abort the `<operation>`
    first.**;
  - as commits with something staged, which `git am` would refuse half way and leave to be aborted: **git am applies
    patches only while nothing is staged: commit or unstage the staged changes first.**;
  - a patch that does not apply, from `patch.check`: a warning **`<Name>` does not apply, so nothing was changed.**,
    with the first 15 reasons of git and **…and K more**. Nothing is changed.
- **Settings**: `commandPreview`.
- **Uses**: `patch.count-commits`, `patch.check`, `patch.am-stopped`, `undo.last`, `preview.confirm`.

Acceptance:

- [ ] A patch of two commits from a file, applied **As commits**, runs `git am --3way <file>` and makes the two commits
      on the current branch with their authors and messages.
- [ ] The same patch applied **To the working tree only** runs `git apply --3way <file>`, stages the changes and makes no
      commit.
- [ ] A patch of `git diff` is applied to the working tree without the question.
- [ ] From the clipboard, the log has `write <temporary file>`, the git command on that file, and `delete <temporary
      file>`; the file is gone afterwards, also when git failed.
- [ ] A patch that does not apply shows git's reasons and changes nothing: the log has no command that changes
      anything.
- [ ] A patch that will conflict names the files and asks first, also with `commandPreview` set to `off`.
- [ ] **As commits** with staged changes is refused before git runs.
- [ ] Undo right after a patch of three commits takes back the three at once.

### `patch.am-stopped` — A patch that stopped on a conflict

When `git am` stops because a commit of the patch does not fit, it waits to be continued, skipped or aborted, as a
cherry-pick does.

- **Kind**: View
- **Function**: `views::patch::am_stopped(state: &OperationState, ui: &mut Ui) -> Vec<Intent>`
- **Entry points**: the banner of the operation in the timeline and the Changes page, shown while `git am` is under
  way: the repository is in an am when `.git/rebase-apply/applying` exists, which is checked before the markers of a
  rebase.
- **Uses**: `conflicts.banner`, `conflicts.continue-operation` (`git -c core.editor=true am --continue`), `conflicts.skip-step`
  (`git am --skip`), `conflicts.abort-operation` (`git am --abort`), the Merge page (`merge-view.page`).

The two sides of a file in conflict are named **The branch checked out** (current) and **The patch being applied**
(incoming), in the Merge page and wherever the sides of a conflict are named.

Acceptance:

- [ ] A patch whose second commit conflicts stops with the first commit made, and the banner shows **Continue**,
      **Skip** and **Abort**.
- [ ] **Continue** after the file is resolved makes that commit and the ones after it.
- [ ] **Skip** leaves that commit out and goes on with the next.
- [ ] **Abort** goes back to where the branch was before the patch, after asking.
- [ ] The Merge page of a file in conflict names its sides **The branch checked out** and **The patch being applied**.

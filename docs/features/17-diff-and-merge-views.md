# Diff, Merge and File pages

The three pages where the text of a file is shown: the **Diff** page, two versions of a file side by side, which is the
only diff of the application; the **Merge** page, a file in conflict in three columns, which is the only place a
conflict is resolved by hand; and the **File** page, a file in Gitardo's own text editor, where it is edited, saved,
searched, and where the blame of a file is shown. The derivations they share are here too: the line diff, the words
changed within a line, the alignment of two sides, the three-way alignment of a file in conflict, and the colors of the
code.

### `diff.page` — The Diff page

Two versions of a file side by side, kept level, with every change joined across the gutter to where it lands.

- **Kind**: View
- **Function**: `views::diff::page_view(state, ui) -> Vec<Intent>`
- **Entry points**: a click on a file in the Changes page when `changes.openOnClick` is `diff`, and **Open changes** on
  its row; a file in the inspector, beside it (`diff.beside-inspector`); a file in the Commit page and in a
  comparison; **Compare with the saved file** in the Changes page; **Compare with…** on a file; the button on the
  header of the Changes page that opens all its diffs in one page; **Show the result** in the review of the
  conflicts; `conflicts.open-file` for a file deleted on one side.
- **Reads**: `diff.read-sides`.
- **Refuses**: a file with a NUL byte on either side: *This file looks binary.* A diff that takes more than 5 seconds to
  compute: *This file has too many differences to display.*
- **Asks**: when the two sides have more than 100,000 lines together: **This file is very large and the diff may be
  slow. Continue?** with **Continue**; anything else opens nothing.
- **Settings**: `diff.collapseUnchanged`.
- **Uses**: `diff.read-sides`, `diff.line-diff`, `diff.word-diff`, `diff.align`, `diff.scroll-together`,
  `diff.resize-sides`, `diff.navigate`, `diff.collapse-unchanged`, `diff.revert-change`, `diff.stage-change`,
  `diff.unstage-change`, `diff.staged-changes`, `diff.edit`, `diff.open-at-line`, `diff.open-file`, `diff.stage-file`,
  `diff.unstage-file`, `diff.discard-file`, `file-view.highlight`.

What it shows, left and right:

| Left | Right | Right side editable |
| --- | --- | --- |
| `HEAD` | the file in the working tree | yes |
| the index | the file in the working tree | yes |
| a commit | the file in the working tree | yes |
| `HEAD` | the index | no |
| the parent of a commit | the commit | no |
| a commit | another commit | no |

- Each side is named over its column: **Working tree**, **Index**, the name of a branch or `HEAD`, a commit by its first
  7 hexadecimal digits, or **Empty** where the file is not. A file not in the left version (untracked, added) shows
  every line added against an empty side; a file not in the right version (deleted) shows every line removed.
- A renamed file is read under its old name on the left and its new name on the right; the header says
  `old/path → new/path`.
- The page is titled **\<file name\> (\<left\> ↔ \<right\>)** in its trail. Asking for the same diff again shows the
  page it has.
- The diffs of several files open in one page, one after the other, each under a header with its path and its own
  title buttons; `F7` goes on from the last change of one to the first of the next.
- The two sides are drawn in two columns of Gitardo's text editor, with the colors of the code, line numbers of their
  own version, blank filler lines that keep the two sides level (`diff.align`), and, in the gutter between them, a band
  for each change joining it to where it lands: green where lines were added, red where they were removed, blue where
  they changed. Within a changed line, the words that changed are marked (`diff.word-diff`). The scroll bar of the right
  side has a mark for each change, in its color.
- The page is computed again when either version changes: the file on disk or in a page that edits it, the index, or
  what `HEAD` is. When nothing it is made from changed, nothing is redrawn.

Acceptance:

- [ ] Opening the changes of an edited file shows `HEAD` on the left and the working tree on the right, titled
      **app.rs (HEAD ↔ Working tree)**.
- [ ] An untracked file of 10 lines shows 10 added lines and an empty left side.
- [ ] A file with a NUL byte refuses with *This file looks binary.* and opens no page.
- [ ] Two files of 60,000 lines each ask before computing; **Continue** shows the diff, and cancelling opens nothing.
- [ ] Opening the same diff twice shows one page.
- [ ] Staging a file from the Changes page while its diff against the index is open redraws the page with no change.

### `diff.beside-inspector` — Files of a commit read beside the inspector

A file of the commit in the inspector opens its diff where the timeline and the side column were, and the inspector
stays on the right as the column of the commit's files, so that every file of the commit can be read one after the
other without it closing.

- **Kind**: View
- **Function**: `views::diff::beside_inspector_view(state: &BesideInspectorState, ui) -> Vec<Intent>`
- **Entry points**: a click on a file in the inspector, of a commit, a stash, the draft or a comparison in its place
  (`compare.in-place`); `Alt+Down` and `Alt+Up` in the Diff page opened so, and `Down` and `Up` in the tree of the
  inspector beside it, for the next and the previous file of the inspector.
- **Uses**: `diff.page`, `details.show`, `details.file-tree`, `compare.in-place`, `graph.resize-panes`,
  `repo.side-column`.
- **Extension**: VS Code's diff editor opens beside the panel, and the details stay: nothing to build.

- The Diff page takes the place of the timeline and of the side column, which is put away while the diff is shown and
  comes back as it was with the timeline
  ([D56](../08-decisions.md#d56--a-side-column-of-repositories-and-branches-what-was-said-in-the-status-bar-diffs-beside-what-opened-them));
  the inspector keeps its width (`graph.resize-panes`).
- The inspector becomes the column of the commit's files, as the tree of the Files page is beside its file
  (`file-view.beside-files`): at its top the commit in one line, its short hash and its subject, with its author and
  its date in its tooltip, and a click on it giving back the whole card; under it, the files of the commit as a tree
  (`details.file-tree`), each with its lines gained and lost, the folders opened or closed as the user leaves them. It
  marks the file whose diff is shown, as a row selected, and keeps it in view. A file clicked gives the tree the
  keyboard, and `Up` and `Down` in the tree move from file to file and show each, in the order of the tree whatever
  the layout of the inspector beside the timeline (Q507).
- A click on another file of the inspector shows its diff in the same page, which is not opened again: the trail
  names the file shown, and `Ctrl+Tab` lists one page for the files of that commit, not one for each file.
- **Open all changes in one Diff page** in the inspector opens the diffs one after the other in the same place.
- The trail's first step, **Timeline**, or `Escape` in the page, brings the timeline back, the same commit selected
  (Q419). Closing the inspector leaves the Diff page, which then takes its width too.
- In the narrow layout, where the inspector is over the timeline, a file opens the Diff page in place of both, as
  before.
- A file opened from anywhere else (the Commit page, the Changes page, a comparison page, **Compare with…**) opens its
  Diff page in place of the timeline and the inspector, as `diff.page` says.

Acceptance:

- [ ] A click on `src/a.rs` in the inspector opens its diff, and the inspector is still shown on the right with
      `src/a.rs` marked.
- [ ] While the diff is shown, the side column is not; **Timeline** in the trail brings it back at the width it had.
- [ ] The inspector beside the diff shows the commit in one line and its files as a tree, the folder of `src/a.rs`
      open.
- [ ] A click on `src/b.rs` then shows its diff in the same page, and `Ctrl+Tab` lists one Diff page.
- [ ] `Alt+Down` in that page shows the diff of the next file of the inspector.
- [ ] After a click on `src/a.rs`, `Down` in the tree shows `src/b.rs`, then `src/c.rs`; `Up` goes back, and stays on
      `src/a.rs`, the first.
- [ ] With the inspector as a list beside the timeline, `Down` from `src/a.rs` shows `README.md`, which the tree lists
      after the folder `src`.
- [ ] **Timeline** in the trail shows the timeline with the same commit selected.
- [ ] In a window narrower than the narrow layout, a click on a file of the inspector opens the Diff page in place of
      the timeline and the inspector.

### `diff.read-sides` — The two versions of a file

Reads each version of a file a Diff page compares.

- **Kind**: Read
- **Function**: `git::read::diff::read_sides(git, &request) -> Result<DiffSides, GitError>`
- **Reads**: the index with `git show :<path>`; a commit with `git show <revision>:<path>`; the working tree from the
  text the File page holds for the file when it is open there with unsaved edits, else from the disk. A version that is
  not there (an untracked file in the index or `HEAD`, a deleted file on disk, a commit with no parent) is none.

Acceptance:

- [ ] With unsaved edits to `app.rs` in a File page, the right side of its diff against `HEAD` shows those edits.
- [ ] A file renamed in a commit is read with its old path in the parent and its new path in the commit.
- [ ] A path with a space and one starting with `:` are read correctly.

### `diff.line-diff` — The lines that changed

The changes between two texts, line by line, used by every diff of the application and by the rules of the conflicts.

- **Kind**: Derivation
- **Function**: `text::diff::line_diff(input) -> Result<Vec<Change>, TooComplex>`

What the result must satisfy:

- It is **minimal**: no other way from the first text to the second removes and adds fewer lines. Implement it with the
  O(ND) algorithm of Myers (1986), searching from the start of the texts; the rules of the conflicts also run it on both
  texts reversed (`conflicts.resolve-by-rules`).
- Lines are compared as they are, spaces included, once `\r\n` is read as `\n`: a file checked out with other line
  endings has no changes.
- A run of removed lines directly followed by a run of added lines is **one** change, of kind *changed*; removed lines
  alone are *removed*, added lines alone *added*. Two changes are never next to each other with no line between.
- Each change says where it is on each side as a hunk header does: its first line from 1 and how many lines; with 0
  lines, the line before the place meant (0 at the start of the file).
- A diff still running after its time limit (5 seconds for a page) stops and says it is too complex, rather than
  hanging.

Acceptance:

- [ ] `a b c` against `a B c` gives one changed change at line 2 of each side, 1 line each.
- [ ] `a b` against `a x b` gives one added change with 0 lines on the left at line 1 and 1 line on the right at line 2.
- [ ] A text and the same text with CRLF give no change.
- [ ] `a␣` against `a` is a change.
- [ ] On random pairs of texts of up to 50 lines from an alphabet of 3 lines, the number of lines removed and added is
      never more than the edit distance computed by dynamic programming.

### `diff.word-diff` — The words that changed in a line

Within a changed line paired with another, the parts that changed, so that they can be marked.

- **Kind**: Derivation
- **Function**: `text::diff::word_diff(input) -> (Vec<Range>, Vec<Range>)`

What the result must satisfy:

- A line is cut into tokens: each run of letters, digits and `_` (letters of any script), each run of spaces and tabs,
  and each other character on its own. The two lists of tokens are compared by a minimal diff (Myers 1986); the ranges
  of characters of the tokens removed on the left and added on the right are the marks.
- Marks that touch or overlap are one mark.
- When more than 65% of the characters of **both** lines changed, nothing is marked: the whole line is the change. A
  line that is empty on either side has no marks either way.
- It runs only on lines paired by `diff.align`: an added or removed line has no word marks.

Acceptance:

- [ ] `let total = a + b;` against `let total = a + c;` marks `b` on the left and `c` on the right, nothing else.
- [ ] `foo(bar)` against `foo(baz)` marks `bar` and `baz` whole, not single letters.
- [ ] `call();` against `    call();` marks the 4 spaces on the right only.
- [ ] Two lines with nothing in common have no marks.

### `diff.align` — The two sides kept level

Puts the lines of the two sides in rows, so that unchanged lines face each other and each change takes the same height
on both sides.

- **Kind**: Derivation
- **Function**: `model::diff::align(input) -> AlignedDiff`
- **Uses**: `diff.line-diff`, `diff.word-diff`.

- Unchanged lines are one row each, with the same text on both sides.
- In a changed change of `L` lines on the left and `R` on the right, the first `min(L, R)` lines of each side face each
  other in rows, with their word marks; the `|L − R|` lines left of the longer side face **filler** rows on the other
  side, which are blank, have no line number and cannot be edited.
- An added change faces filler rows on the left; a removed change faces filler rows on the right.
- Both sides always have the same number of rows; each row of a change knows which change it is of.

Acceptance:

- [ ] 2 lines changed into 5 give 5 rows: 2 paired with word marks, then 3 added lines facing 3 filler rows.
- [ ] Removing 4 lines gives 4 rows with filler on the right, and the next unchanged line faces itself.
- [ ] Every line of each side appears exactly once, in order, and the filler rows of one side are the only ones without
      a line number.

### `diff.scroll-together` — The two sides scroll together

How far to scroll one side when the other is scrolled, so that what is in the middle of one is in the middle of the
other.

- **Kind**: Derivation
- **Function**: `model::diff::scroll_together(input) -> f32`

- Anchors are the top and the bottom of each change on both sides, and the ends of the two texts, in pixels, made never
  to go backwards. Between two anchors the position on one side maps to the other in proportion: one for one through
  unchanged lines, stretched through a change that is taller on one side. Past the last anchor, one for one.
- The position mapped is the middle of the view, not its top: through a long change, the shorter side stays in the
  middle of the screen, with what is around it visible above and below.
- A scroll that this sets is not answered by a scroll of the other side, so that the two never chase each other.
- Navigating to a change (`diff.navigate`) centers it on both sides.
- Anchors are measured again when lines change height: an unchanged region folded or unfolded, the window resized.

Acceptance:

- [ ] Through unchanged lines, scrolling one side 10 lines scrolls the other exactly 10 lines.
- [ ] Through a change of 1 line facing 200 lines, scrolling the long side keeps the single line at the middle of the
      other side until the middle of the view leaves the change.
- [ ] Scrolling the right side to its end brings the left side to its end.

### `diff.resize-sides` — Making one side wider

The gutter between two columns is also the line between them: dragging it makes one wider.

- **Kind**: View
- **Function**: `views::diff::resize_sides_view(state, ui) -> Vec<Intent>`
- **Entry points**: dragging the gutter of a Diff page, the gutters of a Merge page, or the bands of the review of the
  conflicts in **Columns**; double-clicking any of them.

- The gutter gives to one column what it takes from the next; the other columns keep their widths. No column is dragged
  narrower than 80 pixels.
- Dragging does not start on a button of the gutter: the buttons are clicked.
- A double click makes all the columns of the page the same width again.
- The widths are kept with the page while it is open, and a Diff page opens with the widths the last one was left with.

Acceptance:

- [ ] Dragging the gutter 100 pixels to the right widens the left side by 100 pixels and narrows the right by 100.
- [ ] Dragging past the edge stops with 80 pixels left to the narrower column.
- [ ] A double click after dragging gives two equal columns.
- [ ] Pressing on a **+** button in the gutter stages its change and moves nothing.

### `diff.navigate` — The previous and the next change

Goes from one change to the next, says where it is among them, and goes on to the next changed file past the last one.

- **Kind**: View
- **Function**: `views::diff::navigate_view(state, ui) -> Vec<Intent>`
- **Entry points**: `F7` and `Shift+F7` in either side; the arrows ▲ (**Previous change (Shift+F7)**) and ▼ (**Next
  change (F7)**) of the bar at the top of the page; the title buttons **Previous change** and **Next change**.
- **Uses**: `diff.page` to open the next file in place.

- Beside the arrows is **3 / 7** (the current change and how many there are), **– / 7** before any is chosen, or **No
  changes**. The current change is outlined on both sides and in the gutter, and centered on both sides.
- From no change, `F7` goes to the first and `Shift+F7` to the last.
- When the diff is between `HEAD`, the index and the working tree, `F7` on the last change shows *Press F7 again to go
  to the next file* for 4 seconds; pressed again, the page shows in its place the same diff of the next changed file,
  and `Shift+F7` on the first change does the same backwards with *Press Shift+F7 again to go to the previous file*. The
  changed files are those that differ between the same two versions (the staged ones for the index against `HEAD`, the
  unstaged ones for the working tree against the index, both for the working tree against `HEAD`), in the order of their
  paths without regard to case and with numbers in numeric order, going round from the last to the first. Files that
  cannot be shown (binary, too complex) are passed over. With no other: *No other changed files.*
- Against a commit, or between two commits, `F7` on the last change goes round to the first.
- A change folded away by `diff.collapse-unchanged` is unfolded when navigation lands in it.

Acceptance:

- [ ] In a diff with 3 changes, `F7` three times shows **1 / 3**, **2 / 3**, **3 / 3**.
- [ ] With `a.rs`, `b10.rs` and `b9.rs` changed, `F7` twice on the last change of `a.rs` opens `b9.rs`, then `b10.rs`.
- [ ] On the last changed file, going on opens the first one.
- [ ] Against a commit, `F7` on the last change goes to **1 / 3** without a hint.
- [ ] Any other key between the two presses of `F7` does not cancel the hint; a press of `Shift+F7` does.

### `diff.collapse-unchanged` — Folding unchanged lines

Folds long runs of unchanged lines into a bar, so that the changes are seen together.

- **Kind**: View
- **Function**: `views::diff::collapse_unchanged_view(state, ui) -> Vec<Intent>`
- **Entry points**: the fold button **Collapse unchanged regions** of the bar at the top of the page, which switches it
  for that page; the bar **⋯ 12 unchanged lines** (*Click to expand*), which unfolds its lines.
- **Settings**: `diff.collapseUnchanged` (on unless set to `false`) decides whether new pages start folded.

- Each run of unchanged rows keeps 3 lines visible next to each change; at the start and at the end of the file, none
  is kept on the side away from the changes.
- A run is folded only when at least 4 lines would be hidden.
- A file with no changes folds nothing.
- The two sides fold the same rows, so they stay level.
- Finding, going to a line, or navigating into folded lines unfolds them.

Acceptance:

- [ ] A file of 100 lines with one change at line 50 shows lines 47 to 53 and two bars, of 46 and of 47 lines.
- [ ] Two changes with 9 unchanged lines between them show all 9: 3 + 3 kept leaves 3, fewer than 4.
- [ ] With `diff.collapseUnchanged` set to `false`, a new page shows every line, and the fold button folds them.
- [ ] Clicking a bar shows its lines on both sides.

### `diff.staged-changes` — Which changes are in the index already

Tells, in a diff of the working tree against `HEAD`, which changes are staged, so that they are drawn dimmed and offer
to be unstaged.

- **Kind**: Derivation
- **Function**: `model::diff::staged_changes(input) -> Vec<bool>`
- **Reads**: the index with `git show :<path>` (none for an untracked file, whose changes are never staged).
- **Uses**: `diff.line-diff`.

- A change is staged when no difference between the index and the working tree touches its lines in the working tree.
  Staging copies the lines into the index, so once a change is staged nothing differs there; a change staged in part
  still differs, and is not marked.
- Lines are placed as hunk headers place them; an insertion sits between two lines, so two insertions at the same place
  touch, and a change of lines also touches the place just before its first line.
- Only against `HEAD`: against the index or a commit, a change says nothing of what is staged.

Drawn: a staged change has its line backgrounds faded (the text keeps its colors), its band faded, the tooltip
**Staged**, and its button in the gutter is **Unstage this change** in place of **Stage this change**.

Acceptance:

- [ ] After staging one of two changes with `diff.stage-change`, that change is drawn dimmed and the other is not.
- [ ] After staging part of a change with `git add -p`, the change is not dimmed.
- [ ] An untracked file has no dimmed change.

### `diff.change-patch` — The patch of one change

The patch that moves one change into the index, or back out of it, given to `git apply --cached` on standard input.

- **Kind**: Derivation
- **Function**: `text::diff::change_patch(input) -> Option<String>`
- **Uses**: `diff.line-diff`.

- **To stage**: the differences between the exact text of the index and the working tree that touch the lines of the
  change in the working tree (as in `diff.staged-changes`). None when there are none: it is staged already.
- **To unstage**: the differences between the exact text of the index and `HEAD` that touch the change: in the index's
  lines when the diff is of the index against `HEAD`, in `HEAD`'s lines when it is the working tree against `HEAD`. None
  when nothing is staged there.
- Both go forward from the index as it is, so that the line numbers are right whatever else of the file is staged.
- The patch is `diff --git a/<path> b/<path>`, `--- a/<path>`, `+++ b/<path>`, then for each difference a header
  `@@ -<start>,<count> +<start>,<count> @@` with **no line of context**, its old lines with `-` and its new lines with
  `+`. The new start counts the lines that the differences before it in the patch added or removed. The path uses `/`.
- A last line with no line break after it is followed by `\ No newline at end of file`.
- Lines added when staging take the line endings of the index (CRLF when the index has a CRLF, LF otherwise), as
  `git add` would store them; lines put back when unstaging keep theirs.

Acceptance:

- [ ] Staging a 2-line insertion after line 12 gives a patch with the header `@@ -12,0 +13,2 @@` and the two lines.
- [ ] With an earlier change of the same file staged, staging a later one gives a header whose old start is that of the
      index, and `git apply --cached --unidiff-zero` applies it.
- [ ] A change at the last line of a file with no final line break carries `\ No newline at end of file`.
- [ ] Staging into an index stored with CRLF writes CRLF in the added lines.

### `diff.stage-change` — Stage this change

Puts one change of the working tree into the index.

- **Kind**: Operation
- **Function**: `features::diff::stage_change(context, answers) -> Outcome`
- **Entry points**: the **+** button (**Stage this change**) on the band of a change in the gutter.
- **Extension**: the stage button in the gutter of the Flow Diff on a change, offered while the Flow Diff shows a file
  against `HEAD` or the index without a rename (`canStage` in `src/flow/sides.ts`). `ChunkActions` in
  `src/flow/chunks.ts` sends `diff.stage-change` to the engine with a `diffChange` target (`changeTarget`: the path,
  the two sides, the lines of the change on each and their text), through the `runOp` the Flow Diff is given, which is
  `Controller.runIn` and so `Controller.runOnEngine`. The engine checks the change against the file on disk: a file with
  unsaved edits in VS Code is refused with *The file changed since the diff was shown. Try again.*, where the extension
  used to stage against the text in the editor. When the engine's binary is missing, it says so (D49). Ranges staged in
  VS Code's own diff editor (**Stage Selected Ranges**) are VS Code's, not Gitardo's: the Changes view shows their
  result.
- **Offered when**: the right side is the working tree and the left is `HEAD` or the index, the file is not renamed, and
  the change is not staged.
- **Reads**: the index and the working tree again, to check the change is where the page says (`diff.read-sides`).
- **Plan**: `git apply --cached --unidiff-zero --whitespace=nowarn -` with the patch of `diff.change-patch` on standard
  input. Titled **Stage change in \<file\>**; its detail, shown with the command in the preview, is *The patch on
  standard input changes this part of \<file\> in the index:* and the first `@@` line. For an untracked file there is no
  index to patch: the plan is `git add -- <file>`.
- **Danger**: None.
- **Undo**: Impossible with **Undo**, since the index has no reflog; **Unstage this change** takes it back.
- **After**: the page is computed again and the change is drawn dimmed.
- **Refuses**: when the file changed since the page was computed: *The file changed since the diff was shown. Try
  again.* When the file is not UTF-8 and the patch has a character above 127: *Staging a change needs UTF-8 text: stage
  the whole file.* When the patch starts at line 0 or 1 and the file starts with a byte order mark: *The first line of a
  file with a byte order mark cannot be staged as a change: stage the whole file.* When no difference between the index
  and the working tree touches the change: *This change is staged already.*
- **Settings**: `commandPreview` (with `all`, the plan is shown with the patch next to the command).
- **Uses**: `diff.change-patch`, `diff.staged-changes`.

Acceptance:

- [ ] Staging the second of three changes leaves `git diff --cached` with that change only, and the log has the
      `git apply` line exactly.
- [ ] With `commandPreview` set to `all`, the question shows the command and the patch before it runs.
- [ ] Editing the file in another program between drawing and clicking refuses with the message and stages nothing.
- [ ] Stage on an untracked file runs `git add -- <file>`.
- [ ] Extension: in the Flow Diff of `a.txt` against `HEAD`, the stage button of its first change runs
      `diff.stage-change` on the engine, the log has `git apply --cached --unidiff-zero --whitespace=nowarn -` and the
      patch after `standard input:`, and the Changes view then shows `a.txt` half ticked.
- [ ] Extension: with unsaved edits to `a.txt` in VS Code, the stage button of a change is refused with *The file
      changed since the diff was shown. Try again.*, and the index is unchanged.
- [ ] Extension: with `gitardo.commandPreview` set to `all`, *Stage change in a.txt?* shows the command and the note
      with the `@@` line of the change before it runs.
- [ ] Extension: on a file whose encoding is not UTF-8, a change with a character past ASCII is refused with *Staging a
      change needs UTF-8 text: stage the whole file.*, and the index is unchanged.

### `diff.unstage-change` — Unstage this change

Takes one change back out of the index.

- **Kind**: Operation
- **Function**: `features::diff::unstage_change(context, answers) -> Outcome`
- **Entry points**: the **−** button (**Unstage this change**) on a staged change of the working tree against `HEAD`
  (its tooltip **Staged — unstage this change**), and on every change of the index against `HEAD`.
- **Extension**: the unstage button in the gutter of the Flow Diff on a change already staged, drawn dimmed: as
  `diff.stage-change` says, `ChunkActions` sends `diff.unstage-change` with a `diffChange` target through
  `Controller.runOnEngine`, and the engine refuses a change of a file with unsaved edits; a change of a file that `HEAD`
  does not have is unstaged whole, `git restore --staged -- <file>`.
- **Offered when**: the file is not renamed and the change is staged.
- **Reads**: the index and `HEAD` again, to check the change is where the page says.
- **Plan**: `git apply --cached --unidiff-zero --whitespace=nowarn -` with the patch of `diff.change-patch` towards
  `HEAD` on standard input, titled **Unstage change in \<file\>**. A file that is in the index and not in `HEAD` is one
  change: the plan is `git restore --staged -- <file>`, or `git rm --cached -r -- <file>` before the first commit.
- **Danger**: None.
- **Undo**: Impossible with **Undo**; **Stage this change** takes it back.
- **After**: the page is computed again.
- **Refuses**: as `diff.stage-change`; when no difference between the index and `HEAD` touches the change: *Nothing
  of this change is staged.*

Acceptance:

- [ ] Unstaging one of two staged changes leaves the other staged and the working tree unchanged.
- [ ] In the diff of the index against `HEAD`, unstaging the only change removes it from the page.
- [ ] A newly added file unstages with `git restore --staged -- <file>`.
- [ ] Extension: the unstage button on a change staged in the Flow Diff takes it out of the index, and the file on disk
      is unchanged.

### `diff.revert-change` — Revert this change

Puts back in the working tree what the left side has in place of one change.

- **Kind**: Operation
- **Function**: `features::diff::revert_change(context, answers) -> Outcome`
- **Entry points**: the **⟲** button (**Revert this change**) on the band of a change in the gutter.
- **Offered when**: the right side is the working tree (against `HEAD`, the index or a commit).
- **Reads**: the text of the working-tree side, to check the lines of the change are still there.
- **Plan**: the lines of the change are replaced in the working-tree side by the lines the left side has there (a
  removed change puts its lines back after the line before it), as one edit of that side, and the file is saved:
  `write <file>` (`file-view.save`).
- **Danger**: None.
- **Undo**: `Ctrl+Z` in the working-tree side takes the edit back, unsaved until `Ctrl+S`; **Undo** does not.
- **After**: the page is computed again; the change is gone.
- **Refuses**: *The file changed since the diff was shown. Try again.*
- **Uses**: `diff.edit`, `file-view.save`.

Acceptance:

- [x] Reverting a changed line against `HEAD` gives the file the line `HEAD` has, and the page has one change fewer.
- [x] Reverting a removed change at the end of a file with no final line break puts the lines back after the last
      line.
- [x] `Ctrl+Z` right after brings the change back, and the page shows the file unsaved.

### `diff.edit` — Editing the working tree side

The file can be edited where it is shown, when the right side is the working tree.

- **Kind**: View
- **Function**: `views::diff::edit_view(sides, ui) -> Vec<Intent>`, over the rows the list of the page drew (Q336)
- **Entry points**: typing in the right side of a Diff page whose right side is the working tree; `Ctrl+S` (`Cmd+S` on
  macOS) to save.
- **Uses**: `file-view.edit`, `file-view.save`.

- What is typed goes into the file's text, the same text a File page of the file shows: it is unsaved until `Ctrl+S`,
  and the Diff page and the Changes page show it unsaved.
- The diff is computed again a moment (200 ms) after typing stops; what is still being typed is never overwritten by it.
- The left side, and both sides when the right is not the working tree, are read-only.

Acceptance:

- [x] Typing a line in the right side shows it as an added change within half a second, and `Ctrl+S` writes the file
      with `write <file>` in the log.
- [x] An edit made in the File page of the same file shows in the Diff page, and the other way round.
- [x] In a diff of the index against `HEAD`, typing does nothing.

### `diff.open-at-line` — Open the file at this line

Opens the file in the File page at the line of the cursor.

- **Kind**: View
- **Function**: `views::diff::open_at_line_view(state, ui) -> Vec<Intent>`
- **Entry points**: `F4` in either side; **Jump to source** in the context menu of either side.
- **Uses**: `file-view.page`.

- From the right side, the line is the cursor's. From the left side, it is the right side's line on the same row; for a
  removed line, the nearest line of the right side above it (where the text used to be); line 1 when there is none.

Acceptance:

- [x] `F4` on line 30 of the right side opens the File page at line 30.
- [x] `F4` on a removed line of the left side opens the File page at the line above where it was.

### `diff.open-file` — Open file

Opens the file in the File page.

- **Kind**: View
- **Function**: `views::diff::open_file_view(state, ui) -> Vec<Intent>`
- **Entry points**: the title button **Open file** of a Diff page, and of each file of a page of several.
- **Offered when**: the file is in the working tree.
- **Uses**: `file-view.page`.

Acceptance:

- [ ] **Open file** on the diff of `src/app.rs` opens the File page of `src/app.rs` from the working tree.

### `diff.stage-file` — Stage file

Stages the whole file shown.

- **Kind**: Operation
- **Function**: `features::diff::stage_file(context, answers) -> Outcome`
- **Entry points**: the title button **Stage file** of a Diff page.
- **Offered when**: the right side is the working tree and the left is `HEAD` or the index.
- **Plan**: `git add -- <file>`.
- **Danger**: None.
- **Undo**: Impossible with **Undo**; **Unstage file** takes it back.
- **After**: the page is computed again.

Acceptance:

- [ ] **Stage file** runs exactly `git add -- <file>` and every change of a diff against `HEAD` is drawn dimmed.

### `diff.unstage-file` — Unstage file

Takes the whole file out of the index.

- **Kind**: Operation
- **Function**: `features::diff::unstage_file(context, answers) -> Outcome`
- **Entry points**: the title button **Unstage file** of a Diff page.
- **Offered when**: the right side is the index.
- **Plan**: `git restore --staged -- <file>`; before the first commit, `git rm --cached -r -- <file>`.
- **Danger**: None.
- **Undo**: Impossible with **Undo**; **Stage file** takes it back.
- **After**: the page is computed again and shows no change.

Acceptance:

- [ ] In the diff of the index against `HEAD`, **Unstage file** runs `git restore --staged -- <file>`.
- [ ] In a repository with no commit yet, it runs `git rm --cached -r -- <file>`.

### `diff.discard-file` — Discard changes

Takes the file back to its staged version, or deletes it when it is untracked.

- **Kind**: Operation
- **Function**: `features::diff::discard_file(context, answers) -> Outcome`
- **Entry points**: the title button **Discard changes** of a Diff page.
- **After**: the page is computed again; a page whose file is then on neither side closes.
- **Offered when**: the right side is the working tree and the left is `HEAD` or the index.
- **Plan**: `git restore -- <file>` for a tracked file; `git clean -f -- <file>` for an untracked one.
- **Danger**: Destructive, because *Unstaged changes to tracked files will be lost.* or *One untracked file will be
  deleted from disk.*
- **Undo**: Impossible: what was not staged is not kept anywhere.
- **Refuses**: *There is nothing to discard.*

Acceptance:

- [ ] On a tracked file with staged and unstaged changes, **Discard changes** asks with the warning and runs
      `git restore -- <file>`; the staged changes stay.
- [ ] On an untracked file it runs `git clean -f -- <file>` and the page closes with the file.

### `merge-view.page` — The Merge page

A file in conflict in three columns: what the current side has, the result, and what the incoming side has, each
change of a side joined by a band to where it is in the result.

- **Kind**: View
- **Function**: `views::merge_view::page_view(state, ui) -> Vec<Intent>`
- **Entry points**: `conflicts.open-file` on a file with all three versions; **Resolve by hand** on a card of the review
  of the conflicts; **Open in the Merge page** in Quick search and the title button of the same name of a File page,
  for the file of the page in front when it is in conflict.
- **Reads**: `merge-view.read-sides`.
- **Refuses**: a file that is not in conflict with a version on each side: *This file is not in conflict with a change on
  each side.* A NUL byte in any of the four texts: *This file looks binary.* A diff that takes more than 5 seconds:
  *This file has too many differences to display.*
- **Asks**: when the four texts have more than 200,000 lines together: **This file is very large and the merge may be
  slow to show. Continue?** with **Continue**.
- **Settings**: `conflicts.autoResolve` and `conflicts.ai`, for the proposals.
- **Uses**: `merge-view.read-sides`, `merge-view.three-way`, `diff.word-diff`, `merge-view.take-side`,
  `merge-view.accept-simple`, `merge-view.accept-proposals`, `merge-view.show-base`, `merge-view.navigate`,
  `merge-view.open-at-line`, `merge-view.mark-resolved`, `merge-view.close`, `file-view.edit`, `file-view.highlight`,
  `conflicts.side-names`.

- The page is titled **\<file name\> (Current ↔ Incoming)** in its trail; one page per file, and asking again shows it.
- The columns are labelled **Current — \<what the current side is\>**, **\<path\> — Result** and **Incoming — \<what the
  incoming side is\>**, with the words of `conflicts.side-names` (**Current — The branch checked out**, **Incoming —
  The branch being merged** in a merge).
- Each side shows what it changed of the common ancestor, in the color of what became of it, and a band in the gutter
  joins each of those changes to where it is in the result: **green** for a change of the current side, **blue** for one
  of the incoming side, **grey** for a change both sides made the same way, **red** for a conflict. The left gutter has
  the bands of the current side (every region but those of the incoming side alone), the right gutter those of the
  incoming side. Within a changed line of a side, the words that changed against the base are marked.
- The result is the file as git wrote it, with what is typed in it: the lines of git's markers (`<<<<<<<`, `|||||||`,
  `=======`, `>>>>>>>`) are marked, and the two parts of a conflict between them are each joined, by a mark in the
  margin, to the side it comes from. A conflict resolved in the file, whose markers are gone, is drawn faintly.
- The scroll bar of the result has a mark for each region in its color.
- The columns are kept level: in each region, and between two regions, the lines of each column face each other in
  order, the column with fewer lines there having blank rows after them, so that the columns scroll together as one
  list. A region has at least one row, so that its band shows even where it took every line out.
- The page is computed again after each choice, and when the repository is read again: from the file on disk, or from
  the result it holds when that has changes that are not saved.

Acceptance:

- [ ] During a merge of `feature` into `main`, opening a file in conflict shows three columns labelled **Current — The
      branch checked out**, **src/app.rs — Result** and **Incoming — The branch being merged**.
- [ ] A change made only on `main` has a green band from the current side to the result, and none on the right.
- [ ] A conflict has red bands on both sides, and the lines of its markers in the result are marked.
- [ ] Deleting the markers of a conflict by hand and keeping one part draws that region faintly.
- [ ] A file in conflict that both sides added opens with an empty base.

### `merge-view.read-sides` — The versions of a file in conflict

Reads the versions the Merge page compares.

- **Kind**: Read
- **Function**: `git::read::merge_view::read_sides(git, &request) -> Result<MergeSides, GitError>`
- **Reads**: `git ls-files --unmerged -z -- <file>` for its stages; `git show :1:<file>` (the base; empty when there is
  no stage 1), `git show :2:<file>` (current) and `git show :3:<file>` (incoming); the result from the text the File
  page holds for the file when it has unsaved edits, else from the disk; the operation in progress, for the names.

Acceptance:

- [ ] For a file with stages 2 and 3 only, the base is empty and the page opens.
- [ ] Unsaved edits of the file in a File page are the result.

### `merge-view.three-way` — The regions of a file in conflict

Aligns the base, the two sides and the result: what each side changed of the base, which of those changes conflict,
and where each one is in the result as it is now.

- **Kind**: Derivation
- **Function**: `model::merge_view::three_way(input) -> Option<MergeRegions>`
- **Uses**: `diff.line-diff`, `diff.word-diff`, `conflicts.parse-markers`.

What the result must satisfy:

1. The four texts are read with `\r\n` as `\n`.
2. The changes of each side are those of the minimal line diff from the base to that side.
3. Taken in the order of the base, the changes of the two sides that touch (they share a line of the base, or are next
   to each other, as `conflicts.resolve-by-rules` defines it) are put in one **region**; a change that touches none is a
   region of its own. A region holds a stretch of the base and the stretch of each side that stands for it. Its kind is
   **current** or **incoming** when only that side changed it, **both** when the two sides have the same lines there,
   **conflict** otherwise.
4. Where each region is in the result: the result git would have written is made from the regions (the lines of the
   current side between regions, the changed side's lines in a region of one side, the current side's lines in a region
   of both, and `<<<<<<<`, the current lines, `=======`, the incoming lines, `>>>>>>>` for a conflict). It is diffed
   against the real result, and each region's start and end are carried through that diff: a place inside a changed
   stretch, or at an insertion, goes to the start of what replaced it for the start of a region and to its end for the
   end of a region. This finds each region whatever git did differently and whatever was edited since.
5. A region that a conflict still marked in the result overlaps is a **conflict**, whatever the diffs found, and covers
   the whole marked block; its current part is the lines after `<<<<<<<` up to `|||||||` or `=======`, its incoming part
   the lines after `=======` up to `>>>>>>>`. It is **unresolved**.
6. What the result has in any other region is one of, checked in this order: the base, the current side, the incoming
   side, the current then the incoming side, the incoming then the current side; else **edited**.
7. The word marks of each side are those of `diff.word-diff` on its lines paired with the base's.

Acceptance:

- [ ] Base `a b c`, current `a B c`, incoming `a b c` gives one current region over line 2, whose result is the current
      side.
- [ ] Base `a b c`, current `a B c`, incoming `a b C` gives one conflict region over lines 2 and 3 (they touch).
- [ ] With git's markers in the result, the conflict region covers the 7 lines from `<<<<<<<` to `>>>>>>>` and is
      unresolved; once the lines between them are replaced by `B C`, it is resolved and its state is edited.
- [ ] Lines typed in the result above a region move the region down by as many lines.
- [ ] A file whose sides use CRLF gives the same regions as with LF.

### `merge-view.take-side` — Accept a side for a region

Puts a side in place of its region in the result.

- **Kind**: View
- **Function**: `views::merge_view::take_side_view(state, ui) -> Vec<Intent>`
- **Entry points**: the buttons on a band: on the current side **→** (**Accept current**) and, on a conflict, **+**
  (**Accept both, current first**); on the incoming side **←** (**Accept incoming**) and, on a conflict, **+**
  (**Accept both, incoming first**).
- **Offered when**: a button shows only where it would change what the result has in the region; "both" only on a
  conflict.
- **Uses**: `merge-view.three-way`, `file-view.edit`.

- The lines of the region in the result are replaced by the lines of that side (or of both, in that order), as one edit
  of the result: the region is then that choice, and what is after it moves.
- `Ctrl+Z` (`Cmd+Z` on macOS) in the result takes a choice back, and `Ctrl+Y` puts it again.
- What is chosen goes into the file's text and is not saved until `Ctrl+S` (`file-view.save`).
- When something typed in the result has not been taken into the regions yet, the choice waits for the regions of
  that text, so that it lands where the user sees it.

Acceptance:

- [ ] On a conflict `B1` against `B2`, **→** makes the result `B1` and removes the markers; **+** on the current side
      makes it `B1`, `B2`.
- [ ] After **→**, the **→** button of that region is gone and **←** is still there.
- [ ] `Ctrl+Z` after **→** brings back the markers exactly.
- [ ] A region of the current side only, already in the result, has no button.

### `merge-view.accept-simple` — Accept what needs no choice

Accepts at once every change that needs no choice.

- **Kind**: View
- **Function**: `views::merge_view::accept_simple_view(state, ui) -> Vec<Intent>`
- **Entry points**: the button with two ticks over the result.
- **Offered when**: at least one region would change; otherwise it is disabled with the tooltip *No changes to accept
  without a choice*. Its tooltip says *Accept the 3 changes that need no choice: of one side, or the same on both*.
- **Uses**: `merge-view.three-way`.

For each region, where the result does not have it already: a region of the incoming side takes the incoming side; a
region of the current side or of both takes the current side; a conflict whose two sides have the same lines, or whose
incoming side has the base's lines, takes the current side; a conflict whose current side has the base's lines takes the
incoming side. All of it is one edit: one `Ctrl+Z` takes all of it back.

Acceptance:

- [ ] A file where git marked a conflict whose two parts are the same lines (as the `diff3` style can) gets those lines
      once, without markers.
- [ ] A conflict `B1` against `B2` is left as it is.
- [ ] One `Ctrl+Z` after it brings back every marker it removed.

### `merge-view.accept-proposals` — Accept the proposals

Puts in the result what the rules or the AI propose for the conflicts still marked.

- **Kind**: View
- **Function**: `views::merge_view::accept_proposals_view(state, ui) -> Vec<Intent>`
- **Entry points**: the **✦** button (**Accept the proposal**) on the band of the current side of a conflict still
  marked; the sparkle over the result, which accepts all of them.
- **Offered when**: `conflicts.autoResolve` is on and the rules resolve the conflict, or the AI provider was asked about
  it (`ai.resolve-conflicts`). The sparkle is there while any proposal exists, and disabled with *Nothing is proposed for
  the conflicts left* when none is for a conflict still marked.
- **Uses**: `review.proposals`, `conflicts.resolve-by-rules`.

- The tooltip of **✦** gives where it comes from and why: *Accept the proposal of the rules — Changes to lines next to
  each other*, or *Accept the proposal of \<model\>, with high confidence — \<reason\> One line is new: neither side has
  it.*
- The tooltip of the sparkle says *Accept what is proposed for the 2 conflicts, by the rules* (*by the AI*, *1 of them by
  the AI*).
- What is accepted replaces the lines the proposal replaces (for the rules, possibly some lines around the markers), as
  one edit: `Ctrl+Z` takes it back.

Acceptance:

- [ ] With `conflicts.autoResolve` on, a conflict of lines next to each other has a **✦** whose tooltip names the rules;
      clicking it removes its markers and writes both changes.
- [ ] After the AI answered a conflict in the review page, opening the Merge page shows **✦** on it with the model's
      name.
- [ ] The sparkle accepts every proposal at once; one `Ctrl+Z` takes all of them back.

### `merge-view.show-base` — Show the base

Shows the base, what the two sides started from, in a column at the left of the current side.

- **Kind**: View
- **Function**: `views::merge_view::show_base_view(state, ui) -> Vec<Intent>`
- **Entry points**: the fork button over the result: **Show the base** or **Hide the base**.

- The column is labelled **Base — What the two sides started from**, the lines each region changed in the color of the
  region, and bands join it to the current side; it scrolls with the current side.
- Whether it is shown is remembered: the next Merge page opens the same way.

Acceptance:

- [ ] **Show the base** adds the fourth column, joined to the current side by bands; **Hide the base** removes it.
- [ ] After showing it and closing the page, the next Merge page opens with the base shown.

### `merge-view.navigate` — Changes and conflicts left

Goes from one change to the next and says how many conflicts are left.

- **Kind**: View
- **Function**: `views::merge_view::navigate_view(state, ui) -> Vec<Intent>`
- **Entry points**: `F7` and `Shift+F7` in any column; the arrows ▲ (**Previous change (Shift+F7)**) and ▼ (**Next
  change (F7)**) over the result; the title buttons **Previous change** and **Next change**.

- Every region is a stop, and the counter reads **2 / 5**, **– / 5** or **No changes**, as in `diff.navigate`. The
  current region is outlined in every column and centered in each.
- Past the last region, `F7` goes round to the first: the Merge page does not go on to another file.
- Beside the arrows: **2 conflicts left** (**1 conflict left**), **No conflicts left** once none is marked, or **No
  conflicts** when the file had none.

Acceptance:

- [ ] With 3 conflicts marked, the page reads **3 conflicts left**; after **→** on one, **2 conflicts left**.
- [ ] After the last conflict is resolved, it reads **No conflicts left**.
- [ ] `F7` on the last region goes to **1 / N**.

### `merge-view.open-at-line` — Open the file at this line

Opens the file in the File page at the line of the result the cursor stands for.

- **Kind**: View
- **Function**: `views::merge_view::open_at_line_view(state, ui) -> Vec<Intent>`
- **Entry points**: `F4` in any column; **Jump to source** in the context menu of a line.
- **Uses**: `file-view.page`.

- A click on a line puts the cursor on it, marked. With no cursor, `F4` takes the first line of the region gone to,
  or the first line of the file.
- When the result has choices not saved, the File page holds them, not saved, as one edit that `Ctrl+Z` takes back.
- In the result, the line is the cursor's. In a side or the base, a line before a region moves by as many lines as the
  regions before it grew or shrank in the result; a line inside a region goes to the first line of the region in the
  result (for a conflict still marked, the first line of that side's part).

Acceptance:

- [x] `F4` on line 40 of the current side, below a region that has 2 more lines in the result, opens the File page at
      line 42.
- [x] `F4` inside the incoming side of a marked conflict opens the File page at the first line after `=======`.

### `merge-view.mark-resolved` — Mark as resolved

Saves the file and stages it, which tells git that its conflicts are resolved, and closes the page.

- **Kind**: Operation
- **Function**: `features::merge_view::mark_resolved(context, answers) -> Outcome`
- **Entry points**: **Mark as resolved** (*Save the file and stage it, which tells git that its conflicts are
  resolved*), at the bottom of the result.
- **Offered when**: the page is open.
- **Asks**: when markers of git are left in the result: **app.rs still has 2 conflicts marked by git. Mark it as
  resolved all the same, with the markers in it?** with **Mark as resolved**; no default.
- **Plan**: `write <file>` when the result has unsaved changes (what was being typed included), then `git add -- <file>`.
- **Danger**: None; the question above is asked whenever markers are left, since staged they would be committed.
- **Undo**: Impossible with **Undo**: staging leaves nothing in the reflog.
- **After**: the page closes; the file leaves the conflicts of the Changes page.
- **Uses**: `file-view.save`, `conflicts.mark-resolved`.
- **Extension**: **Mark as Resolved** in the Flow Merge. The extension saves the document in VS Code itself and asks
  itself about the markers left, **shapes.ts still has a conflict marked by git. Mark it as resolved all the same, with
  the markers in it?**; then it sends `feature/run` of `merge-view.mark-resolved` to Gitardo's engine with a
  `mergeResult` target (`Controller.runOnEngine` in `src/extension/controller.ts`, `engineRuns` in
  `src/engine/actions.ts`), which stages the file. When the engine's binary is missing, it says so (D49).

Acceptance:

- [ ] With every conflict chosen and the file unsaved, the log has `write src/app.rs` then `git add -- src/app.rs`, and
      the page closes.
- [ ] With 1 conflict left, it asks first; cancelling writes and stages nothing and the page stays.
- [ ] With the file saved already, the plan is `git add -- <file>` alone.
- [ ] Extension: **Mark as Resolved** in the Flow Merge of `shapes.ts` with a marker left asks the question above first;
      on yes the file is saved and staged, and leaves the **Conflicts** group of the Changes view.

### `merge-view.close` — Closing with unsaved changes

Closing a Merge page never loses what was chosen or typed in it.

- **Kind**: View
- **Function**: `app::store::merge_page::lines::closed(state, page) -> Vec<Effect>`
- **Entry points**: closing a Merge page.
- **Uses**: `file-view.page`.

- When the result has changes that are not saved, the file is shown in a File page with them, in the same place, so
  that they are not forgotten: at the line of the cursor, or else of the region gone to, or else at the first line.
  They are one edit there, which `Ctrl+Z` takes back to the file as it is on disk.
- Closed by **Mark as resolved**, or because its file is no longer in conflict, nothing is shown in its place.

Acceptance:

- [x] Choosing **→** on a conflict and closing the page opens the File page of the file, unsaved, with the choice in it.
- [x] Closing a page with nothing unsaved opens nothing.

### `file-view.page` — The File page

A file in Gitardo's own text editor: a file of the working tree, which can be edited, or a file as it is at a revision,
read-only.

- **Kind**: View
- **Function**: `views::file_view::page_view(state, ui) -> Vec<Intent>`
- **Entry points**: a click on a file in the Files page, beside its tree (`file-view.beside-files`); a click on a file
  in the Changes page when `changes.openOnClick` is not `diff`, and **Open file** on its row; **Open file** of a Diff
  page; `F4` in a Diff or Merge page; `conflicts.open-file` for a file both sides added; a line number in the review of
  the conflicts; `merge-view.close`; **Open the file at this revision** on a file of a commit; **Blame the file as it
  was before** in the blame (`blame.`).
- **Reads**: the file from the disk; at a revision, `file-view.read-revision`.
- **Settings**: `editor.wordWrap`, `markdown.openAsPreview`.
- **Uses**: `file-view.read-revision`, `file-view.edit`, `file-view.save`, `file-view.find`, `file-view.go-to-line`,
  `file-view.open-external`, `file-view.highlight`, `blame.` (the blame of the line under the cursor and of the whole
  file, which draw in this page), `file-history.` (the history of the file and of the selected lines, from its context
  menu), `merge-view.page`.

- The page is titled with the file's name in its trail; a file at a revision is titled **\<name\> (\<short hash\>)** and
  is read-only. One page per file and revision: asking again shows it, at the line asked for.
- The editor has line numbers, the colors of the code, a cursor and selections, and the font and tab width of the
  editor settings. It opens at the line asked for, centered, with the cursor at its start.
- A line longer than the page is cut at its right edge, or, with `editor.wordWrap` `on`, wraps into rows as wide as
  the page: after a space between words, inside a word only when the word is longer than the page. Its number, and the
  blame beside it, stay on its first row; its other rows start in the column of the text. A click on a row puts the
  cursor on that row, and a selection is drawn on every row it covers.
- The title buttons: **Open in external editor**; the buttons of the blame; **Open in the Merge page** when the file is
  in conflict with a version on each side; **Preview** and **Source** for a Markdown file
  (`file-view.markdown-preview`), the one shown filled.
- The context menu: cut, copy, paste, **Find**, **Go to line**, and the entries the blame and the file history add.
- A file of the working tree that changes on disk while it has no unsaved edits is read again.
- A file that is not text (a NUL byte) shows *This file is not text.* and nothing else.

Acceptance:

- [ ] Clicking `src/app.rs` in the Files page opens a page **app.rs** with the file, editable.
- [ ] Opening `src/app.rs` at `3f2a1c9` opens a read-only page **app.rs (3f2a1c9)**; typing does nothing.
- [ ] Opening the same file at line 120 when its page is open shows that page, scrolled to line 120.
- [ ] Changing the file in another program while the page has no unsaved edits shows the new text.

### `file-view.beside-files` — A file opened beside the tree of the Files page

A file of the Files page opens on the right of its tree, which stays, so that the files of the working tree can be
opened one after the other.

- **Kind**: View
- **Function**: `views::file_view::beside_files_view(state: &BesideFilesState, ui) -> Vec<Intent>`
- **Entry points**: a click on a file in the Files page; `Enter` on a file of its tree.
- **Uses**: `file-view.page`, `graph.resize-panes`.
- **Extension**: the Explorer of VS Code stays beside the editor: nothing to build.

- The Files page is split in two: its tree on the left, as wide as the branch rail starts (210 px, dragged as
  `graph.resize-panes` says, and remembered for the workspace), and the File page of the file opened on its right.
- The tree marks the file shown and keeps where it was scrolled and which folders were open.
- A click on another file shows it in the same place; a file with unsaved edits stays open in its own page, listed in
  `Ctrl+Tab`, and the place shows the new one.
- The trail reads `Timeline › Files › app.rs`; **Files** shows the tree alone again.
- In the narrow layout the File page takes the whole page, and **Files** in the trail comes back to the tree.

Acceptance:

- [ ] A click on `src/app.rs` in the Files page shows the file on the right, and the tree is still on the left with
      `src/app.rs` marked.
- [ ] A click on `README.md` then shows it in the same place, and the folder `src` is still open in the tree.
- [ ] With unsaved edits in `src/app.rs`, opening `README.md` keeps `src/app.rs` in `Ctrl+Tab`.

### `file-view.markdown-preview` — A Markdown file as it reads

A Markdown file in the File page shown laid out as it reads, with **Preview** and **Source** to switch
([D59](../08-decisions.md#d59--markdown-files-shown-as-they-read)).

- **Kind**: View
- **Function**: `views::file_view::markdown_view(state: &MarkdownState, ui) -> Vec<Intent>`, over the derivation
  `model::markdown::blocks(text: &str) -> Vec<Block>`
- **Entry points**: **Preview** and **Source** in the title row of the File page of a file ending in `.md` or
  `.markdown`; any way of opening such a file while `markdown.openAsPreview` is on.
- **Settings**: `markdown.openAsPreview` (`false`).
- **Uses**: `file-view.page`, `file-view.highlight`.

Rules:

- CommonMark with the tables, the task lists and the strikethrough of GitHub. Headings, paragraphs, emphasis, inline
  code, lists, quotes, rules, code blocks in the colors of the code of their language, tables with their alignment.
  The window has no italic typeface: emphasis is drawn in the medium weight, strong text in the bold one
  ([Q580](../08-decisions.md#open-questions)). A word wider than the page is cut where it overflows; a line of a code
  block wider than it ends with `…`.
- HTML in the file is shown as text, never rendered; nothing in the preview runs.
- A link to a web page opens in the browser; a link to a file of the repository, relative to the file, opens that file
  in its File page, at the revision of the page when it shows one, at the line of a `#L12` after it; a link to a
  heading of the file scrolls to it, the heading named as GitHub names it (`#getting-started`). Another scheme, such as
  `javascript:`, and a path out of the repository go nowhere. An image of the repository is drawn, as wide as the page
  at most, read on a worker; only PNG is read, and an image of another format shows its text, as an image from the
  web, which is not fetched ([Q581](../08-decisions.md#open-questions)).
- The preview follows the text: edits made in **Source** show in **Preview** when it is shown again; a file of the
  working tree changed on disk is read again, as in **Source**.
- With `markdown.openAsPreview` on, a Markdown file opens in **Preview**; otherwise in **Source**. The switch is kept
  for the page while it is open. A file at a revision has the preview too, read-only as its text.
- **Find** in the preview, `Ctrl+F`, finds in the text shown, as find in page does
  ([Q582](../08-decisions.md#open-questions)).
- Extension: with `gitardo.markdown.openAsPreview` on, a Markdown file that Gitardo opens (the Files view, the Changes
  view, a file of a commit) opens in VS Code's preview of Markdown (`markdown.showPreview`), not in its editor.

Acceptance:

- [x] `README.md` with a heading, a list, a table and a Rust code block shows them laid out in **Preview**, the code in
      the colors of Rust.
- [x] **Source** shows the text of the file, editable; **Preview** shows it laid out again, with the edit.
- [x] With `markdown.openAsPreview` on, `docs/guide.md` opened from the Files page opens in **Preview**; off, in
      **Source**.
- [x] `<script>alert(1)</script>` in the file is shown as that text.
- [x] A link to `../CHANGELOG.md` opens the File page of `CHANGELOG.md`; a link to `https://example.com` asks the
      browser to open it.
- [x] An image `media/logo.png` of the repository is drawn; `https://example.com/a.png` shows its text.
- [x] `src/app.rs` has no **Preview**.
- [ ] Extension: with `gitardo.markdown.openAsPreview` on, a click on `README.md` in the Files view opens VS Code's
      preview of it.

### `file-view.read-revision` — A file at a revision

Reads the text of a file as it is in a commit.

- **Kind**: Read
- **Function**: `git::read::file_view::read_revision(git, &request) -> Result<Option<Vec<u8>>, GitError>`
- **Reads**: `git show <revision>:<path>`, the path from the root of the repository with `/`. None when the commit has no
  such file.

Acceptance:

- [ ] The text read is byte for byte the blob of the commit, CRLF included.
- [ ] A path the commit does not have gives none, not an error.

### `file-view.edit` — Editing a file

Typing in a file, with undo and redo, in the File page and in the editable sides of the Diff and Merge pages.

- **Kind**: View
- **Function**: `views::file_view::edit_view(state, ui) -> Vec<Intent>`
- **Entry points**: typing in a File page of the working tree, in the working-tree side of a Diff page, or in the result
  of a Merge page; `Ctrl+Z` (`Cmd+Z`) to undo; `Ctrl+Y` or `Ctrl+Shift+Z` (`Cmd+Shift+Z`) to redo.

- A file has one text while it is open, whichever pages show it: what is typed in one shows in the others, and each
  keeps its own cursor. The text keeps the line endings the file has.
- Typing makes the file unsaved: its page has a dot in its trail, and the Changes page shows it with a dot (`changes.`).
  Nothing is written until it is saved.
- Undo and redo go through what was typed and what the buttons of a page did (a revert of a change, a choice of a side,
  accepting proposals), each as one step.
- Closing the last page of a file that has unsaved edits, other than a Merge page (`merge-view.close`), asks **Save
  changes to app.rs?** with **Save**, **Don't save** and **Cancel**.

Acceptance:

- [ ] Typing in the File page of `app.rs` shows the text in its open Diff page and puts a dot on both pages.
- [ ] Three typed words and `Ctrl+Z` three times give back the saved text, and the dot goes away.
- [ ] A file with CRLF line endings keeps CRLF in the lines typed.
- [ ] Closing the File page of an unsaved file asks; **Don't save** leaves the disk as it was.

### `file-view.save` — Save

Writes the text of a file to the disk.

- **Kind**: Operation
- **Function**: `features::file_view::save(context, answers) -> Outcome`
- **Entry points**: `Ctrl+S` (`Cmd+S` on macOS) in a File page, in the working-tree side of a Diff page and in the
  result of a Merge page; **Save** and **Save all** in the Changes page; `diff.revert-change`;
  `merge-view.mark-resolved`.
- **Offered when**: the file has unsaved edits.
- **Plan**: `write <file>`, with the text as it is, in its own line endings and encoding.
- **Danger**: None.
- **Undo**: Impossible with **Undo**; `Ctrl+Z` in the page goes on taking back the edits, unsaved again.
- **After**: the dot leaves the page and the Changes page; the pages of the file and the repository are read again.
- **Refuses**: when the file changed on disk since it was read: *app.rs changed on disk since it was opened.* with
  **Overwrite** and **Cancel**.

Acceptance:

- [ ] `Ctrl+S` after typing writes the file and the log has exactly `write src/app.rs`.
- [ ] With `commandPreview` set to `all`, saving is shown first as any change is.
- [ ] Saving a file changed on disk meanwhile asks; **Cancel** writes nothing.

### `file-view.find` — Find

Finds text in the file of the page.

- **Kind**: View
- **Function**: `views::file_view::find_view(state, ui) -> Vec<Intent>`
- **Entry points**: `Ctrl+F` (`Cmd+F` on macOS) in a File page or in a column of a Diff or Merge page; `F3` and
  `Shift+F3` (or `Enter` and `Shift+Enter` in the box) for the next and the previous match; `Escape` closes the box.

- The box finds as it is typed, marks every match, and says **3 of 12** or **No results**. Buttons: match case, whole
  word, regular expression.
- Going to a match unfolds what `diff.collapse-unchanged` folded around it.
- In a Diff or Merge page, it finds in the column it was opened from.

Acceptance:

- [ ] Typing `parse` in a file with it 12 times marks 12 matches and reads **1 of 12**; `F3` goes to **2 of 12**.
- [ ] With match case on, `Parse` does not match `parse`.
- [ ] An invalid regular expression says so and marks nothing.

### `file-view.go-to-line` — Go to line

Moves the cursor to a line.

- **Kind**: View
- **Function**: `views::file_view::go_to_line_view(state, ui) -> Vec<Intent>`
- **Entry points**: `Ctrl+G` (`Cmd+L` on macOS) in a File page or a column of a Diff or Merge page; **Go to line** in
  the context menu.
- **Asks**: **Go to line**, a number between 1 and the number of lines, with the current line as the default.

- The line is centered with the cursor at its start; a line past the end goes to the last line. In a Diff page, the
  number is a line of the column's own version.

Acceptance:

- [ ] `Ctrl+G` and `120` put the cursor at the start of line 120, centered.
- [ ] Typing `abc` is refused while typed.

### `file-view.open-external` — Open in external editor

Opens the file in the editor the user works in, outside Gitardo.

- **Kind**: Operation
- **Function**: `features::file_view::open_external(context, answers) -> Outcome`
- **Entry points**: the title button **Open in external editor** of a File page; the same entry in the context menu of a
  file in the Files page and in the Changes page.
- **Offered when**: the file is in the working tree.
- **Plan**: `run <command>`, the command of `editor.external` with the path of the file and the line of the cursor put
  in it, run in the folder of the repository; when `editor.external` is empty, the file is opened with the program the
  system has for it.
- **Danger**: None.
- **Undo**: Impossible, and nothing to take back: it changes nothing in the repository.
- **Refuses**: when the command cannot be started: *The external editor could not be started: \<what the system said\>.*
- **Settings**: `editor.external`.

Acceptance:

- [ ] With `editor.external` naming an editor that takes a path and a line, the page at line 42 runs it with the
      absolute path and 42, and the log has the `run` line.
- [ ] With `editor.external` empty, the file opens in the program the system has for its type.

### `file-view.highlight` — The colors of the code

Cuts the text of a file into tokens of a language and gives each the color the theme has for its kind, for the File,
Diff and Merge pages, the review of the conflicts, and the changes shown in the Commit and comparison pages.

- **Kind**: Derivation
- **Function**: `text::highlight::highlight(input) -> Vec<LineTokens>`

What the result must satisfy:

- The language is told by the name of the file, as below; any other file is plain text, with no colors.

  | Language | Files |
  | --- | --- |
  | TypeScript, TSX | `.ts`, `.mts`, `.cts`; `.tsx` |
  | JavaScript, JSX | `.js`, `.mjs`, `.cjs`; `.jsx` |
  | JSON, JSON with comments | `.json`; `.jsonc` |
  | CSS, SCSS, Less | `.css`; `.scss`; `.less` |
  | HTML, Vue, Svelte, XML | `.html`, `.htm`; `.vue`; `.svelte`; `.xml`, `.svg` |
  | Python, Ruby, PHP, Lua | `.py`; `.rb`; `.php`; `.lua` |
  | Java, Kotlin, Swift, Dart, C# | `.java`; `.kt`, `.kts`; `.swift`; `.dart`; `.cs` |
  | Go, Rust, C, C++ | `.go`; `.rs`; `.c`, `.h`; `.cpp`, `.cc`, `.cxx`, `.hpp`, `.hh` |
  | YAML, TOML, INI | `.yml`, `.yaml`; `.toml`; `.ini` |
  | Markdown | `.md`, `.markdown` |
  | Shell, PowerShell | `.sh`, `.bash`, `.zsh`; `.ps1`, `.psm1` |
  | SQL | `.sql` |
  | Dockerfile | `Dockerfile`, `*.dockerfile` |

- The text is tokenized as a whole, with the state of each line carried to the next, so that a line inside a block
  comment or a string over several lines is colored as one. The grammars to implement are the TextMate grammars of these
  languages (the published format their editors share), or tokenizers that give the same kinds.
- Each token has a kind among: comment, string, number, keyword, operator, punctuation, type, function, variable,
  property, constant, tag, attribute, and plain. The theme in use (dark, light, high contrast dark, high contrast light)
  gives each kind its color and style; changing the theme colors every open page again.
- Each side of a Diff page is colored as the whole text of its version, and only then cut into the rows of
  `diff.align`: a line is never colored out of its context.

Acceptance:

- [ ] In `main.rs`, `fn`, `"text"`, `// note` and `42` get the colors the theme gives keyword, string, comment and number.
- [ ] A line in the middle of a `/* … */` comment of 10 lines is colored as a comment, also when it is the first line
      of a change in a Diff page.
- [ ] A file named `Dockerfile` is colored as a Dockerfile; `notes.xyz` is plain text.
- [ ] Switching from the dark to the light theme recolors an open File page without reopening it.

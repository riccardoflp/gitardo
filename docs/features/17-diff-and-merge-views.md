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
- **Entry points**: a click on a file of the changes unless `changes.openOnClick` is `file`, and **Open changes** of
  the menu of its row; a file in the inspector, beside it (`diff.beside-inspector`); a file in the Commit page and in a
  comparison; **Compare with the saved file** in the Changes page; **Compare with…** on a file; the button on the
  header of the Changes page that opens all its diffs in one page; **Show the result** in the review of the
  conflicts; `conflicts.open-file` for a file deleted on one side; a deleted file of the Files page
  (`file-view.files-changes`).
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

### `diff.select-lines` — Select lines of a diff

Selects some lines of a Diff page, on either side, so that only those are staged, unstaged or reverted
([D78](../08-decisions.md#d78--stage-unstage-and-revert-the-lines-selected-in-a-diff-proposed)).

- **Kind**: View
- **Function**: `views::diff::select_lines_view(state: &LineSelectionState, ui) -> Vec<Intent>`
- **Entry points**: dragging over the lines or over their numbers on either side of the Diff page; `Shift` with a
  click, or with `↑`, `↓`, `Page Up` and `Page Down`, from the line chosen; text selected in the side that is edited.
- **Uses**: `diff.align`, `diff.edit`, `diff.stage-lines`, `diff.unstage-lines`, `diff.revert-lines`.
- **Extension**: Planned: the lines selected in either side of the Flow Diff, which is Monaco and has its own
  selection. In VS Code's diff editor, the selection is VS Code's own, for its **Stage Selected Ranges**.

- The selection is of rows, the rows `diff.align` lays out: from the row of the line where it starts to the row of the
  line where it ends, on whichever side it was made, the filler rows between them included. A line changed into
  another faces it on its row, so that selecting one of the two selects the change of that line whole.
- On the side that is edited, the rows are those of the text selected, a selection inside one line taking its row.
  On a side that is not edited, a click chooses a line, as it does now, and `Shift` with a click or an arrow selects
  the rows from it, without selecting text (Q1070).
- The rows selected are drawn with the background of a selection on both sides and in the gutter, over the colors of
  the changes. A click elsewhere, `Escape`, or a change of the diff that takes away a row of the selection, clears it.
- With some changed line among the rows selected, the context menu of either side has **Stage lines**, **Unstage
  lines** and **Revert lines**, each where its card offers it, and the keys of the page do them: `Alt+S`, `Alt+U` and
  `Alt+R` on Windows and Linux, `Ctrl+Option+S`, `Ctrl+Option+U` and `Ctrl+Option+R` on macOS (Q1071). With no changed
  line among them, the menu has the three items greyed out, and the keys do nothing.

Acceptance:

- [ ] Dragging over the numbers of lines 10 to 14 on the left side selects rows 10 to 14 on both sides, and the
      context menu has **Stage lines**.
- [ ] `Shift+↓` three times from line 20 of the side of `HEAD` selects four rows, and no text.
- [ ] Selecting the new text of a line changed into another selects the row where the two face each other.
- [ ] `Escape` clears the selection, and an edit that takes away a selected line clears it too.
- [ ] Rows selected with no changed line among them give the three items greyed out.

### `diff.lines-patch` — The patch of the lines selected

The patch that takes into a version of a file only the lines selected of the differences between it and another, for
`git apply --cached` on standard input
([D78](../08-decisions.md#d78--stage-unstage-and-revert-the-lines-selected-in-a-diff-proposed)).

- **Kind**: Derivation
- **Function**: `text::diff::lines_patch(input: &LinesPatchInput) -> Option<String>`
- **Uses**: `diff.line-diff`, `diff.align`, `diff.change-patch`.
- **Extension**: Planned, as `diff.stage-lines`.

Its input is the text the patch applies to (the index, to stage or to unstage), the text it takes lines from (the
working tree to stage, `HEAD` to unstage), the rows selected, and where each row is in each of the two texts.

- The differences between the two texts are found as `diff.change-patch` finds them. A line the other text adds is
  taken when its row is selected, and left out otherwise. A line the other text removes is removed when its row is
  selected, and stays otherwise, as a line that does not change.
- The patch has no line of context, as the patch of a change: each run of lines taken is a hunk of its own, so that a
  line that stays splits a run in two. The header of each hunk counts the lines it removes and adds, and its new start
  counts what the hunks before it added and removed.
- The rows of a page whose sides are not the two texts are placed in them as `diff.change-patch` places a change: to
  stage from a diff of the working tree against `HEAD`, by the lines of the working tree, against the index; to
  unstage from it, by the lines of `HEAD`, against the index (Q1072).
- The last line of a text with no line break after it is followed by `\ No newline at end of file`. A line taken after
  that last line needs a line break on it: the change of the last line is taken too, though its row was not selected.
- Lines added when staging take the line endings of the index, as `diff.change-patch` says; lines put back when
  unstaging keep theirs.
- For a file the index does not have (untracked), the patch creates it in the index with the lines taken alone:
  `new file mode 100644`, `100755` for a file git sees as executable, and `--- /dev/null`.
- None when no line taken differs: nothing selected changed.
- Its input gives the rows selected as the lines they have in the index and in the other text, two runs of lines; in a
  change, the n-th old line faces the n-th new one, as `diff.align` lays them out. The store places the rows of a side
  that is neither text in the index through the lines that do not change between the two
  ([Q1302](../08-decisions.md#open-questions)).

Acceptance:

- [x] Of a change of five added lines after line 12, the rows of the second and third give `@@ -12,0 +13,2 @@` and
      those two lines.
- [x] Of a change of lines 4 to 6 into new lines 4 to 6, the row of line 5 alone gives `@@ -5,1 +5,1 @@`, its old line
      and its new one.
- [x] Rows that cross two changes, with the unchanged lines between them, give two hunks, the second starting where the
      first left the index.
- [x] Of three removed lines, selecting the first and the third gives two hunks, and the second line stays: a selection
      is one run of rows, so the first and the third are two selections, each its own hunk, the second left in place.
- [ ] Lines added after a last line with no line break take its change too, and `git apply --cached --unidiff-zero`
      applies the patch.
- [x] Two of the five lines of an untracked file give a patch that creates the file in the index with those two
      lines.
- [x] Rows with no changed line give none.

### `diff.stage-lines` — Stage the lines selected

Puts into the index only the lines selected of the changes of the working tree
([D78](../08-decisions.md#d78--stage-unstage-and-revert-the-lines-selected-in-a-diff-proposed)).

- **Kind**: Operation
- **Function**: `features::diff::stage_lines(context, answers) -> Outcome`
- **Entry points**: **Stage lines** in the context menu of either side of the Diff page while lines are selected;
  `Alt+S` (`Ctrl+Option+S` on macOS) (`diff.select-lines`).
- **Extension**: Planned: **Stage lines** in the context menu of the Flow Diff, sending `diff.stage-lines` to the engine
  with the rows selected, as the stage button of a change sends `diff.stage-change`. In VS Code's diff editor, **Stage
  Selected Ranges** of VS Code's Git does it.
- **Offered when**: the right side is the working tree and the left is `HEAD` or the index, the file is not renamed
  (Q1073), and some line selected differs between the index and the working tree.
- **Reads**: the index and the working tree again, to check the lines are where the page says (`diff.read-sides`).
- **Plan**: `git apply --cached --unidiff-zero --whitespace=nowarn -` with the patch of `diff.lines-patch` on standard
  input. Titled **Stage `<n>` lines in `<file>`**; its detail, shown with the command in the preview, is *The patch on
  standard input changes these lines of `<file>` in the index:* and the `@@` line of each hunk.
- **Danger**: None.
- **Undo**: Impossible with **Undo**, since the index has no reflog; **Unstage lines** on the same lines takes it back.
- **After**: the page is computed again; the lines staged are drawn as `diff.staged-changes` draws what is staged, and
  the selection is cleared.
- **Refuses**: as `diff.stage-change`: when the file changed since the page was computed, *The file changed since the
  diff was shown. Try again.*; for a file that is not UTF-8 with a character above 127 in the patch, *Staging lines
  needs UTF-8 text: stage the whole file.*; for the first line of a file with a byte order mark, *The first line of a
  file with a byte order mark cannot be staged alone: stage the whole file.* When no line selected differs between the
  index and the working tree: *These lines are staged already.*
- **Settings**: `commandPreview`.
- **Uses**: `diff.select-lines`, `diff.lines-patch`, `diff.staged-changes`, `diff.read-sides`.

Acceptance:

- [ ] Of a change of five added lines, staging the second and the third leaves `git diff --cached` with those two
      lines alone, and the log has the `git apply` line exactly.
- [ ] Selecting rows across three changes, with the second staged already, stages what is selected of the first and
      the third.
- [ ] Of a line changed into another, staging its row puts the new line in the index in place of the old.
- [ ] Staging two lines of an untracked file adds the file to the index with those two lines; `git status` says it is
      added and changed.
- [ ] In a file with CRLF in the index, the lines staged have CRLF there.
- [ ] Editing the file in another program between the selection and **Stage lines** refuses, and stages nothing.
- [ ] With `commandPreview` set to `all`, the question shows the command and the patch before it runs.

### `diff.unstage-lines` — Unstage the lines selected

Takes only the lines selected back out of the index, the working tree unchanged
([D78](../08-decisions.md#d78--stage-unstage-and-revert-the-lines-selected-in-a-diff-proposed)).

- **Kind**: Operation
- **Function**: `features::diff::unstage_lines(context, answers) -> Outcome`
- **Entry points**: **Unstage lines** in the context menu of either side of the Diff page while lines are selected;
  `Alt+U` (`Ctrl+Option+U` on macOS) (`diff.select-lines`).
- **Extension**: Planned: **Unstage lines** in the context menu of the Flow Diff, as `diff.stage-lines`. In VS Code's
  diff editor, **Unstage Selected Ranges** of VS Code's Git does it.
- **Offered when**: the page shows the index against `HEAD`, or the working tree against `HEAD`; the file is not
  renamed; and some line selected differs between `HEAD` and the index.
- **Reads**: the index and `HEAD` again, to check the lines are where the page says.
- **Plan**: `git apply --cached --unidiff-zero --whitespace=nowarn -` with the patch of `diff.lines-patch` from the
  index towards `HEAD` on standard input, titled **Unstage `<n>` lines in `<file>`**. A file `HEAD` does not have, all
  of whose lines are selected, is unstaged whole, as `diff.unstage-change` does: `git restore --staged -- <file>`, or
  `git rm --cached -r -- <file>` before the first commit.
- **Danger**: None.
- **Undo**: Impossible with **Undo**; **Stage lines** on the same lines takes it back.
- **After**: the page is computed again, and the selection is cleared.
- **Refuses**: as `diff.stage-lines`; when no line selected differs between `HEAD` and the index: *Nothing of these
  lines is staged.*
- **Uses**: `diff.select-lines`, `diff.lines-patch`, `diff.unstage-change`.

Acceptance:

- [ ] Of five lines staged, unstaging the first two leaves the other three staged, and the working tree unchanged.
- [ ] In the diff of the working tree against `HEAD`, unstaging the rows of a staged change drawn dimmed takes them out
      of the index, and they are drawn as not staged.
- [ ] All the lines of a file added to the index and not in `HEAD` unstage with `git restore --staged -- <file>`.

### `diff.revert-lines` — Revert the lines selected

Puts back in the working tree what the left side has in place of the lines selected
([D78](../08-decisions.md#d78--stage-unstage-and-revert-the-lines-selected-in-a-diff-proposed)).

- **Kind**: Operation
- **Function**: `features::diff::revert_lines(context, answers) -> Outcome`
- **Entry points**: **Revert lines** in the context menu of either side of the Diff page while lines are selected;
  `Alt+R` (`Ctrl+Option+R` on macOS) (`diff.select-lines`).
- **Extension**: Planned: **Revert lines** in the context menu of the Flow Diff, which edits VS Code's document and
  saves it, as its button that reverts a change does ([A5](../transition/04-differences.md#a5-revert-one-change-in-a-diff)).
  In VS Code's diff editor, **Revert Selected Ranges** of VS Code's Git does it.
- **Offered when**: the right side is the working tree (against `HEAD`, the index or a commit), and some line selected
  differs between the two sides.
- **Reads**: the text of the side of the working tree, to check the lines are still there.
- **Plan**: in the side of the working tree, the lines of the rows selected are replaced by the lines the left side has
  on those rows, as one edit of that side, and the file is saved: `write <file>` (`file-view.save`), as
  `diff.revert-change` does with a change (Q1074).
- **Danger**: None.
- **Undo**: `Ctrl+Z` in the side of the working tree takes the edit back, not saved until `Ctrl+S`; **Undo** does not.
- **After**: the page is computed again; the lines reverted are no longer changed, and the selection is cleared.
- **Refuses**: *The file changed since the diff was shown. Try again.*
- **Uses**: `diff.select-lines`, `diff.align`, `diff.edit`, `file-view.save`, `diff.revert-change`.

Acceptance:

- [ ] Of a change of three lines changed into three others, reverting the row of the second gives the file its old
      second line, the other two left as they were.
- [ ] Reverting rows that cross two changes reverts what is selected of both, in one edit that one `Ctrl+Z` takes
      back.
- [ ] Reverting the rows of lines removed puts them back after the line before them.
- [ ] Against a commit, the lines reverted are those of the commit.

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

### `diff.image` — Two versions of an image side by side

The two versions of an image in the Diff page, drawn side by side, so that what changed in a picture is seen and not
refused ([D74](../08-decisions.md#d74--images-are-drawn-where-their-files-are-shown)).

- **Kind**: View
- **Function**: `views::diff::image_view(state, ui) -> Vec<Intent>`
- **Entry points**: every entry point of `diff.page`, for a file that is an image on at least one side.
- **Reads**: `diff.read-sides`, each side read as an image by `file-view.read-image`.
- **Refuses**: a file that is not an image on either side and has a NUL byte, as `diff.page` does: *This file looks
  binary.*
- **Uses**: `diff.page`, `file-view.read-image`, `file-view.image`, `diff.stage-file`, `diff.unstage-file`,
  `diff.discard-file`, `diff.open-file`, `diff.navigate`.
- **Extension**: VS Code's diff editor with its preview of images on both sides, the sides given as bytes by the
  `FileSystemProvider` of the scheme `gitardo`.

Rules:

- A file whose two sides are images, or whose one side is an image and the other has no file, shows the two pictures
  in place of the two columns of text, each named over its column as `diff.page` names it, **Empty** on a side with no
  file.
- The two are drawn at one scale, the largest at which both fit their columns and neither is larger than it is, so
  that a picture that grew looks larger than the other. Each is on the ground of squares of `file-view.image`, centered
  in its column, with its size in pixels, its format and its weight under it.
- Over the two, one line says what changed: `640 × 480 → 1280 × 960 · 24 KB → 81 KB`, or *Only the bytes of the file
  changed; the pixels are the same.* when both decode to the same pixels.
- A side that is an image and cannot be read says why in its column, as `file-view.image` does; the other side is still
  drawn.
- An SVG has the title buttons **Image** and **Text**, the one shown filled: **Image** draws its two pictures, **Text**
  shows the diff of its lines as `diff.page` does, with everything that page has. The page opens on **Image**
  ([Q845](../08-decisions.md#open-questions)); the switch is kept while the page is open.
- The title buttons of the file stay: **Open file**, **Stage file**, **Unstage file**, **Discard changes**. The
  buttons of one change are not there, an image being one change; `F7` and `Shift+F7` count it as one, so that in a
  page of several files they stop on it once.
- The page is drawn again when either side changes, as `diff.page` is: a picture of the working tree saved by another
  program shows its new pixels.
- In a page of several files an image takes its place among them, its height that of its pictures, between 8 and 26
  rows of the page, the pictures fit to the rows. A file whose two sides have the same bytes, as against the index
  once it is staged, says *No changes.* over them and is no change for `F7`.
- Extension: a diff of an image opened from Gitardo (the Gitardo Changes view, the graph, the page of a commit, a
  comparison) opens VS Code's diff editor, which draws the two pictures with its preview of images; the side of a
  revision, of the index or of a conflict is read as bytes from the engine. With the Flow Diff on, an image opens
  VS Code's diff editor all the same ([Q847](../08-decisions.md#open-questions)).

Acceptance:

- [x] A PNG changed in the working tree opens the Diff page with `HEAD` on the left and the working tree on the right,
      both drawn, and the line over them says their sizes.
- [x] A PNG of 100 × 100 that became 200 × 200 is drawn half as wide on the left as on the right.
- [x] An added PNG shows **Empty** on the left and the picture on the right.
- [x] A PNG saved again with the same pixels says *Only the bytes of the file changed; the pixels are the same.*
- [x] An SVG opens on **Image** with its two pictures; **Text** shows the diff of its lines, and the right side of the
      working tree can be edited there.
- [x] **Stage file** on an image of the working tree stages it, and the page against the index has no change left.
- [x] A file with a NUL byte that is not an image still refuses with *This file looks binary.*
- [x] In a page of a text file and an image, `F7` goes from the last change of the text to the image, and on to the
      next file.
- [x] Extension: opening a changed PNG from the Gitardo Changes view shows VS Code's diff editor with the two pictures,
      not their bytes as text.

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
  of the changes when `changes.openOnClick` is `file`, and **Open file** of the menu of its row; **Open file** of a Diff
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

### `file-view.files-changes` — What differs from the last commit, in the tree of the Files page

Each file of the tree of the Files page that differs from `HEAD` is marked as the Changes page marks it, and each folder
holding one has a dot, so that the files changed are seen where they are in the working tree
([D61](../08-decisions.md#d61--the-tree-of-the-files-page-shows-what-differs-from-the-last-commit)).

- **Kind**: Derivation
- **Function**: `model::changes::tree_marks(list: &ChangesList) -> TreeMarks`, from the list of the Changes page,
  drawn by the rows of the tree (`views::files`)
- **Entry points**: the tree of the Files page.
- **Uses**: `changes.list`, `changes.open`, `details.file-tree`, `file-view.beside-files`.
- **Extension**: the Explorer of VS Code marks the changes of git itself, and the extension has no Files view: nothing
  to build.

Rules:

- A file that differs from `HEAD` has its name in the color of its state and the letter of the Changes page at the
  right end of its row, in the same color: `A` added, `M` modified, `D` deleted, `R` renamed, `C` copied, `T` type
  changed, `?` untracked, `!` in conflict. Its staged and unstaged changes are taken together, as the Changes page
  takes them (`changes.list`); a file in conflict is marked so, whatever else it is. A file as committed is drawn as
  before.
- The screen reader hears the state after the name: `app.rs, modified`, `notes.txt, untracked`, `b.rs, renamed from
  src/a.rs`.
- A renamed or copied file whose rename is staged is at its new path; its tooltip says `Renamed from src/a.rs`. The old
  path is not listed.
- A deleted file is listed at its place in the tree, whether its deletion is staged or not, dimmed, its name struck
  through. A click on it, `Enter` or **Open file** opens its Diff page as **Open changes** of the Changes page does
  (`changes.open`), there being no file on disk for a File page.
- A folder holding a file that differs, at any depth, has a dot at the right end of its row, in the color of the most
  important state under it: a conflict, then a deletion, then a modification (a rename, a copy or a change of type
  counts as one), then a new file, added or untracked. A folded folder shows it too; the screen reader hears
  `src, has changes`.
- The marks come from the read of the repository the Changes page lists from, with no other git command, and follow
  every read of it: a file changed on disk is marked once it is read again, a file put back loses its mark. Nothing is
  folded or unfolded by them.
- Ignored files stay unlisted. A file only an editor of Gitardo has changed is not marked
  ([Q600](../08-decisions.md#open-questions)).

Acceptance:

- [x] `src/app.rs` changed on disk shows `M` in the color of a modification, and the screen reader hears
      `app.rs, modified`; `README.md`, as committed, is said by its name alone.
- [x] `notes.txt`, not tracked, shows `?`; `todo.txt`, new and staged, shows `A`.
- [x] `src/gone.rs` deleted from disk and `src/old.rs` whose deletion is staged are both in the tree, after
      `src/app.rs` in the order of their names, struck through with `D`.
- [x] A click on `README.md`, whose deletion is staged, opens its Diff page, not a File page.
- [x] `src/app.rs` renamed to `src/main.rs` and staged shows `main.rs` with `R`, heard as
      `main.rs, renamed from src/app.rs`, its tooltip naming `src/app.rs`; `app.rs` is not listed.
- [x] `conflict.txt` in conflict during a merge shows `!`, heard as `conflict.txt, in conflict`.
- [x] The folder `src`, folded, holding a modified `src/ui/list.rs` and a new `src/new.rs`, shows the dot of a
      modification; once `src/app.rs` is deleted and the repository read again, the dot of a deletion, `src/ui` keeping
      that of a modification, and `src` stays folded.
- [x] Changing `src/app.rs` on disk, with `src` open, marks it `M` once the repository is read again, and putting it
      back takes the mark away; `src` stays open.

### `file-view.file-icons` — The icon of a file or a folder

Beside the name of each file and folder of the repository, the icon of its kind, from the icon theme of Symbols, in the
colors of the theme of the window
([D62](../08-decisions.md#d62--the-icons-of-files-and-folders-are-those-of-symbols-in-the-colors-of-the-theme)).

- **Kind**: View
- **Function**: `render::Canvas::draw_file_icon(icon: FileIcon, origin: Point, size: f32, look: &FileIconLook)`, the
  icon chosen by `render::FileIcons::icon_of(asked: FileOrFolder) -> FileIcon`
- **Entry points**: the tree of the Files page; the rows of the Changes page; the files of a commit in the inspector, the
  Commit page and a stash; the files of a comparison, beside it and on its page; the files and folders of the page of
  new worktrees.
- **Settings**: `fileIcons` (`symbols`).
- **Uses**: `graph.icons`, `file-view.files-changes`.
- **Extension**: its views show the file icon theme of VS Code, Seti when none is installed, and Symbols once the user
  installs it there: nothing to build.

Rules:

- The icons are those of [Symbols](https://github.com/miguelsolorio/vscode-symbols), by Miguel Solorio, under the MIT
  licence, kept in `rust/crates/render/icons/symbols/` with the licence and the commit they were copied from, and
  chosen as its theme chooses them in VS Code: a file by its whole name (`package.json`, `Dockerfile`), then by its
  extensions from the longest (`index.d.ts` before `.ts`), then by its language as VS Code names it from its name
  (`.php`, `Makefile`), else the icon of a document; a folder by its name (`src`, `.github`), else the plain folder; the
  folder of the repository, at the top of the tree, the gray folder. Names are compared whatever their case. A chain of
  folders shown as one row, `src/git`, has the icon of its last folder. The theme of Symbols has no icons of open
  folders: an open folder keeps its icon, its arrow saying it is open.
- The colors of the icons come from the theme: each color of an icon belongs to a family of hues (gray, red, orange,
  yellow, green, teal, sky, blue, violet, pink), and is drawn in the color the theme gives that family, chosen to read on
  its ground, the strong colors in the high contrast themes; the gray of the folders is the muted text. A shade lighter
  or darker than the family's own in the set is drawn as much lighter or darker, so that the parts of an icon stay
  apart. A switch of theme draws the icons again in its colors.
- `fileIcons` says how they are drawn: `symbols`, in the colors of the theme; `monochrome`, every icon in the color of
  the text beside it, as the other icons of the window, its shades kept; `simple`, the plain icons of a file and a
  folder of Lucide, as before the icons of Symbols, and none beside the folders of a tree.
- An icon is 16 logical pixels, as the other icons of the rows, at the place the icon of a file had. In the tree of the
  Files page a folder has its icon after its arrow, and a file the place of an arrow before its icon, so that the names
  under a folder line up; in the trees of the files of a commit, of the Changes page and of a comparison, a folder has
  its icon between its arrow and its name.
- The marks of the Files page (`file-view.files-changes`) are unchanged: the color of the name, its letter, the dot of a
  folder; a deleted file, dimmed, has its icon dimmed with it.
- An icon is drawn into pixels by `resvg` once for its size, the scale of the screen and the colors of the theme, and
  kept; a row drawn again draws nothing new.
- The screen reader hears the name of the row, not its icon, which is decoration.

Acceptance:

- [x] `src/main.rs` has the icon of Rust, `package.json` that of Node by its name, `Dockerfile` that of Docker by its
      name, `index.d.ts` that of the types of TypeScript by its longer extension, `INDEX.PHP` that of PHP by its
      language, and `notes.xyz` and `notes` that of a document.
- [x] The folder `src` has `folder-orange-code`, `SRC` too, `node_modules`, `.github` and `docs` their own, `notes` the
      plain folder, and the top of the tree the gray folder.
- [x] Every entry of the theme names an icon the program has, and every icon draws inside its square at 16 and 32
      pixels.
- [x] Each family of hues reads on the ground of each theme, at least 3:1, 4.5:1 in the high contrast themes; the gray
      is the muted text.
- [x] A point of the icon of Rust is drawn in the orange of the dark, light, high contrast dark and high contrast light
      themes.
- [x] With `monochrome`, the icon of Rust is drawn in the color of the text; two shades of a family stay apart.
- [x] With `simple`, the tree of the Files page with its marks is drawn as it was before the icons of Symbols.
- [x] An icon drawn in a second row is not drawn into pixels again; at another scale, or in another theme, it is, once.
- [x] The Files page of a repository with Rust, TypeScript, CSS, a Dockerfile, a README, `package.json` and the folders
      `.github`, `assets`, `docs`, `src` and `tests` matches its snapshots in the four themes, and in `monochrome` in the
      dark theme.
- [x] The screen reader hears `main.rs` and `src`, and nothing of their icons.
- [x] `fileIcons` set to `monochrome` or `simple` is taken; a value it does not know is the default, and said.

### `file-view.markdown-preview` — A Markdown file as it reads

A Markdown file in the File page shown laid out as it reads, with **Preview** and **Source** to switch
([D59](../08-decisions.md#d59--markdown-files-shown-as-they-read)).

- **Kind**: View
- **Function**: `views::file_view::markdown_view(state: &MarkdownState, ui) -> Vec<Intent>`, over the derivation
  `model::markdown::blocks(text: &str) -> Vec<Block>`
- **Entry points**: **Preview** and **Source** in the title row of the File page of a file ending in `.md` or
  `.markdown`; any way of opening such a file while `markdown.openAsPreview` is on.
- **Settings**: `markdown.openAsPreview` (`false`).
- **Uses**: `file-view.page`, `file-view.highlight`, `file-view.mermaid`.

Rules:

- CommonMark with the tables, the task lists and the strikethrough of GitHub. Headings, paragraphs, emphasis, inline
  code, lists, quotes, rules, code blocks in the colors of the code of their language, tables with their alignment.
  The window has no italic typeface: emphasis is drawn in the medium weight, strong text in the bold one
  ([Q580](../08-decisions.md#open-questions)). A word wider than the page is cut where it overflows; a line of a code
  block wider than it ends with `…`. A block fenced as `mermaid` is drawn as its diagram (`file-view.mermaid`).
- HTML in the file is shown as text, never rendered; nothing in the preview runs.
- A link to a web page opens in the browser; a link to a file of the repository, relative to the file, opens that file
  in its File page, at the revision of the page when it shows one, at the line of a `#L12` after it; a link to a
  heading of the file scrolls to it, the heading named as GitHub names it (`#getting-started`). Another scheme, such as
  `javascript:`, and a path out of the repository go nowhere. An image of the repository is drawn, as wide as the page
  at most, read on a worker by `file-view.read-image`, so that an SVG of the repository is drawn as a PNG is, one pixel
  of it to a pixel of the page; an image it cannot read shows its text, as an image from the web, which is not fetched
  ([Q581](../08-decisions.md#open-questions), answered by D74).
- The preview follows the text: edits made in **Source** show in **Preview** when it is shown again; a file of the
  working tree changed on disk is read again, as in **Source**.
- With `markdown.openAsPreview` on, a Markdown file opens in **Preview**; otherwise in **Source**. The switch is kept
  for the page while it is open. A file at a revision has the preview too, read-only as its text.
- **Find** in the preview, `Ctrl+F`, finds in the text shown, as find in page does
  ([Q582](../08-decisions.md#open-questions)).
- Extension: with `gitardo.markdown.openAsPreview` on, a Markdown file that Gitardo opens from the Gitardo Changes view
  (**Open File**, or a click on it) opens in VS Code's preview of Markdown (`markdown.showPreview`), not in its editor;
  its diffs stay diffs of its text ([Q584](../08-decisions.md#open-questions)).

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
- [x] Extension: with `gitardo.markdown.openAsPreview` on, a click on `README.md` in the Gitardo Changes view opens
      VS Code's preview of it; `src/app.rs` opens in its editor.

### `file-view.mermaid` — A Mermaid diagram as it is drawn

A block of code fenced as `mermaid` in the preview of a Markdown file, drawn as its diagram
([D60](../08-decisions.md#d60--mermaid-diagrams-drawn-in-the-preview-of-markdown)).

- **Kind**: Derivation
- **Function**: `text::mermaid::diagram(source: &str, look: &Look) -> Result<Diagram, NotDrawn>`, the SVG of the
  diagram, drawn into pixels by `render::Image::from_svg`
- **Entry points**: the preview of a Markdown file (`file-view.markdown-preview`) with a block fenced as `mermaid`.
- **Uses**: `file-view.markdown-preview`.
- **Extension**: VS Code's preview of Markdown, through a plugin of its Markdown, with the SVG of
  `read/file-view.mermaid`.

Rules:

- A block of code whose fence names `mermaid`, in any case, is drawn as its diagram, laid out by the library `merman`,
  which reads Mermaid as Mermaid 12 does: flowcharts, sequence, class, state and entity relationship diagrams, Gantt
  charts, pies, Git graphs, mind maps and the other kinds it draws ([Q590](../08-decisions.md#open-questions)). The
  diagram takes the place of the block, as wide as it is, or as the page when it is wider, its height scaled with it.
- Its colors are the window's, in the dark, light and both high contrast themes: the ground of the page, the color of
  text, lines in the muted color, the borders of its shapes in the accent. Its labels are in Inter, measured with it so
  that they fit their shapes. A switch of theme draws it again in the new one.
- It is laid out and drawn on a worker, as an image of the preview is read; until it is, its code shows. A diagram
  drawn is kept for its text and its theme while its page is open, so that scrolling, laying the preview out again or
  coming back to a theme does not draw it again; an edit of its text draws the new text.
- A diagram with an error, or of a kind the library does not draw, shows its block of code as any other, with one line
  under it saying why: *This diagram has an error: \<what the library says\>*, *Gitardo does not draw this kind of
  diagram.*, *This diagram is too large to draw.*
- Nothing in a diagram is followed, run or fetched: a link or a `click` of it does nothing, a label written in HTML is
  drawn as its text, and an image or a font it names is not read.
- The screen reader reads its title and its description, `accTitle` and `accDescr`, or *Mermaid diagram* when it has
  none.
- **Source** shows the text of the file, the block as it is written. **Find** in the preview finds no text in a
  diagram, which is a picture ([Q591](../08-decisions.md#open-questions)).
- Extension: VS Code's preview of Markdown, which does not draw Mermaid, draws a block fenced as `mermaid` as the
  diagram the engine drew for the kind of VS Code's theme (dark, light, high contrast dark or light), in the colors of
  VS Code's default theme of that kind, shown as an image so that nothing in it runs
  ([Q592](../08-decisions.md#open-questions)). A switch of theme draws it again. Until the engine answers, and without the
  engine, the block shows its code; a diagram with an error shows its code and the line saying why.

Acceptance:

- [x] A flowchart and a sequence diagram are drawn as their diagrams, with their labels.
- [x] The same diagram is drawn on the ground of the page in the dark, light, high contrast dark and high contrast light
      themes, its text in the color of text of each.
- [x] `flowchart LR` with `A[Start --> B` shows its code and *This diagram has an error:* with what is wrong;
      `notADiagram` shows its code and *Gitardo does not draw this kind of diagram.*
- [x] A diagram with `click A "https://example.com"` and a label with `<img src=…>` and `<script>` is drawn with no
      link to click, no image read and no script, its label as text.
- [x] A diagram wider than the page is drawn as wide as the page, its height scaled with it.
- [x] A diagram is drawn on a worker once for its text and its theme: laying the preview out again starts no drawing,
      a switch of theme starts one.
- [x] A diagram with `accTitle: How a commit is made` is read by the screen reader as that title; one without as
      *Mermaid diagram*.
- [x] **Source** shows the block of the diagram as its text.
- [x] Extension: with a `mermaid` block, VS Code's preview of Markdown shows the diagram the engine drew for the theme
      of VS Code; without the engine it shows the code.

### `file-view.read-image` — The pixels of an image file

Reads the bytes of a file as an image, whatever its format, for every place that draws one
([D74](../08-decisions.md#d74--images-are-drawn-where-their-files-are-shown)).

- **Kind**: Derivation
- **Function**: `render::image::read_image(bytes: &[u8], path: &str, scale: f32) -> Result<ImageRead, NotRead>`
- **Entry points**: the File page (`file-view.image`), the Diff page (`diff.image`), the page of a commit and a
  comparison (`commit-page.images`), the preview of Markdown (`file-view.markdown-preview`).
- **Extension**: VS Code reads its images in its preview; the engine gives it their bytes.

Rules:

- **Input**: the bytes of the file, its path, and the scale of the screen. **Output**: its pixels, its size in pixels,
  its format and its weight; or why it is not read.
- The format is told by the first bytes: PNG, JPEG, GIF, WebP, BMP and ICO each start with bytes of their own. A file
  that starts with none of them is an SVG when its name ends in `.svg`, in any case; otherwise it is not an image, and
  none of the reasons below is given.
- PNG is read by the library `tiny-skia` reads it with, SVG drawn by `resvg` as `file-view.mermaid` draws a diagram:
  at the scale of the screen so that it stays sharp, with the fonts of the program, nothing outside the file read,
  neither an image it names nor a font of the system. JPEG, GIF, WebP, BMP and ICO are read by one library chosen as
  D22 says: `image`, with only those five formats. A GIF, and an animated PNG or WebP, gives its first frame
  ([Q841](../08-decisions.md#open-questions)); an ICO its largest image.
- An image with more than 64 million pixels is not read: *This image is too large to draw.* Bytes the format cannot
  read: *This image could not be read.* An SVG with an error: *This SVG could not be drawn: \<what the library
  says\>*.
- It runs on a worker; the result is kept for the bytes and the scale while a page that drew it is open.
- The preview of Markdown draws an image of the repository through it, so that an SVG, a JPEG or a GIF named there is
  drawn as a PNG is.

Acceptance:

- [x] The bytes of a PNG of 32 × 16 give 32 × 16 pixels, the format PNG and its weight.
- [x] The same PNG named `logo.dat` is read as a PNG.
- [x] An SVG with `width="10" height="20"` at a scale of 2 gives 20 × 40 pixels and a size of 10 × 20.
- [x] An SVG that names `<image href="secret.png">` and a font not of the program is drawn without either, and reads no
      file.
- [x] A JPEG, a GIF, a WebP, a BMP and an ICO of the tests are each read with their size; an animated GIF gives its
      first frame.
- [x] A PNG cut short gives *This image could not be read.*; a PNG of 10,000 × 10,000 gives *This image is too large
      to draw.*
- [x] `notes.txt` is not an image; `<svg` cut short in `a.svg` gives *This SVG could not be drawn:* with what is wrong.
- [x] `![logo](media/logo.svg)` in a Markdown file is drawn in its preview.

### `file-view.image` — An image as it is drawn

An image file in the File page, drawn as a picture in place of its bytes
([D74](../08-decisions.md#d74--images-are-drawn-where-their-files-are-shown)).

- **Kind**: View
- **Function**: `views::file_view::image_view(state, ui) -> Vec<Intent>`
- **Entry points**: every entry point of `file-view.page`, for a file that `file-view.read-image` reads as an image:
  a click on it in the Files page, **Open file** in a Diff page, **Open the file at this revision** on a file of a
  commit.
- **Reads**: the file from the disk; at a revision, `file-view.read-revision`.
- **Uses**: `file-view.page`, `file-view.read-image`, `file-view.read-revision`, `file-view.edit`, `file-view.save`,
  `file-view.open-external`.
- **Extension**: VS Code's preview of images, the file at a revision given as bytes by a `FileSystemProvider` of the
  scheme `gitardo`.

Rules:

- The picture is drawn centered in the page, as large as it is, smaller to fit the page, never larger until it is
  zoomed. Behind it a pattern of squares in two greys close to the ground of the page shows what is transparent
  ([Q843](../08-decisions.md#open-questions)).
- Under it, one line: its size in pixels, its format, its weight, and the zoom when it is not the fit:
  `1280 × 720 · PNG · 84 KB · 200%`.
- `Ctrl` with the wheel, `Ctrl+=` and `Ctrl+-` zoom in steps (25%, 50%, 100%, 200%, 400%, 800%) about the pointer,
  and `Ctrl+0` goes back to the fit (`Cmd` on macOS). Larger than the page, it is moved by dragging and by the
  wheel, across with `Shift` ([Q1300](../08-decisions.md#open-questions)). Above 100% its pixels stay square and
  sharp, not smoothed, so that a small icon can be looked at.
- An SVG has the title buttons **Preview** and **Source**, the one shown filled, as a Markdown file has: **Preview**
  is the picture, **Source** its text in the editor, which can be edited and saved; **Preview** draws the text as it
  is now. It opens in **Preview** ([Q840](../08-decisions.md#open-questions)), and the switch is kept for the page
  while it is open.
- A file that is not an SVG has no **Source**: its bytes are not text. It cannot be edited; **Open in external editor**
  opens it in the program of the system.
- A file of the working tree changed on disk is read and drawn again; the zoom stays.
- An image that cannot be read shows, in place of the picture, the line `file-view.read-image` gives, *This image
  could not be read.* or *This image is too large to draw.*; an SVG with an error shows that line over **Source**.
- A file at a revision is drawn the same way, read-only; it is titled as `file-view.page` titles it.
- The screen reader reads *Image*, its name, its size and its format.
- Extension: VS Code opens an image of the working tree in its preview of images. An image at a revision, opened from
  Gitardo (**Open the file at this revision**, the history of a file), opens there too, its bytes read by the engine;
  before, it opened as its bytes in a text editor.

Acceptance:

- [x] Clicking `media/logo.png` in the Files page opens a page **logo.png** with the picture and
      `512 × 512 · PNG · 18 KB` under it.
- [x] A picture of 4000 × 3000 in a page of 800 pixels is drawn 800 wide; one of 16 × 16 is drawn 16 wide.
- [x] `Ctrl+=` twice from 100% draws it at 400%, its pixels square; `Ctrl+0` brings back the fit.
- [x] `icons/check.svg` opens in **Preview** as its picture; **Source** shows its text, and an edit of its `fill` shows
      in **Preview**.
- [x] Saving the PNG from another program draws the new picture.
- [x] A PNG cut short says *This image could not be read.* and nothing else.
- [x] `media/logo.png` at `3f2a1c9` opens a read-only page **logo.png (3f2a1c9)** with the picture of that commit.
- [x] Extension: **Open the file at this revision** on a PNG opens VS Code's preview of images with the picture of
      the commit.

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

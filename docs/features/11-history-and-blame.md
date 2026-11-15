# History and blame

Where a file, some lines or a folder come from, and who wrote what: the timeline narrowed to the history of a file, of
lines or of a folder; the blame of the line under the cursor and of every line of a file in the File page; the reflog,
to get back to where `HEAD` has been; and bisect, to find the commit that broke something.

### `file-history.read-names` — The names a file had

Every name a file had in the history of what is checked out, so that its history goes on past the commits that renamed
it.

- **Kind**: Read
- **Function**: `git::read::file_history::read_names(git, request) -> Result<Vec<String>, GitError>`
- **Entry points**: none: used by `file-history.file`.
- **Reads**: `git log --follow -M --name-status -z --format= -- :(literal)<file>`.

Rules:

- The result starts with the name given. Then, going back through the changes git lists, newest first, each change adds
  the name of the file in it, and a rename (`R`) adds its old name too. Each name is listed once.
- The first change that is an addition (`A`) or a copy (`C`) ends the list, and adds nothing: `--follow` would go on into
  the file this one was copied from, or only resembled when it was added, which is another file with its own history.

Acceptance:

- [ ] `src/b.rs`, renamed from `src/a.rs`, which was added as such, gives `src/b.rs`, `src/a.rs`.
- [ ] A file that began as a copy of `x.rs` gives its own names only, not `x.rs`.
- [ ] A file never renamed gives its name alone.
- [ ] A name with `[` in it is passed as `:(literal)<name>` and matches only that file.

### `file-history.file` — Show the history of a file

Narrows the timeline to the commits that changed a file, under every name it had. The graph stays connected, so it
still shows on which branches the file changed.

- **Kind**: Operation
- **Function**: `features::file_history::file(context, answers) -> Outcome`
- **Entry points**: **Show file history** in the context menu of a file in the Files page, of the File page and of its
  title; the clock button next to each file in the inspector; **History of the active file** in the **…** menu of the
  header; **Show file history** in Quick search, for the File page in front.
- **Offered when**: the file is inside a repository of the workspace. In a workspace with several, the repository that
  holds the file (the innermost one) is shown first.
- **Reads**: `file-history.read-names`.
- **Plan**: no step: nothing in the repository changes. The timeline is read again with the names as its paths:
  `git log … --parents … -- :(literal)<name> …` (`--parents` makes each commit's parents the nearest commits listed, so
  the lines stay connected), on the branches the timeline shows.
- **Danger**: None.
- **Undo**: nothing to take back; **Show all commits** on the bar removes the narrowing.
- **After**: the timeline is shown; the bar above its commits says *History of `<path>`*, and, dimmed, *· earlier
  `<old names>`*, joined by `, `, when it had others.
- **Refuses**: a file outside every repository of the workspace, *Open a file of this repository to see its history.*
- **Uses**: `file-history.read-names`.

Acceptance:

- [ ] The history of `src/b.rs`, renamed from `src/a.rs`, lists the commits that changed either name, and the bar says
      *History of src/b.rs · earlier src/a.rs*.
- [ ] The history is limited to the branches the timeline shows when it is narrowed to some.
- [ ] A commit that changed only other files is not listed.
- [ ] If an earlier name was later used for another file, the commits of that file are listed too.

### `file-history.read-committed-lines` — The lines as the commit checked out has them

Before following lines back, they are found in the commit checked out, since git follows them from a commit, while the
file in the File page may have edits not saved or not committed.

- **Kind**: Read
- **Function**: `git::read::file_history::read_committed_lines(git, request) -> Result<CommittedLines, GitError>`
- **Entry points**: none: used by `file-history.lines`.
- **Reads**: `git rev-parse HEAD`; `git cat-file -e <commit>:<file>`; `git show <commit>:<file>`.

Rules:

- With no commit checked out, or none with that file: the file is in no commit.
- Otherwise the lines are mapped by `file-history.map-lines` from the text in the File page to the text of the commit:
  a range of lines of that commit, with the commit, or none (all lines were added since).
- A file renamed in the index is looked for under its old name, which is the name the commit has.

Acceptance:

- [ ] For a file as committed, lines 4–9 give lines 4–9 of `HEAD`.
- [ ] For a file added and not committed, the file is in no commit.
- [ ] For `new.rs` staged as a rename of `old.rs`, the lines are looked for in `old.rs`.

### `file-history.map-lines` — Map lines of a file as it is to the file as committed

Finds which lines of the file as committed some lines of the file as it is now come from.

- **Kind**: Derivation
- **Function**: `model::file_history::map_lines(input) -> Option<LineRange>`

Rules:

- Input: the committed text, the current text, a range of lines of the current text (from 1, both ends included).
  Output: a range of lines of the committed text, or none.
- Lines are split on `\n` or `\r\n`; the empty piece after a final line break is not a line.
- The two texts are compared line by line with a minimal line diff. Between two unchanged parts, a run of changes takes
  some lines out of the committed text and puts some in the current one.
- A selected line that is unchanged stands for the same line of the committed text.
- A selected line put in by a run that also took lines out stands for all the lines that run took out: the line it was
  before it was edited.
- A selected line put in by a run that took nothing out stands for nothing: it was added since.
- The result goes from the smallest committed line any selected line stands for to the largest; with none, nothing.

Example (committed `a b c d e`, current `a B c x d e`, `B` an edit of `b`, `x` added):

- current 2 (`B`) gives 2–2; current 4 (`x`) gives nothing; current 3–5 (`c x d`) gives 3–4; current 1–6 gives 1–5.

Acceptance:

- [ ] Each example above holds.
- [ ] Texts with `\r\n` line endings give the same results as with `\n`.
- [ ] Identical texts give the same range back.

### `file-history.lines` — Show the history of some lines

Narrows the timeline to the commits that changed the lines selected in the File page, or the line of the cursor (`git
log -L`). Git follows the lines back from the commit checked out, through moves too.

- **Kind**: Operation
- **Function**: `features::file_history::lines(context, answers) -> Outcome`
- **Entry points**: **Show history of selected lines** in the context menu of the File page and in Quick search.
- **Offered when**: the File page in front shows a file on disk inside a repository.
- **Reads**: `file-history.read-committed-lines`, with the text of the File page as it is, saved or not.
- **Plan**: no step: nothing in the repository changes. The timeline is read with
  `git log … --parents --no-patch -L<start>,<end>:<path> <commit>`, where the range and the path are those of the
  commit checked out. It follows the lines from that commit only, with no pathspec, whatever branches the timeline shows
  otherwise; what git prints of the changes is not read.
- **Danger**: None.
- **Undo**: nothing to take back; **Show all commits** on the bar removes the narrowing.
- **After**: the timeline is shown; the bar says *History of line 4 of `<path>`* or *History of lines 4–9 of
  `<path>`*; when the lines of the commit are not the ones selected, it adds, dimmed, *· lines 5–10 in the editor*. The
  lines stay those of that commit after another commit is checked out or made, and the bar then adds *· as committed in
  `<short hash>`*.
- **Refuses**:
  - no File page of a file of a repository in front: *Open a file of this repository to see the history of its lines.*
  - the file is in no commit: *`<file name>` is in no commit yet, so its lines have no history.*
  - every line selected was added since: *This line was added after the last commit, and has no history yet.* or *These
    lines were added after the last commit, and have no history yet.*
- **Uses**: `file-history.read-committed-lines`, `file-history.map-lines`.

Which lines are selected:

- Lines count from 1. With no selection, the line of the cursor.
- A selection that ends at the start of a line, below the line it starts on, leaves that last line out, as when whole
  lines are selected.
- The empty line after the last line break is no line: a selection there counts as the last line of the file.

Acceptance:

- [ ] With lines 4–9 selected in a file as committed, the timeline lists the commits `git log -L4,9:<path> HEAD` lists,
      and the bar says *History of lines 4–9 of `<path>`*.
- [ ] Selecting from line 4 to the start of line 10 asks for lines 4–9.
- [ ] With a line inserted above, unsaved, selecting lines 5–10 asks for lines 4–9 and the bar adds *· lines 5–10 in the
      editor*.
- [ ] Selecting only lines typed since the last commit is refused with the message above.
- [ ] With the timeline narrowed to `main`, the history of lines of a file checked out on `feature` still follows them
      from the commit checked out.

### `file-history.folder` — Show the history of a folder

Narrows the timeline to the commits that changed anything in a folder.

- **Kind**: Operation
- **Function**: `features::file_history::folder(context, answers) -> Outcome`
- **Entry points**: **Show folder history** in the context menu of a folder in the Files page.
- **Offered when**: the folder is inside a repository and is not the root folder of a repository, whose history is all
  of it.
- **Plan**: no step: nothing in the repository changes. The timeline is read with `git log … --parents … --
  :(literal)<folder>`. Renames are not followed: git follows only files, so the history of a folder starts where it got
  its name.
- **Danger**: None.
- **Undo**: nothing to take back; **Show all commits** removes the narrowing.
- **After**: the timeline is shown; the bar says *History of the folder `<path>`*. Asked for the root folder of a
  repository (from Quick search), it shows that repository with no narrowing.
- **Refuses**: a folder outside every repository, *Choose a folder of this repository to see its history.*

Acceptance:

- [ ] The history of `src/ui` lists the commits that changed any file under it, and only those.
- [ ] The commits that changed its files while they were under another folder are not listed.
- [ ] The context menu of the root folder of a repository has no **Show folder history**.

### `blame.read-line` — Who last changed one line

- **Kind**: Read
- **Function**: `git::read::blame::read_line(git, request) -> Result<Option<BlameLine>, GitError>`
- **Entry points**: none: used by `blame.current-line`.
- **Reads**: `git blame --porcelain -L <n>,<n> [-w -M] [--contents -] -- <file>`, with the text of the File page on
  standard input when it has edits not saved; `-w -M` when `blame.ignoreWhitespaceAndMoves` is on.

Rules:

- The first line of the output starts with the hash; `author`, `author-time` (seconds) and `summary` are read from their
  lines.
- A hash of zeros is a line that is not committed yet.
- When git fails (an untracked file, a line past the end of the committed file), there is no blame for the line.

Acceptance:

- [ ] A porcelain output for a commit gives its hash, author, date and summary.
- [ ] A hash of `0000000000000000000000000000000000000000` gives a line not committed yet.
- [ ] A failure of git gives nothing, not an error shown to the user.

### `blame.current-line` — The blame of the line under the cursor

The line under the cursor in the File page shows who last changed it, when, and the subject of that commit, dimmed at
the end of the line.

- **Kind**: View
- **Function**: `views::blame::current_line_view(state, ui) -> Vec<Intent>`
- **Entry points**: the File page of a file on disk inside a repository.
- **Offered when**: `blame.enabled` is on, and the selection is within one line.
- **Reads**: `blame.read-line`, 200 ms after the cursor last moved or the File page came to the front; an answer for a
  line the cursor has left since is dropped.
- **Settings**: `blame.enabled` (on by default) turns it on and off; `blame.ignoreWhitespaceAndMoves` blames with `-w -M`.
- **Uses**: `blame.read-line`, `blame.toggle-current-line`, `blame.format-line` (for how long ago).

What it shows:

- After the end of the line, past a gap about three letters wide, in the dimmed color of the theme: `<author>, <how long
  ago> • <summary>`, such as *Ann Lee, 3 days ago • Fix the parser*. How long ago is said as `blame.format-line` says
  `${ago}`.
- A line not committed yet: *Not committed yet*. It takes edits not saved into account.
- Hovering it, for a committed line: the author, the date and time, the summary, and a link **Show `<short hash>` in the
  timeline**, which shows that commit in the timeline. Nothing is shown when git has no blame for the line.

Acceptance:

- [ ] Moving the cursor to a line and waiting 200 ms shows that line's author, age and subject at its end, and only there.
- [ ] Typing on a line makes it say *Not committed yet* once the cursor rests.
- [ ] With `blame.enabled` off, nothing is shown.
- [ ] Selecting across two lines shows nothing.
- [ ] The link of the hover selects the commit in the timeline, loading it if it is further back than the timeline is
      loaded.

### `blame.toggle-current-line` — Toggle the blame of the line under the cursor

- **Kind**: Operation
- **Function**: `features::blame::toggle_current_line(context, answers) -> Outcome`
- **Entry points**: **Toggle inline blame** in Quick search.
- **Plan**: `write <user settings.json>`, with `blame.enabled` set to the opposite of what it is.
- **Danger**: None.
- **Undo**: not by Undo; the command again switches it back.
- **Settings**: `blame.enabled`.

Acceptance:

- [ ] Running it with `blame.enabled` on sets it to `false` in the user's settings, and the text at the end of the line
      goes away.
- [ ] Running it again sets it to `true`.

### `blame.open-line-commit` — Open the commit of the current line

Opens the commit that last changed the line under the cursor as a page of its own.

- **Kind**: Operation
- **Function**: `features::blame::open_line_commit(context, answers) -> Outcome`
- **Entry points**: `Alt+B` (`Ctrl+Alt+B` on macOS) in the File page; **Open commit of current line as a page** in the
  context menu of the File page and in Quick search.
- **Offered when**: the File page shows a file of a repository; the key works while its editor has the keyboard.
- **Reads**: the blame the file shows, when it shows one; otherwise `blame.read-file`.
- **Plan**: no step: nothing in the repository changes.
- **Danger**: None.
- **Undo**: nothing to take back.
- **After**: the commit's page opens (`commit-page.open`), in the repository of the file.
- **Refuses**: no File page of a file of a repository, *Put the cursor on a line of a file of a repository.*; a line not
  committed, *This line is not committed yet.*; a line git has no blame for, *This line is in no commit.*
- **Uses**: `blame.read-file`, `commit-page.open`.

Acceptance:

- [ ] `Alt+B` on a line last changed by `abc1234` opens the page of `abc1234`.
- [ ] `Alt+B` on a line typed since the last commit opens nothing and says the line is not committed yet.

### `blame.reveal-line-commit` — Show the commit of the current line in the timeline

- **Kind**: Operation
- **Function**: `features::blame::reveal_line_commit(context, answers) -> Outcome`
- **Entry points**: `Shift+Alt+B` (`Ctrl+Shift+Alt+B` on macOS) in the File page; **Reveal commit of current line in
  the timeline** in the context menu of the File page and in Quick search.
- **Offered when**: as `blame.open-line-commit`.
- **Reads**: as `blame.open-line-commit`.
- **Plan**: no step: nothing in the repository changes.
- **Danger**: None.
- **Undo**: nothing to take back.
- **After**: the timeline is shown with the commit selected: loaded when it is further back than the timeline is
  loaded, and with the narrowing removed when it leaves the commit out.
- **Refuses**: as `blame.open-line-commit`.
- **Uses**: `blame.read-file`.

Acceptance:

- [ ] `Shift+Alt+B` on a line selects its commit in the timeline.
- [ ] With the timeline narrowed to a branch that does not have the commit, the narrowing is removed and the commit
      selected.

### `blame.read-file` — Who last changed each line of a file

- **Kind**: Read
- **Function**: `git::read::blame::read_file(git, request) -> Result<Option<Vec<Option<BlameLine>>>, GitError>`
- **Entry points**: none: used by `blame.file`, `blame.toggle-file`, `blame.previous` and the commands of the current
  line.
- **Reads**: `git blame --incremental [-w -M] [--contents -] [<revision>] -- <file>`: `-w -M` when
  `blame.ignoreWhitespaceAndMoves` is on; the text of the File page on standard input with `--contents -` when it has
  edits not saved; `<revision>` for a file as it was in a commit.

Rules:

- Each group starts with a line `<hash> <line in the commit> <line now> <count>`: the `<count>` lines from `<line now>`
  (from 1) are of that commit. The lines after it, until the next group, describe the commit the first time it is named:
  `author`, `author-time` (seconds), `summary`, and `previous <hash> <file>`, the commit before it and the name the file
  had there (unquoted when git quotes it).
- A hash of zeros: lines not committed yet.
- A commit that added the file has no `previous`.
- When git fails, the file is in no commit: there is no blame.

Acceptance:

- [ ] A recorded output with two commits and a `previous` line gives each line its commit, and the `previous` of the
      commit that has one.
- [ ] Lines of a commit named a second time, without its details, get the details given the first time.
- [ ] An untracked file gives no blame.

### `blame.format-line` — What is said beside each line

Turns the format of `fileBlame.format` and the blame of a file into the text beside each line.

- **Kind**: Derivation
- **Function**: `model::blame::format_line(input: &FormatInput) -> Vec<String>`
- **Settings**: `fileBlame.format` (`${date} ${author:20}` unless set), `fileBlame.groupLines` (on by default).

Rules of the format:

- `${date}` is the day of the commit, `YYYY-MM-DD` in local time; `${ago}` how long ago it was; `${author}` who made it;
  `${summary}` the first line of its message; `${hash}` its hash. A number after a colon, as `${author:20}`, is the most
  letters it may take (a number below 1 counts as 1). Anything else, an unknown name in `${…}` included, is written as it
  is.
- A format that names none of the five, or a setting that is empty or only spaces, is the default instead.
- A value longer than its most letters is cut to one letter less and ends with `…`. A hash is 7 letters unless a number
  says otherwise, and is cut with no `…`.
- How long ago is said in the largest unit that has passed at least once: years of 365 days, months of 30 days, days,
  hours, minutes: *1 day ago*, *3 days ago*, *2 months ago*; under a minute, *just now*. It is counted from the time the
  labels are made.

Rules of the column:

- Each part of the format is as wide as its widest value among the commits of the file, so that what follows it starts
  in one column on every line; the last part is not widened. Spaces at the end are taken off.
- A line not committed yet says *Not committed yet*. A line git said nothing of says nothing.
- With `fileBlame.groupLines` on, a line of the same commit as the line above it says nothing (it keeps the bar and the
  background of its commit); the first of a run says it. Off, every line says it.

Examples, for the commits *A* (3 days ago, Ann Lee, *Fix the parser*) and *B* (2 months ago, Bob Stone, *Add the
lexer*), with `${ago} ${author:16} ${summary:40}`:

- a line of *A*: `3 days ago   Ann Lee   Fix the parser`;
- a line of *B*: `2 months ago Bob Stone Add the lexer`.

With the default format, an author *Annabelle Fitzgerald-Smith* is said `Annabelle Fitzgeral…`; with `${hash:4}`, the
hash `abc1234…` is said `abc1`.

Acceptance:

- [ ] The examples above hold, spaces included.
- [ ] `${nope} ${author}` writes `${nope} ` and the author.
- [ ] A format of plain text with no `${…}` gives the default.
- [ ] Three lines in a row of one commit, with grouping on, give the text, then two empty strings.
- [ ] An elapsed time of 59 seconds gives *just now*; of 2 hours, *2 hours ago*; of 400 days, *1 year ago*.

### `blame.age-color` — The color of the age of a line

The bar at the left of what is beside a line has the color of its age: the recent color for a line just changed, the
old color for one changed `fileBlame.age.oldAfterDays` ago or more, and the colors in between for the ages in between.

- **Kind**: Derivation
- **Function**: `model::blame::age_color(input: &AgeInput) -> Option<String>`
- **Settings**: `fileBlame.age.enabled` (on), `fileBlame.age.recentColor` (`#e5534b`), `fileBlame.age.oldColor`
  (`#316dca`), `fileBlame.age.oldAfterDays` (730).

Rules:

- Input: the date of the commit, the time now, the two colors and the days. Output: a color `#rrggbb`.
- Recent days count more than old ones: the share of the way from the recent color to the old one is the logarithm of
  one plus the age in days, over the logarithm of one plus the days of the setting, and at most 1. A day is told from a
  week more than a year from a year and a week.
- The share is rounded to the nearest tenth: there are eleven colors, and lines changed about as long ago have the same.
  Each of red, green and blue goes that share of the way, rounded to a whole number.
- A color that is not `#rgb` or `#rrggbb` is its default instead; days below 1 are 730.
- A line not committed yet has the age of now. A line git said nothing of has no bar color (the bar's room is left).

Examples, with the defaults: now gives `#e5534b`; 730 days or more gives `#316dca`; 1 day gives a share of 0.1; 7 days
0.3; 30 days 0.5; 365 days 0.9.

Acceptance:

- [ ] The examples above hold.
- [ ] `fileBlame.age.recentColor` set to `red` uses `#e5534b`.
- [ ] With `fileBlame.age.enabled` off, no bar is drawn.

### `blame.pick-pinned-line` — The commit named on the first line in sight

When the first line of a run of lines of one commit is scrolled out of sight, the first line in sight names the commit,
so the lines at the top of the File page are never left unnamed.

- **Kind**: Derivation
- **Function**: `model::blame::pick_pinned_line(input: &PinnedInput) -> Option<(usize, String)>`

Rules:

- Input: the blame of each line, the texts of `blame.format-line`, the first line in sight. Output: the line to give a
  text to, and the text, or nothing.
- When the first line in sight says nothing and belongs to a commit, the lines above it of the same commit are walked up
  to the first of the run; if that first line is above the one in sight and says something, the line in sight gets its
  text. Otherwise nothing.
- The text given is drawn over what the line has, on the background of the editor, with the highlight when its commit is
  the one of the cursor line.

Acceptance:

- [ ] Scrolled so that line 12 is first in sight, in a run of one commit from line 10 to 20, line 12 shows the text of
      line 10.
- [ ] Scrolled so that line 10 is first in sight, nothing is pinned.
- [ ] With grouping off, nothing is ever pinned.

### `blame.mark-edited-lines` — Lines typed are not committed yet

While a file shows its blame, what is typed says *Not committed yet* at once, so that every line keeps something beside
it and the text does not jump, until git is asked again.

- **Kind**: Derivation
- **Function**: `model::blame::mark_edited_lines(lines: &[Option<BlameLine>], edits: &[LineEdit]) -> Vec<Option<BlameLine>>`

Rules:

- Input: the blame of each line, and the edits as ranges replaced and the text put in. Output: the blame of each line
  after them.
- An edit that starts and ends at the start of a line, and puts in nothing or text ending with a line break, puts in or
  takes out whole lines: it removes as many entries as lines it spans, and inserts one entry not committed for each line
  break of the text put in. Every other edit replaces the entries of the lines it touches, from its first line to its
  last, with one entry not committed per line of the text put in.

Examples (lines 1–5, from 0 in the edits):

- typing `x` in the middle of line 3: line 3 is not committed, the others keep theirs;
- pasting two whole lines at the start of line 2: 7 lines, the new 2 and 3 not committed, the old 2 now 4;
- deleting line 4 whole: 4 lines, the old 5 now 4.

Acceptance:

- [ ] Each example above holds.
- [ ] Pressing `Enter` in the middle of a line gives two lines not committed in place of one.

### `blame.file` — The blame beside the lines of a file

Beside each line of the File page, before its text, the day it was last changed and by whom, with the commit's other
lines brought out and the age of each line in color.

- **Kind**: View
- **Function**: `views::blame::file_view(state, ui) -> Vec<Intent>`
- **Entry points**: the button with a person at the top right of the File page (**Show file blame**), the entry in its
  context menu, **Toggle file blame** in Quick search (`blame.toggle-file`); or on its own, with
  `fileBlame.everyFile`.
- **Offered when**: the File page shows a file on disk inside a repository of the workspace, or a file as it was in a
  commit. It is switched on file by file, and a file keeps it until its page is closed.
- **Reads**: `blame.read-file`; again 400 ms after the last edit, with the text as it is in the page, saved or not; again
  when the file is saved; again when the repository changes (a commit made, a checkout) or the setting
  `blame.ignoreWhitespaceAndMoves` changes. An answer older than one already shown is dropped.
- **Settings**: `fileBlame.format`, `fileBlame.groupLines`, `fileBlame.age.*`, `blame.ignoreWhitespaceAndMoves`,
  `fileBlame.everyFile`.
- **Uses**: `blame.read-file`, `blame.format-line`, `blame.age-color`, `blame.pick-pinned-line`,
  `blame.mark-edited-lines`, `blame.toggle-file`, `blame.previous`, `blame.ignore-whitespace`, `blame.every-file`.

What it shows:

- A column before the text of every line, as wide as the longest text of `blame.format-line`, so that the text of the
  file stays in one column; lines that say nothing keep the room. It is drawn in the dimmed color of the theme, in the
  font of the text.
- At the left of the column, a bar 3 pixels wide in the color of `blame.age-color`.
- What is beside the line under the cursor stands out on the blame highlight background of the theme, with its
  highlight foreground, and so does what is beside every other line of the same commit. In a file longer than the page,
  the scroll bar has a mark, in the highlight foreground color, for each run of those lines that follow each other.
- The first line in sight names its commit when the first line of its run is scrolled away (`blame.pick-pinned-line`).
- Lines not committed yet say *Not committed yet* and have the color of what was just changed; what is typed says so at
  once (`blame.mark-edited-lines`).
- Hovering what is beside a committed line shows the author, the date and time and the summary, and the links **Show
  `<short hash>` in the timeline**, **Open it as a page**, and, when the commit has a commit before it for this
  file, **Blame the file as it was before** (`blame.previous`).
- When lines wrap, the column stays: the parts of a wrapped line after the first start after the column, in line with
  the text, and nothing of the text is drawn under or past the column.
- While the file shows its blame, the title of the File page has the closed eye (**Hide file blame**) in place of the
  person, the funnel (`blame.ignore-whitespace`) and the pin (`blame.every-file`).
- While the blame is read the first time on request, the activity indicator of the status bar says *reading the blame…*.

Acceptance:

- [ ] Showing the blame of a file of 3 commits puts a text before each first line of a run, with the default format
      `<YYYY-MM-DD> <author>`, and the text of the file starts in the same column on every line.
- [ ] Moving the cursor to a line of commit *A* brings out the column of every line of *A*, and the scroll bar of a long
      file has one mark per run of them.
- [ ] Typing a character on a line makes it say *Not committed yet* before git is asked, and 400 ms after the last key
      the blame is read again with the text on standard input.
- [ ] With word wrap on, a line longer than the page wraps with its second part starting in the text column, not under
      the blame.
- [ ] A commit made in a terminal that changes the file updates the column without closing the page.
- [ ] Hiding it and closing the page, then opening the file again, shows no blame.

### `blame.toggle-file` — Show or hide the blame of a file

- **Kind**: Operation
- **Function**: `features::blame::toggle_file(context, answers) -> Outcome`
- **Entry points**: the person button (**Show file blame**) and the closed eye (**Hide file blame**) at the top right of
  the File page; **Toggle file blame** in its context menu and in Quick search.
- **Offered when**: the File page shows a file on disk while the workspace has a repository, or a file as it was in a
  commit.
- **Reads**: `blame.read-file`, to show it.
- **Plan**: no step: nothing in the repository changes. It shows or hides the blame of that file (`blame.file`).
- **Danger**: None.
- **Undo**: the command again.
- **After**: hidden while `fileBlame.everyFile` is on, the file stays without blame until its page is closed.
- **Refuses**: no File page, *Open a file of a repository to see who changed its lines.*; a file git has no blame for,
  *`<file name>` is not in any commit yet, so there is no one to name beside its lines.*, and nothing is shown.
- **Uses**: `blame.read-file`, `blame.file`.

Acceptance:

- [ ] On a file of a repository, it shows the blame; again, it hides it.
- [ ] On a file never committed, it says so and shows nothing.

### `blame.previous` — Blame the file as it was before a commit

Opens the file as the commit that last changed a line found it, with the blame it had then: who had written what the
commit replaced. Going on from there walks back through the history of a line, past the commits that only moved or
reformatted it.

- **Kind**: Operation
- **Function**: `features::blame::previous(context, answers) -> Outcome`
- **Entry points**: **Blame the file as it was before** in the hover of the blame of a line (`blame.file`); **Blame file
  before commit of current line** in the context menu of the File page and in Quick search.
- **Offered when**: the link, when the commit has a commit before it for this file (`previous` of `blame.read-file`);
  the command, on a File page of a file of a repository.
- **Reads**: for the command, the blame of the file, as `blame.open-line-commit`; then `blame.read-file` with the commit
  before and the name the file had there.
- **Plan**: no step: nothing in the repository changes.
- **Danger**: None.
- **Undo**: nothing to take back.
- **After**: a File page opens on the file as it was in the commit before (read-only), its trail naming that commit,
  with its blame shown, the cursor on the line of the same number (or the last line), highlighted as the file it came
  from.
- **Refuses**: as `blame.open-line-commit`; and when the commit of the line added the file, *`<short hash>` is the
  commit that added this file, so there is nothing before it to blame.*
- **Uses**: `blame.read-file`, `blame.file`.

Acceptance:

- [ ] On a line last changed by `B`, whose parent is `A`, it opens the file as of `A`, with `A`'s blame, the trail
      naming `A`.
- [ ] On a file renamed by `B`, it opens the file under its old name as of `A`.
- [ ] On a line of the commit that added the file, it says so and opens nothing.

### `blame.ignore-whitespace` — Ignore whitespace and moved lines in the blame

Leaves a line to who wrote it when all that was changed since is its whitespace or its place in the file.

- **Kind**: Operation
- **Function**: `features::blame::ignore_whitespace(context, answers) -> Outcome`
- **Entry points**: the funnel at the top right of the File page while the file shows its blame (**Ignore whitespace and
  moved lines in blame**, filled while on: **Stop ignoring whitespace and moved lines in blame**); **Toggle ignoring
  whitespace and moved lines in blame** in Quick search.
- **Plan**: `write <user settings.json>`, with `blame.ignoreWhitespaceAndMoves` set to the opposite.
- **Danger**: None.
- **Undo**: not by Undo; the funnel again.
- **After**: the blame of every file that shows one is read again, with or without `-w -M`; so is the blame of the line
  under the cursor.
- **Settings**: `blame.ignoreWhitespaceAndMoves`.
- **Uses**: `blame.read-file`, `blame.read-line`.

Acceptance:

- [ ] With it on, a line re-indented by `B` and written by `A` is said to be `A`'s, and git runs with `-w -M`.
- [ ] The funnel is filled while the setting is on.

### `blame.every-file` — Show the blame in every file

The pin shows the blame in every file that is opened, rather than in the ones it is asked for.

- **Kind**: Operation
- **Function**: `features::blame::every_file(context, answers) -> Outcome`
- **Entry points**: the pin at the top right of the File page while the file shows its blame (**Keep file blame on in
  every file**, pinned while on: **Stop keeping file blame on in every file**); **Toggle file blame in every file** in
  Quick search, from anywhere.
- **Plan**: `write <user settings.json>`, with `fileBlame.everyFile` set to the opposite.
- **Danger**: None.
- **Undo**: not by Undo; the pin again.
- **After**: on, every File page of a file on disk that comes into sight shows its blame, unless it was hidden in it
  (until its page is closed) or git has none for it (asked again when the repository changes). Off, the files that showed
  it only because of the setting lose it; the ones it was asked for one by one keep it.
- **Settings**: `fileBlame.everyFile`.
- **Uses**: `blame.file`, `blame.toggle-file`.

Acceptance:

- [ ] With it on, opening any committed file shows its blame without asking.
- [ ] Hiding the blame of one file while it is on keeps that file without it until its page is closed.
- [ ] Turning it off leaves the blame on the file it had been asked for before, and takes it from the others.

### `reflog.read` — Where `HEAD` has been

- **Kind**: Read
- **Function**: `git::read::reflog::read(git, request) -> Result<Vec<ReflogEntry>, GitError>`
- **Entry points**: none: used by `reflog.browse` (and by Undo).
- **Reads**: `git reflog --max-count=<n> --format=%H%x1f%gs%x1f%gd%x1f%ct <ref>`: one entry per line, its fields
  separated by the byte `0x1f`: the commit, what moved `HEAD` there, the selector (`HEAD@{3}`) and the time in seconds.

Acceptance:

- [ ] A recorded output of three lines gives three entries, newest first, each with its four fields.
- [ ] A subject with spaces and colons, such as `checkout: moving from main to feature`, is kept whole.

### `reflog.browse` — Browse the reflog

Lists the last 200 positions of `HEAD`, to show one, branch from it, reset to it, check it out or cherry-pick it: the
way back to commits that no branch reaches any more, after a reset or a rebase.

- **Kind**: Operation
- **Function**: `features::reflog::browse(context, answers) -> Outcome`
- **Entry points**: **Browse reflog** in Quick search; **Reflog** in the **…** menu of the header, also in the narrow
  layout of the window.
- **Reads**: `reflog.read` of `HEAD`, 200 entries.
- **Asks**:
  1. a pick titled *Reflog: where HEAD has been*, with the placeholder *Pick an entry to go back to it or recover it*;
     each entry has its subject as label, `<short hash> · HEAD@{n}` as description and the local date and time as
     detail; typing filters on the label and the description;
  2. a pick titled *`<short hash>`: `<subject>`*, with **Show in the timeline**, **Create branch here…**, **Reset
     `<branch>` to here…** (`HEAD` when detached), **Check out (detached)** and **Cherry-pick**.
- **Plan**: none of its own: the second answer calls the feature it names with the commit of the entry.
- **Danger**: None of its own; the feature called says its own.
- **Undo**: that of the feature called.
- **After**: **Show in the timeline** selects the commit, loading the history up to it, removing the narrowing that
  leaves it out, or, for a commit on no branch or tag, showing its history alone.
- **Refuses**: with no repository, *No Git repository in this workspace.*
- **Uses**: `reflog.read`, `branch.create`, `reset.to-commit`, `cherry-pick.commit`, `branch.checkout-detached`.

Acceptance:

- [ ] The first pick lists at most 200 entries, newest first.
- [ ] After `git reset --hard HEAD~2` in a terminal, the entry before the reset is listed, and **Create branch here…**
      makes a branch at the lost commit.
- [ ] **Cherry-pick** on an entry runs `cherry-pick.commit` for its commit, as a commit with one parent.
- [ ] Closing either pick does nothing.

### `bisect.read` — The bisect under way

- **Kind**: Read
- **Function**: `git::read::bisect::read(git, request) -> Result<Option<Bisect>, GitError>`
- **Entry points**: none: read with the state of the repository.
- **Reads**: the file `BISECT_START` in the git directory of the worktree; `git for-each-ref --format=%(refname)
  %(objectname) refs/bisect`; with a bad commit and at least one good one,
  `git rev-list --count <bad> --not <good>…`.

Rules:

- Without `BISECT_START`, no bisect is under way.
- `refs/bisect/bad` is the bad commit; each `refs/bisect/good-*` a good one; each `refs/bisect/skip-*` a skipped one.
- The count is how many commits can still be the first bad one: those the bad commit reaches and no good commit reaches,
  the bad one included. Without both marks, there is no count.

Acceptance:

- [ ] With `A→B→C→D→E`, `A` good and `E` bad, the count is 4.
- [ ] With only a bad commit marked, there is no count.
- [ ] With no `BISECT_START`, there is no bisect, even if `refs/bisect/*` are left.

### `bisect.start` — Start a bisect from a commit

Starts finding the commit that broke something, from a commit known to be broken, or one known to work.

- **Kind**: Operation
- **Function**: `features::bisect::start(context, answers) -> Outcome`
- **Entry points**: **Start bisect: this commit is bad** and **Start bisect: this commit is good** in the context menu of
  a commit.
- **Offered when**: no bisect is under way, and no merge, rebase, cherry-pick, revert or patch is under way.
- **Plan**: `git bisect start`, then `git bisect bad <hash>` (or `git bisect good <hash>`). The title is *Start bisect:
  `<short>` is bad* (or *good*).
- **Danger**: None.
- **Undo**: not by Undo, which refuses while a bisect is under way, *End the bisect first.*; **End** (`bisect.end`)
  goes back to where it started.
- **After**: the banner of `bisect.banner` asks for the other mark.
- **Uses**: `bisect.banner`, `bisect.mark`, `bisect.end`.

Acceptance:

- [ ] **Start bisect: this commit is bad** on `E` runs `git bisect start` and `git bisect bad <E>`, and the banner says
      *Bisect: mark a good commit to begin*.
- [ ] While a rebase is stopped, neither entry is offered.

### `bisect.mark` — Mark a commit good or bad

While a bisect is under way, marks any commit of the timeline.

- **Kind**: Operation
- **Function**: `features::bisect::mark(context, answers) -> Outcome`
- **Entry points**: **Bisect: mark as good** and **Bisect: mark as bad** in the context menu of a commit, in place of the
  entries that start one.
- **Offered when**: a bisect is under way, and no other operation is.
- **Plan**: `git bisect good <hash>` or `git bisect bad <hash>`. The title is *Bisect: `<short>` is good* (or *bad*).
  With a good and a bad commit marked, git checks out a commit between them to test.
- **Danger**: None.
- **Undo**: not by Undo (*End the bisect first.*).
- **After**: when a single commit is left that can be the first bad one, the message *Bisect: the first bad commit is
  `<short>` "`<subject>`".* with **Show in the timeline** and **End bisect**.
- **Uses**: `bisect.read`, `bisect.end`.

Acceptance:

- [ ] With `E` marked bad, **Bisect: mark as good** on `A` runs `git bisect good <A>`, and git checks out a commit
      between them.
- [ ] Marking the last commit left says which is the first bad one, with the two buttons.

### `bisect.answer` — Answer for the commit being tested

The banner's **Good**, **Bad** and **Skip** mark the commit git checked out to test.

- **Kind**: Operation
- **Function**: `features::bisect::answer(context, answers) -> Outcome`
- **Entry points**: **Good** (tooltip `git bisect good`), **Bad** (`git bisect bad`) and **Skip** (*git bisect skip:
  this commit cannot be tested*) in the banner of `bisect.banner`.
- **Offered when**: a bisect is under way, a good and a bad commit are marked, and the first bad one is not found yet.
  The buttons are disabled while an operation runs.
- **Plan**: `git bisect good`, `git bisect bad` or `git bisect skip`, with no commit: the one checked out. The title is
  *Bisect: this commit is good* (*bad*, *skipped*).
- **Danger**: None.
- **Undo**: not by Undo (*End the bisect first.*).
- **After**: git checks out the next commit to test, and the banner counts again; when a single commit is left, the
  message of `bisect.mark`.
- **Asks**: when no button gave the answer, **Mark the commit being tested**: **This commit is good**, **This commit
  is bad** or **Skip this commit**.
- **Refuses**: with no bisect under way, *No bisect is in progress.*; once one commit is left, *The bisect found the
  first bad commit already.*
- **Uses**: `bisect.read`, `bisect.banner`.

Acceptance:

- [ ] **Good** runs `git bisect good`, and the banner's count goes down.
- [ ] **Skip** runs `git bisect skip`, and the commit gets the mark *skipped* in the timeline.
- [ ] The answer that leaves one commit shows *Bisect: the first bad commit is …* with **Show in the timeline** and
      **End bisect**.

### `bisect.banner` — The banner of a bisect

Says what the bisect needs next, and takes the answer for the commit checked out.

- **Kind**: View
- **Function**: `views::bisect::banner_view(state, ui) -> Vec<Intent>`
- **Entry points**: above the commits of the timeline, while a bisect is under way.
- **Offered when**: `bisect.read` gives a bisect.
- **Uses**: `bisect.read`, `bisect.answer`, `bisect.end`.

What it says, after a search icon:

- no bad commit marked: *Bisect: mark a bad commit to begin*;
- a bad one and no good one: *Bisect: mark a good commit to begin*;
- both, more than one commit left: *Bisect: test `<short hash of HEAD>` · N commits can still be the first bad one*,
  with **Good**, **Bad** and **Skip**;
- one commit left: *Bisect: the first bad commit is `<short>`*, with **Show**, which selects it in the timeline.

**End** is always there.

Acceptance:

- [ ] With `A` good, `E` bad, and `C` checked out, it says *Bisect: test `<C>` · 4 commits can still be the first bad
      one*.
- [ ] Once found, **Good**, **Bad** and **Skip** are gone, and **Show** selects the commit.
- [ ] Without a bisect, there is no banner.

### `bisect.marks` — The marks of a bisect in the timeline

The commits marked so far carry a label in the timeline: `good`, `bad` or `skipped`.

- **Kind**: View
- **Function**: `views::bisect::marks_view(mark) -> (&'static [Icon], Option<String>)`, the icons of the label of a
  marked commit, which the timeline draws as it draws its other labels (`model::graph::labels` gives them)
- **Entry points**: the labels of the commits in the timeline, while a bisect is under way.
- **Offered when**: `bisect.read` gives a bisect.
- **Uses**: `bisect.read`.

Rules:

- The bad commit has a label `bad`; each good one `good`; each skipped one `skipped`; after the other labels of the
  commit.
- A double click on such a label opens the commit's page, as on a tag.
- These labels do not name a line of the graph, as branch labels do.

Acceptance:

- [ ] After marking `E` bad and `A` good, `E` has a label `bad` and `A` one `good`.
- [ ] After **End**, the labels are gone.

### `bisect.end` — End the bisect

Goes back to where the bisect started.

- **Kind**: Operation
- **Function**: `features::bisect::end(context, answers) -> Outcome`
- **Entry points**: **End** in the banner (tooltip `git bisect reset`); **End bisect** in the message that names the
  first bad commit.
- **Offered when**: a bisect is under way.
- **Plan**: `git bisect reset`. The title is *End bisect*.
- **Danger**: None.
- **Undo**: by the reflog, as a checkout: Undo checks out again the commit that was being tested. The bisect is not
  started again.
- **After**: the banner and the marks are gone; what was checked out before the bisect is checked out.
- **Uses**: `bisect.banner`, `undo.last`.

Acceptance:

- [ ] Started on `main`, **End** runs `git bisect reset` and `main` is checked out again.
- [ ] After **End**, **Start bisect: …** is offered again in the context menu of a commit.

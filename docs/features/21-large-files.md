# Large files

Files that a repository stores with Git LFS: git commits a small pointer in their place, and Git LFS, a program of its
own beside git, keeps their content on a server and downloads it into the working tree. This area covers whether Git
LFS is there and what the repository stores with it, a file left as a pointer shown as such in the pages rather than
as the text of its pointer, downloading it, storing a pattern with Git LFS, and the locks of a server that has them,
with the **Large files** page that gathers them
([D84](../08-decisions.md#d84--large-files-stored-with-git-lfs-and-resolutions-git-remembers-shown-where-the-files-are-proposed)).
Nothing here needs Git LFS to read a repository: without it, the files are pointers, and the pages say so.

### `lfs.read-status` — Whether Git LFS is there, and what the repository stores with it

Whether the program Git LFS is installed and set up for the repository, and the patterns of the files the repository
stores with it.

- **Kind**: Read
- **Function**: `git::read::lfs::read_status(git: &Git, request: &LfsStatusRequest) -> Result<LfsStatus, GitError>`
- **Reads**:
  - `git lfs version`, whose first line is `git-lfs/<version> …`; git saying that `lfs` is not a git command means it
    is not installed. Once for the engine, and again on **Refresh**.
  - `git ls-files -z -- ':(glob)**/.gitattributes'`, the files of attributes git tracks, and the exclude file of
    attributes of the repository (`git rev-parse --git-path info/attributes`), read from disk: each line whose
    attributes have `filter=lfs` gives a pattern, with its file and its line.
  - `git config --get-regexp ^filter\.lfs\.`: the repository is set up when `filter.lfs.process` or
    `filter.lfs.smudge` is set, in any of its configurations.
- **Uses**: `repo.read-state`, with which it runs, and again only when a `.gitattributes` or the configuration changed
  (`repo.watch-changes`).
- **Extension**: built: read by the engine with the state of the repository (`lfs` of `RepositoryState`, protocol 1.13),
  the version only once the repository has patterns or is set up; `read/lfs.read-status` reads it with the version, for
  the **Large files** tab.
- As built, its request carries the version read before (`LfsStatusRequest::version`), so that `git lfs version`
  runs once for the engine, and the patterns read before (`LfsStatusRequest::patterns`), kept when no
  `.gitattributes` changed; a pattern of a commented line is not one. With `repo.read-state`, which is read without
  what was read before, it runs as `read_repository_status`: the patterns and the configuration at each read, and
  `git lfs version` only when the repository has patterns or is set up, so that a repository that stores nothing
  with Git LFS starts no process of it; the **Large files** page reads the version itself.

Acceptance:

- [x] Without Git LFS, it says it is not installed, and still lists the patterns `*.psd filter=lfs diff=lfs merge=lfs
      -text` of `.gitattributes` gives.
- [x] With Git LFS 3.7.1, it reads the version `3.7.1`.
- [x] A pattern in `assets/.gitattributes` is listed with that file; a line with `-filter` or `filter=other` is not.
- [x] A repository whose patterns are there but whose configuration has no `filter.lfs.` key is not set up.
- [x] A read after a change to a file of the working tree only does not read the attributes again.

### `lfs.read-files` — The files stored with Git LFS

The files of the commit checked out that the repository stores with Git LFS, with their sizes and whether their content
is downloaded.

- **Kind**: Read
- **Function**: `git::read::lfs::read_files(git: &Git, request: &LfsFilesRequest) -> Result<Vec<LfsFile>, GitError>`
- **Reads**: `git lfs ls-files --json`: for each file its path, its size, the id of its content (`oid`), whether its
  content is in the store of the repository (`downloaded`) and whether the file on disk is its content rather than its
  pointer (`checkout`). With a Git LFS older than 3.0, which has no `--json`, `git lfs ls-files --long --size`, whose
  `*` says downloaded and `-` not. It runs when the **Large files** page opens and on **Refresh** there, not with the
  reads of the repository (Q1190).
- **Uses**: `lfs.read-status`, by `lfs.page`.
- **Extension**: built: `read/lfs.read-files` (protocol 1.13), with the version `read/lfs.read-status` read, asked when
  the **Large files** tab opens and on **Refresh** there.
- As built, the output of `--json` is read one key on each line, as Git LFS indents it, since serde_json is kept to
  the crates `protocol` and `server` (rule R1.7); its escapes, `\u003c` and the others, are read. The sizes of
  `--long --size` are rounded by Git LFS, `1.2 KB` read as 1,200 bytes. Without Git LFS, an empty version, nothing is run.

Acceptance:

- [x] A repository with `a.bin` downloaded and `b.bin` left as a pointer lists both, `a.bin` downloaded and checked
      out, `b.bin` neither, each with its size.
- [x] A path with spaces and non-ASCII letters comes back as git has it.
- [x] The output of `--long --size` of Git LFS 2.13 gives the same files.
- [x] Without Git LFS it is not run, and the page says so.

### `lfs.pointer` — A file left as a pointer

Tells whether some bytes are a pointer of Git LFS, and which content they stand for.

- **Kind**: Derivation
- **Function**: `model::lfs::pointer(bytes: &[u8]) -> Option<LfsPointer>`
- **Uses**: by `lfs.read-content`, `lfs.show-in-pages`.

Rules:

- A pointer is text of less than 1,024 bytes, lines `<key> <value>` each ended by a line feed alone, its first key
  `version` with `https://git-lfs.github.com/spec/v1` (or `https://hawser.github.com/spec/v1`, of the first
  versions), the other keys in the order of their names, among them `oid`, `sha256:` followed by 64 hexadecimal digits
  in lower case, and `size`, a number of bytes.
- It gives the id, the size, and the other keys as they are. Anything else is not a pointer, and an empty file is an
  empty file.

Acceptance:

- [x] The three lines `git lfs` writes for a file of 2,000 bytes give its id and the size 2,000.
- [x] The same lines with CRLF, with the keys in another order, or with an id of 63 digits are not a pointer.
- [x] A text file that starts with `version https://git-lfs.github.com/spec/v1` and goes on past 1,024 bytes is not a
      pointer.

### `lfs.read-content` — The content of a file stored with Git LFS, from the store of the repository

The content a pointer stands for, read from the store of Git LFS on this computer, never downloaded by a read.

- **Kind**: Read
- **Function**: `git::read::lfs::read_content(git: &Git, request: &LfsContentRequest) -> Result<LfsContent, GitError>`
- **Reads**: `git lfs env`, its line `LocalMediaDir=<folder>`, once for each repository; then, for the pointer of
  `lfs.pointer`, the file `<folder>/<first two digits of the id>/<the next two>/<id>`, whose size is checked against the
  pointer. A missing file, or one of another size, is content not downloaded. It never runs the filter of Git LFS
  (`git cat-file --filters`, `git lfs smudge`), which would download from the server in the middle of a read.
- **Uses**: `lfs.pointer`, by `lfs.show-in-pages`, `diff.read-sides`, `file-view.read-revision`.
- **Extension**: built: `read/lfs.read-content` (protocol 1.13) gives a file of a commit, or of the index for no
  revision, with the content in place of its pointer when the store has it, the pointer with its size, its id and
  `isDownloaded` otherwise; the files of the scheme `gitardo` the extension opens, the sides of its diffs among them,
  are read with it (`reads.fileShownAt`).

Acceptance:

- [x] For a file downloaded, the bytes read are those of the file once checked out.
- [x] For a pointer whose object is not in the store, it says not downloaded, and no process of Git LFS talks to the
      server.
- [x] An object of the store shorter than the size of its pointer is not downloaded.

### `lfs.show-in-pages` — A file stored with Git LFS, in the pages

What the File page, the Diff page and the page of a commit show of a file stored with Git LFS: its content when it is
on this computer, and otherwise that it is not downloaded, with **Download**, never the text of its pointer.

- **Kind**: View
- **Function**: `views::lfs::stored_with_lfs(state: &StoredWithLfsState, ui: &mut Ui) -> Vec<Intent>`
- **Entry points**: drawn by the File page, the Diff page and the page of a commit in place of the text of a file
  whose version is a pointer (`lfs.pointer`) at a path the repository stores with Git LFS.
- **Reads**: `lfs.read-content`; `git check-attr -z filter -- <path>`, which says `lfs` for a path the repository
  stores with Git LFS, so that a file that only looks like a pointer is shown as it is.
- **Uses**: `lfs.pointer`, `lfs.read-content`, `lfs.download`, `diff.page`, `diff.read-sides`, `file-view.page`,
  `file-view.read-revision`, `commit-page.view`, `commit-page.read-patch`.
- **Extension**: built: a version of a commit or of the index that is a pointer opens as its content when it is in the
  store, and otherwise as a text that says **Stored with Git LFS, not downloaded**, with the path, the size and the
  first ten digits of its id, never the lines of its pointer; a message of VS Code says the file is not downloaded, with
  its size, and offers **Download**, once for each version opened. The file of the working tree is VS Code's own, which
  shows the pointer of a file not downloaded as it is; the **Large files** tab says which files are pointers.

- **File page.** A file of the working tree that is a pointer, as after a clone without Git LFS, shows **Stored with
  Git LFS, not downloaded**, its size, **Download**, and no text. A file at a revision shows the content when it is in
  the store, and the same message when not.
- **Diff page.** Each side that is a pointer is read from the store. When every side is there, the diff is the diff of
  the contents, with **Stored with Git LFS** beside the title of the page; a content that is not text is *This file
  looks binary.* as for any file. When a side is not downloaded, the page shows, in place of the diff, **Stored with
  Git LFS** with, for each side, its size and the first digits of its id, and **Download**.
- **Page of a commit.** The diff git gives of the pointers of a file stored with Git LFS, the lines `oid` and `size`
  removed and added, is shown as one line, **Stored with Git LFS: 2.0 MB → 2.1 MB**, with **Open it in the Diff
  page**.
- Sizes are said in the units of the system, as the window says sizes elsewhere.
- As built, the File page and the Diff page read the sides on their worker (`app::work::file_view::read_shown`,
  `app::work::lfs_sides`): a side whose bytes are a pointer (`lfs.pointer`) and whose path `git check-attr` says
  `lfs` for is read from the store. Each side not downloaded is said with its size, its first ten digits and
  **Downloaded** or **Not downloaded**, the Diff page naming them **Before** and **After**; a page that shows one is
  read again with the repository, so that **Download** shows the content once it ran. A file of the working tree that
  is a pointer is said not downloaded even when the store has its content, since editing the content there would
  change the file. The page of a commit marks a diff whose lines are those of two pointers, or of one and nothing,
  when `git check-attr` says `lfs` for its path (`git::read::commit_page::read_patch`), and says it in the note of the
  diff with **Open it in the Diff page**. **Stored with Git LFS** beside the title of the Diff page is in its bar,
  after the buttons. The snapshots are those of the File page of a file not downloaded.

Acceptance:

- [x] After a clone with `GIT_LFS_SKIP_SMUDGE=1`, opening `art/logo.psd` in the File page says **Stored with Git LFS,
      not downloaded** with its size and **Download**, and no line of its pointer.
- [x] The Diff page of a text file stored with Git LFS, changed and downloaded, shows the lines that changed in its
      content.
- [x] The page of a commit that changed `art/logo.psd` shows **Stored with Git LFS: 2.0 MB → 2.1 MB** in place of the
      lines of the pointers.
- [x] A file whose text is a pointer at a path the repository does not store with Git LFS is shown as text.
- [x] Each of these matches its snapshot in each theme.

### `lfs.download` — Download files stored with Git LFS

Downloads the content of files left as pointers, into the working tree for the commit checked out, or into the store
for another commit.

- **Kind**: Operation
- **Function**: `features::lfs::download(context, answers) -> Outcome`
- **Entry points**: **Download** where `lfs.show-in-pages` says a file is not downloaded; **Download** in the menu of
  a file of the **Large files** page; **Download all** on that page; **Download** in the context menu of a file or a
  folder of the Files page that holds files left as pointers.
- **Offered when**: Git LFS is installed (`lfs.read-status`).
- **Plan**: for the files of the commit checked out, `git lfs pull --include=<path>[,<path>…]`, each path from the top
  of the repository, a folder as `<folder>/**`; for every file, `git lfs pull`; for a file of another commit,
  `git lfs fetch --include=<path> <remote> <commit>`, which fills the store and leaves the working tree as it is, the
  remote being the upstream's of the branch checked out, or the default remote.
- **Danger**: None.
- **Undo**: Nothing to take back: the files become what is committed.
- **After**: the page that asked reads the file again and shows its content.
- **Refuses**: without Git LFS: **Git LFS is not installed: install it from git-lfs.com, then try again.** with **Open
  git-lfs.com**; a path holding `,` or a character of patterns, which `--include` cannot name alone: **`<path>` cannot
  be named to Git LFS on its own: download all the files instead.** (Q1192).
- **Uses**: `lfs.read-status`, `lfs.show-in-pages`, `lfs.page`, `remote.default`.
- **Extension**: built: **Download** in the message of a version not downloaded sends `feature/run` of `lfs.download`
  with the target `lfsFiles` (protocol 1.13), the path and the commit of the version, or none for the index
  (`Controller.runOnEngine`); once it ran, the tabs of the scheme `gitardo` said not downloaded in that repository are
  read again and show the content.
- As built, it runs on `Target::LfsFiles`, the paths and the commit; a commit that is the one checked out is pulled.
  A refusal offers an operation only, so the refusal without Git LFS says the message with no **Open git-lfs.com**,
  which the **Large files** page has. A commit not checked out in a repository with no remote is refused: **This
  repository has no remote to download the files of another commit from.** **Download** on a folder of the Files
  page waits for that page to read which files are pointers (Q1190); the folder is downloaded from the **Large
  files** page.

Acceptance:

- [x] **Download** on `art/logo.psd` runs `git lfs pull --include=art/logo.psd`, and the File page shows its content
      after.
- [x] **Download** on the folder `art` runs `git lfs pull --include=art/**`.
- [x] **Download all** runs `git lfs pull`, and the page lists every file downloaded after.
- [x] **Download** in the Diff page of a commit that is not checked out runs `git lfs fetch --include=<path> origin
      <commit>`, the working tree unchanged, and the diff then shows the contents.
- [x] Without Git LFS, it refuses with the message, and nothing runs.

### `lfs.page` — The Large files page

A page with what the repository stores with Git LFS: whether Git LFS is there and set up, the patterns, the files with
their sizes and whether they are downloaded, and the locks.

- **Kind**: View
- **Function**: `views::lfs::page(state: &LfsPageState, ui: &mut Ui) -> Vec<Intent>`
- **Entry points**: **Large files…** in the context menu of the row of a repository in **Repositories** (`repo.tabs`);
  **Large Files (Git LFS)…** in Quick search.
- **Reads**: `lfs.read-status`, `lfs.read-files`, `lfs.read-locks`.
- **Uses**: `lfs.download`, `lfs.set-up`, `lfs.track`, `lfs.untrack`, `lfs.store-committed`, `lfs.lock`,
  `lfs.unlock`, `file-view.page`, `ui.page-trail`.
- **Extension**: planned, in a later milestone: an editor tab with the same sections, or entries of the **Gitardo
  Repositories** view (Q1197).

- The page is titled **Large files** in its trail. Its head says **Git LFS `<version>`**, or **Git LFS is not
  installed** with **Open git-lfs.com**, and, when the repository has patterns and is not set up, **Git LFS is not set
  up in this repository: its files stay pointers.** with **Set up** (`lfs.set-up`).
- **Patterns**: each pattern with the file it is in, **Track a pattern…** above them; the menu of a pattern has **Stop
  storing with Git LFS** and **Open `<file>`**.
- **Files**: each file with its size, **Downloaded** or **Not downloaded**, and the name of whoever locked it; above
  them their count and size, **Download all** with the count and size not downloaded, and **Refresh**. The menu of a
  file: **Download**, **Lock** or **Unlock**, **Open file**, **Show in the Files page**.
- **Locks**: read when the page opens and on **Refresh**; while they are read, **Reading the locks…**; a server
  without locks: **The server of `<remote>` does not support locks.**
- With no pattern and no file: **This repository stores no file with Git LFS.** with **Track a pattern…**.

Acceptance:

- [x] In a repository with `*.psd` tracked and two files, one not downloaded, the page lists the pattern, both files
      with their sizes, and **Download all (1, 2.0 MB)**.
- [x] Without Git LFS, the head says it is not installed, and the patterns are listed all the same.
- [x] A file locked by someone else shows their name, and its menu has **Unlock** with the warning of `lfs.unlock`.
- [x] The page matches its snapshots in each theme, with files and with none.

- As built (phase 3), the page reads on its worker (`app::work::large_files::read_large_files`) the status with the
  version of Git LFS and the files of the commit checked out, again once the repository was read since, on
  **Refresh**, and for another repository; the menu of the row of a repository not shown shows it first. A file is
  said **Downloaded** when its content is in the store and in the working tree, so that **Download all** counts the
  files left as pointers. `git lfs ls-files --json` prints `"files": null` in a repository with no file, which lists
  none.
- As built (phase 4), the locks are read on the same worker, only when the page opens, on **Refresh** and once **Lock**
  or **Unlock** ended; a read of the files after a read of the repository keeps the locks read before. The name of
  whoever locked a file is in a column between the path and the size, **You** for a lock of this user when the server
  verifies. **Lock** and **Unlock** have no icon, as no icon says a lock. A note under **Files** says **Reading the
  locks…**, **The server of `<remote>` does not support locks.**, or what git said when the locks could not be read;
  the remote is the one `remote.default` chooses, as Git LFS asks it.

### `lfs.set-up` — Set up Git LFS for a repository

Sets up the filter and the hooks of Git LFS for one repository, so that its files are downloaded at checkout and stored
with Git LFS at commit.

- **Kind**: Operation
- **Function**: `features::lfs::set_up(context, answers) -> Outcome`
- **Entry points**: **Set up** on the **Large files** page.
- **Offered when**: Git LFS is installed, the repository has patterns, and it is not set up (`lfs.read-status`).
- **Plan**: `git lfs install --local`, which writes the configuration of the repository and its hooks; never the
  user's own configuration (Q1194).
- **Danger**: None.
- **Undo**: Not taken back by Undo: `git lfs uninstall --local`, in a terminal.
- **After**: offers **Download all** when files are left as pointers. When the repository has hooks of its own of the
  same names, Git LFS leaves them and says so: its message is shown.
- **Uses**: `lfs.read-status`, `lfs.download`.
- **Extension**: planned, with `lfs.page`.

Acceptance:

- [x] It runs `git lfs install --local`, and the page no longer says the repository is not set up.
- [x] After it, **Download all** is offered while a file is a pointer.

- As built, the files left as pointers are read before the plan (`Query::LfsPointers`), and **Download all
  (`<count>`)** is offered with them; it refuses once the repository is set up. The message of Git LFS about hooks of
  the repository's own is in the log of the operation, not said apart.

### `lfs.track` — Store files with Git LFS

Has Git LFS store the files that a pattern takes, from their next commit on.

- **Kind**: Operation
- **Function**: `features::lfs::track(context, answers) -> Outcome`
- **Entry points**: **Store with Git LFS…** in the context menu of a file of the Files page and in the menu of a file of
  the changes (`changes.row-menu`); **Track a pattern…** on the **Large files** page.
- **Offered when**: Git LFS is installed and set up for the repository, and no pattern takes the file already.
- **Asks**: **Store with Git LFS**: **All `.<extension>` files**, `*.<extension>` (the default when the name has an
  extension), **This file**, **Everything in `<folder>`**, `<folder>/**`, and a pattern typed.
- **Plan**: `git lfs track <pattern>`, or `git lfs track --filename <path>` for **This file**, which takes its name as
  it is, run at the top of the repository: Git LFS writes the line in the `.gitattributes` there.
- **Danger**: None.
- **Undo**: Not taken back by Undo: **Stop storing with Git LFS** (`lfs.untrack`) does.
- **After**: says **Files that `<pattern>` takes are stored with Git LFS from their next commit. Commit
  `.gitattributes` with them.** When git tracks files that the pattern takes and that are not stored with Git LFS yet,
  it says how many, with **Store them with Git LFS in the next commit** (`lfs.store-committed`).
- **Refuses**: without Git LFS, as `lfs.download`; not set up: **Git LFS is not set up in this repository.** with
  **Set up**.
- **Uses**: `lfs.read-status`, `lfs.set-up`, `lfs.untrack`, `lfs.store-committed`, `changes.row-menu`.
- **Extension**: built: **Store with Git LFS…** in the context menu of a file of VS Code's Explorer, in a repository
  that stores files with Git LFS already, as the Files page offers it, sends `feature/run` of `lfs.track` with the
  target `file` (`Controller.storeWithLfsFromExplorer`), its questions asked with VS Code's picks and input boxes; it is
  not offered on a folder, for which **Everything in `<folder>`** of a file of it, or a pattern typed, does (Q1527).
  **Track a pattern…** is on the **Large files** tab.

Acceptance:

- [x] On `art/logo.psd`, **All `.psd` files** runs `git lfs track *.psd` at the top, and `.gitattributes` has
      `*.psd filter=lfs diff=lfs merge=lfs -text` after.
- [x] **This file** on `art/logo [1].psd` runs `git lfs track --filename "art/logo [1].psd"`.
- [x] With `logo.psd` committed before, the message after counts 1 file and offers to store it with Git LFS.
- [x] Not set up, it refuses with **Set up**, and nothing runs.

- As built, the files committed before are read ahead of the plan with the pathspec that takes what the pattern
  takes (`Query::LfsToStore`: `:(glob)**/<pattern>` for a pattern with no `/`, `:(glob)<pattern>` for one with a `/`,
  `:(literal)<path>` for **This file**), so that the message after can count them; the offer gives them to
  `lfs.store-committed`, which does not read them again. **A pattern…** asks the pattern typed; from the page, with
  no file, it is asked at once. **Store with Git LFS…** is in the menu of a file of the Files page only in a
  repository that has patterns of Git LFS already, since Git for Windows sets Git LFS up for every repository and
  would offer it everywhere: a repository that stores nothing with it starts from **Track a pattern…** on the
  **Large files** page. The entry of the menu of the changes is not built yet.

### `lfs.store-committed` — Store with Git LFS the files committed before

Stages again, as pointers, the files git tracks that a pattern of Git LFS takes but that were committed before the
pattern was there, so that the next commit stores them with Git LFS. The commits before keep them as they are.

- **Kind**: Operation
- **Function**: `features::lfs::store_committed(context, answers) -> Outcome`
- **Entry points**: **Store them with Git LFS in the next commit** in the message after `lfs.track`, and in the menu of
  a pattern of the **Large files** page.
- **Offered when**: such files exist.
- **Reads**: `git ls-files -z -- ':(attr:filter=lfs)'`, the files git tracks that a pattern takes, and
  `lfs.read-files`, those of them stored with Git LFS already: the others are the ones to store.
- **Plan**: `git add --renormalize -- <path>…`, with the paths as `changes.pathspec` writes them.
- **Danger**: None.
- **Undo**: Not taken back by Undo: unstaging the files (`changes.unstage`) does.
- **After**: the files are listed in the changes as modified and staged.
- **Uses**: `lfs.read-files`, `changes.pathspec`, `changes.unstage`.
- **Extension**: built: **Store them with Git LFS in the next commit**, offered on the message after `lfs.track` with
  the files it read (the answer `names`, protocol 1.13) and in the menu of a pattern of the **Large files** tab, sends
  `feature/run` of `lfs.store-committed`.

Acceptance:

- [x] With `logo.psd` committed as a plain file and `*.psd` tracked since, it runs
      `git add --renormalize -- logo.psd`, and the index then has a pointer for it.
- [x] A file stored with Git LFS already is not named.
- [x] The commits before keep `logo.psd` as they had it (Q1193).

- As built, `git lfs ls-files --name-only` also lists the files staged as pointers, so a file added again and not
  committed yet is not named twice. With nothing left to store it refuses: **Every file the patterns of Git LFS take
  is stored with it already.**

### `lfs.untrack` — Stop storing a pattern with Git LFS

Takes a pattern out of the `.gitattributes` at the top, so that the files it took are stored by git from their next
commit on.

- **Kind**: Operation
- **Function**: `features::lfs::untrack(context, answers) -> Outcome`
- **Entry points**: **Stop storing with Git LFS** in the menu of a pattern of the **Large files** page.
- **Offered when**: Git LFS is installed and the pattern is in the `.gitattributes` at the top; a pattern of another
  file has **Open `<file>`** only.
- **Plan**: `git lfs untrack <pattern>`, run at the top of the repository.
- **Danger**: None.
- **Undo**: Not taken back by Undo: **Track a pattern…** with the same pattern does.
- **After**: says that the files the pattern took stay as committed until they change, and that `.gitattributes` is to
  be committed.
- **Uses**: `lfs.read-status`, `lfs.track`.
- **Extension**: planned, with `lfs.page`.

Acceptance:

- [x] On `*.psd` it runs `git lfs untrack *.psd`, and the line is no longer in `.gitattributes`.
- [x] A pattern of `assets/.gitattributes` has **Open assets/.gitattributes** and no **Stop storing with Git LFS**.

- As built, the menu of a pattern gives the pattern as the answer `pattern`; with none given it asks among the
  patterns of the `.gitattributes` at the top, and a pattern of another file is refused with where it is.

### `lfs.read-locks` — The locks of the files stored with Git LFS

The files that someone locked on the server, so that two people do not change a file that cannot be merged, with who
holds each lock.

- **Kind**: Read
- **Function**: `git::read::lfs::read_locks(git: &Git, request: &LfsLocksRequest) -> Result<LfsLocks, GitError>`
- **Reads**: `git lfs locks --verify --json`, which asks the server and gives the locks held by this user (`ours`) and
  by others (`theirs`), each with its id, its path, the name of its owner and when it was locked; when the server does
  not verify, `git lfs locks --json`, the locks with no word on whose they are. It talks to the server, so it runs only
  when the **Large files** page opens and on **Refresh** there, never with the reads of the repository (Q1195). A server
  that does not support locks is said so, and its message kept.
- **Uses**: `lfs.read-status`, by `lfs.page`.
- **Extension**: planned, with `lfs.page`.

Acceptance:

- [x] Parsing `{"ours":[{"id":"1","path":"a.psd","owner":{"name":"reviewer"},"locked_at":"…"}],"theirs":[]}` gives
      one lock of this user on `a.psd`.
- [x] A server that answers that it does not support locks gives no lock and that fact.
- [x] Opening the **Large files** page runs it once; a read of the repository never does.

- As built, Git LFS 3.7.1 answers `git lfs locks` with no lock and success when the server answers `404`, cannot be
  reached, or the remote is a folder: only a failure whose message says that locking is not supported is taken for a
  server without locks, and another failure is said as git said it. The JSON is read by a small reader of the crate
  `git`, which may not depend on `crates/json` (rule R1.3). The tests ask a server of the tests on 127.0.0.1, set as
  `lfs.url` of a clone.

### `lfs.lock` — Lock a file stored with Git LFS

Locks a file on the server, so that others know not to change it.

- **Kind**: Operation
- **Function**: `features::lfs::lock(context, answers) -> Outcome`
- **Entry points**: **Lock** in the menu of a file of the **Large files** page.
- **Offered when**: Git LFS is installed, the server supports locks, and the file is not locked.
- **Plan**: `git lfs lock --json <path>`, the path from the top of the repository.
- **Danger**: None.
- **Undo**: Not taken back by Undo: **Unlock** (`lfs.unlock`) does.
- **After**: the page reads the locks again; the file shows this user as its owner.
- **Refuses**: git-lfs's message when the server refuses, such as a file locked by someone else meanwhile.
- **Uses**: `lfs.read-locks`, `lfs.unlock`.
- **Extension**: planned, with `lfs.page`.

Acceptance:

- [x] **Lock** on `art/logo.psd` runs `git lfs lock --json art/logo.psd`, and the page then shows the lock as this
      user's.

### `lfs.unlock` — Unlock a file stored with Git LFS

Takes a lock away: one's own, or, after a warning, someone else's.

- **Kind**: Operation
- **Function**: `features::lfs::unlock(context, answers) -> Outcome`
- **Entry points**: **Unlock** in the menu of a locked file of the **Large files** page.
- **Offered when**: the file is locked.
- **Plan**: `git lfs unlock <path>` for a lock of this user; `git lfs unlock --force --id=<id>` for a lock of someone
  else's (Q1196). When Git LFS refuses a lock of this user because the file has changes not committed, Gitardo offers
  **Unlock anyway**, the same with `--force`.
- **Danger**: None for a lock of this user; Confirm for someone else's, because **The lock is `<owner>`'s, and they
  are not told: they may be changing the file.**
- **Undo**: Not taken back by Undo: **Lock** takes it again, while no one else has.
- **After**: the page reads the locks again.
- **Uses**: `lfs.read-locks`, `lfs.lock`.
- **Extension**: planned, with `lfs.page`.

Acceptance:

- [x] **Unlock** on a lock of this user runs `git lfs unlock art/logo.psd`.
- [x] **Unlock** on a lock of someone else asks with the warning, then runs `git lfs unlock --force --id=<id>`.
- [x] With the file changed and not committed, Git LFS's refusal is followed by **Unlock anyway**.

- As built, a lock whose owner the server did not say (it does not verify) is unlocked as one of this user's, and
  **Unlock anyway** is offered when what Git LFS said names uncommitted changes.

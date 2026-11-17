# Repositories

How a window finds the repositories of its workspace, which one it shows, what it reads of it, and how it keeps that up
to date on its own while the repository and its files change, from Gitardo or from anything else. The tabs of the
repositories a window of Gitardo Desktop keeps open, and what each keeps while another is shown. Also what the activity
indicator of the status bar says Gitardo is doing, the **Activity** drawer, the **Welcome** page of the first start,
and the folders the user trusts to run what they ask.

### `repo.read-repositories` — Find the repositories of a workspace

The repositories a window can show: the one each folder of the workspace is in, the ones directly inside a folder, and
the submodules of all of these that are checked out.

- **Kind**: Read
- **Function**: `git::read::repo::read_repositories(git, request: &RepositoriesRequest) -> Result<Vec<Repository>, GitError>`
- **Reads**: for each folder of the workspace, in their order:
  - the folders directly inside it that have a `.git` (a folder or a file);
  - `git rev-parse --show-toplevel`, run in the folder, and in each of those folders;
  - for each repository found, when it has a `.gitmodules`: `git config --file .gitmodules --get-regexp
    ^submodule\..*\.path$`, keeping the paths that have a `.git` of their own, and the same again in each of them.
- **Uses**: `repo.name-repositories`.

The result follows these rules:

- A folder in a repository gives that repository; a folder in none gives nothing of its own.
- A folder that holds repositories directly inside it but is itself inside a repository further up (the top level of
  that repository is not the folder itself), such as a home folder kept in git, is about the ones it holds: they come
  first, and the repository around it last.
- Otherwise the repository of the folder comes first, then the ones inside it.
- Each repository is followed at once by its submodules, and each submodule by its own.
- A repository is listed once, however many folders lead to it. Two spellings of the same folder are the same: on
  Windows the case and the slashes do not count.
- A folder that cannot be listed, or a `.gitmodules` git cannot read, gives nothing and is no error.

Acceptance:

- [ ] A workspace of one folder that is a repository gives that repository alone.
- [ ] A folder that is no repository and holds `a/` and `b/`, both repositories, gives `a` and `b`, in that order.
- [ ] A home folder that is a repository and holds `projects-x/` with a `.git` gives `projects-x` first and the home
      folder second.
- [ ] A repository with a submodule `lib/core` that is checked out gives the repository and then `lib/core`; one whose
      submodule is not initialized gives the repository alone.
- [ ] Two workspace folders inside the same repository give it once.
- [ ] On Windows, `C:\Work\App` and `c:/work/app` are one repository.

### `repo.name-repositories` — Name the repositories in lists

How a repository is called on its tab (`repo.tabs`), in the repository menu of the extension, in the picker and in
the activity indicator.

- **Kind**: Derivation
- **Function**: `model::repo::name_repositories(roots: &[PathBuf], folders: &[PathBuf]) -> Vec<String>`

The name is the path of the repository from the first workspace folder it is inside, after the name of that folder:
`work/app` for `app` inside the folder `work`. A repository that is a workspace folder itself, or is inside none, is
named by its own folder: `app`. Slashes are forward slashes on every system.

Acceptance:

- [ ] The repository `/home/me/work/app`, with the workspace folder `/home/me/work`, is named `work/app`.
- [ ] The repository that is the workspace folder `/home/me/app` is named `app`.
- [ ] A submodule `C:\work\app\lib\core` under the folder `C:\work` is named `work/app/lib/core`.

### `repo.open-folder` — Open a folder

Shows a folder, or several, in the window: replaces what the window shows, or adds to it.

- **Kind**: Operation
- **Function**: `app::store::open_folder(state: &mut State, folder: Option<PathBuf>) -> Vec<Effect>`, a function of
  the store: the workspace is the window's, not a repository's (Q43)
- **Entry points**: **Open folder…** on the Welcome page, in the menu of **+** at the end of the tabs, in the
  repository menu of the extension, and in Quick search.
- **Asks**:
  1. The folder, in the system's folder dialog.
  2. When the window already shows a workspace: **Replace the workspace** or **Add to the workspace** (default:
     replace).
  3. Which of its repositories open in tabs, as `repo.choose-tabs` says.
- **Reads**: `repo.read-repositories` for the folders of the new workspace.
- **Plan**: no step: nothing runs in a repository.
- **Danger**: None.
- **Undo**: nothing in a repository changes; the workspace before is among the recent ones.
- **After**: the window shows the new workspace, with the tabs it had or those chosen, and the repository `repo.select`
  remembers for it, or the first one; the workspace goes first among the recent ones of the Welcome page. A folder
  added keeps the tabs open and opens its own after them. The window shows what it showed until the repositories are
  found and chosen.
- **Uses**: `repo.read-repositories`, `repo.read-state`, `repo.choose-tabs`, `welcome.page`.

Acceptance:

- [ ] Opening a folder with two repositories, both chosen, shows the first one and has a tab for each.
- [ ] Adding a folder to a workspace keeps the repository that was shown.
- [ ] Opening a folder with no repository shows `repo.no-repository`.
- [ ] The workspace opened is the first of the recent workspaces on the Welcome page.

### `repo.open-recent` — Open a recent workspace

Opens again a workspace the window showed before, with the tabs it had and the repository that was shown in it.

- **Kind**: Operation
- **Function**: `app::store::open_recent(state: &mut State, index: usize) -> Vec<Effect>`, a function of the store
  (Q43)
- **Entry points**: the list of recent workspaces on the Welcome page; **Open recent** in Quick search.
- **Asks**: the workspace, from Gitardo's picker of the recent ones, the latest first (when not chosen on the page).
- **Reads**: `repo.read-repositories`.
- **Plan**: no step.
- **Danger**: None.
- **Undo**: nothing in a repository changes.
- **After**: the window shows that workspace, with the tabs it had, those whose repositories are still there, and the
  repository shown last in it. A workspace last opened before there were tabs opens a tab for each of its repositories.
- **Refuses**: a workspace whose folders are all gone: "None of the folders of this workspace is there any more." The
  workspace is taken out of the list.

Acceptance:

- [ ] A workspace opened before is listed, and opening it shows the tabs it had and the repository that was shown in it
      last.
- [ ] A workspace whose folder was deleted is refused with that message and leaves the list.

### `repo.select` — Choose the repository shown

Gitardo shows one repository at a time; in a workspace with several, this chooses which.

- **Kind**: Operation
- **Function**: `app::store::workspace::select(state: &mut State, folder: &Path) -> Vec<Effect>`, a function of the
  store (Q43)
- **Entry points**: the tabs (`repo.switch-tab`), and the repository menu of the header in the extension; **Select
  repository…** in the **…** menu of the header; **Select Repository** in Quick search. In Gitardo Desktop, choosing a
  repository of a tab shows that tab, as `repo.switch-tab` does.
- **Offered when**: the workspace has more than one repository, or the window shows a worktree of a repository of the
  workspace that is not in the workspace itself (see `worktree.switch`). The repository menu of the extension lists
  the repositories to choose from only then.
- **Asks**: the repository, in Gitardo's picker titled **Repository shown in Gitardo**: each repository by its name,
  with its folder under it, and the one shown marked **current**; a worktree shown from outside the workspace is listed
  last as **worktree, current**. The text typed matches the names and the folders.
- **Plan**: no step: nothing runs in a repository.
- **Danger**: None.
- **Undo**: nothing in a repository changes; choosing the other one again goes back.
- **After**: the branch rail, the timeline and the draft show the repository chosen: its own branches shown
  (`filter.remember`), the first page of its commits, nothing searched and nothing selected; with
  `repositories.keepOpen` on, what it showed when it was left, as `repo.keep-open` says. The choice is remembered
  for the workspace, and is the one shown the next time the workspace opens, as long as it is still among its
  repositories.
- **Refuses**:
  - while an operation runs: nothing happens; in Gitardo Desktop the tab is shown all the same, as `repo.switch-tab`
    says;
  - from Quick search, with a single repository: "Gitardo: this workspace has a single repository."; with none:
    "Gitardo: no Git repository in this workspace."
- **Uses**: `repo.read-state`, `repo.name-repositories`, `filter.remember`.

Acceptance:

- [ ] In a workspace of two repositories, choosing the second shows its branches and commits, and the next time the
      workspace opens it is the one shown.
- [ ] A remembered repository that is no longer in the workspace gives way to the first one found.
- [ ] With one repository the island has one tab, and the command says "Gitardo: this workspace has a single
      repository."
- [ ] Choosing the repository already shown reads nothing.
- [ ] While an operation runs, choosing another repository does nothing.

### `repo.owner-of-file` — The repository a file is in

When a file is chosen for its history or its blame, the window switches on its own to the repository that has it.

- **Kind**: Derivation
- **Function**: `model::repo::owner_of_file(path: &Path, repositories: &[Repository]) -> Option<&Repository>`
- **Uses**: by `file-history.file`, `blame.toggle-file` and the entries of the Files page.

The owner is the innermost repository of the workspace whose folder contains the file: of a file in a submodule, the
submodule. A path that is the folder of a repository itself is in no repository but is that repository (for the history
of a folder, `file-history.folder`). A file in no repository of the workspace has no owner.

Acceptance:

- [ ] `app/lib/core/x.rs`, with the repositories `app` and its submodule `app/lib/core`, is owned by `app/lib/core`.
- [ ] `app/README.md` is owned by `app`.
- [ ] A file outside every repository has no owner, and the repository shown stays.

### `repo.no-repository` — No repository in the workspace

What the window shows when the workspace has no repository, or the one it had cannot be read.

- **Kind**: View
- **Function**: `views::repo::no_repository_view(state: &NoRepositoryState, ui) -> Vec<Intent>`
- **Entry points**: the timeline and the branch rail, in place of what they list.

The timeline says **No Git repository in this workspace** with a **Check again** button, which looks for the
repositories again (`repo.read-repositories`) and reads the first one. While the repositories are looked for and the
first one is read, it says **Loading repository…**. A read that fails, as when the folder was deleted, comes here too;
the error goes to the Command log.

Acceptance:

- [ ] A workspace of a folder with no repository shows **No Git repository in this workspace** and **Check again**.
- [ ] Running `git init` in the folder and clicking **Check again** shows the new repository with no commits.
- [ ] Deleting the folder of the repository shown brings the window to this page.

### `repo.read-state` — Read the state of the repository

Everything the window shows of a repository but its commits: what is checked out, the branches and how far each is from
its upstream, the remotes, the tags, the stashes, the worktrees, the submodules, what is not committed, and the
operation that stopped half way.

- **Kind**: Read
- **Function**: `git::read::repo::read_state(git, request: &StateRequest) -> Result<RepositoryState, GitError>`
- **Reads**: all at once, with `GIT_OPTIONAL_LOCKS=0`, in the folder of the repository:
  - `git for-each-ref --format=…` with, NUL-separated, `%(refname)`, `%(objectname)`, `%(*objectname)`,
    `%(upstream)`, `%(upstream:remotename)`, `%(upstream:remoteref)`, `%(upstream:track)`, `%(HEAD)` and
    `%(creatordate:unix)`: the local branches with their upstream (its name, its remote, its branch there, how many
    commits the branch has that it has not and the other way round, and whether it is gone), the remote branches but
    `<remote>/HEAD`, and the tags at the commit they point at (an annotated tag at the commit it peels to). Each has the
    date of its commit, or the date an annotated tag was made.
  - `git rev-parse --verify --quiet HEAD`: the commit checked out, none before the first commit.
  - `git symbolic-ref --quiet HEAD`: the branch checked out by its full name, none when detached. (The full name, as a
    branch and a tag can share a name.)
  - `git stash list -z --format=%H%x1f%gd%x1f%gs%x1f%at`: each stash, its `stash@{n}`, its message and date.
  - `git worktree list --porcelain`: each worktree, its folder, its commit, its branch or none, whether it is the main
    one (listed first), whether it is this one and whether it is missing (`prunable`). A bare repository is not listed.
    Then, in each other worktree that is not missing, `git status --porcelain=v2 -z --untracked-files=all` to count
    what is not committed there; this one's count is the one of this read. That `git status` runs on the first read of
    the repository, on **Refresh**, and when something in the worktree's own git folder (`.git/worktrees/<name>/`)
    changed since the read before (`repo.watch-changes`); otherwise the worktree keeps the count read before (M15).
  - `git remote`: the names of the remotes.
  - `git config --get-regexp ^user\.(name|email)$`: who commits are made as.
  - `git config --get branch.sort`: Git's own order of the branches, which `refs.sort` follows while
    `branches.sortOrder` is not set.
  - `git reflog --max-count=300 --format=%gs HEAD`: where HEAD has been, for `refs.recent`. Nothing before the first
    commit.
  - `git status --porcelain=v2 --branch -z --untracked-files=all`: the files staged, the files changed and not staged,
    the untracked files (every file, not their folders), and the files in conflict (`DD`, `AU`, `UD`, `UA`, `DU`, `AA`,
    `UU`). A file changed in both the index and the working tree is in both lists; a rename keeps its old name.
  - The operation in progress, from the files git leaves in the folder of this worktree (`git rev-parse
    --absolute-git-dir`), in this order: `rebase-apply/applying` is a patch (`git am`); `rebase-merge` or
    `rebase-apply` a rebase; `MERGE_HEAD` a merge; `CHERRY_PICK_HEAD` a cherry-pick; `REVERT_HEAD` a revert;
    `sequencer/todo` a cherry-pick, or a revert when its first line starts with `revert`, between two commits of a
    series.
  - During a rebase, whether it stopped on an `edit` to change a commit, or to split one: see `conflicts.continue-operation` and
    `split.commit`.
  - The bisect in progress (`BISECT_START`), with its marks: see `bisect.start`.
  - The submodules: see `submodule.read-list`.
  - With the setting `conflicts.autoResolve`, what can be resolved without a choice in the files in conflict: see
    `conflicts.resolve-by-rules`.
- **Settings**: `readOnStartup` (on by default): when the window opens a workspace as it starts, the repository shown is
  read at once, so that the timeline and the branch rail have it when the window appears. Off, in the user's settings or
  in `.gitardo/settings.json` of a very large repository, the window opens with the repository named but not read, and
  the timeline and the branch rail say **Not read yet** with **Read the repository**; the first click there,
  **Refresh**, or any command that needs the state reads it. GitHub is never asked for the pull requests before the
  first read.
- **Uses**: `repo.schedule-reads`, `graph.read-commits`, `refs.recent`, `submodule.read-list`,
  `conflicts.resolve-by-rules`.

Acceptance:

- [ ] In a repository with no commits, the state has no commit checked out, the branch git will create (`main` or as
      `init.defaultBranch` says), no reflog, and the untracked files.
- [ ] Detached at a commit, the state has that commit and no branch.
- [ ] A branch `release` and a tag `release` are both read, the branch as checked out when it is.
- [ ] A branch two commits ahead of its upstream and one behind reads `ahead 2`, `behind 1`; one whose upstream was
      deleted on the remote and pruned reads as gone.
- [ ] An annotated tag is read at its commit, with the date it was made.
- [ ] `origin/HEAD` is not among the remote branches.
- [ ] A file staged and changed again is both staged and not staged; a file in conflict is only in conflict.
- [ ] Stopped in a cherry-pick, the operation is a cherry-pick; between two commits of a revert of a series, it is a
      revert; during `git am`, a patch, not a rebase.
- [ ] Another worktree with three files not committed counts 3; a missing one counts nothing and is marked missing.
- [ ] With five other worktrees and nothing changed in their git folders, a read after a change to a file of this one
      runs `git status` here only, and each other worktree keeps its count; after `git add` in one of them, the next
      read runs `git status` there too; **Refresh** runs it in all of them.
- [ ] No read takes `index.lock`: while it runs, `git commit` in a terminal does not fail because of it.
- [ ] With `readOnStartup` off, starting the window runs no git command until **Read the repository** is clicked.

### `repo.common-folder` — The git folder the worktrees share

The git folder every worktree of a repository shares, where its references live: the `.git` of the main worktree,
whichever worktree is shown. The extension watches it for the changes of the references (Q398).

- **Kind**: Read
- **Function**: `git::read::repo::read_common_folder(git: &Git, request: &CommonFolderRequest) -> Result<PathBuf, GitError>`
- **Reads**: `git rev-parse --git-common-dir`, which gives the folder from the folder git runs in or in full; it is
  made whole from there, without its `.` and `..`.

Acceptance:

- [ ] In the main worktree, it is its `.git`.
- [ ] In a linked worktree, it is the `.git` of the main worktree, not the worktree's own git folder.
- [ ] From a folder inside the working tree, it is the same folder, in full.

### `repo.watch-changes` — Which changes ask for a read

The repository is read again on its own when it or its files change, from Gitardo, a terminal or another program.
This says which changes count, and how soon.

- **Kind**: Derivation
- **Function**: `model::repo::watch_changes(change: &Change) -> Option<ReadRequest>`
- **Uses**: by `repo.schedule-reads`.

What is watched, and what each change asks for:

| Change | Asks for a read | After | Said as |
| --- | --- | --- | --- |
| In the common git folder (`git rev-parse --git-common-dir`): `HEAD`, `FETCH_HEAD`, `ORIG_HEAD`, `MERGE_HEAD`, `CHERRY_PICK_HEAD`, `REVERT_HEAD`, `BISECT_LOG`, `index`, `config`, `packed-refs`, anything under `refs/`, `logs/refs/stash`, anything under `worktrees/`, and `modules/**/HEAD`, `modules/**/refs/**` of the submodules | yes | 400 ms | `.git/<path> changed` |
| A file of the working tree, anywhere but in a `.git`, that git does not ignore and that is not in the folder of another worktree nested in this one | yes | 1,000 ms (builds and installs write many files at once) | `<path> changed` |
| A file of the working tree that git ignores (`git check-ignore --stdin`, asked once for the files of a burst), such as the output of a build | no | | |
| A file in the folder of another worktree nested in this one, such as `.claude/worktrees/<name>/` | no | | |
| A file saved by Gitardo's own editor | yes | 400 ms | `<path> was saved` |
| Files created, deleted or renamed by Gitardo | yes | 400 ms | `files were created` (`deleted`, `renamed`) |
| The window gets the focus, after a change that asked for nothing while the window was minimized, or more than a minute after the last read | yes | 400 ms | `the window got the focus` |
| The window gets the focus otherwise | no | | |
| A setting under `conflicts`, `pull`, `worktrees`, `branches` or `graph` changes | yes | 400 ms | `the settings changed` |
| Any path with a part named `fsmonitor--daemon`, or `fsmonitor--daemon.ipc` | no | | |
| Any file whose name ends in `.lock` | no | | |

Files git ignores and the folders of nested worktrees are left out because what changes there never changes what a read
shows of this repository, and a build or an agent working in a nested worktree writes there many times a second
(M15). A `.gitignore` that changes is not ignored itself, and asks for a read. A nested worktree is seen by its own
git folder, `.git/worktrees/<name>/`, which asks for a read with the rules of the first row.

The last two rules are what keep a repository from being read for ever: the daemon of `core.fsmonitor` leaves a file
in the folder of a worktree (`.git/worktrees/<name>/fsmonitor--daemon/…`) each time `git status` runs there, as it
does in every read, and on Linux and macOS listens on a socket beside it (`fsmonitor--daemon.ipc`); and a lock is
always followed by the file it locked.

Acceptance:

- [ ] `git commit` in a terminal asks for a read 400 ms after `.git/HEAD` or the branch's ref changes, said as
      `.git/refs/heads/main changed` (or the file that changed first).
- [ ] Saving a file from another editor asks for a read after 1,000 ms.
- [ ] `cargo build` writing in `target/`, which `.gitignore` lists, asks for nothing; changing `.gitignore` asks for a
      read.
- [ ] A file written in a worktree whose folder is inside this one's asks for nothing; `git commit` there asks for a
      read, by `.git/worktrees/<name>/`.
- [ ] Focus taken and given back to the window ten seconds after a read asks for nothing; two minutes after it, it asks
      for a read; after the window was minimized while a file changed, it asks for a read.
- [ ] A change to `.git/worktrees/feature/fsmonitor--daemon/cookies/…` asks for nothing.
- [ ] A change to `.git/worktrees/feature/fsmonitor--daemon.ipc` asks for nothing.
- [ ] A change to `.git/index.lock` asks for nothing.
- [ ] With `core.fsmonitor=true` and nothing changing, the repository is read once and then not again.

### `repo.schedule-reads` — Read again when something changed

Decides when the repository is read: once after a burst of changes, never twice at once, again while something keeps
changing during each read, never during an operation. **Refresh** asks for a read at once.

- **Kind**: Derivation
- **Function**: `model::repo::schedule_reads(schedule: &ReadSchedule, event: &ReadEvent) -> (ReadSchedule, Vec<ReadAction>)`
- **Entry points**: **Refresh**, the first entry of the **…** menu of the header; **Refresh** in Quick search; and
  every change `repo.watch-changes` counts.
- **Settings**: `readOnStartup`, for the first read (see `repo.read-state`).
- **Uses**: `repo.watch-changes`, `repo.read-state`, `graph.read-commits`, `activity.describe-task`.

Each tab has a schedule of its own, kept with what the tab keeps (`repo.keep-open`): these rules are those of the tab
shown, and the tabs not shown follow `repo.watch-hidden`.

The rules:

- A change asks for a read after its delay, counted from the first change of a burst: further changes do not push it
  back, however long the burst lasts, but one with a shorter delay brings it forward. The read is said to be for the
  last change of the burst.
- Changes ask for nothing while the window is minimized, so that nothing of the repository is in sight, and while an
  operation runs: the operation is followed by a read of its own.
- **Refresh**, the end of an operation, choosing a repository, the branches shown, a search, a filter or the next page
  ask for a read at once. A tab shown again that kept what it showed is read only when it is out of date
  (`repo.watch-hidden`).
- Only one read runs at a time. A read asked for while one runs is noted, and one more read follows the one under way,
  however many were asked for meanwhile. That goes on while something changes during each read; each is counted
  ("read 3 times in a row").
- Who asked for a read waits for one that started after they asked, not for all the reads that follow it.
- A read that ran while an operation ran, or while the window was pointed at another repository, is thrown away and
  read again: it can be half from before and half from after, such as a new commit that no branch points at yet.
- The bar along the top of the list of commits runs while the commits of a filter, of the branches just chosen or of
  the next page are read, and stops as soon as the read that has them is done, even when the files go on changing and
  the repository is read again behind it.
- A read that fails while a search narrows the timeline (git refuses the search, as an invalid regular expression) drops
  the search, says "Gitardo: the search failed." with the first line of git's error, and reads again without it. Any
  other failure shows `repo.no-repository`.
- A read whose commit checked out, references and stashes are those of the read before keeps the commits it has and
  runs no `git log`: a change to the files leaves the history as it was. **Refresh**, a filter, a search, the branches
  shown and the next page read the commits as before (M15).
- After each read, the pull requests are read again when they are older than 2 minutes (`github.load-pull-requests`).

Acceptance:

- [ ] Ten files written within half a second lead to one read, about a second after the first.
- [ ] A file that changes every 200 ms for 5 s leads to reads one after another, never two at once, and the activity
      indicator counts them.
- [ ] **Refresh** during a read leads to exactly one more read after it.
- [ ] A file changed and saved leads to a read that runs no `git log`, and the timeline keeps its rows; a commit made in
      a terminal leads to a read that reads the commits again; so does **Refresh**.
- [ ] Nothing is read while an operation runs; one read follows it.
- [ ] A read started before a commit made by Gitardo is not shown: the state shown after the commit has the commit
      on its branch.
- [ ] Choosing **Local** in the branch filter shows the bar until the commits of the local branches are there, even
      while a build in the working tree keeps changing files.

### `activity.describe-task` — Say what Gitardo is doing

The words the activity indicator of the status bar and the **Activity** drawer use for each thing that takes time.

- **Kind**: Derivation
- **Function**: `model::activity::describe_task(task: &Task, now: Instant) -> TaskText`
- **Uses**: by `activity.status-bar` and `activity.what-is-running`.

| What | Title | Detail |
| --- | --- | --- |
| A read of the repository | `Reading <name>`, or `Searching <name>` while a search in the changes (`git log -S` or `-G`) narrows the timeline | `Because <cause>.` for the change that asked for it; from the second read in a row, `Read <n> times in a row: something changed during each read.` before it |
| An operation | its title, as the preview shows it | |
| A comparison | `Comparing <from> ↔ <to>` | |
| A patch being made | `Making a patch of <commits>` | |
| The pull requests | `Reading the pull requests from GitHub` | |
| A command of a new worktree, in Tasks | `Running <command> in <worktree>` | `A command of a new worktree, in Tasks` |

`<name>` is the one `repo.name-repositories` gives. How long it has run is said in whole seconds, `12 s`, and from a
minute on as `1 min 5 s`.

Acceptance:

- [ ] The third read in a row after `src/a.rs` changed says `Read 3 times in a row: something changed during each
      read. Because src/a.rs changed.`
- [ ] 65 seconds is said as `1 min 5 s`, 59.9 seconds as `59 s`.
- [ ] A read while the timeline is narrowed to `git log -S foo` is titled `Searching <name>`.

### `activity.status-bar` — What is running, in the status bar

The status bar at the foot of the window says what Gitardo is running, for how long, and why, what the last operation
said (`activity.said`), and the identity commits are made with.

- **Kind**: View
- **Function**: `views::activity::status_bar_view(state: &StatusBarState, ui) -> Vec<Intent>`
- **Entry points**: the status bar.
- **Uses**: `activity.describe-task`, `activity.what-is-running`, `activity.said`, `identity.switch`.

- While Gitardo has been reading a repository, running an operation, comparing two sides, making a patch or asking
  GitHub for more than half a second, the activity indicator shows a spinning icon with the title of what has run
  longest and for how long, as `Reading app (2 s)`, at the start of the status bar. What is over sooner never shows,
  so that it does not flicker. Where its words would be cut short it shows its icon alone.
- The end of the status bar says the identity commits are made with in the repository shown, `Ada
  <ada@example.com>`, the name or the email alone, or **No identity configured**, when there is room for it after
  what runs; a click switches it (`identity.switch`), except while an operation runs.
- Its tooltip lists everything that runs, the oldest first, each with its title, how long it has run and its detail,
  and ends with **Click to open the Activity drawer.** The times go on every half second; the tooltip is
  drawn again only when its text changes.
- A click opens the Activity drawer (`activity.what-is-running`).
- The status bar keeps its height whatever it says: what was said and its links are drawn in its text
  ([D56](../08-decisions.md#d56--a-side-column-of-repositories-and-branches-what-was-said-in-the-status-bar-diffs-beside-what-opened-them)).

Acceptance:

- [ ] A read of 0.3 s shows nothing; a read of 2 s shows the activity indicator from 0.5 s until it ends.
- [ ] A fetch and a read at once are both in the tooltip, the fetch first if it started first.
- [ ] The tooltip of a read asked for by a change says which file changed.
- [ ] In a window 1000 pixels wide, a read of 2 s says `Reading app (2 s)` in the status bar, and the identity is at
  its end; a click on the identity asks to switch it.
- [ ] With `main` two ahead of `origin/main`, the branch of the header says `main ↑2 ↓0`.

### `activity.said` — What was said, in the status bar

What an operation said when it ended, and the notices of the window, in the status bar, with **Undo** and what it offers
next, in place of the bars that were under the header
([D56](../08-decisions.md#d56--a-side-column-of-repositories-and-branches-what-was-said-in-the-status-bar-diffs-beside-what-opened-them)).

- **Kind**: View
- **Function**: `views::activity::said_view(state: &SaidState, ui) -> Vec<Intent>`
- **Entry points**: the status bar, after what is running.
- **Uses**: `undo.last`, `conflicts.stopped`, `activity.status-bar`, `log.show`.
- **Extension**: VS Code's notifications say it: nothing to build.

- After the indicator of what is running, or at the start of the status bar when nothing runs: the icon of its tone
  (done, warning, error), then its words, in the text of the status bar, cut with `…` where the room ends, and whole in
  its tooltip. Then its actions, as links in the same text: **Undo** when it can be taken back, what it offers next
  (**Create pull request**, **Show conflicts**), and **×**, which puts it away. The identity at the end of the status
  bar gives up its room first, then the words of what is running, which keeps its icon.
- What succeeded leaves after ten seconds, or ten seconds after the pointer left it. A warning or an error stays until
  **×**, the next operation, or the repository shown changes; an error is in the color of errors.
- A click on its words opens the Command log of the bottom panel at the command that said it (`log.show`).
- The notices of the window (`None of the folders of this workspace is there any more.`, `This file looks binary.`)
  are said in the same place, in the same way, with the tone of a warning.
- What an operation that stopped on conflicts says (`conflicts.stopped`) is said here too; the banner of the operation
  that stopped stays at the top of the timeline (`conflicts.banner`).
- Nothing is said in a bar under the header any more, and the status bar is as high as it was.

Acceptance:

- [ ] After a pull that succeeded, the status bar says what it pulled, with **Undo**, and is as high as before.
- [ ] Ten seconds later, without the pointer on it, it is gone.
- [ ] After a push of a branch of a repository on GitHub, it offers **Create pull request**, which opens the page.
- [ ] A failed push says why in the color of errors, and is there until **×**.
- [ ] Words longer than the room are cut with `…`, and the tooltip has them whole.
- [ ] A click on the words opens the Command log at that command.
- [ ] No bar is drawn between the header and the timeline after an operation.

### `activity.what-is-running` — What is running, in the Activity drawer

Lists what Gitardo is doing now, with the way to the commands it runs.

- **Kind**: View
- **Function**: `views::activity::what_is_running_view(state: &ActivityState, ui) -> Vec<Intent>`
- **Entry points**: a click on the activity indicator of the status bar; **Activity** in the bar of views; **Show
  What Is Running** in Quick search.
- **Uses**: `activity.describe-task`, `log.show`, `panel.bottom`.

The drawer lists each thing that runs, with its title, how long it has run and its detail, the times going on every
second while it is open, and the list following what starts and ends. With nothing running it says **Nothing is running
now** and "Gitardo is not reading or changing anything". Under the list, **Show the commands Gitardo runs** shows the
bottom panel at the Command log (`panel.bottom`), which is no longer in the drawer. Opening the drawer while it is open
leaves it as it is.

Once a new worktree has run commands (`worktree-setup.command-steps`), the drawer, higher, has **Tasks** at its right:
a row for each of the latest commands, with its worktree and how it stands (a spinner while it runs, **waits**, a
check, **exit code `<n>`**, **stopped**, **did not run**); below, what the task shown printed, following its end, the
colors of a terminal left out; a click on a row shows what that one printed. While the task shown runs, a box under
its output answers what it asks, `Enter` giving it the line typed, and **Stop** stops it, so that those after it do
not run.

Acceptance:

- [ ] Opened during a long fetch, the drawer lists the fetch, and its time grows each second.
- [ ] When the fetch ends, it leaves the list; with nothing else, the drawer says **Nothing is running now**.
- [ ] **Show the commands Gitardo runs** shows the bottom panel at the Command log.

### `welcome.page` — The Welcome page

On the first start, a page that shows in five steps where the timeline, the branch rail and the inspector are and
what to try first, and opens a folder.

- **Kind**: View
- **Function**: `views::welcome::page_view(state: &WelcomeState, ui) -> Vec<Intent>`
- **Entry points**: shown by itself the first time Gitardo Desktop starts; **Welcome** in Quick search and in the
  Help menu; the window with no workspace.
- **Uses**: `repo.open-folder`, `repo.open-recent`, `filter.show-only`, `commit-page.open`, `compare.choose`,
  `blame.toggle-file`, `worktree.create`.

Its title is **Get started with Gitardo**. At the top, **Open folder…** and the recent workspaces, the latest first,
each by its folders, opening it on a click. Then the five steps, each with a short text, a picture and a button; a step
is ticked once it is done, in any way, and stays ticked:

| Step | Says | Button | Ticked when |
| --- | --- | --- | --- |
| **Open the timeline** | The branches on the left, the commits of every branch in the middle, the selected commit on the right | **Show the timeline** | the timeline is shown |
| **Find the branches** | The branch rail lists the branches in the colors of their lanes, and keeps the stashes, the worktrees and the tags aside; a branch picked there narrows the timeline to its history | **Show the commits**, which narrows the timeline to the branch checked out (`filter.show-only`) | the timeline is narrowed to the branch checked out |
| **Open a commit, compare two branches** | A double click or `Enter` on a commit reads it whole on a page; a comparison says what one side has that the other has not, and what merging them would do | **Open a commit…**, **Compare…** | a commit page or a comparison is opened |
| **See who changed each line** | The File page says who last changed each line | **Open a file…** | a blame is shown |
| **Work in another worktree** | A worktree is another folder of the same repository on a branch of its own | **Create a worktree…** | a worktree is created |

Acceptance:

- [ ] The first start shows the Welcome page; the second does not.
- [ ] **Welcome** in Quick search shows it again, with the steps done ticked.
- [ ] Opening a commit with a double click in the timeline ticks **Open a commit, compare two branches**.
- [ ] A workspace opened from the page is listed first the next time.

## Repository tabs

Gitardo Desktop keeps several repositories open in one window, each a **tab**, and shows one at a time
([D50](../08-decisions.md#d50--repository-tabs-and-what-each-repository-keeps)). Since milestone M16 a tab is a row of
**Repositories**, the first section of the side column, and no longer a tab of an island above the header
([D56](../08-decisions.md#d56--a-side-column-of-repositories-and-branches-what-was-said-in-the-status-bar-diffs-beside-what-opened-them)).
A tab holds a repository with its worktrees and its submodules (`repo.group-tabs`). The tab's are: the header, the
branches of the side column, the timeline, the inspector, the pages and the Changes page. The window's are: the bar of
views, **Repositories**, the Activity drawer with the Tasks, the bottom panel's Command log, the status bar, the
settings and the dialogs.

The extension has no tabs: VS Code's views have the list of `repo.select` for that. It keeps what each repository
showed in the same way, behind the same setting, off by default there (`repo.keep-open`).

### `repo.tabs` — The repositories open, in the side column

**Repositories**, the first section of the side column: a row for each repository open, the one shown marked, what
each is on, and **+** to open another.

- **Kind**: View
- **Function**: `views::repo::tabs_view(state: &TabsState, ui: &mut Ui) -> Vec<Intent>`
- **Entry points**: **Repositories** at the top of the side column (`repo.side-column`). `Ctrl+1` to `Ctrl+8` show the
  repository at that place and `Ctrl+9` the last one; `Ctrl+PageDown` and `Ctrl+PageUp` the next and the one before,
  going round. On macOS `Cmd+1` to `Cmd+9`, `Cmd+Shift+]` and `Cmd+Shift+[`.
- **Offered when**: a workspace is open, even with a single repository, so that **+** is there.
- **Uses**: `repo.switch-tab`, `repo.open-tab`, `repo.close-tab`, `repo.group-tabs`, `repo.name-repositories`,
  `repo.worktree-summary`.

The heading of the section says **Repositories**, with the chevron that folds it on its left and **+** on its right
(`repo.open-tab`). Under it, a row for each repository open, in the order they were opened, each two lines high:

- on the first line, the icon of a repository, its name from `repo.name-repositories`, and, while it shows one of its
  worktrees or submodules, ` › ` and the name of that one, as `repo.group-tabs` says; then, on the right, a spinner
  while an operation runs in it, and its **▾** when it has worktrees or submodules, which opens the menu of
  `repo.group-tabs`;
- on the second line, small and muted, what it shows is on (`repo.worktree-summary`): the branch checked out, or
  `detached at 3f2a1c9`; `● 3` when 3 files are changed and not committed, in the color of changes; `↓2 ↑1` when there
  are commits to pull and to push. What is not there is not said: a repository up to date with nothing changed says its
  branch alone.

The row shown has the background of a row selected, and a bar in the color of the accent on its left. A click on a row
shows it (`repo.switch-tab`). The **×** of a row is on the row under the pointer and the row shown; a middle click
closes it; its context menu has **Close tab** and **Close other tabs** (`repo.close-tab`), **Reveal in the file
manager** and **Copy path**. Its tooltip is the folder of what it shows. A name too long for the row is cut at its start
with `…`, so that the name of the repository's own folder stays. When the rows do not fit the height of the section, it
scrolls.

Acceptance:

- [ ] A window with `api`, `web` and `docs` open, `web` shown, lists three rows under **Repositories** with `web`
      marked, and **+** in the heading.
- [ ] Clicking `docs` shows it; `Ctrl+1` shows `api`; `Ctrl+PageDown` from `docs` shows `api`.
- [ ] `web` on `main`, with 3 files changed and 2 commits to pull, says `main ● 3 ↓2` under its name.
- [ ] `api` on `main`, up to date and with nothing changed, says `main` alone.
- [ ] A repository that runs an operation shows a spinner while another is shown.
- [ ] With twelve repositories open in a section 200 logical pixels high, the section scrolls to the last one.
- [ ] A name longer than its row is cut at its start with `…`, and the tooltip has the whole folder.
- [ ] There is no island above the header.
- [ ] The snapshots of the section, with one repository and with several, in every theme, at 100 % and 200 %, match.

### `repo.side-column` — The side column of repositories and branches

One island on the left of the window, the whole height of it, with **Repositories** above and **Branches** under it,
each folding by its heading, the line between them dragged
([D56](../08-decisions.md#d56--a-side-column-of-repositories-and-branches-what-was-said-in-the-status-bar-diffs-beside-what-opened-them)).

- **Kind**: View
- **Function**: `views::repo::side_column_view(state: &SideColumnState, ui: &mut Ui) -> Vec<Intent>`
- **Entry points**: the left of the window, beside the bar of views, under the header; it is put away and brought
  back by the first button of the top row of the timeline (`graph.toggle-references`).
- **Uses**: `repo.tabs`, `refs.tree`, `graph.resize-panes`, `graph.toggle-references`.
- **Extension**: VS Code's Source Control side bar has the repositories, and the Branches view the branches: nothing to
  build.

- The island goes from under the header to the status bar; the timeline, the inspector, the pages and the bottom
  panel are on its right, and the header above them all (`graph.toolbar`).
- **Repositories** (`repo.tabs`) is above, **Branches** (`refs.tree`, the branch rail as it was, with its filter and
  its scope line) is under it. Each has a heading in capitals of the size of the headings of the rail, with a chevron
  that folds and unfolds the section; a folded section is its heading alone, and the other takes the height.
- The line between them is dragged to share the height (`graph.resize-panes`), the arrows moving it with the keyboard.
  **Repositories** starts at 160 logical pixels, or as tall as its rows when they need less; neither is shorter than 64
  logical pixels while both are open.
- Which sections are folded and the height of **Repositories** are remembered for the workspace.
- The side column stays beside the timeline and beside every page, except a Diff page beside the inspector
  (`diff.beside-inspector`) or beside a comparison (`compare.beside-diff`), which put it away while they are shown and
  give it back as it was.
- In the narrow layout, below 900 logical pixels, it is put away, and the first button of the top row of the
  timeline opens it over the timeline, as wide as it was, until a click outside it or `Escape`.

Acceptance:

- [ ] The side column's top is under the header, level with the timeline's top, and its foot touches the status
      bar's gap.
- [ ] Folding **Branches** gives **Repositories** the whole height but the heading of **Branches**.
- [ ] Folding both leaves two headings at the top, and the rest of the island empty.
- [ ] Dragging the line between the sections changes their heights, and they are the same after the window opens
      again.
- [ ] The side column stays beside the Changes page and the settings page.
- [ ] In a window 800 logical pixels wide, the side column is not shown, and the first button of the top row of the
      timeline opens it over the timeline.

### `repo.worktree-summary` — What a repository or a worktree is on

The branch checked out in a repository or a worktree, how many files are changed there, and how many commits are to
pull and to push, for the rows of **Repositories** and the entries of their **▾**.

- **Kind**: Read
- **Function**: `git::read::worktree_summary(folder: &Path) -> WorktreeSummary`
- **Reads**: `git status --porcelain=v2 --branch --untracked-files=normal` in the folder: the branch or the detached
  commit from its `# branch.head` and `# branch.oid` lines, the commits ahead and behind from `# branch.ab`, the files
  changed from the count of its other lines.
- **Uses**: `repo.common-folder`, `repo.watch-changes`; by `repo.tabs` and `repo.group-tabs`.

- The repository shown takes its summary from its own read (`repo.read-state`), and runs no other.
- Every other repository open and every worktree of each is read once when the window opens, then again only when its
  git folder changes (its `HEAD`, its index, its refs) or the files of its folder change, as `repo.watch-changes` says,
  and not more than once in two seconds for each folder. The reads run in the background, one at a time, after the reads
  of the repository shown.
- A worktree whose folder is gone is said `missing`; a read that fails leaves the last summary, muted.
- Ahead and behind are counted from the upstream of the branch, as `git status` does; a branch without an upstream says
  neither.

Acceptance:

- [ ] A worktree on `feature/login`, with two files changed and one untracked, one commit ahead of its upstream, reads
      as `feature/login`, 3 files, `↑1`.
- [ ] A worktree detached at `3f2a1c9` reads as `detached at 3f2a1c9`.
- [ ] Saving a file in a worktree not shown updates its count within two seconds.
- [ ] Nothing in a worktree changing runs no read of it.
- [ ] A worktree whose folder was deleted is said `missing`.

### `repo.open-tab` — Open a repository in a tab

Opens a repository of the workspace, or of a folder added to it, in a tab of its own.

- **Kind**: Operation
- **Function**: `app::store::tabs::open_tab(state: &mut State, folder: &Path) -> Vec<Effect>`, a function of the
  store (Q43)
- **Entry points**: **+** in the heading of **Repositories**, whose menu lists the repositories of the workspace that
  are not open, then **Open folder…** and **Open workspace file…**; **Open Repository in a Tab** in Quick search.
- **Asks**: the repository, when it was not chosen in the menu, in Gitardo's picker titled **Open a repository in a
  tab**: the repositories of the workspace that are not open, each by its name with its folder under it.
- **Reads**: `repo.read-state` and the first page of commits, for its first read; `repo.read-repositories` for a
  folder added.
- **Plan**: no step: nothing runs in a repository.
- **Danger**: None.
- **Undo**: nothing in a repository changes; `repo.close-tab` closes the tab again.
- **After**: the new row is placed after the one shown and is shown, saying **Loading repository…** until its first
  read is there. The tabs open are remembered with the workspace. A repository open already has its tab shown, and no
  second one opens. A repository that `repo.group-tabs` puts in the group of a tab open is shown in that tab. A folder
  added with **Open folder…** goes to the workspace, and its repositories are offered as `repo.choose-tabs` says.
- **Uses**: `repo.read-state`, `repo.open-folder`, `repo.open-code-workspace`, `repo.choose-tabs`,
  `repo.group-tabs`.

Acceptance:

- [ ] In a workspace of `api`, `web` and `docs` with only `api` open, **+** lists `web` and `docs`; choosing `web`
      opens it after `api` and shows it.
- [ ] Opening a repository that is open already shows its tab and opens no other.
- [ ] Opening a worktree of `api` that is a folder of the workspace shows it in the tab of `api`.
- [ ] The tabs open are the same once the window is closed and opened again.

### `repo.choose-tabs` — Choose the repositories a workspace opens

When a workspace with several repositories is opened for the first time, says which of them open in tabs: all of
them, or the ones the user keeps checked.

- **Kind**: Operation
- **Function**: `app::store::tabs::choose_tabs(state: &mut State, chosen: &[PathBuf]) -> Vec<Effect>`, a function of
  the store (Q43)
- **Entry points**: `repo.open-folder` and `repo.open-code-workspace`, once the repositories of the folders opened
  are found; **Open folder…** in the menu of **+**.
- **Offered when**: the folders opened hold more than one repository that would have a tab of its own
  (`repo.group-tabs`: worktrees and submodules do not count), and the workspace was not opened before. A workspace
  opened before opens the tabs it had (`repo.open-recent`).
- **Asks**: the repositories, in Gitardo's picker of many titled **Repositories to open in tabs**: each by its name
  with its folder under it, in the order they were found, all checked. `Enter` opens the ones checked.
- **Plan**: no step: nothing runs in a repository.
- **Danger**: None.
- **Undo**: nothing in a repository changes; `repo.open-tab` opens later one that was not chosen, `repo.close-tab`
  closes one.
- **After**: a tab for each repository chosen, in that order, and the first shown. The tab shown is read first, the
  others as `repo.watch-hidden` says.
- **Refuses**: nothing checked, or `Escape`: nothing opens, and the window stays as it was.
- **Uses**: `repo.read-repositories`, `repo.group-tabs`, `repo.open-tab`, `repo.watch-hidden`.

Acceptance:

- [ ] Opening a folder that holds eight repositories asks which to open, all checked; `Enter` opens eight tabs and
      shows the first.
- [ ] Unchecking all but `api` and `web` opens those two tabs.
- [ ] A folder that is one repository with two submodules opens one tab and asks nothing.
- [ ] `Escape` opens nothing and leaves the window as it was.
- [ ] A workspace opened before opens the tabs it had, without asking.

### `repo.open-code-workspace` — Open a workspace file of VS Code

Opens the folders a `.code-workspace` file of VS Code lists, so that the window shows the repositories of a workspace
the user already has in VS Code.

- **Kind**: Operation
- **Function**: `app::store::code_workspace::open_code_workspace(state: &mut State, chosen: Option<(PathBuf,
  Result<String, String>)>) -> Vec<Effect>`, a function of the store (Q43), given the file chosen and its text or why
  it could not be read; its folders read by `app::store::code_workspace_folders(text: &str, file: &Path) ->
  Result<Vec<PathBuf>, String>`
- **Entry points**: **Open workspace file…** on the Welcome page, in the menu of **+** and in Quick search.
- **Asks**:
  1. The file, in the system's dialog for opening a file, showing the `.code-workspace` files.
  2. When the window already shows a workspace: **Replace the workspace** or **Add to the workspace** (default:
     replace), as `repo.open-folder` asks.
- **Reads**: the file; `repo.read-repositories` for its folders.
- **Plan**: no step: nothing runs in a repository.
- **Danger**: None.
- **Undo**: nothing in a repository changes; the workspace before is among the recent ones.
- **After**: as `repo.open-folder`, then `repo.choose-tabs`. The workspace is among the recent ones of the Welcome
  page, named after the file.
- **Refuses**:
  - a file that names no folders: "This file names no folders.";
  - a file that cannot be read, or is no JSON: what is wrong with it;
  - a file whose folders are all gone: "None of the folders of this workspace is there any more."
- **Uses**: `repo.open-folder`, `repo.choose-tabs`, `repo.read-repositories`.

The folders are the `path` of each entry of `folders`, in their order: a relative path is from the folder of the file.
The file may have comments and trailing commas, as VS Code allows; its other keys are not read.

Acceptance:

- [ ] A file `work/all.code-workspace` with `"folders": [{ "path": "api" }, { "path": "../web" }]` opens `work/api`
      and `web`, in that order.
- [ ] Comments and a trailing comma in the file are no error.
- [ ] A file with no `folders` is refused with "This file names no folders."
- [ ] The workspace opened is the first of the recent workspaces on the Welcome page, named `all`.

### `repo.close-tab` — Close a tab

Takes a repository out of the window. Nothing in the repository changes.

- **Kind**: Operation
- **Function**: `app::store::tabs::close_tab(state: &mut State, index: usize) -> Vec<Effect>`, a function of the
  store (Q43)
- **Entry points**: the **×** of its row in **Repositories**; a middle click on the row; **Close tab** and **Close
  other tabs** in its context menu; **Close Tab** in Quick search, for the tab shown.
- **Asks**: for a File page of the tab with edits not saved, what closing that page asks, **Save changes to app.rs?**,
  the tab and the page shown while it asks: **Save** saves the file, then the tab closes; **Don't save** closes it with
  the edits lost; **Cancel** keeps it, and the tabs not closed yet. A save that fails keeps the tab.
- **Plan**: no step: nothing runs in a repository.
- **Danger**: None.
- **Undo**: nothing in a repository changes; **+** opens the repository again, with a first read.
- **After**: the tab to its right is shown, or the one to its left when it was the last; closing the only tab shows the
  Welcome page. The repository is no longer watched, its pages are closed, and what it kept is forgotten, but for what
  is kept between runs anyway (the message of the commit box, the branches shown, the folds, the favorites). The tabs
  open are remembered without it.
- **Refuses**: a tab where an operation runs: "Gitardo: wait for `<title>` to end before closing this tab."
- **Uses**: `repo.switch-tab`, `welcome.page`.

Acceptance:

- [ ] Closing `web`, between `api` and `docs`, shows `docs`.
- [ ] Closing the tab on the right end shows the one on its left; closing the only tab shows the Welcome page.
- [ ] Closing a tab where an operation runs is refused with that sentence; closing another tab meanwhile is not.
- [ ] A tab closed is not there once the window is closed and opened again.
- [ ] **Close other tabs** on `web` leaves `web` alone.
- [ ] Closing a tab with a page whose edits are not saved asks what closing that page asks.

### `repo.switch-tab` — Show another tab

Shows the repository of another tab, as it was left.

- **Kind**: Operation
- **Function**: `app::store::tabs::switch_tab(state: &mut State, index: usize) -> Vec<Effect>`, a function of the
  store (Q43)
- **Entry points**: a click on its row in **Repositories**, and the keys of `repo.tabs`; **Go to Repository…** in Quick
  search, which lists the tabs; and whatever shows a repository of another tab: a file of it chosen for its history or
  its blame (`repo.owner-of-file`).
- **Offered when**: the window has two tabs or more.
- **Plan**: no step: nothing runs in a repository.
- **Danger**: None.
- **Undo**: nothing in a repository changes; showing the other tab again goes back.
- **After**: what is below the island is the tab's: what it showed when it was left, as `repo.keep-open` says, read
  again when it is out of date (`repo.watch-hidden`). The tab shown is remembered with the workspace, and is the one
  shown the next time it opens.
- **Refuses**: nothing; showing the tab shown already reads nothing.
- **Uses**: `repo.keep-open`, `repo.watch-hidden`, `repo.owner-of-file`.

While an operation runs, tabs can still be switched, and what the other tabs show can be looked at, searched and
opened. The tab of the operation shows a spinner, keeps what it shows while it runs whatever `repositories.keepOpen`
says, and is shown again before the operation asks a question or shows its plan to confirm. When the operation ends
while another tab is shown, what it says is said over the window, and its tab is read again once it is shown, with
what the operation does after that read, such as selecting the commit it made (`repo.watch-hidden`). A window runs
one operation at a time: another one started meanwhile, in any tab, is refused with
"Gitardo: `<title>` is running in `<repository>`. Try again once it ends."

Acceptance:

- [ ] Showing `web` from `api` and then `api` again shows `api` as `repo.keep-open` says.
- [ ] Showing the tab that is shown already reads nothing.
- [ ] While a pull runs in `api`, `web` can be shown, scrolled and searched, and the tab of `api` shows a spinner.
- [ ] A question the operation of `api` asks while `web` is shown is asked once `api` is shown again.
- [ ] Starting a commit in `web` while a pull runs in `api` is refused with that sentence.
- [ ] **Show File History** of a file of `web`, chosen while `api` is shown, shows the tab of `web`.
- [ ] The tab shown is the one shown when the window opens again.

### `repo.keep-open` — Keep what each repository showed

What a repository keeps while another is shown, so that coming back to it finds it as it was, at once.

- **Kind**: Derivation
- **Function**: `app::store::tabs::kept_on_leaving(view: RepositoryView, keep_open: bool) -> (Option<RepositoryView>,
  Vec<JobId>)`: what the tab keeps, and the reads it started that nobody waits for any more
- **Settings**: `repositories.keepOpen`: `true` in Gitardo Desktop; `gitardo.repositories.keepOpen`, `false`, in the
  extension.
- **Uses**: by `repo.switch-tab`, `repo.group-tabs` and, in the extension, `repo.select`; `repo.watch-hidden`.
- **Extension**: with `gitardo.repositories.keepOpen` on, the repository left by the list of `repo.select`, by VS
  Code's Repositories view (`repo.follow-source-control`) or by **Show in Gitardo** keeps, in memory, the scroll of
  the graph, the commit selected and its details, the search, the branch filter, the references with their sections
  open or closed, and the message of the Changes view; the engine keeps its state as last read. Coming back shows that
  at once, and reads again behind it when it changed meanwhile. The list of `repo.select` puts the worktrees and the
  submodules of a repository under it, indented, as `repo.group-tabs` groups them. With the setting off, choosing
  another repository is as it was before it. Each view keeps what it showed of each repository in the memory of its
  page, by folder (`useKept` of `src/webview/kept.ts`), and the extension host keeps the state the views were last sent,
  what the graph was narrowed to and how far it was read, and the message of the Changes view
  (`src/extension/keptRepositories.ts`, `Controller.leave` and `Controller.comeBack`), as Q412 recommends. A repository
  that kept nothing, as one shown for the first time, is shown as choosing a repository did before. Tested by
  `test/e2e/ui.mjs`, step "with keeping repositories open on, a repository chosen again in the list shows the commit it
  had selected, at the same scroll"; `test/e2e/suite.ts`, the steps that start "with keeping repositories open" and
  "turning keeping repositories open off", and "lists the worktrees of a repository of the workspace under it in the
  list of the repositories"; `test/kept.test.ts` and `test/repositoryGroups.test.ts`.

With the setting on, a repository keeps everything it showed: the commits read, with their pages; the scroll of the
timeline; the commit selected and the inspector; the search and the branch filter; the branch rail, its lists open or
closed and its scroll; the pages open, the one shown, their trails and their scrolls; the Changes page with the
message of the commit box; the comparison in the inspector. With it off, a repository keeps only what is kept between
runs anyway (the branches shown, the folds, the favorites, the message of the commit box): coming back to it is a first
read, nothing searched and nothing selected. A repository with a page whose edits are not saved, a File page or the
result of a Merge page, keeps everything it showed all the same, so that the edits are not lost.

Between runs, the tabs open and the one shown are kept, with what was kept between runs before; the rest starts again.

Acceptance:

- [ ] With the setting on, a commit selected and the timeline scrolled to it in `api`, then `web` shown and searched:
      coming back to `api` shows the same commit selected at the same place, and `web`, shown again, still has its
      search.
- [ ] With the setting on, coming back to a tab shows what it kept at once, with no **Loading repository…**.
- [ ] With the setting on, the page of a commit open in `api` is open again in `api` with its trail, and is not in
      `web`.
- [ ] With the setting off, coming back to `api` shows its first page of commits, its own branches shown, nothing
      selected and nothing searched.
- [ ] Turning the setting off forgets at once what the tabs not shown kept.
- [ ] With the setting off, a File page of `api` with edits not saved is there again, with its edits, when `api` is
      shown again.
- [ ] Extension: with `gitardo.repositories.keepOpen` on, choosing `web` in the list and then `api` again shows `api`
      with the commit it had selected, at the same scroll.
- [ ] Extension: with the setting off, its default, coming back to `api` shows it as choosing a repository did before.
- [ ] Extension: the list of `repo.select` puts a worktree of `api` and a submodule of `api` under `api`, indented.

### `repo.group-tabs` — Worktrees and submodules in the tab of their repository

Which repositories share a tab: a repository, its worktrees and its submodules are one tab, and the **▾** of the tab
chooses which of them it shows.

- **Kind**: Derivation
- **Function**: `model::repo::group_tabs(repositories: &[Repository]) -> Vec<TabGroup>`, each repository with the
  common git folder `repo.read-repositories` read for it
- **Entry points**: the **▾** of a row of **Repositories**, which lists the group: the repository itself, its other
  worktrees, each by its branch or `detached at <short>`, with its folder under it and, on its right, `● 3` for the
  files changed there and `↓2 ↑1` for the commits to pull and to push (`repo.worktree-summary`), then its submodules,
  each by its path, a submodule's own submodules indented under it. The one the row shows has a check. Choosing another
  shows it in the tab, as `worktree.show-in-gitardo` and `submodule.show-in-gitardo` do.
- **Uses**: `repo.common-folder`, `repo.keep-open`, `repo.worktree-summary`, `worktree.show-in-gitardo`,
  `submodule.show-in-gitardo`.
- **Extension**: no tabs and no **▾**; the list of `repo.select` follows the first three rules, each member indented
  under its repository, its worktrees first (`groupRepositories` in `src/shared/repositoryGroups.ts`, tested by
  `test/repositoryGroups.test.ts`).

The rules:

- A worktree is in the group of the repository whose common git folder it shares (`repo.common-folder`), whether it is
  a folder of the workspace or not.
- A submodule is in the group of the repository around it, and so is a submodule of a submodule.
- A repository of the workspace that is neither is a group of its own, and so a tab of its own.
- Each member of a group keeps what it showed apart from the others, as `repo.keep-open` says.
- The tab is named after its repository; while it shows another member, after ` › `: the branch of a worktree, or the
  name of its folder when it is detached, as in `api › feature/login`; the path of a submodule, as in `api › lib/core`.
- Which member a tab shows is kept while the window is open; when the window opens again each tab shows its
  repository, as `worktree.show-in-gitardo` says.
- The group is named after its main worktree when that is a folder of the workspace, otherwise after the first of it
  found. The other worktrees the **▾** lists are those git lists for the repository, a folder of the workspace or not;
  before the tab was read, those of the workspace.
- Closing the tab asks about a page with edits not saved of any member, that member shown, as `repo.close-tab` says.

Acceptance:

- [ ] With `api` and its worktree `api-login`, on `feature/login`, both folders of the workspace, there is one tab
      `api`, whose **▾** lists `api` and `feature/login`.
- [ ] A submodule `lib/core` of `api` is listed in the **▾** of `api`, and has no tab of its own.
- [ ] Choosing `feature/login` in the **▾** names the tab `api › feature/login` and shows that worktree; choosing `api`
      again shows `api` as it was left.
- [ ] Two repositories that share no common git folder and neither of which is in the other are two tabs.
- [ ] The tab of a repository with no other worktree and no submodule has no **▾**.

### `repo.watch-hidden` — Keep the tabs not shown up to date

How the repositories of the tabs not shown follow their changes without reading at every one of them.

- **Kind**: Derivation
- **Function**: `model::repo::schedule_hidden(tab: &HiddenTab, event: &ReadEvent) -> (HiddenTab, Vec<ReadAction>)`
- **Settings**: `repositories.keepOpen`.
- **Uses**: `repo.watch-changes`, `repo.schedule-reads`, `repo.keep-open`, `activity.describe-task`.
- **Extension**: with `gitardo.repositories.keepOpen` on, the repositories the extension keeps follow these rules, the
  window of VS Code taking the place of the tab shown. The repositories it keeps are those left since the window
  opened: none is read in the background, so the rule of the reads one after the other once the workspace opens has
  nothing to read there. Their watchers stay, and a change marks them out of date (`Controller.changedIn`). Tested by
  `test/e2e/suite.ts`, steps "with keeping repositories open on, a commit made in a terminal in a repository not shown
  reads nothing, and showing it reads it once" and "with keeping repositories open on, shows a repository chosen again
  at once as it was left, and reads nothing for it when nothing changed".

The rules:

- Every tab open has its files watched as `repo.watch-changes` says, shown or not.
- A change in a tab not shown reads nothing: it marks the tab out of date, with the last change.
- A tab out of date that is shown shows what it kept at once, and is read again at once, as **Refresh** does, the
  activity indicator saying why. A tab left while a read of it was under way or waiting is out of date, for that read's
  reason; so is the tab of an operation that ended while another was shown.
- With the setting on, once the tab shown has its first read after the workspace opens, the other tabs are read one
  after the other, so that each is there when it is first shown. With it off, a tab is read when it is shown, and a
  tab not shown has no watcher.
- A read of a tab not shown never runs at the same time as an operation or as a read of the tab shown: it waits for
  them. One that runs when they start is stopped, and the tab waits for its turn again.

Acceptance:

- [ ] A commit made in a terminal in `web` while `api` is shown reads nothing; showing `web` then shows the commit
      after one read.
- [ ] With the setting on, opening a workspace of eight tabs reads the tab shown first, then the seven others, one at a
      time.
- [ ] A tab not shown in which nothing changed is shown with no read.
- [ ] While an operation runs in `api`, no other tab is read.

## Trusted folders

A folder is trusted once the user says so; until then nothing it asks to run is run
([01-product.md](../desktop/01-product.md)). What a folder asks to run is, so far, the commands of `worktrees.setupCommands`
(`worktree-setup.command-steps`). Gitardo asks whether to trust a folder when it would run something, never before, and
keeps the answer in the window's stored state, `state.json` beside the user's `settings.json`, under `trustedFolders`:
the list of the folders trusted, as the system writes them. Every window reads it when it starts an operation.

### `trust.is-trusted` — Whether a folder is trusted

- **Kind**: Derivation
- **Function**: `model::trust::is_trusted(folder: &Path, trusted: &[PathBuf]) -> Trust`

A folder is trusted when it is one of the folders trusted, or inside one: trusting a folder trusts every folder in it,
such as the repositories and the submodules it holds, and not the folder that holds it. Names are compared whole, and
as the system compares them: on Windows the case and the slashes do not count. The folder checked is that of the
repository the operation runs in, as the window shows it.

Acceptance:

- [ ] A folder the user trusted is trusted, and a folder never trusted is not.
- [ ] A folder inside a trusted folder is trusted, and the folder that holds a trusted folder is not.
- [ ] Trusting `/work/app` does not trust `/work/app-old`.
- [ ] On Windows, trusting `c:/work/app` trusts `C:\Work\App`.

### `trust.ask` — Ask whether to trust a folder before running what it asks

When an operation would run what a folder asks and the folder is not trusted, Gitardo asks first, so that a repository
just cloned runs nothing from its `.gitardo/settings.json` before the user has read it.

- **Kind**: Operation (a question that the operations which would run what a folder asks put before their plan)
- **Function**: `features::trust::ask(context, answers, asked: &TrustAsked) -> Result<Trust, Box<Outcome>>`
- **Offered when**: the folder of the repository is not trusted (`trust.is-trusted`) and there is something to run;
  otherwise it asks nothing and gives the trust the window has.
- **Asks**: **Run the commands for new worktrees?**, with **`<folder>` is not trusted, and worktrees.setupCommands in
  `<where>` asks to run `<command>`, then `<command>`. Trust it only if you trust who wrote them.**, `<where>` being
  **its .gitardo/settings.json** or **your settings**; a pick of:
  - **Trust this folder and run them**: **What `<folder>` and the folders in it ask to run is run from now on.**
  - **Not now** (default): **Create the worktree without running them.**
- **Plan**: none of its own. With **Trust this folder and run them**, the operation's plan runs the commands, and its
  follow-ups, on success and on failure alike, add the folder to the folders trusted
  (`FollowUp::TrustFolder`), which the window keeps at once, without waiting for the commands. With **Not now**, the
  plan is the one of a folder not trusted: no command runs, and the log and Gitardo say so.
- **Danger**: None.
- **Undo**: **Trusted folders…** in Quick search (`trust.manage`) stops trusting it.
- **Uses**: `trust.is-trusted`; by `worktree.create`, `worktree.create-from-branch`, `worktree.create-at-commit` and
  `worktree.open-in-worktree`, through `worktree-setup.command-steps`.

Escape backs out of the operation, as with any of its questions: no worktree is created.

Acceptance:

- [ ] In a folder not trusted, with `["npm ci", "npm run build"]`, creating a worktree asks first, naming the folder,
      where the commands come from and both commands, with **Not now** proposed.
- [ ] **Trust this folder and run them** runs both commands in Tasks, and the folder is trusted from then on: the next
      worktree asks nothing.
- [ ] **Not now** creates the worktree with its links and copies, runs no command, trusts nothing, and the next
      worktree asks again.
- [ ] In a trusted folder, or with no command to run, nothing is asked.

### `trust.manage` — See the trusted folders and stop trusting some

- **Kind**: Operation
- **Function**: `features::trust::manage(context, answers) -> Outcome`
- **Entry points**: **Trusted folders…** in Quick search.
- **Asks**: **Trusted folders**, with **What a trusted folder and the folders in it ask to run is run. Untick a folder
  to stop trusting it.**: the folders trusted, in the order they were trusted, each ticked.
- **Plan**: no step. Its follow-up takes the folders unticked out of the folders trusted
  (`FollowUp::ForgetTrustedFolders`), and Gitardo says **`<folder>` is no longer trusted.**, or **`<n>` folders are
  no longer trusted.**; with none unticked, **The trusted folders are unchanged.**
- **Danger**: None.
- **Undo**: none: the folder is trusted again the next time Gitardo asks (`trust.ask`).
- **Refuses**: with no folder trusted, **No folder is trusted.**
- **Uses**: `trust.ask`.

Acceptance:

- [ ] **Trusted folders…** in Quick search lists the folders trusted, each ticked.
- [ ] A folder unticked is no longer trusted, in this window and in `state.json`, and the folders still ticked stay
      trusted.
- [ ] With no folder trusted, it says **No folder is trusted.** and asks nothing.

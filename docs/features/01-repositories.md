# Repositories

How a window finds the repositories of its workspace, which one it shows, what it reads of it, and how it keeps that up
to date on its own while the repository and its files change, from Gitardo or from anything else. Also what the
activity indicator of the header says Gitardo is doing, the **Activity** drawer, and the **Welcome** page of the first
start.

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

How a repository is called in the repository menu of the header, in the picker and in the activity indicator.

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
- **Entry points**: **Open folder…** on the Welcome page, in the repository menu of the header, and in Quick search.
- **Asks**:
  1. The folder, in the system's folder dialog.
  2. When the window already shows a workspace: **Replace the workspace** or **Add to the workspace** (default:
     replace).
- **Reads**: `repo.read-repositories` for the folders of the new workspace.
- **Plan**: no step: nothing runs in a repository.
- **Danger**: None.
- **Undo**: nothing in a repository changes; the workspace before is among the recent ones.
- **After**: the window shows the new workspace and the repository `repo.select` remembers for it, or the first one
  found; the workspace goes first among the recent ones of the Welcome page.
- **Uses**: `repo.read-repositories`, `repo.read-state`, `welcome.page`.

Acceptance:

- [ ] Opening a folder with two repositories shows the first one and lists both in the repository menu of the header.
- [ ] Adding a folder to a workspace keeps the repository that was shown.
- [ ] Opening a folder with no repository shows `repo.no-repository`.
- [ ] The workspace opened is the first of the recent workspaces on the Welcome page.

### `repo.open-recent` — Open a recent workspace

Opens again a workspace the window showed before, with the repository that was shown in it.

- **Kind**: Operation
- **Function**: `app::store::open_recent(state: &mut State, index: usize) -> Vec<Effect>`, a function of the store
  (Q43)
- **Entry points**: the list of recent workspaces on the Welcome page; **Open recent** in Quick search.
- **Asks**: the workspace, from Gitardo's picker of the recent ones, the latest first (when not chosen on the page).
- **Reads**: `repo.read-repositories`.
- **Plan**: no step.
- **Danger**: None.
- **Undo**: nothing in a repository changes.
- **After**: the window shows that workspace, with the repository chosen last in it.
- **Refuses**: a workspace whose folders are all gone: "None of the folders of this workspace is there any more." The
  workspace is taken out of the list.

Acceptance:

- [ ] A workspace opened before is listed, and opening it shows the repository that was shown in it last.
- [ ] A workspace whose folder was deleted is refused with that message and leaves the list.

### `repo.select` — Choose the repository shown

Gitardo shows one repository at a time; in a workspace with several, this chooses which.

- **Kind**: Operation
- **Function**: `app::store::workspace::select(state: &mut State, folder: &Path) -> Vec<Effect>`, a function of the
  store (Q43)
- **Entry points**: the repository menu of the header; **Select repository…** in the **…** menu of the header;
  **Select Repository** in Quick search.
- **Offered when**: the workspace has more than one repository, or the window shows a worktree of a repository of the
  workspace that is not in the workspace itself (see `worktree.switch`). The repository menu of the header lists the
  repositories to choose from only then.
- **Asks**: the repository, in Gitardo's picker titled **Repository shown in Gitardo**: each repository by its name,
  with its folder under it, and the one shown marked **current**; a worktree shown from outside the workspace is listed
  last as **worktree, current**. The text typed matches the names and the folders.
- **Plan**: no step: nothing runs in a repository.
- **Danger**: None.
- **Undo**: nothing in a repository changes; choosing the other one again goes back.
- **After**: the branch rail, the timeline and the draft show the repository chosen: its own branches shown
  (`filter.remember`), the first page of its commits, nothing searched and nothing selected. The choice is remembered
  for the workspace, and is the one shown the next time the workspace opens, as long as it is still among its
  repositories.
- **Refuses**:
  - while an operation runs: nothing happens;
  - from Quick search, with a single repository: "Gitardo: this workspace has a single repository."; with none:
    "Gitardo: no Git repository in this workspace."
- **Uses**: `repo.read-state`, `repo.name-repositories`, `filter.remember`.

Acceptance:

- [ ] In a workspace of two repositories, choosing the second shows its branches and commits, and the next time the
      workspace opens it is the one shown.
- [ ] A remembered repository that is no longer in the workspace gives way to the first one found.
- [ ] With one repository the repository menu of the header lists no repository to choose, and the command says
      "Gitardo: this workspace has a single repository."
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
    what is not committed there; this one's count is the one of this read.
  - `git remote`: the names of the remotes.
  - `git config --get-regexp ^user\.(name|email)$`: who commits are made as.
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
- [ ] No read takes `index.lock`: while it runs, `git commit` in a terminal does not fail because of it.
- [ ] With `readOnStartup` off, starting the window runs no git command until **Read the repository** is clicked.

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
| A file of the working tree, anywhere but in a `.git` | yes | 1,000 ms (builds and installs write many files at once) | `<path> changed` |
| A file saved by Gitardo's own editor | yes | 400 ms | `<path> was saved` |
| Files created, deleted or renamed by Gitardo | yes | 400 ms | `files were created` (`deleted`, `renamed`) |
| The window gets the focus | yes | 400 ms | `the window got the focus` |
| A setting under `conflicts`, `pull`, `worktrees`, `branches` or `graph` changes | yes | 400 ms | `the settings changed` |
| Any path with a part named `fsmonitor--daemon` | no | | |
| Any file whose name ends in `.lock` | no | | |

The last two rules are what keep a repository from being read for ever: the daemon of `core.fsmonitor` leaves a file
in the folder of a worktree (`.git/worktrees/<name>/fsmonitor--daemon/…`) each time `git status` runs there, as it
does in every read; and a lock is always followed by the file it locked.

Acceptance:

- [ ] `git commit` in a terminal asks for a read 400 ms after `.git/HEAD` or the branch's ref changes, said as
      `.git/refs/heads/main changed` (or the file that changed first).
- [ ] Saving a file from another editor asks for a read after 1,000 ms.
- [ ] A change to `.git/worktrees/feature/fsmonitor--daemon/cookies/…` asks for nothing.
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

The rules:

- A change asks for a read after its delay, counted from the first change of a burst: further changes do not push it
  back, however long the burst lasts, but one with a shorter delay brings it forward. The read is said to be for the
  last change of the burst.
- Changes ask for nothing while the window is minimized, so that nothing of the repository is in sight, and while an
  operation runs: the operation is followed by a read of its own.
- **Refresh**, the end of an operation, choosing a repository, the branches shown, a search, a filter or the next page
  ask for a read at once.
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
- After each read, the pull requests are read again when they are older than 2 minutes (`github.load-pull-requests`).

Acceptance:

- [ ] Ten files written within half a second lead to one read, about a second after the first.
- [ ] A file that changes every 200 ms for 5 s leads to reads one after another, never two at once, and the activity
      indicator counts them.
- [ ] **Refresh** during a read leads to exactly one more read after it.
- [ ] Nothing is read while an operation runs; one read follows it.
- [ ] A read started before a commit made by Gitardo is not shown: the state shown after the commit has the commit
      on its branch.
- [ ] Choosing **Local** in the branch filter shows the bar until the commits of the local branches are there, even
      while a build in the working tree keeps changing files.

### `activity.describe-task` — Say what Gitardo is doing

The words the activity indicator of the header and the **Activity** drawer use for each thing that takes time.

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

`<name>` is the one `repo.name-repositories` gives. How long it has run is said in whole seconds, `12 s`, and from a
minute on as `1 min 5 s`.

Acceptance:

- [ ] The third read in a row after `src/a.rs` changed says `Read 3 times in a row: something changed during each
      read. Because src/a.rs changed.`
- [ ] 65 seconds is said as `1 min 5 s`, 59.9 seconds as `59 s`.
- [ ] A read while the timeline is narrowed to `git log -S foo` is titled `Searching <name>`.

### `activity.status-bar` — What is running, in the header

The activity indicator of the header says what Gitardo is running, for how long, and why; and the branch of the header
says how far it is from its upstream.

- **Kind**: View
- **Function**: `views::activity::status_bar_view(state: &StatusBarState, ui) -> Vec<Intent>`
- **Entry points**: the activity indicator and the branch of the header.
- **Uses**: `activity.describe-task`, `activity.what-is-running`.

- While Gitardo has been reading a repository, running an operation, comparing two sides, making a patch or asking
  GitHub for more than half a second, the activity indicator shows a spinning icon with the title of what has run
  longest and for how long, as `Reading app (2 s)`. What is over sooner never shows, so that it does
  not flicker. It takes only the room Quick search leaves: where its words would be cut short, and when the window is
  narrow, it shows its icon alone.
- Its tooltip lists everything that runs, the oldest first, each with its title, how long it has run and its detail,
  and ends with **Click to open the Activity drawer.** The times go on every half second; the tooltip is
  drawn again only when its text changes.
- A click opens the Activity drawer (`activity.what-is-running`).
- The branch of the header names the branch checked out, `detached at 3f2a1c9` or `no commits yet`, with `↑<ahead>
  ↓<behind>` from its upstream when it has one.

Acceptance:

- [ ] A read of 0.3 s shows nothing; a read of 2 s shows the activity indicator from 0.5 s until it ends.
- [ ] A fetch and a read at once are both in the tooltip, the fetch first if it started first.
- [ ] The tooltip of a read asked for by a change says which file changed.
- [ ] Beside Quick search in a window 1000 pixels wide, the indicator shows its icon alone and Quick search keeps its
  room; at 1200 pixels it says what runs.
- [ ] With `main` two ahead of `origin/main`, the branch of the header says `main ↑2 ↓0`.

### `activity.what-is-running` — What is running, in the Activity drawer

Lists what Gitardo is doing now, with the way to the commands it runs.

- **Kind**: View
- **Function**: `views::activity::what_is_running_view(state: &ActivityState, ui) -> Vec<Intent>`
- **Entry points**: a click on the activity indicator of the header; the **Activity** button of the header; **Show
  What Is Running** in Quick search.
- **Uses**: `activity.describe-task`, `log.show`.

The drawer lists each thing that runs, with its title, how long it has run and its detail, the times going on every
second while it is open, and the list following what starts and ends. With nothing running it says **Nothing is running
now** and "Gitardo is not reading or changing anything". Under the list, **Show the commands Gitardo runs** shows the
Command log of the drawer. Opening the drawer while it is open leaves it as it is.

Acceptance:

- [ ] Opened during a long fetch, the drawer lists the fetch, and its time grows each second.
- [ ] When the fetch ends, it leaves the list; with nothing else, the drawer says **Nothing is running now**.
- [ ] **Show the commands Gitardo runs** shows the Command log.

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

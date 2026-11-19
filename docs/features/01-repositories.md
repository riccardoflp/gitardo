# Repositories

How a window finds the repositories of its workspace, which one it shows, what it reads of it, and how it keeps that up
to date on its own while the repository and its files change, from Gitardo or from anything else. The tabs of the
repositories a window of Gitardo Desktop keeps open, and what each keeps while another is shown. Also what the activity
indicator of the status bar says Gitardo is doing, the **Activity** tab of the bottom panel, the **Welcome** page of
the first start, and the folders the user trusts to run what they ask.

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

### `repo.clone` — Clone a repository

Copies a repository from its URL, or from its folder on disk, into a new folder, and opens the copy in a tab
([D77](../08-decisions.md#d77--clone-and-create-a-repository-and-the-history-of-a-shallow-clone-proposed)).

- **Kind**: Operation
- **Function**: `features::repo::clone(context, answers) -> Outcome`, given the context of no repository: its step
  runs in the folder chosen to hold the clone (Q1051)
- **Entry points**: **Clone a repository…** on the Welcome page, beside **Open folder…**; in the menu of **+** in the
  heading of **Repositories**, after **Open folder…**; on the timeline of `repo.no-repository`; **Clone Repository…**
  in Quick search. The window has no menu bar, and the title bar it draws has none (`window.title-bar`): the menu of
  **+** is where the window opens what it shows (Q1050).
- **Extension**: Host only: **Git: Clone** of VS Code clones and opens the clone in VS Code, where Gitardo reads it as
  any folder of the workspace.
- **Asks**:
  1. **URL of the repository, or its path on disk**, with `https://github.com/owner/repo.git` as a placeholder, and the
     text of the clipboard as its default when it is one line that starts with `https://`, `ssh://`, `git@` or
     `file://` (Q1052). Refused while typed: **Enter a URL** when empty, **That is not a URL** when it starts with `-`.
  2. The folder that will hold the clone, in the system's folder dialog, starting at the one the last clone went in
     (`cloneFolder` in `state.json`), or at the user's home folder the first time.
  3. **Name of the folder of the clone**, by default the one `repo.folder-from-url` gives. The box says **In `<full
     path>`**, or refuses: **Enter a name**; **Enter the name of one folder** for a name with `/`, `\` or `..`, or one
     that starts with `-`; **`<full path>` is not empty**, since git clones only into a folder that is new or empty.
  4. **How to clone `<name>`**, a pick of many, `Enter` cloning as ticked:
     - **Check out another branch than the remote's default one**, not ticked;
     - **Only the last commits, a shallow clone**, not ticked, and not offered for a path on disk, where git ignores
       the depth;
     - **The submodules too, and theirs**, ticked.
  5. With the first ticked: **Branch to check out**, refused while typed when empty or when git would not take it as
     a branch name (`branch.validate-name`).
  6. With the second ticked: **How many commits to fetch**, `1` by default, refused while typed when it is not a whole
     number of at least 1; its detail says **Only this branch is fetched, as git does for a shallow clone.** (Q1055)
- **Plan**: `git clone --progress [--branch <branch>] [--depth <n>] [--recurse-submodules] -- <url> <name>`, run in the
  folder that holds the clone, titled **Clone `<url>` into `<name>`**. `--progress` makes git print its progress though
  it writes to no terminal, for the **Activity** tab (below).
- **Danger**: None: nothing that was on disk is changed, the folder being new or empty.
- **Undo**: Not taken back by Undo: the folder of the clone is the user's to delete.
- **After**: the folder that holds the clone is kept as `cloneFolder`. The clone is opened in a tab and shown, its
  folder added to the workspace as **Open folder…** adds one, or, with no workspace open, made the workspace
  (`repo.open-tab`, `repo.open-folder`, Q1054); the status bar says **Cloned into `<full path>`**. A shallow clone shows
  where its history ends (`graph.shallow-end`). When git fails, its message is said with **Show the log**; git has
  taken away the folder it made.
- **Refuses**: what the boxes refuse while typed; a folder that has files by the time it runs: **`<full path>` is not
  empty.**
- **Settings**: `commandPreview`.
- **Uses**: `repo.folder-from-url`, `branch.validate-name`, `repo.open-tab`, `repo.open-folder`,
  `activity.describe-task`, `graph.shallow-end`.

While it runs, its row of the **Activity** tab and the activity indicator of the status bar say its title, and its
detail is the last line of progress git printed, `Receiving objects: 45% (1203/2671), 1.20 MiB | 3.40 MiB/s`, read as
git prints it (Q1056). Nothing stops it once it runs (Q1057). The credentials are git's own
([D1](../08-decisions.md#d1--the-git-executable-not-a-library)): git runs with `GIT_TERMINAL_PROMPT=0`, a credential
helper such as Git Credential Manager may open its own window, and a private repository git has no credentials for
fails with git's message (Q1053).

Acceptance:

- [ ] `https://example.com/acme/lib.git` cloned into `C:\work`, with nothing ticked but the submodules, runs
      `git clone --progress --recurse-submodules -- https://example.com/acme/lib.git lib` in `C:\work`; `C:\work\lib`
      opens in a tab and is shown.
- [ ] With the branch `stable` and 50 commits, it runs `git clone --progress --branch stable --depth 50
      --recurse-submodules -- <url> lib`, and the timeline of the clone shows where its history ends.
- [ ] A name whose folder holds a file is refused while typed with **`<full path>` is not empty**; an empty folder of
      that name is taken.
- [ ] `--upload-pack=x` as the URL and `-lib` as the name are refused while typed, and nothing runs.
- [ ] A path on disk does not offer **Only the last commits, a shallow clone**.
- [ ] During a clone of more than half a second, the **Activity** tab lists **Clone `<url>` into lib**, with git's
      last line of progress under it, changing as git prints.
- [ ] A clone of a URL that does not exist says git's message with **Show the log**, and leaves no folder `lib`.
- [ ] The folder dialog of the next clone starts at `C:\work`, after the window is closed and opened again.
- [ ] With no workspace open, the clone becomes the workspace, and is first among the recent ones of the Welcome page.
- [ ] Backing out of any question runs nothing.

### `repo.folder-from-url` — The name of a clone's folder

The name the folder of a clone gets by default: the name of the repository, from its URL or its path.

- **Kind**: Derivation
- **Function**: `model::repo::folder_from_url(url: &str) -> Option<String>`
- **Uses**: by `repo.clone`.
- **Extension**: Host only, as `repo.clone`.

The name is the last part of the URL or the path, after its last `/`, its last `\` or the `:` of `host:path`, once the
slashes at its end are taken away, without `.git` or `.bundle` at its end; for a path that ends with the folder
`.git`, the folder above it. When nothing is left, as for `https://example.com/`, there is none, and the box starts
empty. It is the rule `submodule.add` follows for the folder of a submodule, which may use this function (Q1058).

Acceptance:

- [ ] `https://github.com/owner/lib.git`, `git@github.com:owner/lib.git`, `ssh://host:2222/srv/lib.git/` and
      `C:\repos\lib\` all give `lib`.
- [ ] `C:\repos\lib\.git` gives `lib`, and `https://example.com/acme/tools.bundle` gives `tools`.
- [ ] `https://example.com/` gives none.

### `repo.init` — Create a repository

Makes an empty repository in a folder, with the first branch git's configuration says or the one typed, and opens it in
a tab ([D77](../08-decisions.md#d77--clone-and-create-a-repository-and-the-history-of-a-shallow-clone-proposed)).

- **Kind**: Operation
- **Function**: `features::repo::init(context, answers) -> Outcome`, given the context of no repository, as
  `repo.clone`
- **Entry points**: **Create a repository…** on the Welcome page and in the menu of **+** in the heading of
  **Repositories**, after **Clone a repository…**; **Create Repository…** in Quick search; **Create a repository
  here** on the timeline of `repo.no-repository`, for the folder of the workspace that has none (with several, the
  first; Q1059).
- **Extension**: Host only: **Git: Initialize Repository** of VS Code, and its **Initialize Repository** button in the
  Source Control side bar of a folder with no repository.
- **Asks**:
  1. The folder, in the system's folder dialog; not asked from `repo.no-repository`, which gives it.
  2. When the folder is inside a repository: **`<folder>` is inside the repository `<top>`. Create another repository
     in it?**, with **Create** (Q1060).
  3. Only when git's configuration has no `init.defaultBranch`: **Name of the first branch**, `main` by default (Q1061),
     refused while typed when git would not take it as a branch name (`branch.validate-name`).
- **Reads**: in the folder, `git rev-parse --show-toplevel`, for the repository it is in, if any; `git config --get
  init.defaultBranch`, for the first branch.
- **Plan**: `git init`, run in the folder, or `git init --initial-branch=<name>` when the name was asked. Titled
  **Create a repository in `<folder>`**.
- **Danger**: None.
- **Undo**: Not taken back by Undo: the folder `.git` stays, and deleting it is the user's.
- **After**: the repository is opened in a tab and shown, as after `repo.clone`, or, from `repo.no-repository`, read
  in its place; the timeline has the draft alone, with the files of the folder untracked. The status bar says
  **Created a repository in `<folder>`**.
- **Refuses**: a folder that is the top of a repository already, where `git init` would only initialize it again:
  **`<folder>` is a repository already.**
- **Uses**: `branch.validate-name`, `repo.open-tab`, `repo.open-folder`, `repo.no-repository`, `repo.read-state`.

Acceptance:

- [ ] With `init.defaultBranch` set to `trunk`, creating a repository in `C:\work\new` asks no branch, runs `git init`
      there, and the row of `new` says `trunk`, with no commit.
- [ ] Without `init.defaultBranch`, the branch asked starts as `main`; `release` runs `git init
      --initial-branch=release`.
- [ ] `a..b` as the first branch is refused while typed.
- [ ] A folder that is a repository is refused, and nothing runs.
- [ ] A folder inside a repository asks first; declining runs nothing.
- [ ] **Create a repository here**, on a workspace whose folder has no repository, runs `git init` in it, and the
      timeline shows the new repository with its files untracked.

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
repositories again (`repo.read-repositories`) and reads the first one, and beside it **Create a repository here**, which
makes one in the first folder of the workspace (`repo.init`), and **Clone a repository…** (`repo.clone`). While the
repositories are looked for and the first one is read, it says **Loading repository…**. A read that fails, as when
the folder was deleted, comes here too; the error goes to the Command log.

Acceptance:

- [ ] A workspace of a folder with no repository shows **No Git repository in this workspace** and **Check again**.
- [ ] Running `git init` in the folder and clicking **Check again** shows the new repository with no commits.
- [ ] Deleting the folder of the repository shown brings the window to this page.
- [ ] **Create a repository here** and **Clone a repository…** are beside **Check again**, and run `repo.init` in the
      folder of the workspace and `repo.clone`.

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
  - Whether the repository is a shallow clone, and where its history ends: see `repo.read-shallow`.
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
- **Extension**: `read/repo.read-state` of Gitardo's engine, from `Reads.parts` in `src/engine/reads.ts`, with
  `refresh` (version 1.8 of the protocol) for **Refresh** and for a repository chosen: the engine keeps what each read
  counted in the other worktrees, and counts again only those whose git folder changed (Q462). `repository/open` counts
  them all. `gitardo.readOnStartup` is the setting `readOnStartup`.

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
- [ ] The state of a clone made with `--depth 1` says where its history ends: its one commit.
- [ ] With five other worktrees and nothing changed in their git folders, a read after a change to a file of this one
      runs `git status` here only, and each other worktree keeps its count; after `git add` in one of them, the next
      read runs `git status` there too; **Refresh** runs it in all of them.
- [ ] No read takes `index.lock`: while it runs, `git commit` in a terminal does not fail because of it.
- [ ] With `readOnStartup` off, starting the window runs no git command until **Read the repository** is clicked.

### `repo.read-shallow` — Whether the history is shallow, and where it ends

Whether the repository is a shallow clone, and the commits whose parents were not fetched, where its history ends
([D77](../08-decisions.md#d77--clone-and-create-a-repository-and-the-history-of-a-shallow-clone-proposed)).

- **Kind**: Read
- **Function**: `git::read::repo::read_shallow(git, request: &ShallowRequest) -> Result<Shallow, GitError>`
- **Reads**: `git rev-parse --is-shallow-repository --git-path shallow`, with the reads of `repo.read-state` and at
  once with them; when the first line says `true`, the file the second line names, in the common git folder, which
  holds the hash of each commit whose parents were not fetched, one a line. Git lists those commits with no parent,
  as first commits.
- **Uses**: by `repo.read-state`, `graph.shallow-end`, `sync.deepen` and `sync.unshallow`. A change of that file asks
  for a read, as a change of the references does (`repo.watch-changes`).
- **Extension**: Planned: with `read/repo.read-state` of the engine, for `graph.shallow-end` in the graph.

Acceptance:

- [ ] In a clone made with `--depth 2` of a history of five commits, the repository is shallow, and the older of the
      two commits listed is the one whose parents were not fetched.
- [ ] A clone of the whole history is not shallow, and no file is read.
- [ ] A worktree of a shallow repository reads the same commits as its main worktree.
- [ ] After `git fetch --unshallow` in a terminal, the next read says the repository is not shallow.

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

### `repo.read-git-version` — Read the version of git

Which git runs the operations, so that a feature that needs a newer one says what it does without it, rather than
failing in git's words.

- **Kind**: Read
- **Function**: `git::read::repo::read_git_version(git: &Git, request: &GitVersionRequest) -> Result<GitVersion, GitError>`
- **Reads**: `git --version`, once for each run of the engine, in the C locale. Its line is
  `git version <major>.<minor>.<patch>`, followed on some systems by more (`.windows.1`, ` (Apple Git-155)`), which is
  kept as git said it and not compared.
- **Uses**: by `rebase.update-refs`. Each version a feature needs is a constant with the version and the reason
  (R9.5).
- **Extension**: the engine reads it for itself, under the extension as under the window.

As built, a feature asks for it with `Query::GitVersion`; the engine keeps what it read for as long as it runs, and
answers `GitVersion(None)` when the line is of another form or git does not start, which the feature takes as a git too
old for it. `UPDATE_REFS_VERSION` in `model::rebase` is the first version a feature needs (R9.5).

Acceptance:

- [ ] `git version 2.53.0.windows.1` gives 2.53.0.
- [ ] `git version 2.39.5 (Apple Git-154)` gives 2.39.5.
- [ ] A line of another form gives an error that quotes it, and a feature that needs a version then acts as on a git
      too old for it.
- [ ] It runs once for each engine, however many repositories are open.

### `repo.watch-changes` — Which changes ask for a read

The repository is read again on its own when it or its files change, from Gitardo, a terminal or another program.
This says which changes count, and how soon.

- **Kind**: Derivation
- **Function**: `model::repo::watch_changes(change: &Change) -> Option<ReadRequest>`
- **Uses**: by `repo.schedule-reads`.
- **Extension**: the watchers of `Controller.watch` in `src/extension/controller.ts`, of the common git folder and of
  the working tree. A file of the working tree in the folder of another worktree of `RepoState.worktrees` inside the
  repository asks for nothing; the others of a burst are given, a second after the first of them, to
  `read/repo.watch-changes` of Gitardo's engine (version 1.8), which leaves out those in nested worktrees and those
  `git check-ignore` ignores, a `.gitignore` never, and the repository is read at once when one is left. A file saved,
  created, deleted or renamed in VS Code's editor stands for Gitardo's own editor. VS Code getting the focus stands for
  the window getting it, and a change that came while no view of Gitardo was open for one that came while it was
  minimized; the minute is counted from the end of the last read of the repository shown.

What is watched, and what each change asks for:

| Change | Asks for a read | After | Said as |
| --- | --- | --- | --- |
| In the common git folder (`git rev-parse --git-common-dir`): `HEAD`, `FETCH_HEAD`, `ORIG_HEAD`, `MERGE_HEAD`, `CHERRY_PICK_HEAD`, `REVERT_HEAD`, `BISECT_LOG`, `index`, `config`, `packed-refs`, `shallow`, anything under `refs/`, `logs/refs/stash`, anything under `worktrees/`, and `modules/**/HEAD`, `modules/**/refs/**` of the submodules | yes | 400 ms | `.git/<path> changed` |
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
- [ ] A fetch that changes `.git/shallow` asks for a read after 400 ms, said as `.git/shallow changed`
      (`repo.read-shallow`).

### `repo.schedule-reads` — Read again when something changed

Decides when the repository is read: once after a burst of changes, never twice at once, again while something keeps
changing during each read, never during an operation. **Refresh** asks for a read at once.

- **Kind**: Derivation
- **Function**: `model::repo::schedule_reads(schedule: &ReadSchedule, event: &ReadEvent) -> (ReadSchedule, Vec<ReadAction>)`
- **Entry points**: **Refresh**, the first entry of the **…** menu of the header; **Refresh** in Quick search; and
  every change `repo.watch-changes` counts.
- **Settings**: `readOnStartup`, for the first read (see `repo.read-state`).
- **Uses**: `repo.watch-changes`, `repo.read-state`, `graph.read-commits`, `activity.describe-task`.
- **Extension**: `Controller.refresh` and `Controller.load` in `src/extension/controller.ts`, with the status bar item
  of `activity.status-bar`; **Refresh** of the views and **Gitardo: Refresh** are `Controller.refreshAll`, which reads
  every commit and counts every worktree again. `EngineGraph` of `src/engine/graph.ts` keeps the commits of a read while
  the commit checked out, the references and the stashes are those of the read before.

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

The words the activity indicator of the status bar and the **Activity** tab of the bottom panel use for each thing
that takes time.

- **Kind**: Derivation
- **Function**: `model::activity::describe_task(task: &Task, now: Instant) -> TaskText`
- **Uses**: by `activity.status-bar` and `activity.what-is-running`.

| What | Title | Detail |
| --- | --- | --- |
| A read of the repository | `Reading <name>`, or `Searching <name>` while a search in the changes (`git log -S` or `-G`) narrows the timeline | `Because <cause>.` for the change that asked for it; from the second read in a row, `Read <n> times in a row: something changed during each read.` before it |
| An operation | its title, as the preview shows it | while a step that asks git for its progress (`--progress`) runs, the last line of progress git printed, as git prints it: `Receiving objects: 45% (1203/2671), 1.20 MiB \| 3.40 MiB/s` (`model::activity::progress_line`) |
| A comparison | `Comparing <from> ↔ <to>` | |
| A patch being made | `Making a patch of <commits>` | |
| The pull requests | `Reading the pull requests from GitHub` | |

`<name>` is the one `repo.name-repositories` gives. How long it has run is said in whole seconds, `12 s`, and from a
minute on as `1 min 5 s`. Reads of the same repository one after the other are said on one row, whose title counts
them from the second on, `Reading app ×4` (`model::activity::counted`), and whose detail is that of the latest
([D64](../08-decisions.md#d64--activity-a-tab-of-the-bottom-panel-whose-list-holds-still)).

Acceptance:

- [ ] The third read in a row after `src/a.rs` changed says `Read 3 times in a row: something changed during each
      read. Because src/a.rs changed.`
- [ ] 65 seconds is said as `1 min 5 s`, 59.9 seconds as `59 s`.
- [ ] A read while the timeline is narrowed to `git log -S foo` is titled `Searching <name>`.
- [ ] Four reads of `work/app` one after the other are titled `Reading work/app ×4`, and one alone `Reading
      work/app`.
- [ ] An operation whose step asks git for its progress says, as its detail, the last line git printed, its parts
      started again with a carriage return: `Receiving objects:  45% (1203/2671), 1.20 MiB | 3.40 MiB/s`.

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
- It shows the rows of the **Activity** tab (`activity.what-is-running`): each thing is timed from when the engine
  started it, and reads of the same repository one after the other are one entry that counts them, `Reading app ×4
  (2 s)`, timed from the first, which stays half a second after a read ended for the next to join it
  ([D64](../08-decisions.md#d64--activity-a-tab-of-the-bottom-panel-whose-list-holds-still)).
- The end of the status bar says the identity commits are made with in the repository shown, `Ada
  <ada@example.com>`, the name or the email alone, or **No identity configured**, when there is room for it after
  what runs; a click switches it (`identity.switch`), except while an operation runs.
- Its tooltip lists everything that has run for half a second, in the order it started, each with its title, how
  long it has run and its detail, and ends with **Click to show Activity in the bottom panel.** The times go on every
  half second; the tooltip is drawn again only when its text changes.
- A click shows the bottom panel at its **Activity** tab (`activity.what-is-running`), or hides the panel when it
  shows that tab ([D64](../08-decisions.md#d64--activity-a-tab-of-the-bottom-panel-whose-list-holds-still)).
- The status bar keeps its height whatever it says: what was said and its links are drawn in its text
  ([D56](../08-decisions.md#d56--a-side-column-of-repositories-and-branches-what-was-said-in-the-status-bar-diffs-beside-what-opened-them)).

Acceptance:

- [ ] A read of 0.3 s shows nothing; a read of 2 s shows the activity indicator from 0.5 s until its row ends.
- [ ] A fetch and a read at once are both in the tooltip, the fetch first if it started first.
- [ ] The tooltip of a read asked for by a change says which file changed.
- [ ] In a window 1000 pixels wide, a read of 2 s says `Reading app (2 s)` in the status bar, and the identity is at
  its end; a click on the identity asks to switch it.
- [ ] With `main` two ahead of `origin/main`, the header names `main` without how far it is, and **Sync** says `↑2`
      (Q500).
- [ ] A click on the indicator shows the bottom panel at **Activity**, and another hides the panel.
- [ ] What has run less than half a second is not in the tooltip.
- [ ] Three reads of `app` one after the other are one entry of the indicator that counts them, `Reading app ×3
      (1 s)`.

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
- A click on its words opens the Command log of the bottom panel at the command that said it (`log.show`): the first
  line of the operation, or the step that failed when one failed, brought to the top of the log and marked (Q506).
  **Show Command Log**, or the tab of the log, follows its last line again.
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

### `activity.what-is-running` — What is running, in the Activity tab

Lists what Gitardo is doing now, in a tab of the bottom panel, beside the commands it runs.

- **Kind**: View
- **Function**: `views::activity::what_is_running_view(state: &ActivityState, ui) -> Vec<Intent>`
- **Entry points**: the tab **Activity** of the bottom panel (`panel.bottom`); a click on the activity indicator of the
  status bar; **Activity** in the bar of views; **Show What Is Running** in Quick search.
- **Uses**: `activity.describe-task`, `log.show`, `panel.bottom`.
- **Extension**: a click on the item of the status bar, or **Gitardo: Show What Is Running**, lists what runs in a
  picker of VS Code, kept up to date while it is open, whose last entry, **Show the commands Gitardo runs**, opens the
  output channel **Gitardo**. The tab is the window's alone: VS Code has its own panel.

**Activity** is a tab of the bottom panel, after **Terminal** and before **Command log**, with the panel's height, its
gap and its **Hide the panel**: nothing is drawn over the timeline to say what runs
([D64](../08-decisions.md#d64--activity-a-tab-of-the-bottom-panel-whose-list-holds-still)). The activity indicator of
the status bar and **Activity** in the bar of views show the panel at it, and hide the panel when it shows it; **Show
What Is Running** shows the panel at it (Q680). `Escape` closes nothing there.

The tab lists each thing that runs on a row, with its title, how long it has run and, under it, its detail, the times
going on while it is shown. The list is laid out so that it holds still while it is read:

- Each thing is timed from the moment the engine started it, and ends at the moment the engine says it ended,
  whichever frames drew it.
- Something is listed once it has run half a second, as the activity indicator shows it; what is over sooner is listed
  neither while it runs nor after.
- The rows are in the order things started, and a row stays where it is while it runs; a row is as high whether it
  has a detail or not.
- Reads of the same repository one after the other are one row that counts them, `Reading app ×4`
  (`activity.describe-task`), with the detail of the latest, timed from the start of the first, its time stopped
  between one read and the next. The row waits half a second after a read ended for the next to join it (Q682).
- What ended goes under **Ran**, the latest first, with the time of day it ended, in UTC as the Command log says its
  times, and how long it took: `14:02:09 · took 4 s`, or `took less than 1 s` (Q683). Reads that end one after the
  other, with nothing else ending between them, are one row there too, with the time the last ended and how long they
  took together. **Ran** keeps the last 50 of the session, however long ago they ended.
- The list scrolls, under the title of its column, when it is taller than the tab.
- The detail of an operation whose step asks git for its progress, such as a clone, is the last line of progress git
  printed, changing as git prints (`activity.describe-task`).

With nothing running it says **Nothing is running now** and "Gitardo is not reading or changing anything", where the
first row would be. The commands Gitardo runs are in the tab beside it, **Command log** (`log.show`), and the
commands of a new worktree among the tasks of the **Tasks** tab (`worktree-setup.command-steps`, `tasks.view`).

Acceptance:

- [ ] Shown during a long fetch, the tab lists the fetch, and its time grows each second.
- [ ] When the fetch ends, it leaves what runs; with nothing else, the tab says **Nothing is running now**.
- [ ] A click on the activity indicator shows the bottom panel at **Activity**, between **Terminal** and **Command
      log**, and nothing is drawn over the timeline, which ends above the panel.
- [ ] `Escape` with the panel at **Activity** hides nothing.
- [ ] A click on the tab **Command log**, beside it, shows the commands Gitardo runs.
- [ ] What runs less than half a second is listed neither while it runs nor under **Ran**.
- [ ] During a fetch with forty reads in a row around it, no row moves, flashes or goes before the fetch ends; the
      fetch counts its seconds from its start, and the reads are one row.
- [ ] Two reads that end within the same frame are listed in the same order each time, the one that ended last first.
- [ ] A fetch that ended is under **Ran** with the time it ended and how long it took, and is still there ten minutes
      later.
- [ ] **Ran** lists the latest first and keeps the last 50.
- [ ] Reads of a repository one after the other are one row that counts them, `Reading app ×4`, where the first
      was.
- [ ] Reads that ended one after the other are one row under **Ran**, until something else ends.
- [ ] With more rows than fit, the wheel brings the last into sight.
- [ ] The row of an operation whose step asks git for its progress has git's last line of progress under its title,
      changing as git prints
      ([D77](../08-decisions.md#d77--clone-and-create-a-repository-and-the-history-of-a-shallow-clone-proposed)).

### `welcome.page` — The Welcome page

On the first start, a page that shows in five steps where the timeline, the branch rail and the inspector are and
what to try first, and opens a folder.

- **Kind**: View
- **Function**: `views::welcome::page_view(state: &WelcomeState, ui) -> Vec<Intent>`
- **Entry points**: shown by itself the first time Gitardo Desktop starts; **Welcome** in Quick search and in the
  Help menu; the window with no workspace.
- **Uses**: `repo.open-folder`, `repo.open-recent`, `repo.clone`, `repo.init`, `filter.show-only`,
  `commit-page.open`, `compare.choose`, `blame.toggle-file`, `worktree.create`.

Its title is **Get started with Gitardo**. At the top, **Open folder…**, with **Clone a repository…**, **Create a
repository…** and **Open workspace file…** beside it, and the recent workspaces, the latest first,
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
- [ ] **Clone a repository…** and **Create a repository…**, beside **Open folder…**, run `repo.clone` and `repo.init`.

## Repository tabs

Gitardo Desktop keeps several repositories open in one window, each a **tab**, and shows one at a time
([D50](../08-decisions.md#d50--repository-tabs-and-what-each-repository-keeps)). Since milestone M16 a tab is a row of
**Repositories**, the first section of the side column, and no longer a tab of an island above the header
([D56](../08-decisions.md#d56--a-side-column-of-repositories-and-branches-what-was-said-in-the-status-bar-diffs-beside-what-opened-them)).
A tab holds a repository with its worktrees and its submodules, which are rows under its row since phase 13
(`repo.group-tabs`,
[D57](../08-decisions.md#d57--worktrees-in-the-side-column-pages-that-keep-what-opened-them-the-way-back-in-the-title)). The tab's are: the header, the
branches of the side column, the timeline, the inspector, the pages and the Changes page. The window's are: the bar of
views, **Repositories**, the bottom panel's **Activity** tab and its Command log, the status bar, the
settings and the dialogs.

The extension has no tabs: VS Code's views have the list of `repo.select` for that. It keeps what each repository
showed in the same way, behind the same setting, off by default there (`repo.keep-open`).

### `repo.tabs` — The repositories open, in the side column

**Repositories**, the first section of the side column: a row for each repository open, the rows of its worktrees
and its submodules under it, the one shown marked, what each is on, and **+** to open another.

- **Kind**: View
- **Function**: `views::repo::tabs_view(state: &TabsState, ui: &mut Ui) -> Vec<Intent>`
- **Entry points**: **Repositories** at the top of the side column (`repo.side-column`). `Ctrl+1` to `Ctrl+8` show the
  repository at that place and `Ctrl+9` the last one; `Ctrl+PageDown` and `Ctrl+PageUp` the next and the one before,
  going round. On macOS `Cmd+1` to `Cmd+9`, `Cmd+Shift+]` and `Cmd+Shift+[`.
- **Offered when**: a workspace is open, even with a single repository, so that **+** is there.
- **Uses**: `repo.switch-tab`, `repo.open-tab`, `repo.close-tab`, `repo.group-tabs`, `repo.name-repositories`,
  `repo.worktree-summary`.
- **Extension**: **Gitardo Repositories**, a view of the Source Control side bar above Gitardo's other views, or
  **Gitardo: Show Repositories** ([D70](../08-decisions.md#d70--the-extension-has-a-repositories-view-of-its-own)),
  named as the views that have a namesake of VS Code's beside them are (Q724): the same rows for the repositories
  Gitardo found in the workspace (`repo.read-repositories`), without **+**, **×** and the keys of the tabs, which are
  VS Code's workspace to decide (Q720 to Q723). A row is 22 pixels high, as a row of a list of VS Code; the one
  Gitardo shows has the background of a row selected and a bar on its left, a click on a row shows it
  (`repo.select`), and nothing is chosen while an operation runs, the row of its repository showing a spinner. When
  the name and what it is on do not both fit, each gives way in proportion, the name cut at its start and the branch
  at its end, the counts kept whole. `Tab` moves to the rows, the arrows from one to the next and fold or unfold,
  `Enter` or `Space` is a click; the rows are a tree for the screen reader, each said with what it is on, the one
  shown as selected. The folders are followed by the engine only once the view is opened, and are told of changes
  only while it is in sight: what changed while it was folded is said when it is seen again (Q725). The context menu
  of a row has **Open in terminal**, **Reveal in File Explorer** as the system calls it, **Show in Gitardo** when it
  is not the one shown, and **Copy path**; the row of a worktree of the repository Gitardo shows has the menu that
  worktree has in the Branches view, and the row of a submodule of it the menu of that submodule, both made of the
  state of the repository shown, which is why the rows of the other repositories have the four alone (Q726). The view
  and VS Code's own Repositories view follow each other as `repo.follow-source-control` says, a click on a row being
  a `repo.select`. `RepositoriesApp` in `src/webview/RepositoriesApp.tsx`, `repositoryRowMenu` in
  `src/webview/menus.ts`, `RepositoriesView` in `src/extension/repositoriesView.ts`, the rows in
  `src/shared/repositories.ts`; tested by `test/repositories.test.ts`, `test/menus.test.ts` and the step "lists the
  repositories in the Repositories view with their worktrees and what each is on, counts a file changed in one not
  shown, and shows the row clicked" of `test/e2e/suite.ts`.

The heading of the section says **Repositories**, with the chevron that folds it on its left and **+** on its right
(`repo.open-tab`). Under it, a row for each repository open, in the order they were opened, each one line high, the
height of a row of a list (26 logical pixels), and under the row of a repository with worktrees or submodules, while it
is unfolded, a row for each of them, as `repo.group-tabs` says. From its left, a row has:

- the chevron that folds and unfolds those rows when it has some, the icon of a repository, its name from
  `repo.name-repositories`, and, while its rows are folded and it shows one of its worktrees or submodules, ` › ` and
  the name of that one, as `repo.group-tabs` says;
- on its right, in the small text of details and muted, what it shows is on (`repo.worktree-summary`): the branch
  checked out, or `detached at 3f2a1c9`; `● 3` when 3 files are changed and not committed, in the color of changes;
  `↓2 ↑1` when there are commits to pull and to push. What is not there is not said: a repository up to date with
  nothing changed says its branch alone;
- then a spinner while an operation runs in it, and the room of its **×**.

When the name and what it is on do not both fit, the name keeps at least half the room, cut at its start with `…`, and
the branch is cut with `…`, the counts kept whole.

The row shown has the background of a row selected, and a bar in the color of the accent on its left; the name of
the repository shown, and of its worktree or submodule shown, is in the color of text, the others muted. A click on a
row shows it (`repo.switch-tab`); unfolded, the row of a repository stands for its own folder, and a click on it shows
that folder. Each row and each chevron takes the keyboard with `Tab`, `Enter` or `Space` acting as a click, and tells
the screen reader its name and whether it is the one shown, or whether its rows are unfolded. The **×** of a row is on
the row under the pointer and the row shown, and stays while the pointer is on it, the row still lit as a row under
the pointer; a middle click closes it; its context menu has **Close tab** and **Close other tabs**
(`repo.close-tab`), **Reveal in the file manager** and **Copy path**, then **Large files…**, which shows the
repository and opens its **Large files** page (`lfs.page`), and, on the row shown, **Remember how conflicts are
resolved…**, ticked while git does (`rerere.turn-on`, `rerere.turn-off`), and, while nothing runs in it, **Clean
up…** (`upkeep.clean-up`) and **Look after it in the background…**, ticked while git's maintenance looks after the
repository (`upkeep.start-background`, `upkeep.stop-background`). Its tooltip is the folder of what it
shows. A name too long for the row is cut at its start with `…`, so that the name of the repository's own folder
stays. When the rows do not fit the height of the section, it scrolls.

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
- [ ] A row is one line, 26 logical pixels high, its name on the left and `main ● 3 ↓2` on its right; in a section 200
      logical pixels wide, a long name and a long branch are both cut with `…`, the name keeping half the row.
- [ ] The snapshots of the section, with one repository, with several and with the rows of a worktree, in every
      theme, at 100 % and 200 %, match.
- [ ] With the pointer moved from a row not shown onto its **×** and resting there ten frames, each frame draws the
      **×** and the row with the background of a row under the pointer, and no frame differs from the one before.

### `repo.side-column` — The side column of repositories and branches

One island on the left of the window, the whole height of it, with **Repositories** above, **Changes** under it and
**Branches** last, each folding by its heading, the lines between them dragged
([D56](../08-decisions.md#d56--a-side-column-of-repositories-and-branches-what-was-said-in-the-status-bar-diffs-beside-what-opened-them),
[D71](../08-decisions.md#d71--changes-a-section-of-the-side-column-whose-files-open-their-diffs)).

- **Kind**: View
- **Function**: `views::repo::side_column_view(state: &SideColumnState, ui: &mut Ui) -> Vec<Intent>`
- **Entry points**: the left of the window, beside the bar of views, under the header; it is put away and brought
  back by the first button of the top row of the timeline and of the title row of every page
  (`graph.toggle-references`).
- **Uses**: `repo.tabs`, `changes.section`, `refs.tree`, `graph.resize-panes`, `graph.toggle-references`.
- **Extension**: VS Code's Source Control side bar is the column, and Gitardo's views its sections: the Branches view
  the branches, the Repositories view the repositories (`repo.tabs`), and **Gitardo Changes** the
  changes (`changes.pane`); nothing else to build.

- The island goes from under the header to the status bar; the timeline, the inspector, the pages and the bottom
  panel are on its right, and the header above them all (`graph.toolbar`).
- **Repositories** (`repo.tabs`), with the rows of the worktrees and the submodules of each repository under it
  (`repo.group-tabs`), is above; **Changes** (`changes.section`), the changes of the repository shown, under it;
  **Branches** (`refs.tree`, the branch rail, with its filter box and its scope line) last. Each has a heading in
  capitals of the size of the headings of the rail, 28 logical pixels high, with a chevron that folds and unfolds the
  section on its left and its buttons on its right, on the same line: **+** for **Repositories** (`repo.open-tab`);
  the count of the changed files for **Changes**; **Create branch**, **Compare** and the button of the rail's own
  filter for **Branches** (`branch.pane`), each an icon with its name in its tooltip. A folded section is its heading
  alone, and the open ones take the height; with all three folded the headings are at the top and the rest of the
  island is empty.
- The headings of **Changes** and of **Branches** have a line above them, whether the sections are open or folded
  ([D71](../08-decisions.md#d71--changes-a-section-of-the-side-column-whose-files-open-their-diffs)): two folded
  sections, or a folded one under an open one, are still told apart.
- The side column is dense, as a list one reads at a glance: rows one line high, the gaps between them and between the
  sections those of a list (4 logical pixels above the filter box and between it and the lists, 8 between two lists of
  the rail), and no button less than 24 logical pixels high.
- The line under an open section that has an open section under it, folded ones between them or not, is dragged to
  share the height (`graph.resize-panes`), the arrows moving it with the keyboard; the other lines are drawn and take
  neither the pointer nor the keyboard. **Repositories** starts at 160 logical pixels, or as tall as its rows when they
  need less; **Changes** at 280, or as tall as what it shows when that is less; the last open section takes the rest,
  and none is shorter than 64 logical pixels while another is open. **Changes** is its heading alone while no
  repository is shown.
- Which sections are folded and the heights of **Repositories** and **Changes** are remembered for the workspace.
- The side column stays beside the timeline and beside every page, except a Diff page beside the inspector
  (`diff.beside-inspector`) or beside a comparison (`compare.beside-diff`), which put it away while they are shown and
  give it back as it was.
- In the narrow layout, below 900 logical pixels, it is put away, and the first button of the top row of the
  timeline opens it over the timeline, as wide as it was, until a click outside it or `Escape`.
- The title row of every page starts with the same button, at the same place as in the top row of the timeline, with
  the same icon and tooltip, pressed while the side column is shown
  ([D57](../08-decisions.md#d57--worktrees-in-the-side-column-pages-that-keep-what-opened-them-the-way-back-in-the-title)):
  it puts the side column away from beside the page and brings it back, and in the narrow layout opens it over the
  page. The title row is as high as the top row of the timeline. A page that puts the side column away itself, the
  Diff page beside the inspector or a comparison beside its diff, has no such button (Q540).

Acceptance:

- [ ] The side column's top is under the header, level with the timeline's top, and its foot touches the status
      bar's gap.
- [ ] Folding **Changes** and **Branches** gives **Repositories** the whole height but their headings.
- [ ] Folding all three leaves three headings at the top, a line between each two, and the rest of the island empty.
- [ ] The lines above the headings of **Changes** and of **Branches** are drawn whatever is folded.
- [ ] **Repositories** and **Changes** start as tall as what they show, or at 160 and 280 logical pixels, and
      **Branches** has the rest; **Changes** is its heading alone while no repository is shown.
- [ ] Every heading but the first has its line above it and the parts follow one another without overlapping,
      whatever is folded; only the line under an open section with an open one under it is dragged.
- [ ] Dragging the line between the sections changes their heights, and they are the same after the window opens
      again.
- [ ] The side column stays beside the Changes page and the settings page.
- [ ] In a window 800 logical pixels wide, the side column is not shown, and the first button of the top row of the
      timeline opens it over the timeline.
- [ ] The heading of **Branches** has **Create branch**, **Compare** and the button of the rail's filter on its line,
      each at least 24 logical pixels high.
- [ ] A repository with three worktrees and two local branches wants a side column less high than before phase 14 of
      M16: 382 logical pixels against 572.
- [ ] The first button of the title row of the settings page, of the Changes page, alone or with a diff beside its
      list, and of a Diff page puts the side column away, the page staying, and another click brings it back as it
      was.
- [ ] In a window 800 logical pixels wide, the first button of the title row of the settings page opens the side
      column over the page.
- [ ] The Diff page beside the inspector, which puts the side column away, has no button for it.

### `repo.worktree-summary` — What a repository or a worktree is on

The branch checked out in a repository or a worktree, how many files are changed there, and how many commits are to
pull and to push, for the rows of **Repositories** and the rows of the worktrees and the submodules under them.

- **Kind**: Read
- **Function**: `git::read::summary::worktree_summary(folder: &Path) -> Result<WorktreeSummary, GitError>` (Q505)
- **Reads**: `git status --porcelain=v2 --branch --untracked-files=normal` in the folder: the branch or the detached
  commit from its `# branch.head` and `# branch.oid` lines, the commits ahead and behind from `# branch.ab`, the files
  changed from the count of its other lines.
- **Uses**: `repo.common-folder`, `repo.watch-changes`; by `repo.tabs` and `repo.group-tabs`.
- **Extension**: the engine reads the same summaries for the rows of its Repositories view
  ([D70](../08-decisions.md#d70--the-extension-has-a-repositories-view-of-its-own)), with
  `read/repo.worktree-summary`, `repository/folderChanged` and `repository/summaries` of
  [the protocol](../transition/02-protocol.md#what-the-repositories-are-on): `EngineClient.summaries`,
  `folderChanged` and `onSummaries` in `src/engine/client.ts`, the words of a summary in
  `src/shared/summaries.ts`. `RepositoriesView` (`src/extension/repositoriesView.ts`) watches the folder of each
  repository, worktree and submodule followed, and says which paths changed in it, a change in a git folder to every
  folder that keeps its own git folder there (`routeChange` in `src/shared/repositories.ts`); the engine decides
  whether that asks for a read, and never reads a folder more than once in two seconds, nor while an operation runs.
  The folder Gitardo shows is not said to the engine: its row takes its summary from the state of the views
  (`summaryOfState`), and it is read by the engine once it is left. Tested by
  `rust/crates/server/tests/stdio/summaries.rs`, `test/engineSummaries.test.ts`, `test/summaries.test.ts` and
  `test/repositories.test.ts`.

- The repository shown takes its summary from its own read (`repo.read-state`), and runs no other.
- Every other repository open and every worktree of each is read once when the window opens, then again only when its
  git folder changes (its `HEAD`, its index, its refs) or the files of its folder change, as `repo.watch-changes` says,
  and not more than once in two seconds for each folder. The reads run in the background, one at a time, after the reads
  of the repository shown.
- The read of a repository not shown lists its worktrees too, with `git worktree list` alone, so that they are rows
  under it, each with its own summary, before its tab is shown; until a worktree is read, its row says the branch git
  listed for it.
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
- [ ] With three repositories open, the side column lists them under **Repositories** with the branch, the files
      changed and `↓ ↑` of each, and each worktree as a row under its repository with its own.

### `repo.open-tab` — Open a repository in a tab

Opens a repository of the workspace, or of a folder added to it, in a tab of its own.

- **Kind**: Operation
- **Function**: `app::store::tabs::open_tab(state: &mut State, folder: &Path) -> Vec<Effect>`, a function of the
  store (Q43)
- **Entry points**: **+** in the heading of **Repositories**, whose menu lists the repositories of the workspace that
  are not open, then **Open folder…**, **Clone a repository…** (`repo.clone`), **Create a repository…** (`repo.init`)
  and **Open workspace file…**; **Open Repository in a Tab** in Quick search.
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
- [ ] The menu of **+** has **Clone a repository…** and **Create a repository…** after **Open folder…**, which run
      `repo.clone` and `repo.init`.

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
- [ ] A press and a release on the **×** of a row not shown, after the pointer rested on it ten frames, close that
      tab.

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

Which repositories share a tab: a repository, its worktrees and its submodules are one tab, whose row in
**Repositories** has theirs under it, and a click on one of those shows it in the tab
([D57](../08-decisions.md#d57--worktrees-in-the-side-column-pages-that-keep-what-opened-them-the-way-back-in-the-title)).

- **Kind**: Derivation
- **Function**: `model::repo::group_tabs(repositories: &[Repository]) -> Vec<TabGroup>`, each repository with the
  common git folder `repo.read-repositories` read for it
- **Entry points**: the rows under the row of a repository in **Repositories**, folded and unfolded by the chevron at
  its left: its other worktrees, each by the name of its folder, then its submodules, each by its path, a submodule's
  own submodules indented further; each is one line, as the row of a repository is, and says what it is on on its
  right, `feature/login ● 3 ↓2 ↑1` (`repo.worktree-summary`), ending where the rows of the repositories end. The one
  the tab shows is marked as the row shown is. A click on one shows it in the tab, as `worktree.show-in-gitardo` and
  `submodule.show-in-gitardo` do. The worktree in the header lists the repository, its worktrees and its
  submodules, the same choice (`graph.where`).
- **Uses**: `repo.common-folder`, `repo.keep-open`, `repo.worktree-summary`, `worktree.show-in-gitardo`,
  `submodule.show-in-gitardo`.
- **Extension**: no tabs; the list of `repo.select` follows the first three rules, each member indented under its
  repository, its worktrees first (`groupRepositories` in `src/shared/repositoryGroups.ts`, tested by
  `test/repositoryGroups.test.ts`). The rows of the Repositories view (`repo.tabs`) follow the rules of the rows:
  under a repository, the worktrees git lists for it, a folder of the workspace or not, then its submodules found in
  the workspace, the chevron, the folds remembered for the workspace in the state of the view, and the folded row
  that says the member shown after ` › ` (`repositoryRows` and `drawnRows` in `src/shared/repositories.ts`, tested by
  `test/repositories.test.ts`). With no tab to keep a member, a folded row whose members are not shown stands for
  the folder of its repository (Q727).

The rules:

- A worktree is in the group of the repository whose common git folder it shares (`repo.common-folder`), whether it is
  a folder of the workspace or not.
- A submodule is in the group of the repository around it, and so is a submodule of a submodule.
- A repository of the workspace that is neither is a group of its own, and so a tab of its own.
- Each member of a group keeps what it showed apart from the others, as `repo.keep-open` says.
- The rows of the members are indented under the row of their repository, by the width of an icon and a gap, and a
  submodule of a submodule by one more. The row of a worktree has the icon of a worktree, the row of a submodule the
  icon of a submodule. A worktree not read yet says the branch git listed for it.
- The chevron is right while the rows are folded and down while they are shown; a repository with no other worktree
  and no submodule has none, and its room is kept so that the names stay in line. Every repository starts unfolded;
  which are folded is remembered for the workspace, as the folds of the sections are (`repo.side-column`).
- Unfolded, the row of the repository says its own folder and what that is on, and is marked while that is shown; a
  click on it shows its own folder. Folded, it says the member shown after ` › `, as below, and what that is on, and
  a click shows the tab as it was left.
- The tab is named after its repository; while it shows another member, after ` › `: the branch of a worktree, or the
  name of its folder when it is detached, as in `api › feature/login`; the path of a submodule, as in `api › lib/core`.
- Which member a tab shows is kept while the window is open; when the window opens again each tab shows its
  repository, as `worktree.show-in-gitardo` says.
- The group is named after its main worktree when that is a folder of the workspace, otherwise after the first of it
  found. The other worktrees under it are those git lists for the repository, a folder of the workspace or not: before
  the tab was read, those its read in the background listed (`repo.worktree-summary`), and before that read, those of
  the workspace. A worktree whose folder is gone has no row.
- Closing the tab asks about a page with edits not saved of any member, that member shown, as `repo.close-tab` says.

Acceptance:

- [ ] With `api` and its worktree `api-login`, on `feature/login`, both folders of the workspace, there is one row
      `api`, with the row `api-login` under it, indented, saying `feature/login`.
- [ ] A submodule `lib/core` of `api` is a row under `api`, before the next repository, and has no row of its own.
- [ ] Clicking `api-login` shows that worktree, marks its row rather than the row of `api`, and names the tab
      `api › feature/login`; clicking `api` then shows `api` as it was left.
- [ ] Two repositories that share no common git folder and neither of which is in the other are two rows, and the row
      of a repository with no other worktree and no submodule has no chevron.
- [ ] The chevron of `api` folds the rows under it, which stay folded when the window opens again; folded, the row of
      `api` says `api › feature/login` while that worktree is shown.
- [ ] `Enter` on the chevron, which has the keyboard, folds or unfolds it; the screen reader is told each row, the one
      shown as selected, and whether the chevron is unfolded.

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
  at once as it was left, and reads nothing for it when nothing changed". What the rows of its Repositories view say of
  the repositories not shown is not their state: it is read by the engine in the background, as
  `repo.worktree-summary` says, whatever the setting, and never while an operation runs or a state is read.

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

## Upkeep

How much room a repository takes, cleaning it up, and having git look after it in the background, from the context
menu of its row in **Repositories**, where the things of a repository as a whole are
([D85](../08-decisions.md#d85--notes-in-the-details-of-a-commit-archives-of-a-commit-and-the-upkeep-of-a-repository-from-its-row-proposed)).

### `upkeep.read-size` — The size of a repository

How much room the objects of a repository take, loose and packed, and how much of it git could put away.

- **Kind**: Read
- **Function**: `git::read::upkeep::read_size(git: &Git, request: &SizeRequest) -> Result<RepositorySize, GitError>`
- **Reads**: `git count-objects -v`, lines `<key>: <value>`: `count` and `size`, the loose objects and their KiB;
  `in-pack`, `packs` and `size-pack`, the objects packed, the packs and their KiB; `prune-packable`, the loose objects
  that are in a pack too; `garbage` and `size-garbage`, the files of the object folder that are not objects. Without
  `-H`, whose sizes are for people. It runs when asked, never with the reads of the repository.
- **Uses**: by `upkeep.clean-up`.
- **Extension**: read by the engine for the question of `upkeep.clean-up`: built.

Acceptance:

- [ ] The output `count: 29`, `size: 1`, `in-pack: 1200`, `packs: 2`, `size-pack: 5120` gives 29 loose objects of
      1 KiB and 1,200 packed in 2 packs of 5 MiB.
- [ ] A key it does not know is left aside.

### `upkeep.clean-up` — Clean up a repository

Packs the loose objects of a repository and deletes those nothing has reached for a long time, with `git gc`, saying the
room it takes before and after.

- **Kind**: Operation
- **Function**: `features::upkeep::clean_up(context, answers) -> Outcome`
- **Entry points**: **Clean up…** in the context menu of the row of a repository in **Repositories** (`repo.tabs`),
  on the row shown alone, since an operation runs on the repository the views show; Quick search, **Clean Up the
  Repository…**, is not there yet (Q1455).
- **Offered when**: no operation of Gitardo runs in the repository.
- **Reads**: `upkeep.read-size`.
- **Plan**: `git gc` (Q1220).
- **Danger**: Confirm, because **Commits that no branch, tag or reflog has reached for two weeks, and reflog entries
  older than 90 days, are deleted for good, as git's settings of `gc` say.** The question says, above it, the room the
  repository takes: **`<repository>` takes 1.2 GB: 830 MB in 3 packs, and 45,210 loose objects in 380 MB.**
- **Undo**: Impossible: what git deletes is not kept.
- **After**: says **Cleaned up `<repository>`, which took 1.2 GB.**: the size after is not read again yet, since a
  plan has no step after it that reads (Q1455). It runs with what is running in the status bar and in the Activity
  tab (`activity.status-bar`), as long as it takes.
- **Refuses**: git's message when another `git gc` runs in the repository.
- **Uses**: `upkeep.read-size`, `repo.tabs`, `activity.status-bar`.
- **Extension**: built: **Clean up…** in the context menu of the row of the repository shown in the **Gitardo
  Repositories** view (`repositoryRowMenu`), its question in a modal dialog, sending `feature/run` of `upkeep.clean-up`
  to the engine (Q1459).

Acceptance:

- [ ] On a repository with loose objects, it asks with the size and the warning, then runs `git gc`, and says the size
      after, smaller.
- [ ] With `commandPreview` set to `off`, it still asks.
- [ ] Cancelling runs nothing.

### `upkeep.read-background` — Whether git looks after a repository in the background

Whether the repository is among those git's scheduled maintenance looks after.

- **Kind**: Read
- **Function**: `git::read::upkeep::read_background(git: &Git, request: &BackgroundRequest) -> Result<bool, GitError>`
- **Reads**: `git config --global --get-all maintenance.repo`, the repositories registered, each compared with the
  folder of this one as git writes it (absolute, with `/`); none set reads as none registered.
- **Uses**: `repo.read-state`, with which it runs, again only when the user's configuration changed.
- **Extension**: read by the engine for the menu of the row, sent as `isLookedAfter`: built.

Acceptance:

- [ ] After `git maintenance start` in the repository, it reads as looked after; after
      `git maintenance unregister`, not.
- [ ] A folder written with `\` by another program is compared as the same folder.

### `upkeep.start-background` — Have git look after a repository in the background

Registers the repository for git's scheduled maintenance, which fetches in the background into hidden references,
packs and checks the repository each hour, day and week, so that it stays quick without a clean-up by hand.

- **Kind**: Operation
- **Function**: `features::upkeep::start_background(context, answers) -> Outcome`
- **Entry points**: **Look after it in the background…** in the context menu of the row of a repository in
  **Repositories**, unticked.
- **Offered when**: the repository is not registered (`upkeep.read-background`).
- **Plan**: `git maintenance start`, which registers the repository in the user's configuration of git, sets its
  `maintenance.strategy` to `incremental`, and schedules git in the system's scheduler (the Task Scheduler on Windows,
  launchd on macOS, systemd timers or cron on Linux) when it is not scheduled yet.
- **Danger**: Confirm, because **Git is scheduled to run in the background on this computer, every hour, for each
  repository registered.**
- **Undo**: Not taken back by Undo: **Look after it in the background**, ticked, unregisters it
  (`upkeep.stop-background`).
- **After**: the entry of the menu is ticked.
- **Refuses**: git's message when the system's scheduler refuses, as without the rights to it.
- **Uses**: `upkeep.read-background`, `upkeep.stop-background`, `repo.tabs`.
- **Extension**: built: **Look after it in the background…** in the context menu of the row of the repository shown in
  the **Gitardo Repositories** view, ticked while `isLookedAfter`, sending `feature/run` of `upkeep.start-background`
  to the engine (Q1459).

Acceptance:

- [ ] It asks with the warning, then runs `git maintenance start`, and `git config --global --get-all
      maintenance.repo` lists the repository after.
- [ ] The entry is ticked once the repository is read again.

### `upkeep.stop-background` — Stop having git look after a repository

Takes the repository out of those git's scheduled maintenance looks after; the others stay as they are.

- **Kind**: Operation
- **Function**: `features::upkeep::stop_background(context, answers) -> Outcome`
- **Entry points**: **Look after it in the background**, ticked, in the context menu of the row of a repository in
  **Repositories**.
- **Offered when**: the repository is registered.
- **Plan**: `git maintenance unregister`. The schedule of the system stays for the other repositories registered;
  `git maintenance stop`, which ends it for all of them, is not run (Q1222).
- **Danger**: None.
- **Undo**: Not taken back by Undo: `upkeep.start-background` registers it again.
- **Uses**: `upkeep.read-background`, `upkeep.start-background`.
- **Extension**: built, with `upkeep.start-background`: the same entry, ticked.

Acceptance:

- [ ] It runs `git maintenance unregister`; the repository is no longer listed, and the entry is unticked.
- [ ] Another repository registered stays registered.

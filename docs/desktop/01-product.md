# The product

Gitardo Desktop is a desktop application for Git: a commit graph, history and the everyday operations of Git, in one
window, written in Rust, with libraries where they serve. It does what the Gitardo extension for VS Code does, feature for feature,
in a window of its own. This page says what that window is made of, and how each thing the extension borrowed from
VS Code is done here instead. The features themselves are in the [catalog](../features/README.md).

## Who it is for

Someone who uses Git every day, from any editor, and wants to see the history and act on it without remembering the
commands, while always seeing the exact commands that run. Gitardo Desktop runs the real `git`: what it does can be done
again in a terminal, and what is done in a terminal shows in it at once.

## The window

The window is Gitardo's own, not an editor's ([D17](../08-decisions.md#d17--the-timeline-a-window-of-gitardos-own)): the
history fills it, and everything else opens from it.

```
┌───┬──────────────────────────────────────────────────────────────────────────────────────────┐
│ ⑂ │ gitardo › login › feature/login-page               [⌕ Search  Ctrl K] [↓2 ↑1 Sync ▾] […] │
│   │                                                                                          │
│   │ ╭────────────────╮ ╭───────────────────────────────────────────╮ ╭─────────────────────╮ │
│   │ │ ▾ REPOSITORIES+│ │ ◧ [≡ All branches ▾] [⌕ Find by message…] │ │ 4b830a6 · 2 hours   │ │
│   │ │▌gitardo       ▾│ │ ┌ Draft ────────────────────────────────┐ │ │ Add the login page  │ │
│   │ │ feature ●3 ↑2  │ │ │ Remember the e-mail of the last login │ │ │ The page asks for … │ │
│ ◷ │ │ api            │ │ │ 3 files · 2 staged  [Review] [Commit] │ │ │                     │ │
│ ▤3│ │ main ↓1        │ │ └───────────────────────────────────────┘ │ │ Dana Ruiz           │ │
│ ▣ │ ├╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┤ │ ●   Add the login page  (feature/login…)  │ │ 4 files    +180 −7  │ │
│ ⇄ │ │ ▾ BRANCHES     │ │ ●   Validate the e-mail before sending    │ │ src/login/page.rs   │ │
│   │ │ ● feature ↑2   │ │ ◉─┐ Merge branch 'fix/parser' into main   │ │ [Show the changes]  │ │
│   │ │ ● main         │ │ │ ● Read a section that has no name       │ │ [Cherry-pick] […]   │ │
│   │ │ ORIGIN         │ │                                           │ │                     │ │
│   │ │ ○ origin/main  │ │                                           │ │                     │ │
│   │ │ KEPT ASIDE     │ │                                           │ │                     │ │
│ ⓘ │ │ Stashes     3  │ │                                           │ │                     │ │
│ ⚙ │ ╰────────────────╯ ╰───────────────────────────────────────────╯ ╰─────────────────────╯ │
├───┴──────────────────────────────────────────────────────────────────────────────────────────┤
│ ◔ Fetching origin (2 s)   ✓ Pulled 3 commits  Undo  ×           Dana Ruiz <dana@example.com> │
└──────────────────────────────────────────────────────────────────────────────────────────────┘
 Bar of   The header across the top; under it the side column, the timeline and the inspector
 views                                                                          Status bar
```

- **Bar of views**: a column of icons on the left edge, outside the islands
  ([D21](../08-decisions.md#d21--islands-a-bar-of-views-and-a-status-bar)), under the logo of Gitardo, which is in the
  square at the top left of the window, before the name **Gitardo** that starts the header
  ([D58](../08-decisions.md#d58--the-window-draws-its-own-title-bar-with-the-logo-and-the-name-of-gitardo)): **Timeline**, **Changes** with the number of
  changed files, **Files** and **Compare**; at its foot **Terminal**, which shows and hides the bottom panel,
  **Activity**, which opens and closes the Activity drawer, and **Settings**. The place shown is marked.
- **Side column**: the island on the left, the whole height of the window, with two sections that fold and share its
  height: **Repositories**, a row for each repository open (a **tab**, milestone M12,
  [D50](../08-decisions.md#d50--repository-tabs-and-what-each-repository-keeps)), the one shown marked, each with the
  branch it is on, its files changed and its commits to pull and to push, its worktrees and submodules in its **▾**, and
  **+** to open another; and **Branches**, the branch rail. The tab's are the branches, the header, the timeline, the
  inspector and the pages, kept while another tab is shown (milestone M16,
  [D56](../08-decisions.md#d56--a-side-column-of-repositories-and-branches-what-was-said-in-the-status-bar-diffs-beside-what-opened-them)).
- **Islands**: the side column, the timeline and the inspector, or the page shown, are each a rounded card on a
  ground one step darker, with a gap between them that is dragged to resize them. The header above them is not one.
- **Header**: a bar across the top of the window, not an island, always shown: where one is, the **repository**, the
  **worktree** when it is not the repository's own folder and the **branch** checked out, as `gitardo › login ›
  feature/login-page`, then **Search**, the button of **Quick search** with `Ctrl K` beside it, **Sync**, which shows
  the commits to pull and to push and, clicked, fetches, pulls and pushes them, with its menu (fetch, pull, push and
  their other ways), and **…** (refresh, branch, stash, undo and the switches of the timeline). It gives up its words
  for icons when the window is narrow.
- **Status bar**: a line of small text at the foot of the window: the **activity** indicator on the left (what is
  running, which opens the **Activity** drawer), then what the last operation said, with **Undo** and what it offers
  as links, and on the right the identity commits are made with, which a click switches. It is as high whatever it
  says.
- **Branch rail**: the local branches, each with the color of its lane, so that the rail is also the legend of the
  graph; the branches of each remote, under a heading for each remote (**ORIGIN**); **Kept aside**: stashes,
  worktrees, tags and submodules, each a list that opens in the rail. The lists of Kept aside start closed and the
  others open. A click on a branch narrows the timeline to its history and selects its tip, and the rail keeps listing
  every branch, those the timeline leaves out dimmed; a second click brings the whole history back; a tag, a stash or a
  worktree goes to its commit. What the rail lists is chosen in a filter of its own, the funnel at its top, apart from
  what the timeline shows. The commits of what is checked out are the timeline narrowed to the branch checked out. The
  rail can be narrowed to icons or hidden.
- **Timeline**: the commits with their lanes, two lines a row: its labels as pills, then the subject, then the author
  and the date; the short hash on the right. The labels that do not fit are counted, and listed under the pointer. The
  selected commit is shown in the inspector.
- **Draft**: the first card of the timeline when there are uncommitted changes: the message of the next commit, how
  many files changed and how many are staged, **Review**, which opens the **Changes** page, and **Commit**, and nothing
  else: amending, the message drafted by the AI and the identity are on the Changes page. While the timeline is
  narrowed to branches that do not hold the commit checked out, the draft is not shown, and one commits from the
  Changes page.
- **Inspector**: the selected commit as a card: its hash and date, its message, its author, its files with their lines
  added and removed, and its actions. When the window is narrow it opens over the timeline.
- **Pages**: a commit, a comparison, the diff of a file, the merge of a file in conflict, a file with its blame, the
  review of the conflicts, the changes, the files of the working tree, what new worktrees get, the settings, the
  welcome page. A page opens in place of the timeline and the inspector, with a **trail** above it
  (`Timeline › Commit 4b830a6 › src/login/page.rs`) whose first step comes back to the timeline; a file of the
  inspector opens its Diff page in place of the timeline only, the inspector staying beside it, and a file of the Files
  page opens on the right of its tree, which stays (milestone M14,
  [D53](../08-decisions.md#d53--pages-beside-the-inspector-a-bottom-panel-the-icons-of-lucide-islands-in-the-extension)).
  Each page is open once: asking for it again shows the one there is. `Ctrl+Tab` goes through the pages open.
- **Activity drawer**: what is running, for how long and why; and **Tasks**, the commands run in new worktrees, with
  what they print.
- **Bottom panel**: an island under the timeline and the inspector, or the page shown, with two tabs: the
  **Terminal**, shells in the folder of the repository shown, and the **Command log**, every command that changed
  something. `` Ctrl+` `` shows and hides it; its height is dragged (milestone M14).
- **Quick search** (`Ctrl+K`, `Cmd+K` on macOS): one field that finds commits, branches, files and commands. Every
  command of the catalog that names Quick search among its entry points is there by name.

A window shows one **workspace**: one or more folders, each of which may hold repositories. **Open folder…** adds or
replaces them, a `.code-workspace` file of VS Code gives them too, and the recent workspaces are listed on the welcome
page. The repositories of the workspace the user chooses are open in tabs, all of them or some. A worktree or a
submodule opened "in a new window" is a new window of Gitardo Desktop on that folder.

### The names of the places

The cards of the catalog name the places of the window with these words, and only these:

| Place | What it is |
| --- | --- |
| **Bar of views** | The column of icons on the left edge: Timeline, Changes, Files, Compare, Activity, Settings. |
| **Side column** | The island on the left, the whole height of the window: **Repositories** above, **Branches** under it. |
| **Repositories** | The first section of the side column: a row, a **tab**, for each repository open, and **+**. |
| **Header** | The bar across the top of the window: repository, worktree and branch, Search, Sync, **…**. |
| **Island** | Each rounded card of the window: the side column, the timeline, the inspector, a page. |
| **Status bar** | The line of text at the foot of the window: what is running, what was said, the identity commits are made with. |
| **Branch rail** | **Branches**, the second section of the side column: local branches, a heading for each remote, and **Kept aside** (stashes, worktrees, tags, submodules). |
| **Timeline** | The history with its lanes, in the middle. "The graph" is the drawing of its lanes. |
| **Draft** | The card of the uncommitted changes at the top of the timeline. |
| **Inspector** | The card of the selected commit, on the right. |
| **Changes page** | Every uncommitted file, staged or not, with its diff; opened by **Review** in the draft. |
| **Files page** | The tree of the working tree, each file that differs from the last commit marked, where a file or a folder is picked for its history, its blame or a comparison. |
| **Commit**, **Compare**, **Diff**, **Merge**, **Conflicts**, **File**, **Settings**, **Welcome** and **Worktree setup** pages | The other pages, each opened in place of the timeline with its trail. |
| **Activity drawer** | What is running and the Tasks. |
| **Bottom panel** | The island under the timeline: the terminals and the Command log. |
| **Quick search** | `Ctrl+K`: commits, branches, files and commands. |

## What replaces VS Code

| In the extension | In Gitardo Desktop |
| --- | --- |
| Views in the panel and in the Source Control side bar | The timeline, the branch rail and the inspector of one window. The narrow "Gitardo Graph" view of the side bar becomes the narrow layout of the window, where the inspector opens over the timeline. |
| Editor tabs (webview panels) | Pages, opened in place of the timeline with a trail to come back. |
| The diff editor of VS Code, and the multi-file diff editor | The **Diff** page: the side-by-side diff of the Flow Diff, which is the only diff here. The diffs of several files open in one **Diff** page, one after the other. |
| The merge editor of VS Code | The **Merge** page: the three columns of the Flow Merge. |
| The text editor (for blame, line history, unsaved edits) | The **File** page, and the editable sides of the Diff and Merge pages, all on Gitardo's own text editor. Opening a file outside Gitardo uses the external editor of the setting `editor.external`. |
| The Explorer and its context menus | The **Files** page. |
| Editor context menu, editor title buttons | The context menu and the title buttons of the File page. |
| Quick pick, input box, modal dialog | Gitardo's own picker, input box and dialog, drawn in the window. |
| The command palette | **Quick search** (`Ctrl+K`), which also finds commits, branches and files. |
| The Source Control view and its list of changes | The **draft** at the top of the timeline, and the **Changes** page it opens. |
| Output channel **Gitardo** | The **Command log** in the bottom panel. |
| Status bar item and **Show What Is Running** | The activity indicator of the status bar, and the **Activity** drawer it opens. |
| `settings.json` of the user | `settings.json` in Gitardo's folder of the user's configuration. |
| `.vscode/settings.json` of a repository | `.gitardo/settings.json` of the repository, which can be committed to share it. For worktrees, VS Code's `git.worktreeSymlinkFolders` and `git.worktreeIncludeFiles` in `.vscode/settings.json` are still read as the fallback the extension has. |
| Workspace trust | A folder is trusted once the user says so; until then nothing it asks to run is run. |
| Theme colors and the file icon theme | Gitardo's own themes (dark, light, dark and light high contrast, or as the system is), its own fonts, and the icons of files and folders of Symbols, in the colors of the theme (`file-view.file-icons`). |
| Codicons | The icons of Lucide, drawn from their outlines, one meaning each (`graph.icons`). |
| The integrated terminal of VS Code | The **Terminal** in the bottom panel (`terminal.view`). |
| Language models of VS Code (Copilot) | A language model reached over HTTP, as the settings `ai.*` say, with its key in the credential store of the system. |
| GitHub account of VS Code | A GitHub token from `git credential fill` for `github.com`, or one the user gives, kept in the credential store of the system. Asked for only when the user first loads the pull requests. |
| Tasks of VS Code (setup commands of new worktrees) | **Tasks** in the Activity drawer, which runs each command with the shell of the system, in the worktree. |
| Agents window of VS Code | Not there. A worktree opens in a new window of Gitardo Desktop, in the external editor, or in a terminal. |
| VS Code's Git extension holding worktrees | Not there; but other programs can still hold a folder on Windows, so the retries and the deletion of what is left stay. |
| Walkthrough | The **Welcome** page, shown on the first start, with the same five steps. |
| `Ctrl+F` in a webview | Find in page, Gitardo's own. |
| Buttons added to VS Code's list of changes | Not there: the Changes page has them. |

Everything else the extension does, Gitardo Desktop does the same way, with the same words.

## Settings

The settings of the extension keep their names without the `gitardo.` prefix: `gitardo.graph.pageSize` is
`graph.pageSize`. They are listed, with their defaults, in [features/18-settings.md](../features/18-settings.md). The user's
file comes first unless the repository's file sets the same key; the settings that the extension read only from the
user's file stay so.

## What stays the same

- The real `git` on the `PATH` runs every operation. Gitardo needs Git 2.40 or later for the previews of conflicts, and
  works without them on older versions.
- Every command that changes something is in the log, exactly as it ran, and can be shown before it runs.
- Undo works by the reflog and by what Gitardo remembers of its own operations.
- UI text is short, plain and in sentence case, and says what will happen.

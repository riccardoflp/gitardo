# The product

Gitardo Desktop is a desktop application for Git: a commit graph, history and the everyday operations of Git, in one
window, written in Rust with no dependencies. It does what the Gitardo extension for VS Code does, feature for feature,
in a window of its own. This page says what that window is made of, and how each thing the extension borrowed from
VS Code is done here instead. The features themselves are in the [catalog](features/README.md).

## Who it is for

Someone who uses Git every day, from any editor, and wants to see the history and act on it without remembering the
commands, while always seeing the exact commands that run. Gitardo Desktop runs the real `git`: what it does can be done
again in a terminal, and what is done in a terminal shows in it at once.

## The window

```
┌───────────────────────────────────────────────────────────────────────────────────────────┐
│ Toolbar: repository ▾  branch  Fetch  Pull ▾  Push ▾  Undo  Create branch  …     ⌕ search │
├──────────────┬────────────────────────────────────────────────────────────────────────────┤
│ Side bar     │ Tabs: [Graph] [Commit 3f2a1c9] [main ↔ feature] [Conflicts] [src/app.rs ⇄]   │
│              ├───────────────┬──────────────────────────────────────┬─────────────────────┤
│ ▸ Changes    │ References    │ Graph                                │ Details             │
│ ▸ Commits    │ Recent        │ ● ─ Uncommitted changes              │ message, author,    │
│ ▸ Branches   │ Favorites     │ ●   Fix the parser       main        │ files               │
│ ▸ Stashes    │ Local         │ │ ● Add the login page   feature     │                     │
│ ▸ Files      │ Remotes …     │ ●─┘ Merge …                          │                     │
├──────────────┴───────────────┴──────────────────────────────────────┴─────────────────────┤
│ Panel: Command log │ Tasks                                                                │
├───────────────────────────────────────────────────────────────────────────────────────────┤
│ Status bar: ⟳ Gitardo — reading the repository (0.8 s)                       main ↑2 ↓0   │
└───────────────────────────────────────────────────────────────────────────────────────────┘
```

- **Toolbar**: what applies to the whole repository. It gives up its words for icons when the window is narrow.
- **Side bar**: five panes, each can be folded, resized against the others, or hidden. **Changes** stages and commits.
  **Commits** lists the commits of what is checked out. **Branches** is the references tree on its own. **Stashes**
  lists the stashes. **Files** is the tree of the working tree, which replaces the Explorer of VS Code as the place
  where a file or a folder is chosen for its history, its blame, a comparison, or what new worktrees get.
- **Tabs**: the **Graph** tab is always first and cannot be closed. The others are pages: a commit, a comparison, the
  diff of a file, the merge of a file in conflict, a file with its blame, the review of the conflicts, what new worktrees
  get, the settings, the welcome page. Each page has one tab: asking for it again shows the one it has.
- **Graph tab**: the references, the commits with their lines, and the details of the selected commit, side by side, or
  the details under the commits when the tab is narrow.
- **Panel**: the **Command log**, every command that changed something, and **Tasks**, the commands run in new
  worktrees, with what they print.
- **Status bar**: what Gitardo is running, for how long, and why; the branch and how far it is from its upstream.
- **Command palette** (`Ctrl+Shift+P`, `Cmd+Shift+P` on macOS): every command of the catalog that has the command
  palette among its entry points, by name.

A window shows one **workspace**: one or more folders, each of which may hold repositories. **Open folder…** adds or
replaces them, and the recent workspaces are listed on the welcome page. A worktree or a submodule opened "in a new
window" is a new window of Gitardo Desktop on that folder.

## What replaces VS Code

| In the extension | In Gitardo Desktop |
| --- | --- |
| Views in the panel and in the Source Control side bar | The panes of the side bar and the Graph tab of one window. The narrow "Gitardo Graph" view of the side bar becomes the narrow layout of the Graph tab. |
| Editor tabs (webview panels) | Tabs of the window. |
| The diff editor of VS Code, and the multi-file diff editor | The **Diff** page: the side-by-side diff of the Flow Diff, which is the only diff here. The diffs of several files open in one **Diff** page, one after the other. |
| The merge editor of VS Code | The **Merge** page: the three columns of the Flow Merge. |
| The text editor (for blame, line history, unsaved edits) | The **File** page, and the editable sides of the Diff and Merge pages, all on Gitardo's own text editor. Opening a file outside Gitardo uses the external editor of the setting `editor.external`. |
| The Explorer and its context menus | The **Files** pane. |
| Editor context menu, editor title buttons | The context menu and the title buttons of the File page. |
| Quick pick, input box, modal dialog | Gitardo's own picker, input box and dialog, drawn in the window. |
| Output channel **Gitardo** | The **Command log** in the panel. |
| Status bar item and **Show What Is Running** | The status bar of the window, and the **What is running** page it opens. |
| `settings.json` of the user | `settings.json` in Gitardo's folder of the user's configuration. |
| `.vscode/settings.json` of a repository | `.gitardo/settings.json` of the repository, which can be committed to share it. For worktrees, VS Code's `git.worktreeSymlinkFolders` and `git.worktreeIncludeFiles` in `.vscode/settings.json` are still read as the fallback the extension has. |
| Workspace trust | A folder is trusted once the user says so; until then nothing it asks to run is run. |
| Theme colors and the file icon theme | Built-in themes (dark, light, dark and light high contrast, or as the system is) and a built-in set of file icons. |
| Codicons | Built-in icons, drawn from paths of Gitardo's own. |
| Language models of VS Code (Copilot) | A language model reached over HTTP, as the settings `ai.*` say, with its key in the credential store of the system. |
| GitHub account of VS Code | A GitHub token from `git credential fill` for `github.com`, or one the user gives, kept in the credential store of the system. Asked for only when the user first loads the pull requests. |
| Tasks of VS Code (setup commands of new worktrees) | The **Tasks** panel, which runs each command with the shell of the system, in the worktree. |
| Agents window of VS Code | Not there. A worktree opens in a new window of Gitardo Desktop, in the external editor, or in a terminal. |
| VS Code's Git extension holding worktrees | Not there; but other programs can still hold a folder on Windows, so the retries and the deletion of what is left stay. |
| Walkthrough | The **Welcome** page, shown on the first start, with the same five steps. |
| `Ctrl+F` in a webview | Find in page, Gitardo's own. |
| Buttons added to VS Code's list of changes | Not there: the Changes pane has them. |

Everything else the extension does, Gitardo Desktop does the same way, with the same words.

## Settings

The settings of the extension keep their names without the `gitardo.` prefix: `gitardo.graph.pageSize` is
`graph.pageSize`. They are listed, with their defaults, in [features/18-settings.md](features/18-settings.md). The user's
file comes first unless the repository's file sets the same key; the settings that the extension read only from the
user's file stay so.

## What stays the same

- The real `git` on the `PATH` runs every operation. Gitardo needs Git 2.40 or later for the previews of conflicts, and
  works without them on older versions.
- Every command that changes something is in the log, exactly as it ran, and can be shown before it runs.
- Undo works by the reflog and by what Gitardo remembers of its own operations.
- UI text is short, plain and in sentence case, and says what will happen.

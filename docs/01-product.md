# The product

Gitardo is a VS Code extension for working with git: a commit graph of every branch, the history of files and lines,
and the everyday operations, from staging to interactive rebase, conflicts and worktrees, each shown as the exact git
commands it runs. It is for people who use git every day in VS Code and want to see what a command will do before it
does it. What it does is told to users in the [README](../README.md), which is also its page on the Marketplace; this
page is the map of the product for whoever changes it.

## Who it is for

- A developer in VS Code on Windows, macOS or Linux, with `git` 2.40 or later on the `PATH` (older versions work without
  the conflict previews) and VS Code 1.100 or later.
- Who wants the history in sight while working, and wants git's own commands, not a model of its own: Gitardo runs the
  `git` executable, adds no hidden flags, and logs everything it runs.
- Often with several branches and worktrees at once, and agents working in some of them.

## Where it is

```text
┌ Activity bar ┬ Source Control side bar ─────┬ Editor ───────────────────────────────────────────┐
│              │ Changes   (VS Code's)        │ tab of a commit · comparison · review ·            │
│              │ Graph     (VS Code's)        │ what new worktrees get · Flow Diff · Flow Merge ·  │
│              │ Commits           (Gitardo)  │ graph in a tab                                     │
│              │ Gitardo Graph     (Gitardo)  │                                                    │
│              │ Branches          (Gitardo)  ├ Panel ─────────────────────────────────────────────┤
│              │ Stashes           (Gitardo)  │ Graph: toolbar                                     │
│              │ Gitardo Changes   (hidden)   │ references │ commits with lines and labels │ details│
└──────────────┴──────────────────────────────┴────────────────────────────────────────────────────┘
  Status bar: Gitardo (opens the graph; spins while something runs) · Problems … · inline blame of the line
```

- **Graph**, in the panel: the toolbar, the references, the graph and the details. Also as an editor tab
  (**Gitardo: Open Graph in Editor**).
- **Commits**, **Gitardo Graph**, **Branches**, **Stashes** and **Gitardo Changes**, in the Source Control side bar,
  under VS Code's own views and folded at first. Gitardo has no icon of its own in the activity bar.
- **Tabs of the editor**: a commit, a comparison, the review of the resolutions of conflicts, what new worktrees get,
  the Flow Diff and the Flow Merge.
- **The editor itself**: the blame of the line under the cursor, the blame of the whole file beside its lines, and the
  buttons on the titles of the diff and merge editors that switch to the Flow Diff and the Flow Merge.
- **The Explorer and the Source Control view of VS Code**: entries in their context menus (history of a file or folder,
  compare with, stash this file, link into new worktrees).
- **The Command Palette**: 79 commands, all named **Gitardo: …**, listed in `package.json` under
  `contributes.commands`.
- **The Gitardo output channel**: the log of every command run.
- **The walkthrough** *Get started with Gitardo* (`media/walkthrough/`), which VS Code opens once after installing.

Each of these is described for users in the README: [Graph](../README.md#graph), [Side bar](../README.md#side-bar),
[A commit in an editor tab](../README.md#a-commit-in-an-editor-tab), [Compare](../README.md#compare),
[Worktrees](../README.md#worktrees), [Conflicts](../README.md#conflicts), [Flow Diff](../README.md#flow-diff),
[Flow Merge](../README.md#flow-merge).

## What a user can count on

These hold everywhere in the product, and the cards are checked against them
([the promises](04-feature-contract.md#the-promises-every-card-is-checked-against)):

- Every command that changes the repository or a remote is written to the log, and can be shown before it runs.
- What is shown is what runs.
- What can lose work asks first, and says what would be lost.
- **Undo** takes back the last operation, whether it ran in Gitardo or in a terminal, from the reflog or from what
  Gitardo remembered. After an operation that leaves neither, it takes back the one before, and says which
  ([RS2](13-risks.md)).
- Reading the repository never takes git's locks.
- The repository is read again on its own when its files change, also from outside VS Code.

## Settings

Settings are VS Code settings, all named `gitardo.<area>.<name>` (`gitardo.graph.pageSize`), or `gitardo.<name>` for
those of the whole extension (`gitardo.commandPreview`, `gitardo.readOnStartup`). They are declared in `package.json`
under `contributes.configuration`, with their defaults and descriptions, and read with
`vscode.workspace.getConfiguration('gitardo')`. Three of them have the scope `resource` so that a worktree or a folder
can have its own: `gitardo.worktrees.linkFolders`, `gitardo.worktrees.copyFiles` and `gitardo.worktrees.setupCommands`.
Where VS Code's Git extension has a setting for the same thing, Gitardo reads it when its own is not set
(`git.branchSortOrder`, `git.worktreeSymlinkFolders`, `git.worktreeIncludeFiles`).

Every setting, with the cards that read it, is in [features/18-settings.md](features/18-settings.md).

## What Gitardo does not do

- It does not bundle git, or use a library in its place.
- It does not keep the user's data: what it remembers is VS Code's workspace state, listed in
  [11-data.md](11-data.md).
- It does not talk to a network on its own, except GitHub for the pull requests once a view is open and the user signed
  in, and the language model the user chose, for drafting a message or resolving conflicts when asked.

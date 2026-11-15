# Platform

What Gitardo asks of VS Code, of git and of the operating system. It runs wherever VS Code's desktop build runs:
Windows, macOS and Linux. VS Code for the Web is not supported: there is no `git` to run.

## Minimum versions

| Need | Version | Where it is said |
| --- | --- | --- |
| VS Code | 1.100 | `engines.vscode` in `package.json` |
| Node.js of the extension host | 20 (what VS Code 1.100 ships) | `target: 'node20'` in `esbuild.mjs` |
| git | 2.40 for the conflict previews (`git merge-tree --write-tree`); older versions work without them | README, Command preview |

## What is asked of VS Code

| Area | What Gitardo uses | Notes |
| --- | --- | --- |
| Views | `registerWebviewViewProvider` for the six views; `createWebviewPanel` for the graph in a tab, a commit, a comparison, the review, what new worktrees get, the Flow Diff and the Flow Merge | One page per view; the Content Security Policy allows only the extension's own files and a nonce. |
| Editors | `vscode.diff`, `vscode.changes`, `vscode.open`, `_open.mergeEditor`, a `TextDocumentContentProvider` for the versions of files (`src/extension/content.ts`), decorations for the blame | `_open.mergeEditor` is internal to VS Code: it may change in a release. |
| Commands and menus | 79 commands, menus of the Explorer, the editor title, the Source Control view, `setContext` keys | Declared in `package.json`. |
| Settings | `getConfiguration('gitardo')`, and `git.*` of VS Code's Git extension as fallbacks | Three settings of scope `resource`. |
| State | `ExtensionContext.workspaceState`; each webview's `getState` and `setState` | See [11-data.md](11-data.md). |
| Files | `createFileSystemWatcher` on `.git` and on the working tree | Reads follow the files. |
| Sign-in | `authentication.getSession('github', ['repo'])`, silent unless the user asks to sign in | The token is never stored by Gitardo. |
| Language models | `vscode.lm.selectChatModels`, `sendRequest` | For drafting a message and resolving conflicts, when the user asks. |
| Terminals | `createTerminal` | To open a terminal in a worktree or submodule, and to run the setup commands of a new worktree. |
| VS Code's Git extension | the `git.close` command | So that it lets go of a worktree before it is renamed or removed. |
| The Agents window | `workbench.action.openAgentsWindow` | Internal; offered only when the command exists. It selects the folder only once the user is signed in, so no test can confirm it. |
| Workspace trust | `capabilities.untrustedWorkspaces` is not declared | VS Code therefore does not run Gitardo in Restricted Mode: a folder the user has not trusted runs none of its git commands or settings. |

## What is asked of git

Everything Gitardo reads or does is run by its engine, `gitardo-engine`, which runs git: a child process per command,
with `GIT_TERMINAL_PROMPT=0` so that git never waits for a password it cannot be given, and `GIT_OPTIONAL_LOCKS=0` for
reads. The extension itself runs no git.
Credentials for remotes are git's own (its credential helper, SSH agent); Gitardo neither asks for nor sees them.

## Windows

Windows needs care in three places, each found the slow way:

- **Links to folders.** A new worktree gets the ignored folders it should share as junctions, which need no
  administrator rights (`fs.symlink(…, 'junction')`). `git worktree remove` deletes what is behind a junction, so
  Gitardo unlinks the links to folders outside the worktree first, with an `unlink <path>` line in the log
  (`unlinkFolderLinks`).
- **Folders held open.** A folder a process runs in cannot be moved or removed. VS Code's Git extension keeps
  worktrees open, so Gitardo asks it to close one first (`git.close`); when git still could not delete the folder,
  Gitardo deletes what is left (`recoverHeldWorktree`). Gitardo itself reads nothing while an operation runs, since
  reading runs git in every worktree.
- **Paths.** Paths are compared without case on Windows and macOS, and with `\` turned into `/` where they become the
  paths of a repository.

## macOS and Linux

Links to folders are symbolic links. Nothing else differs; the tests run on the platform at hand, and the maintainer's
machine is Windows, so macOS and Linux are less exercised (see [RS5](13-risks.md)).

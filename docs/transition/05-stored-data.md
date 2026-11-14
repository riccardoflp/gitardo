# Stored data, in the extension and in the engine

What each app keeps between runs, side by side, and how the extension keeps what it keeps once the engine runs its
features. The rule does not change: a key of the workspace state, a setting, or a file Gitardo leaves in `.git` keeps
its name and its meaning ([extension/11-data.md](../extension/11-data.md), R14.2 of the extension's code rules). The
extension's side is [extension/11-data.md](../extension/11-data.md); the desktop's is read from
`rust/crates/app/src/persisted.rs` and `persisted/`, `rust/crates/app/src/settings/`, and the features that write files.

## Where each app keeps things

| What | The extension | Gitardo Desktop |
| --- | --- | --- |
| Settings of the user | VS Code's user settings, `gitardo.*` | `settings.json` in Gitardo's folder: `%APPDATA%\Gitardo Desktop`, `~/Library/Application Support/Gitardo Desktop`, `$XDG_CONFIG_HOME/gitardo-desktop` (`settings/folder.rs`) |
| Settings of a repository | `.vscode/settings.json` of the workspace folder, `gitardo.*` | `.gitardo/settings.json` of the repository (`settings/folder.rs`), with `.vscode/settings.json`'s `git.worktreeSymlinkFolders` and `git.worktreeIncludeFiles` read as a fallback (`features/src/worktree_setup/files.rs`) |
| Choices of the interface | VS Code's workspace state (7 keys) and each webview's state | `state.json` in Gitardo's folder, one per user, written whole (`persisted.rs`) |
| Secrets | none: GitHub through VS Code's sign-in, models through VS Code | the credential store of the system: the GitHub token, the key of the AI provider (`system::read_secret`) |
| Files in `.git` | `rebase-merge/gitardo-split` | `rebase-merge/gitardo-split` (`features/src/split.rs`), `gitardo-rebase-<time>/` (`features/src/message/edit.rs:189`, `rebase.rs:31`) |
| Temporary files | `<temp>/gitardo-rebase-*/`, `<temp>/gitardo-<uuid>.patch`, `<temp>/gitardo-worktree-*/` | `<temp>/gitardo-<process>-<random>.<extension>` (`engine/src/readings.rs:72`) |

## Settings

The names coincide already: the desktop's key is the extension's without `gitardo.` (`app/src/settings/keys.rs` against
`extension/package.json`). 33 keys are in both; 9 are only in the extension and 13 only in the desktop. That is what
lets the extension send its settings to the engine as they are ([02-protocol.md](02-protocol.md#settings)).

Only in the extension, and what becomes of each:

| Key | Becomes |
| --- | --- |
| `gitardo.conflicts.aiModel`, `gitardo.conflicts.aiEffort` | The desktop has `ai.model` and `ai.effort` for every use of AI. The extension keeps its two keys (they are on users' machines) and sends them as `ai.model` and `ai.effort`. Whether the extension gets `gitardo.ai.*` too is Q400. |
| `gitardo.flowDiff.collapseUnchanged` | The desktop's `diff.collapseUnchanged`; sent under that name. The Flow Diff stays in the extension, so only the webview reads it. |
| `gitardo.scm.stashButtons` | VS Code's Source Control only; never sent. |
| `gitardo.worktrees.branchFormat`, `gitardo.worktrees.inputs` | The desktop has `branches.format` and `branches.inputs` for every branch made from a format, the extension both pairs: the worktree's pair for a new worktree's branch. Sent as they are; the engine reads `worktrees.branchFormat` before `branches.format` for a worktree. That is a change to `features` (Q401). |
| `gitardo.worktrees.linkFolders`, `copyFiles`, `setupCommands` | The desktop reads them from `.gitardo/settings.json` and the user's `settings.json`, not from its settings of the window; the extension from VS Code's settings. Sent as settings while `host` is `"vscode"`, and written with `WriteSetting` (Q399). |

Only in the desktop, never sent: `theme`, `font.ui`, `font.code`, `font.size`, `editor.external`, `editor.wordWrap`,
`terminal.external`, `log.level` (the extension's is `logLevel` of `initialize`), `ai.endpoint`, `ai.provider` (sent as
`"vscode"`).

## The extension's workspace state

Each key stays where it is and keeps its name. None moves to the engine: the engine keeps nothing on disk while it runs
under the extension (`host` `"vscode"`).

| Key | Who reads and writes it once the engine runs |
| --- | --- |
| `gitardo.repository` | The extension, as today. |
| `gitardo.scope` | The extension; it sends the branches shown in `TimelineRequest.shown`. The desktop keeps the same thing as `branchesShown` in `state.json`. |
| `gitardo.favorites` | The extension; it sends them where a read needs them (none of the reads does today). The desktop's `favorites` in `state.json`. |
| `gitardo.commitFiles` | The extension's webviews. The desktop's `fileLayouts`. |
| `gitardo.recentComparisons` | The extension. The engine's `compare.choose` takes them as an answer given before it asks (`answers.recent_comparisons(RECENT)`, `features/src/compare/choose.rs:18`), as the desktop gives them from `state.json`: the extension puts them in `answers` of `feature/run`. |
| `worktreeSetupOffered` | The extension, from `Settings` as `worktrees.setupOffered` (the desktop's `FeatureSettings::worktrees.setup_offered`), written on the follow-up the feature gives. |
| `worktreeInputs` | The extension, carried out from `RememberInputs`. The desktop's `branchInputs` in `state.json`. |

The webviews' own state (widths, folds, drafts, which groups are open) does not move: it is the interface's.

## Files in `.git` and in the temporary folder

- **`gitardo-split`**, in `rebase-merge` of the worktree's git folder: the same name and the same meaning in both apps.
  A split stopped by the extension's TypeScript is continued by the engine, and the other way round. Nothing to do.
- **The files of an interactive rebase**: the extension writes them to `<temp>/gitardo-rebase-*/` (`actions.ts:900`),
  the desktop to `<git folder>/gitardo-rebase-<time>/` (`message/edit.rs:189`). The engine's place is the better one: it
  is on the same disk as the repository, it is removed with it, and the folder left by a rebase that stopped is beside
  the rebase and not in the temporary folder (the extension's [RS7](../extension/13-risks.md)). The extension takes the
  engine's place when the rebase area moves, a CHANGELOG entry; a folder `<temp>/gitardo-rebase-*` left by an older
  version is not looked at by either.
- **A patch from the clipboard**: both write it to the temporary folder and delete it after. Only the name differs;
  nothing reads it back.
- **The ignored entries of a worktree**: the extension writes the patterns to a temporary folder; the desktop gives them
  to `git ls-files` on its standard input. Nothing is kept.

## How to make them coincide

1. **Settings**: nothing to rename. The extension sends its keys without the prefix; the two that the desktop names
   otherwise (`conflicts.aiModel` → `ai.model`, `conflicts.aiEffort` → `ai.effort`, `flowDiff.collapseUnchanged` →
   `diff.collapseUnchanged`) are mapped by the extension in the one function that builds `Settings`. A setting added
   later is added to the desktop first (D39) with its name, and to the extension as `gitardo.<that name>`.
2. **The workspace state**: unchanged, owned by the extension.
3. **Files in `.git`**: the engine's names, which are a superset; the move of the rebase files is said in the CHANGELOG.
4. **The desktop's `state.json`** is not read by the extension, nor the extension's workspace state by the desktop:
   a user of both apps chooses favorites twice. Sharing them is an idea, not part of this work.
5. A test of the extension sends the `Settings` of a workspace with every `gitardo.*` key set and checks the engine's
   `FeatureSettings` field by field, so a key that is misnamed on one side fails ([07-rules.md](07-rules.md)).

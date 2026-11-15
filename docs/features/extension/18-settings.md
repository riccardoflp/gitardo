# Settings

Every setting of Gitardo, with its default and the cards that read it. They are VS Code settings, declared in
`package.json` under `contributes.configuration` with their descriptions, which are what the Settings editor shows; the
naming is in [01-product.md](../../extension/01-product.md#settings). A setting whose area has no cards yet names the area.

### `settings.read` — Read the settings when they are needed

Gitardo reads a setting at the moment it is used, not once at start, so a change in the Settings editor applies to the
next read of the repository, the next operation or the next view drawn, without reloading the window.

- **Kind**: Read
- **Function**: `vscode.workspace.getConfiguration('gitardo')`, called where each setting is used; the three settings
  of scope `resource` are read for the folder of the repository (`getConfiguration('gitardo', root)`).
- **Reads**: VS Code's user, workspace and folder settings; for `gitardo.branches.sortOrder`,
  `gitardo.worktrees.linkFolders` and `gitardo.worktrees.copyFiles`, VS Code's `git.branchSortOrder`,
  `git.worktreeSymlinkFolders` and `git.worktreeIncludeFiles` while Gitardo's are not set.
- **Implemented in**: `src/extension/controller.ts` (`graphColumns`, `branchSort`, `pageSize`, `Controller.load`, which
  puts the settings the views need into the state), `src/extension/actions.ts`, `src/extension/worktreeSettings.ts`,
  `src/extension/blame.ts`, `src/extension/fileBlame.ts`.
- **Tested by**: `test/e2e/suite.ts`, the steps that change a setting and check the views and the commands.

Acceptance:

- [ ] Turning `gitardo.graph.showStashes` off in the Settings editor takes the stashes out of an open graph without a
      reload.
- [ ] Setting `gitardo.commandPreview` to `all` makes the next operation ask, without a reload.

## The settings

| Setting | Default | Scope | What it does | Cards |
| --- | --- | --- | --- | --- |
| `gitardo.commandPreview` | `off` | window | `off`: asks only before what can lose work; `all`: shows the commands of every operation first. | `preview.ask`, and every operation |
| `gitardo.identities` | `[]` | window | Names and emails to choose from with **Gitardo: Switch Identity**, saved in the repository's configuration. | Repositories and identities (no cards yet); `changes.pane` |
| `gitardo.blame.enabled` | `true` | window | Who last changed the line under the cursor, at its end. | History and blame (no cards yet) |
| `gitardo.blame.ignoreWhitespaceAndMoves` | `false` | window | `git blame -w -M`, for the line and the file. | History and blame (no cards yet) |
| `gitardo.fileBlame.everyFile` | `false` | window | The blame of a file beside its lines in every file opened. | History and blame (no cards yet) |
| `gitardo.fileBlame.format` | `${date} ${author:20}` | window | What the blame of a file says beside each line. | History and blame (no cards yet) |
| `gitardo.fileBlame.groupLines` | `true` | window | The commit said once for lines that follow each other. | History and blame (no cards yet) |
| `gitardo.fileBlame.age.enabled` | `true` | window | A bar in the color of the line's age. | History and blame (no cards yet) |
| `gitardo.fileBlame.age.recentColor` | `#e5534b` | window | The color of a line just changed. | History and blame (no cards yet) |
| `gitardo.fileBlame.age.oldColor` | `#316dca` | window | The color of a line changed long ago. | History and blame (no cards yet) |
| `gitardo.fileBlame.age.oldAfterDays` | `730` | window | After how many days a line is old. | History and blame (no cards yet) |
| `gitardo.changes.openOnClick` | `file` | window | What a click on a file of the Changes view opens: the file or its diff. | `changes.open` |
| `gitardo.scm.stashButtons` | `true` | window | The stash buttons in VS Code's own Source Control view. | `stash.push`, `stash.push-staged`, `stash.push-file` |
| `gitardo.pull.autostash` | `true` | window | **Pull** adds `--autostash`. | `sync.pull` |
| `gitardo.pull.rebase` | `true` | window | **Pull** adds `--rebase`. | `sync.pull` |
| `gitardo.submodules.updateOnCheckout` | `true` | window | A checkout is followed by `git submodule update --init --recursive`. | `submodule.update-on-checkout`, `branch.checkout`, `branch.checkout-remote`, `branch.checkout-detached`, `branch.create`, `stash.branch` |
| `gitardo.cherryPick.recordOrigin` | `false` | window | Cherry-picks end their message with the commit they come from (`-x`). | `cherry-pick.one`, `cherry-pick.many` |
| `gitardo.branches.openIn` | `checkout` | window | What a double click on a local branch does: check it out, or open it in a worktree. | `branch.checkout`, `worktree.open-in-worktree` |
| `gitardo.readOnStartup` | `true` | window | Read the repository when VS Code starts. | Repositories and identities (no cards yet) |
| `gitardo.graph.pageSize` | `1000` | window | How many commits the graph reads at a time, from 100 to 10,000. | Graph (no cards yet) |
| `gitardo.graph.showStashes` | `true` | window | The stashes in the graph. Written by the toolbar's menu, in the user settings. | `graph.stashes` |
| `gitardo.graph.columns` | `["author", "date", "hash"]` | window | What is shown beside each message. | Graph (no cards yet) |
| `gitardo.graph.dateSeparators` | `true` | window | The lines that name yesterday, this week, each month. Written by the toolbar's menu. | Graph (no cards yet) |
| `gitardo.graph.combineUpstream` | `true` | window | A branch and its upstream on one label while at the same commit. Written by the toolbar's menu. | Graph (no cards yet); `branch.delete-remote` |
| `gitardo.graph.firstParent` | `false` | window | Follow only the first parent of merges. | Graph (no cards yet) |
| `gitardo.graph.traceSelected` | `true` | window | Bring out the history of the selected commit. | Graph (no cards yet) |
| `gitardo.branches.sortOrder` | not set | window | `name` or `date`; not set, VS Code's `git.branchSortOrder` decides, and by name without it. Written by the sections' menu. | `branch.sort` |
| `gitardo.branches.followGraphFilter` | `true` | window | The references list only the branches shown. Written by the switch above the references. | Search and filter (no cards yet) |
| `gitardo.branches.format` | `""` | window | A format for the names of new branches. | `branch.name-format`, `branch.create`, `worktree.create` |
| `gitardo.branches.inputs` | `[]` | window | What each `${input:<id>}` of the format asks. | `branch.name-format`, `branch.create`, `worktree.create` |
| `gitardo.worktrees.openIn` | `window` | window | Where a worktree opens: a new window or the Agents window. | `worktree.open`, `worktree.open-in-worktree`, `worktree.switch` |
| `gitardo.worktrees.folder` | `${repo}.worktrees` | window | Where new worktrees go. | `worktree.folder-path`, `worktree.create` |
| `gitardo.worktrees.linkFolders` | `[]` | resource | Ignored folders linked into each new worktree. Falls back to `git.worktreeSymlinkFolders`. | `worktree-setup.link-and-copy-steps`, `worktree-setup.settings-in-effect`, `worktree-setup.page`, `worktree-setup.save`, `worktree-setup.link-into-new-worktrees` |
| `gitardo.worktrees.copyFiles` | `[]` | resource | Ignored files copied into each new worktree. Falls back to `git.worktreeIncludeFiles`. | `worktree-setup.link-and-copy-steps`, `worktree-setup.settings-in-effect`, `worktree-setup.page`, `worktree-setup.save`, `worktree-setup.copy-into-new-worktrees` |
| `gitardo.worktrees.setupCommands` | `[]` | resource | Commands run in a terminal in each new worktree. | `worktree-setup.command-steps`, `worktree-setup.page`, `worktree-setup.save` |
| `gitardo.worktrees.branchFormat` | `""` | window | The older name of `gitardo.branches.format`, read while that one is not set. Has no description, so the Settings editor shows it bare. | `branch.name-format` |
| `gitardo.worktrees.inputs` | `[]` | window | The older name of `gitardo.branches.inputs`, read while that one is empty. | `branch.name-format` |
| `gitardo.conflicts.autoResolve` | `false` | window | Experimental: offer the rules for conflicts that need no choice. | `conflict.resolve-rules`, `conflict.review`, `operation.banner`, `log.report-failure` |
| `gitardo.conflicts.ai` | `false` | window | Experimental: offer the language model for the conflicts the rules leave. | `conflict.resolve-ai`, `conflict.review`, `log.report-failure` |
| `gitardo.conflicts.aiModel` | `""` | window | The model asked; empty for the first VS Code has. | `conflict.choose-model`, `conflict.resolve-ai` |
| `gitardo.conflicts.aiEffort` | `""` | window | How much the model thinks before it answers. | `conflict.choose-model`, `conflict.resolve-ai` |
| `gitardo.flowDiff.collapseUnchanged` | `true` | window | The Flow Diff folds long runs of unchanged lines. | The Flow Diff and the Flow Merge (no cards yet) |

Settings of VS Code's Git extension that Gitardo reads: `git.branchSortOrder`, `git.worktreeSymlinkFolders`,
`git.worktreeIncludeFiles`; and that it writes, when the user asks on the page of what new worktrees get:
`git.worktreeSymlinkFolders`, `git.worktreeIncludeFiles`.

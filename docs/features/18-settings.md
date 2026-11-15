# Settings

Every setting of Gitardo Desktop, with its default, what it changes, and the card that uses it. The settings are read
from two files, both JSON with comments:

- the user's, `settings.json` in Gitardo's configuration folder (`%APPDATA%\Gitardo Desktop` on Windows,
  `~/.config/gitardo-desktop` on Linux, `~/Library/Application Support/Gitardo Desktop` on macOS);
- the repository's, `.gitardo/settings.json` at its top, which can be committed to share it with everyone who works on
  it. Only the settings marked **Repository** below are read from there, and they come before the user's.

The settings of the extension keep their names without the `gitardo.` prefix. The ones marked **New** are Gitardo
Desktop's own, for what VS Code used to provide.

### `settings.read` — Read the settings

- **Kind**: Read
- **Function**: `app::settings::read(user_file, repository_file) -> Settings`
- **Uses**: the parser of JSON with comments in `crates/json`.

Acceptance:

- [ ] A setting the repository's file sets comes before the user's, for the settings marked **Repository**.
- [ ] A value of the wrong type is ignored, the default is used, and the settings page says which key and why.
- [ ] A change to either file is applied without restarting, within a second.

### `settings.edit` — Open the settings page

A page that lists every setting with its description, its value and where it comes from, and changes it in the file
the user chooses, keeping the file's comments and order.

- **Kind**: View
- **Function**: `views::settings::settings_view(state, ui) -> Vec<Intent>`
- **Entry points**: **Settings** in the **…** menu of the header; **Open Settings** in Quick search; `Ctrl+,`.

Acceptance:

- [ ] Changing a value writes only that key, and the rest of the file, comments included, is unchanged.
- [ ] **Edit in settings.json** opens the file in a File page.
- [ ] A setting marked experimental says so beside its name.

## The list

| Setting | Default | Scope | What it does | Card |
| --- | --- | --- | --- | --- |
| `commandPreview` | `"off"` | User | When the commands are shown before they run: `off` for the ones that can lose work, `all` for everything that changes something. | `preview.confirm` |
| `identities` | `[]` | User | Names, emails and optional signing keys to choose from when switching identity. | `identity.switch` |
| `readOnStartup` | `true` | User, Repository | Read the repository when the window opens, before the timeline is shown. | `repo.read-state` |
| `repositories.keepOpen` | `true` | User | Each repository keeps what it showed while another is shown, and the tabs not shown are read in the background. `gitardo.repositories.keepOpen` in the extension, where it is `false` by default. | `repo.keep-open` |
| `graph.pageSize` | `1000` | User, Repository | How many commits the timeline reads at a time. | `graph.load-more` |
| `graph.showStashes` | `true` | User | Show the stashes in the timeline. | `graph.stashes` |
| `graph.columns` | `["author","date","hash"]` | User | What is shown beside the message of each commit. | `graph.columns` |
| `graph.dateSeparators` | `true` | User | The lines of yesterday, this week, last week and each month. | `graph.date-separators` |
| `graph.combineUpstream` | `true` | User | A branch and its upstream of the same name share one label while at the same commit. | `graph.labels` |
| `graph.firstParent` | `false` | User | Follow only the first parent of merges. | `graph.first-parent` |
| `graph.traceSelected` | `true` | User | Bring out the history of the selected commit. | `graph.trace-selected` |
| `branches.sortOrder` | not set | User | `name` or `date`, for branches, remote branches and tags everywhere they are listed. Not set: by name, or by date when Git's `branch.sort` is `-committerdate` or `committerdate`. | `refs.sort` |
| `branches.followGraphFilter` | `true` | User | The branch rail lists only the branches the timeline is narrowed to. | `filter.list-too` |
| `branches.format` | `""` | User, Repository | A format for the names of new branches and worktrees. | `branch.format-name` |
| `branches.inputs` | `[]` | User, Repository | What the `${input:<id>}` of the format asks for. | `branch.format-name` |
| `branches.openIn` | `"checkout"` | User | What a double click on a local branch does: `checkout` or `worktree`. | `branch.checkout` |
| `pull.rebase` | `true` | User, Repository | **Pull** adds `--rebase`. | `sync.pull` |
| `pull.autostash` | `true` | User, Repository | **Pull** adds `--autostash`. | `sync.pull` |
| `cherryPick.recordOrigin` | `false` | User, Repository | Cherry-picks end their message with `(cherry picked from commit …)`, as `-x` does. | `cherry-pick.commit` |
| `submodules.updateOnCheckout` | `true` | User, Repository | After a checkout, update the submodules to the commits it records. | `submodule.update-on-checkout` |
| `changes.openOnClick` | `"file"` | User | What a click on a file of the Changes page opens: `file` or `diff`. | `changes.open` |
| `worktrees.openIn` | `"window"` | User | Where a worktree opens: `window` (a new window of Gitardo Desktop), `editor` (the external editor) or `terminal`. **New** values: the extension had `window` and `agents`. | `worktree.open` |
| `worktrees.folder` | `"${repo}.worktrees"` | User, Repository | Where new worktrees go. | `worktree.create` |
| `worktrees.linkFolders` | `[]` | User, Repository | Ignored folders to link into new worktrees, in `.gitignore` syntax. | `worktree-setup.link-and-copy-steps` |
| `worktrees.copyFiles` | `[]` | User, Repository | Ignored files and folders to copy into new worktrees, in `.gitignore` syntax. | `worktree-setup.link-and-copy-steps` |
| `worktrees.setupCommands` | `[]` | User, Repository | Commands to run in each new worktree, in Tasks of the Activity drawer, in a trusted folder only. | `worktree-setup.command-steps` |
| `conflicts.autoResolve` | `false` | User | **Experimental.** Offer to resolve the conflicts that need no choice. | `conflicts.resolve-by-rules` |
| `conflicts.ai` | `false` | User | **Experimental.** Offer to ask the AI provider about the conflicts the rules leave. | `ai.resolve-conflicts` |
| `blame.enabled` | `true` | User | The blame of the line under the cursor, at its end, in the File page. | `blame.current-line` |
| `blame.ignoreWhitespaceAndMoves` | `false` | User | Blame with `-w -M`. | `blame.ignore-whitespace` |
| `fileBlame.everyFile` | `false` | User | Show the blame beside the lines of every file opened. | `blame.every-file` |
| `fileBlame.format` | `"${date} ${author:20}"` | User | What the blame says beside each line. | `blame.format-line` |
| `fileBlame.groupLines` | `true` | User | Say the commit once for the lines of it that follow each other. | `blame.file` |
| `fileBlame.age.enabled` | `true` | User | The bar in the color of the age of each line. | `blame.age-color` |
| `fileBlame.age.recentColor` | `"#e5534b"` | User | The color of a line just changed. | `blame.age-color` |
| `fileBlame.age.oldColor` | `"#316dca"` | User | The color of a line changed long ago. | `blame.age-color` |
| `fileBlame.age.oldAfterDays` | `730` | User | After how many days a line has the old color. | `blame.age-color` |
| `diff.collapseUnchanged` | `true` | User | The Diff page folds long runs of unchanged lines, keeping 3 around each change. Was `flowDiff.collapseUnchanged`. | `diff.collapse-unchanged` |
| `ai.provider` | `""` | User | **New.** The kind of HTTP interface of the language model: `anthropic` or `openai-compatible`. Empty: no AI. | `ai.choose-model` |
| `ai.endpoint` | `""` | User | **New.** The base URL of the provider; empty for the provider's own. | `ai.choose-model` |
| `ai.model` | `""` | User | The model asked, as the provider names it. Was `conflicts.aiModel`, now used for drafting commit messages too. | `ai.choose-model` |
| `ai.effort` | `""` | User | How much the model thinks before it answers: empty, `low`, `medium`, `high`, `xhigh` or `max`. Was `conflicts.aiEffort`. | `ai.choose-model` |
| `editor.external` | `""` | User | **New.** The command that opens a file in another editor, with `${file}` and `${line}`, such as `code --goto ${file}:${line}`. Empty: the system's default program for the file. | `file-view.open-external` |
| `editor.wordWrap` | `"off"` | User | **New.** `on` wraps a line longer than the File page between its words, or inside a word longer than the page; `off` cuts it at the right edge. | `file-view.page` |
| `terminal.external` | `""` | User | **New.** The command that opens a terminal in a folder, with `${folder}`. Empty: the system's default terminal. | `worktree.open-in-terminal` |
| `theme` | `"system"` | User | **New.** `system`, `dark`, `light`, `highContrastDark` or `highContrastLight`. | `settings.theme` |
| `font.ui`, `font.code`, `font.size` | `"Inter"`, `"JetBrains Mono"`, `13` | User | **New.** The families and the size of the text of the interface and of code: the fonts that come with the program, or the name of a font of the system. | `settings.theme` |
| `log.level` | `"info"` | User | **New.** `info` logs what changes something; `debug` logs the reads too. | `log.record` |

Settings of the extension that have no place here: `gitardo.scm.stashButtons` (VS Code's list of changes),
`gitardo.worktrees.branchFormat` and `gitardo.worktrees.inputs` (old names of `branches.format` and `branches.inputs`,
which Gitardo Desktop does not read).

### `settings.theme` — Choose the theme and the fonts

- **Kind**: Operation
- **Function**: `features::settings::theme(context, answers) -> Outcome`
- **Entry points**: **Theme…** in the **…** menu of the header and in Quick search; the settings page.
- **Asks**: the theme, from the five of `theme`, shown as the user moves through the list.
- **Plan**: `write <settings file>` with the key changed.
- **Danger**: None
- **Undo**: Choosing again.
- **Settings**: `theme`, `font.ui`, `font.code`, `font.size`.

Acceptance:

- [ ] With `system`, the window follows the system when it switches between dark and light, without restarting.
- [ ] Every place of the window and every page is drawn with the tokens of the theme chosen, and the snapshots of each
      theme are those of the reference.

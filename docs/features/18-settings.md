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
- [ ] `hosting.selfHosted` naming `git.example.com` as `gitlab`, and `ssh.example.com` as `gitea` with its pages at
      `https://code.example.com`, gives both, by their host names in lower case; a kind it does not take, or the
      setting in the repository's file, is not taken, and the settings page says why.

### `settings.edit` — Open the settings page

A page that lists every setting with its description, its value and where it comes from, and changes it in the file
the user chooses, keeping the file's comments and order
([D55](../08-decisions.md#d55--the-settings-page-in-sections-in-the-middle-of-the-window)).

- **Kind**: View
- **Function**: `views::settings::settings_view(state, ui) -> Vec<Intent>`
- **Entry points**: **Settings** in the **…** menu of the header; **Open Settings** in Quick search; `Ctrl+,`.
- **Uses**: `settings.sections`, `settings.search`, `settings.reset`, `settings.reveal`.

The page is centered in the place of the timeline and the inspector, at most 960 logical pixels wide. At its top, the
field of `settings.search`, then two tabs, **User** and **Repository**, which choose the file a change is written in,
and **Edit in settings.json**. Under them, the column of the sections on the left (`settings.sections`) and the list of
the settings on the right.

Each setting is a row: its title, from the table below, in strong text, with **Experimental** beside it when it is;
its description, wrapped on as many lines as it needs; its key, in the text of code, and where its value comes from,
or why a value written in a file was not taken, in red. A setting a file sets has a bar on its left in the color of the
accent and ↺, which takes it back to its default (`settings.reset`). On the right of the row, its control:

| What it holds | Its control | Written |
| --- | --- | --- |
| `true` or `false` | a switch | when switched |
| one of some words | a menu of the words | when one is chosen |
| a number | a field of numbers, which refuses a value out of the range the setting allows and says the range | when the field loses the focus or `Enter` is pressed |
| a string | a field, whose placeholder says the `${…}` the setting can hold | the same |
| a color | its swatch and its hexadecimal value, `#e5534b`, in a field | the same |
| a font | a menu of the fonts that come with the program, then those of the system | when one is chosen |
| a list of strings | a row for each, with a field and ×, and **Add** under them | when a row loses the focus, or is added or taken out |
| `identities` | Identities | Pull and commits | a table of name, email and signing key, a row for each identity, with × and **Add** | the same |

**Repository** lists only the settings marked **Repository** below, and is not there while no repository is shown. In
**User**, a setting the repository's file sets too says `Set by the repository: <value>` under its key, since that value
is the one in force, and its control changes the user's file all the same.

Acceptance:

- [ ] Changing a value writes only that key, and the rest of the file, comments included, is unchanged.
- [ ] **Edit in settings.json** opens the file in a File page.
- [ ] A setting marked experimental says so beside its name.
- [ ] In a window 1,600 logical pixels wide, the page is 960 logical pixels wide and as far from either side.
- [ ] Each row shows the title of its setting, its description whole, and its key.
- [ ] `500` typed in **Commits read at a time** and `Enter` write `"graph.pageSize": 500`; `5` is refused, with the
      range said under the field, and nothing is written.
- [ ] A typed value is written once, when the field loses the focus or `Enter` is pressed, not at each key.
- [ ] A folder added to **Folders to link** writes the list with it, in the order shown.
- [ ] An identity added to the table writes `identities` with its name, its email and its signing key.
- [ ] With no repository shown, **Repository** is not there.
- [ ] With `graph.pageSize` set to `500` in the repository's file and to `2000` in the user's, **User** shows `2000` and
      says `Set by the repository: 500`.

### `settings.sections` — The sections of the settings page

The settings in sections, with a button for each on the left of the page, so that a setting is found by what it is
about.

- **Kind**: View
- **Function**: `views::settings::sections_view(state: &SectionsState, ui) -> Vec<Intent>`
- **Entry points**: the settings page (`settings.edit`).
- **Uses**: `settings.search`, `settings.reveal`.
- **Extension**: `contributes.configuration` of `package.json` is a list of categories, one for each section that has
  settings of the extension, with the same names in the same order; VS Code's Settings editor shows them in the column
  on its left. The settings the desktop alone has are in none.

The sections, in this order, each with its icon: **General**, **Appearance**, **Timeline**, **Branches**, **Pull and
commits**, **Worktrees**, **Files and blame**, **AI and conflicts**, **Terminal and tools**. Each setting is in the
section the table below names; `branches.followGraphFilter`, which is no longer read, is in none and not listed. The
list on the right of the page holds every section, one after the other, each under its name; a click on a section
scrolls the list so that its name is at the top, and the button of the section whose name was last scrolled past is
marked.

Beside the name of each section, how many of its settings a file sets, and a mark in the color of errors while one of
them has a value that was not taken. Under the sections, after a line, **Changed** and **Experimental**: each lists, in
place of the sections, only the settings a file sets, or only the experimental ones, in the order of the table, until it
is clicked again or a section is.

When the page is narrower than 640 logical pixels, the column is a button at the top of the list that names the section
marked and opens the menu of the sections.

Acceptance:

- [ ] The page opens with **General** marked and its first setting at the top of the list.
- [ ] A click on **Worktrees** scrolls the list until **Worktrees** is at its top, and marks it.
- [ ] Scrolling the list from **Timeline** into **Branches** marks **Branches**.
- [ ] With `graph.firstParent` and `pull.rebase` set in the user's file, **Timeline** and **Pull and commits** each say
      1, and **Changed** lists those two alone.
- [ ] **Experimental** lists `conflicts.autoResolve` and `conflicts.ai` alone.
- [ ] A value of the wrong type in a file marks its section in the color of errors.
- [ ] In a page 600 logical pixels wide, the menu at the top reaches every section.
- [ ] `branches.followGraphFilter` is in no section.

### `settings.search` — Find a setting

A field at the top of the settings page that keeps in the list only the settings that answer what is typed.

- **Kind**: Derivation
- **Function**: `views::settings::find(rows: &[SettingRow], query: &str) -> Found`
- **Entry points**: the field at the top of the settings page, which has the focus when the page opens; `Ctrl+F` on the
  page.

A setting answers when every word typed is found, whatever the case, in its title, its key, its description or its
value. The list keeps only the settings that answer, under the names of their sections; the buttons of the sections with
none are dimmed, and their counts say how many answer. With nothing found, the list says `No setting matches "<what
was typed>"`. `Escape` empties the field.

Acceptance:

- [ ] `blame` leaves the settings of **Files and blame** alone, and dims every other section.
- [ ] `page size` finds `graph.pageSize`, by its key.
- [ ] `rebase` finds `pull.rebase`, by its title, and nothing in **Branches**.
- [ ] `#316dca` finds `fileBlame.age.oldColor`, by its value.
- [ ] `Escape` brings back every setting, and the section marked before.

### `settings.reset` — Take a setting back to its default

The button ↺ of a setting a file sets: the key is taken out of the file, and the default, or the value of the other
file, is the one in force.

- **Kind**: Operation
- **Function**: `features::settings::reset(context, answers) -> Outcome`
- **Entry points**: ↺ on the row of a setting the file of the tab shown sets, on the settings page.
- **Offered when**: the file of the tab shown, **User** or **Repository**, sets the key.
- **Plan**: `write <settings file>` with the key taken out.
- **Danger**: None
- **Undo**: setting the value again; the Command log says the value taken out.
- **After**: the row shows the value now in force and where it comes from.

Acceptance:

- [ ] ↺ on `graph.pageSize` in **User** takes `"graph.pageSize": 500` out of the user's file, and leaves every other
      line, comments included, as it was.
- [ ] A setting no file sets has no ↺.
- [ ] With `pull.rebase` set in both files, ↺ in **Repository** takes it out of the repository's file alone, and the row
      then says it comes from the user's.

### `settings.reveal` — Open the settings page at a setting

The settings page opened, or shown, with the list scrolled to a section or a setting, from where the user was.

- **Kind**: Derivation
- **Function**: `views::settings::scroll_to(layout: &ListLayout, place: SettingsAnchor, list_height: f32, rows: &[SettingRow]) -> Option<f32>`
  ([Q481](../08-decisions.md#of-milestone-m16-the-settings-page-and-one-lane-from-q480))
- **Entry points**: **Open Settings: <section>** in Quick search, one for each section; **Open Settings: <title>** in
  Quick search, one for each setting listed.

The function gives how far the list is scrolled so that the name of the section, or the row of the setting, is at its
top, or as near as the end of the list allows. A setting is marked for a second, its row in the color of the selection,
fading.

Acceptance:

- [ ] **Open Settings: Worktrees** opens the page with **Worktrees** at the top of the list, and marked.
- [ ] **Open Settings: Commits read at a time** opens the page with `graph.pageSize` at the top, marked for a second.
- [ ] With the settings page already open, a way in scrolls the page there, and no second page opens.
- [ ] The last setting of the last section is shown whole, at the bottom of the list.

## The list

| Setting | Title | Section | Default | Scope | What it does | Card |
| --- | --- | --- | --- | --- | --- | --- |
| `commandPreview` | Show commands before they run | General | `"off"` | User | When the commands are shown before they run: `off` for the ones that can lose work, `all` for everything that changes something. | `preview.confirm` |
| `identities` | Identities | Pull and commits | `[]` | User | Names, emails and optional signing keys to choose from when switching identity. | `identity.switch` |
| `readOnStartup` | Read on startup | General | `true` | User, Repository | Read the repository when the window opens, before the timeline is shown. | `repo.read-state` |
| `repositories.keepOpen` | Keep repositories open | General | `true` | User | Each repository keeps what it showed while another is shown, and the tabs not shown are read in the background. `gitardo.repositories.keepOpen` in the extension, where it is `false` by default. | `repo.keep-open` |
| `graph.pageSize` | Commits read at a time | Timeline | `1000` | User, Repository | How many commits the timeline reads at a time. | `graph.load-more` |
| `graph.showStashes` | Show the stashes | Timeline | `true` | User | Show the stashes in the timeline. | `graph.stashes` |
| `graph.columns` | Columns beside the message | Timeline | `["author","date","hash"]` | User | What is shown beside the message of each commit. | `graph.columns` |
| `graph.dateSeparators` | Lines between dates | Timeline | `true` | User | The lines of yesterday, this week, last week and each month. | `graph.date-separators` |
| `graph.combineUpstream` | Branch and upstream on one label | Timeline | `true` | User | A branch and its upstream of the same name share one label while at the same commit. | `graph.labels` |
| `graph.firstParent` | Follow only the first parent | Timeline | `false` | User | Follow only the first parent of merges. | `graph.first-parent` |
| `graph.oneLane` | All commits on one line | Timeline | `false` | User | Every commit of the timeline on one line, without lanes; merges are rings. | `graph.one-lane` |
| `graph.compactRows` | Compact rows | Timeline | `false` | User | Each commit on one line: its labels, its message, then the author, the date and the hash in columns. | `graph.compact-rows` |
| `graph.traceSelected` | Bring out the history of the selected commit | Timeline | `true` | User | Bring out the history of the selected commit. | `graph.trace-selected` |
| `branches.sortOrder` | Order of branches and tags | Branches | not set | User | `name` or `date`, for branches, remote branches and tags everywhere they are listed. Not set: by name, or by date when Git's `branch.sort` is `-committerdate` or `committerdate`. | `refs.sort` |
| `branches.followGraphFilter` | Not listed on the page | — | `true` | User | Not read since [D51](../08-decisions.md#d51--the-branch-rail-has-a-filter-of-its-own): the branch rail has its own filter. Kept by its name, so that a file that sets it is no error. | `refs.list-filter` |
| `branches.format` | Names of new branches | Branches | `""` | User, Repository | A format for the names of new branches and worktrees. | `branch.format-name` |
| `branches.inputs` | What the names ask for | Branches | `[]` | User, Repository | What the `${input:<id>}` of the format asks for. | `branch.format-name` |
| `branches.openIn` | Double click on a branch | Branches | `"checkout"` | User | What a double click on a local branch does: `checkout` or `worktree`. | `branch.checkout` |
| `pull.rebase` | Pull with rebase | Pull and commits | `true` | User, Repository | **Pull** adds `--rebase`. | `sync.pull` |
| `pull.autostash` | Pull with autostash | Pull and commits | `true` | User, Repository | **Pull** adds `--autostash`. | `sync.pull` |
| `cherryPick.recordOrigin` | Say where cherry-picks come from | Pull and commits | `false` | User, Repository | Cherry-picks end their message with `(cherry picked from commit …)`, as `-x` does. | `cherry-pick.commit` |
| `submodules.updateOnCheckout` | Update submodules on checkout | Pull and commits | `true` | User, Repository | After a checkout, update the submodules to the commits it records. | `submodule.update-on-checkout` |
| `changes.openOnClick` | Click on a changed file | Pull and commits | `"diff"` | User | What a click on a file of the changes opens: `diff` or `file`. `gitardo.changes.openOnClick` in the extension, where it is `file` by default (Q763). | `changes.open` |
| `hosting.selfHosted` | Hosting services of your own | Pull and commits | `{}` | User | **New.** The GitLab, Gitea or Forgejo of one's own, by the host name of its remotes: `"gitlab"`, `"gitea"` or `"forgejo"`, or `{ "kind": "gitlab", "web": "https://git.example.com" }` when its pages are on another name (Q1155). `gitardo.hosting.selfHosted` in the extension. | `hosting.detect-remote` |
| `worktrees.openIn` | Open worktrees in | Worktrees | `"window"` | User | Where a worktree opens: `window` (a new window of Gitardo), `editor` (the external editor) or `terminal`. **New** values: the extension had `window` and `agents`. | `worktree.open` |
| `worktrees.folder` | Folder of new worktrees | Worktrees | `"${repo}.worktrees"` | User, Repository | Where new worktrees go. | `worktree.create` |
| `worktrees.linkFolders` | Folders to link | Worktrees | `[]` | User, Repository | Ignored folders to link into new worktrees, in `.gitignore` syntax. | `worktree-setup.link-and-copy-steps` |
| `worktrees.copyFiles` | Files to copy | Worktrees | `[]` | User, Repository | Ignored files and folders to copy into new worktrees, in `.gitignore` syntax. | `worktree-setup.link-and-copy-steps` |
| `worktrees.setupCommands` | Commands to run | Worktrees | `[]` | User, Repository | Commands to run in each new worktree, in the Tasks tab, in a trusted folder only. | `worktree-setup.command-steps` |
| `conflicts.autoResolve` | Resolve conflicts by rules | AI and conflicts | `false` | User | **Experimental.** Offer to resolve the conflicts that need no choice. | `conflicts.resolve-by-rules` |
| `conflicts.ai` | Ask the AI about conflicts | AI and conflicts | `false` | User | **Experimental.** Offer to ask the AI provider about the conflicts the rules leave. | `ai.resolve-conflicts` |
| `blame.enabled` | Blame of the current line | Files and blame | `true` | User | The blame of the line under the cursor, at its end, in the File page. | `blame.current-line` |
| `blame.ignoreWhitespaceAndMoves` | Ignore whitespace and moved lines | Files and blame | `false` | User | Blame with `-w -M`. | `blame.ignore-whitespace` |
| `fileBlame.everyFile` | Blame in every file | Files and blame | `false` | User | Show the blame beside the lines of every file opened. | `blame.every-file` |
| `markdown.openAsPreview` | Open Markdown as a preview | Files and blame | `false` | User | Open Markdown files in their preview rather than their text. | `file-view.markdown-preview` |
| `fileBlame.format` | What the blame says | Files and blame | `"${date} ${author:20}"` | User | What the blame says beside each line. | `blame.format-line` |
| `fileBlame.groupLines` | Group the lines of a commit | Files and blame | `true` | User | Say the commit once for the lines of it that follow each other. | `blame.file` |
| `fileBlame.age.enabled` | Color lines by age | Files and blame | `true` | User | The bar in the color of the age of each line. | `blame.age-color` |
| `fileBlame.age.recentColor` | Color of recent lines | Files and blame | `"#e5534b"` | User | The color of a line just changed. | `blame.age-color` |
| `fileBlame.age.oldColor` | Color of old lines | Files and blame | `"#316dca"` | User | The color of a line changed long ago. | `blame.age-color` |
| `fileBlame.age.oldAfterDays` | Days before a line is old | Files and blame | `730` | User | After how many days a line has the old color. | `blame.age-color` |
| `diff.collapseUnchanged` | Fold unchanged lines | Files and blame | `true` | User | The Diff page folds long runs of unchanged lines, keeping 3 around each change. Was `flowDiff.collapseUnchanged`. | `diff.collapse-unchanged` |
| `ai.provider` | Provider | AI and conflicts | `""` | User | **New.** The language model asked: `anthropic` or `openai-compatible` over HTTP, or `copilot` for GitHub Copilot, with the sign-in of its CLI (`ai.copilot`). Empty: no AI. | `ai.choose-model`, `ai.copilot` |
| `ai.endpoint` | Endpoint | AI and conflicts | `""` | User | **New.** The base URL of the provider; empty for the provider's own. Not used by GitHub Copilot, and not listed on the settings page while `ai.provider` is `copilot`. | `ai.choose-model` |
| `ai.model` | Model | AI and conflicts | `""` | User | The model asked, as the provider names it. Was `conflicts.aiModel`, now used for drafting commit messages too. | `ai.choose-model` |
| `ai.effort` | Effort | AI and conflicts | `""` | User | How much the model thinks before it answers: empty, `low`, `medium`, `high`, `xhigh` or `max`. Was `conflicts.aiEffort`. | `ai.choose-model` |
| `editor.external` | External editor | Terminal and tools | `""` | User | **New.** The command that opens a file in another editor, with `${file}` and `${line}`, such as `code --goto ${file}:${line}`. Empty: the system's default program for the file. | `file-view.open-external` |
| `editor.wordWrap` | Wrap long lines | Files and blame | `"off"` | User | **New.** `on` wraps a line longer than the File page between its words, or inside a word longer than the page; `off` cuts it at the right edge. | `file-view.page` |
| `terminal.external` | External terminal | Terminal and tools | `""` | User | **New.** The command that opens a terminal in a folder, with `${folder}`. Empty: the system's default terminal. | `worktree.open-in-terminal` |
| `terminal.shellPath` | Shell of the terminals | Terminal and tools | `""` | User | **New.** The shell the terminals of the bottom panel start. Empty: PowerShell 7 on Windows when it is installed, else Windows PowerShell; the shell of `SHELL` on Linux and macOS. | `terminal.view` |
| `terminal.scrollbackLines` | Lines a terminal keeps | Terminal and tools | `10000` | User | **New.** How many lines a terminal keeps above its screen. | `terminal.view` |
| `theme` | Theme | Appearance | `"system"` | User | **New.** `system`, `dark`, `light`, `highContrastDark` or `highContrastLight`. | `settings.theme` |
| `font.ui`, `font.code`, `font.size` | Interface font, code font, font size | Appearance | `"Inter"`, `"JetBrains Mono"`, `13` | User | **New.** The families and the size of the text of the interface and of code: the fonts that come with the program, or the name of a font of the system. | `settings.theme` |
| `fileIcons` | File icons | Appearance | `"symbols"` | User | **New.** The icons beside the names of files and folders: `symbols` draws those of Symbols in the colors of the theme, `monochrome` the same in the color of the text, `simple` a plain file and folder. | `file-view.file-icons` |
| `log.level` | What the log records | General | `"info"` | User | **New.** `info` logs what changes something; `debug` logs the reads too. | `log.record` |

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
- [ ] In the dark and in the light theme the selected rows of a list that has the focus are tinted with the accent of
      the theme: their ground is the accent mixed into the ground of the page, about one part in six (D69).
- [ ] In every theme the ground of the selected rows of a list, with the focus and without, of the row under the
      pointer, of the row lit in a menu and of selected text has the hue of the accent or none.
- [ ] In every theme the text of a selected row, and muted text on it, are read: 4.5 to 1, 7 to 1 in the high contrast
      themes.
- [ ] In every theme a selected row is told apart from the ground of a page, from that of a card, from the row under
      the pointer and from the selection of a list without the focus.
- [ ] In every theme selected text is told apart from a match of a search and from its ground, on a page, on a card
      and in an input box, and its text is read.
- [ ] A commit selected in the timeline, on its tall rows and on its compact rows, stands on the selection of the
      theme while its list has the focus, and on the selection of a list without the focus once the window loses it;
      the snapshots of each theme are those of the reference.
- [ ] In the dark and in the light theme every color of the interface is a step of the ground, has the hue of the
      accent, or is the color of a state: an error, a warning, information, success, a conflict, the red behind
      **Close**. A file added, removed or changed wears the color of success, of an error or of a warning, and the
      main button, the ring of the focus, a match of a search and the first lane wear the accent.

# Tasks

The tasks of a repository, listed in the **Tasks** tab of the bottom panel, each run in a terminal beside the list, and
the groups that run several at once ([D65](../08-decisions.md#d65--tasks-a-tab-of-the-bottom-panel-with-the-tasks-of-the-repository-and-their-groups)).
What the extension Run My Tasks does in VS Code; in the extension, VS Code's tasks do it, and every card is *Host only*
there.

### `tasks.read` — Read the tasks of a repository

The tasks the worktree shown defines, by source, as the Tasks tab and **Run Task…** list them.

- **Kind**: Read
- **Function**: `features::tasks::read_tasks(context) -> Tasks`
- **Uses**: `repo.watch-changes`, `trust.is-trusted`.
- **Extension**: VS Code reads its tasks itself, with the providers of its extensions: nothing to build.

- **Workspace** is the tasks of `.vscode/tasks.json` of the worktree shown, a JSON file with comments, in the order of
  the file: each by its `label`, of `type` `shell` or `process`, with its `command`, `args`, `options.cwd`,
  `options.env` and `options.shell`, the overrides of `windows`, `linux` and `osx` for the system it runs on, its
  `group`, `dependsOn`, `dependsOrder`, `isBackground` and the `beginsPattern` and `endsPattern` of its problem matcher.
  A task with `hide` set is not listed, but runs as one another depends on. A task of another `type` (`npm`, `cargo`,
  `typescript`, those of extensions) is listed dimmed, with **This kind of task runs in VS Code only.**
- The variables `${workspaceFolder}`, `${workspaceFolderBasename}`, `${userHome}`, `${pathSeparator}`, `${cwd}` and
  `${env:NAME}` are resolved, the folder being the worktree shown. A task that uses another (`${file}`,
  `${input:…}`, `${command:…}`, `${config:…}`) is listed, and says which one it cannot be run with.
- **npm** is the scripts of every `package.json` of the worktree outside `node_modules` and the folders git ignores, one
  group for each file, named by its folder (`extension`), each script by its name, its command under it. They run with
  the package manager the lock file beside the `package.json`, or the nearest above it, names: `pnpm`, `yarn`, `bun`,
  else `npm`.
- A file that cannot be read as JSON lists nothing of it, and says **`.vscode/tasks.json` could not be read: `<why>`.**
  at the top of the list, with **Open the file**.
- The tasks are read again when `.vscode/tasks.json` or a `package.json` changes, or a lock file comes or goes, in the
  worktree shown; a task running is left running and keeps its row, even when the file no longer has it.

Acceptance:

- [ ] A `tasks.json` with a comment and two shell tasks lists both, in its order, under **Workspace**.
- [ ] A task with `windows.command` runs that command on Windows and its own `command` elsewhere.
- [ ] `${workspaceFolder}` in `options.cwd` is the folder of the worktree shown, in a linked worktree too.
- [ ] A task with `${input:name}` is listed and says that it cannot be run because of `${input:name}`.
- [ ] A task of `type` `cargo` is listed dimmed, saying it runs in VS Code only.
- [ ] `extension/package.json` with `build` and `test` lists them under **npm**, in a group named `extension`; a
      `package.json` under `node_modules` lists nothing.
- [ ] With `pnpm-lock.yaml` beside the `package.json`, `build` runs `pnpm run build`.
- [ ] A `tasks.json` with a missing comma lists the npm scripts and says it could not be read.
- [ ] Adding a task to `tasks.json` lists it without **Refresh**.

### `tasks.view` — The Tasks tab

The tab of the bottom panel with the tasks of the repository on the left, and on the right the terminal of the task
selected.

- **Kind**: View
- **Function**: `views::tasks::tasks_view(state: &TasksTabState, ui) -> Vec<Intent>`
- **Entry points**: **Tasks** in the tabs of the bottom panel (`panel.bottom`); **Show Tasks** in Quick search;
  `tasks.quick-run`, which shows the tab at the task it runs.
- **Uses**: `tasks.read`, `tasks.run`, `tasks.stop`, `tasks.edit-groups`, `tasks.run-group`, `panel.bottom`,
  `terminal.view`, `file-view.page`.
- **Extension**: VS Code's own tasks and terminals, and Run My Tasks for those who have it: nothing to build.

- **Tasks** is a tab of the bottom panel, after **Terminal**. Beside its name, the number of tasks running, when some
  are.
- On the left, a column as wide as a third of the panel, its width dragged and kept: at its top a search field, every
  word of which must be in the name of a task or of its group or source; then **Groups** (`tasks.edit-groups`), then
  each source of `tasks.read`, each folding by its heading. Each row has the task's name, its command dimmed after it
  when there is room, and at its right **Run** or **Stop**; a spinner while it runs, a check when it ended with exit
  code 0, the exit code in red otherwise.
- On the right, the terminal of the task selected, drawn as those of `terminal.view`, with what it printed, the keys
  going to it while it runs; under its last line, once it ended, **The task ended with exit code `<n>`** and how long
  it ran. A task never run in the session shows its command, its folder and **Run**.
- A click on a row selects it and shows its terminal; a double click runs it, or shows it when it is running. Its
  context menu: **Run**, **Stop**, **Restart**, **Add to group**, **Open in tasks.json** (the File page at the line of
  its label), **Copy command**.
- In the title of the column, **Open tasks.json** (made with an empty `tasks` when there is none, after asking),
  **Collapse all** and **Expand all**.
- In a folder not trusted, the tab lists the tasks, every **Run** dimmed, and says at its top **Tasks run commands of
  the repository: trust this folder to run them.** with **Trust this folder** (`trust.ask`).
- Each repository tab has its own tasks and their terminals, kept while another tab is shown; the task selected and
  the width of the column are each repository tab's.

Acceptance:

- [ ] **Tasks** in the tabs of the panel shows the list and, with nothing run, the command of the task selected.
- [ ] `watch` typed in the search leaves only the tasks whose name or source holds it.
- [ ] Running a task shows its terminal on the right and a count of 1 beside **Tasks**.
- [ ] Selecting another task shows its terminal; coming back shows the first as it was.
- [ ] A task that ended with exit code 2 shows `2` in red on its row and says so under its output.
- [ ] **Open in tasks.json** opens the File page at the line of its label.
- [ ] In a folder not trusted, **Run** is dimmed and **Trust this folder** asks to trust it.

### `tasks.run` — Run a task

Runs a task in a terminal of its own, after the tasks it depends on.

- **Kind**: Operation
- **Function**: `features::tasks::run_task(context, answers) -> Outcome`
- **Entry points**: **Run** on its row in the Tasks tab, a double click on the row, **Run** and **Restart** in its
  context menu; **Run Task…** (`tasks.quick-run`); **Run all** on a group (`tasks.run-group`).
- **Uses**: `tasks.read`, `trust.is-trusted`, `terminal.view`.
- **Extension**: VS Code runs its own tasks: nothing to build.
- **Danger**: None, though what the task runs is the repository's: Gitardo runs nothing in a folder not trusted.
- **Undo**: Not taken back by Undo: what the task did is the task's own.
- **After**: the Tasks tab is shown at the task, unless it was run from a group or as one another depends on.

- A `shell` task runs its command line with the shell of `options.shell`, else that of `terminal.shellPath`; a
  `process` task runs its command with its arguments, with no shell. Both run in a pseudo-terminal of the system (ConPTY
  on Windows) with `TERM=xterm-256color`, in `options.cwd` or the worktree shown, with `options.env` over the
  environment of Gitardo, so that what it prints has its colors and what it asks can be answered.
- The tasks of `dependsOn` run first: one after the other with `dependsOrder` `sequence`, all at once otherwise. A
  task waits for those it depends on to end with exit code 0, or, for a background task, to be ready: once its
  `endsPattern` is printed, or, without one, once it has printed nothing for two seconds after its `beginsPattern` or
  its start. One that fails stops nothing that runs, but the task waiting for it does not run, and says **It did not
  run: `<task>` ended with exit code `<n>`.** A task among those it depends on already running is not run again.
- A task already running, run again, asks **`<task>` is running.** with **Restart**, which stops it as `tasks.stop`
  does and runs it again, and **Leave it running**.
- A dependency in a loop, or a task named in `dependsOn` that `tasks.read` does not have, runs nothing and says which.
- What the task changes in the repository shows as anything done in a terminal does (`repo.watch-changes`).

Acceptance:

- [ ] A shell task `git status` prints in color in its terminal, and its row shows a check once it ended.
- [ ] A process task runs its command with its arguments as given, a space in one included.
- [ ] `options.env` `FOO=1` is in the environment of the task: `echo $FOO` prints `1`.
- [ ] A task that reads a line from its input gets the line typed in its terminal.
- [ ] `dependsOn` two tasks in `sequence` runs the second once the first ended with exit code 0, and not at all when it
      ended with 1.
- [ ] A task that depends on a background task with `endsPattern` `ready` starts once that one prints `ready`, while
      it keeps running.
- [ ] Running a task that runs asks with **Restart** and **Leave it running**; **Restart** ends the first run before
      the second starts.
- [ ] Two tasks that depend on each other run nothing, and the loop is said.
- [ ] In a folder not trusted, nothing runs.

### `tasks.stop` — Stop a task

Ends a task that runs, and what it started.

- **Kind**: Operation
- **Function**: `features::tasks::stop_task(context, answers) -> Outcome`
- **Entry points**: **Stop** on its row in the Tasks tab and in its context menu; **Stop all** on a group
  (`tasks.run-group`); `Ctrl+C` in its terminal, which the task gets as it would in any terminal.
- **Uses**: `tasks.run`.
- **Extension**: VS Code stops its own tasks: nothing to build.
- **Danger**: None.
- **Undo**: Not taken back by Undo: **Run** runs it again.
- **After**: its row shows **stopped**, and its terminal keeps what it printed.

- **Stop** sends the task an interrupt, as `Ctrl+C` does, then, if it still runs three seconds later, ends it and every
  process it started: the job object of the task on Windows, its process group elsewhere.
- Closing a repository tab or the window with tasks running asks **`<n>` tasks are running.** with **Stop them and
  close** and **Cancel**; closing stops them as **Stop** does.

Acceptance:

- [ ] **Stop** on a task running `npm run watch` ends it and the node process it started, on Windows too.
- [ ] A task that ignores the interrupt is ended three seconds later.
- [ ] Its row says **stopped** and its terminal keeps its output.
- [ ] Closing the window with a task running asks first; **Cancel** leaves it running.

### `tasks.edit-groups` — Make and change groups of tasks

Groups of tasks of the repository, made by the user, listed first in the Tasks tab, the same in every worktree.

- **Kind**: Operation
- **Function**: `features::tasks::edit_groups(context, answers) -> Outcome`
- **Entry points**: **New group** in the title of **Groups** in the Tasks tab; **Add to group** in the context menu of a
  task, with the groups and **New group…**; a task dragged onto a group; **Rename**, **Delete**, **Move up** and **Move
  down** in the context menu of a group or of a task in it, `F2` on a group; a group or a task in it dragged to another
  place.
- **Uses**: `tasks.read`.
- **Extension**: Run My Tasks has its groups in VS Code: nothing to build.
- **Danger**: None.
- **Undo**: Not taken back by Undo: a group deleted is made again by hand, or imported (`tasks.import-groups`).
- **After**: every window and worktree of the repository lists the groups as they are now.

- A group has a name, unique in the repository, and its tasks in their order, each by its source and its name
  (`Workspace` `build`, `npm` `extension` `test`). A task is in a group once at most; it can be in several groups.
- The groups are kept in `gitardo/task-groups.json` in the common git folder of the repository (`git rev-parse
  --git-common-dir`), so that every worktree has the same, written whole on each change; Gitardo reads it again when it
  changes. Outside a repository there are no groups.
- A task of a group that `tasks.read` no longer has stays in the group, dimmed, saying **`<task>` is not in this
  worktree.**, so that another worktree that has it keeps it.
- **Delete** a group asks **Delete the group `<name>`?**; the tasks stay where they are.

Acceptance:

- [ ] **New group** named `Servers` lists an empty group under **Groups**.
- [ ] A task dragged onto `Servers` is listed in it; dragged above another task of the group, it goes before it.
- [ ] The groups are the same in another worktree of the repository, without reading anything again by hand.
- [ ] A task removed from `tasks.json` stays in its group, dimmed, saying that it is not in this worktree.
- [ ] Renaming a group to the name of another is refused, saying so.
- [ ] `gitardo/task-groups.json` written by a change is read back with the same groups, in the same order.

### `tasks.run-group` — Run or stop a group of tasks

Runs every task of a group, in its order, or stops those of them that run.

- **Kind**: Operation
- **Function**: `features::tasks::run_group(context, answers) -> Outcome`
- **Entry points**: **Run all** and **Stop all** on the heading of a group in the Tasks tab, and in its context menu; a
  group chosen in **Run Task…** (`tasks.quick-run`).
- **Uses**: `tasks.run`, `tasks.stop`, `tasks.edit-groups`.
- **Extension**: Run My Tasks runs its groups in VS Code: nothing to build.
- **Danger**: None, as `tasks.run`.
- **Undo**: Not taken back by Undo.
- **After**: the Tasks tab is shown with the first task of the group selected.

- **Run all** runs each task of the group as `tasks.run` does, in the order of the group, each started as soon as the
  one before it started, without waiting for it to end; a task already running is left running. A task the worktree
  does not have is left out and said.
- **Stop all** stops every task of the group that runs, as `tasks.stop` does.
- The heading of a group shows a spinner while one of its tasks runs, and **Stop all** in place of **Run all**.

Acceptance:

- [ ] **Run all** on a group of two tasks runs both, the first started first.
- [ ] A task of the group already running is not run again, nor asked about.
- [ ] **Stop all** ends both.
- [ ] A task of the group missing from the worktree is left out, the others run, and the missing one is said.

### `tasks.import-groups` — Import groups of tasks

Adds the groups of a file saved by **Export groups…**, or by **Export Groups...** of Run My Tasks, to those of the
repository.

- **Kind**: Operation
- **Function**: `features::tasks::import_groups(context, answers) -> Outcome`
- **Entry points**: **Import groups…** in the **…** menu of **Groups** in the Tasks tab, and in Quick search.
- **Uses**: `tasks.edit-groups`, `tasks.read`.
- **Extension**: Run My Tasks imports its own groups in VS Code: nothing to build.
- **Danger**: None.
- **Undo**: Not taken back by Undo: a group imported is deleted as any other.
- **After**: the groups imported are listed under **Groups**.

- The file is chosen with the system's dialog; Gitardo reads the format Run My Tasks 0.8.0 writes, and lists its
  groups to choose from, all chosen.
- For a group named as one of the repository, Gitardo asks **`<name>` is already a group.** with **Merge** (the tasks
  it lacks added after its own), **Replace** and **Skip**, and **Do the same for the others** when there are more.
- The tasks are taken by their source and their name; the launch configurations of the file are left out, since
  Gitardo has no debugger. Once done, Gitardo says how many groups were imported, the tasks the worktree lacks, and
  how many launch configurations were left out.
- A file that is not such a file imports nothing and says so.

Acceptance:

- [ ] A file exported by Run My Tasks 0.8.0 with two groups imports both, with their tasks in their order.
- [ ] **Merge** on a group named as one of the repository adds only the tasks it lacks.
- [ ] A group with a launch configuration is imported without it, and the import says one was left out.
- [ ] A JSON file of another kind imports nothing and says it is not a file of groups.

### `tasks.export-groups` — Export groups of tasks

Saves groups of the repository to a file another repository, or Run My Tasks in VS Code, can import.

- **Kind**: Operation
- **Function**: `features::tasks::export_groups(context, answers) -> Outcome`
- **Entry points**: **Export groups…** in the **…** menu of **Groups** in the Tasks tab, and in Quick search.
- **Uses**: `tasks.edit-groups`.
- **Extension**: Run My Tasks exports its own groups in VS Code: nothing to build.
- **Danger**: None.
- **Undo**: Not taken back by Undo: the file is the user's.
- **After**: Gitardo says where the file was saved, with **Show in folder**.

- Gitardo asks which groups, all chosen, then where, with the system's dialog, `task-groups.json` proposed; the file
  is the format Run My Tasks 0.8.0 reads, with no launch configurations.

Acceptance:

- [ ] Two groups exported and imported in another repository give the same groups, in the same order.
- [ ] The file exported is imported by Run My Tasks 0.8.0 with its groups.

### `tasks.quick-run` — Run a task from anywhere

A picker of every task and group of the repository, from Quick search or a key, wherever the window is.

- **Kind**: View
- **Function**: `views::tasks::quick_run_view(state: &QuickRunState, ui) -> Vec<Intent>`
- **Entry points**: **Run Task…** in Quick search; `Ctrl+Shift+T` (`Cmd+Shift+T` on macOS), as in Run My Tasks.
- **Uses**: `tasks.read`, `tasks.run`, `tasks.run-group`, `graph.search-button`.
- **Extension**: VS Code's **Tasks: Run Task**, and Run My Tasks' picker for those who have it: nothing to build.

- The picker lists the groups first, then the tasks by source, as the Tasks tab orders them, each with its command
  under its name, its words filtered as typed; a task running has its spinner.
- `Enter` runs the task or the group chosen (`tasks.run`, `tasks.run-group`) and shows the bottom panel at the Tasks
  tab, the task selected; a task running is shown rather than run again.

Acceptance:

- [ ] `Ctrl+Shift+T` from the timeline opens the picker with the groups first.
- [ ] `build` typed then `Enter` runs `build` and shows its terminal in the Tasks tab.
- [ ] A task running chosen in the picker shows its terminal and runs nothing.

# Undo, command preview and the log

What every operation gets from the machinery that runs it: the question before it with its exact commands, the log of
what ran, what is offered when git refuses, what is said when it fails, **Undo** afterwards, and the item of the status
bar while something takes time. The features of the other areas name these cards rather than repeating them.

### `preview.ask` — Show the exact commands before an operation runs

Before an operation changes anything, Gitardo can show the commands it will run and the files it will write, and wait
for the user to agree. It does so for every operation when the user asked for it, and always for those that can lose
work or rewrite history.

- **Kind**: Operation
- **Function**: `VscodeAsker.confirm(plan)` in `src/engine/vscode.ts`, which the engine calls, through
  `Controller.runOnEngine` in `src/extension/controller.ts`, for a plan it wants confirmed.
- **Entry points**: every operation of every area; the setting `gitardo.commandPreview`.
- **Offered when**: the engine asks it: `gitardo.commandPreview` is `all`, or the plan's danger asks for a
  confirmation, and the feature did not ask already.
- **Asks**: a modal dialog titled with the plan's title and a question mark, *Create branch feature/x?*, with one
  button, **Run**, and VS Code's **Cancel**. A warning dialog when the plan is destructive, an information dialog
  otherwise. Its detail is the reason of the plan's danger, if any, then *Gitardo will run in `<root>`:* followed by
  the lines of its steps, one per line, as the engine writes them.
- **Plan**: none of its own: it decides whether the plan of the operation runs.
- **Danger**: None: it is the question itself.
- **Undo**: Nothing to undo: cancelling runs nothing.
- **Settings**: `gitardo.commandPreview`: `off` (default) asks only before `danger` and `confirm` operations; `all`
  asks before every operation that changes the repository or a remote, staging included.
- **Uses**: `log.write`.
- **Implemented in**: `VscodeAsker` (`src/engine/vscode.ts`) and `Controller.runOnEngine`; the plans, their danger
  and the lines of their steps are the engine's.
- **Tested by**: `test/e2e/suite.ts`, *stages and commits without asking when the preview is off*, *shows the exact
  commands and waits when the preview is on*; `test/e2e/ui.mjs`, the step where, with the command preview on, the dialog
  shows the exact command and **Cancel** runs nothing.

Acceptance:

- [ ] With `gitardo.commandPreview` `off`, staging a file runs `git add` without a dialog.
- [ ] With `gitardo.commandPreview` `all`, staging a file shows a dialog whose detail is *Gitardo will run in
      `<root>`:* and the `git add` command, and nothing runs until **Run** is pressed.
- [ ] **Cancel** runs no command and writes nothing to the log.
- [ ] A `danger` operation, such as a hard reset, shows a warning dialog whatever the setting.
- [ ] The commands in the dialog are, character for character, the lines of the steps written to the log and run, as
      the engine writes them.
- [ ] An operation asked for while another runs does nothing: `Controller.runOnEngine` returns without asking or
      running.

### `preview.fallback` — Offer the other way when git refuses

When git refuses an operation in a way the operation foresaw, typically because it would need to be forced, Gitardo
offers the forced variant instead of only reporting the failure: deleting a branch that is not merged, removing a
worktree with changes.

- **Kind**: Operation
- **Function**: the operations the engine offers when a step of its plan fails (the follow-ups `offerRun` of the
  failure), shown as buttons of the error message by `Controller.reportEngineFailure` in
  `src/extension/controller.ts`, and the one it offers with a refusal, shown by `Controller.offerAfterRefusal`.
- **Entry points**: every feature of the engine that offers one, such as **Delete anyway** for a branch not merged.
- **Offered when**: the engine says so: git refused the first plan in a way the feature foresaw.
- **Asks**: the offered operation runs on the engine as any operation does, asked as `preview.ask` says, with its own
  title, commands and danger.
- **Plan**: the engine's plan of the offered operation.
- **Danger**: the offered operation's own, usually Destructive, because it is the forced variant.
- **Undo**: what the offered operation's own card says.
- **Refuses**: when nothing is offered, the failure is reported as any failure (`log.report-failure`).
- **Uses**: `preview.ask`, `log.report-failure`.
- **Implemented in**: `Controller.reportEngineFailure` and `Controller.offerAfterRefusal`, after
  `Controller.runOnEngine`.
- **Tested by**: `test/e2e/suite.ts`, the steps that delete a branch that is not merged; *none* for the matching itself.

Acceptance:

- [ ] What git printed when the first plan failed is written to the log.
- [ ] When the engine offers an operation, the error message has its button, which runs it with its own dialog, and
      **Cancel** leaves the repository as the first attempt left it.
- [ ] When nothing is offered, the user sees the failure of the first attempt, once.

### `log.write` — Write every command and its output to the log

Every command Gitardo runs to change something, and everything it does itself, is written to the **Gitardo** output
channel as it happens, so that the user can see and repeat it.

- **Kind**: Operation
- **Function**: `EngineClient.logLine` in `src/engine/client.ts`, which writes the lines the engine logs to the
  `LogOutputChannel` made in `src/extension/extension.ts`.
- **Entry points**: every operation.
- **Plan**: for each step of a plan, a line `> <step>` as the engine writes it: `> git <arguments>`, then *standard
  input:* and the input when the command reads one, `> write <path>` for a file written, and the steps that are not
  git (`symlink`, `copy`, `run`, `unlink`, `delete`); the other lines of the engine after `[engine]`; what git printed
  when a step failed, and an error as an error line; `> git.close <folder>` when VS Code's Git extension lets go of a
  folder (`closeInVSCodeGit` in `src/extension/actions.ts`).
- **Danger**: None: it changes nothing.
- **Undo**: Nothing to undo.
- **After**: reads of the repository (`git log`, `git status` and the like) are written at the debug level, which the
  channel shows only when its level is set to Debug.
- **Implemented in**: `EngineClient.logLine`; `WindowEngine.start` (`src/engine/vscode.ts`) tells the engine the level
  of the channel, so that it logs the reads only at the debug level; `Activity` writes there too.
- **Tested by**: `test/e2e/suite.ts`, the steps that compare the logged lines with the commands shown, for instance
  `> git pull --autostash` and `> git fetch origin refs/heads/lagging:refs/heads/lagging`.

Acceptance:

- [ ] Each command run by an operation appears in the channel as `> git …`, before its output.
- [ ] A command fed a patch on its standard input is followed by *standard input:* and the patch.
- [ ] Files written by the operation appear as `> write <path>` before the commands.
- [ ] When a command fails, what git printed is written as an error, and no later command of the operation runs.
- [ ] A read never appears at the default level of the channel.

### `log.show` — Show the command log

**Gitardo: Show Command Log** opens the **Gitardo** output channel.

- **Kind**: View
- **Function**: the `gitardo.showLog` command in `src/extension/extension.ts`, which calls `log.show()`.
- **Entry points**: **Gitardo: Show Command Log** in the Command Palette; **Show Log** on the message of a failure;
  **Show the commands Gitardo runs** in the list of what is running.
- **Implemented in**: `src/extension/extension.ts`; `Controller.reportFailure`; `Activity`.
- **Tested by**: `test/e2e/suite.ts`, *registers every command it contributes*.

Acceptance:

- [ ] **Gitardo: Show Command Log** shows the Output panel with the **Gitardo** channel selected.

### `log.report-failure` — Say why an operation failed

When an operation fails, Gitardo says so in one line with what git said that matters, or, when the operation stopped on
conflicts, how many files have them and what to do next.

- **Kind**: View
- **Function**: `Controller.reportEngineFailure(failure, title, repo)` in `src/extension/controller.ts`, which hands a
  step that failed with files left in conflict to `Controller.reportFailure(op, error)`.
- **Entry points**: the end of every operation that failed.
- **After**:
  - With files in conflict: a warning *`<title>` stopped: 1 file has conflicts. Resolve them, then continue or abort
    the `<operation>`.* (*N files have conflicts*; *stage the files* when no operation is under way, as after a stash
    that did not apply cleanly), followed by *Gitardo can resolve the conflict automatically.*, *all N conflicts* or
    *N of the M conflicts* when the rules can, and the buttons **Resolve Automatically** (when the rules can),
    **Resolve with AI** (when `gitardo.conflicts.ai` is on and some conflict is left to it) and **Show Log**.
  - Otherwise: an error *Gitardo:* followed by the engine's words of why it failed, with a button for each operation
    the engine offers in its place and **Show Log**.
- **Settings**: `gitardo.conflicts.autoResolve`, `gitardo.conflicts.ai`.
- **Uses**: `conflict.resolve-rules`, `conflict.resolve-ai`, `log.show`.
- **Implemented in**: `Controller.reportEngineFailure` and `Controller.reportFailure`.
- **Tested by**: `test/e2e/suite.ts`, the steps of a merge that stops on conflicts with the automatic resolution on.

Acceptance:

- [ ] A push refused by the remote shows *Gitardo:* and the engine's words of why, with **Show Log**.
- [ ] A merge that stops on conflicts in two files shows *Merge … stopped: 2 files have conflicts. Resolve them, then
      continue or abort the merge.*
- [ ] **Resolve Automatically** is offered only when the rules can resolve at least one conflict, and with
      `gitardo.conflicts.autoResolve` off it is never run (`conflict.resolve-rules`).

### `activity.track` — Show what is running in the status bar

While Gitardo has been reading a repository, comparing two sides or asking GitHub for more than half a second,
**Gitardo** with a spinning icon shows in the status bar, with what each of those is in its tooltip.

- **Kind**: View
- **Function**: `Activity.start(title, detail)` and `Activity.track(title, work)` in `src/extension/activity.ts`.
- **Entry points**: the status bar item **Gitardo Activity**, beside the item that opens the graph.
- **Offered when**: something has been running for 500 ms; it goes when the last thing ends.
- **After**: the tooltip lists each thing, oldest first, in bold with how long it has run, then its detail: what asked
  for a read (such as a file that changed) and how many times in a row the repository was read because something
  changed during each read. It ends with *Click to see what Gitardo is doing in a list.*
- **Implemented in**: `src/extension/activity.ts`; `Controller.load` (reads), `Controller.refreshPulls` (GitHub), the
  comparisons. An operation run by the engine is not tracked there: the views show it as busy.
- **Tested by**: *none* found by name; the status bar item is reached by `test/e2e/suite.ts` only through the commands.

Acceptance:

- [ ] A read that ends within 500 ms never shows the item.
- [ ] A read of the repository that runs for two seconds shows **Gitardo** with a spinning icon after half a second,
      with its title in the tooltip, and the item goes when it ends.

### `activity.show` — List what is running

A click on the item, or **Gitardo: Show What Is Running**, lists the same as the tooltip, kept up to date while it is
open, with the way to the commands Gitardo runs in its log.

- **Kind**: View
- **Function**: the `gitardo.showActivity` command (`SHOW_ACTIVITY`), handled by `Activity` in
  `src/extension/activity.ts`.
- **Entry points**: the status bar item; **Gitardo: Show What Is Running** in the Command Palette.
- **Uses**: `log.show`.
- **Implemented in**: `src/extension/activity.ts`.
- **Tested by**: `test/e2e/suite.ts`, *registers every command it contributes*.

Acceptance:

- [ ] The list is titled **Gitardo**, with the placeholder *What Gitardo is doing now*.
- [ ] With nothing running, it lists **Nothing is running now**, *Gitardo is not reading or changing anything*.
- [ ] With a read running, it lists the read with its time (*3 s*, *1 min 5 s*), counted again every second while the
      list is open.
- [ ] Its last entry, **Show the commands Gitardo runs**, opens the **Gitardo** output channel.

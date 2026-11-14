# Undo, command preview and the log

What every operation gets from the machinery that runs it: the question before it with its exact commands, the log of
what ran, what is offered when git refuses, what is said when it fails, **Undo** afterwards, and the item of the status
bar while something takes time. The features of the other areas name these cards rather than repeating them.

### `preview.ask` — Show the exact commands before an operation runs

Before an operation changes anything, Gitardo can show the commands it will run and the files it will write, and wait
for the user to agree. It does so for every operation when the user asked for it, and always for those that can lose
work or rewrite history.

- **Kind**: Operation
- **Function**: `Controller.run(op, options)` and `Controller.describe(repo, op)` in `src/extension/controller.ts`.
- **Entry points**: every operation of every area; the setting `gitardo.commandPreview`.
- **Offered when**: `gitardo.commandPreview` is `all`, or the operation has `danger` or `confirm`, and the operation
  was not asked already by the feature itself (`asked`: the confirmation of **Resolve Conflicts Automatically**, and the
  `then` of an operation, which is said along with it).
- **Asks**: a modal dialog titled with the operation's title and a question mark, *Create branch feature/x?*, with one
  button, **Run**, and VS Code's **Cancel**. A warning dialog when the operation has `danger`, an information dialog
  otherwise. Its detail is the operation's `note`, if any, then:
  - with no files to write: *Gitardo will run in `<root>`:* followed by the commands, one per line;
  - with files to write: *Gitardo will write in `<root>`:* followed by the files, then *and then run:* and the
    commands;
  - with a `then`: the commands followed by *and once that is done:* and the commands of the `then`;
  - with neither commands nor files: only the `note`.
- **Plan**: none of its own: it decides whether the plan of the operation runs.
- **Danger**: None: it is the question itself.
- **Undo**: Nothing to undo: cancelling runs nothing.
- **Settings**: `gitardo.commandPreview`: `off` (default) asks only before `danger` and `confirm` operations; `all`
  asks before every operation that changes the repository or a remote, staging included.
- **Uses**: `log.write`.
- **Implemented in**: `Controller.run` (`src/extension/controller.ts`), `formatCommand` (`src/git/exec.ts`), the
  `danger`, `confirm` and `note` fields set by the builders of `src/git/ops.ts` and the cases of `resolveAction`.
- **Tested by**: `test/e2e/suite.ts`, *stages and commits without asking when the preview is off*, *shows the exact
  commands and waits when the preview is on*; `test/e2e/ui.mjs`, the step where, with the command preview on, the dialog
  shows the exact command and **Cancel** runs nothing.

Acceptance:

- [ ] With `gitardo.commandPreview` `off`, staging a file runs `git add` without a dialog.
- [ ] With `gitardo.commandPreview` `all`, staging a file shows a dialog whose detail is *Gitardo will run in
      `<root>`:* and the `git add` command, and nothing runs until **Run** is pressed.
- [ ] **Cancel** runs no command and writes nothing to the log.
- [ ] A `danger` operation, such as a hard reset, shows a warning dialog whatever the setting.
- [ ] The commands in the dialog are, character for character, the commands written to the log and run, as
      `formatCommand` writes them: an argument with characters other than letters, digits, `_` and `@%+=:,./-` is
      quoted.
- [ ] An operation that writes files lists them before the commands, under *Gitardo will write in `<root>`:*.
- [ ] An operation asked for while another runs does nothing: `Controller.run` returns without asking or running.

### `preview.fallback` — Offer the other way when git refuses

When git refuses an operation in a way the operation foresaw, typically because it would need to be forced, Gitardo
offers the forced variant instead of only reporting the failure: deleting a branch that is not merged, removing a
worktree with changes.

- **Kind**: Operation
- **Function**: `Controller.runWithFallback(op, asked)` in `src/extension/controller.ts`; the `fallback` field of
  `GitOp` (`{ when: RegExp; op: GitOp }`), set by the builders of `src/git/ops.ts`.
- **Entry points**: every operation whose builder sets a `fallback`.
- **Offered when**: the first operation failed and what git printed matches the fallback's `when`.
- **Asks**: the fallback is asked as any operation is (`preview.ask`), with its own title, commands and danger.
- **Plan**: the commands of the fallback.
- **Danger**: the fallback's own, usually Destructive, because it is the forced variant.
- **Undo**: what the fallback's own card says.
- **Refuses**: when git's message does not match `when`, the failure is reported as any failure (`log.report-failure`).
- **Uses**: `preview.ask`, `log.report-failure`.
- **Implemented in**: `Controller.runWithFallback`, called by `Controller.action`.
- **Tested by**: `test/e2e/suite.ts`, the steps that delete a branch that is not merged; *none* for the matching itself.

Acceptance:

- [ ] The first attempt runs without its failure being shown (`silent`), and its error is written to the log.
- [ ] When git's output matches `when`, the fallback is asked with its own dialog, and **Cancel** leaves the repository
      as the first attempt left it.
- [ ] When it does not match, the user sees the failure of the first attempt, once.

### `log.write` — Write every command and its output to the log

Every command Gitardo runs to change something, and everything it does itself, is written to the **Gitardo** output
channel as it happens, so that the user can see and repeat it.

- **Kind**: Operation
- **Function**: the body of `Controller.run` in `src/extension/controller.ts`, writing to the `LogOutputChannel` made
  in `src/extension/extension.ts`.
- **Entry points**: every operation.
- **Plan**: for each command, a line `> git <arguments>` as `formatCommand` writes it, then *standard input:* and the
  input when the command reads one, then what git printed; for each file the operation writes, `> write <path>`; the
  lines the operation's `prepare`, `recover` and `finish` write themselves (`symlink`, `copy`, `run`, `unlink`,
  `delete`, `git.close`); an error as an error line.
- **Danger**: None: it changes nothing.
- **Undo**: Nothing to undo.
- **After**: reads of the repository (`git log`, `git status` and the like) are written at the debug level, which the
  channel shows only when its level is set to Debug.
- **Implemented in**: `Controller.run`; `Repo`'s constructor gets the debug level of the log for the reads; `Activity` writes there too.
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
- **Function**: `Controller.reportFailure(op, error)` in `src/extension/controller.ts`.
- **Entry points**: the end of every operation that failed and has no matching fallback.
- **After**:
  - With files in conflict: a warning *`<title>` stopped: 1 file has conflicts. Resolve them, then continue or abort
    the `<operation>`.* (*N files have conflicts*; *stage the files* when no operation is under way, as after a stash
    that did not apply cleanly), followed by *Gitardo can resolve the conflict automatically.*, *all N conflicts* or
    *N of the M conflicts* when the rules can, and the buttons **Resolve Automatically** (when the rules can),
    **Resolve with AI** (when `gitardo.conflicts.ai` is on and some conflict is left to it) and **Show Log**.
  - Otherwise: an error *`<title>` failed. `<line>`* where the line is the first that git starts with `error:` or
    `fatal:`, or else its first line, with **Show Log**.
- **Settings**: `gitardo.conflicts.autoResolve`, `gitardo.conflicts.ai`.
- **Uses**: `conflict.resolve-rules`, `conflict.resolve-ai`, `log.show`.
- **Implemented in**: `Controller.reportFailure`.
- **Tested by**: `test/e2e/suite.ts`, the steps of a merge that stops on conflicts with the automatic resolution on.

Acceptance:

- [ ] A push refused by the remote shows *Push failed. error: failed to push some refs to …* with **Show Log**.
- [ ] A merge that stops on conflicts in two files shows *Merge … stopped: 2 files have conflicts. Resolve them, then
      continue or abort the merge.*
- [ ] **Resolve Automatically** is offered only when the rules can resolve at least one conflict, and with
      `gitardo.conflicts.autoResolve` off it is never run (`conflict.resolve-rules`).

### `undo.last` — Undo the last operation

**Undo** takes back the last commit, amend, checkout, merge, pull, rebase or reset, including those made from a
terminal, by reading the reflog. Right after an operation the reflog cannot take back, it takes it back from what
Gitardo remembered: a branch deleted, a stash dropped, conflicts resolved, a squash merge, a series of commits made by
one cherry-pick or one patch, the split of the last commit. Undoing twice redoes.

- **Kind**: Operation
- **Function**: `undoOp(info, branch)` in `src/git/ops.ts`, from `Repo.undoInfo(branch, working)` in
  `src/git/repo.ts`; or the operation remembered in `Controller.undoable`: `restoreBranchOp`, `restoreStashOp`,
  `restoreConflictsOp`, `undoSquashOp`, or `undoOp` of a series.
- **Entry points**: **Undo** in the toolbar of the graph; **Gitardo: Undo Last Operation** in the Command Palette; the
  **…** of the headers of the Commits view and the Gitardo Graph; **Undo** on the messages after resolving conflicts, a
  squash merge and a split.
- **Asks**: always, with the operation's title and note (`confirm`), or a warning for restoring conflicts (`danger`).
- **Reads**: the top entry of the reflog of `HEAD`, to tell whether what Gitardo remembered is still the last thing that
  happened; the last two entries of the reflog of `HEAD`, and of the current branch's own reflog; whether the files
  already look like the commit to go back to (`git diff --quiet <previous>`).
- **Plan**, by what the reflog's last entry is:
  - a checkout (`checkout: moving from <from> to …`): `> git checkout <from>`, or `> git checkout --detach <hash>` when
    it came from a detached `HEAD`, followed by the update of submodules when that is on;
  - a commit or an amend: `> git reset --soft <previous>`: the changes of the commit stay staged;
  - anything else, when there are changes and the files already look like `<previous>`: `> git reset --mixed
    <previous>`;
  - anything else: `> git reset --keep <previous>`, which git refuses if it would overwrite uncommitted changes;
  - remembered: a deleted branch, `> git branch <name> <hash>`; a dropped stash, `> git stash store -m <message>
    <hash>`; resolved conflicts, `> git checkout --merge -- <files>`; a squash merge, `> git reset --merge`; a series,
    `> git reset --keep <before>`; the split of the last commit, `> git reset --mixed <the commit>`.
- **Danger**: Confirm, because it moves a branch or `HEAD`; Destructive for restoring conflicts, because what was
  changed in those files since is lost.
- **Undo**: Undoing again takes back the undo, by the same rules: the reset or checkout it made is the reflog's last
  entry. A restored branch or stash is not removed again by a second undo.
- **Refuses**: while an operation is under way, *Finish or abort the `<operation>` first.*; during a bisect, *End the
  bisect first.*; with no reflog, or a branch with fewer than two entries, *There is nothing to undo.* Each as an error
  message prefixed with *Gitardo:*.
- **After**: what Gitardo remembered is forgotten once undone, or as soon as another operation succeeds, another
  repository is shown, or the window is reloaded.
- **Uses**: `preview.ask`, `log.write`, `submodule.update-on-checkout`.
- **Implemented in**: action `undo` (`src/shared/protocol.ts`); its case in `resolveAction` (`src/extension/actions.ts`);
  `Repo.undoInfo` (`src/git/repo.ts`); `undoOp`, `undoSquashOp`, `restoreBranchOp`, `restoreStashOp`,
  `restoreConflictsOp` (`src/git/ops.ts`); `Controller.action`, which runs the remembered operation when the reflog's
  top entry is the one recorded with it, and records it after the operations listed above.
- **Tested by**: `test/integration.test.ts`, *undoes commits, checkouts, resets and rebases*, *merges as git does by
  default, always with a merge commit, as a fast-forward only, or as a squash, and undoes each*, *undoes a squash merge
  that stopped on conflicts, which leaves no merge to abort*, *splits the last commit by taking its changes out of it,
  which undo puts back*, *undoes a whole series of cherry-picks by aborting it*; `test/e2e/suite.ts`, *undoes the last
  commit, and restores a deleted branch*.
- **Differs**: right after an operation that leaves no entry in the reflog of `HEAD` or of the branch, and that Gitardo
  remembers nothing of, **Undo** takes back the operation before it; its question names that operation, but nothing
  says it is not the last one: after
  a stash, a push, a fetch, a tag, a remote, a branch created without checking it out or renamed, a branch cleanup, a
  fast-forward of another branch, any operation on a worktree or a submodule. After a stash it can reset the last
  commit with `--soft`. The cause is `Repo.undoInfo` (`src/git/repo.ts:1053`), which reads only the reflog, and
  `Controller.action` (`src/extension/controller.ts:1689`), which forgets what it remembered after every operation
  and records nothing for these. Also, any operation through `Controller.action`, staging a file included, forgets a
  branch deleted or a stash dropped just before, while one run by `Controller.runIn` (the Flow Diff) or outside
  `Controller.action` (sync, branch cleanup, remotes) does not.

Acceptance:

- [ ] After a commit, **Undo** runs `git reset --soft <the commit before it>`, and the commit's changes are staged.
- [ ] After `git checkout feature` in a terminal, **Undo** runs `git checkout main` when `main` was checked out before.
- [ ] After a rebase of `feature`, **Undo** moves `feature` back to where its own reflog had it before the rebase, not to
      an intermediate `HEAD` of the rebase.
- [ ] After deleting the branch `old` in Gitardo, **Undo** runs `git branch old <its hash>`; after any other operation in
      between, it does not.
- [ ] After dropping a stash in Gitardo, **Undo** runs `git stash store -m <its message> <its hash>`.
- [ ] After a squash merge, **Undo** runs `git reset --merge`, also when the squash stopped on conflicts.
- [ ] After a cherry-pick of three commits in one operation, **Undo** takes back all three, also when it stopped and
      was continued in Gitardo; continued from a terminal, it takes back the last commit only.
- [ ] During a merge that stopped, **Undo** says *Finish or abort the merge first.* and runs nothing.
- [ ] **Undo** always asks first, whatever `gitardo.commandPreview` says.
- [ ] **Undo** twice in a row after a commit leaves the commit as it was.
- [ ] Right after a stash, a push or a tag, **Undo** says there is nothing it can take back, rather than taking back the
      operation before (differs).

### `activity.track` — Show what is running in the status bar

While Gitardo has been reading a repository, running an operation, comparing two sides or asking GitHub for more than
half a second, **Gitardo** with a spinning icon shows in the status bar, with what each of those is in its tooltip.

- **Kind**: View
- **Function**: `Activity.start(title, detail)` and `Activity.track(title, work)` in `src/extension/activity.ts`.
- **Entry points**: the status bar item **Gitardo Activity**, beside the item that opens the graph.
- **Offered when**: something has been running for 500 ms; it goes when the last thing ends.
- **After**: the tooltip lists each thing, oldest first, in bold with how long it has run, then its detail: what asked
  for a read (such as a file that changed) and how many times in a row the repository was read because something
  changed during each read. It ends with *Click to see what Gitardo is doing in a list.*
- **Implemented in**: `src/extension/activity.ts`; `Controller.load` (reads), `Controller.run` (operations),
  `Controller.refreshPulls` (GitHub), the comparisons.
- **Tested by**: *none* found by name; the status bar item is reached by `test/e2e/suite.ts` only through the commands.

Acceptance:

- [ ] A read that ends within 500 ms never shows the item.
- [ ] An operation that runs for two seconds shows **Gitardo** with a spinning icon after half a second, with its title
      in the tooltip, and the item goes when it ends.

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

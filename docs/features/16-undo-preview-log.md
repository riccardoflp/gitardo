# Undo, command preview and the log

What every operation goes through: the preview of its commands, the log of what ran, and Undo. These are the engine's
features; the operations of the other files only say, on their cards, what their plan is, how dangerous it is and what
Undo needs.

### `preview.confirm` — Show the commands before they run

Before a plan runs, the engine decides from `commandPreview` and the plan's danger whether to show it, and waits for the
user's answer.

- **Kind**: View
- **Function**: `views::feature::confirm_view(state: &ConfirmState, ui) -> Vec<Intent>`
- **Entry points**: every operation, through the engine, which decides with `engine::needs_confirmation`.
- **Asks**: one dialog with the plan's title, its steps exactly as they will be logged, the data passed on standard
  input or through a file beside the step that takes it (the patch of `git apply --cached`, the todo list of an
  interactive rebase, a patch from the clipboard), the reason when the plan is destructive, and **Run** and **Cancel**.
  **Run** is the default only when the plan is not destructive.
- **Plan**: none of its own: on **Run**, the plan it was given runs unchanged.
- **Danger**: None
- **Undo**: Nothing to undo: **Cancel** runs nothing.
- **Settings**: `commandPreview` (`off` or `all`).

The table of when it asks is in [04-feature-contract.md](../desktop/04-feature-contract.md#danger-and-confirmation). With
`commandPreview` set to `off`, a plan is shown first when it is `Destructive` or `Confirm`. These are destructive, and
every card of them says so: force push, hard reset, discarding changes, applying, popping or dropping a stash,
deleting a branch or a tag, deleting a remote branch, removing a remote, removing a worktree, aborting an operation,
undo, interactive rebase, squash, split, fixup, branch cleanup, a merge, cherry-pick or patch predicted to conflict,
and taking one side of a file in conflict when the file has lines typed in it that neither side nor the base has, which
would be lost. Resolving conflicts automatically is asked by its own question, which is its confirmation whatever
`commandPreview` says; skipping a step of an operation asks first whenever the working tree has changes.

Acceptance:

- [ ] With `commandPreview` `off`, creating a branch runs without a dialog, and a hard reset shows its command and a
      warning first.
- [ ] With `commandPreview` `all`, staging a file shows `git add -- :(literal)<path>` before it runs.
- [ ] **Cancel** runs nothing and logs nothing.
- [ ] The steps shown are byte for byte the lines the log then has, in the same order.
- [ ] Staging a change in the Diff page shows the patch given on standard input beside the command.

### `log.record` — Log every command that changes something

Every step of every plan is written to the command log before it starts, and its result after.

- **Kind**: Operation (of the engine; it has no entry point of its own)
- **Function**: `engine::log::record(step, result)`
- **Plan**: the lines are, by step: the git command as it runs (arguments quoted where they hold spaces), `write <path>`,
  `symlink <folder>`, `copy <path>`, `unlink <path>`, `delete <path>`, `run <command>`. A failed step is followed by
  what git or the system said.
- **Danger**: None
- **Undo**: Not applicable.
- **Settings**: `log.level` (`info` or `debug`): at `debug`, the reads are logged too, with how long each took.

Acceptance:

- [ ] Every command that changes the repository or a remote is in the log, exactly as it ran.
- [ ] Reads (`git log`, `git status` and the like) are not in the log at `info`, and are at `debug`.
- [ ] A step that fails has its error under it, and the steps after it are not run nor logged.
- [ ] A step that fails and that the plan gets past, as a cherry-pick that changes nothing is skipped, has what git said
  under it as a warning, not as an error.
- [ ] A token or a key never appears in the log, also when a request to GitHub or to the AI provider fails.

### `log.show` — Show the command log

The **Command log**, a tab of the bottom panel, lists what the log recorded, newest last, and follows it as it
grows.

- **Kind**: View
- **Function**: `views::log::log_view(state, ui) -> Vec<Intent>`
- **Entry points**: **Command log** in the bottom panel (`panel.bottom`), the tab beside **Activity**; **Show Command
  Log** in Quick search.
- **Settings**: `log.level`.

Acceptance:

- [ ] Opening it shows the commands of the session, each with the time it started.
- [ ] Text can be selected and copied.
- [ ] It keeps the last 10,000 lines and says how many were dropped.
- [ ] It is a tab of the bottom panel, beside **Activity**.

### `undo.last` — Undo the last operation

**Undo** takes back the newest operation of Gitardo, as its journal keeps it: a commit, amend, checkout, merge, pull,
rebase or reset by the reflog, and what the reflog does not see from the journal itself. With nothing in the journal,
after a commit made in a terminal for one, it takes back the last move of the reflog. When the newest operation is
one it cannot take back, a push, a fetch, a tag, a stash or a worktree, it says so rather than take back the one
before it. It always shows what it is about to do first. With nothing in the journal, undoing twice redoes.

- **Kind**: Operation
- **Function**: `features::undo::last(context, answers) -> Outcome`
- **Entry points**: **Undo** in the **…** menu of the header; **Undo Last Operation** in Quick search; the **Undo** of
  the messages that follow an operation.
- **Extension**: **Undo** in the toolbar of the graph, whose tooltip names the newest operation of the journal (*Undo
  "Commit"*, or *"Push main to origin" cannot be undone*, from `journal/last`, which checks the journal against the
  state of the last `repo.read-state` instead of reading the repository again, and reads it only when no state was
  read since the last operation (M15)); **Gitardo: Undo Last Operation** in the
  Command Palette and in the **…** of the headers of the Commits view and the Gitardo Graph; the **Undo** of the
  message after resolving conflicts automatically (`Controller.say`). Each sends `feature/run` of `undo.last` to
  Gitardo's engine (`Controller.runOnEngine` in `src/extension/controller.ts`), its plan always shown in a modal dialog
  first (`VscodeAsker` in `src/engine/vscode.ts`). Every operation is the engine's, so its journal has them all. When
  the engine's binary is missing, it says so (D49).
- **Offered when**: the journal has an entry, or the reflog of `HEAD` has a previous position.
- **Reads**: the newest entry of the journal of Gitardo's own operations for this repository; the last two entries of
  the reflog of `HEAD`
  and, when a branch is checked out, of `refs/heads/<branch>` (a rebase moves `HEAD` many times but the branch once, so
  the branch's own reflog says where it was before); `git diff --quiet <previous>` when there are uncommitted changes;
  the state of the repository, to make sure nothing happened since the operation in the journal.
- **Plan**: the steps the journal entry or the reflog entry calls for:
  - the last entry of the reflog of `HEAD` is `checkout: moving from <from> to …`: `git checkout <from>`, or
    `git checkout --detach <hash>` when `<from>` is a commit, then the submodule update when
    `submodules.updateOnCheckout` asks for it;
  - otherwise the branch (or `HEAD`) goes back to its previous position with `git reset --<mode> <previous>`, where the
    mode is `--soft` when the entry undone is a commit or an amend (`commit: …`, `commit (amend): …`), so that its
    changes stay staged; `--mixed` when there are uncommitted changes and the files already are as `<previous>` has
    them (after a soft or mixed reset), so that only `HEAD` and the index go back; and `--keep` otherwise, which git
    refuses when it would overwrite uncommitted changes;
  - a split of the last commit: `git reset --mixed <the commit that was split>`, since the commit may have added files
    that are untracked now;
  - a branch deleted in Gitardo: `git branch <name> <commit it was at>`;
  - a branch created in Gitardo and not checked out: `git branch -D <name>`;
  - a stash dropped in Gitardo: `git stash store -m <message> <commit of the stash>`;
  - conflicts resolved by Gitardo: `git checkout --merge -- <files>`;
  - a squash merge, until something is committed: `git reset --merge`;
  - a cherry-pick of several commits or a patch applied as commits, also continued in Gitardo after a conflict:
    `git reset --keep <HEAD before it>`, all of it at once;
  - a squash, or a split of an older commit: `git reset --keep <HEAD before it>`.

  A branch restored this way does not get its upstream back; the dialog says so.
- **Danger**: Destructive, because it moves a branch or rewrites files; the dialog says what it takes back in words
  (*Undo the commit "Fix the parser"*) and the commands. A branch deleted or a stash dropped in Gitardo is only
  confirmed, since making it again loses nothing.
- **Undo**: it takes its entry out of the journal, so that Undo again takes back the operation before; with nothing
  in the journal, Undo again redoes what was undone, by the reflog.
- **After**: selects the commit `HEAD` is at, and says what was undone.
- **Refuses**: when there is nothing to undo, *Nothing to undo*; when the newest operation cannot be taken back,
  *"`<its title>`" cannot be undone.* and the reason its plan gave; while a merge, a rebase, a cherry-pick, a revert or a
  patch is under way, *Finish or abort the `<operation>` first.*, but for a squash merge or a resolution of conflicts,
  which are taken back from where they stopped; when the journal no longer matches the repository (something happened
  since), it falls back to the reflog; when uncommitted changes are in the way of `--keep`, *Undo would overwrite
  changes that are not committed: commit or stash them first*.

Acceptance:

- [ ] After a commit made in a terminal, Undo resets to its parent with `--soft`, and the changes are staged.
- [ ] After a rebase of the checked out branch, Undo resets the branch to where its own reflog had it before the
      rebase, with `--keep`.
- [ ] After `git checkout feature` from `main`, Undo runs `git checkout main`.
- [ ] Undo after Undo puts the commit back.
- [ ] After deleting a branch in Gitardo, Undo creates it again at the same commit, after a confirmation that is not a
      warning.
- [ ] After dropping a stash in Gitardo, Undo puts it back in the list with its message.
- [ ] After a cherry-pick of three commits that stopped on a conflict and was continued in Gitardo, one Undo takes back
      all three.
- [ ] After a squash merge with nothing committed since, Undo runs `git reset --merge` and the index is clean.
- [ ] After conflicts resolved by Gitardo, Undo marks them again in every file, staged ones included.
- [ ] With an operation made in a terminal after one of Gitardo's, Undo takes back the terminal's, by the reflog.
- [ ] With `submodules.updateOnCheckout` on and submodules in what was checked out before, Undo of a checkout runs
      `git checkout <from>` and then `git submodule update --init --recursive`.
- [ ] A branch deleted, then a branch created only: Undo deletes the new branch, and Undo again creates the deleted one.
- [ ] A stash dropped, then a commit: Undo takes back the commit, then the drop.
- [ ] Right after a push, Undo says it cannot take back the push, and takes back nothing.
- [ ] During a merge stopped on conflicts, Undo refuses with *Finish or abort the merge first.*
- [ ] Extension: a branch deleted, then a branch created: **Undo** removes the new branch, and **Undo** again restores
      the deleted one, from the journal.
- [ ] Extension: `journal/last` asked right after `repo.read-state` runs no git; asked after an operation with no read
      since, it reads the state once.
- [ ] Extension: a stash dropped, then a commit made: **Undo** takes back the commit, then the drop.
- [ ] Extension: a squash merge, a resolution of conflicts and a split of the last commit are each taken back whole.
- [ ] Extension: with `gitardo.engine.path` naming a missing file, **Undo** shows **Gitardo's engine is not available
      on this system.** with **Show the log**, and runs nothing.
- [ ] Extension: once the engine restarted, its journal is empty and **Undo** goes by the reflog, as after a reload.
- [ ] Extension: **Undo** always asks first, whatever `gitardo.commandPreview` says.

### `undo.journal` — Remember what Undo needs

The engine keeps, for each repository, a journal of Gitardo's operations, newest last, each with what Undo needs of
it: a move of `HEAD` the reflog takes back, the commit a branch was at, a branch created, what was checked out before a
branch with no history (`branch.create-orphan`, whose move git writes in no reflog), the commit of a stash, the
files whose conflicts were resolved, the `HEAD` before a series, or why it cannot be taken back. It holds while `HEAD`
is what the newest entry left, and is emptied once `HEAD` moves by something Gitardo did not do. An operation that
cannot be taken back and changed only files and the index (staging, a save, a discard) adds nothing, and Undo goes
past it. It is kept in memory: a new engine starts with an empty journal, and Undo then goes by the reflog.

- **Kind**: Derivation
- **Function**: `engine::undo::journal_after(journal, plan, state_before, state_after) -> Journal`
- **Danger**: None
- **Undo**: Not applicable.

Acceptance:

- [ ] The entries are dropped when `HEAD` has moved since by something Gitardo did not do.
- [ ] A move of `HEAD` by Gitardo is an entry on top of the ones before, and once Undo took it back, the one before it
      holds again.
- [ ] A plan that cannot be taken back is an entry with its reason once a reference changed, and nothing when only
      files and the index did.
- [ ] The journal keeps at most 50 entries per repository. (A number of Gitardo Desktop's own: the extension keeps
      what it needs in memory only. Whether entries survive a restart is open question Q4.)

### `undo.moved-branches` — Take back the branches a rewrite moved

The journal entry of a rewrite that moved the other branches of a stack keeps where each was, so that one Undo puts
them back with the branch rewritten.

- **Kind**: Derivation
- **Function**: `engine::undo::moved_branches(entry: &JournalEntry, state: &RepoState) -> MovedBranchesUndo`
- **Uses**: `undo.journal`, `undo.last`, `rebase.update-refs`, `rebase.read-stacked-branches`.
- **Extension**: the engine's, in the plan of `undo.last` that the extension runs on the engine.

Rules:

- A plan that moves branches with `--update-refs` or `update-ref` lines (`rebase.update-refs`) records in its journal
  entry each branch it is to move with the commit it was on, as `rebase.read-stacked-branches` read it, and, once it
  ran, the commit it is on.
- Undo of that entry reads where each of those branches is now. Its plan is the step of `undo.last` for the branch
  rewritten, `git reset --keep <before>`, then one
  `git update-ref -m "undo: back to before the rebase" --stdin`, with one line
  `update refs/heads/<branch> <before> <after>` for each branch on standard input, shown beside the step as the
  preview shows what a step reads. Git makes them all or none, and refuses a branch that is no longer at `<after>`.
- A branch that moved since the rewrite, by a commit or anything else, is left out of the lines, and the dialog of
  Undo says so: "`p1` moved since the rebase and stays where it is." The others go back.
- A branch the rewrite left behind (`rebase.update-refs`) is not in the entry.
- The dialog says it in words: *Undo the rebase of `p3`, and put `p1` and `p2` back where they were.*
- With no entry in the journal (the engine started again since), Undo goes by the reflog as before, and takes back the
  branch rewritten alone (Q1133).

Acceptance:

- [ ] After a rebase of `p3` that moved `p1` and `p2`, one Undo puts `p3`, `p1` and `p2` back on their commits of
      before; the log has `git reset --keep <before>` and `git update-ref … --stdin` with two lines.
- [ ] With a commit made on `p1` since the rebase, Undo puts `p3` and `p2` back, leaves `p1` where it is, and its
      dialog says so.
- [ ] After a rebase that moved no other branch, the plan of Undo has no `git update-ref`.
- [ ] Extension: one **Undo** after a rebase that moved `p1` puts both branches back.

### `activity.track` — Say what is running

Every read, plan, comparison and request that may take longer than a frame is registered while it runs, with what it is
in words, when it started and what asked for it, so that the activity indicator of the status bar (`activity.status-bar`)
and the **Activity** tab (`activity.what-is-running`) can say it. The words are those of `activity.describe-task`.

- **Kind**: Operation (of the engine; it has no entry point of its own)
- **Function**: `engine::activity::track(description, cause) -> ActivityGuard`, which ends the entry when dropped.
- **Danger**: None
- **Undo**: Not applicable.

Acceptance:

- [ ] A plan of a push shows **Pushing main to origin** in the activity indicator of the status bar while `git push` runs
      for more than half a second, and the time it has run.

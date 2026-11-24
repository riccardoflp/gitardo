# Where the extension and the engine differ, operation by operation

This lists, for every pair of [the map](03-map.md), where the extension's operation and the engine's
feature differ: the plan (commands, flags, order), the questions, the danger, the refusals, Undo, and what is offered
after or when git refuses. Reads and views are left out, but for the reads stage 5 compared, under
[Reads (stage 5)](#reads-stage-5), and those of the graph stage 7 compared, under
[Reads of the graph (stage 7)](#reads-of-the-graph-stage-7). Each difference gets a verdict:

- **Bug (extension)**: the extension is wrong and the engine is right; the move fixes it, and the entry gives the
  CHANGELOG line.
- **Bug (engine)**: the engine is wrong (often because its code does less than its own card), or both are wrong and
  the engine's copy is the one that stays; fix it in the engine.
- **Choice**: the maintainer decides; the entry gives a recommendation and says whether an engine option is needed.
- **CHANGELOG**: the engine's behaviour is fine and wins; the entry gives the line users of the extension will read.

The rule that decides is D45: the engine wins unless a difference is decided otherwise here; each difference the
extension's users will see becomes a line of the extension's CHANGELOG when its area moves; a bug is fixed on whichever
side it is. A **Choice** stays open until it is written as a decision; the open ones are among the questions from Q393
on in [08-decisions.md](../08-decisions.md#open-questions). The differences of an area are settled in the stage of the
[roadmap](06-roadmap.md) that moves it.

Sources, from the root of the repository:

- Extension: paths starting with `extension/src/` (some entries shorten them to `git/…`, `extension/…`, `webview/…`,
  `flow/…`, which are all under `extension/src/`). The main files are `extension/src/git/ops.ts` (the `GitOp` of each
  operation), `extension/src/extension/actions.ts` (`resolveAction`: questions, refusals) and
  `extension/src/extension/controller.ts` (`Controller.run` at 2420, `Controller.action` at 1632, `sync` at 1587).
- Engine: paths starting with `rust/crates/`; some entries shorten them to `features/…`, `model/…` or `engine/…`, which
  are `rust/crates/<crate>/src/…`. The journal is `rust/crates/engine/src/undo.rs`; confirmation and follow-ups are
  `rust/crates/engine/src/drive.rs`.
- Cards: `docs/features/*.md` (desktop) and `docs/features/extension/*.md` (extension).

Entries are numbered by area (A changes and stashes, B branches and sync, C merge and rewriting, D conflicts and undo,
W worktrees, S submodules). Where two areas found the same difference, it is written once and the other entry points
to it, marked **Merged into …**, and counted only once.

Four things cut across areas and are best decided once:

1. **Undo after an operation it cannot take back** (D7): on both sides, Undo right after a stash, push, fetch, tag,
   branch created without checkout, worktree or submodule operation takes back the operation before it, without
   saying so. A13, W11 and the last line of the identical list of area B are this one.
2. **The journal lasts longer than the extension's one remembered operation** (D9): branches deleted, stashes
   dropped, squash merges, conflicts resolved and series are kept for Undo while HEAD stays, up to 50, instead of until
   the next operation. A11, B4 and C3 are instances of it. `undo.last` reads the journal before the reflog
   (`rust/crates/features/src/undo/last.rs:35-38`), which settles the question C3 left open.
3. **The submodules after a checkout that is not called a checkout** (S2): `branch.create` with check out and
   `stash.branch` do not update the submodules in the engine, though their cards say so. A10 and B1 are this one.
   Fixed in stage 2.
4. **The fallback after git refuses** (B3, W7): the engine offers **Delete anyway** and **Remove anyway** on the
   failure (`FollowUp::OfferRun`), where the extension asked the forced variant at once; `submodule.deinitialize` in
   the engine still asks at once (`FollowUp::OfferWhenFailed`). Either is fine, but the engine should pick one.

## Changes and commit

### A1. Unstage all while a merge or cherry-pick is stopped on conflicts

- **Feature**: `changes.unstage-all`; extension `unstageAll` (`unstageAllOp`).
- **Extension**: the menu of the uncommitted changes row offers **Unstage all changes** whenever something is staged,
  conflicts or not (`extension/src/webview/menus.ts:160`), and the action is always `git reset` (or
  `git rm --cached -r -- .` without HEAD) (`extension/src/git/ops.ts:611-613`). Only the header box of the Changes view
  avoids it while files are in conflict, by sending `unstage` with the staged files instead
  (`extension/src/webview/ChangesApp.tsx:207-212`). The extension card records this as a **Differs**: `git reset` ends
  the merge and forgets the conflicts (`docs/features/extension/05-changes-and-commit.md:211-215`).
- **Engine**: the feature itself names the staged files with `git restore --staged -- <path>…` (both names of a rename)
  whenever a file is in conflict, from every entry point (`rust/crates/features/src/changes/unstage_all.rs:20-22`), and
  refuses "There is nothing to unstage." when nothing is staged (line 17).
- **Verdict**: **Bug (extension)**, fixed by the move. CHANGELOG: "Unstage all changes from the menu of the uncommitted
  changes no longer ends a merge or cherry-pick stopped on conflicts: it unstages the staged files one by one."
- **Fixed in stage 4** (area 1), with **Unstage all** run by the engine; in the extension's CHANGELOG.

### A2. Unstage all during an operation with no conflict left

- **Feature**: `changes.unstage-all`; extension `unstageAll`.
- **Extension**: runs `git reset` whenever no file is in conflict (`extension/src/webview/ChangesApp.tsx:207-212`,
  `extension/src/git/ops.ts:611-613`), also while a merge or cherry-pick waits to be committed with every conflict
  resolved and staged; `git reset` then drops `MERGE_HEAD`/`CHERRY_PICK_HEAD` (the extension card's **Differs**,
  `docs/features/extension/05-changes-and-commit.md:211-215`).
- **Engine**: the same: the only guard is `tree.conflicted.is_empty()`, not `repository.operation`
  (`rust/crates/features/src/changes/unstage_all.rs:20-24`); the desktop card has the same plan
  (`docs/features/05-changes-and-commit.md:270-272`).
- **Verdict**: **Bug (engine)**, shared with the extension. Recommendation: name the staged files (`git restore --staged
  -- …`) also while `repository.operation` is set, so no unstage ever ends an operation. No option needed.
- **Fixed in stage 4** (area 1): `unstage_all` names the staged files while an operation is under way too
  (`rust/crates/features/src/changes/unstage_all.rs`).

### A3. Discard before the first commit: the warning

- **Feature**: `changes.discard`, `changes.discard-all`; extension `discardChanges` (`discardChangesOp`).
- **Extension**: before the first commit the warning says the staged files are deleted: "One staged file will be
  deleted from disk." (`extension/src/git/ops.ts:668`).
- **Engine**: the plan is the same (`git rm -r --force --quiet -- <path>`,
  `rust/crates/features/src/changes/discard.rs:34`, `discard_all.rs:97-99`, `127-130`), but the warning is always
  `back_to_last_commit_warning`, "One file goes back to the last commit, staged changes included…", also when there is
  no last commit (`rust/crates/features/src/changes/discard.rs:36`, `discard_all.rs:102`, `133`;
  text at `changes/files.rs:79-87`).
- **Verdict**: **Bug (engine)**: the warning speaks of a last commit that does not exist and does not say the files
  are deleted. Take the extension's sentence.
- **Settled in M48**: before the first commit the warning is "One staged file will be deleted from disk.", or "N staged
  files will be deleted from disk.", in both apps (`staged_deleted_warning`, `rust/crates/features/src/changes/files.rs`),
  with tests.

### A4. Discard on a Diff page with nothing unstaged

- **Feature**: `diff.discard-file`; extension Flow Diff `fileAction('discard')` (`discardOp`).
- **Extension**: `discardOp([file])` runs `git restore -- <file>` for any tracked file, even one whose changes are all
  staged, where it does nothing after the warning (`extension/src/flow/flowDiff.ts:131`,
  `extension/src/git/ops.ts:616-634`).
- **Engine**: refuses "There is nothing to discard." unless the file is untracked or has unstaged changes
  (`rust/crates/features/src/diff/discard_file.rs:27-33`).
- **Verdict**: **CHANGELOG**: "Discard changes on a diff whose changes are all staged now says there is nothing to
  discard instead of asking and doing nothing."

### A5. Revert one change in a diff

- **Feature**: `diff.revert-change`; extension Flow Diff `revertChunk`.
- **Extension**: applies a `WorkspaceEdit` to the open document and saves it, so the revert is in the editor's undo
  history (Ctrl+Z) and goes through VS Code's own save (`extension/src/flow/chunks.ts:54-90`).
- **Engine**: a `Step::Write` of the whole file as the left side has it, `Danger::None`, `Undo::Impossible("The file
  is written as the left side has it; Undo does not take it back.")` (`rust/crates/features/src/diff/revert_change.rs:12-14`,
  `48-53`).
- **Verdict**: **Choice**. Writing the file under VS Code would bypass an editor with unsaved edits of the same file
  and lose Ctrl+Z, which extension users rely on. Recommendation: the extension keeps applying the reverted lines
  through VS Code's editor (host-side), using the engine only for the check that the change is where it was shown; no
  engine option needed, but the host must be able to take the new text instead of a `Write` step.

### A6. Fix up during a bisect

- **Feature**: `fixup.commit`; extension `fixup` (`fixupOp`).
- **Extension**: refuses only while an operation is in progress, "Finish or abort the … first."
  (`extension/src/extension/actions.ts:587-592`).
- **Engine**: `refuse_under_way` also refuses during a bisect with `bisect::END_FIRST`
  (`rust/crates/features/src/fixup/commit.rs:19-21`, `rust/crates/features/src/rebase.rs:35-51`). The commands are the
  same: `git add --all` when nothing is staged, `git commit --fixup=<hash>`,
  `git -c sequence.editor=true rebase -i --autosquash --autostash <hash>^|--root` (`fixup/commit.rs:36-56`;
  `extension/src/git/ops.ts:1034-1046`), and both confirm.
- **Verdict**: **CHANGELOG**: "Adding changes to an earlier commit is refused during a bisect: end the bisect first."
- **In the CHANGELOG in stage 4** (area 3), where the fixup moved to the engine.

### A7. Stage all / unstage all / commit amend refuse up front

- **Feature**: `changes.stage-all`, `commit.amend`; extension `stageAll`, `commit` with `amend`.
- **Extension**: `stageAllOp` always runs `git add --all` (`extension/src/git/ops.ts:600-602`); an amend before the
  first commit goes to git, which fails (`extension/src/git/ops.ts:1021-1028`). The conflict-aware naming of files for
  **Stage all** is done by the view (`extension/src/webview/ChangesApp.tsx:201-206`).
- **Engine**: `changes.stage-all` refuses "There is nothing to stage." and names the other files itself while files are
  in conflict (`rust/crates/features/src/changes/stage_all.rs:22-30`); `commit.amend` refuses "There is no commit to
  amend yet." (`rust/crates/features/src/commit/amend.rs:17-19`).
- **Verdict**: **CHANGELOG** (barely visible): "Stage all and Amend say why when there is nothing to stage or no commit
  to amend, instead of running git." The plans are otherwise the same.
- **In the CHANGELOG in stage 4** (area 1).

## Stashes

### A8. Stash a file whose rename is staged

- **Feature**: `stash.push-file`; extension `stashPush` with `paths` (`stashPushOp`).
- **Extension**: `git stash push --include-untracked [-m …] -- <new> <old>`, both names from `pathsOf`
  (`extension/src/webview/ChangesApp.tsx:17`, `323`; `extension/src/git/ops.ts:557-567`), with nothing run first.
- **Engine**: for a staged rename it runs `git reset --quiet -- <old>` first, "as git does not find the old name
  otherwise", then the same push naming both (`rust/crates/features/src/stash/push_file.rs:37-57`, `62-64`; card
  `docs/features/08-stashes.md:141-144`).
- **Verdict**: **Bug (extension)** (git refuses a pathspec that is not in the index), fixed by the move. CHANGELOG:
  "Stashing a file whose rename is staged now stashes both its names instead of failing."
- **Fixed in stage 4** (area 1), with the stashes run by the engine; in the extension's CHANGELOG.

### A9. Stash with nothing to stash, or before the first commit

- **Feature**: `stash.push`, `stash.push-staged`, `stash.push-file`; extension `stashPush`.
- **Extension**: asks the message and runs git whatever the state (`extension/src/extension/actions.ts:517-529`); the
  menus hide the entries before the first commit (`extension/src/webview/menus.ts:155-156`), and with no changes
  `git stash push` only says "No local changes to save".
- **Engine**: refuses before asking: "There is nothing to stash onto before the first commit.", "There are no changes
  to stash.", "Nothing is staged." (`rust/crates/features/src/stash.rs:54-60`, `stash/push.rs:16-21`,
  `stash/push_staged.rs:14-19`).
- **Verdict**: **CHANGELOG**: "Stash says there are no changes to stash before asking for a message."
- **In the CHANGELOG in stage 4** (area 1).

### A10. Create a branch from a stash: the submodules

- **Feature**: `stash.branch`; extension `stashBranch`.
- **Extension**: `git stash branch <name> stash@{N}`, then the update of the submodules to what the stash's commit
  records, through `updatingSubmodules(…, '<ref>^')` (`extension/src/extension/actions.ts:538-542`, `883-893`).
- **Engine**: the plan has the one step and no submodule update (`rust/crates/features/src/stash/branch.rs:44-54`);
  `update_after_checkout` is used only by the checkouts and Undo
  (`rust/crates/features/src/branch/checkout.rs:6`, `checkout_detached.rs:6`, `checkout_remote.rs:11`,
  `checkout_remote_and_rebase.rs:8`, `tag/checkout.rs:6`, `undo/last.rs:62`). The desktop card says it is followed by
  the update of the submodules (`docs/features/08-stashes.md:304-306`).
- **Verdict**: **Merged into S2** (Bug (engine)): the code does not do what its card says. Add `update_after_checkout`
  to `stash.branch`. **Fixed in stage 2**, with S2.

### A11. Undo of a stash dropped: how long, and how many

- **Feature**: `stash.drop` with `stash.undo-drop`; extension `stashDrop` with `restoreStashOp`.
- **Extension**: one remembered operation, `this.undoable`, kept only while the top of the reflog of `HEAD` is what it
  was, and cleared by any other operation that succeeds (`extension/src/extension/controller.ts:1644-1652`, `1693`,
  `1703-1705`). Dropping two stashes in a row leaves only the last one to undo.
- **Engine**: a journal entry `UndoRecord::StashDropped { message, commit }` (`rust/crates/features/src/stash/drop.rs:22-25`),
  kept while `HEAD` (commit and branch) is what it was, up to 50 entries, newest first
  (`rust/crates/engine/src/undo.rs:7`, `45-84`). Both run `git stash store -m <message> <hash>` after a confirmation
  (`extension/src/git/ops.ts:997-999`; `rust/crates/features/src/stash/undo_drop.rs:27-40`).
- **Verdict**: **Merged into D9** (CHANGELOG): "Undo takes back several stash drops in turn, newest first, as long as nothing moved
  HEAD since; another operation in between no longer makes them lost to Undo."

### A12. Preview of apply and pop

- **Feature**: `stash.apply`, `stash.pop` with `stash.preview`; extension `stashApply`/`stashPop`
  (`withStashPreview`).
- **Extension**: counts as changed here the staged and unstaged files that are not untracked, and says the predicted
  conflicts even when git will refuse the stash for files changed here
  (`extension/src/extension/actions.ts:142-156`).
- **Engine**: counts the files in conflict too, and says nothing of the conflicts when git will refuse
  (`rust/crates/features/src/stash.rs:110-116`, `rust/crates/model/src/stash/preview.rs:36-81`). Command, title, note
  and confirmation are the same (`stash.rs:131-134`; `extension/src/git/ops.ts:575-590`).
- **Verdict**: **CHANGELOG**: "Before a stash is applied, files in conflict count as changes git refuses to touch, and
  no conflicts are predicted for a stash git will refuse."
- **In the CHANGELOG in stage 4** (area 1).

### A13. Undo right after a stash push (shared, not a difference)

- **Feature**: `stash.push*` with `undo.last`; extension `stashPush` with `undo`.
- **Extension**: records nothing, and `undoInfo` reads the branch reflog, which `git stash push` does not write, so
  **Undo** takes back the operation before (`docs/features/extension/08-stashes.md:55-57`).
- **Engine**: the same: the push plans are `Undo::Impossible` (`rust/crates/features/src/stash.rs:33-34`, `93`), and
  `undo.last` goes by the reflog of the branch when the journal is empty (`rust/crates/features/src/undo/last.rs:44-79`),
  with no guard for a stash pushed since.
- **Verdict**: **Merged into D7** (Bug (engine)), shared with the extension (the same holds for apply and pop). Recommendation: a push,
  apply or pop drops what the journal and the reflog would undo, or Undo says what it is about to take back as older
  than the last operation. No option needed.

## Branches and tags


### B1. A new branch checked out does not update the submodules in the engine

- Feature: `branch.create` (Create and check out); extension `Action` `createBranch`.
- Extension: after **Create and check out**, the op goes through `updatingSubmodules(op, action.at, …)`
  (`extension/src/extension/actions.ts:246-247`), so `git checkout -b <name> <start>` is followed by
  `git submodule update --init --recursive` when `gitardo.submodules.updateOnCheckout` is on and either side has
  submodules, and it is confirmed when a submodule would lose commits (`actions.ts:883-893`).
- Engine: `plan()` builds only `git checkout -b <name> <start>` or `git branch <name> <start>`, with
  `Danger::None` and no submodule step (`rust/crates/features/src/branch/create.rs:166-183`).
  `update_after_checkout` is called by `checkout.rs`, `checkout_detached.rs`, `checkout_remote.rs`,
  `checkout_remote_and_rebase.rs`, `tag/checkout.rs` and `undo/last.rs`, and by nothing in `branch/create.rs` (grep
  over `crates/features/src`). The desktop card itself asks for the update (`docs/features/06-branches-tags-remotes.md:195-198`).
- Verdict: **Merged into S2** (Bug (engine)). The plan does not do what its own card says. `create.rs` should call
  `update_after_checkout(context, answers, &start, plan)` in `Mode::CheckOut`. **Fixed in stage 2**, with S2. (The same grep shows that `stash.branch`
  does not call it either, while the extension's `stashBranch` updates the submodules (`actions.ts:541`). That one is
  for the stash area.)
- **Fixed in stage 2**, with S2; **Create and check out** has run in the engine since stage 4 (area 1).

### B2. Checking out a local branch "as it is" reads `.gitmodules` at the wrong revision in the extension

- Feature: `branch.checkout-remote`; extension `checkout` with `target: 'remote'`.
- Extension: `updatingSubmodules(op, action.ref, …)` always uses the remote branch as the revision checked out next
  (`actions.ts:234-235`). When the local branch is checked out as it is, git checks out `<name>` and not the remote
  branch (`ops.ts:223-228`), so `.gitmodules` is read at the wrong commit.
- Engine: the revision is the remote branch after a fast-forward or a rebase, and the local branch for **as it is**
  (`rust/crates/features/src/branch/checkout_remote.rs:79-87`).
- Verdict: **Bug (extension)**. The move fixes it. CHANGELOG: "Checking out a local branch as it is, from its remote
  branch, now updates the submodules to what that branch records."
- **Fixed in stage 4** (area 2), with the remote checkout run by the engine; in the extension's CHANGELOG.

### B3. "Delete anyway" is offered after the failure instead of asked at once

- Feature: `branch.delete`; extension `deleteBranch`.
- Extension: when `git branch -d` says "not fully merged", `runWithFallback` runs the `-D` op straight away. The user
  sees only its warning, **Delete branch `<name>` (force)?** (`extension/src/extension/controller.ts:1736-1742`,
  `extension/src/git/ops.ts:241-256`).
- Engine: the `-d` plan has `FollowUp::OfferRun` **Delete anyway**, matched on "not fully merged"
  (`rust/crates/features/src/branch/delete.rs:71`, `77-85`). `for_the_window` hands it to the window with the failure
  (`rust/crates/engine/src/drive.rs:465-481`), so the error is shown first and the warning follows a click. Both plans
  are titled "Delete branch `<name>`", with no "(force)" (`delete.rs:63`).
- Verdict: **CHANGELOG**. "When git refuses to delete a branch it finds not fully merged, Gitardo now shows git's
  refusal with **Delete anyway**, instead of going straight to the forced delete."
- **In the CHANGELOG in stage 4** (area 2). The error has **Delete anyway** beside **Show Log**: the `offerRun`
  follow-up of protocol 1.3 carries the feature, its target and the answer `merged` told `false`, which the server now
  gives the feature from its first call.

### B4. How long Undo keeps a deleted branch

- Feature: `branch.delete` and `undo.last`; extension `Controller.action` with `restoreBranchOp`.
- Extension: one remembered record, kept while the top of the `HEAD` reflog is unchanged
  (`controller.ts:1644-1652`). It is cleared by any other action that succeeds (`controller.ts:1693`). So after
  deleting a branch and then, for example, deleting a tag, **Undo** falls back on the reflog and takes back the last
  move of `HEAD` instead. The extension cards note this, e.g. `docs/features/extension/06-branches-tags-remotes.md:638`.
- Engine: a journal of up to 50 entries (`rust/crates/engine/src/undo.rs:7`, `81-82`). An entry holds while `HEAD`
  (commit and branch) is what it was (`undo.rs:57-65`), whatever other plans ran in between. Successive Undos bring
  back the deleted branches one after the other.
- Verdict: **Merged into D9** (CHANGELOG). "Undo brings back a deleted branch as long as nothing has moved HEAD, even after other
  operations, and several deleted branches can be brought back one after the other."
- **In the CHANGELOG in stage 3**, with D9; the branches are deleted by the engine since stage 4 (area 2).

### B5. The Undo of a deleted branch is a warning in the engine

- Feature: `undo.last` (journal entry `BranchDeleted`); extension `restoreBranchOp`.
- Extension: `git branch <name> <hash>` with `confirm: true`, asked as a question (`ops.ts:969-971`).
- Engine: every journal Undo plan is `Danger::Destructive`, the branch one included
  (`rust/crates/features/src/undo/journal.rs:22-28`, `42-45`).
- Verdict: **Choice**. I recommend `Danger::Confirm` for `BranchDeleted` and `StashDropped`: making a branch again
  loses nothing. The danger of `ConflictsResolved` can stay Destructive. No option is needed. (This belongs to the undo
  area too.)
- **Decided in stage 4 (area 2), as recommended; to be confirmed by the maintainer.** `BranchDeleted` is
  `Danger::Confirm`, saying the branch does not get its upstream back; `StashDropped` already was, and the other records
  stay Destructive (`rust/crates/features/src/undo/journal.rs`). In both CHANGELOGs.

### B6. A remote branch whose local branch is in another worktree

- Feature: `branch.checkout-remote`; extension `checkout` with `target: 'remote'`.
- Extension: the action itself opens the worktree that has the local branch (`actions.ts:696-702`).
- Engine: the feature refuses with "`<name>` is checked out in the worktree `<folder>`."
  (`checkout_remote.rs:50-52`, `94-104`). The card leaves the routing to `branch.open` and the menu, which open the
  worktree instead (`docs/features/06-branches-tags-remotes.md:87-88`).
- Verdict: **Choice**. I recommend that the extension's menu and double click go through `branch.open` and
  `worktree.open`, as the desktop does. Users then see no change, and no engine option is needed. Only a caller that
  bypasses `branch.open` would meet the refusal.
- **Decided in stage 4 (area 2), as recommended; to be confirmed by the maintainer.** Nothing changed: the extension's
  menus and double click already open the worktree that has the local branch (`openBranch` and `openRemoteBranch` in
  `extension/src/webview/menus.ts`), so only a caller that goes past them meets the refusal.

### B10. At a commit, the format asks what `${base}` is last

- Feature: `branch.create` with `branches.format` set, invoked at a commit; extension `createBranch`.
- Extension: after **Use the format**, it asks first what `${base}` is (the commit or a branch that has it), then the
  inputs in their order (`extension/src/extension/actions.ts:1330`).
- Engine: the inputs are asked in the order they first appear, then `${base}` when it is at a commit
  (`rust/crates/model/src/branch/from_format.rs:133`, the card `branch.name-from-format`).
- Verdict: **CHANGELOG**: "At a commit, **Create branch** with a format asks for its inputs first and then what
  `${base}` is." Found by the e2e test of stage 4, run with the engine; the questions and the name made are the same.
- **In the CHANGELOG in stage 4** (area 1, where the e2e found it).

## Remotes and sync

### B7. Delete a branch from its remote: the branch by its short name, on both sides

- Feature: `remote.delete-branch`; extension `deleteRemoteBranch`.
- Extension: `git push <remote> --delete <branch>` with the short name (`ops.ts:294-305`). Its card records that a
  remote with a tag of that name refuses the push as ambiguous (`docs/features/extension/06-branches-tags-remotes.md:405-406`).
- Engine: the same short name, `["push", remote, "--delete", branch]`
  (`rust/crates/features/src/remote/delete_branch.rs:21-24`).
- Verdict: **Bug (engine)**. The bug is on both sides, and the engine's copy is the one that will remain. It should
  pass `refs/heads/<branch>`.
- **Fixed in stage 4** (area 2): the branch is passed as `refs/heads/<branch>` when a tag of the repository has its
  name, and by its short name otherwise (`delete_branch.rs`, test
  `remote_delete_branch__a_branch_a_tag_has_the_name_of_is_named_in_full`). In both CHANGELOGs.

### B8. Sync: fetch, pull and push in one click (desktop only)

- Feature: `sync.sync`; the extension has no counterpart. It has Fetch, Pull and Push as separate buttons.
- Engine: one plan. It runs `git fetch --all --prune`, then the pull of the settings
  (`git pull --rebase --autostash` by default), then `git push <remote> <branch>` only when the upstream counts say
  the branch is ahead (`rust/crates/features/src/sync/sync.rs:57-82`).
  - A branch with no upstream is only published, after the confirmation "`<branch>` has no upstream. Publish it to
    `<remote>`?" (`sync.rs:24-29`, `41-54`).
  - It refuses with "Commit or stash the changes first: the pull cannot rebase over them." when `pull.rebase` is on,
    `pull.autostash` is off and something is staged or unstaged (`sync.rs:30-37`).
  - Danger is `None`, Undo is by the reflog, and it is followed by the autostash check (`sync.rs:74-81`).
- Verdict: **CHANGELOG**. "New: **Sync** fetches, pulls the way **Pull** does and pushes when the branch is ahead, in
  one click; a branch with no upstream is published after asking."
- **In the CHANGELOG in stage 4** (area 2). Given to the extension at the smallest entry point that matches the desktop:
  **Sync: fetch, pull, push** in the menu beside **Push**, and **Gitardo: Sync** in the Command Palette and the **…** of
  the Commits and Gitardo Graph views. It has no TypeScript operation: without the engine it says it needs it and runs
  nothing.

### B9. Switch identity: "Edit the list of identities…"

- Feature: `identity.switch`; extension `Controller.switchIdentity`.
- Extension: the pick ends with **Edit the list of identities…** (`gitardo.identities`), which opens the settings
  (`controller.ts:2107-2109`, `2120`).
- Engine: the pick has the profiles, **Enter a name and email…** and **Use the global identity**, with no entry for
  the settings (`rust/crates/features/src/identity/switch.rs:66-110`).
- Verdict: **Choice**. I recommend keeping it as a host entry. That needs either an engine option for an extra choice
  that the host handles, or a separate command in the extension's palette. Otherwise the CHANGELOG line is "**Edit the
  list of identities…** is no longer in the identity pick: open the setting `gitardo.identities` instead."
- **Decided in stage 4 (area 2), as recommended; to be confirmed by the maintainer.** Kept as a host entry with no
  engine option: the extension adds **Edit the list of identities…** at the end of the engine's pick
  (`HostHooks.choices` in `extension/src/engine/vscode.ts`) and opens the setting itself. Users see no change.

## Merge, rebase, cherry-pick, revert, reset

### C1. Revert skips a commit that changes nothing; the extension stops on it

- Feature: `revert.commit`, extension action `revert`.
- Extension: `revertOp` runs `git revert --no-edit [-m 1] <hash>` (`git/ops.ts:522-524`), and `resolveAction` returns
  it with no `recover` (`extension/actions.ts:489-490`). So a revert whose changes are already taken back stops with
  git's "nothing to commit" message. Only **Continue** and **Skip** get `skipEmptyPicks` (`actions.ts:603-605`).
- Engine: the same command, plus `on_failure: skip_when_empty()` (`features/src/revert/commit.rs`, the plan;
  `features/src/cherry_pick.rs:14-16`). `cherry-pick.skip-empty` then warns in the log and runs `git revert --skip`
  (`features/src/cherry_pick/skip_empty.rs`).
- Verdict: **CHANGELOG**. "A revert of a commit whose changes are already taken back is now skipped, with a warning in
  the log, instead of stopping."
- **Checked in stage 4** (area 3): no difference. Git leaves no stop to skip after a single revert whose changes are
  taken back already: it fails with *nothing to commit* and leaves no `REVERT_HEAD`, so `cherry-pick.skip-empty` finds
  nothing and the engine fails with git's words, as the extension does (`extension/test/engineHistory.test.ts`). The
  skip applies to a series, as for `cherry-pick`. Not in the CHANGELOG.

### C2. Cherry-pick, revert and bisect are refused before git runs while another operation is under way

- Features: `cherry-pick.commit` (`cherryPick`), `revert.commit` (`revert`), `bisect.start` and `bisect.mark`
  (`bisect`).
- Extension: `cherryPick`, `revert` and `bisect` check nothing about an operation under way
  (`actions.ts:362-366`, `489-490`, `594-595`). Git refuses with its own words, for example "cherry-pick is already in
  progress".
- Engine: `refuse_under_way` gives *Finish or abort the `<operation>` first.* for a cherry-pick and a revert
  (`features/src/history.rs:12-18`, called at `features/src/cherry_pick/commit.rs:26-28` and `features/src/revert/commit.rs:21-23`).
  `refuse_while_operation` does the same for starting or marking a bisect (`features/src/bisect.rs`, the function of
  that name; used in `bisect/start.rs` and `bisect/mark.rs`).
- Verdict: **CHANGELOG**. "Cherry-pick, revert and bisect now say *Finish or abort the … first* while a merge, a rebase
  or another operation is under way, instead of passing on git's error."
- **In the CHANGELOG in stage 4** (area 3).

### C3. A squash merge's Undo outlives the next operation in the engine

- Features: `merge.choose-way` (way `squash`) and `merge.undo-squash`. Extension: action `merge`, with `undoSquashOp`
  remembered by `Controller.action`.
- Extension: after a squash merge (also one that stopped on conflicts), `this.undoable = undoSquash`
  (`extension/controller.ts:1681-1685`, `1706-1709`). Any later successful action clears it first:
  `this.undoable = undefined` (`controller.ts:1693`). So "squash merge, stage a file, Undo" takes back whatever the
  reflog shows instead (`repo.ts:1053-1071`).
- Engine: `squash` records `Undo::Journal(UndoRecord::SquashMerge)` (`features/src/merge/choose_way.rs`, fn `squash`).
  The entry holds while `HEAD` stays where it was (`engine/src/undo.rs:57-65`), so staging a file keeps it. It also
  holds after a stop (`model/src/contract/journal.rs:50-52`, `engine/src/drive.rs:277-279`). The extension keeps it
  after a stop only when files are left in conflict (`controller.ts:1684`).
- Verdict: **Merged into D9** (CHANGELOG). "Undo after a squash merge takes it back even after other operations that do not move
  HEAD, such as staging a file." `undo.last` reads the journal before the reflog
  (`rust/crates/features/src/undo/last.rs:35-38`), so the squash merge is what it takes back.
- **In the CHANGELOG in stage 4** (area 3), checked with a file staged after the squash merge and after one that
  stopped on conflicts (`extension/test/engineHistory.test.ts`).

### C4. Rebase of a branch held by another worktree, refused before git runs

- Feature: `rebase.branch-onto`, extension action `rebase` with `branch`.
- Extension: `rebaseOp` runs `git rebase <onto> <branch>` and refuses only names that start with `-`
  (`git/ops.ts:337-341`). Git refuses a branch checked out elsewhere with its own message.
- Engine: when no branch is given, it asks for one from a list that leaves out branches checked out here or in another
  worktree. It refuses *`<name>` is checked out in another worktree: rebase it there.*
  (`features/src/rebase/branch_onto.rs`, fn `branch_onto`).
- Verdict: **CHANGELOG**. "Rebasing a branch that another worktree has checked out now says to rebase it there,
  before git runs."
- **Fixed in stage 4** (area 3) and **in the CHANGELOG**. The refusal could not be reached through the protocol: the
  extension gives the branch dropped as the answer of the question, and the list of the question left out a branch
  held by another worktree, so the engine refused *No other local branch can be rebased onto …* or the answer was not
  one of the choices. The list now keeps such a branch, described *checked out in another worktree*, and the answer
  is refused with the words above (`features/src/rebase/branch_onto.rs`).

## Rewriting (reword, squash, split)

### C5. The files of an interactive rebase are written after you agree, in the git folder, and logged

- Features: `rebase.interactive`, `squash.commits`, `squash.into-previous`, `split.commit` (older commit),
  `message.edit` (older commit). Extension: `rebaseInteractive`, `squash`, `split`, `reword` through
  `interactiveRebase`.
- Extension: `interactiveRebase` writes the todo list and the message files into a `gitardo-rebase-XXXXXX` folder under
  the system temporary folder while the question is built, before the user agrees, with no `write` line in the log
  (`actions.ts:899-921`). The folder is removed only after a clean finish (`actions.ts:918-919`). The card records this
  as **Differs** (`docs/features/extension/07-merge-and-rewrite.md:297-301`).
- Engine: the message files and `todo` are `Step::Write` steps of the plan, so they run after the confirmation and are
  logged. They go into `<git folder>/gitardo-rebase`, which is deleted once no rebase is under way, after success and
  after a stop (`features/src/rebase.rs:97-137`; `engine/src/drive.rs:346-370`).
- Verdict: **CHANGELOG**. "The todo list and messages of an interactive rebase, a squash or a split are now written
  into the repository's git folder only once you agree, each listed in the log and the question, and are removed once
  the rebase ends."
- **In the CHANGELOG in stage 4** (area 3); where the files go is in `docs/extension/11-data.md`.

### C6. Editing an older commit's message can leave its folder behind in `.git`

- Feature: `message.edit` (older commit), extension action `reword`.
- Extension: it uses the shared temporary folder and its clean-up (`actions.ts:288-297`, `899-921`).
- Engine: `rebase_plan` uses a folder of its own, `<git folder>/gitardo-rebase-<now>`. It deletes the folder with a
  `Step::Delete` only after `git rebase -i` succeeds, and has `on_failure: Vec::new()`
  (`features/src/message/edit.rs:187-247`). When the rebase stops, for example on a conflict in one of the commits
  picked after it, nothing deletes the folder later. The other rewrites use `rewrite_plan`, whose
  `DeleteWhenRebaseEnded` follow-up covers that case (`rebase.rs:128-136`).
- Verdict: **Bug (engine)**. Build `message.edit` on `rewrite_plan`, or give it the same `DeleteWhenRebaseEnded`
  follow-ups.
- **Fixed in stage 4** (area 3): `rebase_plan` is built on `rewrite_plan`, so its files go into
  `<git folder>/gitardo-rebase`, with no `Step::Delete`, and `DeleteWhenRebaseEnded` after success and after a stop
  (`features/src/message/edit.rs`); in the desktop's CHANGELOG.

### C7. The message box refuses a message left as it was

- Feature: `message.edit`, extension action `reword`.
- Extension: it refuses only an empty message, *Enter a message.* (`actions.ts:280-281`). An unchanged message still
  runs the amend or the rebase.
- Engine: the question is `Validation::NewMessage { old }` (`message/edit.rs:154-165`), which says *The message is
  unchanged.* (`model/src/contract/question.rs:158-166`).
- Verdict: **CHANGELOG**. "Editing a commit's message now refuses a message that is the same as before." This is minor;
  the extension's box may already prevent it in the view, which was not checked.
- **Checked in stage 4** (area 3): no one sees it. The box of the extension's view cancels a message left as it was
  instead of sending it (`extension/src/webview/Detail.tsx`, `MessageEditor`), so the engine's refusal is never
  reached from the extension. Not in the CHANGELOG.

### C8. Interactive rebase is refused while a bisect or an operation is under way

- Feature: `rebase.interactive`, extension action `rebaseInteractive`.
- Extension: `case 'rebaseInteractive'` calls `interactiveRebase` with no check (`actions.ts:274-275`), whereas
  `reword`, `squash` and `split` refuse an operation and a bisect (`actions.ts:278-279`, `302-303`, `332-333`).
- Engine: `interactive` starts with `refuse_under_way`, which covers both: *Finish or abort the … first.* or *End the
  bisect first.* (`features/src/rebase/interactive.rs:18-21`, `rebase.rs:35-51`). It also refuses a todo list that does
  not name each commit from the oldest up to `HEAD` once (`interactive.rs:47-70`).
- Verdict: **CHANGELOG**. "Interactive rebase is now refused while a bisect is under way, as rewording, squashing and
  splitting already were." The extension's gap is an inconsistency with its own siblings, which the engine fixes.
- **In the CHANGELOG in stage 4** (area 3).

## Bisect and patches

### C9. Bisect refuses marks once the first bad commit is found, and ending a bisect when none is under way

- Features: `bisect.answer`, `bisect.end`, `bisect.start`. Extension actions `bisect` and `bisectReset`.
- Extension: `bisectOp` refuses only **Skip** with no bisect (`git/ops.ts:1049-1057`). `bisectResetOp` always runs
  `git bisect reset` (`ops.ts:1059-1061`). A bisect that is already under way gets no special refusal when started
  again: with a hash it only marks.
- Engine: `bisect.answer` refuses *The bisect found the first bad commit already.* when one commit is left
  (`features/src/bisect/answer.rs`). `bisect.end` refuses *No bisect is in progress.* (`bisect/end.rs`, via
  `bisect_under_way`). `bisect.start` refuses *A bisect is under way already.* (`bisect/start.rs`).
- Verdict: **CHANGELOG**. "Once the bisect has found the first bad commit, Good, Bad and Skip say so instead of running
  git again."
- **In the CHANGELOG in stage 4** (area 3), with **End bisect** when none is under way. `bisect.answer` read its
  answer without asking, so the answer the extension gives could not reach it; it now asks **Mark the commit being
  tested** when no button answered, and refuses once the first bad commit is found before asking
  (`features/src/bisect/answer.rs`).

## Conflicts

### D1. Continue refuses while a file is still in conflict

- Feature: `conflicts.continue-operation`; extension `Action` `continue`.
- Extension: `continue` runs `continueOp` straight away whatever the index holds (`extension/actions.ts:600-603`,
  `git/ops.ts:1209-1211`); git itself refuses, and `reportFailure` shows the stop message again
  (`extension/controller.ts:2495-2516`).
- Engine: refuses before git runs with *Resolve the conflicts and stage the files first.*
  (`features/src/conflicts/continue_operation.rs:29-31`; test at 160-173 of the same file).
- Verdict: **CHANGELOG**. "Continue now says to resolve and stage the files first while a file is still in conflict,
  rather than running git and failing."
- **In the CHANGELOG in stage 4** (area 4), with **Continue** run by the engine.

### D2. Skip throws away resolved conflicts without asking, on both sides

- Feature: `conflicts.skip-step`; extension `Action` `skip`.
- Extension: `skipOp` has no `danger` or `confirm` (`git/ops.ts:1256-1259`); the extension card flags this against the
  promises (`extension/09-conflicts.md`, **Differs** of `operation.skip`, around line 191).
- Engine: the same, `Danger::None` and `Undo::ByReflog` (`features/src/conflicts/skip_step.rs:38-42`), and the desktop
  card keeps it so (`09-conflicts.md:120`).
- Verdict: **Bug (engine)**, shared by both. `git <operation> --skip` drops what was resolved in the step. Recommend
  `Danger::Confirm` with *What was resolved in this step is thrown away*, at least when files of the step were staged;
  no option needed. (The engine also refuses at a stop to edit or split, `skip_step.rs:21-23`, where the extension only
  hides the button: same for the user.)
- **Fixed in stage 4** (area 4): `skip_step` asks with `Danger::Confirm`, *What was resolved in this step is thrown
  away.*, while files are staged, changed or in conflict, and runs at once when the working tree is clean; the
  extension's **Skip** runs it, and its `skipOp` stays the fallback as it was. In the extension's CHANGELOG.

### D3. Resolve automatically is asked twice with `commandPreview` set to `all`

- Feature: `conflicts.resolve-automatically`; extension `Action` `resolveConflicts`.
- Extension: the **Resolve / Review** question is the confirmation; the op then runs with `asked: true`, so the
  preview is skipped (`extension/controller.ts:1670-1673`, `1748-1752`, `2425`).
- Engine: the same question is an `Outcome::Ask` (`features/src/conflicts/resolve_automatically.rs:64-71`,
  `133-149`), and the plan that follows is `Danger::None` (`resolve_automatically.rs:234`). `needs_confirmation`
  returns true for `None` under `all` (`engine/src/preview.rs:12-16`), and `confirm_and_run` asks it
  (`engine/src/drive.rs:256`): a second dialog. The desktop card says the question is the confirmation "whatever
  `commandPreview` says" (`09-conflicts.md:525`).
- Verdict: **Bug (engine)**, minor. A plan whose question was its confirmation needs a way to say so (for example a
  `Danger` or plan flag "already confirmed").
- **Fixed in stage 4** (area 4): `Danger::Agreed`, a plan the user agreed to in the feature's own question, is never
  shown again (`engine/src/preview.rs`); the plans of **Resolve** and of **Review** have it. The protocol sends it as
  `none`, so no version is raised for it.

### D4. The review writes over a file changed while its preview was open

- Feature: `review.write`; extension `Controller.writeResolutions` from `ConflictReviewTab.write`.
- Extension: the check that each file is still what the review read runs when the files are written, after any
  confirmation (`git/exec.ts:65-71`, called from `extension/controller.ts:2442-2445`).
- Engine: the files are read and compared before the plan is made (`features/src/review/write.rs:46-60`); the plan's
  `Step::Write` steps carry no expected text (`review/write.rs:71-74`). With `commandPreview` `all` the plan is shown
  after the check, so a file saved in an editor while the dialog is open is overwritten. `resolve_automatically`
  avoids this by reading again after its question (`resolve_automatically.rs:72-80`).
- Verdict: **Bug (engine)**, minor. Read the files again after the confirmation, or give `Step::Write` an expected
  content checked when it runs.
- **Fixed in stage 4** (area 4), the first way: with `commandPreview` `all`, `review.write` asks its plan as its own
  question (**Write**) before it reads the files again, and the plan is then `Danger::Agreed`; with `off` nothing is
  shown between the read and the write. A file saved while the question is open is refused as any changed file is.

### D5. The log of a review's write says how many conflicts each file got and by whom

- Feature: `review.write`; extension `writeResolutionsOp`.
- Extension: the counts are only in the note, shown when the preview asks; the log has `> write <file>` alone
  (`git/ops.ts:1154-1170`, `extension/controller.ts:2443`; extension card **Differs** at
  `extension/09-conflicts.md:1065`). Its "by whom" never counts the rules when mixed with others
  (`git/ops.ts:1155-1159`: two by the rules and one by the AI reads *1 by the AI*).
- Engine: the plan starts with `Step::Warn` lines, one per file, *a.txt: 3 of 3 conflicts (2 by the rules, 1 by the
  AI)*, then what is left and the AI warning, all logged (`review/write.rs:66-70`, `100-150`).
- Verdict: **CHANGELOG**. "The log now says, for each file the review writes, how many of its conflicts were resolved
  and by whom, and warns when the AI wrote some."
- **In the CHANGELOG in stage 4** (area 4), with the review's write run by the engine.

### D6. Taking one side is a question, not a warning, while the desktop's own list calls it destructive

- Feature: `conflicts.take-side`; extension `Action` `takeSide`.
- Extension: `confirm: true` (`git/ops.ts:1193`, `1203`); its card flags that what was typed in the file is lost
  (`extension/09-conflicts.md`, **Differs** at line 370).
- Engine: `Danger::Confirm` (`features/src/conflicts/take_side.rs:130`), as its card says (`09-conflicts.md:308`); but
  `preview.confirm` lists "taking one side of a file in conflict" and "resolving conflicts automatically" among the
  destructive plans (`16-undo-preview-log.md`, the paragraph after the `preview.confirm` list).
- Verdict: **Choice**. Recommend `Danger::Destructive` for a file whose working-tree text differs from what git wrote
  when it stopped (typed edits lost), `Confirm` otherwise; at least make the desktop's `preview.confirm` list agree
  with the cards. No option needed.
- **Decided in stage 4 (area 4), as recommended; to be confirmed by the maintainer**: `take_side` reads the texts of
  the files in conflict, and the plan is `Danger::Destructive` when the file on disk has a line that neither side nor
  the base has, other than git's markers (*… has lines that neither side has, typed in it since git stopped: they are
  lost too.*), `Confirm` otherwise. What git wrote when it stopped is not read again: a hand resolution made only by
  deleting lines and markers is still asked as `Confirm`. The paragraph of `preview.confirm` agrees with the cards.

## Undo, preview and log

### D7. Undo takes back the operation before one it cannot see, on both sides

- Feature: `undo.last`; extension `Action` `undo`.
- Extension: after a stash, push, fetch, tag, branch created without checkout, worktree or submodule operation, Undo
  reads the reflog and takes back the one before, with nothing saying so (`git/repo.ts:1053-1071`;
  `extension/16-undo-preview-log.md`, **Differs** of `undo.last` around line 199, and the same line on many cards of
  `extension/06-branches-tags-remotes.md`, e.g. line 222).
- Engine: the same. A plan with `Undo::Impossible` (push `features/src/sync/push.rs:26`, fetch `sync/fetch.rs:23`, tag
  `tag/create.rs:44`) leaves nothing in the journal (`engine/src/undo.rs:67-80` adds only `Undo::Journal`), and
  `undo::last` then goes to the reflog (`features/src/undo/last.rs:35-78`); after a stash push the last commit would be
  reset `--soft` (`last.rs:131-133`).
- **Fixed in stage 3**: the journal is a stack of Gitardo's operations, each plan that cannot be taken back an entry
  with its reason once a reference changed, and `undo.last` refuses with it (`engine/src/undo.rs`,
  `features/src/undo/last.rs`); the extension records its own operations with `journal/record`.
- Verdict: **Bug (engine)**, shared. The journal should also record "the last plan cannot be undone" (its reason)
  until `HEAD` moves, and `undo.last` should refuse with that reason, or at least say the reflog entry is not the last
  operation. No option needed. A13 (stashes) and W11 (worktrees, setup, submodules) found the same, and so did the
  last line of the identical list of area B (push, fetch, tags, rename, fast-forward).

### D8. Undo does not refuse while an operation is under way

- Feature: `undo.last`; extension `Action` `undo`.
- Extension: refuses with *Finish or abort the `<operation>` first.* (`extension/actions.ts:387-388`), except for what
  it remembers right after a squash merge stopped on conflicts (`extension/controller.ts:1648-1652`, `1684`).
- Engine: only a bisect is refused (`features/src/undo/last.rs:31-34`). During a stopped merge, rebase, cherry-pick or
  patch it plans a checkout or a `git reset --soft/--mixed/--keep` from the reflog (`last.rs:52-77`); during a stopped
  series it runs `git reset --keep <head before>` from the journal (`features/src/undo/journal.rs:60-63`, the entry
  kept by `holds_after_a_stop`, `model/src/contract/journal.rs:50-52`, `engine/src/drive.rs:277-279`), which leaves the
  sequencer of the cherry-pick behind. After a take-side of a file one side deleted (`Undo::Impossible`,
  `take_side.rs:123-125`) the next Undo goes to the reflog mid-merge for the same reason.
- **Fixed in stage 3**: `undo.last` refuses with *Finish or abort the `<operation>` first.* unless the newest entry
  is a squash merge or a resolution of conflicts.
- Verdict: **Bug (engine)**. Refuse as the extension does while `repository.operation` is set, unless the newest journal
  entry is a squash merge or a resolution of conflicts (which are meant to be taken back mid-operation); a stopped
  series is better taken back by **Abort**. No option needed.

### D9. What Undo remembers: one operation cleared by any other, against a journal of 50 that holds while HEAD stays

- Feature: `undo.journal`, `undo.last`, `conflicts.restore`; extension `Controller.undoable`.
- Extension: one remembered operation, valid while the top entry of `HEAD`'s reflog is unchanged
  (`extension/controller.ts:1644-1652`), and forgotten after any operation that succeeds through `Controller.action`,
  staging a file included (`controller.ts:1693`), but not after one run by `runIn` or outside `action`
  (`controller.ts:2414-2417`; extension card `16-undo-preview-log.md`, **Differs** of `undo.last`).
- Engine: up to 50 entries, oldest dropped first (`engine/src/undo.rs:7`, `81-82`), each holding while `HEAD` (commit
  and branch) is what it was once its plan ran (`undo.rs:57-61`); an Undo drops only the entry it took back
  (`undo.rs:61-63`), so two branches deleted are brought back by two Undos, and a stage in between forgets nothing.
- **In the CHANGELOG in stage 3**, with **Undo** run by the engine in the extension.
- Verdict: **CHANGELOG**. "Undo now remembers up to 50 of Gitardo's own operations the reflog cannot see (branches
  deleted, stashes dropped, conflicts resolved, series), for as long as nothing moves HEAD: staging a file no longer
  makes it forget them, and undoing again brings back the one before." A11 (stash drops), B4 (deleted branches) and C3
  (squash merge) are instances of it.

### D10. Undo asks with a warning, and says "Nothing to undo"

- Feature: `undo.last`; extension `undoOp`, `restoreBranchOp`, `restoreStashOp`, `undoSquashOp`.
- Extension: `confirm: true`, an information dialog (`git/ops.ts:1005`, `1016`, `970`, `998`, `331`); refusal *There
  is nothing to undo.* (`extension/actions.ts:392`); notes per reset mode (`git/ops.ts:1007-1012`).
- Engine: `Danger::Destructive` with the title as the reason (`features/src/undo/last.rs:177-183`,
  `undo/journal.rs:21-28`, branch adds *The branch does not get its upstream back.*); refusal *Nothing to undo*
  (`last.rs:80-82`); with `--keep` and changes in the way, *Undo would overwrite changes that are not committed: commit
  or stash them first* (`last.rs:156-162`); after, it selects `HEAD` and says *Undid …* (`last.rs:184-187`).
- **In the CHANGELOG in stage 3**, with **Undo** run by the engine in the extension.
- Verdict: **CHANGELOG**. "Undo now asks with a warning, says in words what it takes back, and selects the commit HEAD
  is at once done."

### D11. The log keeps no output of commands that succeeded, nor what was given on standard input

- Feature: `log.record`; extension `Controller.run`.
- Extension: logs `> <command>`, then `standard input:` and the input, then whatever git printed, at info
  (`extension/controller.ts:2446-2453`); `> write <file>` for writes (`2443`).
- Engine: logs the step line before it runs and only what git said when it failed (`engine/src/steps.rs:93-108`,
  `engine/src/log.rs:78-102`); the line has no `> ` prefix (`model/src/contract/plan.rs:119-149`) and no standard
  input.
- Verdict: **Choice**. Users read git's output in the log (push hints and pull request links, what a pull or an
  autostash did, rebase messages). Recommend the engine log git's output under a successful step too (a new
  `LogKind`), at least at `debug`, and the standard input beside the step that took it, as `preview.confirm` already
  shows it. An engine change, not an option.

### D12. The automatic skip of an empty pick is asked first under `commandPreview` `all`

- Feature: `cherry-pick.skip-empty` inside `conflicts.continue-operation`, `conflicts.skip-step`; extension
  `skipEmptyPicks` in `recover`.
- Extension: the skip runs inside the operation already confirmed, logged as a warning and `> git cherry-pick --skip`
  (`git/ops.ts:1219-1240`).
- Engine: the skip is a feature of its own run from `on_failure` (`features/src/cherry_pick.rs:14-16`,
  `cherry_pick/skip_empty.rs:31-43`) through `run_feature`, so its `Danger::None` plan is confirmed again under `all`
  (`engine/src/drive.rs:321-339`, `256`).
- Verdict: **CHANGELOG**. "With commandPreview set to all, skipping a cherry-picked commit that changes nothing is now
  shown and asked first."
- **Settled in M48**: the engine's way is kept (D90); the line is in the extension's CHANGELOG, and the card
  `cherry-pick.skip-empty` says it.

## Worktrees and their setup

### W1. A format of branch names with `${input:…}` is not used when creating a worktree

- **Feature**: `worktree.create`, `worktree.create-from-branch`, `worktree.create-at-commit`; extension `worktreeAdd`
  (`addWorktree`).
- **Extension**: with `gitardo.branches.format` set, it offers **Use the format** and, when picked, asks every
  `${input:<id>}` of `gitardo.branches.inputs` in turn, remembering the last values (`nameFromFormat`,
  `extension/src/extension/actions.ts:1251-1279`, called at `actions.ts:1408-1412`; inputs asked by `askInput`,
  `actions.ts:1212-1245`).
- **Engine**: `format_of` treats a format that contains `${input:` as not set, so the **Use the format** question is
  never asked and the name is typed (`features/worktree/names.rs:24-30`, used by `features/worktree/create.rs:31-37`).
  The desktop card says the format goes through `branch.name-from-format`, which asks the inputs
  (`docs/features/12-worktrees.md:91-92`, `:138`, `:173`).
- **Verdict**: **Bug (engine)**. The code does less than its own card; a team format such as
  `PRJ-${input:ticket}-${base}` would silently stop being offered for worktrees. Fix by asking through
  `branch.name-from-format`, as the card says.
- **Fixed in stage 4** (area 5): the three features that make a worktree ask the format through
  `branch.name-from-format`, its inputs after the start, and keep them (`features/worktree/names.rs`, `formatted`), with
  a test each.

### W2. The `${base}` picker of a worktree at a commit lacks the branches that contain it

- **Feature**: `worktree.create-at-commit`; extension `worktreeAdd` with a commit (`commitBase`).
- **Extension**: lists the branches at the commit, then the short hash, then up to 50 other branches that contain the
  commit, **Branch that has this commit** (`extension/src/extension/actions.ts:1316-1335`).
- **Engine**: `base_question` lists only the branches pointing at the commit and the short hash
  (`features/worktree/create_at_commit.rs:75-106`). The desktop card asks for the containing branches too
  (`docs/features/12-worktrees.md:170-173`).
- **Verdict**: **Bug (engine)**: the code is behind its card. Add the `branch::read_containing` read (map row 2a
  `branchesContaining`).
- **Fixed in stage 4** (area 5): `${base}` of the commit read with `branch::read_containing`, the branches at the commit
  first, then the short hash, then those that have it (`features/worktree/create_at_commit.rs`), with a test.

### W3. `${base}` for a worktree started from a detached HEAD

- **Feature**: `worktree.create`; extension `worktreeAdd` without `ref`.
- **Extension**: the **Detached** choice gives `${base}` the short hash (`extension/src/extension/actions.ts:1385-1386`).
- **Engine**: the choice is the literal `HEAD` and the format is filled with it, giving names such as `HEAD-spike`
  (`features/worktree/create.rs:20`, `:44-45`, `:82`).
- **Verdict**: **Bug (engine)**, minor: use the short hash of `HEAD` as `${base}`, as for a commit in
  `worktree.create-at-commit`.
- **Fixed in stage 4** (area 5): the short hash of a detached `HEAD` is its `${base}` (`features/worktree/create.rs`,
  `base_of_start`), with a test.

### W4. Where the settings of new worktrees are read and saved

- **Feature**: `worktree-setup.link-and-copy-steps`, `worktree-setup.command-steps`, `worktree-setup.save`,
  `worktree-setup.link-into-new-worktrees` and the others of `give.rs`, `worktree-setup.bring-*`; extension `withSetup`,
  `setUpExistingOp`, `saveSetup`, `setUpFromExplorer`.
- **Extension**: reads and writes VS Code settings `gitardo.worktrees.linkFolders`, `copyFiles`, `setupCommands`
  (workspace folder, workspace file or user), falling back on VS Code's `git.worktreeSymlinkFolders` and
  `git.worktreeIncludeFiles` (`extension/src/extension/actions.ts:793-826`; card
  `docs/features/extension/12-worktrees.md:734` `setup.source`, `:883-906`).
- **Engine**: reads three files itself, the repository's `.gitardo/settings.json`, Gitardo's user `settings.json` and
  the repository's `.vscode/settings.json` for VS Code's two keys, with the keys `worktrees.linkFolders` etc. without the
  `gitardo.` prefix (`features/worktree_setup/files.rs:1-24`, `:55-63`); `save` writes those files
  (`features/worktree_setup/save.rs:21-73`).
- **Verdict**: **Choice**. The rules say a setting keeps its name and meaning (`AGENTS.md`, `docs/extension/11-data.md`).
  Recommendation: an engine option where the host supplies the settings in effect and the places to write them (the
  extension passing VS Code's configuration), so that `gitardo.worktrees.*` in VS Code settings keeps working. An engine
  option is needed.
- **Decided in stage 4** (area 5), as recommended; to be confirmed by the maintainer. Through the settings rather than
  a new method: with `host` `vscode`, the engine takes `worktrees.linkFolders`, `copyFiles` and `setupCommands` from the
  settings the extension sends (`extension/src/engine/settings.ts`, with the `git.*` fallbacks) and reads no file
  (`server/src/settings.rs`, `features/worktree_setup/files.rs`, `settings_given`). Writing them stays in the
  extension, in VS Code's settings, until `WriteSetting` (Q399): under VS Code the engine refuses `worktree-setup.save`,
  the Explorer's entries and the others that write its own files.
- **Settled in M48**: `FollowUp::WriteSetting` has a `target`, the workspace folder, the workspace or the user (Q399,
  protocol 1.13). Under VS Code, `worktree-setup.save`, `worktree-setup.remove-from` and the four entries of the
  Explorer plan follow-ups in place of the steps that write a file, and the extension writes them with
  `WorkspaceConfiguration.update` under `gitardo.worktrees.*` (and `git.*` for VS Code's two), each list where it is
  set; `feature/run` gives them `setup`, what VS Code sets in the repository's folder and where. `saveSetup` and the
  Explorer's TypeScript are gone from `src/extension/worktreeSettings.ts`. The desktop keeps its steps.
  `worktree-setup.edit-settings-file` stays refused under VS Code: opening VS Code's own file of the settings is the
  extension's, which writes nothing.

### W5. Who decides that the setup commands may run

- **Feature**: `worktree-setup.command-steps` with `trust.ask`; extension `withSetup` / `runSetupCommands`.
- **Extension**: runs the commands only in a trusted VS Code workspace and otherwise logs and warns that they did not
  run; nothing is asked (`docs/features/extension/12-worktrees.md:906-908`).
- **Engine**: asks whether to trust the folder before the plan (`trust::ask`), and records the folder trusted as a
  follow-up (`features/worktree_setup/new_worktree.rs:48-53`; `features/worktree/add.rs:89-93`).
- **Verdict**: **Choice**. Recommendation: inside VS Code, let the host answer trust from workspace trust (an engine
  option, "trust given by the host"), so no second trust question appears; keep `trust.ask` for the desktop.
- **Decided in stage 4** (area 5), as recommended; to be confirmed by the maintainer. With `trust` `vscode` the
  engine takes the folder as trusted and asks nothing (`server/src/settings.rs`); with `host.terminal` it hands the
  setup commands over in `waiting`, and the extension runs them in VS Code's terminals, which run nothing in a
  workspace not trusted (`extension/src/extension/controller.ts`, `worktreeMade`).

### W6. Removing a worktree whose folder is gone

- **Feature**: `worktree.remove`; extension `worktreeRemove`.
- **Extension**: **Remove…** is offered on a missing worktree too and runs `git worktree remove <folder>`
  (`docs/features/extension/12-worktrees.md:608-609`; `extension/src/extension/actions.ts:400-422`, no check for a missing
  one).
- **Engine**: refuses with *The folder of this worktree is missing from `<folder>`: prune the worktrees to forget it.*
  (`features/worktree.rs:77-85`, used at `features/worktree/remove.rs:76-80`); it also refuses the main worktree itself
  (`remove.rs:81-85`), which the extension leaves to git.
- **Verdict**: **CHANGELOG**: "Removing a worktree whose folder is gone now says to prune the worktrees instead."
- **In the CHANGELOG** in stage 4 (area 5), with **Remove** run by the engine.

### W7. The forced removal after git refuses a worktree with changes

- **Feature**: `worktree.remove`; extension `worktreeRemoveOp` fallback.
- **Extension**: when git says `modified or untracked files` or `--force`, it asks at once, as a warning, *Remove
  worktree <folder> (force)?* and runs `git worktree remove --force` (`extension/src/git/ops.ts:792-806`,
  `extension/src/extension/controller.ts:1736-1742`).
- **Engine**: the plan fails and the window offers a **Remove anyway** button on the failure, which runs the feature
  again with `force` (Destructive) (`features/worktree/remove.rs:150-165`; `engine/src/drive.rs:465-481`). Note that
  `submodule.deinitialize` uses `OfferWhenFailed`, which asks at once like the extension
  (`features/submodule/deinitialize.rs:47`, `engine/src/drive.rs:447-460`): the engine is not consistent with itself.
- **Verdict**: **CHANGELOG**: "When git refuses to remove a worktree with changes, the message now has a Remove anyway
  button instead of asking at once." Consider `OfferWhenFailed` here for consistency with deinitialize.
- **In the CHANGELOG** in stage 4 (area 5). The engine offers **Remove anyway** on the failure, as it is: the
  follow-up `offerRun` is now sent in the error's `data.followUps` (protocol 1.3), and the extension shows it as a
  button beside **Show Log**. `submodule.deinitialize` keeps asking at once (`OfferWhenFailed`), as the extension did.

### W8. Folders left empty when the branch of a removed worktree is kept

- **Feature**: `worktree.remove` with its branch; extension `worktreeRemove` with `deleteBranch`.
- **Extension**: `git branch -d` runs before the empty folders go (`finish`), so when git keeps an unmerged branch the
  folders stay (`extension/src/git/ops.ts:786-802`, `extension/src/extension/actions.ts:409`; **Differs**,
  `docs/features/extension/12-worktrees.md:640-643`).
- **Engine**: the empty folders are removed right after `git worktree remove`, before `git branch -d`
  (`features/worktree/remove.rs:126-139`).
- **Verdict**: **Bug (extension)**, fixed by the move. CHANGELOG: "A folder left empty by removing a worktree now goes even when its branch is kept for not
  being merged."
- **Fixed in stage 4** (area 5), with **Remove** run by the engine; in the CHANGELOG.

### W9. Folders left empty when renaming a worktree and its branch fails half way

- **Feature**: `worktree.rename`; extension `worktreeRename` (`renameWorktree`).
- **Extension**: `git worktree move`, `git branch -m`, then the empty folders in `finish`, which does not run when
  `branch -m` fails (`extension/src/git/ops.ts:810-819`, `extension/src/extension/actions.ts:1504-1511`; **Differs**,
  `docs/features/extension/12-worktrees.md:582-584`).
- **Engine**: the same order, `worktree move`, `branch -m`, then `remove_empty_folders`, and a plan stops at the first
  failed step (`features/worktree/rename.rs:203-213`).
- **Verdict**: **Bug (engine)**, present in both: put the empty-folder steps right after `git worktree move`, as
  `worktree.remove` does.
- **Fixed in stage 4** (area 5): the empty folders go right after `git worktree move`, before `git branch -m`
  (`features/worktree/rename.rs`), with a test.

### W10. A remote branch opened in a worktree is passed by its short name

- **Feature**: `worktree.open-in-worktree`; extension `openInWorktree`.
- **Extension**: `git worktree add --track -b <branch> <folder> <remote>` with the remote branch through
  `refArg('remote', …)`, `refs/remotes/<name>` when a namesake would make it ambiguous
  (`extension/src/extension/actions.ts:765`).
- **Engine**: passes `remote.to_owned()` as typed (`features/worktree/open_in_worktree.rs:46-55`), while
  `worktree.create` does use `ref_argument` for the same case (`features/worktree/create.rs:110-122`).
- **Verdict**: **Bug (engine)**, minor: use `ref_argument(RefKind::RemoteBranch, …)` here too.
- **Fixed in stage 4** (area 5): `ref_argument(RefKind::RemoteBranch, …)` (`features/worktree/open_in_worktree.rs`),
  with a test.

### W11. Undo right after an operation that Undo does not take back

- **Feature**: every worktree, setup and submodule operation (`Undo::Impossible`); extension `Controller.action`.
- **Extension**: records nothing, so **Undo** right after takes back the operation before it from the reflog
  (`extension/src/extension/controller.ts:1693`; **Differs** on `worktree.create`, `rename`, `remove`, `prune`,
  `setup.bring-ignored`, `submodule.add` and others, `docs/features/extension/12-worktrees.md:195`, `:582`, `:640`,
  `:689`, `:1284`, `docs/features/extension/13-submodules.md:277`).
- **Engine**: the plans say *Not taken back by Undo* (`features/worktree/add.rs:58-63`, `features/worktree/remove.rs:170`,
  `features/submodule/add.rs:75`), but `undo.last` goes by the journal and then the reflog with no knowledge that the last
  plan was not undoable (`features/undo/last.rs:29-79`; `engine/src/undo.rs:51-84` keeps only `Undo::Journal` plans), so
  it takes back the operation before, the same as the extension.
- **Verdict**: **Merged into D7** (Bug (engine)), shared with the extension (promise 5). Cross-cutting, probably decided once in the
  Undo area: the journal could keep a "not undoable" marker that makes `undo.last` refuse or warn.
- Left to the Undo area, as D7 says.

### W12. Saving what new worktrees get is now a logged plan

- **Feature**: `worktree-setup.save`, `worktree-setup.link-into-new-worktrees`, `copy-into-new-worktrees`,
  `stop-linking`, `stop-copying`; extension `saveSetup`, `setUpFromExplorer`.
- **Extension**: writes the settings outside `Controller.run`: nothing is shown first or logged (**Differs**,
  `docs/features/extension/12-worktrees.md:1212`, `:1328`, `:1369`, `:1413`).
- **Engine**: a plan whose steps write the file, logged like any other (`features/worktree_setup/save.rs:46-72`;
  `features/worktree_setup/give.rs:189-190`).
- **Verdict**: **CHANGELOG**: "Saving what new worktrees get, from the page or the Explorer, now goes into the log of
  Gitardo." (Depends on W4 for where it is written.)
- **Not moved in stage 4** (area 5): saving what new worktrees get stays in the extension, in VS Code's settings,
  until `WriteSetting` (Q399, W4); the line goes into the CHANGELOG with that move.
- **Settled in M48**, with W4: saving is the engine's plan, shown first under `commandPreview` `all` with the settings
  it writes as its lines, and each setting written is in the log; the line is in the extension's CHANGELOG. **Stop
  Linking into New Worktrees** now takes out the link pattern of the path alone, and **Stop Copying into New Worktrees**
  the copy pattern, as in the desktop, where the extension took both out with either.

## Submodules

### S1. Cloning from a folder on disk is offered again from the step that failed

- **Feature**: `submodule.allow-file-protocol` (used by `submodule.add`, `update-to-recorded`, `update-to-latest`,
  `initialize`, `update-on-checkout`); extension `allowingDisk`.
- **Extension**: the fallback is the whole operation again with `-c protocol.file.allow=always`
  (`extension/src/git/ops.ts:848-861`). For `submodule.add`, when `git submodule add` succeeded and the nested
  `git submodule update --init --recursive` is the one refused, the retry runs `git submodule add` again, which git
  refuses as the folder is in the index already.
- **Engine**: the plan is offered again from the failed step on (`model/src/submodule/file_protocol.rs:33-57`,
  `engine/src/drive.rs:447-456`).
- **Verdict**: **Bug (extension)**, fixed by the move. CHANGELOG: "Allowing a submodule to be cloned from a folder on
  disk now runs again only the commands that had not run."
- **In the CHANGELOG** in stage 4 (area 5), with the submodule operations run by the engine.

### S2. No submodule update after creating and checking out a branch, or after a branch from a stash

- **Feature**: `submodule.update-on-checkout` in `branch.create` (check out) and `stash.branch`; extension
  `updatingSubmodules` on `createBranch` and `stashBranch`.
- **Extension**: both end with `git submodule update --init --recursive` when `gitardo.submodules.updateOnCheckout` is on
  and there are submodules (`extension/src/extension/actions.ts:246-247`, `:541`, `:883-893`).
- **Engine**: only the checkouts, `tag.checkout` and `undo.last` call `update_after_checkout`
  (`features/branch/checkout.rs:28`, `checkout_detached.rs:27`, `checkout_remote.rs:149`,
  `checkout_remote_and_rebase.rs:34`, `features/tag/checkout.rs`, `features/undo/last.rs:47-55`); `branch.create` runs
  `git checkout -b` alone (`features/branch/create.rs:165-182`) and `stash.branch` likewise
  (`features/stash/branch.rs:44-46`). The desktop card says both use it (`docs/features/13-submodules.md:452-453`).
- **Verdict**: **Bug (engine)**: wrap the two plans in `update_after_checkout`, as the card says. A10 and B1 found
  the same.
- **Fixed in stage 2**: `branch.create` with **Create and check out** and `stash.branch` end with
  `update_after_checkout` (`features/branch/create.rs`, `features/stash/branch.rs`), with a test each.

### S3. No retry on Windows when a submodule folder is held

- **Feature**: `submodule.deinitialize`, `submodule.remove`; extension `lettingGoOf`.
- **Extension**: closes the submodule and those inside it in VS Code's Git (host-only), then runs a command that failed
  with `could not remove`, `Permission denied` or `Device or resource busy` again, 20 times 200 ms apart
  (`extension/src/extension/actions.ts:1175-1195`).
- **Engine**: the held-folder recovery covers only `git worktree move` and `git worktree remove`
  (`model/src/worktree/held.rs:64-84`, `engine/src/steps.rs:129-160`). The desktop cards promise the retry
  (`docs/features/13-submodules.md:288`, `:314-315`).
- **Verdict**: **Bug (engine)**: extend `recover_held_folder` to `submodule deinit` and `rm` of a submodule. It matters
  most inside VS Code, whose Git extension watches the submodule folders.
- **Fixed in stage 4** (area 5): `recover_held_folder` tries `git submodule deinit` and `git rm` again on Windows
  while the folder is held, as it does `git worktree move` and `remove` (`model/src/worktree/held.rs`), with a test
  each.

### S4. The folder of a new submodule is not a literal pathspec

- **Feature**: `submodule.add`; extension `submoduleAddOp`.
- **Extension**: `git submodule update --init --recursive -- <pathspec(dir)>`, `:(literal)` for a folder with `*?[]\`
  or a leading `:` (`extension/src/git/ops.ts:77-79`, `:872`).
- **Engine**: passes the folder as typed (`features/submodule/add.rs:70`), although `submodule.rs` has `literal()` for
  the other submodule commands (`features/submodule.rs:123-130`).
- **Verdict**: **Bug (engine)**, minor (rule R9.2): use `literal(folder)` in that step.
- **Fixed in stage 4** (area 5): `literal(folder)` in the update step (`features/submodule/add.rs`), with a test.

### S5. Refusals added where there is nothing to do

- **Feature**: `submodule.update-to-recorded`, `submodule.stage-recorded`, `submodule.update-to-latest`; extension
  `submoduleUpdate`, `submoduleRecord`.
- **Extension**: runs the update whatever the submodule's state (`extension/src/extension/actions.ts:444-447`,
  `:868-876`); refuses staging only when not initialized (`actions.ts:448-451`); updates to the latest also when not
  initialized (`extension/src/git/ops.ts:890-897`, `--init`).
- **Engine**: refuses *No commit is recorded for `<folder>` yet.* and *`<folder>` is on the recorded commit already.*
  (`features/submodule/update_to_recorded.rs:21`, `:24`); staging on the recorded commit is refused
  (`features/submodule/stage_recorded.rs:25`); update to latest refuses one not initialized, *…: update it first.*
  (`features/submodule/update_to_latest.rs`, via `initialized`, `features/submodule.rs` `initialized`).
- **Verdict**: **CHANGELOG**: "Updating or staging a submodule that is already on its recorded commit now says so
  instead of running git; update a submodule before taking it to the latest of its branch."
- **In the CHANGELOG** in stage 4 (area 5), with the submodule operations run by the engine.

### S6. A clone that cannot be deleted fails the removal of a submodule

- **Feature**: `submodule.remove`; extension `submoduleRemove`.
- **Extension**: deletes `.git/modules/<name>` in `finish`, after the commands, only when it exists, and a failure is a
  warning in the log (`extension/src/extension/actions.ts:455-474`).
- **Engine**: the delete is the last step of the plan (`features/submodule/remove.rs:39`); a missing folder is fine
  (`engine/src/steps.rs:223-234`) but a failure to delete fails the plan, though the desktop card says it "is said in
  the log and stops nothing" (`docs/features/13-submodules.md:313-314`).
- **Verdict**: **Bug (engine)**, minor: the plan reports a failure after the submodule was removed and staged. Make the
  delete a step that goes on after failing, as links and copies do.
- **Fixed in stage 4** (area 5): the clone is deleted by a `Cleanup` follow-up, run once the plan succeeded, whose
  failure is only said in the log (`features/submodule/remove.rs`), with a test.

### Host-only, noted

- After creating a worktree the extension offers **Open in Agents Window** / **Open in New Window**
  (`extension/src/extension/actions.ts:1446-1453`); the engine offers new window, external editor and terminal
  (`features/worktree/add.rs:98-111`).
- The setup commands run as VS Code tasks in the extension and as `Step::Task` in the engine, whose note says "in
  Tasks" rather than "in a terminal of this window" (`features/worktree_setup/new_worktree.rs:172-174`,
  `extension/src/extension/actions.ts:804-806`).
- Closing a worktree or submodule in VS Code's Git before moving, removing or deinitializing it
  (`closeInVSCodeGit`, `extension/src/extension/actions.ts:1100-1120`) has no engine counterpart; the host must keep
  doing it before the plan.

## Reads (stage 5)

Stage 5 asked each read of version 1.4 of [the protocol](02-protocol.md#reads) of the engine and of the extension's
TypeScript against the same throwaway repositories (`extension/test/engineReads.test.ts`). They answer the same but for
what follows. R2 to R5 are not differences a user sees: the engine wins (D45), and a caller moved to the engine reads
its shape.

### R1. A place past the end of a reflog

- **Feature**: `search.read-revision`, and `details.read-commit` for a stash; extension `Repo.resolve`.
- **Extension**: `git rev-parse --verify --quiet <revision>^{commit}` with its failure allowed: `stash@{3}` with two
  stashes, or `HEAD@{99}`, is no commit (`extension/src/git/repo.ts:463-468`).
- **Engine**: git ends with 128 for such a name ("log for 'stash' only has 2 entries"), which `read_revision` took for
  a failure: the read answered `gitFailed` where the name stands for no commit (`rust/crates/git/src/read/search.rs`).
- **Verdict**: **Bug (engine)**. **Fixed in stage 5**: a place past the end of a reflog is no commit, an acceptance
  line of `search.read-revision` and its test (`search_read_revision__a_place_past_the_end_of_a_reflog_gives_none`),
  and a stash past the last is `null` for `details.read-commit`. The desktop's CHANGELOG says it.

### R2. A file a commit does not have

- **Feature**: `file-view.read-revision`, `diff.read-sides`; extension `Repo.show`.
- **Extension**: `git show <ref>:<file>` with its failure allowed gives an empty text for a file the commit does not
  have, as for an empty file (`extension/src/git/repo.ts:1086-1088`).
- **Engine**: `null` for a file the commit does not have or a name that is no commit, the bytes otherwise
  (`rust/crates/git/src/read/file_view.rs`).
- **Verdict**: the engine tells the two apart; nothing a user sees.

### R3. The newlines at the end of the message git prepared

- **Feature**: `commit.prepared-message`; extension `Repo.squashMessage`.
- **Extension**: `SQUASH_MSG` as it is, with the newlines at its end (`extension/src/git/repo.ts:232-236`).
- **Engine**: the message without the whitespace at its end, and `null` for an empty one
  (`rust/crates/git/src/read/commit.rs`).
- **Verdict**: git strips them when it commits; nothing a user sees but the cursor at the end of the message.

### R4. The version before, for the blame of one line

- **Feature**: `blame.read-line`; extension `Repo.blameLine`.
- **Extension**: `parseBlameLine` reads the commit, its author, its time and its summary, and leaves out the `previous`
  line of git (`extension/src/git/parse.ts:338-350`); `Repo.blame` reads it.
- **Engine**: `previous`, the commit and the name the file had before, for one line as for the whole file.
- **Verdict**: the engine gives more; nothing a user sees.

### R5. The same answer in another shape

- `details.read-commit`: the message whole where the extension splits a subject and a body; for the uncommitted
  changes no id, no author and an empty message, where the extension says `WORKTREE`, empty authors and the subject
  **Uncommitted changes**; the untracked files of the changes are among `files`, added, on both sides.
- `compare.read-sides`: `onlyInFirst` and `onlyInSecond` are the extension's `behind` and `ahead`; the sides are
  `first` and `second` for `from` and `to`; the engine says what a merge would do in `merge`, where the extension has
  `conflicts`, and has no `since` and `matched`.
- `submodule.read-list`: `parent` is `null` at the top, `''` in the extension; what is checked out is in `checkout`.
- `worktree-setup.read-ignored`: the kind of an entry is a tagged union where the extension has the flags `folder` and
  `link`; a file counts as one file in both.
- `worktree-setup.read-what-patterns-take`: each pattern with how many paths it takes and the first of them, where the
  extension keeps the counts by pattern.

Three reads have no read of the extension to compare with: `worktree.read-folder-links`, which the extension reads only
inside `unlinkFolderLinks` as it removes them (`extension/src/git/worktreeSetup.ts:233`);
`worktree-setup.explain-change`, read inside `changeSetup` (`extension/src/git/worktreeSetup.ts:490`); and
`github.read-pull-requests`, of which only the repository found on GitHub (`parseGitHubRemote`) and the answer without
a token are compared, since GitHub is not asked offline.

Moving the views onto these reads (`extension/src/engine/reads.ts`, tested by `extension/test/reads.test.ts`) found
two more, which compare the reads as the views ask them.

### R6. The names of the sides in a patch from the working tree

- **Feature**: `compare.read-patch`, the patch to copy; extension `Repo.compareText`, for **Copy patch** of a
  comparison whose first side is the working tree.
- **Extension**: `git diff -R <second>`, in which git names the sides `b/` and `a/`, the other way round from every
  other patch (`extension/src/git/repo.ts`, `diffSides`).
- **Engine**: the sides named `a/` and `b/`; the lines are the same.
- **Verdict**: **CHANGELOG**. `git apply` takes both. "The patch copied from a comparison that starts at the working
  tree names its sides `a/` and `b/`, as every other patch does."

### R7. A comparison narrowed to a path, or from the working tree since they diverged

- **Feature**: `compare.read-sides`; extension `Repo.compareSides`.
- **Extension**: how many commits each side has, and what a merge would do, are those of the whole sides, also when
  the comparison is narrowed to a path; from the working tree in **Since they diverged**, the files are compared from
  the merge base of `HEAD` and the other side (`extension/src/git/repo.ts`, `compareSides`).
- **Engine**: narrowed, it counts only the commits that change the path, as its card says, and so asks no merge when
  one side has none of them; from the working tree, the files are compared with the other side whatever the mode
  (`git diff -R <second>`).
- **Verdict**: **Choice** ([Q407](../08-decisions.md#open-questions)). Until it is decided, the extension reads these
  two cases in TypeScript, and every other comparison from the engine. Recommended: what the extension does today,
  which is what the tab of a comparison says (the sides' distance in its header, the files since the merge base in
  **Since they diverged**); the engine's card changes to it. **Fixed in stage 10** as recommended:
  `compare.read-sides` counts the whole sides, asks the merge of the whole sides, and compares the working tree from
  the merge base in **Since they diverged**, with an acceptance line and a test for each; the desktop's CHANGELOG says
  it.

## The views on the engine's shapes (stage 6)

Stage 6 made the state and the details of a commit the engine's types in the views, and `Repo` read them in the same
shapes. The views show what they showed; one difference of the engine's answer is kept out of them, and is not counted
in the summary, since no user sees it.

### V1. The order of the uncommitted changes

- **Feature**: `details.read-commit` with `uncommitted`; extension `Repo.commitDetail(WORKTREE)`.
- **Extension**: the files in conflict first, then those of the index, of the working tree, and the untracked ones,
  each once, in the order `git status` gives them.
- **Engine**: the same files, sorted by path.
- **Verdict**: the view's order is kept. The details of the uncommitted changes are made from the working tree of the
  state, with or without the engine (`uncommittedDetail` of `extension/src/shared/state.ts`), not asked of the engine;
  `extension/test/engineReads.test.ts` checks that the two hold the same files.

## Reads of the graph (stage 7)

Stage 7 asked the three reads of the graph of version 1.5 of [the protocol](02-protocol.md#the-graph) of the engine and
of `Repo` against the same throwaway repositories, the rows with the lines laid out from them, page after page, in
every way the graph can be shown: the branches shown, the first parents, the history of a file, a folder or some lines,
a branch alone and the searches shown as a list (`extension/test/engineGraph.test.ts`). They answer the same but for
R8.

### R8. Two stashes made in the same second on one commit

- **Feature**: `graph.place-stashes`, by `graph.read-commits`; extension `Repo.withStashes`.
- **Extension**: the stashes are sorted by date, newest first, and each is put above the commits it is not older than;
  two of the same second keep the order of `git stash list`, so the second goes above the first: `stash@{1}` above
  `stash@{0}` (`extension/src/git/repo.ts`, `withStashes`).
- **Engine**: `stash@{0}` above `stash@{1}`, as the card says, "the newest first" (`model::graph::place_stashes`).
- **Verdict**: **CHANGELOG**: the engine is right and wins (D45). A second is what git keeps of the date, so this is
  only seen with stashes made by a script or one right after the other. The line of the extension's CHANGELOG is the
  one of stage 7 on the graph read by the engine.

## Found by the end-to-end suite once the areas were merged

The areas of stage 4 were built side by side, each without the end-to-end suite, which starts VS Code. Run on the
branch that joins them, with the engine, it found what follows.

### E1. A failed step the plan gets past is logged as an error

- **Feature**: `log.record`, with `conflicts.continue-operation` and `cherry-pick.skip-empty`; extension
  `skipEmptyPicks` in `recover` (`extension/src/git/ops.ts:1219-1240`).
- **Extension**: what git said when a cherry-pick it was asked to continue changes nothing is logged as a warning, and
  `git cherry-pick --skip` runs after it.
- **Engine**: the same steps, but what git said stayed a `Failure` of the log once `recover` got past it
  (`engine/src/drive.rs`, `recover`), which the extension shows as an error.
- Verdict: **Bug (engine)**, fixed: `CommandLog::got_past_failure` makes it a warning when a recovery or an operation
  offered in its place ran; acceptance line and test `log_record__a_failure_the_plan_got_past_is_written_as_a_warning`.

### E2. Renaming the main worktree, and Open in worktree on the branch checked out here

- **Feature**: `worktree.rename`, `worktree.open-in-worktree`; extension `worktreeRename`, `openInWorktree`
  (`extension/src/extension/actions.ts`).
- **Extension**: git's error, shown as an error (*cannot move the main worktree*), and an error of the extension's
  own for a branch checked out in this window.
- **Engine**: refused before git runs, shown as a warning, as every refusal of the engine is.
- Verdict: **CHANGELOG**. "Renaming the main worktree, and **Open in worktree** on the branch checked out in this window,
  are refused with a warning before git runs."

### E3. What a worktree has already is not said when ignored files are brought into it

- **Feature**: `worktree-setup.bring-into-worktree`; extension `bringIgnored` (`extension/src/git/worktreeSetup.ts:292`,
  `307`).
- **Extension**: logs *node_modules is in the worktree already.* for each folder or file the worktree has.
- **Engine**: plans only what the worktree lacks, and says nothing of the rest.
- Verdict: **CHANGELOG**. "Bringing ignored files into a worktree lists only what it brings, not what the worktree had
  already."

### E4. The main worktree let go of in VS Code's Git before its rename is refused

- **Feature**: `worktree.rename`; extension `Controller.letGoInVSCodeGit` (area 5).
- **Extension, with the engine**: VS Code's Git extension closed the worktree before `feature/run`, so that Windows
  lets it move; for the main worktree, which the engine then refuses to move, the repository stayed closed in VS Code's
  Git, and its diffs could no longer be served.
- Verdict: **Bug (extension)**, fixed: the main worktree is never closed, since git neither moves nor removes it.

### E5. The questions of adding a submodule, and when the folder is checked

- **Feature**: `submodule.add`; extension `addSubmodule` (`extension/src/extension/actions.ts:961`, area 5).
- **Extension**: titles each box *Add submodule*, asks in its message, refuses a folder that is not empty while it is
  typed, and says where the folder goes as it changes.
- **Engine**: asks in the desktop's words, with the question in the title (*URL of the repository, or its path on
  disk*, *Folder to clone it in, from the root of the repository*), checks while typed only that the folder is inside
  the repository and has no submodule, refuses a folder that is not empty once it is answered, and says where it goes
  once, for the name proposed.
- Verdict: **CHANGELOG**. "Adding a submodule asks in the words of Gitardo's desktop, and says that a folder is not
  empty once it is entered rather than while it is typed."

## AI and GitHub (stage 4, area 6)

Area 6 asked VS Code's language models and its sign-in to GitHub through the engine (`host/languageModel`,
`host/languageModels`, `host/githubToken`). `extension/test/engineAi.test.ts` asks each question with a fake model as
the extension's TypeScript asked it and as the engine asks it: the draft of a commit message is the same request,
cleaned the same; the question about a conflict sends the same parts of the file and makes the same proposal of the
same answer; the model chosen and the effort reach the model the same. What differs follows.

### G1. The words of the question about a conflict

- **Feature**: `ai.conflict-question`, `ai.ask-conflict`; extension `buildConflictPrompt`
  (`extension/src/shared/conflictPrompt.ts`).
- **Extension**: its own task and form ("when they cannot, choose the one…", "Reply in exactly this form", "what you
  kept"), and the sides named from `CONFLICT_SIDES` with the subjects of `Repo.conflictSubjects`.
- **Engine**: the task and form of `rust/crates/model/src/ai/question.rs`, and the sides named by
  `conflicts.side-names`, the same words. The parts between tags, `<before>`, `<base>`, `<current>`, `<incoming>` and
  `<after>`, are the same, and so is the reading and trimming of the answer.
- **Verdict**: the engine wins (D45); nothing a user sees but what the model answers.

### G2. The choices of the model that resolves conflicts

- **Feature**: `ai.choose-model`; extension `chooseConflictModel` (`extension/src/extension/ai.ts`).
- **Extension**: **The first model of VS Code**, then each model with *vendor · family*, the one set marked
  `$(check)`; then **As the model is set up in VS Code** and the five efforts.
- **Engine**: the desktop's picks under a host whose models they are (`uses_host_models`): **The first model of the
  provider**, each model with its id, the one set first rather than marked; **As the model is set up**. It writes the
  two settings through the follow-up `writeSetting`, which the extension writes in the user's settings as
  `gitardo.conflicts.aiModel` and `gitardo.conflicts.aiEffort`, as it did.
- **Verdict**: **Choice**, decided: the engine's words, *to be confirmed by the maintainer*. Recommended later: under a
  host the first choice says **The first model**, without naming the provider. CHANGELOG: "Choosing the model that
  resolves conflicts offers the models as Gitardo Desktop does."
- **Settled in M48**: the engine's picks are kept (D90), and under a host the first choice says **The first model**
  (`model_question`, `rust/crates/features/src/ai/choose_model.rs`); the line is in the extension's CHANGELOG.

### G3. The model the draft of a commit message is asked of

- **Feature**: `commit.draft-message`; extension `draftCommitMessage`.
- **Extension**: VS Code's first model, whatever `gitardo.conflicts.aiModel` says, with no effort.
- **Engine**: the desktop asks the model of `ai.model` with `ai.effort`, which the extension sends from
  `gitardo.conflicts.aiModel` and `gitardo.conflicts.aiEffort` (Q400).
- **Verdict**: **Choice**, decided: under the extension the engine asks VS Code's first model with no effort
  (`model` and `effort` null in `host/languageModel`), as the extension did, since the setting is named for the
  conflicts and keeps its meaning ([stored data](../extension/11-data.md)). *To be confirmed by the maintainer*,
  with Q400: a `gitardo.ai.model` of its own if the draft is to have one.

### G4. Backing out of signing in to GitHub

- **Feature**: `github.load-pull-requests`; extension `Controller.refreshPulls` with `prompt`.
- **Extension**: VS Code's sign-in closed fails `getSession`, and Gitardo says "Gitardo: could not read pull
  requests." with VS Code's error.
- **Engine**: the host gives no token, and the feature backs out without a word; the desktop's box that asks for a
  token never shows under the extension, whose sign-in takes its place.
- **Verdict**: **CHANGELOG**. "Closing GitHub's sign-in that **Sign in to GitHub** opens says nothing."

### G5. Refusals of the engine around GitHub and the models

- **Feature**: `compare.open-on-github`, `ai.choose-model`.
- **Extension**: a comparison whose two sides are the same commit opened GitHub's page anyway; with no model,
  choosing one said "Gitardo: no language model is available in VS Code…".
- **Engine**: "The two sides are the same commit." as a warning, and nothing opened; with no model, "Gitardo: No
  language model is available in VS Code…", the extension's words given by the host.
- **Verdict**: **CHANGELOG**. "Opening on GitHub a comparison of a commit with itself says the two sides are the same
  commit."
- **Settled in M48**: the engine's refusals are kept (D90); the line is in the extension's CHANGELOG.

### G6. Plans with no step confirmed under `gitardo.commandPreview` set to `all`

- **Feature**: `ai.choose-model`, `github.load-pull-requests`, `compare.open-on-github`; `preview::needs_confirmation`
  (`rust/crates/engine/src/preview.rs`).
- **Extension**: none of the three asked anything more than its picks.
- **Engine**: each is a plan with no step and no danger, which the engine asks to confirm when every plan is to be
  shown (`all`), with an empty list of commands; with the default, it asks nothing.
- **Verdict**: **Choice**, open. Recommended: a plan with no step and no danger is never confirmed, on both apps; an
  engine change, not an option.
- **Settled in M48**, as recommended (D90): `needs_confirmation` takes the plan, and one with no danger, no step and no
  setting for the host to write is never confirmed, under `all` too, in both apps (`rust/crates/engine/src/preview.rs`).
  `ai.choose-model` under a host writes its two settings through follow-ups, so its plan is `Agreed`: its picks are the
  agreement, and nothing more is asked.

## Stage 10 (`src/git` removed)

What the extension still did in TypeScript when stage 10 began, against what the engine does in its place.

### X1. A change of the Flow Diff in a file with unsaved edits

- **Feature**: `diff.stage-change`, `diff.unstage-change`.
- **Extension**: the patch of the change was made from the text in the editor, saved first.
- **Engine**: the change is sent with its lines and their text (`Target::DiffChange`), and the engine compares them with
  the file on disk: a file with unsaved edits is refused with *The file changed since the diff was shown. Try again.*
- **Verdict**: **CHANGELOG**. "Staging or unstaging a change in the Flow Diff of a file with unsaved edits says the
  file changed since the diff was shown; save it first." Recommended to confirm.

### X2. What the extension does without the engine

- **Feature**: every one.
- **Extension**: ran its own TypeScript operations and reads (D49).
- **Engine**: none to run; the views say **Gitardo's engine is not available on this system.** with **Show the log**,
  and a command that needs the engine shows *Gitardo: Gitardo's engine is not available on this system.* with **Show
  the log**.
- **Verdict**: **CHANGELOG**, as D49 decided.

### X3. A checkout of a reference of another kind than the menu said

- **Feature**: `branch.checkout`, `tag.checkout`, `branch.checkout-remote`.
- **Extension**: an action naming, say, a tag as a branch fell back to `checkoutOp`, which checked out whatever git
  took the name for.
- **Engine**: `engineRuns` gives no feature for it, and nothing runs. No menu posts such an action.
- **Verdict**: **Choice**, recommended as built: nothing runs.

### X4. The commit message drafted from the subjects of the last commits

- **Feature**: `commit.draft-message`.
- **Extension**: took the subjects of the commits the graph showed, filtered or not.
- **Engine**: takes the last 10 subjects of the history, whatever the graph is filtered by.
- **Verdict**: **Choice**, recommended as built: the request is the engine's, as Gitardo Desktop sends it.

### X5. Smaller ones

- **Messages after a squash merge and a split**: with the engine they have no **Show changes** and **Undo** buttons,
  which the TypeScript code gave; **Undo** in the toolbar takes them back. Seen in the code, not confirmed in a window.
- **Failures** read *Gitardo: \<the engine's words\>*, not *\<title\> failed. \<git's line\>*, unless files are left in
  conflict.
- **Continue at a split with files of the commit left** (`split.continue`): the extension asked in a dialog with
  **Commit What Is Left** and **Go Back to Committing**; the engine's pick, whose detail is one line, is shown as a
  quick pick with the question as its title and **Commit what is left** first (`src/engine/vscode.ts:102`). Same
  choices, same default. Found by the end-to-end suite.
- **A refusal of the engine**, such as **Continue** at an edit stop with changes not committed (`rebase.edit-stop`), is
  a warning, *Gitardo: \<the reason\>*, where the extension showed an error.

### X6. The stashes read while the last one goes

- **Feature**: `stash.read-list`, and `repo.read-state`, which reads the stashes with the rest.
- **Extension**: did not show it; its TypeScript reads ran at other times.
- **Engine**: when the last stash is dropped, or a pull takes back the changes it set aside, while the stashes are read,
  `refs/stash` is gone between git's check and its read: `git stash list` fails with `bad revision 'refs/stash'`, and
  the read failed with it, logged as an error. Found by the end-to-end suite.
- **Verdict**: **Bug (engine)**, fixed in stage 10: such a read has no stash (`read_stashes` in
  `rust/crates/git/src/read/repo/stashes.rs`).

## Summary

Entries marked **Merged into …** are not counted again.

| Area | Bug (extension) | Bug (engine) | Choice | CHANGELOG | Total |
| --- | --- | --- | --- | --- | --- |
| Changes and commit (A1–A7) | 1 | 2 | 1 | 3 | 7 |
| Stashes (A8, A9, A12; A10, A11, A13 merged) | 1 | 0 | 0 | 2 | 3 |
| Branches and tags (B2, B3, B5, B6, B10; B1, B4 merged) | 1 | 0 | 2 | 2 | 5 |
| Remotes, sync and identity (B7–B9) | 0 | 1 | 1 | 1 | 3 |
| Merge, rebase, cherry-pick, revert, reset (C1, C2, C4; C3 merged) | 0 | 0 | 0 | 3 | 3 |
| Rewriting (C5–C8) | 0 | 1 | 0 | 3 | 4 |
| Bisect and patches (C9) | 0 | 0 | 0 | 1 | 1 |
| Conflicts (D1–D6) | 0 | 3 | 1 | 2 | 6 |
| Undo, preview and log (D7–D12) | 0 | 2 | 1 | 3 | 6 |
| Worktrees and their setup (W1–W12; W11 merged) | 1 | 5 | 2 | 3 | 11 |
| Submodules (S1–S6) | 1 | 4 | 0 | 1 | 6 |
| Reads (R1, R6, R7; R2–R5 not seen by users) | 0 | 1 | 1 | 1 | 3 |
| Reads of the graph (R8) | 0 | 0 | 0 | 1 | 1 |
| Found by the end-to-end suite (E1–E5) | 1 | 1 | 0 | 3 | 5 |
| AI and GitHub (G2–G6; G1 not seen by users) | 0 | 0 | 3 | 2 | 5 |
| Stage 10 (X1–X6; X5 counted once) | 0 | 1 | 2 | 3 | 6 |
| **Total** | **6** | **21** | **14** | **34** | **75** |

Of the 19 engine bugs, five are in the extension too, so whichever side wins they get fixed in the engine: A2 (Unstage
all during an operation), B7 (remote branch by its short name), D2 (Skip without asking), D7 (Undo after what it cannot
take back), W9 (empty folders after a failed rename). Several others are engine code doing less than its own card
says: S2 (with A10 and B1), W1, W2, S3, S6, and in part A3, C6, W10.

The five differences that most need a decision, in order:

1. **D7, Undo after an operation it cannot take back** (both sides): it takes back the one before, silently. Proposed:
   the journal also records "the last plan cannot be undone" until HEAD moves, and `undo.last` refuses with that reason.
2. **D8, the engine's Undo does not refuse while a merge, rebase, cherry-pick or patch is stopped**: the extension
   refuses ("Finish or abort the … first."); the engine refuses only during a bisect
   (`rust/crates/features/src/undo/last.rs:29-33`) and plans resets from the reflog, or `git reset --keep` of a series
   that leaves the sequencer behind. Proposed: refuse as the extension does, except for a squash merge or a resolution.
3. **W4, where the settings of new worktrees live**: the extension reads and writes `gitardo.worktrees.*` in VS Code's
   settings; the engine reads and writes `.gitardo/settings.json` and its own user file. The rule on stored data means
   the engine needs an option for the host to supply these settings and where to write them.
4. **A5, reverting one change of a diff**: the engine writes the file; the extension edits VS Code's document, keeping
   Ctrl+Z and unsaved edits. Proposed: the host applies the text, the engine only checks; the host must be able to take
   the new text instead of a `Write` step.
5. **D11, what the log keeps**: the engine logs neither git's output under a successful step nor the standard input,
   both of which the extension's log has and users read (push hints, pull and autostash messages). Proposed: log them,
   at least at debug; an engine change, not an option.

Close behind: W5 (who answers trust for the setup commands inside VS Code: an engine option recommended), D2 (Skip
throws away what was resolved, asked by neither side), B5 (the journal's Undo of a deleted branch or dropped stash is
asked as a warning; Confirm recommended).

## Pairs compared and found identical

### Changes and commit; stashes

- `changes.stage` / `stage` (one path; `git add -- <pathspec>`), and `conflicts.mark-resolved` / `stage` on a file in
  conflict: same command, `:(literal)` rule the same (`extension/src/git/ops.ts:77-79`, `rust/crates/model/src/changes.rs:20-27`).
- `changes.unstage` / `unstage`: `git restore --staged`, or `git rm --cached -r` without HEAD, both names of a rename.
- `changes.discard` / `discard` and `discardChanges` with one file; `changes.discard-all` / `discardAll` and
  `discardChanges` (header and row menu): the plans, warnings (but A3), danger and the refusal "Abort the … to discard
  all of it." are the same. The known item "discard keeping staged changes" is not a difference users see: the views
  of the extension already send `discardChanges` (staged too) whenever something of the file, or of the list, is staged
  (`extension/src/webview/ChangesApp.tsx:281`, `329`), which is the engine's plan; the unstaged-only `discard` and
  `discardAll` are only sent when nothing is staged, and the Flow Diff's unstaged-only discard maps to
  `diff.discard-file`. No engine operation is missing.
- `commit.create` / `commit`: the Stage all question, `git add --all` then `git commit -m <trimmed>`, refusals.
  `commit.amend` / `commit` with `amend`: `--amend -m` or `--amend --no-edit` (but A7).
- `fixup.commit` / `fixup`: commands and confirmation (but A6).
- `diff.stage-file`, `diff.unstage-file` / Flow Diff `fileAction` stage and unstage.
- `diff.stage-change`, `diff.unstage-change` / Flow Diff `chunkOp`: `git apply --cached --unidiff-zero
  --whitespace=nowarn -`, an untracked file added whole, a file not in HEAD unstaged whole, UTF-8 and byte order mark
  refusals (`extension/src/flow/chunks.ts:93-170`; `rust/crates/features/src/diff/stage_change.rs`,
  `unstage_change.rs`, `diff/change.rs:19-23`).
- `merge-view.mark-resolved` / Flow Merge `markResolved`: the markers question with the same words, save, `git add`,
  close (`extension/src/flow/merge.ts:153-170`; `rust/crates/features/src/merge_view/mark_resolved.rs:24-75`).
- `stash.push`, `stash.push-staged`: commands, prompts (but A9).
- `stash.apply`, `stash.pop`, `stash.drop`: commands, notes, confirm/destructive (but A11, A12).
- `stash.undo-drop` / `restoreStashOp`: `git stash store -m <message> <hash>`, confirmed.
- `stash.autostash-left` / the warning after an autostash pull: same rule and words
  (`extension/src/extension/controller.ts:1612-1620`; `docs/features/08-stashes.md:367-390`).

### Branches, tags, remotes, sync, identity

Plans, questions, danger, refusals and undo match for these pairs. Where something differs, it is only a title, a
plain wording, or a message where the extension was silent.

- **Checkouts:**
  - `branch.checkout`: `checkout` with `target: 'branch'`.
  - `branch.checkout-detached`: `target: 'detached'` with a hash.
  - `tag.checkout`: `target: 'detached'` with a tag. Both use `--detach` and the ref named in full when it has a
    namesake.
  - `branch.checkout-remote`: `--track`; the fast-forward `checkout <name>` and `merge --ff-only`; the question with
    the same labels and **rebase** first; "checked out already, with everything … has". When **as it is** is chosen on
    a branch checked out, the engine says "`<name>` is checked out already." where the extension says nothing.
  - `branch.checkout-remote-and-rebase` (`rebase --autostash <remote> <name>`). The engine also refuses when the branch
    is checked out or only behind; the menu offers it in neither case.
- **Branches:**
  - `branch.rename` (`git branch -m`; the old name counts as taken).
  - `branch.delete`: the `merge-base --is-ancestor refs/heads/<name>` read against the upstream or `HEAD`; `-d` with
    Confirm and the same text, `-D` with Destructive and the same text.
  - `branch.fast-forward`: `fetch <remote> refs/heads/<x>:refs/heads/<y>`, or
    `-C <worktree> pull --ff-only --no-rebase --no-autostash`; the same refusal text.
  - `branch.clean-up`: `branch --merged HEAD`; the same candidates, ticks, order (prune, worktree remove with `--force`
    when dirty, `-d`, `-D`, empty folders) and danger texts. The engine writes "N files not committed" where the
    extension writes its own count of changes.
- **Tags:** `tag.create` (`tag -a <name> -m <message> <commit>` or lightweight); `tag.delete` (Destructive, same
  text); `tag.push` (`push <remote> refs/tags/<name>`, with `origin` first).
- **Fetch and pull:**
  - `sync.fetch` (`fetch --all --prune`, the no-remote refusal offering **Add remote…**).
  - `sync.pull`: the default is `--rebase --autostash` on both sides, each setting leaving out its option. The
    autostash-left warning has the same rule in `stash.autostash-left` and `controller.ts:1612-1620`.
  - `sync.pull-with`: the four ways, the default one marked.
- **Push:**
  - `sync.push`: upstream, `<branch>:<upstream>`, `--set-upstream` to `origin` or the first remote, `refs/heads/`
    when the name has a namesake, the detached-HEAD refusal. The pull request is offered after the first push.
  - `sync.push-force-with-lease` and `sync.push-force`: the flag before `--set-upstream`, Destructive, and the 10
    commits overwritten with "…and more". With no upstream, or a gone one, the engine still warns ("This replaces …
    whatever it has") where the extension had no note.
- **Remotes:** `remote.manage`, `remote.add` (`origin` proposed when there is no remote), `remote.fetch`
  (`fetch --prune <name>`), `remote.set-url`, `remote.rename` (nothing runs when nothing changed; the engine says so),
  `remote.remove` (Destructive, same text).
- **Identity:** `identity.switch` and `identity.use-global` (`config user.name`, `user.email`, `user.signingkey`, or
  `--unset-all user.signingkey` when the profile has no key; unset in the order name, email, signingkey; the same
  refusal when nothing is local).
- **Found in stage 4** (area 2), by the scenario tests and the e2e steps made to hold on both paths, with the same
  commands: the engine joins what **Clean up branches** says of a branch with commas (**merged into main, with its
  worktree merged**) where the extension wrote ` · `; it writes **This repository has no remote.** and **Add remote…**
  where the extension wrote **this repository has no remote.** and **Add Remote…**; when git refuses to clone a
  submodule from a folder on disk after a checkout, it asks to run the rest of the checkout again under the checkout's
  own title (**Check out `<branch>` from a folder on disk?**) where the extension asked **Update the submodules from a
  folder on disk?**; and it refuses a detached checkout of the commit or tag checked out already, which the menus do
  not offer.
- **Undo after operations that do not move `HEAD`** (push, fetch, tags, rename, fast-forward): on both sides Undo
  falls back on the reflog and takes back the operation before. The extension cards mark this as **Differs**; it is not
  a difference between the two implementations (`rust/crates/features/src/undo/last.rs:30-76`).

### Merge, rebase, cherry-pick, revert, reset, rewriting, bisect, patches

These pairs matched in plan, questions, danger, refusals and undo, apart from wording of titles. In stage 4 (area 3)
each was run both ways on twin repositories by `extension/test/engineHistory.test.ts`, which found them the same once
C4, C6 and the bisect answer were fixed; the titles that differ are in the extension's CHANGELOG (*Cherry-pick 2
commits onto main*, *as 1 commit*), and so are the refusals said as warnings with git's reasons in the message.

- `merge` default and `mode: 'ask'` against `merge.into-current` and `merge.choose-way`. Same four ways, labels and
  descriptions; the same fast-forward-only refusal before git with both messages; the same conflict-preview text
  ("No conflicts expected." or the list of 15 and "…and N more"); squash note and `git reset --merge` undo with Confirm
  (`actions.ts:615-650`, `ops.ts:308-334`; `merge/choose_way.rs`, `merge/undo_squash.rs`, `history.rs:101-130`). The
  engine qualifies a branch as `refs/heads/<name>` when a tag has its name (`merge/into_current.rs` test); the extension
  gets the ref from the menus already qualified (not re-checked).
- `rebase` (current branch) against `rebase.onto`: `git rebase <onto>`, Danger None, reflog undo.
- `reword` against `message.edit`, plan side. For HEAD, `git commit --amend --only --allow-empty -F -` with the message
  on stdin. For an older commit, `pick` plus `exec git commit --amend -F "<file>"` with `-c sequence.editor=cp … -c
  core.editor=true rebase -i --autostash <parent>|--root`. Both confirm when the commit is pushed, and both refuse the
  same cases with the same words.
- `squash` against `squash.commits` and `squash.into-previous`: the subject question with the oldest's subject as
  default; the same message joining (`ops.ts:375-382` against `model/src/squash.rs:21-34`); reword plus fixups plus
  picks with `--autostash`; Confirm; reflog undo; the same refusals.
- `split` against `split.commit`. HEAD: `git reset --mixed HEAD~`, Confirm, remembered undo `git reset --mixed <hash>`.
  Older: `edit` plus picks with `--autostash`, then `reset --mixed HEAD~`, and the `gitardo-split` marker in
  `rebase-merge`. Same refusals for a merge, the first commit and a commit off the line.
- `continue` while splitting against `split.continue`, and `continue` while stopped to edit against `rebase.edit-stop`.
  Same question (*Commit What Is Left* / *Go Back to Committing*), the same `add --all -- <files>`,
  `commit --quiet -m <message>` and `-c core.editor=true rebase --continue`, and the same refusals of other changes.
  The engine also refuses while files are in conflict, which git would refuse anyway.
- `cherryPick` against `cherry-pick.commit`: `cherry-pick [-x] [-m 1] <hash>`, a preview against `<hash>^` except for
  a merge, and empty picks skipped.
- `cherryPickMany` against `selection.cherry-pick`: oldest first with one command, `-x` by the setting, a merge
  refused, and the series undone with `git reset --keep <head before>`, carried through continue
  (`controller.ts:1674-1678`, `1718-1726`; `selection/cherry_pick.rs`, `conflicts/continue_operation.rs:45-72`,
  `undo/journal.rs:60-63`).
- `reset` against `reset.to-commit`: Soft, Mixed or Hard with the same descriptions and no default; Hard is
  danger/Destructive with the same note.
- `bisect` (first mark) against `bisect.start`: `git bisect start` then `git bisect <mark> <hash>`; marks with a hash
  against `bisect.mark`.
- `applyPatch` against `patch.apply`: from a file (same filters) or the clipboard through a temporary file;
  `am --3way` or `apply --3way`, with "As commits" first or default; `git apply --check` first, with the same refusal;
  the same staged-changes refusal for `am`; the conflict preview; the series undo for `am`. Applying to the working
  tree has nothing of its own for Undo on either side. The engine marks it `Undo::Impossible`, but only the app's
  `can_undo` reads that (`app/src/work/operations.rs:207`).

### Conflicts; undo, preview and log

- `conflicts.abort-operation` / `abort`: `git <operation> --abort`, destructive with the same sentence, nothing to
  undo, the series forgotten (`git/ops.ts:1261-1268`; `features/src/conflicts/abort_operation.rs:24-38`). Only the
  refusal with no operation says *patch* where the extension says *am* (`conflicts.rs:63`, `actions.ts:115`).
- `conflicts.continue-operation` / `continue`: the command `git -c core.editor=true <operation> --continue`
  (`ops.ts:1210`; `continue_operation.rs:51-55`), the hand-over to the edit and split stops (`actions.ts:601-602`;
  `continue_operation.rs:32-36`), the skip of an empty cherry-pick or revert, and a series carried for Undo
  (`controller.ts:1718-1726`; `continue_operation.rs:45-49`, `65-72`).
- `cherry-pick.skip-empty` conditions: no unmerged files and the files as `HEAD`, the same warning words
  (`ops.ts:1242-1254`; `skip_empty.rs:12-14`, `33-37`).
- `conflicts.take-side` / `takeSide`: the commands (`checkout --ours|--theirs -- <file>` then `add`, or `rm`), the
  titles and notes, the refusal *<file> is not in conflict.*, Undo by restoring the conflicts only when both sides
  were there (`ops.ts:1184-1206`, `controller.ts:1710-1712`; `take_side.rs:43-135`).
- `conflicts.restore` / `restoreConflictsOp`: `git checkout --merge -- <files>`, destructive with the same sentence
  (`ops.ts:1174-1181`; `restore.rs:193-216`).
- `conflicts.resolve-automatically` / `resolveConflicts`: the setting refusal with **Open settings**, the two other
  refusals, the **Resolve / Review** question, the files staged when left with no conflict, the report with
  **Continue** (`controller.ts:1637-1642`, `1755-1767`, `ops.ts:1110-1133`; `resolve_automatically.rs:34-82`,
  `200-230`). One wording: for a single conflict, *The conflict cannot be resolved…* against *None of the 1 conflict…*.
- `review.write`: the refusal *No resolution is accepted.*, the changed-file refusal wording, `Danger::None`, Undo by
  restoring the conflicts of the files written (`ops.ts:1147-1171`, `controller.ts:557-562`; `review/write.rs:22-98`).
- `conflicts.stopped` after a failure: the same text, **Resolve automatically** / **Resolve with AI** / **Show log**
  and the same conditions (`controller.ts:2495-2521`; desktop card `09-conflicts.md:152-178`; matched by the card,
  the view code not read).
- `undo.last` by the reflog: checkout back (detached when from a hash) with the submodule update, `--soft` after a
  commit or amend, `--mixed` when there are changes and the files match, `--keep` otherwise, the branch's own reflog
  when one is checked out, untracked files not counted (`repo.ts:1053-1071`, `actions.ts:394`; `undo/last.rs:40-147`).
- The journal records: branch deleted, stash dropped, conflicts resolved, squash merge (also after a stop), series,
  split of the last commit (`controller.ts:1676-1726`, `ops.ts:327-334`, `969-999`; `undo/journal.rs:39-68`).
- `preview.confirm`: the table (`off` asks for danger or confirm, `all` asks for everything) and a warning for
  danger (`controller.ts:2424-2427`; `engine/src/preview.rs:7-17`).

### Worktrees and their setup; submodules

- `worktree.create`: questions in the same order (format, start, name), `git worktree add --no-track -b <name> <folder>
  <start>`, Danger None, the folder check at run time, **Choose What It Gets…** offered once.
- `worktree.create-from-branch`: `git worktree add <folder> <branch>` by the short name, or `--no-track -b` with the
  format; folder question defaulting to the branch.
- `worktree.create-at-commit`: `--no-track -b <name>` or `--detach` with the folder named after the short hash.
- `worktree.open-in-worktree`: opens the existing worktree, refuses the branch checked out here and a missing worktree
  with the same words, `--track -b` for a remote branch with no local branch; nothing asked.
- `worktree.remove`: questions (keep or delete the branch, keep by default), Confirm notes, `branch -d` after, Destructive
  for `--force`, links to outside folders unlinked on Windows only, the held-folder recovery of `git worktree remove`.
- `worktree.rename`: refusals (main, folder of the window), folder name rules inside and outside the folder of the
  worktrees, the branch-rename question and its proposed name, the held-folder retry of `git worktree move`.
- `worktree.prune`: `git worktree prune`; the engine's refusal with no missing worktree matches the extension's "Offered
  when".
- `worktree-setup.link-and-copy-steps` and `command-steps`: links first, then copies, then commands; a failed link or
  copy does not stop the rest; opening offered when a command fails.
- `worktree-setup.bring-into-worktree` / `bring-into-other-worktrees`: Confirm, same refusals and wording.
- `submodule.add`: the three questions, `git submodule add [-b <branch>] -- <url> <folder>` then `git submodule update
  --init --recursive -- <folder>`, the non-empty folder refused, Danger None.
- `submodule.update-to-recorded`: plan and the Confirm when commits on a detached HEAD would be lost or changes exist.
- `submodule.update-to-latest`: `update --init --remote`, then `-C <folder> submodule update --init --recursive`.
- `submodule.stage-recorded`: `git [-C <parent>] add -- <path>`.
- `submodule.deinitialize`: Confirm, then `deinit --force` offered at once as Destructive on `local modifications`.
- `submodule.remove`: `deinit --force`, `rm --force`, the clone deleted only inside `modules`, Destructive.
- `submodule.sync-urls`: `git submodule sync --recursive`.
- `submodule.update-on-checkout` on the checkouts: the note, the Confirm only for commits on no branch, the file-protocol
  offer.

## Pairs not reached

### Changes and commit; stashes

- `stageHunk`, `unstageHunk`, `discardHunk` (`hunkOp`): not compared in detail, since no view posts them (map rows
  1a-30 to 1a-32); their live counterparts are covered above.
- `changes.save` against `Controller.save`: host-only saving, not compared.
- The engine's `stash.preview` read (`Query::Stash`) against `Repo.stashFiles` and `conflictsWith(ref, ref^1)`: compared by
  what is said, not by the git commands of the read.

### Branches, tags, remotes, sync, identity

- The submodule update itself (`submodule.update-on-checkout` and its confirmation when commits would be left behind)
  was compared only as far as the checkout plans hand over to it. `model::submodule::update_on_checkout` was not read.
- The reflog Undo of a pull or checkout (`undo.last` reset modes) belongs to the undo area.
- `github.offer-after-push` and `stash.autostash-left` were compared from their desktop cards
  (`docs/features/15-github.md:187-209`, `docs/features/08-stashes.md:367-395`), not from the app code that carries out
  `FollowUp::CheckAutostash`. In stage 4 (area 2) the extension does both itself after a fetch, a pull, a push or
  **Sync** on either path, from the state read before and after (`Controller.afterSync`), since the server does not
  send `checkAutostash`.
- `Validation::Url` of `remote.add` was not compared line by line with the extension's URL check
  (`controller.ts:1946`).

### Merge, rebase, cherry-pick, revert, reset, rewriting, bisect, patches

- The interactive rebase editor's own rules (`rebase.editor`, `rebase.todo`) and drag-and-drop on the graph
  (`rewrite.drag-and-drop`). **Reached in stage 4** (area 3) as far as the rows go: the extension's editor sends its
  steps, which `todoRows` in `extension/src/engine/actions.ts` writes as the rows of `rebase.interactive`. A reword
  that gives a whole message, which the rows cannot say, still runs as the extension's `GitOp`. The editor's own rules
  and the drag-and-drop are the host's and were not compared.
- Whether `undo.last` prefers the journal over the reflog when both have something (it affects C3; it belongs to the
  undo area). **Reached in stage 4** (area 3): it does, for a squash merge, a series of cherry-picks, a patch of
  commits and a split of the last commit (`extension/test/engineHistory.test.ts`).
- The view-side checks of the extension's message box (C7) and the commit-box fill after a split
  (`reportSplit`, `controller.ts:1701`, against `FollowUp::FillCommitMessage`). **Reached in stage 4** (area 3): the
  box cancels an unchanged message (C7), and `fillCommitMessage` gives the box the whole message of the commit split,
  as `reportSplit` did.

### Conflicts; undo, preview and log

- Whether the window offers **Undo** beside **Continue** after `conflicts.resolve-automatically` (the plan's `after`
  has only `Say` and `Offer Continue`, `resolve_automatically.rs:211-230`; the view was not read).
- `review.write`'s refusal of two resolutions over the same lines (`review/write.rs:36-44`) against
  `applyReplacements` in the extension (`src/git/conflicts.ts`, not read).
- Whether `resolveConflicts` with `paths` (a subset of files, `actions.ts:546-548`) is posted anywhere; the engine
  always takes every file in conflict.
- `conflicts.mark-resolved` against `stage` on a file in conflict beyond the plan (`git add -- <file>`, the same): the
  several-paths case of **Stage all** is the changes area's.
- The default button of the confirmation (the desktop makes **Run** the default only when not destructive,
  `16-undo-preview-log.md`, `preview.confirm`); VS Code's modal was not checked.
- Checked in stage 4 (area 4): the extension keeps **Undo** beside **Continue** on the message after resolving
  automatically, a button the engine's plan does not offer; two resolutions over the same lines are refused by the
  engine's sentence (*Two resolutions accepted in … take the place of the same lines, so nothing was written.*)
  where the extension said *Two resolutions are for the same lines of the file.*, which only an error of the review
  can reach; `resolveConflicts` with `paths` is sent only by the e2e suite, and runs in the engine when the paths are
  every file in conflict, in TypeScript otherwise; a file in conflict staged from the Changes view runs
  `conflicts.mark-resolved`, the same `git add`.

### Worktrees and their setup; submodules

- The worktrees removed inside `branch.clean-up` (`cleanupBranchesOp`, `extension/src/git/ops.ts:267-291`, which uses
  `--force` for a dirty worktree and `worktree prune` for a missing one) against `features/branch/clean_up.rs`.
- `worktree-setup.remove-from`, `worktree-setup.explain-change` and the line-by-line rules of `give.rs` against
  `changeSetup` (`extension/src/git/worktreeSetup.ts:490`).
- What the patterns take: the extension computes the folders and files after `git worktree add` against the new
  worktree (`extension/src/git/worktreeSetup.ts:266-313`), the engine before it against the worktree's commit
  (`features/worktree_setup/new_worktree.rs:56-71`); not compared case by case.
- The validation messages of names and folders while typed (`WorktreeNameRule`, `Validation::SubmoduleFolder`).

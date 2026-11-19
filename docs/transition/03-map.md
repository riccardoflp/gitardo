# The map: what the extension does, and what replaces it in the engine

Every entry point of the extension that changes something, and every read of git it makes, with the feature or the
read of the engine that takes its place, as the code was on 2026-11-14. This is the list the stages of the
[roadmap](06-roadmap.md) work through, area by area; the differences of behavior between the two sides of a row are in
[04-differences.md](04-differences.md).

Paths that start with `src/` or `test/` are in `extension/`; those that start with `crates/` are in `rust/`. The
extension's cards are in [docs/features/extension/](../features/extension/README.md), the desktop's in
[docs/features/](../features/README.md). The functions of `OPERATIONS` (`crates/features/src/lib.rs:57-253`) are in
`crates/features/src/<area>/<name>.rs`; a path is written only where it is not that.

Match: **same** (the desktop feature does what the extension does), **partial** (the desktop covers part; the row says
what is missing or extra), **none** (no desktop feature).

Exceptions to the `<area>/<name>.rs` rule, found with `grep "pub fn <name>("`:
`commit-page.open` is `commit_page.rs`; `commits.copy-hash`/`copy-message` are `commits.rs`; `reflog.browse` is
`reflog.rs`; `settings.theme` is `settings.rs`; `github.load-pull-requests` is `github.rs`; `compare.change-side` is
`compare/with.rs`; `compare.with-current` and `compare.selected-before` are `compare/in_place.rs`; the four
`submodule.open-in-new-window`/`open-in-terminal`/`reveal`/`show-in-gitardo` are `submodule/open.rs`;
`submodule.copy-path`/`copy-url` are `submodule/copy.rs`; `worktree-setup.bring-*` are `worktree_setup/bring.rs`; the
four `worktree-setup.link-into-new-worktrees`/`copy-into-new-worktrees`/`stop-linking`/`stop-copying` are
`worktree_setup/give.rs`; `worktree-setup.edit-settings-file` is `worktree_setup/edit.rs`;
`worktree-setup.remove-from` is `worktree_setup/remove.rs`; `rebase.edit-stop` is `rebase/continue_edit.rs`;
`split.continue` is `split/continue_split.rs`.

## 1. Operations

### 1a. The 61 kinds of `Action`

`Action` is the union at `src/shared/protocol.ts:488-586`; each kind is a `case` of `resolveAction` in
`src/extension/actions.ts:229-609`, run by `Controller.action` (`src/extension/controller.ts:1632-1730`).

| # | Action kind (protocol.ts line; actions.ts case) | What it does | Desktop feature(s) | Match |
| --- | --- | --- | --- | --- |
| 1 | `checkout` (493; 233) | Checks out a branch, a remote branch in the local branch of its name (opening the worktree that has it instead, `actions.ts:697-702`), or a commit or tag detached; then updates the submodules (`updatingSubmodules`, `actions.ts:883`) | One to many: `target: 'branch'` → `branch.checkout`; `'remote'` → `branch.checkout-remote`; `'remote'` with `rebase: true` (menus.ts:412) → `branch.checkout-remote-and-rebase`; `'detached'` with a hash → `branch.checkout-detached`; `'detached'` with a tag ref (`src/webview/menus.ts:434`) → `tag.checkout` | same |
| 2 | `createBranch` (494; 238) | Asks the name (with the format of branch names), create only or create and check out | `branch.create` (with `branch.format-name`, `branch.name-from-format`) | same |
| 3 | `renameBranch` (495; 250) | `git branch -m` | `branch.rename` | same |
| 4 | `deleteBranch` (496; 255) | `-d`, or `-D` when not merged; fallback `-D` on "not fully merged" (`src/git/ops.ts:255`); undo by `restoreBranchOp` | `branch.delete` (its card has the same **Delete anyway** fallback) | same |
| 5 | `fastForward` (498; 262) | Fetch into a branch not checked out, or `pull --ff-only` in the worktree that has it (`ops.ts:172-193`) | `branch.fast-forward` (same two plans) | same |
| 6 | `deleteRemoteBranch` (499; 265) | `git push <remote> --delete` | `remote.delete-branch` | same |
| 7 | `merge` (501; 268) | Merge as git's settings say, or `mode: 'ask'` for the four ways (incl. squash), with conflict preview (`actions.ts:615-650`) | `merge.into-current` (default); `merge.choose-way` (the four ways); preview is `merge.read-predicted-conflicts` | same |
| 8 | `rebase` (503; 271) | Rebase the current branch, or `branch` when given | `rebase.onto`; with `branch` → `rebase.branch-onto` | same |
| 9 | `rebaseInteractive` (505; 274) | Runs the todo list of the rebase editor (`interactiveRebase`, `actions.ts:899`) | `rebase.interactive` | same |
| 10 | `reword` (510; 277) | Amend for HEAD, interactive rebase for an older commit, warns when pushed | `message.edit` | same |
| 11 | `squash` (515; 301) | Several commits into one by interactive rebase; also "Squash into the commit before it" (`menus.ts:210-211`) | `squash.commits`; from the one-commit menu entry → `squash.into-previous` | same |
| 12 | `split` (521; 331) | Reset for HEAD, rebase stopped at `edit` for an older one | `split.commit` | same |
| 13 | `undo` (522; 387) | Reflog-based undo (`undoOp`, `ops.ts:1001`); before that, what Gitardo remembered (`controller.ts:1648-1651`, see rows 1b-18 to 1b-22) | `undo.last` (reflog and journal); the remembered ones also → `stash.undo-drop`, `conflicts.restore`, `merge.undo-squash`. One to many | same |
| 14 | `worktreeAdd` (524; 397) | On a new branch from a base asked, for a branch, or at a commit; then link/copy/commands of the setup (`withSetup`, `actions.ts:799`) | `worktree.create` (no `ref`), `worktree.create-from-branch` (`isBranch`), `worktree.create-at-commit` (a commit); setup is `worktree-setup.link-and-copy-steps` / `command-steps` | same |
| 15 | `worktreeOpen` (529; 430) | Opens in a new window, or in VS Code's Agents window (`openWorktree`, `actions.ts:731-743`) | `worktree.open`, `worktree.open-in-new-window`. `where: 'agents'` has no desktop counterpart (host-only); the desktop has `worktree.open-in-editor` instead | partial |
| 16 | `openInWorktree` (531; 433) | Opens the worktree of a branch, making it first | `worktree.open-in-worktree` | same |
| 17 | `worktreeRemove` (533; 400) | `git worktree remove`, branch too if wanted, `--force` fallback (`ops.ts:805`), unlink folder links, close it in VS Code's Git first (`closeInVSCodeGit`) | `worktree.remove` (with `worktree.recover-held-folder`, `worktree.remove-empty-folders`). Closing it in VS Code's Git is host-only | same |
| 18 | `worktreeRename` (534; 423) | `git worktree move`, and the branch if wanted (`renameWorktree`, `actions.ts:1461`) | `worktree.rename` | same |
| 19 | `worktreeTerminal` (534; 435) | VS Code integrated terminal in the folder; also used for submodules (`menus.ts:544`) | `worktree.open-in-terminal`; on a submodule → `submodule.open-in-terminal` | same |
| 20 | `worktreeReveal` (534; 438) | `revealFileInOS`; also used for submodules (`menus.ts:545`) | `worktree.reveal`; on a submodule → `submodule.reveal` | same |
| 21 | `worktreePrune` (535; 425) | `git worktree prune` | `worktree.prune` | same |
| 22 | `worktreeSetUp` (537; 427) | Brings ignored files into one worktree or every other one (`setUpExistingOp`, `actions.ts:832`) | `worktree-setup.bring-into-worktree` (`path`), `worktree-setup.bring-into-other-worktrees` (none) | same |
| 23 | `submoduleAdd` (539; 442) | Asks URL, folder, branch; file-protocol fallback (`ops.ts:859`) | `submodule.add` (with `submodule.allow-file-protocol`) | same |
| 24 | `submoduleUpdate` (544; 444) | Recorded commit, cloning first when not initialized; `remote` for the latest of its branch | `submodule.update-to-recorded`; not initialized (`menus.ts:548`) → `submodule.initialize`; `remote` → `submodule.update-to-latest` | same |
| 25 | `submoduleRecord` (546; 448) | Stages the submodule's commit | `submodule.stage-recorded` | same |
| 26 | `submoduleDeinit` (548; 453) | `git submodule deinit`, `--force` fallback (`ops.ts:935`) | `submodule.deinitialize` | same |
| 27 | `submoduleRemove` (550; 455) | Removes folder, `.gitmodules` entry and clone | `submodule.remove` | same |
| 28 | `submoduleSync` (552; 475) | `git submodule sync --recursive` | `submodule.sync-urls` | same |
| 29 | `submoduleOpen` (554; 477) | Opens the submodule folder in a new VS Code window | `submodule.open-in-new-window` | same |
| 30 | `stageHunk` (555; 484) | `git apply --cached -` of a `git diff` hunk (`hunkOp`, `ops.ts:680`) | `diff.stage-change`. No view posts this action (extension card `changes.stage-hunk`, `docs/features/extension/05-changes-and-commit.md:233-235`); the live counterpart is the Flow Diff's `chunkOp` (row 1b-28). Desktop builds a zero-context patch from the Diff page only | partial |
| 31 | `unstageHunk` (555; 485) | `git apply --cached --reverse -` | `diff.unstage-change`; same remark as row 30 | partial |
| 32 | `discardHunk` (555; 486) | `git apply --reverse -` on the working tree | `diff.revert-change`, which edits and saves the file instead of running git; no view posts the action | partial |
| 33 | `cherryPick` (556; 362) | One commit, conflict preview, `-x` per `gitardo.cherryPick.recordOrigin`, skips empty picks in `recover` (`skipEmptyPicks`, `ops.ts:1219`) | `cherry-pick.commit`; the automatic skip is `cherry-pick.skip-empty` | same |
| 34 | `cherryPickMany` (558; 368) | Several selected commits, oldest first | `selection.cherry-pick` | same |
| 35 | `applyPatch` (560; 384) | From a file or the clipboard, `git am --3way` or `git apply --3way`, checked first | `patch.apply` (with `patch.check`, `patch.count-commits`) | same |
| 36 | `revert` (561; 489) | `git revert` (`-m 1` for a merge) | `revert.commit` | same |
| 37 | `reset` (562; 492) | Soft, mixed or hard, asked | `reset.to-commit` | same |
| 38 | `createTag` (563; 501) | Lightweight or annotated | `tag.create` | same |
| 39 | `deleteTag` (564; 511) | `git tag -d` | `tag.delete` | same |
| 40 | `pushTag` (565; 514) | `git push <remote> <tag>` | `tag.push` | same |
| 41 | `stashPush` (567; 517) | Everything, only staged, or some files, with an optional message | One to many: none given → `stash.push`; `staged` → `stash.push-staged`; `paths` → `stash.push-file` | same |
| 42 | `stashApply` (568; 531) | With the preview of what git would stop on (`withStashPreview`, `actions.ts:134`) | `stash.apply` (with `stash.preview`) | same |
| 43 | `stashPop` (568; 533) | Same, then drop | `stash.pop` | same |
| 44 | `stashDrop` (568; 535) | Drop; undo remembered as `restoreStashOp` | `stash.drop` (undo is `stash.undo-drop`) | same |
| 45 | `stashBranch` (568; 538) | `git stash branch` | `stash.branch` | same |
| 46 | `stage` (569; 544) | `git add -- <paths>`; on a file in conflict it is **Mark as resolved** (`src/webview/ChangesApp.tsx:244`); several paths when files are in conflict and **Stage all** names the others (`ChangesApp.tsx:201-206`) | `changes.stage`; on a file in conflict → `conflicts.mark-resolved`. Desktop's card is one file per tick | same |
| 47 | `unstage` (569; 554) | `git restore --staged` or `git rm --cached` before the first commit | `changes.unstage` | same |
| 48 | `discard` (569; 558) | Unstaged side only: `git restore` / `git clean -f`, staged part kept (`discardOp`, `ops.ts:616`) | `diff.discard-file` has the same plan but only from the Diff page; `changes.discard` (Changes page) runs this plan only while nothing of the file is staged, and takes the staged part too otherwise | partial |
| 49 | `resolveConflicts` (571; 546) | Writes the rule resolutions, after Resolve/Review (`confirmWrites`, `controller.ts:1748`) | `conflicts.resolve-automatically` (with `conflicts.resolve-by-rules`) | same |
| 50 | `takeSide` (573; 550) | Whole file from ours or theirs, or delete it as that side did | `conflicts.take-side` | same |
| 51 | `stageAll` (574; 552) | `git add --all` | `changes.stage-all` | same |
| 52 | `unstageAll` (574; 556) | `git reset` / `git rm --cached -r .` | `changes.unstage-all` | same |
| 53 | `discardAll` (574; 560) | The unstaged side of every unstaged file, staged changes kept | `changes.discard-all` from the header with nothing staged has this plan; with something staged it takes the staged changes too. The extension's view switches to `discardChanges` in that case (`ChangesApp.tsx:281`), so the UI behaves the same, but no desktop operation discards only the unstaged side of all files while something is staged | partial |
| 54 | `discardChanges` (579; 565) | Back to the last commit, staged too, untracked deleted; all without `paths` (`discardChangesOp`, `ops.ts:641`) | `paths` of one file → `changes.discard`; several or none → `changes.discard-all` (header plan, or the draft row plan `-- .`) | same |
| 55 | `commit` (580; 570) | `git commit -m`, asks to stage all when nothing is staged; `amend` | `commit.create` (same Stage all question); `amend: true` → `commit.amend` | same |
| 56 | `fixup` (582; 587) | `commit --fixup` then `rebase -i --autosquash --autostash` | `fixup.commit` | same |
| 57 | `bisect` (584; 594) | Marks good/bad/skip, starting a bisect on the first mark (`bisectOp`, `ops.ts:1049`) | One to many: no bisect yet → `bisect.start`; with `hash` → `bisect.mark`; without `hash` (banner, `BisectBanner.tsx:13`) → `bisect.answer` | same |
| 58 | `bisectReset` (585; 596) | `git bisect reset` | `bisect.end` | same |
| 59 | `continue` (586; 600) | `--continue` with `core.editor=true`; a rebase stopped to edit refuses uncommitted changes (`refuseUncommitted`, `actions.ts:656`); a split commits what is left (`continueSplit`, `actions.ts:667`); skips empty picks | `conflicts.continue-operation`; stopped to edit → `rebase.edit-stop`; stopped to split → `split.continue`; empty pick → `cherry-pick.skip-empty`. One to many | same |
| 60 | `skip` (586; 604) | `<operation> --skip` | `conflicts.skip-step` (and `cherry-pick.skip-empty` in `recover`) | same |
| 61 | `abort` (586; 606) | `<operation> --abort` | `conflicts.abort-operation` | same |

### 1b. Controller and Flow methods that build and run a `GitOp` outside `resolveAction`

Found with `grep "this\.run(\|runIn(\|Op("` over `src/extension` and `src/flow`. `Controller.run` is at
`controller.ts:2420`; `runIn` (`controller.ts:2414`) is handed to the Flow Diff and Flow Merge in
`src/extension/extension.ts:38`.

| # | Extension entry (file:line) | What it does | Desktop feature(s) | Match |
| --- | --- | --- | --- | --- |
| 1 | `Controller.sync('fetch')` (`controller.ts:1587`, `syncOp` `ops.ts:108`) | `git fetch --all --prune` | `sync.fetch` | same |
| 2 | `Controller.pull()` (`controller.ts:1583`) | The pull of the settings (`gitardo.pull.rebase`/`autostash`); warns about a kept autostash (`controller.ts:1612-1620`) | `sync.pull` (with `stash.autostash-left`) | same |
| 3 | `Controller.sync('pull' / 'pullRebase' / 'pullAutostash' / 'pullRebaseAutostash')` (`ops.ts:109-113`) | One of the four pulls | `sync.pull-with` | same |
| 4 | `Controller.sync('push')` (`ops.ts:115-132`) | Push, `--set-upstream` for a first push; offers a pull request after it (`controller.ts:1622-1626`) | `sync.push` (with `github.offer-after-push`) | same |
| 5 | `Controller.sync('pushForceWithLease')` | Push `--force-with-lease`, with the commits overwritten (`commitsOnlyIn`, `controller.ts:1601`) | `sync.push-force-with-lease` (with `sync.read-overwritten`) | same |
| 6 | `Controller.sync('pushForce')` | Push `--force` | `sync.push-force` | same |
| 7 | `Controller.manageRemotes` (`controller.ts:1933`) | The list of remotes and the menu of one | `remote.manage` | same |
| 8 | `manageRemotes` → `remoteAddOp` (`controller.ts:1957`) | `git remote add` | `remote.add` | same |
| 9 | `manageRemotes` → `fetchRemoteOp` (`controller.ts:1979`) | `git fetch --prune <remote>` | `remote.fetch` | same |
| 10 | `manageRemotes` → `remoteSetUrlOp` (`controller.ts:1984`) | `git remote set-url` | `remote.set-url` | same |
| 11 | `manageRemotes` → `remoteRenameOp` (`controller.ts:1991`) | `git remote rename` | `remote.rename` | same |
| 12 | `manageRemotes` → `remoteRemoveOp` (`controller.ts:2001`) | `git remote remove` | `remote.remove` | same |
| 13 | `Controller.switchIdentity` → `identityOp` (`controller.ts:2105`, `2114`) | `git config user.name/email/signingkey` | `identity.switch` | same |
| 14 | `Controller.switchIdentity` → `clearIdentityOp` (`controller.ts:2118`) | `git config --unset-all user.*` | `identity.use-global` | same |
| 15 | `Controller.cleanupBranches` (`controller.ts:1476-1524`) | Deletes merged or gone branches with their worktrees | `branch.clean-up` | same |
| 16 | `Controller.takeCompareFile` (`controller.ts:1239-1250`, `takeFileOp` `ops.ts:977`) | The file of the second side of a comparison into the working tree and index | `compare.take-file` | same |
| 17 | `Controller.writeResolutions` (`controller.ts:557-562`), from `ConflictReviewTab.write` (`src/extension/conflictReview.ts:182-197`, `writeResolutionsOp` `ops.ts:1147`) | Writes the accepted resolutions, stages files with no conflict left | `review.write` | same |
| 18 | `Controller.action` remembered undo: `restoreBranchOp` (`controller.ts:1704`, `ops.ts:969`) | `git branch <name> <hash>` after a delete | `undo.last` (journal entry "a branch deleted in Gitardo") | same |
| 19 | remembered undo: `restoreStashOp` (`controller.ts:1705`, `ops.ts:997`) | `git stash store` after a drop | `stash.undo-drop` (and `undo.last`) | same |
| 20 | remembered undo: `restoreConflictsOp` (`controller.ts:560`, `1711`, `1714`; `ops.ts:1174`) | `git checkout --merge -- <files>` | `conflicts.restore` (and `undo.last`) | same |
| 21 | remembered undo: `undoSquashOp` (`controller.ts:1682`, `ops.ts:327`) | Takes back a squash merge | `merge.undo-squash` (and `undo.last`) | same |
| 22 | remembered undo of a series (`controller.ts:1676-1678`, `1718-1726`) | `git reset --keep` to before a cherry-pick of several or `git am` | `undo.last` (journal) | same |
| 23 | `Controller.runWithFallback` (`controller.ts:1736-1742`) | Runs `op.fallback` when git refuses in its way: `branch -D` (`ops.ts:255`), `worktree remove --force` (`ops.ts:805`), file protocol for a submodule (`ops.ts:859`), `deinit --force` (`ops.ts:935`) | Inside `branch.delete`, `worktree.remove`, `submodule.allow-file-protocol`, `submodule.deinitialize` | same |
| 24 | `Controller.action` runs `ran.then` (`controller.ts:1688`): `submodulesUpdateOp` (`ops.ts:901`) after a checkout | Updates the submodules | `submodule.update-on-checkout` (Derivation, not in `OPERATIONS`) | same |
| 25 | Flow Diff `fileAction('stage')` (`src/flow/flowDiff.ts:128`) | `git add` of the file shown | `diff.stage-file` | same |
| 26 | Flow Diff `fileAction('unstage')` (`flowDiff.ts:129`) | Unstage the file shown | `diff.unstage-file` | same |
| 27 | Flow Diff `fileAction('discard')` (`flowDiff.ts:131`) | `discardOp` of the file shown | `diff.discard-file` | same |
| 28 | Flow Diff `stageChunk` → `chunkOp('Stage')` (`src/flow/chunks.ts:93-110`, `157`, `165`); whole file `stageOp` for an untracked one (`chunks.ts:98`) | `git apply --cached --unidiff-zero` | `diff.stage-change` (same command, same untracked case) | same |
| 29 | Flow Diff `unstageChunk` → `chunkOp('Unstage')` (`chunks.ts:117-144`), `unstageOp` when HEAD lacks the file (`chunks.ts:129`) | Patch from index towards HEAD | `diff.unstage-change` | same |
| 30 | Flow Merge `markResolved` → `stageOp` (`src/flow/merge.ts:153-170`) | Saves and stages the file, closes the page | `merge-view.mark-resolved` | same |

### 1c. Other extension entry points that desktop operations replace

These change no repository through a `GitOp` (they open, copy, write settings, or save documents), but the desktop
models them as operations of `OPERATIONS`, so they are listed to place every one of the 157.

| # | Extension entry (file:line) | What it does | Desktop feature(s) | Match |
| --- | --- | --- | --- | --- |
| 1 | Flow Diff `revertChunk` (`src/flow/chunks.ts:55-90`) | Puts the left side's lines back through a `WorkspaceEdit` and saves | `diff.revert-change` | same |
| 2 | `save` message → `Controller.save` (`controller.ts:2399`, `1861-1869`) | Saves the dirty VS Code editors of some files | `changes.save` (Gitardo's own editors) | same |
| 3 | `copy` message (`controller.ts:2408`; posted from `menus.ts:28`, `Detail.tsx:249`, `CommitApp.tsx:91`) | Clipboard | `commits.copy-hash`, `commits.copy-message`, `worktree.copy-path`, `submodule.copy-path`, `submodule.copy-url` | same |
| 4 | `Controller.openCommit` (`controller.ts:820`) | The commit in a tab of its own | `commit-page.open` | same |
| 5 | `Controller.goToCommit` (`controller.ts:912`), `goTo` message (`controller.ts:803`) | Asks a revision, selects it | `search.go-to-commit` (and `search.go-to-hash`, View) | same |
| 6 | `Controller.compare` (`controller.ts:963`) | Asks both sides, or only the other with `from` (`menus.ts:73-75`) | `compare.choose`; with `from` → `compare.with` | same |
| 7 | `Controller.compareWith` (`controller.ts:992`) | A file or folder of the working tree with a revision | `compare.path-with` | same |
| 8 | `getComparison` message (`controller.ts:2381`; `App.tsx:130-134`, `menus.ts:65-69`, `menus.ts:217`) | Comparison in the details pane | `compare.with-current`; "selected before" → `compare.selected-before` | same |
| 9 | `Controller.pickCompareSide` (`controller.ts:1217`) | Another side in the tab | `compare.change-side` | same |
| 10 | `Controller.copyComparePatch` (`controller.ts:1253`) | The comparison as a patch to the clipboard | `compare.copy-patch` | same |
| 11 | `Controller.openCompareOnGitHub` (`controller.ts:1298`) | The comparison on github.com | `compare.open-on-github` | same |
| 12 | `Controller.compareFile` (`controller.ts:1016`) | A file with a revision in VS Code's diff editor | `details.compare-file-with` (in the Diff page) | same |
| 13 | `Controller.formatPatch` (`controller.ts:1269`) | Commits as one patch, to the clipboard or a file | `patch.copy`; `save` → `patch.save` | same |
| 14 | `Controller.browseReflog` (`controller.ts:1446`) | Pick a reflog entry, then show, branch, reset, check out or cherry-pick it | `reflog.browse` | same |
| 15 | `fileHistory` / `folderHistory` / `lineHistory` (`controller.ts:653`, `710`, `671`) | Narrow the graph by `setFilter` | `file-history.file`, `file-history.folder`, `file-history.lines` | same |
| 16 | `gitardo.toggleFileBlame` and the like → `FileBlame.toggle` (`extension.ts:185`, `src/extension/fileBlame.ts:200`) | Blame beside every line | `blame.toggle-file` | same |
| 17 | `gitardo.toggleBlame` → setting `blame.enabled` (`extension.ts:172`) | Current-line blame | `blame.toggle-current-line` | same |
| 18 | setting `blame.ignoreWhitespaceAndMoves` (`extension.ts:175`) | `-w -M -C` in blame | `blame.ignore-whitespace` | same |
| 19 | setting `fileBlame.everyFile` (`extension.ts:178`) | Blame in every file | `blame.every-file` | same |
| 20 | `FileBlame.openLine` (`extension.ts:180`, `fileBlame.ts:230`) | Open the commit of the line | `blame.open-line-commit` | same |
| 21 | `FileBlame.revealLine` (`extension.ts:181`, `fileBlame.ts:236`) | Show it in the graph | `blame.reveal-line-commit` | same |
| 22 | `FileBlame.previous` (`extension.ts:182`, `fileBlame.ts:245`) | Blame before that commit | `blame.previous` | same |
| 23 | `refreshPulls` / `signInGitHub` (`controller.ts:1548`, `1392`; `src/extension/github.ts:8-21`) | Pull requests through VS Code's GitHub sign-in | `github.load-pull-requests`, which asks for a token and keeps it (`github.token`); the sign-in itself is host-only | partial |
| 24 | `chooseConflictModel` (`src/extension/ai.ts:66`) | Picks a VS Code language model | `ai.choose-model` (a model of the AI provider, and how much it thinks) | partial |
| 25 | `Controller.switchWorktree` (`controller.ts:1411`) | Quick pick of the worktrees | `worktree.switch` | same |
| 26 | `Controller.selectRepo` (`controller.ts:2018`) to a worktree or a submodule | The views point at it | `worktree.show-in-gitardo`; for a submodule → `submodule.show-in-gitardo`; another repository is `repo.select` (Operation card, not in `OPERATIONS`) | same |
| 27 | **Show in the graph** → `reveal` message (`menus.ts:247`, `controller.ts:2343`) on a worktree's commit | Selects it | `worktree.show-in-graph` (and `commit-page.show-in-graph`, View) | same |
| 28 | `saveWorktreeSetup` message → `saveSetup` (`controller.ts:2288`, `src/extension/worktreeSettings.ts:102-120`) | Writes the patterns and commands; `patterns: null` removes them | `worktree-setup.save`; `null` → `worktree-setup.remove-from`. `alsoVSCode` (writing `git.worktreeSymlinkFolders`) is host-only | same |
| 29 | `editWorktreeSetup` message (`controller.ts:2302`) | Opens the settings JSON | `worktree-setup.edit-settings-file` | same |
| 30 | `Controller.setUpFromExplorer` (`controller.ts:1052`; commands at `extension.ts:206-213`) | From the Explorer context menu: link, copy, stop | `worktree-setup.link-into-new-worktrees`, `worktree-setup.copy-into-new-worktrees`, `worktree-setup.stop-linking`, `worktree-setup.stop-copying` (from the Files page; `worktree-setup.explain-change`). The Explorer entry is host-only | partial |
| 31 | `Controller.stashFromScm` (`controller.ts:743`; `extension.ts:257-259`) | Stash from VS Code's Source Control lists | `stash.push-file`, `stash.push-staged`, `stash.push`. The entry from VS Code's list is host-only | partial |
| 32 | `Controller.setFavorite` (`controller.ts:627`) | Favorite branches in the workspace state | `refs.favorite` (View card, not in `OPERATIONS`) | same |

## 2. Reads

### 2a. The public methods of `Repo` (`src/git/repo.ts:136-1088`)

Callers found with `grep "\.<method>("` over `src` (the preview server `src/dev/server.ts` is left out unless it is
the only caller). Desktop reads are in `crates/git/src/read/`; engine queries are the `Query` arms of
`crates/engine/src/queries.rs:22-77`.

| Repo method (line) | What it reads | Called from | Desktop read (card) | Match |
| --- | --- | --- | --- | --- |
| `commonDir` (167) | `--git-common-dir` | `controller.ts:1914` (watch), `2051` (`worktreeOwner`, Repositories view) | No `read_*`; the app reads the `commondir` file itself (`crates/app/src/work/repository_watch.rs:79-89`, `repo.watch-changes`) | partial |
| `gitDirAt` (173) | Git folder of a submodule or of the repository | `actions.ts:354`, `459`, `918` | `submodule::read_git_folder` (`submodule.rs:166`, `Query::GitFolder`) | same |
| `hasSubmodulesAt` (178) | `.gitmodules` at a revision | `actions.ts:885` | `branch::read_has_file` (`branch.rs:122`, `Query::HasFile`; `submodule.update-on-checkout`) | same |
| `lineAfter` (187) | Commits after one up to HEAD, no merges | `actions.ts:288`, `351`, `949`; `controller.ts:1663` | `rewrite::read_line_after` (`rewrite.rs:42`), `rebase::line::read_line` (`rebase/line.rs:26`) (`rebase.find-line-before-head`) | same |
| `remoteBranchesWith` (201) | Remote branches containing a commit | `actions.ts:928` | `rebase::read_pushed` (`rebase.rs:36`, `rebase.read-pushed`) | same |
| `stashFiles` (208) | Files a stash changes, its untracked files | `actions.ts:141` | `stash::files::read_files` (`stash/files.rs:41`, `stash.read-files`) | same |
| `isMergedForDelete` (221) | Merged into HEAD or upstream | `actions.ts:258` | `branch::read_is_ancestor` (`branch.rs:49`, `Query::IsAncestor`, asked by `crates/features/src/branch/delete.rs:48`) | same |
| `canFastForwardTo` (227) | HEAD is an ancestor of ref | `actions.ts:618` | `branch::read_is_ancestor` (asked by `merge/choose_way.rs:43`) | same |
| `squashMessage` (232) | `SQUASH_MSG` | `controller.ts:1681` | No `read_*`; the app reads `SQUASH_MSG` itself (`crates/app/src/work/operations.rs:180-189`) | partial |
| `commitMessage` (238) | Full message | `actions.ts:305`, `683`, `907`; `controller.ts:1788` | `rewrite::read_message` (`rewrite.rs:61`), `rebase::line::read_messages` (`rebase/line.rs:112`) | same |
| `working` (243) | Status of the working tree | `flowDiff.ts:131`, `142`; inside `state`, `commitDetail` | `changes::read_working_tree` (`changes.rs:26`, `changes.read-working-tree`) | same |
| `ignored` (248) | Which unsaved files git ignores | `controller.ts:1856` | `ignored::read_ignored` (`ignored.rs:15`, for `changes.unsaved`) | same |
| `operation` (268) | Merge/rebase/cherry-pick/revert/am in progress | `conflictReview.ts:116`, `controller.ts:532`, `548`, `flow/merge.ts:242`, `467`; inside `state` | `repo/operation.rs` inside `read_state` (`repo.read-state`) | same |
| `rebaseStop` (289) | Where an interactive rebase stopped | inside `state` (`repo.ts:659`) | `rebase::read_stop` (`rebase.rs:90`, `rebase.read-stop`) | same |
| `changedPaths` (303) | Paths a commit changed | `actions.ts:668` | `rebase::line::read_commit_files` (`rebase/line.rs:130`, `Query::CommitFiles`) | same |
| `pathNames` (310) | Names a file had | inside `selection` (`repo.ts:802`) | `file_history::read_names` (`file_history.rs:46`, `file-history.read-names`) | same |
| `conflicted` (325) | Files in conflict | `conflictReview.ts:110`, `controller.ts:525` | `conflicts::read_files` (`conflicts.rs:57`, `conflicts.read-files`) | same |
| `conflictSubjects` (334) | Subjects of the two sides | `conflictReview.ts:117`, `275` | `review::read_side_names` (`review.rs:130`, `conflicts.side-names`) | same |
| `conflictNames` (348) | Names of the two sides | `conflictReview.ts:117` | `review::read_side_names` (`conflicts.side-names`) | same |
| `conflictStages` (392) | Index stages of a file | `actions.ts:551`, `controller.ts:529`, `1658`, `2607`, `flow/merge.ts:200`, `237`, `459` | `conflicts::read_stages` (`conflicts/texts.rs:43`, `Query::ConflictStages`) | same |
| `deletedConflicts` (405) | Files deleted on one side, with the other's text | `conflictReview.ts:114` | `conflicts::read_files` (stages per side, `conflicts.rs:30-38`) and `conflicts::read_texts` (`conflicts/texts.rs:20`) (`review.deleted-card`) | same |
| `conflictPlans` (422) | Texts of files in conflict, resolved by the rules | `actions.ts:548`, `conflictReview.ts:112`; inside `state` | `conflicts::read_texts` (`Query::ConflictTexts`) plus `conflicts.resolve-by-rules` (Derivation) | same |
| `bisect` (444) | Marks and suspects of a bisect | inside `state` | `bisect::read` (`bisect.rs:33`, `bisect.read`) | same |
| `localIdentityKeys` (457) | `user.*` set in the repository | `controller.ts:2098` | `identity::read_identity` (`identity.rs:33`, `identity.read`; local keys `identity.rs:43-49`) | same |
| `resolve` (463) | Commit of a revision | `controller.ts:806`, `1105`, `1148`, `1183`, `1376`, `1381`; `chunks.ts:128`, `flowDiff.ts:129`, `flow/sides.ts:157` | `search::read_revision` (`search.rs:93`, `search.read-revision`), `compare::resolve_revision` (`compare.rs:72`, `compare.resolve-revision`) | same |
| `compare` (471) | Files between two commits | `controller.ts:2382` | `compare::read_files` (`compare.rs:96`, `compare.read-files`) | same |
| `mergeBase` (477) | Merge base | inside `compareSides` | inside `compare::sides::read_sides` (`compare/parse.rs:37`) | same |
| `compareSides` (488) | A comparison tab: base, counts, commits, cherry marks, files, conflicts | `controller.ts:1198` | `compare::sides::read_sides` (`compare/sides.rs:144`, `compare.read-sides`) | same |
| `comparePatch` (515) | Lines of a comparison | `controller.ts:1205` | `compare::patch::read_patch` (`compare/patch.rs:72`, `compare.read-patch`) | same |
| `compareText` (521) | Comparison as a `git apply` patch | `controller.ts:1255` | `readings::compare_patch` (`Query::ComparePatch`, `queries.rs:69`) for `compare.copy-patch` | same |
| `formatPatch` (530) | `git format-patch --stdout` | `controller.ts:1274` | `patch::read_format_patch` (`patch.rs:97`, `patch.read-format-patch`) | same |
| `checkPatch` (543) | `git apply --check` | `actions.ts:215` | `patch::check::check` (`patch/check.rs:91`, `patch.check`) | same |
| `submodules` (580) | Submodules and where they stand | inside `state` | `submodule::read_list` (`submodule.rs:41`, `submodule.read-list`) | same |
| `state` (641) | Everything the views show: rows, refs, working tree, stashes, worktrees, submodules, operation, bisect, identity… | `controller.ts:569-577`, `1219`, `ai.ts:23` | `repo::read_state` (`repo.rs:67`, `repo.read-state`), `graph::commits::read_commits` (`graph/commits.rs:107`, `graph.read-commits`), `worktree::read_list` (`worktree.rs:41`), `stash::list::read_list` (`stash/list.rs:20`) | same |
| `position` (834) | Index of a commit in the history listed | `controller.ts:781-789` | `graph::position::read_position` (`graph/position.rs:44`, `graph.read-position`) | same |
| `matches` (847) | Search of the whole history | `controller.ts:927` | `search::read_matches` (`search.rs:35`, `search.read-matches`) | same |
| `branchLog` (859) | The Commits view: commits of HEAD and which are unpushed | `controller.ts:2329` (`getLog` message) | No read; the desktop has no Commits view. `graph.read-commits` with `graph.unpushed` (Derivation) hold the parts | none |
| `blameLine` (884) | One line's blame | `src/extension/blame.ts:113` | `blame::read_line` (`blame.rs:46`, `blame.read-line`) | same |
| `blame` (897) | Whole-file blame | `fileBlame.ts:280`, `360` | `blame::read_file` (`blame.rs:98`, `blame.read-file`) | same |
| `mergedBranches` (909) | `git branch --merged` | `controller.ts:1479` | `branch::read_merged` (`branch.rs:64`, `Query::MergedBranches`) | same |
| `branchesContaining` (915) | Branches containing a commit | `actions.ts:1319` (`commitBase`) | `branch::read_containing` (`branch.rs:91`, `Query::Containing`) | same |
| `remoteUrl` (920) | URL of a remote | inside `remotes` (`repo.ts:927`) | `remote.rs:43-51` (`url_arguments`, `parse_url`) | same |
| `remotes` (925) | Remotes with URLs | `controller.ts:1922`, `1937` | `remote::read_remotes` (`remote.rs:30`, `remote.read-remotes`) | same |
| `diffText` (931) | Diff for the commit message draft | `ai.ts:21` | No `read_*`; the app runs the diff in `crates/app/src/work/ai.rs` (card `commit.draft-message`, Derivation, "Reads" line) | partial |
| `commitDetail` (936) | Details of a commit or stash | `controller.ts:826`, `2333`, `2337` | `details::read_commit` (`details.rs:132`, `details.read-commit`) | same |
| `commitPatch` (995) | Lines a commit changed | `controller.ts:2338` | `commit_page::read_patch` (`commit_page.rs:70`, `commit-page.read-patch`) | same |
| `hunks` (1004) | Hunks of a file's diff | No caller in `src`; only `test/awkward.test.ts:105`, `109`, `140` | `diff::read_sides` with `diff.change-patch` would replace it | none |
| `conflictsWith` (1027) | `git merge-tree` prediction | `actions.ts:154`, `365`, `646`; inside `compareSides` | `merge::read_predicted_conflicts` (`merge.rs:35`, `merge.read-predicted-conflicts`) | same |
| `divergence` (1037) | Ahead/behind | `actions.ts:641`, `703`; inside `compareSides` | `branch::read_counts` (`branch.rs:17`, `Query::Counts`) | same |
| `commitsOnlyIn` (1043) | Commits a force push overwrites | `controller.ts:1601` | `sync::read_overwritten` (`sync.rs:44`, `sync.read-overwritten`) | same |
| `reflog` (1048) | Reflog entries | `controller.ts:558`, `1449`, `1644`, `1697`, `1723`; inside `undoInfo` | `reflog::read_entries` (`reflog.rs:26`, `reflog.read`) | same |
| `undoInfo` (1053) | What Undo takes back from the reflog | `actions.ts:391` | `undo.last` reads the reflog (`Query::Reflog`) and `Query::FilesMatch` (`crates/features/src/undo/last.rs:43-143`); `undo.journal` | same |
| `committedLines` (1077) | Lines as the commit checked out has them | `controller.ts:691` | `file_history::read_committed_lines` (`file_history.rs:151`, `file-history.read-committed-lines`) | same |
| `show` (1085) | A file at a revision | `content.ts:61`, `fileBlame.ts` via content, `flow/merge.ts:205`, `465`, `repo.ts:411`, `433`, `1080` | `file_view::read_revision` (`file_view.rs:34`, `file-view.read-revision`); `diff::read_sides` (`diff.rs:69`, `diff.read-sides`); `merge_view::read_sides` (`merge_view.rs:50`, `merge-view.read-sides`) | same |

The field `firstParent` (`controller.ts:568`) is a setting of `state`; the desktop has `graph.first-parent`.

### 2b. Other reads of the extension

| Extension read (file:line) | What it reads | Desktop read (card) | Match |
| --- | --- | --- | --- |
| `fetchPulls` (`src/extension/github.ts:13-21`, query `src/git/github.ts:25-85`) | Open pull requests, GraphQL | `crates/engine/src/github.rs` (`github.read-pull-requests`) | same |
| `githubSession` (`src/extension/github.ts:8`) | VS Code GitHub authentication | `github::read_token` (`crates/git/src/read/github.rs:37`, `github.token`) and the engine's credential store | partial (host-only sign-in) |
| `parseGitHubRemote` (`src/git/github.ts:9`), `detectGitHub` (`controller.ts:1921`) | Which remote is on GitHub | `github.detect-remote` (Derivation) | same |
| `BlameAnnotator.update` (`src/extension/blame.ts:98-113`) | Current-line blame | `blame::read_line` (`blame.read-line`, view `blame.current-line`) | same |
| `FileBlame.read` / `underCursor` (`fileBlame.ts:358`, `271`) | File blame | `blame::read_file` (`blame.read-file`) | same |
| `GitFileSystem` (`src/extension/content.ts`) | `gitardo:` files, as bytes, for VS Code's editors, its preview of images and its diff editor (revision, index `:0`, stages `:1-3`, proposals) | `file_view::read_revision`, `diff::read_sides`, `merge_view::read_sides`; the URI scheme is host-only | same |
| `readVersion` (`src/flow/sides.ts:91-118`) | Sides of a Flow Diff | `diff::read_sides` (`diff.read-sides`) | same |
| Flow Merge stages (`flow/merge.ts:199-205`, `458-465`) | Base/ours/theirs | `merge_view::read_sides` (`merge-view.read-sides`) | same |
| `scanIgnored` (`src/git/worktreeSetup.ts:344`) | What git ignores, with sizes | `worktree_setup::ignored::read_ignored` (`worktree_setup/ignored.rs:28`, `worktree-setup.read-ignored`) | same |
| `previewSetup`, `foldersToLink`, `pathsToCopy` (`worktreeSetup.ts:437`, `69`, `112`) | What the patterns take | `worktree_setup::patterns::read_what_patterns_take` (`worktree_setup/patterns.rs:54`, `worktree-setup.read-what-patterns-take`) | same |
| `proposedCommands` (`worktreeSetup.ts:419`) | Lock files committed at the top | `worktree_setup::paths::read_committed_at_top` (`worktree_setup/paths.rs:87`, `worktree-setup.propose-commands`) | same |
| `unlinkFolderLinks` (`worktreeSetup.ts:233`) | Links to folders outside a worktree | `worktree::folder_links::read_folder_links` (`worktree/folder_links.rs:31`, `worktree.read-folder-links`) | same |
| `changeSetup` (`worktreeSettings.ts`, used at `controller.ts:1077`) | Whether a path chosen in the Explorer is tracked or ignored | `worktree_setup::paths::read_path_status` (`worktree_setup/paths.rs:68`, `Query::PathStatuses`, `worktree-setup.explain-change`) | same (not verified line by line, see Doubts) |
| `discoverRepos` (`src/git/discover.ts:40`) | Repositories of the workspace | `repo::repositories::read_repositories` (`repo/repositories.rs:37`, `repo.read-repositories`) | same |
| `draftCommitMessage` (`src/extension/ai.ts:11-23`) | Diff and last subjects for the language model | `crates/app/src/work/ai.rs` (`commit.draft-message`) | partial (model is host-only) |
| `askResolution` (`ai.ts:99`) | One conflict asked of the model | `ai.ask-conflict` (Read; `crates/engine/src/ai.rs`) | partial (model is host-only) |
| `SourceControlSelection` (`src/extension/sourceControlSelection.ts:39-198`) | VS Code Git extension's selected repository | none | none (host-only) |
| Flow refresher (`src/flow/refresher.ts:55`) | `rev-parse --absolute-git-dir` to watch | `repo/location.rs:20` (inside `read_state`), `repo.watch-changes` | same |

## 3. Gaps, extension side

Extension features with no desktop counterpart, or only a partial one.

Host-only (they make sense only inside VS Code; the engine does not need them):

- The Agents window: `worktreeOpen` with `where: 'agents'` and the offers after create (`actions.ts:722-743`, `819-822`,
  `1448-1451`); setting `gitardo.worktrees.openIn`.
- Following VS Code's Source Control Repositories view: `SourceControlSelection` (`src/extension/sourceControlSelection.ts`),
  `Controller.followSourceControl` (`controller.ts:2062`), `worktreeOwner` with `Repo.commonDir` (`controller.ts:2050`),
  and `onDidChooseRepo` (`extension.ts:44`). Extension card `repo.follow-source-control`.
- Closing a worktree in VS Code's Git extension before removing or moving it: `closeInVSCodeGit`, `refreshVSCodeGit`,
  `inVSCodeGit` (`actions.ts:1071-1131`), used by rows 1a-17, 1a-18 and 1b-15.
- Entries from VS Code's own lists: the Explorer context menu of the worktree setup (`extension.ts:206-213`, row 1c-30)
  and the Source Control stash commands (`extension.ts:257-259`, row 1c-31). The operations exist on the desktop; only
  the entry point is the host's.
- VS Code's diff editor and multi-diff editor: `openDiff`, `openChanges`, `openFile`, `openChange` messages
  (`controller.ts:2387-2398`, `2570-2620`), `compareWithSaved` (`controller.ts:2401-2403`), the Flow Diff's
  **Open in diff editor** (`flowDiff.ts:110`) and `openFromDiffEditor` (`flowDiff.ts:94`). Extension cards
  `changes.open-all`, `changes.compare-with-saved`. The desktop has its own Diff and File pages.
- VS Code's merge editor: `openConflict` with `editor: 'mergeEditor'` (`controller.ts:531-533`), Flow Merge
  `openInMergeEditor` / `leaveMergeEditor` (`flow/merge.ts:232`, `198`); the deleted-on-one-side diff of
  `openDeletedConflict` (`controller.ts:546-554`, card `conflict.open-deleted`) uses `vscode.diff`.
- VS Code language models: `draftCommitMessage`, `askResolution`, `chooseConflictModel` (`src/extension/ai.ts`). The
  desktop uses its own AI provider (`crates/engine/src/ai.rs`), so the feature is there; the model source differs.
- VS Code GitHub authentication: `githubSession` (`src/extension/github.ts:8`). The desktop reads a token instead.
- Settings written through VS Code's configuration: the `set*` messages (`controller.ts:2239-2259`), `alsoVSCode` in
  `saveSetup` (`worktreeSettings.ts:113`), the fallback to VS Code's `git.branchSortOrder` (desktop reads git's
  `branch.sort` in `repo/branch_sort.rs`).
- The `gitardo:` content provider (`content.ts`), the file icon theme and syntax theme taken from VS Code
  (`fileIconTheme.ts`, `fileIcons.ts`, `src/flow/theme.ts`), the status bar item and output channel
  (`activity.ts`, `extension.ts:92`, `253-254`).
- VS Code integrated terminal for setup commands (`src/extension/worktreeCommands.ts`); the desktop runs
  `worktree-setup.command-steps` in its own way.

Real gaps the engine would need first:

- **The Commits view** (`ViewKind 'commits'`, `protocol.ts:430`): `Repo.branchLog` (`repo.ts:859-882`) through the
  `getLog` message (`controller.ts:2327-2331`). The desktop has no read that lists the commits of HEAD with which are
  unpushed against the upstream or every remote; `graph.read-commits` plus `graph.unpushed` would have to be combined
  or a read added. (The side graph view, `'sideGraph'`, reuses the state and needs nothing new.)
- **Discard the unstaged side only**: `discard` and `discardAll` (rows 1a-48, 1a-53) keep what is staged. The desktop
  offers that plan only from a Diff page, one file (`diff.discard-file`); `changes.discard` and `changes.discard-all`
  take the staged changes too when there are some. Either the engine gets an operation "discard unstaged changes of
  these files", or the extension's Changes view adopts the desktop's semantics (a visible change, to be decided).
- **Several files at once in Changes**: the extension's `stage`/`unstage` take a list of paths (`ChangesApp.tsx:201-212`,
  to stage all but the files in conflict); the desktop cards of `changes.stage`/`changes.unstage` are one file per tick.
  How the desktop's `changes.stage-all` handles files in conflict was not checked (see Doubts).
- **Hunk actions from a `git diff` hunk** (`stageHunk`, `unstageHunk`, `discardHunk`, rows 1a-30 to 1a-32): no view posts
  them, so nothing would be lost by dropping them; the live per-change staging is the Flow Diff's, which maps to
  `diff.stage-change` / `diff.unstage-change` / `diff.revert-change`.
- **Squash message and common dir read outside the git crate** (`squashMessage`, `commonDir`): the desktop reads them in
  `crates/app`, not as reads of `crates/git` or engine queries. For the extension to use the engine, they would need
  to move into the engine (or the extension keeps reading them itself).
- **The draft of the commit message**: the desktop's diff for it is run in `crates/app/src/work/ai.rs`, not exposed as
  an engine query; the extension would need it as one, or keep `Repo.diffText`.

## 4. Gaps, desktop side

Desktop operations of `OPERATIONS` and reads with no extension counterpart: the work the extension gets later, by area.

Operations with no counterpart:

- Sync: `sync.sync` (`crates/features/src/sync/sync.rs`): fetch, pull, then push in one click, with the counts on the
  button. The extension has Fetch, Pull and Push apart (`src/webview/Toolbar.tsx:353-368`). Real work for the
  extension.
- Worktrees: `worktree.open-in-editor` (opens the worktree in `editor.external`). Inside VS Code the host is the editor;
  it would at most be "open in another editor". Probably not wanted in the extension.
- File page: `file-view.save`, `file-view.open-external`. In the extension VS Code's editors save and are the editor:
  nothing to do.
- Settings: `settings.theme` (`crates/features/src/settings.rs`). VS Code's theme applies: nothing to do.
- Trust: `trust.manage` (`crates/features/src/trust/manage.rs`), and `trust.ask` (an Operation card not in
  `OPERATIONS`, `trust/ask.rs`). VS Code's workspace trust covers it; the extension should decide whether the engine's
  trust check applies inside VS Code (see Doubts).

Desktop operations that are separate where the extension folds them into another entry (all placed in section 1, listed
here because the extension has no entry point of their own):

- `cherry-pick.skip-empty`: the extension does it inside `recover` of cherry-pick, continue and skip
  (`skipEmptyPicks`, `ops.ts:1219`), with no entry of its own.
- `stash.undo-drop`, `merge.undo-squash`, `conflicts.restore`: the extension reaches them only through **Undo**
  (`controller.ts:1648`) and the review's **Undo** button (`ConflictsApp.tsx:64`).
- `bisect.start` / `bisect.mark` / `bisect.answer`: one action `bisect` in the extension.
- `squash.into-previous`: an entry of `squash` in the extension (`menus.ts:210`).
- `rebase.edit-stop`, `split.continue`: branches of `continue` (`actions.ts:600-603`).

Desktop Operation cards not in `OPERATIONS` (for completeness): `repo.open-folder`, `repo.open-recent` (VS Code opens
folders: host), `repo.select` (extension `selectRepo`/`pickRepo`, `controller.ts:2018`, `2068`), `trust.ask`,
`log.record` (extension `Controller.run` logs, `controller.ts:2448-2453`), `activity.track` (extension
`src/extension/activity.ts`).

Reads with no extension counterpart:

- GitHub: `github::read_http_proxy` (`crates/git/src/read/github.rs:76`), the proxy for the GitHub request; and the
  token read (`read_token`, `github.rs:37`). The extension relies on VS Code's `fetch` and sign-in.
- Repository: `repo/branch_sort.rs` (git's `branch.sort`); the extension reads VS Code's setting.
- Worktrees: none beyond those mapped.
- AI: the list of models of the provider (`Query::AiModels`, answered by the engine, `queries.rs:62-64`).
- Engine-only queries with no git behind them: `Query::Clipboard`, `Query::TemporaryFile`, `Query::FileContents`,
  `Query::FileTexts`/`PathExists`/`FolderLinks`/`HasFiles` (`readings::on_disk`), `Query::Journal`. The extension does
  these with Node or VS Code APIs.

## 5. Counts

Section 1 (operations), by row:

| Table | Rows | same | partial | none |
| --- | --- | --- | --- | --- |
| 1a. `Action` kinds | 61 | 55 | 6 (`worktreeOpen`, `stageHunk`, `unstageHunk`, `discardHunk`, `discard`, `discardAll`) | 0 |
| 1b. Controller and Flow `GitOp` runners | 30 | 30 | 0 | 0 |
| 1c. Other entry points that desktop operations replace | 32 | 28 | 4 (GitHub sign-in, AI model, Explorer setup entries, Source Control stash entries) | 0 |
| Total | 123 | 113 | 10 | 0 |

Section 2 (reads), by row:

| Table | Rows | same | partial | none |
| --- | --- | --- | --- | --- |
| 2a. `Repo` public methods | 54 | 49 | 3 (`commonDir`, `squashMessage`, `diffText`) | 2 (`branchLog`, `hunks`) |
| 2b. Other reads | 18 | 14 | 3 (`githubSession`, `draftCommitMessage`, `askResolution`) | 1 (`SourceControlSelection`) |
| Total | 72 | 63 | 6 | 3 |

`OPERATIONS`: 157 identifiers. 151 are placed in section 1 as the replacement of an extension entry; 6 have no
extension counterpart and are in section 4: `sync.sync`, `worktree.open-in-editor`, `file-view.save`,
`file-view.open-external`, `settings.theme`, `trust.manage`.

## 6. Doubts

- **`discard` and `discardAll` semantics** (rows 1a-48, 1a-53): marked partial because the desktop's Changes page
  operations take staged changes too. Whether the extension's Changes view should keep its two buttons (unstaged only,
  and all) or take the desktop's one is a decision for the maintainer, not something the code decides.
- **`stageHunk`/`unstageHunk`/`discardHunk`**: the extension card says no view posts them; I did not find a poster in
  `src/webview` either (`grep stageHunk\|discardHunk src/webview` is empty). Marked partial rather than none because
  the desktop has the same user feature through the Diff page.
- **`changes.stage` for several files**: the desktop card speaks of one file per tick; I did not read
  `crates/features/src/changes/stage.rs` to see whether it takes a list, nor how `changes.stage-all` treats files in
  conflict (the extension names the other files one by one when there are conflicts, `ChangesApp.tsx:199-206`).
- **`changeSetup` against `read_path_status`**: matched from the card `worktree-setup.explain-change` and
  `Query::PathStatuses` (`queries.rs:73`); I did not read `changeSetup` in `src/extension/worktreeSettings.ts` line by
  line.
- **Trust inside VS Code**: the desktop asks before running what a folder's `.gitardo/settings.json` asks
  (`trust.ask`). The extension's settings come from VS Code, which has workspace trust of its own; whether the engine's
  check should run in the extension is open.
- **`worktreeTerminal` and `submoduleOpen`**: marked same although the extension opens VS Code's integrated terminal and
  a VS Code window, and the desktop the system terminal (`terminal.external`) and a Gitardo Desktop window. The feature
  is the same; the host decides where.
- **`commonDir`, `squashMessage`, `diffText`**: the desktop reads these in `crates/app`, outside `crates/git/src/read`
  and the engine's queries, so "partial" means "the desktop does it, but not where the extension could ask the engine
  for it".
- **`changeSetup` in 2b** is counted same though not verified line by line (see above).
- **`Repo.hunks`** is used by no code of `src`, only by `test/awkward.test.ts`: dropping it drops those tests too.
- **Desktop path of a few functions**: where a feature's function is not in `<area>/<name>.rs`, the file was found with
  `grep "pub fn <name>("` (the exceptions listed at the top); the identifier → function pairs are those of
  `crates/features/src/lib.rs:58-252`.

## Placement check

Checked with a script over this file and `crates/features/src/lib.rs`: the 61 `Action` kinds are the 61 rows of table
1a, and each of the 157 identifiers of `OPERATIONS` (157 distinct) appears in this document. 151 of them are the
replacement of an extension entry in section 1; the other 6 (`sync.sync`, `worktree.open-in-editor`, `file-view.save`,
`file-view.open-external`, `settings.theme`, `trust.manage`) are in section 4. None is left unplaced.

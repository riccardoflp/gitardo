# The catalog of features

Every feature, one card each, 151 in all. The template and the rules of the cards are in the
feature contract. This index is written by `catalog.py --index`: change a status here in the commit that
earns it, and run the script again after adding or renaming a card; the statuses are kept. A card with a
**Differs** line is *Differs*; when the line goes, it becomes *Documented*.

| File | Area | Cards |
| --- | --- | --- |
| [01-repositories.md](01-repositories.md) | Repositories | 2 |
| [06-branches-tags-remotes.md](06-branches-tags-remotes.md) | Branches, tags, remotes and sync | 24 |
| [07-merge-and-rewrite.md](07-merge-and-rewrite.md) | Merge, rebase and rewriting history | 28 |
| [09-conflicts.md](09-conflicts.md) | Conflicts | 34 |
| [12-worktrees.md](12-worktrees.md) | Worktrees | 38 |
| [13-submodules.md](13-submodules.md) | Submodules | 17 |
| [16-undo-preview-log.md](16-undo-preview-log.md) | Undo, command preview and the log | 7 |
| [18-settings.md](18-settings.md) | Settings | 1 |

## Repositories

| Feature | Title | Kind | Status |
| --- | --- | --- | --- |
| [`repo.select`](01-repositories.md#reposelect--choose-the-repository-shown) | Choose the repository shown | View | Documented |
| [`repo.follow-source-control`](01-repositories.md#repofollow-source-control--follow-the-repositories-view-of-vs-code) | Follow the Repositories view of VS Code | View | Documented |

## Branches, tags, remotes and sync

| Feature | Title | Kind | Status |
| --- | --- | --- | --- |
| [`branch.checkout`](06-branches-tags-remotes.md#branchcheckout--check-out-a-branch) | Check out a branch | Operation | Documented |
| [`branch.checkout-remote`](06-branches-tags-remotes.md#branchcheckout-remote--check-out-a-remote-branch-in-the-local-branch-of-its-name) | Check out a remote branch in the local branch of its name | Operation | Documented |
| [`branch.checkout-detached`](06-branches-tags-remotes.md#branchcheckout-detached--check-out-a-commit-or-a-tag-detached) | Check out a commit or a tag, detached | Operation | Documented |
| [`branch.name-format`](06-branches-tags-remotes.md#branchname-format--name-a-new-branch-from-a-format) | Name a new branch from a format | Derivation | Documented |
| [`branch.rename`](06-branches-tags-remotes.md#branchrename--rename-a-branch) | Rename a branch | Operation | Differs |
| [`branch.delete`](06-branches-tags-remotes.md#branchdelete--delete-a-branch) | Delete a branch | Operation | Documented |
| [`branch.delete-remote`](06-branches-tags-remotes.md#branchdelete-remote--delete-a-branch-from-its-remote) | Delete a branch from its remote | Operation | Differs |
| [`branch.favorite`](06-branches-tags-remotes.md#branchfavorite--mark-a-branch-as-a-favorite) | Mark a branch as a favorite | View | Documented |
| [`branch.cleanup`](06-branches-tags-remotes.md#branchcleanup--clean-up-branches) | Clean up branches | Operation | Differs |
| [`branch.sort`](06-branches-tags-remotes.md#branchsort--list-branches-and-tags-by-name-or-by-date) | List branches and tags by name or by date | Derivation | Documented |
| [`tag.create`](06-branches-tags-remotes.md#tagcreate--create-a-tag) | Create a tag | Operation | Differs |
| [`tag.push`](06-branches-tags-remotes.md#tagpush--push-a-tag) | Push a tag | Operation | Documented |
| [`tag.delete`](06-branches-tags-remotes.md#tagdelete--delete-a-tag) | Delete a tag | Operation | Differs |
| [`remote.manage`](06-branches-tags-remotes.md#remotemanage--list-the-remotes-and-choose-what-to-do-with-one) | List the remotes and choose what to do with one | View | Documented |
| [`remote.add`](06-branches-tags-remotes.md#remoteadd--add-a-remote) | Add a remote | Operation | Documented |
| [`remote.fetch`](06-branches-tags-remotes.md#remotefetch--fetch-one-remote) | Fetch one remote | Operation | Differs |
| [`remote.change-url`](06-branches-tags-remotes.md#remotechange-url--change-the-url-of-a-remote) | Change the URL of a remote | Operation | Documented |
| [`remote.rename`](06-branches-tags-remotes.md#remoterename--rename-a-remote) | Rename a remote | Operation | Documented |
| [`remote.remove`](06-branches-tags-remotes.md#remoteremove--remove-a-remote) | Remove a remote | Operation | Documented |
| [`sync.fetch`](06-branches-tags-remotes.md#syncfetch--fetch-every-remote) | Fetch every remote | Operation | Differs |
| [`sync.pull`](06-branches-tags-remotes.md#syncpull--pull-in-one-of-four-ways) | Pull, in one of four ways | Operation | Documented |
| [`sync.push`](06-branches-tags-remotes.md#syncpush--push-the-branch-checked-out) | Push the branch checked out | Operation | Differs |
| [`sync.push-force`](06-branches-tags-remotes.md#syncpush-force--force-push-with-or-without-a-lease) | Force push, with or without a lease | Operation | Documented |
| [`sync.fast-forward`](06-branches-tags-remotes.md#syncfast-forward--fast-forward-a-branch-that-is-not-checked-out) | Fast-forward a branch that is not checked out | Operation | Differs |

## Merge, rebase and rewriting history

| Feature | Title | Kind | Status |
| --- | --- | --- | --- |
| [`merge.into-current`](07-merge-and-rewrite.md#mergeinto-current--merge-into-the-current-branch) | Merge into the current branch | Operation | Documented |
| [`merge.choose-way`](07-merge-and-rewrite.md#mergechoose-way--choose-the-way-to-merge) | Choose the way to merge | Operation | Documented |
| [`merge.squash`](07-merge-and-rewrite.md#mergesquash--squash-merge) | Squash merge | Operation | Documented |
| [`merge.preview-conflicts`](07-merge-and-rewrite.md#mergepreview-conflicts--say-which-files-will-conflict) | Say which files will conflict | Read | Documented |
| [`rebase.onto`](07-merge-and-rewrite.md#rebaseonto--rebase-onto-a-commit-or-a-branch) | Rebase onto a commit or a branch | Operation | Documented |
| [`rebase.interactive`](07-merge-and-rewrite.md#rebaseinteractive--interactive-rebase) | Interactive rebase | Operation | Differs |
| [`rebase.editor`](07-merge-and-rewrite.md#rebaseeditor--the-interactive-rebase-editor) | The interactive rebase editor | View | Documented |
| [`rebase.todo`](07-merge-and-rewrite.md#rebasetodo--the-todo-list-of-an-interactive-rebase) | The todo list of an interactive rebase | Derivation | Documented |
| [`rebase.edit-stop`](07-merge-and-rewrite.md#rebaseedit-stop--stop-to-edit-a-commit) | Stop to edit a commit | Read | Documented |
| [`cherry-pick.one`](07-merge-and-rewrite.md#cherry-pickone--cherry-pick-a-commit) | Cherry-pick a commit | Operation | Differs |
| [`cherry-pick.many`](07-merge-and-rewrite.md#cherry-pickmany--cherry-pick-several-commits) | Cherry-pick several commits | Operation | Differs |
| [`revert.commit`](07-merge-and-rewrite.md#revertcommit--revert-a-commit) | Revert a commit | Operation | Documented |
| [`reset.to`](07-merge-and-rewrite.md#resetto--reset-the-current-branch-to-a-commit) | Reset the current branch to a commit | Operation | Documented |
| [`rewrite.reword`](07-merge-and-rewrite.md#rewritereword--edit-the-message-of-a-commit) | Edit the message of a commit | Operation | Differs |
| [`rewrite.message-box`](07-merge-and-rewrite.md#rewritemessage-box--the-box-that-edits-a-message) | The box that edits a message | View | Documented |
| [`rewrite.squash`](07-merge-and-rewrite.md#rewritesquash--squash-commits-into-one) | Squash commits into one | Operation | Differs |
| [`rewrite.squash-message`](07-merge-and-rewrite.md#rewritesquash-message--the-message-of-squashed-commits) | The message of squashed commits | Derivation | Documented |
| [`rewrite.split`](07-merge-and-rewrite.md#rewritesplit--split-a-commit) | Split a commit | Operation | Differs |
| [`rewrite.split-rest`](07-merge-and-rewrite.md#rewritesplit-rest--commit-what-is-left-of-a-split-and-continue) | Commit what is left of a split and continue | Operation | Documented |
| [`rewrite.split-left`](07-merge-and-rewrite.md#rewritesplit-left--what-is-left-of-a-commit-being-split) | What is left of a commit being split | Derivation | Documented |
| [`rewrite.fixup`](07-merge-and-rewrite.md#rewritefixup--add-changes-to-an-earlier-commit-fixup) | Add changes to an earlier commit (fixup) | Operation | Documented |
| [`rewrite.reach`](07-merge-and-rewrite.md#rewritereach--which-commits-a-rewrite-can-reach) | Which commits a rewrite can reach | Derivation | Documented |
| [`rewrite.pushed-warning`](07-merge-and-rewrite.md#rewritepushed-warning--warn-that-a-rewritten-commit-is-pushed) | Warn that a rewritten commit is pushed | Read | Documented |
| [`rewrite.drag-and-drop`](07-merge-and-rewrite.md#rewritedrag-and-drop--drag-and-drop-onto-a-row-of-the-graph) | Drag and drop onto a row of the graph | View | Documented |
| [`bisect.start`](07-merge-and-rewrite.md#bisectstart--start-a-bisect) | Start a bisect | Operation | Documented |
| [`bisect.mark`](07-merge-and-rewrite.md#bisectmark--mark-the-commit-tested) | Mark the commit tested | Operation | Documented |
| [`bisect.end`](07-merge-and-rewrite.md#bisectend--end-the-bisect) | End the bisect | Operation | Documented |
| [`bisect.banner`](07-merge-and-rewrite.md#bisectbanner--the-bisect-banner) | The bisect banner | View | Documented |

## Conflicts

| Feature | Title | Kind | Status |
| --- | --- | --- | --- |
| [`operation.banner`](09-conflicts.md#operationbanner--the-banner-of-an-operation-under-way) | The banner of an operation under way | View | Documented |
| [`operation.stopped`](09-conflicts.md#operationstopped--what-a-stop-on-conflicts-says) | What a stop on conflicts says | View | Documented |
| [`operation.continue`](09-conflicts.md#operationcontinue--continue-the-operation-under-way) | Continue the operation under way | Operation | Documented |
| [`operation.skip`](09-conflicts.md#operationskip--skip-the-step-the-operation-stopped-on) | Skip the step the operation stopped on | Operation | Differs |
| [`operation.abort`](09-conflicts.md#operationabort--abort-the-operation-under-way) | Abort the operation under way | Operation | Documented |
| [`conflict.open`](09-conflicts.md#conflictopen--open-a-file-in-conflict) | Open a file in conflict | View | Documented |
| [`conflict.open-deleted`](09-conflicts.md#conflictopen-deleted--show-a-file-one-side-deleted-as-a-diff-of-what-it-was) | Show a file one side deleted as a diff of what it was | View | Documented |
| [`conflict.sides`](09-conflicts.md#conflictsides--name-the-two-sides-of-a-conflict) | Name the two sides of a conflict | Derivation | Documented |
| [`conflict.take-side`](09-conflicts.md#conflicttake-side--take-a-whole-file-from-one-side) | Take a whole file from one side | Operation | Differs |
| [`conflict.mark-resolved`](09-conflicts.md#conflictmark-resolved--mark-a-file-as-resolved) | Mark a file as resolved | Operation | Documented |
| [`conflict.files-read`](09-conflicts.md#conflictfiles-read--read-the-files-in-conflict) | Read the files in conflict | Read | Documented |
| [`conflict.markers`](09-conflicts.md#conflictmarkers--find-the-conflicts-git-marked-in-a-file) | Find the conflicts git marked in a file | Derivation | Documented |
| [`conflict.rule-separate`](09-conflicts.md#conflictrule-separate--changes-to-lines-next-to-each-other) | Changes to lines next to each other | Derivation | Documented |
| [`conflict.rule-same`](09-conflicts.md#conflictrule-same--the-same-change-on-both-sides) | The same change on both sides | Derivation | Documented |
| [`conflict.rule-whitespace`](09-conflicts.md#conflictrule-whitespace--spaces-or-line-endings-only) | Spaces or line endings only | Derivation | Documented |
| [`conflict.rule-one-side`](09-conflicts.md#conflictrule-one-side--a-change-on-one-side-only) | A change on one side only | Derivation | Documented |
| [`conflict.rules-guard`](09-conflicts.md#conflictrules-guard--when-the-rules-leave-a-conflict-as-git-marked-it) | When the rules leave a conflict as git marked it | Derivation | Documented |
| [`conflict.resolve-rules`](09-conflicts.md#conflictresolve-rules--resolve-the-conflicts-that-need-no-choice) | Resolve the conflicts that need no choice | Operation | Documented |
| [`conflict.review`](09-conflicts.md#conflictreview--the-review-of-the-resolutions) | The review of the resolutions | View | Documented |
| [`conflict.review-files`](09-conflicts.md#conflictreview-files--the-files-of-the-review-and-its-filters) | The files of the review and its filters | View | Documented |
| [`conflict.review-card`](09-conflicts.md#conflictreview-card--a-conflict-in-the-review) | A conflict in the review | View | Documented |
| [`conflict.review-columns`](09-conflicts.md#conflictreview-columns--a-conflict-in-three-columns) | A conflict in three columns | View | Documented |
| [`conflict.proposal-lines`](09-conflicts.md#conflictproposal-lines--where-the-lines-of-a-proposal-come-from) | Where the lines of a proposal come from | Derivation | Documented |
| [`conflict.proposal-warnings`](09-conflicts.md#conflictproposal-warnings--what-looks-like-a-mistake-in-a-proposal) | What looks like a mistake in a proposal | Derivation | Documented |
| [`conflict.choose`](09-conflicts.md#conflictchoose--choose-a-side-or-write-the-lines-in-the-review) | Choose a side, or write the lines, in the review | View | Documented |
| [`conflict.accept`](09-conflicts.md#conflictaccept--accept-a-proposed-resolution) | Accept a proposed resolution | View | Documented |
| [`conflict.show-result`](09-conflicts.md#conflictshow-result--show-the-result-of-the-accepted-resolutions) | Show the result of the accepted resolutions | View | Documented |
| [`conflict.write`](09-conflicts.md#conflictwrite--write-the-accepted-resolutions) | Write the accepted resolutions | Operation | Differs |
| [`conflict.deleted-card`](09-conflicts.md#conflictdeleted-card--a-file-deleted-on-one-side-in-the-review) | A file deleted on one side, in the review | View | Documented |
| [`conflict.resolve-ai`](09-conflicts.md#conflictresolve-ai--ask-the-ai-how-to-resolve-conflicts) | Ask the AI how to resolve conflicts | Read | Documented |
| [`conflict.choose-model`](09-conflicts.md#conflictchoose-model--choose-the-model-that-resolves-conflicts) | Choose the model that resolves conflicts | Operation | Documented |
| [`conflict.ai-prompt`](09-conflicts.md#conflictai-prompt--the-question-sent-to-the-model-about-a-conflict) | The question sent to the model about a conflict | Derivation | Documented |
| [`conflict.ai-answer`](09-conflicts.md#conflictai-answer--read-the-models-answer-as-a-proposal) | Read the model's answer as a proposal | Derivation | Documented |
| [`conflict.restore`](09-conflicts.md#conflictrestore--mark-the-conflicts-again) | Mark the conflicts again | Operation | Documented |

## Worktrees

| Feature | Title | Kind | Status |
| --- | --- | --- | --- |
| [`worktree.section`](12-worktrees.md#worktreesection--the-worktrees-section) | The Worktrees section | View | Documented |
| [`worktree.folder`](12-worktrees.md#worktreefolder--where-a-new-worktree-goes) | Where a new worktree goes | Derivation | Documented |
| [`worktree.branch-name`](12-worktrees.md#worktreebranch-name--the-name-a-format-makes) | The name a format makes | Derivation | Documented |
| [`worktree.create`](12-worktrees.md#worktreecreate--create-a-worktree) | Create a worktree | Operation | Differs |
| [`worktree.create-for-branch`](12-worktrees.md#worktreecreate-for-branch--create-a-worktree-for-a-branch) | Create a worktree for a branch | Operation | Differs |
| [`worktree.create-at-commit`](12-worktrees.md#worktreecreate-at-commit--create-a-worktree-at-a-commit) | Create a worktree at a commit | Operation | Differs |
| [`worktree.open-in-worktree`](12-worktrees.md#worktreeopen-in-worktree--open-a-branch-in-a-worktree-made-for-it) | Open a branch in a worktree made for it | Operation | Differs |
| [`worktree.open`](12-worktrees.md#worktreeopen--open-a-worktree) | Open a worktree | View | Documented |
| [`worktree.open-terminal`](12-worktrees.md#worktreeopen-terminal--open-a-terminal-in-a-worktree) | Open a terminal in a worktree | View | Documented |
| [`worktree.reveal`](12-worktrees.md#worktreereveal--show-a-worktree-in-the-file-manager) | Show a worktree in the file manager | View | Documented |
| [`worktree.copy-path`](12-worktrees.md#worktreecopy-path--copy-the-path-of-a-worktree) | Copy the path of a worktree | View | Documented |
| [`worktree.show-in-graph`](12-worktrees.md#worktreeshow-in-graph--show-what-a-worktree-has-checked-out-in-the-graph) | Show what a worktree has checked out in the graph | View | Documented |
| [`worktree.switch`](12-worktrees.md#worktreeswitch--switch-worktree) | Switch worktree | View | Documented |
| [`worktree.show-here`](12-worktrees.md#worktreeshow-here--show-another-worktree-in-this-window) | Show another worktree in this window | View | Documented |
| [`worktree.select`](12-worktrees.md#worktreeselect--choose-the-worktree-shown) | Choose the worktree shown | View | Documented |
| [`worktree.banner`](12-worktrees.md#worktreebanner--the-worktree-banner) | The worktree banner | View | Documented |
| [`worktree.rename`](12-worktrees.md#worktreerename--rename-a-worktree) | Rename a worktree | Operation | Differs |
| [`worktree.remove`](12-worktrees.md#worktreeremove--remove-a-worktree) | Remove a worktree | Operation | Differs |
| [`worktree.prune`](12-worktrees.md#worktreeprune--prune-missing-worktrees) | Prune missing worktrees | Operation | Differs |
| [`worktree.branch-elsewhere`](12-worktrees.md#worktreebranch-elsewhere--the-branch-another-worktree-has) | The branch another worktree has | View | Documented |
| [`setup.source`](12-worktrees.md#setupsource--which-settings-apply) | Which settings apply | Derivation | Documented |
| [`setup.folders-to-link`](12-worktrees.md#setupfolders-to-link--the-folders-a-pattern-links) | The folders a pattern links | Read | Documented |
| [`setup.paths-to-copy`](12-worktrees.md#setuppaths-to-copy--the-files-a-pattern-copies) | The files a pattern copies | Read | Documented |
| [`setup.link-and-copy`](12-worktrees.md#setuplink-and-copy--link-and-copy-into-a-new-worktree) | Link and copy into a new worktree | Operation | Differs |
| [`setup.commands`](12-worktrees.md#setupcommands--run-commands-in-a-new-worktree) | Run commands in a new worktree | Operation | Documented |
| [`setup.propose-commands`](12-worktrees.md#setuppropose-commands--the-commands-proposed) | The commands proposed | Derivation | Documented |
| [`setup.scan`](12-worktrees.md#setupscan--what-git-ignores-here) | What git ignores here | Read | Documented |
| [`setup.classify`](12-worktrees.md#setupclassify--what-is-proposed-for-each) | What is proposed for each | Derivation | Documented |
| [`setup.choose`](12-worktrees.md#setupchoose--the-patterns-a-choice-writes) | The patterns a choice writes | Derivation | Documented |
| [`setup.preview`](12-worktrees.md#setuppreview--what-the-patterns-take) | What the patterns take | Read | Documented |
| [`setup.page`](12-worktrees.md#setuppage--the-tab-of-what-new-worktrees-get) | The tab of what new worktrees get | View | Documented |
| [`setup.save`](12-worktrees.md#setupsave--save-what-new-worktrees-get) | Save what new worktrees get | Operation | Differs |
| [`setup.offer`](12-worktrees.md#setupoffer--the-offer-to-choose-what-new-worktrees-get) | The offer to choose what new worktrees get | View | Documented |
| [`setup.bring-ignored`](12-worktrees.md#setupbring-ignored--bring-ignored-files-into-worktrees-that-exist) | Bring ignored files into worktrees that exist | Operation | Differs |
| [`setup.explorer-link`](12-worktrees.md#setupexplorer-link--link-into-new-worktrees-from-the-explorer) | Link into new worktrees from the Explorer | Operation | Differs |
| [`setup.explorer-copy`](12-worktrees.md#setupexplorer-copy--copy-into-new-worktrees-from-the-explorer) | Copy into new worktrees from the Explorer | Operation | Differs |
| [`setup.explorer-stop`](12-worktrees.md#setupexplorer-stop--stop-linking-or-copying-into-new-worktrees) | Stop linking or copying into new worktrees | Operation | Differs |
| [`setup.explorer-change`](12-worktrees.md#setupexplorer-change--what-became-of-what-was-chosen-in-the-explorer) | What became of what was chosen in the Explorer | Derivation | Documented |

## Submodules

| Feature | Title | Kind | Status |
| --- | --- | --- | --- |
| [`submodule.read`](13-submodules.md#submoduleread--what-is-read-of-each-submodule) | What is read of each submodule | Read | Documented |
| [`submodule.status`](13-submodules.md#submodulestatus--where-a-submodule-stands) | Where a submodule stands | Derivation | Documented |
| [`submodule.section`](13-submodules.md#submodulesection--the-submodules-section) | The Submodules section | View | Documented |
| [`submodule.open-window`](13-submodules.md#submoduleopen-window--open-a-submodule-in-a-new-window) | Open a submodule in a new window | View | Documented |
| [`submodule.terminal`](13-submodules.md#submoduleterminal--open-a-terminal-in-a-submodule) | Open a terminal in a submodule | View | Documented |
| [`submodule.reveal`](13-submodules.md#submodulereveal--reveal-a-submodule-in-the-file-manager) | Reveal a submodule in the file manager | View | Documented |
| [`submodule.show-in-gitardo`](13-submodules.md#submoduleshow-in-gitardo--show-a-submodule-in-gitardo) | Show a submodule in Gitardo | View | Documented |
| [`submodule.copy`](13-submodules.md#submodulecopy--copy-the-path-or-the-url-of-a-submodule) | Copy the path or the URL of a submodule | View | Documented |
| [`submodule.add`](13-submodules.md#submoduleadd--add-a-submodule) | Add a submodule | Operation | Differs |
| [`submodule.initialize`](13-submodules.md#submoduleinitialize--initialize-a-submodule) | Initialize a submodule | Operation | Differs |
| [`submodule.update`](13-submodules.md#submoduleupdate--update-a-submodule-to-the-recorded-commit) | Update a submodule to the recorded commit | Operation | Differs |
| [`submodule.update-remote`](13-submodules.md#submoduleupdate-remote--update-a-submodule-to-the-latest-of-its-branch) | Update a submodule to the latest of its branch | Operation | Differs |
| [`submodule.record`](13-submodules.md#submodulerecord--stage-the-commit-of-a-submodule-as-the-recorded-one) | Stage the commit of a submodule as the recorded one | Operation | Differs |
| [`submodule.sync`](13-submodules.md#submodulesync--synchronize-the-urls-of-the-submodules) | Synchronize the URLs of the submodules | Operation | Differs |
| [`submodule.deinit`](13-submodules.md#submoduledeinit--deinitialize-a-submodule) | Deinitialize a submodule | Operation | Differs |
| [`submodule.remove`](13-submodules.md#submoduleremove--remove-a-submodule) | Remove a submodule | Operation | Differs |
| [`submodule.update-on-checkout`](13-submodules.md#submoduleupdate-on-checkout--update-the-submodules-after-a-checkout) | Update the submodules after a checkout | Operation | Documented |

## Undo, command preview and the log

| Feature | Title | Kind | Status |
| --- | --- | --- | --- |
| [`preview.ask`](16-undo-preview-log.md#previewask--show-the-exact-commands-before-an-operation-runs) | Show the exact commands before an operation runs | Operation | Documented |
| [`preview.fallback`](16-undo-preview-log.md#previewfallback--offer-the-other-way-when-git-refuses) | Offer the other way when git refuses | Operation | Documented |
| [`log.write`](16-undo-preview-log.md#logwrite--write-every-command-and-its-output-to-the-log) | Write every command and its output to the log | Operation | Documented |
| [`log.show`](16-undo-preview-log.md#logshow--show-the-command-log) | Show the command log | View | Documented |
| [`log.report-failure`](16-undo-preview-log.md#logreport-failure--say-why-an-operation-failed) | Say why an operation failed | View | Documented |
| [`activity.track`](16-undo-preview-log.md#activitytrack--show-what-is-running-in-the-status-bar) | Show what is running in the status bar | View | Documented |
| [`activity.show`](16-undo-preview-log.md#activityshow--list-what-is-running) | List what is running | View | Documented |

## Settings

| Feature | Title | Kind | Status |
| --- | --- | --- | --- |
| [`settings.read`](18-settings.md#settingsread--read-the-settings-when-they-are-needed) | Read the settings when they are needed | Read | Documented |

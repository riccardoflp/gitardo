# The catalog of features

Every feature, one card each, 99 in all. The template and the rules of the cards are in the
feature contract. This index is written by `catalog.py --index`: change a status here in the commit that
earns it, and run the script again after adding or renaming a card; the statuses are kept. A card with a
**Differs** line is *Differs*; when the line goes, it becomes *Documented*.

| File | Area | Cards |
| --- | --- | --- |
| [01-repositories.md](01-repositories.md) | Repositories | 2 |
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

# The catalog of features

Every feature, one card each, 65 in all. The template and the rules of the cards are in the
feature contract. This index is written by `catalog.py --index`: change a status here in the commit that
earns it, and run the script again after adding or renaming a card; the statuses are kept. A card with a
**Differs** line is *Differs*; when the line goes, it becomes *Documented*.

| File | Area | Cards |
| --- | --- | --- |
| [01-repositories.md](01-repositories.md) | Repositories | 2 |
| [12-worktrees.md](12-worktrees.md) | Worktrees | 38 |
| [13-submodules.md](13-submodules.md) | Submodules | 17 |
| [16-undo-preview-log.md](16-undo-preview-log.md) | Undo, command preview and the log | 7 |
| [18-settings.md](18-settings.md) | Settings | 1 |

## Repositories

| Feature | Title | Kind | Status |
| --- | --- | --- | --- |
| [`repo.select`](01-repositories.md#reposelect--choose-the-repository-shown) | Choose the repository shown | View | Documented |
| [`repo.follow-source-control`](01-repositories.md#repofollow-source-control--follow-the-repositories-view-of-vs-code) | Follow the Repositories view of VS Code | View | Documented |


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

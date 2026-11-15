# The catalog of features

Every feature, one card each, 96 in all. The template and the rules of the cards are in the
feature contract. This index is written by `catalog.py --index`: change a status here in the commit that
earns it, and run the script again after adding or renaming a card; the statuses are kept. A card with a
**Differs** line is *Differs*; when the line goes, it becomes *Documented*.

| File | Area | Cards |
| --- | --- | --- |
| [01-repositories.md](01-repositories.md) | Repositories | 2 |
| [06-branches-tags-remotes.md](06-branches-tags-remotes.md) | Branches, tags, remotes and sync | 24 |
| [07-merge-and-rewrite.md](07-merge-and-rewrite.md) | Merge, rebase and rewriting history | 28 |
| [09-conflicts.md](09-conflicts.md) | Conflicts | 34 |
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

# The catalog of features

Every feature of Gitardo Desktop, one card each, 406 in all. A card says what the feature does from the user's side,
which function it is, what it asks, what it runs, how it is undone, and what has to be true for it to be done. The
template and the rules of the cards are in [04-feature-contract.md](../04-feature-contract.md).

**Status**: *Specified* once the card is complete; *Reviewed* once someone who did not write it has read it and its
questions are answered; *Built* once its function and the tests of its acceptance list pass `cargo xtask check`;
*Seen* once it has been used in the application as the card says. Change the status here in the commit that earns it.

| File | Area | Cards |
| --- | --- | --- |
| [01-repositories.md](01-repositories.md) | Repositories | 14 |
| [02-graph.md](02-graph.md) | Graph | 38 |
| [03-search-and-filter.md](03-search-and-filter.md) | Search and filter | 23 |
| [04-details-and-commit-page.md](04-details-and-commit-page.md) | Details, commits and the page of a commit | 17 |
| [05-changes-and-commit.md](05-changes-and-commit.md) | Changes and commit | 19 |
| [06-branches-tags-remotes.md](06-branches-tags-remotes.md) | Branches, tags, remotes and sync | 36 |
| [07-history-rewriting.md](07-history-rewriting.md) | History rewriting | 30 |
| [08-stashes.md](08-stashes.md) | Stashes | 14 |
| [09-conflicts.md](09-conflicts.md) | Conflicts | 35 |
| [10-compare.md](10-compare.md) | Compare | 24 |
| [11-history-and-blame.md](11-history-and-blame.md) | History and blame | 30 |
| [12-worktrees.md](12-worktrees.md) | Worktrees | 45 |
| [13-submodules.md](13-submodules.md) | Submodules | 20 |
| [14-patches.md](14-patches.md) | Patches | 7 |
| [15-github.md](15-github.md) | GitHub | 6 |
| [16-undo-preview-log.md](16-undo-preview-log.md) | Undo, command preview and the log | 6 |
| [17-diff-and-merge-views.md](17-diff-and-merge-views.md) | Diff, Merge and File pages | 39 |
| [18-settings.md](18-settings.md) | Settings | 3 |

## Repositories

| Feature | Title | Kind | Status |
| --- | --- | --- | --- |
| [`repo.read-repositories`](01-repositories.md#reporead-repositories--find-the-repositories-of-a-workspace) | Find the repositories of a workspace | Read | Built |
| [`repo.name-repositories`](01-repositories.md#reponame-repositories--name-the-repositories-in-lists) | Name the repositories in lists | Derivation | Built |
| [`repo.open-folder`](01-repositories.md#repoopen-folder--open-a-folder) | Open a folder | Operation | Specified |
| [`repo.open-recent`](01-repositories.md#repoopen-recent--open-a-recent-workspace) | Open a recent workspace | Operation | Specified |
| [`repo.select`](01-repositories.md#reposelect--choose-the-repository-shown) | Choose the repository shown | Operation | Specified |
| [`repo.owner-of-file`](01-repositories.md#repoowner-of-file--the-repository-a-file-is-in) | The repository a file is in | Derivation | Built |
| [`repo.no-repository`](01-repositories.md#repono-repository--no-repository-in-the-workspace) | No repository in the workspace | View | Built |
| [`repo.read-state`](01-repositories.md#reporead-state--read-the-state-of-the-repository) | Read the state of the repository | Read | Built |
| [`repo.watch-changes`](01-repositories.md#repowatch-changes--which-changes-ask-for-a-read) | Which changes ask for a read | Derivation | Built |
| [`repo.schedule-reads`](01-repositories.md#reposchedule-reads--read-again-when-something-changed) | Read again when something changed | Derivation | Built |
| [`activity.describe-task`](01-repositories.md#activitydescribe-task--say-what-gitardo-is-doing) | Say what Gitardo is doing | Derivation | Built |
| [`activity.status-bar`](01-repositories.md#activitystatus-bar--what-is-running-in-the-header) | What is running, in the header | View | Built |
| [`activity.what-is-running`](01-repositories.md#activitywhat-is-running--what-is-running-in-the-activity-drawer) | What is running, in the Activity drawer | View | Built |
| [`welcome.page`](01-repositories.md#welcomepage--the-welcome-page) | The Welcome page | View | Built |

## Graph

| Feature | Title | Kind | Status |
| --- | --- | --- | --- |
| [`graph.read-commits`](02-graph.md#graphread-commits--read-a-page-of-the-graph) | Read a page of the graph | Read | Built |
| [`graph.read-position`](02-graph.md#graphread-position--where-a-commit-is-in-the-history-listed) | Where a commit is in the history listed | Read | Built |
| [`graph.view`](02-graph.md#graphview--the-timeline) | The timeline | View | Built |
| [`graph.toolbar`](02-graph.md#graphtoolbar--the-header) | The header | View | Specified |
| [`graph.more-menu`](02-graph.md#graphmore-menu--the--menu-of-the-header) | The … menu of the header | View | Specified |
| [`graph.load-more`](02-graph.md#graphload-more--load-the-next-commits-while-scrolling) | Load the next commits while scrolling | View | Built |
| [`graph.lanes`](02-graph.md#graphlanes--lay-out-the-lines-of-the-graph) | Lay out the lines of the graph | Derivation | Built |
| [`graph.fit-width`](02-graph.md#graphfit-width--how-wide-the-graph-is-drawn) | How wide the graph is drawn | Derivation | Built |
| [`graph.resize-graph`](02-graph.md#graphresize-graph--give-the-graph-less-room-or-more) | Give the graph less room or more | View | Built |
| [`graph.resize-panes`](02-graph.md#graphresize-panes--resize-the-branch-rail-and-the-inspector) | Resize the branch rail and the inspector | View | Built |
| [`graph.toggle-references`](02-graph.md#graphtoggle-references--put-the-branch-rail-away) | Put the branch rail away | View | Built |
| [`graph.labels`](02-graph.md#graphlabels--the-labels-of-each-commit) | The labels of each commit | Derivation | Built |
| [`graph.combine-upstream`](02-graph.md#graphcombine-upstream--a-branch-and-its-upstream-on-one-label) | A branch and its upstream on one label | Derivation | Built |
| [`graph.pull-request-badge`](02-graph.md#graphpull-request-badge--pull-request-on-a-label) | Pull request on a label | View | Specified |
| [`graph.unpushed`](02-graph.md#graphunpushed--commits-not-pushed-yet) | Commits not pushed yet | Derivation | Built |
| [`graph.trace-history`](02-graph.md#graphtrace-history--the-history-of-a-commit-among-the-rows) | The history of a commit among the rows | Derivation | Built |
| [`graph.hover-history`](02-graph.md#graphhover-history--the-history-of-a-label-under-the-pointer) | The history of a label under the pointer | View | Built |
| [`graph.trace-selected`](02-graph.md#graphtrace-selected--the-history-of-the-selected-commit) | The history of the selected commit | View | Specified |
| [`graph.columns`](02-graph.md#graphcolumns--what-is-shown-with-each-message) | What is shown with each message | View | Built |
| [`graph.date-separators`](02-graph.md#graphdate-separators--lines-between-days-weeks-and-months) | Lines between days, weeks and months | Derivation | Specified |
| [`graph.first-parent`](02-graph.md#graphfirst-parent--only-the-first-parent-of-merges) | Only the first parent of merges | View | Built |
| [`graph.fold-rows`](02-graph.md#graphfold-rows--the-rows-of-the-timeline-with-merges-folded) | The rows of the timeline with merges folded | Derivation | Built |
| [`graph.fold-merge`](02-graph.md#graphfold-merge--fold-the-commits-a-merge-brought-in) | Fold the commits a merge brought in | View | Specified |
| [`graph.fold-all`](02-graph.md#graphfold-all--fold-all-merges) | Fold all merges | View | Built |
| [`graph.lane-tips`](02-graph.md#graphlane-tips--the-branch-each-line-comes-from) | The branch each line comes from | Derivation | Built |
| [`graph.sticky-branch-names`](02-graph.md#graphsticky-branch-names--branch-names-that-stay-at-the-top) | Branch names that stay at the top | View | Built |
| [`graph.scroll-marks`](02-graph.md#graphscroll-marks--marks-beside-the-scroll-bar) | Marks beside the scroll bar | View | Built |
| [`graph.keyboard`](02-graph.md#graphkeyboard--move-through-the-timeline-with-the-keyboard) | Move through the timeline with the keyboard | View | Built |
| [`graph.uncommitted-row`](02-graph.md#graphuncommitted-row--the-draft-at-the-top-of-the-timeline) | The draft at the top of the timeline | View | Built |
| [`graph.place-stashes`](02-graph.md#graphplace-stashes--where-the-stashes-go-among-the-commits) | Where the stashes go among the commits | Derivation | Built |
| [`graph.stashes`](02-graph.md#graphstashes--stashes-in-the-timeline) | Stashes in the timeline | View | Specified |
| [`graph.show-in-graph`](02-graph.md#graphshow-in-graph--go-to-a-commit-in-the-timeline) | Go to a commit in the timeline | View | Specified |
| [`refs.tree`](02-graph.md#refstree--the-branch-rail) | The branch rail | View | Built |
| [`refs.filter`](02-graph.md#refsfilter--filter-the-branch-rail-by-name) | Filter the branch rail by name | View | Specified |
| [`refs.scope-line`](02-graph.md#refsscope-line--the-line-of-the-branches-shown-at-the-top-of-the-branch-rail) | The line of the branches shown, at the top of the branch rail | View | Built |
| [`refs.sort`](02-graph.md#refssort--sort-the-branches-and-tags) | Sort the branches and tags | Derivation | Specified |
| [`refs.recent`](02-graph.md#refsrecent--the-branches-checked-out-last) | The branches checked out last | Derivation | Built |
| [`refs.favorite`](02-graph.md#refsfavorite--add-a-branch-to-favorites) | Add a branch to favorites | View | Built |

## Search and filter

| Feature | Title | Kind | Status |
| --- | --- | --- | --- |
| [`search.box`](03-search-and-filter.md#searchbox--the-search-box) | The search box | View | Built |
| [`search.loaded`](03-search-and-filter.md#searchloaded--find-among-the-loaded-commits) | Find among the loaded commits | Derivation | Built |
| [`search.read-matches`](03-search-and-filter.md#searchread-matches--search-the-whole-history) | Search the whole history | Read | Built |
| [`search.history`](03-search-and-filter.md#searchhistory--find-in-the-whole-history) | Find in the whole history | View | Built |
| [`search.step`](03-search-and-filter.md#searchstep--step-through-the-matches) | Step through the matches | Derivation | Built |
| [`search.list-mode`](03-search-and-filter.md#searchlist-mode--only-the-commits-found-as-a-list) | Only the commits found, as a list | View | Built |
| [`search.go-to-hash`](03-search-and-filter.md#searchgo-to-hash--go-to-a-commit-by-its-hash) | Go to a commit by its hash | View | Built |
| [`search.read-revision`](03-search-and-filter.md#searchread-revision--the-commit-a-name-stands-for) | The commit a name stands for | Read | Built |
| [`search.go-to-commit`](03-search-and-filter.md#searchgo-to-commit--go-to-commit) | Go to commit… | Operation | Specified |
| [`filter.picker`](03-search-and-filter.md#filterpicker--choose-the-branches-shown) | Choose the branches shown | View | Built |
| [`filter.label`](03-search-and-filter.md#filterlabel--what-the-branches-shown-are-called) | What the branches shown are called | Derivation | Built |
| [`filter.preset`](03-search-and-filter.md#filterpreset--all-current-upstream-local-remotes) | All, Current, Upstream, Local, Remotes | Derivation | Built |
| [`filter.tick`](03-search-and-filter.md#filtertick--tick-the-branches-to-show) | Tick the branches to show | Derivation | Built |
| [`filter.rank-names`](03-search-and-filter.md#filterrank-names--which-names-match-best) | Which names match best | Derivation | Built |
| [`filter.list-too`](03-search-and-filter.md#filterlist-too--filter-the-list-of-branches-too) | Filter the list of branches too | View | Built |
| [`filter.kinds`](03-search-and-filter.md#filterkinds--show-local-branches-remote-branches-tags) | Show local branches, remote branches, tags | Derivation | Built |
| [`filter.show-only`](03-search-and-filter.md#filtershow-only--show-only-this-history) | Show only this history | Derivation | Built |
| [`filter.show-too`](03-search-and-filter.md#filtershow-too--show-this-history-too) | Show this history too | Derivation | Built |
| [`filter.hide`](03-search-and-filter.md#filterhide--hide-this-history) | Hide this history | Derivation | Built |
| [`filter.show-all`](03-search-and-filter.md#filtershow-all--show-all-branches) | Show all branches | Derivation | Built |
| [`filter.bar`](03-search-and-filter.md#filterbar--the-bar-of-what-the-timeline-is-narrowed-to) | The bar of what the timeline is narrowed to | View | Specified |
| [`filter.log-arguments`](03-search-and-filter.md#filterlog-arguments--which-history-the-timeline-lists) | Which history the timeline lists | Derivation | Built |
| [`filter.remember`](03-search-and-filter.md#filterremember--the-branches-shown-remembered-per-repository) | The branches shown, remembered per repository | Derivation | Built |

## Details, commits and the page of a commit

| Feature | Title | Kind | Status |
| --- | --- | --- | --- |
| [`details.read-commit`](04-details-and-commit-page.md#detailsread-commit--read-the-details-of-a-commit) | Read the details of a commit | Read | Built |
| [`details.show`](04-details-and-commit-page.md#detailsshow--details-of-the-selected-commit) | Details of the selected commit | View | Built |
| [`details.message-title`](04-details-and-commit-page.md#detailsmessage-title--the-title-of-a-message) | The title of a message | Derivation | Built |
| [`details.message-unwrap`](04-details-and-commit-page.md#detailsmessage-unwrap--lines-of-a-message-put-back-together) | Lines of a message put back together | Derivation | Built |
| [`details.show-more`](04-details-and-commit-page.md#detailsshow-more--show-more-of-a-long-message) | Show more of a long message | View | Built |
| [`details.file-tree`](04-details-and-commit-page.md#detailsfile-tree--files-as-a-tree-of-directories) | Files as a tree of directories | Derivation | Built |
| [`details.show-all-files`](04-details-and-commit-page.md#detailsshow-all-files--the-first-300-files-and-show-all) | The first 300 files, and Show all | View | Built |
| [`details.compare-file-with`](04-details-and-commit-page.md#detailscompare-file-with--compare-a-file-with-a-branch-tag-or-commit) | Compare a file with a branch, tag or commit | Operation | Specified |
| [`details.stash`](04-details-and-commit-page.md#detailsstash--details-of-a-stash) | Details of a stash | View | Built |
| [`commits.copy-hash`](04-details-and-commit-page.md#commitscopy-hash--copy-the-hash-of-a-commit) | Copy the hash of a commit | Operation | Built |
| [`commits.copy-message`](04-details-and-commit-page.md#commitscopy-message--copy-the-message-of-a-commit) | Copy the message of a commit | Operation | Built |
| [`commit-page.open`](04-details-and-commit-page.md#commit-pageopen--open-a-commit-as-a-page) | Open a commit as a page | Operation | Built |
| [`commit-page.read-patch`](04-details-and-commit-page.md#commit-pageread-patch--read-the-lines-a-commit-changed) | Read the lines a commit changed | Read | Built |
| [`commit-page.diff-lines`](04-details-and-commit-page.md#commit-pagediff-lines--the-lines-of-each-diff-and-how-many-are-sent) | The lines of each diff, and how many are sent | Derivation | Built |
| [`commit-page.view`](04-details-and-commit-page.md#commit-pageview--the-page-of-a-commit) | The page of a commit | View | Built |
| [`commit-page.initial-folds`](04-details-and-commit-page.md#commit-pageinitial-folds--which-diffs-start-folded) | Which diffs start folded | Derivation | Built |
| [`commit-page.show-in-graph`](04-details-and-commit-page.md#commit-pageshow-in-graph--show-a-commit-in-the-timeline) | Show a commit in the timeline | View | Built |

## Changes and commit

| Feature | Title | Kind | Status |
| --- | --- | --- | --- |
| [`changes.pane`](05-changes-and-commit.md#changespane--the-changes-page) | The Changes page | View | Specified |
| [`changes.read-working-tree`](05-changes-and-commit.md#changesread-working-tree--read-what-is-not-committed) | Read what is not committed | Read | Built |
| [`changes.list`](05-changes-and-commit.md#changeslist--one-list-of-the-changed-files) | One list of the changed files | Derivation | Built |
| [`changes.unsaved`](05-changes-and-commit.md#changesunsaved--files-with-edits-not-saved) | Files with edits not saved | Derivation | Specified |
| [`changes.open`](05-changes-and-commit.md#changesopen--what-a-file-of-the-changes-page-opens) | What a file of the Changes page opens | Derivation | Specified |
| [`changes.stage`](05-changes-and-commit.md#changesstage--stage-a-file) | Stage a file | Operation | Built |
| [`changes.unstage`](05-changes-and-commit.md#changesunstage--unstage-a-file) | Unstage a file | Operation | Built |
| [`changes.stage-all`](05-changes-and-commit.md#changesstage-all--stage-all-changes) | Stage all changes | Operation | Built |
| [`changes.unstage-all`](05-changes-and-commit.md#changesunstage-all--unstage-all-changes) | Unstage all changes | Operation | Built |
| [`changes.discard`](05-changes-and-commit.md#changesdiscard--discard-the-changes-of-a-file) | Discard the changes of a file | Operation | Built |
| [`changes.discard-all`](05-changes-and-commit.md#changesdiscard-all--discard-all-changes) | Discard all changes | Operation | Specified |
| [`changes.save`](05-changes-and-commit.md#changessave--save-a-file-or-all-of-them) | Save a file, or all of them | Operation | Built |
| [`changes.pathspec`](05-changes-and-commit.md#changespathspec--a-file-named-as-itself) | A file named as itself | Derivation | Built |
| [`commit.create`](05-changes-and-commit.md#commitcreate--commit) | Commit | Operation | Specified |
| [`commit.amend`](05-changes-and-commit.md#commitamend--amend-the-last-commit) | Amend the last commit | Operation | Built |
| [`commit.draft-message`](05-changes-and-commit.md#commitdraft-message--draft-the-commit-message) | Draft the commit message | Derivation | Specified |
| [`identity.read`](05-changes-and-commit.md#identityread--read-the-identity) | Read the identity | Read | Built |
| [`identity.switch`](05-changes-and-commit.md#identityswitch--switch-identity) | Switch identity | Operation | Built |
| [`identity.use-global`](05-changes-and-commit.md#identityuse-global--use-the-global-identity-again) | Use the global identity again | Operation | Built |

## Branches, tags, remotes and sync

| Feature | Title | Kind | Status |
| --- | --- | --- | --- |
| [`branch.checkout`](06-branches-tags-remotes.md#branchcheckout--check-out-a-branch) | Check out a branch | Operation | Built |
| [`branch.open`](06-branches-tags-remotes.md#branchopen--what-a-double-click-on-a-branch-does) | What a double click on a branch does | Derivation | Built |
| [`branch.checkout-remote`](06-branches-tags-remotes.md#branchcheckout-remote--check-out-a-remote-branch) | Check out a remote branch | Operation | Built |
| [`branch.checkout-remote-and-rebase`](06-branches-tags-remotes.md#branchcheckout-remote-and-rebase--check-out-a-branch-and-rebase-it-onto-its-remote-branch) | Check out a branch and rebase it onto its remote branch | Operation | Built |
| [`branch.checkout-detached`](06-branches-tags-remotes.md#branchcheckout-detached--check-out-a-commit) | Check out a commit | Operation | Built |
| [`branch.create`](06-branches-tags-remotes.md#branchcreate--create-a-branch) | Create a branch | Operation | Specified |
| [`branch.format-name`](06-branches-tags-remotes.md#branchformat-name--expand-the-format-of-branch-names) | Expand the format of branch names | Derivation | Built |
| [`branch.name-from-format`](06-branches-tags-remotes.md#branchname-from-format--ask-for-the-parts-of-the-format) | Ask for the parts of the format | Derivation | Specified |
| [`branch.validate-name`](06-branches-tags-remotes.md#branchvalidate-name--what-git-does-not-take-as-a-name) | What git does not take as a name | Derivation | Built |
| [`branch.ref-argument`](06-branches-tags-remotes.md#branchref-argument--a-branch-named-in-full-when-it-has-a-namesake) | A branch named in full when it has a namesake | Derivation | Built |
| [`branch.rename`](06-branches-tags-remotes.md#branchrename--rename-a-branch) | Rename a branch | Operation | Built |
| [`branch.delete`](06-branches-tags-remotes.md#branchdelete--delete-a-branch) | Delete a branch | Operation | Built |
| [`branch.clean-up`](06-branches-tags-remotes.md#branchclean-up--clean-up-branches) | Clean up branches | Operation | Built |
| [`branch.fast-forward`](06-branches-tags-remotes.md#branchfast-forward--fast-forward-a-branch-that-is-not-checked-out) | Fast-forward a branch that is not checked out | Operation | Built |
| [`branch.pane`](06-branches-tags-remotes.md#branchpane--the-branch-rail-its-buttons-and-its-comparison) | The branch rail, its buttons and its comparison | View | Specified |
| [`tag.create`](06-branches-tags-remotes.md#tagcreate--create-a-tag) | Create a tag | Operation | Built |
| [`tag.push`](06-branches-tags-remotes.md#tagpush--push-a-tag) | Push a tag | Operation | Built |
| [`tag.delete`](06-branches-tags-remotes.md#tagdelete--delete-a-tag) | Delete a tag | Operation | Built |
| [`tag.checkout`](06-branches-tags-remotes.md#tagcheckout--check-out-a-tag) | Check out a tag | Operation | Built |
| [`remote.read-remotes`](06-branches-tags-remotes.md#remoteread-remotes--read-the-remotes) | Read the remotes | Read | Built |
| [`remote.default`](06-branches-tags-remotes.md#remotedefault--the-remote-a-new-upstream-or-a-tag-goes-to) | The remote a new upstream or a tag goes to | Derivation | Built |
| [`remote.manage`](06-branches-tags-remotes.md#remotemanage--manage-remotes) | Manage remotes | Operation | Specified |
| [`remote.fetch`](06-branches-tags-remotes.md#remotefetch--fetch-one-remote) | Fetch one remote | Operation | Built |
| [`remote.set-url`](06-branches-tags-remotes.md#remoteset-url--change-the-url-of-a-remote) | Change the URL of a remote | Operation | Built |
| [`remote.rename`](06-branches-tags-remotes.md#remoterename--rename-a-remote) | Rename a remote | Operation | Built |
| [`remote.remove`](06-branches-tags-remotes.md#remoteremove--remove-a-remote) | Remove a remote | Operation | Built |
| [`remote.add`](06-branches-tags-remotes.md#remoteadd--add-a-remote) | Add a remote | Operation | Specified |
| [`remote.delete-branch`](06-branches-tags-remotes.md#remotedelete-branch--delete-a-branch-from-its-remote) | Delete a branch from its remote | Operation | Built |
| [`sync.fetch`](06-branches-tags-remotes.md#syncfetch--fetch) | Fetch | Operation | Built |
| [`sync.pull`](06-branches-tags-remotes.md#syncpull--pull) | Pull | Operation | Built |
| [`sync.pull-with`](06-branches-tags-remotes.md#syncpull-with--pull-one-way-or-another) | Pull one way or another | Operation | Built |
| [`sync.push`](06-branches-tags-remotes.md#syncpush--push) | Push | Operation | Built |
| [`sync.sync`](06-branches-tags-remotes.md#syncsync--sync-with-the-upstream) | Sync with the upstream | Operation | Built |
| [`sync.read-overwritten`](06-branches-tags-remotes.md#syncread-overwritten--read-what-a-force-push-would-overwrite) | Read what a force push would overwrite | Read | Built |
| [`sync.push-force-with-lease`](06-branches-tags-remotes.md#syncpush-force-with-lease--push-with---force-with-lease) | Push with `--force-with-lease` | Operation | Built |
| [`sync.push-force`](06-branches-tags-remotes.md#syncpush-force--push-with---force) | Push with `--force` | Operation | Built |

## History rewriting

| Feature | Title | Kind | Status |
| --- | --- | --- | --- |
| [`selection.select`](07-history-rewriting.md#selectionselect--select-several-commits) | Select several commits | View | Built |
| [`selection.bar`](07-history-rewriting.md#selectionbar--the-bar-of-a-selection-of-several-commits) | The bar of a selection of several commits | View | Built |
| [`selection.menu`](07-history-rewriting.md#selectionmenu--the-context-menu-of-a-selection-of-several-commits) | The context menu of a selection of several commits | View | Built |
| [`selection.cherry-pick`](07-history-rewriting.md#selectioncherry-pick--cherry-pick-several-commits) | Cherry-pick several commits | Operation | Specified |
| [`cherry-pick.commit`](07-history-rewriting.md#cherry-pickcommit--cherry-pick-a-commit) | Cherry-pick a commit | Operation | Built |
| [`cherry-pick.skip-empty`](07-history-rewriting.md#cherry-pickskip-empty--get-past-a-commit-that-changes-nothing) | Get past a commit that changes nothing | Operation | Built |
| [`revert.commit`](07-history-rewriting.md#revertcommit--revert-a-commit) | Revert a commit | Operation | Built |
| [`reset.to-commit`](07-history-rewriting.md#resetto-commit--reset-the-current-branch-to-a-commit) | Reset the current branch to a commit | Operation | Built |
| [`merge.read-predicted-conflicts`](07-history-rewriting.md#mergeread-predicted-conflicts--predict-the-conflicts-of-a-merge) | Predict the conflicts of a merge | Read | Built |
| [`merge.into-current`](07-history-rewriting.md#mergeinto-current--merge-into-the-current-branch) | Merge into the current branch | Operation | Built |
| [`merge.choose-way`](07-history-rewriting.md#mergechoose-way--merge-into-the-current-branch-choosing-how) | Merge into the current branch, choosing how | Operation | Built |
| [`merge.undo-squash`](07-history-rewriting.md#mergeundo-squash--take-back-a-squash-merge) | Take back a squash merge | Operation | Built |
| [`rebase.onto`](07-history-rewriting.md#rebaseonto--rebase-the-current-branch-onto-a-commit-or-a-branch) | Rebase the current branch onto a commit or a branch | Operation | Built |
| [`rebase.branch-onto`](07-history-rewriting.md#rebasebranch-onto--rebase-another-local-branch-onto-a-commit) | Rebase another local branch onto a commit | Operation | Built |
| [`rebase.find-line-before-head`](07-history-rewriting.md#rebasefind-line-before-head--the-line-of-commits-an-interactive-rebase-can-rewrite) | The line of commits an interactive rebase can rewrite | Derivation | Built |
| [`rebase.read-pushed`](07-history-rewriting.md#rebaseread-pushed--where-a-commit-was-pushed) | Where a commit was pushed | Read | Built |
| [`rebase.todo-page`](07-history-rewriting.md#rebasetodo-page--the-interactive-rebase-page) | The interactive rebase page | View | Built |
| [`rebase.build-todo-list`](07-history-rewriting.md#rebasebuild-todo-list--the-todo-list-of-an-interactive-rebase) | The todo list of an interactive rebase | Derivation | Built |
| [`rebase.interactive`](07-history-rewriting.md#rebaseinteractive--run-an-interactive-rebase) | Run an interactive rebase | Operation | Built |
| [`rebase.read-stop`](07-history-rewriting.md#rebaseread-stop--where-an-interactive-rebase-stopped-to-change-a-commit) | Where an interactive rebase stopped to change a commit | Read | Built |
| [`rebase.edit-stop`](07-history-rewriting.md#rebaseedit-stop--the-banner-of-a-rebase-stopped-to-edit-a-commit) | The banner of a rebase stopped to edit a commit | View | Built |
| [`message.edit`](07-history-rewriting.md#messageedit--edit-a-commit-message) | Edit a commit message | Operation | Specified |
| [`squash.join-messages`](07-history-rewriting.md#squashjoin-messages--the-message-of-commits-squashed-into-one) | The message of commits squashed into one | Derivation | Built |
| [`squash.commits`](07-history-rewriting.md#squashcommits--squash-commits-into-one) | Squash commits into one | Operation | Built |
| [`squash.into-previous`](07-history-rewriting.md#squashinto-previous--squash-a-commit-into-the-commit-before-it) | Squash a commit into the commit before it | Operation | Built |
| [`split.commit`](07-history-rewriting.md#splitcommit--split-a-commit) | Split a commit | Operation | Built |
| [`split.continue`](07-history-rewriting.md#splitcontinue--continue-a-split) | Continue a split | Operation | Built |
| [`fixup.commit`](07-history-rewriting.md#fixupcommit--add-changes-to-an-earlier-commit) | Add changes to an earlier commit | Operation | Built |
| [`drag.list-drop-entries`](07-history-rewriting.md#draglist-drop-entries--what-a-drop-offers) | What a drop offers | Derivation | Built |
| [`drag.drop`](07-history-rewriting.md#dragdrop--drag-a-commit-or-a-branch-onto-the-timeline) | Drag a commit or a branch onto the timeline | View | Specified |

## Stashes

| Feature | Title | Kind | Status |
| --- | --- | --- | --- |
| [`stash.read-list`](08-stashes.md#stashread-list--read-the-stashes) | Read the stashes | Read | Built |
| [`stash.read-files`](08-stashes.md#stashread-files--read-what-a-stash-changes) | Read what a stash changes | Read | Built |
| [`stash.pane`](08-stashes.md#stashpane--the-stashes-list) | The Stashes list | View | Built |
| [`stash.push`](08-stashes.md#stashpush--stash-all-changes) | Stash all changes | Operation | Built |
| [`stash.push-staged`](08-stashes.md#stashpush-staged--stash-the-staged-changes) | Stash the staged changes | Operation | Built |
| [`stash.push-file`](08-stashes.md#stashpush-file--stash-a-file-or-the-files-ticked) | Stash a file, or the files ticked | Operation | Built |
| [`stash.preview`](08-stashes.md#stashpreview--what-applying-a-stash-would-stop-on) | What applying a stash would stop on | Derivation | Built |
| [`stash.apply`](08-stashes.md#stashapply--apply-a-stash) | Apply a stash | Operation | Built |
| [`stash.pop`](08-stashes.md#stashpop--pop-a-stash) | Pop a stash | Operation | Built |
| [`stash.drop`](08-stashes.md#stashdrop--drop-a-stash) | Drop a stash | Operation | Built |
| [`stash.undo-drop`](08-stashes.md#stashundo-drop--undo-a-stash-dropped) | Undo a stash dropped | Operation | Built |
| [`stash.branch`](08-stashes.md#stashbranch--create-a-branch-from-a-stash) | Create a branch from a stash | Operation | Built |
| [`stash.in-graph`](08-stashes.md#stashin-graph--stashes-in-the-timeline) | Stashes in the timeline | Derivation | Built |
| [`stash.autostash-left`](08-stashes.md#stashautostash-left--the-stash-a-pull-leaves) | The stash a pull leaves | Derivation | Built |

## Conflicts

| Feature | Title | Kind | Status |
| --- | --- | --- | --- |
| [`conflicts.banner`](09-conflicts.md#conflictsbanner--the-banner-of-an-operation-that-stopped) | The banner of an operation that stopped | View | Built |
| [`conflicts.continue-operation`](09-conflicts.md#conflictscontinue-operation--continue) | Continue | Operation | Built |
| [`conflicts.skip-step`](09-conflicts.md#conflictsskip-step--skip) | Skip | Operation | Built |
| [`conflicts.abort-operation`](09-conflicts.md#conflictsabort-operation--abort) | Abort | Operation | Built |
| [`conflicts.stopped`](09-conflicts.md#conflictsstopped--the-message-when-an-operation-stops-on-conflicts) | The message when an operation stops on conflicts | View | Built |
| [`conflicts.read-files`](09-conflicts.md#conflictsread-files--the-files-in-conflict-and-their-sides) | The files in conflict and their sides | Read | Built |
| [`conflicts.side-names`](09-conflicts.md#conflictsside-names--what-the-two-sides-are) | What the two sides are | Derivation | Built |
| [`conflicts.open-file`](09-conflicts.md#conflictsopen-file--open-a-file-in-conflict) | Open a file in conflict | View | Built |
| [`conflicts.mark-resolved`](09-conflicts.md#conflictsmark-resolved--mark-as-resolved) | Mark as resolved | Operation | Built |
| [`conflicts.take-side`](09-conflicts.md#conflictstake-side--take-the-current-side-take-the-incoming-side) | Take the current side, take the incoming side | Operation | Built |
| [`conflicts.parse-markers`](09-conflicts.md#conflictsparse-markers--the-conflicts-git-marked-in-a-file) | The conflicts git marked in a file | Derivation | Built |
| [`conflicts.resolve-by-rules`](09-conflicts.md#conflictsresolve-by-rules--the-conflicts-that-need-no-choice) | The conflicts that need no choice | Derivation | Built |
| [`conflicts.resolve-automatically`](09-conflicts.md#conflictsresolve-automatically--resolve-conflicts-automatically) | Resolve conflicts automatically | Operation | Built |
| [`conflicts.restore`](09-conflicts.md#conflictsrestore--undo-a-resolution) | Undo a resolution | Operation | Specified |
| [`review.page`](09-conflicts.md#reviewpage--the-review-of-the-resolutions) | The review of the resolutions | View | Specified |
| [`review.file-list`](09-conflicts.md#reviewfile-list--the-files-and-the-filters) | The files and the filters | View | Built |
| [`review.card`](09-conflicts.md#reviewcard--a-conflict-as-a-card) | A conflict, as a card | View | Specified |
| [`review.side-diff`](09-conflicts.md#reviewside-diff--what-a-side-changed-of-the-base) | What a side changed of the base | Derivation | Built |
| [`review.fingerprint`](09-conflicts.md#reviewfingerprint--the-same-conflict-wherever-it-is) | The same conflict, wherever it is | Derivation | Built |
| [`review.proposals`](09-conflicts.md#reviewproposals--what-is-proposed-for-each-conflict) | What is proposed for each conflict | Derivation | Built |
| [`review.proposal-lines`](09-conflicts.md#reviewproposal-lines--where-the-lines-of-a-proposal-come-from) | Where the lines of a proposal come from | Derivation | Built |
| [`review.proposal-warnings`](09-conflicts.md#reviewproposal-warnings--what-looks-like-a-mistake-in-a-proposal) | What looks like a mistake in a proposal | Derivation | Built |
| [`review.choose`](09-conflicts.md#reviewchoose--choose-a-side-or-write-the-lines) | Choose a side, or write the lines | View | Specified |
| [`review.accept`](09-conflicts.md#reviewaccept--accept) | Accept | View | Built |
| [`review.accept-file`](09-conflicts.md#reviewaccept-file--accept-the-file) | Accept the file | View | Built |
| [`review.accept-all`](09-conflicts.md#reviewaccept-all--accept-all-proposed) | Accept all proposed | View | Built |
| [`review.clear`](09-conflicts.md#reviewclear--clear) | Clear | View | Built |
| [`review.write`](09-conflicts.md#reviewwrite--write-the-resolutions) | Write the resolutions | Operation | Built |
| [`review.report`](09-conflicts.md#reviewreport--what-was-written) | What was written | View | Built |
| [`review.columns`](09-conflicts.md#reviewcolumns--columns) | Columns | View | Specified |
| [`review.show-result`](09-conflicts.md#reviewshow-result--show-the-result) | Show the result | View | Built |
| [`review.deleted-card`](09-conflicts.md#reviewdeleted-card--a-file-deleted-on-one-side) | A file deleted on one side | View | Built |
| [`ai.resolve-conflicts`](09-conflicts.md#airesolve-conflicts--resolve-conflicts-with-ai) | Resolve conflicts with AI | View | Specified |
| [`ai.ask-conflict`](09-conflicts.md#aiask-conflict--one-question-about-one-conflict) | One question about one conflict | Read | Specified |
| [`ai.conflict-question`](09-conflicts.md#aiconflict-question--what-is-sent-about-a-conflict) | What is sent about a conflict | Derivation | Specified |
| [`ai.read-answer`](09-conflicts.md#airead-answer--the-answer-read) | The answer, read | Derivation | Built |
| [`ai.trim-answer`](09-conflicts.md#aitrim-answer--the-lines-around-the-conflict-written-again) | The lines around the conflict, written again | Derivation | Built |
| [`ai.choose-model`](09-conflicts.md#aichoose-model--choose-the-model-that-resolves-conflicts) | Choose the model that resolves conflicts | Operation | Specified |

## Compare

| Feature | Title | Kind | Status |
| --- | --- | --- | --- |
| [`compare.resolve-revision`](10-compare.md#compareresolve-revision--which-commit-a-revision-is) | Which commit a revision is | Read | Built |
| [`compare.side-choices`](10-compare.md#compareside-choices--what-a-side-can-be-chosen-from) | What a side can be chosen from | Derivation | Built |
| [`compare.recent`](10-compare.md#comparerecent--the-comparisons-opened-before) | The comparisons opened before | Derivation | Built |
| [`compare.choose`](10-compare.md#comparechoose--compare-branches-tags-or-commits) | Compare branches, tags or commits | Operation | Built |
| [`compare.follow-sides`](10-compare.md#comparefollow-sides--followed-sides-and-fixed-ones) | Followed sides and fixed ones | Derivation | Built |
| [`compare.initial-mode`](10-compare.md#compareinitial-mode--how-two-sides-are-compared-at-first) | How two sides are compared at first | Derivation | Built |
| [`compare.read-sides`](10-compare.md#compareread-sides--read-a-comparison) | Read a comparison | Read | Built |
| [`compare.read-patch`](10-compare.md#compareread-patch--read-the-lines-of-a-comparison) | Read the lines of a comparison | Read | Built |
| [`compare.page`](10-compare.md#comparepage--the-comparison-page) | The comparison page | View | Built |
| [`compare.summary`](10-compare.md#comparesummary--how-far-apart-and-what-merging-would-do) | How far apart, and what merging would do | Derivation | Built |
| [`compare.change-side`](10-compare.md#comparechange-side--choose-another-side) | Choose another side | Operation | Built |
| [`compare.swap`](10-compare.md#compareswap--swap-the-two-sides) | Swap the two sides | View | Built |
| [`compare.set-mode`](10-compare.md#compareset-mode--since-they-diverged-or-everything-that-differs) | Since they diverged, or everything that differs | View | Built |
| [`compare.narrow-to-path`](10-compare.md#comparenarrow-to-path--narrow-a-comparison-to-a-file-or-a-folder) | Narrow a comparison to a file or a folder | View | Built |
| [`compare.commits`](10-compare.md#comparecommits--the-commits-only-in-each-side) | The commits only in each side | View | Built |
| [`compare.take-file`](10-compare.md#comparetake-file--take-the-version-of-the-second-side) | Take the version of the second side | Operation | Built |
| [`compare.copy-patch`](10-compare.md#comparecopy-patch--copy-a-comparison-as-a-patch) | Copy a comparison as a patch | Operation | Built |
| [`compare.open-on-github`](10-compare.md#compareopen-on-github--open-the-comparison-on-github) | Open the comparison on GitHub | Operation | Specified |
| [`compare.path-with`](10-compare.md#comparepath-with--compare-a-file-or-a-folder-with-a-branch-tag-or-commit) | Compare a file or a folder with a branch, tag or commit | Operation | Specified |
| [`compare.with`](10-compare.md#comparewith--compare-with) | Compare with… | Operation | Built |
| [`compare.with-current`](10-compare.md#comparewith-current--compare-with-the-current-branch) | Compare with the current branch | Operation | Built |
| [`compare.selected-before`](10-compare.md#compareselected-before--compare-two-commits-of-the-timeline) | Compare two commits of the timeline | Operation | Built |
| [`compare.read-files`](10-compare.md#compareread-files--read-the-files-that-differ-between-two-commits) | Read the files that differ between two commits | Read | Built |
| [`compare.in-place`](10-compare.md#comparein-place--the-comparison-in-the-inspector) | The comparison in the inspector | View | Built |

## History and blame

| Feature | Title | Kind | Status |
| --- | --- | --- | --- |
| [`file-history.read-names`](11-history-and-blame.md#file-historyread-names--the-names-a-file-had) | The names a file had | Read | Built |
| [`file-history.file`](11-history-and-blame.md#file-historyfile--show-the-history-of-a-file) | Show the history of a file | Operation | Specified |
| [`file-history.read-committed-lines`](11-history-and-blame.md#file-historyread-committed-lines--the-lines-as-the-commit-checked-out-has-them) | The lines as the commit checked out has them | Read | Built |
| [`file-history.map-lines`](11-history-and-blame.md#file-historymap-lines--map-lines-of-a-file-as-it-is-to-the-file-as-committed) | Map lines of a file as it is to the file as committed | Derivation | Built |
| [`file-history.lines`](11-history-and-blame.md#file-historylines--show-the-history-of-some-lines) | Show the history of some lines | Operation | Specified |
| [`file-history.folder`](11-history-and-blame.md#file-historyfolder--show-the-history-of-a-folder) | Show the history of a folder | Operation | Specified |
| [`blame.read-line`](11-history-and-blame.md#blameread-line--who-last-changed-one-line) | Who last changed one line | Read | Built |
| [`blame.current-line`](11-history-and-blame.md#blamecurrent-line--the-blame-of-the-line-under-the-cursor) | The blame of the line under the cursor | View | Specified |
| [`blame.toggle-current-line`](11-history-and-blame.md#blametoggle-current-line--toggle-the-blame-of-the-line-under-the-cursor) | Toggle the blame of the line under the cursor | Operation | Specified |
| [`blame.open-line-commit`](11-history-and-blame.md#blameopen-line-commit--open-the-commit-of-the-current-line) | Open the commit of the current line | Operation | Specified |
| [`blame.reveal-line-commit`](11-history-and-blame.md#blamereveal-line-commit--show-the-commit-of-the-current-line-in-the-timeline) | Show the commit of the current line in the timeline | Operation | Specified |
| [`blame.read-file`](11-history-and-blame.md#blameread-file--who-last-changed-each-line-of-a-file) | Who last changed each line of a file | Read | Built |
| [`blame.format-line`](11-history-and-blame.md#blameformat-line--what-is-said-beside-each-line) | What is said beside each line | Derivation | Built |
| [`blame.age-color`](11-history-and-blame.md#blameage-color--the-color-of-the-age-of-a-line) | The color of the age of a line | Derivation | Built |
| [`blame.pick-pinned-line`](11-history-and-blame.md#blamepick-pinned-line--the-commit-named-on-the-first-line-in-sight) | The commit named on the first line in sight | Derivation | Built |
| [`blame.mark-edited-lines`](11-history-and-blame.md#blamemark-edited-lines--lines-typed-are-not-committed-yet) | Lines typed are not committed yet | Derivation | Built |
| [`blame.file`](11-history-and-blame.md#blamefile--the-blame-beside-the-lines-of-a-file) | The blame beside the lines of a file | View | Specified |
| [`blame.toggle-file`](11-history-and-blame.md#blametoggle-file--show-or-hide-the-blame-of-a-file) | Show or hide the blame of a file | Operation | Specified |
| [`blame.previous`](11-history-and-blame.md#blameprevious--blame-the-file-as-it-was-before-a-commit) | Blame the file as it was before a commit | Operation | Specified |
| [`blame.ignore-whitespace`](11-history-and-blame.md#blameignore-whitespace--ignore-whitespace-and-moved-lines-in-the-blame) | Ignore whitespace and moved lines in the blame | Operation | Specified |
| [`blame.every-file`](11-history-and-blame.md#blameevery-file--show-the-blame-in-every-file) | Show the blame in every file | Operation | Specified |
| [`reflog.read`](11-history-and-blame.md#reflogread--where-head-has-been) | Where `HEAD` has been | Read | Built |
| [`reflog.browse`](11-history-and-blame.md#reflogbrowse--browse-the-reflog) | Browse the reflog | Operation | Built |
| [`bisect.read`](11-history-and-blame.md#bisectread--the-bisect-under-way) | The bisect under way | Read | Built |
| [`bisect.start`](11-history-and-blame.md#bisectstart--start-a-bisect-from-a-commit) | Start a bisect from a commit | Operation | Built |
| [`bisect.mark`](11-history-and-blame.md#bisectmark--mark-a-commit-good-or-bad) | Mark a commit good or bad | Operation | Built |
| [`bisect.answer`](11-history-and-blame.md#bisectanswer--answer-for-the-commit-being-tested) | Answer for the commit being tested | Operation | Built |
| [`bisect.banner`](11-history-and-blame.md#bisectbanner--the-banner-of-a-bisect) | The banner of a bisect | View | Built |
| [`bisect.marks`](11-history-and-blame.md#bisectmarks--the-marks-of-a-bisect-in-the-timeline) | The marks of a bisect in the timeline | View | Built |
| [`bisect.end`](11-history-and-blame.md#bisectend--end-the-bisect) | End the bisect | Operation | Built |

## Worktrees

| Feature | Title | Kind | Status |
| --- | --- | --- | --- |
| [`worktree.read-list`](12-worktrees.md#worktreeread-list--read-the-worktrees-and-how-they-stand) | Read the worktrees and how they stand | Read | Built |
| [`worktree.folder-path`](12-worktrees.md#worktreefolder-path--where-a-new-worktree-goes) | Where a new worktree goes | Derivation | Built |
| [`worktree.create`](12-worktrees.md#worktreecreate--create-a-worktree) | Create a worktree | Operation | Specified |
| [`worktree.create-from-branch`](12-worktrees.md#worktreecreate-from-branch--put-a-branch-in-a-new-worktree) | Put a branch in a new worktree | Operation | Specified |
| [`worktree.create-at-commit`](12-worktrees.md#worktreecreate-at-commit--create-a-worktree-at-a-commit) | Create a worktree at a commit | Operation | Specified |
| [`worktree.section`](12-worktrees.md#worktreesection--the-worktrees-list) | The Worktrees list | View | Specified |
| [`worktree.open`](12-worktrees.md#worktreeopen--open-a-worktree-where-the-settings-say) | Open a worktree where the settings say | Operation | Specified |
| [`worktree.open-in-new-window`](12-worktrees.md#worktreeopen-in-new-window--open-a-worktree-in-a-new-window) | Open a worktree in a new window | Operation | Specified |
| [`worktree.open-in-editor`](12-worktrees.md#worktreeopen-in-editor--open-a-worktree-in-the-external-editor) | Open a worktree in the external editor | Operation | Specified |
| [`worktree.open-in-terminal`](12-worktrees.md#worktreeopen-in-terminal--open-a-terminal-in-a-worktree) | Open a terminal in a worktree | Operation | Specified |
| [`worktree.reveal`](12-worktrees.md#worktreereveal--reveal-a-worktree-in-the-file-manager) | Reveal a worktree in the file manager | Operation | Specified |
| [`worktree.show-in-graph`](12-worktrees.md#worktreeshow-in-graph--show-the-commit-of-a-worktree-in-the-timeline) | Show the commit of a worktree in the timeline | Operation | Specified |
| [`worktree.show-in-gitardo`](12-worktrees.md#worktreeshow-in-gitardo--show-another-worktree-in-this-window) | Show another worktree in this window | Operation | Specified |
| [`worktree.banner`](12-worktrees.md#worktreebanner--the-banner-of-a-worktree-shown-in-this-window) | The banner of a worktree shown in this window | View | Specified |
| [`worktree.rename`](12-worktrees.md#worktreerename--rename-a-worktree) | Rename a worktree | Operation | Specified |
| [`worktree.copy-path`](12-worktrees.md#worktreecopy-path--copy-the-path-of-a-worktree) | Copy the path of a worktree | Operation | Specified |
| [`worktree.remove`](12-worktrees.md#worktreeremove--remove-a-worktree) | Remove a worktree | Operation | Specified |
| [`worktree.prune`](12-worktrees.md#worktreeprune--prune-missing-worktrees) | Prune missing worktrees | Operation | Specified |
| [`worktree.read-folder-links`](12-worktrees.md#worktreeread-folder-links--read-the-links-to-folders-outside-a-worktree) | Read the links to folders outside a worktree | Read | Built |
| [`worktree.recover-held-folder`](12-worktrees.md#worktreerecover-held-folder--get-past-a-folder-that-windows-holds) | Get past a folder that Windows holds | Derivation | Specified |
| [`worktree.remove-empty-folders`](12-worktrees.md#worktreeremove-empty-folders--leave-no-empty-folders-behind) | Leave no empty folders behind | Derivation | Specified |
| [`worktree.branch-in-another-worktree`](12-worktrees.md#worktreebranch-in-another-worktree--the-branch-of-another-worktree) | The branch of another worktree | Derivation | Specified |
| [`worktree.open-in-worktree`](12-worktrees.md#worktreeopen-in-worktree--open-a-branch-in-a-worktree-of-its-own) | Open a branch in a worktree of its own | Operation | Specified |
| [`worktree.switch`](12-worktrees.md#worktreeswitch--switch-worktree) | Switch worktree | Operation | Specified |
| [`worktree-setup.settings-in-effect`](12-worktrees.md#worktree-setupsettings-in-effect--what-new-worktrees-get-as-set) | What new worktrees get, as set | Derivation | Built |
| [`worktree-setup.folders-to-link`](12-worktrees.md#worktree-setupfolders-to-link--the-folders-to-link) | The folders to link | Derivation | Built |
| [`worktree-setup.paths-to-copy`](12-worktrees.md#worktree-setuppaths-to-copy--the-files-and-folders-to-copy) | The files and folders to copy | Derivation | Built |
| [`worktree-setup.link-and-copy-steps`](12-worktrees.md#worktree-setuplink-and-copy-steps--linking-and-copying-into-a-new-worktree) | Linking and copying into a new worktree | Derivation | Built |
| [`worktree-setup.command-steps`](12-worktrees.md#worktree-setupcommand-steps--commands-in-a-new-worktree) | Commands in a new worktree | Derivation | Specified |
| [`worktree-setup.read-ignored`](12-worktrees.md#worktree-setupread-ignored--read-what-git-ignores-with-its-size) | Read what git ignores, with its size | Read | Built |
| [`worktree-setup.propose`](12-worktrees.md#worktree-setuppropose--what-is-proposed-for-what-git-ignores) | What is proposed for what git ignores | Derivation | Built |
| [`worktree-setup.propose-commands`](12-worktrees.md#worktree-setuppropose-commands--commands-proposed-from-lock-files) | Commands proposed from lock files | Derivation | Built |
| [`worktree-setup.read-what-patterns-take`](12-worktrees.md#worktree-setupread-what-patterns-take--ask-git-what-the-patterns-take) | Ask git what the patterns take | Read | Built |
| [`worktree-setup.page`](12-worktrees.md#worktree-setuppage--choose-what-new-worktrees-get) | Choose what new worktrees get | View | Specified |
| [`worktree-setup.choose`](12-worktrees.md#worktree-setupchoose--choose-what-a-line-gets) | Choose what a line gets | Derivation | Built |
| [`worktree-setup.save`](12-worktrees.md#worktree-setupsave--save-what-new-worktrees-get) | Save what new worktrees get | Operation | Built |
| [`worktree-setup.remove-from`](12-worktrees.md#worktree-setupremove-from--remove-the-settings-of-new-worktrees-from-a-place) | Remove the settings of new worktrees from a place | Operation | Built |
| [`worktree-setup.edit-settings-file`](12-worktrees.md#worktree-setupedit-settings-file--edit-the-settings-of-new-worktrees-in-their-file) | Edit the settings of new worktrees in their file | Operation | Specified |
| [`worktree-setup.link-into-new-worktrees`](12-worktrees.md#worktree-setuplink-into-new-worktrees--link-a-folder-into-new-worktrees) | Link a folder into new worktrees | Operation | Built |
| [`worktree-setup.copy-into-new-worktrees`](12-worktrees.md#worktree-setupcopy-into-new-worktrees--copy-a-file-or-a-folder-into-new-worktrees) | Copy a file or a folder into new worktrees | Operation | Built |
| [`worktree-setup.stop-linking`](12-worktrees.md#worktree-setupstop-linking--stop-linking-a-folder-into-new-worktrees) | Stop linking a folder into new worktrees | Operation | Built |
| [`worktree-setup.stop-copying`](12-worktrees.md#worktree-setupstop-copying--stop-copying-into-new-worktrees) | Stop copying into new worktrees | Operation | Built |
| [`worktree-setup.explain-change`](12-worktrees.md#worktree-setupexplain-change--why-a-choice-from-the-files-page-changes-nothing) | Why a choice from the Files page changes nothing | Derivation | Built |
| [`worktree-setup.bring-into-worktree`](12-worktrees.md#worktree-setupbring-into-worktree--bring-ignored-files-into-a-worktree) | Bring ignored files into a worktree | Operation | Built |
| [`worktree-setup.bring-into-other-worktrees`](12-worktrees.md#worktree-setupbring-into-other-worktrees--bring-ignored-files-into-the-other-worktrees) | Bring ignored files into the other worktrees | Operation | Built |

## Submodules

| Feature | Title | Kind | Status |
| --- | --- | --- | --- |
| [`submodule.read-list`](13-submodules.md#submoduleread-list--read-the-submodules-and-where-they-stand) | Read the submodules and where they stand | Read | Built |
| [`submodule.status`](13-submodules.md#submodulestatus--where-a-submodule-stands) | Where a submodule stands | Derivation | Built |
| [`submodule.section`](13-submodules.md#submodulesection--the-submodules-list) | The Submodules list | View | Built |
| [`submodule.open-in-new-window`](13-submodules.md#submoduleopen-in-new-window--open-a-submodule-in-a-new-window) | Open a submodule in a new window | Operation | Built |
| [`submodule.open-in-terminal`](13-submodules.md#submoduleopen-in-terminal--open-a-terminal-in-a-submodule) | Open a terminal in a submodule | Operation | Built |
| [`submodule.reveal`](13-submodules.md#submodulereveal--reveal-a-submodule-in-the-file-manager) | Reveal a submodule in the file manager | Operation | Built |
| [`submodule.show-in-gitardo`](13-submodules.md#submoduleshow-in-gitardo--show-a-submodule-in-this-window) | Show a submodule in this window | Operation | Specified |
| [`submodule.update-to-recorded`](13-submodules.md#submoduleupdate-to-recorded--update-a-submodule-to-the-recorded-commit) | Update a submodule to the recorded commit | Operation | Built |
| [`submodule.stage-recorded`](13-submodules.md#submodulestage-recorded--stage-the-commit-of-a-submodule-as-the-recorded-one) | Stage the commit of a submodule as the recorded one | Operation | Built |
| [`submodule.update-to-latest`](13-submodules.md#submoduleupdate-to-latest--update-a-submodule-to-the-latest-of-its-branch) | Update a submodule to the latest of its branch | Operation | Built |
| [`submodule.initialize`](13-submodules.md#submoduleinitialize--initialize-a-submodule) | Initialize a submodule | Operation | Built |
| [`submodule.deinitialize`](13-submodules.md#submoduledeinitialize--deinitialize-a-submodule) | Deinitialize a submodule | Operation | Built |
| [`submodule.remove`](13-submodules.md#submoduleremove--remove-a-submodule) | Remove a submodule | Operation | Built |
| [`submodule.copy-path`](13-submodules.md#submodulecopy-path--copy-the-path-of-a-submodule) | Copy the path of a submodule | Operation | Built |
| [`submodule.copy-url`](13-submodules.md#submodulecopy-url--copy-the-url-of-a-submodule) | Copy the URL of a submodule | Operation | Built |
| [`submodule.add`](13-submodules.md#submoduleadd--add-a-submodule) | Add a submodule | Operation | Built |
| [`submodule.sync-urls`](13-submodules.md#submodulesync-urls--synchronize-the-urls-of-the-submodules) | Synchronize the URLs of the submodules | Operation | Built |
| [`submodule.allow-file-protocol`](13-submodules.md#submoduleallow-file-protocol--clone-a-submodule-from-a-folder-on-disk) | Clone a submodule from a folder on disk | Derivation | Built |
| [`submodule.update-on-checkout`](13-submodules.md#submoduleupdate-on-checkout--update-the-submodules-after-a-checkout) | Update the submodules after a checkout | Derivation | Built |
| [`submodule.follow-changes`](13-submodules.md#submodulefollow-changes--follow-what-happens-in-the-submodules) | Follow what happens in the submodules | Derivation | Built |

## Patches

| Feature | Title | Kind | Status |
| --- | --- | --- | --- |
| [`patch.read-format-patch`](14-patches.md#patchread-format-patch--read-commits-as-one-patch) | Read commits as one patch | Read | Built |
| [`patch.count-commits`](14-patches.md#patchcount-commits--how-many-commits-a-patch-has) | How many commits a patch has | Derivation | Built |
| [`patch.copy`](14-patches.md#patchcopy--copy-as-patch) | Copy as patch | Operation | Built |
| [`patch.save`](14-patches.md#patchsave--save-as-patch) | Save as patch… | Operation | Built |
| [`patch.check`](14-patches.md#patchcheck--check-a-patch-before-applying-it) | Check a patch before applying it | Read | Built |
| [`patch.apply`](14-patches.md#patchapply--apply-a-patch) | Apply a patch | Operation | Built |
| [`patch.am-stopped`](14-patches.md#patcham-stopped--a-patch-that-stopped-on-a-conflict) | A patch that stopped on a conflict | View | Built |

## GitHub

| Feature | Title | Kind | Status |
| --- | --- | --- | --- |
| [`github.detect-remote`](15-github.md#githubdetect-remote--the-repository-on-github) | The repository on GitHub | Derivation | Built |
| [`github.token`](15-github.md#githubtoken--read-the-github-token) | Read the GitHub token | Read | Specified |
| [`github.load-pull-requests`](15-github.md#githubload-pull-requests--load-pull-requests-from-github) | Load pull requests from GitHub | Operation | Specified |
| [`github.read-pull-requests`](15-github.md#githubread-pull-requests--read-the-open-pull-requests) | Read the open pull requests | Read | Specified |
| [`github.open-pull-request`](15-github.md#githubopen-pull-request--open-or-create-the-pull-request-of-a-branch) | Open or create the pull request of a branch | Derivation | Specified |
| [`github.offer-after-push`](15-github.md#githuboffer-after-push--offer-to-create-a-pull-request-after-the-first-push) | Offer to create a pull request after the first push | Derivation | Specified |

## Undo, command preview and the log

| Feature | Title | Kind | Status |
| --- | --- | --- | --- |
| [`preview.confirm`](16-undo-preview-log.md#previewconfirm--show-the-commands-before-they-run) | Show the commands before they run | View | Specified |
| [`log.record`](16-undo-preview-log.md#logrecord--log-every-command-that-changes-something) | Log every command that changes something | Operation | Specified |
| [`log.show`](16-undo-preview-log.md#logshow--show-the-command-log) | Show the command log | View | Built |
| [`undo.last`](16-undo-preview-log.md#undolast--undo-the-last-operation) | Undo the last operation | Operation | Specified |
| [`undo.journal`](16-undo-preview-log.md#undojournal--remember-what-undo-needs) | Remember what Undo needs | Derivation | Built |
| [`activity.track`](16-undo-preview-log.md#activitytrack--say-what-is-running) | Say what is running | Operation | Built |

## Diff, Merge and File pages

| Feature | Title | Kind | Status |
| --- | --- | --- | --- |
| [`diff.page`](17-diff-and-merge-views.md#diffpage--the-diff-page) | The Diff page | View | Built |
| [`diff.read-sides`](17-diff-and-merge-views.md#diffread-sides--the-two-versions-of-a-file) | The two versions of a file | Read | Built |
| [`diff.line-diff`](17-diff-and-merge-views.md#diffline-diff--the-lines-that-changed) | The lines that changed | Derivation | Built |
| [`diff.word-diff`](17-diff-and-merge-views.md#diffword-diff--the-words-that-changed-in-a-line) | The words that changed in a line | Derivation | Built |
| [`diff.align`](17-diff-and-merge-views.md#diffalign--the-two-sides-kept-level) | The two sides kept level | Derivation | Built |
| [`diff.scroll-together`](17-diff-and-merge-views.md#diffscroll-together--the-two-sides-scroll-together) | The two sides scroll together | Derivation | Built |
| [`diff.resize-sides`](17-diff-and-merge-views.md#diffresize-sides--making-one-side-wider) | Making one side wider | View | Built |
| [`diff.navigate`](17-diff-and-merge-views.md#diffnavigate--the-previous-and-the-next-change) | The previous and the next change | View | Built |
| [`diff.collapse-unchanged`](17-diff-and-merge-views.md#diffcollapse-unchanged--folding-unchanged-lines) | Folding unchanged lines | View | Built |
| [`diff.staged-changes`](17-diff-and-merge-views.md#diffstaged-changes--which-changes-are-in-the-index-already) | Which changes are in the index already | Derivation | Built |
| [`diff.change-patch`](17-diff-and-merge-views.md#diffchange-patch--the-patch-of-one-change) | The patch of one change | Derivation | Built |
| [`diff.stage-change`](17-diff-and-merge-views.md#diffstage-change--stage-this-change) | Stage this change | Operation | Built |
| [`diff.unstage-change`](17-diff-and-merge-views.md#diffunstage-change--unstage-this-change) | Unstage this change | Operation | Built |
| [`diff.revert-change`](17-diff-and-merge-views.md#diffrevert-change--revert-this-change) | Revert this change | Operation | Specified |
| [`diff.edit`](17-diff-and-merge-views.md#diffedit--editing-the-working-tree-side) | Editing the working tree side | View | Specified |
| [`diff.open-at-line`](17-diff-and-merge-views.md#diffopen-at-line--open-the-file-at-this-line) | Open the file at this line | View | Specified |
| [`diff.open-file`](17-diff-and-merge-views.md#diffopen-file--open-file) | Open file | View | Specified |
| [`diff.stage-file`](17-diff-and-merge-views.md#diffstage-file--stage-file) | Stage file | Operation | Built |
| [`diff.unstage-file`](17-diff-and-merge-views.md#diffunstage-file--unstage-file) | Unstage file | Operation | Built |
| [`diff.discard-file`](17-diff-and-merge-views.md#diffdiscard-file--discard-changes) | Discard changes | Operation | Built |
| [`merge-view.page`](17-diff-and-merge-views.md#merge-viewpage--the-merge-page) | The Merge page | View | Built |
| [`merge-view.read-sides`](17-diff-and-merge-views.md#merge-viewread-sides--the-versions-of-a-file-in-conflict) | The versions of a file in conflict | Read | Built |
| [`merge-view.three-way`](17-diff-and-merge-views.md#merge-viewthree-way--the-regions-of-a-file-in-conflict) | The regions of a file in conflict | Derivation | Built |
| [`merge-view.take-side`](17-diff-and-merge-views.md#merge-viewtake-side--accept-a-side-for-a-region) | Accept a side for a region | View | Built |
| [`merge-view.accept-simple`](17-diff-and-merge-views.md#merge-viewaccept-simple--accept-what-needs-no-choice) | Accept what needs no choice | View | Built |
| [`merge-view.accept-proposals`](17-diff-and-merge-views.md#merge-viewaccept-proposals--accept-the-proposals) | Accept the proposals | View | Specified |
| [`merge-view.show-base`](17-diff-and-merge-views.md#merge-viewshow-base--show-the-base) | Show the base | View | Built |
| [`merge-view.navigate`](17-diff-and-merge-views.md#merge-viewnavigate--changes-and-conflicts-left) | Changes and conflicts left | View | Built |
| [`merge-view.open-at-line`](17-diff-and-merge-views.md#merge-viewopen-at-line--open-the-file-at-this-line) | Open the file at this line | View | Built |
| [`merge-view.mark-resolved`](17-diff-and-merge-views.md#merge-viewmark-resolved--mark-as-resolved) | Mark as resolved | Operation | Built |
| [`merge-view.close`](17-diff-and-merge-views.md#merge-viewclose--closing-with-unsaved-changes) | Closing with unsaved changes | View | Built |
| [`file-view.page`](17-diff-and-merge-views.md#file-viewpage--the-file-page) | The File page | View | Built |
| [`file-view.read-revision`](17-diff-and-merge-views.md#file-viewread-revision--a-file-at-a-revision) | A file at a revision | Read | Built |
| [`file-view.edit`](17-diff-and-merge-views.md#file-viewedit--editing-a-file) | Editing a file | View | Built |
| [`file-view.save`](17-diff-and-merge-views.md#file-viewsave--save) | Save | Operation | Built |
| [`file-view.find`](17-diff-and-merge-views.md#file-viewfind--find) | Find | View | Built |
| [`file-view.go-to-line`](17-diff-and-merge-views.md#file-viewgo-to-line--go-to-line) | Go to line | View | Built |
| [`file-view.open-external`](17-diff-and-merge-views.md#file-viewopen-external--open-in-external-editor) | Open in external editor | Operation | Built |
| [`file-view.highlight`](17-diff-and-merge-views.md#file-viewhighlight--the-colors-of-the-code) | The colors of the code | Derivation | Built |

## Settings

| Feature | Title | Kind | Status |
| --- | --- | --- | --- |
| [`settings.read`](18-settings.md#settingsread--read-the-settings) | Read the settings | Read | Built |
| [`settings.edit`](18-settings.md#settingsedit--open-the-settings-page) | Open the settings page | View | Specified |
| [`settings.theme`](18-settings.md#settingstheme--choose-the-theme-and-the-fonts) | Choose the theme and the fonts | Operation | Specified |

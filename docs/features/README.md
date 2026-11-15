# The catalog of features

Every feature of Gitardo, one card each, 415 in all, with its status in each app. A card says what the feature does
from the user's side, which function it is, what it asks, what it runs, how it is undone, and what has to be true for
it to be done. The template and the rules of the cards are in [04-feature-contract.md](../desktop/04-feature-contract.md).

**Desktop**: *Specified* once the card is complete; *Reviewed* once someone who did not write it has read it and its
questions are answered; *Built* once its function and the tests of its acceptance list pass `cargo xtask check`;
*Seen* once it has been used in the application as the card says. **Extension**: *Planned* while the extension does
not do it yet and is to get it from the engine; *Built* once the extension does what the card says; *Seen* once it
has been used in VS Code as the card says; *Differs* while the extension does it otherwise, with the entries of
[transition/04-differences.md](../transition/04-differences.md) that say how, as *Differs (A5)*; *Host only* when
VS Code does it for the extension, which needs nothing of the card. A card is never *Built*, *Seen* or *Differs* in
the extension while it is not *Built* or *Seen* in the desktop, without a decision that says so
([D39](../08-decisions.md#d39--desktop-first-and-a-status-for-each-app)). Change a status here in the commit that
earns it.

| File | Area | Cards |
| --- | --- | --- |
| [01-repositories.md](01-repositories.md) | Repositories | 17 |
| [02-graph.md](02-graph.md) | Graph | 41 |
| [03-search-and-filter.md](03-search-and-filter.md) | Search and filter | 23 |
| [04-details-and-commit-page.md](04-details-and-commit-page.md) | Details, commits and the page of a commit | 17 |
| [05-changes-and-commit.md](05-changes-and-commit.md) | Changes and commit | 19 |
| [06-branches-tags-remotes.md](06-branches-tags-remotes.md) | Branches, tags, remotes and sync | 36 |
| [07-history-rewriting.md](07-history-rewriting.md) | History rewriting | 30 |
| [08-stashes.md](08-stashes.md) | Stashes | 14 |
| [09-conflicts.md](09-conflicts.md) | Conflicts | 38 |
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

| Feature | Title | Kind | Desktop | Extension |
| --- | --- | --- | --- | --- |
| [`repo.read-repositories`](01-repositories.md#reporead-repositories--find-the-repositories-of-a-workspace) | Find the repositories of a workspace | Read | Built | Built |
| [`repo.name-repositories`](01-repositories.md#reponame-repositories--name-the-repositories-in-lists) | Name the repositories in lists | Derivation | Built | Built |
| [`repo.open-folder`](01-repositories.md#repoopen-folder--open-a-folder) | Open a folder | Operation | Built | Host only |
| [`repo.open-recent`](01-repositories.md#repoopen-recent--open-a-recent-workspace) | Open a recent workspace | Operation | Built | Host only |
| [`repo.select`](01-repositories.md#reposelect--choose-the-repository-shown) | Choose the repository shown | Operation | Built | Built |
| [`repo.owner-of-file`](01-repositories.md#repoowner-of-file--the-repository-a-file-is-in) | The repository a file is in | Derivation | Built | Built |
| [`repo.no-repository`](01-repositories.md#repono-repository--no-repository-in-the-workspace) | No repository in the workspace | View | Built | Built |
| [`repo.read-state`](01-repositories.md#reporead-state--read-the-state-of-the-repository) | Read the state of the repository | Read | Built | Built |
| [`repo.watch-changes`](01-repositories.md#repowatch-changes--which-changes-ask-for-a-read) | Which changes ask for a read | Derivation | Built | Built |
| [`repo.schedule-reads`](01-repositories.md#reposchedule-reads--read-again-when-something-changed) | Read again when something changed | Derivation | Built | Built |
| [`activity.describe-task`](01-repositories.md#activitydescribe-task--say-what-gitardo-is-doing) | Say what Gitardo is doing | Derivation | Built | Built |
| [`activity.status-bar`](01-repositories.md#activitystatus-bar--what-is-running-in-the-status-bar) | What is running, in the status bar | View | Built | Built |
| [`activity.what-is-running`](01-repositories.md#activitywhat-is-running--what-is-running-in-the-activity-drawer) | What is running, in the Activity drawer | View | Built | Built |
| [`welcome.page`](01-repositories.md#welcomepage--the-welcome-page) | The Welcome page | View | Built | Host only |
| [`trust.is-trusted`](01-repositories.md#trustis-trusted--whether-a-folder-is-trusted) | Whether a folder is trusted | Derivation | Built | Host only |
| [`trust.ask`](01-repositories.md#trustask--ask-whether-to-trust-a-folder-before-running-what-it-asks) | Ask whether to trust a folder before running what it asks | Operation | Built | Host only |
| [`trust.manage`](01-repositories.md#trustmanage--see-the-trusted-folders-and-stop-trusting-some) | See the trusted folders and stop trusting some | Operation | Built | Host only |

## Graph

| Feature | Title | Kind | Desktop | Extension |
| --- | --- | --- | --- | --- |
| [`graph.read-commits`](02-graph.md#graphread-commits--read-a-page-of-the-graph) | Read a page of the graph | Read | Built | Built |
| [`graph.read-position`](02-graph.md#graphread-position--where-a-commit-is-in-the-history-listed) | Where a commit is in the history listed | Read | Built | Built |
| [`graph.view`](02-graph.md#graphview--the-timeline) | The timeline | View | Built | Built |
| [`graph.toolbar`](02-graph.md#graphtoolbar--the-header) | The header | View | Built | Built |
| [`graph.search-button`](02-graph.md#graphsearch-button--the-button-of-quick-search) | The button of Quick search | View | Built | Host only |
| [`graph.islands`](02-graph.md#graphislands--the-islands-of-the-window) | The islands of the window | View | Built | Host only |
| [`graph.view-bar`](02-graph.md#graphview-bar--the-bar-of-views) | The bar of views | View | Built | Host only |
| [`graph.more-menu`](02-graph.md#graphmore-menu--the--menu-of-the-header) | The … menu of the header | View | Built | Built |
| [`graph.load-more`](02-graph.md#graphload-more--load-the-next-commits-while-scrolling) | Load the next commits while scrolling | View | Built | Built |
| [`graph.lanes`](02-graph.md#graphlanes--lay-out-the-lines-of-the-graph) | Lay out the lines of the graph | Derivation | Built | Built |
| [`graph.fit-width`](02-graph.md#graphfit-width--how-wide-the-graph-is-drawn) | How wide the graph is drawn | Derivation | Built | Built |
| [`graph.resize-graph`](02-graph.md#graphresize-graph--give-the-graph-less-room-or-more) | Give the graph less room or more | View | Built | Built |
| [`graph.resize-panes`](02-graph.md#graphresize-panes--resize-the-branch-rail-and-the-inspector) | Resize the branch rail and the inspector | View | Built | Built |
| [`graph.toggle-references`](02-graph.md#graphtoggle-references--put-the-branch-rail-away) | Put the branch rail away | View | Built | Built |
| [`graph.labels`](02-graph.md#graphlabels--the-labels-of-each-commit) | The labels of each commit | Derivation | Built | Built |
| [`graph.combine-upstream`](02-graph.md#graphcombine-upstream--a-branch-and-its-upstream-on-one-label) | A branch and its upstream on one label | Derivation | Built | Built |
| [`graph.pull-request-badge`](02-graph.md#graphpull-request-badge--pull-request-on-a-label) | Pull request on a label | View | Built | Built |
| [`graph.unpushed`](02-graph.md#graphunpushed--commits-not-pushed-yet) | Commits not pushed yet | Derivation | Built | Built |
| [`graph.trace-history`](02-graph.md#graphtrace-history--the-history-of-a-commit-among-the-rows) | The history of a commit among the rows | Derivation | Built | Built |
| [`graph.hover-history`](02-graph.md#graphhover-history--the-history-of-a-label-under-the-pointer) | The history of a label under the pointer | View | Built | Built |
| [`graph.trace-selected`](02-graph.md#graphtrace-selected--the-history-of-the-selected-commit) | The history of the selected commit | View | Built | Built |
| [`graph.columns`](02-graph.md#graphcolumns--what-is-shown-with-each-message) | What is shown with each message | View | Built | Built |
| [`graph.date-separators`](02-graph.md#graphdate-separators--lines-between-days-weeks-and-months) | Lines between days, weeks and months | Derivation | Built | Built |
| [`graph.first-parent`](02-graph.md#graphfirst-parent--only-the-first-parent-of-merges) | Only the first parent of merges | View | Built | Built |
| [`graph.fold-rows`](02-graph.md#graphfold-rows--the-rows-of-the-timeline-with-merges-folded) | The rows of the timeline with merges folded | Derivation | Built | Built |
| [`graph.fold-merge`](02-graph.md#graphfold-merge--fold-the-commits-a-merge-brought-in) | Fold the commits a merge brought in | View | Built | Built |
| [`graph.fold-all`](02-graph.md#graphfold-all--fold-all-merges) | Fold all merges | View | Built | Built |
| [`graph.lane-tips`](02-graph.md#graphlane-tips--the-branch-each-line-comes-from) | The branch each line comes from | Derivation | Built | Built |
| [`graph.sticky-branch-names`](02-graph.md#graphsticky-branch-names--branch-names-that-stay-at-the-top) | Branch names that stay at the top | View | Built | Built |
| [`graph.scroll-marks`](02-graph.md#graphscroll-marks--marks-beside-the-scroll-bar) | Marks beside the scroll bar | View | Built | Built |
| [`graph.keyboard`](02-graph.md#graphkeyboard--move-through-the-timeline-with-the-keyboard) | Move through the timeline with the keyboard | View | Built | Built |
| [`graph.uncommitted-row`](02-graph.md#graphuncommitted-row--the-draft-at-the-top-of-the-timeline) | The draft at the top of the timeline | View | Built | Built |
| [`graph.place-stashes`](02-graph.md#graphplace-stashes--where-the-stashes-go-among-the-commits) | Where the stashes go among the commits | Derivation | Built | Built |
| [`graph.stashes`](02-graph.md#graphstashes--stashes-in-the-timeline) | Stashes in the timeline | View | Built | Built |
| [`graph.show-in-graph`](02-graph.md#graphshow-in-graph--go-to-a-commit-in-the-timeline) | Go to a commit in the timeline | View | Built | Built |
| [`refs.tree`](02-graph.md#refstree--the-branch-rail) | The branch rail | View | Built | Built |
| [`refs.filter`](02-graph.md#refsfilter--filter-the-branch-rail-by-name) | Filter the branch rail by name | View | Built | Built |
| [`refs.scope-line`](02-graph.md#refsscope-line--the-line-of-the-branches-shown-at-the-top-of-the-branch-rail) | The line of the branches shown, at the top of the branch rail | View | Built | Built |
| [`refs.sort`](02-graph.md#refssort--sort-the-branches-and-tags) | Sort the branches and tags | Derivation | Built | Built |
| [`refs.recent`](02-graph.md#refsrecent--the-branches-checked-out-last) | The branches checked out last | Derivation | Built | Built |
| [`refs.favorite`](02-graph.md#refsfavorite--add-a-branch-to-favorites) | Add a branch to favorites | View | Built | Built |

## Search and filter

| Feature | Title | Kind | Desktop | Extension |
| --- | --- | --- | --- | --- |
| [`search.box`](03-search-and-filter.md#searchbox--the-search-box) | The search box | View | Built | Built |
| [`search.loaded`](03-search-and-filter.md#searchloaded--find-among-the-loaded-commits) | Find among the loaded commits | Derivation | Built | Built |
| [`search.read-matches`](03-search-and-filter.md#searchread-matches--search-the-whole-history) | Search the whole history | Read | Built | Built |
| [`search.history`](03-search-and-filter.md#searchhistory--find-in-the-whole-history) | Find in the whole history | View | Built | Built |
| [`search.step`](03-search-and-filter.md#searchstep--step-through-the-matches) | Step through the matches | Derivation | Built | Built |
| [`search.list-mode`](03-search-and-filter.md#searchlist-mode--only-the-commits-found-as-a-list) | Only the commits found, as a list | View | Built | Built |
| [`search.go-to-hash`](03-search-and-filter.md#searchgo-to-hash--go-to-a-commit-by-its-hash) | Go to a commit by its hash | View | Built | Built |
| [`search.read-revision`](03-search-and-filter.md#searchread-revision--the-commit-a-name-stands-for) | The commit a name stands for | Read | Built | Built |
| [`search.go-to-commit`](03-search-and-filter.md#searchgo-to-commit--go-to-commit) | Go to commit… | Operation | Built | Built |
| [`filter.picker`](03-search-and-filter.md#filterpicker--choose-the-branches-shown) | Choose the branches shown | View | Built | Built |
| [`filter.label`](03-search-and-filter.md#filterlabel--what-the-branches-shown-are-called) | What the branches shown are called | Derivation | Built | Built |
| [`filter.preset`](03-search-and-filter.md#filterpreset--all-current-upstream-local-remotes) | All, Current, Upstream, Local, Remotes | Derivation | Built | Built |
| [`filter.tick`](03-search-and-filter.md#filtertick--tick-the-branches-to-show) | Tick the branches to show | Derivation | Built | Built |
| [`filter.rank-names`](03-search-and-filter.md#filterrank-names--which-names-match-best) | Which names match best | Derivation | Built | Built |
| [`filter.list-too`](03-search-and-filter.md#filterlist-too--filter-the-list-of-branches-too) | Filter the list of branches too | View | Built | Built |
| [`filter.kinds`](03-search-and-filter.md#filterkinds--show-local-branches-remote-branches-tags) | Show local branches, remote branches, tags | Derivation | Built | Built |
| [`filter.show-only`](03-search-and-filter.md#filtershow-only--show-only-this-history) | Show only this history | Derivation | Built | Built |
| [`filter.show-too`](03-search-and-filter.md#filtershow-too--show-this-history-too) | Show this history too | Derivation | Built | Built |
| [`filter.hide`](03-search-and-filter.md#filterhide--hide-this-history) | Hide this history | Derivation | Built | Built |
| [`filter.show-all`](03-search-and-filter.md#filtershow-all--show-all-branches) | Show all branches | Derivation | Built | Built |
| [`filter.bar`](03-search-and-filter.md#filterbar--the-bar-of-what-the-timeline-is-narrowed-to) | The bar of what the timeline is narrowed to | View | Built | Built |
| [`filter.log-arguments`](03-search-and-filter.md#filterlog-arguments--which-history-the-timeline-lists) | Which history the timeline lists | Derivation | Built | Built |
| [`filter.remember`](03-search-and-filter.md#filterremember--the-branches-shown-remembered-per-repository) | The branches shown, remembered per repository | Derivation | Built | Built |

## Details, commits and the page of a commit

| Feature | Title | Kind | Desktop | Extension |
| --- | --- | --- | --- | --- |
| [`details.read-commit`](04-details-and-commit-page.md#detailsread-commit--read-the-details-of-a-commit) | Read the details of a commit | Read | Built | Built |
| [`details.show`](04-details-and-commit-page.md#detailsshow--details-of-the-selected-commit) | Details of the selected commit | View | Built | Built |
| [`details.message-title`](04-details-and-commit-page.md#detailsmessage-title--the-title-of-a-message) | The title of a message | Derivation | Built | Built |
| [`details.message-unwrap`](04-details-and-commit-page.md#detailsmessage-unwrap--lines-of-a-message-put-back-together) | Lines of a message put back together | Derivation | Built | Built |
| [`details.show-more`](04-details-and-commit-page.md#detailsshow-more--show-more-of-a-long-message) | Show more of a long message | View | Built | Built |
| [`details.file-tree`](04-details-and-commit-page.md#detailsfile-tree--files-as-a-tree-of-directories) | Files as a tree of directories | Derivation | Built | Built |
| [`details.show-all-files`](04-details-and-commit-page.md#detailsshow-all-files--the-first-300-files-and-show-all) | The first 300 files, and Show all | View | Built | Built |
| [`details.compare-file-with`](04-details-and-commit-page.md#detailscompare-file-with--compare-a-file-with-a-branch-tag-or-commit) | Compare a file with a branch, tag or commit | Operation | Built | Built |
| [`details.stash`](04-details-and-commit-page.md#detailsstash--details-of-a-stash) | Details of a stash | View | Built | Built |
| [`commits.copy-hash`](04-details-and-commit-page.md#commitscopy-hash--copy-the-hash-of-a-commit) | Copy the hash of a commit | Operation | Built | Built |
| [`commits.copy-message`](04-details-and-commit-page.md#commitscopy-message--copy-the-message-of-a-commit) | Copy the message of a commit | Operation | Built | Planned |
| [`commit-page.open`](04-details-and-commit-page.md#commit-pageopen--open-a-commit-as-a-page) | Open a commit as a page | Operation | Built | Built |
| [`commit-page.read-patch`](04-details-and-commit-page.md#commit-pageread-patch--read-the-lines-a-commit-changed) | Read the lines a commit changed | Read | Built | Built |
| [`commit-page.diff-lines`](04-details-and-commit-page.md#commit-pagediff-lines--the-lines-of-each-diff-and-how-many-are-sent) | The lines of each diff, and how many are sent | Derivation | Built | Built |
| [`commit-page.view`](04-details-and-commit-page.md#commit-pageview--the-page-of-a-commit) | The page of a commit | View | Built | Built |
| [`commit-page.initial-folds`](04-details-and-commit-page.md#commit-pageinitial-folds--which-diffs-start-folded) | Which diffs start folded | Derivation | Built | Built |
| [`commit-page.show-in-graph`](04-details-and-commit-page.md#commit-pageshow-in-graph--show-a-commit-in-the-timeline) | Show a commit in the timeline | View | Built | Built |

## Changes and commit

| Feature | Title | Kind | Desktop | Extension |
| --- | --- | --- | --- | --- |
| [`changes.pane`](05-changes-and-commit.md#changespane--the-changes-page) | The Changes page | View | Built | Built |
| [`changes.read-working-tree`](05-changes-and-commit.md#changesread-working-tree--read-what-is-not-committed) | Read what is not committed | Read | Built | Built |
| [`changes.list`](05-changes-and-commit.md#changeslist--one-list-of-the-changed-files) | One list of the changed files | Derivation | Built | Built |
| [`changes.unsaved`](05-changes-and-commit.md#changesunsaved--files-with-edits-not-saved) | Files with edits not saved | Derivation | Built | Built |
| [`changes.open`](05-changes-and-commit.md#changesopen--what-a-file-of-the-changes-page-opens) | What a file of the Changes page opens | Derivation | Built | Built |
| [`changes.stage`](05-changes-and-commit.md#changesstage--stage-a-file) | Stage a file | Operation | Built | Built |
| [`changes.unstage`](05-changes-and-commit.md#changesunstage--unstage-a-file) | Unstage a file | Operation | Built | Built |
| [`changes.stage-all`](05-changes-and-commit.md#changesstage-all--stage-all-changes) | Stage all changes | Operation | Built | Differs (A7) |
| [`changes.unstage-all`](05-changes-and-commit.md#changesunstage-all--unstage-all-changes) | Unstage all changes | Operation | Built | Differs (A1, A2) |
| [`changes.discard`](05-changes-and-commit.md#changesdiscard--discard-the-changes-of-a-file) | Discard the changes of a file | Operation | Built | Differs (A3) |
| [`changes.discard-all`](05-changes-and-commit.md#changesdiscard-all--discard-all-changes) | Discard all changes | Operation | Built | Differs (A3) |
| [`changes.save`](05-changes-and-commit.md#changessave--save-a-file-or-all-of-them) | Save a file, or all of them | Operation | Built | Host only |
| [`changes.pathspec`](05-changes-and-commit.md#changespathspec--a-file-named-as-itself) | A file named as itself | Derivation | Built | Built |
| [`commit.create`](05-changes-and-commit.md#commitcreate--commit) | Commit | Operation | Built | Built |
| [`commit.amend`](05-changes-and-commit.md#commitamend--amend-the-last-commit) | Amend the last commit | Operation | Built | Differs (A7) |
| [`commit.draft-message`](05-changes-and-commit.md#commitdraft-message--draft-the-commit-message) | Draft the commit message | Derivation | Built | Built |
| [`identity.read`](05-changes-and-commit.md#identityread--read-the-identity) | Read the identity | Read | Built | Built |
| [`identity.switch`](05-changes-and-commit.md#identityswitch--switch-identity) | Switch identity | Operation | Built | Differs (B9) |
| [`identity.use-global`](05-changes-and-commit.md#identityuse-global--use-the-global-identity-again) | Use the global identity again | Operation | Built | Built |

## Branches, tags, remotes and sync

| Feature | Title | Kind | Desktop | Extension |
| --- | --- | --- | --- | --- |
| [`branch.checkout`](06-branches-tags-remotes.md#branchcheckout--check-out-a-branch) | Check out a branch | Operation | Built | Built |
| [`branch.open`](06-branches-tags-remotes.md#branchopen--what-a-double-click-on-a-branch-does) | What a double click on a branch does | Derivation | Built | Built |
| [`branch.checkout-remote`](06-branches-tags-remotes.md#branchcheckout-remote--check-out-a-remote-branch) | Check out a remote branch | Operation | Built | Differs (B2, B6) |
| [`branch.checkout-remote-and-rebase`](06-branches-tags-remotes.md#branchcheckout-remote-and-rebase--check-out-a-branch-and-rebase-it-onto-its-remote-branch) | Check out a branch and rebase it onto its remote branch | Operation | Built | Built |
| [`branch.checkout-detached`](06-branches-tags-remotes.md#branchcheckout-detached--check-out-a-commit) | Check out a commit | Operation | Built | Built |
| [`branch.create`](06-branches-tags-remotes.md#branchcreate--create-a-branch) | Create a branch | Operation | Built | Built |
| [`branch.format-name`](06-branches-tags-remotes.md#branchformat-name--expand-the-format-of-branch-names) | Expand the format of branch names | Derivation | Built | Built |
| [`branch.name-from-format`](06-branches-tags-remotes.md#branchname-from-format--ask-for-the-parts-of-the-format) | Ask for the parts of the format | Derivation | Built | Built |
| [`branch.validate-name`](06-branches-tags-remotes.md#branchvalidate-name--what-git-does-not-take-as-a-name) | What git does not take as a name | Derivation | Built | Built |
| [`branch.ref-argument`](06-branches-tags-remotes.md#branchref-argument--a-branch-named-in-full-when-it-has-a-namesake) | A branch named in full when it has a namesake | Derivation | Built | Built |
| [`branch.rename`](06-branches-tags-remotes.md#branchrename--rename-a-branch) | Rename a branch | Operation | Built | Built |
| [`branch.delete`](06-branches-tags-remotes.md#branchdelete--delete-a-branch) | Delete a branch | Operation | Built | Differs (B3, D9) |
| [`branch.clean-up`](06-branches-tags-remotes.md#branchclean-up--clean-up-branches) | Clean up branches | Operation | Built | Built |
| [`branch.fast-forward`](06-branches-tags-remotes.md#branchfast-forward--fast-forward-a-branch-that-is-not-checked-out) | Fast-forward a branch that is not checked out | Operation | Built | Built |
| [`branch.pane`](06-branches-tags-remotes.md#branchpane--the-branch-rail-its-buttons-and-its-comparison) | The branch rail, its buttons and its comparison | View | Built | Built |
| [`tag.create`](06-branches-tags-remotes.md#tagcreate--create-a-tag) | Create a tag | Operation | Built | Built |
| [`tag.push`](06-branches-tags-remotes.md#tagpush--push-a-tag) | Push a tag | Operation | Built | Built |
| [`tag.delete`](06-branches-tags-remotes.md#tagdelete--delete-a-tag) | Delete a tag | Operation | Built | Built |
| [`tag.checkout`](06-branches-tags-remotes.md#tagcheckout--check-out-a-tag) | Check out a tag | Operation | Built | Built |
| [`remote.read-remotes`](06-branches-tags-remotes.md#remoteread-remotes--read-the-remotes) | Read the remotes | Read | Built | Built |
| [`remote.default`](06-branches-tags-remotes.md#remotedefault--the-remote-a-new-upstream-or-a-tag-goes-to) | The remote a new upstream or a tag goes to | Derivation | Built | Built |
| [`remote.manage`](06-branches-tags-remotes.md#remotemanage--manage-remotes) | Manage remotes | Operation | Built | Built |
| [`remote.fetch`](06-branches-tags-remotes.md#remotefetch--fetch-one-remote) | Fetch one remote | Operation | Built | Built |
| [`remote.set-url`](06-branches-tags-remotes.md#remoteset-url--change-the-url-of-a-remote) | Change the URL of a remote | Operation | Built | Built |
| [`remote.rename`](06-branches-tags-remotes.md#remoterename--rename-a-remote) | Rename a remote | Operation | Built | Built |
| [`remote.remove`](06-branches-tags-remotes.md#remoteremove--remove-a-remote) | Remove a remote | Operation | Built | Built |
| [`remote.add`](06-branches-tags-remotes.md#remoteadd--add-a-remote) | Add a remote | Operation | Built | Built |
| [`remote.delete-branch`](06-branches-tags-remotes.md#remotedelete-branch--delete-a-branch-from-its-remote) | Delete a branch from its remote | Operation | Built | Differs (B7) |
| [`sync.fetch`](06-branches-tags-remotes.md#syncfetch--fetch) | Fetch | Operation | Built | Built |
| [`sync.pull`](06-branches-tags-remotes.md#syncpull--pull) | Pull | Operation | Built | Built |
| [`sync.pull-with`](06-branches-tags-remotes.md#syncpull-with--pull-one-way-or-another) | Pull one way or another | Operation | Built | Built |
| [`sync.push`](06-branches-tags-remotes.md#syncpush--push) | Push | Operation | Built | Built |
| [`sync.sync`](06-branches-tags-remotes.md#syncsync--sync-with-the-upstream) | Sync with the upstream | Operation | Built | Planned |
| [`sync.read-overwritten`](06-branches-tags-remotes.md#syncread-overwritten--read-what-a-force-push-would-overwrite) | Read what a force push would overwrite | Read | Built | Built |
| [`sync.push-force-with-lease`](06-branches-tags-remotes.md#syncpush-force-with-lease--push-with---force-with-lease) | Push with `--force-with-lease` | Operation | Built | Built |
| [`sync.push-force`](06-branches-tags-remotes.md#syncpush-force--push-with---force) | Push with `--force` | Operation | Built | Built |

## History rewriting

| Feature | Title | Kind | Desktop | Extension |
| --- | --- | --- | --- | --- |
| [`selection.select`](07-history-rewriting.md#selectionselect--select-several-commits) | Select several commits | View | Built | Built |
| [`selection.bar`](07-history-rewriting.md#selectionbar--the-bar-of-a-selection-of-several-commits) | The bar of a selection of several commits | View | Built | Built |
| [`selection.menu`](07-history-rewriting.md#selectionmenu--the-context-menu-of-a-selection-of-several-commits) | The context menu of a selection of several commits | View | Built | Built |
| [`selection.cherry-pick`](07-history-rewriting.md#selectioncherry-pick--cherry-pick-several-commits) | Cherry-pick several commits | Operation | Built | Built |
| [`cherry-pick.commit`](07-history-rewriting.md#cherry-pickcommit--cherry-pick-a-commit) | Cherry-pick a commit | Operation | Built | Differs (C2) |
| [`cherry-pick.skip-empty`](07-history-rewriting.md#cherry-pickskip-empty--get-past-a-commit-that-changes-nothing) | Get past a commit that changes nothing | Operation | Built | Differs (D12) |
| [`revert.commit`](07-history-rewriting.md#revertcommit--revert-a-commit) | Revert a commit | Operation | Built | Differs (C1, C2) |
| [`reset.to-commit`](07-history-rewriting.md#resetto-commit--reset-the-current-branch-to-a-commit) | Reset the current branch to a commit | Operation | Built | Built |
| [`merge.read-predicted-conflicts`](07-history-rewriting.md#mergeread-predicted-conflicts--predict-the-conflicts-of-a-merge) | Predict the conflicts of a merge | Read | Built | Built |
| [`merge.into-current`](07-history-rewriting.md#mergeinto-current--merge-into-the-current-branch) | Merge into the current branch | Operation | Built | Built |
| [`merge.choose-way`](07-history-rewriting.md#mergechoose-way--merge-into-the-current-branch-choosing-how) | Merge into the current branch, choosing how | Operation | Built | Differs (D9) |
| [`merge.undo-squash`](07-history-rewriting.md#mergeundo-squash--take-back-a-squash-merge) | Take back a squash merge | Operation | Built | Differs (D9) |
| [`rebase.onto`](07-history-rewriting.md#rebaseonto--rebase-the-current-branch-onto-a-commit-or-a-branch) | Rebase the current branch onto a commit or a branch | Operation | Built | Built |
| [`rebase.branch-onto`](07-history-rewriting.md#rebasebranch-onto--rebase-another-local-branch-onto-a-commit) | Rebase another local branch onto a commit | Operation | Built | Differs (C4) |
| [`rebase.find-line-before-head`](07-history-rewriting.md#rebasefind-line-before-head--the-line-of-commits-an-interactive-rebase-can-rewrite) | The line of commits an interactive rebase can rewrite | Derivation | Built | Built |
| [`rebase.read-pushed`](07-history-rewriting.md#rebaseread-pushed--where-a-commit-was-pushed) | Where a commit was pushed | Read | Built | Built |
| [`rebase.todo-page`](07-history-rewriting.md#rebasetodo-page--the-interactive-rebase-page) | The interactive rebase page | View | Built | Built |
| [`rebase.build-todo-list`](07-history-rewriting.md#rebasebuild-todo-list--the-todo-list-of-an-interactive-rebase) | The todo list of an interactive rebase | Derivation | Built | Built |
| [`rebase.interactive`](07-history-rewriting.md#rebaseinteractive--run-an-interactive-rebase) | Run an interactive rebase | Operation | Built | Differs (C5, C8) |
| [`rebase.read-stop`](07-history-rewriting.md#rebaseread-stop--where-an-interactive-rebase-stopped-to-change-a-commit) | Where an interactive rebase stopped to change a commit | Read | Built | Built |
| [`rebase.edit-stop`](07-history-rewriting.md#rebaseedit-stop--the-banner-of-a-rebase-stopped-to-edit-a-commit) | The banner of a rebase stopped to edit a commit | View | Built | Built |
| [`message.edit`](07-history-rewriting.md#messageedit--edit-a-commit-message) | Edit a commit message | Operation | Built | Differs (C5, C6, C7) |
| [`squash.join-messages`](07-history-rewriting.md#squashjoin-messages--the-message-of-commits-squashed-into-one) | The message of commits squashed into one | Derivation | Built | Built |
| [`squash.commits`](07-history-rewriting.md#squashcommits--squash-commits-into-one) | Squash commits into one | Operation | Built | Differs (C5) |
| [`squash.into-previous`](07-history-rewriting.md#squashinto-previous--squash-a-commit-into-the-commit-before-it) | Squash a commit into the commit before it | Operation | Built | Differs (C5) |
| [`split.commit`](07-history-rewriting.md#splitcommit--split-a-commit) | Split a commit | Operation | Built | Differs (C5) |
| [`split.continue`](07-history-rewriting.md#splitcontinue--continue-a-split) | Continue a split | Operation | Built | Built |
| [`fixup.commit`](07-history-rewriting.md#fixupcommit--add-changes-to-an-earlier-commit) | Add changes to an earlier commit | Operation | Built | Differs (A6) |
| [`drag.list-drop-entries`](07-history-rewriting.md#draglist-drop-entries--what-a-drop-offers) | What a drop offers | Derivation | Built | Built |
| [`drag.drop`](07-history-rewriting.md#dragdrop--drag-a-commit-or-a-branch-onto-the-timeline) | Drag a commit or a branch onto the timeline | View | Built | Built |

## Stashes

| Feature | Title | Kind | Desktop | Extension |
| --- | --- | --- | --- | --- |
| [`stash.read-list`](08-stashes.md#stashread-list--read-the-stashes) | Read the stashes | Read | Built | Built |
| [`stash.read-files`](08-stashes.md#stashread-files--read-what-a-stash-changes) | Read what a stash changes | Read | Built | Built |
| [`stash.pane`](08-stashes.md#stashpane--the-stashes-list) | The Stashes list | View | Built | Built |
| [`stash.push`](08-stashes.md#stashpush--stash-all-changes) | Stash all changes | Operation | Built | Differs (A9, D7) |
| [`stash.push-staged`](08-stashes.md#stashpush-staged--stash-the-staged-changes) | Stash the staged changes | Operation | Built | Differs (A9, D7) |
| [`stash.push-file`](08-stashes.md#stashpush-file--stash-a-file-or-the-files-ticked) | Stash a file, or the files ticked | Operation | Built | Differs (A8, A9, D7) |
| [`stash.preview`](08-stashes.md#stashpreview--what-applying-a-stash-would-stop-on) | What applying a stash would stop on | Derivation | Built | Differs (A12) |
| [`stash.apply`](08-stashes.md#stashapply--apply-a-stash) | Apply a stash | Operation | Built | Differs (A12) |
| [`stash.pop`](08-stashes.md#stashpop--pop-a-stash) | Pop a stash | Operation | Built | Differs (A12) |
| [`stash.drop`](08-stashes.md#stashdrop--drop-a-stash) | Drop a stash | Operation | Built | Differs (D9) |
| [`stash.undo-drop`](08-stashes.md#stashundo-drop--undo-a-stash-dropped) | Undo a stash dropped | Operation | Built | Differs (D9) |
| [`stash.branch`](08-stashes.md#stashbranch--create-a-branch-from-a-stash) | Create a branch from a stash | Operation | Built | Differs (S2) |
| [`stash.in-graph`](08-stashes.md#stashin-graph--stashes-in-the-timeline) | Stashes in the timeline | Derivation | Built | Built |
| [`stash.autostash-left`](08-stashes.md#stashautostash-left--the-stash-a-pull-leaves) | The stash a pull leaves | Derivation | Built | Built |

## Conflicts

| Feature | Title | Kind | Desktop | Extension |
| --- | --- | --- | --- | --- |
| [`conflicts.banner`](09-conflicts.md#conflictsbanner--the-banner-of-an-operation-that-stopped) | The banner of an operation that stopped | View | Built | Built |
| [`conflicts.continue-operation`](09-conflicts.md#conflictscontinue-operation--continue) | Continue | Operation | Built | Differs (D1, D12) |
| [`conflicts.skip-step`](09-conflicts.md#conflictsskip-step--skip) | Skip | Operation | Built | Differs (D2, D12) |
| [`conflicts.abort-operation`](09-conflicts.md#conflictsabort-operation--abort) | Abort | Operation | Built | Built |
| [`conflicts.stopped`](09-conflicts.md#conflictsstopped--the-message-when-an-operation-stops-on-conflicts) | The message when an operation stops on conflicts | View | Built | Built |
| [`conflicts.read-files`](09-conflicts.md#conflictsread-files--the-files-in-conflict-and-their-sides) | The files in conflict and their sides | Read | Built | Built |
| [`conflicts.side-names`](09-conflicts.md#conflictsside-names--what-the-two-sides-are) | What the two sides are | Derivation | Built | Built |
| [`conflicts.open-file`](09-conflicts.md#conflictsopen-file--open-a-file-in-conflict) | Open a file in conflict | View | Built | Built |
| [`conflicts.mark-resolved`](09-conflicts.md#conflictsmark-resolved--mark-as-resolved) | Mark as resolved | Operation | Built | Built |
| [`conflicts.take-side`](09-conflicts.md#conflictstake-side--take-the-current-side-take-the-incoming-side) | Take the current side, take the incoming side | Operation | Built | Differs (D6) |
| [`conflicts.parse-markers`](09-conflicts.md#conflictsparse-markers--the-conflicts-git-marked-in-a-file) | The conflicts git marked in a file | Derivation | Built | Built |
| [`conflicts.resolve-by-rules`](09-conflicts.md#conflictsresolve-by-rules--the-conflicts-that-need-no-choice) | The conflicts that need no choice | Derivation | Built | Built |
| [`conflicts.resolve-automatically`](09-conflicts.md#conflictsresolve-automatically--resolve-conflicts-automatically) | Resolve conflicts automatically | Operation | Built | Differs (D3) |
| [`conflicts.restore`](09-conflicts.md#conflictsrestore--undo-a-resolution) | Undo a resolution | Operation | Built | Differs (D9) |
| [`review.page`](09-conflicts.md#reviewpage--the-review-of-the-resolutions) | The review of the resolutions | View | Built | Built |
| [`review.file-list`](09-conflicts.md#reviewfile-list--the-files-and-the-filters) | The files and the filters | View | Built | Built |
| [`review.card`](09-conflicts.md#reviewcard--a-conflict-as-a-card) | A conflict, as a card | View | Built | Built |
| [`review.side-diff`](09-conflicts.md#reviewside-diff--what-a-side-changed-of-the-base) | What a side changed of the base | Derivation | Built | Built |
| [`review.fingerprint`](09-conflicts.md#reviewfingerprint--the-same-conflict-wherever-it-is) | The same conflict, wherever it is | Derivation | Built | Built |
| [`review.proposals`](09-conflicts.md#reviewproposals--what-is-proposed-for-each-conflict) | What is proposed for each conflict | Derivation | Built | Built |
| [`review.proposal-lines`](09-conflicts.md#reviewproposal-lines--where-the-lines-of-a-proposal-come-from) | Where the lines of a proposal come from | Derivation | Built | Built |
| [`review.proposal-warnings`](09-conflicts.md#reviewproposal-warnings--what-looks-like-a-mistake-in-a-proposal) | What looks like a mistake in a proposal | Derivation | Built | Built |
| [`review.choose`](09-conflicts.md#reviewchoose--choose-a-side-or-write-the-lines) | Choose a side, or write the lines | View | Built | Built |
| [`review.accept`](09-conflicts.md#reviewaccept--accept) | Accept | View | Built | Built |
| [`review.accept-file`](09-conflicts.md#reviewaccept-file--accept-the-file) | Accept the file | View | Built | Built |
| [`review.accept-all`](09-conflicts.md#reviewaccept-all--accept-all-proposed) | Accept all proposed | View | Built | Built |
| [`review.clear`](09-conflicts.md#reviewclear--clear) | Clear | View | Built | Built |
| [`review.write`](09-conflicts.md#reviewwrite--write-the-resolutions) | Write the resolutions | Operation | Built | Differs (D4, D5) |
| [`review.report`](09-conflicts.md#reviewreport--what-was-written) | What was written | View | Built | Built |
| [`review.columns`](09-conflicts.md#reviewcolumns--columns) | Columns | View | Built | Built |
| [`review.show-result`](09-conflicts.md#reviewshow-result--show-the-result) | Show the result | View | Built | Built |
| [`review.deleted-card`](09-conflicts.md#reviewdeleted-card--a-file-deleted-on-one-side) | A file deleted on one side | View | Built | Built |
| [`ai.resolve-conflicts`](09-conflicts.md#airesolve-conflicts--resolve-conflicts-with-ai) | Resolve conflicts with AI | View | Built | Built |
| [`ai.ask-conflict`](09-conflicts.md#aiask-conflict--one-question-about-one-conflict) | One question about one conflict | Read | Built | Built |
| [`ai.conflict-question`](09-conflicts.md#aiconflict-question--what-is-sent-about-a-conflict) | What is sent about a conflict | Derivation | Built | Built |
| [`ai.read-answer`](09-conflicts.md#airead-answer--the-answer-read) | The answer, read | Derivation | Built | Built |
| [`ai.trim-answer`](09-conflicts.md#aitrim-answer--the-lines-around-the-conflict-written-again) | The lines around the conflict, written again | Derivation | Built | Built |
| [`ai.choose-model`](09-conflicts.md#aichoose-model--choose-the-model-that-resolves-conflicts) | Choose the model that resolves conflicts | Operation | Built | Built |

## Compare

| Feature | Title | Kind | Desktop | Extension |
| --- | --- | --- | --- | --- |
| [`compare.resolve-revision`](10-compare.md#compareresolve-revision--which-commit-a-revision-is) | Which commit a revision is | Read | Built | Built |
| [`compare.side-choices`](10-compare.md#compareside-choices--what-a-side-can-be-chosen-from) | What a side can be chosen from | Derivation | Built | Built |
| [`compare.recent`](10-compare.md#comparerecent--the-comparisons-opened-before) | The comparisons opened before | Derivation | Built | Built |
| [`compare.choose`](10-compare.md#comparechoose--compare-branches-tags-or-commits) | Compare branches, tags or commits | Operation | Built | Built |
| [`compare.follow-sides`](10-compare.md#comparefollow-sides--followed-sides-and-fixed-ones) | Followed sides and fixed ones | Derivation | Built | Built |
| [`compare.initial-mode`](10-compare.md#compareinitial-mode--how-two-sides-are-compared-at-first) | How two sides are compared at first | Derivation | Built | Built |
| [`compare.read-sides`](10-compare.md#compareread-sides--read-a-comparison) | Read a comparison | Read | Built | Built |
| [`compare.read-patch`](10-compare.md#compareread-patch--read-the-lines-of-a-comparison) | Read the lines of a comparison | Read | Built | Built |
| [`compare.page`](10-compare.md#comparepage--the-comparison-page) | The comparison page | View | Built | Built |
| [`compare.summary`](10-compare.md#comparesummary--how-far-apart-and-what-merging-would-do) | How far apart, and what merging would do | Derivation | Built | Built |
| [`compare.change-side`](10-compare.md#comparechange-side--choose-another-side) | Choose another side | Operation | Built | Built |
| [`compare.swap`](10-compare.md#compareswap--swap-the-two-sides) | Swap the two sides | View | Built | Built |
| [`compare.set-mode`](10-compare.md#compareset-mode--since-they-diverged-or-everything-that-differs) | Since they diverged, or everything that differs | View | Built | Built |
| [`compare.narrow-to-path`](10-compare.md#comparenarrow-to-path--narrow-a-comparison-to-a-file-or-a-folder) | Narrow a comparison to a file or a folder | View | Built | Built |
| [`compare.commits`](10-compare.md#comparecommits--the-commits-only-in-each-side) | The commits only in each side | View | Built | Built |
| [`compare.take-file`](10-compare.md#comparetake-file--take-the-version-of-the-second-side) | Take the version of the second side | Operation | Built | Built |
| [`compare.copy-patch`](10-compare.md#comparecopy-patch--copy-a-comparison-as-a-patch) | Copy a comparison as a patch | Operation | Built | Built |
| [`compare.open-on-github`](10-compare.md#compareopen-on-github--open-the-comparison-on-github) | Open the comparison on GitHub | Operation | Built | Built |
| [`compare.path-with`](10-compare.md#comparepath-with--compare-a-file-or-a-folder-with-a-branch-tag-or-commit) | Compare a file or a folder with a branch, tag or commit | Operation | Built | Built |
| [`compare.with`](10-compare.md#comparewith--compare-with) | Compare with… | Operation | Built | Built |
| [`compare.with-current`](10-compare.md#comparewith-current--compare-with-the-current-branch) | Compare with the current branch | Operation | Built | Built |
| [`compare.selected-before`](10-compare.md#compareselected-before--compare-two-commits-of-the-timeline) | Compare two commits of the timeline | Operation | Built | Built |
| [`compare.read-files`](10-compare.md#compareread-files--read-the-files-that-differ-between-two-commits) | Read the files that differ between two commits | Read | Built | Built |
| [`compare.in-place`](10-compare.md#comparein-place--the-comparison-in-the-inspector) | The comparison in the inspector | View | Built | Built |

## History and blame

| Feature | Title | Kind | Desktop | Extension |
| --- | --- | --- | --- | --- |
| [`file-history.read-names`](11-history-and-blame.md#file-historyread-names--the-names-a-file-had) | The names a file had | Read | Built | Built |
| [`file-history.file`](11-history-and-blame.md#file-historyfile--show-the-history-of-a-file) | Show the history of a file | Operation | Built | Built |
| [`file-history.read-committed-lines`](11-history-and-blame.md#file-historyread-committed-lines--the-lines-as-the-commit-checked-out-has-them) | The lines as the commit checked out has them | Read | Built | Built |
| [`file-history.map-lines`](11-history-and-blame.md#file-historymap-lines--map-lines-of-a-file-as-it-is-to-the-file-as-committed) | Map lines of a file as it is to the file as committed | Derivation | Built | Built |
| [`file-history.lines`](11-history-and-blame.md#file-historylines--show-the-history-of-some-lines) | Show the history of some lines | Operation | Built | Built |
| [`file-history.folder`](11-history-and-blame.md#file-historyfolder--show-the-history-of-a-folder) | Show the history of a folder | Operation | Built | Built |
| [`blame.read-line`](11-history-and-blame.md#blameread-line--who-last-changed-one-line) | Who last changed one line | Read | Built | Built |
| [`blame.current-line`](11-history-and-blame.md#blamecurrent-line--the-blame-of-the-line-under-the-cursor) | The blame of the line under the cursor | View | Built | Built |
| [`blame.toggle-current-line`](11-history-and-blame.md#blametoggle-current-line--toggle-the-blame-of-the-line-under-the-cursor) | Toggle the blame of the line under the cursor | Operation | Built | Built |
| [`blame.open-line-commit`](11-history-and-blame.md#blameopen-line-commit--open-the-commit-of-the-current-line) | Open the commit of the current line | Operation | Built | Built |
| [`blame.reveal-line-commit`](11-history-and-blame.md#blamereveal-line-commit--show-the-commit-of-the-current-line-in-the-timeline) | Show the commit of the current line in the timeline | Operation | Built | Built |
| [`blame.read-file`](11-history-and-blame.md#blameread-file--who-last-changed-each-line-of-a-file) | Who last changed each line of a file | Read | Built | Built |
| [`blame.format-line`](11-history-and-blame.md#blameformat-line--what-is-said-beside-each-line) | What is said beside each line | Derivation | Built | Built |
| [`blame.age-color`](11-history-and-blame.md#blameage-color--the-color-of-the-age-of-a-line) | The color of the age of a line | Derivation | Built | Built |
| [`blame.pick-pinned-line`](11-history-and-blame.md#blamepick-pinned-line--the-commit-named-on-the-first-line-in-sight) | The commit named on the first line in sight | Derivation | Built | Built |
| [`blame.mark-edited-lines`](11-history-and-blame.md#blamemark-edited-lines--lines-typed-are-not-committed-yet) | Lines typed are not committed yet | Derivation | Built | Built |
| [`blame.file`](11-history-and-blame.md#blamefile--the-blame-beside-the-lines-of-a-file) | The blame beside the lines of a file | View | Built | Built |
| [`blame.toggle-file`](11-history-and-blame.md#blametoggle-file--show-or-hide-the-blame-of-a-file) | Show or hide the blame of a file | Operation | Built | Built |
| [`blame.previous`](11-history-and-blame.md#blameprevious--blame-the-file-as-it-was-before-a-commit) | Blame the file as it was before a commit | Operation | Built | Built |
| [`blame.ignore-whitespace`](11-history-and-blame.md#blameignore-whitespace--ignore-whitespace-and-moved-lines-in-the-blame) | Ignore whitespace and moved lines in the blame | Operation | Built | Built |
| [`blame.every-file`](11-history-and-blame.md#blameevery-file--show-the-blame-in-every-file) | Show the blame in every file | Operation | Built | Built |
| [`reflog.read`](11-history-and-blame.md#reflogread--where-head-has-been) | Where `HEAD` has been | Read | Built | Built |
| [`reflog.browse`](11-history-and-blame.md#reflogbrowse--browse-the-reflog) | Browse the reflog | Operation | Built | Built |
| [`bisect.read`](11-history-and-blame.md#bisectread--the-bisect-under-way) | The bisect under way | Read | Built | Built |
| [`bisect.start`](11-history-and-blame.md#bisectstart--start-a-bisect-from-a-commit) | Start a bisect from a commit | Operation | Built | Differs (C2, C9) |
| [`bisect.mark`](11-history-and-blame.md#bisectmark--mark-a-commit-good-or-bad) | Mark a commit good or bad | Operation | Built | Differs (C2) |
| [`bisect.answer`](11-history-and-blame.md#bisectanswer--answer-for-the-commit-being-tested) | Answer for the commit being tested | Operation | Built | Differs (C9) |
| [`bisect.banner`](11-history-and-blame.md#bisectbanner--the-banner-of-a-bisect) | The banner of a bisect | View | Built | Built |
| [`bisect.marks`](11-history-and-blame.md#bisectmarks--the-marks-of-a-bisect-in-the-timeline) | The marks of a bisect in the timeline | View | Built | Built |
| [`bisect.end`](11-history-and-blame.md#bisectend--end-the-bisect) | End the bisect | Operation | Built | Differs (C9) |

## Worktrees

| Feature | Title | Kind | Desktop | Extension |
| --- | --- | --- | --- | --- |
| [`worktree.read-list`](12-worktrees.md#worktreeread-list--read-the-worktrees-and-how-they-stand) | Read the worktrees and how they stand | Read | Built | Built |
| [`worktree.folder-path`](12-worktrees.md#worktreefolder-path--where-a-new-worktree-goes) | Where a new worktree goes | Derivation | Built | Built |
| [`worktree.create`](12-worktrees.md#worktreecreate--create-a-worktree) | Create a worktree | Operation | Built | Differs (W1, W3) |
| [`worktree.create-from-branch`](12-worktrees.md#worktreecreate-from-branch--put-a-branch-in-a-new-worktree) | Put a branch in a new worktree | Operation | Built | Differs (W1) |
| [`worktree.create-at-commit`](12-worktrees.md#worktreecreate-at-commit--create-a-worktree-at-a-commit) | Create a worktree at a commit | Operation | Built | Differs (W1, W2) |
| [`worktree.section`](12-worktrees.md#worktreesection--the-worktrees-list) | The Worktrees list | View | Built | Built |
| [`worktree.open`](12-worktrees.md#worktreeopen--open-a-worktree-where-the-settings-say) | Open a worktree where the settings say | Operation | Built | Built |
| [`worktree.open-in-new-window`](12-worktrees.md#worktreeopen-in-new-window--open-a-worktree-in-a-new-window) | Open a worktree in a new window | Operation | Built | Built |
| [`worktree.open-in-editor`](12-worktrees.md#worktreeopen-in-editor--open-a-worktree-in-the-external-editor) | Open a worktree in the external editor | Operation | Built | Host only |
| [`worktree.open-in-terminal`](12-worktrees.md#worktreeopen-in-terminal--open-a-terminal-in-a-worktree) | Open a terminal in a worktree | Operation | Built | Built |
| [`worktree.reveal`](12-worktrees.md#worktreereveal--reveal-a-worktree-in-the-file-manager) | Reveal a worktree in the file manager | Operation | Built | Built |
| [`worktree.show-in-graph`](12-worktrees.md#worktreeshow-in-graph--show-the-commit-of-a-worktree-in-the-timeline) | Show the commit of a worktree in the timeline | Operation | Built | Built |
| [`worktree.show-in-gitardo`](12-worktrees.md#worktreeshow-in-gitardo--show-another-worktree-in-this-window) | Show another worktree in this window | Operation | Built | Built |
| [`worktree.banner`](12-worktrees.md#worktreebanner--the-banner-of-a-worktree-shown-in-this-window) | The banner of a worktree shown in this window | View | Built | Built |
| [`worktree.rename`](12-worktrees.md#worktreerename--rename-a-worktree) | Rename a worktree | Operation | Built | Differs (W9) |
| [`worktree.copy-path`](12-worktrees.md#worktreecopy-path--copy-the-path-of-a-worktree) | Copy the path of a worktree | Operation | Built | Built |
| [`worktree.remove`](12-worktrees.md#worktreeremove--remove-a-worktree) | Remove a worktree | Operation | Built | Differs (W6, W7, W8) |
| [`worktree.prune`](12-worktrees.md#worktreeprune--prune-missing-worktrees) | Prune missing worktrees | Operation | Built | Built |
| [`worktree.read-folder-links`](12-worktrees.md#worktreeread-folder-links--read-the-links-to-folders-outside-a-worktree) | Read the links to folders outside a worktree | Read | Built | Built |
| [`worktree.recover-held-folder`](12-worktrees.md#worktreerecover-held-folder--get-past-a-folder-that-windows-holds) | Get past a folder that Windows holds | Derivation | Built | Built |
| [`worktree.remove-empty-folders`](12-worktrees.md#worktreeremove-empty-folders--leave-no-empty-folders-behind) | Leave no empty folders behind | Derivation | Built | Built |
| [`worktree.branch-in-another-worktree`](12-worktrees.md#worktreebranch-in-another-worktree--the-branch-of-another-worktree) | The branch of another worktree | Derivation | Built | Built |
| [`worktree.open-in-worktree`](12-worktrees.md#worktreeopen-in-worktree--open-a-branch-in-a-worktree-of-its-own) | Open a branch in a worktree of its own | Operation | Built | Differs (W10) |
| [`worktree.switch`](12-worktrees.md#worktreeswitch--switch-worktree) | Switch worktree | Operation | Built | Built |
| [`worktree-setup.settings-in-effect`](12-worktrees.md#worktree-setupsettings-in-effect--what-new-worktrees-get-as-set) | What new worktrees get, as set | Derivation | Built | Differs (W4) |
| [`worktree-setup.folders-to-link`](12-worktrees.md#worktree-setupfolders-to-link--the-folders-to-link) | The folders to link | Derivation | Built | Built |
| [`worktree-setup.paths-to-copy`](12-worktrees.md#worktree-setuppaths-to-copy--the-files-and-folders-to-copy) | The files and folders to copy | Derivation | Built | Built |
| [`worktree-setup.link-and-copy-steps`](12-worktrees.md#worktree-setuplink-and-copy-steps--linking-and-copying-into-a-new-worktree) | Linking and copying into a new worktree | Derivation | Built | Differs (W4) |
| [`worktree-setup.command-steps`](12-worktrees.md#worktree-setupcommand-steps--commands-in-a-new-worktree) | Commands in a new worktree | Derivation | Built | Differs (W4, W5) |
| [`worktree-setup.read-ignored`](12-worktrees.md#worktree-setupread-ignored--read-what-git-ignores-with-its-size) | Read what git ignores, with its size | Read | Built | Built |
| [`worktree-setup.propose`](12-worktrees.md#worktree-setuppropose--what-is-proposed-for-what-git-ignores) | What is proposed for what git ignores | Derivation | Built | Built |
| [`worktree-setup.propose-commands`](12-worktrees.md#worktree-setuppropose-commands--commands-proposed-from-lock-files) | Commands proposed from lock files | Derivation | Built | Built |
| [`worktree-setup.read-what-patterns-take`](12-worktrees.md#worktree-setupread-what-patterns-take--ask-git-what-the-patterns-take) | Ask git what the patterns take | Read | Built | Built |
| [`worktree-setup.page`](12-worktrees.md#worktree-setuppage--choose-what-new-worktrees-get) | Choose what new worktrees get | View | Built | Built |
| [`worktree-setup.choose`](12-worktrees.md#worktree-setupchoose--choose-what-a-line-gets) | Choose what a line gets | Derivation | Built | Built |
| [`worktree-setup.save`](12-worktrees.md#worktree-setupsave--save-what-new-worktrees-get) | Save what new worktrees get | Operation | Built | Differs (W4, W12) |
| [`worktree-setup.remove-from`](12-worktrees.md#worktree-setupremove-from--remove-the-settings-of-new-worktrees-from-a-place) | Remove the settings of new worktrees from a place | Operation | Built | Differs (W4) |
| [`worktree-setup.edit-settings-file`](12-worktrees.md#worktree-setupedit-settings-file--edit-the-settings-of-new-worktrees-in-their-file) | Edit the settings of new worktrees in their file | Operation | Built | Differs (W4) |
| [`worktree-setup.link-into-new-worktrees`](12-worktrees.md#worktree-setuplink-into-new-worktrees--link-a-folder-into-new-worktrees) | Link a folder into new worktrees | Operation | Built | Differs (W4, W12) |
| [`worktree-setup.copy-into-new-worktrees`](12-worktrees.md#worktree-setupcopy-into-new-worktrees--copy-a-file-or-a-folder-into-new-worktrees) | Copy a file or a folder into new worktrees | Operation | Built | Differs (W4, W12) |
| [`worktree-setup.stop-linking`](12-worktrees.md#worktree-setupstop-linking--stop-linking-a-folder-into-new-worktrees) | Stop linking a folder into new worktrees | Operation | Built | Differs (W4, W12) |
| [`worktree-setup.stop-copying`](12-worktrees.md#worktree-setupstop-copying--stop-copying-into-new-worktrees) | Stop copying into new worktrees | Operation | Built | Differs (W4, W12) |
| [`worktree-setup.explain-change`](12-worktrees.md#worktree-setupexplain-change--why-a-choice-from-the-files-page-changes-nothing) | Why a choice from the Files page changes nothing | Derivation | Built | Built |
| [`worktree-setup.bring-into-worktree`](12-worktrees.md#worktree-setupbring-into-worktree--bring-ignored-files-into-a-worktree) | Bring ignored files into a worktree | Operation | Built | Differs (W4) |
| [`worktree-setup.bring-into-other-worktrees`](12-worktrees.md#worktree-setupbring-into-other-worktrees--bring-ignored-files-into-the-other-worktrees) | Bring ignored files into the other worktrees | Operation | Built | Differs (W4) |

## Submodules

| Feature | Title | Kind | Desktop | Extension |
| --- | --- | --- | --- | --- |
| [`submodule.read-list`](13-submodules.md#submoduleread-list--read-the-submodules-and-where-they-stand) | Read the submodules and where they stand | Read | Built | Built |
| [`submodule.status`](13-submodules.md#submodulestatus--where-a-submodule-stands) | Where a submodule stands | Derivation | Built | Built |
| [`submodule.section`](13-submodules.md#submodulesection--the-submodules-list) | The Submodules list | View | Built | Built |
| [`submodule.open-in-new-window`](13-submodules.md#submoduleopen-in-new-window--open-a-submodule-in-a-new-window) | Open a submodule in a new window | Operation | Built | Built |
| [`submodule.open-in-terminal`](13-submodules.md#submoduleopen-in-terminal--open-a-terminal-in-a-submodule) | Open a terminal in a submodule | Operation | Built | Built |
| [`submodule.reveal`](13-submodules.md#submodulereveal--reveal-a-submodule-in-the-file-manager) | Reveal a submodule in the file manager | Operation | Built | Built |
| [`submodule.show-in-gitardo`](13-submodules.md#submoduleshow-in-gitardo--show-a-submodule-in-this-window) | Show a submodule in this window | Operation | Built | Built |
| [`submodule.update-to-recorded`](13-submodules.md#submoduleupdate-to-recorded--update-a-submodule-to-the-recorded-commit) | Update a submodule to the recorded commit | Operation | Built | Differs (S5) |
| [`submodule.stage-recorded`](13-submodules.md#submodulestage-recorded--stage-the-commit-of-a-submodule-as-the-recorded-one) | Stage the commit of a submodule as the recorded one | Operation | Built | Differs (S5) |
| [`submodule.update-to-latest`](13-submodules.md#submoduleupdate-to-latest--update-a-submodule-to-the-latest-of-its-branch) | Update a submodule to the latest of its branch | Operation | Built | Differs (S5) |
| [`submodule.initialize`](13-submodules.md#submoduleinitialize--initialize-a-submodule) | Initialize a submodule | Operation | Built | Built |
| [`submodule.deinitialize`](13-submodules.md#submoduledeinitialize--deinitialize-a-submodule) | Deinitialize a submodule | Operation | Built | Differs (S3) |
| [`submodule.remove`](13-submodules.md#submoduleremove--remove-a-submodule) | Remove a submodule | Operation | Built | Differs (S3, S6) |
| [`submodule.copy-path`](13-submodules.md#submodulecopy-path--copy-the-path-of-a-submodule) | Copy the path of a submodule | Operation | Built | Built |
| [`submodule.copy-url`](13-submodules.md#submodulecopy-url--copy-the-url-of-a-submodule) | Copy the URL of a submodule | Operation | Built | Built |
| [`submodule.add`](13-submodules.md#submoduleadd--add-a-submodule) | Add a submodule | Operation | Built | Differs (S1, S4) |
| [`submodule.sync-urls`](13-submodules.md#submodulesync-urls--synchronize-the-urls-of-the-submodules) | Synchronize the URLs of the submodules | Operation | Built | Built |
| [`submodule.allow-file-protocol`](13-submodules.md#submoduleallow-file-protocol--clone-a-submodule-from-a-folder-on-disk) | Clone a submodule from a folder on disk | Derivation | Built | Differs (S1) |
| [`submodule.update-on-checkout`](13-submodules.md#submoduleupdate-on-checkout--update-the-submodules-after-a-checkout) | Update the submodules after a checkout | Derivation | Built | Differs (S2) |
| [`submodule.follow-changes`](13-submodules.md#submodulefollow-changes--follow-what-happens-in-the-submodules) | Follow what happens in the submodules | Derivation | Built | Built |

## Patches

| Feature | Title | Kind | Desktop | Extension |
| --- | --- | --- | --- | --- |
| [`patch.read-format-patch`](14-patches.md#patchread-format-patch--read-commits-as-one-patch) | Read commits as one patch | Read | Built | Built |
| [`patch.count-commits`](14-patches.md#patchcount-commits--how-many-commits-a-patch-has) | How many commits a patch has | Derivation | Built | Built |
| [`patch.copy`](14-patches.md#patchcopy--copy-as-patch) | Copy as patch | Operation | Built | Built |
| [`patch.save`](14-patches.md#patchsave--save-as-patch) | Save as patch… | Operation | Built | Built |
| [`patch.check`](14-patches.md#patchcheck--check-a-patch-before-applying-it) | Check a patch before applying it | Read | Built | Built |
| [`patch.apply`](14-patches.md#patchapply--apply-a-patch) | Apply a patch | Operation | Built | Built |
| [`patch.am-stopped`](14-patches.md#patcham-stopped--a-patch-that-stopped-on-a-conflict) | A patch that stopped on a conflict | View | Built | Built |

## GitHub

| Feature | Title | Kind | Desktop | Extension |
| --- | --- | --- | --- | --- |
| [`github.detect-remote`](15-github.md#githubdetect-remote--the-repository-on-github) | The repository on GitHub | Derivation | Built | Built |
| [`github.token`](15-github.md#githubtoken--read-the-github-token) | Read the GitHub token | Read | Built | Host only |
| [`github.load-pull-requests`](15-github.md#githubload-pull-requests--load-pull-requests-from-github) | Load pull requests from GitHub | Operation | Built | Built |
| [`github.read-pull-requests`](15-github.md#githubread-pull-requests--read-the-open-pull-requests) | Read the open pull requests | Read | Built | Built |
| [`github.open-pull-request`](15-github.md#githubopen-pull-request--open-or-create-the-pull-request-of-a-branch) | Open or create the pull request of a branch | Derivation | Built | Built |
| [`github.offer-after-push`](15-github.md#githuboffer-after-push--offer-to-create-a-pull-request-after-the-first-push) | Offer to create a pull request after the first push | Derivation | Built | Built |

## Undo, command preview and the log

| Feature | Title | Kind | Desktop | Extension |
| --- | --- | --- | --- | --- |
| [`preview.confirm`](16-undo-preview-log.md#previewconfirm--show-the-commands-before-they-run) | Show the commands before they run | View | Built | Built |
| [`log.record`](16-undo-preview-log.md#logrecord--log-every-command-that-changes-something) | Log every command that changes something | Operation | Built | Differs (D11) |
| [`log.show`](16-undo-preview-log.md#logshow--show-the-command-log) | Show the command log | View | Built | Host only |
| [`undo.last`](16-undo-preview-log.md#undolast--undo-the-last-operation) | Undo the last operation | Operation | Built | Differs (B5, D7, D8, D9, D10) |
| [`undo.journal`](16-undo-preview-log.md#undojournal--remember-what-undo-needs) | Remember what Undo needs | Derivation | Built | Differs (D9) |
| [`activity.track`](16-undo-preview-log.md#activitytrack--say-what-is-running) | Say what is running | Operation | Built | Built |

## Diff, Merge and File pages

| Feature | Title | Kind | Desktop | Extension |
| --- | --- | --- | --- | --- |
| [`diff.page`](17-diff-and-merge-views.md#diffpage--the-diff-page) | The Diff page | View | Built | Built |
| [`diff.read-sides`](17-diff-and-merge-views.md#diffread-sides--the-two-versions-of-a-file) | The two versions of a file | Read | Built | Built |
| [`diff.line-diff`](17-diff-and-merge-views.md#diffline-diff--the-lines-that-changed) | The lines that changed | Derivation | Built | Built |
| [`diff.word-diff`](17-diff-and-merge-views.md#diffword-diff--the-words-that-changed-in-a-line) | The words that changed in a line | Derivation | Built | Built |
| [`diff.align`](17-diff-and-merge-views.md#diffalign--the-two-sides-kept-level) | The two sides kept level | Derivation | Built | Built |
| [`diff.scroll-together`](17-diff-and-merge-views.md#diffscroll-together--the-two-sides-scroll-together) | The two sides scroll together | Derivation | Built | Built |
| [`diff.resize-sides`](17-diff-and-merge-views.md#diffresize-sides--making-one-side-wider) | Making one side wider | View | Built | Built |
| [`diff.navigate`](17-diff-and-merge-views.md#diffnavigate--the-previous-and-the-next-change) | The previous and the next change | View | Built | Built |
| [`diff.collapse-unchanged`](17-diff-and-merge-views.md#diffcollapse-unchanged--folding-unchanged-lines) | Folding unchanged lines | View | Built | Built |
| [`diff.staged-changes`](17-diff-and-merge-views.md#diffstaged-changes--which-changes-are-in-the-index-already) | Which changes are in the index already | Derivation | Built | Built |
| [`diff.change-patch`](17-diff-and-merge-views.md#diffchange-patch--the-patch-of-one-change) | The patch of one change | Derivation | Built | Built |
| [`diff.stage-change`](17-diff-and-merge-views.md#diffstage-change--stage-this-change) | Stage this change | Operation | Built | Built |
| [`diff.unstage-change`](17-diff-and-merge-views.md#diffunstage-change--unstage-this-change) | Unstage this change | Operation | Built | Built |
| [`diff.revert-change`](17-diff-and-merge-views.md#diffrevert-change--revert-this-change) | Revert this change | Operation | Built | Differs (A5) |
| [`diff.edit`](17-diff-and-merge-views.md#diffedit--editing-the-working-tree-side) | Editing the working tree side | View | Built | Built |
| [`diff.open-at-line`](17-diff-and-merge-views.md#diffopen-at-line--open-the-file-at-this-line) | Open the file at this line | View | Built | Built |
| [`diff.open-file`](17-diff-and-merge-views.md#diffopen-file--open-file) | Open file | View | Built | Built |
| [`diff.stage-file`](17-diff-and-merge-views.md#diffstage-file--stage-file) | Stage file | Operation | Built | Built |
| [`diff.unstage-file`](17-diff-and-merge-views.md#diffunstage-file--unstage-file) | Unstage file | Operation | Built | Built |
| [`diff.discard-file`](17-diff-and-merge-views.md#diffdiscard-file--discard-changes) | Discard changes | Operation | Built | Differs (A4) |
| [`merge-view.page`](17-diff-and-merge-views.md#merge-viewpage--the-merge-page) | The Merge page | View | Built | Built |
| [`merge-view.read-sides`](17-diff-and-merge-views.md#merge-viewread-sides--the-versions-of-a-file-in-conflict) | The versions of a file in conflict | Read | Built | Built |
| [`merge-view.three-way`](17-diff-and-merge-views.md#merge-viewthree-way--the-regions-of-a-file-in-conflict) | The regions of a file in conflict | Derivation | Built | Built |
| [`merge-view.take-side`](17-diff-and-merge-views.md#merge-viewtake-side--accept-a-side-for-a-region) | Accept a side for a region | View | Built | Built |
| [`merge-view.accept-simple`](17-diff-and-merge-views.md#merge-viewaccept-simple--accept-what-needs-no-choice) | Accept what needs no choice | View | Built | Built |
| [`merge-view.accept-proposals`](17-diff-and-merge-views.md#merge-viewaccept-proposals--accept-the-proposals) | Accept the proposals | View | Built | Built |
| [`merge-view.show-base`](17-diff-and-merge-views.md#merge-viewshow-base--show-the-base) | Show the base | View | Built | Built |
| [`merge-view.navigate`](17-diff-and-merge-views.md#merge-viewnavigate--changes-and-conflicts-left) | Changes and conflicts left | View | Built | Built |
| [`merge-view.open-at-line`](17-diff-and-merge-views.md#merge-viewopen-at-line--open-the-file-at-this-line) | Open the file at this line | View | Built | Built |
| [`merge-view.mark-resolved`](17-diff-and-merge-views.md#merge-viewmark-resolved--mark-as-resolved) | Mark as resolved | Operation | Built | Built |
| [`merge-view.close`](17-diff-and-merge-views.md#merge-viewclose--closing-with-unsaved-changes) | Closing with unsaved changes | View | Built | Built |
| [`file-view.page`](17-diff-and-merge-views.md#file-viewpage--the-file-page) | The File page | View | Built | Host only |
| [`file-view.read-revision`](17-diff-and-merge-views.md#file-viewread-revision--a-file-at-a-revision) | A file at a revision | Read | Built | Built |
| [`file-view.edit`](17-diff-and-merge-views.md#file-viewedit--editing-a-file) | Editing a file | View | Built | Host only |
| [`file-view.save`](17-diff-and-merge-views.md#file-viewsave--save) | Save | Operation | Built | Host only |
| [`file-view.find`](17-diff-and-merge-views.md#file-viewfind--find) | Find | View | Built | Host only |
| [`file-view.go-to-line`](17-diff-and-merge-views.md#file-viewgo-to-line--go-to-line) | Go to line | View | Built | Host only |
| [`file-view.open-external`](17-diff-and-merge-views.md#file-viewopen-external--open-in-external-editor) | Open in external editor | Operation | Built | Host only |
| [`file-view.highlight`](17-diff-and-merge-views.md#file-viewhighlight--the-colors-of-the-code) | The colors of the code | Derivation | Built | Built |

## Settings

| Feature | Title | Kind | Desktop | Extension |
| --- | --- | --- | --- | --- |
| [`settings.read`](18-settings.md#settingsread--read-the-settings) | Read the settings | Read | Built | Host only |
| [`settings.edit`](18-settings.md#settingsedit--open-the-settings-page) | Open the settings page | View | Built | Host only |
| [`settings.theme`](18-settings.md#settingstheme--choose-the-theme-and-the-fonts) | Choose the theme and the fonts | Operation | Built | Host only |

## The extension's identifiers

Each card of [the extension's catalog](extension/README.md), in the order of its index, and the cards of this
catalog it is an alias of ([D44](../08-decisions.md#d44--one-catalog-the-desktops-with-a-status-for-each-app)).

| Extension card | Desktop card | Note |
| --- | --- | --- |
| `repo.select` ([card](extension/01-repositories.md#reposelect--choose-the-repository-shown)) | `repo.select` | |
| `repo.follow-source-control` ([card](extension/01-repositories.md#repofollow-source-control--follow-the-repositories-view-of-vs-code)) | — | Host only |
| `changes.view` ([card](extension/05-changes-and-commit.md#changesview--the-changes-view)) | `changes.pane` | |
| `changes.stage` ([card](extension/05-changes-and-commit.md#changesstage--stage-a-file)) | `changes.stage` | |
| `changes.unstage` ([card](extension/05-changes-and-commit.md#changesunstage--unstage-a-file)) | `changes.unstage` | |
| `changes.stage-all` ([card](extension/05-changes-and-commit.md#changesstage-all--stage-all-changes)) | `changes.stage-all` | |
| `changes.unstage-all` ([card](extension/05-changes-and-commit.md#changesunstage-all--unstage-all-changes)) | `changes.unstage-all` | |
| `changes.stage-hunk` ([card](extension/05-changes-and-commit.md#changesstage-hunk--stage-or-unstage-one-change-of-a-file)) | `diff.stage-change`, `diff.unstage-change` | No view posts it; the Flow Diff stages one change |
| `changes.discard` ([card](extension/05-changes-and-commit.md#changesdiscard--discard-the-changes-of-a-file)) | `changes.discard` | |
| `changes.discard-all` ([card](extension/05-changes-and-commit.md#changesdiscard-all--discard-all-changes)) | `changes.discard-all` | |
| `changes.open` ([card](extension/05-changes-and-commit.md#changesopen--open-a-file-or-its-diff)) | `changes.open` | VS Code's editors in place of the pages |
| `changes.open-all` ([card](extension/05-changes-and-commit.md#changesopen-all--open-all-changes-in-one-editor)) | `changes.open` | VS Code's multi-diff editor in place of one Diff page |
| `changes.layout` ([card](extension/05-changes-and-commit.md#changeslayout--list-the-files-or-show-them-as-a-tree)) | `changes.pane` | The list or tree switch of the page |
| `changes.unsaved` ([card](extension/05-changes-and-commit.md#changesunsaved--files-with-edits-not-saved)) | `changes.unsaved` | |
| `changes.save` ([card](extension/05-changes-and-commit.md#changessave--save-a-file-from-the-changes-view)) | `changes.save` | VS Code saves its own editors |
| `changes.compare-with-saved` ([card](extension/05-changes-and-commit.md#changescompare-with-saved--compare-with-the-saved-file)) | `changes.open` | VS Code's diff editor in place of the Diff page |
| `commit.box` ([card](extension/05-changes-and-commit.md#commitbox--the-commit-box)) | `changes.pane` | The commit box of the page |
| `commit.create` ([card](extension/05-changes-and-commit.md#commitcreate--commit)) | `commit.create` | |
| `commit.amend` ([card](extension/05-changes-and-commit.md#commitamend--amend-the-last-commit)) | `commit.amend` | |
| `commit.start-message` ([card](extension/05-changes-and-commit.md#commitstart-message--the-message-the-box-starts-from)) | `commit.create` | Its paragraph on the message prepared by the repository |
| `commit.draft-message` ([card](extension/05-changes-and-commit.md#commitdraft-message--draft-the-message-with-the-language-model)) | `commit.draft-message` | VS Code's language models in place of the AI provider |
| `branch.checkout` ([card](extension/06-branches-tags-remotes.md#branchcheckout--check-out-a-branch)) | `branch.checkout` | |
| `branch.checkout-remote` ([card](extension/06-branches-tags-remotes.md#branchcheckout-remote--check-out-a-remote-branch-in-the-local-branch-of-its-name)) | `branch.checkout-remote`, `branch.checkout-remote-and-rebase` | |
| `branch.checkout-detached` ([card](extension/06-branches-tags-remotes.md#branchcheckout-detached--check-out-a-commit-or-a-tag-detached)) | `branch.checkout-detached`, `tag.checkout` | |
| `branch.name-format` ([card](extension/06-branches-tags-remotes.md#branchname-format--name-a-new-branch-from-a-format)) | `branch.format-name`, `branch.name-from-format` | |
| `branch.rename` ([card](extension/06-branches-tags-remotes.md#branchrename--rename-a-branch)) | `branch.rename` | |
| `branch.delete` ([card](extension/06-branches-tags-remotes.md#branchdelete--delete-a-branch)) | `branch.delete` | |
| `branch.delete-remote` ([card](extension/06-branches-tags-remotes.md#branchdelete-remote--delete-a-branch-from-its-remote)) | `remote.delete-branch` | |
| `branch.favorite` ([card](extension/06-branches-tags-remotes.md#branchfavorite--mark-a-branch-as-a-favorite)) | `refs.favorite` | |
| `branch.cleanup` ([card](extension/06-branches-tags-remotes.md#branchcleanup--clean-up-branches)) | `branch.clean-up` | |
| `branch.sort` ([card](extension/06-branches-tags-remotes.md#branchsort--list-branches-and-tags-by-name-or-by-date)) | `refs.sort` | |
| `tag.create` ([card](extension/06-branches-tags-remotes.md#tagcreate--create-a-tag)) | `tag.create` | |
| `tag.push` ([card](extension/06-branches-tags-remotes.md#tagpush--push-a-tag)) | `tag.push` | |
| `tag.delete` ([card](extension/06-branches-tags-remotes.md#tagdelete--delete-a-tag)) | `tag.delete` | |
| `remote.manage` ([card](extension/06-branches-tags-remotes.md#remotemanage--list-the-remotes-and-choose-what-to-do-with-one)) | `remote.manage` | |
| `remote.add` ([card](extension/06-branches-tags-remotes.md#remoteadd--add-a-remote)) | `remote.add` | |
| `remote.fetch` ([card](extension/06-branches-tags-remotes.md#remotefetch--fetch-one-remote)) | `remote.fetch` | |
| `remote.change-url` ([card](extension/06-branches-tags-remotes.md#remotechange-url--change-the-url-of-a-remote)) | `remote.set-url` | |
| `remote.rename` ([card](extension/06-branches-tags-remotes.md#remoterename--rename-a-remote)) | `remote.rename` | |
| `remote.remove` ([card](extension/06-branches-tags-remotes.md#remoteremove--remove-a-remote)) | `remote.remove` | |
| `sync.fetch` ([card](extension/06-branches-tags-remotes.md#syncfetch--fetch-every-remote)) | `sync.fetch` | |
| `sync.pull` ([card](extension/06-branches-tags-remotes.md#syncpull--pull-in-one-of-four-ways)) | `sync.pull`, `sync.pull-with` | |
| `sync.push` ([card](extension/06-branches-tags-remotes.md#syncpush--push-the-branch-checked-out)) | `sync.push` | |
| `sync.push-force` ([card](extension/06-branches-tags-remotes.md#syncpush-force--force-push-with-or-without-a-lease)) | `sync.push-force-with-lease`, `sync.push-force` | |
| `sync.fast-forward` ([card](extension/06-branches-tags-remotes.md#syncfast-forward--fast-forward-a-branch-that-is-not-checked-out)) | `branch.fast-forward` | |
| `merge.into-current` ([card](extension/07-merge-and-rewrite.md#mergeinto-current--merge-into-the-current-branch)) | `merge.into-current` | |
| `merge.choose-way` ([card](extension/07-merge-and-rewrite.md#mergechoose-way--choose-the-way-to-merge)) | `merge.choose-way` | |
| `merge.squash` ([card](extension/07-merge-and-rewrite.md#mergesquash--squash-merge)) | `merge.choose-way`, `merge.undo-squash` | The squash way and its Undo |
| `merge.preview-conflicts` ([card](extension/07-merge-and-rewrite.md#mergepreview-conflicts--say-which-files-will-conflict)) | `merge.read-predicted-conflicts` | |
| `rebase.onto` ([card](extension/07-merge-and-rewrite.md#rebaseonto--rebase-onto-a-commit-or-a-branch)) | `rebase.onto`, `rebase.branch-onto` | |
| `rebase.interactive` ([card](extension/07-merge-and-rewrite.md#rebaseinteractive--interactive-rebase)) | `rebase.interactive` | |
| `rebase.editor` ([card](extension/07-merge-and-rewrite.md#rebaseeditor--the-interactive-rebase-editor)) | `rebase.todo-page` | |
| `rebase.todo` ([card](extension/07-merge-and-rewrite.md#rebasetodo--the-todo-list-of-an-interactive-rebase)) | `rebase.build-todo-list` | |
| `rebase.edit-stop` ([card](extension/07-merge-and-rewrite.md#rebaseedit-stop--stop-to-edit-a-commit)) | `rebase.read-stop`, `rebase.edit-stop` | |
| `cherry-pick.one` ([card](extension/07-merge-and-rewrite.md#cherry-pickone--cherry-pick-a-commit)) | `cherry-pick.commit`, `cherry-pick.skip-empty` | |
| `cherry-pick.many` ([card](extension/07-merge-and-rewrite.md#cherry-pickmany--cherry-pick-several-commits)) | `selection.cherry-pick` | |
| `revert.commit` ([card](extension/07-merge-and-rewrite.md#revertcommit--revert-a-commit)) | `revert.commit` | |
| `reset.to` ([card](extension/07-merge-and-rewrite.md#resetto--reset-the-current-branch-to-a-commit)) | `reset.to-commit` | |
| `rewrite.reword` ([card](extension/07-merge-and-rewrite.md#rewritereword--edit-the-message-of-a-commit)) | `message.edit` | |
| `rewrite.message-box` ([card](extension/07-merge-and-rewrite.md#rewritemessage-box--the-box-that-edits-a-message)) | `message.edit` | The box it asks with |
| `rewrite.squash` ([card](extension/07-merge-and-rewrite.md#rewritesquash--squash-commits-into-one)) | `squash.commits`, `squash.into-previous` | |
| `rewrite.squash-message` ([card](extension/07-merge-and-rewrite.md#rewritesquash-message--the-message-of-squashed-commits)) | `squash.join-messages` | |
| `rewrite.split` ([card](extension/07-merge-and-rewrite.md#rewritesplit--split-a-commit)) | `split.commit` | |
| `rewrite.split-rest` ([card](extension/07-merge-and-rewrite.md#rewritesplit-rest--commit-what-is-left-of-a-split-and-continue)) | `split.continue` | |
| `rewrite.split-left` ([card](extension/07-merge-and-rewrite.md#rewritesplit-left--what-is-left-of-a-commit-being-split)) | `split.continue` | What it reads of the split |
| `rewrite.fixup` ([card](extension/07-merge-and-rewrite.md#rewritefixup--add-changes-to-an-earlier-commit-fixup)) | `fixup.commit` | |
| `rewrite.reach` ([card](extension/07-merge-and-rewrite.md#rewritereach--which-commits-a-rewrite-can-reach)) | `rebase.find-line-before-head` | |
| `rewrite.pushed-warning` ([card](extension/07-merge-and-rewrite.md#rewritepushed-warning--warn-that-a-rewritten-commit-is-pushed)) | `rebase.read-pushed` | |
| `rewrite.drag-and-drop` ([card](extension/07-merge-and-rewrite.md#rewritedrag-and-drop--drag-and-drop-onto-a-row-of-the-graph)) | `drag.drop`, `drag.list-drop-entries` | |
| `bisect.start` ([card](extension/07-merge-and-rewrite.md#bisectstart--start-a-bisect)) | `bisect.start` | |
| `bisect.mark` ([card](extension/07-merge-and-rewrite.md#bisectmark--mark-the-commit-tested)) | `bisect.mark`, `bisect.answer` | |
| `bisect.end` ([card](extension/07-merge-and-rewrite.md#bisectend--end-the-bisect)) | `bisect.end` | |
| `bisect.banner` ([card](extension/07-merge-and-rewrite.md#bisectbanner--the-bisect-banner)) | `bisect.banner` | |
| `stash.push` ([card](extension/08-stashes.md#stashpush--stash-all-changes)) | `stash.push` | |
| `stash.push-staged` ([card](extension/08-stashes.md#stashpush-staged--stash-only-the-staged-changes)) | `stash.push-staged` | |
| `stash.push-files` ([card](extension/08-stashes.md#stashpush-files--stash-some-files)) | `stash.push-file` | |
| `stash.preview` ([card](extension/08-stashes.md#stashpreview--what-applying-a-stash-would-stop-on)) | `stash.preview` | |
| `stash.apply` ([card](extension/08-stashes.md#stashapply--apply-a-stash)) | `stash.apply` | |
| `stash.pop` ([card](extension/08-stashes.md#stashpop--pop-a-stash)) | `stash.pop` | |
| `stash.drop` ([card](extension/08-stashes.md#stashdrop--drop-a-stash)) | `stash.drop`, `stash.undo-drop` | |
| `stash.branch` ([card](extension/08-stashes.md#stashbranch--create-a-branch-from-a-stash)) | `stash.branch` | |
| `stash.list` ([card](extension/08-stashes.md#stashlist--the-stashes-of-a-repository)) | `stash.read-list` | |
| `stash.contents` ([card](extension/08-stashes.md#stashcontents--what-a-stash-holds)) | `stash.read-files` | |
| `stash.view` ([card](extension/08-stashes.md#stashview--the-stashes-view)) | `stash.pane` | |
| `stash.graph` ([card](extension/08-stashes.md#stashgraph--stashes-in-the-graph-and-its-references)) | `graph.stashes`, `graph.place-stashes`, `stash.in-graph` | |
| `operation.banner` ([card](extension/09-conflicts.md#operationbanner--the-banner-of-an-operation-under-way)) | `conflicts.banner` | |
| `operation.stopped` ([card](extension/09-conflicts.md#operationstopped--what-a-stop-on-conflicts-says)) | `conflicts.stopped` | |
| `operation.continue` ([card](extension/09-conflicts.md#operationcontinue--continue-the-operation-under-way)) | `conflicts.continue-operation` | |
| `operation.skip` ([card](extension/09-conflicts.md#operationskip--skip-the-step-the-operation-stopped-on)) | `conflicts.skip-step` | |
| `operation.abort` ([card](extension/09-conflicts.md#operationabort--abort-the-operation-under-way)) | `conflicts.abort-operation` | |
| `conflict.open` ([card](extension/09-conflicts.md#conflictopen--open-a-file-in-conflict)) | `conflicts.open-file` | VS Code's merge editor in place of the Merge page |
| `conflict.open-deleted` ([card](extension/09-conflicts.md#conflictopen-deleted--show-a-file-one-side-deleted-as-a-diff-of-what-it-was)) | `conflicts.open-file` | VS Code's diff editor in place of the Diff page |
| `conflict.sides` ([card](extension/09-conflicts.md#conflictsides--name-the-two-sides-of-a-conflict)) | `conflicts.side-names` | |
| `conflict.take-side` ([card](extension/09-conflicts.md#conflicttake-side--take-a-whole-file-from-one-side)) | `conflicts.take-side` | |
| `conflict.mark-resolved` ([card](extension/09-conflicts.md#conflictmark-resolved--mark-a-file-as-resolved)) | `conflicts.mark-resolved` | |
| `conflict.files-read` ([card](extension/09-conflicts.md#conflictfiles-read--read-the-files-in-conflict)) | `conflicts.read-files` | |
| `conflict.markers` ([card](extension/09-conflicts.md#conflictmarkers--find-the-conflicts-git-marked-in-a-file)) | `conflicts.parse-markers` | |
| `conflict.rule-separate` ([card](extension/09-conflicts.md#conflictrule-separate--changes-to-lines-next-to-each-other)) | `conflicts.resolve-by-rules` | One of its rules |
| `conflict.rule-same` ([card](extension/09-conflicts.md#conflictrule-same--the-same-change-on-both-sides)) | `conflicts.resolve-by-rules` | One of its rules |
| `conflict.rule-whitespace` ([card](extension/09-conflicts.md#conflictrule-whitespace--spaces-or-line-endings-only)) | `conflicts.resolve-by-rules` | One of its rules |
| `conflict.rule-one-side` ([card](extension/09-conflicts.md#conflictrule-one-side--a-change-on-one-side-only)) | `conflicts.resolve-by-rules` | One of its rules |
| `conflict.rules-guard` ([card](extension/09-conflicts.md#conflictrules-guard--when-the-rules-leave-a-conflict-as-git-marked-it)) | `conflicts.resolve-by-rules` | What it leaves as git marked it |
| `conflict.resolve-rules` ([card](extension/09-conflicts.md#conflictresolve-rules--resolve-the-conflicts-that-need-no-choice)) | `conflicts.resolve-automatically` | |
| `conflict.review` ([card](extension/09-conflicts.md#conflictreview--the-review-of-the-resolutions)) | `review.page` | |
| `conflict.review-files` ([card](extension/09-conflicts.md#conflictreview-files--the-files-of-the-review-and-its-filters)) | `review.file-list` | |
| `conflict.review-card` ([card](extension/09-conflicts.md#conflictreview-card--a-conflict-in-the-review)) | `review.card`, `review.proposals`, `review.side-diff` | |
| `conflict.review-columns` ([card](extension/09-conflicts.md#conflictreview-columns--a-conflict-in-three-columns)) | `review.columns` | |
| `conflict.proposal-lines` ([card](extension/09-conflicts.md#conflictproposal-lines--where-the-lines-of-a-proposal-come-from)) | `review.proposal-lines` | |
| `conflict.proposal-warnings` ([card](extension/09-conflicts.md#conflictproposal-warnings--what-looks-like-a-mistake-in-a-proposal)) | `review.proposal-warnings` | |
| `conflict.choose` ([card](extension/09-conflicts.md#conflictchoose--choose-a-side-or-write-the-lines-in-the-review)) | `review.choose` | |
| `conflict.accept` ([card](extension/09-conflicts.md#conflictaccept--accept-a-proposed-resolution)) | `review.accept`, `review.accept-file`, `review.accept-all`, `review.clear` | |
| `conflict.show-result` ([card](extension/09-conflicts.md#conflictshow-result--show-the-result-of-the-accepted-resolutions)) | `review.show-result` | |
| `conflict.write` ([card](extension/09-conflicts.md#conflictwrite--write-the-accepted-resolutions)) | `review.write`, `review.report` | |
| `conflict.deleted-card` ([card](extension/09-conflicts.md#conflictdeleted-card--a-file-deleted-on-one-side-in-the-review)) | `review.deleted-card` | |
| `conflict.resolve-ai` ([card](extension/09-conflicts.md#conflictresolve-ai--ask-the-ai-how-to-resolve-conflicts)) | `ai.resolve-conflicts`, `ai.ask-conflict` | VS Code's language models in place of the AI provider |
| `conflict.choose-model` ([card](extension/09-conflicts.md#conflictchoose-model--choose-the-model-that-resolves-conflicts)) | `ai.choose-model` | VS Code's language models in place of the AI provider |
| `conflict.ai-prompt` ([card](extension/09-conflicts.md#conflictai-prompt--the-question-sent-to-the-model-about-a-conflict)) | `ai.conflict-question` | |
| `conflict.ai-answer` ([card](extension/09-conflicts.md#conflictai-answer--read-the-models-answer-as-a-proposal)) | `ai.read-answer`, `ai.trim-answer` | |
| `conflict.restore` ([card](extension/09-conflicts.md#conflictrestore--mark-the-conflicts-again)) | `conflicts.restore` | |
| `worktree.section` ([card](extension/12-worktrees.md#worktreesection--the-worktrees-section)) | `worktree.section` | |
| `worktree.folder` ([card](extension/12-worktrees.md#worktreefolder--where-a-new-worktree-goes)) | `worktree.folder-path` | |
| `worktree.branch-name` ([card](extension/12-worktrees.md#worktreebranch-name--the-name-a-format-makes)) | `branch.format-name`, `branch.name-from-format` | For a worktree |
| `worktree.create` ([card](extension/12-worktrees.md#worktreecreate--create-a-worktree)) | `worktree.create` | |
| `worktree.create-for-branch` ([card](extension/12-worktrees.md#worktreecreate-for-branch--create-a-worktree-for-a-branch)) | `worktree.create-from-branch` | |
| `worktree.create-at-commit` ([card](extension/12-worktrees.md#worktreecreate-at-commit--create-a-worktree-at-a-commit)) | `worktree.create-at-commit` | |
| `worktree.open-in-worktree` ([card](extension/12-worktrees.md#worktreeopen-in-worktree--open-a-branch-in-a-worktree-made-for-it)) | `worktree.open-in-worktree` | |
| `worktree.open` ([card](extension/12-worktrees.md#worktreeopen--open-a-worktree)) | `worktree.open`, `worktree.open-in-new-window` | The Agents window is host only |
| `worktree.open-terminal` ([card](extension/12-worktrees.md#worktreeopen-terminal--open-a-terminal-in-a-worktree)) | `worktree.open-in-terminal` | |
| `worktree.reveal` ([card](extension/12-worktrees.md#worktreereveal--show-a-worktree-in-the-file-manager)) | `worktree.reveal` | |
| `worktree.copy-path` ([card](extension/12-worktrees.md#worktreecopy-path--copy-the-path-of-a-worktree)) | `worktree.copy-path` | |
| `worktree.show-in-graph` ([card](extension/12-worktrees.md#worktreeshow-in-graph--show-what-a-worktree-has-checked-out-in-the-graph)) | `worktree.show-in-graph` | |
| `worktree.switch` ([card](extension/12-worktrees.md#worktreeswitch--switch-worktree)) | `worktree.switch` | |
| `worktree.show-here` ([card](extension/12-worktrees.md#worktreeshow-here--show-another-worktree-in-this-window)) | `worktree.show-in-gitardo` | |
| `worktree.select` ([card](extension/12-worktrees.md#worktreeselect--choose-the-worktree-shown)) | `worktree.show-in-gitardo` | Its entry from the branch in the toolbar |
| `worktree.banner` ([card](extension/12-worktrees.md#worktreebanner--the-worktree-banner)) | `worktree.banner` | |
| `worktree.rename` ([card](extension/12-worktrees.md#worktreerename--rename-a-worktree)) | `worktree.rename` | |
| `worktree.remove` ([card](extension/12-worktrees.md#worktreeremove--remove-a-worktree)) | `worktree.remove`, `worktree.recover-held-folder`, `worktree.remove-empty-folders` | |
| `worktree.prune` ([card](extension/12-worktrees.md#worktreeprune--prune-missing-worktrees)) | `worktree.prune` | |
| `worktree.branch-elsewhere` ([card](extension/12-worktrees.md#worktreebranch-elsewhere--the-branch-another-worktree-has)) | `worktree.branch-in-another-worktree` | |
| `setup.source` ([card](extension/12-worktrees.md#setupsource--which-settings-apply)) | `worktree-setup.settings-in-effect` | |
| `setup.folders-to-link` ([card](extension/12-worktrees.md#setupfolders-to-link--the-folders-a-pattern-links)) | `worktree-setup.folders-to-link` | |
| `setup.paths-to-copy` ([card](extension/12-worktrees.md#setuppaths-to-copy--the-files-a-pattern-copies)) | `worktree-setup.paths-to-copy` | |
| `setup.link-and-copy` ([card](extension/12-worktrees.md#setuplink-and-copy--link-and-copy-into-a-new-worktree)) | `worktree-setup.link-and-copy-steps` | |
| `setup.commands` ([card](extension/12-worktrees.md#setupcommands--run-commands-in-a-new-worktree)) | `worktree-setup.command-steps` | VS Code's tasks in place of the Tasks of the drawer |
| `setup.propose-commands` ([card](extension/12-worktrees.md#setuppropose-commands--the-commands-proposed)) | `worktree-setup.propose-commands` | |
| `setup.scan` ([card](extension/12-worktrees.md#setupscan--what-git-ignores-here)) | `worktree-setup.read-ignored` | |
| `setup.classify` ([card](extension/12-worktrees.md#setupclassify--what-is-proposed-for-each)) | `worktree-setup.propose` | |
| `setup.choose` ([card](extension/12-worktrees.md#setupchoose--the-patterns-a-choice-writes)) | `worktree-setup.choose` | |
| `setup.preview` ([card](extension/12-worktrees.md#setuppreview--what-the-patterns-take)) | `worktree-setup.read-what-patterns-take` | |
| `setup.page` ([card](extension/12-worktrees.md#setuppage--the-tab-of-what-new-worktrees-get)) | `worktree-setup.page` | |
| `setup.save` ([card](extension/12-worktrees.md#setupsave--save-what-new-worktrees-get)) | `worktree-setup.save`, `worktree-setup.remove-from`, `worktree-setup.edit-settings-file` | |
| `setup.offer` ([card](extension/12-worktrees.md#setupoffer--the-offer-to-choose-what-new-worktrees-get)) | `worktree.create` | The offer once a worktree is created |
| `setup.bring-ignored` ([card](extension/12-worktrees.md#setupbring-ignored--bring-ignored-files-into-worktrees-that-exist)) | `worktree-setup.bring-into-worktree`, `worktree-setup.bring-into-other-worktrees` | |
| `setup.explorer-link` ([card](extension/12-worktrees.md#setupexplorer-link--link-into-new-worktrees-from-the-explorer)) | `worktree-setup.link-into-new-worktrees` | The Explorer menu in place of the Files page |
| `setup.explorer-copy` ([card](extension/12-worktrees.md#setupexplorer-copy--copy-into-new-worktrees-from-the-explorer)) | `worktree-setup.copy-into-new-worktrees` | The Explorer menu in place of the Files page |
| `setup.explorer-stop` ([card](extension/12-worktrees.md#setupexplorer-stop--stop-linking-or-copying-into-new-worktrees)) | `worktree-setup.stop-linking`, `worktree-setup.stop-copying` | The Explorer menu in place of the Files page |
| `setup.explorer-change` ([card](extension/12-worktrees.md#setupexplorer-change--what-became-of-what-was-chosen-in-the-explorer)) | `worktree-setup.explain-change` | |
| `submodule.read` ([card](extension/13-submodules.md#submoduleread--what-is-read-of-each-submodule)) | `submodule.read-list` | |
| `submodule.status` ([card](extension/13-submodules.md#submodulestatus--where-a-submodule-stands)) | `submodule.status` | |
| `submodule.section` ([card](extension/13-submodules.md#submodulesection--the-submodules-section)) | `submodule.section` | |
| `submodule.open-window` ([card](extension/13-submodules.md#submoduleopen-window--open-a-submodule-in-a-new-window)) | `submodule.open-in-new-window` | |
| `submodule.terminal` ([card](extension/13-submodules.md#submoduleterminal--open-a-terminal-in-a-submodule)) | `submodule.open-in-terminal` | |
| `submodule.reveal` ([card](extension/13-submodules.md#submodulereveal--reveal-a-submodule-in-the-file-manager)) | `submodule.reveal` | |
| `submodule.show-in-gitardo` ([card](extension/13-submodules.md#submoduleshow-in-gitardo--show-a-submodule-in-gitardo)) | `submodule.show-in-gitardo` | |
| `submodule.copy` ([card](extension/13-submodules.md#submodulecopy--copy-the-path-or-the-url-of-a-submodule)) | `submodule.copy-path`, `submodule.copy-url` | |
| `submodule.add` ([card](extension/13-submodules.md#submoduleadd--add-a-submodule)) | `submodule.add`, `submodule.allow-file-protocol` | |
| `submodule.initialize` ([card](extension/13-submodules.md#submoduleinitialize--initialize-a-submodule)) | `submodule.initialize` | |
| `submodule.update` ([card](extension/13-submodules.md#submoduleupdate--update-a-submodule-to-the-recorded-commit)) | `submodule.update-to-recorded` | |
| `submodule.update-remote` ([card](extension/13-submodules.md#submoduleupdate-remote--update-a-submodule-to-the-latest-of-its-branch)) | `submodule.update-to-latest` | |
| `submodule.record` ([card](extension/13-submodules.md#submodulerecord--stage-the-commit-of-a-submodule-as-the-recorded-one)) | `submodule.stage-recorded` | |
| `submodule.sync` ([card](extension/13-submodules.md#submodulesync--synchronize-the-urls-of-the-submodules)) | `submodule.sync-urls` | |
| `submodule.deinit` ([card](extension/13-submodules.md#submoduledeinit--deinitialize-a-submodule)) | `submodule.deinitialize` | |
| `submodule.remove` ([card](extension/13-submodules.md#submoduleremove--remove-a-submodule)) | `submodule.remove` | |
| `submodule.update-on-checkout` ([card](extension/13-submodules.md#submoduleupdate-on-checkout--update-the-submodules-after-a-checkout)) | `submodule.update-on-checkout` | |
| `preview.ask` ([card](extension/16-undo-preview-log.md#previewask--show-the-exact-commands-before-an-operation-runs)) | `preview.confirm` | |
| `preview.fallback` ([card](extension/16-undo-preview-log.md#previewfallback--offer-the-other-way-when-git-refuses)) | `branch.delete`, `worktree.remove`, `submodule.deinitialize`, `submodule.allow-file-protocol` | The offers after a refusal, inside these cards |
| `log.write` ([card](extension/16-undo-preview-log.md#logwrite--write-every-command-and-its-output-to-the-log)) | `log.record` | |
| `log.show` ([card](extension/16-undo-preview-log.md#logshow--show-the-command-log)) | `log.show` | VS Code's output channel in place of the Command log |
| `log.report-failure` ([card](extension/16-undo-preview-log.md#logreport-failure--say-why-an-operation-failed)) | `conflicts.stopped` | |
| `undo.last` ([card](extension/16-undo-preview-log.md#undolast--undo-the-last-operation)) | `undo.last`, `undo.journal` | |
| `activity.track` ([card](extension/16-undo-preview-log.md#activitytrack--show-what-is-running-in-the-status-bar)) | `activity.track`, `activity.status-bar`, `activity.describe-task` | |
| `activity.show` ([card](extension/16-undo-preview-log.md#activityshow--list-what-is-running)) | `activity.what-is-running` | |
| `settings.read` ([card](extension/18-settings.md#settingsread--read-the-settings-when-they-are-needed)) | `settings.read` | VS Code's configuration in place of the files |

# Glossary

The words of Gitardo, as the user reads them on screen and as the code names them. One meaning per word. A word that is
not here means what a dictionary, or git's own documentation, says. The **Avoid** column lists words that are always
wrong in the documents, in UI text and in the README: `docs/catalog.py` reports them.

*Source: the ubiquitous language of Domain-Driven Design (Eric Evans, 2003); arc42 §12. Built from the README, the
titles of the commands in `package.json`, the words of the webviews, and the types of `src/shared/protocol.ts`.*

## The views

| Term | Means | Not to be confused with | Avoid | In code |
| --- | --- | --- | --- | --- |
| panel | The bottom panel of VS Code, where the **Graph** view is, next to the terminal. | side bar, bottom panel | | `gitardo` view container |
| bottom panel | In Gitardo Desktop, the island under the timeline with the terminals and the Command log. | panel (VS Code's) | | `views::panel`, `panel.bottom` |
| Tasks tab | In Gitardo Desktop, the tab of the bottom panel with the tasks of `.vscode/tasks.json` and the scripts of `package.json`, each run in a terminal beside the list (D65). A **task** is one of them, a **group** a list of them made by the user. | the tasks of VS Code, the commands of new worktrees before M19 | | `views::tasks`, `tasks.view` |
| side bar | The **Source Control** side bar of VS Code, where the **Commits**, **Gitardo Graph**, **Branches**, **Stashes** and **Gitardo Changes** views are. | panel | sidebar | `scm` view container |
| Repositories view | The **Repositories** view of VS Code's side bar, where its Git extension lists the repositories and worktrees it has open; Source Control is on one of them at a time. | repository | | `SourceControlSelection` |
| graph | The **Graph** view: the references, the commits of every branch with their lines and labels, and the details, side by side. Also the commit graph itself. | Gitardo Graph | | view kind `graph`, `App.tsx` |
| Gitardo Graph | The view of the side bar with the graph without the references, two lines per commit. | graph | | view kind `sideGraph` |
| Commits view | The view of the side bar with the commits of what is checked out, and whether each is pushed. | graph | | view kind `commits`, `CommitsApp.tsx` |
| Branches view | The view of the side bar with the references tree. | references | | view kind `branches`, `BranchesApp.tsx` |
| Stashes view | The view of the side bar with the stashes and their files. | | | view kind `stashes`, `StashesApp.tsx` |
| Changes view | **Gitardo Changes**: the view of the side bar that stages and commits, hidden until asked for. | uncommitted changes (the row of the graph), VS Code's own Changes | staging area | view kind `changes`, `ChangesApp.tsx` |
| toolbar | The row of buttons above the graph: fetch, pull, push, undo, the search, the **View** menu of the switches of the graph, the **…** menu of the actions. | | | `Toolbar.tsx` |
| references | The tree beside the graph, and in the Branches view: recent, favorites, local branches, remote branches, tags, stashes, worktrees, submodules. | label | | `RefsTree.tsx` |
| label | The name of a branch, remote branch, tag or stash on the row of its commit in the graph, outlined in the color of its line. | reference (the thing a label names) | badge, pill | `labels.ts` |
| combined label | One label for a branch and its upstream of the same name while they are at the same commit. | | | `gitardo.graph.combineUpstream` |
| details | The summary of the selected commit beside the graph (under it when the panel is narrow): message, files. | tab of a commit | detail view | `Detail.tsx`, `CommitDetail` |
| tab of a commit | An editor tab with one commit read whole: message, labels, author, parents, files and their diffs. | details | commit page | view kind `commit`, `CommitApp.tsx` |
| comparison | A tab comparing two sides, with how far apart they are, the commits only on each and the files that differ. | diff (of one file) | | view kind `compare`, `CompareApp.tsx`, `Comparison` |
| side (of a comparison) | One of the two things compared: a branch, remote branch, tag, revision, commit or the working tree. | side (of a conflict) | | `ComparedRef`, `CompareSide` |
| review | The tab that lists the conflicts with what is proposed for each, to accept, change or reject before anything is written. | merge editor, Flow Merge | | view kind `conflicts`, `ConflictsApp.tsx`, `ConflictReview` |
| what new worktrees get | The tab **Choose What New Worktrees Get…**: which ignored folders are linked, which files copied and which commands run in a new worktree. | | | view kind `worktreeSetup`, `WorktreeSetupApp.tsx` |
| Flow Diff | Gitardo's side-by-side diff in Monaco, which the diff editor switches to with a button of its title. | diff editor | | `src/flow`, `flowDiff.ts` |
| Flow Merge | Gitardo's three-column merge in Monaco, which the merge editor switches to. | merge editor, review | | `src/flow/merge.ts` |
| diff editor, merge editor | VS Code's own editors, which Gitardo opens. | Flow Diff, Flow Merge | | `vscode.diff`, `_open.mergeEditor` |
| banner | The bar above the graph that says an operation is under way, or a bisect, or a worktree, with its buttons. | | | `OperationBanner.tsx`, `BisectBanner.tsx`, `WorktreeBanner.tsx` |

## Git, as Gitardo shows it

| Term | Means | Not to be confused with | Avoid | In code |
| --- | --- | --- | --- | --- |
| repository | A git repository of the workspace; Gitardo shows one at a time, and Gitardo Desktop keeps several open, each in a tab. | worktree | | `Repo`, `RepoState` |
| working tree | The files of the repository on disk, with what is not committed. | worktree | working directory, working copy | `WorkingState` |
| index | What is staged: the next commit as it stands. | working tree | | `WorkingState.staged` |
| untracked file | A file of the working tree that git does not track and does not ignore. | ignored file | | status `?` |
| uncommitted changes | What the working tree and the index have that `HEAD` has not, shown as the first row of the graph. | Changes view | | `WorkingState`, `FileChange` |
| worktree | A further checkout of the same repository, in a folder of its own, made by `git worktree add`. | working tree, clone | | `Worktree` |
| main worktree | The folder the repository was made in, which git cannot move or remove. | worktree | | `Worktree.main` |
| missing worktree | A worktree whose folder is gone; **Prune** forgets it. | | | `Worktree.prunable` |
| junction | The link to a folder that a new worktree gets on Windows, which needs no administrator rights. | symbolic link (macOS, Linux) | | `fs.symlink(…, 'junction')` |
| Agents window | VS Code's window for agent sessions, where a worktree can open. | | | `AGENTS_WINDOW` |
| branch, local branch | A branch of the repository, `refs/heads/…`. | remote branch | | `Branch` |
| remote branch | A branch of a remote as the last fetch saw it, `refs/remotes/…`. Git calls it a remote-tracking branch, and so do a few texts of Gitardo about remotes. | upstream | | `NamedRef` (kind `remote`) |
| default remote | `origin`, or the first remote when there is none of that name: where new upstreams and tags are pushed. | | | |
| gone (upstream) | An upstream whose branch was deleted on the remote. | | | `Upstream.gone` |
| upstream | The remote branch a local branch pulls from and pushes to. | remote branch in general | | `Upstream` |
| tag | A tag, `refs/tags/…`, with or without a message. | | | `NamedRef` (kind `tag`) |
| favorite | A branch marked with **Add to favorites**, listed first, remembered per repository. | recent | | `favorites` |
| recent | The local branches checked out last, up to five, listed above the others. | favorite | | `recent` |
| stash | Changes set aside with `git stash`, `stash@{n}`. | | | `Stash` |
| submodule | A repository inside this one, at a commit this one records. | worktree | | `Submodule` |
| recorded commit | The commit of a submodule that the repository around it records. | | | |
| initialize, deinitialize (a submodule) | Clone a submodule and check out its recorded commit; empty its folder while keeping it in `.gitmodules`. | remove (a submodule) | | `git submodule init`, `deinit` |
| operation | Something Gitardo runs that changes the repository, a remote or files: one plan of the engine, shown, logged and undone as one. | operation under way | | `feature/run`, `Controller.runOnEngine` |
| operation under way | A merge, rebase, cherry-pick, revert or `git am` that git stopped, waiting for **Continue**, **Skip** or **Abort**. | operation | | `Operation`, `RepoState.operation` |
| series | Several commits made by one operation: a cherry-pick of several commits, or a patch applied as commits. **Undo** takes it back whole. | | | the engine's journal |
| todo list | The steps of an interactive rebase: pick, reword, edit, squash, fixup or drop for each commit. | | | `RebaseStep`, `todoRows` |
| current side, incoming side | The two sides of a conflict: what was checked out, and what the operation under way brings. What each is depends on the operation (for a rebase, the current side is the branch rebased onto). | side (of a comparison) | | `CONFLICT_SIDES` |
| base | What the lines of a conflict were before either side changed them. | | | index stage 1 |
| conflict | A part of a file that two sides changed differently, marked by git with `<<<<<<<`. | | | `Conflict`, `ConflictFile` |
| conflict that needs no choice | A conflict whose sides change the same text the same way, or differ only in whitespace, or change different lines that git still marked: the rules can resolve it. | | auto-resolve | `Resolution`, `src/shared/conflicts.ts` |
| proposal | What is proposed for one conflict, by the rules or by the language model, waiting in the review to be accepted. | resolution (what is written) | | `Proposal` |
| choice (in the review) | A side, both sides, or lines written by hand, that the user put in place of a proposal in the review: a proposal of its own, **Your choice**. | proposal (of the rules or the AI) | | `Choice`, `choices` |
| take a side | Resolve a whole file with the version of one side of the operation under way. | | | action `takeSide` |
| branches shown | The branches, remote branches and tags the graph is narrowed to, chosen with the button before the search. The README calls choosing them "filter by branch". | search | | `Scope`, `src/shared/scope.ts` |
| search | Finding commits in the graph by message, author, hash or change. | branches shown, branches listed, Search page | | `Filter`, `Search` |
| Search page | In Gitardo Desktop, the page that finds files, folders and lines of text in the files of the worktree shown, with the preview of a result beside them (D73). Its **scope** is where it looks: **Repository**, **Changes**, or the folder or file **Search here** named. | search (of commits), Quick search, Find in a file | | `views::file_search`, `file-search.page` |
| branches listed | The branches, remote branches and tags the references list, chosen with the funnel at their top, apart from the branches shown. | branches shown | | `RepoState.listed`, `listedRefs` in `src/shared/scope.ts` |
| fold (a merge) | Put the commits a merge brought in under one row of the graph. | | | `src/webview/folds.ts` |
| edit a message | Give a commit a new message: an amend for the last commit, an interactive rebase for an older one. | | | action `reword` |
| squash | Make one commit of several that follow one another. | squash merge | | action `squash` |
| squash merge | `git merge --squash`: the changes of a branch staged as one, nothing committed. | squash | | `MergeMode` `squash` |
| split | Take the changes of a commit out of it, to commit them again in several commits. | | | action `split` |
| fixup | Fold the staged changes into an earlier commit, with `git commit --fixup` and an autosquash rebase. | squash | | action `fixup` |
| patch | Changes as text, as `git format-patch` or `git diff` makes them. | | | `Patch` |
| bisect | Finding the commit that broke something with `git bisect`. | | | `Bisect` |
| identity | A name and email to commit with, from `gitardo.identities`. | | | `Identity` |
| pull request | A pull request of GitHub, shown on the label of its branch. | | | `PullRequest` |

## What Gitardo does around git

| Term | Means | Not to be confused with | Avoid | In code |
| --- | --- | --- | --- | --- |
| command preview | The question that shows the exact commands before they run, for every operation when `gitardo.commandPreview` is `all`, and always for those that can lose work. | | | `VscodeAsker.confirm` |
| log | The **Gitardo** output channel: every command run, what Gitardo did itself, and errors. | reflog | | `log` in `extension.ts` |
| Undo | **Undo Last Operation**: takes back the last operation from the reflog, or from what Gitardo remembered of it. | | | action `undo`, `undo.last` |
| what is running | The **Gitardo** item of the status bar while something takes more than half a second, and **Gitardo: Show What Is Running**. | log | | `Activity` |
| link, copy (into new worktrees) | What a new worktree gets of the ignored folders and files of the repository: a junction or symbolic link, or a copy. | | | `SetupChoice`, `src/shared/worktreeSetup.ts` |

## The two apps and the engine

| Term | Means | Not to be confused with | Avoid | In code |
| --- | --- | --- | --- | --- |
| Gitardo Desktop | The app with a window of its own, in `rust/`. | the engine | | crate `app` |
| the extension | The app for VS Code, in `extension/`. | | | `extension/src` |
| engine | What reads git and runs the features for both apps: the Rust crates without a window. Under the extension it runs as the program `gitardo-engine`, beside the extension host. | Gitardo Desktop | | crates `engine`, `features`, `git`, `model`; binary `gitardo-engine` |
| host | Whatever shows the engine's work and answers its questions: the window of Gitardo Desktop, or VS Code through the extension. | | | `Asker`, `HostAsker` |
| host question | A question the engine asks the host for what only the host has: the clipboard, a GitHub token, an answer of a language model, a dialog of files, a secret. | question (asked of the user) | | `host/…` requests |
| protocol | The messages between the extension and `gitardo-engine`, JSON-RPC over the standard input and output. | | | crate `protocol` |
| tab (of a repository) | In Gitardo Desktop, a repository open in the window, with its worktrees and its submodules, a row of **Repositories** in the side column; one is shown at a time, and each keeps what it showed. | page, tab of a commit | | `app::store::tabs`, `repo.tabs` |
| side column | In Gitardo Desktop, the island on the left, from under the header to the status bar, with **Repositories** above and **Branches**, the branch rail, under it. | side bar | | `repo.side-column` |
| menu of the view | The menu at the end of the header, right before **…**, with every switch of what the timeline shows and how it draws it; **…** has the actions. In the extension, **View** in the toolbar of the graph. | view (of the bar of views) | | `graph.view-menu` |
| desktop first | The order every feature is built in: the engine and the window of Gitardo Desktop, then the extension. | | | D39 |
| status (of a card) | Where a feature stands in one app: **Specified**, **Reviewed**, **Built**, **Seen** in the desktop; **Planned**, **Built**, **Seen**, **Differs**, **Host only** in the extension. | | | the columns of `docs/features/README.md` |
| fallback | What the extension ran in TypeScript when the engine's binary was missing, until stage 10 removed `src/git`; since then, without the engine, it says so with **Show the log**. Also what an operation offers when git refuses it, such as **Delete anyway**. | | | D49, `offerRun` |

# Changelog

What changed in each version of Gitardo. The versions follow [Semantic Versioning](https://semver.org).

## Unreleased

### Added

- A file stored with Git LFS opens by its content, at a commit, in the index and on either side of a diff Gitardo opens, when the store of the repository has it. One whose content is not downloaded, as in a clone made without downloading, opens as a text that says **Stored with Git LFS, not downloaded**, with its size and the first digits of its id, never the lines of its pointer, and a message offers **Download**, which downloads it with Gitardo's engine and shows its content.
- **Store with Git LFS…** in the context menu of a file of VS Code's Explorer, in a repository that stores files with Git LFS already, asks how its files are named, all the files of its extension, this file, everything in its folder or a pattern typed, and has Git LFS write the pattern in the `.gitattributes` at the top. When files the pattern takes were committed before, the message after counts them and offers **Store them with Git LFS in the next commit**.

## 1.3.0 - 2026-11-24

### Added

- Each tab of the graph in the editor shows a repository, a worktree or a submodule of its own: **Open in a new tab** in the menu of a row of the Gitardo Repositories view, or **Gitardo: Open Repository in a New Tab**, opens one, titled `Gitardo · <repository>`, and the list of the repositories in its toolbar changes that tab alone. Two tabs can show the same repository with their own filters, and it is read once for both. The panel and the side bar follow the tab of the graph last in focus, and the tabs come back after a reload on what they showed.
- Once the whole history of a repository of 10,000 commits or more is read, and git keeps no commit-graph file of it, a notification says that git can read its history about twice as fast with one, with **Write it**, which runs `git commit-graph write --reachable --changed-paths`, and **Not for this repository**, which is remembered. The menu of the row of the repository shown in the **Gitardo Repositories** view has **Speed up reading the history** while there is no such file. Git's configuration is not changed.
- **Gitardo: Check Out Pull Request…**, in the Command Palette and in the **…** menu of the graph, picks one of the open pull requests of GitHub or merge requests of GitLab, those from forks included, or takes its number typed, also on Gitea and Forgejo, and checks it out with Gitardo's engine: one from a branch of the repository as its remote branch, one from a fork into a local branch named `<owner>/<branch>`.
- **Gitardo: Sign In to GitLab** asks for a personal access token of the GitLab of the main remote, kept in the secret storage of the extension for its host name. `gitardo.hosting.selfHosted`, under **Pull and commits**, names a GitLab, Gitea or Forgejo of one's own by the host name of its remotes.
- Repositories on GitLab, Bitbucket, Azure DevOps, Gitea and Forgejo, as on GitHub: the context menu of a pushed branch has **Create merge request…** on GitLab and **Create pull request…** on the others, the first push of a branch offers it, and the Compare tab has **Open on GitLab**, **Open on Bitbucket** and so on, in **Since they diverged** only on Bitbucket, Gitea and Forgejo. Once **Load merge requests from GitLab**, in the **…** menu, has a token, the labels of the graph show `!12` for the open merge request of a branch, in the color of its pipeline, and its menu has **Open merge request !12**.
- **Check out pull request #12**, or **Check out merge request !12** on GitLab, in the context menu of a remote branch whose label shows one.
- **Lock…** in the menu of a worktree, in the Worktrees section and in the **Gitardo Repositories** view, has git keep it whatever happens to its folder, with the reason typed; **Unlock** takes the lock away. A locked worktree has a lock after its label in the Worktrees section, whose tooltip says the reason, and one whose folder is gone says `missing, locked`. **Remove…** and **Rename…** on a locked worktree say its reason first and unlock it, and **Prune missing worktrees** is not offered while the only missing worktree is locked.
- **Ignore…** in the context menu of a file of the **Gitardo Changes** view, and of a file or a folder of VS Code's Explorer, has git ignore it by a pattern you choose, this file, all the files of its extension or its folder, or one you type, written at the end of the `.gitignore` of the repository, of the folder of the file, or only on this computer, in the exclude file of the repository; on a file git tracks it offers to stop tracking it too. **Stop tracking, keep on disk…** takes a file or a folder out of the index, after a warning, while it stays on disk. **Hide its changes from git…** has git leave the changes of a tracked file alone, skipped in the working tree or assumed unchanged, and the group **Hidden from git** at the end of the Gitardo Changes view lists such files, each opening its diff with the last commit, with **Show its changes to git again** on each and **Show all to git again** in its **…**; the Explorer offers it on a file hidden from git. The context menu of a file of the Gitardo Changes view also has what the buttons of its row do. Gitardo's engine runs all of them.
- Rebasing a branch, the interactive rebase, editing the message of an older commit, squashing, splitting and the fixup move with it the other local branches on the commits they make again, a stack of branches: `git rebase --update-refs`, and one **Undo** puts them all back. A branch checked out in another worktree stays, and so do all of them with a Git older than 2.38, which the plan says. `gitardo.rebase.updateRefs` chooses **As git says**, its default, **Move them** or **Leave them**.
- What a rewrite changed, commit by commit, in a tab of its own: the commits made again the same, changed, added and dropped, and under a changed one its diff of diffs; a click on a short hash opens that commit. **See What Changed** is on the message after a rebase, an interactive rebase, a fixup or an amend, and after a fetch that found the upstream of a branch force pushed; the menu of a branch has **What its last rewrite changed**, and **Compare with `origin/feature` commit by commit** when the two have diverged, as the menu beside **Push** has; the menu of a remote branch has **What the force push changed**; and the warning of a force push has **Compare First**, which pushes nothing and opens the tab.
- The tab of a commit and the tab of a comparison show an image they changed, a PNG, a JPEG, a GIF, a WebP, a BMP or an ICO, as its two versions side by side in place of *Binary file.*, each at most 240 pixels high, with its size and weight under it and **Empty** on a side that has none; a click opens them in the diff editor. They are read once the diff is shown; a version over 10 MB says *Too large to show here.* An SVG shows its lines, with **Show the images** on its header for its two pictures. They are drawn as images of `data:` URIs, in which nothing of an SVG runs or is fetched.

### Changed

- The views cost less on a large repository. Each view is sent only what it shows of the repository, and only what changed: a file saved sends the Branches view nothing, and the graph draws a refresh in a few milliseconds where it took about 50. Views that open together read the repository once, a saved file that git ignores asks for no read, the blame of the line under the cursor is asked once for each line and version of the file, and each view loads only its own page, 282 KB of script for the Changes view where every view loaded 474.
- Which hosting service the main remote is on, and the address of the page that creates a pull request, come from Gitardo's engine, for GitHub too.
- **Ask Gitardo** in VS Code's Chat: ask `@gitardo` how to do something with Gitardo, in your own words and in any language, and it answers in a few steps with the names of the controls as they are on screen and where they are, written as the answer comes, with a button under it for each feature it names that has a command, as **Stash Changes** for `@gitardo how do I put my changes aside`. It is asked of the model chosen in the Chat, with your question, the earlier questions of the Chat and a guide to Gitardo's features, never anything of your repository. **Gitardo: Ask How To…** opens the Chat with `@gitardo ` typed, and says so when VS Code has no language model.
- The context menu of a local branch, in the graph, the references and the Branches view, has **Set upstream…** for a branch that follows nothing, or **Change upstream…** and **Stop tracking `origin/work`** for one that follows a remote branch; **Gitardo: Set Upstream of the Current Branch…** is in the Command Palette.
- The context menu of a tag has **Delete from origin** and **Delete here and from origin**, or **Delete from a remote…** and **Delete here and from a remote…** with several remotes. They warn first, and delete the tag here only once the remote took it away.
- **Gitardo: Create Branch with No History…**, in the Command Palette and in the **…** of the Branches view, starts a branch whose first commit has no parent, with no files or with the files kept and staged; Undo right after checks out again what was checked out before.
- The details beside the graph and the tab of a commit have a line **Signature** for a signed commit: `Good, Ada <ada@example.com>` in green, a signature git doubts in the color of a warning, **Bad** in red, and **Signed, not checked** or **Cannot be checked** dimmed, with the kind of signature, the key, its fingerprint and its trust in the tooltip. It is read after the details, for the commit shown alone, and a commit not signed has no line.
- **Commit options**, the button beside **Commit** in the Gitardo Changes view, has **Sign this commit**, **Add Signed-off-by**, **Co-authors…** and **Skip the hooks**, for the next commit or amend alone: once it is made, each goes back to where it starts. **Sign this commit** starts as Git's `commit.gpgSign` says, and the new setting `gitardo.commit.signOff` starts **Add Signed-off-by** ticked. **Co-authors…** picks people among the authors of the history, or typed as `Name <email>`, each named in a `Co-authored-by` line. A line under the box says what differs, such as `Not signed · With Ben, +1`.
- **Check tags on origin**, in the **…** of the Tags list of the references and of the Branches view, and **Gitardo: Check Tags on Remote** ask the remote for its tags, and mark the tags it lacks in the list and on their labels, `· 2 not on origin` after the count of the list; the marks follow what Gitardo pushes, deletes or makes until VS Code is closed. **Push tags…**, in the same **…**, and **Gitardo: Push Tags…** push the tags the remote lacks, all ticked, in one push.
- The note of a commit, the text `git notes` keeps beside it, is shown under its message in the details beside the graph and in the tab of the commit, in a block **Note** with its lines as they were written. **Add note…** in the context menu of a commit writes it; a commit with a note has **Edit note…** and **Remove note**, also on the pencil and the bin of the block, and **Undo** takes them back. The menu of the title **Remotes**, among the references and in the Branches view, has **Fetch notes from `<remote>`** and **Push notes to `<remote>`**, which `git fetch` and `git push` do not do by themselves.
- **Save as archive…** in the context menu of a commit, a branch, a remote branch and a tag saves its files as a zip or a tar.gz, every file or one folder, under a folder of its name.
- The context menu of the row of the repository shown in the **Gitardo Repositories** view has **Clean up…**, which says the room the repository takes and runs `git gc`, and **Look after it in the background…**, ticked while git's scheduled maintenance looks after the repository, which registers it with `git maintenance start` or, ticked, unregisters it.
- An image at a commit, in the index or in a conflict, opened from Gitardo, opens in VS Code's preview of images, and the diff of an image in VS Code's diff editor with its two pictures: the files of the scheme `gitardo` are given to VS Code as their bytes. Before, an image at a revision opened as its bytes in a text editor.

### Fixed

- An operation asked while another runs, such as **Unstage** in the Flow Diff clicked while the **Stage** before it still ends, is refused with "Gitardo: `<operation>` is running in `<repository>`. Try again once it ends.", as Gitardo Desktop says it. Before, it was dropped without a word, and counted as done.

## 1.2.0 - 2026-11-18

### Added

- **Gitardo Repositories**, a view of the Source Control side bar above Gitardo's other views, or **Gitardo: Show Repositories**: a row for each repository of the workspace, and under it its other worktrees, in the workspace or not, and its submodules, folded and unfolded by the chevron at its left. Each row says what is checked out there, the branch or `detached at 3f2a1c9`, `● 3` for three files changed and not committed, `↓2 ↑1` for the commits to pull and to push, read by Gitardo's engine once and then only when something changed in that folder. A click on a row shows it in Gitardo; the one shown has a bar on its left, and the repository an operation runs in a spinner. The rows take the keyboard, and what is folded is remembered for the workspace. The context menu of a row has **Open in terminal**, **Reveal in File Explorer**, **Show in Gitardo** and **Copy path**, and for a worktree and a submodule of the repository shown what their menus have in the Branches view. The view and VS Code's own Repositories view follow each other, as the repository shown already does.
- **Show all commits on one line** in the **…** menu of the Graph view, or `gitardo.graph.oneLane`, off by default: every commit of the graph on one straight line, without lanes, merges as rings, in a column one lane wide; **Fold all merges** and **Unfold all merges** leave the menu while it is on.
- The settings of Gitardo are in sections in the Settings editor, with the names and the order of the sections of Gitardo Desktop's settings page: **General**, **Timeline**, **Branches**, **Pull and commits**, **Worktrees**, **Files and blame** and **AI and conflicts**.
- `gitardo.graph.islands`, off by default: the references, the commits and the details of the Graph view are each a rounded island on a darker ground, with the toolbar above them, and the gap between two islands is the line dragged to resize them.
- `gitardo.markdown.openAsPreview`, off by default: a Markdown file opened from the Gitardo Changes view, with **Open File** or a click, opens in VS Code's preview of Markdown rather than in its editor.
- VS Code's preview of Markdown draws a block fenced as `mermaid` as its diagram, laid out and drawn by Gitardo's engine in the colors of VS Code's default theme of the kind of yours, as an image in which nothing runs or is fetched. Until it is drawn, and without the engine, the block shows its code; a diagram with an error, or of a kind not drawn, shows its code with a line saying why. Another extension that draws Mermaid in the preview, when installed, is left its blocks.

### Changed

- The toolbar of the graph has two menus where **…** had one, as Gitardo Desktop has: **View**, the button with the sliders right before **…**, has every switch of the graph, in three groups: what it shows (the local branches, the remote branches, the tags, the stashes, only the first parent of merges), how it draws it (all commits on one line, the history of the selected commit, the lines between days, weeks and months, a branch and its upstream on one label, the folds of the merges), and the author, the date and the hash of each row. **…** keeps the actions alone: **Refresh**, the comparison, **Go to commit…**, the history of the active file, the reflog, the branch cleanup, patches, remotes, submodules, the repository, the identity and the pull requests.
- The graph reads the history with one read as git prints it: the first commits are shown as soon as they are read, and the others are added under them a page at a time, without waiting for the list to be scrolled, up to 100,000 commits; past those, the next ones are read as the list nears its end. While the rest comes, the last row says **Loading more commits…**; the rows, the selection and the scroll position stay as they are. Each page is laid out alone from where the lines of the graph stopped above it, so the history of a large repository comes in at the same pace from its first page to its last.
- A read after a file changed, VS Code got the focus or an operation ended keeps the commits of the graph when what is checked out, the branches, the tags and the stashes are those of the read before: it reads no commits, and the rows stay as they are. **Refresh**, a filter, a search, the branches shown and the settings read the commits again.
- A file that git ignores, such as the output of a build, and a file of another worktree inside the repository, such as `.claude/worktrees/<name>`, no longer have the repository read again. Gitardo's engine is asked once for the files that changed in a burst, a second after the first of them; a `.gitignore` that changes still has it read.
- What is not committed in another worktree is counted again only when something changed in its own git folder, on **Refresh**, or when the repository is chosen: a read after a change to a file counts the worktree shown only, however many worktrees the repository has.
- VS Code getting the focus reads the repository only when its last read ended more than a minute before, or something changed while no view of Gitardo was open; switching to another program and back no longer reads it each time.

### Fixed

- The graph no longer stops for seconds while the commits of a long history come. The lines between days, weeks and months wrote the name of the month of every commit listed each time more commits came, two seconds for 56,000 commits: a **Refresh** of such a history took 4 to 10 seconds, in blocks, with the view not drawing in between. The name is now written once for each month, and the same **Refresh** has its last commit drawn in about a second.
- In a repository of a little more than a page of commits, 1,000 unless `gitardo.graph.pageSize` says otherwise, the graph could stop at its first page, its last row saying **Loading more commits…** for good, until the list was scrolled to its end: the read of the other commits had ended before the view was given the first, and nothing told it after. The commits read meanwhile now come with the first.
- While the commits of a long history came, each page of them was sent with the whole state of the repository to every view of Gitardo, the Changes, Branches and Stashes views too, which drew themselves again for nothing, and a search of the whole history was asked of git again at each page. The pages now go to the Graph views alone, and the search is asked again only when the commits listed are others.
- The graph stays where it is scrolled to when its commits are read again, after a commit, a fetch or **Refresh**. The read starts from its first page, and the list was cut to it: scrolled to row 28,000 of 56,000, it was thrown to row 992 and left there, and the graph got narrower and wider again. The list now keeps its length and the graph its width until the commits are back, with the rows it had before dimmed under those read so far.
- **Refresh**, a branch or a tag made, a fetch that brings nothing new and a change of the settings sent every commit of the graph to the view again, 56,000 on a large repository, as if they were others. The commits are still read again, and those that are the same stay as they are: the view is sent only what changed.

## 1.0.0 - 2026-11-17

### Added

- `gitardo.repositories.keepOpen`, off by default: each repository you leave keeps what the views showed of it, the commit selected, the scroll of the graph, the search, the branches shown, the sections of the references open and the message of the Changes view, and shows it again at once when it is chosen again. A repository left is watched, and read again only once it is shown after something changed in it.

- A filter of their own for the references beside the graph and the Branches view: the funnel at their top opens **Branches listed**, the same list as the branches shown in the graph, with **Show local branches**, **Show remote branches** and **Show tags** at its bottom, and the line above the names says what it lists, with **Show all**. It is remembered for each repository, and it and the branches shown in the graph never change each other.

### Changed

- The references beside the graph and the Branches view list every branch whatever the graph is narrowed to, the ones it leaves out dimmed with their eye. **Filter the list of branches too** and the switch above the references are gone, and `gitardo.branches.followGraphFilter` is no longer read.
- The list of the repositories puts the worktrees and the submodules of a repository under it, indented, and **Gitardo: Select Repository** lists them in the same order.
- On macOS and Linux too, **Remove…** and **Gitardo: Clean Up Branches** take out of a worktree its links to folders outside it before removing it, with an `unlink <path>` line for each: git no longer refuses a worktree whose `node_modules` link is left out by `node_modules/` in `.gitignore`.
- With `core.fsmonitor` on, on Linux and macOS, a worktree no longer has its repository read again and again: the socket the daemon listens on in its git folder (`fsmonitor--daemon.ipc`) no longer counts as a change, as the daemon's folder already did not.
- A change seen just before a read no longer has the repository read once more after it, and with `gitardo.repositories.keepOpen` on, a repository left just after such a read is no longer marked out of date.
- A read of the repository that an operation overtakes, such as the review of the conflicts writing a resolution while the versions of that conflict were read, is dropped and read again: it no longer logs an error and empties the views until the next read.
- The headers of the views in the side bar, such as the branch checked out on **Branches** and the count on **Stashes**, are filled as soon as a view opens, also when the repository was read before it: they stayed empty until the next read.
- The stashes are read without an error when the last one goes while they are read, as when it is dropped or a pull takes back the changes it set aside.

- **Create branch…** is run by Gitardo's engine when the extension has it for your system: the name is checked as you type in the engine's words, the commands are in the log as the others are, with `>` before them, and the new branch is selected in the graph once it is checked out. Without the engine, Gitardo creates the branch as before.
- **Undo** is run by Gitardo's engine when the extension has it, and takes back the newest of Gitardo's operations, whichever ran it. It now remembers up to 50 of them, for as long as nothing moves HEAD from outside Gitardo: staging a file no longer makes it forget a branch deleted or a stash dropped, and undoing again brings back the one before. Right after an operation it cannot take back, a push, a fetch, a tag, a stash or a worktree, it says so instead of taking back the one before it, and the tooltip of **Undo** names the operation it would take back.
- **Undo** now asks with a warning, says in words what it takes back, and selects the commit HEAD is at once done. A branch created without checking it out is deleted by **Undo**.
- Staging, unstaging and discarding files, **Stage all**, **Unstage all**, **Discard all changes…**, **Commit**, **Amend**, and every way to stash, apply, pop, drop a stash or create a branch from one are run by Gitardo's engine when the extension has it, with the same commands as before, and without the engine as before.
- **Unstage all changes** no longer ends a merge or a cherry-pick: while one is stopped on conflicts, or waits to be committed with none left, it unstages the staged files by name instead of running `git reset`.
- **Stage all** and **Amend** say why when there is nothing to stage or no commit to amend, instead of running git.
- Stashing a file whose rename is staged now stashes both its names instead of failing.
- **Stash** says there are no changes to stash, or that nothing is staged, before asking for a message.
- Before a stash is applied, files in conflict count as changes git refuses to touch, and no conflicts are predicted for a stash git will refuse.
- At a commit, **Create branch** with a format asks for its inputs first and then what `${base}` is.
- Gitardo comes in a package for each system, with its engine in it: Windows and Linux on x64 and Arm, Alpine Linux on x64, macOS on Intel and Apple silicon. The package with no engine in it works as before, on its TypeScript code.
- The questions and confirmations of the operations Gitardo's engine runs use the words of Gitardo's desktop: a name between backticks, as in **Create branch \`topic\`**, and a few titles worded otherwise, such as **Discard the changes of 2 files**.
- Checking out a branch, a remote branch, a commit or a tag, renaming, deleting, fast-forwarding and cleaning up branches, creating, deleting and pushing tags, deleting a branch from its remote, **Fetch**, every way to pull and push, **Manage Remotes** and **Switch Identity** are run by Gitardo's engine when the extension has it, with the same commands as before, and without the engine as before.
- New: **Sync** fetches, pulls the way **Pull** does and pushes when the branch is ahead, in one click; a branch with no upstream is published after asking. It is in the menu beside **Push** and in the Command Palette as **Gitardo: Sync**, and needs Gitardo's engine.
- When git refuses to delete a branch it finds not fully merged, Gitardo now shows git's refusal with **Delete anyway**, instead of going straight to the forced delete.
- **Undo** of a deleted branch asks as a question rather than a warning, and says the branch does not get its upstream back.
- **Delete from remote** names the branch as `refs/heads/<branch>` when a tag has its name, which git would otherwise refuse as matching more than one.
- Checking out a local branch as it is, from its remote branch, now updates the submodules to what that branch records.
- **Clean up branches** joins what it says of a branch with commas, the warning that a repository has no remote offers **Add remote…**, and a checkout whose submodules git refuses to clone from a folder on disk asks to run the rest of the checkout again from a folder on disk.
- Merging, rebasing, cherry-picking, reverting, resetting, editing a message, squashing, the fixup, splitting, the rebase editor, bisect, and copying, saving and applying patches are run by Gitardo's engine when the extension has it, with the same commands as before, and without the engine as before.
- Cherry-pick, revert and bisect now say *Finish or abort the … first* while a merge, a rebase or another operation is under way, instead of passing on git's error.
- **Undo** after a squash merge takes it back even after other operations that do not move HEAD, such as staging a file.
- Rebasing a branch that another worktree has checked out now says to rebase it there, before git runs.
- The todo list and messages of an interactive rebase, a squash, a split or a new message for an older commit are now written into the repository's git folder only once you agree, each listed in the log, and are removed once the rebase ends, also after it stopped.
- The rebase editor is now refused while a bisect is under way, as rewording, squashing and splitting already were.
- Adding changes to an earlier commit is refused during a bisect: end the bisect first.
- Once the bisect has found the first bad commit, **Good**, **Bad** and **Skip** say so instead of running git again, and **End bisect** says that no bisect is in progress when none is.
- Cherry-picking several commits asks **Cherry-pick 2 commits onto main**, with the command but not the list of the commits, and **Undo** names it so; a patch of one commit is applied **as 1 commit**.
- A merge that cannot be a fast-forward only, a patch of a merge commit and a patch that does not apply are said as warnings, with git's reasons in the message itself; what a squash merge and a split of the last commit say after has no buttons.
- **Continue**, **Skip** and **Abort** of a merge, rebase, cherry-pick, revert or patch stopped on conflicts, taking a whole file from one side, **Mark as resolved** in the Changes view and in the Flow Merge, **Resolve automatically** and the write of the resolutions accepted in the review, the rules' and the AI's alike, are run by Gitardo's engine when the extension has it, with the same commands as before, and without the engine as before. A rebase stopped to edit or split a commit still goes on as before.
- **Continue** says to resolve and stage the files first while a file is still in conflict, rather than running git and failing.
- **Skip** now asks first while files are staged, changed or in conflict, since git throws away what was resolved in the step.
- Taking a whole file from one side warns, rather than asks, when the file has lines neither side has, typed in it since git stopped: they are lost too.
- The question of **Resolve automatically** says what it writes and runs in a sentence, and it is the only question asked, also with `gitardo.commandPreview` set to `all`.
- With `gitardo.commandPreview` set to `all`, writing the resolutions accepted in the review asks first, and a file saved while the question is open is not written over.
- The log says, for each file the review writes, how many of its conflicts were resolved and by whom, and warns when the AI wrote some; the files written are named by their whole path.
- Creating, removing, renaming and pruning worktrees, **Open in worktree**, bringing what git ignores into worktrees, and adding, updating, initializing, staging, deinitializing, removing and synchronizing submodules are run by Gitardo's engine when the extension has it, with the same commands as before, and without the engine as before. What new worktrees get still comes from `gitardo.worktrees.linkFolders`, `copyFiles` and `setupCommands`, and the setup commands still run in a terminal of this window, only in a trusted workspace.
- Removing a worktree whose folder is gone now says to prune the worktrees instead.
- When git refuses to remove a worktree with changes, the message now has a **Remove anyway** button instead of asking at once.
- A folder left empty by removing a worktree now goes even when its branch is kept for not being merged, and one left empty by renaming a worktree goes even when renaming its branch fails.
- Allowing a submodule to be cloned from a folder on disk now runs again only the commands that had not run.
- Updating or staging a submodule that is already on its recorded commit now says so instead of running git; update a submodule before taking it to the latest of its branch.
- Creating a worktree with `gitardo.branches.format` set asks for the inputs of the format after what the new branch starts from, and at a commit asks what `${base}` is among the branches at it, its short hash and the branches that have it; from a detached HEAD, `${base}` is its short hash.
- Renaming the main worktree, and **Open in worktree** on the branch checked out in this window, are refused with a warning before git runs.
- Bringing ignored files into a worktree lists only what it brings, not what the worktree had already.
- Adding a submodule asks in the words of Gitardo's desktop, and says that a folder is not empty once it is entered rather than while it is typed.
- The patch copied from a comparison that starts at the working tree names its sides `a/` and `b/`, as every other patch does.
- The commits of the graph, where a commit is in it and what a search finds are read by Gitardo's engine when the extension has it, a page at a time as before, the next pages from the last commit read; a read that another takes the place of, such as a page scrolled past, is cancelled. Without the engine, Gitardo reads them as before. Two stashes made in the same second on one commit are listed the newest first.
- Drafting the commit message, asking the AI about conflicts, choosing the model that resolves them, **Sign in to GitHub**, loading the pull requests and **Open on GitHub** in the Compare tab are run by Gitardo's engine when the extension has it: the engine writes what the language model is asked, and the extension sends it to the model of VS Code, as before.
- Choosing the model that resolves conflicts offers the models as Gitardo Desktop does: **The first model of the provider**, then each model, and **As the model is set up** for how much it thinks.
- Closing GitHub's sign-in that **Sign in to GitHub** opens says nothing.
- Opening on GitHub a comparison of a commit with itself says the two sides are the same commit.
- Without Gitardo's engine for its system, Gitardo says so instead of running on its TypeScript code: every view says **Gitardo's engine is not available on this system.** with **Show the log**, and so does every command that needs the engine. Gitardo is published as one package per system, each with its engine.
- Staging or unstaging a change in the Flow Diff of a file with unsaved edits says the file changed since the diff was shown; save it first.

## 0.11.0 - 2026-11-12

### Added

- When the repository has more than one worktree, the branch checked out at the start of the toolbar of the graph is a button that lists the worktrees by their branch and shows the one picked, as **Show in Gitardo** does.
- The repository Gitardo shows follows the one clicked in the Repositories view of VS Code's Source Control side bar, worktrees included. While that view lists one repository at a time, one chosen in Gitardo is selected there too.
- In the review of the conflicts, each card says in a sentence what the proposal does with the two sides (**Keeps both sides: main first, then feature/map-view**), with a bar and counts of how many of its lines come from each side, from both and from neither, and each line proposed is marked in the color of the side it comes from.
- A proposal that keeps twice a name both sides gave a value to, such as a key of JSON or a variable, or that leaves a JSON file no longer JSON, says so on its card with its lines marked; **Accept all proposed** and **Accept the file** leave it to be accepted on its own.
- **Current only**, **Incoming only**, **Both, current first**, **Both, incoming first** and **Edit** on each card of the review: what you choose or write takes the place of what is proposed, accepted at once, and **Drop your choice** brings back the proposal.
- Line numbers beside every line of code in the review of the conflicts, in **Cards** and in **Columns**, as the file would have them with the lines shown in place of the conflict.

### Changed

- While the views show another worktree, the list of repositories keeps the repository it is of chosen, instead of listing the worktree.
- The cards of the review show what is proposed first; what each side changed is under **Compare with the sides**, shown at once when nothing is proposed. Where the proposal comes from, why, and **Accept** are together at the bottom of the card.
- In **Columns**, the lines wrap instead of scrolling sideways, each column shows the lines of the file around the conflict, and the pointer on a line lights up the lines it is joined to.

## 0.10.0 - 2026-11-04

### Added

- **Merge into `<current branch>`…** after **Merge into `<current branch>`** in the context menus and in the menu of a branch dropped on the commit checked out: the other ways to merge, always with a merge commit (`--no-ff`), as a fast-forward only (`--ff-only`), which Gitardo refuses before git runs when it cannot be one, and as a squash (`--squash`), which stages the changes and commits nothing. After a squash the commit box starts from the message Git prepares, and **Undo** takes the staged changes out again.
- The setting `gitardo.cherryPick.recordOrigin`, off by default: each commit cherry-picked, on its own, with others selected or by drag and drop, ends its message with `(cherry picked from commit …)`, as `git cherry-pick -x` writes it.
- **Get started with Gitardo**, a walkthrough in the Welcome page of VS Code: where the graph in the panel and the views in the Source Control side bar are, then a commit in an editor tab and a comparison, the blame and worktrees, each with a button to try it.
- **Gitardo: Show History of Selected Lines**, also in the context menu of the editor: the graph narrowed to the commits that changed the lines selected, or the line of the cursor, followed back from the commit checked out with `git log -L`, through moves too. Lines edited and not committed yet are found in that commit first; lines added since are said to have no history.
- **Show Folder History** in the context menu of a folder in the Explorer: the graph narrowed to the commits that changed anything in it, with the folder named in the bar above the graph. Renames of the folder are not followed.
- Commands to run in each new worktree once Gitardo has created it and made its links and copies, such as `npm ci`, in the setting `gitardo.worktrees.setupCommands` or in a section of the tab of **Gitardo: Choose What New Worktrees Get…**, which proposes them from the lock files of the repository. They run one after the other as tasks in a terminal of this window, in whatever shell VS Code runs tasks with, and one that fails stops those after it. A worktree that opens in another window or in the Agents window opens once they have succeeded. They run only in a trusted workspace.
- **Edit** in the editor of an interactive rebase: the rebase stops right after that commit is made, for you to amend it or make more commits after it, and the banner says "Stopped at … to edit it" instead of only that a rebase is in progress. **Continue** goes on whether you amended it, committed more or changed nothing, and says what to do with changes that are not committed.
- **Split this commit…** in the context menu of a commit in the graph: its changes go back into the working tree, unstaged, to commit them in as many commits as you like, with its message in the box of the Changes view to start from. The last commit is split with a reset; an older one by a rebase that stops there, with a banner that says so, and makes the commits after it again on **Continue**, which offers to commit what is left of it with the message it had. **Undo** right after takes back the whole split.
- **Copy as patch** and **Save as patch…** in the context menu of a commit and of several selected commits in the graph, and **Copy as patch** in the bar above the graph for a selection: the commits as one patch, oldest first, as `git format-patch` makes it, with their authors, dates and messages, to the clipboard or to a file you choose. Not offered on a merge commit.
- **Gitardo: Apply Patch…**, in the Command Palette and in the **…** menu of the toolbar of the graph: a patch from a file or from the clipboard, checked first with `git apply --check`, which says what would not apply before anything changes, and which files would be left in conflict. A patch made by `git format-patch` can be made into commits again with `git am --3way`, or applied to the working tree only with `git apply --3way`, as any other patch. When `git am` stops on a conflict, the banner offers **Continue**, **Skip** and **Abort**, and **Undo** takes back all the commits it made.

### Changed

- The merges folded in the graph stay folded while the view is hidden, when it is loaded again and after VS Code restarts, for each repository.
- In the cards of the review of the conflicts, each side shows what it changed of the base, as a diff: the lines taken out and the lines written, with the words that changed marked, between a few lines of the file numbered as they are in it.
- The review of the conflicts lists the files in conflict beside their conflicts, each with how many are accepted and whether some have nothing proposed: a click shows the conflicts of that file alone. Filters over the list keep to the conflicts not accepted, those with nothing proposed, or those from the AI.
- In the review of the conflicts, each side is named by its branch, or by the short hash of its commit, with the subject of that commit, rather than only as the branch checked out and the branch being merged.
- In the review of the conflicts, a conflict accepted folds to one line, and a file whose conflicts are all accepted folds to its name, so that what is left to look at stays in view. The arrow beside each unfolds it.
- The **Columns** of the review of the conflicts take the whole width of the tab, and their lines no longer wrap: a column too narrow for them scrolls sideways, with the other two. The bands between the columns can be dragged to make one wider, for every conflict at once, as in the Flow Merge.
- **Fold all merges** in the **…** menu of the graph is a switch with a check: while it is on, the merges loaded later, as the graph is scrolled or more of the history is read, are folded too, but those unfolded by hand, until **Unfold all merges**. It is kept with the folds.

### Fixed

- A file in conflict because one side deleted it and the other changed it showed nothing in the review of the conflicts, and opened from the Changes view as the plain file, with no way to see what changed. The review now gives it a card with what the side that kept it changed, as a diff, and **Keep it, with these changes** or **Delete it**; from the Changes view it opens in the diff editor, as it was beside what it is now.
- A workspace folder that holds repositories but is itself inside another one, such as a home folder kept in git, showed that other repository first, with its own branches rather than those of the repositories in the folder. The repositories in the folder now come first, and the one around it last in the list.

## 0.9.0 - 2026-10-29

### Changed

- A branch and its upstream of the same name share one label in the graph while they are at the same commit, with the icons of the branch and of the remote, and of the worktree when another worktree has the branch: `feature` and `origin/feature` are no longer two labels saying the same. Its context menu ends with what only the upstream has, such as **Delete `feature` from `origin`**. **Put a branch and its upstream on one label** in the **…** menu of the toolbar, or the setting `gitardo.graph.combineUpstream`, gives them a label each again.
- The repository is read when VS Code starts, not when the first view of Gitardo is opened: the graph, the commits and the branches are there as soon as the view is, and are read again behind them. The setting `gitardo.readOnStartup`, off, leaves it until a view opens, for a very large repository.

## 0.8.0 - 2026-10-27

### Added

- **Squash into one** in the bar above the graph and **Squash N commits into one…** in the context menu, for commits selected that follow one another before `HEAD`, and **Squash into the commit before it…** in the context menu of a commit: one commit with the changes of them all, under a subject you give and what their messages say. The commits after them are made again, and what is not committed is put back.
- **Compare with…** in the context menu of a file in the details of a commit, in the tab of a commit and in the tab of a comparison: the file as it is in that commit, as the second side of the comparison has it, or in the working tree for the changes not committed, beside the branch, tag, commit or working tree you choose, in the diff editor.
- **Fast-forward to `<upstream>`** on a branch that another worktree has checked out, while nothing is uncommitted there: the branch is pulled in that worktree as a fast-forward only, and its files move with it.

## 0.7.0 - 2026-10-22

### Added

- **Sort by name** and **Sort by date** in the **…** on the title of the Local, Remotes and Tags sections beside the graph and in the Branches view, or the setting `gitardo.branches.sortOrder`: branches, remote branches and tags by name, as before, or the one committed to last first, wherever they are listed. Not set, it is by name, or by date when you set VS Code's `git.branchSortOrder` to `committerdate`.
- **Fold the commits it merged into one row** in the context menu of a merge in the graph, on the arrow after its message and with `←` while it is selected: "3 commits of feature" in their place, which a click, the arrow or `→` opens again. **Fold all merges** and **Unfold all merges** in the **…** menu do it for every merge loaded.
- The names of the branches whose tips are scrolled out of the top of the graph, while their lines go on down it, stay at the top in the colors of their lines. A click goes back up to the tip.
- Marks beside the scroll bar of the graph for the commit checked out, the one selected and what a search found.
- A line in the graph before the first commit of yesterday, of this week, of last week and of each month before, with its name. **Separate the days, weeks and months** in the **…** menu of the toolbar, or the setting `gitardo.graph.dateSeparators`, takes them away.
- `Shift` held with the pointer on a label in the graph brings out only the commits that branch has and the one checked out has not: what merging it would bring.
- Keys to move around the graph: `Home` to the commit checked out, `Alt`+`↓` to the first parent of the commit selected and `Alt`+`↑` to the commit made on top of it, `Alt`+`Shift`+`↓` and `Alt`+`Shift`+`↑` to the next merge and the one before.
- **Show the author**, **Show the date** and **Show the hash** in the **…** menu of the toolbar of the graph, or the setting `gitardo.graph.columns`, switch each of them on and off beside the messages of the commits.
- **Show only the first parent of merges** in the **…** menu of the toolbar of the graph, or the setting `gitardo.graph.firstParent`: each branch along its own line, a merge as one commit without what it brought in.
- **Upstream** in the list of the branches shown in the graph: what is checked out and its upstream, to see what is ahead of it and what is behind. It follows you as you switch branch.
- The commit selected in the graph brings out its history, the commits it comes from and the ones made on top of it, and dims the rest, less than the pointer on a label does. **Bring out the history of the selected commit** in the **…** menu of the toolbar, or the setting `gitardo.graph.traceSelected`, switches it off.
- The label of a branch in the graph ends with the number of its open pull request on GitHub, in the color of its checks, once the pull requests are loaded. A click opens it.
- The commits of a local branch that its upstream does not have yet have a ring around their dot in the graph: what the next push of that branch sends.
- The pointer on the label of a branch, remote branch or tag in the graph, on a branch, tag, stash or worktree in the references beside it, or on a branch, tag or worktree in the Branches view of the side bar, brings out its history in the graph: the commits it reaches and their lines stay as they are, and the rest fades.
- **Show local branches**, **Show remote branches** and **Show tags** in the **…** menu of the toolbar switch each kind of reference on and off in the graph. The list of the branches shown has **Remotes** beside **Local**, for the remote branches alone. What is checked out stays in the graph either way.
- The stashes in the graph, each above the commit it was made on while that commit is shown: a square with no fill, a dashed line down to the commit, and a label with a dotted border. Their context menu applies, pops, drops them or makes a branch of them. **Show stashes** in the **…** menu of the toolbar, or the setting `gitardo.graph.showStashes`, takes them out.
- The references beside the graph and the Branches view can list every branch while the graph is narrowed, with the ones it leaves out dimmed and an eye on each that adds its history to the graph: **Filter the list of branches too** at the bottom of the list of the branches shown, the button on the line above the references, or the setting `gitardo.branches.followGraphFilter`.
- `gitardo.graph.pageSize`: how many commits the graph reads at a time, from 100 to 10,000, 1,000 by default.

### Changed

- The filter of the references beside the graph, of the Branches view and of the list of the branches shown in the graph lists the names that match best first: the name itself, then a remote branch of that name, then the names that start with it, then those with a part that does, then the rest.
- Scrolling down the graph no longer stops at the end of the commits loaded to wait for the next: they are read while there are still hundreds to scroll through. Only the commits that are new are sent to the graph, so the next ones come as quickly however far down it is.

### Fixed

- A context menu opened close to the right edge of a view, such as the one of **…**, could put the words beside an entry under it instead of on its line.

## 0.6.0 - 2026-10-22

### Added

- **Gitardo** with a spinning icon in the status bar while Gitardo reads a repository, runs an operation, compares two sides or asks GitHub for its pull requests. Its tooltip, a click on it and **Gitardo: Show What Is Running** say what each of those is, for how long it has run, what asked for a read, and how many times in a row the repository was read because something changed during each read.
- **Show in the graph** in the context menu of a branch, remote branch, tag or worktree, in the references tree and in the Branches view: it opens the graph at the commit, loading it when it is further back than the history is loaded, and showing all the branches again when the ones shown leave it out.

### Fixed

- A click on a branch or tag in the references tree of the graph did nothing when its commit was further back than the history was loaded. It now loads it and goes to it.
- In a large repository whose files keep changing, such as while it builds, the bar of the graph ran and the commits stayed dimmed for minutes after they were read. The wait is over as soon as the commits asked for are read; the reads that follow update the graph without it.
- With `core.fsmonitor` on and another worktree, Gitardo read the repository again and again: each read left a file of fsmonitor in the folder of git of that worktree, which it took for a change. Those files, and the locks git takes and lets go of, no longer ask for a read.

## 0.5.0 - 2026-10-21

### Added

- The columns of the Flow Diff and of the Flow Merge can be made wider or narrower by dragging the gutter between them, and even again by double-clicking it. The widths are kept with the page.
- The Flow Merge: a file in conflict in three columns, what the current side has, the result and what the incoming side has, with a band joining each change of a side to where it is in the result, in the color of what became of it: taken from one side, made the same way by both, or in conflict. The markers of git are marked in the result, and the parts of a conflict between them are joined to the side each comes from. The buttons on the bands take the current side, the incoming one, or both in either order, in place of a region of the result, and the result can be edited: what is chosen or typed goes into the file, unsaved until `Ctrl`+`S`. One button accepts at once the changes that need no choice, another shows the base in a fourth column, and **Mark as Resolved** saves the file and stages it, asking first when markers of git are left in it. The **Open in Flow Merge** button in the title of the merge editor opens it in its place, and its own button goes back; nothing opens in it otherwise.
- The graph shows that it is loading. While the commits of a filter or of the branches just chosen are read, a bar runs along the top of the list and the commits of before are dimmed. While the next commits are read, as the list is scrolled down, the bar runs too, and the row after the last commit says that more are loading. Switching to another repository shows the same bar while its commits are read.
- The tab of a comparison runs the same bar along its top, and dims the commits and files it showed, while the two are compared again after a side, the way to compare or the path changed. **Load more commits…** in the Commits view says that they are loading until they come.
- The file blame keeps saying whose the lines in sight are when the first line of their commit is scrolled out of sight: with `gitardo.fileBlame.groupLines` on, the first line in sight says it too.
- The code in the diffs of the tab of a commit and of a comparison is in the colors your theme gives it in the editor, or those of Dark+ and Light+ when the theme cannot be read. What colors it is loaded only once diffs are shown.
- Once the tab of a commit or of a comparison is scrolled down, a round button in its corner takes it back to the top.
- Files have the icon your file icon theme gives them in the Explorer: in the Changes view, the details of a commit, the Commits and Stashes views, the tabs of a commit and of a comparison with the headers of their diffs, the files a merge would conflict in, and the page of what new worktrees get. The directories of a tree have theirs too when the theme has icons for folders, without the arrow beside them when the theme hides it. With the theme set to **None**, files are listed as before.
- The review of the resolutions of the conflicts: a tab with every conflict beside what is proposed for it, to accept one by one, by file or all at once, to look at in the diff editor, or to resolve by hand in the Flow Merge or the merge editor. **Columns** shows each conflict in three columns, the current side, what is proposed and the incoming side, with bands joining each line proposed to the side it comes from. Nothing is written until **Write** is pressed; then a report says what was resolved and by whom, what was staged and what is left, with **Undo** and **Continue**. The buttons of the banner and of a file in the Changes view open it.
- Resolving conflicts with AI, experimental and off until `gitardo.conflicts.ai` is on: the language model of VS Code, such as GitHub Copilot, is asked about the conflicts the rules leave, and its answers come into the review as proposals, with the reason, how sure it is and the lines neither side has. The lines around the conflict that a model writes again with its answer are left out of the proposal. **Gitardo: Choose the Model That Resolves Conflicts**, also a button in the review, chooses the model among those of VS Code and how much it thinks, from low to max. Each card shows the model waiting, then its answer as it is written, and a bar counts the conflicts answered. **Resolve with AI** is in the banner, on the row of a file, in the command palette and in the message that says an operation stopped.
- The Flow Merge offers what the rules and the AI propose: a ✦ button on the band of a conflict, with the reason in its tooltip, and one over the result for all of them.
- The Changes view shows the files with edits not saved in an editor, with a dot after their names as on their tabs, and buttons to save one or all of them. The ones git sees no change in yet are in the list too, until they are saved, with a button to compare one with the file on disk.
- In the context menu of the Explorer, **Link into New Worktrees** on a folder and **Copy into New Worktrees** on a file or a folder add the pattern of its path to what new worktrees get, after asking git whether that gives it anything, and say why not when it does not: git tracks it, does not ignore it, or a pattern gives it that already. **Stop Linking into New Worktrees** and **Stop Copying into New Worktrees** take that pattern out again.

### Changed

- The tab that chooses what new worktrees get lists what git ignores in sections by kind: what Gitardo does not know first, then dependencies, settings and secrets, the editor's settings, build output, other worktrees and logs. Each section says what it is, how big and what its files get, and those where nothing is proposed or chosen start closed. What has the same name in several places, such as the `node_modules` of each package, is one line that chooses for all of them by the pattern of the name, and unfolds into each place.
- In the tab of a commit and of a comparison, a click on a file opens it in the diff editor, as it does everywhere else, rather than going to its changes in the page.
- The diffs of the files are in those tabs only once **Show the changes** is pressed, and **Hide the changes** takes them away. The eye on the row of a file shows them and goes to that file.
- Resolving the conflicts that need no choice from the command palette asks **Resolve** or **Review**, which opens the review, in place of the preview in the diff editor and the second question.
- The references beside the graph, in the panel and in an editor tab, start with their sections all closed when VS Code starts, as the Branches view does, and remember the ones opened since until VS Code is closed.

## 0.4.0 - 2026-10-19

### Changed

- The Changes view lists the files staged and not staged together, each once, with a box that stages it when ticked and unstages it when not. A file only partly staged has its box half ticked, and ticking it stages the rest; the box on the header of the list does the same for all of them. Files in conflict are still listed apart, above.
- Clicking a file in the Changes view opens the file. The new setting `gitardo.changes.openOnClick` makes it open its diff instead, as the list of changes of VS Code opens it: the index against the file while some of it is not staged, `HEAD` against the index once all of it is. The buttons **Open File** and **Open Changes** on its row open either one. Files no longer unfold into their hunks there: changes are staged one by one in the diff editor or in the Flow Diff.
- The Changes view shows its files under their directories, or one after the other, with the button on the header of the list.
- **Discard changes** on a file of the Changes view, and **Discard all changes** on its header, take back what is staged too, as in the graph, after a warning.
- On the page of what new worktrees get, **Link**, **Copy** and **Nothing** change only their own line, at once, without waiting for git to work out again what all the patterns take, which took seconds in a large checkout. Git is asked when the tab opens and with **Look again**; until then the tab says the patterns have changed.
- The sections of the Branches view are all closed when VS Code starts. The ones opened or closed since are remembered until VS Code is closed, also when the view is hidden and shown again. The references beside the graph remember theirs as before.

### Fixed

- **Stage Selected Ranges**, **Unstage Selected Ranges** and **Revert Selected Ranges** of VS Code did nothing in the diffs the Changes view opens: they only know the versions of a file that VS Code's Git extension serves. Those diffs now take them from it, when it has the repository open.
- Discarding all the changes of a file taken out of the index with `git rm --cached` and left on disk deleted it, as if it were untracked. It now goes back to the version of the last commit.

## 0.3.0 - 2026-10-16

### Added

- The worktrees Gitardo creates get what git ignores that the new settings `gitardo.worktrees.linkFolders` and `gitardo.worktrees.copyFiles` ask for, or, where they are not set, VS Code's experimental settings `git.worktreeSymlinkFolders` and `git.worktreeIncludeFiles` ask for, as the ones VS Code creates do: the folders that git ignores and that match are linked from the folder you are in, the files copied, each said in the log. On Windows, where the links are junctions and git would delete what is in them along with the worktree, Gitardo takes them out before removing a worktree.
- **Gitardo: Choose What New Worktrees Get…**, also in the new **…** of the Worktrees section: a tab that lists what git ignores, with its size, proposes what to link, what to copy and what to leave, and saves the patterns for the workspace folder or the user, where they can also be edited by hand or in settings.json. The first worktree created while nothing is set offers it once. **Bring ignored files…** gives worktrees that are there already what new ones get.
- Comparisons in a tab of their own. **Compare with…**, **Gitardo: Compare Branches, Tags or Commits…** and the compare buttons open a tab with the two sides, which a click chooses again and a button swaps; how many commits each has that the other has not and where they diverged; those commits, each side's apart, the ones whose change the other side has too, cherry-picked or rebased, marked; the files, as a tree or as a list with the lines each gained and lost; and the diff of each in the page. Two branches are compared from where they diverged, as a pull request shows them, and a button shows everything that differs instead. A side named by a branch follows it as it moves. The comparison in the details has **Open in a tab**.
- In the tab of a comparison: the working tree as a side, read again as the files change; what merging one side into the other would do, fast-forward, merge cleanly or conflict in the files named; a box that narrows it to a file or a folder; a button on each file that takes the other side's version into the working tree; **Copy as patch**; **Open on GitHub**. The comparisons opened last are offered first when one is asked for. **Compare with Branch, Tag or Commit…** in the context menu of the Explorer and of an editor tab compares a file or a folder as it is now with the branch, tag or commit chosen.
- The Flow Diff, from the Flow Diff extension: the diff of a file side by side in two editors, joined by bands that show where each change lands, with buttons to revert, stage and unstage each change, the file editable in place, and `F7` from one change to the next. The **Open in Flow Diff** button in the title of a diff editor opens the same diff in it, in place of the editor, and its own button goes back. Nothing opens in it unless asked.
- A **Submodules** section among the references, beside the graph and in the Branches view, when the repository has submodules, theirs included. An icon says where each stands against the commit recorded for it: a green check on it, a yellow arrow up or down with the number of commits it is ahead or behind, and its branch and files not committed are beside it. Its context menu opens it in a new window, a terminal or the views of this window, initializes it, updates it to the recorded commit or to the latest of its branch, stages the commit it is on as the recorded one, deinitializes and removes it, its clone in `.git/modules` included. The **+** of the section and **Gitardo: Add Submodule…** add one, and offer to allow cloning from a folder on disk when git refuses it.
- The submodules are updated to the commits recorded for them after a checkout, with `git submodule update --init --recursive`, which clones the ones the commit adds. The checkout asks first when that would leave commits in a submodule that no branch has. The setting `gitardo.submodules.updateOnCheckout` turns it off.
- The row of the uncommitted changes in the graph has a context menu: stash them, or the staged ones, stage or unstage them all, and discard them all, staged changes and untracked files included, after a warning. In the details it opens, each file can be stashed or have all its changes discarded, and files ticked are stashed or discarded together.
- **Edit message…** on a commit, and a pencil beside its message in the details: the whole message is edited in place, and saved with `Ctrl`+`Enter`. The last commit is amended without what is staged, an older one reworded by an interactive rebase that sets the changes not committed aside, after asking; a commit pushed already is said first.
- A question before applying or popping a stash, which says what it would stop on: the files that would be in conflict, worked out with `git merge-tree` without touching anything, the files not committed that make git refuse it, or an untracked file it took along that is there again.

### Changed

- The format for the names of new branches is offered by **Create branch…** too, not only by **Create Worktree**, and its settings are now `gitardo.branches.format` and `gitardo.branches.inputs`. `${base}` is the branch or the tag the branch is created at, the branch checked out from the toolbar, and at a commit the commit or a branch that has it. `gitardo.worktrees.branchFormat` and `gitardo.worktrees.inputs` are still read where the new settings are not set.
- Deleting a tag asks first, and so does deleting a branch whose commits are merged, into its upstream or into the current branch: it says that none is lost and that **Undo** brings it back. One that is not merged still warns that its commits would be lost.

### Fixed

- A commit reworded in the interactive rebase lost the body of its message: what is typed there is its new subject, and the body stays.
- Renaming or removing a worktree on Windows while VS Code's Git extension had it open failed, as the extension holds its folder. VS Code's Git now closes the worktree first, a folder held for a moment more is tried again, and when git has already let go of the worktree its folder is deleted by Gitardo.
- A cherry-pick of several commits no longer gets stuck when a conflict is resolved by keeping what the branch has, or when one of the commits is there already: git found nothing to commit and refused to continue, and **Continue** failed each time. That commit is now skipped, as a rebase does, said in the log, and the rest are picked.

## 0.2.0 - 2026-10-15

### Added

- Worktrees you name: the **+** on the Worktrees section and **Gitardo: Create Worktree…** ask for a name and a branch to start from, and make a new branch of that name in a folder of that name, slashes making folders in folders, in the folder that the new setting `gitardo.worktrees.folder` names (`${repo}.worktrees` next to the repository by default). **Create worktree…** on a branch or a commit asks for a name instead of a whole path.
- A branch that another worktree has checked out is shown as that worktree's, with its icon in the list and in the graph. A double click opens the worktree, where the new setting `gitardo.worktrees.openIn` says, and its context menu opens, renames and removes the worktree instead of offering to check out or delete the branch, which git refuses.
- **Open in worktree** on a branch: its worktree opens, made first in the folder of the worktrees and named after the branch when there is none, with nothing asked. On a remote branch, with a local branch that tracks it. The new setting `gitardo.branches.openIn`, set to `worktree`, makes it what a double click on a branch does.
- **Gitardo: Switch Worktree…** lists the worktrees and opens the one picked, or creates another.
- **Show in Gitardo** for a worktree and **Show worktree in Gitardo** for its branch: the views of this window show that worktree, its changes, commits and branches, to work in it from here. A banner says which, with the way back.
- How each worktree stands, in the Worktrees section and in **Switch Worktree…**: its branch against its upstream, the number of files not committed in it, read with `git status` in each worktree, and its pull request.
- A format for the names of the branches of new worktrees, `gitardo.worktrees.branchFormat`, such as `PRJ-${input:ticket}-${input:kind}-${base}`: the inputs that `gitardo.worktrees.inputs` defines are typed or picked from a list, `${base}` is the branch it starts from without its remote, or at a commit the commit or a branch that has it. Creating a worktree asks whether to use the format, which comes first, or a branch name.
- **Open in Agents Window** for a worktree, once it is created and in its context menu: VS Code's Agents window opens on it, so that agent sessions work in it. Where VS Code has no Agents window it is not offered, and if it fails to open the worktree opens in a new window.
- **Rename…**, **Open in terminal** and **Reveal in File Explorer** in the context menu of a worktree. Renaming moves its folder with `git worktree move` and can rename its branch too; **Remove…** can delete its branch along with it.
- **Gitardo: Clean Up Branches** lists the merged branches and the ones whose upstream is gone that another worktree has, and removes the worktree with the branch. One with files not committed is left unticked, and if ticked is removed with `--force` after a warning.
- **Fast-forward to `<upstream>`** in the context menu of a local branch that is not checked out: it fetches the upstream and moves the branch up to it, without leaving the branch you are on. Git only does it when the branch has no commits of its own.

### Changed

- Gitar is now Gitardo, with an identifier of its own: `RiccardoFilippozzi.gitardo`. On the Marketplace it is another extension, so the one named Gitar is to be uninstalled and this one installed. Its commands and settings are named after it too, `gitardo.*` where they were `gitar.*`: the settings saved under the old names are to be set again.
- A double click on the label of a branch in the graph checks the branch out, as a double click does where the branches are listed, instead of opening its commit; on a remote branch, it checks it out in a local branch. Elsewhere on the row it still opens the commit.
- Checking out a remote branch when a local branch of its name is there already checks that one out, where git refused: fast-forwarded to the remote branch when it has no commits of its own. When each has commits the other has not, you choose between rebasing the local ones onto the remote branch and leaving the branch as it is; the context menu of the remote branch also has **Check out `<branch>` and rebase it onto this** then. When another worktree has that local branch, its worktree opens instead, where it is worked on, and the context menu of the remote branch opens it.

### Fixed

- The blame of a file beside its lines with word wrap on: the first part of a long line went past the edge of the editor, which folded the line without counting what is beside it. The room for it is now kept by an inlay hint, which the editor counts. With inlay hints switched off, in a font of their own or cut shorter than the room, it is drawn before the line as before.

## 0.1.0 - 2026-10-13

The first release.

### Views

- **Graph**, in the bottom panel or in an editor tab: the commit graph, the references beside it and the details of the selected commit.
- **Commits**, in the Source Control side bar: the commits of the branch that is checked out, each with whether it is pushed and the files it changed.
- **Gitardo Graph**, **Branches** and **Stashes**, in the side bar: the graph, the references and the stashes on their own, for when the panel is closed.
- **Gitardo Changes**, hidden until asked for: staging and committing by file, by hunk or by line.
- A commit in an editor tab of its own, with its whole message and what it changed in each file.
- A button in the status bar that opens the graph, wherever the workspace has a repository.

### History

- Search in the loaded commits, or in the whole history by message, author, added or removed text and regular expression.
- The graph narrowed to the branches you choose, to the history of a file, or to one branch, tag or commit.
- Comparison of any two branches, tags or commits, with the files that differ.
- The reflog, to find a commit no branch reaches any more.
- Blame of the line under the cursor, and of a whole file beside its lines, colored by age.

### Operations

- Checkout, branches, tags, worktrees, merge, rebase, cherry-pick of one or several commits, revert and reset, from context menus or by dragging a branch.
- Interactive rebase in an editor of its own: reorder, reword, squash, fixup and drop.
- Fixup of an earlier commit with the changes that are staged.
- Bisect from the graph.
- Undo of the last commit, amend, checkout, merge, pull, rebase or reset, read from the reflog.
- Fetch, pull and push, with the ways to pull and to force a push in a menu. The Pull button rebases and sets the changes that are not committed aside, unless its settings say otherwise.
- Stashes of everything, of one file or of what is staged, also from the list of changes of VS Code.
- Cleanup of the branches that are merged, and management of the remotes.

### Safety

- Every command that Gitardo runs is logged exactly as it was run, and can be shown before it runs.
- The operations that can lose work ask first, and say what would be lost.
- Conflicts open in the merge editor, with continue, skip and abort in a banner. Resolving the ones that need no choice is offered as an experiment, off by default.

### Also

- Several repositories in a workspace, nested ones and submodules included.
- Identities to switch between, for each repository.
- Pull requests of the branches of a GitHub repository, after signing in.

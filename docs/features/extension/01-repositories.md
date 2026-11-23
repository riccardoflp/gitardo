# Repositories

Which repository the views show: the list of the repositories of the workspace, and how the one shown follows the one
selected in VS Code's Repositories view. How the repositories are found, reading them and the identities have no cards
yet ([D33](../../08-decisions.md#d33--cards-first-for-the-areas-where-a-mistake-costs)); the README describes them.

### `repo.select` — Choose the repository shown

The list of the repositories of the workspace, which points every view at the one chosen and remembers it for the
workspace.

- **Kind**: View
- **Function**: `RepoSelect` in `src/webview/RepoSelect.tsx`, which posts `selectRepo`; `Controller.selectRepo(root)`
  in `src/extension/controller.ts`.
- **Entry points**: the list at the start of the toolbar of the graph and at the top of the Changes view; **Gitardo:
  Select Repository** in the Command Palette and **Select repository…** in the **…** of the toolbar (`pickRepo`), a
  picker titled `Repository shown in Gitardo`; a click on a row of the Repositories view (`repo.tabs`), which posts
  the same `selectRepo`, for a repository, a worktree or a submodule, of the repository shown or of another.
- **Offered when**: the workspace has two repositories or more; **Gitardo: Select Repository** also while the views
  show a worktree that is not in the workspace.
- **Uses**: `worktree.show-in-gitardo`, `repo.follow-source-control`.
- **Implemented in**: `RepoSelect`, used by `Toolbar` (`src/webview/Toolbar.tsx`) and `ChangesApp`
  (`src/webview/ChangesApp.tsx`); `selectRepo`, `pickRepo` and `open` in `src/extension/controller.ts`. The choice is
  kept in the workspace state under `gitardo.repository`. The order of the list and how far each is indented are
  `groupRepositories` of `src/shared/repositoryGroups.ts`, from what `repository/discover` says of each repository
  (`RepoInfo.depth`). What a repository left keeps, with `gitardo.repositories.keepOpen` on, is `repo.keep-open`'s.
- **Tested by**: `test/e2e/suite.ts`, steps "narrows the graph to the history of a file, switching repository if
  needed", "follows the repository selected in VS Code's Repositories view, selects there the one shown, and
  shows a worktree of another", "lists the worktrees of a repository of the workspace under it in the list of the
  repositories" and "lists the repositories in the Repositories view with their worktrees and what each is on,
  counts a file changed in one not shown, and shows the row clicked"; `test/repositoryGroups.test.ts`.

Acceptance:

- [ ] The list names each repository as the workspace has it, and its tooltip is the path of what the views show.
- [ ] Choosing another repository points the graph, the Changes view, the Commits view and the Branches view at it,
  and the next time the workspace opens, it is the one shown.
- [ ] While the views show a worktree that is not in the workspace, the repository it is of stays chosen in the list;
  the worktree is chosen with the branch in the toolbar of the graph (`worktree.show-in-gitardo`).
- [ ] The list is disabled while an operation runs.
- [ ] The list puts a worktree of a repository and a submodule of it under it, indented, the worktree first, and
  **Gitardo: Select Repository** lists them in the same order.
- [ ] A click on a row of the Repositories view shows its repository, worktree or submodule in every view and marks
  the row; choosing another in the list of the toolbar marks that one.

### `repo.follow-source-control` — Follow the Repositories view of VS Code

The repository Gitardo shows follows the one Source Control is on in VS Code, the one last clicked in its Repositories
view or whose changes were last clicked; while the view lists one repository at a time, one chosen in Gitardo is
selected there too.

- **Kind**: View
- **Function**: `SourceControlSelection` in `src/extension/sourceControlSelection.ts`, with `follow` for the view to
  Gitardo and `select` for Gitardo to the view; `Controller.followSourceControl(root)` and the event
  `Controller.onDidChooseRepo` in `src/extension/controller.ts`.
- **Entry points**: a click on a repository in the Repositories view, or on the changes of one; whatever chooses the
  repository shown in Gitardo (`repo.select`, `worktree.show-in-gitardo`, `submodule.show-in-gitardo`,
  **Show File History** of a file of another repository), a click on a row of Gitardo's own Repositories view
  (`repo.tabs`) among them.
- **Offered when**: VS Code's Git extension is enabled. Gitardo to the view, also only while
  `scm.repositories.selectionMode` is `single` and the view has the repositories of Git alone.
- **Reads**: the repositories of VS Code's Git extension and which one Source Control is on (`ui.selected`), through
  its API; `scm.repositories.selectionMode`.
- **Plan**: none in git. Selecting in the view runs the view's own command for the repository,
  `workbench.scm.action.toggleRepositoryVisibility.scm<n>`: VS Code gives extensions no other way to select there.
- **Uses**: `repo.select`, `worktree.show-in-gitardo`.
- **Implemented in**: `SourceControlSelection`, made and started in `activate` (`src/extension/extension.ts`);
  `followSourceControl`, `worktreeOwner` and `onDidChooseRepo` in `src/extension/controller.ts`.
- **Tested by**: `test/e2e/suite.ts`, step "follows the repository selected in VS Code's Repositories view, selects
  there the one shown, and shows a worktree of another".

Acceptance:

- [ ] Clicking a repository in the Repositories view, or its changes, shows it in Gitardo, whether the view lists one
  repository at a time or several.
- [ ] The row of what VS Code's view chose is the one marked in Gitardo's Repositories view, and the row clicked
  there is selected in VS Code's view as any repository chosen in Gitardo is.
- [ ] A worktree the view lists, of a repository of the workspace, is shown as `worktree.show-in-gitardo` shows it; a
  repository that is neither in the workspace nor a worktree of one of its repositories is not followed.
- [ ] A repository VS Code has just opened, onto which it may move Source Control on its own, is not followed for 1.5
  seconds; nor is anything before Gitardo has looked for the repositories of the workspace, so the view makes Gitardo
  read nothing at start.
- [ ] With `scm.repositories.selectionMode` set to `single`, choosing a repository or a worktree in Gitardo selects it
  in the view, the only one listed; nothing is done when it is selected already, or when VS Code does not have it open.
- [ ] With `scm.repositories.selectionMode` set to `multiple`, the default, choosing one in Gitardo leaves the view as
  it is: the only command would take repositories out of the view without knowing which it lists.
- [ ] The commands of the view are matched to the repositories of Git by their numbers, in the order the Git extension
  lists them, leaving out the worktrees VS Code keeps out of the view (folders named `copilot-…` or `agents-…`). When
  the view has more or fewer repositories than that, Gitardo selects nothing there and says why in the log at the debug
  level; when a command does not select the repository asked for, Gitardo says so in the log and selects nothing there
  again until the window is reloaded.
- [ ] What Gitardo selects in the view is not followed back.

### `repo.tab-repository` — Each tab of the graph shows a repository of its own

A tab of the graph in the editor shows the repository, the worktree or the submodule it was opened on, and keeps it,
whatever the other views show
([D89](../../08-decisions.md#d89--in-the-extension-each-tab-of-the-graph-shows-a-repository-of-its-own-proposed)).

- **Kind**: View
- **Function**: `Controller.attach(webview, 'graph', repository)`, which ties the view to the session of its
  repository, and `RepositorySession` in `src/extension/repositorySession.ts`: the state, the reads, the watchers and
  what each view was sent, for one repository; the list of the toolbar posts `selectRepo`, which moves its own view.
- **Entry points**: **Open in a new tab** in the context menu of a row of the **Gitardo Repositories** view (`repo.tabs`),
  on a repository, a worktree or a submodule; **Gitardo: Open Repository in a New Tab** in the Command Palette, a picker
  of the repositories, worktrees and submodules as **Gitardo: Select Repository** lists them; **Gitardo** in the status
  bar and **Gitardo: Open Graph in Editor**, on the repository the panel shows.
- **Offered when**: the workspace has a repository.
- **Uses**: `repo.select`, `worktree.show-in-gitardo`, `repo.keep-open`, `repo.follow-tab`.
- **Implemented in**: not yet (M45).
- **Tested by**: not yet (M45).

The title of the tab is `Gitardo · <repository>`, with `› <worktree>` or `› <submodule>` after it, as the rows of the
Repositories view name them (Q1520). Its toolbar, its menus, its keys and the banners in it act on its repository: an
operation runs there, a commit opened from it opens in that repository, **Show in graph** from another view does not
move it. The list of the repositories and the menu of the worktrees in its toolbar move this tab alone. Two tabs on the
same repository each keep their selection, their filter, the branches shown, their scroll and their folds; the
repository is read once for both.

Acceptance:

- [ ] With `api` and `web` in the workspace, **Open in a new tab** on `web` while the panel shows `api` opens a tab
  titled `Gitardo · web` showing the history of `web`; the panel still shows `api`.
- [ ] Choosing `docs` in the list of that tab's toolbar shows `docs` in that tab, and no other view changes.
- [ ] A worktree `feature/login` of `api` opened in a new tab is titled `Gitardo · api › feature/login`, and a commit
  made from its uncommitted row is made in that worktree.
- [ ] Two tabs on `api` with different filters keep them; a file saved in `api` asks one read, and both tabs show it.
- [ ] Closing the last view of a repository stops watching it, unless `gitardo.repositories.keepOpen` is on.
- [ ] An operation asked in one tab while another runs in a second is refused with *… is running in …*.

### `repo.follow-tab` — The panel and the side bar follow the tab in focus

The **Graph** view of the panel and the views of the side bar show the repository of the tab of the graph last in
focus, as they show the repository chosen.

- **Kind**: View
- **Function**: `Controller.followTab(panel)`, on `onDidChangeViewState` of a tab of the graph when it becomes
  active; the session the panel and the side bar are on.
- **Entry points**: a click in a tab of the graph, or any way to bring one into focus.
- **Offered when**: a tab of the graph is open.
- **Uses**: `repo.tab-repository`, `repo.select`, `repo.follow-source-control`.
- **Implemented in**: not yet (M45).
- **Tested by**: not yet (M45).

The panel and the views **Gitardo Changes**, **Commits**, **Gitardo Graph**, **Branches** and **Stashes** follow the
tab of the graph that last came into focus; until one has, they show the repository chosen as before (`repo.select`),
and the workspace state keeps it under `gitardo.repository`. Choosing another in the panel or in the side bar, or
following VS Code's Repositories view (`repo.follow-source-control`), moves them and no tab. A tab of a commit, of a
comparison or of conflicts does not move them (Q1521). The commands of the Command Palette that act on the repository
shown act on theirs (Q1522).

Acceptance:

- [ ] With a tab on `api` and one on `web`, a click in the tab of `web` shows `web` in the panel and in **Gitardo
  Changes**; a click in the tab of `api` shows `api` there.
- [ ] Choosing `docs` in the list of the panel's toolbar shows `docs` in the panel and the side bar, and neither tab
  changes.
- [ ] With `scm.repositories.selectionMode` set to `single`, bringing the tab of `web` into focus selects `web` in VS
  Code's Repositories view, as `repo.follow-source-control` selects the repository chosen.

### `repo.keep-tabs` — The tabs of the graph come back after a reload

A tab of the graph comes back when VS Code opens the workspace again, on what it showed.

- **Kind**: View
- **Function**: a `WebviewPanelSerializer` of `gitardo.graphEditor`, registered in `activate`
  (`src/extension/extension.ts`), which reattaches the page with the repository kept in its state.
- **Entry points**: opening the workspace, or reloading the window, with tabs of the graph open.
- **Offered when**: a tab of the graph was open when the window closed.
- **Uses**: `repo.tab-repository`.
- **Implemented in**: not yet (M45).
- **Tested by**: not yet (M45).

The page keeps, in its own state (`setState`), the root of its repository, its worktree or submodule, its filter and
the branches shown. A tab whose folder is no longer there says **`<folder>` is no longer there.** with **Close** (Q1523).
Nothing is added to the workspace state.

Acceptance:

- [ ] A tab on `web` with a filter by author, after **Developer: Reload Window**, shows `web` with the same filter, and
  the panel shows what it showed.
- [ ] A tab on a worktree removed while VS Code was closed says the folder is no longer there, and **Close** closes it.

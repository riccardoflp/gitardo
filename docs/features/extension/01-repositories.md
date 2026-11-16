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
  picker titled `Repository shown in Gitardo`.
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
  shows a worktree of another" and "lists the worktrees of a repository of the workspace under it in the list of the
  repositories"; `test/repositoryGroups.test.ts`.

Acceptance:

- [ ] The list names each repository as the workspace has it, and its tooltip is the path of what the views show.
- [ ] Choosing another repository points the graph, the Changes view, the Commits view and the Branches view at it,
  and the next time the workspace opens, it is the one shown.
- [ ] While the views show a worktree that is not in the workspace, the repository it is of stays chosen in the list;
  the worktree is chosen with the branch in the toolbar of the graph (`worktree.show-in-gitardo`).
- [ ] The list is disabled while an operation runs.
- [ ] The list puts a worktree of a repository and a submodule of it under it, indented, the worktree first, and
  **Gitardo: Select Repository** lists them in the same order.

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
  **Show File History** of a file of another repository).
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

# Testing

How a change is checked, from the cheapest check to the most real, where each kind of test lives, and what no test
reaches. Pick the check that can see what changed.

## The commands

```bash
npm install
npm run build        # dist/: the extension, the webviews, the Flow Diff, the preview server, the e2e suite
npm run typecheck    # tsc, over src and test
npm test             # Vitest: unit tests and integration tests against throwaway repositories
npm run test:e2e     # builds, then runs test/e2e/suite.ts inside a real VS Code
npm run test:vsix    # packages, then runs the same suite on what the .vsix contains
npm run test:ui      # builds, then drives a real VS Code window with clicks and keys; screenshots in dist/ui
npm run check:docs   # checks the catalog of features: cards, identifiers, glossary words
npm run preview -- <folder>...   # the webviews in a browser, against the repositories in the folders
npm run package      # a minified build packed into gitardo-<version>.vsix
```

| Script | Purpose |
| --- | --- |
| `npm run build` | Bundles the extension, the webview and the preview server into `dist/`. |
| `npm run watch` | Rebuilds on change. |
| `npm run typecheck` | Type-checks with `tsc`. |
| `npm test` | Runs the tests with Vitest. The integration tests run the real commands against throwaway repositories, and resolve conflicts marked by git itself, hundreds of them made at random. |
| `npm run test:e2e` | Builds, then runs the end-to-end tests inside a real VS Code: see below. |
| `npm run test:ui` | Builds, then uses the extension in a real VS Code window, with real clicks and keys, saving screenshots: see below. |
| `npm run package` | Makes a minified build and packs it into `gitardo-<version>.vsix`. It leaves the minified build in `dist/`; `npm run build` brings the development one back. |
| `npm run test:vsix` | Packages, then runs the end-to-end tests on what the `.vsix` contains instead of on this folder. |
| `npm run check:docs` | Runs `docs/catalog.py` on the extension's cards (`docs/features/extension`) and documents: every card has the lines its kind needs, every identifier written in them names a card of either catalog, and none of them uses a word the glossary says to avoid. Needs Python 3. |
| `npm run preview -- <folder>...` | Serves the webview at `http://localhost:5174` against the repositories found in the folders, read-only, to work on the UI without an Extension Host. Add `?view=changes`, `commits`, `branches` or `stashes` for a view of the side bar, `?view=worktreeSetup` for the tab that chooses what new worktrees get, `?view=commit&commit=<revision>` for the tab of a commit, `?view=compare&from=<revision>&to=<revision>` for the tab of a comparison, and `?theme=light` for the light theme, and `--port=<number>` before the folders to serve a second one beside it. |

`F5` in VS Code launches an Extension Development Host with Gitardo loaded (`.vscode/launch.json`).

The commands run in `extension/`. `npm test` there collects only the extension's tests, so the worktrees under
`.claude/worktrees` at the root of the repository no longer add theirs.

## The kinds, from cheapest to most real

| Kind | Checks | Where | How it runs |
| --- | --- | --- | --- |
| Types | That the code holds together | `src`, `test` | `npm run typecheck` |
| Unit | Parsers on recorded git output, the lanes of the graph, the rules of conflicts, the blame's format, messages, menus, folds, the tree of files, the Flow Diff's diffs | `test/*.test.ts`, `test/flow/*.test.ts` | `npm test` |
| Integration | Operations: their commands run on throwaway repositories, and what the repository is after | `test/integration.test.ts`, `test/submodules.test.ts`, `test/awkward.test.ts`, `test/conflicts.test.ts`, `test/resolve.test.ts` | `npm test` |
| Preview | What a webview shows, against a real repository, in a browser | `src/dev/server.ts` | `node dist/dev-server.js --port=<n> <folder>` |
| End to end | Commands, the controller and what VS Code is asked, inside a real VS Code with stubbed dialogs | `test/e2e/suite.ts` | `npm run test:e2e`, `npm run test:vsix` |
| Window | What only a real window shows: context menus, real dialogs, clicks at coordinates, themes, the Content Security Policy | `test/e2e/ui.mjs` | `npm run test:ui` |
| Performance | Times on a large repository; numbers to read, not a test | `test/perf` | by hand, below |

1. `npm run typecheck` and `npm test` for anything in `src/git` or `src/shared`. Integration tests make real
   repositories in a temporary folder: add one when an operation changes.
2. The preview, for anything a webview shows: `node dist/dev-server.js --port=<n> <folder>` and open
   `http://localhost:<n>/?view=<kind>`. Views: `graph`, `changes`, `commits`, `sideGraph`, `branches`, `stashes`,
   `worktreeSetup`, `commit&commit=<revision>`, `compare&from=<revision>&to=<revision>`. Add `&theme=light` for the
   light theme. The server serves the build it started with: rebuild and restart it after a change.
3. `npm run test:e2e`, for commands, the controller and what VS Code is asked. VS Code's dialogs are stubbed there.
4. `npm run test:ui`, for what only a real window shows. Look at the screenshots in `dist/ui`, not only at the count.

## Throwaway repositories

The integration tests make each repository with `git init --initial-branch=main` in a temporary folder, with the
identity `Gitardo Test <test@gitardo.invalid>` and signing off (`git config commit.gpgsign false`), run the commands of the operation exactly as
`Controller.run` would (the last one with the operation's standard input), and check the repository afterwards.

`test/awkward.test.ts` is the other half: file names with spaces, accents, brackets and leading dashes, a repository
with no commits, a detached `HEAD`, a shallow clone, several root commits, a merge of three parents, a branch and a tag
of the same name, a submodule.

## End-to-end tests

`npm run test:e2e` starts the VS Code installed on the machine as a separate instance, with a throwaway profile and a
throwaway workspace holding two small repositories and a bare remote, and runs `test/e2e/suite.ts` inside its Extension
Host. Your own VS Code window, settings and extensions are not touched; a second window opens for about half a minute
and closes by itself. Pass the path of the executable as an argument, or set `VSCODE_PATH`, if it is not in a standard
place.

The suite drives the extension through its commands and its controller, with VS Code's dialogs, input boxes and pickers
answered by stubs that record what they were asked. Among other things it checks that the webviews of all five views
load, that only the views that list commits are sent them, that the commands shown in a confirmation are the ones that
run and the ones logged, that diffs and the merge editor open with the right content, that nothing about resolving
conflicts automatically is offered or run until its setting is on, and that with a language model standing in for a
real one its answers show as they come and are written only when accepted.

`npm run test:vsix` runs the same suite on the packaged extension: the `.vsix` is unpacked into the throwaway folder and
loaded from there, which catches a file left out of the package or broken by minification.

What that suite cannot see is what only eyes and hands can: how the views look, the blame annotations, drag and drop,
the context menus of the webviews, and the real dialogs, which VS Code refuses to show while it runs tests.

## Using it like a person

`npm run test:ui` starts the same throwaway VS Code, this time not in test mode, and drives its window through the
DevTools protocol (`test/e2e/ui.mjs`): real clicks and right-clicks at the coordinates of what is on screen, real
keystrokes, the real command palette, input boxes, pickers and confirmation dialogs. It goes through the graph in five
themes (the two dark and the two light ones VS Code comes with, and the high contrast one), the toolbar at 1050 and 800
pixels, the side bar views, staging and committing, the context menus, the command preview's dialog and its **Cancel**,
the blame of the line and of the file, the search, the branches shown, drag and drop, the sashes, the tab of a commit
and of a comparison, the Stashes view, the automatic resolution of conflicts and its review, the Flow Diff and the Flow
Merge. Its steps are the list in the file itself; each has a screenshot.

A screenshot of each step is saved to `dist/ui`, or to the folder given with `--out=<folder>`: looking at them is half
the point. Errors thrown or logged inside the webviews are printed at the end. The window is sized through the protocol,
so the screenshots do not depend on the screen.

## Large repositories

Three scripts in `test/perf` show how Gitardo behaves when there is a lot of everything. None of them is a test: they
print numbers to read. The numbers that matter are the scenarios of [12-quality.md](12-quality.md).

```bash
node test/perf/big-repo.mjs ../big 50000 2000
git -C ../big pack-refs --all
git -C ../big commit-graph write --reachable
node dist/perf.js ../big
node test/perf/vscode.mjs ../big
```

- `big-repo.mjs <folder> [commits on main] [changed files]` makes a repository with `git fast-import`: a long main line
  with side branches merged back into it, 250 branches never merged, 2,000 tags, remote branches, and a working tree with
  that many changed and untracked files. The two git commands after it leave the references and the history the way
  `git gc` or `git maintenance` leave them in a repository that is looked after; without them git itself takes about
  three times as long to list the commits.
- `dist/perf.js <repository>` times what Gitardo reads: the state at 1,000, 5,000 and 20,000 commits, the details of a
  commit, the working tree, each kind of search, the history of a file, blame.
- `vscode.mjs <repository> [--out=<folder>]` opens the repository in the throwaway VS Code of `npm run test:ui` and
  times what a person waits for: opening the graph, a refresh, a new file showing up, the next pages of commits,
  selecting a commit, typing a commit message. It saves screenshots to `dist/perf`, and adds and removes one file in the
  repository.

Measure rendering only on the minified build (`node esbuild.mjs --production`): React's development build is several
times slower, in proportion to the rows. That build, and `npm run package`, leave `dist/` minified until the next
`npm run build`.

## From a card to a test

Each acceptance line of a card is checked by one of the kinds above, and the card's **Tested by** line names the test
by its file and its title. A line with no test says so in **Tested by**. When a change adds a test for a line, it names
it on the card in the same commit. The names of the tests are sentences from the user's side; they do not carry the
identifier of the card (see [D35](../08-decisions.md#d35--tests-are-named-by-what-the-user-sees)).

## With the engine

From stage 1 of [the transition](../transition/06-roadmap.md) the extension starts `gitardo-engine`. The tests find it
as the extension does: `gitardo.engine.path` when set, which the test scripts set to `../rust/target/debug/
gitardo-engine`, built first with `cargo build -p server` and `CARGO_PROFILE_DEV_DEBUG=0`.

- `test/engine.test.ts` (Vitest) starts the binary and checks the protocol: `initialize`, the state of the throwaway
  repositories against `Repo.state`, cancelling, a restart after a crash.
- The integration tests of a moved area run each operation through `feature/run`, with a fake host that answers the
  questions from the test; they keep the same names, so that a card's **Tested by** line still finds them.
- The scenarios shared with the desktop ([desktop/06-testing.md](../desktop/06-testing.md#scenarios-run-by-both-apps))
  compare the engine's plan and the final state with the TypeScript `GitOp`'s, operation by operation, until `src/git`
  is removed.
- The e2e and UI suites run once with the engine and once with `gitardo.engine.path` naming a missing file, which
  checks the fallback ([D49](../08-decisions.md#d49--when-the-engines-binary-is-missing-proposed)).

## What a change runs before it is committed

- Always: `npm run typecheck` and `npm test`.
- A change in `../rust/` that the extension uses: `cargo xtask check` there, then `npm run test:e2e` here.
- A change to the documents: `npm run check:docs`.
- A change to a webview: the preview, in a dark and a light theme.
- A change to commands, the controller or what VS Code is asked: `npm run test:e2e`.
- A change to menus, dialogs, the look of a view, or the package: `npm run test:ui`, and `npm run test:vsix` before a
  release.

## Things learned the slow way

- When the browser pane of a preview is hidden, `requestAnimationFrame` never fires and timers run once a second.
- `0 passed, 0 failed (VS Code exited with code 1)` from the e2e or UI run is usually a VS Code update waiting for a
  restart, not the code: VS Code then refuses new instances until it is restarted.
- On Windows, worktrees removed during the tests can be held by VS Code's own Git extension: see `closeInVSCodeGit` and
  `recoverHeldWorktree` in `src/extension/actions.ts`. Also on Windows, `git worktree remove` deletes what is behind a
  junction; Gitardo unlinks such links first (`unlinkFolderLinks`).

## What no test reaches

Signing in to GitHub, the Agents window of VS Code, drafting a commit message and asking a real model how to resolve a
conflict need a signed-in account and a language model: no test reaches them, and no one signs in on the maintainer's
behalf to try. Before a release they are tried by hand:

- [ ] **Gitardo: Sign In to GitHub**, then the pull requests on the labels of their branches, and opening one.
- [ ] Drafting a commit message in the Changes view.
- [ ] **Resolve Conflicts with AI** on a real conflict, and its answers in the review and in the Flow Merge.
- [ ] Opening a worktree in the Agents window.

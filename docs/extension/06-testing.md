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
npm run package      # gitardo-<target>-<version>.vsix for each engine built
```

| Script | Purpose |
| --- | --- |
| `npm run build` | Bundles the extension, the webview and the preview server into `dist/`. |
| `npm run watch` | Rebuilds on change. |
| `npm run typecheck` | Type-checks with `tsc`. |
| `npm test` | Runs the tests with Vitest. The scenarios run the engine's features against throwaway repositories (skipped, and said, when the engine is not built), and the conflicts are resolved as marked by git itself, hundreds of them made at random. It also runs `npm run check:docs` when Python 3 is on the `PATH`, and says it skipped it when not (`test/docs.test.ts`, Q378), checks the imports between the folders of `src` (`test/imports.test.ts`), and that no file of TypeScript of `src` is over 1,000 lines (`test/fileLength.test.ts`, R5.2). |
| `npm run test:e2e` | Builds, then runs the end-to-end tests inside a real VS Code: see below. |
| `npm run test:ui` | Builds, then uses the extension in a real VS Code window, with real clicks and keys, saving screenshots: see below. |
| `npm run package` | Makes a minified build and packs it (`scripts/package.mjs`) into `gitardo-<target>-<version>.vsix` for each of the seven targets whose engine it finds, holding `bin/<target>/gitardo-engine` only; there is no universal vsix, since without its engine Gitardo only says so (D49), and a release waits for the vsix of every target. The engines are looked for in the folder `--binaries` names, laid out by target as the pipeline does, then in `../rust/target/<triple>/release/`, and for this computer in `../rust/target/release/`. `--target <target>` makes that one only, `--every-target` fails when an engine is missing. It prints the size of each, for the release notes. It leaves the minified build in `dist/`; `npm run build` brings the development one back. |
| `npm run test:vsix` | Packages, then runs the end-to-end tests on what the `.vsix` of this computer's target contains, with the engine packed in it and no `gitardo.engine.path`. |
| `npm run check:docs` | Runs `docs/catalog.py` on the extension's cards (`docs/features/extension`) and documents: every card has the lines its kind needs, every identifier written in them names a card of either catalog, none of them uses a word the glossary says to avoid, and every test a card of either catalog names after **Tested by** is there. Needs Python 3; `npm test` runs it too. |
| `npm run preview -- <folder>...` | Serves the webview at `http://localhost:5174` against the repositories found in the folders, read-only, to work on the UI without an Extension Host. Add `?view=changes`, `commits`, `branches` or `stashes` for a view of the side bar, `?view=worktreeSetup` for the tab that chooses what new worktrees get, `?view=commit&commit=<revision>` for the tab of a commit, `?view=compare&from=<revision>&to=<revision>` for the tab of a comparison, and `?theme=light` for the light theme, `--islands` before the folders for the graph as `gitardo.graph.islands` draws it, and `--port=<number>` before the folders to serve a second one beside it. |

`F5` in VS Code launches an Extension Development Host with Gitardo loaded (`.vscode/launch.json`).

The commands run in `extension/`. `npm test` there collects only the extension's tests, so the worktrees under
`.claude/worktrees` at the root of the repository no longer add theirs.

## The kinds, from cheapest to most real

| Kind | Checks | Where | How it runs |
| --- | --- | --- | --- |
| Types | That the code holds together | `src`, `test` | `npm run typecheck` |
| Unit | Parsers on recorded git output, the lanes of the graph, the rules of conflicts, the blame's format, messages, menus, folds, the tree of files, the Flow Diff's diffs | `test/*.test.ts`, `test/flow/*.test.ts` | `npm test` |
| Checks | The imports between the folders of `src`, as [02-architecture.md](02-architecture.md#who-may-use-whom) draws them; the length of its files of TypeScript, 1,000 lines at most (R5.2); the documents, as `npm run check:docs` checks them | `test/imports.test.ts`, `test/fileLength.test.ts`, `test/docs.test.ts` | `npm test` |
| Integration | Operations and reads through the engine, on throwaway repositories, and what the repository is after | `test/engine*.test.ts`, `test/reads.test.ts`, `test/review.test.ts`, `test/conflicts.test.ts` | `npm test`, with the engine built |
| Preview | What a webview shows, against a real repository, in a browser | `src/dev/server.ts` | `node dist/dev-server.js --port=<n> <folder>` |
| End to end | Commands, the controller and what VS Code is asked, inside a real VS Code with stubbed dialogs | `test/e2e/suite.ts` | `npm run test:e2e`, `npm run test:vsix` |
| Window | What only a real window shows: context menus, real dialogs, clicks at coordinates, themes, the Content Security Policy | `test/e2e/ui.mjs` | `npm run test:ui` |
| Performance | Times on a large repository; QA1 to QA3 of [12-quality.md](12-quality.md) fail above their time times one and a half | `test/perf` | by hand, below; `vscode.mjs --limits` by the pipeline on Linux |

1. `npm run typecheck` and `npm test` for anything in `src/engine` or `src/shared`. The scenarios of the engine make
   real repositories in a temporary folder: add one when what the extension sends the engine changes; an operation
   itself is tested in the engine, in `rust/`.
2. The preview, for anything a webview shows: `node dist/dev-server.js --port=<n> <folder>` and open
   `http://localhost:<n>/?view=<kind>`. Views: `graph`, `changes`, `commits`, `sideGraph`, `branches`, `stashes`,
   `worktreeSetup`, `commit&commit=<revision>`, `compare&from=<revision>&to=<revision>`. Add `&theme=light` for the
   light theme. The server serves the build it started with: rebuild and restart it after a change.
3. `npm run test:e2e`, for commands, the controller and what VS Code is asked. VS Code's dialogs are stubbed there.
4. `npm run test:ui`, for what only a real window shows. Look at the screenshots in `dist/ui`, not only at the count.

## Throwaway repositories

The scenarios make each repository with `git init --initial-branch=main` in a temporary folder, with the identity
`Gitardo Test <test@gitardo.invalid>` and signing off (`git config commit.gpgsign false`), run the engine's feature with
the target and answers `engineRuns` gives it, as `Controller.runOnEngine` would, and check the repository afterwards
with git's own output (`test/git.ts`, git for the tests, which the extension itself never runs). The awkward
repositories (names with spaces, accents, brackets and leading dashes, no commits, a detached `HEAD`, a shallow clone,
several roots, a branch and a tag of the same name, a submodule) are the engine's tests, in `rust/`.

## End-to-end tests

`npm run test:e2e` starts the VS Code installed on the machine as a separate instance, with a throwaway profile and a
throwaway workspace holding two small repositories and a bare remote, and runs `test/e2e/suite.ts` inside its Extension
Host. Your own VS Code window, settings and extensions are not touched; a second window opens for about half a minute
and closes by itself. Pass the path of the executable as an argument, or set `VSCODE_PATH`, if it is not in a standard
place.

The suite drives the extension through its commands and its controller, with VS Code's dialogs, input boxes and pickers
answered by stubs that record what they were asked. Among other things it checks that the webviews of all the views
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

What the mouse does over a webview goes to the page inside it, which is another process. A screenshot taken while
that page draws can leave Chromium not knowing where the page is, until the next frame is drawn, which in a window
that nothing moves in may be seconds away: a move or a press is then given to the frame of the webview in the
workbench, and the page gets nothing, on a machine that is busy more often than on one that is not. `moveTo` of
`test/e2e/window.mjs` counts the moves the workbench gets on such a frame, and makes again the move before a press
when it was lost, once a frame captured has put things right, or, when that is not enough, once the frames of the
webviews were said not to take the mouse and then to take it again; a move lost six times fails the step by its
name, and the suite says at its end how many were made again. It is the move that is made again and not the press,
which the workbench answers by giving the frame the focus, at which a menu open in the page closes. One step that loses a click fails those after it that count on what it did, such as
the commit of the step that stages a file.

## Large repositories

The scripts in `test/perf` show how Gitardo behaves when there is a lot of everything. They print numbers to read; the
numbers that matter are the scenarios of [12-quality.md](12-quality.md), and `vscode.mjs --limits` fails when one of
QA1 to QA3 takes longer than the time that page gives it, times one and a half
([D91](../08-decisions.md#d91--the-checks-pass-on-every-system-and-what-was-found-weak-is-checked-again-proposed),
Q1519).

```bash
node test/perf/big-repo.mjs ../big 50000 2000
git -C ../big pack-refs --all
git -C ../big commit-graph write --reachable
node dist/perf.js ../big
node test/perf/vscode.mjs ../big
node test/perf/graph-load.mjs ../big --commit
```

- `big-repo.mjs <folder> [commits on main] [changed files]` makes a repository with `git fast-import`: a long main line
  with side branches merged back into it, 250 branches never merged, 2,000 tags, remote branches, and a working tree with
  that many changed and untracked files. The two git commands after it leave the references and the history the way
  `git gc` or `git maintenance` leave them in a repository that is looked after; without them git itself takes about
  three times as long to list the commits.
- `dist/perf.js <repository>` times what Gitardo reads: the whole history as the controller reads it (the state, then
  one read whose pages are laid out as they come, D54), the state at 1,000, 5,000 and 20,000 commits, the details of a
  commit, the working tree, each kind of search, the history of a file, blame. `--engine=<binary>` names the engine,
  the release build for the numbers of M15.
- `vscode.mjs <repository> [--out=<folder>] [--limits]` opens the repository in the throwaway VS Code of
  `npm run test:ui` and times what a person waits for: opening the graph, a refresh, a new file showing up, the next
  pages of commits, selecting a commit, typing a commit message. It saves screenshots to `dist/perf`, and adds and
  removes one file in the repository. It says each time against its limit, read from the table of
  [12-quality.md](12-quality.md) and multiplied by 1.5: QA1, the graph opened once its command is chosen in the
  Command Palette, its name typed there being the script's time; QA2, a new file shown, less the second the working
  tree's changes wait (`WORKTREE_REFRESH_DELAY`); QA3, each next page of commits, received and drawn, while the last
  row says more are loading, which on the large repository is seldom, since the pages come without being asked for
  (D54). QA4, "no key waits", gives no time and has no limit. With `--limits` a time over its limit fails
  the run. The pipeline runs it so on Linux alone, on a repository made by `big-repo.mjs <folder> 50000 2000`, with the
  minified build (`node esbuild.mjs --production`) and the engine in release, `GITARDO_ENGINE` naming it.
- `switch-fixture.mjs <folder> [commits on main] [changed files]` makes what choosing a repository is timed on, in a
  folder away from any repository such as `~/.cache/gitardo-perf/m50`: `workspace/large`, the repository of
  `big-repo.mjs` with the two git commands after it, `workspace/beside`, a repository of 40 commits that is the one
  shown when the window opens, and `worktrees/worktree-1` to `worktree-3`, three worktrees of the large one, each on
  one of its branches.
- `vscode.mjs <folder> --switch` (`switch.mjs`) opens `workspace` of that folder in three windows, one after the
  other, and in each chooses, in the menu of the repositories and in that of the worktrees of the graph: the large
  repository, never shown in the window; one of its worktrees, never shown; the large repository again after the
  small one, with `gitardo.repositories.keepOpen` as it is by default, and then with it on. Each choice is made once
  every commit of the history has come and two seconds have passed, and is timed until the view has the first state
  of the folder chosen that leaves it with commits of it, the first page of a history or, for a worktree whose rows
  the view has, a state with no commit, and from there until it has drawn it; for the worktree it times the state
  after that one too, its first step, with what it has checked out. It prints the median of the three windows, how
  many commits that first state carried, and the git processes the engine started between the choice and the rows drawn,
  by command, counted from the events git writes when `GIT_TRACE2_EVENT` names a file, which a shell script that
  starts the engine sets; on Windows they are not counted. With `--limits` it fails when the median
  of a choice is over the time of its scenario of [12-quality.md](12-quality.md) times one and a half: QA15 a
  repository chosen again, QA16 a worktree chosen for the first time, QA17 a repository never shown; `CHOICES` of
  `switch.mjs` names the scenario of each. The pipeline does not run it (Q1745).
- `changes-fixture.mjs <folder> [repositories] [folders of a build in each] [files in each folder]` makes what changes
  made by git outside are timed on, in a folder away from any repository such as `~/.cache/gitardo-perf/m52`:
  `workspace/repository-01` to `repository-30`, thirty repositories of five commits, each with a folder `build` it
  ignores, of 7,000 folders with a file each, and `workspace/written`, a folder that is no repository. The builds are
  what the watcher VS Code starts over the workspace has to go through before it says anything, and it is the folders
  that count: with 210,000 of them, about 800 MB made in some seconds, a window on a fast disk on Linux hears nothing
  until fourteen to eighteen seconds after it started, some seven after the graph has its first rows; with 60,000 it
  hears of everything from the first rows. On Linux each folder takes one of the watches of `fs.inotify.max_user_watches`.
  The first repository is the one shown when the window opens and the one the measure changes; its tag `fixture` is
  where each measure starts from, and `worktrees`, beside `workspace`, is where the measure adds its worktrees.
- `vscode.mjs <folder> --changes` (`changes.mjs`) opens `workspace` of that folder in one window, with the graph of the
  panel on the first repository and the section **Local** of its references open, and makes there, with git from
  outside, a commit, a branch and a worktree in turn, each timed from when git has made it until the graph shows its
  name: the subject of the commit in a row, the branch, or the branch of the worktree, in the list. It does so in
  three moments: from the first rows of the graph after the window starts, a change every second for 25 seconds, each
  given up after 5; while a process of its own writes files without end in `workspace/written`, ten in one of 400
  folders at each turn and the ten of 400 turns before removed, 5,000 a second if the machine lets it, a change every
  4 seconds from the eighth, nine of them, each given up after 6; and in the 15 seconds after that process is stopped,
  a change every 2.5 seconds, each given up after 6. A change is made at its time whether those before were shown or
  not, so one not shown yet may be shown by the read the next asks for, as it would be for a person. It prints each
  time, or `not seen`, with how long after the window started the change was made; for each moment the median of
  those seen, the worst, how many were not seen, and the median of each kind; how many files were written a second;
  and how many times `fileWatcher.log` of the window says *started ignoring events due to too many file change events
  at once*, which is VS Code dropping what it is told of. It leaves the repository as the fixture made it. `MOMENTS`
  of `changes.mjs` names the scenario of [12-quality.md](12-quality.md) of each moment, whose time the worst may not
  pass with `--limits`: none yet, QA18 and QA19 coming with the last phase of milestone M52. The pipeline does not
  run it.
- `graph-load.mjs <repository> [--out=<folder>] [--commit] [--profile]` opens the repository in the same VS Code and
  records how its graph loads, as the eye sees it: every 50 ms, how many commits the list has, where it is scrolled
  to, how many rows are drawn, whether the last row says **Loading more commits…**, how wide the graph is; each message
  with commits the view gets, and each frame that took more than 50 ms. It does so while the graph opens, after
  **Refresh** at the top of the list, while the list is scrolled to its end, after **Refresh** from the middle of the
  list and, with `--commit`, after an empty commit it makes on what is checked out, the only thing it changes. It
  prints each change of what the list shows with its time, and saves screenshots to `dist/graph-load`. With
  `--profile` it also says in which functions the view spent the first **Refresh**, by name on the development build.
  Run it on the large repository and on one of between one and two thousand commits, where a read ends right after
  its first page. `GITARDO_ENGINE` names the engine, the release build for numbers to compare.

What `graph-load.mjs` reported on 2026-11-18 on the large repository (56,123 commits), with the minified build and the
release build of the engine, before milestone M25 and after its first five phases, one run of each build right after
the other:

| | Before M25 | After phases 1 to 5 |
| --- | --- | --- |
| **Refresh** at the top: the last row drawn | 4.6 s | 0.9 s |
| the longest frame of the view meanwhile | 1.9 s | 71 ms |
| **Refresh** from row 28,064: the last row drawn | 4.0 s | 1.0 s |
| the longest frame of the view meanwhile | 1.9 s | 82 ms |
| where the list is after it | row 992 | row 28,064 |
| A commit made outside, at row 28,064: the last row drawn | 4.6 s | 1.7 s |
| the longest frame of the view meanwhile | 1.8 s | 82 ms |
| where the list is after it | row 992 | row 28,064 |
| The width of the graph during a **Refresh** | 62, 48, 62 px | 62 px |
| On 1,250 commits, after **Refresh** | 1,000 rows and **Loading more commits…** until scrolled to the end | every row, and no such row |

The times are of one run each and move with what else the machine does: while other builds ran, the build of before
took 3.9 s, 9.3 s and 10.3 s for the same three reads, with frames of up to 4.1 s, and the build of after 1.4 to 3.8 s,
with no frame over 140 ms. Of the 1.7 s after a commit made outside, 0.9 s pass before the first page: the wait for
the files of `.git` to stop changing. The row the list is at, the width of the graph and the rows of the small
repository do not move from run to run.

Measure rendering only on the minified build (`node esbuild.mjs --production`): React's development build is several
times slower, in proportion to the rows. That build, and `npm run package`, leave `dist/` minified until the next
`npm run build`.

## From a card to a test

Each acceptance line of a card is checked by one of the kinds above, and the card's **Tested by** line names the test
by its file and its title. A line with no test says so in **Tested by**. The file, in backticks, and each title after
it, in double quotes or in italics, are checked to be there by `npm run check:docs` and by `cargo xtask check`
(rule R2.6): a title is found as the start of a string of the file, so "the steps that start …" names them all. When a change adds a test for a line, it names
it on the card in the same commit. The names of the tests are sentences from the user's side; they do not carry the
identifier of the card (see [D35](../08-decisions.md#d35--tests-are-named-by-what-the-user-sees)).

## With the engine

From stage 1 of [the transition](../transition/06-roadmap.md) the extension starts `gitardo-engine`. The tests find it
at `../rust/target/debug/gitardo-engine`, built first with `cargo build -p server` and `CARGO_PROFILE_DEV_DEBUG=0`, or
where the environment variable `GITARDO_ENGINE` says; without it they are skipped, and the run says why. Inside VS Code
the extension finds it from `gitardo.engine.path`, which the e2e and UI scripts set to the same binary
(`engineSetting` in `test/e2e/fixture.mjs`); `npm run test:e2e -- --no-engine` sets it to a file that is not there.

- `test/engine.test.ts` (Vitest) starts the binary and checks the protocol: `initialize`, the state of the throwaway
  repositories against what git itself lists (`for-each-ref`, `worktree list`, `status`), a burst of changes,
  cancelling, the restarts after the engine is killed, and the settings sent. It runs `branch.create` through
  `feature/run` with an asker that answers from the test, checks the name with `question/validate`, and runs the same
  answers through `gitardo-cli`, comparing the plans and the repositories after. It takes operations back with
  `feature/run` of `undo.last`, newest first, across a restart of the engine too, and after a commit made outside
  Gitardo by the reflog.
- The scenarios run each operation through `feature/run`, with a fake host that answers the questions from the test;
  they keep the names they had, so that a card's **Tested by** line still finds them. What they share is
  `test/engine/harness.ts`: throwaway repositories, an asker that answers by key, `runFeature`, `engineState`, the
  state as the views read it, and `snapshot`, what a repository is compared by.
- Each area has its scenario test, `test/engine<Area>.test.ts` (`engineChanges.test.ts` for the changes, the commit
  and the stashes; `engineHistory.test.ts` for merge, rebase, cherry-pick, revert, reset, the rewriting, bisect and the
  patches; `engineConflicts.test.ts` for the conflicts, whose review's write takes its target from the review as
  `src/engine/resolutions.ts` builds it; `engineWorktrees.test.ts` for worktrees, what new worktrees get and
  submodules): each operation runs, with the target and answers `engineRuns` (`src/engine/actions.ts`) gives it, as
  the engine's feature, and the commands it ran and the repository after are checked line by line. Until stage 10 they
  were compared with the TypeScript `GitOp` in a second copy; the lines they now expect are the ones both gave, but
  where [04-differences.md](../transition/04-differences.md) says why not. `engineBranches.test.ts`, for the branches,
  tags, remotes, identity and sync, gives the repository a bare one beside it as `origin`, cloned by a colleague who
  pushes, so that fetches and pushes reach only folders the test made.
- `test/engineReads.test.ts` asks each read of the protocol through the typed methods of `EngineClient`, and checks the
  answer against what git itself prints of the same throwaway repositories (`git show`, `git diff`, `git blame
  --line-porcelain`, …). `test/reads.test.ts` asks the reads as the views ask them, through `src/engine/reads.ts`,
  against git's own output, and, with the binary missing, checks that each read throws `EngineMissing`.
- `test/engineSummaries.test.ts` follows repositories and their worktrees through the engine (1.10, milestone M22), and
  checks what each is said to be on, that a folder said changed is said again, that a worktree added is followed once
  its repository lists it, and that the folders are followed again after the engine was ended and started again.
- `test/engineHelp.test.ts` asks Ask Gitardo through the engine (1.11, milestone M31) with a fake model of the Chat,
  and checks that the model chosen there is asked both requests, that the text of the second is written as it comes
  with each feature as the bold title of its card, and that `stash.push` has the button **Stash Changes**;
  `test/help.test.ts` checks the table of the features and their commands against `package.json` and the guide of the
  extension, what `@gitardo` writes and **Gitardo: Ask How To…**, without the engine.
- `test/engineGraph.test.ts` reads the graph through the engine (`src/engine/graph.ts`), in every way the graph can be
  shown, as one read whose pages come as git prints them (1.8, D54), each page laid out under the rows before it as all
  the rows at once, with a stash of a later page placed where all the commits read at once put it, the commits kept
  while the history is the same and read again when it changed, and checks the rows, the positions and the matches
  against `git log` with the same arguments; R8
  of [04-differences.md](../transition/04-differences.md#reads-of-the-graph-stage-7) is its own test.
- `test/engineAi.test.ts`, for the AI and GitHub, answers the engine's host questions with a fake language model,
  which keeps what it was asked: what the draft of a commit message and the question about a conflict send the model,
  what the same answer gives, that cancelling a read cancels the question to the model, and that choosing the model
  leaves the two settings as `writeSetting`.
- `test/engineWorktreeSettings.test.ts` runs `worktree-setup.save`, `worktree-setup.remove-from` and
  `worktree-setup.copy-into-new-worktrees` on the engine as the extension sends them, with `setup`, and checks that the
  plan has no step and leaves `writeSetting` with its `target`, and the line the log writes of each.
- `test/engineWithoutGit.test.ts` checks what version 1.7 of the protocol added for the extension to run no git: the
  repositories of the workspace's folders (`repository/discover`), a file of a comparison taken from its second side,
  and a change of the Flow Diff staged and unstaged with the `diffChange` target, or refused when the file no longer
  has it.
- The e2e and UI suites run once with the engine and once with `gitardo.engine.path` naming a missing file
  ([D49](../08-decisions.md#d49--when-the-engines-binary-is-missing-proposed)): without the engine they check only that
  the views say **Gitardo's engine is not available on this system.** with **Show the log**, and that a few commands
  (**Fetch**, **Undo**, **Create Branch…**) say it too, and stop there.

## The engine's shapes

From stage 6 the state of a repository and the details of a commit are the engine's types, written by hand in
`src/shared/state.ts` ([02-architecture.md](02-architecture.md#the-views-shapes)). They are checked against the JSON the
engine writes, not against a schema, with no dependency added:

- `rust/crates/protocol/tests/shapes.rs` writes a state and the details of a commit as golden files,
  `tests/golden/state-filled.json` and `tests/golden/commit-detail-filled.json`, with every optional part filled and
  every list holding one entry, so that every field has a value. It is a golden test of the engine: when the JSON of
  `protocol` changes, `cargo xtask check` fails until the files are written again with `GITARDO_GOLDEN_RECORD=1`.
- `test/engineShapes.ts`, which `npm run typecheck` checks and nothing runs, reads the two files as types
  (`resolveJsonModule`) and makes each side assignable to the other: `RepositoryState` and `CommitDetail`, with every
  field filled and every string a string, to what the files hold, and back. A field added, removed or renamed on either
  side, or a value of another kind (a number for a string, a list for an object), is a type error there.

What it cannot see: the strings an enumeration has, since TypeScript reads every string of a JSON file as `string`, and
which fields may be null, since none is null in the files. The values are checked by the tests that read the same
repositories with git: `test/engine.test.ts` checks the engine's state against what git lists, and
`test/engineReads.test.ts` the details of a commit, of a stash and of the uncommitted changes against `git show` and
`git diff`.

## What a change runs before it is committed

- Always: `npm run typecheck` and `npm test`.
- A change in `../rust/` that the extension uses: `cargo xtask check` there, then `npm run test:e2e` here.
- A change to the documents: `npm run check:docs`, which `npm test` runs too when Python 3 is there.
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

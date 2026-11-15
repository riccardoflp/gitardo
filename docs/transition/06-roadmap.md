# The roadmap of the transition

From two apps with two engines to one engine under both, in stages. Each stage ends when its acceptance lines hold and
`cargo xtask check`, `npm run typecheck`, `npm test`, `npm run test:e2e` and `npm run check:docs` pass; it is one
commit or a few, never mixed with the next stage. Every stage that touches a feature follows D39: **desktop first**,
that is the engine and the window of Gitardo Desktop with the card, the tests and `rust/CHANGELOG.md`, **then the
extension**, with its interface, its tests and `extension/CHANGELOG.md`.

The milestones of each app before this work are in [desktop/07-roadmap.md](../desktop/07-roadmap.md) (M0 to M11; M11
waits for macOS, stage 9 here) and [extension/07-roadmap.md](../extension/07-roadmap.md).

Until stage 1 is done, `main-desktop` may still get commits; they come into `main-merge` with a plain
`git merge main-desktop`, which follows the move into `rust/` and of the documents into `docs/` by itself: a probe
merge of a commit that changes `crates/engine/src/undo.rs` and `docs/06-testing.md` on top of `main-desktop` landed in
`rust/crates/engine/src/undo.rs` and `docs/desktop/06-testing.md` with no conflict. `-Xsubtree=rust` must **not** be
used: with it git lost the move of the documents and reported a modify/delete conflict on `rust/docs/06-testing.md`.
After stage 1 the work is done only in `main-merge` (D38).

## Stage 1 — The server, with the smallest protocol

**Desktop first.** Nothing changes in the window; the engine is made usable without it.

- Separate what `engine` uses of `platform` (`link_folder`, the three functions of the credential store) from the
  window's libraries, as Q393 decides; the picture of the crates in
  [desktop/02-architecture.md](../desktop/02-architecture.md) is changed first, in its own commit, since `cargo xtask
  check` checks the edges against it (R1.3).
- `rust/crates/protocol`: the types of [02-protocol.md](02-protocol.md) for the messages of this stage, with `serde`
  and `serde_json` (D42), and their conversions from `model`.
- `rust/crates/server`, the binary `gitardo-engine`: `--stdio`, the framing, `initialize`, `shutdown`, `exit`,
  `$/cancel`, `repository/open`, `repository/close`, `repository/changed`, `repository/state`, `read/repo.read-state`,
  `log/line`, `activity/changed`. One thread reads standard input, one writes standard output, each request runs on a
  thread of its own.
- `history_of`, `commits_request` and the reading of the settings move out of `crates/app` to where the server can use
  them (`engine` or a new crate, decided in the stage), and the window keeps using them from there.

**Then the extension.**

- `extension/src/engine/`: the client. It starts the binary from `gitardo.engine.path` or `bin/<platform>/`, sends
  `initialize` with the settings, restarts it as [01-engine.md](01-engine.md#when-something-goes-wrong) says, and copies
  its standard error and `log/line` into the **Gitardo** output channel.
- Nothing the user sees changes: the engine is started, its state is compared with `Repo.state` in the tests, and no
  view uses it yet.

Acceptance:

- `gitardo-engine --stdio` answers `initialize` with protocol `1.0`, and an `initialize` with protocol `2.0` with the
  error `-32602` and its own version in `data`.
- `repository/open` of a throwaway repository answers the same branches, tags, stashes, worktrees, head and working
  tree as `Repo.state` reads there, for each repository of the extension's integration tests (`test/`).
- `repository/changed` after a file is written sends one `repository/state` for a burst of ten changes.
- A request with an unknown method answers `-32601`; the engine keeps running.
- `$/cancel` of a read that runs answers `cancelled`, and no `git` process of it is left.
- The engine killed from outside is started again once, the repositories opened again; a third exit within five
  minutes is said in the output channel and not restarted.
- `cargo tree -p server` lists none of winit, softbuffer, arboard, rfd, accesskit.
- The binary of a release build for `x86_64-pc-windows-msvc` is under 15 MB.
- The window of Gitardo Desktop passes its application tests and snapshots unchanged.

## Stage 2 — The proof: `branch.create`

The first feature run by the engine for the extension, end to end, with its questions in VS Code.

**Desktop first**: `branch.create` is Built in the desktop. The differences of [04-differences.md](04-differences.md)
for it are fixed in the engine first: S2 (with B1), the submodules not updated after a new branch is checked out.

**Then the extension**: the entries **Create branch…** (the toolbar, the menus of a commit, of a branch, of a tag)
send `feature/run` of `branch.create`; `feature/ask` is answered with `showInputBox` and `showQuickPick`;
`question/validate` checks the name as it is typed; `feature/confirm` shows the plan; the follow-ups select the new
branch. The `GitOp` of `createBranch` stays in `src/git/ops.ts` for the fallback when the binary is missing (D49).

Acceptance:

- From the graph, **Create branch…** on a commit asks the name, refuses `feature x` with the engine's sentence while it
  is typed, refuses a name taken, creates the branch with the commands the engine logs, and the graph shows it.
- With `gitardo.branches.format` set, the name is proposed from the format, and the inputs given are remembered in
  `worktreeInputs`.
- With check out ticked in a repository with submodules and `gitardo.submodules.updateOnCheckout` on, the submodules
  are updated, as in the extension today.
- **Undo** right after takes the branch back (the reflog, still in TypeScript at this stage).
- With `gitardo.engine.path` naming a missing file, **Create branch…** runs the TypeScript operation and the output
  channel says the engine is missing.
- The extension's card of `branch.create` and the desktop's one are merged: one card, Desktop Built, Extension Built.
- A scenario test runs `branch.create` with the same answers through `gitardo-cli` and through the extension, and
  compares the plans and the final states of the repository
  ([07-rules.md](07-rules.md#how-the-extension-is-tested-with-the-engine-in-short)).

## Stage 3 — Undo in the engine

Done early, because from stage 4 on some operations run in the engine and some in TypeScript, and **Undo** must take
back the newest of them, whichever ran it.

**Desktop first**: D7 and D8 of [04-differences.md](04-differences.md) are decided and fixed in the engine: the journal
records that the last plan cannot be undone, and `undo.last` refuses while an operation is stopped. The journal keeps
its 50 entries and holds while `HEAD` stays.

**Then the extension**: `journal/record` after every `GitOp` that still runs in TypeScript, with what it remembered
converted to an `UndoRecord`; **Undo** is `feature/run` of `undo.last` always; `journal/last` gives its label. The
extension's own memory of the last operation (`Controller.undoable`, `controller.ts:256`) is removed.

Acceptance:

- A branch deleted in TypeScript, then a branch created by the engine: **Undo** removes the new branch; **Undo** again
  restores the deleted one, from the journal.
- A stash dropped in TypeScript, then a commit made in TypeScript: **Undo** takes back the commit, then the drop.
- **Undo** right after a push says it cannot take back a push, and takes back nothing.
- **Undo** during a merge stopped on conflicts refuses with the engine's sentence.
- A squash merge, a resolution of conflicts and a split are each taken back whole.
- The restart of the engine empties the journal, and **Undo** then goes by the reflog, as the extension did after a
  reload.

## Stage 4 — The operations, area by area

One area at a time, in this order, each a stage of its own with its own commits:

1. Changes and commit, stashes.
2. Branches, tags, remotes, identity; **Sync** given to the extension (B8).
3. Merge, rebase, cherry-pick, revert, reset; rewriting (reword, squash, fixup, split, the rebase editor); bisect;
   patches.
4. Conflicts: the rules, the review, the proposals, Continue, Skip, Abort.
5. Worktrees and what new worktrees get; submodules.
6. AI and GitHub, through the host questions (`host/languageModel`, `host/githubToken`).

For each area:

- **Desktop first**: the engine bugs and the choices of [04-differences.md](04-differences.md) for the area are fixed
  or decided, with tests; a read that only `crates/app` does today becomes a card of the engine (Q398); the cards of
  the area in [features/extension/](../features/extension/README.md) are merged into the desktop's cards
  ([07-rules.md](07-rules.md#waiting-for-a-stage)).
- **Then the extension**: each entry of the area sends `feature/run`; its `resolveAction` case and its `GitOp` are
  kept only as the fallback of D49; the CHANGELOG of the extension gets one line for each difference its users will see.

Acceptance, for every area:

- Every operation of the area that the extension has runs in the engine, and the e2e and UI tests of the area pass
  with the engine.
- Every difference of the area in [04-differences.md](04-differences.md) is fixed, decided or in the CHANGELOG.
- The scenario tests of the area compare the plans and the final states of the two paths, for each operation.
- The area's cards are one set, each with its status for the two apps.

## Stage 5 — The reads but the graph

The state, the details of a commit, the comparison, the blame, the history of a file, the conflicts' reads, the
worktree setup's reads, the pull requests: every `read/…` of [02-protocol.md](02-protocol.md#reads) but
`graph.read-commits`, `graph.read-position` and `search.read-matches`. The Commits view gets its read in the engine
first (Q397).

Acceptance:

- The views show the same thing with the engine's reads, checked by the e2e suite and by comparing the old and the new
  read on the repositories of the integration tests.
- The state read once per burst of changes, not once per view: the e2e test **sends the commits to the graph once**
  still passes.
- The reads are timed in the log (`LogKind::Read`) and none of the e2e suite takes more than twice what the
  TypeScript read took, compared once on the large repository.

## Stage 6 — The webviews on the engine's shapes

`RepoState` and `CommitDetail` of `src/shared/protocol.ts` become the engine's types (generated from `protocol`, or
written to match and checked by a test), and the conversion of stage 1 goes away. Nothing the user sees changes.

Acceptance: `src/engine` has no conversion of the state left; `npm run typecheck` passes with the types checked against
`protocol`'s JSON by a test.

The types are written to match, in `extension/src/shared/state.ts`, and the check is against the JSON the engine
writes, not against a schema: `protocol` has no schema of its messages, and writing one would add a dependency or a
second description to keep in step. `rust/crates/protocol/tests/shapes.rs` writes a state and the details of a commit
with every optional part filled as golden files, and `extension/test/engineShapes.ts` makes `npm run typecheck` fail
when a field is added, removed or renamed on either side, or holds a value of another kind
([extension/06-testing.md](../extension/06-testing.md#the-engines-shapes)).

## Stage 7 — The graph reads

Last, because they are the hottest path and the extension's own reading is tuned (`ac28e4c`, the prefetch 500 rows
ahead and only the new rows sent).

**Desktop first**: `graph.read-commits`, `graph.read-position` and `search.read-matches` are measured in the engine on
the repository `extension/test/perf/big-repo.mjs` makes (56,000 commits, 2,375 references, 1,779 changed files), the
one of QA1 to QA3 in [extension/12-quality.md](../extension/12-quality.md), with a benchmark of the engine added to
[desktop/06-testing.md](../desktop/06-testing.md).

Acceptance, on that repository and the maintainer's Windows machine, with the minified build:

- A page of 1,000 rows (the default of `gitardo.graph.pageSize`) is answered in under 300 ms, from the request to the
  last byte of the answer, as QA3 asks of the extension today.
- The graph opened from the Command Palette shows its first commits in about 1.5 s (QA1), and a saved file refreshes
  the views in about 250 ms (QA2), measured with `test/perf/vscode.mjs` as today.
- Scrolling to the end of that history in the graph never shows an empty row for more than one frame of the prefetch.
- A page asked and then scrolled past is cancelled with `$/cancel`.
- The lanes drawn are the same as before, compared row by row on the repositories of the integration tests.

## Stage 8 — One vsix per platform

`npm run package` builds one vsix per target with `vsce package --target`: `win32-x64`, `win32-arm64`, `linux-x64`,
`linux-arm64`, `darwin-x64`, `darwin-arm64`, `alpine-x64`; each holds `bin/<target>/gitardo-engine`. Until stage 10 a
universal vsix without a binary may still be published, and runs on the TypeScript code (D49).

Acceptance:

- Each vsix installs in VS Code on its platform and opens the graph of a repository with the engine; the vsix for
  `win32-x64` is checked by hand by the maintainer, the others by the pipeline of stage 9.
- The vsix of a platform holds that platform's binary only, and its size is said in the release notes.
- `extension/README.md` and `extension/CHANGELOG.md` in the vsix are as in the repository.

## Stage 9 — The pipeline

GitHub Actions: on every push, `cargo xtask check` on Windows, Linux and macOS (`macos-14`, both targets, which closes
M11 of the desktop, D48), the extension's checks on the three systems with the engine, and the vsix of every target as
artifacts. Publishing stays by hand (D36): the pipeline builds, the maintainer publishes.

Acceptance:

- A push to `main-merge` runs every check on the three systems and keeps the seven vsix and the binaries of the desktop
  as artifacts.
- The e2e suite of the extension passes on macOS with the engine.
- A failing check on one system fails the run and names the system.
- Nothing is published, pushed or tagged by the pipeline.

## Stage 10 — `src/git` removed

Only once every target has its binary from stage 9, since until then the TypeScript code is what runs where the binary
is missing (D49). `src/git/ops.ts`, `repo.ts`, `parse.ts`, `discover.ts`, `conflicts.ts`, `review.ts`,
`worktreeSetup.ts` and their tests are deleted; the fallback of D49 becomes the sentence alone; the universal vsix is
no longer published. `journal/record` is removed from the protocol (a major version only if a client still sent it:
none does).

Acceptance:

- `extension/src/git` does not exist; no file of `extension/src` runs `git`.
- With no binary, every view says **Gitardo's engine is not available on this system.** with **Show the log**.
- The integration tests of the extension that tested `src/git` are either moved to the engine's tests, where the same
  case is not there yet, or deleted, one by one, with the reason in the commit.

## The first step, for the session that starts it

Stage 1, desktop first, once the documents are approved. What that session needs:

1. Read [01-engine.md](01-engine.md), [02-protocol.md](02-protocol.md) up to **Reads**, this stage, D37 to D49 and
   Q393 in [08-decisions.md](../08-decisions.md), and [desktop/02-architecture.md](../desktop/02-architecture.md).
2. Ask the maintainer Q393 (how `platform` is split) if it is still open, with a recommendation.
3. In a worktree of its own from `main-merge`: change the picture of the crates first (one commit), then split
   `platform`, then `protocol`, then `server`, each a commit, with `cargo xtask check` passing after each, and the build
   with `CARGO_PROFILE_DEV_DEBUG=0`.
4. Then the client in `extension/src/engine`, behind nothing the user sees, with the acceptance tests above as tests of
   the extension (`test/engine.test.ts`) against the debug binary.
5. Stop at the end of stage 1 and report; stage 2 starts on the maintainer's word.

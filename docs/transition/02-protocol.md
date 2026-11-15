# The protocol between the extension and the engine

What the extension and `gitardo-engine --stdio` say to each other: every request, answer and notification, with the
shape of its JSON. Why there is an engine process and how it lives is in [01-engine.md](01-engine.md). The types named
here are those of `rust/crates/model` and `rust/crates/engine`; the crate `protocol` (D42) holds their JSON form and
converts them, so this page is what `protocol` must be, field by field. Where a type is long and plain, its fields are
given once in [Types](#types) and the messages name it.

This is version **1.7** of the protocol. Version 1.0 is what stage 1 built: starting and stopping, the repositories,
their state and the log. Version 1.1 adds what stage 2 built for the features: `feature/run`, `feature/ask`,
`feature/confirm`, `question/validate`, `feature/followUps` and `settings/changed`. Version 1.2 adds what stage 3 built
for Undo: `journal/last` and `journal/record`. Version 1.3 adds what stage 4 built for the conflicts (area 4): the
targets `resolutions`, the resolutions accepted on the review for `review.write`, and `mergeResult`, the file of a
Merge page for `merge-view.mark-resolved`. A plan the user agreed to in its own question (`Danger::Agreed`) is sent
with the danger `none`, and the engine does not send `feature/confirm` for it.

Version 1.3 adds what area 2 of stage 4 (branches, tags, remotes) needed to offer an operation after git refused one:
the follow-up `offerRun { label, feature, target: Target, answers: Record<string, Answer> }`, sent after a plan and in
`data.followUps` of `stepFailed`, and the answer `{ kind: "flag", value: boolean }`, which only an `offerRun` carries,
so that **Delete anyway** runs `branch.delete` told the branch is not merged, and **Remove anyway** runs
`worktree.remove` given `force` (area 5). A `flag` given in `feature/run` is known from the first call, as the desktop's
window gives what it knows, since no question asks it; the other answers given still answer their questions when asked.
An `offerRun` whose target or one of whose answers has no JSON yet is not sent, since it would run otherwise than
offered. The extension shows its `label` as a button on the message and, when it is clicked, sends `feature/run` with
that `feature`, `target` and `answers`. The data of an error is written in camel case, `followUps`, as this page always
said; before 1.3 the server wrote `follow_ups`.

Version 1.4 adds what stage 5 built: every read of the table of [Reads](#reads) but the three of the graph, and the host
question `host/githubToken` that `read/github.read-pull-requests` asks. The rest is not built yet.

Version 1.4 writes the parameters of some reads as the engine's functions take them, where the table of 1.2 had
guessed them: `diff.read-sides` takes the two versions of the file (`FileVersion` of the engine) rather than the two
sides of a comparison; `merge-view.read-sides` answers with the engine's names, `current` and `incoming`, and the file
as it is now; `compare.read-sides` and `compare.read-patch` take the request of `compare.read-sides`;
`compare.read-files` takes `from` and `to`; `blame.read-line` has no revision, which the engine's read does not take,
and both blame reads leave whitespace to the setting `blame.ignoreWhitespaceAndMoves`; `file-history.read-names` has no
revision; `worktree-setup.propose-commands` gives the file that calls for each command; `branch.read-log` takes a count
only, and answers whether there are more commits and which upstream the commits not pushed are counted against.

Version 1.5 adds what stage 7 built: the three reads of the graph, `read/graph.read-commits`,
`read/graph.read-position` and `read/search.read-matches`. Their `TimelineRequest` is written as [The graph](#the-graph)
gives it, with two additions to what 1.4 guessed: a `narrowing` may be a search shown as a list (`{ kind: "search",
mode, text }`), as the desktop's timeline lists one, and the answer of `read/search.read-matches` says whether the
search found more than the 5,000 commits it gives (`hasMore`).

Version 1.6 adds what area 6 of stage 4 (AI and GitHub) built: the host questions `host/languageModel` and
`host/languageModels` as [Host questions](#host-questions) now write them, without `system`, `stream` and
`host/languageModelPart`; the reads `read/commit.draft-message` and `read/ai.ask-conflict`, which ask them; the
follow-up `writeSetting { key, value }` (Q399), which `ai.choose-model` leaves under the extension in place of writing
the user's settings file; and the target `comparison`, a comparison page as a whole, for `compare.open-on-github`.
`host/githubToken` is asked by a feature too, with `ask: true`, for `github.load-pull-requests`. The engine sends
`$/cancel` for a question it asked of the extension and no longer waits for.

Version 1.7 adds what stage 10 needed once the extension has no git of its own (`extension/src/git` removed):
`repository/discover` answered, each repository with its `superproject` and the git folder its worktrees share,
`commonFolder`; the target `comparison` with the `file` of the page it was invoked on, for `compare.take-file`; and the
target `diffChange`, a change of a diff, for `diff.stage-change` and `diff.unstage-change` from the gutter of the Flow
Diff. `journal/record` is removed: no client sends it, since no operation runs outside the engine any more, so the
major version stays. `compare.read-sides` counts the whole sides of a comparison narrowed to a path and compares the
working tree from the merge base in **Since they diverged** (Q407), which changes no field.

## Transport

- **JSON-RPC 2.0** (D41) over the standard input and output of the engine, each message framed as the Language Server
  Protocol frames it: a `Content-Length: <bytes>` header, an empty line, the JSON in UTF-8. The extension can then use
  the framing of `vscode-jsonrpc` or write its own in a few lines, and the engine writes it with `serde_json` and no
  other library.
- Both sides send requests: the extension asks the engine to read and run; the engine asks the extension the questions
  of a feature and the [host questions](#host-questions). Each side answers the other's requests in any order.
- Notifications go one way, from the engine (state, log, activity) or from the extension (changes on disk, settings).
- What the engine writes on its standard error is for people: the extension copies it to the **Gitardo** output
  channel, marked as the engine's.

## Conventions of the JSON

- Field names in `camelCase`, as `serde(rename_all = "camelCase")` makes them from the names of `model`.
- An enum with data is an object with a `kind`: `{"kind": "branch", "name": "main"}`. An enum without data is a string
  in `camelCase`: `"cherryPick"`.
- `Option` is a field that may be `null`; a list is never `null`, but may be empty.
- A commit is its full id, 40 or 64 hexadecimal digits, as a string (`CommitId`).
- A time is a `Timestamp`: whole seconds since 1970-01-01 UTC (`model/src/identifier.rs`).
- A path is a string, as the system gives it; paths of files inside a repository are from the top of its working tree,
  with `/`, as git prints them.
- Words for the user (`Text` of `model/src/contract/words.rs`) are plain strings, already in their final form.
- Bytes (the contents of a file, a patch that may not be UTF-8) are `{"base64": "…"}`; text known to be UTF-8 is a
  string.
- Every request about a repository has `repository`: the folder of the top of its working tree, as
  `RepositoryState::folder` has it, and as `repository/open` was given it.

## Errors

A request that fails answers with a JSON-RPC error:

| Code | Name | When |
| --- | --- | --- |
| -32700, -32600, -32601, -32602, -32603 | JSON-RPC's own | Not JSON, not a request, no such method, wrong parameters, an answer the engine could not write (a bug) |
| 1 | `notOpen` | The repository was not opened, or was closed |
| 2 | `gitFailed` | A read of git failed (`EngineError::Query`, `GitError`) |
| 3 | `stepFailed` | A step of a plan failed (`EngineError::Step`), with what ran before it |
| 4 | `answerRefused` | An answer that its question does not take (`EngineError::AnswerRefused`) |
| 5 | `endlessFeature` | A feature that kept asking without deciding (`EngineError::EndlessFeature`), a bug |
| 6 | `busy` | An operation was asked while another runs in the same repository |
| 7 | `cancelled` | The request was cancelled with `$/cancel` |
| 8 | `unknownFeature` | No feature has that identifier |

`message` is the sentence for the user, as the engine's `Display` writes it. `data` is
`{"output": string | null, "steps": string[], "followUps": FollowUp[]}`: what git printed, the lines of the steps that
ran, for the log, and, for `stepFailed`, the follow-ups of the plan for a failure that are the extension's.

A feature that **refuses** is not an error: its answer is a `Finished` of kind `refused`.

## Starting and stopping

### `initialize`

The extension's first request.

```ts
params: {
  protocol: "1.7",
  client: { name: "gitardo-vscode", version: string },   // the version of the extension
  settings: Settings,                                     // see "Settings"
  host: {
    clipboard: boolean,        // it answers host/clipboard
    githubToken: boolean,      // it answers host/githubToken
    languageModel: boolean,    // it answers host/languageModel
    dialogs: boolean,          // it answers host/openFile, host/saveFile, host/chooseFolder
    secrets: boolean,          // it answers host/secret*
    terminal: boolean          // it runs the setup commands of a worktree itself (host/runTask)
  },
  logLevel: "info" | "debug"
}
result: { protocol: "1.x", engine: { version: string, git: string | null } }   // git: `git --version`, null when not found
```

A major version that differs is answered with the error `-32602` and the engine's version in `data`; the extension
stops there ([01-engine.md](01-engine.md#starting)).

### `shutdown` and `exit`

`shutdown` (request, no parameters, answer `null`): the engine ends what runs, writes nothing more. `exit`
(notification): it ends the process. An engine whose standard input closes ends as after `exit`.

### `$/cancel`

Notification, either way: `{ id }`, the id of a request. A read is stopped, its git process killed, and it answers
`cancelled`. A question asked of the extension and cancelled by the engine is closed by the extension: from 1.6 the
engine sends `$/cancel` with the id of its question when the request it was asked for is cancelled, and the extension
stops what it does for it, a language model's answer first. An operation
whose steps have started is not stopped; the request is ignored.

## Repositories

### `repository/discover`

```ts
params: { folders: string[] }                 // the folders of the workspace
result: { repositories: DiscoveredRepository[] }
```

`repo.read-repositories` (1.7): the repositories in those folders and around them, in the order the extension shows
them, the repositories in a folder before the one around it, each followed by its submodules. `DiscoveredRepository`:
`{ folder, superproject: string | null, commonFolder: string | null }`, `commonFolder` the git folder its worktrees
share (`repo.common-folder`), by which the extension tells which repository of the workspace a worktree chosen in
VS Code's Repositories view belongs to; `null` when git could not say it. It needs no repository opened, and opens
none; the extension also asks it with the folder of a file alone, to find the repository the file is in.

### `repository/open`

```ts
params: { repository: string }
result: { state: RepositoryState }
```

Makes an `Engine` for the repository and reads its state (`repo.read-state`). Opening one already open answers its
state.

### `repository/close`

`{ repository }` → `null`. Drops the `Engine`, its journal and its log.

### `repository/changed`

Notification from the extension: `{ repository, paths: string[] }`, the paths its watchers saw change, from the top of
the working tree, or in the git folder with a leading `.git/`. The engine leaves out what its own steps changed and
what git's `fsmonitor` writes, reads again once per burst, and sends `repository/state`.

### `repository/state`

Notification from the engine: `{ repository, state: RepositoryState, reason: "changed" | "ran" | "opened" }`. Sent
after a read again, and after every plan that ran.

## Settings

### `settings/changed`

Notification from the extension: `{ settings: Settings }`, all of them, whenever one changes. The features of the
engine see the new values from their next call; one that is asking keeps the snapshot it started with.

`Settings` is a JSON object whose keys are the names of the settings without `gitardo.`: `"graph.pageSize": 500`,
`"worktrees.linkFolders": ["node_modules/"]`. Those are the keys of Gitardo Desktop's `settings.json` already: the
desktop names `graph.pageSize` what the extension names `gitardo.graph.pageSize` (`crates/settings/src/keys.rs`,
`extension/package.json`). The engine builds its `FeatureSettings` (`model/src/contract/context.rs`) from that object
with the code that reads the desktop's `settings.json` (`feature_settings` of the crate `settings`, which stage 1 moved
out of `app` for it). The extension sends the values already resolved: the workspace's over the user's, the default of each
where none is set, and VS Code's `git.*` setting in place of an unset `gitardo.*` one where D31 says so
(`git.branchSortOrder`, `git.worktreeSymlinkFolders`, `git.worktreeIncludeFiles`).

Four keys are added that only the extension sends:

| Key | Value | Says |
| --- | --- | --- |
| `host` | `"vscode"` | The engine runs under the extension: the settings files of the desktop are not read nor written (below). |
| `ai.provider` | `"vscode"` | The language model is asked of the host (`host/languageModel`), not over HTTP. |
| `trust` | `"vscode"` | VS Code's workspace trust stands for `trust.ask` (Q396). |
| `utcOffsetMinutes` | number | The offset of the system, for names a format makes from dates. |

The keys the desktop has and the extension does not (`theme`, `font.*`, `editor.*`, `terminal.external`, `log.level`,
`ai.endpoint`) are not sent; those the extension has and the desktop does not are listed in
[05-stored-data.md](05-stored-data.md#settings), each with what becomes of it.

While `host` is `"vscode"` the engine reads no `settings.json` of its own and no `.gitardo/settings.json`, and the
features that write a setting (`worktree-setup.save`, `worktree-setup.edit-settings-file`, the toggles of the graph)
do not write a file: their plan ends in a follow-up `WriteSetting { key, value, target: "user" | "workspace" }`, which
the extension carries out with `WorkspaceConfiguration.update`, as `src/extension/worktreeSettings.ts` does today. That
follow-up is new in `model` (Q399).

## Reads

Every read is a request `read/<identifier of its card>`, with `repository` and the fields of its request. They never
change the repository and run at the same time as each other and as an operation. The engine logs each, as
`LogKind::Read` with how long it took.

| Method | Parameters besides `repository` | Result | Card (desktop) |
| --- | --- | --- | --- |
| `read/repo.read-state` | — | `RepositoryState` | `repo.read-state` |
| `read/graph.read-commits` | `TimelineRequest` | `CommitPage` | `graph.read-commits` |
| `read/graph.read-position` | `{ commit, timeline: TimelineRequest }` | `{ index: number \| null }` | `graph.read-position` |
| `read/search.read-matches` | `{ text, mode: SearchMode, timeline: TimelineRequest }` | `{ commits: CommitId[], hasMore }` | `search.read-matches` |
| `read/search.read-revision` | `{ revision }` | `{ commit: CommitId \| null }` | `search.read-revision` |
| `read/details.read-commit` | `{ commit }`, `{ stash: number }` or `{ uncommitted: true }` | `CommitDetail \| null` | `details.read-commit` |
| `read/commit-page.read-patch` | `{ commit, path: string \| null }` | `Patch` | `commit-page.read-patch` |
| `read/changes.read-working-tree` | — | `WorkingTree` | `changes.read-working-tree` |
| `read/changes.read-ignored` | `{ paths: string[] }` | `{ ignored: string[] }` | (`changes.unsaved`) |
| `read/diff.read-sides` | `{ left: Version, right: Version }` | `{ left: Bytes \| null, right: Bytes \| null }` | `diff.read-sides` |
| `read/file-view.read-revision` | `{ path, revision }` | `Bytes \| null` | `file-view.read-revision` |
| `read/merge-view.read-sides` | `{ path, unsaved: Bytes \| null }` | `MergeSides` | `merge-view.read-sides` |
| `read/conflicts.read-files` | — | `ConflictFile[]` | `conflicts.read-files` |
| `read/conflicts.side-names` | — | `{ current: Side, incoming: Side }` | `conflicts.side-names` |
| `read/conflicts.resolve-by-rules` | `{ paths: string[] \| null }` | `ConflictProposals[]` | `conflicts.resolve-by-rules` |
| `read/compare.read-sides` | `CompareRequest` | `CompareView` | `compare.read-sides` |
| `read/compare.read-files` | `{ from, to }` | `FileChange[]` | `compare.read-files` |
| `read/compare.read-patch` | `CompareRequest` and `{ mergeBase: CommitId \| null, files: FileChange[], form: "lines" \| "text" }` | `Patch` | `compare.read-patch` |
| `read/compare.resolve-revision` | `{ revision }` | `{ commit: CommitId \| null }` | `compare.resolve-revision` |
| `read/blame.read-line` | `{ path, line, contents: Bytes \| null }` | `BlameLine \| null` | `blame.read-line` |
| `read/blame.read-file` | `{ path, revision: string \| null, contents: Bytes \| null }` | `(BlameLine \| null)[] \| null` | `blame.read-file` |
| `read/file-history.read-names` | `{ path }` | `{ names: string[] }` | `file-history.read-names` |
| `read/file-history.read-committed-lines` | `{ path, renamedFrom: string \| null, text, lines: LineRange }` | `CommittedLines` | `file-history.read-committed-lines` |
| `read/reflog.read` | `{ reference: string, count: number }` | `ReflogPosition[]` | `reflog.read` |
| `read/remote.read-remotes` | — | `{ name, url }[]` | `remote.read-remotes` |
| `read/identity.read` | — | `{ identity: { name, email }, localKeys: string[] }` | `identity.read` |
| `read/submodule.read-list` | — | `Submodule[]` | `submodule.read-list` |
| `read/worktree.read-folder-links` | `{ worktree }` | `{ paths: string[] }` | `worktree.read-folder-links` |
| `read/worktree-setup.read-ignored` | `{ worktree }` | `IgnoredEntry[]` | `worktree-setup.read-ignored` |
| `read/worktree-setup.read-what-patterns-take` | `{ requests: TakeRequest[] }` | `SetupPreview[]` | `worktree-setup.read-what-patterns-take` |
| `read/worktree-setup.propose-commands` | — | `{ commands: { command, file }[] }` | `worktree-setup.propose-commands` |
| `read/worktree-setup.explain-change` | `{ paths: string[] }` | `PathStatus[]` | `worktree-setup.explain-change` |
| `read/github.read-pull-requests` | `{ remote: string \| null }` | `PullRequests` | `github.read-pull-requests` |
| `read/branch.read-log` | `{ count: number }` | `{ commits: Commit[], hasMore, unpushed: CommitId[] \| null, upstream: string \| null }` | `branch.read-log` |
| `read/repo.common-folder` | — | `{ folder: string }` | `repo.common-folder` |
| `read/commit.prepared-message` | — | `{ message: string \| null }` | `commit.prepared-message` |
| `read/commit.draft-message` | `{ subjects: string[] }` | `{ kind: "drafted", message } \| { kind: "failed", message }` | `commit.draft-message` (1.6) |
| `read/ai.ask-conflict` | `{ path, lines: string[], conflict: { start, end, current: string[], incoming: string[], base: string[] \| null } }` | `{ kind: "answered", model, lines: string[], reason, confidence } \| { kind: "failed", message, stopsTheRest }` | `ai.ask-conflict` (1.6) |

The three reads at the end had no desktop card: the map found them read in `crates/app` or not at all
([03-map.md](03-map.md#3-gaps-extension-side)). Stage 5 made each a card of the engine, desktop first (D39), before the
extension uses it (Q397, Q398).

The server answers each read on a thread of its own, never on the thread of the repository's engine, so that reads run
at the same time as each other and as an operation. Each gets the token that `$/cancel` cancels, which kills the git it
runs, and is logged as `LogKind::Read` with how long it took, as `Read <method> in <repository>`. A read whose
parameters name a commit by anything but its full id answers `-32602`; one whose git fails answers `gitFailed`.
`read/github.read-pull-requests` asks the extension `host/githubToken` with `{ host: "github.com", ask: false }` when
it said `host.githubToken` at `initialize`, and reads nothing without a token.

The two reads of 1.6 ask a language model of VS Code. `read/commit.draft-message` reads the working tree, the diff
of `commit.read-draft-diff` and, when nothing is staged, the untracked files, writes the request of
`commit.draft-message` with the subjects the extension gives, asks VS Code's first model with `host/languageModel`
(`model` and `effort` null, as the extension asked it), and cleans the answer; with nothing changed it answers
`failed` with *There are no changes to describe.* without asking. `read/ai.ask-conflict` is the question about one
conflict of the review, which stays in the extension: the extension gives the file's lines and the conflict, `base`
the base as far as it knows it; the engine reads the names of the two sides (`conflicts.side-names`), writes the
question (`ai.conflict-question`), asks the model of `ai.model` with the effort of `ai.effort`, then reads and trims
the answer (`ai.read-answer`, `ai.trim-answer`). Both give `read` in `host/languageModel`, the id of the read, so that
the extension shows the answer as it comes where the read was asked; a model that fails or refuses is a `failed`
answer, in the host's words; cancelled, the read answers `cancelled`.

The reads a feature asks for itself (`Outcome::Read` with a `Query`) are not in this list: the engine answers them
inside `feature/run`, and they never reach the extension, but for the host questions.

### The graph

`TimelineRequest` is what the timeline of the desktop asks (`crates/engine/src/timeline.rs`), which the engine
turns into a `CommitsRequest` (`crates/git/src/read/graph/commits.rs`):

```ts
TimelineRequest = {
  shown: BranchesShown | null,     // null: every branch; else the branches ticked in the graph's selector
  parents: "all" | "first",        // gitardo.graph.firstParent
  alone: CommitId | null,          // only this commit's history (graph.show-in-graph of a commit not listed)
  narrowing: Narrowing | null,     // the history of a file, of lines, of a folder, or a search shown as a list
  pageSize: number,                // gitardo.graph.pageSize
  pages: number,                   // how many pages from the newest commit
  skip: number,                    // the rows the extension has already: the read starts after them
  stashes: boolean                 // gitardo.graph.showStashes
}
BranchesShown = { kind: "current" } | { kind: "upstream" }
    | { kind: "kinds", local: boolean, remote: boolean, tags: boolean }
    | { kind: "references", names: string[] }   // full names, refs/heads/main
Narrowing = { kind: "file", path, earlier: string[] }   // earlier: the names it had before, newest first
    | { kind: "lines", path, lines: LineRange, commit: CommitId, inEditor: LineRange | null }
    | { kind: "folder", path }
    | { kind: "search", mode: SearchMode, text }
SearchMode = "message" | "author" | "changes" | "changesPattern"
CommitPage = { rows: Row[], hasMore: boolean }
Row = { kind: "commit", commit: Commit }
    | { kind: "stash", commit: Commit, index: number }
    | { kind: "uncommitted", parent: CommitId | null, files: number }
Commit = { id, parents: CommitId[], authorName, authorEmail, authorDate: Timestamp, subject }
```

`history_of` and `commits_request` are in the engine (`engine::timeline`, moved from `crates/app` in stage 1), since the
extension must not build git's arguments. In `read/graph.read-commits` the fields of `TimelineRequest` are beside
`repository`; the two other reads take it as `timeline`, and how much of it is read does not matter to them. `skip`
makes the next page carry only the new rows, as the extension's prefetch does since `ac28e4c`
(`gitardo.graph.pageSize`, 500 rows ahead): the extension asks from the last commit it has, `skip` one less than the
commits it has, and checks that the page starts with it. The lanes stay computed by the extension
(`src/shared/graph.ts`) from `parents`. A read of the graph never lists the row of the changes not committed, which
the extension adds; a stash comes with its index, which the extension names `stash@{index}`. A history git cannot
list, such as a search of a pattern git refuses, is the error `gitFailed`; a read cancelled with `$/cancel` stops its
git and answers `cancelled`. A page of the repository of 56,000 commits was to answer in less than 300 ms
([06-roadmap.md](06-roadmap.md#stage-7--the-graph-reads)); what `cargo xtask perf` measured is in
[the desktop's testing](../desktop/06-testing.md).

## Features

### `feature/run`

The extension's request to run a feature, an Operation or a Derivation of the catalog that is in
`features::OPERATIONS` (`crates/features/src/lib.rs`).

```ts
params: {
  repository: string,
  feature: string,                 // "branch.create"
  target: Target,                  // what it was asked on
  answers: Record<string, Answer>, // answers given before it asks: from an offer (OfferedRun), or a view that knows them
  journal: boolean,                // false only in the tests that check the plan alone; read from stage 3
  inputs: Record<string, string>   // the values of the inputs of branches.format used last (worktreeInputs)
}
result: Finished
```

The answer comes when the feature has finished: refused, backed out of, not confirmed, or run. In between the engine
asks the extension, as requests on the same connection: `feature/ask` for each question, `feature/confirm` for the
plan when it must be confirmed, and the host questions. Only one `feature/run` runs at a time in a repository, from
its start to its answer, questions included; a second is answered `busy`, and the extension drops it without a word,
as `Controller.run` does today while `this.running` is set (`controller.ts:2421`). The engine of each repository lives
on a thread of its own (`crates/server/src/repository.rs`), which takes one feature at a time.

An answer in `answers` answers the question of its key without asking it, once. A `confirmed` answer is also read by a
feature that never asks it: **Remove anyway** sends `force` so (1.3). `inputs` is what the desktop keeps in
the workspace's file and the extension in its workspace state: the engine reads it as `FeatureSettings::branch_values`,
the defaults of the questions of `branches.format`, and a plan that used them ends with `RememberInputs`.

```ts
Finished =
  | { kind: "ran", plan: Plan, steps: string[], wentOnAfter: StepFailure[], waiting: Step[] }
  | { kind: "refused", message: string, offer: { feature: string, label: string } | null }
  | { kind: "backedOut" }
  | { kind: "notConfirmed", plan: Plan }
```

`state_after` of the engine's `Finished::Ran` is not sent: a `repository/state` notification follows every plan.
`waiting` holds the setup commands of a new worktree when the extension said `host.terminal`: it runs them in VS Code's
terminal, as `src/extension/worktreeCommands.ts` does.

A step that fails answers the error `stepFailed`. The follow-ups of the plan for a failure (`Recover`, `OfferRun`,
`OfferWhenFailed`, `AllowFileProtocol`) are carried out by the engine where they are its own, and come back in the
error's `data.followUps` where the extension must offer them.

### `feature/ask`

The engine's request for an answer to a question (`Asker::ask`).

```ts
params: { run: number, question: Question }   // run: the id of the feature/run request it belongs to
result: { answer: Answer | null }             // null: the user backed out (Escape)
```

### `feature/confirm`

The engine's request to confirm a plan (`Asker::confirm`), sent when its `danger` is not `none` or
`gitardo.commandPreview` asks for it.

```ts
params: { run: number, plan: Plan }
result: { run: boolean }
```

### `question/validate`

The extension's request while the user types in an input box: whether the text so far is an answer the question takes.

```ts
params: { repository: string, validation: Validation, text: string }
result: { problem: string | null }            // the sentence under the box, null when the text is fine
```

`Validation` is pure (`model/src/contract/question.rs`); the request exists only because the extension cannot run
Rust. The extension calls it on each change, without waiting on the previous one, and shows the answer of the latest.

### `feature/followUps`

Notification from the engine, right before the answer of a `feature/run` that ran: `{ run, followUps: FollowUp[] }`,
the follow-ups that are the host's to carry out, in their order. The engine keeps those that are its own. Each is an
object with a `kind` and the fields of its variant. Stage 2 sends `say`, `selectHead`, `selectCommit`,
`selectBeforeHead`, `openCommitPage`, `emptyCommitMessage`, `preparedMessage`, `fillCommitMessage`,
`readPullRequests`, `offer`, `copyToClipboard`, `openUrl`, `openReview`, `closeMergePage`, `openWindow`, `showFolder`,
`rememberInputs` and `editFile` (`crates/protocol/src/feature/follow_up.rs`); version 1.3 adds `offerRun`; the others
are sent from the stage that moves their features.

`offerRun` is `{ kind: "offerRun", label, feature, target: Target, answers: Record<string, Answer> }`: a button named
`label` that runs `feature` on `target` with `answers` given, as **Remove anyway** after git refuses a worktree with
changes, or **Choose What It Gets…** (`worktree-setup.page`) after a worktree is made. An offer whose target or answers
cannot cross (an answer of the engine's own reads, such as a list of names) is left out.

| `FollowUp` | Carried out by | In the extension |
| --- | --- | --- |
| `Say` | extension | an information message |
| `SelectHead`, `SelectCommit`, `SelectBeforeHead` | extension | the graph selects the row |
| `OpenCommitPage` | extension | the Commit tab |
| `EmptyCommitMessage`, `FillCommitMessage`, `PreparedMessage` | extension | the commit box of the Changes view |
| `ReadPullRequests` | extension | `read/github.read-pull-requests` again |
| `KeepGitHubToken` | not sent | the extension takes the token from VS Code's sign-in |
| `KeepAiKey` | not sent | VS Code's language models need no key |
| `Offer`, `OfferRun` | extension | a button on the message, which sends `feature/run` |
| `CopyToClipboard` | extension | `vscode.env.clipboard` |
| `OpenUrl` | extension | `vscode.env.openExternal` |
| `CheckAutostash`, `CheckBisect` | engine | it reads again and adds a `Say` |
| `Recover`, `OfferWhenFailed`, `AllowFileProtocol` | engine | on a failure, see above |
| `DeleteWhenRebaseEnded`, `Cleanup` | engine | |
| `OpenComparison`, `CompareInPlace` | extension | the Compare tab |
| `OpenReview` | extension | the review of the conflicts |
| `CloseMergePage` | extension | closes the Flow Merge or the merge editor of that file |
| `OpenWindow` | extension | `vscode.openFolder` in a new window |
| `ShowFolder` | extension | the repository shown becomes that folder |
| `ShowHistory` | extension | the graph narrowed to a file, lines or folder |
| `ShowBlame`, `HideBlame`, `OpenFile` | extension | the blame of an editor, an editor at a line |
| `RememberInputs` | extension | the workspace state `worktreeInputs` |
| `EditFile` | extension | an editor of that file at that line |
| `OpenFileDiff` | extension | VS Code's diff editor or the Flow Diff |
| `TrustFolder`, `ForgetTrustedFolders` | not sent | VS Code's workspace trust (Q396) |
| `WriteSetting` (1.6, Q399) | extension | `WorkspaceConfiguration.update` in the user's settings, `{ key, value }` by the desktop's name, `value` null to remove it; `ai.model` and `ai.effort` are `gitardo.conflicts.aiModel` and `gitardo.conflicts.aiEffort` (Q400) |

## Undo and the journal

### `journal/last`

`{ repository }` → `{ entry: JournalEntry | null }`: the newest entry that still holds, for the label of **Undo**.

`journal/record`, by which the extension told the engine of an operation it ran in TypeScript, was removed in 1.7
with that TypeScript (stage 10).

## The log and the activity

### `log/line`

Notification from the engine: `{ repository: string | null, entry: LogEntry }`, each line as it is written.

```ts
LogEntry = { time: Timestamp, kind: "step" | "failure" | "warning" | { kind: "read", milliseconds: number }, line: string }
```

The extension writes it in the **Gitardo** output channel as today (`Controller.run` logs, `controller.ts`). Reads are
sent only at `logLevel` `"debug"`.

### `activity/changed`

Notification from the engine: `{ running: ActivityEntry[] }`, all that runs in every repository, whenever it changes.
`ActivityEntry = { number, description: string, cause: string, started: Timestamp }`. The extension's status bar item
(`src/extension/activity.ts`) shows it.

## Host questions

What only VS Code has, asked by the engine with requests to the extension while a feature runs or a read needs it.
Each answers `null` when the user backs out or the host cannot.

| Method | Params | Result | Stands for, in the engine |
| --- | --- | --- | --- |
| `host/clipboard` | — | `{ text: string \| null }` | `Asker::clipboard`, `Query::Clipboard`; not sent: the extension gives the text as the answer `clipboard` instead (stage 4, area 3) |
| `host/githubToken` | `{ host: string, ask: boolean }` | `{ token: string \| null }` | `Query::GitHubToken`, `github::read_token` and the credential store |
| `host/languageModel` | `LanguageModelRequest` | `{ text: string, model: string } \| { refused: string }` | `ai::LanguageModel::ask`, over HTTP in the desktop (`ai::ProviderModel`) |
| `host/languageModels` | `{}` | `{ models: { id, name }[] }` | `Host::language_models`, `Query::AiModels`, `ai::list_models` |
| `host/openFile` | `FileQuestion` | `{ paths: string[] }` | `QuestionKind::OpenFile` |
| `host/saveFile` | `FileQuestion` | `{ path: string \| null }` | `QuestionKind::SaveFile` |
| `host/secret` | `{ name }` | `{ value: string \| null }` | `system::read_secret` |
| `host/storeSecret` | `{ name, value: string \| null }` | `null` | `system::write_secret`, `delete_secret` |

`LanguageModelRequest = { purpose: "commitMessage" | "conflict", model: string | null, effort: "low" | "medium" |
"high" | "xhigh" | "max" | null, messages: { role: "user" | "assistant", text: string }[], read: id | null }`
(1.6). `model` is the id of the model, null for VS Code's first; `effort` goes as `configuration.reasoningEffort` of
the request, null for as the model is set up; `purpose` chooses the justification VS Code shows. `read` is the id of
the extension's read the question is asked for, null when a feature asks: the extension shows the text as it comes
where that read was asked, and answers with the whole text and the name of the model. `refused` is why it could not
ask, in the extension's words: no model, the model chosen not there, or the model refused. The prompt and the reading
of the answer stay in the engine (`crates/engine/src/ai.rs`). Before 1.6 this page had a `system` text, a `stream`
flag and `host/languageModelPart` notifications: VS Code's models take no system message, and the parts are the
extension's to show, so 1.6 has none of them.

The engine calls the AI provider and GitHub itself in the desktop (`ai.rs`, `github.rs`). Running under the
extension, the `Engine` gets a host for those (`engine::Host`, given with `Engine::ask_host_with`, as it gets a
`SecretStore` with `keep_secrets_in`): the server's `HostQuestions` (`crates/server/src/host.rs`) asks the extension
`host/githubToken`, `host/languageModels` and `host/languageModel`, and the desktop gives none and keeps the HTTP. Under
a host, no token or key is kept in the credential store (`KeepGitHubToken`, `KeepAiKey`), the user backing out of
signing in to GitHub backs out of the feature, and the features that ask a model do it through
`engine::ai::LanguageModel`, which the provider and the host both are. `FeatureSettings.uses_host_models` tells
`ai.choose-model` that the models are the host's: it needs no provider or key, and leaves `writeSetting` for the two
settings.

## How the Asker is translated

`Asker` (`crates/engine/src/drive.rs:39`) has three methods. In the server, each `feature/run` runs on the thread of
its repository's engine with a `HostAsker` (`crates/server/src/asker.rs`), which holds the id of the request and sends
its requests through the writer of standard output, waiting for their answers:

| `Asker` | Becomes | In the extension |
| --- | --- | --- |
| `ask(&Question) -> Option<Answer>` | `feature/ask`, waits for its answer | by `QuestionKind`, below |
| `confirm(&Plan) -> bool` | `feature/confirm`, waits | the modal dialog of today, with the plan's commands (`gitardo.commandPreview`) |
| `clipboard() -> Option<String>` | the text answer `clipboard` given with `feature/run`, taken once; none without it | `vscode.env.clipboard.readText()`, read by the controller before it runs **Apply patch** |

The answers given in `feature/run` are taken first: a question whose key has one is not sent. A question has `key`,
`kind`, `title`, `detail`, `default`. In VS Code:

| `QuestionKind` | VS Code | `Answer` sent |
| --- | --- | --- |
| `text { placeholder, validation }` | `showInputBox`, with `question/validate` as its `validateInput` | `{ kind: "text", text }` |
| `secret` | `showInputBox` with `password: true` | `{ kind: "text", text }` |
| `pick { choices }` | `showQuickPick` | `{ kind: "choice", identifier }` |
| `search { choices }` | `showQuickPick` with `matchOnDescription` | `{ kind: "choice", identifier }` |
| `pickMany { choices }` | `showQuickPick` with `canPickMany` | `{ kind: "choices", identifiers }` |
| `confirm { accept }` | `showWarningMessage` modal, the button `accept` | `{ kind: "confirmed" }` |
| `openFile` | `showOpenDialog` (through `host/openFile` when the engine asks outside a feature) | `{ kind: "path", path }` |
| `saveFile` | `showSaveDialog` | `{ kind: "path", path }` |

The engine checks each answer again (`check_answer` in `drive.rs`), so a wrong answer is `answerRefused`, never a plan
run on it. The questions the extension asks today in `src/extension/actions.ts` and `src/extension/controller.ts` are
replaced by these, feature by feature; their words become the engine's, which is where the two apps already differ in
places ([04-differences.md](04-differences.md)).

## Types

The fields are those of `model`; this section says only what the JSON adds or leaves out.

- **`RepositoryState`** (`model/src/state.rs`): `folder`, `gitFolder`, `head: { commit: CommitId | null, branch:
  string | null }` (the full name, `refs/heads/main`), `branches: Branch[]`, `remoteBranches: RemoteBranch[]`,
  `tags: Tag[]`, `stashes: Stash[]`, `worktrees: Worktree[]`, `remotes: string[]`, `identity: { name, email }`,
  `branchSort: string | null`, `reflogSubjects: string[]`, `workingTree: WorkingTree`, `operation: "merge" |
  "rebase" | "cherryPick" | "revert" | "patch" | … | null`, `rebaseStop: RebaseStop | null`, `bisect: Bisect | null`,
  `submodules: Submodule[]`.
- **`Branch`**: `name`, `commit`, `upstream: { reference, remote, remoteBranch, ahead, behind, isGone } | null`,
  `isCheckedOut`, `date`. **`RemoteBranch`**: `name`, `remote`, `commit`, `date`. **`Tag`**: `name`, `commit`, `date`.
  **`Stash`**: `commit`, `index`, `message`, `date`. **`Worktree`**: `folder`, `commit`, `branch`, `isMain`,
  `isCurrent`, `isMissing`, `uncommitted: number | null`.
- **`WorkingTree`**: `staged: FileChange[]`, `unstaged: FileChange[]`, `untracked: string[]`, `conflicted:
  ConflictedFile[]`. **`FileChange`**: `path`, `originalPath: string | null`, `kind: "added" | "modified" | "deleted" |
  "renamed" | "copied" | …`.
- **`CommitDetail`** (`model/src/details.rs`): `id: CommitId | null`, `parents`, `author`, `committer` (each
  `{ name, email, date }` or `null`), `message`, `files: FileChange[]`, `untrackedFiles: FileChange[]`.
- **`Target`** (`model/src/contract/context.rs`): `{ kind: "head" }`, `{ kind: "commit", commit }`, `{ kind:
  "branch", name }`, `{ kind: "remoteBranch", name }`, `{ kind: "tag", name }`, `{ kind: "stash", index }`, `{ kind:
  "remote", name }`, `{ kind: "files", paths }`, `{ kind: "file", path }`, `{ kind: "changes" }`, `{ kind: "commits",
  commits }`, `{ kind: "submodule", path }`, `{ kind: "worktree", folder }`, `{ kind: "path", path }`, built at stage
  2, and the kinds with a structure of their own: `diffChange`, `comparison`, `mergeResult`, `resolutions`,
  `fileSave`, `fileSaves`, `fileAt`, `setupSettings`, `lines`, `lineOfFile`, `listedFile`, each with the fields of its
  type in `model`, built with the stage that moves the area that uses it. Built in 1.3: `{ kind: "resolutions", files:
  FileResolutions[] }` and `{ kind: "mergeResult", path, unsaved: string | null, marked }` (the text not saved, or
  `null` when the file on disk is the result; how many conflicts git still marks in it). Built in 1.6: `{ kind:
  "comparison", first: CompareSide, second: CompareSide, mode: "sinceDiverged" | "everything", path: string | null,
  mergeBase: CommitId | null }`, a comparison page as a whole, each side `{ name, side: { kind: "commit", commit } |
  { kind: "workingTree" }, isFollowed, isGone }`. Built in 1.7: `file: FileChange | null` in `comparison`, the file of
  the page it was invoked on (`compare.take-file`), and `{ kind: "diffChange", path, left: DiffSide, right: DiffSide,
  leftLines, rightLines, leftText: string[], rightText: string[] }`, a change of a diff, each `DiffSide` `{ kind:
  "head" }`, `{ kind: "index" }`, `{ kind: "workingTree" }`, `{ kind: "commit", commit }` or `{ kind: "empty" }`, the
  lines `{ start, end }` counted from 0 with the end left out, and the text of those lines as the diff showed them.
- **`FileResolutions`** (`model/src/review/resolutions.rs`, 1.3): `{ path, text, conflicts, accepted: Proposal[] }`:
  the text the proposals were made from, which the file must still have to be written, and how many conflicts it has.
  **`Proposal`**: `{ source, from, to, text, newLines }`, the lines `from` to `to` of the file (from 0, `to` left out)
  replaced by `text`, each line with what ends it; `source` is `{ kind: "rules", case: "spacesOrLineEndings" |
  "sameChange" | "oneSide" | "nextToEachOther" }`, `{ kind: "ai", model, reason, confidence: "high" | "medium" | "low"
  }` or `{ kind: "choice", choice: "currentOnly" | "incomingOnly" | "bothCurrentFirst" | "bothIncomingFirst" |
  "written" }`.
- **`Question`** (`model/src/contract/question.rs`): `{ key, kind: QuestionKind, title, detail: string | null,
  default: Answer | null }`. **`Choice`**: `{ identifier, label, description: string | null }`. **`Validation`**,
  every kind an object, those without data too, since `question/validate` sends it back: `{ kind: "anything" }`,
  `{ kind: "referenceName", taken: string[], takenMessage }`, `{ kind: "url" }`, `{ kind: "newMessage", old }`,
  `{ kind: "required", message }`, `{ kind: "optionalReferenceName" }`, `{ kind: "submoduleFolder", submodules }`,
  `{ kind: "worktreeName", branches, emptyName, worktrees, beside, takenFolders }`, `{ kind: "pattern", pattern }`.
  **`FileQuestion`**: `{ folder: string | null, fileName: string | null, filters: { name, extensions }[] }`.
- **`Answer`** (`model/src/contract/answers.rs`): only the kinds a person gives travel to the engine: `text`,
  `choice`, `choices`, `confirmed`, `path`, and `flag { value }` (1.3) in the `answers` of an `offerRun`, which the
  extension sends back as it received them. The other kinds are answers of the engine's own queries and never cross;
  an `offerRun` that would need one is not sent.
- **`Plan`** (`model/src/contract/plan.rs`): `{ title, steps: Step[], danger: "none" | { kind: "confirm", reason } |
  { kind: "destructive", reason }, undo: "byReflog" | { kind: "impossible", reason } | { kind: "journal" } | {
  kind: "takesBack", number }, after: FollowUp[] }`. The `UndoRecord` of `journal` stays in the engine.
- **`Step`**: `{ kind: "git", arguments: string[], directory: string | null, input: string | null }`, `{ kind: "write",
  path }`, `{ kind: "link", from, to }`, `{ kind: "copy", from, to }`, `{ kind: "unlink", path }`, `{ kind: "delete",
  path }`, `{ kind: "run", command, directory }`, `{ kind: "task", command, directory }`, `{ kind: "makeFolder",
  path }`, `{ kind: "deleteEmptyFolder", path }`, `{ kind: "warn", text }`, each with a `line` too: the line the
  engine logs for it (`Step::log_line`). The extension shows them only, in the confirmation and in the log, by that
  line.
- **`JournalEntry`** (`protocol/src/journal.rs`): `{ number, title, undoable: boolean, head: Head }`; `undoable` is
  false for an operation Undo cannot take back. The record stays in the engine.

The shapes of the reads (`protocol/src/reads/`), each from the engine's type it names:

- **`Bytes`**: `{ base64: string }`, padded with `=`.
- **`Version`** (`FileVersion` of `git::read::diff`): `{ version: { kind: "absent" } | { kind: "index" } | { kind:
  "commit", commit } | { kind: "base" } | { kind: "workingTree", unsaved: Bytes | null }, path }`.
- **`MergeSides`**: `{ base, current, incoming, result: Bytes, operation: Operation | null, hasBothSides: boolean }`,
  each side empty when its stage has no file.
- **`ConflictFile`**: `{ path, base, current, incoming }`, each stage `{ mode, object } | null`.
- **`Side`** (`model::conflicts`): `{ role: "branchCheckedOut" | "branchBeingMerged" | "rebasedOnto" |
  "commitBeingReplayed" | "commitBeingPicked" | "stateBeforeReverted" | "patchBeingApplied" | "yourFiles" |
  "changesBeingApplied", name: string | null, subject: string | null }`.
- **`ConflictProposals`**: `{ path, leftAlone: "tooLarge" | "noMarkers" | "binary" | "notUtf8" | "tooManyLines" |
  "markersDoNotPair" | "tooComplex" | null, conflicts: { line, span: Span, proposal: { case: "spacesOrLineEndings" |
  "sameChange" | "oneSide" | "nextToEachOther", replaced: Span, lines: { text, ending }[] } | null }[] }`, a `Span`
  being `{ start, end }`, lines of the file from 0, the end left out.
- **`CompareRequest`**: `{ first, second, mode: "sinceDiverged" | "everything", path: string | null }`, each side
  `{ kind: "commit", commit } | { kind: "workingTree" }`. **`CompareView`**: `{ mergeBase: CommitId | null,
  onlyInFirst, onlyInSecond, files: FileChange[], commits: { side: "first" | "second", id, authorName, authorDate,
  subject, isAlsoInOtherSide }[], merge: { kind: "notAsked" } | { kind: "clean" } | { kind: "conflicts", files } |
  { kind: "unknown" } }`.
- **`Patch`**: `{ kind: "files", files: FilePatch[] } | { kind: "unmatched" } | { kind: "text", text: Bytes }`;
  `FilePatch = { path, originalPath, kind, hunks: { header, lines: { kind: "context" | "added" | "removed" |
  "noNewline", text, before: number | null, after: number | null }[] }[], added, removed, isBinary, isPastLimit }`.
- **`BlameLine`**: `{ kind: "notCommitted" } | { kind: "committed", commit, author, authorTime, summary, previous:
  { commit, path } | null }`.
- **`LineRange`**: `{ start, end }`, from 1, the end in. **`CommittedLines`**: `{ kind: "inNoCommit" } | { kind:
  "committed", commit, path, lines: LineRange | null }`.
- **`ReflogPosition`**: `{ commit, subject, selector, time }`.
- **`IgnoredEntry`**: `{ path, kind: { kind: "file", bytes } | { kind: "folder", files, bytes, hasMore } | { kind:
  "link" } }`. **`TakeRequest`**: `{ folder, link: string[], copy: string[], commit: string | null }`.
  **`SetupPreview`**: `{ linked, copied, linkPatterns, copyPatterns, already }`, each pattern `{ pattern, count, paths
  }`. **`PathStatus`**: `{ isFolder, tracked: "nothing" | "it" | "filesIn", ignored: "whole" | "part" | "nothing" }`.
- **`PullRequests`**: `{ kind: "read", byBranch: Record<string, { number, title, url, isDraft, branch, checks: "passed"
  | "failed" | "running" | null }> } | { kind: "notOnGitHub" } | { kind: "noToken" } | { kind: "failed", message,
  isUnauthorized }`.
- **`Commit`** is that of [The graph](#the-graph).

The protocol's own tests round-trip each type through JSON and check that every variant of every enum above has its
JSON form ([07-rules.md](07-rules.md)).

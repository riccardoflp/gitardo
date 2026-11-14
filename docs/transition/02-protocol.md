# The protocol between the extension and the engine

What the extension and `gitardo-engine --stdio` say to each other: every request, answer and notification, with the
shape of its JSON. Why there is an engine process and how it lives is in [01-engine.md](01-engine.md). The types named
here are those of `rust/crates/model` and `rust/crates/engine`; the crate `protocol` (D42) holds their JSON form and
converts them, so this page is what `protocol` must be, field by field. Where a type is long and plain, its fields are
given once in [Types](#types) and the messages name it.

This is version **1.0** of the protocol. Nothing of it is built yet.

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
| -32700, -32600, -32601, -32602 | JSON-RPC's own | Not JSON, not a request, no such method, wrong parameters |
| 1 | `notOpen` | The repository was not opened, or was closed |
| 2 | `gitFailed` | A read of git failed (`EngineError::Query`, `GitError`) |
| 3 | `stepFailed` | A step of a plan failed (`EngineError::Step`), with what ran before it |
| 4 | `answerRefused` | An answer that its question does not take (`EngineError::AnswerRefused`) |
| 5 | `endlessFeature` | A feature that kept asking without deciding (`EngineError::EndlessFeature`), a bug |
| 6 | `busy` | An operation was asked while another runs in the same repository |
| 7 | `cancelled` | The request was cancelled with `$/cancel` |
| 8 | `unknownFeature` | No feature has that identifier |

`message` is the sentence for the user, as the engine's `Display` writes it. `data` is
`{"output": string | null, "steps": string[]}`: what git printed, and the lines of the steps that ran, for the log.

A feature that **refuses** is not an error: its answer is a `Finished` of kind `refused`.

## Starting and stopping

### `initialize`

The extension's first request.

```ts
params: {
  protocol: "1.0",
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
`cancelled`. A question asked of the extension and cancelled by the engine is closed by the extension. An operation
whose steps have started is not stopped; the request is ignored.

## Repositories

### `repository/discover`

```ts
params: { folders: string[] }                 // the folders of the workspace
result: { repositories: DiscoveredRepository[] }
```

`repo.read-repositories`: the repositories in those folders and around them, in the order the extension shows them
(`src/git/discover.ts:40`, the repositories in a folder before the one around it). `DiscoveredRepository`: `{ folder,
gitFolder, isWorktreeOf: string | null, isSubmoduleOf: string | null }`.

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
| `read/search.read-matches` | `{ text, mode: SearchMode, timeline: TimelineRequest }` | `{ commits: CommitId[] }` | `search.read-matches` |
| `read/search.read-revision` | `{ revision }` | `{ commit: CommitId \| null }` | `search.read-revision` |
| `read/details.read-commit` | `{ commit }` or `{ stash: number }` | `CommitDetail \| null` | `details.read-commit` |
| `read/commit-page.read-patch` | `{ commit, path: string \| null }` | `Patch` | `commit-page.read-patch` |
| `read/changes.read-working-tree` | — | `WorkingTree` | `changes.read-working-tree` |
| `read/changes.read-ignored` | `{ paths: string[] }` | `{ ignored: string[] }` | (`changes.unsaved`) |
| `read/diff.read-sides` | `FileSides` | `{ left: Bytes \| null, right: Bytes \| null }` | `diff.read-sides` |
| `read/file-view.read-revision` | `{ path, revision }` | `Bytes \| null` | `file-view.read-revision` |
| `read/merge-view.read-sides` | `{ path }` | `{ base, ours, theirs, merged }` (each `Bytes \| null`) | `merge-view.read-sides` |
| `read/conflicts.read-files` | — | `ConflictFile[]` | `conflicts.read-files` |
| `read/conflicts.side-names` | — | `{ ours: SideName, theirs: SideName }` | `conflicts.side-names` |
| `read/conflicts.resolve-by-rules` | `{ paths: string[] \| null }` | `Proposal[]` | `conflicts.resolve-by-rules` |
| `read/compare.read-sides` | `ComparisonTarget` | `ComparisonSides` | `compare.read-sides` |
| `read/compare.read-files` | `{ left, right }` | `FileChange[]` | `compare.read-files` |
| `read/compare.read-patch` | `ComparisonTarget` | `Patch` | `compare.read-patch` |
| `read/compare.resolve-revision` | `{ revision }` | `{ commit: CommitId \| null }` | `compare.resolve-revision` |
| `read/blame.read-line` | `{ path, line, revision: string \| null, contents: Bytes \| null }` | `BlameLine \| null` | `blame.read-line` |
| `read/blame.read-file` | `{ path, revision: string \| null, contents: Bytes \| null }` | `(BlameLine \| null)[] \| null` | `blame.read-file` |
| `read/file-history.read-names` | `{ path, revision }` | `{ names: string[] }` | `file-history.read-names` |
| `read/file-history.read-committed-lines` | `CommittedLinesQuery` | `CommittedLines` | `file-history.read-committed-lines` |
| `read/reflog.read` | `{ reference: string, count: number }` | `ReflogEntry[]` | `reflog.read` |
| `read/remote.read-remotes` | — | `Remote[]` | `remote.read-remotes` |
| `read/identity.read` | — | `IdentityKeys` | `identity.read` |
| `read/submodule.read-list` | — | `Submodule[]` | `submodule.read-list` |
| `read/worktree.read-folder-links` | `{ worktree }` | `{ paths: string[] }` | `worktree.read-folder-links` |
| `read/worktree-setup.read-ignored` | `{ worktree }` | `IgnoredEntry[]` | `worktree-setup.read-ignored` |
| `read/worktree-setup.read-what-patterns-take` | `{ requests: TakeRequest[] }` | `SetupPreview[]` | `worktree-setup.read-what-patterns-take` |
| `read/worktree-setup.propose-commands` | — | `{ commands: string[] }` | `worktree-setup.propose-commands` |
| `read/worktree-setup.explain-change` | `{ paths: string[] }` | `PathStatus[]` | `worktree-setup.explain-change` |
| `read/github.read-pull-requests` | `{ remote }` | `PullRequests` | `github.read-pull-requests` |
| `read/branch.read-log` | `{ upstreamOnly: boolean, count: number }` | `{ commits: Commit[], unpushed: CommitId[] }` | none yet (Q397) |
| `read/repo.common-folder` | — | `{ folder: string }` | none yet (Q398) |
| `read/commit.prepared-message` | — | `{ message: string \| null }` | none yet (Q398) |

Three reads at the end have no desktop card: the map found them read in `crates/app` or not at all
([03-map.md](03-map.md#3-gaps-extension-side)). Each gets its card in the engine, desktop first (D39), before the
extension uses it.

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
  narrowing: HistoryOf | null,     // the history of a file, of lines, of a folder
  pageSize: number,                // gitardo.graph.pageSize
  pages: number,                   // how many pages from the newest commit
  skip: number,                    // the rows the extension has already: the read starts after them
  stashes: boolean                 // gitardo.graph.showStashes
}
CommitPage = { rows: Row[], hasMore: boolean }
Row = { kind: "commit", commit: Commit }
    | { kind: "stash", commit: Commit, index: number }
    | { kind: "uncommitted", parent: CommitId | null }
Commit = { id, parents: CommitId[], authorName, authorEmail, authorDate: Timestamp, subject }
```

`history_of` and `commits_request` are in the engine (`engine::timeline`, moved from `crates/app` in stage 1), since the
extension must not build git's arguments. `skip` makes the next page carry only the new rows, as the extension's
prefetch does since `ac28e4c` (`gitardo.graph.pageSize`, 500 rows ahead). The lanes stay computed by the webviews
(`src/shared/graph.ts`) from `parents`. A page of the repository of 56,000 commits answers in less than 300 ms
([06-roadmap.md](06-roadmap.md#stage-7--the-graph-reads)).

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
  journal: boolean                 // false only in the tests that check the plan alone
}
result: Finished
```

The answer comes when the feature has finished: refused, backed out of, not confirmed, or run. In between the engine
asks the extension, as requests on the same connection: `feature/ask` for each question, `feature/confirm` for the
plan when it must be confirmed, and the host questions. Only one `feature/run` that reaches a plan runs at a time in a
repository; a second is answered `busy`, and the extension drops it without a word, as `Controller.run` does today
while `this.running` is set (`controller.ts:2421`).

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
the follow-ups that are the host's to carry out, in their order. The engine keeps those that are its own.

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
| `WriteSetting` (new, Q399) | extension | `WorkspaceConfiguration.update`, in the user's or the workspace's settings |

## Undo and the journal

### `journal/last`

`{ repository }` → `{ entry: JournalEntry | null }`: the newest entry that still holds, for the label of **Undo**.

### `journal/record`

From the extension, only while some operations still run in TypeScript
([06-roadmap.md](06-roadmap.md#stage-3--undo-in-the-engine)): `{ repository, title: string, record: UndoRecord | null,
head: CommitId | null }` → `{ number }`. The extension tells the engine what it ran, with what its `GitOp` remembered
(`writes`, the dropped stash, the deleted branch) converted to an `UndoRecord`, or `null` when the reflog is enough.
**Undo** is then always `feature/run` of `undo.last`, and finds the newest operation whichever of the two ran it. The
request goes away with `src/git`.

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
| `host/clipboard` | — | `{ text: string \| null }` | `Asker::clipboard`, `Query::Clipboard` |
| `host/githubToken` | `{ host: string, ask: boolean }` | `{ token: string \| null }` | `Query::GitHubToken`, `github::read_token` and the credential store |
| `host/languageModel` | `LanguageModelRequest` | `{ text: string } \| { refused: string }` | `ai::ask`, `ai::ask_conflict` over HTTP |
| `host/languageModels` | — | `{ models: { id, name }[] }` | `Query::AiModels`, `ai::list_models` |
| `host/openFile` | `FileQuestion` | `{ paths: string[] }` | `QuestionKind::OpenFile` |
| `host/saveFile` | `FileQuestion` | `{ path: string \| null }` | `QuestionKind::SaveFile` |
| `host/secret` | `{ name }` | `{ value: string \| null }` | `system::read_secret` |
| `host/storeSecret` | `{ name, value: string \| null }` | `null` | `system::write_secret`, `delete_secret` |

`LanguageModelRequest = { purpose: "commitMessage" | "conflict", model: string | null, effort: "low" | "medium" |
"high", system: string, messages: { role: "user" | "assistant", text: string }[], stream: boolean }`. The prompt and
the reading of the answer stay in the engine (`crates/engine/src/ai.rs`): the extension only sends the messages to
`vscode.lm` and gives back the text. When `stream` is on, the extension sends `host/languageModelPart` notifications
`{ request: id, text }` as parts arrive, for the commit box to fill as it does now.

The engine today calls the AI provider and GitHub itself (`ai.rs`, `github.rs`, `drive.rs:228-234`). Running under the
extension, the `Engine` gets a host for those, as it gets a `SecretStore` with `keep_secrets_in`: one function that
the server gives, which asks the extension, and that the desktop leaves at the HTTP of today. That is the only change
the protocol asks of `engine`, beside separating `platform` (Q393).

## How the Asker is translated

`Asker` (`crates/engine/src/drive.rs:39`) has three methods. In the server, each `feature/run` runs on a thread of its
own with a `HostAsker`, which holds the id of the request and a channel to the writer of standard output:

| `Asker` | Becomes | In the extension |
| --- | --- | --- |
| `ask(&Question) -> Option<Answer>` | `feature/ask`, waits for its answer | by `QuestionKind`, below |
| `confirm(&Plan) -> bool` | `feature/confirm`, waits | the modal dialog of today, with the plan's commands (`gitardo.commandPreview`) |
| `clipboard() -> Option<String>` | `host/clipboard`, waits | `vscode.env.clipboard.readText()` |

A question has `key`, `kind`, `title`, `detail`, `default`. In VS Code:

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
  commits }`, `{ kind: "submodule", path }`, `{ kind: "worktree", folder }`, `{ kind: "path", path }`, and the kinds
  with a structure of their own: `diffChange`, `comparison`, `mergeResult`, `resolutions`, `fileSave`, `fileSaves`,
  `fileAt`, `setupSettings`, `lines`, `lineOfFile`, `listedFile`, each with the fields of its type in `model`.
- **`Question`** (`model/src/contract/question.rs`): `{ key, kind: QuestionKind, title, detail: string | null,
  default: string | null }`. **`Choice`**: `{ identifier, label, description: string | null }`. **`Validation`**:
  `"anything" | { kind: "referenceName", taken: string[], takenMessage } | "url" | { kind: "newMessage", old } |
  "required" | "optionalReferenceName" | "submoduleFolder" | "worktreeName" | { kind: "pattern", pattern }`.
  **`FileQuestion`**: `{ folder: string | null, fileName: string | null, filters: { name, extensions }[] }`.
- **`Answer`** (`model/src/contract/answers.rs`): only the kinds a person gives travel to the engine: `text`,
  `choice`, `choices`, `confirmed`, `path`. The other kinds are answers of the engine's own queries and never cross,
  except in `answers` of an `OfferedRun`, which the extension sends back as it received them.
- **`Plan`** (`model/src/contract/plan.rs`): `{ title, steps: Step[], danger: "none" | { kind: "confirm", reason } |
  { kind: "destructive", reason }, undo: "byReflog" | { kind: "impossible", reason } | { kind: "journal" } | {
  kind: "takesBack", number }, after: FollowUp[] }`. The `UndoRecord` of `journal` stays in the engine.
- **`Step`**: `{ kind: "git", arguments: string[], directory: string | null, input: string | null }`, `{ kind: "write",
  path }`, `{ kind: "link", from, to }`, `{ kind: "copy", from, to }`, `{ kind: "unlink", path }`, `{ kind: "delete",
  path }`, `{ kind: "run", command, directory }`, `{ kind: "task", command, directory }`, `{ kind: "makeFolder",
  path }`, `{ kind: "deleteEmptyFolder", path }`, `{ kind: "warn", text }`. The extension shows them only, in the
  confirmation and in the log: the line of each is the one the engine logs.
- **`JournalEntry`** (`model/src/contract/journal.rs`): `{ number, title, head: CommitId | null }`; the record stays
  in the engine.
- **`UndoRecord`**, sent only in `journal/record`: `{ kind: "branchDeleted", name, commit }`, `{ kind: "stashDropped",
  commit, message }`, `{ kind: "conflictsResolved", … }`, `{ kind: "squashMerge", … }`, `{ kind: "series", records }`,
  `{ kind: "splitLast", … }`, the fields of each from `journal.rs` at stage 3.

The protocol's own tests round-trip each type through JSON and check that every variant of every enum above has its
JSON form ([07-rules.md](07-rules.md)).

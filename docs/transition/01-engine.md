# One engine for the two apps

Gitardo Desktop and the extension for VS Code run the same features on the same engine, the Rust one. The extension
becomes an interface of VS Code over it: it shows, it asks, it opens editors, and it hands every read and every
operation of git to the engine. This page says why, what the engine is as a program, how the extension starts it and
talks to it, and what stays in TypeScript. The messages themselves are in [02-protocol.md](02-protocol.md); the order of
the work is in [06-roadmap.md](06-roadmap.md). The decisions it rests on are D37 to D49 in
[08-decisions.md](../08-decisions.md#d37--one-engine-the-rust-one-for-both-apps).

## Why

- **One behavior.** The two apps do the same things: the catalog has 415 cards for the desktop, and the
  [map](03-map.md) finds a desktop feature for each of the 123 entry points of the extension that change something.
  Two implementations of the same feature drift apart: [04-differences.md](04-differences.md) lists where they already
  have. With one engine a fix, a new refusal or a new step reaches both apps at once.
- **Written once, tested once.** An operation of the desktop is a pure function, `(Context, Answers) -> Outcome`, with
  a test for each line of its card's acceptance list, run as a table and as a plan against a throwaway repository
  ([desktop/04-feature-contract.md](../desktop/04-feature-contract.md)). The extension's operations are built in
  `src/git/ops.ts` (1,268 lines) and asked for in `src/extension/actions.ts` (1,513 lines), with their questions mixed
  into VS Code's dialogs, which D26 kept so because moving them was a rewrite. The engine is that rewrite, already
  done.
- **Desktop first.** A feature is built in the engine and the window of the desktop, then given an interface in the
  extension (D39). The extension stops being a second place where features are designed.

What it costs: a process beside the extension host, a protocol to keep, a binary for each platform in the package, and
for a while two paths for the operations not moved yet.

## What the engine is

The crates of `rust/` without a window: `model`, `text`, `json`, `features`, `git`, `engine`, `http`, `tls`. Together
about 79,500 lines of Rust with their tests. Two crates are added:

- **`protocol`**: the types of the messages, with `serde` (D42), converted from and to those of `model`. `model` does
  not change for the protocol, and a change to `model` changes the protocol only where `protocol` says so.
- **`server`**: the binary `gitardo-engine`. `gitardo-engine --stdio` reads requests on its standard input and writes
  answers and notifications on its standard output; what it logs for people goes to its standard error.

`gitardo-cli` (`crates/app/src/bin/gitardo-cli`) stays as it is: it is the same engine driven by arguments, and the
integration tests of the features use it.

One crate has to change before `server` can be built lean: `engine` depends on `platform`
(`crates/engine/Cargo.toml`), and `platform` brings the libraries of the window (winit, softbuffer, arboard, rfd,
accesskit). What `engine` uses of it is `link_folder` and the three functions of the credential store. The first
stage of the roadmap separates them, so that `gitardo-engine` links no window library; how is open (Q393).

## The process

### Starting

The extension starts one `gitardo-engine --stdio` for its extension host, that is one per window of VS Code, the first
time a repository is shown (D41). With Remote SSH, WSL or a container the extension host runs on the remote machine,
and so does the engine, from the package installed there.

The binary is found in the package, at `bin/<platform>/gitardo-engine[.exe]`, where `<platform>` is the target of the
vsix (`win32-x64`, `linux-arm64`, …). `gitardo.engine.path`, a setting for development only, names another binary; F5
sets it to `rust/target/debug/gitardo-engine`.

The first message is `initialize`: the version of the protocol the extension speaks, the version of the extension, the
settings, and what the host can answer ([02-protocol.md](02-protocol.md#initialize)). The engine answers with its own
version of the protocol and its version. A major version that differs ends it there: the extension says
**Gitardo's engine is version 2.1 and this extension needs 1.x. Reinstall the extension.** and goes on as if the binary
were missing.

### Several repositories

The process keeps an `Engine` for each repository the extension opened, as `crates/engine/src/drive.rs` already makes
one per `Git`. Every request names its repository by the folder of its root, as `RepositoryState::folder` has it.
`repository/open` makes the `Engine` and reads the state; `repository/close` drops it, with its journal. Reads of
different repositories run at the same time; in one repository one operation runs at a time, as today in the
extension (`Controller.run`) and in the window (`Q110` of the desktop), and reads go on while it runs.

The engine watches no files in the extension: VS Code's file system watchers do it already, and the extension sends
`repository/changed` when they fire. The engine reads again, once per burst, and sends the new state.

### What the engine asks of the extension

The engine drives a feature from its first call to its end (`Engine::run_feature`), asking an `Asker`
(`crates/engine/src/drive.rs`) for each question and for the confirmation of the plan. In the server the `Asker` is a
`HostAsker`: it sends the question to the extension as a request and waits on the feature's own thread for the answer,
which VS Code's input boxes, pickers and dialogs give. A question answered with nothing (Escape) ends the feature
without running anything, as today.

Some things only VS Code knows or does. They are asked of the extension as **host questions**, requests from the engine
to the extension ([02-protocol.md](02-protocol.md#host-questions)): the text on the clipboard, a GitHub token (from
VS Code's sign-in), an answer of a language model (through VS Code's language models), the folder and the files of the
system's dialogs, the secrets VS Code keeps for the extension. The settings are not asked: the extension sends them at
`initialize` and again whenever one changes (D43), so that the `Context` of a feature stays a snapshot.

### Running, the log and Undo

Plans run in the engine: the steps, the log, the journal of Undo, the follow-ups. The extension shows what the engine
sends: each line of the log as it comes, in the **Gitardo** output channel; what is running, in the status bar; the
follow-ups that are about VS Code (select a commit, open a tab, copy, open a URL), carried out by the extension.

While some operations still ran in TypeScript, the engine's journal had to know about them, or Undo would take back the
wrong one: the extension told the engine, with `journal/record`, what it remembered of each operation it ran itself,
and **Undo** always ran in the engine ([06-roadmap.md](06-roadmap.md#stage-3--undo-in-the-engine)). Since stage 10
every operation is the engine's, and `journal/record` is gone.

### Cancelling

A question can always be cancelled, as today. A read the extension no longer wants (a page of the graph that was
scrolled past, a comparison whose tab was closed) is cancelled with `$/cancel`, which cancels its `CancelToken`: the git
process it runs is killed. An operation whose steps have started is not cancelled, in either app.

### When something goes wrong

- **A request that fails** gets an error with a code and a message for the user, in the words of the
  [desktop's rule R4.3](../desktop/03-code-rules.md), and what git printed as its data, for the log.
- **A feature that refuses** is not an error: its answer says so, with the refusal's words and its offer.
- **The binary is missing or does not start** (a platform with no build yet, macOS until D48, a binary blocked by an
  antivirus): until stage 10 every operation ran in TypeScript then, the moved ones too, whose `GitOp` stayed as the
  fallback. Since stage 10, which removed `src/git`, the views say **Gitardo's engine is not available on this
  system.**, with **Show the log**, and so does every command that needs the engine, and nothing else (D49, proposed).
- **The engine exits or crashes**: the extension starts it again once and reopens the repositories; the operation that
  was running is said as failed, with what the engine wrote on its standard error. A third exit within five minutes
  stops the restarts until the window is reloaded.
- **It stops answering**: no time limit is put on operations, which can take long (a clone, a push); a read that takes
  more than 30 seconds shows as running in the status bar like any other.

### The version of the protocol

`major.minor`. A minor version adds messages or fields that the other side may ignore; a major version changes or
removes one. The engine and the extension come from the same commit and are packed together, so they agree but for
`gitardo.engine.path` and a broken install; the check at `initialize` is for those.

## What stays in the extension, what goes to the engine

| Stays in TypeScript | Goes to the engine |
| --- | --- |
| The webviews: every view and tab, React, `src/webview` | Every read of git: `src/git/repo.ts`, `parse.ts`, `discover.ts` |
| VS Code's Source Control integration, the Repositories view, the Explorer menu | Every operation: `src/git/ops.ts`, and the questions of `src/extension/actions.ts` |
| The diff editor, the multi-diff editor, the merge editor, the `gitardo:` documents | The rules of conflicts and the proposals: `src/git/conflicts.ts`, `review.ts` |
| The Flow Diff and the Flow Merge (Monaco, their worker) | What new worktrees get: `src/git/worktreeSetup.ts` |
| The blame decorations, drawn from the engine's blame | The blame itself, line and file |
| AI through VS Code's language models, answered as host questions | The prompts and the parsing of the answers (`ai.ask-conflict`, `commit.draft-message`) |
| GitHub sign-in through VS Code, the token given as a host question | The GitHub query of the pull requests |
| The settings, read with VS Code's configuration and sent to the engine | The log, the journal of Undo, what is running |
| The workspace state and each webview's state ([05-stored-data.md](05-stored-data.md)) | |
| `src/shared` for the webviews: lanes, tree, labels, messages | The same derivations in `model`, for the engine's own use |

The lanes of the graph are drawn by the webviews from the rows the engine sends, as today from the rows `Repo.state`
gives; `src/shared/graph.ts` stays until the rows carry their lanes.

## What the extension's code becomes

- `src/git` is emptied area by area, and removed at the end.
- `Controller.run` and `Controller.action` hand each action to the engine as `feature/run`, with the target the view
  knows. `resolveAction` shrinks to the actions not moved yet.
- A new `src/engine` holds the client: the process, the messages, the host questions, the conversion from the engine's
  state to the `RepoState` the webviews know, until they take the engine's shape.
- `src/shared/protocol.ts` keeps what the webviews and the extension say to each other; it does not hold the engine's
  messages.

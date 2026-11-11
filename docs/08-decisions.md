# Decisions

The decisions that shape Gitardo Desktop, each with why, and the questions still open. A decision is changed by writing
a new one that replaces it, never by editing the old one away.

## D1 — The git executable, not a library

Gitardo Desktop runs the `git` on the `PATH` for every read and every operation, as the extension does.

- The command preview, the log and "do it again in a terminal" need real git commands.
- Hooks, credentials, `fsmonitor`, signing, `includeIf`, worktrees, sparse checkouts and every configuration of the
  user work because git itself runs.
- Writing git's object store, packs, index and merge machinery from scratch would be most of the project, and would
  still differ from git in its corners.

The cost is a process per read; it is paid back by asking git for many things at once, by machine formats, and by
reading only what a view needs.

## D2 — No dependencies

No crate from outside the workspace (see [02-architecture.md](02-architecture.md#what-no-dependencies-means)), with
one exception, TLS ([D13](#d13--one-library-for-tls)). Everything the program does is in this repository, readable, and changeable without waiting for anyone. The price is
that some parts are written here that a library would give: fonts, PNG, deflate, JSON, diffs. Each is written from its
public specification and kept small by doing only what Gitardo needs.

## D3 — Drawing in software

The program draws every frame into a buffer of pixels, and the system only shows it. One drawing code for every system,
deterministic snapshots, and no GPU API to bind. A window of 2560 × 1440 at 60 frames a second is within what a CPU
does when only the regions that changed are drawn. A GPU backend, through the system's APIs, can come later behind the
same `render` interface if measurements ask for it.

## D4 — Our own text rendering

Fonts are read and rasterized by Gitardo, not by the system, so that text looks and measures the same everywhere and the
snapshots are stable. The first release covers Latin, Greek, Cyrillic, CJK characters one by one, and symbols; it does
not shape right-to-left or complex scripts, nor draw color emoji. File names and messages in those scripts are still
shown, character by character.

## D5 — HTTPS through curl

*Replaced by [D13](#d13--one-library-for-tls).*

The standard library has no TLS, and writing TLS and its cryptography here would be a risk no one should take.
`curl` is on every supported system and speaks to the system's certificates. Gitardo runs it as it runs git, with the
request on standard input so that tokens never show in the list of processes. The alternatives were the system's own
HTTP APIs (WinHTTP, NSURLSession, nothing standard on Linux), which would mean three implementations.

## D6 — Windows first

The maintainer works on Windows, and Windows has the most corners (junctions, folders held by other programs, line
endings). Linux and macOS come after the program is complete on Windows, behind the same `platform` interface.

## D7 — One store, reduced on the UI thread

All state of a window is in one store, changed only by `reduce(state, message)`. Views are functions of it. This makes
every change traceable to a message and every view testable by giving it a state.

## D8 — Features as pure functions

Every operation is a pure function that asks questions, asks for reads, refuses, or returns a plan (see
[04-feature-contract.md](04-feature-contract.md)). Questions, previews, the log, Undo and tests are written once in the
engine and work for every feature.

## D9 — The diff and merge are Gitardo's own pages

There is no editor of VS Code to fall back on, so the Flow Diff and the Flow Merge of the extension become the only
diff and merge of the application, and Gitardo has its own text editor for the editable sides, the File page and blame.

## D10 — The AI provider is configured, not discovered

VS Code offered the language models of the user's account. Here the user names a provider, a model and a key
(`ai.*` settings, key in the credential store). Nothing is sent until the user asks, as in the extension.

## D11 — GitHub through git's credentials

The token for GitHub comes from `git credential fill` for `https://github.com`, which works with the credential
manager most users already have, or from a token the user gives, kept in the system's credential store. Gitardo never
opens a sign-in page on its own and never signs in for the user.

## D12 — Numbers that are Gitardo Desktop's own

Where the extension had no number because VS Code decided (the length of the log, the size of the undo journal), the
cards give one and say it is new: the command log keeps 10,000 lines, the journal 50 entries per repository.

## D13 — One library, for TLS

*Replaces [D5](#d5--https-through-curl).*

Gitardo speaks HTTPS itself for two things only: the API of GitHub and the AI provider. Fetching, pulling, pushing and
cloning, over HTTPS or SSH, are done by the `git` process, which has its own TLS and runs the system's `ssh`; they need
nothing of this.

TLS and its cryptography are the one part not written here: a mistake there is a security hole, not a bug, and the
cryptography must be checked by people who do only that. So D2 has one exception:

- **`rustls`**, TLS 1.2 and 1.3 written in Rust, with **`ring`** as its cryptography, and only the features it needs
  (`default-features = false`; `std`, `tls12`, `ring`);
- **`rustls-native-certs`**, which reads the root certificates the system trusts, so that a company's own certificate
  authority is trusted as it is by the browser and by git.

How the exception is kept an exception:

- These crates, and the packages they bring with them, are the only packages of `Cargo.lock` outside the workspace.
  Their names and exact versions are in `xtask/allowed-packages.txt`, and `cargo xtask check` fails on any other
  package, or on another version of these.
- Only `crates/tls` depends on them. It gives the rest of the program an interface of Gitardo's own, a stream that
  reads and writes, opened with a host name and a port, so that replacing the library changes that crate only. No type
  of the library appears outside it.
- HTTP itself is Gitardo's: HTTP/1.1, chunked bodies, server-sent events and proxies, over `std::net::TcpStream` and
  that stream, in `crates/http`.
- Updating them is a commit of its own that says why (a security advisory, a fix that Gitardo needs) and changes the
  allowlist in the same commit.
- Their own build scripts and `unsafe` are theirs: the rules R1.2 and R5 are about the crates of the workspace.

Why not the others:

- **`curl` as a process** (D5): a process per request, a different curl on each system, errors that are harder to tell
  apart while an answer streams, and a token that passes through another program.
- **`native-tls`**: on Linux it needs OpenSSL installed, and it behaves differently on each system.
- **`aws-lc-rs`** in place of `ring`: it needs CMake and NASM to build on Windows.
- **Writing TLS here**: no.

## D14 — The types of the contract are in `model`

*Answers a question of the implementation side, 2026-11-12.*

The types every feature speaks in (`Context`, `Answers`, `Outcome`, `Question`, `Query`, `Refusal`, `Plan`, `Step`,
`Danger`, `Undo`, `FollowUp`, `Target`) are data of the domain, as `RepositoryState` is, and live in `model`. `features`
returns them; `engine` runs the plans, answers the queries and keeps the journal of Undo, so `engine` uses `model`
directly. The engine calls a feature as a plain function, `fn(&Context, &Answers) -> Outcome`, that `app` hands it: it
needs no edge to `features`.

## D15 — `app` may use every crate below it

*Answers Q6, 2026-11-12.*

`app` owns the windows and the event loop, draws the frame, keeps the store and reads the settings, so it uses
`platform`, `ui`, `render`, `model` and `git` as well as what the picture drew. It may use every crate of the workspace
but `testkit` (a dev-dependency only) and `xtask`; nothing uses `app`.

## Open questions

Questions from the implementation side go here, with the card they are about, until the specification side answers
them in the card.

- **Q1** — The name of the application and of its binary: *Gitardo Desktop* and `gitardo` are working names.
- **Q2** — Linux: X11 only for the first Linux release, or Wayland as well?
- **Q3** — Which AI providers must be supported at first: `anthropic` and `openai-compatible`, or others?
- **Q4** — Should the undo journal survive a restart of the application? The extension keeps it in memory only.
- **Q5** — Accessibility: exposing the views to screen readers needs the system's accessibility APIs (UI Automation,
  AT-SPI over D-Bus, NSAccessibility). In which milestone?

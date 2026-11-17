# Decisions

The decisions that shape Gitardo, its engine and its two apps, Gitardo Desktop and the extension for VS Code, each with
why and what was rejected, and the questions still open. A decision is never edited away: when it changes, a new one
replaces it, and the old one says *Replaced by*.

*Source: Michael Nygard, "Documenting Architecture Decisions" (2011).*

## How they are numbered

The two apps had a list each until they came into one repository. Gitardo Desktop's decisions and questions keep their
numbers, since its code, its cards and its commits cite them: D1 to D22 and Q1 to Q374. The extension's come after
them: its D1 to D14 are D23 to D36 here, and its Q1 to Q18 are Q375 to Q392, so that a number found in an old commit of
the extension is its own number minus 22, or minus 374. Decisions taken for the one repository start at D37.

| Decision | Here | | Question | Here |
| --- | --- | --- | --- | --- |
| D1 | D23 | | Q1 | Q375 |
| D2 | D24 | | Q2 | Q376 |
| D3 | D25 | | Q3 | Q377 |
| D4 | D26 | | Q4 | Q378 |
| D5 | D27 | | Q5 | Q379 |
| D6 | D28 | | Q6 | Q380 |
| D7 | D29 | | Q7 | Q381 |
| D8 | D30 | | Q8 | Q382 |
| D9 | D31 | | Q9 | Q383 |
| D10 | D32 | | Q10 | Q384 |
| D11 | D33 | | Q11 | Q385 |
| D12 | D34 | | Q12 | Q386 |
| D13 | D35 | | Q13 | Q387 |
| D14 | D36 | | Q14 | Q388 |
|  |  | | Q15 | Q389 |
|  |  | | Q16 | Q390 |
|  |  | | Q17 | Q391 |
|  |  | | Q18 | Q392 |

## The decisions of Gitardo Desktop, D1 to D22

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

*Replaced by [D20](#d20--libraries-where-they-serve). It had been narrowed by
[D16](#d16--libraries-for-the-window-the-text-and-the-pixels), which let in the libraries of the window, the text and
the pixels.*

No crate from outside the workspace (see [02-architecture.md](desktop/02-architecture.md#dependencies)), with
one exception, TLS ([D13](#d13--one-library-for-tls)). Everything the program does is in this repository, readable, and changeable without waiting for anyone. The price is
that some parts are written here that a library would give: fonts, PNG, deflate, JSON, diffs. Each is written from its
public specification and kept small by doing only what Gitardo needs.

## D3 — Drawing in software

The program draws every frame into a buffer of pixels, and the system only shows it. One drawing code for every system,
deterministic snapshots, and no GPU API to bind. A window of 2560 × 1440 at 60 frames a second is within what a CPU
does when only the regions that changed are drawn. A GPU backend, through the system's APIs, can come later behind the
same `render` interface if measurements ask for it.

## D4 — Our own text rendering

*Replaced by [D16](#d16--libraries-for-the-window-the-text-and-the-pixels) and
[D18](#d18--the-fonts-come-with-the-program).*

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
[04-feature-contract.md](desktop/04-feature-contract.md)). Questions, previews, the log, Undo and tests are written once in the
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

*Replaces [D5](#d5--https-through-curl). How its libraries are kept in their crate is now
[D22](#d22--libraries-with-judgment), and the allowlist it speaks of is gone; why these are the ones for TLS, and why
only `crates/tls` uses them, still stands.*

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

## D16 — Libraries for the window, the text and the pixels

*The maintainer's decision, 2026-11-12. Narrows [D2](#d2--no-dependencies), replaces
[D4](#d4--our-own-text-rendering), answers Q24, Q25, Q26 and Q28. How its libraries are kept in their crates is now
[D22](#d22--libraries-with-judgment), and the allowlist it speaks of is gone; which libraries, and why, still stands.*

The first version of milestone M2 wrote the window of Windows, the reader and rasterizer of TrueType fonts and the
rasterizer of shapes here. They worked, but they are the parts where a library gives the most and Gitardo's own code
the least: text that needs shaping (Arabic, Indic scripts, right-to-left, emoji in color, which commit messages do
have), input methods, screen readers, and a window on Linux and macOS, each of which would be thousands of lines of
calls to the system. The toolkit above them, which gives Gitardo its look and its speed, stays Gitardo's own.

The libraries let in, each used by the crates named and by no other:

| Library | For | Used by |
| --- | --- | --- |
| `winit` | Windows, input, input methods, scale, cursors, the event loop, on Windows, Linux (X11 and Wayland) and macOS | `platform` |
| `softbuffer` | Showing a frame of pixels drawn in memory in a window | `platform` |
| `arboard` | The clipboard | `platform` |
| `rfd` | The system's dialogs to choose a folder or a file | `platform` |
| `accesskit`, `accesskit_winit` | What is on screen, told to the screen reader of the system | `ui` (`accesskit`, the tree of what is drawn), `platform` (`accesskit_winit`) |
| `tiny-skia` | Shapes, paths, anti-aliasing, images | `render` |
| `cosmic-text` | Text: fonts, shaping, right-to-left, fallback fonts, emoji, line breaking, the glyphs | `render` |

How they are kept in their place:

- Their names and exact versions, and the packages they bring, are in `xtask/allowed-packages.txt`, each with the crates
  of the workspace that may depend on it; `cargo xtask check` fails on a package not listed, at another version, or used
  by another crate.
- No type of `winit`, `softbuffer`, `arboard`, `rfd`, `tiny-skia` or `cosmic-text` appears in the interface of
  `platform` or `render`: the rest of the program sees the types of Gitardo, so that replacing a library changes one
  crate. The one exception is the tree of `accesskit`, which is plain data that `ui` builds and `platform` hands over.
- Drawing stays in software ([D3](#d3--drawing-in-software)): `tiny-skia` and `cosmic-text` draw into the frame buffer
  of `render`, and `softbuffer` shows it. A GPU backend is still a later decision.
- `ui` keeps its own types of input, which `app` makes from those of `platform` (Q25), and the toolkit, its widgets,
  its layout and its look are Gitardo's.
- The program still writes the rest itself: git, the engine, the features, diffs, JSON, PNG, HTTP, and the parts of the
  system no library above covers (junctions, killing a tree of processes, the credential store, watching files), behind
  `platform`.
- Updating one is a commit of its own that says why and changes the allowlist.

Why not a whole toolkit:

- **gpui** (the toolkit of the Zed editor): no release since October 2025; using it means depending on the repository
  of Zed.
- **iced**: no screen reader support and no list that draws only the rows in sight; restyling it to a look of
  Gitardo's own costs about what the toolkit already written cost.
- **egui**: the weakest text of all (no right-to-left, no emoji in color, no fonts of the system), and its look fights
  every change.
- **Slint**: a markup language of its own and a licence to weigh, for views that are mostly drawn by hand, like the graph.
- **A web view** (Tauri): not a program drawn in Rust, and several hundred megabytes of memory.

## D17 — The Timeline: a window of Gitardo's own

*The maintainer's decision, 2026-11-12. Replaces the window of the first version of [01-product.md](desktop/01-product.md),
which was shaped like VS Code, and answers Q27. [D21](#d21--islands-a-bar-of-views-and-a-status-bar) adds a status bar
and a bar of views, and makes Quick search a button.*

The window is not a copy of an editor's: there is no side bar of folding panes, no row of tabs, no panel and no status
bar. The history is the window, and everything else opens from it.

- **The timeline** fills the middle: the commits with their lanes, two lines a row (the subject; the author, the date),
  labels as pills in the color of their lane. The uncommitted changes are a **draft** at its top: the message of the
  next commit and a **Commit** button, with **Review** for the files.
- **The branch rail**, on the left, lists the branches in the colors of their lanes, so it is also the legend of the
  graph; then the remote branches, and what is kept aside: stashes, worktrees, tags.
- **The inspector**, on the right, is the selected commit as a card: its message, its author, its files, its actions.
- **The header** holds the repository, the branch and how far it is from its upstream, **Quick search** (`Ctrl+K`,
  `Cmd+K` on macOS: commits, branches, files and commands in one field), what is running, **Sync**, and the
  **Activity** drawer with the command log and the tasks.
- **Pages** (a commit, a comparison, a diff, a merge, the conflicts, a file, the settings) open in place of the timeline
  with a trail to come back; `Ctrl+Tab` goes through the pages open.
- The look is calm and warm: a dark graphite or a light paper ground, one accent, the lanes in four colors told apart by
  lightness as well as hue, rounded rows and cards, Inter for the interface and JetBrains Mono for hashes and code
  ([D18](#d18--the-fonts-come-with-the-program)). The themes are dark, light, and the two of high contrast.

The pictures of the direction chosen are the canvas *Gitardo Desktop directions*, artboards *A · Timeline*.

## D18 — The fonts come with the program

*2026-11-12. Replaces the system fonts of the first version of [05-platform.md](desktop/05-platform.md).*

Inter (for the interface) and JetBrains Mono (for hashes and code), both under the SIL Open Font License, are part of
the program, in `crates/render/fonts/` with their licences. The text looks and measures the same on every system, and
the snapshots of the tests are drawn with the same fonts as the program. A character neither has is looked for in the
fonts of the system, through `cosmic-text`. The settings `font.ui` and `font.code` can name a font of the system
instead.

## D19 — The answers about the Timeline window

*The maintainer's decision, 2026-11-12. Answers Q37, Q38, Q39 and Q40, asked when the catalog moved to the Timeline
([D17](#d17--the-timeline-a-window-of-gitardos-own)).*

- **Sync** (Q37). A click on **Sync** in the header fetches, pulls as the settings of how to pull say, then pushes what
  is ahead (`sync.sync`); fetch, pull and push, with their other ways, stay in its menu, and **Branch**, **Stash**,
  **Undo** and **Refresh** stay entries of the **…** menu of the header. One click does what the counts on it promise,
  as other Git clients do.
- **The commits of what is checked out** (Q38) are the timeline narrowed to the branch checked out
  (`filter.show-only`), read by `graph.read-commits`, with what is not pushed ringed by `graph.unpushed`; the cards
  `commits.view`, `commits.read-log` and `commits.push-state` are gone, with their code. One history, read once, with
  the filter and the search of the timeline, instead of a second list.
- **The branch rail** (Q39). A click on a branch narrows the timeline to its history and selects its tip, and a second
  click brings the whole history back; a tag, a stash or a worktree still goes to its commit. The lists of **Kept
  aside**, **Submodules** among them, start closed and the others open, and each remote has a heading of its own.
  The rail is the legend and the filter of the timeline.
- **The draft** (Q40) shows the message, **Commit**, **Review** and the counts only; **Amend last commit**, the button
  that drafts the message with the AI and the identity are on the Changes page only, which is also where one commits
  while the timeline is narrowed to branches that do not hold the commit checked out. The draft stays a calm card, and
  the rest of the commit box is where there is room for it.

## D20 — Libraries where they serve

*Replaced by [D22](#d22--libraries-with-judgment), which keeps its reasons and drops the allowlist.*

*The maintainer's decision, 2026-11-12. Replaces [D2](#d2--no-dependencies). [D13](#d13--one-library-for-tls) and
[D16](#d16--libraries-for-the-window-the-text-and-the-pixels) stay as the reasons for the libraries they name.*

Writing everything here was meant to keep the program readable and free of other people's schedules. D16 showed the
price: the parts a library does well (text, the window, the screen reader) cost the most to write and gave the least,
and the time went there instead of into the features. From now on a crate of the workspace may depend on a library
from crates.io when the library does a job better than code written here would: highlighting code, watching files,
the credential store, property tests, and the like.

What a library must be:

- under a licence that MIT code may use: MIT, Apache-2.0, BSD, ISC, Zlib, Unicode, or MPL-2.0 used unchanged;
- kept up, with a release in the last year, or small and finished;
- built by Cargo alone. A library that needs a C compiler, CMake or a library installed on the system is a decision of
  its own, as `aws-lc-rs` was in D13;
- used with the fewest features that work, named in the `Cargo.toml` that uses it.

How libraries are kept in their place:

- `xtask/allowed-packages.txt` names each library a manifest names, with the crates of the workspace that may use it,
  and `cargo xtask check` fails on a library not listed there or used by a crate it does not name. The versions are
  those of `Cargo.lock`, and what the libraries bring is not listed. Adding a library is a line in that file, in the
  commit that first uses it, which says why.
- The pure crates (`features`, `model`, `text`, `json`) use only libraries that do no I/O, start no thread and read no
  clock ([R3](desktop/03-code-rules.md)).
- The types of a library stay in the crates the allowlist names for it, so that replacing it changes those crates only.
- The build scripts and procedural macros of libraries are theirs (a `derive` of `serde` is fine). The crates of the
  workspace still have none of their own.
- `git` is still run as a program, not replaced by a library ([D1](#d1--the-git-executable-not-a-library)), and the
  clean room still holds: a library is used through its interface and its documentation, never copied
  ([00-clean-room.md](desktop/00-clean-room.md)).
- What is already written here (PNG, deflate, JSON, diffs, HTTP, the file watcher, the credential store) stays until a
  library is worth the change. Each replacement is a commit of its own, and the tests stay as they are.

## D21 — Islands, a bar of views and a status bar

*The maintainer's decision, 2026-11-14. Changes [D17](#d17--the-timeline-a-window-of-gitardos-own) where it said there
is no status bar and that Quick search is a box of the header; the rest of D17 stays.*

The window was one flat ground with the inspector as its only card, and two boxes of text one above the other (Quick
search in the header, the search of the timeline) that looked alike and did different things. The maintainer chose,
among three drafts, the one with a bar of views.

- **Islands.** The header, the branch rail, the timeline and the inspector, or the page shown, are each a rounded card
  of their own, an **island**, on a ground one step darker in the dark themes and one step deeper in the light ones,
  with a gap between them. The gap is where the panes are dragged to be resized. In the high contrast themes the
  islands are told apart by their border.
- **The bar of views**, a column of icons on the left edge of the window, outside the islands: **Timeline**,
  **Changes** with the number of changed files, **Files** and **Compare**, then, at its foot, **Activity** and
  **Settings**. Each shows its place with one click; the one shown is marked. It does not replace the draft, the trail
  or `Ctrl+Tab`: it is one more way to the same places, always in the same spot.
- **A status bar**, a line of small text at the foot of the window, outside the islands: what is running on the left,
  which opens the Activity drawer, as the indicator of the header did; on the right the identity commits are made
  with, which a click switches, as on the Changes page. Nothing is only there.
- **Quick search is a button**, **Search** with `Ctrl K` beside it, since the box only ever opened the picker. When the
  window is narrow it keeps its icon. The one box of text left at the top is the search of the timeline, with the
  branch filter beside it.

The pictures of the drafts are the page *Gitardo Desktop a riquadri*, variant C.

## D22 — Libraries with judgment

*The maintainer's decision, 2026-11-14. Replaces [D20](#d20--libraries-where-they-serve): the allowlist and its check
are gone. [D13](#d13--one-library-for-tls) and [D16](#d16--libraries-for-the-window-the-text-and-the-pixels) stay as
the reasons for the libraries they chose.*

The allowlist made every library a stop to ask, and the stop gave little once D20 let libraries in: the choice is still
worth thinking about, but whoever writes the code makes it, and says why in the commit. Libraries are not to be
avoided; they are to be chosen. A crate of the workspace may depend on a library from crates.io when it does the job
better than code written here would.

What to weigh before adding one:

- **What it saves against what it brings.** A library for text, a window or TLS saves years; one for twenty lines
  that are easy to write saves nothing, and brings its packages, its build time and its updates.
- **Whether one already used does the job.** Two libraries for the same job is one too many.
- **Whether it is kept up**: a release in the last year, or small and finished, and more than one person behind it
  for the parts that matter.
- **What it needs to build and to run.** A C compiler, CMake or a library installed on the system (WebKitGTK on
  Linux, for a web view) is a cost to say in the README, next to what the program needs.
- **The fewest features that work**, named in the `Cargo.toml` that uses it.
- **Its licence**, the one thing not weighed: MIT code must be able to use it (MIT, Apache-2.0, BSD, ISC, Zlib,
  Unicode, or MPL-2.0 used unchanged). A library under another licence is not used.

What still holds, because it keeps a library easy to replace rather than because it limits which:

- A library is used by the crate whose job it serves, behind that crate's interface: no type of it appears in the
  interface, so that replacing it changes that crate only ([R1.6](desktop/03-code-rules.md)). Where a type has to cross, as
  the tree of `accesskit` does between `ui` and `platform`, the code says why.
- The pure crates (`features`, `model`, `text`, `json`) use only libraries that do no I/O, start no thread and read no
  clock ([R3](desktop/03-code-rules.md)).
- TLS and cryptography only through `crates/tls` ([D13](#d13--one-library-for-tls), [R1.5](desktop/03-code-rules.md)).
- `git` is run as a program ([D1](#d1--the-git-executable-not-a-library)), and the clean room holds: a library is used
  through its interface and its documentation, never copied ([00-clean-room.md](desktop/00-clean-room.md)).
- The crates of the workspace still have no build script and no procedural macro of their own
  ([R1.2](desktop/03-code-rules.md)); those of libraries are theirs. No crate depends on a folder outside the workspace.
- The commit that adds a library says why in its message, and an update that changes what the program does is a
  commit of its own.

`cargo xtask check` no longer reads a list of libraries. It still fails on a build script or a procedural macro of the
workspace, and on a dependency on a folder outside it.

## The decisions of the extension, D23 to D36

The choices Gitardo is built on, each with why and what was rejected. A decision is never edited away: when it changes,
a new one replaces it, and the old one says *Replaced by*. Most of these were made while the code was written and are
reconstructed here from the history and from the maintainer's words; they say so. Where the reason is not known, it is
an open question at the end, not a guess.

## D23 — A VS Code extension, with its views in the panel and the Source Control side bar

*D1 of the extension until the two apps shared one repository.*

Gitardo lives inside VS Code: the graph in the bottom panel next to the terminal, its other views in the Source Control
side bar under VS Code's own, its pages as editor tabs. It has no icon of its own in the activity bar: git already has
its place there, and a second one would split it. The views were first in a container of their own; they moved to the
Source Control side bar in `3d5998b`. A standalone application is a separate project, not a mode of this one.

*Reconstructed from `d790289` and `3d5998b`, and from the maintainer's choice of the three zones of the graph.*

## D24 — Run the git executable, never a library

*D2 of the extension until the two apps shared one repository.*

Gitardo runs `git` from the `PATH` for everything it reads and does, with the arguments as a list and no shell. What it
shows is then what git would do for the user in a terminal, with the user's configuration, hooks, credentials and
version, and the user can run the same commands. A library (isomorphic-git, libgit2 bindings) was not used: it would
behave differently from the user's git in the corners (hooks, filters, configuration) and add a native or large
dependency. The cost: a process per command, and parsing what git prints.

*Reconstructed from `d790289` and the README's Command preview.*

## D25 — React for the views, Monaco only where text is edited

*D3 of the extension until the two apps shared one repository.*

Every view and tab is one React bundle, picked by the `data-view` of the page. Monaco, which is heavy, is loaded only by
the Flow Diff and the Flow Merge, where text is compared and edited side by side; the colors of code elsewhere come
from shiki, loaded on demand. *Reconstructed from the maintainer's choice of React, `137bfd1` and `9e97d6e`.*

## D26 — An operation is a GitOp built before it runs

*D4 of the extension until the two apps shared one repository.*

Everything that changes the repository is first data: a title, the exact commands, whether it can lose work, what
Gitardo writes itself, and what to offer if git refuses. One function, `Controller.run`, asks, runs, logs and reads
again. That gives the command preview, the log and Undo to every operation for nothing, and lets the tests run an
operation's commands on a real repository without VS Code. The questions an operation asks are still asked by its case
of `resolveAction`, with VS Code's dialogs, rather than returned as data: moving them would make every operation
testable without VS Code, but costs a rewrite of `actions.ts` for features that work; it is not planned. See
[the feature contract](extension/04-feature-contract.md#an-operation-as-it-is-built-today).

*Reconstructed from `src/git/ops.ts` and AGENTS.md, "From a click to git".*

## D27 — Show the exact commands, and always ask before losing work

*D5 of the extension until the two apps shared one repository.*

`gitardo.commandPreview` set to `all` shows the commands of every operation before they run; `off`, the default, asks
only before what can lose work or rewrite history, which is asked whatever the setting. The commands shown are the ones
that run, with no flag added after. The alternative, asking before everything always, was rejected as noise; asking
before nothing, as unsafe.

*Reconstructed from the maintainer's choice at the start of the project (then `gitar.commandPreview`).*

## D28 — Undo reads the reflog, and remembers only what it cannot tell

*D6 of the extension until the two apps shared one repository.*

**Undo** takes back the last operation by reading the reflog of `HEAD`, so it also undoes what was done in a terminal,
and undoing twice redoes. What leaves no trace there (a deleted branch, a dropped stash, conflicts resolved, a squash
merge, a series of commits made in one operation) Gitardo remembers itself, in memory, for as long as the top of the
reflog stays what it was. It is not kept across a reload of the window: a stale undo that moves a branch is worse than
none. *Reconstructed from `Controller.action` and the README's Undo.*

## D29 — Resolving conflicts automatically, and with AI, is experimental and off

*D7 of the extension until the two apps shared one repository.*

The rules for the conflicts that need no choice, and the language model, only **propose**; nothing is written until the
user accepts, in the review or in the Flow Merge, and Undo marks the conflicts again. Both are behind settings that are
off by default (`gitardo.conflicts.autoResolve`, `gitardo.conflicts.ai`) and marked experimental in the Settings
editor: a wrong resolution can be committed without being noticed. *Reconstructed from `11c06c4`.*

## D30 — Everything in English, and the former names nowhere

*D8 of the extension until the two apps shared one repository.*

Code, comments, UI text, the README, the CHANGELOG, these documents and commit messages are in English. The extension
had other names before 0.2.0; they are written nowhere but the 0.2.0 entry of the CHANGELOG, which explains the change.
The identifier changed with the name (`gitardo.*`), knowing that it made a new listing on the Marketplace.
*Reconstructed from `eda8186` and the maintainer's instructions.*

## D31 — Settings are `gitardo.*`, and VS Code's Git settings are read when Gitardo's are not set

*D9 of the extension until the two apps shared one repository.*

Every setting is a VS Code setting named `gitardo.<area>.<name>`. Where VS Code's Git extension has a setting for the
same thing, Gitardo reads it while its own is not set (`git.branchSortOrder`, `git.worktreeSymlinkFolders`,
`git.worktreeIncludeFiles`), so that a user of both configures it once. *Reconstructed from the settings in
`package.json`.*

## D32 — The README is for users; `docs/` is for whoever changes the code

*D10 of the extension until the two apps shared one repository.*

The README is the extension's page on the Marketplace and its **Details** tab in VS Code, so it says only what a user
needs. What a developer needs (the scripts, the tests, the layout of the source, the architecture, the rules) is in
`docs/`, and `AGENTS.md` is the way in. The CHANGELOG and the walkthrough stay as they are: they are part of the
product. Before this, the README had a **Development** section and `AGENTS.md` held the architecture and the rules;
both moved into `docs/` when it was written, so that nothing is said twice.

## D33 — Cards first for the areas where a mistake costs

*D11 of the extension until the two apps shared one repository.*

The catalog has cards for the areas whose operations can lose work, rewrite history or leave the repository in a state
that is hard to read: changes and commits, branches and sync, merging and rewriting, stashes, conflicts, worktrees,
submodules, and Undo with the preview and the log. The other areas (the graph, search, the tab of a commit,
comparisons, history and blame, patches, GitHub, the Flow Diff and the Flow Merge, repositories) are described for
users in the README and get their cards the first time one of their features is changed, in that commit.

Why: the README already describes every feature at the level of a card, so cards add most where they add the function,
the tests, the danger and the undo of an operation; writing all of them at once would cost much and age before it was
read. What it costs: a change to a feature of an area without cards first writes the cards of that area, or at least of
the features it touches.

To change this choice, replace this decision: to write the whole catalog at once, list the areas in
[04-feature-contract.md](extension/04-feature-contract.md#the-areas) as *Written* as their files come, and run
`npm run check:docs`; to write fewer, mark areas *Later* and delete their files.

## D34 — No linter, no formatter

*D12 of the extension until the two apps shared one repository.*

The project has `tsc --noEmit` in strict mode and no ESLint or Prettier: the code is laid out as Prettier would lay it
out with a width of 160, by hand, matching the file. A linter would enforce rules that review catches today (R5, R6)
but adds configuration and dependencies to keep. *Reconstructed from AGENTS.md; the reason for leaving Prettier out is
Q377.*

## D35 — Tests are named by what the user sees

*D13 of the extension until the two apps shared one repository.*

Tests are named with a sentence of what the user sees (`it('discards a file taken out of the index and left on disk back
to HEAD, rather than deleting it')`), not with the identifier of a card. The card's **Tested by** line names the test by
its file and title, so that the link goes from the documents to the tests. The cost: a test renamed or removed leaves a
**Tested by** line that no tool finds (see [RS6](extension/13-risks.md)). The alternative, starting each test's name with its
card's identifier, would rename hundreds of tests for a link the card already gives.

## D36 — The maintainer publishes by hand

*D14 of the extension until the two apps shared one repository. Replaced by
[D52](#d52--a-tag-publishes-the-release-once-the-maintainer-approves), which keeps the maintainer's hand on every
release and lets the pipeline do the upload.*

No agent pushes, tags or publishes. A release is a commit with the version and the CHANGELOG, a tag, and a package made
with `npm run package` that the maintainer uploads to the Marketplace. *Reconstructed from AGENTS.md.*

## The decisions of the one repository and the one engine, from D37

*The maintainer's decisions, 2026-11-14, taken when the two apps came into one repository to share one engine. The
documents of the transition are in [transition/](transition/README.md). D49 is proposed, not taken.*

## D37 — One engine, the Rust one, for both apps

Gitardo Desktop and the extension run the same features on one engine: the Rust crates of `rust/` without a window.
The extension becomes an interface of VS Code over it: it shows, asks and opens editors, and hands every read and every
operation of git to the engine ([transition/01-engine.md](transition/01-engine.md)). The TypeScript engine
(`extension/src/git`) is emptied area by area and removed ([transition/06-roadmap.md](transition/06-roadmap.md)). Two
engines doing the same features already drift apart ([transition/04-differences.md](transition/04-differences.md)
counts 54 differences), and the Rust one is the one written from cards with a test per acceptance line. Rejected:
keeping two engines and a shared catalog (the drift stays); moving the desktop onto the TypeScript engine (a runtime in
the desktop, and the features rewritten as pure functions anyway). Once an area has moved, [D26](#d26--an-operation-is-a-gitop-built-before-it-runs)
and [D28](#d28--undo-reads-the-reflog-and-remembers-only-what-it-cannot-tell) no longer hold for it: an operation is a
feature of the engine, and Undo is the engine's journal.

## D38 — One repository: `rust/`, `extension/`, and the documents at the root

The branch `main-merge`, from `main`, holds both apps: the Rust workspace in `rust/`, the extension in `extension/`,
and at the root only what both share or what must be there: `AGENTS.md` and `CLAUDE.md`, `README.md`, `docs/` with
one catalog, `SECURITY.md`, `LICENSE`, `.gitignore`, `.gitattributes`, `.github/`, `.vscode/`. The history of
`main-desktop` came in whole through a branch that moved its files into `rust/` and a merge with
`--allow-unrelated-histories`, so `git log --follow` reaches the first commit of each file. Until stage 1 of the
roadmap, commits on `main-desktop` come in with a plain `git merge main-desktop`, which follows the moves by itself
(tried once: `-Xsubtree=rust` loses the move of the documents and must not be used); after it, the work was done only
here. Once the transition was done, `main-merge` came into `main` by a fast-forward and was deleted, with
`main-desktop` (2026-11-16): the work is done on `main`. Rejected: two repositories with the engine as a package (the desktop
first rule needs one commit for both); `engine/` and `vscode/` as names (the folder holds the whole desktop, not only
the engine).

## D39 — Desktop first, and a status for each app

A feature is built first in the engine and the window of Gitardo Desktop, with its card, its tests and
`rust/CHANGELOG.md`; then in the extension, with its interface, its tests and `extension/CHANGELOG.md`. A card has a
status for each app; it may be Built in the desktop and Planned in the extension, never Built in the extension while
not in the desktop, unless a decision says why for that card. A feature that only VS Code can host (the Agents window,
the Repositories view of Source Control) has an extension card marked Host only, and needs no desktop card. Rejected:
the extension first, since it has the users (the engine would then be written after the fact, as the TypeScript one
was); both at once in every commit (each change twice as large, and the window's tests and VS Code's run on different
machines).

## D40 — The clean room ends

*Replaces the rule of [desktop/00-clean-room.md](desktop/00-clean-room.md), which stays as the record of how the
desktop was written.*

The desktop was written from the documents, never from the extension's code. With one engine for both, the extension's
code is read to move its behavior, and its differences are found by reading both
([transition/04-differences.md](transition/04-differences.md)). What stays: no code of another project is copied, the
libraries are used through their interface and their documentation ([D22](#d22--libraries-with-judgment)), and a
behavior moves into the engine through its card and its tests, not by translating TypeScript line by line.

## D41 — JSON-RPC over the standard input and output, one process per window

The extension starts `gitardo-engine --stdio`, one process for its extension host (one per window of VS Code, and on
the remote machine with Remote SSH, WSL or a container), with one `Engine` per repository opened in it. The messages
are JSON-RPC 2.0, framed as the Language Server Protocol frames them; both sides send requests, since the engine asks
the questions of a feature ([transition/02-protocol.md](transition/02-protocol.md)). Rejected: a library loaded into
Node (N-API: a crash of the engine takes the extension host down, and one binary per Node ABI); WebAssembly (no
processes, so no `git`); a process per repository (twice the memory for a workspace of many repositories, and the
journal of a worktree apart from its repository's); a socket (a port to protect, and nothing gained over pipes).

## D42 — The messages' types in a crate `protocol`, with `serde`

The types of the messages are in `rust/crates/protocol`, with `serde` and `serde_json`, converted from and to those of
`model`, and versioned `major.minor`. `model` stays without `serde`: a change to the contract changes the protocol only
where `protocol` says so, and the window does not compile JSON code it does not use. Rejected: `serde` on the types of
`model` (every rename in `model` would change the protocol silently); a JSON written by hand with `crates/json` (every
type twice, by hand, for some 150 types).

## D43 — The extension sends the settings; the host answers what only it has

The extension sends the settings at `initialize` and again whenever one changes, resolved (the workspace's over the
user's, defaults, the `git.*` fallbacks of [D31](#d31--settings-are-gitardo-and-vs-codes-git-settings-are-read-when-gitardos-are-not-set)),
under the names of the desktop's `settings.json`, which are the extension's without `gitardo.`. The engine reads no
settings file while it runs under the extension, and a feature that writes a setting asks the extension to write it.
The engine asks the extension, as host questions, what only VS Code has: the clipboard, a GitHub token from VS Code's
sign-in, an answer of a language model, the dialogs of files, the secrets. Under the extension,
[D10](#d10--the-ai-provider-is-configured-not-discovered) and [D11](#d11--github-through-gits-credentials) give way to
VS Code's language models and sign-in. Rejected: the engine asking for each setting when it needs it (a round trip in
the middle of every feature, and a `Context` that changes while a feature runs); the engine reading VS Code's
`settings.json` (it cannot see the user's profile, the remote, or the defaults).

## D44 — One catalog, the desktop's, with a status for each app

The catalog is `docs/features/`, with a column for each app in its index. The extension's 184 cards stay in
`docs/features/extension/` as the source of each area until the stage of the roadmap that moves the area merges them
into the desktop's cards; their identifiers become aliases, listed at the end of the index. Rejected: the extension's
catalog as the base (181 cards, written after the code, against 415 written before it); merging every card now (the
merge is the work of each stage, with the code of the area in front of it).

## D45 — Where the two apps differ, the engine wins, but for what is decided otherwise

Each difference of [transition/04-differences.md](transition/04-differences.md) is a bug, a choice, or a line of the
CHANGELOG. A bug is fixed on whichever side it is, in the engine when both have it. A choice is decided case by case,
with an option of the engine where the host must differ. Otherwise the engine's behavior is the one both apps get, and
each difference the extension's users will see is a line of its CHANGELOG in the release that moves the area.

## D46 — `cargo xtask check` checks the catalog of both apps

The checks of the catalog are xtask's: it reads `docs/` at the root, the cards of both apps and their statuses, and
fails when an extension status is ahead of the desktop's ([D39](#d39--desktop-first-and-a-status-for-each-app)).
`npm run check:docs` calls it. `docs/catalog.py` checks the extension's cards until the parts of it that xtask lacks
(the lines a card's kind needs, the words of the glossary) are ported, then it is removed. Rejected: two checkers for
one catalog.

## D47 — A README and a CHANGELOG for each app, and a short one at the root

`extension/README.md` is the page of the Marketplace and stays as it is, links and images included, with `vsce
package --baseContentUrl` and `--baseImagesUrl` pointing into `extension/`; `extension/CHANGELOG.md` is the
extension's, `rust/CHANGELOG.md` the desktop's. The root `README.md` is short: the two apps and where each lives.
`LICENSE` stays at the root and `npm run package` copies it into `extension/` for `vsce`, which takes it only from
there.

## D48 — macOS on GitHub Actions

The engine and the desktop are built and checked on macOS (`macos-14`, both targets) by GitHub Actions, which closes M11
of the desktop's roadmap, needing no Mac at hand. Until stage 10 the extension on a platform with no binary used its
TypeScript code for everything; since then it says that the engine is not there
([D49](#d49--when-the-engines-binary-is-missing-proposed)).

## D49 — When the engine's binary is missing (proposed)

*Proposed by the agent that wrote the transition's documents; the maintainer had no preference.*

While `extension/src/git` is there, every operation runs in TypeScript when the binary is missing or does not start,
those already moved included: their `GitOp` is kept as the fallback until `src/git` is removed. The output channel
says once **Gitardo's engine is not available on this system, so Gitardo uses its older code.** That is what keeps
macOS working until [D48](#d48--macos-on-github-actions) gives it a binary. `src/git` is removed only once every target
has its binary (stage 10 of the roadmap, after the pipeline of stage 9); from then on, every view says **Gitardo's
engine is not available on this system.** with **Show the log**, and nothing else. The extension never downloads a
binary. Rejected: an operation already moved refusing at once when the binary is missing (it would take features away
from macOS for the whole transition).

Since stage 10 `extension/src/git` is gone, and the fallback with it: when the binary is missing or does not start,
every view says **Gitardo's engine is not available on this system.** with a **Show the log** button, and every
command that needs the engine shows the same sentence with **Show the log**; the extension reads and runs nothing.
There is no universal vsix any more: one vsix per target, each with its engine, and a release waits for all of them.

## D50 — Repository tabs, and what each repository keeps

*Asked by the maintainer, who works on eight repositories of one workspace at once; the details proposed by the agent
and approved.*

A window of Gitardo Desktop keeps several repositories open, each in a tab of an island of its own above the header,
and shows one at a time ([the cards of the tabs](features/01-repositories.md#repository-tabs)). A tab is opened for
each repository the user opens: a workspace with several asks which of them to open, all checked, so that the eight
can be opened at once or only some. A worktree and a submodule are in the tab of their repository, chosen with the
**▾** of the tab, not in tabs of their own. Everything below the island is the tab's and is kept while another tab is
shown, behind the setting `repositories.keepOpen`, on by default; with it off, the tabs stay and a repository left is
forgotten, as choosing a repository did before. The repository menu of the header is gone. The tabs not shown stay
watched, and are read again once they are shown; a window still runs one operation at a time, and tabs can be switched
while it runs. The island fits a narrow window by narrowing its tabs, then listing those that do not fit in **… N**,
then becoming one button. A `.code-workspace` file of VS Code can be opened, for its folders.

The extension keeps its list of repositories and gets no tabs: it gets the same keeping, behind
`gitardo.repositories.keepOpen`, off by default there, and its list groups the worktrees and the submodules under
their repository. Desktop first ([D39](#d39--desktop-first-and-a-status-for-each-app)), in milestone M12 of
[the roadmap](desktop/07-roadmap.md#m12--repository-tabs-done).

Rejected: an island for each repository (it does not fit a narrow window); a tab for each repository found, submodules
included (too many, and not the ones the user chose); tabs for worktrees and submodules (they are the same project,
and would crowd the island); tabs in the extension (VS Code has the list and the Repositories view for that); keeping
the repository menu of the header beside the tabs (two ways to do one thing); operations in several tabs at once (one
at a time keeps the log, the undo and the questions in one order; see Q409).

## D51 — The branch rail has a filter of its own

*Asked by the maintainer: a click on a branch of the rail hid every other branch of it; the details proposed by the
agent.*

The branch rail lists what its own filter says, never what the timeline is narrowed to. A click on a branch narrows
the timeline to its history and selects its tip, as before, and the rail keeps listing every branch it listed, the ones
the timeline leaves out dimmed, each with the eye that adds its history to the timeline. The rail's filter is a button
of its own at the top of the rail, with the picker of the timeline's branch filter (the presets, the branches ticked one
by one, the kinds of references) over the rail alone: the two filters never change each other, and each is remembered
per repository apart from the other. `branches.followGraphFilter`, and `gitardo.branches.followGraphFilter` in the
extension, are no longer read, and **Filter the list of branches too** leaves the timeline's branch filter. The line at
the top of the rail says what the rail's own filter lists. Desktop first, then the extension, in milestone M13 of
[the roadmap](desktop/07-roadmap.md#m13--the-branch-rails-own-filter-done).

Rejected: keeping `branches.followGraphFilter` with `false` as its default (two ways to do one thing, and a click still
hides branches for whoever had it on); one filter shared by the rail and the timeline (what the maintainer asked
against); a rail that follows the timeline until its own filter is used (the rail would still change under a click).

## D52 — A tag publishes the release, once the maintainer approves

*The maintainer's decision, 2026-11-16: publishing by hand meant seven vsix to upload one by one, and the tokens of
Azure DevOps that `vsce publish` used stop working on 2026-12-01. Replaces
[D36](#d36--the-maintainer-publishes-by-hand).*

No agent pushes, tags or publishes, as before. The two apps have one version, from 1.0.0, so that one tag releases
both: `extension/package.json` and the workspace's `rust/Cargo.toml` always name the same. A release is a commit on
`main` with that version in both and its section in both CHANGELOGs, and a tag `vX.Y.Z` of that version, which the
maintainer pushes. The tag runs `.github/workflows/release.yml`: it checks that the tag, the two versions and the two
CHANGELOGs agree and that the commit is on `main`, runs the whole of `check.yml` on it, and once that is green:

- the seven vsix wait for the maintainer's approval in the GitHub environment `marketplace`, then are published on the
  Marketplace by its trusted publishing (`vsce publish --oidc`), with no token stored anywhere: the Marketplace's
  policy names the repository, the workflow and the environment;
- a draft release on GitHub, on the tag, gets the seven vsix, the desktop of Windows x64, Linux x64 and macOS on Apple
  silicon in an archive each, not signed, and the section of the version of each CHANGELOG as notes; the maintainer
  reads it and publishes it.

The workflow can be run again from the Actions tab on the same tag: what is already on the Marketplace is skipped, and
the files of the draft are replaced. A push of a branch runs `check.yml` alone and publishes nothing, as before. A
version may hold changes of one app only; its section in the other CHANGELOG then says so.

Rejected: publishing on every push of `main` (a release is the maintainer's choice, not every commit's); a run started
by hand only (a tag is made for every release anyway, and the approval already asks); a token of Azure DevOps in a
secret (they stop working on 2026-12-01, and a stored token can leak); an application of Microsoft Entra ID with a
federated credential (it works, but needs Azure for what trusted publishing does alone); a release on GitHub published
at once (the desktop is not signed, and the maintainer reads the notes first); a version of its own for the desktop
(two numbers to keep apart for builds made from one commit, and tags that would have to say which app).

## D53 — Pages beside the inspector, a bottom panel, the icons of Lucide, islands in the extension

*Asked by the maintainer for milestone M14; the details proposed by the agent, the four choices below answered by the
maintainer.*

Five changes to how the window looks and keeps its parts, and one to the extension
([the roadmap](desktop/07-roadmap.md#m14--the-window-keeps-its-sides)):

- **The selected commit dims whole rows.** While a history is brought out, by a label under the pointer
  (`graph.hover-history`) or by the commit selected (`graph.trace-selected`), the rows it leaves out are dimmed whole,
  their message, labels, author, date and hash, and not only their lines and dots, as the extension does. The
  extension does it already; the window catches up.
- **A file opens beside the inspector.** A file of the commit or of the comparison in the inspector opens its Diff page
  in place of the timeline only: the inspector stays on the right, marks the file shown, and another of its files
  replaces the diff in the same page, so that every file of a commit can be read one after the other without the
  inspector closing (`diff.beside-inspector`). The same for the Files page: a file opens on the right of its tree, which
  stays (`file-view.beside-files`). The other ways to a Diff or File page still open it in place of the timeline and
  the inspector, as before.
- **A bottom panel.** An island under the timeline and the inspector, or the page shown, above the status bar, with the
  **Terminal** and the **Command log** in tabs (`panel.bottom`). The terminal is a real shell, in the folder of the
  repository shown, interactive, with the colors and the keys of a terminal, several at once (`terminal.view`). The
  Command log leaves the Activity drawer for the panel: whatever opened it there opens it here. The drawer keeps what is
  running and the Tasks. The terminal is built on a library of terminal emulation and pseudo-terminals, weighed as D22
  says; `alacritty_terminal` (Apache-2.0, ConPTY on Windows, a pty on Linux and macOS, its grid and scrollback) is the
  first candidate ([Q417](#of-milestone-m14-from-q417)).
- **The icons of Lucide.** Every icon of the window is one of [Lucide](https://lucide.dev)'s (ISC licence), drawn from
  its outlines, in place of the ones drawn here; each meaning has one icon and each icon one meaning, as the table of
  `graph.icons` says, so that the same action looks the same everywhere and as it does in other applications. The
  outlines are kept in the repository with the licence, and the icons are drawn as before, in the color and the size
  asked.
- **Islands in the extension.** The setting `gitardo.graph.islands`, off by default, draws the references, the commits
  and the details of the Graph view each as a rounded card on a ground one step darker, with the gap between them as
  the line that is dragged, as the window does since D21 (`graph.islands`). The other views keep their look.

The maintainer chose: in the panel the Command log only, beside the terminal (the drawer keeps the rest); a real shell
rather than a line that runs one command; Lucide among Lucide, the codicons of VS Code and Material Symbols; the islands
in the Graph view of the extension only.

Rejected: the whole Activity drawer in the panel (the maintainer's choice); a line that runs one command and shows its
output (no program that asks, no colors, no prompt of its own); the codicons (the CC-BY licence asks for attribution in
the application, and the window would look like VS Code again, which D16 left); a new Diff page for each file clicked
in the inspector (they would pile up in `Ctrl+Tab`); islands in every webview of the extension (the side bar views are
too narrow for the gaps).

## D54 — The timeline is read by one git log, as it prints

*Asked by the maintainer for milestone M15, after the reads of the graph were timed; the details proposed by the agent.*

Every page of the timeline was a `git log` of its own, from the newest commit, with `--skip` for the commits loaded:
git walks the history from the top for each page, so loading all of it costs one walk per page. Timed on 2026-11-17
on a clone of GIMP (73,581 commits, 468 references, Windows, git alone, no engine): its 74 pages took 61.1 s, 37.7 s
with a commit-graph file; one `git log` of the whole history took 1.4 s, with or without one, and its first 1,000
commits came in 0.97 s, 0.45 s with one, as soon as the first page does today. The layout of the timeline is made
again over every row at each page, which grows the same way.

So the history shown is read by one `git log -z --date-order`, without `--max-count` or `--skip`, read as git prints
it (`graph.read-commits`):

- **The rows come in pages as they are printed.** The first `graph.pageSize` commits go to the timeline as soon as
  they are read, as the first page does today; the next ones follow, a page at a time, while git goes on, without
  waiting for the list to be scrolled. The setting keeps its name and meaning: how many commits the timeline gets at a
  time.
- **The read goes to the end of the history, up to 100,000 commits** ([Q435](#of-milestone-m15-from-q435)). There it
  stops, and its git with it; the next commits are read as they are today, by a read from the last commit loaded with
  `--skip`, once the last row drawn is 500 rows from the end (`graph.load-more`), and that read goes on the same way.
  A history of less than 100,000 commits is walked once.
- **The layout goes on from the last row** (`graph.lanes`): the lanes as they stand under the last row laid out are
  kept, and the rows of a new page are laid out from them, which gives the cells a layout of every row gives.
- **Another history starts again.** Another filter, branches, search or narrowing, a reference moved by a fetch, a
  commit or any operation: the read running is cancelled, its git killed, and a new one starts. The rows listed stay,
  dimmed, until the first page of the new read, as they do today; a read that finds the same commits as those listed
  changes none of the rows.
- **The extension gets the pages as they are read**, in version 1.8 of [the protocol](transition/02-protocol.md):
  `read/graph.read-commits` sends each page as a notification while it reads and answers when its read ends
  ([Q436](#of-milestone-m15-from-q435)); the webview gets only the new rows, as it does since `ac28e4c`, and lays them
  out from its last row (`src/shared/graph.ts`).

This answers [Q56](#open-questions): the pages after the first no longer need `--skip` below 100,000 commits. Desktop
first, then the extension, in milestone M15 of [the roadmap](desktop/07-roadmap.md#m15--the-whole-history-in-one-read).

Rejected: reading the whole history before showing any of it (the first rows would wait for the last, and on a
history of a million commits for tens of seconds and hundreds of megabytes); keeping one `git log` open and stopped by
the pipe while the list is not scrolled (a process held open for each tab, while a read past 100,000 commits that
starts again with `--skip` costs one walk more, and is rare); writing a commit-graph file without asking (it is the
user's repository; it may be proposed apart); a cache of the rows on disk (another stored file, for a read that now
takes a second or two).

## D55 — The settings page in sections, in the middle of the window

*Asked by the maintainer for milestone M16: the settings page is one long list against the left edge, and should be in
sections, in the middle, with the sections beside it as buttons; the details proposed by the agent and accepted by the
maintainer as proposed.*

The settings page of Gitardo Desktop (`settings.edit`) changes in five ways
([the roadmap](desktop/07-roadmap.md#m16--the-settings-in-sections-and-a-side-column)):

- **In the middle, in sections.** The page is centered in the place of the timeline and the inspector, at most 960
  logical pixels wide. On its left a column of buttons, one for each section, with its icon and its name; on its right
  the settings of every section, one after the other under the name of each, in one list that scrolls. A click on a
  section scrolls the list to it; the button of the section at the top of the list is marked as the list scrolls. The
  sections are **General**, **Appearance**, **Timeline**, **Branches**, **Pull and commits**, **Worktrees**, **Files and
  blame**, **AI and conflicts** and **Terminal and tools**, which hold the settings as the table of
  [18-settings.md](features/18-settings.md#the-list) says (`settings.sections`). Under them, apart, **Changed** lists
  only the settings a file sets, and **Experimental** only the experimental ones. Beside each section, how many of its
  settings a file sets, and a mark when one of them has a value that was not taken. Narrower than 640 logical pixels,
  the column becomes a menu at the top of the page that names the section shown.
- **A title for each setting.** Each setting has a title in sentence case, the first line of its row (**Commits read at
  a time**), with its description under it, as long as it needs to be, and its key in small under that, with the file
  that sets it. A setting a file sets has a bar on its left and a button that takes it back to its default
  (`settings.reset`).
- **Every value can be changed on the page.** A number in a field that refuses a value out of its range; a string in a
  field that says the `${…}` it can hold; a list as rows to add, change and take out, with `identities` as a table of
  name, email and signing key; a color as a swatch and its hexadecimal value; a font from the fonts that come with the
  program and those of the system. A field is written to the file when it loses the focus or `Enter` is pressed,
  never at each key.
- **User and Repository as two tabs.** The choice of the file written moves from the row of radio buttons to two tabs at
  the top of the page, **User** and **Repository**. **Repository** lists only the settings marked **Repository**, and is
  not there while no repository is shown. In **User**, a setting the repository's file also sets says so, `Set by the
  repository: 500`, since that value is the one in force.
- **A search, and a way in from Quick search.** A field at the top of the page, with the focus when the page opens,
  finds settings by title, key, description and value, and the sections with nothing found are dimmed
  (`settings.search`). **Open Settings: <section>** and **Open Settings: <title of a setting>** in Quick search open the
  page scrolled to that section or that setting (`settings.reveal`).

In the extension the settings are VS Code's, in its Settings editor: `contributes.configuration` becomes a list of
categories, the same sections with the same names and order, which the Settings editor shows in the column on its left.
Nothing else changes there.

Rejected: one section shown at a time, the others hidden (a search would have to say in which section each result is,
and settings near each other could not be read one after the other); a dialog or a window of its own for the settings
(the pages are where the window shows a whole thing, and `Ctrl+Tab` reaches them); the key as the title of a row, as
before (it reads as code, and the title says what the setting does); writing a field at each key (a file read again at
each key, and the window drawn again with half a value); a separate section for the experimental settings (each stays
with the feature it changes; **Experimental** lists them all).

## D56 — A side column of repositories and branches, what was said in the status bar, diffs beside what opened them

*Asked by the maintainer for milestone M16, with the settings page of D55; the details proposed by the agent, the
choices the maintainer left to it said below.*

Seven changes to the window of Gitardo Desktop
([the roadmap](desktop/07-roadmap.md#m16--the-settings-in-sections-and-a-side-column)):

- **All commits on one line.** A switch of the **…** menu, **Show all commits on one line** (`graph.oneLane`, off by
  default), draws every dot of the timeline on one line, in the order listed, with the line straight from one to the
  next: no lanes, no lines between lanes, one color. A merge is a ring rather than a dot, so that it is still told
  apart. The labels, the folds of the rows and the selection are as before; **Fold all merges** is not offered while it
  is on (`graph.one-lane`).
- **A side column, the whole height under the header.** The island of tabs above the header goes. On the left of the
  window, from under the header to the status bar, one island holds two sections, one above the other: **Repositories**
  and **Branches**, which is the branch rail as it was. Each section folds and unfolds by its heading, and the line
  between them is dragged to share the height; both are remembered for the workspace (`repo.side-column`).
  **Repositories** has a row for each repository open, in place of its tab: its name, and under it, small, the branch
  checked out in what it shows, how many files are changed there and how many commits are to pull and to push; its **▾**
  lists its worktrees, each with the same three, and its submodules, and shows the one chosen (`repo.tabs`,
  `repo.group-tabs`, `repo.worktree-summary`). The side column stays beside the timeline and beside every page, since
  the repositories must be reachable from all of them, as the tabs were; the Diff pages of a commit and of a comparison
  put it away, below. The button that puts it away and brings it back is the first of the top row of the timeline,
  beside the branch filter; in the narrow layout it is put away, and that button opens it over the timeline.
- **The header, a bar across the top of the window.** The header is no longer an island: it is a bar on the ground
  of the window, across its whole width on the right of the bar of views, above the side column, the timeline and the
  pages, always shown whatever is under it, so that one always sees where one is. It says, from its left, the
  repository shown, the worktree or the submodule shown when it is not the repository's own folder, and the branch
  checked out, as `gitardo › login › feature/login-page`; the repository opens the list of the repositories open, and
  the worktree the **▾** of its row. Then **Search**, **Sync** and **…**. The branch is named without how far it is from
  its upstream: **Sync** says it, `↓3 ↑2`, and only **Sync** (`graph.toolbar`). The button that put the branch rail
  away leaves it for the top row of the timeline (`graph.toggle-references`).
- **What was said, in the status bar.** What an operation said when it ended, with **Undo** and what it offers next,
  and the notices, leave their bars under the header for the status bar, which keeps its height: after what is
  running, the icon of its tone, its words cut with `…` when they are long and whole in its tooltip, its actions as
  links in the text of the status bar, and **×**. What succeeded leaves after ten seconds unless the pointer is on it;
  a warning or an error stays until **×** or the next operation (`activity.said`).
- **A diff beside its commit.** A file of the inspector opens its Diff page as M14 made it, beside the inspector, and
  now the side column is put away while it is shown, and the inspector becomes the column of the commit's files: the
  commit in one line at its top, and its files as the tree of the Files page has them, the file shown marked, its
  folders kept open or closed (`diff.beside-inspector`). The timeline brings everything back as it was.
- **A comparison beside its diffs.** A file of the comparison page opens its diff on the left, and the comparison stays
  on the right as a column: its two sides with the arrows that swap them, how far apart they are in one line, its
  commits folded, and its files as a tree, the file shown marked (`compare.beside-diff`). On the right, where the
  inspector is, so that the files of a commit and of a comparison are always in the same place.
- **The comparison chooser.** The two questions one after the other of `compare.choose` become one dialog: **From**
  and **To** side by side with the arrows that swap them, the list of what the side being chosen can be, in groups,
  with a bar to scroll it, filtered by what is typed, a revision typed taken as it is when git knows it, the comparisons
  opened before at its foot, what `compare.summary` says once both are chosen, and **Compare** (`compare.chooser`).
- **Menus and pickers longer than the window.** A menu under a button opens below it and never moves above it; when it
  is taller than the room under it, it ends at the foot of the window and scrolls, with the wheel, a bar, and the arrows
  keeping the item lit in view (`ui.menu-scroll`). The pickers get the same bar and keep the row lit in view
  (`ui.picker-scroll`).

The maintainer left two choices to the agent: a long menu scrolls rather than splitting in two columns, and the
comparison stays on the right of its diffs. The chooser is the agent's proposal, asked for since the picker of the
sides did not scroll as one expects.

In the extension, the Graph view gets the switch of one line (`gitardo.graph.oneLane`); the rest is VS Code's there
(its Source Control repositories, its notifications, its diff editor and Quick Pick), and nothing changes.

Rejected: a menu in two columns when it is too long (the arrows would have to cross from one column to the other, a
submenu would open over the second column, and a menu twice as wide covers the timeline); the comparison on the left of
its diffs (the files of a commit and those of a comparison would be on opposite sides); keeping the island of tabs above
the side column (two lists of the repositories); how far the branch is from its upstream both beside its name and on
**Sync** (the maintainer's choice: once, on **Sync**); a status bar taller when something was said (the maintainer's
choice); what was said as a card over the timeline (it covers the rows being read, and is one more thing to put away);
the header as an island over the timeline and the inspector only (the maintainer's choice: a bar over everything,
always shown, so that the repository, the worktree and the branch are always seen); the two questions of
`compare.choose` kept with a better picker (the first side is not seen while the second is chosen,
and changing one side asks both again).

## D57 — Worktrees in the side column, pages that keep what opened them, the way back in the title

*Asked by the maintainer for milestone M16 after trying the window its first twelve phases built; the details
proposed by the agent, as the maintainer left them to it.*

Five changes to the window of Gitardo Desktop
([the roadmap](desktop/07-roadmap.md#m16--the-settings-in-sections-and-a-side-column)):

- **Worktrees in the side column, not in a menu.** The worktrees and the submodules of a repository are rows under its
  row in **Repositories**, folded out by a chevron at its left and indented, each with its branch, its files changed and
  `↓ ↑`; the one shown is marked, and a click shows it. The **▾** menu of D56 goes, since one had to open it to see
  what each worktree is on (`repo.group-tabs`, `repo.worktree-summary`).
- **A side column that takes less room.** Its rows on one line where they fit, smaller gaps, the heading of
  **Branches** and its buttons on one line, so that more repositories and branches are seen at once
  (`repo.side-column`).
- **The side column put away from every page.** The button that puts it away is in the title row of every page, at the
  same place as in the top row of the timeline, since the pages need the room as much as the timeline
  (`repo.side-column`).
- **The Changes page beside what it opens.** A file of the Changes page opens its diff or the file on the right of the
  list of changes, which stays as a column, as the Files page keeps its tree since M14, so that a review goes from file
  to file without coming back to the list (`changes.beside-file`).
- **The way back, in the title of a page.** The title row of a page names the pages it was reached through, each a way
  back to that page as it was left, rather than always `Timeline › <page>`. Its limits: the way is the chain of pages
  each opened from the one before; a page opened from the bar of views, Quick search, the side column or a key starts
  the way again from the timeline; going back to a name drops what came after it; a page already on the way is gone
  back to rather than added again, so the way has no loops; at most four names are shown, those in between folded into
  **…**; `Alt+Left` goes back one (`ui.page-trail`).

In the extension nothing changes: VS Code has its own Source Control repositories, its editors beside the Changes view
and its own way back.

## Open questions

### Of Gitardo Desktop, Q1 to Q374

Questions from the implementation side go here, with the card they are about, until the specification side answers
them in the card.

- **Q1** — The name of the application and of its binary: *Gitardo Desktop* and `gitardo` are working names.
- **Q2** — Linux: X11 only for the first Linux release, or Wayland as well?
- **Q3** — Which AI providers must be supported at first: `anthropic` and `openai-compatible`, or others?
- **Q4** — Should the undo journal survive a restart of the application? The extension keeps it in memory only.
- **Q5** — Accessibility: exposing the views to screen readers needs the system's accessibility APIs (UI Automation,
  AT-SPI over D-Bus, NSAccessibility). In which milestone?

From milestone M1 (each was built with the reading given, until the card says otherwise):

- **Q7** — `reflog.read`: the card reads the time of an entry with `%ct`, the date of the commit, not when `HEAD`
  moved there. Built as the card says; `reflog.browse` probably wants the date of the move (`%gd` with
  `--date=unix`).
- **Q8** — `rebase.read-stop`: git 2.53 removes `rebase-merge/amend` when the rebase continues, though the card says
  git leaves it. Both are read; nothing checks that `amend` is the hash of the last `edit`.
- **Q9** — `refs.sort`: the card says that with the setting not set branches sort by name; `18-settings.md` says by
  date when git's `branch.sort` is `-committerdate`. Only the two orders are built, not how the setting is resolved.
- **Q10** — `graph.date-separators` returns the words of a separator ("Yesterday", "October 2026") from `model`, as
  its signature asks, while rule R10.2 puts the words of the UI in `views/text`. Dates use one UTC offset for all,
  so a change to daylight saving between two dates is not seen.
- **Q11** — `graph.fold-rows` names a fold after the branch that labels the second parent, which needs the labels its
  signature does not take: `Fold::branch(labels)` and `Fold::title(labels)` were added. When two folded merges
  compete for the same commits, the one higher in the list wins.
- **Q12** — `graph.labels`: the marks of a bisect and the badges of pull requests are not in the state of the
  repository, so the labels leave them out for now; among the branches of one commit, the one checked out comes
  first.
- **Q13** — `graph.unpushed` narrowed to Local: "no ring" holds only when the upstream is not an ancestor of the
  branch; when the upstream is merely behind, its commit is among the rows and the rings show.
- **Q14** — Return types that differ from the cards' signatures: `details.read-commit` gives `Option<CommitDetail>`
  (none for a hash that is not a commit); `commit-page.diff-lines` and `commit-page.read-patch` give
  `PatchLines { Files, Unmatched }`, which is how "the lines cannot be shown" is said; `compare.read-patch` gives
  `ComparePatch { Lines, Text }`; `reflog.read` gives `model::reflog::ReflogPosition`, since `ReflogEntry` is the
  contract's type for Undo. The cards should say these.
- **Q15** — `compare.read-sides`: `git merge-base` and `git merge-tree` take no pathspec, so with the comparison
  narrowed to a path the merge base and the predicted conflicts are those of the whole merge.
- **Q16** — `compare.read-sides`: the uncommitted changes taken as a commit are the staged, the not staged, the
  untracked (as added) and the conflicted (as modified) files, one entry per path; a staged file keeps its staged
  kind unless it is now deleted on disk.
- **Q17** — `patch.read-format-patch`: the commits picked in any order are put in order with
  `git merge-base --octopus` and `git rev-list --topo-order`, since commit dates are not reliable after a rebase.
- **Q18** — Every diff whose lines are shown, copied or applied is read with `--src-prefix=a/ --dst-prefix=b/`, so that
  `diff.noprefix` or `diff.mnemonicPrefix` in the user's configuration does not break it. Should the cards say so?
- **Q19** — `repo.read-repositories`: acceptance line 3 has the home folder itself hold `projects-x`, but the rules
  put the projects first only when the workspace folder is inside a repository further up. Built as the rules say:
  with the home folder itself as the workspace folder, the home repository comes first.
- **Q20** — `changes.read-working-tree` returns the state's `WorkingTree`, with untracked files in their own list
  rather than as not staged with status `?`; it reads `git status` in its first format, while `repo.read-state`
  reads the second, as each card says. Should both read the same format?
- **Q21** — Words of the UI outside `views/text` (rule R10.2): `SubmoduleStatus::sentence` in `model` (without the
  backticks around the short hash) and `describe_predicted_conflicts` in `git::read::merge`, as their cards return
  text. Move them to `views/text` once it exists?
- **Q22** — `worktree-setup.read-what-patterns-take` asks git with `ls-files --others --ignored --exclude=<pattern>
  --directory` in the checkout, and `check-ignore --no-index -z --stdin` for "git ignores the folder", so no
  temporary file is written; clashes are judged on the files of `HEAD`.
- **Q23** — Card **Function** lines that named a module without its crate (`read::…`, `worktree::…`, `submodule::…`)
  now start with `git::read::`, since `cargo xtask check` takes the first segment as the crate.
- **Q29** — UI text of the toolkit's gallery is in `crates/ui/src/gallery`, not in `views/src/text` (R10.2), since it
  is a demonstration of the widgets and not a view. Move it once `views` exists, or leave it?
- **Q30** — A focused widget that scrolls out of sight is no longer drawn, so it loses the focus. Should the focus stay
  on it?
- **Q31** — An input box keeps its own text while it is edited and takes the view's text only when the view gives one
  that differs from what the box last gave (so a view may normalize it). Is that the rule, or should the view always
  own the text?
- **Q32** — Dialogs put the main button last, on the right, as macOS and GNOME do; Windows puts the default first.
  Which order? And should Enter in a danger confirmation run the action, or should Cancel be the default there?
- **Q33** — Fuzzy ranking for the quick pick is in `ui::fuzzy`, since `ui` may not use `text`, where doc 02 puts
  matching. Should `ui` be allowed to use `text`?
- **Q34** — A modal dialog does not stop the view under it from reading its own shortcut keys; the demo checks that
  no overlay is open. Should the toolkit hold the keys back while a modal is open?
- **Q35** — Drawing a whole frame at scale 2 takes 14.6 ms for the 1920×1080 gallery, over the 8 ms of R10.5 (6 ms at
  scale 1, 0.3 ms when nothing changed). Most frames redraw a few tiles; is the rule about the whole frame?
- **Q36** — Other libraries that may make sense, not let in: `syntect` or `tree-sitter` for the colors of code in the
  diffs, `notify` for watching files, `keyring` for the credential store. Each would be a decision of its own.

From the move of the catalog to the Timeline ([D17](#d17--the-timeline-a-window-of-gitardos-own)), each written in the
card with the reading given, until the card says otherwise:

- **Q41** — `activity.status-bar`: the branch of the header says how far it is from its upstream, `main ↑2 ↓0`, as the
  status bar did; the picture in [01-product.md](desktop/01-product.md) also says how far it is from the default branch
  (`2 ahead · 12 behind main`). Which words, and which card reads the second count? The activity indicator drops the
  `Gitardo —` before the title of what runs.
- **Q42** — `graph.view`, `graph.columns`, `graph.resize-panes`: every row now has two lines (labels and message, then
  author and date) with the hash at its end, in the wide layout too; in the narrow layout the branch rail keeps its
  icons and the inspector opens over the commits, half the height to start with and resized as the details were under
  the commits. Is that the narrow layout meant, and should the inspector over the commits close by itself when the
  selection goes?

From the first slice of milestone M3, the window of `gitardo` (the store, the places empty but real, opening
repositories, the settings), each built with the reading given until it is answered:

- **Q43** — `repo.open-folder`, `repo.open-recent`, `repo.select`: their cards name `features::repo::…(context,
  answers) -> Outcome`, but the `Context` of the feature contract holds one repository, not the workspace; there is no
  kind of question for the system's folder dialog or for **Replace the workspace** / **Add to the workspace**; and an
  outcome cannot say "show this workspace". They are reduced in the store of `app` for now (`app::store`), with the
  card's acceptance lines as tests. Should the contract gain a workspace in `Context`, a folder question and a
  follow-up that changes the workspace, or are these cards of the store, not features?
- **Q44** — `settings.theme` is an operation in `features`, which may not use `crates/json` to write the settings
  file. For now the theme and the fonts are only read (`app::settings`) and applied when the files change; nothing
  writes them. Which crate writes the settings file? (Settled since: `features` writes the settings files with `json`, as
  `worktree-setup.save` does, and `settings.theme` is built so; Q292.)
- **Q45** — What is kept between runs is in `state.json` in Gitardo's configuration folder, beside `settings.json`
  (`%APPDATA%\Gitardo Desktop`, `~/Library/Application Support/Gitardo Desktop`, `$XDG_CONFIG_HOME/gitardo-desktop`
  or `~/.config/gitardo-desktop`); [11-data.md](extension/11-data.md) does not name the file. On Linux, should it be under
  `$XDG_STATE_HOME` instead, since it is state and not configuration?
- **Q46** — The settings files are flat JSON with dotted keys (`"font.size": 14`), as VS Code's are; an object
  `{"font": {"size": 14}}` is not read. Should nested objects be read too?
- **Q47** — `views` may not use `render`, yet the toolkit's interface takes and gives its types (`Rect`, `Color`,
  `Theme`, `Icon`…). `ui` now re-exports them so that the views can name them. Is that the reading of the picture of
  doc 02, or should `views` use `render` directly?
- **Q48** — The pictures of the places with a real repository are tests of `app` (`crates/app/tests/places`), since
  `views` may not use `git` or `testkit` beyond its dev-dependencies and its tests need a `RepositoryState` read from
  a repository. Is that where they belong?
- **Q49** — Words of the window that no card gives, chosen for now: the repository pill with no workspace says **No
  folder open**; its tooltip is "Choose the repository shown, or open a folder"; the Welcome page heads its list
  **Recent**; a notice is shown as a bar under the header with a **Dismiss** button; the question of a folder opened
  is titled "Open {folder}"; the Activity drawer says what asked for a read: "Opening the workspace", "Showing the
  repository". Are these the words?
- **Q50** — The pictures of the steps of the Welcome page are, for now, the icon of the place each is about. Which
  pictures?
- **Q51** — With no workspace the window shows the Welcome page without its trail, since there is no timeline to go
  back to; the first start on a folder shows the Welcome page over the workspace, with the trail. Is that the first
  start meant?
- **Q52** — `readOnStartup` is read but not honoured: the repository is always read when the window opens. And a
  repository chosen is read with the `graph.pageSize` of the repository shown before it, until its own settings file
  is read a moment later. Should the read wait for the repository's settings?
- **Q53** — `repo.no-repository`, acceptance line 3: deleting the folder of the repository shown brings the window to
  the page at the next read (**Refresh** in the **…** menu), since nothing watches the repository yet
  (`repo.watch-changes`, `repo.schedule-reads`). The test reads again by hand. Is that enough for the card, until
  the watcher exists?
- **Q54** — `settings.read`, acceptance line 2: the problems of the settings (which key, which file, why) are kept for
  the settings page (`settings.edit`), which does not exist yet; nothing shows them. The card stays **Specified**
  until that page says them.
- **Q55** — `graph.resize-panes`: the card starts the branch rail at 210 px and the inspector at 320 px, while the
  theme's tokens for them say 248 and 344. The panes follow the card; the tokens are unused by the timeline. Should
  the tokens change to the card's sizes, or the card to the tokens?
- **Q56** — `graph.load-more`: the next pages are read with `--skip=<commits loaded - 1>` added to the read of
  `graph.read-commits`, so that git prints only the new page and the last commit loaded again; when that commit is
  not where it was, every page is read again and replaces the rows. The card of the read does not name `--skip`.
  Is that the reading? Without a commit-graph file, `--date-order` walks the whole history before the first commit
  of any page (about 170 ms for 56,000 commits here), which is most of the time of a page. *Answered by
  [D54](#d54--the-timeline-is-read-by-one-git-log-as-it-prints) in milestone M15: one read lists the history as git
  prints it, and `--skip` is left for the commits past 100,000.*
- **Q57** — `graph.uncommitted-row`: the draft says **Describe the next commit** in place of a message, and its
  **Review** and **Commit** buttons, and every entry of its menu and of a stash row's menu, are shown but cannot be
  clicked, with the tooltip "Not yet", until the operations they run exist (M4 and later). Is showing them disabled
  the reading, rather than leaving them out?
- **Q58** — `graph.columns` and `graph.date-separators`: dates are said and the days counted in UTC, since the
  program does not read the local time zone yet; the tooltip of a date says `UTC`. Should the local time zone be read
  (through which library), and the separators counted in it?
- **Q59** — `graph.first-parent`, `graph.fold-all`, `graph.trace-selected`: these are entries of **…**
  (`graph.more-menu`), so their functions are views that turn the entry chosen into the intent; the menu itself is
  drawn by `graph.more-menu`. Is that the shape meant for a card whose only control is a menu entry?
- **Q60** — `graph.fold-merge`, line 4: the folds are kept for each repository with the hashes sorted, at most 200 of
  them, not the 200 used last. Should the last ones be kept instead?
- **Q61** — `graph.scroll-marks`: the marks are not drawn while every row is in sight, where the mark of a row would
  stand beside another row (row `i` of `n` at `(i + 0.5) / n` of the height). Line 2 is checked on the places of the
  marks, since no search fills them yet (`search.`). Is hiding them while nothing scrolls the reading?
- **Q62** — `graph.resize-graph`, line 3, and `graph.uncommitted-row`, line 3: the timeline cannot be narrowed yet
  (`filter.`). The width of the graph is kept for the workspace whatever the timeline shows, and the draft is left out
  whenever the commit checked out is not among the rows read; the tests check those rules. Are the cards **Built** on
  that, or should they wait for the filter? *Since the branch filter is built, both lines are checked with the
  timeline really narrowed (to `feature`, and to **Current**).*
- **Q63** — `graph.fold-rows`: `model::graph::foldable_merges` was added beside `fold_rows`, to tell which merges get
  an arrow and an entry (`graph.fold-merge`, line 5) without folding them. Is a second function of the derivation
  right, or should the card name it?
- **Q64** — The sizes of the panes, the width of the graph and whether the branch rail is put away are kept for each
  workspace under the key `panes` of the state file, and the folds for each repository under `folds`. `11-data.md`
  does not exist in this branch, so these keys are named nowhere else. Where should they be written down?
- **Q65** — `graph.view`: the inspector shows only the hash, the subject, the author and the date of the row selected,
  and an empty card while nothing is; what it says while nothing is selected is not in a card. Which words?
  *Answered in slice 5 of M3 (`m3-inspector`)*: while nothing is selected the inspector says **Select a commit to see
  its message and the files it changed.**, and **Loading…** while the details of the row selected are read; with
  them it is the card of `details.show`.
- **Q66** — `refs.filter`, line 2, and `filter.picker`, line 4: the names are ranked by `filter.rank-names` within
  each list of the rail (**Local**, one list per remote) and within each group of the branch filter (**Local**,
  **Remotes**, **Tags**), as the text of both cards says. On screen, `maintenance` (under **Local**) then comes before
  `origin/main` (under its remote), while the two lines want `main`, `origin/main`, `maintenance`. The order of the
  ranking itself is tested as the lines say. Both cards stay *Specified* until this is answered: should a filter typed
  give one list of every match, the best first, or are the lines about the order within a list?
- **Q67** — `filter.picker`: the card puts the button "at the top of the timeline before the search box"; in the
  window of D17 the search box is Quick search in the header, so the button is in the header, after the branch
  checked out and before Quick search. Is that the place?
- **Q68** — `refs.tree`: the entries of the menus of the rail whose operations come in later milestones are shown but
  cannot be chosen, as Q57 does for the draft: on a branch **Check out**, **Rename…**, **Delete** (on the one checked
  out, **Rename…** alone); on a remote branch **Check out as local branch**, **Delete**; on a tag **Delete**; on a
  stash **Apply**, **Pop**, **Create branch from stash…**, **Drop**; **Create worktree…** on the Worktrees list and
  **Add submodule…** on the Submodules list. The entries of the branch filter, **Show in the timeline**, the
  favorites and the sort work. Is showing them disabled the reading?
- **Q69** — The state file has two new keys: `favorites`, for each repository folder the full names of its favorites
  in the order they were added, and `branchesShown`, for each repository folder what its timeline shows (a preset by
  its name, the kinds shown, or the full names ticked, as `filter.remember` stores them); each keeps the 20 repositories
  used last. Nothing is kept for a repository whose timeline shows everything. `branches.sortOrder` and
  `branches.followGraphFilter` are written in the user's settings file. Where should these keys be written down (Q64)?
- **Q70** — `refs.tree`: the lists opened or closed are kept by the window, the same for every repository, and not
  between runs, as the card says ("until the application is closed"). Is a list opened in one repository meant to be
  open in another, as line 1 reads?
- **Q71** — `refs.tree`: a list of local branches or of a remote that has no entry left once the branches the timeline
  leaves out are taken away (the rail following the timeline) is not listed, so that a narrowed rail has no empty
  heading such as **ORIGIN 0**; the lists of **Kept aside** are always listed. While something is typed, the count of
  an empty list says `0` rather than `0 of 0`. Is that the reading?
- **Q72** — `refs.tree`: the Submodules list is never shown yet, since the submodules are not read for the rail until
  `submodule.` (M8). A worktree's branch, its folder and how far it is from its upstream are shown; the pull request
  of a branch (`graph.pull-request-badge`), the double click on an entry (`branch.open`) and dragging a branch onto the
  timeline (`drag.drop`) are not built, since those cards come later.
- **Q73** — `refs.favorite`: **Add to favorites** is in the menus of the branch rail only; the labels of the timeline
  have no menu yet, so the entry point on them, and **Add `origin/feature` to favorites** on a label shared with its
  upstream, wait for the menu of a label. Which card brings that menu?
- **Q74** — `filter.kinds`: **Chosen in the filter**, on the three entries greyed out while some references are
  ticked, is drawn where a menu entry shows its shortcut, since menu entries have no tooltip. Is that enough, or should
  menu entries get tooltips?
- **Q75** — The functions of two cards differ from their cards: `filter.label` takes `Option<&BranchesShown>` (none
  is everything, as in every other function of the filter), and `filter.log-arguments` takes the commit checked out as
  `Option<&CommitId>`, the type of a hash in `model`. Should the cards be changed?
- **Q76** — Narrowing the timeline reads the first page of the history of the branches shown, and only that: the state
  of the repository stays as it was last read, and the rows read are dimmed meanwhile. A narrowing while the
  repository is being read starts that read again with the branches shown. With some references ticked the stashes
  are not listed (the read of `graph.read-commits` lists them only when nothing is ticked); with a preset they are. Is
  that the reading of `graph.stashes` for a narrowed timeline?
- **Q77** — `graph.labels`: a commit with more labels than its row has room for drew the last ones squeezed past the
  end of the row, over the hash. A label with less room than its own height is now left out. Should a wide row count
  the labels left out, as the narrow layout does (`+3`)?
- **Q78** — `refs.scope-line`: the line has the filter icon, what the timeline is narrowed to, the eye button and
  **Show all**; in a rail of 210 px the icon is left out when the words would not fit beside it. Is the eye the
  button the card means ("its button lists all the references instead"), with the tooltip **List every branch, the
  others dimmed** and **List only the branches the timeline shows**?
- **Q79** — `search.box`: the box is a row of its own at the top of the timeline, as its card says, and the box in the
  middle of the header is Quick search (`Ctrl+K`), which opens the picker of the toolkit with what was typed in it.
  The picker gained `query`, what its box holds when it opens. The button of the branch filter stays in the header
  (Q67), though the card of `filter.picker` puts it before this box. Is the header box Quick search's alone, as D17
  draws it, and should the filter's button move beside the search box?
- **Q80** — Quick search: no card describes it, so it lists the commands whose cards name it among their entry points,
  in the words of those cards, then the branches, remote branches and tags, then at most 2,000 commits loaded. The
  commands whose features are not built yet are listed with **Not yet** and do nothing; files are left out until the
  File page (M7). Should Quick search have a card, and which entries, in which order?
- **Q81** — `search.list-mode`: whether the funnel is pressed is kept under the key `searchAsList` of the state file,
  written only while it is pressed. Like Q64 and Q69, where should the key be written down?
- **Q82** — `search.box`: the tooltip of each mode is on the list button, for the mode chosen; the entries of a menu
  of the toolkit have no tooltips (as in Q74). Should menu entries get them, or is the tooltip of the button enough?
- **Q83** — `search.go-to-hash` and `graph.show-in-graph`: a commit that the timeline does not list takes away, one
  at a time, what narrows it: a search shown as a list or the history of another commit alone, then the branches shown
  (`filter.show-all`); a commit that is still not listed, being on no branch or tag, is shown with its history alone,
  `--date-order <hash>` in place of the branches shown, under the bar `Only the history of <short hash> · on no branch
  or tag`, until **Show all commits**. The history alone is not kept between runs. The bar of a search shown as a
  list, or of a commit alone, is the bar of `filter.bar`, drawn by `views::search::list_mode_view` (the function of
  `search.list-mode`) with the words of `filter.bar`; while one of them is there, the bar of the branches shown is not
  drawn, and the branches are said after `· in`. Is that the order meant, and is a commit made since then on no branch
  counted as one "lost to a reset"?
- **Q84** — `search.go-to-commit`, line 3: the history of a file is narrowed by `history.` (M7), so the card stays
  **Specified** (built since, with the history of a file: Q285). The dialog of the window checks each value with `search.read-revision` as it is typed and goes to the
  commit itself, as the operation `features::search::go_to_commit` does through the contract: the contract gained
  `Query::Revision`, `Answer::Commit` and `FollowUp::SelectCommit` for it, and `Validation::Anything` stands for the
  check made while typing. Should the window run the operation through the engine once operations run there (M4)?
- **Q85** — `search.go-to-hash`: "no loaded commit matches it" is read as no loaded commit's hash starting with the
  text; a loaded commit whose message holds the text does not hide `Enter ↵`. Is that the reading?
- **Q86** — `search.history`, line 3: what a search of the whole history found is asked again whenever a read of the
  repository, or the narrowing of the timeline to other branches, changes the rows; loading more pages does not ask
  again, since the search already covers the whole history shown. Is that enough, or should a change of
  `graph.firstParent` ask again too?
- **Q87** — `search.list-mode`: the funnel is drawn with the filter icon of the branch filter (`Icon::Filter`, three
  lines that get shorter), rather than an icon of its own, so that the window has one icon for "only some commits".
  Is one icon the reading?
- **Q88** — `app` now uses `text`: the store shapes the message of a commit once, when its details are read, with
  `text::details::message_title` (which unwraps it, `details.message-unwrap`), rather than the views shaping it at
  every frame. The edge is in the picture of `02-architecture.md` (D15), but it is new. Is the store the place, or
  should `views` call `text` while it draws?
- **Q89** — What opens a later page is shown and cannot be used yet, with the tooltip **Not yet: the Diff page and
  the history of a file come later**: a click on a file of the inspector or of the page of a commit, **Open all
  changes in one Diff page**, **Show the history of this file**, **Compare with…** in the context menu of a file and
  of the header of a diff (`details.compare-file-with`), **Open in the Diff page**, **Open it in the Diff page** and
  the button at the end of the header of a diff. Is that the reading, as for Q57?
- **Q90** — `commit-page.open` is `features::commit_page::open(context, answers) -> Outcome`, as its card says, and
  is in `features::OPERATIONS`: on a commit it opens its page at once; with no commit, as from Quick search, it asks
  **Open commit**, reads the commit with `Query::Revision` and refuses a name git does not know. Its plan has no step
  and says what follows with `FollowUp::OpenCommitPage`, which the contract gains for it. The window does not run it
  through the engine yet: like `search.go-to-commit`, the store of `app` (`app::store`) opens the page itself, and
  **Open Commit as a Page…** of Quick search opens the box of **Go to commit** titled **Open commit**, with the prompt
  and the refusal of the card and **Open as a page** for its button. What is typed is read by `search.read-revision`,
  as **Go to commit** does, rather than `compare.resolve-revision`, which comes with the comparison page. Is one box for
  both right, and should the store run both through the engine once it shows questions?
- **Q91** — The lines of a diff are in the colors of their kind (added, removed, context) and in the font of code,
  but not in the colors of the code: no tokenizer is used yet. Which library, or a highlighter of
  Gitardo's own, and from which milestone?
- **Q92** — The icons of the inspector and of the page come from the icons there are: the eye of a file
  (**Show its changes in this page**) is an arrow down, the list layout a minus, the tree a folder, and **Open as a
  page, with its changes** the dot of a commit. Should the set of icons gain an eye and a list?
- **Q93** — How the files are laid out is kept between runs in `state.json` under `fileLayouts`, as
  `{"inspector": "list" | "tree", "page": "list" | "tree"}`, written only once one differs from a first start (the list
  in the inspector, the tree in the pages). Like Q64, the key is named nowhere else. Is one layout for every page
  right, as `commit-page.view` says, rather than one for each repository?
- **Q94** — A stash is a commit with two or three parents: the commit it was made on, its index and its untracked
  files. The inspector and the page show only the first as its **Parent**, and do not say **against the first parent**
  after its count, since the stash is not a merge to the user. Is that the reading of `details.stash`?
- **Q95** — The draft selected shows the files of the uncommitted changes, read as a commit, under the title
  **Uncommitted changes**, with no author, hash or parents, and without the boxes to tick and the buttons of
  `stash.push-file` and `changes.discard`, which come with them in M4. Its button to open a page is not there. Is that
  enough for now?
- **Q96** — What the cards left *Specified* in this slice wait on: `details.show` lines 6 and 7 (the Diff page; that
  the button is not there with one file is tested), `details.stash` line 3 (the Diff page), `details.compare-file-with`
  (the Diff page and `compare.side-choices`), `commit-page.view` lines 8 and 9 (their links are drawn and cannot be
  used until the Diff page; every line has its test), and `graph.view` line 4, whose double click on a commit opens its page but on the label of a branch must check it out
  (`branch.checkout`, M4). Should `commit-page.view` be *Built* on its tests, with the links disabled?
- **Q97** — `commit-page.open` is offered on a stash, so a double click and `Enter` on the row of a stash in the
  timeline now open its page too, where `graph.view` says "a commit". Is that the reading?
- **Q98** — **Show in the timeline** goes to the commit as `graph.show-in-graph` does, through the same code as a
  step of a search and **Go to commit**: the history is loaded up to it, what narrows the timeline is taken away when
  it leaves the commit out, and a commit no branch or tag reaches (a stash dropped since the page opened, a commit lost
  to a reset) is shown with its history alone, under the bar with **Show all commits**. Is that right for a dropped
  stash too, or should the page say that it is no longer in the repository?

From slice 6 of milestone M3, what runs, the Command log, the reads of changes and the settings page, each built with
the reading given until it is answered:

- **Q99** — `activity.describe-task`: the words of a task are made in `model` (`model::activity`), as the card's
  function says, like the words of Q10. Its tests are in `views`, since a pure crate may not read the time, even in
  its tests. Is `model` the place of these words?
- **Q100** — `activity.what-is-running`: the drawer is a card over the bottom of what is under the header, as the
  gallery draws it. **What is running** comes first, and **Command log** beside it once **Show the commands Gitardo
  runs** is clicked; **Hide the commands** puts the log away; `Escape` closes the drawer. The **Tasks** column of the
  gallery (the tasks of a new worktree) is left out until `worktree.create`. Under what runs, the drawer lists what
  ran in the last 30 seconds, at most five, with a check and `Took 2 s`, which no card says. The words chosen are "No
  command has run yet", "12 older lines were dropped", "Took 2 s" and "Hide the commands". Are these the drawer and the
  words?
- **Q101** — `log.record`: no plan runs in the window yet. At `debug` its Command log has one line for each read of
  the repository, `read <folder>: the state and 2 pages of commits (12 ms)`, not one for each git command the read
  runs, and what git said when a read failed. Should each command of a read be logged? Line 4 (no token or key in the
  log of a request to GitHub or to the AI provider) waits for those requests (M9), so the card stays **Specified**.
- **Q102** — `log.show`: the time of each line is said in UTC, as the dates of the timeline are (Q58).
- **Q103** — `repo.watch-changes`: only Windows is watched (`ReadDirectoryChangesW`, in `platform`); on Linux and
  macOS `Watcher::new` says the system is not supported, and only **Refresh** reads again. A change the system reports
  for a folder, the time it was written, is left out: the files created or deleted in it are reported on their own.
  Without that rule, `refs/heads` and `refs/remotes` were reported a moment after a read, which asked for another
  read. Is leaving the folders out right, and which library or calls watch on Linux and macOS?
- **Q104** — `repo.schedule-reads`: **Refresh** while a read runs is noted and followed by one more read, as the card
  says. With no read running, **Refresh** still looks for the repositories of the workspace again before reading, as
  in slice 1. A change of a setting under `conflicts`, `pull`, `worktrees`, `branches` or `graph` asks for a read at
  once, not after 400 ms, since the watcher of the settings files says no moment. The next pages of the timeline
  (`graph.load-more`), the first page of the branches shown (Q76), a search of the whole history and the reads of the
  inspector and of the page of a commit are read outside the schedule too. A narrowing while a read of the repository
  runs starts that read again with the branches shown, and the schedule counts it as the read running, with the cause
  **Filter**. The schedule says when the bar along the top of the commits stops, but the timeline keeps its own bar,
  which the narrowing and the next pages use too. Are these readings right?
- **Q105** — `repo.read-state`, `readOnStartup`: when the window opens, the setting is taken from the user's file only,
  because the repository's file is not known until the repositories are found, and finding them runs git. With it off,
  the window names the workspace by its first folder, and **Read the repository**, **Refresh** or **Check again** look
  for the repositories and read the one shown. Should a repository remembered for the workspace have its own file read
  first?
- **Q106** — `settings.edit`: the page changes switches and choices, and shows numbers, strings and lists without
  changing them, until an input box or the File page (M6) does. While **Repository settings** is chosen, a setting the
  repository's file may not set cannot be changed, with the tooltip "Only the user settings can set this". The words
  chosen are "Write changes in", "Default", "From user settings", and for a value not taken "User settings: not a
  number. The default is used." Line 2 (**Edit in settings.json** opens the file in a File page) waits for the File
  page (M6): the button is shown disabled with the tooltip "Not yet", and the card stays **Specified**. (Built since:
  the button runs `worktree-setup.edit-settings-file`, Q290.)
- **Q107** — `settings.theme`: **Theme…** in the **…** menu opens a picker of the five themes. The theme picked is
  written in the user's file by the store (Q44 is still open), and the window takes it when the watcher reads the file
  again. The picker shows no theme as the user moves through the list, which the card asks. Should it, and should the
  theme be written only when the picker closes? The card stays **Specified** (Q44). (Built since, Q292; moving through the list
  still shows no theme.)
- **Q108** — Cards of this slice left **Specified**: `repo.open-folder`, `repo.open-recent` and `repo.select` (Q43);
  `settings.theme` (Q44, Q107); `settings.edit` (Q106); `log.record` (Q101). `repo.read-repositories` is **Built** on
  the reading of Q19, and `welcome.page` on Quick search (slice 4) and the page of a commit (slice 5): the page of a
  commit opened in any way ticks **Open a commit, compare two branches**. Its buttons **Show the commits**, **Open a
  commit…** and **Compare…** stay disabled, as the rest of the steps but the first: should **Show the commits** narrow
  the timeline to the branch checked out and **Open a commit…** open the box of **Open commit** now? **Show Command
  Log** and **Open Settings** in Quick search now run. No key was added to `state.json`.
- **Q109** — `graph.view` counts the labels past three only in the narrow layout. A row of the wide layout with many
  labels shrank its message to `…`, so now its message keeps half of the first line, or all of it when shorter, and the
  labels take the rest: as many named as fit, then the count of the others, `+2`, with their names in its tooltip, as
  in the narrow layout. Should the card say so, and is half the right share?
- **Q110** — Operations run from the window (M4, slice 1). The engine runs each on a thread of the operations of the
  window, one at a time, with one engine for each repository kept for its undo journal; a question or a plan to
  confirm is sent to the window and the thread waits for the answer. Choices made without a card: `preview.confirm`
  is the dialog of the window (`views::feature::confirm_view`), since the engine already decides when to ask
  (`engine::needs_confirmation`), so its card is now a View; `Enter` does not press the button of a dialog that cannot
  be undone (the toolkit's `DialogRole::Danger`), as the card says **Run** is the default only when the plan is not
  destructive; the button of a question of text or of a pick says its title, **Create branch**; once an operation ran,
  the bar over the timeline says what the plan's **Say** says, or `<title>: done.`, with **Undo** unless Undo cannot
  take it back or it was Undo itself; a step that failed says `` `<the step>` failed: <what git said> ``. **Sync** and
  the button of its menu are one control; **Stash** in **…**, and **Sync**, **Pull** and **Push**, are disabled until
  their operations are built. Are these right?
- **Q111** — Branches and tags (M4). `branch.checkout`, `branch.checkout-remote`, `branch.checkout-detached` and
  `tag.checkout` read whether `HEAD` or the revision checked out has a `.gitmodules`, and with
  `submodules.updateOnCheckout` on add `git submodule update --init --recursive` to their plan. This stands in for
  `submodule.update-on-checkout` (M8), which has the Confirm when the update would leave commits behind: should the
  checkouts call it once it is built, and is a plain step right until then?
- **Q112** — `branch.format-name` fills `${date}` in UTC: the context of an operation has no offset of the local time
  yet. `branch.name-from-format` needs a regular expression for `pattern`, the values given last kept between runs and
  a read of the branches that contain a commit, so it is not built, nor line 6 of `branch.create` (the pick between
  **Use the format** and **Type a branch name**); both cards stay **Specified**. Which crate may keep the values given
  last, and is `${date}` in UTC acceptable until then?
- **Q113** — `tag.push` pushes to `origin`, or to the first remote without one, through a function of its own until
  `remote.default` is built by the slice of the remotes; then it calls that.
- **Q114** — `branch.clean-up` runs `git worktree prune`, then `git worktree remove [--force] <path>`, then
  `git branch -d` and `git branch -D`; it leaves out the steps that unlink the folders linked into a worktree on Windows
  and remove the folders left empty, which come with the worktrees (M7). Its pick says of each branch, joined with
  commas, "merged into `<branch>`", "upstream is gone, not merged", "with its worktree `<folder>`", "`<n>` files not
  committed" and "with its missing worktree"; its title is "Delete `<n>` branches and `<m>` worktrees". Are the words
  right?
- **Q115** — `branch.delete` decides between `-d` and `-D` by reading whether the branch is in its upstream, or in
  `HEAD` without one or when it is gone; when `git branch -d` still fails, git's words are shown and nothing offers
  `-D` after. Should it?
- **Q116** — `branch.checkout-remote` asks "`<name>` has `<n>` commits that `<remote>` has not, which has `<m>` commits
  of its own", with **Check out `<name>` and rebase it onto `<remote>`** and **Check out `<name>` as it is**. On the
  branch checked out, level with its remote branch, nothing runs and it says "`<name>` is checked out already, with
  everything `<remote>` has." An operation started on a reference that changed since its menu was drawn refuses with
  words of its own: "There is no branch `<name>`.", "`<name>` is checked out already.", "`<name>` is checked out in
  the worktree `<folder>`.", "The commit of `<tag>` is checked out already.". Are the words right?
- **Q117** — The menus. A branch, a remote branch and a tag have the same operations in the branch rail and on their
  label in the timeline; the menu of a label has them, then those of its commit (**Create branch here…**, **Create tag
  here…**, **Check out commit (detached)** unless it is checked out), then the fold of a merge. **Open in worktree**,
  the entries of a worktree and **Delete** of a remote branch are shown greyed out until the worktrees (M7) and the
  remotes are built. A double click on a label does what `branch.open` says and never opens the page of the commit,
  also on the branch checked out, where it does nothing; `Enter` while the pointer rests on a label does the same, since
  the list does not tell the two apart. In the branch rail the first click of a double click narrows the timeline, as a
  click does, before the double click checks the branch out. Is that right?
- **Q118** — Cards of this slice left **Specified**: `branch.create` and `branch.name-from-format` (Q112);
  `branch.pane`, whose top with the branch checked out and **Create branch**, and the **…** of the Worktrees list, are
  not built; `undo.last`, whose lines about a stash, a cherry-pick, a squash merge and conflicts wait for those areas.
  (`undo.last` is built since: Q293.)
- **Q119** — The operations on the files of the Changes page (`changes.stage`, `changes.unstage`, `changes.discard`)
  are given their file as a new kind of target, `Target::File(path)`, and the header of the page as `Target::Changes`,
  which is how `changes.discard-all` tells the header (the files of the list, the conflicts left alone) from the draft
  (the whole working tree); `gitardo-cli` takes them as `--target file:<path>` and `--target changes:list`. Words the
  cards do not give: an operation on one file given none says "Choose a file of the Changes page.";
  `changes.stage-all` and `changes.unstage-all` with nothing to do say "There is nothing to stage." and "There is
  nothing to unstage."; the refusal of `changes.discard-all` from the draft names the operation `merge`, `rebase`,
  `cherry-pick`, `revert` or `patch`. Undo never offers to take these back: their plans say Undo cannot, with what
  does ("Unstaging it takes it back."). With `commandPreview` `off`, the header's **Discard all changes** names in
  `git restore --source=HEAD --staged --worktree` every tracked file of the list and leaves the untracked files that
  are not staged to `git clean -f`, since `git restore` refuses a path git does not know. Are these right?
- **Q120** — `identity.switch`: each profile is one choice, `Name <email>`, with **In use** under the one that matches
  the identity used now; the identity used now is said under the title of the pick (**Now: `Name <email>`**), since a
  pick has no placeholder. A name or an email left empty is refused with "A name and an email are both needed:
  nothing was changed.", as an operation cannot end with nothing. **Edit the list of identities…** is not in the pick
  yet: the settings page cannot be opened at a key, and an operation has no follow-up that opens a page. The plan,
  when `commandPreview` shows it, says the commands but not "This is saved in the configuration of this repository
  only.", since the dialog says a reason only for a plan that is `Confirm` or `Destructive`. `commit.amend` before the
  first commit says "There is no commit to amend yet." Should the cards say so?
- **Q121** — The Changes page (`changes.pane`) and the draft. The message box is one line for now: the toolkit has no
  input box of several lines, and a message pasted with blank lines keeps them. The draft shows the first line of the
  message, or **Describe the next commit**, but it is typed in the Changes page: **Review** opens the page with the
  keyboard in the message box, as **Show changes** does, and **Commit** of the draft is the one of the page, so it
  reads **Amend** while **Amend last commit** is ticked there. The message is kept for each repository in `state.json`
  under `commitMessages` (`[{"folder", "message"}]`, written when the window closes, forgotten once committed) and the
  layout of the list under `fileLayouts.changes`, `list` to start with. The buttons of the files and of the header are
  icons with their names as tooltips; those whose operations come with other areas are shown disabled: **Open file**,
  **Open changes** and **Open all changes in one Diff page** (`diff.page`, `file-view.page`), the stashes
  (`stash.push`, `stash.push-staged`, `stash.push-file`), **Save** and **Save all** (`changes.save`), and the
  buttons of a file in conflict (`conflicts.*`); a click on a file in conflict opens nothing yet (`merge-view.page`).
  The sparkle button is disabled until the AI provider (M9, `commit.draft-message`). **Switch identity…** is put after
  **Undo** in **…**, with the operations. Are these right?
- **Q122** — Cards of the area left **Specified**: `changes.pane` (a click on a file in conflict opens the Merge page,
  and **Save all** appears with a file not saved: both need pages and editors of other milestones); `changes.unsaved`
  and `changes.save` (Gitardo has no editor yet, so no file is ever not saved); `changes.open` (the Diff and File
  pages); `changes.discard-all` (its line on the files only an editor has changed waits for `changes.unsaved`);
  `commit.create` (git's prepared message, `SQUASH_MSG` or the message of a commit being split, is not read yet:
  should `repo.read-state` read it, or the commit box when it is empty?); `commit.draft-message` (the AI provider,
  M9). `preview.confirm` has its line on staging a file tested, and stays Specified for its first line, which is not
  of this area. (Both built since: the empty box takes `SQUASH_MSG` after the operation that leaves it, and the
  first line of `preview.confirm` is tested from the window; Q289.)
- **Q123** — `stash.push-file` on a rename that is staged: `git stash push -- <old> <new>` fails, as the old name is no
  longer in the index (`pathspec ... did not match`). The plan first runs `git reset --quiet -- <old name>`, then
  stashes both names; the card says so now. Popping the stash brings both names back, but the rename is no longer
  staged. Is that right, or should the plan stash with `--staged` as well?
- **Q124** — `sync.pull-with` asks its way as a pick, with the key `way` and the choices `plain`, `rebase`,
  `autostash` and `rebase-autostash` (`gitardo-cli sync.pull-with way=rebase`); the entries of the menu of **Sync**
  and of Quick search give the answer before it asks. The pick is titled **Pull** and marks the default way of the
  settings **default**.
- **Q125** — `sync.sync` and `stash.apply` when the pull or the stash stops on a conflict: the conflicts banner is the
  card of `conflicts.`, not built yet. The tests check what it reads instead: a rebase stopped with the files in
  conflict, nothing pushed, the stash kept.
- **Q126** — Cards of M4 left **Specified**: `remote.manage` (typing `github` in its pick needs a pick that filters by
  the URL too, and its last line needs `github.`), `remote.add` (**Load pull requests from GitHub** needs `github.`),
  and `stash.pane` (the Stashes list as its own place is not built; built since, see Q147).
- **Q127** — A refusal of `sync.fetch`, `sync.pull`, `sync.push` and `sync.sync` in a repository with no remote offers
  **Add remote…** (`remote.add`); the command line prints the offer after the reason. The forced pushes and sync share
  the refusals of `sync.push`; sync also refuses, without running anything, when the pull would rebase over changes
  that `pull.autostash` off does not set aside.
- **Q128** — The window (M4): the menu of **Sync** lists **Fetch**; **Pull** with `↓N` when there is something to pull
  and its command in the tooltip, then the four ways, the one **Pull** takes marked **default** on the right; **Push**
  with `↑N`, **Push (force with lease)** and **Push (force)**. The way `git pull` alone is **Pull (no options)** in the
  menu too, not **Pull**, so that two entries do not have the same words. A way chosen gives `sync.pull-with` its answer
  and nothing is asked. Not done here: the ways to pull and the forced pushes in Quick search (they are not among its
  commands yet); **Add remote…** on a refusal, which the window says without the button; **Stash changes…** in the
  menu of the draft, left to the slice of the changes; **Show Stashes** after a pull that left its autostash, which
  waits for `stash.pane` (built since, with `stash.autostash-left`: see Q152). So `stash.autostash-left` and `graph.toolbar` (its narrow window line has no test yet) stay
  **Specified**, and so does `undo.last`, whose other lines belong to other slices. (`graph.toolbar` is built since:
  Q286.)
- **Q129** — `tag.push` now asks `model::remote::default` for its remote (Q113), and the menu of the draft runs
  **Stash changes…** and **Stash staged changes…** (`stash.push`, `stash.push-staged`).
- **Q130** — `diff.line-diff` returns `Result<Vec<Change>, TooComplex>` and takes, beside the two texts, a function
  that says whether to go on, which the worker reads the clock and its cancel token in, since `text` does no I/O; its
  card now says so. It is the O(ND) algorithm of Myers (1986) with the linear-space search of the middle snake of the
  same paper (section 4b), on lines numbered once, after the lines common at the start and the end and the lines found
  on one side only are taken out. The result is minimal on the lines searched, but the middle snake searches from both
  ends, not "from the start of the texts" as the card says: two diffs of the same size may differ in which equal lines
  they pair. Does `conflicts.resolve-by-rules`, which runs it on both texts reversed, need the forward search alone?
- **Q131** — `diff.align` is `model::diff::align`, but `model` and `text` do not use each other: the word marks of
  each pair of lines are given to it as a function, which the worker fills with `text::diff::word_diff`. Is that the
  reading of the edges, or should `model` use `text`?
- **Q132** — `diff.scroll-together` and `diff.align` disagree: with filler rows keeping the two sides level, every row
  is as tall on both sides, so the two sides of a page are one list, scrolled as one, and the mapping is the identity.
  `model::diff::scroll_together` is built and tested on anchors that are not level, as for a side that has no filler
  rows, but the page does not call it. Should the card say the two sides of the Diff page are one list, and keep the
  mapping for the Merge page or the review of the conflicts in **Columns**?
- **Q133** — The Diff page refuses a binary file or one with too many differences with the notice of the window (the
  bar over the timeline), and asks before a very large file with a dialog of the window, **Continue** and **Cancel**.
  A page of one file opens only once its file is read, so that nothing opens when it refuses; a page of several files
  opens at once and says *This file looks binary.* or *This file has too many differences to display.* under the header
  of such a file, and a very large file in it is read without asking. `F7` going on to another file reads it without
  asking either. Are these right?
- **Q134** — The words chosen without a card: **No changes.** under the header of a file with no change, **Git could
  not read this file.**, **Loading…**, the tooltip **Staged** on a change staged, and **Lines of the diff** for the
  screen reader. The band of a changed change in the gutter is in the color of a modified file of the theme, amber in
  the dark and light themes, where the card says blue: the themes have no blue for it. Which color should it be?
- **Q135** — Where the page is opened from, and between which versions: a file of the inspector or of the page of a
  commit, between the first parent of the commit (or **Empty** for a root commit) and the commit; a file of the draft in
  the inspector, between `HEAD` and the working tree; an untracked file of a stash, between **Empty** and the third
  parent of the stash, named by the stash's short hash. **Open all changes in one Diff page** and **Open in the Diff
  page** are titled as the page of the commit is, its short hash and subject. Q96 is answered: the links of
  `details.show` lines 6 and 7, `details.stash` line 3 and `commit-page.view` lines 8 and 9 open the Diff page, and the
  three cards are **Built**.
- **Q136** — What the Diff page does not have yet, though `diff.page` is **Built** on its acceptance lines: the colors
  of the code (Q91, `file-view.highlight`), editing the working tree side (`diff.edit`: the text editor of `editor` is
  not written yet), the unsaved text of a File page as the working tree (no File page yet), the title buttons of each
  file of a page of several files, and the buttons of the gutter (`diff.stage-change`, `diff.unstage-change`,
  `diff.revert-change`, M5). `diff.resize-sides` stays **Specified** for its line 4 (a **+** in the gutter stages its
  change), the rest of its lines tested. `diff.open-file` and `diff.open-at-line` stay **Specified**: they open the
  File page (`file-view.page`, M6). `details.compare-file-with` stays **Specified**: it needs `compare.side-choices`
  and `compare.resolve-revision`, which come with the comparison. `diff.staged-changes` is **Built** on a change staged
  with `git add` of a version that has it, since `diff.stage-change` comes in M5. Is that enough for now?
- **Q137** — `diff.collapseUnchanged` is read from the user's settings file only, as the window reads it when it
  opens; the fold button of a page switches that page alone, and a page keeps the folds clicked open until it is
  switched. `F7` landing in a fold unfolds nothing yet, since folds keep three lines next to each change and a change is
  never in a fold. The width of the left side is kept with the page and for the next page while the window is open, not
  in `state.json`. Are these right?
- **Q138** — `changes.open` is built, and **Open changes** and a click on a file of the Changes page with
  `changes.openOnClick` set to `diff` open the Diff page. What it decides on the other pages waits for them: **Open
  file** and a click with `file` (the File page, M6) do nothing yet, nor does a click on a file in conflict (the Merge
  page); **Compare with the saved file** waits for an editor. **Open all changes in one Diff page** on the Changes page
  stays disabled: a Diff page has one pair of versions for all its files, while each file of the Changes page has its
  own (the index against the file on disk, or the last commit against the index). Should a Diff page take a pair of
  versions for each file, or should the button open the last commit against the file on disk for every file?
- **Q139** — `github.detect-remote` is built as a derivation ahead of M9. Its last line, the badges going when the
  remote on GitHub is renamed away from `origin`, is tested on what it decides: the result becomes nothing. The badges
  come with `graph.pull-request-badge`, whose tests will check that they follow it. Is that enough to call it built?
- **Q140** — A change of a Diff page reaches its operations as `Target::DiffChange`, a `model::diff::DiffChange` with
  its path, the two versions (`DiffSide`), its lines on each side and their text as the page showed them; the
  operations read the file again with a new query, `Query::FileVersions` (the file in `HEAD` or the commit compared
  with, in the index and on disk, answered by `diff.read-sides`), and refuse with *The file changed since the diff was
  shown. Try again.* when either side no longer has those lines there. An edit elsewhere in the file is no refusal.
  Is that the check the cards mean?
- **Q141** — `features` now uses `text` (an edge the architecture draws), for `text::diff::change_patch`. The patch is
  computed with no time limit, since an operation reads no clock. The last line of a text with no line break is told
  apart from the same line with one, so that a line added after it also changes it and its patch carries
  `\ No newline at end of file`. A path with a quote or a control character is written in double quotes, as git does.
  Are these right?
- **Q142** — The detail of the plan of `diff.stage-change`, *The patch on standard input changes this part of
  \<file\> in the index:* with the first `@@` line, is not shown: a `Plan` has no field for it, and the preview shows
  the patch under its command already. Should `Plan` get a detail, or should the card drop it?
- **Q143** — Words chosen without a card: *This change is staged already.* and *Nothing of this change is staged.*,
  now on the cards; *Choose a change of a Diff page.*, *Choose the file of a Diff page.*, *Only a change of the working
  tree against HEAD or the index can be staged.*, *Only a change of the index or the working tree against HEAD can be
  unstaged.* and *Only a change of the working tree can be reverted.*, which the window never shows since it offers
  the buttons only where they apply; the titles **Stage change in \<file\>**, **Unstage change in \<file\>**, **Revert
  change in \<file\>**, **Stage \<file\>**, **Unstage \<file\>** and **Discard the changes of \<file\>**. Are they
  right?
- **Q144** — The buttons of the gutter are drawn on the first row of the band of their change, side by side, 13
  pixels each since the gutter is as wide as a control, clear of the grip of the line between the sides: **+** or
  **−**, then **⟲** (the icon **Refresh**). The **−** of a change of the working tree staged already is named by its
  tooltip, **Staged — unstage this change**. The title buttons **Discard changes** (×), **Stage file** (+) and
  **Unstage file** (−) are icons beside the fold button of the bar, on a page of one file only: a page of several
  files has no title buttons for each file yet (Q136). Should the gutter be wider, or the buttons show only on hover?
- **Q145** — `diff.revert-change` stays **Specified**: its line 3 (`Ctrl+Z` brings the change back, unsaved) needs the
  editor of the working tree side (`diff.edit`), which is not written. Until then it writes the file at once with
  `write <file>`, its lines put back in the line endings of the file, the end of the file as the left side has it when
  the change ends both, and Undo says it does not take it back. Its lines 1 and 2 are tested. Is that enough for now?
- **Q146** — A Diff page of one file closes when a read finds the file on neither side, as after **Discard changes**
  of an untracked file; the page shown before it, or the timeline, shows in its place. This holds for any way the file
  goes, an untracked file deleted by another program included. Is that right?
- **Q147** — `stash.pane` is `views::refs::stashes_view`, which draws one row of the Stashes list (its title, a stash,
  a line of its files, or the words of the list with no stash), not `views::stash::pane` over a pane of its own: the
  list is rows of the branch rail, so that the filter box, **Show all `<n>`…** and drawing only the rows in sight work
  for it as for the other lists (Q126). A stash's row is drawn by the rail's entry (its click, its menu, its tooltip)
  with the arrow, the message, the age and the buttons added by the list. The card now names that function. Is that
  right, or should the list become a pane with a scroll of its own?
- **Q148** — The buttons of the Stashes list: **Stash all changes…** on its title is a **+**, shown under the pointer
  as the **…** of the other titles, and always while the list is open with no stash, so that "the button above" is
  there to see; **Apply**, **Pop** and **Drop…** on a row are a check, an arrow up and a cross, shown under the pointer
  in place of the age of the stash, as the rail is 210 pixels wide. Should they be other icons, or always shown?
- **Q149** — Words chosen without a card for the Stashes list: **Show the files of the stash** and **Hide the files
  of the stash** on the arrow, **Git could not read this stash.** under a stash whose files git did not give (or that
  is gone), the tooltip **Untracked: it was not in the commit the stash was made on** on an untracked file, and the full
  date of a stash in UTC under `stash@{N}: <message>` in its tooltip. Its menu keeps **Show in the timeline** above
  the four entries of the card. Are these right?
- **Q150** — The files of a stash unfolded are read with `details.read-commit` on a worker, kept by the commit of the
  stash until a read of the repository no longer lists it, and read again when unfolded after git could not read them.
  Folding a stash keeps what was read.
- **Q151** — **Show Stashes** opens the Stashes list, empties the filter box of the rail, and shows the timeline with
  the rail when it was put away; it does not scroll the rail to the list, as a scroll area of `ui` cannot be moved by
  its view yet. With many branches the list may be below what is in sight. Should `ui` let a view bring a part of a
  scroll area into sight?
- **Q152** — `stash.autostash-left` in the window: once the repository is read again after a pull with `--autostash`
  (`FollowUp::CheckAutostash`), the warning of the card replaces what the pull said, with a warning icon and **Show
  Stashes** beside it. The stashes before the pull are those of the last read before it ended. The line of the card on
  the conflicts banner cannot be tested until `conflicts.banner` (M6): the tests check the files left in conflict
  (`UU a.txt`) and the stash named `autostash` first in the list. Is that enough to call it **Built**?
- **Q153** — The commit operations of M5 are built: `merge.into-current`, `merge.choose-way`, `merge.undo-squash`,
  `cherry-pick.commit`, `cherry-pick.skip-empty`, `revert.commit`, `reset.to-commit` and `selection.select` are
  **Built**. A stop on conflicts is tested as the M4 sync slice did (Q125): the operation fails, and the repository is
  left as git leaves it, since the conflicts banner comes in M6. Is that enough until then?
- **Q154** — A plan whose danger is None shows no reason, so *No conflicts expected.* (`merge.into-current`,
  `cherry-pick.commit`) and *In this order:* with the commits (`selection.cherry-pick`) are not said: the dialog says a
  reason only for Confirm and Destructive. Should a plan carry a note the dialog shows whatever its danger?
- **Q155** — A choice of a pick has a description and no detail, so each way of `merge.choose-way` says *`<command>` ·
  `<what it does>`* as its description; the card now says so. A squash merge always asks first, with its note, also
  when no conflict is predicted. **Show changes** is not in the message after a squash merge: no operation opens the
  Changes page yet. Are these right?
- **Q156** — The message git prepares after a squash merge is read from `SQUASH_MSG` in the git folder by the thread of
  the operations, once the plan ran, and put in the commit box when its message is empty or only spaces. Right?
- **Q157** — `merge.undo-squash` takes back its entry of the journal (`Undo::TakesBack`), so that an Undo after it
  does not find the squash merge again; the card says its Undo is none, which is still true for the user. Right?
- **Q158** — When a cherry-pick or a revert fails, the engine runs its follow-up `cherry-pick.skip-empty` (a new
  `FollowUp::Recover`); when that refuses, because nothing is stopped on an empty commit, the original failure is the
  one shown. The card says it never refuses: it refuses inside the engine, and the user never sees it. A squash merge
  or a series of cherry-picks that stops keeps its entry of the journal, so that Undo still takes it back whole. Right?
- **Q159** — `message.edit` asks for the message with the question dialog of the window, not in a box in the
  inspector, which is not built; and it reads no bisect, since `repo.read-state` does not say whether one is under way.
  It stays **Specified** for its lines 4 (`Escape` in the box) and 5 (a bisect). For an older commit, the interactive
  rebase writes its files in `gitardo-rebase-<time>` inside the git folder, not in the temporary folder of the system,
  since `features` reads no environment; that plan is the one `rebase.interactive` will have, and the two should be
  one when the rebase slice is merged. Right? (Lines 4 and 5 are built since, with the question dialog: Q289.)
- **Q160** — **Edit message…** is in the menu of every commit while no operation is under way, not only where
  `rebase.find-line-before-head` gives a line: the menu does not read git, and the operation refuses a commit off the
  line with the card's words. **Merge into `<branch>`…** has no hint *choose how*: an item of a menu has a shortcut
  and a tooltip, no hint. Right?
- **Q161** — `Cmd`+click selects as `Ctrl`+click on macOS in the timeline only, where the selection reads the keys:
  the window does not yet map `Cmd` to `Ctrl` for every shortcut, as `ui::Modifiers::command` says it will. Should it?
- **Q162** — `selection.bar` and `selection.menu` are built since: **Squash into one** (`squash.commits`), **Copy as
  patch** and **Save as patch…** (`patch.copy`, `patch.save`), which leave the commits selected, and **Compare these
  two commits**, which runs `compare.with` on the older with the newer as its side already answered, so that the page
  opens with no question. `selection.cherry-pick` stays **Specified** for its line 5 (continued in Gitardo, M6). The
  bar and the menu empty the selection when their cherry-pick is chosen, before the dialog, not once it ran. Right?
  (Line 5 is built since: Q293.)
- **Q163** — The words chosen without a card: *Finish or abort the `<operation>` first.* for every operation of this
  slice; *Choose a commit first.*; *There is nothing to merge into before the first commit.*; *`<name>` is the commit
  checked out already.*; *There is no squash merge to undo.*; *Nothing is stopped on a commit that changes nothing.*;
  *There is nothing to cherry-pick onto before the first commit.*; *The message is unchanged.*; the titles *Merge
  `<name>` into `<current>`* with *(no fast-forward)*, *(fast-forward only)* or *(squash)*, *Cherry-pick `<short>`
  onto `<current>`*, *Revert `<short>`*, *Reset `<current>` to `<short>`*, *Skip `<short>`*, and *Cherry-pick N commits
  onto `<current>`*. Are they right?
- **Q164** — The files of an interactive rebase, the todo list and the new messages, are written in
  `gitardo-rebase` in the git directory of the worktree, not in a new folder of the system's temporary folder: the
  `exec` lines of a stopped rebase read them while it is stopped, maybe after the window was closed and opened again,
  and a folder of the repository is found again and deleted when the rebase ends, by any later plan. The plan says so
  with `FollowUp::DeleteWhenRebaseEnded`, which the engine runs after the plan, and after a plan that fails. Is that
  right?
- **Q165** — **Continue** of the banner of `rebase.edit-stop` is the operation `features::rebase::continue_edit`,
  registered under the identifier of its card, as `split.continue` is its own. `conflicts.continue-operation` (M6)
  should hand over to it, and to `split.continue`, while `rebase.read-stop` gives a stop. Is that how it should be?
- **Q166** — `split.continue` counts a file the commit split added among the files of it still changed, though it is
  untracked after `git reset --mixed HEAD~`: otherwise **Continue** would go on and leave that file out of the branch.
  Is that right?
- **Q167** — Several commits selected are a target of their own, `Target::Commits`, oldest or newest first as they
  were clicked; `squash.commits` runs on it, and `gitardo-cli` names it `commits:<hash>,<hash>…`. Since the oldest is
  not known before the line is read, the squash reads the line from the commit below each one it finds, until the line
  holds them all. Is that right?
- **Q168** — The reads before a rewrite are `git rev-list --parents -n 1`, `git merge-base --is-ancestor` and one
  `git log -z --topo-order` of the commits of `HEAD` that the parents of the commit lack, which gives their messages
  too, where `message.edit` names `git rev-list --topo-order --parents <hash>..HEAD` and `git rev-parse <hash>^`. They
  give the same line. Should `message.edit` read it the same way?
- **Q169** — `split.commit`, `squash.commits` and `squash.into-previous` say what the window does after them with
  `FollowUp::FillCommitMessage` (the message of the draft) and `FollowUp::SelectBeforeHead` (the commit as many before
  `HEAD`); `gitardo-cli` prints them. A rebase run from `gitardo-cli` and `gitardo` no longer opens an editor git was
  told of in `GIT_EDITOR` or `GIT_SEQUENCE_EDITOR`, which would wait for no one. Are these right?
- **Q170** — **Abort** of the banner of a stop is greyed out, its tooltip saying to run `git rebase --abort` for now,
  until `conflicts.abort-operation` (M6) is built. The banner shows the icon of a commit and the rows of the page the
  icon **…** as their grip, since the icons have no pencil and no grip yet. Should those icons be drawn now?
- **Q171** — `drag.drop` stays **Specified**: the rows of the timeline are a list whose own pointer handling selects,
  opens and shows the menu of a row, and a drag source over a row takes those clicks away; the list of the toolkit needs
  rows that can be dragged first, and the branch rail too. `rebase.onto` and `rebase.branch-onto` are **Built** on what
  their acceptance lines check of the operations: the run, and what `drag.list-drop-entries` offers; the gesture comes
  with `drag.drop`, which runs `rebase.branch-onto` with the preset `features::rebase::dropped(<branch>)`. Is that
  right?
- **Q172** — The entries of the menus offer nothing while a merge, rebase, cherry-pick, revert or patch is under way,
  but are still offered during a bisect: the state of the repository does not read a bisect yet. The fixup and the
  interactive rebase page look for the line to `HEAD` among the rows loaded only. Is that enough for now?
- **Q173** — **Squash into one** of the selection bar (`selection.bar`, built apart) is offered when
  `views::rebase::can_squash(repository, rows, selected)` says so, and runs `squash.commits` on
  `Target::Commits(selected)`. The interactive rebase page runs `rebase.interactive` on `Target::Head` with the rows
  under the answer `todo`, and **Continue** of the banner runs `rebase.edit-stop` or `split.continue` on `Target::Head`.
  Is that right?
- **Q174** — The bar of a selection keeps **N commits selected** whole and **Clear selection** always, then its other
  buttons in the order of the card, each that still fits: in a narrow timeline **Copy as patch** goes first, then
  **Cherry-pick onto …**, and **Squash into one** last. The menu of the selection still has every entry. Should the
  bar rather wrap, or show a **More** button?

From milestone M6, the rules that resolve conflicts with no choice (each built with the reading given, until the card
says otherwise):

- **Q175** — `conflicts.resolve-by-rules` is `text::conflicts::resolve_by_rules(input: &RulesInput) -> RuleResolutions`,
  not in `model` as its card said: it uses `text::conflicts::parse_markers` and `text::diff::line_diff`, and `model`
  may not use `text`. Its input is the file and the three versions of the index as bytes, and `keep_going`, which stops
  the diffs as `diff.line-diff` does: a file whose diffs are stopped is left alone (`LeftAlone::TooComplex`), and so is
  one whose base or side is not UTF-8. The stretch of a conflict is none when no anchor is found before another
  conflict. Is that right?
- **Q176** — Readings of step 5 of the card, each needed by the tests against git: (a) the changes of each side to the
  stretch are found by a minimal line diff of the stretch alone; the pairing of the whole file (step 1) only finds the
  anchors. With lines that repeat, the two pairings can differ, and a conflict was then resolved otherwise than from
  its three parts, which the line about 150 files forbids. (b) A line both sides have alike, where both wrote it in
  place of lines of the base, stands at the first line of the base both allow, which it then replaces on both sides,
  else at the first place between lines both allow; standing between lines, it left `5x` against `6y` in a conflict
  both sides started with `5x`. (c) What is between two such lines is put together once, grouping changes that
  overlap: two sides with different lines there leave the group to the rules of spaces, with no further search for
  lines alike. (d) The two sides *met* only in a group where both changed lines differently; a group where both made
  the same change does not count. (e) Lines kept by both sides are written as the current side has them. Are these
  right?
- **Q177** — Three checks the card does not list leave a conflict, each found by the files of lines that repeat, where
  the result had a `}` too many or was resolved from the whole file but not from its parts: (a) in step 4, the stretch
  with its anchors is found in a side's own file only where the anchors are paired with the same lines of the base as
  in the file; (b) every line of the stretch outside the markers is kept by both sides from the same line of the base,
  or written by both in a change of their own: a line one kept and the other wrote is one git took from one side and
  wrote outside the markers; (c) when git wrote the base between the markers (`diff3`, `zdiff3`), the base of the
  stretch is the lines of the base both kept above the markers, then that base, then those kept below: else the
  stretch is paired otherwise than git paired it. Should the card say
  them in steps 4 and 6?
- **Q178** — The tests merge with `git merge-tree --write-tree`, `merge.conflictStyle` set for each style, every file of
  a test in one throwaway repository, rather than with `git merge` or `git merge-file -p` for each file as the
  acceptance lines say: it is the same merge of git without a working tree, hundreds of files in a few processes. A
  resolution is checked against the merge expected, which is checked against what git writes when it merges the same
  changes kept apart by a line between each two. The thousands of files of M6's exit run in
  `conflicts_resolve_by_rules__thousands_of_files_against_git`, ignored by default
  (`cargo test -p text --test conflict_rules -- --ignored`), since the `cargo xtask fuzz` of
  [06-testing.md](desktop/06-testing.md#conflicts-against-git) does not exist. Two kinds of random files are not made with a
  collision, because the rules rightly read them another way: different lines both sides add at one place beside a
  line one side changes, which a diff reads as that change; and a collision beside a line both sides write the same
  way, which reads as *one side added more of the same*. Should `cargo xtask fuzz` run the ignored test?
- **Q179** — `conflicts.side-names` is built as a derivation of what the reads found, `SideNamesInput`: the operation,
  the branch checked out, and for `HEAD`, the commit brought in and `onto` their hash, the branches at them and their
  subject, with `head-name` and the subject of a patch. The reads its card lists are not gathered by any function yet:
  should they be one read, `git::read::conflicts::read_side_names`, built with the window of the conflicts? Choices
  made: a branch at a commit is the first one `for-each-ref` lists that does not end in `/HEAD`, so a local branch
  before a remote one, by its short name; the current side of a rebase with no branch at `onto` is its short hash, and
  its subject is that of `HEAD`; a stash has no subjects; the words are in `model` (`SideRole::words`), not in
  `views/src/text/`, since several pages and the questions to the AI provider use them. Are these right?
- **Q180** — The bisect is read with the state of the repository, as the operation under way is, so the banner and the
  marks come with every read. `bisect.marks` is not a view of its own: `model::graph::labels` gives the labels `bad`,
  `good` and `skipped` after the others, and `views::bisect::marks_view(mark)` gives the icon each is drawn with (a
  check, an error sign, a minus), so its card's Function line changed. `undo.last` refuses with *End the bisect first.*
  while a bisect is under way. The message that names the first bad commit is the one said once an operation ends, its
  **Undo** left out, with **Show in the timeline** and **End bisect**; once it is put away, the banner still says it,
  with **Show** and **End**. Are these right?
- **Q181** — The files of `patch.save` and `patch.apply` are chosen in the system's dialog (the same as **Open
  folder…**), not a dialog of Gitardo's own, so their cards now say so; `gitardo-cli` takes `path=<path>` and
  `file=<path>`, and the text of the clipboard as `clipboard=<text>`. The temporary file of a patch from the clipboard
  is deleted by a step the engine runs after the plan whether it succeeded or not (`FollowUp::Cleanup`), shown in the
  plan and logged. A plan has no note of its own: what `patch.apply` says of the commits and of the conflicts is shown
  only when it asks first, for conflicts, so **No conflicts expected.** is never shown. The journal of Undo lives in
  the window, so the Undo of a patch of commits at once is in the window only; `gitardo-cli` undoes it one commit at a
  time by the reflog. `patch.am-stopped` is now **Built**: the banner of an operation stopped on a conflict
  (`conflicts.banner`) draws it through `views::patch::am_stopped` while `git am` is under way, and the Merge page
  names its sides; it takes the `BannerState` of the banner, as no `OperationState` exists (Q240). Are these right?
- **Q182** — The questions of a side of a comparison are the window's own pick: its choices carry no group heading
  (**Opened before**, **Branches**…), since a choice has none, and a revision typed is checked only once accepted, not
  while typing. The comparisons opened before are given to `compare.choose` by the window before it asks
  (`Answer::Recent`). `reflog.browse` offers only the actions whose feature is built; all five are since, and the
  card is built with a test of its line 3. Its entries have no date and time, which a pure feature cannot put in the
  local zone. `compare.path-with` is not built: its entry points are the Files and File pages of M6, and it is
  best written with them, on the helpers of `features::compare`. Are these right?
- **Q183** — The comparison page lists the files that differ without their changes: a click opens a file, and **Open
  in the Diff page** all of them, in the Diff page, so **Show the changes**, the folds, the tree of the files, their
  context menu (**Compare with…**, which is `details.compare-file-with`) and `Ctrl+F` in the page are not there yet,
  nor the folding of the two groups of commits; a commit **also in** the other side says so in words, not greyed out.
  A comparison page and the operations run from it keep the repository of the page as it was last read, so its
  choices are those of that repository when the window shows another. A page whose side follows a name, or is the
  working tree, is compared again at every read of its repository, which also moves a branch side that moved. The
  comparisons opened before are kept in memory, for each repository, not between runs. Selecting another row of the
  timeline closes the comparison in the inspector. Are these right, and which of the missing parts should come first?
- **Q184** — The **…** menu of the header now follows the groups and the order of `graph.more-menu`, with what is built:
  **Compare branches, tags or commits…** is in it, and **Switch identity…** has a group of its own. The card stays
  **Specified** while **History of the active file** (`file-history.file`, M7), **Add submodule…** (`submodule.add`,
  M8) and **Load pull requests from GitHub** (M9) are missing. The reflog is **Reflog**, as `reflog.browse` names it,
  not **Browse reflog…** as this card does, and **Settings** and **Theme…** close the menu though the card does not
  list them. Which name should the reflog have?
- **Q185** — The operation `git am` is named **Patch (git am)** in the banner, as its card says, and *patch* inside a
  sentence and a title: **Abort patch**, **Skip this step of the patch**, *the patch can continue*. **Abort** of the
  banner of a stop to edit or split a commit (Q170) now runs `conflicts.abort-operation`, and **Continue** of
  `conflicts.continue-operation` at such a stop hands over to `rebase.edit-stop` or `split.continue`. Are these right?
- **Q186** — **Resolve … automatically** in the banner opens `review.page` with every file in conflict; **Resolve
  automatically** in the message of `conflicts.stopped` runs `conflicts.resolve-automatically`, as its card says. Its
  question offers **Resolve** and **Review**, and **Review** runs nothing and asks the window to open the review of
  those files (`FollowUp::OpenReview`), which the window does with `store::review::open(state, Some(files))`, putting
  away the message of the operation. The question is the confirmation, so the plan's danger is None rather than
  Confirm: a Confirm plan would be shown a second time. The rules are given no time limit (`keep_going` is always yes),
  since a feature reads no clock; their own limits of size and lines stop them. Are these right?
- **Q187** — **Resolve with AI** is drawn, disabled, in the banner and the message when `conflicts.ai` is on, until
  `ai.resolve-conflicts` is built (M9). `conflicts.restore` stays **Specified** for its third line, the answers of the
  AI provider shown again. Is that right?
- **Q188** — The window counts the conflicts the rules resolve after every read of a repository with files in
  conflict, on a worker (`git::read::conflicts::read_texts`, the first 200 files), only when `conflicts.autoResolve`
  or `conflicts.ai` is on; the last count stays until the next one replaces it. Both settings are read from the
  user's file only and are off by default. `gitardo-cli` has no switch for them, so `conflicts.resolve-automatically`
  refuses there; its window tests turn the setting on. Is that right?
- **Q189** — `conflicts.stopped` needs the title of the plan whose step failed: the engine's failure of a step now
  carries it (`StepFailure::plan`), and the window shows the message in place of the failure once the repository read
  again has files in conflict, keeping the count of files of that read. What an operation offers next
  (`FollowUp::Offer`, and the offer of a refusal) is now a button beside what it said: **Continue** after the rules left
  no file in conflict, **Open settings**, which opens the settings page, after a refusal. Are these right?
- **Q190** — A file both sides changed opens in the Merge page; a file both sides added, or added on one side only,
  opens in the File page with git's markers in it (`OpenAt::File`); a file one side deleted opens, as its card says,
  in the Diff page of its base (`DiffVersion::Base`, `:1:<file>`, which offers no button in the gutter) beside the
  working tree. Where it opens is decided by `views::conflicts::open_target(file, operation) -> Option<OpenAt>`, beside
  the card's `open_file_view`. The line of a review card comes in the intent (`ConflictsIntent::OpenAtLine`): the File
  page opens at it, the Merge page at the region of the result that line of the file on disk is in. Are these right?
- **Q191** — The buttons of a file in conflict on the Changes page are **+** (**Mark as resolved**), then **↓** for the
  incoming side and **↑** for the current one: the set of icons has no left and right arrows for the ← and → of the
  card of `conflicts.take-side`. Should arrows to the left and the right be drawn for them?
- **Q192** — The Undo of a resolution is `conflicts.restore` whether it is run from **Undo** or by itself, so the title
  of the Undo of `UndoRecord::ConflictsResolved` changed to **Undo: restore the conflicts of \<file\>**. Its plan says
  `Undo::TakesBack` of the entry it restores, as `stash.undo-drop` does, rather than Impossible: once it ran, Undo
  goes by the reflog again. Is that right?
- **Q193** — `merge-view.accept-proposals` takes the proposals of the rules straight from
  `text::conflicts::resolve_by_rules`, computed with the regions on the worker when `conflicts.autoResolve` is on, and
  matched to the region of the conflict whose markers they replace. `review.proposals` does not exist on this branch,
  so the proposals of the AI are not there, and the card stays **Specified** for its second line. Should the review
  page and the Merge page share one list of proposals for a file, kept by the store (`review.proposals`), which the
  Merge page would read in place of computing the rules again, and the review page keep up to date when a proposal is
  accepted on either?
- **Q194** — The result of the Merge page still cannot be typed in: the choices are edits of a text the page holds,
  with an undo and a redo of its own (`Ctrl+Z`, `Ctrl+Y` and `Ctrl+Shift+Z` in the page), saved only by **Mark as
  resolved**, which writes it before staging. To type, `F4` opens the File page at that line with the choices not
  saved in it (`merge-view.open-at-line`), and closing the page hands them over the same way (`merge-view.close`).
  Should the page hold the editor's text of the file, so that its undo and the edits of the page are one history, as
  the cards of `merge-view.take-side` and `merge-view.page` say?
- **Q195** — Answered: `merge-view.open-at-line` and `merge-view.close` are **Built**. A click on a line of any column
  puts the cursor on it; `F4` and **Jump to source** open the File page at the line of the result it stands for
  (`model::merge_view::result_line`). A page closed with choices not saved shows them in the File page, not saved, at
  the line of its cursor or of the region gone to. The function of `merge-view.close` is
  `app::store::merge_page::lines::closed`, not a view: a page is closed by the store, which forgets it, and there is
  nothing for a view to draw (Q237).
- **Q196** — The page refuses a file not in conflict with a version on each side with *This file is not in conflict
  with a change on each side.*, without the name of the file, since a notice of the window holds no text of its own;
  the card of `merge-view.page` now says so. Should notices take a text, so that this one, and others, can name what
  they are about?
- **Q197** — The columns of the Merge page are kept level by blank rows (`model::merge_view::merge_rows`) and scroll as
  one list, rather than each side scrolling with the result as `diff.scroll-together` would; the page is computed
  again after each choice and at each read of the repository, not a quarter of a second after typing, since nothing is
  typed in it yet. The columns cannot be made wider by hand (`diff.resize-sides`). The card of `merge-view.page` says
  so now. The title buttons **Previous change** and **Next change** of `merge-view.navigate` are not there: the window
  has no title buttons of a page yet. Are these right?
- **Q198** — Whether the base is shown is kept between runs in `state.json` under `mergeShowsBase`, written only when
  it is shown. Is that the right place and name?
- **Q199** — Answered: `merge-view.mark-resolved` is given its file by a target of its own,
  `Target::MergeResult(MergeResolution)`: the path, the bytes to write when the result has unsaved changes, and how
  many conflicts git still marks in it. Its plan ends with `FollowUp::CloseMergePage(path)`, which closes the Merge
  page of that file in the repository shown; `gitardo-cli` prints it as *Close the Merge page of \<path\>*. It now
  takes its staging step from `conflicts.mark-resolved` (`features::conflicts::staging`), so the two stage a file the
  same way.
- **Q200** — `git::read::merge_view::read_sides` now says whether the index has a version of the file on each side
  (`MergeSides::has_both_sides`, from stages 2 and 3), which the page needs to refuse a file deleted on one side; the
  page reads the setting `conflicts.autoResolve` where the banner reads it (`FeatureSettings::conflicts_auto_resolve`,
  a setting of the user, off by default). Are these right?
- **Q201** — A choice made while the regions of the result are computed again waits for them and is made then, as the
  card of `merge-view.take-side` says; `Ctrl+Z` and `Ctrl+Y` drop a choice that waits. When the file on disk changes
  while the page is open and has nothing unsaved, the page takes the file as it is now, and `Ctrl+Z` brings back what it
  held before. Are these right?
- **Q202** — `review.side-diff` stays in `model` as its card says, taking the line diff and the word diff of `text` as
  functions (as `model::diff::align` takes the word marks), since `model` cannot use `text`; the worker computes the
  two side diffs of every conflict when it reads the review, so a card shows them at once. Is this right?
- **Q203** — `review.write` reads the files again through a new query, `Query::FileTexts` (answered with
  `Answer::Files`), since an answer key is fixed and the files are many. Its detail is in `Step::Warn` lines before the
  `write` steps, since a plan has no detail of its own, so the log has it too. Its target is a new
  `Target::Resolutions`, with every file of the page, its text as read and the proposals accepted in it. Its undo is
  the journal's `ConflictsResolved`, which `undo.last` runs as `git checkout --merge -- <file>…`, the command
  `conflicts.restore` runs. Are these right?
- **Q204** — The names of the two sides are read by a new `git::read::review::read_side_names` (the operation, the
  branch checked out, `HEAD`, the commit brought in, and in a rebase `onto` and `head-name`, in `git am`
  `final-commit`). The slice that builds the banner may read the same; the two should become one when merged. Is this
  right?
- **Q205** — The review page opens from **Review the resolutions of the conflicts** in Quick search, from **Resolve …
  automatically** in `conflicts.banner` (every file), from the wand of a file in conflict on the Changes page (that
  file alone, `ReviewIntent::Open`), drawn only while `conflicts.autoResolve` is on, and from **Review** in the question
  of `conflicts.resolve-automatically`. The wand is a new icon of the set, `Icon::Wand`. The sparkle is not drawn until
  `ai.resolve-conflicts` is built (M9). With both settings off, the refusal is said where an operation's message is,
  with **Open settings** beside it. `review.page` stays Specified for its line about the sparkle (the AI). Is this
  right?
- **Q206** — What is accepted is kept with the page, by file and fingerprint, and forgotten once the page closes or
  the accepted resolutions are written; the choices are kept for the window by repository, file and fingerprint; Cards
  or Columns and the widths of the columns, in thousandths of the width, are kept for the window, not between runs.
  The report stays until something is accepted again. Is this right?
- **Q207** — No proposal of the AI exists yet: `review.proposals` is given no answers, and **Ask the AI about the
  rest**, the button with the model's name, **Ask again** and **Ask the AI** are left out until `ai.resolve-conflicts`
  is built; what the cards and the counts say of the AI is built and tested on data. `review.accept-all` is built with
  its line about the AI checked on proposals of the rules only, and `review.card` stays Specified for a card the AI
  failed on, which cannot be had before the AI is asked. Is this right?
- **Q208** — The review hands over to the rest of the window: **Resolve by hand** opens the Merge page of the file
  with the region of the conflict current and in the middle (`MergeIntent::OpenAt`, by the line of its first marker);
  **Keep it, with these changes** and **Delete it** run `conflicts.take-side` in the repository of the page, with the
  side given, so that it asks only its confirmation; **Continue the merge** runs `conflicts.continue-operation` at
  `HEAD`; **Undo** in the report runs `conflicts.restore`. The line number of a card, the files under **Also in
  conflict** and the eye beside **Keep it** and **Delete it**, which shows a deleted file's base beside it, open the
  file through `conflicts.open-file`. **Conflict at line N** is drawn as a button of words, with *Open the file at this
  line* as its tooltip. Is this right?
- **Q209** — **Edit** in `review.choose` is shown but disabled: the toolkit has no box of several lines yet, and
  `review.choose` stays Specified until it does; the four other choices are built and accepted at once. Should the
  box of several lines come with the File page of M6?
- **Q210** — In `review.columns` the bands are drawn in a gutter on each side of the middle column, and the widths
  are dragged with two splitters per conflict, kept for every conflict at once; under 600 pixels the columns stand one
  above the other, without bands. The card stays Specified until its lines about wrapping and about the pointer have
  their tests, which need a way to look at what a frame drew. Is this right?
- **Q211** — `review.show-result` opens the Diff page with a new version, `DiffVersion::Resolved`, holding each
  file's text with the accepted resolutions in place, read as the working tree with that text and named **Resolved as
  accepted**; its lines cannot be staged or reverted. Is this right?
- **Q212** — `conflicts.autoResolve` and `conflicts.ai` are read from the user's settings file once, into
  `FeatureSettings`, for the banner, the Merge page and the review alike. The toolkit now also names `Path` and `FillRule` for the views, for the bands. Is this right?
- **Q213** — The editor of the File page is the widget `ui::CodeEditor`, kept small: text, caret, selection by mouse
  and keys, typing, Undo and Redo, cut, copy and paste through the clipboard effects, one color and style for each
  kind of code. It leaves out wrapping long lines, scrolling sideways (a long line is cut at the right), characters
  drawn two columns wide, several cursors, italic and bold styles, folding, the minimap and an input method for
  composed characters. Which of these should come first?
- **Q214** — **Paste** is in the context menu of the File page but cannot be clicked: the window reads the clipboard
  only on `Ctrl+V`, so the menu would have to ask it for the text first. Is `Ctrl+V` alone right for now?
- **Q215** — A file that is not UTF-8 is decoded with each invalid byte as `U+FFFD` and stays editable, so saving it
  writes those bytes changed. Should such a file be read only instead, or decoded as Latin-1?
- **Q216** — The cards name the Files page, Quick search and the blame as ways to open a file in its page; none of them
  exists yet, so the File page opens from Changes (a click when `changes.openOnClick` is `file`, and **Open file**) and
  from **Open the file at this revision** on a file of a commit. **Save all** on Changes now runs `changes.save`
  (**Built**), given the texts of the File pages with edits not saved (`Target::FileSaves`); **Save** on a row still
  saves the File page of that file through `file-view.save`, which asks before writing over a file changed on disk
  (Q239). Are these right?
- **Q217** — `file-view.save` logs the path on disk, `write C:\…\src\app.rs`, not `write src/app.rs`, since the step
  writes where the path names; is the path from the top of the repository wanted in the log instead?
- **Q218** — **Open in external editor** refuses with the engine's own words when the command fails to start, and a
  command of `editor.external` without `${file}` gets the path put after it, in quotes when it has a space, and an
  empty one opens the file with the program the system has for it. `editor.external` is a setting of the user only,
  not of a repository, so that a repository cannot name a command to run. Are these right?
- **Q219** — Undo takes typing back a word at a time: a space after a word, a new line, a paste, a cut and a move of
  the caret each end a step. Is that the grouping wanted?
- **Q220** — There is one File page for each file: opening it again shows the same page at the line asked, so two
  pages of one file, each with its own cursor, cannot be had; a file of the working tree and the same file at a
  revision are two pages. Is that right?
- **Q221** — **Go to line** with a number past the last line goes to the last line rather than refusing, and `0` to
  the first. Is that right?
- **Q222** — The submodules are read with the state of the repository (`repo.read-state` runs `submodule.read-list`
  beside its other reads), so that the operations, the Submodules list and **…** see them in their `Context`. Is a
  read of its own, after the state, wanted instead for repositories with many submodules?
- **Q223** — The offers made after a step fails live in the engine: a plan names them in `on_failure`
  (`FollowUp::AllowFileProtocol`, and `FollowUp::OfferWhenFailed` with the words git must have said and the plan
  offered, which `submodule.deinitialize` uses for `--force`). The offer of `submodule.allow-file-protocol` runs the
  plan again from the step that failed, not from its first step, since the steps before it are done; the card says so
  now. Is this right?
- **Q224** — A `delete` step whose path is already gone counts as done, so that `submodule.remove` does not fail when
  git removed the clone itself. A deletion that fails for another reason still stops the plan, where the card of
  `submodule.remove` says it is only logged. Should the step of that card be allowed to fail?
- **Q225** — A folder with files in it is refused by `submodule.add` once it is chosen, with **`<path>` is not
  empty.**, rather than while it is typed, since the check of a typed answer cannot read the disk. Is this right?
- **Q226** — `submodule.open-in-terminal` and `submodule.reveal` run one `run` step in the submodule's folder (the
  cards said no steps; they now give the commands), and the terminal is the system's: `terminal.external` is not read
  yet. Should it come with the worktree operations that open a terminal too?
- **Q227** — `submodule.open-in-new-window` ends with `FollowUp::OpenWindow`, which the window carries out by starting
  `gitardo <folder>` again (`Effect::OpenWindow`). The operations of `worktree.` that open a window may want the same
  follow-up; should they share it?
- **Q228** — `submodule.show-in-gitardo` stays Specified: it needs the banner of `worktree.banner` to show which
  folder the window shows and to go back. Its entry is in the menu of a submodule, greyed out. Is this right?
- **Q229** — On Windows, `submodule.deinitialize` and `submodule.remove` do not yet run a command again when the folder
  is held, as their cards say: that retry comes with `worktree.recover-held-folder`. Is this right?
- **Q230** — `submodule.update-on-checkout` is used by `branch.checkout`, its remote and detached forms, the checkout
  and rebase of a remote branch and `tag.checkout`; not yet by `branch.create` when it checks out, `stash.branch` and
  `undo.last` of a checkout, which its card lists. Should those come in the same milestone?
- **Q231** — `submodule.update-to-recorded` asks first when the submodule has files not committed too, not only when
  it has commits on a detached HEAD, since git may stop on them. Is this right?
- **Q232** — The inputs of `update_on_checkout` (`CheckoutUpdate`: the checkout's plan, the setting, whether there are
  submodules on either side, the folder and the submodules) and of `allow_file_protocol` (`FailedPlan`: the plan, the
  step that failed and what git said) are structs of their own; the cards name them `input`. Is this right?
- **Q233** — `submodule.section` is drawn by `views::refs::submodules::section`, one row of the rail at a time, as the
  Stashes list is, so that the filter box and the rows in sight work for it too; its card names that function now.
  A submodule is listed by its folder below the one that has it (`b` under `vendor/a`), and the filter matches its
  whole folder. A pencil icon is added to the toolkit for the files not committed. Is this right?
- **Q234** — The marks of `submodule.status` are drawn as the number then its arrow, `2↑ 1↓`, as its card writes them,
  where the branches of the rail write `↑1 ↓2`. Should they agree, and which way?
- **Q235** — The review of the conflicts opens from a file in conflict on the Changes page by a new icon, a wand
  (`Icon::Wand`), drawn only while `conflicts.autoResolve` is on, with *Resolve its conflicts automatically, after
  looking at what is proposed* as its tooltip; the sparkle stays the AI's. Is the wand right, and should it be there
  with `conflicts.autoResolve` off too?
- **Q236** — On the Merge page a click on a line of any column puts the cursor on it, marked with the color of a
  selection; `F4` with no cursor takes the first line of the region gone to (`F7`), or the first line. Taking a side
  does not make its region the one gone to. Is that right?
- **Q237** — The function of `merge-view.close` is `app::store::merge_page::lines::closed`, in the store, not
  `views::merge_view::close_view`: a page is closed by the store when it leaves the open pages, and nothing is drawn
  for it. A page closed by **Mark as resolved**, or because its file is no longer in conflict, hands nothing over. Is
  that right?
- **Q238** — With choices not saved, `F4`, **Jump to source** and closing the page give the File page the result as
  one edit not saved, which `Ctrl+Z` there takes back; the Merge page, when it stays open, keeps its own text, so the
  two hold copies that do not follow each other until one is saved and the other reads the disk again. Should the two
  pages share one text instead (Q194)?
- **Q239** — `changes.save` writes every file given without reading it first, as its card asks nothing: **Save all**
  writes over a file another program changed since its page read it, where **Save** on a row and `Ctrl+S`
  (`file-view.save`) ask **Overwrite** first. Should `changes.save` read the files and ask the same, which needs a
  query that reads several files at once?
- **Q240** — `patch.am-stopped` is the banner of every operation that stopped (`conflicts.banner`), drawn through
  `views::patch::am_stopped` while `git am` is under way; it adds nothing of its own, since what the card asks of
  the banner, the commands and the names of the sides were already there. Its signature takes `&BannerState`, not an
  `OperationState`, which does not exist. Should the banner of a patch say more, such as the subject of the commit
  that stopped?
- **Q241** — `compare.path-with` is still not built: its entry points are in the Files page, which does not exist
  yet. Is it right to leave it for the Files page?

From milestone M8, the settings of new worktrees (`worktree-setup.`), each built with the reading given until it is
answered:

- **Q242** — The operations that write the settings of new worktrees (`worktree-setup.save`, `remove-from`,
  `link-into-new-worktrees`…) edit the files in place, keeping their comments, which only `crates/json` can do. So
  `features` may now use `json`, a pure crate, and the picture of doc 02 says so. This also answers Q44 for these
  settings: the operation writes the file, through a `write` step. Is the new edge right?
- **Q243** — `worktree-setup.link-and-copy-steps`: the engine makes a link on Windows as a junction itself
  (`platform::link_folder`, through `DeviceIoControl`), so `engine` now uses `platform`, as the picture allows. The
  log names the path from the top of the worktree, `symlink node_modules` and `copy .env`, not the full paths. What
  is in the worktree already is said as **`<path>` is in the worktree already.** by the engine when it finds it on disk,
  and by a warning of the plan for what the worktree's commit has; what would be made under a link or a file of the
  worktree is said as **`<path>` is not made: `<folder>` in the worktree is a link or a file.**, and a path to copy
  that holds the worktree as **`<path>` holds the worktree, so it is not copied.**, two sentences no card gives. A link
  met inside a folder copied is made again as a link to the same place, a junction when it is a link to a folder. Are
  these right?
- **Q244** — `worktree-setup.explain-change`: the card says that only **will be** and **still** write the settings;
  **New worktrees will no longer get `<name>`.** writes them too, since the pattern of its path is taken out. Is that
  the reading?
- **Q245** — `worktree-setup.propose`: the folder `.yarn/cache` is a dependency chosen by its path, `/.yarn/cache/`,
  since a pattern with a `/` in its middle is from the top in `.gitignore` syntax anyway. Is that right?
- **Q246** — The four entries of the Files page (`worktree-setup.link-into-new-worktrees`, `copy-into-new-worktrees`,
  `stop-linking`, `stop-copying`) say what changed nothing as a refusal, which the window shows as a warning, and what
  changed in a `FollowUp::Say` with **Saved in the settings of the repository** (or **of the user**), then offer
  **Choose What New Worktrees Get…**. They read git through two new queries, `Query::PathStatuses` (whether git tracks
  or ignores each path, and whether it is a folder) and `Query::SetupTakes` (what the patterns take, for the checkout
  or for a worktree's commit). A path chosen outside the repository is refused with **`<path>` is in no repository of
  the workspace.**; picking the repository of each path is the Files page's, which M7 builds. Is that right?
- **Q247** — The page and the Files page pass what they save through two new targets, `Target::SetupSettings` (the
  place, the patterns, the commands and the box of VS Code's settings) and `Target::Worktree(folder)` for
  `worktree-setup.bring-into-worktree`. The operations find the user's settings file in
  `FeatureSettings::user_settings_file`, which the window fills and `gitardo-cli` leaves empty, so that a test never
  writes the settings of the person running it. Is that right?
- **Q248** — `worktree-setup.bring-into-worktree` and `bring-into-other-worktrees` confirm with the note of the links
  and copies, **Gitardo will link the folders matching …, from `<folder>`, where git ignores them.**; the first is
  titled **Bring ignored files into `<name>`**, a title the card does not give. What failed while the others went on
  is said once at the end, after what the plan says, in the sentence the window shows when the plan ran, not as an
  error, since the rest was made. Are these right?
- **Q249** — The features crate may now use `json`: the switches of the blame (`blame.toggle-current-line`,
  `blame.ignore-whitespace`, `blame.every-file`) read the user's settings file and write it back with one key changed,
  keeping its comments, as a `Write` step of their plan. The edge is drawn in `docs/02-architecture.md` and in the rules
  of `cargo xtask check`. Is that right, rather than a step of the engine that sets a setting?
- **Q250** — A switch of the blame writes the user's settings file through the engine, like any step, and the window
  reads the settings again when the watcher sees the file change; with no user's settings file (no folder of the
  configuration), the switch refuses. Is that right?
- **Q251** — The Files page has no card, and none of its entry points is named: it opens from **Show files** in Quick
  search only. Should it have a card, and a place in the window of its own, such as a button of the header or of the
  rail?
- **Q252** — The Files page lists the files of the repository shown (`git ls-files --cached --others
  --exclude-standard`), without the files of its submodules and with no switch to another repository of the workspace,
  so the third acceptance line of `compare.path-with` (a file of another repository switches the window to it first) is
  not built, and the card stays **Specified**. Is listing only the repository shown right, and how should a file of
  another repository be reached?
- **Q253** — The blame of the line of the cursor waits 200 ms, and the blame of a file 400 ms after the last edit,
  inside the job on the worker, which sleeps in steps and stops when it is cancelled; the store cancels the job when
  what it was read with changes. Is waiting on the worker, rather than with a timer of the window, right?
- **Q254** — The editor has no word wrap yet (Q213), so the fourth acceptance line of `blame.file` (the column stays
  when lines wrap) cannot be built, and `blame.file` stays **Specified** until it is.
- **Q255** — The icons have no person, no closed eye and no pin, so the buttons of the blame in the title of the File
  page are the eye (**Show file blame**), the cross (**Hide file blame**), the funnel and the star; a button on is drawn
  as a secondary button rather than filled. Should the icons the card names be drawn?
- **Q256** — The themes have no colors for the highlight of the blame, so the lines of the commit of the cursor are on
  the color of an inactive selection with the color of the text, and the marks of the scroll bar are in the color of the
  text. Should the themes get colors of their own for it?
- **Q257** — The dates of the blame, in the column and in how long ago, are said in UTC, since the window does not know
  the offset of the time zone of the person yet. Is that right until it does?
- **Q258** — `FollowUp::SelectCommit` now also shows the timeline when a page is in front, so that **Reveal commit of
  current line in the timeline** and the link of a hover, run from the File page, show the commit selected. Is that
  right for every operation that selects a commit?
- **Q259** — The hover of the blame at the end of the line of the cursor has only **Show `<short hash>` in the
  timeline**, as its card says, and the hover of the column has the three links; **History of the active file** in the
  **…** menu of the header is not built, since the menu has no place for the File page yet. Are these right?
- **Q260** — The clock beside a file of the inspector runs `file-history.file` on the file of that path in the working
  tree, also when the commit of the inspector renamed it or it is gone from the working tree. Is running it on the path
  the commit has right?
- **Q261** — `worktree.open-in-editor`, `worktree.open-in-terminal` and `worktree.reveal` run one step,
  `run <command>` in the worktree's folder, where their cards said no steps, so that the program is started by the
  engine and its command is in the log like the others; their cards say so now. Is this right?
- **Q262** — `worktree.open-in-new-window` gives `FollowUp::OpenWindow`, the follow-up the submodules made (Q227), and
  the window starts `gitardo <folder>`. A window that cannot be started says nothing, as for a submodule. Its acceptance,
  a second window on the worktree with this one left as it was, is checked up to `Effect::OpenWindow`, since the tests
  open no window. Is this enough?
- **Q263** — `editor.external` opens a folder by putting it in place of `${folder}` when the command has it, and
  otherwise after the program of the command, its `${file}` and `${line}` left out. The card of the setting names only
  `${file}` and `${line}`. Is this right?
- **Q264** — What the operations of worktrees read is in `FeatureSettings::worktrees`: `worktrees.folder`,
  `worktrees.openIn`, `terminal.external`, the home folder for a `~`, the folder of the window while another worktree is
  shown, and whether **Choose What It Gets…** was offered. Reading the settings reads no environment, so the window and
  `gitardo-cli` set the home folder when an operation starts. Whether **Choose What It Gets…** was offered is kept for
  the window only, not stored. Is this right?
- **Q265** — The operations that make a worktree call `features::worktree::setup_steps`, which hands over to
  `features::worktree_setup::new_worktree_steps` and appends its steps after `git worktree add`; what it says goes in a
  `FollowUp::Say` after **Worktree created in …**. **Choose What It Gets…** is offered while that gives neither steps nor
  a note, which is when no pattern and no command is in effect. The button runs `worktree-setup.page`, which is still
  Specified: until it is built the button refuses. Is this right?
- **Q266** — Offers with a button are a follow-up of their own, `FollowUp::OfferRun`, with a target and answers:
  **Open in new window**, **Open in external editor** and **Open in terminal** after a worktree is made, and **Remove
  anyway** after git refused to remove one, kept from the follow-ups of the failure (`Plan::on_failure`, which the
  engine now hands to the window in `StepFailure::on_failure`) when git's words have one of its own. Main-desktop has
  `FollowUp::Offer` (an operation at `HEAD`, no answers) and `FollowUp::OfferWhenFailed` (a plan offered in place of the
  one that failed); **Remove anyway** could be the second. Should the three become one?
- **Q267** — `worktree.remove` deletes the branch last, after the empty folders, where the card put `git branch -d`
  before them, so that a branch git keeps, not merged, leaves nothing else undone; the card says so now. Is this right?
- **Q268** — `worktree.remove` and `worktree.rename` refuse a missing worktree too, with the sentence of
  `worktree.open-in-worktree`, **… is missing from …: prune the worktrees …**, since git can neither remove nor move a
  folder that is gone. Is this right?
- **Q269** — The folders `worktree.rename` needs for the new place are made by a new step, `Step::MakeFolder`, and the
  empty folders are deleted by `Step::DeleteEmptyFolder`, which leaves a folder that has something in it, so that
  `worktree.remove-empty-folders` gives its steps before the move or the removal has run. Two new queries read the disk:
  `Query::PathExists` (a file, a folder or a link at a path, for the folder of a new worktree, where
  `Query::HasFiles` of the submodules looks only for something in a folder) and `Query::FolderLinks`. Is this right?
- **Q270** — `model::worktree::branch_in_another_worktree` gives an `Option<WorktreeBranch>`, none for a branch that
  no other worktree has, where the card gave a `WorktreeBranch`; the card says so now. In the branch rail the worktree's
  icon is at the end of the row of such a branch, with the count of its files not committed, rather than in place of the
  branch's icon. Is this right?
- **Q271** — The Worktrees list is drawn by the rows of the branch rail (`views::refs::entry::worktree_row`), with its
  words from `views::worktree`, not by a `views::worktree::section` of its own; the card says so now. The title of the
  list has **…** and no **+**, since the titles of the lists kept aside have no buttons of their own. **Choose what new
  worktrees get…** and **Bring ignored files into the other worktrees…** in that **…**, and **Bring ignored files…** in
  the menu of a worktree, are left to the setup of new worktrees. Is this right?
- **Q272** — The label of a worktree is worked out by the views from `worktrees.folder`; when it starts with `~`, the
  views, which do not know the home folder, name the worktrees by their folders. Is this good enough?
- **Q273** — **Back to `<name>`** of the banner asks `Intent::SelectRepository` with the folder of the window, the
  intent of the list of repositories, which also forgets that another folder was shown. The banner shows a submodule
  too (`submodule.show-in-gitardo`, built now, Q228): **Showing the submodule `lib` on `main`**, named by its path from
  the folder of the window. A folder shown is taken for a submodule when the worktrees it lists do not have the folder
  of the window. Is this right?
- **Q274** — `worktree.create-at-commit` lists, for `${base}`, the branches at the commit and the short hash, not yet
  the other branches that have the commit, which needs a read of `git branch --contains`. Should that read come in M9?
- **Q275** — The date of `${date}` in the names of new worktrees is the day in UTC, as for new branches; a format with
  `${input:…}` is not used until `branch.name-from-format` asks for them. Is this right?
- **Q276** — `worktree.switch` stays Specified: the question that picks is a list with no box to type in, and its card
  says typing matches the label, the description and the folder. Its description says `<n> not committed` in words where
  the card has a pencil, since a choice has no icons. Should the pick of Quick search, which has a box, be used? (Built
  since with a question of its own kind, Q291.)
- **Q277** — `Message::SettingsRead` holds its `Settings` in a `Box`, since the settings of worktrees made it the
  largest message. Is this right?
- **Q278** — The card of `github.read-pull-requests` named `net::github::…`, a crate that does not exist; the read is
  `engine::github::read_pull_requests`, beside the jobs that run it, with the engine now allowed to use `json` and
  `http`; the card says so now. Should the requests to web services have a crate of their own, `net`, between the engine
  and `http`?
- **Q279** — `github.token` reads the credential store through the engine (`engine::SecretStore`, the store of Windows
  by default), which hands what it found to `git::read::github::read_token` in `TokenRequest::kept`, since `git` may not
  use `platform`. The token is asked with `Query::GitHubToken`, given back as `Answer::Token`, and kept by the follow-up
  `FollowUp::KeepGitHubToken` once the plan has run. Is this right?
- **Q280** — The certificates of the tests are in `crates/tls/fixtures`, and the TLS and HTTP test servers are in
  `tls::test_server` and `http::test_server`, behind a feature `test-server` that only tests turn on, where
  [testing](desktop/06-testing.md) named `testkit/certificates/`. Is this right?
- **Q281** — A question of kind `QuestionKind::Secret` is shown like a text question by the views for now: what is
  typed is not hidden until the input box has a mode that hides it. Should hiding come with the input box of M9 or later?
- **Q282** — `rustls` 0.23.46 needs `rustls-pki-types` and `rustls-webpki`; their newest versions were less than two
  weeks old, so `Cargo.lock` holds 1.15.1 and 0.103.16. Is pinning them in the lock, not in a `Cargo.toml`, enough?
- **Q283** — Outside Windows the credential store says it is not supported, so a token is asked again on each start,
  until M10 and M11 bring the stores of macOS and Linux. Is this right?
- **Q284** — `http` goes through a proxy only by `CONNECT`, for `https` addresses, and refuses a proxy whose address is
  `https`; the proxy is `HTTPS_PROXY`, else `http.proxy` of git, with the hosts of `NO_PROXY` reached directly, as
  [the platform](desktop/05-platform.md) says. Is that enough?

From the cards left over up to M8 (each built with the reading given, until the card says otherwise):

- **Q285** — `graph.show-in-graph` is the function every way to a commit already called, `app::store::search::go_to`,
  and its card names it now, since what it does is a change of the store and draws nothing of its own. The intent
  `GraphIntent::GoTo` goes through it too, so that **Show in the timeline** and a click on a tag, a stash or a
  worktree in the branch rail, the parents in the inspector, **Show** of the bisect and a side of a comparison now load
  the history up to the commit and take away what narrows the timeline, where they only selected a commit loaded. A
  stash not shown in the timeline is then shown with its history alone. The bar of the history of a file or a folder
  says the branches shown after `· in`, dimmed, as `filter.bar` says; the history of some lines does not. Are these
  right?
- **Q286** — `graph.toolbar` in a narrow window: **Sync** keeps the counts beside the icon of **Refresh** and gives up
  its word (`↓3  ↑2`), the branch filter is its icon and the branch checked out its name without the distance; the
  name of the repository stays in its pill, being a name rather than words. Is that the narrow header meant?
- **Q287** — `graph.date-separators` is built on its derivation, which counts the days in the local time it is given,
  and on the switch that turns it off. The window still gives it an offset of 0 from UTC, until Q58 says how the local
  time zone is read. Is it right to call the card built?
- **Q288** — `diff.open-file` is the title button of a page of one file whose right side is the working tree, between
  **Discard changes** and the fold button; a page of several files has no title button for each file yet, as it has
  none of `diff.stage-file`. Should the files of a page of several have a bar of their own with these buttons?
- **Q289** — `commit.create` is built: the commit box empty after a squash merge holds what git prepared in
  `SQUASH_MSG`, and **Commit** makes the commit with it. `preview.confirm` is built: with `commandPreview` `off` a
  branch is made at once and a hard reset shows its command and its warning first. `message.edit` is built with the
  question dialog of the window in place of a box in the inspector (Q159): `Escape` in it runs nothing, and while a
  bisect is under way **Edit message…** is not in the menu of a commit and the operation says *End the bisect first.*
  The pencil of the inspector is not drawn, as the inspector has no box to edit a message in. Should the box come with
  the multi-line editor, or is the dialog the box?
- **Q290** — `worktree-setup.edit-settings-file` is built, and **Edit in settings.json** of the settings page runs it
  for the file chosen there, since `worktree-setup.page` is not built yet. It ends with a new follow-up,
  `FollowUp::EditFile(path, line)`, which opens a File page to edit: the repository's file by its path from the top of
  the working tree, the user's file by its whole path, which the File page reads and saves as it is and leaves out of
  the Changes page. From the settings page it too opens at `worktrees.linkFolders` when that is set. The plan is titled
  *Edit `<path>`* and has no steps unless it makes the repository's file. Are these right?
- **Q291** — `worktree.switch` asks with a new kind of question, `QuestionKind::Search`: the dialog of the window
  draws a box over the radio buttons, and typing keeps the choices whose label, description or identifier holds what is
  typed, whatever the case; the identifier of a worktree is its folder, which its description also shows. The dialog
  keeps the height of every choice while typing, and answers with the choice picked while it is found, or else with the
  first one found, so `Enter` after typing opens it. The command line answers it as a pick. The choices still have no
  icons, and no pull request is said until M9 reads them. Is a kind of question of its own right, rather than the pick
  of Quick search?
- **Q292** — `settings.theme` is an operation now: the picker of the themes runs it with the theme picked, and it
  writes `theme` in the user's settings file with the rest of the file kept, logged in the Command log as every write;
  with no theme given, as from the command line, it asks among the five. Its title is *Use the theme `<name>`*, and
  Undo says to choose again. The picker still shows no theme while the user moves through it (Q107), since the
  picker of the toolkit says nothing until a choice is made. Should it say which item is under the keyboard?
- **Q293** — `conflicts.continue-operation` of a cherry-pick or a patch reads the newest entry of the journal first;
  when it is the series a cherry-pick or a patch of Gitardo left as it stopped, the plan that goes on leaves the same
  series, so that one Undo resets with `--keep` to where `HEAD` was before the first commit. Continued from a terminal,
  `HEAD` moved without Gitardo, the entry is dropped and Undo takes back the last commit by the reflog. With these,
  `selection.cherry-pick` and `undo.last` are built; the line of `undo.last` on a terminal's operation after one of
  Gitardo's is tested with a commit made in a terminal after a cherry-pick. Right?

- **Q294** — The card of `ai.ask-conflict` named `ai::read::ask_conflict`, a crate that does not exist; the question is
  `engine::ai::ask_conflict(sending, provider, &request, pieces)`, beside the GitHub reads, since it goes over `http`
  and its key comes from `engine::SecretStore`. It returns the answer with the model that wrote it
  (`ConflictReply`), and `pieces` is told each piece of text as it comes. The card says so now. Is this right?
- **Q295** — The address of the provider: for `anthropic`, `ai.endpoint` (default `https://api.anthropic.com`) is the
  base, followed by `/v1/messages` and `/v1/models`; for `openai-compatible`, `ai.endpoint` (default
  `https://api.openai.com/v1`) already holds `/v1`, followed by `/chat/completions` and `/models`, as servers of that
  kind are set up. Is this right?
- **Q296** — The requests carry `anthropic-version: 2026-10-21` and `max_tokens` 32000 for `anthropic`, with the effort
  as `output_config.effort`; for `openai-compatible` the effort is `reasoning_effort`, where `xhigh` and `max` are both
  sent as `xhigh`, the most such servers take. Is this right?
- **Q297** — The requests to the AI provider go through `HTTPS_PROXY` and `NO_PROXY` only: git's `http.proxy` is not
  read for them yet, since the engine has no read of git's configuration at hand there. Should it be?
- **Q298** — The key of the AI provider is kept in the credential store under `ai.<provider>` (`ai.anthropic`,
  `ai.openai-compatible`); `ai.choose-model` asks for it, as a secret, when the provider wants one and none is kept or
  the one kept is refused, and keeps it with `FollowUp::KeepAiKey` once the choice is written. The card does not say
  where the key comes from the first time. Is this right?
- **Q299** — `ai.choose-model` also refuses with no AI provider set up (*No AI provider is set up.* with **Open
  settings**), when the user's settings file is not JSON with comments, and with the error of the provider when it
  cannot list its models; the card names only the refusal with no model listed. Is this right?
- **Q300** — `FeatureSettings` holds the AI settings in a `Box`, since they made `Effect` too large for clippy. Is this
  right?
- **Q301** — The card of `ai.resolve-conflicts` named `views::ai::resolve_conflicts_view`; the view is
  `views::review::resolve_conflicts_view`, the row of the head of the review page that asks (**Ask the AI about the
  rest (N)** and the button with the model's name, or the bar with **Stop** while the AI is asked), beside the other
  views of the page. The questions are asked by `app::work::ai::Asking`, one thread per question, which takes the key
  from the credential store; the store keeps two under way. The card says so now. Is this right?
- **Q302** — The row that asks the AI is a row of its own in the head of the review page, over **Accept all proposed**
  and the others, and the bar *Asking the AI: 3 of 5 conflicts answered* takes its place while the AI is asked. Is
  that the place?
- **Q303** — The button with the model's name shows the identifier of `ai.model` (*claude-test · high effort*), and
  **First model of the provider** while it is empty, since the names the provider gives are known only once it is
  asked for its models. Should the page ask the provider for them when it opens?
- **Q304** — With no AI provider set up, the cards say *No AI provider is set up.* with **Ask again** at once, and no
  request is made. Is this right?
- **Q305** — Q187 is answered: **Resolve with AI** is enabled in the banner and in the message, and opens the review
  page and asks.
- **Q306** — `merge-view.accept-proposals` stays **Specified** for its second line: the Merge page computes its
  proposals on its own regions with the rules only, and joining the answers kept by `review.fingerprint` to them needs
  the review's reading of the conflicts in the read of the Merge page. Should that come with the next slice of the
  Merge page?
- **Q307** — The Activity drawer cannot cancel what it lists yet, so while the message is drafted the sparkle becomes
  a button (*Stop drafting the message: the box is left as it was*) that stops it, where the card has the sparkle
  disabled and the drawer cancelling. Should the drawer cancel, and the sparkle go back to disabled?
- **Q308** — The diff of `commit.draft-message` is read on the thread of the question, with git, before the request;
  a draft refused by the provider or failed is said in the message of the window, as the failure of an operation is.
  Is this right?
- **Q309** — Q281 is answered in this slice: an input box of the toolkit hides what is typed when its view asks
  (`InputBox::hidden`), drawing a dot for each character, telling the screen reader the dots, and copying nothing
  from it; a question of kind `QuestionKind::Secret` asks for such a box. Cutting from it still takes the text out of
  the box, without putting it on the clipboard. Is this right?
- **Q310** — "Every 120 s while a token is kept" is read as the card says it: after each read of the repository shown,
  when the last request is more than 120 seconds old, with no timer of its own. A window whose repository is not read
  for a while does not ask GitHub meanwhile. Is that enough, or should the pull requests have a wake of their own?
- **Q311** — The window looks for the repository on GitHub with a job of its own (`git remote`, `git remote get-url`)
  after a read whose remotes have other names, and after an operation that says `FollowUp::ReadPullRequests`, since
  `RepositoryState` keeps only the names of the remotes. Should the state read keep their URLs instead?
- **Q312** — The reads of the pull requests run on a thread of the window (`app::work::github`), not on the workers
  nor through the engine: they read the token from the credential store themselves, never ask git's credential helper,
  and take the token out of the store when GitHub answers 401. Is a thread of `app` changing the credential store
  right, or should that go through the engine?
- **Q313** — A 401 to a read made on its own is kept quiet, as the card says of every error of such a read, with a
  warning line in the Command log; the token is still taken out of the store, and the badges go away. Should the
  window say it, as it does after **Load pull requests from GitHub**?
- **Q314** — In the branch rail the badge of a pull request is drawn at the right of the row of a local branch and the
  pull request is said in its tooltip, but the badge is not a button of its own: a click on it is a click on the row,
  as before. The rows of the remote branches have no badge. Should the rail's badge open the pull request too?
- **Q315** — **Open pull request #n** and **Create pull request…** come after the merge and rebase entries of a local
  branch, before **Rename**; on a remote branch, before **Delete from remote**. A remote branch's link and badge do not
  check that its remote is the main remote, as the card of `github.open-pull-request` says nothing of it. Is that the
  place and the rule wanted?
- **Q316** — The rows of the Worktrees list (`worktree.section`) do not show the pull request of their branch yet: they
  are drawn by the views of `worktree.`, left as they are to keep this slice's edits of shared views small. Should it
  come in a slice of `worktree.`?
- **Q317** — `drag.drop` is built. A row of the timeline is dragged through its list: the list notes the row pressed
  and says once that a drag starts when the pointer has moved a few pixels, and the timeline then drags the branch or
  remote branch whose label the button was pressed on, or else the commit of the row; the draft, a stash and a folded
  row are not dragged. The branch rail drags its branches and remote branches, in every list, Favorites and Recent
  included; the pointer over them now shows that they can be dragged. What is dragged travels as words the toolkit
  carries (`commit <hash>`, `branch <name>`, `remote <name>`), and the store keeps what was dropped while its menu is
  open. A row that does not take what is dragged shows the pointer that forbids, a stash and a folded row included.
  The entries of the menu are the words of `drag.list-drop-entries` without the marks of code around the names
  (**Cherry-pick 3f2a1c9 onto main**), as the other menus say names; `rebase.branch-onto` runs on the commit with the
  answer `branch` given, since `views` does not use `features` for its presets. Pressing a row to drag it selects the
  row, as a click does. Are these right?
- **Q318** — `changes.pane` and `changes.discard-all` are built on what the pages of other milestones now give: a
  click on the name of a file in conflict opens the Merge page (`conflicts.open-file`), **Save all** is on the header
  while a File page has edits not saved, and **Discard all changes** of the header names only what git sees changed,
  so a file that only a File page changed keeps its edits and its file on disk. The lines of Q122 that waited on them
  are answered. Should **Discard all changes** also say, in its warning, that the edits not saved are kept?
- **Q319** — `log.record` is **Built**. Its fourth line is tested against test servers that refuse the secret and
  say it back: GitHub, for a token kept and one from git's credential helper, read by the window on its own and from
  **Load pull requests from GitHub**, and the AI provider, for a key, from the sparkle of the commit box. A refused
  read of GitHub is a warning line of the log (Q313); a refused request to the AI provider is said, not logged.
  Should a failed request to the AI provider be written to the Command log too?
- **Q320** — Word wrap in the File page (Q213, Q254) is switched by a new setting, `editor.wordWrap`, `off` by default
  as in VS Code, and with it `blame.file` is built (Q254). It has no button, key or entry of a menu yet; should it have
  one, such as `Alt+Z` or an entry of the context menu of the editor? Lines are wrapped in columns of the font of code
  (`editor::wrap_line`): after a space or a tab, which stay at the end of the row they close even past its width, and
  inside a word only when it is longer than a row; each row counts its tab stops from its own start. The rows of every
  line are counted again each frame, which costs the length of the text; should they be kept between frames once a
  file of a few megabytes is measured as too slow? The arrows up and down, `Home`, `End` and Page Up and Down still go
  by lines of the file, not by rows, and a click past the end of a row that is not the last of its line puts the
  cursor before the row's last character, so that it stays on that row. Should the arrows go by rows, as VS Code's do?
- **Q321** — `review.columns` is built (Q210): its lines wrap with the same `editor::wrap_line`, between words, the
  spaces at the end of a row left out, and each line of a column, and its number on its first row, is told to the
  screen reader as a label where it is drawn, which is also how its tests find them; the outline the pointer draws is
  tested by the pixels of the edges of the lines. Is a label for each line and each number right for a screen reader,
  or should a column be one text whose value is its lines?

From the slice of the setup page and of comparing a path, each built with the reading given until it is
answered:

- **Q322** — `details.compare-file-with` is built. The menus give the file and its list as a new target,
  `Target::ListedFile` (the files of a commit, named by its short hash, with its first parent; of a comparison, with
  its two sides; or the uncommitted changes), and the operation opens the Diff page through a new follow-up,
  `FollowUp::OpenFileDiff`, with the two sides and the path. The file is compared under its path on its own side on
  both sides, also when it was renamed. The picker's placeholder is **The branch, tag or commit to compare the file
  with**, which the card does not give; a file a root commit deleted, which has no version, is refused with
  **`<name>` has no version to compare here.** The sides of the comparison in the inspector are fixed at their
  commits, named as it names them. The untracked files of a stash are taken from its third parent. Are these right?
- **Q323** — `compare.path-with` is built (Q241, Q252): run on a file or a folder of another repository of the
  workspace, the window first shows that repository, as choosing it in the list of repositories does (the page shown is
  put away), and the operation runs once it is read, its choices those of that repository; the innermost repository
  holds a path inside two. Only this operation switches so; the Files page still lists only the repository shown, so
  the switch is reached from operations given such a path. Should the history of a file (`file-history.file`) switch
  in the same way?
- **Q324** — `worktree-setup.page` is built, as the page `Page::NewWorktrees`. It is not an operation of the
  catalog: the store opens it when an entry runs `worktree-setup.page` (the offer after the first worktree, the
  message of the Files page entries, the entry of the Worktrees list) and from Quick search. The sections opened or
  closed by hand are kept for each worktree while the window is open, not in `state.json`; changes not saved are
  dropped when the page is closed. The page reads the three settings files again each time the window reads its
  settings or the repository again, so a change of `.vscode/settings.json`, which is not watched, shows at the next
  read. **Save** compares only the settings file of the place chosen, not VS Code's. A line of the header with nothing
  set says **Linked now: nothing**, without **from nothing**. **Bring into the other worktrees** is shown disabled
  while there is no other worktree, and named so. The places of an unfolded name are all listed: only a section stops
  at 200 lines. Are these right?
- **Q325** — `worktree-setup.command-steps` is left Specified. Its plan steps are built and tested, but four of its
  five acceptance lines need **Tasks** in the Activity drawer, which Q100 left out: the engine runs a `Run` step with no
  input and drops what it prints, so a command cannot be watched or answered, and nothing yet waits for the last
  command before **Open in worktree**. Building it means a column of tasks in the drawer, a command run with its output
  kept and its input fed from the drawer, and the warning that offers to open the worktree after a failure. Should it be
  a slice of its own, with the drawer's Tasks column first?
- **Q326** — The card of `graph.more-menu` named **Browse reflog…** where the card of `reflog.browse` says
  **Reflog**; the menu keeps **Reflog**, and the card says so now, with **Settings** and **Theme…** as its seventh
  group, as the cards of `settings.edit` and `settings.theme` place them. **History of the active file** is always in
  the menu: it shows the history of the File page in front, and with none it runs `file-history.file` on nothing,
  which says *Open a file of this repository to see its history.* Should it be greyed out instead while no File page
  is in front?
- **Q327** — `compare.open-on-github` reads the remotes itself (`Query::Remotes`) for the repository on GitHub and the
  name of the main remote, which decides whether an upstream "is on the GitHub remote"; the button is shown from what
  the window knows of GitHub (`github.detect-remote`) for the repository of the page. Plans may now end in
  `FollowUp::OpenUrl`, which the window opens with `platform::open_url`. It refuses the working tree, the same commit
  and another host, which the card did not list. Is this right?
- **Q328** — Q306 is answered: the Merge page proposes the answers the review keeps for its file (`conflicts.ai` on),
  each matched to a conflict still marked by `review.fingerprint`, with `review.proposals` making the lines in the
  endings of the file and counting the new ones; the rules still come first. The answers are taken when the page
  reads its regions, so an answer that comes while the page is open shows at its next read. Should it show at once?
- **Q329** — Q316 is answered: a row of the Worktrees list draws the badge of the pull request of its branch at its
  right, after the pencil, and its tooltip adds the line of the badge; a missing worktree has none. The card gains an
  acceptance line for it. Is this right?
- **Q330** — `repo.open-folder`, `repo.open-recent` and `repo.select` are built as cards of the store, not features,
  as the lead decided for Q43: their Function lines name `app::store::open_folder`, `app::store::open_recent` and
  `app::store::workspace::select`, which reduce them, and their acceptance lines are tests of the store. The system's
  folder dialog is the platform's (`platform::FileDialog`), shown for `Effect::ChooseFolder`. Can the maintainer
  confirm that the workspace stays out of the feature contract?
- **Q331** — `refs.sort` is built on the lead's answer to Q9: with `branches.sortOrder` not set, the branches sort by
  name, unless Git's `branch.sort` of the repository shown is `-committerdate` or `committerdate`, which give the order
  by date (`model::refs::order_in_force`). `branch.sort` is read with the state of the repository, by
  `git config --get branch.sort` in a thread of its own (`RepositoryState::branch_sort`). `committerdate`, the oldest
  first for git, gives the order by date too, the latest first, as the lead said. Should other keys of `branch.sort`,
  such as `-creatordate`, give the order by date as well?
- **Q332** — `refs.filter` is built on the lead's answer to Q66: while something is typed in the branch rail, one list
  of the matches of every list (local, each remote, stashes, worktrees, tags, submodules) takes the place of the lists,
  the best first, under a title **MATCHES** with `<matches> of <all>`, a word the card did not give; **Favorites** and
  **Recent** give no entry, since their branches are among the local ones. Each entry keeps the row and the menu of its
  list. The branch filter (`filter.picker`) lists its matches the same way, without the headings **Local**,
  **Remotes** and **Tags**. Line 3 of `refs.filter` now says that a match of a closed list is shown while text is
  typed. In the narrow layout, where there is no box, nothing changed. Is this the list the maintainer meant?
- **Q333** — `branch.name-from-format` is built on the lead's answer to Q112. The `regex` crate checks `pattern`, in
  `model` (a new dependency there, `regex` already being allowed for `text`); a pattern that is not one accepts what
  is typed, and is refused before any question. A format with more than ten inputs is refused too, since the answers
  come back by the place of the input (`input1` to `input10`). The values given last are kept by workspace in the
  window's persisted state (`branchInputs`), given to the feature in `FeatureSettings::branch_values`, and remembered
  by `FollowUp::RememberInputs` only when the plan runs, not when the questions are answered and backed out of.
  `${date}` is filled with the local offset the window gives in `FeatureSettings::utc_offset`, read on Windows from
  the system and 0 (UTC) elsewhere, and in `gitardo-cli`. The Function line returns `Result<FormatName, Outcome>`,
  since the read of the branches at a commit and a refusal are not questions. `branch.create` asks **Use the format**
  first; `worktree.create` still uses only formats without `${input:…}`, in UTC. Right?
- **Q334** — `branch.pane` is built as `views::branch::pane(state: &ReferencesState, ui)`: the state of the rail is
  that of `refs.tree`, so no `BranchesState` was made. Its top, over the filter box, names the branch checked out with
  the icon of a branch, or `detached at <short hash>`, and under it has **Create branch** and **Compare**; at the width
  the rail starts with, **Create branch** is only `+`, with its name as the tooltip, so that both fit. Both are
  disabled before the first commit. The narrow layout, with only dots and icons, has no top. The header names the
  branch checked out too, so the top says it twice. The buttons of the top do not wait for an operation running, as
  the header's **Branch** does. Is this the top the maintainer meant?
- **Q335** — `remote.manage` and `remote.add` are built now that the pull requests are read in the window: the first
  pick of `remote.manage` is a search (`QuestionKind::Search`, Q291), which finds a remote by its name or its URL;
  the lines about GitHub are tested against the test server of `github.load-pull-requests`. Renaming `origin` while a
  fork is the other remote makes the fork the main remote until `origin` comes back, as `github.detect-remote` says,
  and the pull requests are read from each in turn. Is a search the pick the card meant?
- **Q336** — `diff.edit` is not the editor of code of the File page laid over the page: the rows of a Diff page have
  fillers and folds that a text has not. Each side is one widget over the rows the list drew, which places the caret,
  selects and takes the keys, mapped to commands by the same function as the editor of code (`ui::key_commands`); its
  function is `edit_view(sides, ui)`, `sides` being the rows drawn. The bars of the folds stay above it. The caret
  does not blink, and **Paste** in its menu is greyed as in the File page (Q214). Is one widget per side right?
- **Q337** — The text a Diff page edits is that of the File page of the file; with no File page open, a File page out
  of sight holds it, kept while it has edits not saved or a Diff page shows the file, and **Open file** shows it.
  Each Diff page keeps a caret of its own, which the edits made elsewhere move. Closing a Diff page with edits not saved
  asks nothing: the edits stay, and the Changes page shows the file with its dot. Should closing ask, as the File
  page does?
- **Q338** — Until the diff is computed again, 200 ms after the last key, the rows follow each edit at once: a line
  edited loses its marks, a line added takes the change of the rows around it when both are of the same change, and
  the buttons of the gutter are not drawn. A diff read while keys are still typed is dropped, and the next one is
  read. Right?
- **Q339** — **Revert this change** in the window does what its plan says without running `diff.revert-change`: the
  lines of the change are replaced in the text of the side as one edit, the file is saved with `file-view.save`
  (`write <file>`), and `Ctrl+Z` brings the change back unsaved. The operation stays for the command line, which has no
  text to edit, and the card keeps its function. When the text is not the one the page shows, it says *The file
  changed since the diff was shown. Try again.* Is it right to leave both?
- **Q340** — `F4` and **Jump to source** need a caret: on a page where no line was clicked yet, `F4` does nothing.
  Should it open at the current change, or at the first line?
- **Q341** — `changes.unsaved` gives the paths as strings from the top of the working tree with `/`, as the rest of
  the Changes page does, rather than a type of path of its own. A file typed in is listed once git has said whether it
  ignores it, which it is asked on a worker when a file becomes or stops being unsaved and when the repository is read
  again; when git cannot say, none is left out. The editable side of the Merge page is not one of the editors yet, as
  it keeps its own text. Right?
- **Q342** — **Open all changes in one Diff page** opens every file of the list between `HEAD` and the working tree,
  as Q138 offered, rather than each with the pair of versions **Open changes** gives it: a Diff page keeps one pair for
  all its files, and this pair shows every change of a file, what is staged drawn dimmed, and lets the files be edited
  (`diff.edit`). The card says so now. Is one pair right, or should a Diff page take a pair for each file?
- **Q343** — **Edit** on a card of the review opens a box under the choices: an editor of code with no gutter, 3 to 12
  lines high, with **Use these lines** and **Cancel**; `Ctrl+Enter` and `Escape` work while it has the keyboard. One
  box is open at a time, and it closes with the page. The lines used are kept, like the other choices, by file and
  fingerprint while the window is open, and a choice is accepted when it is made, not when the page is opened again:
  after closing and opening the page, the choice is proposed again but waits to be accepted. Should a choice stay
  accepted across closing the page?

From the slice of the Tasks of the Activity drawer, which answers Q325 and builds `worktree-setup.command-steps`:

- **Q344** — A command of a new worktree is a step of its own, `Step::Task { command, directory, name }`, logged
  `run <command>` as `Run` is. In the window the engine runs a plan up to its first task and hands that task and the
  steps after it back (`Finished::Ran::waiting`, `TaskSteps::HandedOver`); a thread of their own runs them one after
  the other (`engine::TaskRunner`), with what they print sent to the window every 50 ms at most and the lines typed in
  the drawer given to the task running. `gitardo-cli` still runs them in place, unseen. What the plan does once it
  succeeded (its offers, `OpenWindow`) is held by the store until the last task exits with 0; meanwhile only its
  sentence is said, and the drawer opens on the tasks. A step after the tasks that fails, such as the external
  editor of **Open in worktree**, is said as the failure of a plan, with its own follow-ups. Is this the split the
  card meant?
- **Q345** — The **Tasks** column: the words **TASKS**, **waits**, **did not run**, **exit code `<n>`**,
  **stopped**, **could not start**, **Nothing printed yet**, **Stop** (with the tooltip "Stop this command; the commands
  after it do not run") and the box **Answer `<command>`, then press Enter**, which no card gave. It shows the last four
  tasks, keeps the last five runs of commands and the last 5,000 lines of each task, leaves out the colors and the moves
  of a terminal, writes a line again after a carriage return, and shows a line typed as a terminal would, followed by a
  line feed (`\n` on every system) to the task. The drawer is 380 pixels high while it shows the tasks, against 280.
  Are these the column and the words?
- **Q346** — The warning of a command that failed says **so the command after it did not run.** when one did not run,
  and ends at the worktree when none did, where the card's sentence says **the `<k>` commands**; a shell that could not
  start says **`<command>` could not be started in `<worktree>`: `<what the system said>`**. Right?
- **Q347** — On Windows a command of the user's, a task or a `Run` step, now runs as `cmd /S /C "<command>"` given as
  it is, since the quoting of an argument put a backslash before each of its own quotes, which `cmd` keeps. **Stop**
  kills the shell only, as cancelling a plan does: what the shell started, such as `node`, may outlive it, until the
  platform can kill a tree of processes. Should that come with this?
- **Q348** — The commands run only in a trusted folder, and no folder can be trusted yet (`Trust::NotTrusted` is
  always given to the features): in the window they never run until trusted folders are built (roadmap M8). The tests
  set the trust themselves. Should trusted folders be the next slice?
- **Q349** — Each task is among the activities while it runs, titled `Running <command> in <worktree>` with **A
  command of a new worktree, in Tasks** as its detail, so that the header and **What is running** show it; the table of
  `activity.describe-task` gains that row. Is that wanted?
From milestone M10 (Linux):

- **Q350** — The Linux part of `platform` declares the few functions of the C library it uses (`inotify_init1`,
  `inotify_add_watch`, `inotify_rm_watch`, `poll`) by hand in `crates/platform/src/linux/libc.rs`, as the Windows part
  does for its libraries, with the values of the flags of `x86_64`, `aarch64` and `riscv64`, instead of taking the
  `libc` crate. Is that right, or should `libc` be allowed for `platform`?
- **Q351** — Watching files: inotify watches one folder at a time, so the watcher adds a watch for every folder under
  the one watched when it opens (on the thread that asks, before the watcher's thread starts, so that nothing is
  missed), and one for each folder created or moved in, whose contents are then said as changed. Links to folders are
  not followed. A folder beyond `fs.inotify.max_user_watches` is left unwatched without a word, and after an overflow
  every folder is asked for again. Should the window say when folders could not be watched, or watch fewer folders
  (not `.git/objects`, not ignored ones)?
- **Q352** — High contrast is read with `gsettings get org.gnome.desktop.a11y.interface high-contrast` each time a
  window gets the keyboard back, as on Windows: a process started on the UI thread at each focus. The time of a double
  click is read once from `org.gnome.desktop.peripherals.mouse double-click` (400 ms without `gsettings`), its distance
  the usual 4 pixels. Is a process at each focus acceptable, or should the reading move off the UI thread?
- **Q353** — Credentials, until a Secret Service library is chosen: the file `credentials` in
  `$XDG_CONFIG_HOME/gitardo-desktop` (or `~/.config/gitardo-desktop`), mode `0600`, its folder made `0700`, one line
  per secret (name, tab, secret, with `\\`, `\t`, `\n`, `\r` escaped), written whole beside it and renamed into place.
  `platform` finds the folder itself, by the same rule as `app::settings::configuration_folder`, since it cannot use
  `app`. The settings page and **Choose a model** still say "the credential store of the system": the texts of
  `views` and `features` were not changed in this slice. Should they say, on Linux, that the secret is in a file, and
  should `app` give the folder to `platform` instead?
- **Q354** — Opening an address runs `xdg-open`, with a thread that waits for it so that no ended process stays
  behind. Revealing a folder, the terminal and the external editor are unchanged above `platform`: `xdg-open
  <folder>`, `x-terminal-emulator` (Debian and Ubuntu only) unless `terminal.external` is set, and `editor.external`.
  Should the default terminal try others (`gnome-terminal`, `konsole`, `xterm`) when `x-terminal-emulator` is not
  there?
- **Q355** — Clipboard, dialogs and fonts stay with the libraries, as built: `arboard` over X11, and on Wayland through
  XWayland, since its Wayland part is not built; `rfd` through the desktop's portal, falling back to `zenity`; the
  fonts of the system for the fallback read by `fontdb` from the usual folders, without the configuration of
  fontconfig. Is that enough for a first Linux release?
- **Q356** — Killing a process tree on cancel is not built on either system: `git` and the commands of the user are
  killed alone (`Child::kill`). On Linux the simplest way is a process group of its own for each command
  (`CommandExt::process_group(0)`) and a signal to the group, in `engine` and `git`. Should it come in a slice of its
  own, with job objects on Windows?
- **Q357** — The clippy of Rust 1.100, on Linux, asks for `map_or_default` where the code wrote
  `.map(…).unwrap_or_default()` (the pedantic `map_unwrap_or`), which the clippy of 1.99 on Windows does not; every
  such place was changed, in a commit of its own, and 1.99 accepts the change. The workspace now names its toolchain,
  1.99.1 with rustfmt and clippy, in `rust-toolchain.toml`, so that every system runs the same clippy; moving to a
  newer Rust is then a commit of its own, with what its clippy asks. Is that right?
- **Q358** — Tests that assumed Windows: the path of the temporary patch of `patch.apply` is quoted only on Windows,
  so its test finds it quoted or bare; the helper of the folder-links tests of `git` is for Windows only, as those
  tests are; the `pre-push` hook of an activity test is made executable, without which git on Linux does not run it.
  The tests of `platform` that open real windows, use the clipboard or watch folders, and the test of the demo window,
  run on Linux too, and return at once when there is no display (`DISPLAY`, `WAYLAND_DISPLAY`); on X11 a window never
  shown does not say the place it was given, so only Windows checks it. The tests of the window (`app/tests/places`)
  run git with the person's configuration, not the isolated one of `testkit`: on a machine whose git has no identity
  (as a fresh WSL) about twenty of them fail on "Committer identity unknown"; they pass with one. Should those tests
  isolate git as `testkit` does?
- **Q359** — Two snapshots differed on Linux by two pixels each, `commit-page-dark` and `pull-request-badge-dark`, at
  the right end of a pill whose corners are half its height. The cause: `tiny-skia`, when it clips a cubic curve at
  the edge of the mask it fills (a band of `fill_rounded_rect` that ends inside the curve of a pill, or the clip of a
  view that cuts one), finds where the curve crosses with `cos` and `powf`, which come from the system's C library and
  differ in the last bit between Windows's and glibc (checked: `mul_add` and `sqrt` agree, `cos`, `sin`, `powf` and
  `ln` do not). `render` now never lets the library clip a curve: the area a mask is rasterized over grows until no
  curve crosses its edges, and only the pixels wanted are kept. The pictures are then the same on both systems, and
  nineteen snapshots changed by a few anti-aliased pixels at rounded corners (at most 85 pixels, at most 16 levels of
  a channel), re-recorded once and looked at: `commit-page-dark`, `commit-page-light`, `commit-page-head-light`,
  `pull-request-badge-dark`, `gallery-light`, `gallery-dark`, `gallery-high-contrast-light`,
  `gallery-high-contrast-dark`, `gallery-light-2x`, `gallery-dark-2x`, `gallery-over-light`, `activity-dark-2x`,
  `confirmation-dark-2x`, `dialog-dark-2x`, `drag-dark-2x`, `find-dark-2x`, `menu-dark-2x`, `picker-dark-2x`,
  `pull-dark-2x`. Is growing the area right, rather than turning the curves into lines in `render` before filling?
- **Q360** — `platform::utc_offset`, which came with `branch.name-from-format` while this slice was built, has a
  Windows part only: on Linux `${date}` in the names of new branches is counted in UTC, as Q333 says for every other
  system. Answered by the lead: the Linux part is built so, declaring `localtime_r` of the C library and reading
  `tm_gmtoff`, the same on glibc and musl; macOS still counts in UTC.

From the slice of the trusted folders, which answers Q348 and builds `trust.is-trusted`, `trust.ask` and
`trust.manage`:

- **Q361** — Trusting a folder trusts every folder in it, and the folder checked is that of the repository the
  operation runs in, as the window shows it. A worktree in `${repo}.worktrees` is beside the repository, not in it, so
  a window that shows it is asked about it on its own, although it shares the repository's git folder. Its checkout
  may hold another `.gitardo/settings.json`, from a branch someone else wrote, which is why it was left so. Should a
  worktree be trusted with the main worktree of its repository?
- **Q362** — The commands of the user's own settings file are held back in a folder not trusted too, as
  `worktree-setup.command-steps` said before (**in a trusted folder only**); the question then says they come from
  **your settings**. The folder does not ask for those: should the user's own commands run without asking?
- **Q363** — What else a folder asks for: among the settings a repository's `.gitardo/settings.json` may set, only
  `worktrees.setupCommands` runs a program, and it is the only one held back. Others act on what the repository says
  without running anything of its own: `worktrees.folder` (where new worktrees go, possibly outside the repository),
  `worktrees.linkFolders` and `worktrees.copyFiles` (links and copies of what git ignores, into the new worktree only),
  and `submodules.updateOnCheckout` (git fetches the submodules from the URLs of `.gitmodules`, as `git clone
  --recurse-submodules` would). `editor.external` and `terminal.external` run programs but are read from the user's file
  only, and git's hooks are git's (a clone gets none, and `core.hooksPath` is not cloned). The rest (`readOnStartup`,
  `graph.pageSize`, `branches.format`, `branches.inputs`, `pull.rebase`, `pull.autostash`, `cherryPick.recordOrigin`)
  run nothing. Should `worktrees.folder` from the repository's file be held to a folder beside the repository, or
  `submodules.updateOnCheckout` from it be asked, until the folder is trusted?
- **Q364** — The answer is kept in `state.json` under `trustedFolders`, a list of the folders as the system writes them,
  never cut as the recent ones are (this branch has no `11-data.md`: the module comment of `app::persisted` names the
  key, and the cards say it). The folder is trusted by a follow-up, `FollowUp::TrustFolder`, among those of the plan on
  success and on failure alike, which the window carries out as soon as the operation ends, before what follows the
  plan waits for its tasks; **Trusted folders…** takes folders out with `FollowUp::ForgetTrustedFolders`. A plan shown
  by `commandPreview` and not run trusts nothing, nor does backing out of a later question. Should the folder be
  trusted as soon as the question is answered instead?
- **Q365** — `trust.ask` is a function of `features` that the operations making worktrees call through
  `worktree-setup.command-steps`, `ask(context, answers, asked) -> Result<Trust, Box<Outcome>>`, not an operation of its
  own: its card says **Operation** with a note, and it is not among `OPERATIONS`. Is that the kind it should have?
- **Q366** — **Trusted folders…** is in Quick search only: a pick of the folders trusted, each ticked, in the order they
  were trusted; with none unticked it says **The trusted folders are unchanged.** The Settings page, which lists the
  settings of the files, has no section for them, and no command trusts a folder before something would run in it.
  Should the Settings page list them, and should **Trust this folder** be offered in Quick search?
- **Q367** — Each window reads `state.json` when it starts and writes it whole, as for the other keys: a folder trusted
  in one window is not trusted in another window already open until that one starts again, and the window that writes
  last wins. Is that enough?
- **Q368** — `gitardo-cli` is given no trust, so in a folder with commands it asks **Run the commands for new
  worktrees?**, answered with `trust-folder=trust` or `trust-folder=not-now` like any question, and prints
  `Trust <folder>` for the follow-up without keeping it. Right?

From milestone M11 (macOS):

- **Q369** — The macOS part of `platform` is written without a Mac: it is checked by `cargo fmt` and by clippy with
  `-D warnings` for `aarch64-apple-darwin`, from Linux, over `platform` and every crate that does not need `ring`
  (all but `tls`, `http`, `engine` and `app`, whose build compiles C with the compiler of Apple). None of it has run,
  its own tests included, and nothing links it. The roadmap keeps M11 open until `cargo xtask check` passes on a Mac.
  Should the code stay on the branch until then, or wait for a Mac to be written against?
  *Answered by [D48](#d48--macos-on-github-actions)*: the code stayed, and the pipeline runs `cargo xtask check` on
  macOS and builds both targets of the Mac; its run of 7d01480 (2026-11-15) passed on all three systems, which closes
  M11.
- **Q370** — As on Linux, the libraries of the system are declared by hand, one file each: `CoreFoundation` (strings,
  arrays, dictionaries, bytes, `CFRelease`), `CoreServices` (`FSEvents`), `Security` (the keychain), `libobjc` (with
  `AppKit` linked for its classes) and `libSystem` (`localtime_r`, the queues of `libdispatch`). The `objc2` crates are
  in the build already, through `winit`, `arboard` and `rfd`, but are not used directly. Should `platform` take
  `objc2-app-kit` and `objc2-core-foundation` instead ([D22](#d22--libraries-with-judgment))?
- **Q371** — Watching files: one stream of `FSEvents` per folder, each file said (`FileEvents`), a change alone said at
  once (`NoDefer`), with a latency of 50 ms, on a queue of `libdispatch` of its own whose callback sends the changes to
  the thread of the folder. The stream keeps what it sends to through its `retain` and `release`, so that a callback
  late after the watcher is dropped sends into nothing. A folder whose times alone changed is left out, as Q103 says.
  The paths start with the folder as the system writes it (`/private/var` for `/var`), and are given back from the
  folder asked. Is a stream per folder right, rather than one stream for every folder of the window?
- **Q372** — Credentials: a generic password of the keychain of the user, of the service `Gitardo Desktop`, whose
  account is the name of the secret; writing updates the item, and adds it when there is none. macOS asks the user
  whether a program may read an item that another program, or another build of the same one, wrote: a build that is
  not signed may be asked about on each new build. The program is a plain binary: no `.app` bundle, `Info.plist`, icon
  or signature is made. Should packaging for macOS come with this milestone or with the first release there?
- **Q373** — High contrast is `accessibilityDisplayShouldIncreaseContrast` of `NSWorkspace`, read at each focus as on
  the other systems; the time of a double click is `doubleClickInterval` of `NSEvent`, read once (500 ms when it says
  nothing usable between 1 and 5000 ms); a notch of the wheel is 3 lines, as on Linux. Opening an address runs `open`,
  with a thread that waits for it, the same code as `xdg-open` on Linux now. Revealing, the terminal and the external
  editor are unchanged above `platform`. Is that enough?
- **Q374** — Tests: the tests of the clipboard and of watching folders run on macOS too; those that open windows
  (`platform/tests/window.rs`, `app/tests/demo`) do not, since `AppKit` runs the event loop on the first thread of the
  program only, and the tests of Rust run on threads of their own. Should those run on macOS in a test binary of their
  own without the harness (`harness = false`), whose `main` runs them on the first thread?

### Of the extension, Q375 to Q392

- **Q375** — The glossary's **branches shown** is `Scope` in the code and "filter by branch" in the README, while `Filter`
  in the code is the search. Which word wins, in the code and on screen? Nothing is renamed until this is answered.
- **Q376** — **Operation** means both what Gitardo runs (`GitOp`, **Undo Last Operation**) and what git has under way
  (`Operation`, the banner). Keep both, with *operation under way* for the second, or rename one?
- **Q377** — Why is there no Prettier in the project, when the code is laid out as Prettier lays it out? (D34)
- **Q378** — `npm run check:docs` needs Python, which the project does not otherwise need, so it is not part of
  `npm test`. Keep it apart, port the check to a Vitest test, or add it to `npm test` and require Python?
- **Q379** — The private channel for reporting a vulnerability in [SECURITY.md](../SECURITY.md): an email address, or
  GitHub's private vulnerability reporting on the repository?

The questions the cards raised, each naming the cards it would change:

- **Q380** — Staging, unstaging and discarding one hunk are built (`changes.stage-hunk`) but no view offers them. Offer
  them, or remove the code?
- **Q381** — Amending a commit already pushed asks nothing and says nothing, while rewording, squashing and splitting one
  warn. Should the amend warn too? `commit.amend`, `rewrite.pushed-warning`.
- **Q382** — Is reverting one change in the Flow Diff a discard, to be asked first as the README's list asks before
  discarding? And should discarding there take the file back to `HEAD`, as the Changes view does, rather than to the
  index? `changes.discard`, RS16.
- **Q383** — Deleting a branch whose upstream is another local branch: should it offer to delete the upstream too, as
  it does for a remote branch? `branch.delete`.
- **Q384** — A rebase onto a branch, and a pull with `--rebase`, rewrite the commits not yet pushed and ask nothing. Is
  that the intended line between `confirm` and nothing? `rebase.onto`, `sync.pull`.
- **Q385** — Undo after a fixup leaves the `fixup!` commit when the autosquash stopped, and a fixup of a pushed commit
  does not warn. `rewrite.fixup`, `undo.last`.
- **Q386** — The rebase editor runs without `--autostash` and without the warning for pushed commits that the other
  rewrites have. On purpose? `rebase.editor`, `rewrite.pushed-warning`.
- **Q387** — The menus offer merge, rebase, cherry-pick and revert while an operation is under way, and git refuses.
  Hide them instead? `merge.into-current`, `operation.banner`.
- **Q388** — What should **Undo** do right after **Abort**? Today it takes back whatever the reflog shows last.
  `operation.abort`, `undo.last`.
- **Q389** — The review reads the conflicts of at most 200 files; what does the user see of the others?
  `conflict.files-read`, `conflict.review-files`.
- **Q390** — **Sync** of submodules acts on all of them, while the other submodule entries act on one. `submodule.sync`.
- **Q391** — Stashes are named to git by position (`stash@{n}`); one pushed in a terminal between reading and running
  moves the others. Name them by their commit instead? `stash.apply`, `stash.pop`, `stash.drop`.
- **Q392** — The message drafted by the language model is not shown as running, and a message typed in the box is kept
  only once the Changes view has been opened. `commit.draft-message`, `commit.box`, `activity.track`.

### Of the transition to one engine, from Q393

Raised while writing [transition/](transition/README.md). Each has a recommendation; Q393 to Q395 were answered at the
start of stage 1, as recommended.

- **Q393** — How is `platform` split, so that `engine` and `gitardo-engine` link no window library? `engine` uses only
  `link_folder` and the three functions of the credential store (`crates/engine/Cargo.toml`). Recommended: a new crate
  `system` with those, which `platform` and `engine` both use, since a Cargo feature `window` of `platform` would make
  the edges of the picture depend on features, which `cargo xtask check` does not read. *Answered in stage 1*: the
  crate `system`, which also holds the bindings of `kernel32.dll` and of `CoreFoundation` that `platform` shares.
- **Q394** — The framing of the messages: `Content-Length` headers, as the Language Server Protocol, or one JSON per
  line? And does the extension take `vscode-jsonrpc` for it? Recommended: the headers, written in a few lines of the
  extension's own, no new package. *Answered in stage 1*: the headers, written by hand on both sides.
- **Q395** — Where do `history_of` and `commits_request` (`crates/app/src/store/timeline/mod.rs`) and the reading of
  the settings (`crates/app/src/settings/read.rs`) go, so that the server uses them? Recommended: into `engine` for the
  history, and a crate `settings` for the reading of the settings, which the window uses from there. *Answered in
  stage 1*: `engine::timeline` holds `TimelineRequest`, `history_of` and `commits_request`; the crate `settings` holds
  the files, the keys, the `Lookup` of the two files and `feature_settings`, and the window reads its own settings
  (the theme, the fonts, the switches of the timeline) with them.
- **Q396** — Trust inside VS Code: the desktop asks before running what a folder's `.gitardo/settings.json` asks
  (`trust.ask`); under the extension the settings come from VS Code, which has workspace trust. Recommended: the
  extension sends `trust: "vscode"`, and the engine runs the setup commands of a worktree only in a trusted workspace,
  asking nothing of its own ([W5](transition/04-differences.md#w5-who-decides-that-the-setup-commands-may-run)).
- **Q397** — The Commits view of the extension (`Repo.branchLog`) has no read in the engine. Recommended: a card
  `branch.read-log` in the desktop first, made of `graph.read-commits` and `graph.unpushed`, and a view of it in the
  desktop only if it earns one. *Answered in stage 5*: the card `branch.read-log`, a read of the engine
  (`git::read::branch::read_log`) with the format of `graph.read-commits` and git's own count of what is not pushed;
  no view of it in the desktop yet.
- **Q398** — Three reads are done in `crates/app` and not in the engine: the common git folder of a worktree, the
  message git prepared (`SQUASH_MSG`), and the diff given to the AI for a commit message. Recommended: a Read card for
  each in the engine, the window using them from there. *Answered in stage 5* for the first two: the cards
  `repo.common-folder` and `commit.prepared-message`, which the extension reads with `read/…`; the window still reads
  both itself (`repository_watch.rs`, `operations.rs`) until it is moved to them. *Answered in stage 4, area 6* for
  the third: the card `commit.read-draft-diff`, which the window's draft uses and the engine reads for
  `read/commit.draft-message`.
- **Q399** — A feature that writes a setting under the extension (`worktree-setup.save`, the toggles of the graph):
  recommended a new follow-up `WriteSetting { key, value, target }` that the window carries out by writing its
  `settings.json` and the extension with `WorkspaceConfiguration.update`, in place of the steps that write the file.
  *Partly built in stage 4, area 6*: `FollowUp::WriteSetting { key, value }`, without `target` (the user's settings),
  left by `ai.choose-model` only under the extension (`uses_host_models`); the window still writes its file there,
  and the other features wait.
- **Q400** — The extension's `gitardo.conflicts.aiModel` and `gitardo.conflicts.aiEffort` against the desktop's
  `ai.model` and `ai.effort`. Recommended: the extension keeps its keys and sends them as `ai.*`; a `gitardo.ai.*`
  pair only if a second use of AI needs another model.
- **Q401** — The extension has `gitardo.worktrees.branchFormat` and `gitardo.worktrees.inputs` beside
  `gitardo.branches.format` and `gitardo.branches.inputs`; the desktop has only the second pair. Recommended: the
  engine reads the worktree's pair first for a new worktree's branch, and the desktop gets the two keys.
- **Q402** — Reverting one change of a diff: the engine writes the file (`diff/revert_change.rs`), the extension edits
  the open document so that Ctrl+Z works and edits not saved are kept
  ([A5](transition/04-differences.md#a5-revert-one-change-in-a-diff)). Recommended: under the extension the plan
  gives the new text in a follow-up and the extension applies it to the document; the engine only checks that the
  change is still where it was shown.
- **Q403** — Undo right after an operation it cannot take back, and Undo while an operation is stopped
  ([D7, D8](transition/04-differences.md#d7-undo-takes-back-the-operation-before-one-it-cannot-see-on-both-sides)).
  Recommended, before stage 3: the journal records that the last plan cannot be undone until `HEAD` moves, and
  `undo.last` refuses while a merge, rebase, cherry-pick, revert or patch is stopped, but for a squash merge and a
  resolution of conflicts.
- **Q404** — The engine's log keeps neither git's output under a step that succeeded nor what was given on standard
  input ([D11](transition/04-differences.md#d11-the-log-keeps-no-output-of-commands-that-succeeded-nor-what-was-given-on-standard-input)).
  Recommended: log both, the output at `info` (push hints and autostash messages are read there), the input at
  `debug`.
- **Q405** — After git refuses to delete a branch or remove a worktree, the engine offers **Delete anyway** and
  **Remove anyway**, while the extension asked the forced variant at once, and the engine's `submodule.deinitialize`
  still asks at once (B3, W7). Recommended: the engine's offer after the failure, for all three.
- **Q406** — The lanes of the graph: computed by the webviews from the rows (`src/shared/graph.ts`), or sent with the
  rows by the engine, which has the same derivation for the window? Recommended: the webviews until stage 7, then
  measured both ways on the large repository.
- **Q407** — A comparison narrowed to a path counts, in the engine, only the commits that change it, and asks no merge
  when one side has none; from the working tree in **Since they diverged**, the engine compares the files with the
  other side, the extension from the merge base
  ([R7](transition/04-differences.md#r7-a-comparison-narrowed-to-a-path-or-from-the-working-tree-since-they-diverged)).
  Recommended: what the extension does, and `compare.read-sides` changed to it; until then the extension reads those
  two cases in TypeScript. *Taken as recommended in stage 10, to be confirmed: `compare.read-sides` counts the whole
  sides and compares the working tree from the merge base, and the extension reads every comparison from the engine.*

### Of the repository tabs, from Q408

Questions of milestone M12 ([D50](#d50--repository-tabs-and-what-each-repository-keeps)), with their cards.

- **Q408** — A sign on each tab for what waits there: how many files changed, the commits to pull and to push, a
  conflict or an operation stopped half way (`repo.tabs`). Proposed to the maintainer, not decided; nothing of it is in
  the cards until it is.
- **Q409** — Operations in several tabs at the same time: each repository already has its engine on a thread of its
  own in `gitardo-engine`, but the window has one Command log, one Undo and one dialog at a time
  (`repo.switch-tab`). Recommended: one at a time in M12, as the card says, and asked again once it is used.
- **Q410** — How much a window may keep: eight tabs of large repositories with many pages of commits each. Recommended:
  everything, measured by `cargo xtask perf` with eight tabs of the large repository; if it is too much, the commits
  beyond the first page of a tab not shown for ten minutes are dropped and read again when it is shown.
- **Q411** — Moving a tab by dragging it, and moving it into a window of its own (`repo.tabs`). Recommended: dragging
  within the island in M12 if it costs little, no moving between windows.
- **Q412** — In the extension, where each webview keeps what a repository showed (`repo.keep-open`): in its own state
  (`vscode.setState`), by the folder of the repository, or in the extension host. Recommended: each webview, in memory,
  by folder, the host keeping only the message of the Changes view. Built so in phase 6 of M12, the host keeping as
  well, as the controller kept before for the repository shown, the state the views were last sent, what the graph
  was narrowed to and how far it was read.

### Of the branch rail's own filter, from Q413

Questions of milestone M13 ([D51](#d51--the-branch-rail-has-a-filter-of-its-own)), each built with the reading given
until the card says otherwise.

- **Q413** — `refs.listed`: **Current** lists the branch checked out and its upstream, as the card says, and
  **Upstream** the same, since nothing reads the branch a branch was made from. The ticks of the list under **Current**
  are those of `filter.tick`: the branch checked out alone. Is a branch's base to be read for **Upstream**?
- **Q414** — `refs.listed`: with every branch ticked gone, the rail lists everything, as `filter.remember` does for the
  timeline; what is kept is not written again at a read, only when the choice changes. Is that the reading?
- **Q415** — `refs.scope-line`: the line says what `filter.label` says, `Local branches`, where the card's example has
  `Local`. The **Tags** list is left out while the filter of the rail lists no tag, as a remote list with no branch
  listed is (Q71); stashes, worktrees and submodules are always listed.
- **Q416** — `refs.list-filter`: **Show local branches**, **Show remote branches** and **Show tags** are three check
  boxes at the bottom of **Branches listed**, where the timeline's list had **Filter the list of branches too**, greyed
  out as `filter.kinds` says, with **Chosen in the filter** as their tooltip. The button is the funnel alone, at the end
  of the line that names the branch checked out, and its tooltip is `Branches listed in the branch rail: <label>`.
  `branches.followGraphFilter` stays on the settings page, said to be no longer read.

### Of milestone M14, from Q417

Questions of milestone M14 ([D53](#d53--pages-beside-the-inspector-a-bottom-panel-the-icons-of-lucide-islands-in-the-extension)),
each built with the reading given until the card says otherwise.

- **Q417** — `terminal.view`: which library. Recommended: `alacritty_terminal`, which has the parser, the grid, the
  scrollback, the selection and the pseudo-terminal of each system (ConPTY on Windows), and is kept up with Alacritty;
  its types stay in a crate `terminal` of its own (R1.6), which `app` uses. Else `portable-pty` for the processes and
  `vte` for the parser, the grid written here.
- **Q418** — `panel.bottom`: are the terminals the repository tab's or the window's? Recommended: each repository tab
  has its own, started in its folder (D50: what is under the tabs is the tab's); the panel being open, its height and
  the tab chosen in it are the window's.
- **Q419** — `diff.beside-inspector`: a Diff page opened from the inspector shows the commit selected when it was
  opened; selecting another commit in the inspector is not possible while the timeline is not shown. Recommended:
  **Timeline** in the trail, or `Escape`, brings the timeline back with the same commit selected, and the page stays
  in `Ctrl+Tab`.
- **Q420** — `graph.icons`: the icons of Lucide are drawn at 24 units with a stroke of 2. Recommended: drawn at 16
  logical pixels, the stroke scaled to 1.5 so that they keep the weight of the text beside them, and the outlines
  turned into paths once, when the program starts.
- **Q421** — `terminal.view`: the card reads `editor.fontFamily` and `editor.fontSize`, which Gitardo Desktop does not
  have. Built: the terminal is drawn in the font of code, `font.code` at the size `font.size` gives code, as every
  other place that shows code.
- **Q422** — `terminal.view`: no italic and no bold of the font of code is drawn by the window yet. Built: italic text
  is drawn upright, and bold text is drawn twice a hair apart, which makes it heavier without changing its width.
- **Q423** — `terminal.view`: `Ctrl+V` pastes, as it does in every input of the window, rather than going to the
  shell; `Ctrl+Shift+V` pastes too, there and in the inputs.
- **Q424** — `terminal.view`: **Kill the terminal** on the last terminal of a repository tab hides the panel, as the
  Terminal tab would start another at once otherwise.
- **Q425** — `terminal.view`: the wheel over a program that draws on the second screen (a pager, an editor), which has
  no scrollback, sends it the up and down arrows, three lines for a turn; elsewhere it scrolls into what was printed
  before. On Windows the acceptance line of `tput cols` is checked with `[Console]::WindowWidth`, which PowerShell
  reads from the same size.
- **Q426** — `panel.bottom`: the tabs of the panel are text buttons. Once the icons of Lucide came (`graph.icons`),
  **Terminal** in the bar of views draws `square-terminal`, **Hide the panel** `x`, **New terminal** `plus` and
  **Kill the terminal** `trash-2`, as the table says.
- **Q427** — `diff.beside-inspector`: which file is the next one for `Alt+Down` while the files are laid out as a
  tree, and what `Escape` does while the Activity drawer is open. Built: the order the inspector shows them in, files
  of folded directories included, then the untracked files of a stash; from a page of all the files, the first.
  `Escape` closes the drawer first, as it did, and brings the timeline back once it is closed.
- **Q428** — `diff.beside-inspector`: a file of more than 100,000 lines clicked while the page beside the inspector
  already shows another. Built: the page is open, so the file is shown as one with too many differences, as `F7`
  going from file to file shows it, rather than asking; only the first file of the page asks.
- **Q429** — `file-view.beside-files`: the narrow layout of the Files page. Built: below 700 logical pixels, where the
  inspector goes over the timeline, the File page takes the whole page; a file that has its own page already is shown
  beside the tree too and keeps its page.
- **Q430** — `graph.icons`: a button that opens a menu below it (the arrow of **Sync**, the list of a repository tab,
  the mode of the search box, the action of a step of a rebase) had the chevron down, which the table gives to a row
  unfolded. Built: `chevrons-up-down`, a line of its own in the table; the arrow of an entry of a menu that opens a menu
  to the side keeps `chevron-right`, read as folded: more inside, shown when opened.
- **Q431** — `graph.icons`: buttons and marks the table did not have, each given a line: **Accept the changes that need
  no choice** of the Merge page `check-check` (the card's two ticks), **Stop drafting the message** `circle-stop` (it
  had the cross of closing), **Open in external editor** `external-link`, **Take incoming** and **Take current** of a
  file in conflict `circle-arrow-down` and `circle-arrow-up` (two plus signs side by side could not be told apart; the
  arrow of the comparison page that takes a file into the working tree is the incoming one), the grip of a step of a
  rebase `grip-vertical` (it had the three dots of More), the banner of a bisect `binoculars` (it had the magnifying
  glass of Search), a submodule not initialized `circle-dashed` (it had the minus of Unstage), and **Show the base** of
  the Merge page `git-fork` (it had the clock of Recent).
- **Q432** — `graph.icons`: the proposals of the Merge page come from the rules or from the AI, and **Use what is
  proposed** of the worktree setup from what Gitardo found. Built: all draw `sparkles` with the messages the AI drafts,
  one meaning, made for you, to look at before taking it; the wand stays what the rules resolved, in the Changes view
  and in the review. Else the proposals of the rules a wand and those of the AI sparkles, which the button that accepts
  all of them at once cannot say.
- **Q433** — `graph.icons`: some lines of the table share a meaning with an entry of the bar of views, and are built as
  one: the Changes view and opening the changes of a file `file-diff`, the Compare view and Compare
  `git-compare-arrows`, the Activity view and what is running `activity`; **Show in the timeline** draws the Timeline's
  `git-graph`. A submodule added and not committed draws the pencil, the sticky name of a branch its branch, the button
  that shows or hides the branch rail the branch, and the funnel of the blame (ignore whitespace) stays. The box that narrows the branch rail by name keeps the magnifying
  glass: typing a name is a search, and the toolkit makes a box with it a search box for the screen reader. The person with
  a pen of the blame of a file is filled while the blame is shown, in place of the closed eye.
- **Q434** — `graph.icons`: Q420 asks for the outlines turned into paths when the program starts. Built: each outline
  is read into paths the first time its icon is drawn and kept with the icons rasterized, which is once for the
  program without reading the 79 files before the first frame; a test reads all of them. The path data is parsed with
  `svgtypes` (MIT or Apache-2.0, kept up with `resvg`), which turns its arcs into curves.

### Of milestone M15, from Q435

Questions of milestone M15 ([D54](#d54--the-timeline-is-read-by-one-git-log-as-it-prints)), each built with the
reading given until the card says otherwise.

- **Q435** — `graph.read-commits`: where one read stops. Recommended: at 100,000 commits, whatever `graph.pageSize` is,
  so that a history of that size is walked once and a tab keeps some 25 MB of rows at most before it is scrolled
  (Q410); past it, each read from the last commit with `--skip` lists 100,000 more.
- **Q436** — The protocol: how the extension gets the pages. Recommended: the request `read/graph.read-commits` takes
  `stream: true` (a field added, version 1.8); the engine sends `read/graph.rows { read, rows: Row[] }` for each page
  as it is read, `read` the id of the request, and answers the request with `{ rows: [], hasMore }` once its read ends,
  at the end of the history or at 100,000 commits. `$/cancel` stops it as any read. Without `stream`, the read answers
  as today, for an older extension.
- **Q437** — `repo.watch-hidden`: a tab not shown reads its history again when a reference moves in it. Recommended:
  it reads its first page only, and the rest once it is shown, so that eight tabs do not walk eight histories at once.
- **Q438** — `graph.load-more`: the bar along the top of the list ran while a page was read. Recommended: it runs
  until the first page of a read is there; while the rest is read, the last row says **Loading more commits…**, as it
  does while there are more.
- **Q440** — `graph.read-commits`: a stash whose date puts it among the commits of a page, made on a commit of a
  later page, would be placed where all the commits read at once put it only once its commit is read. Built: the page
  waits, and is given with the stash once its commit is read, or without it when the read ends without it (the stash
  is made on a commit the history shown does not have). Pages stay of `graph.pageSize` commits otherwise; a page held
  so is given with the next.
- **Q441** — `graph.lanes`: a page added under the timeline is laid out alone from the lanes under the row above it,
  but folding a merge (`graph.fold-rows`) can move rows of earlier pages, and which merges can be folded and which
  commits are not pushed depend on the commits further down. Built: with any merge folded, the timeline is laid out
  whole at each page, as before; with none, the lanes, the tips of the lines, the rows of each commit and the lines
  between days and months of the new rows are worked out alone, and the merges that can be folded and the commits not
  pushed again over all the rows, which costs a walk of the rows but no layout.
- **Q442** — `repo.schedule-reads`: which reads keep the commits when the history is the same. The card names those
  that read them again (**Refresh**, a filter, a search, the branches shown, the next page). Built: a change of a file
  of the working tree or of the git folder, a file saved or made by Gitardo, the focus and the end of an operation keep
  them; the settings and a repository chosen read them again, since they can change what is listed. The history is the
  same when the commit checked out, the branches, the remote branches, the tags and the stashes point where they did,
  and the arguments of `git log` are the same.
- **Q443** — `graph.read-commits` in the engine of the extension: the state its last `repo.read-state` read can be
  older than the repository when a reference moved since and the extension has not read the state again. Built: the
  engine keeps the state of every read of the state of a repository opened (asked, after an operation, after the
  changes it watches), the same state `journal/last` checks Undo against, and the reads of the graph
  (`graph.read-commits`, `graph.read-position`, `search.read-matches`) take it; only a repository whose state was not
  read since it was opened, or since an operation ended, asks git. The extension reads the state after each change it
  sees, before the commits.
- **Q460** — `repo.watch-changes`: when the files of a burst are asked of git. Built: a second after the first of them
  changed, the delay of a file of the working tree, every file changed until then is given to one
  `git check-ignore -z --stdin`; the read the others ask for starts as soon as git answered, since their second is
  over. The files that change while git is asked wait for the next question, a second after the first of them; a read
  that starts meanwhile, for any reason, sees them, and they are not asked. When git cannot answer, every file asked is
  taken as one it does not ignore. The Activity drawer says the question as **Asking git which files it ignores**, as
  it says the files not saved of the Changes page.
- **Q461** — `repo.watch-changes`: a tab not shown (`repo.watch-hidden`) is still marked out of date by a change to a
  file git ignores or to a nested worktree, and read once it is shown. Built so: asking git for each tab not shown
  would start a git for each burst of each tab, for a read that waits until the tab is shown anyway.
- **Q462** — `repo.read-state`: what tells that something changed in the git folder of another worktree. Built: the
  size and the time of change of each file at the top of its own git folder (`HEAD`, `index`, `ORIG_HEAD`,
  `MERGE_HEAD`…), of those of `rebase-merge`, `rebase-apply` and `sequencer`, and of `logs/HEAD`, leaving out the
  `.lock` files and the socket of the file system monitor; a worktree whose git folder cannot be read is counted every
  time. **Refresh** and choosing a repository count every worktree again. In the extension, version 1.8 of the
  protocol says it with `refresh` on `read/repo.read-state`; `repository/open` counts every worktree (phase 9 of M15).
- **Q463** — `repo.watch-changes`: from when the minute of the focus is counted. Built: from the end of the last read
  of the repository shown, whatever asked for it, kept by the window for each folder read, so that a tab shown again
  after a minute is read when the window gets the focus; a repository never read asks for a read. A change missed
  while the window was minimized is forgotten by the next read that starts, whatever asked for it.
- **Q520** — `repo.watch-changes` in the extension: what stands for the window minimized, whose changes are read when
  it gets the focus. Built: no view of Gitardo open (the graph, the Source Control views, a tab of Gitardo), since VS
  Code says nothing of its window being minimized; a change then marks the repository, and VS Code getting the focus
  reads it, as a change missed does in the desktop.
- **Q521** — `graph.load-more` in the extension: how often the pages a read adds are sent to the views. Built: those
  that come within 150 ms of the first together, and the last at once, with only the rows the views do not have; a
  page each time would draw the graph again for each of the 56 pages of a history of 56,000 commits.
- **Q522** — `graph.read-commits` in the extension: what the controller's cancel stops. Built: the wait for the first
  page of a read; once a page came, the read goes on in `EngineGraph` until it ends, another history is asked for, or
  another repository is shown, so that a read of the state that another takes the place of does not throw away the
  commits read.
- **Q523** — the name of the read that says which files of a burst ask for a read. Built: `read/repo.watch-changes`,
  after the card whose behavior it is, with `{ paths }` asked like `read/changes.read-ignored`; a request, not the
  notification `repository/changed`, since the extension reads the state itself and only asks whether a read is due.


### Of milestone M16, the settings page and one lane, from Q480

Questions of the phases of milestone M16 that make the settings page and all commits on one line
([D55](#d55--the-settings-page-in-sections-in-the-middle-of-the-window)), each built with the reading given until the
card says otherwise.

- **Q480** — `settings.sections`: the icon of each section. Built: **General** the gear of Settings, **Appearance**
  Lucide's `palette` (a meaning added to the table of `graph.icons`), **Timeline** `git-graph`, **Branches**
  `git-branch`, **Pull and commits** `git-commit-horizontal`, **Worktrees** the worktree's, **Files and blame** the
  person with a pen of the blame, **AI and conflicts** the sparkles of what the AI drafts, **Terminal and tools** the
  terminal's; each draws the same thing it draws elsewhere.
- **Q481** — `settings.reveal`: the card gives `scroll_to(rows, place, list_height) -> f32`, but the place of a row
  depends on the height of its description wrapped to the width of the list, which the rows alone do not know. Built:
  `scroll_to(layout, place, list_height, rows) -> Option<f32>`, from the list laid out for the width of the frame;
  nothing when the list does not show that place.
- **Q482** — `settings.sections`: the name of a section near the end of the list cannot reach its top. Built: when
  the list is scrolled to its end, the section last asked for is marked while its name is in sight; otherwise the
  section whose name was last scrolled past, and **General** at the top.
- **Q483** — `settings.edit`: the card's table has no control for `branches.inputs`, a list of objects, nor for
  `ai.provider` and `ai.effort`, whose values the card lists. Built: `branches.inputs` is a field of its JSON, written
  only when it is JSON; `ai.provider` and `ai.effort` are fields whose placeholders say their values, since an empty
  value means something for both.
- **Q484** — `settings.edit`: `fileBlame.age.oldAfterDays` has no range in the card. Built: from 1 to 36,500 days,
  as `settings.read` takes it.
- **Q485** — `settings.edit`: **Add** under a list of strings writes the list with an empty entry, which its field then
  fills; an entry left empty when its field loses the focus is taken out, so that no empty pattern stays in the file.
  A list or the table of `identities` is drawn under the key of its row, across the row, since a column of 280 logical
  pixels holds neither.
- **Q486** — `settings.reset`: ↺ is named `Back to the default: <title>` for the screen reader and its tooltip, and
  draws `undo-2`, the icon of what goes back to how it was.
- **Q487** — `settings.edit`: **User** and **Repository** are tabs for the screen reader too, a role the toolkit did
  not have; each row of a setting is a group named by its title, selected while it is marked, so that the row
  `settings.reveal` marks is said and can be tested. While no repository is shown, the page shows **User** whatever tab
  was chosen before.
- **Q488** — `settings.reveal`: **Open Settings: <title>** empties the search of the page when something is typed in
  it, and shows **User** when **Repository** is shown and the setting is not one its file may set, since the setting
  would not be listed otherwise. Its entries come after the commands of Quick search, the sections first.
- **Q489** — `graph.one-lane`: the line between two rows is dashed when the upper row is the uncommitted changes or a
  stash, the line each had down to its commit. `graph.oneLane` is the one setting under `graph.` whose change, when the
  settings file is read again, asks no read of the repository: it only lays the rows out again. The lanes themselves
  are not changed; `one_lane` is used in their place while the setting is on.

### Of the side column and the pages beside, from Q500

Questions of phases 7 to 11 of milestone M16
([D56](#d56--a-side-column-of-repositories-and-branches-what-was-said-in-the-status-bar-diffs-beside-what-opened-them)),
each built with the reading given until the card says otherwise.

- **Q500** — `activity.status-bar`: its last acceptance line asked the branch of the header to say `main ↑2 ↓0`, which
  D56 and `graph.where` take away. Built: the line now asks that the header name `main` without how far it is, and that
  **Sync** say `↑2`.
- **Q501** — `graph.view`: in the narrow layout the branch rail was narrowed to its icons, where `repo.side-column` puts
  the side column away. Built: below 900 logical pixels the side column is put away and its button opens it over the
  timeline, as wide as it was; the rail shows its icons only when it is narrower than 120 logical pixels, which the
  side column never is.
- **Q502** — `repo.tabs`: the row of a repository has "the icon of a repository", which the table of `graph.icons` does
  not have. Built: the icon of a worktree, `folder-git-2`, whose line of the table now says a repository open too: a
  repository open is the folder of its main worktree, and no other outline is added.
- **Q503** — `repo.tabs`: **Reveal in the file manager** in the menu of a row not shown would run in a repository the
  window has not read. Built: the menu of the row shown has it, as `worktree.reveal` does; **Copy path** is on every
  row and copies the folder of what the row shows.
- **Q504** — `repo.side-column`: in the narrow layout, **Toggle the Side Column** in Quick search has no button to open
  the column over. Built: the button at the top of the timeline opens it over the timeline and closes it; Quick search
  puts it away or brings it back, which is seen once the window is wide again. **Repositories** starts at 160 logical
  pixels with its heading.
- **Q505** — `repo.worktree-summary`: its function gave a `WorktreeSummary` alone, where git may fail to run. Built:
  it gives `Result<WorktreeSummary, GitError>`, a folder gone is `Ok` and said `missing`, and an error leaves the last
  summary, muted, as the card says. Whether a git folder changed is said by `app::work::summaries::git_folder_changed`,
  from the times its `HEAD`, its index, `packed-refs`, `FETCH_HEAD` and its folders of refs were written; the store
  forces a read when the files of the folder change.
- **Q506** — `activity.said`: a click on its words opens the Command log "at the command that said it", where the
  Command log keeps no link from what was said to its command. Built: what an operation said keeps the place, among
  every line the log recorded, of the first line the operation added, or of the step before what git said when a step
  failed; the click shows the Command log of the bottom panel with that line at the top, marked. A line dropped since,
  past the last 10,000, shows the log at its end. What failed keeps its links, and **×** is an icon with **Dismiss** in
  its tooltip.
- **Q507** — `diff.beside-inspector`: a click on the commit in one line at the top of the inspector "gives back the
  whole card", which the inspector beside a Diff page cannot show without leaving the page. Built: the click brings
  the timeline back, as **Timeline** in the trail does, where the inspector is the whole card of the same commit.
  `Up` and `Down` in the tree are built: a file clicked gives the tree the keyboard, and `Down` and `Up` show the next
  and the previous file in the same page, as `Alt+Down` and `Alt+Up` in the Diff page do, both in the order the tree
  lists the files.
- **Q508** — `compare.beside-diff`: each file of the column is to have "its lines gained and lost", which
  `compare.read-sides` does not read (it gives the status of each file, not its counts). Built: the column shows the
  letter of the status of each file, as the comparison page does; counts would need `git diff --numstat` in that read.
- **Q509** — `compare.beside-diff`: **Commits** in the column could hold thousands of commits. Built: unfolded, it lists
  the first 100 of the comparison, as rows that open the commit page; the comparison page keeps the whole list.
- **Q510** — `compare.beside-diff`: whether **Open in the Diff page** of the comparison page also opens beside it.
  Built: yes, every file in one diff on the left with no file marked in the column; `Alt+Down` then shows the first.
- **Q511** — `compare.beside-diff`: whether the trail's step of the comparison, or `Escape`, closes the Diff page.
  Built: it is kept, hidden, and shows the next file clicked; it is closed with the comparison page.
- **Q512** — `compare.chooser`: **Recent commits** are "the 20 last of the branch checked out". Built: the 20 first
  rows of the first parent of `HEAD` among the commits the timeline has read, with their dates; none when the timeline
  shows another repository than the chooser. A branch, a tag or a remote branch has a date only when the timeline has
  read its commit.
- **Q513** — `compare.chooser`: what `Home` and `End` do while text is typed. Built: they move in the field, as in every
  field; with nothing typed they light the first and the last row of the list.
- **Q514** — `compare.chooser` and `compare.change-side`: **Compare** after both sides of a page were changed. Built: it
  runs `compare.choose` with both sides into the same page, which keeps the path it was narrowed to; one side changed
  runs `compare.change-side` as before, and neither changed closes the dialog with nothing run.
- **Q515** — `compare.chooser`: `Enter` with a row lit and both boxes held. Built: with nothing typed it opens the
  comparison; with text typed it chooses the row lit, so that a typed name is chosen before it is compared.

### Of the worktrees in the side column and a side column that takes less room, from Q530

Questions of phases 13 and 14 of milestone M16
([D57](#d57--worktrees-in-the-side-column-pages-that-keep-what-opened-them-the-way-back-in-the-title)), each built
with the reading given until the card says otherwise.

- **Q530** — `repo.group-tabs`: whether a repository starts with the rows of its worktrees and its submodules folded,
  and what its own row says once they are shown. Built: every repository starts unfolded, since the maintainer asked
  to see them; the folded ones are remembered for the workspace by their folders, `foldedTabs` in `panes`. Unfolded,
  the row of the repository stands for its own folder, says what that is on and is marked only while that is shown,
  a click on it showing that folder; folded, it says the member shown after ` › ` and a click shows the tab as it was
  left, as before.
- **Q531** — `repo.group-tabs`: what the row of a worktree is named. Built: the name of its folder, as the header names
  it (`graph.where`), with its branch on the line of what it is on, so that the branch is not said twice; until it is
  read, that line says the branch `git worktree list` gave. The room of the chevron is kept on every row while one
  repository open has one, so that the names stay in line, and not otherwise. The chevron's target is as high as its
  row, and it is the name `Worktrees and submodules` with whether it is unfolded for the screen reader.
- **Q532** — `repo.side-column` and `branch.pane`: what the top of the branch rail keeps once its buttons are on
  the line of the heading of **Branches**. Built: nothing: the name of the branch checked out goes, since the header
  says it always (D56), and **Create branch**, **Compare** and the filter are icons with their names in their tooltips,
  in that order, the filter last as **+** is last in the heading of **Repositories**. The headings are 28 logical pixels
  high, `control_height` less a small gap, so that their buttons are 28 square.
- **Q533** — `repo.tabs`: how a row of one line shares its room between the name and what it is on. Built: what it is
  on is on the right, in the small text of details; when both do not fit, the name keeps the larger of what is left
  and half the row, cut at its start, and the branch is cut with `…` by the label, the counts kept whole. The rows of
  the rail are 28 logical pixels for a local branch and 26 for the others, the gap between two lists 8, and the filter
  box and the scope line keep their heights, 4 apart.
- **Q534** — `repo.side-column`: how "less high than before phase 14" is measured. Built: the height the side column
  wants for a repository with three worktrees and two local branches, **Repositories** to the foot of its last row
  and **Branches** to the foot of the last title of **Kept aside**, both unfolded: 572 logical pixels before
  phase 14, 382 after.

### Of the title row of the pages, from Q540

Questions of phases 15 and 17 of milestone M16
([D57](#d57--worktrees-in-the-side-column-pages-that-keep-what-opened-them-the-way-back-in-the-title)), each built with
the reading given until the card says otherwise.

- **Q540** — `repo.side-column`: whether the Diff page beside the inspector and a comparison beside its diff, which put
  the side column away while they are shown (D56), have the button of the side column in their title row. Built: no;
  the column of the commit's files or of the comparison takes its place there, and a button that put away nothing
  would do nothing seen. Every other page has it.
- **Q541** — `graph.toggle-references`: its tooltip said **… beside the commits**, which the button of a page is not;
  and "the same key" of phase 15, where the button of the timeline has no key of its own. Built: the tooltip is
  **Hide the repositories and the branches** and **Show the repositories and the branches** on the timeline and on
  every page; the button is reached with `Tab` like every button, and has no key of its own on either.
- **Q542** — `ui.page-trail`: whether each step of the way keeps a copy of how its page was left (its selection, its
  scroll, the file shown), and what the exit's `Timeline › <commit> › <file>` names the Diff page of a file of a commit.
  Built: the way keeps the pages alone; each page keeps what it showed while it is open, and nothing else changes it
  while another page is shown, so going back shows it as it was left. Each page is named as its own title row names
  it: the Diff page of `a.rs` of a commit is `a.rs (f947b97 ↔ eab0959)`, which names the commit, and the File page
  opened from it by `F4` is `a.rs (eab0959)`.
- **Q543** — `ui.page-trail`: which pages start the way again. Built: a page shown while an intent of the bar of views,
  of the **…** menu, of `Ctrl+,` or `Ctrl+Tab`, of Quick search, of the branch rail or of the repositories of the side
  column is reduced; any other page shown while a page is shown goes on from it. A page that opens only once a read
  ends, such as a comparison asked from the menu of a branch of the side column, goes on from the page shown when it
  opens, since the store no longer knows what asked for it; the menus of the side column ask what the same menus of
  the timeline ask.

### Of the Changes page beside what it opens, from Q550

- **Q550** — `changes.beside-file`: what the place does when the diff of its file changes or disappears, as when the
  file is staged wholly while the index against the file on disk is shown, or discarded. Built: once the repository is
  read again, a diff of one file shows what **Open changes** gives that file now (the last commit against the index
  once it is all staged, the index against the file on disk once some of it is unstaged); a file no longer in the list
  leaves the place, and the whole page is shown, rather than the next file, which would show a diff the user did not
  ask for; a File page follows the same rule, since the place shows a file of the list.
- **Q551** — `changes.beside-file`: which ways to the Changes page keep the file beside it. Built: **Changes** in the
  trail, the Changes view of the bar of views, **Show changes** in Quick search and **Review** in the draft show the
  whole page, as D57 says of a page opened from the bar of views or Quick search; `Ctrl+Tab` comes back to the page
  with the file beside it, as it was left, and so does the name of the Changes page in the trail of a page opened from
  the page beside its list, as every name of a way goes back to its page as it was left; that name is the name of the
  page beside the list, as the title row of the Changes page ends with it.
- **Q552** — `changes.beside-file`: what the column holds, and whether the side column stays. Built: the column is the
  whole page, its commit box and its identity included, with narrower margins, so that one commits without leaving the
  review; 360 logical pixels wide where it starts, since its rows keep their buttons, and remembered for the workspace
  as `changesColumn`. The side column stays, as beside the Files page; phase 15 puts it away from the title row.
- **Q553** — `changes.beside-file`: whether `Escape` brings the whole page back, as it does beside a comparison.
  Built: no, since `Escape` closes the box that finds in the File page shown there; **Changes** in the trail does it.

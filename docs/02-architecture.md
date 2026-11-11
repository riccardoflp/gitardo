# Architecture

Gitardo Desktop is one Cargo workspace with no dependency outside it but one, for TLS. This page says what its crates are, which may use
which, how a click becomes a git command, and where state lives. The rules for writing the code are in
[03-code-rules.md](03-code-rules.md); the operating systems are in [05-platform.md](05-platform.md).

## What "no dependencies" means

- Every `[dependencies]` entry of every `Cargo.toml` is a `path` to a crate of this workspace, except in
  `crates/tls`. There is no `build.rs` in the workspace, and no procedural macro.
- **The one exception is TLS** ([D13](08-decisions.md#d13--one-library-for-tls)): `crates/tls` depends on `rustls`, with
  `ring` for its cryptography, and on `rustls-native-certs` for the certificates the system trusts. They and what they
  bring are the only packages of `Cargo.lock` outside the workspace, at the versions of `xtask/allowed-packages.txt`.
- The Rust standard library is used freely: `std` is the toolchain, not a dependency.
- The libraries the operating system ships (`user32.dll`, `libX11.so`, `AppKit.framework`) are called through `extern`
  declarations written here. They are the system, not a dependency: the program would not run without the system
  anyway.
- One program is run as an external process: `git`, found on the `PATH`, which is what the application is about. It
  does its own networking for fetch, pull, push and clone, over HTTPS or SSH.
- `rustfmt` and `clippy` come with the toolchain and are used.

## Crates

```
crates/
  app/        the binary: windows, the event loop, the store, settings, persistence
  views/      every pane and page, as functions of state that return intents
  ui/         the toolkit: layout, widgets, lists, menus, dialogs, pickers, drag and drop, focus, find in page
  editor/     the text editor: buffer, cursor, selection, undo, wrap, highlighting hooks
  render/     pixels: the frame buffer, shapes, anti-aliasing, fonts and glyphs, icons, themes
  platform/   the operating system: windows, input, clipboard, timers, file watching, dialogs, credentials, open/reveal
  features/   every operation, as a function that asks questions and returns a plan
  engine/     runs plans: preview, steps, log, undo journal, activity, follow-ups
  git/        git itself: running it, reads and their parsers, repository discovery
  model/      the data of the domain and its derivations: commits, references, lanes, file trees, names, messages
  text/       algorithms on text: diffs, conflict rules, syntax tokenizers, patterns, fuzzy ranking, templates
  json/       JSON and JSON with comments: parse, write, edit in place keeping comments
  http/       HTTP/1.1 requests, chunked bodies, server-sent events for streamed answers, proxies
  tls/        the only crate with a dependency: a TLS stream over TCP, with the system's root certificates
  testkit/    for tests only: throwaway repositories, golden files, pixel snapshots, a deterministic random source
xtask/        the project's own checks and chores: `cargo xtask check`, `cargo xtask snapshots`, …
```

### Who may use whom

```
app ──► views ──► ui ──► render ──► platform
 │        │        └───► editor ──► render
 │        └──────► model, features (identifiers and questions only)
 ├──► engine ──► git ──► model, text
 │       └─────► http, platform (processes, files)
 ├──► features ──► model, text, git (types only, never running)
 └──► json, http ──► tls ──► rustls, rustls-native-certs (the exception)
```

- Nothing depends on `app`. Nothing below `views` knows about panes or pages.
- `features`, `model`, `text` and `json` are pure: no I/O, no threads, no clock. They are where most of the code and
  most of the tests are.
- `git` runs processes but knows nothing of the UI. `engine` is the only crate that changes the disk or the repository.
- `platform` is the only crate with `unsafe`. Everything above it sees safe functions.
- `tls` is the only crate with a dependency outside the workspace. No type of that library leaves it.
- `testkit` is a `dev-dependency` only.

Cargo refuses a cycle; `cargo xtask check` also refuses an edge that is not in this picture.

## From a click to git

```
 platform event ─► ui (hit test) ─► view returns Intent ─► store.reduce(Intent)
                                                              │
               ┌──────────────────────────────────────────────┘
               ▼
  Intent::Feature { id, target } ─► engine.start(feature, Context)
               │
               ▼
    feature(context, answers) ─► Ask ─► dialog ─► answer ─► call again
                                 Read ─► worker runs query ─► result ─► call again
                                 Refuse ─► message
                                 Run(plan) ─► preview? ─► steps on a worker ─► log, undo journal
                                                                    │
                                                                    ▼
                                         repository read again ─► Event::State ─► store.reduce ─► redraw
```

1. The platform turns what the system sends (a click, a key, a resize) into an `InputEvent`.
2. The `ui` finds what is under it and gives it to the view that drew it. The view returns `Intent`s: what the user
   wants, in the words of the domain, such as `Intent::Feature { id: "branch.create", target: Target::Commit(id) }`
   or `Intent::Graph(GraphIntent::Fold(merge))`.
3. The store reduces each intent. An intent that is only about a view (a fold, a width, a selection) changes that
   view's state and nothing else. An intent that names a feature goes to the engine.
4. The engine calls the feature with the `Context` and the `Answers` so far, and does what the `Outcome` says, until the
   outcome is a plan that has run or a refusal. Questions are shown as dialogs on the UI thread; reads run on a worker.
5. A plan runs on a worker, step by step. Each step is logged before it starts. The undo journal records what the plan
   says it needs.
6. When the plan is done, or when the file watcher says something changed, the repository is read again and the new
   state is sent to the store as an `Event`. The views are drawn from it.

## Threads

| Thread | Does | Talks through |
| --- | --- | --- |
| UI | The event loop of each window: input, reduce, layout, draw, present. Never waits on git or the disk. | `std::sync::mpsc` channels to and from the others; the platform's wake-up to be woken when a message arrives. |
| Workers (a small pool) | Reads, plans, HTTP requests, syntax colors of large files. | Jobs in, events out. Each job has a `CancelToken` and an `Activity` entry. |
| Watcher | Waits for the system to say files changed, groups what comes in a short time, and asks for a read. | Events out. |

No lock is shared between crates. State belongs to the UI thread; workers get copies or `Arc`s of immutable snapshots
and send back results.

## State

- **One store per window**, owned by the UI thread: the repositories of the workspace, the state of each as last read,
  the state of every pane and page, the dialogs open, the activity, the log.
- **`reduce(state, message) -> Vec<Effect>`** is the only function that changes it. `Effect`s are what the store asks
  of the outside (start a job, open a window, write the persisted state) and are carried out after the reduce.
- **The repository state is immutable** once read: a read makes a new `RepositoryState`, and views compare by
  generation number to know what changed. The commits of the graph are kept apart from the rest, so that a read that
  finds the same commits does not rebuild the graph.
- **Persisted state** (sizes, folds, favorites, the branches the graph is narrowed to, recent comparisons, the choices
  of each page) is written to one file per user, keyed by repository path, a moment after it changes and when the
  window closes.

## Drawing

Every frame is drawn by the program into a buffer of pixels in memory, and the platform shows the buffer in the window.
There is no GPU code and no system widget.

- `render` draws shapes (rectangles, rounded rectangles, lines, circles, Bézier curves for the lines of the graph) with
  anti-aliasing, glyphs from fonts it reads and rasterizes itself, and icons from paths of its own.
- `ui` lays out and draws widgets with `render`. It redraws only what changed: each widget says whether its content
  changed since the last frame, and the regions that did are drawn again and sent to the window.
- Long lists draw only the rows in sight: the graph of a hundred thousand commits costs what a screen of rows costs.
- The scale of the screen (HiDPI) is applied when drawing, not when laying out: layout is in logical pixels.

## Long work and cancellation

Everything that may take longer than a frame runs on a worker with an `Activity`: a name in words, a start time and why
it was asked. The status bar shows the activities that have run for more than half a second, and the **What is
running** page lists them. A job that is no longer wanted, such as the read of a page that was closed, is cancelled
through its token, and the git process it runs is killed.

## Where things are

| You want to change | Look in |
| --- | --- |
| What an operation asks or runs | `crates/features/src/<area>/<action>.rs`, and its card |
| How git output is read | `crates/git/src/read/<what>.rs` |
| How a pane or page looks or reacts | `crates/views/src/<name>/` |
| A widget used by several views | `crates/ui/src/widget/` |
| A setting | `crates/app/src/settings/`, and [features/18-settings.md](features/18-settings.md) |
| What runs before an operation, the log, Undo | `crates/engine/` |
| Something only one operating system needs | `crates/platform/src/<os>/` |

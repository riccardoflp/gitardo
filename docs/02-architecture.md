# Architecture

Gitardo Desktop is one Cargo workspace that uses libraries where they serve
([D20](08-decisions.md#d20--libraries-where-they-serve)): so far TLS, and the libraries of the window, the text and the
pixels. This page says what its crates are, which may use which, how a click becomes a git command, and where
state lives. The rules for writing the code are in [03-code-rules.md](03-code-rules.md); the operating systems are in
[05-platform.md](05-platform.md).

## Dependencies

- A crate depends on crates of this workspace and on the libraries `xtask/allowed-packages.txt` names for it
  ([D20](08-decisions.md#d20--libraries-where-they-serve)). There is no `build.rs` in the workspace and no procedural
  macro written here; libraries may have theirs.
- **TLS** ([D13](08-decisions.md#d13--one-library-for-tls)): `crates/tls` depends on `rustls`, with `ring` for its
  cryptography, and on `rustls-native-certs` for the certificates the system trusts.
- **The window, the text and the pixels** ([D16](08-decisions.md#d16--libraries-for-the-window-the-text-and-the-pixels)):
  `platform` depends on `winit`, `softbuffer`, `arboard`, `rfd` and `accesskit_winit`; `render` on `tiny-skia` and
  `cosmic-text`; `ui` on `accesskit`, for the tree of what is on screen.
- Their versions are those of `Cargo.lock`. No type of a library leaves the crates the allowlist names for it: the
  tree of `accesskit`, which `ui` builds and `platform` hands to the system, is named for both.
- The Rust standard library is used freely: `std` is the toolchain, not a dependency.
- What no library covers yet (junctions, killing a tree of processes, the credential store, watching files) is asked
  of the libraries the operating system ships (`kernel32.dll`, `libc`, `Security.framework`) through `extern`
  declarations written here, in `platform`. A library may take over one of them when it is worth the change.
- One program is run as an external process: `git`, found on the `PATH`, which is what the application is about. It
  does its own networking for fetch, pull, push and clone, over HTTPS or SSH.
- `rustfmt` and `clippy` come with the toolchain and are used.

## Crates

```
crates/
  app/        the binary: windows, the event loop, the store, settings, persistence
  views/      every place and page of the window, as functions of state that return intents
  ui/         the toolkit: layout, widgets, lists, menus, dialogs, pickers, drag and drop, focus, find in page
  editor/     the text editor: buffer, cursor, selection, undo, wrap, highlighting hooks
  render/     pixels: the frame buffer, shapes and text drawn into it, the fonts of the program, icons, themes
  platform/   the operating system: windows, input, the frame shown, clipboard, timers, the screen reader, file
              dialogs, file watching, credentials, open/reveal
  features/   every operation, as a function that asks questions and returns a plan
  engine/     runs plans: preview, steps, log, undo journal, activity, follow-ups
  git/        git itself: running it, reads and their parsers, repository discovery
  model/      the data of the domain and its derivations: commits, references, lanes, file trees, names, messages
  text/       algorithms on text: diffs, conflict rules, syntax tokenizers, patterns, fuzzy ranking, templates
  json/       JSON and JSON with comments: parse, write, edit in place keeping comments
  http/       HTTP/1.1 requests, chunked bodies, server-sent events for streamed answers, proxies
  tls/        a TLS stream over TCP, with the system's root certificates
  testkit/    for tests only: throwaway repositories, golden files, pixel snapshots, a deterministic random source
xtask/        the project's own checks and chores: `cargo xtask check`, `cargo xtask snapshots`, …
```

### Who may use whom

```
app ──► views ──► ui ──► render ──► platform
 │        │        ├───► editor ──► render
 │        │        └───► accesskit (D16)
 │        └──────► model, features (identifiers and questions only)
 ├──► engine ──► git ──► model, text
 │       ├─────► model (the plans it runs, the queries it answers)
 │       ├─────► json (the answers of web services)
 │       └─────► http, platform (processes, files, the credential store)
 ├──► features ──► model, text, json, git (types only, never running)
 ├──► json, http ──► tls ──► rustls, rustls-native-certs (D13)
 │   render ──► tiny-skia, cosmic-text (D16)
 │   platform ──► winit, softbuffer, arboard, rfd, accesskit_winit (D16)
 └──► every other crate below it: platform, ui, render, model, git, text (D15)
```

- Nothing depends on `app`, and `app` may use any crate but `testkit` and `xtask`
  ([D15](08-decisions.md#d15--app-may-use-every-crate-below-it)). Nothing below `views` knows about the places of the window or its pages.
- The types of the feature contract are in `model`
  ([D14](08-decisions.md#d14--the-types-of-the-contract-are-in-model)).
- `features`, `model`, `text` and `json` are pure: no I/O, no threads, no clock. They are where most of the code and
  most of the tests are.
- `features` edits the settings files its operations write with `json`, so that their comments are kept.
- `git` runs processes but knows nothing of the UI. `engine` is the only crate that changes the disk or the repository.
- `platform` is the only crate with `unsafe`. Everything above it sees safe functions.
- Each crate uses the libraries the allowlist names for it ([D20](08-decisions.md#d20--libraries-where-they-serve));
  today those are `tls`, `render`, `platform` and `ui`
  ([D13](08-decisions.md#d13--one-library-for-tls), [D16](08-decisions.md#d16--libraries-for-the-window-the-text-and-the-pixels)).
  The pure crates use only libraries that do no I/O.
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
  the state of every place and page, the dialogs open, the activity, the log.
- **`reduce(state, message) -> Vec<Effect>`** is the only function that changes it. `Effect`s are what the store asks
  of the outside (start a job, open a window, write the persisted state) and are carried out after the reduce.
- **The repository state is immutable** once read: a read makes a new `RepositoryState`, and views compare by
  generation number to know what changed. The commits of the graph are kept apart from the rest, so that a read that
  finds the same commits does not rebuild the graph.
- **Persisted state** (sizes, folds, favorites, the branches the graph is narrowed to, recent comparisons, the choices
  of each page, the folders trusted) is written to one file per user, keyed by repository path, a moment after it changes and when the
  window closes.

## Drawing

Every frame is drawn by the program into a buffer of pixels in memory, and the platform shows the buffer in the window.
There is no GPU code and no system widget.

- `render` draws shapes (rectangles, rounded rectangles, lines, circles, Bézier curves for the lines of the graph) with
  `tiny-skia`, text with `cosmic-text` in the fonts that come with the program
  ([D18](08-decisions.md#d18--the-fonts-come-with-the-program)), and icons from paths of its own.
- `ui` lays out and draws widgets with `render`. It redraws only what changed: each frame records what each part of the
  window drew, and only the parts that differ from the last frame are drawn again and sent to the window. It also
  builds the tree of what is on screen for the screen reader.
- `platform` shows the frame with `softbuffer`, in a window of `winit`.
- Long lists draw only the rows in sight: the graph of a hundred thousand commits costs what a screen of rows costs.
- The scale of the screen (HiDPI) is applied when drawing, not when laying out: layout is in logical pixels.

## Long work and cancellation

Everything that may take longer than a frame runs on a worker with an `Activity`: a name in words, a start time and why
it was asked. The activity indicator of the header shows the activities that have run for more than half a second,
and the **Activity** drawer lists them. A job that is no longer wanted, such as the read of a page that was closed, is cancelled
through its token, and the git process it runs is killed.

## Where things are

| You want to change | Look in |
| --- | --- |
| What an operation asks or runs | `crates/features/src/<area>/<action>.rs`, and its card |
| How git output is read | `crates/git/src/read/<what>.rs` |
| How a place or a page looks or reacts | `crates/views/src/<name>/` |
| A widget used by several views | `crates/ui/src/widget/` |
| A setting | `crates/app/src/settings/`, and [features/18-settings.md](features/18-settings.md) |
| What runs before an operation, the log, Undo | `crates/engine/` |
| Something only one operating system needs | `crates/platform/src/<os>/` |

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

*Replaced by [D20](#d20--libraries-where-they-serve). It had been narrowed by
[D16](#d16--libraries-for-the-window-the-text-and-the-pixels), which let in the libraries of the window, the text and
the pixels.*

No crate from outside the workspace (see [02-architecture.md](02-architecture.md#dependencies)), with
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

*Replaces [D5](#d5--https-through-curl). How its libraries are listed and kept in their crate is now
[D20](#d20--libraries-where-they-serve); why these are the ones for TLS still stands.*

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
[D4](#d4--our-own-text-rendering), answers Q24, Q25, Q26 and Q28. How its libraries are listed is now
[D20](#d20--libraries-where-they-serve); which libraries, and why, still stands.*

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

*The maintainer's decision, 2026-11-12. Replaces the window of the first version of [01-product.md](01-product.md),
which was shaped like VS Code, and answers Q27.*

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

*2026-11-12. Replaces the system fonts of the first version of [05-platform.md](05-platform.md).*

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
  clock ([R3](03-code-rules.md)).
- The types of a library stay in the crates the allowlist names for it, so that replacing it changes those crates only.
- The build scripts and procedural macros of libraries are theirs (a `derive` of `serde` is fine). The crates of the
  workspace still have none of their own.
- `git` is still run as a program, not replaced by a library ([D1](#d1--the-git-executable-not-a-library)), and the
  clean room still holds: a library is used through its interface and its documentation, never copied
  ([00-clean-room.md](00-clean-room.md)).
- What is already written here (PNG, deflate, JSON, diffs, HTTP, the file watcher, the credential store) stays until a
  library is worth the change. Each replacement is a commit of its own, and the tests stay as they are.

## Open questions

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
  status bar did; the picture in [01-product.md](01-product.md) also says how far it is from the default branch
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
  writes them. Which crate writes the settings file?
- **Q45** — What is kept between runs is in `state.json` in Gitardo's configuration folder, beside `settings.json`
  (`%APPDATA%\Gitardo Desktop`, `~/Library/Application Support/Gitardo Desktop`, `$XDG_CONFIG_HOME/gitardo-desktop`
  or `~/.config/gitardo-desktop`); [11-data.md](11-data.md) does not name the file. On Linux, should it be under
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
  of any page (about 170 ms for 56,000 commits here), which is most of the time of a page.
- **Q57** — `graph.uncommitted-row`: the draft says **Describe the next commit** in place of a message, and its
  **Review** and **Commit** buttons, and every entry of its menu and of a stash row's menu, are shown but cannot be
  clicked, with the tooltip "Not yet", until the operations they run exist (M4 and later). Is showing them disabled
  the reading, rather than leaving them out?
- **Q58** — `graph.columns` and `graph.date-separators`: dates are said and the days counted in UTC, since the
  program does not read the local time zone yet; the tooltip of a date says `UTC`. Should the local time zone be read
  (through which library on the allowlist), and the separators counted in it?
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
  **Specified**. The dialog of the window checks each value with `search.read-revision` as it is typed and goes to the
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
  but not in the colors of the code: no tokenizer is on the allowlist of D16. Which library, or a highlighter of
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
  page (M6): the button is shown disabled with the tooltip "Not yet", and the card stays **Specified**.
- **Q107** — `settings.theme`: **Theme…** in the **…** menu opens a picker of the five themes. The theme picked is
  written in the user's file by the store (Q44 is still open), and the window takes it when the watcher reads the file
  again. The picker shows no theme as the user moves through the list, which the card asks. Should it, and should the
  theme be written only when the picker closes? The card stays **Specified** (Q44).
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
  of this area.
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
  **Specified**, and so does `undo.last`, whose other lines belong to other slices.
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
  one when the rebase slice is merged. Right?
- **Q160** — **Edit message…** is in the menu of every commit while no operation is under way, not only where
  `rebase.find-line-before-head` gives a line: the menu does not read git, and the operation refuses a commit off the
  line with the card's words. **Merge into `<branch>`…** has no hint *choose how*: an item of a menu has a shortcut
  and a tooltip, no hint. Right?
- **Q161** — `Cmd`+click selects as `Ctrl`+click on macOS in the timeline only, where the selection reads the keys:
  the window does not yet map `Cmd` to `Ctrl` for every shortcut, as `ui::Modifiers::command` says it will. Should it?
- **Q162** — `selection.bar` stays **Specified** for its line 1 (**Squash into one**, from `squash.commits`: built
  since, with the menu's **Squash N commits into one…**), and has no **Copy as patch** (`patch.copy`). `selection.menu` stays **Specified** for its lines 1 and 2 (**Compare these two
  commits** waits for the Compare page), and has no **Copy as patch** nor **Save as patch…**. `selection.cherry-pick`
  stays **Specified** for its line 5 (continued in Gitardo, M6). The bar and the menu empty the selection when their
  cherry-pick is chosen, before the dialog, not once it ran. Right?
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
- **Q174** — The bar of a selection keeps **N commits selected** whole and leaves out, from the left, the buttons that
  do not fit beside it: in a narrow timeline **Squash into one** goes first, then **Cherry-pick onto …**, and **Clear
  selection** stays. The menu of the selection still has every entry. Should the bar rather wrap, or show a **More**
  button?

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
  writes them. Which crate writes the settings file? (Settled since: `features` writes the settings files with `json`, as
  `worktree-setup.save` does, and `settings.theme` is built so; Q292.)
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
  [06-testing.md](06-testing.md#conflicts-against-git) does not exist. Two kinds of random files are not made with a
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
  [testing](06-testing.md) named `testkit/certificates/`. Is this right?
- **Q281** — A question of kind `QuestionKind::Secret` is shown like a text question by the views for now: what is
  typed is not hidden until the input box has a mode that hides it. Should hiding come with the input box of M9 or later?
- **Q282** — `rustls` 0.23.46 needs `rustls-pki-types` and `rustls-webpki`; their newest versions were less than two
  weeks old, so `Cargo.lock` holds 1.15.1 and 0.103.16. Is pinning them in the lock, not in a `Cargo.toml`, enough?
- **Q283** — Outside Windows the credential store says it is not supported, so a token is asked again on each start,
  until M10 and M11 bring the stores of macOS and Linux. Is this right?
- **Q284** — `http` goes through a proxy only by `CONNECT`, for `https` addresses, and refuses a proxy whose address is
  `https`; the proxy is `HTTPS_PROXY`, else `http.proxy` of git, with the hosts of `NO_PROXY` reached directly, as
  [the platform](05-platform.md) says. Is that enough?

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


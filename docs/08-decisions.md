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

*Narrowed by [D16](#d16--libraries-for-the-window-the-text-and-the-pixels), which lets in the libraries of the window,
the text and the pixels.*

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

## D16 — Libraries for the window, the text and the pixels

*The maintainer's decision, 2026-11-12. Narrows [D2](#d2--no-dependencies), replaces
[D4](#d4--our-own-text-rendering), answers Q24, Q25, Q26 and Q28.*

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

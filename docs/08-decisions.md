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

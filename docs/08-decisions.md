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
- **Q24** — The platform has one `EventLoop` per UI thread, with any number of windows whose events carry a
  `WindowId`, and `Window::open(&event_loop, &options)`. Doc 02 says "the event loop of each window": is one loop for
  all the windows of the UI thread what is meant?
- **Q25** — `ui` may not depend on `platform` (`ui → render, editor`), so it has input types of its own, which `app`
  translates from `platform::InputEvent`, and it hands back the cursor, the text to copy and the caret's place for the
  input method as the output of a frame. Paste comes in as an event carrying the text. Should `ui` be allowed to use
  `platform`'s input types instead?
- **Q26** — The platform swallows F10 and Alt pressed alone, so that Windows does not enter the menu mode of a menu bar
  Gitardo does not have; Alt+F4 and Alt+Space still reach the system. Should that be the application's choice?
- **Q27** — The colors of the five themes and the names of their tokens (`render::theme`) are the renderer's own
  design, checked only for contrast (4.5:1, and 7:1 in high contrast): they need a review from the product side.
- **Q28** — The renderer reads TrueType outlines only: a font of CFF outlines (`.otf`) is refused and the next font of
  the list is used. GPOS is read for pair kerning only (the first glyph's advance), with no shaping and no
  right-to-left text.
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

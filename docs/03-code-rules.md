# Code rules

These rules are what keeps Gitardo Desktop easy to read, to change and to trust years from now. Each has a number, so
that a review can say which one a change breaks. A rule that a tool can check is checked by a tool (`rustfmt`,
`clippy`, `cargo xtask check`); the others are checked in review. A change that needs to break a rule changes this page
first, in its own commit, with the reason.

## 1. Dependencies

- **R1.1** A crate may depend on a library from crates.io, in `[dependencies]`, `[dev-dependencies]` or
  `[build-dependencies]`, when it does the job better than code written here would, weighed as
  [D22](08-decisions.md#d22--libraries-with-judgment) says: what it saves against what it brings, whether one already
  used does the job, whether it is kept up, what it needs to build and run, the fewest features. Its licence is one MIT
  code may use. The commit that adds it says why. No crate depends on a folder outside the workspace. A pure crate
  uses only libraries that do no I/O, start no thread and read no clock.
- **R1.2** No `build.rs` in the crates of the workspace, and no procedural macro written here; those of libraries are
  theirs. `macro_rules!` only where it removes repetition that a function cannot, and never to hide control flow.
- **R1.3** The edges between crates are those of [02-architecture.md](02-architecture.md#who-may-use-whom). A new edge
  changes that picture first.
- **R1.4** External programs: only `git`, only through `crates/git`.
- **R1.5** TLS ([D13](08-decisions.md#d13--one-library-for-tls)): `rustls` with `ring`, and
  `rustls-native-certs`, used by `crates/tls` only, with only the features named there. No type of theirs appears in
  the interface of `crates/tls`, and no other library does TLS or cryptography.
- **R1.6** A library is used by the crate whose job it serves, and no type of it appears in that crate's interface, so
  that replacing it changes one crate: the window, the clipboard and the dialogs in `crates/platform`, shapes and text
  in `crates/render` ([D16](08-decisions.md#d16--libraries-for-the-window-the-text-and-the-pixels)). Where a type has
  to cross, as the tree of `accesskit` does between `ui` and `platform`, the code says why.

## 2. Features are functions

- **R2.1** Every feature of the catalog is one public function whose name comes from its identifier
  (`branch.create` → `features::branch::create`), with `/// Feature: branch.create` in its doc comment. One feature
  per file, named after the action: `crates/features/src/branch/create.rs`.
- **R2.2** An operation has the signature `fn(context: &Context, answers: &Answers) -> Outcome` and is pure: no I/O,
  no clock, no environment, no randomness, no threads, no global. What it needs comes in `context`, or is asked for
  with `Outcome::Read`.
- **R2.3** An operation changes nothing itself. Everything it does is a `Step` of its `Plan`. Only `engine` runs steps.
- **R2.4** A view has the signature `fn(state: &State, ui: &mut Ui) -> Vec<Intent>`. It draws and reports; it does not
  change state, run anything or call a feature.
- **R2.5** A read is split into `<what>_arguments(request)`, which builds the git arguments, and
  `parse_<what>(output)`, which parses them. Both are pure; only `read_<what>` runs git.
- **R2.6** Every line of a card's acceptance list has a test whose name starts with the feature's identifier
  (`branch_create__…`). `cargo xtask check` matches cards, functions and tests. Rust does not count the double
  underscore as snake case, so a module of such tests says
  `#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]`.

## 3. Purity and effects

- **R3.1** Pure crates (`model`, `text`, `features`, `json`) have `#![forbid(unsafe_code)]` and use nothing from
  `std::fs`, `std::process`, `std::net`, `std::thread`, `std::env` or `std::time::SystemTime::now`.
  `cargo xtask check` greps for them.
- **R3.2** Time is a value: `Context::now` and `Clock` passed in. Never `SystemTime::now()` outside `app` and
  `platform`.
- **R3.3** No global mutable state: no `static mut`, no `static` with interior mutability, no lazily initialized
  singletons. What is shared is passed.
- **R3.4** Output that a person or a test compares is deterministic: no iteration over a `HashMap` or `HashSet` reaches
  the screen, the log or a file. Use `BTreeMap`, or sort.

## 4. Errors

- **R4.1** No panic in shipped code: `unwrap`, `expect`, `panic!`, `unreachable!`, `todo!`, `unimplemented!` and
  indexing that can go out of bounds are denied by clippy outside `#[cfg(test)]`. Use `get`, `?`, and types that make
  the bad case impossible.
- **R4.2** Each crate has one error enum, `Error`, with a variant per thing that can go wrong and the data the user needs
  to understand it. No `String` errors, no `Box<dyn Error>` across crates.
- **R4.3** An error that reaches the user is said in words of the domain, what happened and what to do:
  *Git refused to delete feature/login because it is not merged. Delete it anyway?*, not *exit status 1*. What git
  printed is kept with it and shown under **Details**.
- **R4.4** Errors are not swallowed. A failure that the program decides to ignore is logged at the debug level with the
  reason it is ignored, in a comment beside it.
- **R4.5** A failing step stops its plan, and the steps already run are listed with the failure. Nothing goes on as if
  it had worked. The links and copies a worktree gets are the one exception, which their card makes
  (`worktree-setup.link-and-copy-steps`): the others are still made, and those that failed are said together at the
  end.

## 5. Unsafe and the operating system

- **R5.1** `unsafe` only in `crates/platform`. Every other crate has `#![forbid(unsafe_code)]`.
- **R5.2** Every `extern` block is in one file per system library (`platform/src/windows/kernel32.rs`), with only the
  functions, types and constants used, each with the name the system documentation gives it. What a library of R1.6
  does (the window, the input, the clipboard) is not asked of the system a second time.
- **R5.3** Every `unsafe` block is as small as it can be and has a `// SAFETY:` comment saying why each requirement of the
  call holds. Clippy's `undocumented_unsafe_blocks` is denied.
- **R5.4** Unsafe calls are wrapped at once in safe functions that own what they create (`Drop` releases it) and return
  `Result`. No raw pointer or handle leaves `platform`.

## 6. Size and shape

- **R6.1** A function has at most 60 lines, 5 parameters and 3 levels of nesting. More parameters become a struct with a
  name; more nesting becomes a function with a name.
- **R6.2** A file has at most 500 lines, tests included in a separate `tests.rs` beside it when they are long.
- **R6.3** A module does one thing that its name says. `utils`, `helpers`, `common`, `misc` are not names.
- **R6.4** Public items are few: `pub` only for what another crate uses, `pub(crate)` for the rest of the crate.
- **R6.5** Types carry meaning: `CommitId`, `BranchName`, `RepositoryPath`, not `String`. A function that takes two
  values of the same type in a row takes a struct instead, so that they cannot be swapped.
- **R6.6** No boolean parameters that change what a function does: two functions, or an enum with named variants.
- **R6.7** Enums are matched exhaustively; `_ =>` only where a new variant can truly be treated like the others, with a
  comment saying so.

## 7. Names

- **R7.1** Names are words, not abbreviations: `entry`, `commit`, `reference`, `repository`, not `e`, `c`, `rf`, `repo`
  (except in identifiers of features and settings, which are the product's). Loop variables too.
- **R7.2** Types and modules are nouns; functions are verbs, or nouns for what they return (`parse_branches`,
  `lanes`). A function that returns a `bool` reads as a question: `is_merged`, `has_upstream`.
- **R7.3** The words are those of git and of the catalog: a *reference* is a branch, a remote branch or a tag; the
  *upstream* of a branch; the *working tree*; the *index*; *what is checked out*.
- **R7.4** No negated names: `is_hidden`, not `is_not_shown`.

## 8. Comments and documentation

- **R8.1** Every public item has a doc comment. Comments are sentences in plain words that say what a thing is, or why
  it is so, from what the user sees: `/// The commits of what is checked out, newest first.` Not how the code does it.
- **R8.2** A comment that explains a surprising line says why: the git version that needs it, the operating system that
  behaves so, the card it implements.
- **R8.3** An algorithm from a publication names it: `/// Myers, "An O(ND) Difference Algorithm and Its Variations", 1986.`
- **R8.4** No commented-out code, no `TODO` without the number of an open question in
  [08-decisions.md](08-decisions.md).
- **R8.5** A change a user can see changes its card in the same commit. A card is never behind the code.

## 9. Git

- **R9.1** Git runs with arguments, never through a shell, and with `--` before paths.
- **R9.2** A path that git could read as a pattern is passed as `:(literal)<path>`; a branch that shares its name with a
  tag or a remote branch is passed as `refs/heads/<name>`.
- **R9.3** Reads use machine formats (`-z`, `--porcelain=v2`, `--format` with `%x00`), `GIT_OPTIONAL_LOCKS=0`,
  `core.quotepath=false`, and the C locale for anything parsed. Reads never ask a question and never take a lock.
- **R9.4** Commands that would open an editor are given the flag that prevents it, visibly: `--no-edit`, or
  `-c core.editor=true` when continuing. The commands shown in the preview are exactly the commands that run.
- **R9.5** Every git version requirement is a constant with the version and the reason, and the feature says what it does
  on an older git.

## 10. User interface

- **R10.1** UI text is short, plain and in sentence case, and says what will happen: **Take the version of main into
  the working tree**. A tooltip finishes the thought of its button. The text is the card's, word for word.
- **R10.2** All UI text is in `crates/views/src/text/`, one module per area, so that it can be read in one place and
  translated later.
- **R10.3** Colors, sizes and spacing come from the theme's tokens, never a literal in a view. Every view is looked at in
  the dark, light and high contrast themes.
- **R10.4** Everything a mouse can do, a keyboard can do. Every interactive widget has a label for accessibility.
- **R10.5** The UI thread never waits: no git, no disk, no network on it. A frame is drawn in under 8 ms on the reference
  machine; work that might not be moves to a worker.

## 11. Tests

- **R11.1** Pure code is tested with plain inputs and outputs, as tables of cases.
- **R11.2** Parsers are tested on output recorded from real git into `tests/golden/`, including the awkward cases:
  names with spaces, accents, brackets and leading dashes, an empty repository, a detached `HEAD`, a shallow clone, a
  merge of three parents, a branch and a tag of the same name.
- **R11.3** Operations are tested twice: as a function (context and answers in, outcome out), and as a plan run by the
  engine against a throwaway repository, checking what git then says.
- **R11.4** Views are tested by driving them with intents and events and comparing the pixels they draw with reference
  snapshots, in each theme.
- **R11.5** A bug fixed gets a test that failed before the fix.
- **R11.6** Tests are deterministic: a random source is seeded and the seed printed on failure; no test depends on the
  clock, the network or the order tests run in.

## 12. Formatting and lints

- **R12.1** `rustfmt` with the `rustfmt.toml` at the root: `max_width = 120`, `edition = "2024"`,
  `newline_style = "Unix"`, `use_field_init_shorthand = true`. Comments wrap at about 120 columns.
- **R12.2** `clippy` with the lints of `[workspace.lints]` in the root `Cargo.toml`, warnings denied:

  ```toml
  [workspace.lints.rust]
  unsafe_code = "deny"            # allowed again only in crates/platform
  missing_docs = "deny"
  unused_results = "deny"

  [workspace.lints.clippy]
  pedantic = { level = "deny", priority = -1 }
  unwrap_used = "deny"
  expect_used = "deny"
  panic = "deny"
  indexing_slicing = "deny"
  todo = "deny"
  unimplemented = "deny"
  dbg_macro = "deny"
  print_stdout = "deny"
  print_stderr = "deny"
  undocumented_unsafe_blocks = "deny"
  too_many_lines = "deny"         # with too-many-lines-threshold = 60 in clippy.toml
  too_many_arguments = "deny"     # with too-many-arguments-threshold = 5
  cognitive_complexity = "deny"
  ```

- **R12.3** Files use LF line endings and UTF-8. `.gitattributes` says `* text=auto eol=lf`.

## 13. Changes and commits

- **R13.1** One commit per piece of work. Its message is one sentence that says what changed for the user, in the
  imperative, with no prefix: `Show the branches checked out last above the others`.
- **R13.2** A commit passes `cargo xtask check`, which runs `rustfmt --check`, `clippy`, the tests and the checks of the
  catalog.
- **R13.3** A change a user can see changes its card and the **Unreleased** section of `CHANGELOG.md` in the same commit.
- **R13.4** Never push or publish from an agent session: the maintainer does.

## What `cargo xtask check` checks

| Check | Rules |
| --- | --- |
| No dependency on a folder outside the workspace; no `build.rs` and no procedural macro in the workspace | R1.1, R1.2 |
| The edges between crates are the allowed ones | R1.3 |
| Every function that says `/// Feature:` has a card, in the crate and with the name the card gives; every card the index marks *Built* or *Seen* has its function and at least as many tests named after it as its acceptance list has lines; every test named `<feature>__…` has a card; the index and the cards list the same features, of the same kinds | R2.1, R2.6 |
| Pure crates use no I/O, time, threads or environment | R3.1, R3.2 |
| `unsafe` appears only under `crates/platform`, each block with `// SAFETY:` | R5.1, R5.3 |
| Files are at most 500 lines | R6.2 |
| `rustfmt --check`, `clippy -- -D warnings`, `cargo test` | R12, R11 |
| Line endings are LF | R12.3 |

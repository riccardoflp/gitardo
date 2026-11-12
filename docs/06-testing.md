# Testing

Every line of every card's acceptance list is a test. This page says what kinds of tests there are, where they live and
how they run, all with the standard test harness of Rust and no testing library.

## The kinds, from cheapest to most real

| Kind | Tests | Where | Runs |
| --- | --- | --- | --- |
| **Unit** | Pure functions: derivations, parsers on recorded output, operations as tables of context and answers to outcome. | `#[cfg(test)] mod tests` beside the code, or `tests.rs` beside the module. | `cargo test`, in milliseconds. |
| **Golden** | Parsers and formatters against files recorded from real git: the output of each read in each awkward repository. | `crates/git/tests/golden/<read>/<case>.out` and `.expected`. | `cargo test`; `cargo xtask golden --record` records again with the git installed and shows the differences. |
| **Integration** | Plans run by the engine against throwaway repositories, checking what git says after. The features are run by `gitardo-cli`, through the engine, since `features` cannot reach the engine itself. | `crates/engine/tests/`; `crates/app/tests/cli/` for the features. | `cargo test`, in seconds; needs `git` on the `PATH`. |
| **Network** | `crates/http` against a local server on `127.0.0.1` started by the test: plain HTTP for requests, chunked bodies, server-sent events, redirects and proxies; and `crates/tls` against a local TLS server of the same library, with a test certificate authority kept in `testkit/certificates/`, for a trusted certificate, an untrusted one and a wrong host name. | `crates/http/tests/`, `crates/tls/tests/`. | `cargo test`; nothing leaves the machine. |
| **Views** | A view driven with intents and events, its pixels compared with a reference snapshot, in each theme. | `crates/views/tests/`, snapshots in `crates/views/tests/snapshots/<view>/<case>-<theme>.png`. | `cargo test`; `cargo xtask snapshots --accept` takes the new pixels as the reference, after they are looked at. |
| **Application** | The whole program in a hidden window, driven by synthetic input through the platform layer, against throwaway repositories. | `crates/app/tests/`. | `cargo xtask app-tests`, on each system. |
| **Performance** | Times of what a person waits for, on a large generated repository. Not pass or fail: numbers to read, kept between releases. | `xtask/src/perf/`. | `cargo xtask perf <folder>`. |

## Throwaway repositories

`testkit::Repository::new()` makes an empty repository in a folder of its own under the temporary folder, with a fixed
identity, fixed dates (`GIT_AUTHOR_DATE` and `GIT_COMMITTER_DATE` advanced one minute per commit) and
`core.autocrlf=false`, and deletes it when dropped. It has helpers in the words of the domain: `commit("Add the parser",
&[("src/parser.rs", "…")])`, `branch("feature")`, `merge("feature")`, `conflict()`. A `Remote` is a bare repository
beside it.

The awkward repositories, used by every read's golden tests:

- file names with spaces, accents, brackets, a leading dash, and one that is also a pathspec pattern (`pages/[id].tsx`);
- a repository with no commits; a detached `HEAD`; a shallow clone; several root commits; a merge of three parents;
- a branch and a tag of the same name; a branch and a remote branch of the same name;
- a submodule, and a submodule inside it; a worktree; a worktree whose folder is gone;
- a merge, a rebase, a cherry-pick, a revert and a `git am` stopped on a conflict;
- a file deleted on one side and changed on the other; a file added on both sides; a binary file in conflict.

## Determinism

- The random source is `testkit::Random`, a xorshift generator. A test that uses it prints its seed when it fails, and
  `GITARDO_TEST_SEED=<seed>` runs it again with that seed.
- Times are given, never read: `Context::now` is fixed in tests.
- Snapshots are drawn with the fonts that come with the program (`crates/render/fonts/`), with no fallback to the fonts
  of the system, at scale 1 and 2, so that they are the same on every machine.
- PNG files are written and read by `testkit` itself: deflate with fixed codes to write, a full inflate to read. A
  snapshot that differs writes the actual image and an image of the differences beside the reference
  (`testkit::png::assert_snapshot`).

## Conflicts against git

The rules that resolve conflicts with no choice are tested against git itself: random files made mostly of lines that
repeat (closing braces, empty lines), changed at random on two branches, merged by git, and resolved by the rules. A
resolution is right when it is the result git gives if the two sides are merged in a way that does not conflict (the
same change applied in two steps). Hundreds of cases run in every `cargo test`, thousands in `cargo xtask fuzz`.

## What a change runs before it is committed

`cargo xtask check`, which is `rustfmt --check`, `clippy` with warnings denied, `cargo test`, and the checks of the
catalog and of the rules in [03-code-rules.md](03-code-rules.md#what-cargo-xtask-check-checks). A change to a view also
runs `cargo xtask snapshots`, and someone looks at the images that changed.

## What no test reaches

Signing in to GitHub with a real account, a real language model, and the dialogs of the operating system (choosing a
folder, a file). They are behind interfaces that the tests replace with fakes that record what they were asked; the real
ones are tried by hand before a release, from the checklist in [07-roadmap.md](07-roadmap.md#before-a-release).

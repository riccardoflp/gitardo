# Testing

Every line of every card's acceptance list is a test. This page says what kinds of tests there are, where they live and
how they run, with the standard test harness of Rust, and a testing library where one serves, as
[D22](../08-decisions.md#d22--libraries-with-judgment) says.

## The kinds, from cheapest to most real

| Kind | Tests | Where | Runs |
| --- | --- | --- | --- |
| **Unit** | Pure functions: derivations, parsers on recorded output, operations as tables of context and answers to outcome. | `#[cfg(test)] mod tests` beside the code, or `tests.rs` beside the module. | `cargo test`, in milliseconds. |
| **Golden** | Parsers and formatters against files recorded from real git: the output of each read in each awkward repository. | `crates/git/tests/golden/<read>/<case>.out` and `.expected`. | `cargo test`; `cargo xtask golden --record` records again with the git installed and shows the differences. |
| **Integration** | Plans run by the engine against throwaway repositories, checking what git says after. The features are run by `gitardo-cli`, through the engine, since `features` cannot reach the engine itself. | `crates/engine/tests/`; `crates/app/tests/cli/` for the features. | `cargo test`, in seconds; needs `git` on the `PATH`. |
| **Network** | `crates/http` against a local server on `127.0.0.1` started by the test (`http::test_server`): plain HTTP and HTTPS for requests, chunked bodies, server-sent events, redirects, proxies, cancelling and timeouts; and `crates/tls` against a local TLS server of the same library, with a test certificate authority kept in `crates/tls/fixtures/`, for a trusted certificate, an untrusted one and a wrong host name. The crates above reach the same server through the feature `test-server` of `http`, which only their dev-dependencies turn on, and send requests for `api.github.com` to it with `Http::with_address`. | `crates/http/src/tests.rs`, `crates/tls/src/tests.rs`, and the tests of the reads that send requests. | `cargo test`; nothing leaves the machine. |
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

## Scenarios run by both apps

While the extension moves onto the engine ([transition/06-roadmap.md](../transition/06-roadmap.md)), each operation of
an area that moves gets a scenario: a throwaway repository, a target and the answers to the operation's questions,
written once as JSON in `rust/crates/app/tests/scenarios/`. The scenario runs twice, through `gitardo-cli` and through
the extension's TypeScript `GitOp`, and the test compares the two plans (the commands, in order) and the two
repositories after (the references, `HEAD`, the index and the files of the working tree). A difference fails the test
unless it is one of [transition/04-differences.md](../transition/04-differences.md), named in the scenario. Once the
area has moved, the scenario runs the engine through `gitardo-engine --stdio` from the extension's tests, and the
TypeScript side is dropped with `src/git`.

The protocol has its own tests: each type of `rust/crates/protocol` is turned into JSON and back, and every variant of
every enum of [transition/02-protocol.md](../transition/02-protocol.md) has its JSON written once in a golden file.

## The engine on a large repository

`cargo xtask perf <folder> [times]` times the reads of the graph in `gitardo-engine --stdio`, the engine the extension
starts, built first with `cargo build --release -p server`. It frames the requests of JSON-RPC itself, with no other
crate, and times each one from the request to the last byte of its answer, as the extension waits for it: the first
page of 1,000 rows, the next three pages and the 50th, each asked with `skip` from the last row before it, the position
of the last commit of the 50th page, a search of the messages and one of the authors, and `repo.read-state`. Each read
is asked five times unless it says otherwise; the table gives the rows, the bytes of the answer and the fastest, median
and slowest time. It is not pass or fail. The repository is the one `extension/test/perf/big-repo.mjs` makes, outside
the repository of Gitardo ([extension/06-testing.md](../extension/06-testing.md#large-repositories)).

What stage 7 measured, on 2026-11-15, in its own run and not on the maintainer's machine: Windows 11, git
2.53.0.windows.1, the release build, `node extension/test/perf/big-repo.mjs <folder> 50000 1779` (56,120 commits,
249 branches never merged, 2,000 tags, 2,375 references, 1,779 changed files). The median, in milliseconds, first of
five runs on the repository as the script leaves it, then of ten after `git pack-refs --all` and
`git commit-graph write --reachable`, as the extension's large repository is measured. Other work ran on the machine
at the same time, so the slowest runs were up to half as long again as the median:

| Read | Rows | Bytes | As made | With a commit-graph |
| --- | ---: | ---: | ---: | ---: |
| `graph.read-commits`, page 1 | 1,000 | 253,667 | 359.9 | 167.0 |
| `graph.read-commits`, page 2 with `skip` | 1,001 | 253,870 | 361.6 | 209.0 |
| `graph.read-commits`, page 3 with `skip` | 1,001 | 253,864 | 369.2 | 198.8 |
| `graph.read-commits`, page 4 with `skip` | 1,001 | 253,755 | 358.7 | 203.8 |
| `graph.read-commits`, page 50 with `skip` | 1,001 | 253,707 | 329.8 | 177.2 |
| `graph.read-position`, a commit of page 50 | | 50 | 391.4 | 304.8 |
| `search.read-matches`, the messages, "the parser" | 5,000 | 215,063 | 348.3 | 316.7 |
| `search.read-matches`, the authors, "Grace" | 210 | 9,094 | 353.8 | 425.5 |
| `repo.read-state` | | 290,451 | 278.6 | 314.7 |

Without a commit-graph, `git log` alone takes about 310 ms to list the first 1,001 commits of that repository, so a page
is git's time and some 50 ms of the engine's; with one, about 120 ms. The pages after the first take about as long,
since git walks the history from the top to reach them. The engine reads the references and the stashes only when the
branches shown or the stashes need them, which took a page of the repository as made from about 510 ms to 360 ms.

## What a change runs before it is committed

`cargo xtask check`, which is `rustfmt --check`, `clippy` with warnings denied, `cargo test`, and the checks of the
catalog and of the rules in [03-code-rules.md](03-code-rules.md#what-cargo-xtask-check-checks). A change to a view also
runs `cargo xtask snapshots`, and someone looks at the images that changed. A change to the engine that the extension
uses also runs the extension's `npm run test:e2e` against the debug binary.

## What no test reaches

Signing in to GitHub with a real account, a real language model, and the dialogs of the operating system (choosing a
folder, a file). They are behind interfaces that the tests replace with fakes that record what they were asked; the real
ones are tried by hand before a release, from the checklist in [07-roadmap.md](07-roadmap.md#before-a-release).

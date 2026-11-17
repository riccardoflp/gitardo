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
| **Performance** | Times of what a person waits for, on a large generated repository, and the git processes a read starts. Not pass or fail: numbers to read, kept between releases. | `xtask/src/perf/`, and a test of `crates/app/tests/places/speed.rs` it runs. | `cargo xtask perf <folder> [times] [--engine]`. |

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
area has moved, the scenario runs the engine through `gitardo-engine --stdio` from the extension's tests; since stage
10 the TypeScript side is gone with `src/git`, and the extension's scenarios check the engine alone.

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

Since milestone M15 it also times four things more
([the roadmap](07-roadmap.md#m15--the-whole-history-in-one-read-and-reads-that-cost-less)):

- **The whole history**, page after page as the extension asks for it, each page with `skip` from the last row before
  it until the engine says there are no more: the time to the first page and to the last row.
- **The branches shown changed**, as the extension asks for it with version 1.8 of the protocol: `graph.read-commits`
  with `stream`, for every branch, the current branch, the local branches and one local branch not checked out, each
  after a read of another history: the time to the first page, in its notification `read/graph.rows`, and to the
  answer, after the last row.
- **A read after a change**, as the extension asks for it once a file of the working tree changed: `repo.read-state`,
  the first page of `graph.read-commits` and `journal/last`, one after the other, a file of the working tree written
  before each (`gitardo-perf-change.txt`, removed at the end), ten times at least; and the git processes each read
  starts, counted from the events that git writes when `GIT_TRACE2_EVENT` names a file, each by its command. A git
  that another git starts is not counted.
- **The whole history in the window**, read and laid out as `app` does it: the folder opened, then every next page
  asked for as the list is scrolled to its end, by the test
  `speed::the_whole_history_of_the_perf_repository_read_and_laid_out_page_after_page` of `app`, built in release, on
  the repository `GITARDO_PERF_REPOSITORY` names; with the time the layouts of the timeline took, over every row read
  at each page; then the branches shown changed to the current branch, to the local branches and to one branch, each
  timed from the intent to its first page in the timeline and to its last row. `--engine` leaves it out, and with it
  the build of the tests of `app` in release.

What phase 1 of M15 measured, on 2026-11-17, before the rest of M15: the same machine and git as above, the release
build, on the large repository as the script leaves it (56,120 commits), and on a clone of the repository of Gitardo
(515 commits) with five other worktrees in `.claude/worktrees`. Three other agents built Rust on the machine at the
same time, so the slowest runs were up to twice the median; a second run an hour later gave medians up to twice these.

| The whole history | Pages | First page | Last row |
| --- | ---: | ---: | ---: |
| The engine, every page with `skip`, the large repository | 57 | 354.7 | 20,849.3 |
| The window, the large repository, opened and scrolled to its end | 57 | 654.9 | 26,661.9 |
| The window: of which the layouts of every page over every row before it | | | 1,847.8 |

| A read after a change, the median | The large repository | Git processes | Five other worktrees | Git processes |
| --- | ---: | --- | ---: | --- |
| `repo.read-state` | 267.4 | 11 (`status` 1, `rev-parse` 2, `config` 2, 6 others) | 638.2 | 16 (`status` 6, `rev-parse` 2, `config` 2, 6 others) |
| `graph.read-commits`, page 1 | 337.8 | 4 (`log` 1, `rev-parse` 1, `symbolic-ref` 1, `stash` 1) | 252.0 | 4 (the same) |
| `journal/last` | 263.5 | 11 (as `repo.read-state`) | 684.8 | 16 (as `repo.read-state`) |
| All three | 877.5 | 26 | 1,459.1 | 36 |

A page after the first costs the engine as much as the first, since git walks the history from the top for each: the
whole history is 57 walks. The window's layout of the timeline is made again over every row at each page, which
grows the same way. A read after a change reads the state twice, once for `journal/last`, and runs `git status` in
every worktree each time.

What the same measure gave on 2026-11-17 after phases 2, 3 and 7 of M15 (the history read with one `git log` as it
prints, each page laid out alone, the commits not read again when the history is the same), on the same repositories.
The machine was busier than for the measure before: the reads no phase changed, such as `repo.read-state`, took 1.4
to 1.8 times as long, so the times say less than the git processes. The engine still reads page after page with
`skip` for the extension, until it streams the pages too.

| The whole history | Pages | First page | Last row |
| --- | ---: | ---: | ---: |
| The engine, every page with `skip`, the large repository | 57 | 426.0 | 28,526.9 |
| The window, the large repository, opened with one read to its end | 57 | 1,747.5 | 4,359.7 |
| Laying out every page over every row before it, which the window no longer does | | | 4,508.4 |

| A read after a change, the median | The large repository | Git processes | Five other worktrees | Git processes |
| --- | ---: | --- | ---: | --- |
| `repo.read-state` | 473.7 | 11 (the same) | 860.4 | 16 (the same) |
| `graph.read-commits`, page 1 | 508.5 | 1 (`log` 1) | 134.3 | 1 (`log` 1) |
| `journal/last` | 494.8 | 11 (the same) | 957.0 | 16 (the same) |
| All three | 1,483.2 | 23 | 1,874.4 | 33 |

`git log` alone takes as long to print the first 1,001 commits of the large repository as it streams as with
`--max-count=1001`, so the first page of the window waits for git as before. A page of `graph.read-commits` takes
the references and the stashes from the last `repo.read-state` of the same session, which leaves `git log` alone. In
the window, a read for a file changed or saved runs no `git log` while the history is the same, which
`same_history.rs` of `app` checks, and which this measure does not time.

What the same measure gave on 2026-11-18 with all of M15 and M16 built, on the same repositories, with nothing else
building on the machine, so that the times can be read against those of phase 1:

| The whole history | Pages | First page | Last row |
| --- | ---: | ---: | ---: |
| The engine, every page with `skip`, the large repository | 57 | 300.2 | 17,207.3 |
| The window, the large repository, opened with one read to its end | 57 | 596.2 | 1,442.2 |

| A read after a change, the median | The large repository | Git processes | Five other worktrees | Git processes |
| --- | ---: | --- | ---: | --- |
| `repo.read-state` | 256.9 | 11 | 164.6 | 11 (`status` 1) |
| `graph.read-commits`, page 1 | 295.5 | 1 (`log` 1) | 36.2 | 1 (`log` 1) |
| `journal/last` | 0.5 | 0 | 0.3 | 0 |
| All three | 558.7 | 12 | 201.6 | 12 |

| The branches shown changed, the large repository, the median | Commits | First page | Last row |
| --- | ---: | ---: | ---: |
| The engine, streamed (protocol 1.8), every branch | 56,120 | 295.6 | 386.1 |
| The engine, streamed, the current branch | 55,355 | 170.8 | 255.9 |
| The engine, streamed, the local branches | 56,120 | 185.4 | 271.2 |
| The engine, streamed, one branch (`feature/001-export`) | 230 | 26.3 | 26.4 |
| The window, the current branch | 55,355 | 186.8 | 1,019.5 |
| The window, the local branches | 56,120 | 203.2 | 1,046.8 |
| The window, one branch (`feature/001-export`) | 230 | 46.6 | 46.6 |

Each read of the engine follows a read of another history, as the extension asks one when the branches shown change;
each change of the window follows the whole history read, and is timed from the intent to the first page in the
timeline. Another branch filter shows its first page in less than a third of the time the first page took before M15
(654.9 ms in the window, 354.7 ms in the engine), and every branch streamed shows it no later than a page did then.

The window reads and lays out every commit of the large repository in under 1.5 s, and shows its first page sooner
than before M15. A read after a change takes a seventh of the time of phase 1 on the repository with five other
worktrees, and two thirds of it on the large repository, where `repo.read-state` and the first page are git's own
time. The engine's whole history page after page with `skip` is what a client of version 1.7 of the protocol still
asks; the extension asks for the pages streamed (version 1.8), which phase 9 measured at 1.4 to 2.2 s for every commit
of the large repository, read and laid out.

## What a change runs before it is committed

`cargo xtask check`, which is `rustfmt --check`, `clippy` with warnings denied, `cargo test`, and the checks of the
catalog and of the rules in [03-code-rules.md](03-code-rules.md#what-cargo-xtask-check-checks). A change to a view also
runs `cargo xtask snapshots`, and someone looks at the images that changed. A change to the engine that the extension
uses also runs the extension's `npm run test:e2e` against the debug binary.

## What no test reaches

Signing in to GitHub with a real account, a real language model, and the dialogs of the operating system (choosing a
folder, a file). They are behind interfaces that the tests replace with fakes that record what they were asked; the real
ones are tried by hand before a release, from the checklist in [07-roadmap.md](07-roadmap.md#before-a-release).

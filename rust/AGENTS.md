# Working on Gitardo Desktop

Gitardo Desktop is a desktop application for Git, written in Rust with libraries where they serve, and its engine is
becoming the engine of the extension too ([D37](../docs/08-decisions.md#d37--one-engine-the-rust-one-for-both-apps)).
This file is for whoever changes this folder, person or agent; the rules of the whole repository are in
[../AGENTS.md](../AGENTS.md). What the program does is in [README.md](README.md) and in [docs/](../docs/README.md);
keep the two apart. Paths here are from `rust/`.

## Ground rules

- **Desktop first.** A feature is built here first, then in the extension
  ([D39](../docs/08-decisions.md#d39--desktop-first-and-a-status-for-each-app)). A change to the engine that the
  extension uses keeps [the protocol](../docs/transition/02-protocol.md) working: a field added is a minor version, a
  field changed or removed a major one, said in the crate `protocol`.
- **The clean room has ended** ([D40](../docs/08-decisions.md#d40--the-clean-room-ends)). The extension's code in
  `../extension/` may be read, to move its behavior here through a card and its tests, never by translating it line by
  line. The source of other projects is still never copied. When a card is not enough, write the question in
  [docs/08-decisions.md](../docs/08-decisions.md) under **Open questions** and stop there.
- **Libraries with judgment.** A library from crates.io may be added when it does the job better than code written
  here would, weighed as decision D22 says (what it saves against what it brings, whether it is kept up, what it needs
  to build), under a licence MIT code may use; the commit that adds it says why. Its types stay in the crate that uses
  it (rule R1.6), TLS and cryptography stay in `crates/tls` (rule R1.5), and nothing depends on a folder outside the
  repository.
- **Every feature is a function**, as [the feature contract](../docs/desktop/04-feature-contract.md) says, named after
  its card, with a test for each line of its acceptance list.
- **The code rules** of [docs/desktop/03-code-rules.md](../docs/desktop/03-code-rules.md) apply to every line.
  `cargo xtask check` must pass before a commit.
- Everything in the repository is in English: code, comments, UI text, documents, commit messages.
- Commit only when asked, one commit per piece of work. Never push or publish.
- A change a user can see changes its card, with its status in each app, and the **Unreleased** section of
  `CHANGELOG.md` in the same commit; the extension's CHANGELOG too when the extension shows it.
- An idea beyond what was asked is proposed first, not built.

## Commands

```bash
cargo build                 # every crate
cargo test                  # unit, golden and integration tests; needs git on the PATH
cargo xtask check           # rustfmt, clippy, tests, and the checks of the catalog and of the rules
cargo xtask snapshots       # the view snapshots; --accept takes the new pixels as the reference
cargo xtask golden --record # records git's output for the golden tests again
cargo run -p app            # the application
cargo run -p app --bin gitardo-cli -- <feature> <answers>…   # a feature without a window
```

## Where to start

1. Read [the product](../docs/desktop/01-product.md) and [the feature contract](../docs/desktop/04-feature-contract.md).
2. Find the stage in progress in [the transition's roadmap](../docs/transition/06-roadmap.md) (the milestones before it
   are in [docs/desktop/07-roadmap.md](../docs/desktop/07-roadmap.md)), and its cards in the
   [catalog](../docs/features/README.md).
3. Build one card at a time: the tests of its acceptance list, then its function, then mark it built in the index.

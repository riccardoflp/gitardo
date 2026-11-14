# Working on Gitardo Desktop

Gitardo Desktop is a desktop application for Git, written in Rust with libraries where they serve, in a clean room. This file is
for whoever changes this branch, person or agent. What the program does is in [README.md](README.md) and in
[docs/](docs/); keep the two apart.

## Ground rules

- **Clean room.** Work from the documents in `docs/` and from git's own documentation. Do not open the source of the
  Gitardo extension (`src/` and `test/` of the `main` branch), nor copy the source of any other project. The
  documentation of the libraries it uses may be read. When a card is not enough, write the question in [docs/08-decisions.md](docs/08-decisions.md) under
  **Open questions** and stop there. See [docs/00-clean-room.md](docs/00-clean-room.md).
- **Libraries with judgment.** A library from crates.io may be added when it does the job better than code written
  here would, weighed as decision D22 says (what it saves against what it brings, whether it is kept up, what it needs
  to build), under a licence MIT code may use; the commit that adds it says why. Its types stay in the crate that uses
  it (rule R1.6), TLS and cryptography stay in `crates/tls` (rule R1.5), and nothing depends on a folder outside the
  repository.
- **Every feature is a function**, as [docs/04-feature-contract.md](docs/04-feature-contract.md) says, named after its
  card, with a test for each line of its acceptance list.
- **The code rules** of [docs/03-code-rules.md](docs/03-code-rules.md) apply to every line. `cargo xtask check` must pass
  before a commit.
- Everything in the repository is in English: code, comments, UI text, documents, commit messages.
- Commit only when asked, one commit per piece of work. Never push or publish.
- A change a user can see changes its card and the **Unreleased** section of `CHANGELOG.md` in the same commit.
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

1. Read [docs/00-clean-room.md](docs/00-clean-room.md), [docs/01-product.md](docs/01-product.md) and
   [docs/04-feature-contract.md](docs/04-feature-contract.md).
2. Find the milestone in progress in [docs/07-roadmap.md](docs/07-roadmap.md), and its cards in the
   [catalog](docs/features/README.md).
3. Build one card at a time: the tests of its acceptance list, then its function, then mark it built in the index.

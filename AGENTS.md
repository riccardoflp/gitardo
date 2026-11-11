# Working on Gitardo Desktop

Gitardo Desktop is a desktop application for Git, written in Rust with no dependencies but one for TLS, in a clean room. This file is
for whoever changes this branch, person or agent. What the program does is in [README.md](README.md) and in
[docs/](docs/); keep the two apart.

## Ground rules

- **Clean room.** Work from the documents in `docs/` and from git's own documentation. Do not open the source of the
  Gitardo extension (`src/` and `test/` of the `main` branch), nor the source of any other project for a part you are
  building. When a card is not enough, write the question in [docs/08-decisions.md](docs/08-decisions.md) under
  **Open questions** and stop there. See [docs/00-clean-room.md](docs/00-clean-room.md).
- **No dependencies.** Only the standard library and the crates of this workspace. Never add a crate from crates.io,
  git or a path outside the repository. The one exception is TLS: `rustls` with `ring` and `rustls-native-certs`,
  used by `crates/tls` only, at the versions of `xtask/allowed-packages.txt` (rule R1.5, decision D13). Do not use the
  exception for anything else.
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

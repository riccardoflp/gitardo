# Working on Gitardo

Gitardo is a commit graph, history and everyday Git operations, in two apps that share this repository:

- **the extension for VS Code**, in `extension/` (TypeScript): the graph in the bottom panel, views of the Source
  Control side bar and editor tabs, published on the Marketplace;
- **Gitardo Desktop**, in `rust/` (a Cargo workspace): a window of its own, with the engine that runs every operation.

This file is for whoever changes their code, person or agent, and holds for both. What each app does for its users is
in its README ([the extension](extension/README.md), [Gitardo Desktop](rust/README.md)); how they are built, the rules
of their code and the catalog of their features are in [docs/](docs/README.md). Keep them apart. Working in `rust/`,
read [rust/AGENTS.md](rust/AGENTS.md) too.

## Ground rules

- Everything in the repository is in English: code, comments, UI text, READMEs, CHANGELOGs, the documents, commit
  messages.
- Commit only when asked. When several pieces of work are waiting, make one commit for each, not one for all of them.
  Never push or publish: the maintainer publishes on the Marketplace by hand.
- Gitardo has had other names. Do not write them anywhere, except where the 0.2.0 entry of the extension's CHANGELOG
  explains the change of name.
- A change a user of the extension can see goes into its README, where the feature is described, into the
  **Unreleased** section of its CHANGELOG, and into its card in `docs/features/extension/`, in the same commit. An area
  that has no cards yet gets the cards of the features the change touches
  ([D33](docs/08-decisions.md#d33--cards-first-for-the-areas-where-a-mistake-costs)). A change to Gitardo Desktop goes
  into its card in `docs/features/` and into `rust/CHANGELOG.md`.
- In the extension, everything that changes the repository is a `GitOp` run by the `Controller`, so that it is shown,
  logged and undone like the rest ([the feature contract](docs/extension/04-feature-contract.md)). A card's acceptance
  lines, and [the promises](docs/extension/04-feature-contract.md#the-promises-every-card-is-checked-against), are what
  a change is checked against.
- Use the words of the [glossary](docs/09-glossary.md), never those of its **Avoid** column.
- Follow the code rules of the part you change ([the extension](docs/extension/03-code-rules.md),
  [Gitardo Desktop](docs/desktop/03-code-rules.md)); a change that needs to break one changes that page first.
- A key of the workspace state, a setting or a file Gitardo leaves in `.git` keeps its name and meaning
  ([stored data](docs/extension/11-data.md)).
- An idea beyond what was asked is proposed first, not built.

## Commands

The extension's commands run in `extension/`:

```bash
cd extension
npm install
npm run build        # dist/: the extension, the webviews, the Flow Diff, the preview server, the e2e suite
npm run typecheck    # tsc, over src and test
npm test             # Vitest: unit tests and integration tests against throwaway repositories
npm run check:docs   # the extension's catalog: cards, identifiers, glossary words (needs Python 3)
npm run test:e2e     # builds, then runs test/e2e/suite.ts inside a real VS Code
npm run test:ui      # builds, then drives a real VS Code window with clicks and keys; screenshots in dist/ui
npm run preview -- <folder>...   # the webviews in a browser, against the repositories in the folders
npm run package      # a minified build packed into gitardo-<version>.vsix
```

Gitardo Desktop's run in `rust/`:

```bash
cd rust
cargo build                 # every crate
cargo test                  # unit, golden and integration tests; needs git on the PATH
cargo xtask check           # rustfmt, clippy, tests, and the checks of the catalog in docs/features and of the rules
cargo run -p app            # the application
```

`F5` in VS Code, with the root of the repository open, launches an Extension Development Host (`.vscode/launch.json`).

`npm test` collects only the tests under `extension/`, so the worktrees under `.claude/worktrees` no longer add theirs.
A Rust build takes 10 to 15 GB for each target: `CARGO_PROFILE_DEV_DEBUG=0` keeps it smaller, and a `target` folder
that is no longer needed is deleted.

Which check sees which change, and what was learned the slow way about running them, is in
[the extension's testing](docs/extension/06-testing.md) and [Gitardo Desktop's](docs/desktop/06-testing.md).

## Where to start

1. [docs/README.md](docs/README.md): which document says what, for each app and for both.
2. The architecture of the part you change: [the extension](docs/extension/02-architecture.md),
   [Gitardo Desktop](docs/desktop/02-architecture.md).
3. Its feature contract ([the extension](docs/extension/04-feature-contract.md),
   [Gitardo Desktop](docs/desktop/04-feature-contract.md)), the card of the feature you change, from
   [docs/features/](docs/features/README.md) or [docs/features/extension/](docs/features/extension/README.md), and its
   tests.
4. The code rules before writing code, and the testing page before calling it done.

## How the code is written, in short

Comments are sentences that say what a thing is or why, from what the user sees; names are words, not abbreviations;
UI text is short, plain, in sentence case and says what will happen; a commit message is one sentence in the
imperative, with no prefix; files use LF. TypeScript is laid out as Prettier would with a width of 160, Rust by
`rustfmt` with a width of 120; comments wrap at about 120.

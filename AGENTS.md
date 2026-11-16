# Working on Gitardo

Gitardo is a commit graph, history and everyday Git operations, in two apps that share this repository:

- **Gitardo Desktop**, in `rust/` (a Cargo workspace): a window of its own, and the engine that runs every operation;
- **the extension for VS Code**, in `extension/` (TypeScript): the graph in the bottom panel, views of the Source
  Control side bar and editor tabs, published on the Marketplace.

The two are moving to one engine, the Rust one, with the extension as an interface of VS Code over it
([D37](docs/08-decisions.md#d37--one-engine-the-rust-one-for-both-apps)); how, and in which order, is in
[docs/transition/](docs/transition/README.md).

This file is for whoever changes their code, person or agent, and holds for both. Working in a folder, read its own
too: [rust/AGENTS.md](rust/AGENTS.md), [extension/AGENTS.md](extension/AGENTS.md). What each app does for its users is
in its README ([Gitardo Desktop](rust/README.md), [the extension](extension/README.md)); how they are built, the rules
of their code and the catalog of their features are in [docs/](docs/README.md). Keep them apart.

## Ground rules

- **Desktop first.** A feature is built in the engine and the window of Gitardo Desktop first, with its card, its
  tests and `rust/CHANGELOG.md`; then in the extension, with its interface, its tests and `extension/CHANGELOG.md`. A
  card may be Built in the desktop and Planned in the extension, never Built in the extension while it is not in the
  desktop, unless a decision says why ([D39](docs/08-decisions.md#d39--desktop-first-and-a-status-for-each-app)). A fix
  of a behavior both apps have is made in the engine.
- Everything in the repository is in English: code, comments, UI text, READMEs, CHANGELOGs, the documents, commit
  messages.
- Commit only when asked. When several pieces of work are waiting, make one commit for each, not one for all of them.
  Never push, tag or publish: the maintainer pushes the tag of a release, and the pipeline publishes it once they
  approve ([D52](docs/08-decisions.md#d52--a-tag-publishes-the-release-once-the-maintainer-approves)).
- Gitardo has had other names. Do not write them anywhere, except where the 0.2.0 entry of the extension's CHANGELOG
  explains the change of name.
- A change a user can see goes, in the same commit, into its card in the [catalog](docs/features/README.md) with the
  status of each app, into the **Unreleased** section of the CHANGELOG of each app it reaches, and into that app's
  README where the feature is described.
- Use the words of the [glossary](docs/09-glossary.md), never those of its **Avoid** column.
- Follow the code rules of the part you change ([Gitardo Desktop](docs/desktop/03-code-rules.md),
  [the extension](docs/extension/03-code-rules.md)); a change that needs to break one changes that page first.
- A key of the workspace state, a setting or a file Gitardo leaves in `.git` keeps its name and meaning
  ([stored data](docs/extension/11-data.md), [in both apps](docs/transition/05-stored-data.md)).
- An idea beyond what was asked is proposed first, not built.

## Commands

```bash
cd rust
cargo xtask check    # rustfmt, clippy, the tests, the rules of the code, and the catalog of docs/features
cargo run -p app     # Gitardo Desktop
```

```bash
cd extension
npm install
npm run build        # dist/: the extension, the webviews, the Flow Diff, the preview server, the e2e suite
npm test             # Vitest: unit tests and integration tests against throwaway repositories
npm run test:e2e     # builds, then runs test/e2e/suite.ts inside a real VS Code
```

The other commands of each part are in its AGENTS.md. `F5` in VS Code, with the root of the repository open, launches
an Extension Development Host (`.vscode/launch.json`).

A Rust build takes 10 to 15 GB for each target: `CARGO_PROFILE_DEV_DEBUG=0` keeps it smaller, and a `target` folder
that is no longer needed is deleted.

## Where to start

1. [docs/README.md](docs/README.md): which document says what, for each app and for both.
2. [docs/transition/README.md](docs/transition/README.md): where the move to one engine stands, and its next stage.
3. The architecture and the feature contract of the part you change, the card of the feature from
   [docs/features/](docs/features/README.md), and its tests.
4. The code rules before writing code, and the testing page before calling it done.

## How the code is written, in short

Comments are sentences that say what a thing is or why, from what the user sees; names are words, not abbreviations;
UI text is short, plain, in sentence case and says what will happen; a commit message is one sentence in the
imperative, with no prefix; files use LF. Rust is laid out by `rustfmt` with a width of 120, TypeScript as Prettier
would with a width of 160; comments wrap at about 120.

# Working on Gitardo

Gitardo is a VS Code extension: a commit graph, history and everyday Git operations, in the bottom panel, in views of the
Source Control side bar and in editor tabs. This file is for whoever changes its code, person or agent. What Gitardo
does for its users is in [README.md](README.md); how it is built, the rules of its code and the catalog of its features
are in [docs/](docs/README.md). Keep the three apart.

## Ground rules

- Everything in the repository is in English: code, comments, UI text, README, CHANGELOG, the documents, commit
  messages.
- Commit only when asked. When several pieces of work are waiting, make one commit for each, not one for all of them.
  Never push or publish: the maintainer publishes on the Marketplace by hand.
- Gitardo has had other names. Do not write them anywhere, except where the 0.2.0 entry of the CHANGELOG explains the
  change of name.
- A change a user can see goes into the README, where the feature is described, into the **Unreleased** section of
  the CHANGELOG, and into its card in `docs/features/`, in the same commit. An area that has no cards yet gets the cards
  of the features the change touches ([D11](docs/08-decisions.md#d11--cards-first-for-the-areas-where-a-mistake-costs)).
- Everything that changes the repository is a `GitOp` run by the `Controller`, so that it is shown, logged and undone
  like the rest ([the feature contract](docs/04-feature-contract.md)). A card's acceptance lines, and
  [the promises](docs/04-feature-contract.md#the-promises-every-card-is-checked-against), are what a change is checked
  against.
- Use the words of the [glossary](docs/09-glossary.md), never those of its **Avoid** column.
- Follow the [code rules](docs/03-code-rules.md); a change that needs to break one changes that page first.
- A key of the workspace state, a setting or a file Gitardo leaves in `.git` keeps its name and meaning
  ([stored data](docs/11-data.md)).
- An idea beyond what was asked is proposed first, not built.

## Commands

The extension is in `extension/`; its commands run there.

```bash
cd extension
npm install
npm run build        # dist/: the extension, the webviews, the Flow Diff, the preview server, the e2e suite
npm run typecheck    # tsc, over src and test
npm test             # Vitest: unit tests and integration tests against throwaway repositories
npm run check:docs   # the catalog of features: cards, identifiers, glossary words (needs Python 3)
npm run test:e2e     # builds, then runs test/e2e/suite.ts inside a real VS Code
npm run test:ui      # builds, then drives a real VS Code window with clicks and keys; screenshots in dist/ui
npm run preview -- <folder>...   # the webviews in a browser, against the repositories in the folders
npm run package      # a minified build packed into gitardo-<version>.vsix
```

`F5` in VS Code, with the root of the repository open, launches an Extension Development Host (`.vscode/launch.json`).

`npm test` collects only the tests under `extension/`, so the worktrees under `.claude/worktrees` no longer add theirs.

Which check sees which change, and what was learned the slow way about running them, is in
[docs/06-testing.md](docs/06-testing.md).

## Where to start

1. [docs/02-architecture.md](docs/02-architecture.md): the folders, who may use whom, and how a click becomes git
   commands.
2. [docs/04-feature-contract.md](docs/04-feature-contract.md): what a feature is in the code, and the list of areas.
3. The card of the feature you change, from [docs/features/](docs/features/README.md), and its tests.
4. [docs/03-code-rules.md](docs/03-code-rules.md) before writing code, and [docs/06-testing.md](docs/06-testing.md)
   before calling it done.

## How the code is written, in short

Comments are sentences that say what a thing is or why, from what the user sees; names are words, not abbreviations;
UI text is short, plain, in sentence case and says what will happen; a commit message is one sentence in the
imperative, with no prefix; code is laid out as Prettier would with a width of 160, comments wrap at about 120, files
use LF. The full list, numbered, is [docs/03-code-rules.md](docs/03-code-rules.md).

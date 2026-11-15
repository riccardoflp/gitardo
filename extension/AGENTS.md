# Working on the extension

The extension for VS Code, in this folder. The rules of the whole repository are in [../AGENTS.md](../AGENTS.md); these
are the ones of this folder. Paths here are from `extension/`.

## Ground rules

- **The extension runs no git.** Everything that reads or changes the repository is Gitardo's engine,
  `gitardo-engine`: a read is a `read/<card>` request (`src/engine/reads.ts`), an operation a `feature/run` sent by
  `Controller.runOnEngine`, so that it is shown, logged and undone like the rest
  ([the feature contract](../docs/extension/04-feature-contract.md)). Without the binary for the system, the views and
  the commands say so, with **Show the log**, and nothing else
  ([D49](../docs/08-decisions.md#d49--when-the-engines-binary-is-missing-proposed)).
- A new feature, or new behavior of one, goes into the engine first, desktop first, and is then used here
  ([D39](../docs/08-decisions.md#d39--desktop-first-and-a-status-for-each-app)); what runs no git, such as what the
  views draw, stays in `src/shared` and `src/webview`.
- A change a user can see goes into `README.md` (the page of the Marketplace), into the **Unreleased** section of
  `CHANGELOG.md`, and into its card, in the same commit: the desktop's card in `../docs/features/` once its area is
  merged, else the extension's in `../docs/features/extension/`.
- `README.md` and `CHANGELOG.md` are what the Marketplace shows. Their links and images stay relative to this folder;
  `npm run package` points them into `extension/` on GitHub
  ([D47](../docs/08-decisions.md#d47--a-readme-and-a-changelog-for-each-app-and-a-short-one-at-the-root)).
- A key of the workspace state or a setting keeps its name and meaning ([stored data](../docs/extension/11-data.md));
  the settings are sent to the engine under the desktop's names, which are the same without `gitardo.`
  ([05-stored-data.md](../docs/transition/05-stored-data.md)).

## Commands

```bash
npm install
npm run build        # dist/: the extension, the webviews, the Flow Diff, the preview server, the e2e suite
npm run typecheck    # tsc, over src and test
npm test             # Vitest: unit tests and integration tests against throwaway repositories
npm run check:docs   # the extension's cards, identifiers and glossary words (needs Python 3)
npm run test:e2e     # builds, then runs test/e2e/suite.ts inside a real VS Code
npm run test:ui      # builds, then drives a real VS Code window with clicks and keys; screenshots in dist/ui
npm run preview -- <folder>...   # the webviews in a browser, against the repositories in the folders
npm run package      # one vsix per target with its engine, with ../LICENSE copied in; a release waits for all of them
```

Which check sees which change, and how the extension is tested with the engine, is in
[the extension's testing](../docs/extension/06-testing.md).

## Where to start

1. [The architecture](../docs/extension/02-architecture.md) and [the feature contract](../docs/extension/04-feature-contract.md).
2. [The protocol with the engine](../docs/transition/02-protocol.md), for an area that has moved.
3. The card of the feature you change and its tests.
4. [The code rules](../docs/extension/03-code-rules.md) before writing code.

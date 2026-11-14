# Working on the extension

The extension for VS Code, in this folder. The rules of the whole repository are in [../AGENTS.md](../AGENTS.md); these
are the ones of this folder. Paths here are from `extension/`.

## Ground rules

- **The engine is being moved in.** The areas whose operations run in the engine are listed in
  [the roadmap](../docs/transition/06-roadmap.md); in those, an entry sends `feature/run` to `gitardo-engine`, and its
  `GitOp` in `src/git/ops.ts` is only the fallback for a missing binary
  ([D49](../docs/08-decisions.md#d49--when-the-engines-binary-is-missing-proposed)). Do not add behavior to a `GitOp`
  of a moved area: add it to the engine, desktop first, then use it here.
- In an area not moved yet, everything that changes the repository is a `GitOp` run by the `Controller`, so that it is
  shown, logged and undone like the rest ([the feature contract](../docs/extension/04-feature-contract.md)); a new
  feature is not started here before it is in the engine ([D39](../docs/08-decisions.md#d39--desktop-first-and-a-status-for-each-app)).
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
npm run package      # a minified build packed into gitardo-<version>.vsix, with ../LICENSE copied in
```

Which check sees which change, and how the extension is tested with the engine, is in
[the extension's testing](../docs/extension/06-testing.md).

## Where to start

1. [The architecture](../docs/extension/02-architecture.md) and [the feature contract](../docs/extension/04-feature-contract.md).
2. [The protocol with the engine](../docs/transition/02-protocol.md), for an area that has moved.
3. The card of the feature you change and its tests.
4. [The code rules](../docs/extension/03-code-rules.md) before writing code.

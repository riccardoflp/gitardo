# Code rules

Numbered rules, so that a review can cite "R4.2" and a reader can tell a convention from a whim. Most are the rules the
code already follows, written down; each says where it comes from and how it is checked:

- *Enforced*: a command the project runs fails when the rule is broken. The table at the end says which.
- *Followed*: true of the code today, checked only by review.
- *Target*: not yet true everywhere; new code and code that is changed anyway follow it.

A change that needs to break a rule changes this page first, in its own commit, with the reason.

## R1 — Dependencies

- **R1.1** The extension needs only VS Code (1.100 or later, `engines.vscode`) and `git` on the `PATH` at run time.
  Libraries from npm are bundled by esbuild into `dist/`; nothing is installed beside the extension. *Source: project
  choice ([D24](../08-decisions.md#d24--run-the-git-executable-never-a-library)). Followed.*
- **R1.2** A new runtime dependency is proposed first, with what it costs in the size of the package. *Source:
  AGENTS.md, "an idea beyond what was asked is proposed first". Review.*
- **R1.3** Every file the extension needs at run time is let into the package in `.vscodeignore`, the engine's
  `bin/<target>/` too; `npm run test:vsix` runs the end-to-end suite on what the package holds. *Source: AGENTS.md. Enforced by `npm run test:vsix` for what the
  suite reaches.*
- **R1.4** The folders use each other as [the architecture](02-architecture.md#who-may-use-whom) draws it:
  `src/shared` imports nothing of the others, and no file of `src` runs `git`; there is no `src/git` (stage 10 of
  [the transition](../transition/06-roadmap.md)). *Source: project choice. Followed, with the edges marked on that
  page; no tool checks the imports, and `grep -rn child_process src` finds `src/engine/client.ts` alone.*
- **R1.5** The extension runs one program: `gitardo-engine`, from `bin/<platform>/` of the package or from
  `gitardo.engine.path`, started and spoken to by `src/engine` only, which does not import `src/webview`. Git is the
  engine's to run. *Source:
  [D41](../08-decisions.md#d41--json-rpc-over-the-standard-input-and-output-one-process-per-window). Not checked by a
  tool.*

## R2 — Features

- **R2.1** Everything that changes the repository, a remote or a file of the repository is a feature of the engine,
  sent with `feature/run` by `Controller.runOnEngine` (or `Controller.runIn`, which hands it there), also what Gitardo
  does itself rather than git. *Source: AGENTS.md, "From a click to git". Followed; checked per card by
  [the promises](04-feature-contract.md#the-promises-every-card-is-checked-against).*
- **R2.2** The features an action of a view runs, with their target and answers, are worked out by `engineRuns` of
  `src/engine/actions.ts` from the action and the state alone, so that the tests check them without VS Code; what an
  operation runs is the engine's, tested in `rust/` and by the scenarios of `test/engine*.test.ts` on throwaway
  repositories. *Source: project choice. Followed.*
- **R2.3** An operation that can lose work or rewrite history sets `danger`, and is in the README's list under
  [Command preview](../../extension/README.md#command-preview). *Source: README. Review.*
- **R2.4** What the reflog cannot take back is recorded for **Undo** in the engine's journal, by the feature that ran.
  *Source: project choice ([D28](../08-decisions.md#d28--undo-reads-the-reflog-and-remembers-only-what-it-cannot-tell)).
  Review.*
- **R2.5** A visible change updates its card in `docs/features/` in the same commit, when its area has cards, and
  writes the cards of its area when it has none yet. *Source: project choice
  ([D33](../08-decisions.md#d33--cards-first-for-the-areas-where-a-mistake-costs)). Review; `docs/catalog.py` finds
  identifiers that no longer have a card.*

## R3 — Purity

- **R3.1** `src/shared`, the Derivations among them (`conflicts.ts`, `review.ts`), reads no files, runs no process and
  uses no clock, so that it runs in the webviews and in the tests. *Source: project choice. Followed.*
- **R3.2** Reads never take git's optional locks: the engine's reads run git with `GIT_OPTIONAL_LOCKS=0`. *Source:
  R9.3 of the desktop's [code rules](../desktop/03-code-rules.md). Followed.*

## R4 — Errors

- **R4.1** What git prints when it fails reaches the user: the error message says the engine's words, and the log has
  the whole output. *Source: `Controller.reportEngineFailure`, `Controller.reportFailure`. Followed.*
- **R4.2** Errors are `Error` objects with a sentence for the user (`EngineError` for the engine's); no string is
  thrown. *Source: project choice. Followed: no `throw` of a string in `src`.*
- **R4.3** What follows an operation in VS Code (its follow-ups, a message, a read again) must not throw once the
  engine has run it: a failure there is logged as a warning, never shown as the operation failing. *Source:
  `Controller.runOnEngine`. Followed.*
- **R4.4** Nothing is swallowed without a reason in a comment or a line in the log. *Source: project choice. Review.*

## R5 — Size and shape

- **R5.1** A module does what its name says; no `utils` grows into a second home. *Source: project choice. Followed,
  but for `src/webview/util.ts`, which is small.*
- **R5.2** A file over about 1,000 lines is split along a line the reader sees, when it is changed anyway.
  *Source: project choice. Target: `controller.ts` (2,700 lines) is over it; see [RS1](13-risks.md).*
- **R5.3** Types carry meaning: unions of string literals (`MergeMode`, `ResetMode`, `Operation`), not bare strings or
  booleans that switch behavior. *Source: project choice. Followed.*
- **R5.4** A `switch` on a union handles every case. *Source: project choice. Review: `tsc` checks it only where the
  result is used.*

## R6 — Names

- **R6.1** Names are words, not abbreviations: `entry`, `listed`, `shown`, not `e`, `lst`, `vis`. *Source: AGENTS.md.
  Followed.*
- **R6.2** The words of the [glossary](../09-glossary.md), with the name its **In code** column gives, and never a word of
  its **Avoid** column, in the code, the UI text and the documents. *Source: Domain-Driven Design's ubiquitous language.
  Enforced for the documents by `docs/catalog.py`; review for the code.*

## R7 — Comments

- **R7.1** Comments are sentences in plain words. They say what a thing is or why it is so, from what the user sees,
  not how the code does it: `/** The commits of what is checked out, for the view that asked. */`. Read the comments
  around a change and write the same way. *Source: AGENTS.md. Followed.*
- **R7.2** A surprising line says why, in a comment above it. *Source: project choice. Followed.*
- **R7.3** No commented-out code, and no TODO: an idea is proposed, a gap is a row of the
  [risks](13-risks.md). *Source: project choice. Followed: no TODO in `src`.*
- **R7.4** The function that builds an operation names its card in its doc comment, as `Feature: branch.create`.
  *Source: project choice. Target.*

## R8 — Running git

- **R8.1** The extension runs no git: the engine does, its arguments as a list, never through a shell, and the
  extension starts the engine with `spawn` and its arguments as a list. *Source: CWE-88; stage 10 of
  [the transition](../transition/06-roadmap.md). Followed: `EngineClient` in `src/engine/client.ts` is the only caller
  of `spawn` in `src`.*
- **R8.2** The commands shown and logged are the commands that run: the plan the engine shows is the plan it runs, a
  command that would open an editor carries the flag that prevents it in sight (`--no-edit`, `-c core.editor=true`).
  *Source: README, Command preview. Followed, by the engine.*
- **R8.3** A name that becomes an argument is refused when it starts with `-`; a file whose name is a pattern is passed
  as `:(literal)<name>`; a branch that shares its name with a tag or a remote branch is `refs/heads/<name>`: the
  engine's checks, which the extension's targets do not get around (`refArg` of `src/shared/refs.ts` names a reference
  in full where a menu could mean two). *Source: CWE-88; README, Command preview. Followed; checked per card.*
- **R8.4** What git prints is parsed from machine-readable formats (`--format` with separators, `-z`, `--porcelain`)
  where git has one, by the engine. *Source: project choice. Followed.*
- **R8.5** The version of git a feature needs is said where it is checked (2.40 for `git merge-tree --write-tree`, the
  conflict previews), and without it the feature is skipped, not failed. *Source: README. Followed.*
- **R8.6** Every area is the engine's ([transition/06-roadmap.md](../transition/06-roadmap.md)): nothing in `src`
  runs git or builds its commands, and new behavior goes into the engine first. Without the engine's binary, a view
  or a command says **Gitardo's engine is not available on this system.** with **Show the log**, and runs nothing else.
  *Source: [D39](../08-decisions.md#d39--desktop-first-and-a-status-for-each-app),
  [D49](../08-decisions.md#d49--when-the-engines-binary-is-missing-proposed). Review.*

## R9 — User interface

- **R9.1** UI text is short, plain and in sentence case, and says what will happen: **Show the changes**, **Open in the
  diff editor**, **Take the version of main into the working tree**. A tooltip finishes the thought of its button.
  *Source: AGENTS.md. Followed.*
- **R9.2** Colors and sizes come from VS Code's CSS variables, icons from the Codicons; a change is looked at in a dark
  and a light theme. *Source: AGENTS.md. Review; `npm run test:ui` takes screenshots in five themes.*
- **R9.3** The webviews never block: reading and running are in the extension host, and the views only post messages.
  *Source: project choice. Followed.*
- **R9.4** The Content Security Policy of the webviews allows only the extension's own scripts and styles. *Source:
  VS Code's webview guide. Followed; `npm run test:ui` reports what the page logs.*

## R10 — Tests

- **R10.1** A parser is tested on what git really prints, including the awkward cases (`test/awkward.test.ts`: spaces,
  accents, brackets and leading dashes in names, no commits, a detached `HEAD`, a shallow clone). *Source: project
  choice. Followed.*
- **R10.2** An operation that changes is tested by running its commands on a throwaway repository
  (`test/integration.test.ts`). *Source: AGENTS.md. Followed.*
- **R10.3** A fixed bug gets a test that failed before the fix. *Source: project choice. Review.*
- **R10.4** Tests are deterministic: repositories made fresh in a temporary folder, with a fixed identity and no
  signing. *Source: `test/integration.test.ts`. Followed.*

## R11 — Layout of the code

- **R11.1** Code is laid out as Prettier lays it out with a width of 160: single quotes, semicolons, trailing commas,
  two spaces. There is no formatter in the project: match the file you are in. *Source: AGENTS.md. Followed, but for
  long strings and regular expressions, which Prettier would not break either (98 lines of `src` are longer).*
- **R11.2** Comments wrap at about 120 columns. *Source: AGENTS.md. Followed.*
- **R11.3** Files use LF line endings; on Windows, check `git ls-files --eol` after a script rewrote a file. *Source:
  AGENTS.md. Followed: every text file of the index is LF.*
- **R11.4** TypeScript is `strict`, with `verbatimModuleSyntax`; `any` is not used. *Source: `tsconfig.json`. Enforced by
  `npm run typecheck` for `strict`; followed for `any` (one use in `src`).*

## R12 — Changes and commits

- **R12.1** Everything in the repository is in English: code, comments, UI text, documents, commit messages. *Source:
  AGENTS.md. Review.*
- **R12.2** One commit per piece of work, made only when asked; never pushed, tagged or published by an agent: the
  maintainer pushes the tag of a release, and the pipeline publishes the vsix of every target once they approve, since
  there is no universal one to publish without them (D49, D52). *Source: AGENTS.md. Review.*
- **R12.3** A commit message is one sentence that says what changed for the user, in the imperative, with no prefix.
  *Source: AGENTS.md. Review.*
- **R12.4** A change a user can see goes into the README, where the feature is described, into the **Unreleased**
  section of the CHANGELOG, and into its card, in the same commit. *Source: AGENTS.md; Keep a Changelog. Review.*
- **R12.5** Gitardo's former names are written nowhere, except where the 0.2.0 entry of the CHANGELOG explains the
  change of name. *Source: AGENTS.md. Review.*

## R13 — Secrets and untrusted input

Each rule answers a threat of the [security page](10-security.md).

- **R13.1** The GitHub token is asked of VS Code's authentication at each use, never stored, logged or sent anywhere
  but `api.github.com`. *Answers T2. Followed: `src/extension/github.ts`.*
- **R13.2** What a repository holds (file names, messages, branch names, `.gitmodules`, the output of git) is data:
  passed to git as arguments (R8.1, R8.3), shown as text in the webviews, never as HTML. *Answers T1, T4. Followed: React
  escapes text; the two uses of `innerHTML` in `src/flow/webview` write fixed markup.*
- **R13.3** No network call the user did not cause or a setting did not allow: GitHub only for a repository whose remote
  is on github.com, with a session the user granted; the language model only when the user asks for a draft or a
  resolution. *Answers T3, T5. Followed.*
- **R13.4** Commands that run in a terminal (`gitardo.worktrees.setupCommands`) are said in the question before they run
  and come only from settings, which VS Code reads in a trusted workspace only. *Answers T6. See the worktree cards.*

## R14 — Stored data

- **R14.1** What is kept in VS Code's workspace state is read defensively: a value of the wrong shape is ignored, never
  a crash. *Source: project choice. Target: the favorites are checked; the other keys of [11-data.md](11-data.md) are
  taken as written, since only Gitardo writes them.*
- **R14.2** A key of the workspace state, a setting name or a file Gitardo leaves in `.git` keeps its name and meaning;
  a change to one reads the old one too. *Source: project choice. Review.*

## What the tools check

| Command | Checks | Rules |
| --- | --- | --- |
| `npm run typecheck` | `tsc --noEmit` with `strict`, over `src` and `test` | R11.4 |
| `npm test` | Vitest: parsers, derivations, operations against real repositories | R8.3, R10.1, R10.2, R10.4 |
| `npm run test:e2e`, `npm run test:vsix` | The suite inside a real VS Code; on the packed extension | R1.3, R2.1 |
| `npm run test:ui` | A real window, five themes, errors logged by the webviews | R9.2, R9.4 |
| `npm run check:docs` | `docs/catalog.py`: cards, identifiers, glossary words | R6.2, R2.5 |

Everything else is checked by review. There is no linter and no formatter in the project; adding them is not planned
([D34](../08-decisions.md#d34--no-linter-no-formatter)).

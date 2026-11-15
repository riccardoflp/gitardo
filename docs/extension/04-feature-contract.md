# The feature contract

What a feature of Gitardo is in the code, the kinds of feature, how they are named, and the template of a card. The
catalog in [features/](../features/extension/README.md) is written to this page.

Since stage 10 of [the transition](../transition/06-roadmap.md) every area has moved: a feature that reads or changes
the repository is the engine's, written to [Gitardo Desktop's feature contract](../desktop/04-feature-contract.md), and
its card is the desktop's, with a status for each app; the extension gives it an interface, sends it `feature/run` or
asks its read ([transition/02-protocol.md](../transition/02-protocol.md)), and runs no git of its own. What follows is
what the extension adds around them.

## Why

A feature that changes the repository is a plan of the engine, built as data before anything runs. The engine can then
show it first, log it command by command, keep what **Undo** needs, and the extension reads the repository again, the
same way for every feature (see [From a click to git](02-architecture.md#from-a-click-to-git)). The features are many;
that machinery is written once. The cards hold the features to it: a card says what the feature asks, what it runs, how
dangerous it is and how it is taken back, and the reviewer checks the code against those lines.

## The kinds

| Kind | What it is | Where it lives today |
| --- | --- | --- |
| **Operation** | Something the user does that changes the repository, a remote, the working tree or a file Gitardo writes. A feature of the engine; the extension asks its questions with VS Code. | A case of `Action` (`src/shared/protocol.ts`), the features `engineRuns` gives it (`src/engine/actions.ts`), run by `Controller.runOnEngine`; what is VS Code's alone is `hostAction` (`src/extension/actions.ts`). |
| **Read** | Asks git, GitHub or the language model something and returns data. Changes nothing, takes no lock. | A read of the engine, asked by a method of `Reads` (`src/engine/reads.ts`), in the view's shape by `src/engine/shapes.ts`. |
| **View** | What a view, a tab or a page shows and what the user can do there. Posts messages, changes nothing itself. | A React component in `src/webview` (or the Flow Diff's in `src/flow/webview`), and its entries in `src/webview/menus.ts`. |
| **Derivation** | Turns data into other data: the lanes of the graph, the tree of files, the name of a reference, what the rules propose for a conflict. | A function of `src/shared` with no I/O. |

A feature the user sees is usually one Operation or View plus the Reads and Derivations it uses; its card names them.

## An operation, as it is built today

An operation goes through four places, and a card names each of them.

1. **The action.** A case of `Action` in `src/shared/protocol.ts`, with what the view knows: the commit, the branch, the
   file. The view posts it with `act()` (`src/webview/menus.ts`).
2. **The features.** `engineRuns` (`src/engine/actions.ts`) gives the features of the engine that run it, with their
   `Target` and the answers the action already carries; an action that is none of them runs nothing. What is VS
   Code's alone, such as opening a folder in a window, is done by `hostAction` (`src/extension/actions.ts`).
3. **The questions and the plan.** `Controller.runOnEngine` sends `feature/run`; the engine asks what the action does
   not carry, and `VscodeAsker` (`src/engine/vscode.ts`) asks it with VS Code's input boxes, pickers and dialogs. The
   engine builds the plan, its steps exactly as they run, its danger and what it says, refuses with a message when it
   cannot go on, and asks first when `gitardo.commandPreview` or the danger says so.
4. **The run.** The engine runs the plan, logs each step to the **Gitardo** output channel through `log/line`, keeps
   what **Undo** needs in its journal, and hands the extension its follow-ups; the extension reads the repository
   again. Without the engine, the command says **Gitardo's engine is not available on this system.** with **Show the
   log** (D49).

The commands of an operation are tested in the engine, in `rust/`, and the extension's side by the scenarios of
`test/engine*.test.ts`, on throwaway repositories. Derivations are functions of `src/shared` with no I/O, tested on
their own.

## Identifiers

`area.action`, lower case, words joined by `-`: `branch.create`, `stash.drop`, `conflict.take-side`. The identifier
heads its card, and the card's **Implemented in** line names the code it is. The code does not carry the identifier
yet; writing it in the doc comment of the function that builds the operation, as `Feature: branch.create`, is a target
for code that is changed anyway (R7.4 in the [code rules](03-code-rules.md)).

## The areas

Each area has a file in `features/`, the prefixes of its identifiers, and whether its cards are written. Which areas
have cards, and why the others do not yet, is [D33](../08-decisions.md#d33--cards-first-for-the-areas-where-a-mistake-costs).
An area without cards is still described for users in the [README](../../extension/README.md); its cards are written the first time
the area is changed after this page.

| File | Area | Prefixes | Cards |
| --- | --- | --- | --- |
| `01-repositories.md` | Repositories, reading and identities | `repo`, `identity` | Some: `repo.select`, `repo.follow-source-control` |
| `02-graph.md` | The graph and its toolbar, labels, folds, keys, drag and drop | `graph`, `refs`, `details` | Later |
| `03-search-and-filter.md` | Search, go to a commit, branches shown | `search`, `filter` | Later |
| `04-commit-tab.md` | A commit in an editor tab, the side bar views | `commit-tab`, `side` | Later |
| `05-changes-and-commit.md` | Staging, discarding, committing, the Changes view | `changes`, `commit` | Written |
| `06-branches-tags-remotes.md` | Branches, tags, remotes, fetch, pull, push, branch cleanup | `branch`, `tag`, `remote`, `sync` | Written |
| `07-merge-and-rewrite.md` | Merge, rebase, cherry-pick, revert, reset, rewording, squash, split, fixup, bisect | `merge`, `rebase`, `cherry-pick`, `revert`, `reset`, `rewrite`, `bisect` | Written |
| `08-stashes.md` | Stashes | `stash` | Written |
| `09-conflicts.md` | Conflicts, the operation under way, the review of resolutions, AI | `conflict`, `operation` | Written |
| `10-compare.md` | Comparisons | `compare` | Later |
| `11-history-and-blame.md` | File, folder and line history, blame, reflog | `history`, `blame`, `reflog` | Later |
| `12-worktrees.md` | Worktrees and what new worktrees get | `worktree`, `setup` | Written |
| `13-submodules.md` | Submodules | `submodule` | Written |
| `14-patches.md` | Copying, saving and applying patches | `patch` | Later |
| `15-github.md` | GitHub sign-in, pull requests | `github` | Later |
| `16-undo-preview-log.md` | Undo, the command preview, the log, what is running | `undo`, `preview`, `log`, `activity` | Written |
| `17-flow.md` | The Flow Diff and the Flow Merge | `flow-diff`, `flow-merge` | Later |
| `18-settings.md` | Every setting, with the cards that read it | `settings` | Written |

## The card

Leave out a line that does not apply rather than writing "none", except **Danger** and **Undo** on an operation, which
are always said, and **Tested by**, which says *none* when nothing tests it.

````markdown
### `area.action` — Title in sentence case

What it is for, in one or two sentences, from the user's side.

- **Kind**: Operation | Read | View | Derivation
- **Function**: the feature of the engine it runs (or the read, the component, the derivation), with where the
  extension sends it: `branch.create`, from `engineRuns` in `src/engine/actions.ts`.
- **Entry points**: where the user finds it, with the words on screen: the context menu of a branch, **Gitardo: Create
  Branch…** in the Command Palette, a key.
- **Offered when**: the conditions under which the entry points show it.
- **Asks**: the questions, in order, with their defaults and the words of the choices.
- **Reads**: what it reads before it can decide, and why.
- **Plan**: the commands, as the log shows them (`> git branch feature/x main`), and the lines of what Gitardo does
  itself (`write`, `symlink`, `copy`, `run`, `unlink`, `delete`, `git.close`).
- **Danger**: None (asked only when `gitardo.commandPreview` is `all`) | Confirm, because… | Destructive, because…
- **Undo**: what **Undo** does right after it, and from where: the reflog, or what Gitardo remembered; or why it cannot.
- **After**: what is selected, said or offered once it has run.
- **Refuses**: when, and what it says.
- **Settings**: the settings that change what it does.
- **Uses**: other features, reads and derivations, by identifier.
- **Implemented in**: every place it is, as it is now: the `Action` case, its case in `engineRuns`, the menu entry,
  what `Controller.runOnEngine` does with its follow-ups.
- **Tested by**: the tests, by file and title, or *none*.
- **Differs**: where the code does not do what this card says, with the file and line. Only when it applies.

Acceptance:

- [ ] A sentence a test can check, from the user's side, with exact values.
- [ ] …
````

A Derivation card replaces the operation lines with the **rules of its result**: inputs, outputs, invariants, and
worked examples (input → output).

### The promises every card is checked against

These are what Gitardo promises for every operation, and what a reviewer reads a card for first. A card says each one
that applies as an acceptance line. Where the code does otherwise, the line ends with `(differs)` and the card has a
**Differs** line.

1. **It runs as a plan of the engine.** Everything that changes the repository, a remote or a file goes through
   `Controller.runOnEngine` (or `Controller.runIn`, which hands it there): it is shown first when
   `gitardo.commandPreview` is `all` or the plan has a danger, and each command and each thing Gitardo does itself is
   written to the **Gitardo** output channel.
2. **What is shown is what runs.** The commands in the question and in the log are the ones that run, with no flag
   added after. A command that would open an editor carries the flag that prevents it, in sight (`--no-edit`,
   `-c core.editor=true`).
3. **Names are not options.** A name typed or chosen that becomes an argument is refused when it starts with `-`. A file
   whose name is also a pattern to git is passed as `:(literal)<name>`. A branch that shares its name with a tag or a
   remote branch is named `refs/heads/<name>` where the short name would mean the other one.
4. **It asks before losing work.** An operation of the list in the README's
   [Command preview](../../extension/README.md#command-preview) asks first whatever `gitardo.commandPreview` says: with a warning
   (`danger`) when it can lose work, with a question (`confirm`) when it rewrites history or moves a branch in a way
   that is easy to take back. A card and that list agree; an operation that can lose work and is not on the list, or
   asks with a question where it can lose work, differs.
5. **Undo is said.** The card says what **Undo** does right after it. When the reflog cannot take it back, Gitardo
   remembers how until something else happens in the repository.
6. **Reads take no locks.** What only reads runs with `GIT_OPTIONAL_LOCKS=0`, so it never fights with the user's own
   git in a terminal.

### What makes an acceptance line good

- It can fail. "Works correctly" cannot; "Creating `feature/x` from `main` runs `git branch feature/x main`" can.
- It is about what the user sees or what the repository ends up as, not about internal calls.
- It has the exact numbers, texts and commands: 100 entries, *Nothing to undo*, `--force-with-lease`.
- It covers the refusals and the edge cases the card names, not only the happy path.

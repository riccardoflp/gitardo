# The feature contract

What a feature of Gitardo is in the code, the kinds of feature, how they are named, and the template of a card. The
catalog in [features/](features/README.md) is written to this page.

## Why

A feature that changes the repository is built as data, a `GitOp`, before anything runs. The controller can then show
it first, log it command by command, keep what **Undo** needs and read the repository again, the same way for every
feature (see [From a click to git](02-architecture.md#from-a-click-to-git)). The features are many; that machinery is
written once. The cards hold the features to it: a card says what the feature asks, what it runs, how dangerous it is
and how it is taken back, and the reviewer checks the code against those lines.

## The kinds

| Kind | What it is | Where it lives today |
| --- | --- | --- |
| **Operation** | Something the user does that changes the repository, a remote, the working tree or a file Gitardo writes. Asks what it needs, then becomes a `GitOp`. Runs nothing itself. | A case of `Action` (`src/shared/protocol.ts`), its case in `resolveAction` (`src/extension/actions.ts`), and the function of `src/git/ops.ts` that builds its `GitOp`. A few are methods of `Controller` that build a `GitOp` themselves. |
| **Read** | Asks git, GitHub or the language model something and returns data. Changes nothing, takes no lock. | A method of `Repo` (`src/git/repo.ts`) and the parser of what git prints (`src/git/parse.ts`). |
| **View** | What a view, a tab or a page shows and what the user can do there. Posts messages, changes nothing itself. | A React component in `src/webview` (or the Flow Diff's in `src/flow/webview`), and its entries in `src/webview/menus.ts`. |
| **Derivation** | Turns data into other data: the lanes of the graph, the tree of files, the name of a reference, what the rules propose for a conflict. | A function of `src/shared` or `src/git` with no I/O. |

A feature the user sees is usually one Operation or View plus the Reads and Derivations it uses; its card names them.

## An operation, as it is built today

An operation goes through four places. Every new operation follows the same path, and a card names each of them.

1. **The action.** A case of `Action` in `src/shared/protocol.ts`, with what the view knows: the commit, the branch, the
   file. The view posts it with `act()` (`src/webview/menus.ts`).
2. **The questions.** Its case in `resolveAction` (`src/extension/actions.ts`) asks what the action does not carry, with
   VS Code's input boxes, pickers and dialogs, reads what it needs through `Repo`, refuses with a message when it cannot
   go on, and returns the `GitOp`, or nothing when the user cancelled.
3. **The plan.** A function of `src/git/ops.ts` builds the `GitOp`: its `title`, its `commands` exactly as they run, and
   what it may also have:

   | Field | Means |
   | --- | --- |
   | `danger` | It can lose work: always asked first, with a warning, whatever `gitardo.commandPreview` says. |
   | `confirm` | Always asked first, as a question rather than a warning. |
   | `note` | What the question says before the commands. |
   | `input` | What the last command reads on its standard input, such as a patch. |
   | `writes` | Files Gitardo writes before the commands, all or none, each only if it is still what it was read as. |
   | `fallback` | What to offer when git refuses with a given message, typically the forced variant. |
   | `then` | An operation of its own that runs once this one succeeded, said along with it (the submodules after a checkout). |
   | `prepare`, `recover`, `finish`, `cleanup`, `onSuccess` | What Gitardo does itself before, during and after the commands, each saying in the log what it does. |

4. **The run.** `Controller.action` hands the `GitOp` to `Controller.run`, which asks first when it must, runs the
   commands, logs them, reads the repository again, and then records what **Undo** needs when the reflog alone cannot
   take it back.

The target for new code and for code that is changed anyway is the same shape with the questions kept apart from the
plan: a function of `src/git/ops.ts` that takes everything it needs as arguments and returns the `GitOp`, so that an
integration test can run it on a throwaway repository without VS Code. That is how most of `ops.ts` is written already.
Moving the questions out of `resolveAction` into data is not planned: see [D4](08-decisions.md#d4--an-operation-is-a-gitop-built-before-it-runs).

Reads and Derivations already have this shape: a `Repo` method builds the arguments and a function of `parse.ts` turns
what git prints into data, which `test/git.test.ts` checks on recorded output.

## Identifiers

`area.action`, lower case, words joined by `-`: `branch.create`, `stash.drop`, `conflict.take-side`. The identifier
heads its card, and the card's **Implemented in** line names the code it is. The code does not carry the identifier
yet; writing it in the doc comment of the function that builds the operation, as `Feature: branch.create`, is a target
for code that is changed anyway (R7.4 in the [code rules](03-code-rules.md)).

## The areas

Each area has a file in `features/`, the prefixes of its identifiers, and whether its cards are written. Which areas
have cards, and why the others do not yet, is [D11](08-decisions.md#d11--cards-first-for-the-areas-where-a-mistake-costs).
An area without cards is still described for users in the [README](../README.md); its cards are written the first time
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
- **Function**: the function that builds its `GitOp` (or the `Repo` method, the component, the derivation), with its
  file: `createBranchOp(name, at, checkout)` in `src/git/ops.ts`.
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
- **Implemented in**: every place it is, as it is now: the `Action` case, the case of `resolveAction`, the builder, the
  menu entry, what `Controller.action` does after it.
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

1. **It runs as a `GitOp`.** Everything that changes the repository, a remote or a file goes through `Controller.run`
   (or `Controller.runIn`): it is shown first when `gitardo.commandPreview` is `all` or the operation is `danger` or
   `confirm`, and each command and each thing Gitardo does itself is written to the **Gitardo** output channel.
2. **What is shown is what runs.** The commands in the question and in the log are the ones that run, with no flag
   added after. A command that would open an editor carries the flag that prevents it, in sight (`--no-edit`,
   `-c core.editor=true`).
3. **Names are not options.** A name typed or chosen that becomes an argument is refused when it starts with `-`. A file
   whose name is also a pattern to git is passed as `:(literal)<name>`. A branch that shares its name with a tag or a
   remote branch is named `refs/heads/<name>` where the short name would mean the other one.
4. **It asks before losing work.** An operation of the list in the README's
   [Command preview](../README.md#command-preview) asks first whatever `gitardo.commandPreview` says: with a warning
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

# The clean room

Gitardo Desktop is written from scratch, in a clean room: from what Gitardo does, never from how the extension does it.
This page says what that means in practice, for people and for agents.

## Why

- **A design of its own.** The extension is TypeScript in VS Code, React in webviews and Monaco in the diffs. A
  translation would carry their shape into a program that has none of them: callbacks where Rust wants ownership,
  a DOM where there is a pixel buffer. Starting from behavior gives the Rust program the shape Rust wants.
- **No hidden debt.** What is not in the specification is not in the program. Every behavior is written down before it
  is built, so the documents and the program say the same thing.
- **Clean provenance.** Everything in this branch is written here: no code is copied from the extension, nor from any
  other project. Algorithms come from their published descriptions, formats from their specifications.

## The two sides

| | Specification side | Implementation side |
| --- | --- | --- |
| Who | Whoever writes or changes the documents in `docs/`. | Whoever writes code in this branch. |
| May read | The extension: its README, its behavior, its source on the `main` branch. Git's documentation. | This branch only. Git's documentation. Public specifications (PNG, deflate, Win32, X11, AppKit, HTTP). Papers that describe algorithms. The documentation and the public interface of the libraries the program depends on. |
| May not | Put code, pseudo-code translated from code, or internal names of the extension in a document. | Open the source of the extension (`src/`, `test/` on `main`), or the source of another project to copy a part it is building. |
| Writes | Behavior, git commands, rules with examples, acceptance criteria. | Rust, tests, and the changes a document needs when the code shows it was unclear (as a question first, see below). |

One person may be on both sides at different times, but not for the same feature at the same time: a feature
specified from the source is built by someone else, or after the source is out of mind and only the card is in front of
them. For agents this is a rule of the session: an implementation session works in a worktree of `main-desktop` and is
told not to read the `main` branch.

Running the extension to see what it does is allowed on both sides: behavior is what the clean room copies.

## When the specification is not enough

The implementation side does not guess and does not look at the extension's source. It writes the question in
[08-decisions.md](08-decisions.md) under **Open questions**, with the identifier of the card. The specification side
answers by changing the card (behavior, not code), and the question moves to the decisions with its answer.

## What counts as copying

Not allowed in this branch:

- code of the extension, in any language, or rewritten line by line in Rust;
- the extension's names of functions, types, files and messages between its parts;
- code from any library or project (a diff library, a font rasterizer, a JSON parser), whatever its license;
- assets of the extension: icons (Codicons), images, screenshots.

Allowed:

- the git commands and their arguments, which are git's interface;
- the words the user reads (labels, messages, tooltips), which are the product and are written in the cards;
- the names of the settings, which are the product's interface with its users;
- published algorithms implemented from their description (Myers' diff, the histogram diff of git's documentation,
  deflate), with the source named in the doc comment of the code;
- the libraries of [D13](08-decisions.md#d13--one-library-for-tls) and
  [D16](08-decisions.md#d16--libraries-for-the-window-the-text-and-the-pixels), used as dependencies of the crates
  named there through their public interface and their documentation, never copied into the workspace;
- the fonts of [D18](08-decisions.md#d18--the-fonts-come-with-the-program), under their licence, which is kept beside
  them.

## How a feature goes through the room

1. **Specified.** Its card in [features/](features/README.md) has every line of the template and its acceptance list.
2. **Reviewed.** Someone who did not write the card reads it and asks what is unclear. The answers go in the card.
3. **Built.** The function named by the card is written, with a test for each line of its acceptance list.
4. **Checked.** `cargo xtask check` passes: the card, the function and the tests name each other.
5. **Seen.** The feature is used in the application as the card says, and its screenshots are looked at.

The status of each card is in the index of the catalog.

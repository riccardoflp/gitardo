# The documents of Gitardo

Gitardo shows a commit graph and runs everyday git operations, each as the exact commands it shows and logs, in two
apps: the extension for VS Code (`extension/`) and Gitardo Desktop (`rust/`). These documents are for whoever changes
their code, person or agent. What each app does for its users is in its README: [the extension](../extension/README.md),
which is also its page on the Marketplace, and [Gitardo Desktop](../rust/README.md). [AGENTS.md](../AGENTS.md) is the
way in.

Some documents hold for both apps and are here; the others were written for one app and are in its folder. A path in
the extension's documents (`src/git/ops.ts`) is from `extension/`, one in Gitardo Desktop's (`crates/engine`) from
`rust/`.

## For both apps

| File | Says |
| --- | --- |
| [08-decisions.md](08-decisions.md) | The decisions (D1, …) with why and what was rejected, and the open questions (Q1, …), of both apps, with how their numbers were joined. |
| [09-glossary.md](09-glossary.md) | Each word of the product with one meaning, what it is not, the words to avoid, its name in the code. |
| [features/](features/README.md) | The catalog of Gitardo Desktop: one file per area, one card per feature, and its index. Checked by `cargo xtask check`. |
| [features/extension/](features/extension/README.md) | The extension's cards, for the areas that have them, and their index. Checked by `npm run check:docs`. |
| [transition/](transition/README.md) | The move to one engine, the Rust one, for both apps: the engine, the protocol, the map, the differences, stored data, the roadmap, the rules that change. |
| [catalog.py](catalog.py) | The check of the extension's cards, run by `npm run check:docs`. |

## Gitardo Desktop

| File | Says |
| --- | --- |
| [desktop/00-clean-room.md](desktop/00-clean-room.md) | How the program was written from behavior, never from the extension's code, and who may read what. |
| [desktop/01-product.md](desktop/01-product.md) | The window, its panes and pages, and what replaces each thing the extension takes from VS Code. |
| [desktop/02-architecture.md](desktop/02-architecture.md) | The crates, which may use which, how a click becomes a git command, threads, state, drawing. |
| [desktop/03-code-rules.md](desktop/03-code-rules.md) | The numbered rules every line of Rust follows, and what `cargo xtask check` enforces. |
| [desktop/04-feature-contract.md](desktop/04-feature-contract.md) | The four kinds of feature, their signatures, questions, plans, steps, and the template of a card. |
| [desktop/05-platform.md](desktop/05-platform.md) | What is asked of Windows, Linux and macOS, and how; text, fonts, HTTP. |
| [desktop/06-testing.md](desktop/06-testing.md) | Unit, golden, integration, view snapshots, application tests, performance. |
| [desktop/07-roadmap.md](desktop/07-roadmap.md) | The milestones, from the empty workspace to three systems, and the checks before a release. |

## The extension

| File | Says |
| --- | --- |
| [extension/01-product.md](extension/01-product.md) | The product as the user meets it: where each view is, what a user can count on, how settings are named. |
| [extension/02-architecture.md](extension/02-architecture.md) | The pieces that run, the folders and who may use whom, how a click becomes git commands, the state, where things are. |
| [extension/03-code-rules.md](extension/03-code-rules.md) | Numbered rules (R1.1, …), each with its source, and whether a tool enforces it. |
| [extension/04-feature-contract.md](extension/04-feature-contract.md) | What a feature is in the code, the kinds, the identifiers, the list of areas, the template of a card and the promises every card is checked against. |
| [extension/05-platform.md](extension/05-platform.md) | What is asked of VS Code, of git and of Windows, macOS and Linux. |
| [extension/06-testing.md](extension/06-testing.md) | The scripts, the kinds of tests from cheapest to most real, how a card's lines become tests, what no test reaches. |
| [extension/07-roadmap.md](extension/07-roadmap.md) | What is left to do on the documents and on the gaps they found, in order. |
| [extension/10-security.md](extension/10-security.md) | The threat model: trust boundaries, threats (T1, …), mitigations, what is not defended. |
| [extension/11-data.md](extension/11-data.md) | What the extension keeps between runs, where, and what never changes. |
| [extension/12-quality.md](extension/12-quality.md) | The qualities that matter most and their scenarios (QA1, …), with numbers. |
| [extension/13-risks.md](extension/13-risks.md) | Risks and debt (RS1, …): what, what if it bites, what is done about it. |

The rules of the two apps are numbered apart, each from R1.1: a rule is named with its page, as *R2.1 of the
extension's code rules*, wherever both could be meant.

At the root of the repository: [AGENTS.md](../AGENTS.md) (with `CLAUDE.md` pointing to it) for whoever changes the code,
[SECURITY.md](../SECURITY.md) for reporting a vulnerability. Each app has its CHANGELOG:
[the extension's](../extension/CHANGELOG.md), [Gitardo Desktop's](../rust/CHANGELOG.md).

## Keeping them true

A change a user can see changes its card, its app's README and its app's CHANGELOG in the same commit.
`cargo xtask check`, in `rust/`, fails when a card of `features/` and the Rust code do not name each other.
`npm run check:docs`, in `extension/`, fails when a card of `features/extension/` lacks the lines its kind needs, when an
identifier written in the extension's documents names no card, or when a word the glossary says to avoid is used.

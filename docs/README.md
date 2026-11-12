# The documents of Gitardo

Gitardo is a VS Code extension that shows a commit graph and runs everyday git operations, each as the exact commands it
shows and logs. These documents are for whoever changes its code, person or agent: how it is built, the rules of its
code, and a catalog of its features with what each must do. What it does for its users is in the
[README](../README.md) of the repository, which is also its page on the Marketplace; `AGENTS.md` is the way in.

The code exists and is released (0.10.0). The documents describe it as it is; where the code does otherwise than a
document wants, the card says **Differs**, and the gap is in [the risks](13-risks.md) and [the roadmap](07-roadmap.md).

## The documents

| File | Says |
| --- | --- |
| [01-product.md](01-product.md) | The product as the user meets it: where each view is, what a user can count on, how settings are named. |
| [02-architecture.md](02-architecture.md) | The pieces that run, the folders and who may use whom, how a click becomes git commands, the state, where things are. With the C4 context and containers diagrams. |
| [03-code-rules.md](03-code-rules.md) | Numbered rules (R1.1, …), each with its source, and whether a tool enforces it. |
| [04-feature-contract.md](04-feature-contract.md) | What a feature is in the code, the kinds, the identifiers, the list of areas, the template of a card and the promises every card is checked against. |
| [05-platform.md](05-platform.md) | What is asked of VS Code, of git and of Windows, macOS and Linux. |
| [06-testing.md](06-testing.md) | The scripts, the kinds of tests from cheapest to most real, how a card's lines become tests, what no test reaches. |
| [07-roadmap.md](07-roadmap.md) | What is left to do on the documents and on the gaps they found, in order. |
| [08-decisions.md](08-decisions.md) | The decisions (D1, …) with why and what was rejected, and the open questions (Q1, …). |
| [09-glossary.md](09-glossary.md) | Each word of the product with one meaning, what it is not, the words to avoid, its name in the code. |
| [10-security.md](10-security.md) | The threat model: trust boundaries, threats (T1, …), mitigations, what is not defended. |
| [11-data.md](11-data.md) | What Gitardo keeps between runs, where, and what never changes. |
| [12-quality.md](12-quality.md) | The qualities that matter most and their scenarios (QA1, …), with numbers. |
| [13-risks.md](13-risks.md) | Risks and debt (RS1, …): what, what if it bites, what is done about it. |
| [features/](features/README.md) | The catalog: one file per area, one card per feature, and its index. |
| [catalog.py](catalog.py) | The check of the catalog, run by `npm run check:docs`. |

At the root of the repository: [AGENTS.md](../AGENTS.md) (with `CLAUDE.md` pointing to it) for whoever changes the code,
[SECURITY.md](../SECURITY.md) for reporting a vulnerability, [CHANGELOG.md](../CHANGELOG.md) for what changed in each
release.

## What to read first

1. [02-architecture.md](02-architecture.md), *From a click to git*.
2. [04-feature-contract.md](04-feature-contract.md), and the card of what you are changing.
3. [03-code-rules.md](03-code-rules.md) and [09-glossary.md](09-glossary.md).
4. [06-testing.md](06-testing.md), before calling it done.

## What is not here

- **Cards for every area.** The catalog has the areas where a mistake costs; the others are described in the README and
  get their cards when they are next changed
  ([D11](08-decisions.md#d11--cards-first-for-the-areas-where-a-mistake-costs)).
- **Proposals.** Ideas are proposed to the maintainer in conversation before they are built; there is no folder of
  written proposals, since one maintainer decides.
- **A clean room.** This is the project itself, not a rewrite of another.

## Keeping them true

A change a user can see changes its card, the README and the CHANGELOG in the same commit. `npm run check:docs` fails
when a card lacks the lines its kind needs, when an identifier written in the documents names no card, or when a word
the glossary says to avoid is used. Run it with any change to `docs/`.

# Quality

The qualities that matter most to Gitardo besides its features, and the scenarios that make them measurable.

*Source: ISO/IEC 25010:2023 for the qualities; arc42 §10 for quality scenarios.*

## The goals

In order; a higher one wins when two pull apart.

1. **Safety of the user's work** (ISO 25010: safety, functional correctness). No operation loses work without asking
   first and saying what would be lost; what can be undone is undone by **Undo**. Traded for: more questions than a
   tool that trusts the user, and operations that refuse rather than guess.
2. **Transparency** (interaction capability: self-descriptiveness). What Gitardo runs is what the user can read in the
   question and the log, and could type. Traded for: no shortcuts that git has no command for, and odd-looking but
   exact commands (`:(literal)`, `refs/heads/`, `-c core.editor=true`).
3. **Speed on large repositories** (performance efficiency). The graph, the views and typing stay quick on a repository
   of tens of thousands of commits, thousands of references and changed files. Traded for: lists that are cut and say
   so, pages of commits read as the user scrolls, diffs shown on demand.
4. **Following the repository** (reliability). The views show what the repository is now, also after commands from a
   terminal, without the user refreshing. Traded for: reads of the repository while files change, which the status bar
   then shows.
5. **Fitting VS Code** (compatibility, interaction capability). Views where VS Code keeps git, its themes, icons, keys
   and editors; the five default themes look right.

## Scenarios

Times are those measured on the maintainer's machine with the minified build, on a repository made by
`test/perf/big-repo.mjs` (56,000 commits, 2,375 references, 1,779 changed files), as the README stated them before this
page; they are targets to keep. `test/perf/vscode.mjs --limits` reads the times of QA1 to QA3 from this table and fails
above them times one and a half, and the pipeline runs it on Linux
([D91](../08-decisions.md#d91--the-checks-pass-on-every-system-and-what-was-found-weak-is-checked-again-proposed),
Q1519): a time changed here changes the limit.

| | When | What must happen | Measure | Checked by |
| --- | --- | --- | --- | --- |
| QA1 | The graph is opened from the Command Palette on the large repository | Its first commits are on screen | about 1.5 s | `test/perf/vscode.mjs`, at most 1.5 times this |
| QA2 | A file is saved in the large repository | The views show the new state, the rows of the graph stay where they were | a refresh in about 250 ms | `test/perf/vscode.mjs`, at most 1.5 times this, once the working tree's wait of 1 s is over |
| QA3 | The graph is scrolled towards the end of what is loaded | The next page of `gitardo.graph.pageSize` commits (1,000 by default, 100 to 10,000) is there before the end is reached | about 300 ms a page | `test/perf/vscode.mjs`, at most 1.5 times this |
| QA4 | A commit message is typed in the Changes view of the large repository | Typing is not slowed by the list of 1,779 files under it | no key waits | `test/perf/vscode.mjs` (prints it; no limit, as no time is given, Q1590) |
| QA5 | Any view but the two graphs is open | It is sent only the parts of the state it shows, never the commits; a tab of a commit or of a comparison only the references, stashes, worktrees and pull requests its labels need; a tab of conflicts or of a range no state | 0 rows in its `state` messages; nothing to the Branches view when a file is saved | `partsOf` in `src/shared/stateParts.ts`; `test/stateParts.test.ts`; `test/e2e/suite.ts`, the views are sent only what they show, and the Branches view nothing when a file is saved |
| QA6 | A refresh finds the same history, or the same references, stashes, worktrees, working tree, submodules or pull requests | What a view was sent is not sent again: of the commits only those after the ones it has, the row of the uncommitted changes apart; of every other part only one whose key changed; nothing at all when none did | only new rows and the parts that changed | `stateFor` in `src/extension/sentParts.ts`; `test/stateParts.test.ts`; `test/e2e/suite.ts`, the commits are sent to the graph once |
| QA7 | A commit with a huge diff is opened in a tab | The tab opens; files past 20,000 lines say so and link to the diff editor; a file over 800 lines, and every file after 5,000 lines shown, starts folded | 20,000 / 800 / 5,000 lines | README, *A commit in an editor tab*; *none* by name |
| QA8 | Two branches far apart are compared | The newest 2,000 commits on each side are listed, and not looked for on the other side beyond that | 2,000 commits | the engine's `compare.read-sides` |
| QA9 | A section of the references has more than 100 entries | It lists 100 and **Show all** | 100 entries; a filter box from 15 references | README, *Graph* |
| QA10 | Something runs for less than half a second | The status bar does not flicker | shown after 500 ms | `SHOWN_AFTER` in `src/extension/activity.ts` |
| QA11 | Files keep changing in the working tree | The repository is read once per burst | 400 ms after `.git` changes, 1 s after the working tree's | `REFRESH_DELAY`, `WORKTREE_REFRESH_DELAY` in `src/extension/controller.ts` |
| QA12 | The resolution of conflicts is worked out on a large file | It gives up rather than hang | 50,000 lines, 1,500 edits | `MAX_LINES`, `MAX_EDITS` in `src/shared/conflicts.ts`; `test/conflicts.test.ts` |
| QA13 | Every view in each of the five default themes | Buttons have a shape against the panel; labels in the color of their line | screenshots | `npm run test:ui` |
| QA14 | Any operation that can lose work, with `gitardo.commandPreview` `off` | It asks first | 100% of the list in the README's Command preview | the cards' Danger lines; `test/e2e/suite.ts` for some |

Measured again on 2026-11-24, after milestone M44, with VS Code in Xvfb on Linux: QA1 2.8 s; QA2 900 ms until the
view has a new state, since a refresh that finds the same history now sends nothing until the whole history is read
again, and 6 ms to draw it (48 before); QA4 no key waits. The details are in the
[roadmap](../desktop/07-roadmap.md#m44--reads-that-cost-less-in-the-extension-most-of-all-done).

QA1 to QA3 fail the pipeline above one and a half times their time; QA4 is printed, not checked. With the limits, on
2026-11-24, with the minified build and the engine in release, in Xvfb on Linux: QA1 1.8 s from the command chosen
(at most 2.25 s); QA2 225 ms once the second of the working tree's wait is over (at most 375 ms); QA3 not reached, every
commit being loaded before the list was scrolled to its end.

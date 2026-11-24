# Risks and debt

What is known to be weak in Gitardo and not yet fixed. A **risk** may go wrong; **debt** is already wrong and costs on
every change. Most rows come from writing the cards: each names the cards whose **Differs** line it is. A row is removed
in the commit that fixes it, which says so in its message; the [roadmap](07-roadmap.md) says when each is dealt with.

*Source: arc42 §11.*

## Where the user's work can be lost

| | Kind | What it is | What happens if it bites | How likely | What we do |
| --- | --- | --- | --- | --- | --- |
| RS15 | Risk | Removing a worktree takes its links to folders outside it out before `git worktree remove`, which git on Windows would empty through them and git elsewhere would refuse when it does not ignore them; when git then refuses the removal, as it does a worktree with changes, the worktree is left without its links. The empty folders and the branch come after the removal, in that order, and no longer stay behind. `worktree.remove`, `branch.clean-up`; shown by `worktree_remove__a_removal_git_refuses_still_takes_the_links_out_first` in `rust/crates/server/tests/stdio/risks.rs`. | A worktree without its links, such as `node_modules`, until **Remove anyway** removes it or what new worktrees get is brought into it again. | Low: it needs links and a removal git refuses. | Put the links back when git refuses, once the engine reads where each leads |

## Where what runs is not what is shown

| | Kind | What it is | What happens if it bites | How likely | What we do |
| --- | --- | --- | --- | --- | --- |
| RS16 | Debt | Two writes of the extension are not logged, and are not operations: reverting a change or the lines selected in the Flow Diff, and **Save** and **Save all** in **Gitardo Changes**. Each edits or saves a file through its editor in VS Code, as typing and `Ctrl`+`S` do: no git runs, the index and the references do not change, and the edit stays in the history of the editor, where `Ctrl`+`Z` takes it back, which an operation of the engine would lose. Gitardo Desktop, which has no such editor, runs them as operations (`diff.revert-change`, `diff.revert-lines`, `changes.save`). What new worktrees get, saved from its tab and the four entries of the Explorer, is planned and logged by the engine since M48. | What a revert or a save changed is not in the log, as what is typed in a file is not. | Certain | Accepted: edits of a file in its editor, not operations |
| RS17 | Debt | The README says more than the code in places: the log of written resolutions has only `write <file>` lines, not the counts it describes; the README says the resolutions with AI are confirmed, while writing from the review asks nothing unless the preview is `all`; the README's list of what asks first leaves out removing a submodule and the forced deinitialize, which do ask. A few entries are named differently on screen. `conflict.write`, `submodule.remove`, `submodule.deinit`. | A user, or a reviewer, believes the README. | Certain | M1: decide which is right, then fix the other |

## Debt in the code and the tools

| | Kind | What it is | What happens if it bites | How likely | What we do |
| --- | --- | --- | --- | --- | --- |
| RS1 | Debt | Four files are over 1,000 lines: `controller.ts` (2,600), `actions.ts` (1,500), `ops.ts` (1,300), `repo.ts` (1,100). R5.2. | Changes there are slower to review and collide between parallel branches. | Certain | Split along a seam when a change touches one anyway; no rewrite |
| RS4 | Risk | Several windows of the same workspace share its workspace state; the last to write a key wins. | Favorites marked in one window are lost when another writes its own. | Low | Accepted |
| RS5 | Risk | The maintainer works on Windows; macOS and Linux are exercised only by whoever runs the tests there. | A path, a link or a case-sensitivity problem ships unnoticed. | Medium | Run `npm test` on the other two before a release, when a machine is at hand |
| RS18 | Debt | QA4 of [12-quality.md](12-quality.md), typing a message, gives no time, so `test/perf/vscode.mjs` prints it and fails on QA1 to QA3 only. Q1590. | Typing gets slower on large repositories without anyone noticing. | Low | A time for QA4, once Q1590 is answered |
| RS20 | Debt | Ten areas have no cards yet (D33). | A change there has no acceptance list to be checked against. | Certain | Their cards are written when they are next changed |

# Risks and debt

What is known to be weak in Gitardo and not yet fixed. A **risk** may go wrong; **debt** is already wrong and costs on
every change. Most rows come from writing the cards: each names the cards whose **Differs** line it is. A row is removed
in the commit that fixes it, which says so in its message; the [roadmap](07-roadmap.md) says when each is dealt with.

*Source: arc42 §11.*

## Where the user's work can be lost

| | Kind | What it is | What happens if it bites | How likely | What we do |
| --- | --- | --- | --- | --- | --- |
| RS13 | Risk | A branch is given to git by its short name where a tag or a remote branch may have the same name: deleting it from its remote, starting a branch of a worktree from it. `branch.delete-remote`, `worktree.create-for-branch`. | git refuses, or starts the new branch from the tag. | Low | M3 |
| RS14 | Risk | **Clean Up Branches** lists the branches merged into `HEAD` but deletes them with `git branch -d`, which checks against the upstream when there is one: such a branch is refused, and the commands after it do not run, while the worktrees removed before stay removed. `branch.cleanup`. | A cleanup half done. | Low | M3 |
| RS15 | Risk | When the second command of removing or renaming a worktree fails (`git branch -d` refusing an unmerged branch), the empty parent folders stay, as `finish` runs only after every command succeeded. On Windows, the links to folders are taken out before a removal that git may then refuse. `worktree.remove`, `worktree.rename`. | Empty folders left; a worktree without its links. | Low | M3 |

## Where what runs is not what is shown

| | Kind | What it is | What happens if it bites | How likely | What we do |
| --- | --- | --- | --- | --- | --- |
| RS7 | Debt | The todo list and the messages of an interactive rebase are written to a temporary folder while the operation is built, before the question, with no `write` line in the log; they are deleted only when the rebase ends in the same command. For editing an older message, squash and split, the question shows neither the todo list nor the new message. Some reads of these features run without `GIT_OPTIONAL_LOCKS=0`. `rebase.interactive`, `rewrite.reword`, `rewrite.squash`, `rewrite.split`; threat T12. | Temporary folders with commit messages pile up; the user agrees to a rebase without seeing its steps. | Certain, at every stop or cancel | M3 |
| RS16 | Debt | Gitardo writes settings and files outside `Controller.run`: the page of what new worktrees get and the four Explorer entries write settings; reverting one change in the Flow Diff edits and saves the file; **Save** in the Changes view saves editors. None is shown first or logged. `setup.save`, `setup.explorer-link`, `setup.explorer-copy`, `setup.explorer-stop`. | What changed is not in the log. | Certain | M3, or a decision that these are not operations |
| RS17 | Debt | The README says more than the code in places: the log of written resolutions has only `write <file>` lines, not the counts it describes; the README says the resolutions with AI are confirmed, while writing from the review asks nothing unless the preview is `all`; the README's list of what asks first leaves out removing a submodule and the forced deinitialize, which do ask. A few entries are named differently on screen. `conflict.write`, `submodule.remove`, `submodule.deinit`. | A user, or a reviewer, believes the README. | Certain | M1: decide which is right, then fix the other |

## Debt in the code and the tools

| | Kind | What it is | What happens if it bites | How likely | What we do |
| --- | --- | --- | --- | --- | --- |
| RS1 | Debt | Four files are over 1,000 lines: `controller.ts` (2,600), `actions.ts` (1,500), `ops.ts` (1,300), `repo.ts` (1,100). R5.2. | Changes there are slower to review and collide between parallel branches. | Certain | Split along a seam when a change touches one anyway; no rewrite |
| RS4 | Risk | Several windows of the same workspace share its workspace state; the last to write a key wins. | Favorites marked in one window are lost when another writes its own. | Low | Accepted |
| RS5 | Risk | The maintainer works on Windows; macOS and Linux are exercised only by whoever runs the tests there. | A path, a link or a case-sensitivity problem ships unnoticed. | Medium | Run `npm test` on the other two before a release, when a machine is at hand |
| RS18 | Debt | QA4 of [12-quality.md](12-quality.md), typing a message, gives no time, so `test/perf/vscode.mjs` prints it and fails on QA1 to QA3 only. Q1590. | Typing gets slower on large repositories without anyone noticing. | Low | A time for QA4, once Q1590 is answered |
| RS20 | Debt | Ten areas have no cards yet (D33). | A change there has no acceptance list to be checked against. | Certain | Their cards are written when they are next changed |

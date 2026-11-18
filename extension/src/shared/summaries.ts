/**
 * What a repository, a worktree or a submodule is on (`repo.worktree-summary`): the branch checked out there, the files
 * changed and the commits to pull and to push, as the engine reads them for the rows of the Repositories view
 * (`read/repo.worktree-summary`, `repository/summaries`, milestone M22) or as the read of the repository shown says
 * them, and the words a row says them with.
 */

import { type RepositoryState, type Worktree, checkedOutBranch } from './state';

/** What a folder has checked out (`SummaryHeadJson` of the crate `protocol`). */
export type SummaryHead = { kind: 'branch'; name: string } | { kind: 'detached'; commit: string } | { kind: 'noCommits' } | { kind: 'missing' };

/** The branch checked out in a folder, how many files are changed there, and how far it is from its upstream. */
export interface WorktreeSummary {
  head: SummaryHead;
  /** How many files are changed and not committed, untracked files included. */
  changed: number;
  /** The commits to push; null for a branch without an upstream, or detached. */
  ahead: number | null;
  /** The commits to pull; null for a branch without an upstream, or detached. */
  behind: number | null;
}

/** What the engine says of a folder it follows (`FolderSummaryJson`). */
export interface FolderSummary {
  folder: string;
  /** What it is on, as read last; null when no read of it answered yet. */
  summary: WorktreeSummary | null;
  /** Whether the last read of it failed, and so `summary` is the one before. */
  isStale: boolean;
  /** The worktrees git lists for its repository, when it is one of the folders asked and git could list them. */
  worktrees: Worktree[] | null;
}

/** The summary of a repository from its own read (`WorktreeSummary::of_state` of the crate `model`). */
export function summaryOfState(state: Pick<RepositoryState, 'head' | 'branches' | 'workingTree'>): WorktreeSummary {
  const branch = checkedOutBranch(state);
  const head: SummaryHead =
    branch !== null ? { kind: 'branch', name: branch } : state.head.commit ? { kind: 'detached', commit: state.head.commit.slice(0, 7) } : { kind: 'noCommits' };
  const upstream = branch === null ? null : (state.branches.find((each) => each.name === branch)?.upstream ?? null);
  const followed = upstream && !upstream.isGone ? upstream : null;
  const { staged, unstaged, untracked, conflicted } = state.workingTree;
  const paths = new Set([...staged.map((file) => file.path), ...unstaged.map((file) => file.path), ...untracked, ...conflicted.map((file) => file.path)]);
  return { head, changed: paths.size, ahead: followed?.ahead ?? null, behind: followed?.behind ?? null };
}

/** What a row says a folder is on, part by part; a part with nothing to say is empty. */
export interface SummaryWords {
  /** The branch, `detached at 3f2a1c9`, `no commits yet` or `missing`. */
  head: string;
  /** `● 3` when 3 files are changed and not committed. */
  changed: string;
  /** `↓2 ↑1`: the commits to pull, then those to push, each only when there are some. */
  distance: string;
}

/** The words of a summary, as the desktop writes them (`repo.worktree-summary`). */
export function summaryWords(summary: WorktreeSummary): SummaryWords {
  const { head } = summary;
  const said =
    head.kind === 'branch' ? head.name : head.kind === 'detached' ? `detached at ${head.commit}` : head.kind === 'noCommits' ? 'no commits yet' : 'missing';
  const distance = [summary.behind ? `↓${summary.behind}` : '', summary.ahead ? `↑${summary.ahead}` : ''].filter(Boolean).join(' ');
  return { head: said, changed: summary.changed > 0 ? `● ${summary.changed}` : '', distance };
}

/** What a summary says to a screen reader: `on main, 3 files changed, 2 to pull, 1 to push`. */
export function summarySpoken(summary: WorktreeSummary): string {
  const { head } = summary;
  const on = head.kind === 'branch' ? `on ${head.name}` : head.kind === 'detached' ? `detached at ${head.commit}` : head.kind === 'noCommits' ? 'no commits yet' : 'missing';
  return [
    on,
    summary.changed > 0 ? `${summary.changed} ${summary.changed === 1 ? 'file' : 'files'} changed` : '',
    summary.behind ? `${summary.behind} to pull` : '',
    summary.ahead ? `${summary.ahead} to push` : '',
  ]
    .filter(Boolean)
    .join(', ');
}

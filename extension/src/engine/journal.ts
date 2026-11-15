/**
 * What the engine's journal is told of an operation the extension runs in TypeScript, so that **Undo**, which the engine
 * runs, finds the newest operation whichever of the two ran it (`journal/record`, `docs/transition/02-protocol.md`).
 */

import type { RepoState } from '../shared/protocol';

/** What is checked out, as the engine names it: the branch by its full name, none when detached. */
export interface Head {
  commit: string | null;
  branch: string | null;
}

/** What Undo needs of an operation (`UndoRecord` of `rust/crates/protocol/src/journal.rs`). */
export type UndoRecord =
  | { kind: 'reflog' }
  | { kind: 'notUndoable'; reason: string }
  | { kind: 'branchCreated'; name: string }
  | { kind: 'branchDeleted'; name: string; commit: string }
  | { kind: 'stashDropped'; message: string; commit: string }
  | { kind: 'conflictsResolved'; files: string[] }
  | { kind: 'squashMerge' }
  | { kind: 'series'; headBefore: string }
  | { kind: 'splitLast'; commit: string };

/** The newest entry of the journal, for the label of Undo. */
export interface JournalEntry {
  number: number;
  title: string;
  undoable: boolean;
  head: Head;
}

/** What is checked out in a state the extension read. */
export function headOf(state: Pick<RepoState, 'head' | 'branch'>): Head {
  return { commit: state.head, branch: state.branch ? `refs/heads/${state.branch}` : null };
}

/**
 * What Undo needs of an operation that said nothing of its own, from the states read before and after it: a move of
 * `HEAD` is taken back by the reflog; a change of a reference Undo does not take back is said with its reason; and an
 * operation that changed only files and the index leaves nothing, so that Undo goes past it.
 */
export function recordOf(before: RepoState, after: RepoState): UndoRecord | null {
  if (before.head !== after.head || before.branch !== after.branch) return { kind: 'reflog' };
  const changed = (pick: (state: RepoState) => unknown) => JSON.stringify(pick(before)) !== JSON.stringify(pick(after));
  const what = [
    changed((state) => state.branches.map((branch) => [branch.name, branch.hash])) && 'branches',
    changed((state) => state.remoteBranches.map((branch) => [branch.name, branch.hash])) && 'remote branches',
    changed((state) => state.tags.map((tag) => [tag.name, tag.hash])) && 'tags',
    changed((state) => state.stashes.map((stash) => stash.hash)) && 'stashes',
    changed((state) => state.worktrees.map((worktree) => [worktree.path, worktree.hash, worktree.branch])) && 'worktrees',
    changed((state) => state.remotes) && 'remotes',
    changed((state) => state.submodules.map((submodule) => [submodule.path, submodule.head])) && 'submodules',
  ].filter((name): name is string => Boolean(name));
  if (!what.length) return null;
  const list = what.length === 1 ? what[0] : `${what.slice(0, -1).join(', ')} and ${what[what.length - 1]}`;
  return { kind: 'notUndoable', reason: `It changed the ${list}, which Undo does not take back.` };
}

import type { Row } from '../shared/protocol';

/** The merges the graph folds into one row, for one repository. */
export type Folds = ReadonlySet<string>;

export const NO_FOLDS: Folds = new Set();

/** The folds of a repository as a view keeps them, through a reload and a restart of VS Code. */
export interface SavedFolds {
  root: string;
  merges: string[];
}

/** How many repositories a view keeps the folds of, the ones shown last, and how many merges of each, the ones folded last. */
export const KEPT_REPOSITORIES = 20;
export const KEPT_MERGES = 200;

/** The folds with a merge folded, or unfolded. The same folds when it already is. */
export function foldMerge(folds: Folds, merge: string, folded: boolean): Folds {
  if (folds.has(merge) === folded) return folds;
  const merges = new Set(folds);
  if (folded) merges.add(merge);
  else merges.delete(merge);
  return merges;
}

/**
 * The folds without the merges that are no longer in the history, after a rebase or a reset: `rows` must be the whole
 * history. The same folds when none is gone.
 */
export function withoutGone(folds: Folds, rows: readonly Pick<Row, 'hash'>[]): Folds {
  if (!folds.size) return folds;
  const listed = new Set(rows.map((row) => row.hash));
  const kept = [...folds].filter((merge) => listed.has(merge));
  return kept.length === folds.size ? folds : new Set(kept);
}

/** The folds kept for a repository; none when nothing is kept, or what is kept is not what this version keeps. */
export function recallFolds(saved: unknown, root: string): Folds {
  const entry = Array.isArray(saved) ? saved.find((candidate: Partial<SavedFolds> | null) => candidate?.root === root) : undefined;
  if (!entry || !Array.isArray(entry.merges)) return NO_FOLDS;
  return new Set(entry.merges.filter((merge: unknown) => typeof merge === 'string'));
}

/** What a view keeps of the folds of the repositories, once those of `root` are these: they go last, as the ones shown last. */
export function rememberFolds(saved: unknown, root: string, folds: Folds): SavedFolds[] {
  const others = (Array.isArray(saved) ? (saved as (SavedFolds | null)[]) : []).filter((entry): entry is SavedFolds => !!entry && entry.root !== root);
  if (!folds.size) return others.slice(-KEPT_REPOSITORIES);
  return [...others, { root, merges: [...folds].slice(-KEPT_MERGES) }].slice(-KEPT_REPOSITORIES);
}

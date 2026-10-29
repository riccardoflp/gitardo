import type { Row } from '../shared/protocol';

/**
 * The merges the graph folds into one row, for one repository: those folded one by one, or, once **Fold all merges** is
 * chosen, every merge, those loaded later too, but the ones unfolded one by one since.
 */
export interface Folds {
  /** Every merge is folded, but those in `merges`. */
  all: boolean;
  /** The merges folded one by one; while `all` is on, the merges unfolded one by one. */
  merges: ReadonlySet<string>;
}

export const NO_FOLDS: Folds = { all: false, merges: new Set() };
export const ALL_FOLDED: Folds = { all: true, merges: new Set() };

/** The folds of a repository as a view keeps them, through a reload and a restart of VS Code. */
export interface SavedFolds {
  root: string;
  all?: boolean;
  merges: string[];
}

/** How many repositories a view keeps the folds of, the ones shown last, and how many merges of each, the ones folded or unfolded last. */
export const KEPT_REPOSITORIES = 20;
export const KEPT_MERGES = 200;

/** The folds with a merge folded, or unfolded. The same folds when it already is. */
export function foldMerge(folds: Folds, merge: string, folded: boolean): Folds {
  // While every merge is folded, the merges listed are those that are not.
  const listed = folded !== folds.all;
  if (folds.merges.has(merge) === listed) return folds;
  const merges = new Set(folds.merges);
  if (listed) merges.add(merge);
  else merges.delete(merge);
  return { all: folds.all, merges };
}

/** The merges folded, among them those of the rows while every merge is folded. */
export function foldedAmong(folds: Folds, rows: readonly Pick<Row, 'hash' | 'parents'>[]): ReadonlySet<string> {
  if (!folds.all) return folds.merges;
  // A stash keeps only its first parent in the graph: a row with more is a merge.
  return new Set(rows.filter((row) => row.parents.length > 1 && !folds.merges.has(row.hash)).map((row) => row.hash));
}

/**
 * The folds without the merges that are no longer in the history, after a rebase or a reset: `rows` must be the whole
 * history. The same folds when none is gone.
 */
export function withoutGone(folds: Folds, rows: readonly Pick<Row, 'hash'>[]): Folds {
  if (!folds.merges.size) return folds;
  const listed = new Set(rows.map((row) => row.hash));
  const kept = [...folds.merges].filter((merge) => listed.has(merge));
  return kept.length === folds.merges.size ? folds : { all: folds.all, merges: new Set(kept) };
}

/** The folds kept for a repository; none when nothing is kept, or what is kept is not what this version keeps. */
export function recallFolds(saved: unknown, root: string): Folds {
  const entry = Array.isArray(saved) ? saved.find((candidate: Partial<SavedFolds> | null) => candidate?.root === root) : undefined;
  if (!entry || !Array.isArray(entry.merges)) return NO_FOLDS;
  return { all: entry.all === true, merges: new Set(entry.merges.filter((merge: unknown) => typeof merge === 'string')) };
}

/** What a view keeps of the folds of the repositories, once those of `root` are these: they go last, as the ones shown last. */
export function rememberFolds(saved: unknown, root: string, folds: Folds): SavedFolds[] {
  const others = (Array.isArray(saved) ? (saved as (SavedFolds | null)[]) : []).filter((entry): entry is SavedFolds => !!entry && entry.root !== root);
  if (!folds.all && !folds.merges.size) return others.slice(-KEPT_REPOSITORIES);
  const merges = [...folds.merges].slice(-KEPT_MERGES);
  return [...others, folds.all ? { root, all: true, merges } : { root, merges }].slice(-KEPT_REPOSITORIES);
}

/**
 * What the list of commits keeps while the pages of a read come (D54, D72): whether the commits listed are those of
 * before with more under them, and how long the list and how wide its graph stay while the same history is read again
 * from its first page.
 */
import { WORKTREE, type Row } from '../shared/protocol';

/**
 * Whether `rows` are the rows of `before` with others under them, or none: the pages of a read as they come, which
 * leave the commits listed what they were. The row of the uncommitted changes, which comes again with each state for
 * the number in its text, counts for being there. The rows a view keeps are the same objects from a state to the
 * next, so that each is told apart at a glance.
 */
export function addedUnder(before: readonly Row[], rows: readonly Row[]): boolean {
  if (rows.length < before.length) return false;
  return before.every((row, index) => row === rows[index] || (index === 0 && row.hash === WORKTREE && rows[0].hash === WORKTREE));
}

/** What a list keeps of the history it lists while its commits are read again: how many it had, and how many lanes. */
export interface Held {
  /** The history listed: the repository and what the graph is narrowed to. */
  listing: string;
  /** How many rows were loaded, the commits of folded merges among them. */
  loaded: number;
  /** The most lanes a row of the list had. */
  lanes: number;
}

/**
 * What the list keeps once it has `loaded` rows of the history `listing`, whose rows have `lanes` lanes at most (D72).
 * While the same history has fewer rows than it had and more are to come, its commits are being read again from the
 * first page: the list keeps the length it had, so that it stays where it is scrolled to, and the graph its width.
 * Another history, a read that ended, and a list that has grown back to what it was keep only what they are.
 */
export function held(before: Held | null, listing: string, loaded: number, lanes: number, more: boolean): Held {
  if (!before || before.listing !== listing || !more || loaded >= before.loaded) return { listing, loaded, lanes };
  return { listing, loaded: before.loaded, lanes: Math.max(before.lanes, lanes) };
}

/** What says which history a state lists: its repository, and what narrows the graph or changes its rows. */
export function listingOf(state: { root: string; filter: unknown; scope: unknown; firstParent: boolean; showStashes: boolean }): string {
  return JSON.stringify([state.root, state.filter, state.scope, state.firstParent, state.showStashes]);
}

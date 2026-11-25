import { createHash } from 'node:crypto';
import { keyedBy, laidOutOf } from '../engine/graph';
import type { Filter, GraphLine, HostMessage, RepoState, Row, StatePart, ViewKind } from '../shared/protocol';
import { draftOf, partOf, partsIn, partsOf } from '../shared/stateParts';

/**
 * What each view is sent of the state (D88, QA5, QA6): of the parts its kind shows, only those that changed since it
 * was last sent them, as the commits already were.
 */

/**
 * What a view was last sent of the state: the key of each part, and of the rows the list the view has the first of, how
 * many of them, the row of the uncommitted changes counted, what identifies those, and whether more were said to come.
 */
export interface Sent {
  parts: ReadonlyMap<StatePart, string>;
  rows?: { key: string; count: number; hasMore: boolean; of?: readonly Row[] };
}

/** How many commits one message carries at most, so that no view waits for a whole history to draw its first rows (QA6). */
export interface Most {
  /** To a view that keeps none of the commits it has: the first page, which it draws at once. */
  first: number;
  /** To a view that keeps the commits it has: a piece of those that come behind them. */
  piece: number;
}

/** No bound: every commit the view has not got in one message. */
const ALL: Most = { first: Infinity, piece: Infinity };

/** What a part was last keyed from, field by field, and its key: the same values have the same key. */
const lastKeyed = new Map<StatePart, { values: [string, unknown][]; key: string }>();

/** The fields of a part and their values, which the state shares with the one before it where they did not change. */
function valuesOf(state: RepoState, part: Exclude<StatePart, 'rows'>): [string, unknown][] {
  if (part === 'draft') return [['draft', draftOf(state.rows)]];
  return Object.entries(partOf(state, part));
}

/**
 * Identifies a part of the state but the rows: two states with the same key have the same part. A read of the
 * repository gives new objects even where nothing changed, so it is the text of the part that is keyed; when its
 * fields are the very values keyed last, as for a state that only added rows, the key is the same without it.
 */
export function partKey(state: RepoState, part: Exclude<StatePart, 'rows'>): string {
  const values = valuesOf(state, part);
  const last = lastKeyed.get(part);
  if (last && last.values.length === values.length && last.values.every(([field, value], index) => values[index][0] === field && values[index][1] === value)) {
    return last.key;
  }
  const key = createHash('sha1').update(JSON.stringify(Object.fromEntries(values))).digest('hex');
  lastKeyed.set(part, { values, key });
  return key;
}

/** Whether the lines of two cells of the graph are drawn the same. */
function sameLines(one: readonly GraphLine[], other: readonly GraphLine[]): boolean {
  if (one.length !== other.length) return false;
  for (let index = 0; index < one.length; index++) {
    const line = one[index];
    const second = other[index];
    if (line.from !== second.from || line.to !== second.to || line.color !== second.color || !line.dashed !== !second.dashed) return false;
  }
  return true;
}

/** Whether two rows are the same commit, or the same stash, drawn the same way: a view that has one has the other. */
function sameRow(one: Row, other: Row): boolean {
  if (one === other) return true;
  if (one.hash !== other.hash || one.stash !== other.stash || one.parents.length !== other.parents.length) return false;
  if (one.graph.col !== other.graph.col || one.graph.color !== other.graph.color) return false;
  if (one.parents.some((parent, index) => parent !== other.parents[index])) return false;
  return sameLines(one.graph.top, other.graph.top) && sameLines(one.graph.bottom, other.graph.bottom);
}

/**
 * How many commits a view keeps of those it has, at their start and at their end, when it is brought to the commits of
 * a state: the ones that are the same, drawn the same way. `have` is how many commits the view has, the first of
 * `before` from `beforeFrom`; `now` are the commits of the state from `nowFrom`, the row of the uncommitted changes of
 * each list left out. The commits kept at the end are found only for a view that is to have every commit of the state.
 */
function keptOf(before: readonly Row[], beforeFrom: number, have: number, now: readonly Row[], nowFrom: number): { start: number; end: number } {
  const count = now.length - nowFrom;
  const most = Math.min(have, count);
  let start = 0;
  while (start < most && sameRow(before[beforeFrom + start], now[nowFrom + start])) start++;
  let end = 0;
  while (end < most - start && sameRow(before[beforeFrom + have - 1 - end], now[now.length - 1 - end])) end++;
  return { start, end };
}

/**
 * What brings a view of this kind from what it was sent before to this state, and what it has been sent once it gets
 * it; no message when it already has every part of it. `rowsKey` identifies the first `count` rows of the state, the row
 * of the uncommitted changes among them: rows with the same key are the same commits laid out the same way. Of the
 * commits, the view is sent those it has not got: the ones after those it has when the rows are the same as far as
 * that, and otherwise the ones between those it keeps at the start and at the end of its list, as when the row of the
 * uncommitted changes of another worktree moves the lines of the first rows (`repo.share-history`) or a commit is made
 * on top. No message carries more commits than `most` says: `more` is true while the view has not got them all, and
 * the same state sent to it again brings the next piece.
 */
export function stateFor(
  kind: ViewKind,
  state: RepoState,
  before: Sent | undefined,
  rowsKey: (count: number) => string,
  most: Most = ALL,
): { message?: HostMessage; sent: Sent; more: boolean } {
  const wanted = partsOf(kind);
  const parts = new Map(before?.parts);
  const changed: StatePart[] = [];
  for (const part of wanted) {
    if (part === 'rows') continue;
    const key = partKey(state, part);
    if (parts.get(part) === key) continue;
    parts.set(part, key);
    changed.push(part);
  }
  const others = (): HostMessage | undefined => (changed.length ? { type: 'state', state: partsIn(state, changed), parts: changed } : undefined);
  if (!wanted.includes('rows')) return { message: others(), sent: { parts }, more: false };

  // Most reads follow a change to the files, which leaves the history as it was, and loading more of it only adds
  // commits after the others: the thousands a view already has are not sent to it again.
  const total = state.rows.length;
  const old = before?.rows;
  if (old && old.count === total && old.hasMore === state.hasMore && rowsKey(total) === old.key) {
    return { message: others(), sent: { parts, rows: { ...old, of: state.rows } }, more: false };
  }
  // The row of the uncommitted changes is not among the commits: what is kept counts the commits alone.
  const from = draftOf(state.rows) ? 1 : 0;
  const count = total - from;
  let kept = { start: 0, end: 0 };
  if (old && old.count <= total && rowsKey(old.count) === old.key) kept.start = Math.max(0, old.count - from);
  else if (old?.of) {
    const oldFrom = draftOf(old.of) ? 1 : 0;
    kept = keptOf(old.of, oldFrom, old.count - oldFrom, state.rows, from);
  }
  // More than one message carries: the commits kept at the end wait for those before them, which come first.
  const bound = kept.start + kept.end > 0 ? most.piece : most.first;
  if (count - kept.start - kept.end > bound) kept.end = 0;
  const upTo = Math.min(count - kept.end, kept.start + bound);
  const have = kept.end ? count : upTo;
  const hasMore = have < count || state.hasMore;
  const rows = { key: rowsKey(from + have), count: from + have, hasMore, of: state.rows };
  const keeps = kept.start > 0 || kept.end > 0;
  const message: HostMessage = {
    type: 'state',
    state: { ...partsIn(state, changed), rows: state.rows.slice(from + kept.start, from + upTo), hasMore },
    parts: [...changed, 'rows'],
    ...(keeps ? { kept: kept.start } : {}),
    ...(kept.end ? { keptLast: kept.end } : {}),
  };
  return { message, sent: { parts, rows }, more: have < count };
}

/**
 * Identifies the first `count` rows of a state: two states with the same key have the same rows, as far as that.
 * A commit never changes, so its hash stands for all of it, and the lines of the graph of a row only depend on the rows
 * above it. The row of the uncommitted changes only counts for being there or not, and for the commit it is made on,
 * which move the lines of the graph: how many changes it says there are is sent apart.
 * The filter is part of it because the history of a file gives the same commits other parents, and so is following
 * only the first parent of each merge, which leaves a merge one parent.
 */
export function rowsKey(state: RepoState, filter: Filter | null, count = state.rows.length): string {
  // The rows of the engine's reads keep what identifies them as they are laid out, page after page.
  const laid = laidOutOf(state.rows);
  if (laid) return laid.keyAt(count);
  const hash = createHash('sha1');
  hash.update(`${state.root}\0${JSON.stringify(filter)}\0${state.firstParent}\0`);
  for (let index = 0; index < count; index++) hash.update(keyedBy(state.rows[index]));
  return hash.digest('hex');
}

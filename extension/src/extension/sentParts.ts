import { createHash } from 'node:crypto';
import type { HostMessage, RepoState, StatePart, ViewKind } from '../shared/protocol';
import { commitsOf, draftOf, partOf, partsIn, partsOf } from '../shared/stateParts';

/**
 * What each view is sent of the state (D88, QA5, QA6): of the parts its kind shows, only those that changed since it
 * was last sent them, as the commits already were.
 */

/** What a view was last sent of the state: the key of each part, and of the rows how many, and whether more were said to come. */
export interface Sent {
  parts: ReadonlyMap<StatePart, string>;
  rows?: { key: string; count: number; hasMore: boolean };
}

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

/**
 * What brings a view of this kind from what it was sent before to this state, and what it has been sent once it gets
 * it; no message when it already has every part of it. `rowsKey` identifies the first `count` rows of the state, the row
 * of the uncommitted changes among them: rows with the same key are the same commits laid out the same way.
 */
export function stateFor(kind: ViewKind, state: RepoState, before: Sent | undefined, rowsKey: (count: number) => string): { message?: HostMessage; sent: Sent } {
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
  if (!wanted.includes('rows')) return { message: changed.length ? { type: 'state', state: partsIn(state, changed), parts: changed } : undefined, sent: { parts } };

  // Most reads follow a change to the files, which leaves the history as it was, and loading more of it only adds
  // commits after the others: the thousands a view already has are not sent to it again.
  const count = state.rows.length;
  const rows = { key: rowsKey(count), count, hasMore: state.hasMore };
  const old = before?.rows;
  const same = old && old.count <= count && rowsKey(old.count) === old.key ? old.count : 0;
  const sendRows = !old || same !== count || old.hasMore !== state.hasMore;
  if (!changed.length && !sendRows) return { sent: { parts, rows } };
  if (!sendRows) return { message: { type: 'state', state: partsIn(state, changed), parts: changed }, sent: { parts, rows } };
  // The row of the uncommitted changes is not among the commits: what is kept counts the commits alone.
  const kept = same - (draftOf(state.rows) ? 1 : 0);
  const commits = commitsOf(state.rows);
  const message: HostMessage = {
    type: 'state',
    state: { ...partsIn(state, changed), rows: kept > 0 ? commits.slice(kept) : commits, hasMore: state.hasMore },
    parts: [...changed, 'rows'],
    ...(kept > 0 ? { kept } : {}),
  };
  return { message, sent: { parts, rows } };
}

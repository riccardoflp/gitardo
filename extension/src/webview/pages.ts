/** What the list of commits keeps while the pages of a read come (D54): whether the commits listed are those of before with more under them. */
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

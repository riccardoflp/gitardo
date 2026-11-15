/**
 * The newest entry of the engine's journal, which **Undo** takes back, for its label (`journal/last`,
 * `docs/transition/02-protocol.md`).
 */

import type { Head } from '../shared/state';

/** What is checked out, as the engine names it: the branch by its full name, none when detached. */
export type { Head };

/** The newest entry of the journal, for the label of Undo. */
export interface JournalEntry {
  number: number;
  title: string;
  undoable: boolean;
  head: Head;
}

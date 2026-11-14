import { computeDiff, DiffTooComplexError } from '../flow/diff/computeDiff';
import type { CharRange } from '../flow/diff/model';

/** A line of what a side of a conflict changed of the base: one it kept, one it took out, or one it wrote. */
export interface ChangedLine {
  kind: 'same' | 'removed' | 'added';
  /** Which line it is, from 0: of the base for a line taken out, of the side for the others. */
  index: number;
  text: string;
  /** The words that changed, in a line changed rather than replaced. */
  words?: CharRange[];
}

/** How long the lines of a side are compared with those of the base before it is shown as it is, in milliseconds. */
const TIME = 200;

/**
 * What a side changed of the base, line by line as a unified diff shows it: in each change, what was taken out comes
 * before what was written in its place. Null when the two are too far apart to be compared in time.
 */
export function sideChanges(base: string, side: string): ChangedLine[] | null {
  let model;
  try {
    model = computeDiff({ oldText: base, newText: side, leftLabel: '', rightLabel: '', languageId: '', filePath: '', timeoutMs: TIME });
  } catch (error) {
    if (error instanceof DiffTooComplexError) return null;
    throw error;
  }
  const lines: ChangedLine[] = [];
  for (let at = 0; at < model.rows.length; ) {
    const row = model.rows[at];
    if (row.chunkId === undefined) {
      lines.push({ kind: 'same', index: (row.right.lineNumber ?? 1) - 1, text: row.right.text ?? '' });
      at++;
      continue;
    }
    const end = model.chunks[row.chunkId].rowEnd;
    const rows = model.rows.slice(at, end);
    for (const [kind, cells] of [
      ['removed', rows.map((each) => each.left)],
      ['added', rows.map((each) => each.right)],
    ] as const) {
      for (const cell of cells) {
        if (cell.kind === 'filler') continue;
        lines.push({ kind, index: (cell.lineNumber ?? 1) - 1, text: cell.text ?? '', ...(cell.highlights ? { words: cell.highlights } : {}) });
      }
    }
    at = end;
  }
  return lines;
}

/**
 * Line correspondence between the two sides of an aligned model.
 * No `vscode` import — unit-testable with plain node.
 */
import type { AlignedDiffModel } from './model';

/**
 * The right-side line matching `leftLine`: the same row when it has a right
 * line, otherwise (a removed line) the nearest right line above it — where
 * the removed text used to be. 1-based; 1 when there is none above.
 */
export function rightLineForLeft(model: AlignedDiffModel, leftLine: number): number {
  let lastRight = 1;
  for (const row of model.rows) {
    if (row.right.lineNumber !== undefined && row.left.lineNumber === leftLine) {
      return row.right.lineNumber;
    }
    if (row.left.lineNumber === leftLine) {
      return lastRight;
    }
    if (row.right.lineNumber !== undefined) {
      lastRight = row.right.lineNumber;
    }
  }
  return lastRight;
}

/** A selection of one side of the Flow Diff, as Monaco gives it: its lines from 1, and the column its end is at, from 1. */
export interface SideSelection {
  side: 'left' | 'right';
  startLine: number;
  endLine: number;
  endColumn: number;
}

/** Rows of the aligned model, from `start` to `end` left out. */
export interface RowSpan {
  start: number;
  end: number;
}

/** Lines of one side, from 0, the end left out, with their text. */
export interface SideLines {
  start: number;
  end: number;
  text: string[];
}

const cellOf = (row: AlignedDiffModel['rows'][number], side: 'left' | 'right') => (side === 'left' ? row.left : row.right);

/**
 * The rows a selection of a side takes (`diff.select-lines`): from the row of the line it starts on to the row of the
 * line it ends on, the filler rows between them included. A selection that ends at the start of a line does not take
 * that line; a caret alone takes the row of its line. Undefined when a line is not in the model.
 */
export function rowsOfSelection(model: AlignedDiffModel, selection: SideSelection): RowSpan | undefined {
  const { side, startLine, endLine, endColumn } = selection;
  const last = endColumn === 1 && endLine > startLine ? endLine - 1 : endLine;
  const rowOf = (line: number) => model.rows.findIndex((row) => cellOf(row, side).lineNumber === line);
  const first = rowOf(startLine);
  const end = rowOf(last);
  if (first < 0 || end < first) return undefined;
  return { start: first, end: end + 1 };
}

/** Whether some of the rows is of a change: Stage lines, Unstage lines and Revert lines are greyed out otherwise. */
export function hasChangedRow(model: AlignedDiffModel, rows: RowSpan): boolean {
  return model.rows.slice(rows.start, rows.end).some((row) => row.chunkId !== undefined);
}

/**
 * The lines of a side in some rows, as the engine takes them in a change (`diff.stage-lines`): from the first line in
 * them to the last; where the side has only filler rows there, no line, at the place of the next line of that side.
 */
export function sideLinesOfRows(model: AlignedDiffModel, rows: RowSpan, side: 'left' | 'right'): SideLines {
  const inside = model.rows.slice(rows.start, rows.end).map((row) => cellOf(row, side)).filter((cell) => cell.lineNumber !== undefined);
  if (inside.length) {
    const start = (inside[0].lineNumber ?? 1) - 1;
    return { start, end: start + inside.length, text: inside.map((cell) => cell.text ?? '') };
  }
  const after = model.rows.slice(rows.end).find((row) => cellOf(row, side).lineNumber !== undefined);
  const next = after ? (cellOf(after, side).lineNumber ?? 1) - 1 : model.rows.filter((row) => cellOf(row, side).lineNumber !== undefined).length;
  return { start: next, end: next, text: [] };
}

/**
 * What reverting some rows writes in the side of the working tree (`diff.revert-lines`): its lines in those rows are
 * replaced by the lines the left side has in the rows of a change, and kept in the rows that do not change.
 */
export function revertedRows(model: AlignedDiffModel, rows: RowSpan): { right: SideLines; text: string[] } {
  const text: string[] = [];
  for (const row of model.rows.slice(rows.start, rows.end)) {
    const kept = row.chunkId === undefined ? row.right : row.left;
    if (kept.lineNumber !== undefined) text.push(kept.text ?? '');
  }
  return { right: sideLinesOfRows(model, rows, 'right'), text };
}

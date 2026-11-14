import { diffArrays } from 'diff';

/** Lines `start` to `end` of a file, from 1. */
export interface LineRange {
  start: number;
  end: number;
}

/** The lines of a text, whichever line endings it has, with no empty one after the last line break. */
function linesOf(text: string): string[] {
  const lines = text.split(/\r?\n/);
  if (lines.at(-1) === '') lines.pop();
  return lines;
}

/**
 * The lines of a file as committed that some lines of it as it is now come from: the same lines where they are
 * unchanged, and all the lines an edit replaced where they were edited. Null when they are all lines added since.
 */
export function committedRange(committed: string, current: string, { start, end }: LineRange): LineRange | null {
  let first = Infinity;
  let last = -Infinity;
  const take = (from: number, to: number) => {
    first = Math.min(first, from);
    last = Math.max(last, to);
  };
  // Lines of each side passed so far.
  let before = 0;
  let after = 0;
  // A run of changes between two unchanged parts: the lines it took out of the committed file and put in the new one.
  const run: { removed: LineRange | null; added: LineRange | null } = { removed: null, added: null };
  const endOfRun = () => {
    const { removed, added } = run;
    if (removed && added && added.start <= end && added.end >= start) take(removed.start, removed.end);
    run.removed = run.added = null;
  };
  for (const change of diffArrays(linesOf(committed), linesOf(current))) {
    const count = change.value.length;
    if (change.removed) {
      run.removed = { start: run.removed?.start ?? before + 1, end: before + count };
      before += count;
    } else if (change.added) {
      run.added = { start: run.added?.start ?? after + 1, end: after + count };
      after += count;
    } else {
      endOfRun();
      // The selected lines that are among these unchanged ones, by their place in the committed file.
      const from = Math.max(start, after + 1);
      const to = Math.min(end, after + count);
      if (from <= to) take(before + from - after, before + to - after);
      before += count;
      after += count;
    }
  }
  endOfRun();
  return first <= last ? { start: first, end: last } : null;
}

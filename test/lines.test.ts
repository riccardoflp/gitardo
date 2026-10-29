import { describe, expect, it } from 'vitest';
import { committedRange } from '../src/git/lines';

const text = (...lines: string[]) => lines.map((line) => `${line}\n`).join('');

describe('committedRange', () => {
  const committed = text('one', 'two', 'three', 'four', 'five');

  it('keeps the lines of a file as committed', () => {
    expect(committedRange(committed, committed, { start: 2, end: 4 })).toEqual({ start: 2, end: 4 });
    // Whatever the line endings of either side.
    expect(committedRange(committed, committed.replaceAll('\n', '\r\n'), { start: 1, end: 5 })).toEqual({ start: 1, end: 5 });
  });

  it('finds lines moved by lines added or removed above them', () => {
    const current = text('zero', 'one', 'two', 'three', 'four', 'five');
    expect(committedRange(committed, current, { start: 3, end: 4 })).toEqual({ start: 2, end: 3 });
    expect(committedRange(committed, text('one', 'three', 'four', 'five'), { start: 2, end: 3 })).toEqual({ start: 3, end: 4 });
  });

  it('takes the lines an edit replaced, and leaves out the lines added', () => {
    const edited = text('one', 'TWO', 'three', 'four', 'five');
    expect(committedRange(committed, edited, { start: 2, end: 2 })).toEqual({ start: 2, end: 2 });
    const added = text('one', 'two', 'new', 'newer', 'three', 'four', 'five');
    expect(committedRange(committed, added, { start: 2, end: 4 })).toEqual({ start: 2, end: 2 });
    expect(committedRange(committed, added, { start: 3, end: 6 })).toEqual({ start: 3, end: 4 });
    // A selection that goes over lines removed since takes them too.
    expect(committedRange(committed, text('one', 'five'), { start: 1, end: 2 })).toEqual({ start: 1, end: 5 });
  });

  it('has nothing for lines that are all new', () => {
    const added = text('one', 'two', 'new', 'newer', 'three', 'four', 'five');
    expect(committedRange(committed, added, { start: 3, end: 4 })).toBeNull();
    expect(committedRange('', text('only'), { start: 1, end: 1 })).toBeNull();
    expect(committedRange(committed, text('one', 'two', 'three', 'four', 'five', 'six'), { start: 6, end: 6 })).toBeNull();
  });
});

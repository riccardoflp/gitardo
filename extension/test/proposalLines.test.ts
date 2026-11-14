import { describe, expect, it } from 'vitest';
import { choiceText, proposalLines, repeatedKeys, sameLines, summarize } from '../src/shared/proposalLines';

const text = (...lines: string[]) => lines.map((line) => `${line}\n`).join('');

/** What the review says of a proposal: where its lines come from, and what it does with the two sides. */
const read = (ours: string, theirs: string, proposed: string, base: string | null = null) => {
  const { origins } = proposalLines(ours, theirs, proposed, base);
  return { origins, summary: summarize(ours, theirs, proposed, origins) };
};

describe('where the lines of a proposal come from', () => {
  it('pairs the lines that are the same in order, spaces at the ends aside', () => {
    expect(sameLines(['a', ' b', 'c'], ['b', 'c ', 'd'])).toEqual([
      [1, 0],
      [2, 1],
    ]);
    expect(sameLines([], ['a'])).toEqual([]);
  });

  it('marks each line by the side it comes from, both, the base, none, or as empty', () => {
    const { origins } = read(text('same', 'ours'), text('same', 'theirs'), text('same', 'ours', '', 'theirs', 'old', 'mine'), text('same', 'old'));
    expect(origins).toEqual(['both', 'ours', 'blank', 'theirs', 'base', 'new']);
  });

  it('still says where a line comes from when the proposal changes the order of the lines', () => {
    expect(read(text('a', 'b'), text('c'), text('b', 'a', 'c')).origins).toEqual(['ours', 'ours', 'theirs']);
  });
});

describe('what a proposal does with the two sides', () => {
  const ours = text('  "version": "1.2.1",', '  "shipping": true');
  const theirs = text('  "version": "1.3.0",', '  "discounts": true');

  it('takes one side, or both one after the other', () => {
    expect(read(ours, theirs, ours).summary).toEqual({ shape: 'current', ours: 2, theirs: 0, both: 0, base: 0, fresh: 0 });
    expect(read(ours, theirs, theirs).summary.shape).toBe('incoming');
    expect(read(ours, theirs, ours + theirs).summary).toEqual({ shape: 'current-first', ours: 2, theirs: 2, both: 0, base: 0, fresh: 0 });
    expect(read(ours, theirs, theirs + ours).summary.shape).toBe('incoming-first');
  });

  it('mixes them, writes lines of its own, or leaves out both', () => {
    expect(read(ours, theirs, text('  "version": "1.3.0",', '  "shipping": true')).summary).toMatchObject({ shape: 'mixed', ours: 1, theirs: 1, fresh: 0 });
    expect(read(ours, theirs, text('  "version": "2.0.0"')).summary).toMatchObject({ shape: 'mixed', fresh: 1 });
    expect(read(ours, theirs, '').summary.shape).toBe('nothing');
  });

  it('leaves aside empty lines and the spaces at the ends of lines when it tells a side', () => {
    expect(read(text('a', 'b'), text('c'), text('  a', '', 'b  ')).summary.shape).toBe('current');
  });
});

describe('what looks like a mistake in a proposal', () => {
  it('finds a name given a value by each side at the same depth', () => {
    const ours = text('  "version": "1.2.1",', '  "retries": 3,', '  "shipping": true');
    const theirs = text('  "version": "1.3.0",', '  "retries": 3,', '  "discounts": true');
    const proposed = ours + theirs;
    expect(repeatedKeys(proposed, proposalLines(ours, theirs, proposed).origins)).toEqual([{ kind: 'repeated', key: 'version', lines: [0, 3] }]);
  });

  it('finds a variable declared twice, and properties and settings written as they are in code', () => {
    const ours = text('  const count = items.length;', 'timeout = 60');
    const theirs = text('  const count = items.reduce(sum, 0);', 'timeout = 30');
    const proposed = ours + theirs;
    expect(repeatedKeys(proposed, proposalLines(ours, theirs, proposed).origins).map((warning) => warning.kind === 'repeated' && warning.key)).toEqual(['count', 'timeout']);
  });

  it('leaves alone the same name at another depth, a name of one side only, comparisons and arrows', () => {
    const ours = text('a: 1', 'if (x == 1) {', 'x => 1');
    const theirs = text('  a: 2', 'if (x == 2) {', 'x => 2');
    const proposed = ours + theirs;
    expect(repeatedKeys(proposed, proposalLines(ours, theirs, proposed).origins)).toEqual([]);
    expect(repeatedKeys(text('a: 1', 'a: 2'), ['new', 'new'])).toEqual([]);
  });
});

describe('what a choice of the two sides puts in place of a conflict', () => {
  it('is one side, or both in the order chosen, each ending its last line', () => {
    expect(choiceText('current', text('a'), text('b'))).toBe(text('a'));
    expect(choiceText('incoming', text('a'), text('b'))).toBe(text('b'));
    expect(choiceText('current-first', 'a', text('b'))).toBe(text('a', 'b'));
    expect(choiceText('incoming-first', text('a'), text('b'))).toBe(text('b', 'a'));
  });
});

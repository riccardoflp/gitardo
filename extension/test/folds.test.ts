import { describe, expect, it } from 'vitest';
import {
  ALL_FOLDED,
  foldedAmong,
  foldMerge,
  KEPT_MERGES,
  KEPT_REPOSITORIES,
  NO_FOLDS,
  recallFolds,
  rememberFolds,
  withoutGone,
  type Folds,
} from '../src/webview/folds';

const some = (...merges: string[]): Folds => ({ all: false, merges: new Set(merges) });
const rows = [
  { hash: 'm1', parents: ['c1', 'f1'] },
  { hash: 'c1', parents: ['m2'] },
  { hash: 'm2', parents: ['c2', 'f2'] },
  { hash: 'c2', parents: [] },
];

describe('foldMerge', () => {
  it('folds and unfolds one merge', () => {
    const folded = foldMerge(NO_FOLDS, 'm1', true);
    expect(folded).toEqual(some('m1'));
    expect(foldMerge(folded, 'm1', false)).toEqual(some());
  });

  it('gives the same folds back when nothing changes', () => {
    const folded = foldMerge(NO_FOLDS, 'm1', true);
    expect(foldMerge(folded, 'm1', true)).toBe(folded);
    expect(foldMerge(folded, 'm2', false)).toBe(folded);
    expect(foldMerge(ALL_FOLDED, 'm1', true)).toBe(ALL_FOLDED);
  });

  it('while every merge is folded, lists the ones unfolded by hand', () => {
    const unfolded = foldMerge(ALL_FOLDED, 'm1', false);
    expect(unfolded).toEqual({ all: true, merges: new Set(['m1']) });
    expect(foldMerge(unfolded, 'm1', true)).toEqual(ALL_FOLDED);
  });
});

describe('foldedAmong', () => {
  it('is the merges folded one by one', () => {
    const folds = some('m2', 'gone');
    expect(foldedAmong(folds, rows)).toBe(folds.merges);
  });

  it('is every merge of the rows while all are folded, those loaded later too, but those unfolded by hand', () => {
    expect([...foldedAmong(ALL_FOLDED, rows)]).toEqual(['m1', 'm2']);
    const more = [...rows, { hash: 'm3', parents: ['c3', 'f3'] }];
    expect([...foldedAmong(foldMerge(ALL_FOLDED, 'm1', false), more)]).toEqual(['m2', 'm3']);
  });
});

describe('withoutGone', () => {
  it('forgets the merges that are no longer in the history', () => {
    expect(withoutGone(some('m1', 'gone'), rows)).toEqual(some('m1'));
    expect(withoutGone({ all: true, merges: new Set(['gone', 'm2']) }, rows)).toEqual({ all: true, merges: new Set(['m2']) });
  });

  it('gives the same folds back when every merge is still there', () => {
    const folds = some('m1');
    expect(withoutGone(folds, rows)).toBe(folds);
    expect(withoutGone(ALL_FOLDED, rows)).toBe(ALL_FOLDED);
  });
});

describe('rememberFolds and recallFolds', () => {
  it('keeps the folds of each repository apart', () => {
    let saved = rememberFolds(undefined, '/one', some('a'));
    saved = rememberFolds(saved, '/two', some('b', 'c'));
    expect(recallFolds(saved, '/one')).toEqual(some('a'));
    expect(recallFolds(saved, '/two')).toEqual(some('b', 'c'));
    expect(recallFolds(saved, '/three')).toBe(NO_FOLDS);
  });

  it('keeps whether every merge is folded, and the merges unfolded by hand', () => {
    const saved = rememberFolds([], '/one', foldMerge(ALL_FOLDED, 'm1', false));
    expect(saved).toEqual([{ root: '/one', all: true, merges: ['m1'] }]);
    expect(recallFolds(saved, '/one')).toEqual({ all: true, merges: new Set(['m1']) });
    expect(recallFolds(rememberFolds([], '/one', ALL_FOLDED), '/one')).toEqual(ALL_FOLDED);
  });

  it('puts the repository saved last at the end, and forgets one with nothing folded', () => {
    let saved = rememberFolds([], '/one', some('a'));
    saved = rememberFolds(saved, '/two', some('b'));
    saved = rememberFolds(saved, '/one', some('a', 'd'));
    expect(saved.map((entry) => entry.root)).toEqual(['/two', '/one']);
    expect(rememberFolds(saved, '/two', NO_FOLDS).map((entry) => entry.root)).toEqual(['/one']);
  });

  it('keeps as many repositories and merges as it has room for, the last ones', () => {
    let saved: unknown = [];
    for (let index = 0; index < KEPT_REPOSITORIES + 5; index++) saved = rememberFolds(saved, `/repo${index}`, some('m'));
    const kept = saved as { root: string }[];
    expect(kept).toHaveLength(KEPT_REPOSITORIES);
    expect(kept[0].root).toBe('/repo5');

    const many = some(...Array.from({ length: KEPT_MERGES + 10 }, (_, index) => `m${index}`));
    const [entry] = rememberFolds([], '/big', many);
    expect(entry.merges).toHaveLength(KEPT_MERGES);
    expect(entry.merges[0]).toBe('m10');
  });

  it('recalls nothing from what it does not understand', () => {
    expect(recallFolds(null, '/one')).toBe(NO_FOLDS);
    expect(recallFolds({ root: '/one' }, '/one')).toBe(NO_FOLDS);
    expect(recallFolds([null, { root: '/one', merges: 'a' }], '/one')).toBe(NO_FOLDS);
    expect(recallFolds([{ root: '/one', all: 'yes', merges: ['a', 3] }], '/one')).toEqual(some('a'));
    expect(rememberFolds({ not: 'a list' }, '/one', some('a'))).toEqual([{ root: '/one', merges: ['a'] }]);
  });
});

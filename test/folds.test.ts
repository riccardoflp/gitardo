import { describe, expect, it } from 'vitest';
import { foldMerge, KEPT_MERGES, KEPT_REPOSITORIES, NO_FOLDS, recallFolds, rememberFolds, withoutGone } from '../src/webview/folds';

describe('foldMerge', () => {
  it('folds and unfolds one merge', () => {
    const folded = foldMerge(NO_FOLDS, 'm1', true);
    expect([...folded]).toEqual(['m1']);
    expect([...foldMerge(folded, 'm1', false)]).toEqual([]);
  });

  it('gives the same folds back when nothing changes', () => {
    const folded = foldMerge(NO_FOLDS, 'm1', true);
    expect(foldMerge(folded, 'm1', true)).toBe(folded);
    expect(foldMerge(folded, 'm2', false)).toBe(folded);
  });
});

describe('withoutGone', () => {
  it('forgets the merges that are no longer in the history', () => {
    const folds = new Set(['m1', 'm2']);
    expect([...withoutGone(folds, [{ hash: 'm2' }, { hash: 'c1' }])]).toEqual(['m2']);
  });

  it('gives the same folds back when every merge is still there', () => {
    const folds = new Set(['m1']);
    expect(withoutGone(folds, [{ hash: 'm1' }])).toBe(folds);
  });
});

describe('rememberFolds and recallFolds', () => {
  it('keeps the folds of each repository apart', () => {
    let saved = rememberFolds(undefined, '/one', new Set(['a']));
    saved = rememberFolds(saved, '/two', new Set(['b', 'c']));
    expect([...recallFolds(saved, '/one')]).toEqual(['a']);
    expect([...recallFolds(saved, '/two')]).toEqual(['b', 'c']);
    expect(recallFolds(saved, '/three')).toBe(NO_FOLDS);
  });

  it('puts the repository saved last at the end, and forgets one with nothing folded', () => {
    let saved = rememberFolds([], '/one', new Set(['a']));
    saved = rememberFolds(saved, '/two', new Set(['b']));
    saved = rememberFolds(saved, '/one', new Set(['a', 'd']));
    expect(saved.map((entry) => entry.root)).toEqual(['/two', '/one']);
    expect(rememberFolds(saved, '/two', NO_FOLDS).map((entry) => entry.root)).toEqual(['/one']);
  });

  it('keeps as many repositories and merges as it has room for, the last ones', () => {
    let saved: unknown = [];
    for (let index = 0; index < KEPT_REPOSITORIES + 5; index++) saved = rememberFolds(saved, `/repo${index}`, new Set(['m']));
    const kept = saved as { root: string }[];
    expect(kept).toHaveLength(KEPT_REPOSITORIES);
    expect(kept[0].root).toBe('/repo5');

    const many = new Set(Array.from({ length: KEPT_MERGES + 10 }, (_, index) => `m${index}`));
    const [entry] = rememberFolds([], '/big', many);
    expect(entry.merges).toHaveLength(KEPT_MERGES);
    expect(entry.merges[0]).toBe('m10');
  });

  it('recalls nothing from what it does not understand', () => {
    expect(recallFolds(null, '/one')).toBe(NO_FOLDS);
    expect(recallFolds({ root: '/one' }, '/one')).toBe(NO_FOLDS);
    expect(recallFolds([null, { root: '/one', merges: 'a' }], '/one')).toBe(NO_FOLDS);
    expect([...recallFolds([{ root: '/one', merges: ['a', 3] }], '/one')]).toEqual(['a']);
    expect(rememberFolds({ not: 'a list' }, '/one', new Set(['a']))).toEqual([{ root: '/one', merges: ['a'] }]);
  });
});

import { describe, expect, it, vi } from 'vitest';
import { engineRuns } from '../src/engine/actions';
import { engineSettings } from '../src/engine/settings';
import { buttonWords, changedParts, diffTone, forcePushed, hasDiverged, type RangeDiffPair } from '../src/shared/rangeDiff';
import { branchAt, upstreamOf } from './states';

// The menus post what is chosen to the extension; here they are only read.
const posted = vi.hoisted(() => [] as unknown[]);
vi.mock('../src/webview/vscode', () => ({ post: (message: unknown) => posted.push(message) }));
const { rewriteEntries } = await import('../src/webview/menus');

const following = (ahead: number, behind: number, commit = 'bbb') => branchAt('feature', commit, { upstream: upstreamOf('origin/feature', { ahead, behind }) });

describe('range-diff.compare', () => {
  it('after a fetch that found origin/feature force pushed while feature follows it, says which upstream it was', () => {
    expect(forcePushed({ branches: [following(0, 0)] }, { branches: [following(1, 2)] })).toBe('origin/feature');
  });

  it('finds no force push when the branch itself moved, nor when only new commits came in', () => {
    expect(forcePushed({ branches: [following(0, 0)] }, { branches: [following(2, 0, 'ccc')] })).toBeNull();
    expect(forcePushed({ branches: [following(1, 0)] }, { branches: [following(1, 3)] })).toBeNull();
  });

  it('offers What its last rewrite changed on a branch, and Compare with its upstream commit by commit once they diverged', () => {
    expect(
      rewriteEntries(following(0, 0))
        .filter(Boolean)
        .map((entry) => entry && entry !== 'separator' && entry.label),
    ).toEqual(['What its last rewrite changed']);
    const diverged = following(2, 1);
    expect(hasDiverged(diverged)).toBe(true);
    const labels = rewriteEntries(diverged).map((entry) => entry && entry !== 'separator' && entry.label);
    expect(labels).toEqual(['What its last rewrite changed', 'Compare with origin/feature commit by commit']);
  });

  it('runs range-diff.compare on the engine, against the upstream when asked', () => {
    expect(engineRuns({ kind: 'rangeDiff', target: 'branch', name: 'feature', withUpstream: true }, { branches: [], remoteBranches: [], tags: [] })).toEqual([
      {
        feature: 'range-diff.compare',
        target: { kind: 'branch', name: 'feature' },
        answers: { 'range-diff.with': { kind: 'choice', identifier: 'upstream' } },
      },
    ]);
    expect(engineRuns({ kind: 'rangeDiff', target: 'remote', name: 'origin/feature' }, { branches: [], remoteBranches: [], tags: [] })).toEqual([
      { feature: 'range-diff.compare', target: { kind: 'remoteBranch', name: 'origin/feature' }, answers: {} },
    ]);
  });

  it('gives the buttons of the engine the words of VS Code buttons: Compare First, See What Changed', () => {
    expect(buttonWords('Compare first')).toBe('Compare First');
    expect(buttonWords('See what changed')).toBe('See What Changed');
  });
});

describe('range-diff.page', () => {
  it('shows a line of the diff of diffs lost before, gained now, or as a heading', () => {
    expect(diffTone('-    Add B')).toBe('lost');
    expect(diffTone('+    Add B, better')).toBe('gained');
    expect(diffTone('  ## Commit message ##')).toBe('heading');
    expect(diffTone('     context')).toBe('context');
  });

  it('says what changed in a changed pair', () => {
    const pair = (messageChanged: boolean, changesChanged: boolean): RangeDiffPair => ({
      kind: 'changed',
      before: 'a',
      now: 'b',
      subject: 'Add B',
      messageChanged,
      changesChanged,
      diff: [],
    });
    expect(changedParts(pair(true, false))).toBe('message');
    expect(changedParts(pair(true, true))).toBe('message, changes');
  });
});

describe('gitardo.rebase.updateRefs', () => {
  it('is sent to the engine as rebase.updateRefs when set, and left out as As git says', () => {
    expect(engineSettings({ 'rebase.updateRefs': false }, new Set(), {}, 0)['rebase.updateRefs']).toBe(false);
    expect('rebase.updateRefs' in engineSettings({ 'rebase.updateRefs': null }, new Set(['rebase.updateRefs']), {}, 0)).toBe(false);
  });
});

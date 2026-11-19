import { describe, expect, it } from 'vitest';
import type { Branch, PullRequest, RepoState } from '../src/shared/protocol';
import { buildLabels } from '../src/webview/labels';
import { branchAt, headAt, remoteAt, upstreamOf, worktreeAt } from './states';

const pull = (number: number): PullRequest => ({ number, title: `Change ${number}`, url: `https://github.com/o/r/pull/${number}`, draft: false, checks: 'success' });

const state = (overrides: Partial<RepoState>) =>
  ({
    head: headAt('aaa', 'main'),
    branches: [],
    remoteBranches: [],
    tags: [],
    stashes: [],
    worktrees: [],
    remotes: ['origin'],
    pulls: {},
    hosting: { service: 'gitHub', host: 'github.com', remote: 'origin', canCheckOut: true, comparesSinceDiverged: false },
    bisect: null,
    combineUpstream: true,
    ...overrides,
  }) as Partial<RepoState> as RepoState;

describe('the labels of the commits in the graph', () => {
  const feature: Branch = branchAt('feature', 'bbb', { upstream: upstreamOf('origin/feature') });

  it('puts the pull request of a branch on its label, once, where the local branch tracks the remote one', () => {
    const labels = buildLabels(
      state({
        branches: [feature],
        remoteBranches: [
          remoteAt('origin/feature', 'bbb'),
          remoteAt('origin/other', 'ccc'),
        ],
        pulls: { feature: pull(7), other: pull(9) },
      }),
    );
    expect(labels.get('bbb')).toEqual([
      { kind: 'branch', name: 'feature', current: false, worktree: undefined, remote: 'origin/feature', pull: { request: pull(7), branch: 'feature' } },
    ]);
    // A remote branch no local one tracks has it on its own label.
    expect(labels.get('ccc')?.[0].pull).toEqual({ request: pull(9), branch: 'other' });
  });

  it('has no pull request where the repository is not on GitHub, or the branch was never pushed', () => {
    const pulls = { feature: pull(7) };
    const remoteBranches = [remoteAt('origin/feature', 'ccc')];
    expect(buildLabels(state({ branches: [{ ...feature, upstream: null }], pulls })).get('bbb')?.[0].pull).toBeUndefined();
    expect(buildLabels(state({ remoteBranches, pulls, hosting: null })).get('ccc')?.[0].pull).toBeUndefined();
  });

  it('gives a branch and its upstream one label at the same commit, and one each once they part', () => {
    const remoteBranches = [remoteAt('origin/feature', 'bbb')];
    const worktrees = [worktreeAt('C:/work/feature', 'feature')];
    const together = buildLabels(state({ branches: [feature], remoteBranches, worktrees }));
    expect(together.get('bbb')).toEqual([expect.objectContaining({ kind: 'branch', name: 'feature', worktree: 'C:/work/feature', remote: 'origin/feature' })]);
    const apart = buildLabels(state({ branches: [{ ...feature, commit: 'ccc' }], remoteBranches }));
    expect(apart.get('ccc')).toEqual([expect.objectContaining({ kind: 'branch', remote: undefined })]);
    expect(apart.get('bbb')).toEqual([expect.objectContaining({ kind: 'remote', name: 'origin/feature' })]);
  });

  it('gives a branch and its upstream a label each when the settings keep them apart', () => {
    const remoteBranches = [remoteAt('origin/feature', 'bbb')];
    const labels = buildLabels(state({ branches: [feature], remoteBranches, combineUpstream: false }));
    expect(labels.get('bbb')?.map((label) => [label.kind, label.name, label.remote])).toEqual([
      ['branch', 'feature', undefined],
      ['remote', 'origin/feature', undefined],
    ]);
  });

  it('keeps the label of an upstream of another name, and of another remote', () => {
    const renamed: Branch = { ...feature, upstream: upstreamOf('origin/other') };
    const remoteBranches = [
      remoteAt('origin/other', 'bbb'),
      remoteAt('fork/feature', 'bbb'),
    ];
    const labels = buildLabels(state({ branches: [renamed], remoteBranches, remotes: ['origin', 'fork'] }));
    expect(labels.get('bbb')?.map((label) => [label.kind, label.name, label.remote])).toEqual([
      ['branch', 'feature', undefined],
      ['remote', 'origin/other', undefined],
      ['remote', 'fork/feature', undefined],
    ]);
  });
});

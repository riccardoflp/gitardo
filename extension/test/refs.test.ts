import { describe, expect, it } from 'vitest';
import type { Branch, RepoState, Upstream, Worktree } from '../src/shared/protocol';
import { byMatch, canFastForward, recentBranches, refArg, sortRefs } from '../src/shared/refs';
import { branchAt, headAt, remoteAt, tagAt, upstreamOf } from './states';

describe('the order of the references', () => {
  it('lists the branches by name as git lists them, or the one committed to last first', () => {
    const refs = {
      branches: [
        branchAt('main', 'aaa', { date: 1700000000, isCheckedOut: true }),
        branchAt('old', 'bbb', { date: 1600000000 }),
        branchAt('local', 'ccc', { date: 1650000000 }),
      ],
      remoteBranches: [],
      tags: [],
    };
    expect(sortRefs(refs, 'date').branches.map((branch) => branch.name)).toEqual(['main', 'local', 'old']);
    expect(sortRefs(refs, 'name').branches.map((branch) => branch.name)).toEqual(['main', 'old', 'local']);
  });

  it('keeps the order of the names among branches committed to at the same time', () => {
    const refs = {
      branches: [branchAt('a', '1', { date: 5 }), branchAt('b', '2', { date: 9 }), branchAt('c', '3', { date: 5, isCheckedOut: true })],
      remoteBranches: [],
      tags: [
        { name: 'v1', commit: '4', date: 1 },
        { name: 'v2', commit: '5', date: 2 },
      ],
    };
    const sorted = sortRefs(refs, 'date');
    expect(sorted.branches.map((branch) => branch.name)).toEqual(['b', 'a', 'c']);
    expect(sorted.tags.map((tag) => tag.name)).toEqual(['v2', 'v1']);
    expect(refs.branches[0].name).toBe('a');
  });

  it('lists the names that match what was typed best first', () => {
    const names = ['feature/main-menu', 'maintenance', 'origin/main', 'domain', 'main', 'fix', 'origin/maintenance'];
    expect(byMatch(names, (name) => name, 'main')).toEqual(['main', 'origin/main', 'maintenance', 'feature/main-menu', 'origin/maintenance', 'domain']);
    expect(byMatch(names, (name) => name, '')).toBe(names);
    expect(byMatch(['Main', 'xmain'], (name) => name, 'main')).toEqual(['Main', 'xmain']);
  });
});

describe('the name given to git for a reference', () => {
  it('means the branch when a tag has the same name, and keeps a name only one thing has as it is', () => {
    const state = {
      branches: [branchAt('main', 'aaa'), branchAt('release', 'bbb')],
      remoteBranches: [remoteAt('origin/main', 'aaa')],
      tags: [tagAt('release', 'ccc')],
    } as Partial<RepoState> as RepoState;
    expect(refArg('branch', 'release', state)).toBe('refs/heads/release');
    expect(refArg('tag', 'release', state)).toBe('refs/tags/release');
    expect(refArg('branch', 'main', state)).toBe('main');
  });
});

describe('recent branches', () => {
  const moves = [
    'checkout: moving from feature to main',
    'commit: Change a',
    'checkout: moving from main to feature',
    'rebase (finish): returning to refs/heads/main',
    'checkout: moving from fix to main',
    'checkout: moving from 1a2b3c4d5e6f7a8b9c0d1a2b3c4d5e6f7a8b9c0d to fix',
    'checkout: moving from gone to main',
    'checkout: moving from old to gone',
  ].join('\n');

  // The moves of HEAD as the engine reads them, the latest first, with the branches there are and the one checked out.
  const state = (subjects: string, names: string[], current: string | null) => ({
    reflogSubjects: subjects ? subjects.split('\n') : [],
    branches: names.map((name) => branchAt(name, 'a'.repeat(40))),
    head: headAt('a'.repeat(40), current),
  });

  it('are the ones checked out last, the latest first, without the current one and the ones that are gone', () => {
    expect(recentBranches(state(moves, ['feature', 'fix', 'main', 'old', 'never'], 'main'))).toEqual(['feature', 'fix', 'old']);
  });

  it('count the one just left when no branch is checked out, and stop at the limit', () => {
    expect(recentBranches(state(moves, ['feature', 'fix', 'main', 'old'], null))).toEqual(['main', 'feature', 'fix', 'old']);
    expect(recentBranches(state(moves, ['feature', 'fix', 'main', 'old'], null), 2)).toEqual(['main', 'feature']);
    expect(recentBranches(state('', ['main'], 'main'))).toEqual([]);
  });
});

describe('fast-forward', () => {
  const upstream = (overrides: Partial<Upstream> = {}): Upstream => upstreamOf('up/main', { behind: 2, ...overrides });
  const offered = (branch: Partial<Branch>, worktrees: Partial<Worktree>[] = []) => {
    const repo = {
      branches: [branchAt('work', 'aaa', { isCheckedOut: true }), branchAt('feature', 'bbb', branch)],
      worktrees,
    } as Partial<RepoState> as RepoState;
    return canFastForward(repo.branches[1], repo);
  };

  it('is offered for a branch with no commits of its own, even when the last fetch saw nothing new', () => {
    expect(offered({ upstream: upstream() })).toBe(true);
    expect(offered({ upstream: upstream({ behind: 0 }) })).toBe(true);
  });

  it('is not offered for what git would refuse or what Pull already does', () => {
    expect(offered({})).toBe(false);
    expect(offered({ upstream: upstream({ ahead: 1 }) })).toBe(false);
    expect(offered({ upstream: upstream({ isGone: true }) })).toBe(false);
    expect(offered({ isCheckedOut: true, upstream: upstream() })).toBe(false);
  });

  it('is offered for a branch that another worktree has checked out only while nothing is uncommitted there', () => {
    const clean: Partial<Worktree> = { folder: 'C:/work/feature', branch: 'refs/heads/feature', isCurrent: false, isMissing: false, uncommitted: 0 };
    expect(offered({ upstream: upstream() }, [clean])).toBe(true);
    expect(offered({ upstream: upstream() }, [{ ...clean, uncommitted: 2 }])).toBe(false);
    expect(offered({ upstream: upstream() }, [{ ...clean, uncommitted: null }])).toBe(false);
    expect(offered({ upstream: upstream() }, [{ ...clean, isMissing: true }])).toBe(false);
    expect(offered({ upstream: upstream({ ahead: 1 }) }, [clean])).toBe(false);
  });
});

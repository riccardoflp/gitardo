/**
 * What a repository is on as a row of the Repositories view says it (`repo.worktree-summary`): the summary of the
 * repository shown from its own read, and the words of a summary.
 */
import { describe, expect, it } from 'vitest';
import { type WorktreeSummary, summaryOfState, summarySpoken, summaryWords } from '../src/shared/summaries';
import { branchAt, headAt, upstreamOf, workingTreeOf } from './states';

const commit = '3f2a1c9'.padEnd(40, '0');
const changed = (path: string) => ({ path, originalPath: null, kind: 'modified' as const });

describe('the summary of the repository shown, from its own read', () => {
  it('says the branch, each file changed once, and the commits to push and to pull of its upstream', () => {
    const state = {
      head: headAt(commit, 'feature/login'),
      branches: [branchAt('feature/login', commit, { upstream: upstreamOf('origin/feature/login', { ahead: 1, behind: 2 }) })],
      workingTree: workingTreeOf({
        staged: [changed('a.txt')],
        unstaged: [changed('a.txt'), changed('b.txt')],
        untracked: ['new.txt'],
        conflicted: [{ path: 'c.txt', kind: 'bothModified' }],
      }),
    };
    expect(summaryOfState(state)).toEqual({ head: { kind: 'branch', name: 'feature/login' }, changed: 4, ahead: 1, behind: 2 });
  });

  it('says neither ahead nor behind for a branch without an upstream, or whose upstream is gone', () => {
    const state = { head: headAt(commit, 'main'), branches: [branchAt('main', commit)], workingTree: workingTreeOf() };
    expect(summaryOfState(state)).toEqual({ head: { kind: 'branch', name: 'main' }, changed: 0, ahead: null, behind: null });
    const gone = { ...state, branches: [branchAt('main', commit, { upstream: upstreamOf('origin/main', { ahead: 3, isGone: true }) })] };
    expect(summaryOfState(gone).ahead).toBeNull();
  });

  it('says the short commit of a detached HEAD, and that there is no commit yet', () => {
    expect(summaryOfState({ head: headAt(commit), branches: [], workingTree: workingTreeOf() }).head).toEqual({ kind: 'detached', commit: '3f2a1c9' });
    expect(summaryOfState({ head: headAt(null, 'main'), branches: [], workingTree: workingTreeOf() }).head).toEqual({ kind: 'branch', name: 'main' });
    expect(summaryOfState({ head: headAt(null), branches: [], workingTree: workingTreeOf() }).head).toEqual({ kind: 'noCommits' });
  });
});

describe('the words of a summary', () => {
  const on = (summary: Partial<WorktreeSummary>): WorktreeSummary => ({ head: { kind: 'branch', name: 'main' }, changed: 0, ahead: null, behind: null, ...summary });

  it('says the branch alone when nothing is changed and nothing is to pull or to push', () => {
    expect(summaryWords(on({ ahead: 0, behind: 0 }))).toEqual({ head: 'main', changed: '', distance: '' });
    expect(summarySpoken(on({}))).toBe('on main');
  });

  it('says the files changed after a dot, and the commits to pull before those to push', () => {
    expect(summaryWords(on({ changed: 3, ahead: 1, behind: 2 }))).toEqual({ head: 'main', changed: '● 3', distance: '↓2 ↑1' });
    expect(summaryWords(on({ changed: 3, ahead: 0, behind: 2 })).distance).toBe('↓2');
    expect(summaryWords(on({ ahead: 1, behind: 0 })).distance).toBe('↑1');
    expect(summarySpoken(on({ changed: 1, ahead: 1, behind: 2 }))).toBe('on main, 1 file changed, 2 to pull, 1 to push');
  });

  it('says a detached commit, a branch with no commit and a folder that is gone', () => {
    expect(summaryWords(on({ head: { kind: 'detached', commit: '3f2a1c9' } })).head).toBe('detached at 3f2a1c9');
    expect(summaryWords(on({ head: { kind: 'noCommits' } })).head).toBe('no commits yet');
    expect(summaryWords(on({ head: { kind: 'missing' } })).head).toBe('missing');
  });
});

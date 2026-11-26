/**
 * Which repository each group of views shows (D89, `repo.tab-repository`): a session for each repository a group shows,
 * made when the first group comes to it and let go when the last one leaves it, whatever the slashes and the case of
 * its folder.
 */
import { describe, expect, it } from 'vitest';
import { Sessions, keptTab, restoredTab, sameHistory, sharesHistory, tabTitle } from '../src/extension/sessions';
import { worktreeAt } from './states';
import type { RepoInfo } from '../src/shared/protocol';

/** A session as the registry sees it: its folder, and whether it was let go and by which group last. */
interface FakeSession {
  root: string;
  letGo?: string;
}

function registry() {
  const made: FakeSession[] = [];
  const sessions = new Sessions<FakeSession, string>({
    make: (root) => {
      const session = { root };
      made.push(session);
      return session;
    },
    letGo: (session, last) => {
      session.letGo = last;
    },
  });
  return { sessions, made };
}

describe('the sessions of the repositories shown', () => {
  it('make one session for each repository, shared by the groups that show it', () => {
    const { sessions, made } = registry();
    const panel = sessions.move('panel', 'C:\\work\\api');
    const tab = sessions.move('tab', 'c:/work/API');
    expect(panel.made).toBe(true);
    expect(tab.made).toBe(false);
    expect(tab.session).toBe(panel.session);
    expect(made).toHaveLength(1);
    expect(sessions.groupsOn(panel.session)).toEqual(['panel', 'tab']);
    expect(sessions.find('C:/work/api')).toBe(panel.session);
  });

  it('let go of a session once the last group leaves it, and not before', () => {
    const { sessions } = registry();
    const { session: api } = sessions.move('panel', '/work/api');
    sessions.move('tab', '/work/api');
    const moved = sessions.move('panel', '/work/web');
    expect(moved.left).toBe(api);
    expect(api.letGo).toBeUndefined();
    sessions.leave('tab');
    expect(api.letGo).toBe('tab');
    expect(sessions.find('/work/api')).toBeUndefined();
    expect(sessions.all().map((session) => session.root)).toEqual(['/work/web']);
  });

  it('leave a group where it is when it is moved to the repository it shows', () => {
    const { sessions, made } = registry();
    const first = sessions.move('tab', '/work/api');
    const again = sessions.move('tab', '\\work\\API');
    expect(again).toEqual({ session: first.session, made: false });
    expect(made).toHaveLength(1);
  });

  it('make a session again for a repository whose session was let go', () => {
    const { sessions, made } = registry();
    sessions.move('tab', '/work/api');
    sessions.leave('tab');
    const back = sessions.move('tab', '/work/api');
    expect(back.made).toBe(true);
    expect(made).toHaveLength(2);
    expect(made[0].letGo).toBe('tab');
    expect(sessions.sessionOf('tab')).toBe(made[1]);
  });

  it('say which session each group is on, and none for a group that left', () => {
    const { sessions } = registry();
    const { session } = sessions.move('panel', '/work/api');
    sessions.move('tab', '/work/web');
    expect(sessions.sessionOf('panel')).toBe(session);
    sessions.leave('panel');
    expect(sessions.sessionOf('panel')).toBeUndefined();
    sessions.leave('panel');
    expect(sessions.all().map((each) => each.root)).toEqual(['/work/web']);
  });
});

describe('the title of a tab of the graph', () => {
  const repos: RepoInfo[] = [
    { root: '/work/api', name: 'api', depth: 0 },
    { root: '/work/api.worktrees/login', name: 'login', depth: 1 },
    { root: '/work/api/vendor/ui', name: 'vendor/ui', depth: 1 },
    { root: '/work/web', name: 'web', depth: 0 },
  ];
  const api = repos[0];

  it('name the repository it shows', () => {
    expect(tabTitle('/work/web', repos, undefined)).toBe('Gitardo · web');
    expect(tabTitle('\\work\\API', repos, undefined)).toBe('Gitardo · api');
  });

  it('name a worktree or a submodule of the list after the repository it is under', () => {
    expect(tabTitle('/work/api.worktrees/login', repos, undefined)).toBe('Gitardo · api › login');
    expect(tabTitle('/work/api/vendor/ui', repos, undefined)).toBe('Gitardo · api › vendor/ui');
  });

  it('name a worktree or a submodule outside the list after the repository it was opened from', () => {
    expect(tabTitle('/elsewhere/hotfix', repos, api)).toBe('Gitardo · api › hotfix');
    expect(tabTitle('/work/api/vendor/icons/', repos, api)).toBe('Gitardo · api › vendor/icons');
    expect(tabTitle('/elsewhere/hotfix', repos, undefined)).toBe('Gitardo · hotfix');
  });

  it('say only Gitardo while it shows no repository', () => {
    expect(tabTitle(undefined, repos, api)).toBe('Gitardo');
  });
});

describe('what a tab of the graph comes back on after a reload', () => {
  it('take the folder, the filter and the branches shown the page kept', () => {
    const kept = { refsWidth: 200, tab: { root: '/work/web', filter: { kind: 'author', query: 'Ada' }, scope: { kind: 'current' } } };
    expect(keptTab(kept)).toEqual({ root: '/work/web', filter: { kind: 'author', query: 'Ada' }, scope: { kind: 'current' } });
  });

  it('keep every branch shown, and no filter', () => {
    expect(keptTab({ tab: { root: '/work/web', filter: null, scope: null } })).toEqual({ root: '/work/web', scope: null });
  });

  it('take nothing from a page that kept no folder, or not of the right kind', () => {
    expect(keptTab(undefined)).toEqual({});
    expect(keptTab({ refsWidth: 200 })).toEqual({});
    expect(keptTab({ tab: { root: 42 } })).toEqual({});
    expect(keptTab({ tab: { root: '/work/web', filter: 'author', scope: [] } })).toEqual({ root: '/work/web' });
  });
});

describe('which tab of the graph follows the repository chosen after a reload', () => {
  const following = { tab: { root: '/work/web', filter: null, scope: null, follows: true } };
  const keptOn = (root: string, more: object = {}) => ({ tab: { root, filter: null, scope: null, ...more } });

  it('reads whether the page said it followed', () => {
    expect(keptTab(following)).toEqual({ root: '/work/web', scope: null, follows: true });
    expect(keptTab(keptOn('/work/web', { follows: false }))).toEqual({ root: '/work/web', scope: null, follows: false });
    expect(keptTab(keptOn('/work/web', { follows: 'yes' }))).toEqual({ root: '/work/web', scope: null });
  });

  it('repo.keep-tabs: the tab that followed follows again, whatever it showed, and takes nothing of what its page kept', () => {
    expect(restoredTab(following, '/work/api', false)).toEqual({ follows: true });
    expect(restoredTab({ tab: { follows: true } }, undefined, false)).toEqual({ follows: true });
  });

  it('repo.keep-tabs: a tab kept on a repository comes back on it, the repository chosen too', () => {
    const kept = keptOn('/work/api', { follows: false, filter: { kind: 'author', query: 'Ada' } });
    expect(restoredTab(kept, '/work/api', false)).toEqual({ follows: false, root: '/work/api', filter: { kind: 'author', query: 'Ada' }, scope: null });
    expect(restoredTab(keptOn('/work/web', { follows: false }), '/work/api', false)).toEqual({ follows: false, root: '/work/web', scope: null });
  });

  it('repo.keep-tabs: of the tabs kept before one followed, the first on the repository chosen follows and the others keep theirs', () => {
    expect(restoredTab(keptOn('/work/web'), '/work/api', false)).toEqual({ follows: false, root: '/work/web', scope: null });
    expect(restoredTab(keptOn('\\work\\API'), '/work/api', false)).toEqual({ follows: true });
    expect(restoredTab(keptOn('/work/api'), '/work/api', true)).toEqual({ follows: false, root: '/work/api', scope: null });
  });

  it('repo.keep-tabs: a page that kept no folder follows, as it showed the repository of the panel', () => {
    expect(restoredTab({ refsWidth: 200 }, '/work/api', false)).toEqual({ follows: true });
    expect(restoredTab(undefined, undefined, false)).toEqual({ follows: true });
    expect(restoredTab({ refsWidth: 200 }, '/work/api', true)).toEqual({ follows: false });
  });

  it('repo.tab-follows: there is one tab that follows at most, and a second one that says it did keeps what it showed', () => {
    expect(restoredTab(following, '/work/api', true)).toEqual({ follows: false, root: '/work/web', scope: null });
  });
});

describe('a worktree that takes the history its repository has read', () => {
  const shown = { root: '/work/api', worktrees: [worktreeAt('/work/api', 'main', { isMain: true, isCurrent: true }), worktreeAt('/work/api-fix', 'fix'), worktreeAt('/work/gone', 'old', { isMissing: true })] };

  it('shares the commits of the repository shown when it is one of its worktrees, and the repository those of its worktree', () => {
    expect(sharesHistory(shown, '/work/api', '/work/api-fix')).toBe(true);
    expect(sharesHistory(shown, '/work/api', '\\work\\API-fix')).toBe(true);
    const fromWorktree = { root: '/work/api-fix', worktrees: shown.worktrees };
    expect(sharesHistory(fromWorktree, '/work/api-fix', '/work/api')).toBe(true);
  });

  it('shares nothing with a submodule, another repository, a worktree whose folder is gone, or with no state shown of the one left', () => {
    expect(sharesHistory(shown, '/work/api', '/work/api/vendor/lib')).toBe(false);
    expect(sharesHistory(shown, '/work/api', '/work/web')).toBe(false);
    expect(sharesHistory(shown, '/work/api', '/work/gone')).toBe(false);
    expect(sharesHistory(null, '/work/api', '/work/api-fix')).toBe(false);
    // The views still show another repository than the one left: its worktrees are not those of the list.
    expect(sharesHistory(shown, '/work/web', '/work/api-fix')).toBe(false);
    expect(sharesHistory(shown, '/work/api', '/work/api')).toBe(false);
  });

  it('asks for the same history only with nothing searched and the same branches shown, which are not those of what is checked out', () => {
    const main = { kind: 'refs' as const, refs: ['refs/heads/main'] };
    expect(sameHistory({ filter: null, scope: null }, { filter: null, scope: null })).toBe(true);
    expect(sameHistory({ filter: null, scope: main }, { filter: null, scope: { kind: 'refs', refs: ['refs/heads/main'] } })).toBe(true);
    expect(sameHistory({ filter: null, scope: main }, { filter: null, scope: null })).toBe(false);
    expect(sameHistory({ filter: { kind: 'message', query: 'fix' }, scope: null }, { filter: null, scope: null })).toBe(false);
    expect(sameHistory({ filter: null, scope: { kind: 'current' } }, { filter: null, scope: { kind: 'current' } })).toBe(false);
  });
});

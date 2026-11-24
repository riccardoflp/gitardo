/**
 * Which repository each group of views shows (D89, `repo.tab-repository`): a session for each repository a group shows,
 * made when the first group comes to it and let go when the last one leaves it, whatever the slashes and the case of
 * its folder.
 */
import { describe, expect, it } from 'vitest';
import { Sessions, keptTab, tabTitle } from '../src/extension/sessions';
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

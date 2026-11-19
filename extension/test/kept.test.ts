/**
 * What the repositories left keep while another is shown, with `gitardo.repositories.keepOpen` on: in the extension
 * host, the state and the watchers of each and the message of its commit box; in each view, what it showed of each.
 */
import { describe, expect, it } from 'vitest';
import type { KeptRepository } from '../src/extension/keptRepositories';
import { KeptRepositories } from '../src/extension/keptRepositories';
import type { RepoState } from '../src/shared/protocol';
import { KeptViews } from '../src/webview/kept';

/** What a repository keeps, with a watcher that says whether it was stopped. */
function kept(root: string): KeptRepository & { stopped: () => boolean } {
  let stopped = false;
  return {
    state: { root } as RepoState,
    filter: { kind: 'path', path: 'b.txt' },
    scope: null,
    limit: 2000,
    hosting: null,
    pulls: {},
    pullsAt: 0,
    watcher: { dispose: () => (stopped = true) },
    stopped: () => stopped,
  };
}

describe('the repositories kept by the extension', () => {
  it('keep what a repository showed until it is shown again, and then no longer', () => {
    const repositories = new KeptRepositories();
    const api = kept('C:\\work\\api');
    repositories.keep('C:\\work\\api', api);
    expect(repositories.has('c:/work/api')).toBe(true);
    const back = repositories.take('c:/Work/API');
    expect(back?.filter).toEqual({ kind: 'path', path: 'b.txt' });
    expect(back?.limit).toBe(2000);
    expect(repositories.has('C:\\work\\api')).toBe(false);
    expect(api.stopped()).toBe(false);
  });

  it('mark a repository kept as out of date with the last change seen in it, and read nothing for it', () => {
    const repositories = new KeptRepositories();
    repositories.keep('/work/web', kept('/work/web'));
    repositories.keep('/work/api', kept('/work/api'));
    repositories.changed('/work/web', '.git/refs/heads/main changed');
    repositories.changed('/work/web', 'a.txt changed');
    // A repository that is not kept has nothing to mark.
    repositories.changed('/work/docs', 'a.txt changed');
    expect(repositories.take('/work/web')?.changed).toBe('a.txt changed');
    expect(repositories.take('/work/api')?.changed).toBeUndefined();
    expect(repositories.has('/work/docs')).toBe(false);
  });

  it('mark all of them out of date at once, as when the settings change', () => {
    const repositories = new KeptRepositories();
    repositories.keep('/work/web', kept('/work/web'));
    repositories.keep('/work/api', kept('/work/api'));
    repositories.changedAll('the settings changed');
    expect([repositories.take('/work/web')?.changed, repositories.take('/work/api')?.changed]).toEqual(['the settings changed', 'the settings changed']);
  });

  it('keep the message of the commit box of each repository, an empty one too', () => {
    const repositories = new KeptRepositories();
    repositories.keepMessage('/work/api', 'Fix the login');
    repositories.keepMessage('/work/web', '');
    expect(repositories.message('/Work/Api')).toBe('Fix the login');
    expect(repositories.message('/work/web')).toBe('');
    expect(repositories.message('/work/docs')).toBeUndefined();
  });

  it('forget what they kept, and stop watching them, all at once or those asked for', () => {
    const repositories = new KeptRepositories();
    const api = kept('/work/api');
    const web = kept('/work/web');
    repositories.keep('/work/api', api);
    repositories.keep('/work/web', web);
    repositories.keepMessage('/work/web', 'Draft');
    repositories.forget((root) => root.endsWith('web'));
    expect([repositories.has('/work/api'), repositories.has('/work/web'), repositories.message('/work/web')]).toEqual([true, false, undefined]);
    expect([api.stopped(), web.stopped()]).toEqual([false, true]);
    repositories.forget();
    expect(repositories.has('/work/api')).toBe(false);
    expect(api.stopped()).toBe(true);
  });

  it('stop the watchers a repository kept before when it keeps others', () => {
    const repositories = new KeptRepositories();
    const first = kept('/work/api');
    repositories.keep('/work/api', first);
    repositories.keep('/work/api', kept('/work/api'));
    expect(first.stopped()).toBe(true);
  });
});

describe('what a view keeps of the repositories it was pointed away from', () => {
  it('gives each repository back what it showed, by its folder whatever its slashes and its case', () => {
    const views = new KeptViews();
    views.leave('C:\\work\\api', 'graph', { selected: 'abc' });
    views.leave('C:\\work\\api', 'scroll', 480);
    views.leave('C:\\work\\web', 'scroll', 0);
    expect(views.back('c:/work/api', 'graph')).toEqual({ selected: 'abc' });
    expect(views.back('c:/work/api', 'scroll')).toBe(480);
    expect(views.back('C:\\work\\web', 'scroll')).toBe(0);
    expect(views.back('C:\\work\\docs', 'scroll')).toBeUndefined();
  });

  it('forgets all of it at once, as when the setting is turned off', () => {
    const views = new KeptViews();
    views.leave('/work/api', 'scroll', 480);
    views.forget();
    expect(views.back('/work/api', 'scroll')).toBeUndefined();
  });
});

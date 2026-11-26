/**
 * What the extension does with what the engine says of the git folders it watches (`repo.watch-git`, D93): a change the
 * engine saw is one of VS Code's watcher too, read once for the two; a repository kept is only marked out of date; and
 * the engine is told which repositories a view shows, and what it answers changed unseen is read again.
 */
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { GitFolderChangedParams } from '../src/engine/messages';
import { Burst } from '../src/extension/burst';
import { CHANGED_UNSEEN, GitFoldersHeard, gitFolderCause } from '../src/extension/gitFoldersHeard';
import { KeptRepositories, type KeptRepository } from '../src/extension/keptRepositories';
import { Sessions } from '../src/extension/sessions';
import type { RepoState } from '../src/shared/protocol';

/** How long a read waits after the first change of a burst, as `REFRESH_DELAY` of the sessions. */
const DELAY = 400;

/** The session of a repository shown, as far as changes go: one read for a burst of them, with the cause said last. */
class Shown {
  readonly reads: string[] = [];
  private cause = '';
  private readonly burst = new Burst(() => this.reads.push(this.cause));

  constructor(readonly root: string) {}

  scheduleRefresh(cause: string, delay = DELAY): void {
    this.cause = cause;
    this.burst.heard(delay);
  }
}

/** A window with its views, and an engine that notes what it is told and answers what `changed` holds. */
function window() {
  const views = new Map<string, unknown>();
  const groups = new Map<string, string>();
  const sessions = new Sessions<Shown, string>({ make: (root) => new Shown(root), letGo: () => {} });
  const kept = new KeptRepositories();
  const debug: string[] = [];
  const told: string[][] = [];
  const engine = {
    available: true,
    changed: [] as string[],
    say: (_change: GitFolderChangedParams): void => {},
    shown(repositories: string[]) {
      told.push(repositories);
      return Promise.resolve(this.changed.splice(0));
    },
    onGitFolderChanged(handler: (change: GitFolderChangedParams) => void) {
      this.say = handler;
      return { dispose: () => {} };
    },
  };
  const log = { debug: (message: string) => debug.push(message) };
  const heard = new GitFoldersHeard<string, string, Shown>({ log, views, groups, followers: 'panel', sessions, kept }, engine);
  /** Opens a view in a group. */
  const open = (view: string, group: string) => {
    views.set(view, 'graph');
    groups.set(view, group);
  };
  return { views, groups, sessions, kept, debug, told, engine, heard, open };
}

const keeps = (root: string): KeptRepository => {
  return { state: { root } as RepoState, filter: null, scope: null, limit: 1000, hosting: null, pulls: {}, pullsAt: 0 };
};

describe('a change the engine saw in a git folder', () => {
  beforeEach(() => void vi.useFakeTimers());
  afterEach(() => void vi.useRealTimers());

  it('is said in the words of a change the watcher of VS Code saw, and says so when the engine found it by looking again', () => {
    expect(gitFolderCause(['.git/refs/heads/main', '.git/HEAD'], false)).toBe('.git/refs/heads/main changed');
    expect(gitFolderCause(['.git'], false)).toBe('.git changed');
    expect(gitFolderCause(['.git'], true)).toBe("Gitardo's engine found by looking again that .git changed");
  });

  it('asks for one read when the watcher of VS Code and the engine both say it', () => {
    const { sessions, heard } = window();
    const { session } = sessions.move('panel', '/work/app');
    // The watcher of VS Code first, then the engine a moment after, with the batches git's command makes.
    session.scheduleRefresh('.git/refs/heads/main changed');
    vi.advanceTimersByTime(120);
    heard.heard({ repository: '/work/app', paths: ['.git/refs/heads/main', '.git/HEAD'], found: false });
    heard.heard({ repository: '/work/app', paths: ['.git/index'], found: false });
    vi.advanceTimersByTime(DELAY);
    expect(session.reads).toEqual(['.git/index changed']);
    // And the engine first: the read is no later for what the other says after it.
    heard.heard({ repository: '/work/app', paths: ['.git/packed-refs'], found: false });
    vi.advanceTimersByTime(300);
    session.scheduleRefresh('.git/packed-refs changed');
    vi.advanceTimersByTime(100);
    expect(session.reads).toHaveLength(2);
    vi.advanceTimersByTime(1000);
    expect(session.reads).toHaveLength(2);
  });

  it('reaches the session of the repository the engine names, whatever its slashes and its case, and each worktree by its own name', () => {
    const { sessions, heard } = window();
    const app = sessions.move('panel', 'C:\\work\\app').session;
    const feature = sessions.move('tab', 'C:\\work\\app-feature').session;
    // The engine says a change of the git folder they share to each repository opened, with its own name.
    heard.heard({ repository: 'c:/work/app', paths: ['.git/refs/heads/main'], found: false });
    heard.heard({ repository: 'C:\\work\\app-feature', paths: ['.git/refs/heads/main'], found: false });
    vi.advanceTimersByTime(DELAY);
    expect(app.reads).toEqual(['.git/refs/heads/main changed']);
    expect(feature.reads).toEqual(['.git/refs/heads/main changed']);
  });

  it('only marks out of date a repository kept that no view shows, and asks nothing for one neither shown nor kept', () => {
    const { sessions, kept, heard } = window();
    const app = sessions.move('panel', '/work/app').session;
    kept.keep('/work/lib', keeps('/work/lib'));
    heard.heard({ repository: '/work/lib', paths: ['.git/refs/heads/main'], found: false });
    heard.heard({ repository: '/work/other', paths: ['.git/HEAD'], found: false });
    vi.advanceTimersByTime(DELAY);
    expect(kept.outOfDate('/work/lib')).toBe('.git/refs/heads/main changed');
    expect(app.reads).toEqual([]);
  });

  it('says in the log, at the debug level, that the engine found a change nothing had said', () => {
    const { sessions, debug, heard } = window();
    const { session } = sessions.move('panel', '/work/app');
    heard.heard({ repository: '/work/app', paths: ['.git/HEAD'], found: false });
    expect(debug).toEqual([]);
    heard.heard({ repository: '/work/app', paths: ['.git'], found: true });
    expect(debug).toEqual(["Gitardo's engine found a change in the git folder of /work/app that nothing had said: .git."]);
    vi.advanceTimersByTime(DELAY);
    expect(session.reads).toEqual(["Gitardo's engine found by looking again that .git changed"]);
  });
});

describe('the repositories the engine is told a view shows', () => {
  it('are those of the sessions a view is there for, told once for what changes together and again only when they change', async () => {
    const { sessions, told, heard, open, views, groups } = window();
    sessions.move('panel', '/work/app');
    // With no view, nothing is shown: the engine is told nothing.
    await heard.tellShown();
    expect(told).toEqual([]);
    open('graph', 'panel');
    void heard.tellShown();
    sessions.move('tab', '/work/lib');
    open('tab of lib', 'tab');
    await heard.tellShown();
    expect(told).toEqual([['/work/app', '/work/lib']]);
    // A view more of the same repositories changes nothing.
    open('branches', 'panel');
    await heard.tellShown();
    expect(told).toHaveLength(1);
    // The panel pointed elsewhere: the repository left and the one chosen are told together.
    sessions.move('panel', '/work/other');
    await heard.tellShown();
    expect(told[1]).toEqual(['/work/lib', '/work/other']);
    // The tab closed, and then every view of the panel.
    sessions.leave('tab');
    views.delete('tab of lib');
    groups.delete('tab of lib');
    await heard.tellShown();
    expect(told[2]).toEqual(['/work/other']);
    views.clear();
    groups.clear();
    await heard.tellShown();
    expect(told[3]).toEqual([]);
  });

  it('listens to the engine from the first time it tells it, and asks nothing of a window without one', async () => {
    const { sessions, told, engine, heard, open } = window();
    vi.useFakeTimers();
    try {
      engine.available = false;
      const { session } = sessions.move('panel', '/work/app');
      open('graph', 'panel');
      await heard.tellShown();
      expect(told).toEqual([]);
      engine.available = true;
      await heard.tellShown();
      expect(told).toEqual([['/work/app']]);
      engine.say({ repository: '/work/app', paths: ['.git/HEAD'], found: false });
      vi.advanceTimersByTime(DELAY);
      expect(session.reads).toEqual(['.git/HEAD changed']);
    } finally {
      vi.useRealTimers();
    }
  });

  it('reads again at once a repository shown again whose git folder changed with nothing said, and marks one left meanwhile', async () => {
    const { sessions, kept, engine, heard, open } = window();
    vi.useFakeTimers();
    try {
      open('graph', 'panel');
      const lib = sessions.move('panel', '/work/lib').session;
      engine.changed = ['/work/lib'];
      await heard.tellShown();
      vi.advanceTimersByTime(0);
      expect(lib.reads).toEqual([CHANGED_UNSEEN]);
      // Left before the engine answered: what it keeps is out of date.
      let answer: (changed: string[]) => void = () => {};
      engine.shown = () => new Promise((resolve) => (answer = resolve));
      sessions.move('panel', '/work/app');
      const telling = heard.tellShown();
      await vi.advanceTimersByTimeAsync(0);
      sessions.move('panel', '/work/lib');
      kept.keep('/work/app', keeps('/work/app'));
      answer(['/work/app']);
      await telling;
      expect(kept.outOfDate('/work/app')).toBe(CHANGED_UNSEEN);
    } finally {
      vi.useRealTimers();
    }
  });

  it('goes on when the engine cannot be told, and tells it the repositories shown the next time they change', async () => {
    const { sessions, told, debug, engine, heard, open } = window();
    open('graph', 'panel');
    sessions.move('panel', '/work/app');
    const shown = engine.shown.bind(engine);
    engine.shown = () => Promise.reject(new Error("Gitardo's engine is not running."));
    await heard.tellShown();
    expect(debug).toEqual(["Could not tell Gitardo's engine which repositories are shown: Gitardo's engine is not running."]);
    engine.shown = shown;
    sessions.move('panel', '/work/lib');
    await heard.tellShown();
    expect(told).toEqual([['/work/lib']]);
  });
});

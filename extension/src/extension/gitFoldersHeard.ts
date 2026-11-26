/**
 * What the engine says of the git folders it watches itself (`repo.watch-git`, D93): VS Code's watcher of the workspace
 * starts late and drops changes while many files are written, so the engine watches the git folder of each repository
 * opened and says what changed there, and every two seconds looks again at those a view shows, which the extension tells
 * it here. A change it says is taken as one VS Code's watcher said: a repository shown is read again, once for a burst
 * whoever said it, and one kept is only marked out of date.
 */
import type * as vscode from 'vscode';
import type { GitFolderChangedParams } from '../engine/messages';
import { reads } from '../engine/reads';
import type { RepositorySession, ViewGroup } from './repositorySession';

/** What the status bar says a repository shown again is read for, when its git folder changed while no view showed it. */
export const CHANGED_UNSEEN = 'its git folder changed while it was not shown';

/** The session of a repository shown, as far as a change heard of asks of it. */
interface Session {
  readonly root: string;
  scheduleRefresh(cause: string, delay?: number): void;
}

/** What is asked of the window: its views, the groups they are in, and the repositories shown and kept. */
export interface GitFoldersHost<View, Group, Shown extends Session> {
  readonly log: Pick<vscode.LogOutputChannel, 'debug'>;
  /** Every view and tab of Gitardo. */
  readonly views: ReadonlyMap<View, unknown>;
  /** The group each view that shows a repository is in. */
  readonly groups: ReadonlyMap<View, Group>;
  /** The group of the panel and of the side bar, whose views include the tabs of commits and the Repositories view. */
  readonly followers: Group;
  /** The session of each repository a group of views shows. */
  readonly sessions: { all(): Shown[]; find(root: string): Shown | undefined; groupsOn(session: Shown): Group[] };
  /** The repositories left that keep what they showed. */
  readonly kept: { changed(root: string, cause: string): void };
}

/** The engine, as far as the git folders it watches go. */
export interface GitFoldersEngine {
  readonly available: boolean;
  shown(repositories: string[]): Promise<string[]>;
  onGitFolderChanged(handler: (change: GitFolderChangedParams) => void): { dispose(): void };
}

/**
 * What a change the engine saw is said as, in the words of a change VS Code's watcher saw: `.git/refs/heads/main
 * changed`, from the first of its paths. One it found by looking again, nothing having said it, says so.
 */
export function gitFolderCause(paths: readonly string[], found: boolean): string {
  const changed = `${paths[0] ?? '.git'} changed`;
  return found ? `Gitardo's engine found by looking again that ${changed}` : changed;
}

/** Hears the engine's changes of the git folders, and tells it which repositories a view shows. */
export class GitFoldersHeard<View = vscode.Webview, Group = ViewGroup, Shown extends Session = RepositorySession> {
  /** The repositories the engine was last told a view shows. */
  private said: string[] = [];
  /** The telling that waits for the end of the turn, so that a group moved from a repository to another tells once. */
  private telling?: Promise<void>;
  /** The last request, which the next one waits for: the engine takes each list in place of the one before. */
  private asked: Promise<unknown> = Promise.resolve();
  /** The engine's changes are listened to, from the first time it is told anything: it lasts as long as the window. */
  private listening?: { dispose(): void };

  constructor(
    private readonly host: GitFoldersHost<View, Group, Shown>,
    private readonly engine: GitFoldersEngine = reads,
  ) {}

  /** Whether a view is there to show what is read of a session. */
  hasViews(session: Shown): boolean {
    const groups = this.host.sessions.groupsOn(session);
    // The panel's views include the tabs of commits and the Repositories view, as they did when one repository was shown.
    if (groups.includes(this.host.followers)) return this.host.views.size > 0;
    return [...this.host.groups].some(([view, group]) => groups.includes(group) && this.host.views.has(view));
  }

  /**
   * A change the engine saw in the git folder of a repository opened, taken as one of VS Code's watcher: the session of
   * a repository shown asks for a read, in the burst of what the other watcher says of it, and a repository kept is
   * marked out of date. A change found by looking again is said in the log (Q1795).
   */
  heard({ repository, paths, found }: GitFolderChangedParams): void {
    if (found) this.host.log.debug(`Gitardo's engine found a change in the git folder of ${repository} that nothing had said: ${paths.join(', ')}.`);
    this.changedIn(repository, gitFolderCause(paths, found));
  }

  /** A change in a repository: one shown is read again for it, after `delay`, and one kept is only marked out of date. */
  private changedIn(root: string, cause: string, delay?: number): void {
    const session = this.host.sessions.find(root);
    if (session) session.scheduleRefresh(cause, delay);
    else this.host.kept.changed(root, cause);
  }

  /**
   * The repositories a view shows may have changed: a group was pointed elsewhere, a view was opened or closed, a
   * repository was left. When they did, the engine is told them all, once for what changes in one turn, and looks at
   * their git folders again every two seconds from then on. Those it answers were not shown and changed with nothing
   * said of it: what was kept of them is not trusted, and each is read again at once.
   */
  tellShown(): Promise<void> {
    this.telling ??= Promise.resolve().then(() => {
      this.telling = undefined;
      return this.tellNow();
    });
    return this.telling;
  }

  private async tellNow(): Promise<void> {
    if (!this.engine.available) return;
    this.listening ??= this.engine.onGitFolderChanged((change) => this.heard(change));
    const shown = this.host.sessions
      .all()
      .filter((session) => this.hasViews(session))
      .map((session) => session.root);
    if (shown.length === this.said.length && shown.every((root) => this.said.includes(root))) return;
    this.said = shown;
    const asked = this.asked.then(() => this.engine.shown(shown));
    this.asked = asked.catch(() => undefined);
    let changed: string[];
    try {
      changed = await asked;
    } catch (error) {
      // An engine that ended says so itself, and is told again once it is started: the watchers go on meanwhile.
      return this.host.log.debug(`Could not tell Gitardo's engine which repositories are shown: ${error instanceof Error ? error.message : String(error)}`);
    }
    for (const root of changed) this.changedIn(root, CHANGED_UNSEEN, 0);
  }
}

/**
 * The repositories of the workspace read ahead (`repo.watch-hidden`, D92): with `gitardo.repositories.keepOpen` on,
 * once the repository shown has its first read, the others of the list are read one after the other, each kept as if
 * it had been shown and left, so that it is shown at once when it is first chosen. Their worktrees and their
 * submodules are read when they are first shown (Q1742).
 */
import { reads } from '../engine/reads';
import { samePath } from '../shared/paths';
import type { Scope } from '../shared/protocol';
import type { ControllerHost } from './controllerHost';
import { ReadAhead } from './readAhead';
import { SCOPE_KEY } from './repositoriesShown';
import { errorText, keepOpen, pageSize } from './repositorySession';

/** Reads ahead the repositories of the list that no view shows, and says when each may be read. */
export class RepositoriesAhead {
  /** Whose turn it is. */
  readonly turns: ReadAhead;

  constructor(
    private readonly host: Pick<
      ControllerHost,
      'allGroups' | 'completeAhead' | 'followers' | 'kept' | 'locating' | 'log' | 'makeSession' | 'memento' | 'repos' | 'running' | 'sessions'
    >,
  ) {
    this.turns = new ReadAhead({
      waiting: () => this.waiting(),
      isFree: () => this.isFree(),
      read: (root, stop) => this.read(root, stop),
      failed: (root, error) => this.host.log.warn(`Could not read ${root} ahead: ${errorText(error)}`),
    });
  }

  /** The repositories of the list that no group shows and that keep nothing, in its order: not their worktrees, nor their submodules. */
  private waiting(): string[] {
    if (!keepOpen()) return [];
    return this.host.repos.filter((repo) => repo.depth === 0 && !this.host.sessions.find(repo.root) && !this.host.kept.has(repo.root)).map((repo) => repo.root);
  }

  /**
   * Whether a repository may be read ahead now: the setting is on, the repository the panel shows has its first read,
   * which waits for the user when `gitardo.readOnStartup` is off, no operation runs, and no repository shown is being
   * read, its state or the pages of its history.
   */
  private isFree(): boolean {
    if (!keepOpen() || !reads.available || this.host.running || this.host.locating || !this.host.followers.state) return false;
    if (this.host.sessions.all().some((session) => session.loading)) return false;
    return !this.host.allGroups().some((group) => group.session && group.graph.isReading(group.session.root));
  }

  /**
   * Reads a repository no view shows and keeps what was read: its state, with the commits of the history it would be
   * shown with, and its watchers, which from then on mark it out of date. The commits are kept with those of the panel,
   * which shows it without reading them again. Nothing is sent to any view. False when nothing was kept.
   */
  private async read(root: string, stop: AbortSignal): Promise<boolean> {
    const { kept, sessions } = this.host;
    const graph = this.host.followers.graph;
    const session = this.host.makeSession(root);
    // A worktree of a repository shown takes its history once it is chosen (`repo.share-history`): it is not read ahead.
    const shown = (folder: string) => sessions.all().some((each) => each.commonFolder !== undefined && samePath(each.commonFolder, folder));
    let isKept = false;
    kept.readingAhead(root);
    try {
      const scope = this.host.memento.get<Record<string, Scope>>(SCOPE_KEY, {})[root] ?? null;
      const read = await session.readAhead(graph, scope, stop, shown);
      const state = read && !stop.aborted ? await this.host.completeAhead(session, read) : null;
      // Chosen, or the setting turned off, while it was read: what was read is not what it keeps.
      if (!state || stop.aborted || sessions.find(root) || !keepOpen()) return false;
      kept.keep(root, {
        state,
        filter: null,
        scope: state.scope,
        limit: pageSize(),
        hosting: session.hosting,
        pulls: {},
        pullsAt: 0,
        commonFolder: session.commonFolder,
        watcher: session.watcher,
        changed: kept.readAhead(root),
      });
      isKept = true;
      return true;
    } finally {
      kept.readAhead(root);
      session.dispose(isKept);
      // The pages a read stopped still asks for are of no use, unless a group came to show the repository meanwhile.
      if (!isKept && !sessions.find(root)) graph.stop(root);
    }
  }
}

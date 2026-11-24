import { samePath } from '../shared/paths';
import type { Filter, RepoInfo, Scope } from '../shared/protocol';

/**
 * Which repository each group of views shows, and the session of each repository shown (D89, `repo.tab-repository`):
 * one session for each repository a group shows, made when the first group comes to it and let go when the last one
 * leaves it. A group is the panel and the side bar together, or a tab of the graph in the editor. Nothing here reads
 * or watches anything: the sessions do, and this only says which is which.
 */

/** A folder as the sessions are told apart by, whatever its slashes and its case, as `samePath` compares them. */
const key = (root: string) => root.replace(/\\/g, '/').toLowerCase();

/** What becomes of a group put on the session of a repository. */
export interface Moved<Session> {
  session: Session;
  /** The session was made for it: no group showed that repository. */
  made: boolean;
  /** The session it was on before, when it was another one. */
  left?: Session;
}

export class Sessions<Session extends { readonly root: string }, Group> {
  private readonly byRoot = new Map<string, Session>();
  private readonly groups = new Map<Group, Session>();

  constructor(
    private readonly hooks: {
      /** Makes the session of a repository that no group shows. */
      make(root: string): Session;
      /** Lets go of a session no group shows any more; `last` is the group that left it last. */
      letGo(session: Session, last: Group): void;
    },
  ) {}

  /** The session a group is on, if it shows a repository. */
  sessionOf(group: Group): Session | undefined {
    return this.groups.get(group);
  }

  /** The session of a repository, if a group shows it. */
  find(root: string): Session | undefined {
    return this.byRoot.get(key(root));
  }

  /** The groups that show the repository of a session, in the order they came to it. */
  groupsOn(session: Session): Group[] {
    return [...this.groups].filter(([, on]) => on === session).map(([group]) => group);
  }

  /** Every session, in the order they were made. */
  all(): Session[] {
    return [...this.byRoot.values()];
  }

  /**
   * Puts a group on the session of a repository, made when no group shows it; the session it leaves is let go when it
   * was the last group there. A group already there stays where it is.
   */
  move(group: Group, root: string): Moved<Session> {
    const before = this.groups.get(group);
    if (before && samePath(before.root, root)) return { session: before, made: false };
    this.leave(group);
    let session = this.byRoot.get(key(root));
    const made = !session;
    if (!session) {
      session = this.hooks.make(root);
      this.byRoot.set(key(root), session);
    }
    this.groups.set(group, session);
    return { session, made, ...(before ? { left: before } : {}) };
  }

  /** Takes a group off its session, which is let go when it was the last group there. */
  leave(group: Group): void {
    const session = this.groups.get(group);
    if (!session) return;
    this.groups.delete(group);
    if (this.groupsOn(session).length) return;
    this.byRoot.delete(key(session.root));
    this.hooks.letGo(session, group);
  }
}

const forward = (folder: string) => folder.replace(/\\/g, '/').replace(/\/+$/, '');
const lastName = (folder: string) => forward(folder).split('/').pop() || folder;

/** A folder from the repository it is of, as the Repositories view names its rows: its path inside it, else its last name. */
function memberName(repository: string, folder: string): string {
  const [outer, inner] = [forward(repository), forward(folder)];
  return inner.toLowerCase().startsWith(`${outer.toLowerCase()}/`) ? inner.slice(outer.length + 1) : lastName(folder);
}

/**
 * The title of a tab of the graph (`repo.tab-repository`, Q1520): `Gitardo · <repository>`, and after ` › ` the worktree
 * or the submodule it shows, named as the rows of the Repositories view name them. `home` is the repository of the
 * workspace a worktree or a submodule outside the list is of.
 */
export function tabTitle(root: string | undefined, repos: readonly RepoInfo[], home: RepoInfo | null | undefined): string {
  if (!root) return 'Gitardo';
  const at = repos.findIndex((repo) => samePath(repo.root, root));
  if (at >= 0 && repos[at].depth === 0) return `Gitardo · ${repos[at].name}`;
  // A worktree or a submodule the workspace has, under the repository of its group in the list.
  const owner = at >= 0 ? repos.slice(0, at).findLast((repo) => repo.depth === 0) : home && !samePath(home.root, root) ? home : undefined;
  if (!owner) return `Gitardo · ${at >= 0 ? repos[at].name : lastName(root)}`;
  return `Gitardo · ${owner.name} › ${memberName(owner.root, root)}`;
}

/**
 * What a tab of the graph brought back after a reload is to show, from the state its page kept (`repo.keep-tabs`): the
 * folder it showed, with its filter and the branches it showed; nothing of what is not there or not of the right kind,
 * as from a page kept before tabs kept what they show, which opens on the repository the panel shows.
 */
export function keptTab(kept: unknown): { root?: string; filter?: Filter; scope?: Scope | null } {
  const tab = typeof kept === 'object' && kept !== null ? (kept as { tab?: unknown }).tab : undefined;
  if (typeof tab !== 'object' || tab === null) return {};
  const { root, filter, scope } = tab as { root?: unknown; filter?: unknown; scope?: unknown };
  if (typeof root !== 'string' || !root) return {};
  const kind = (value: unknown) => (typeof value === 'object' && value !== null ? (value as { kind?: unknown }).kind : undefined);
  return {
    root,
    ...(typeof kind(filter) === 'string' ? { filter: filter as Filter } : {}),
    ...(scope === null || typeof kind(scope) === 'string' ? { scope: scope as Scope | null } : {}),
  };
}

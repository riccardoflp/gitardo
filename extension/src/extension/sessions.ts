import { samePath } from '../shared/paths';
import type { Filter, RepoInfo, RepoState, Scope } from '../shared/protocol';

/**
 * Which repository each group of views shows, and the session of each repository shown (D89, `repo.tab-repository`):
 * one session for each repository a group shows, made when the first group comes to it and let go when the last one
 * leaves it. A group is the panel, the side bar and the tab of the graph that follows them together, or a tab of the
 * graph kept on a repository (D94). Nothing here reads or watches anything: the sessions do, and this only says which
 * is which.
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

/**
 * Whether a folder shares its commits and its references with the repository a group shows: a worktree of it, or the
 * repository itself from one of its worktrees, as the list of the worktrees of the state shown says. A submodule is
 * another repository, and shares nothing (`repo.share-history`).
 */
export function sharesHistory(shown: Pick<RepoState, 'root' | 'worktrees'> | null, left: string, chosen: string): boolean {
  if (!shown || !samePath(shown.root, left) || samePath(left, chosen)) return false;
  return shown.worktrees.some((worktree) => !worktree.isMissing && samePath(worktree.folder, chosen));
}

/** What narrows the history a group lists: its search and the branches it shows. */
type Narrowed = { filter: Filter | null; scope: Scope | null };

/**
 * Whether a worktree chosen asks for the history the worktree left listed: nothing searched in either, and the same
 * branches shown, which are not those of what is checked out, each worktree having its own.
 */
export function sameHistory(left: Narrowed, chosen: Narrowed): boolean {
  if (left.filter || chosen.filter) return false;
  if (left.scope?.kind === 'current' || chosen.scope?.kind === 'current') return false;
  return JSON.stringify(left.scope) === JSON.stringify(chosen.scope);
}

const forward = (folder: string) => folder.replace(/\\/g, '/').replace(/\/+$/, '');
const lastName = (folder: string) => forward(folder).split('/').pop() || folder;

/** A folder from the repository it is of, as the Repositories view names its rows: its path inside it, else its last name. */
function memberName(repository: string, folder: string): string {
  const [outer, inner] = [forward(repository), forward(folder)];
  return inner.toLowerCase().startsWith(`${outer.toLowerCase()}/`) ? inner.slice(outer.length + 1) : lastName(folder);
}

/**
 * The title of a tab of the graph kept on a repository (`repo.tab-repository`, Q1520): `Gitardo · <repository>`, and after ` › ` the worktree
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
 * What the page of a tab of the graph kept, read from its state (`repo.keep-tabs`): the folder it showed, with its filter
 * and the branches it showed, and whether it followed the repository chosen; nothing of what is not there or not of the
 * right kind, as from a page kept before tabs kept what they show. A page kept before one tab followed says nothing of
 * following.
 */
export function keptTab(kept: unknown): { root?: string; filter?: Filter; scope?: Scope | null; follows?: boolean } {
  const tab = typeof kept === 'object' && kept !== null ? (kept as { tab?: unknown }).tab : undefined;
  if (typeof tab !== 'object' || tab === null) return {};
  const { root, filter, scope, follows } = tab as { root?: unknown; filter?: unknown; scope?: unknown; follows?: unknown };
  const following = typeof follows === 'boolean' ? { follows } : {};
  if (typeof root !== 'string' || !root) return following;
  const kind = (value: unknown) => (typeof value === 'object' && value !== null ? (value as { kind?: unknown }).kind : undefined);
  return {
    root,
    ...(typeof kind(filter) === 'string' ? { filter: filter as Filter } : {}),
    ...(scope === null || typeof kind(scope) === 'string' ? { scope: scope as Scope | null } : {}),
    ...following,
  };
}

/** A tab of the graph brought back: the one that follows the repository chosen, or one kept on a folder with what it showed. */
export interface RestoredTab {
  follows: boolean;
  root?: string;
  filter?: Filter;
  scope?: Scope | null;
}

/**
 * What a tab of the graph brought back after a reload becomes, from the state its page kept (`repo.keep-tabs`, D94).
 * The one that followed the repository chosen follows again, and the others come back on what they showed. A page kept
 * before one tab followed says neither: the first such tab on the repository chosen, or that kept no folder, becomes
 * the one that follows, and the others keep theirs. `following` is whether a tab follows already, there being one at
 * most: a tab brought back after it keeps the repository it showed, or the one chosen when it kept none.
 */
export function restoredTab(kept: unknown, chosen: string | undefined, following: boolean): RestoredTab {
  const { follows, ...tab } = keptTab(kept);
  const onChosen = !tab.root || (!!chosen && samePath(tab.root, chosen));
  if (!following && (follows ?? onChosen)) return { follows: true };
  return { follows: false, ...tab };
}

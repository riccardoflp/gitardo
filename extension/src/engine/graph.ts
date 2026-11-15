/**
 * The graph read by Gitardo's engine, from stage 7 of the transition: the rows of the timeline, the position of a
 * commit and the matches of a search, asked with the timeline the graph shows (`read/graph.read-commits`,
 * `read/graph.read-position`, `read/search.read-matches`). The lines of the graph are laid out here from the parents of
 * the rows; the next pages are asked with `skip`, so that only the rows the views do not have are read.
 */
import { createHash } from 'node:crypto';
import * as path from 'node:path';
import { changedPaths } from '../shared/changes';
import { GraphLayout } from '../shared/graph';
import { GRAPH_COLUMNS, WORKTREE, type Commit, type Filter, type RepoParts, type RepoState, type Row, type Scope, type Search } from '../shared/protocol';
import { pruneScope } from '../shared/scope';
import type { EngineClient } from './client';
import type { BranchesShown, CommitPage, EngineCommit, EngineRow, Narrowing, SearchMode, Timeline } from './messages';

/** A commit of the graph, or a stash among them with its name. */
export type Listed = Commit & { stash?: string };

/**
 * What the graph shows, as its commits are asked for. `scope` is the one the references have now; `names` are those
 * of the file of a `path` filter, its name now first; `stashes` says whether they are placed among the commits, never
 * with a filter. `head`, `refs` and `stashList` are the state just read, which tells whether the history may have
 * changed since.
 */
export interface GraphAsked {
  limit: number;
  filter: Filter | null;
  scope: Scope | null;
  names: string[];
  stashes: boolean;
  firstParent: boolean;
  head: string | null;
  refs: unknown;
  stashList: unknown;
}

/** What the graph is read for: how many commits, narrowed how, of which branches, with the stashes or not. */
export interface StateAsked {
  limit: number;
  filter: Filter | null;
  scope: Scope | null;
  stashes: boolean;
  /** Whether only the first parent of each merge is followed: what was merged is left out. */
  firstParent: boolean;
}

/** Reads the commits of the graph, and whether there are more. */
export type CommitsReader = (asked: GraphAsked) => Promise<GraphCommits>;

/**
 * The state of a repository from what `repo.read-state` read, with the commits of the graph `read` gives, the row of the
 * uncommitted changes above them, and the lines of the graph laid out. The parts that are the window's own, such as
 * the favorites, are left at their defaults for the caller to fill.
 */
export async function graphState(client: EngineClient, root: string, parts: RepoParts, asked: StateAsked, read: CommitsReader, signal?: AbortSignal): Promise<RepoState> {
  const { limit, filter, stashes, firstParent } = asked;
  const head = parts.head.commit;
  const changes = changedPaths(parts.workingTree);
  // A branch chosen for the graph may have been deleted since.
  const shown = pruneScope(asked.scope, parts);
  // A file is followed under the names it had before, or its history would end at the commit that renamed it. A folder
  // is not: git follows only files.
  const names = filter?.kind === 'path' ? (filter.folder ? [filter.path] : await client.fileNames(root, filter.path, signal)) : [];
  const refs = { branches: parts.branches, remoteBranches: parts.remoteBranches, tags: parts.tags };
  const commits = await read({ limit, filter, scope: shown, names, stashes: stashes && !filter, firstParent, head, refs, stashList: parts.stashes });
  const listed = commits.commits;

  const layout = new GraphLayout();
  const rows: Row[] = [];
  // Search results are unrelated commits: drawing lines between them would suggest a history that is not there.
  const connected = !filter || filter.kind === 'path' || filter.kind === 'lines' || filter.kind === 'ref';
  // Nor are the changes shown above a graph that leaves out the commit they are made on.
  const headListed = !shown || !head || listed.some((commit) => commit.stash === undefined && commit.hash === head);
  if (changes.length && !filter && headListed) {
    // Before the first commit there is no HEAD to hang the changes on, and they are all there is to show.
    const parents = head ? [head] : [];
    const subject = `Uncommitted changes (${changes.length})`;
    rows.push({ hash: WORKTREE, parents, author: '', email: '', date: 0, subject, graph: layout.next(WORKTREE, parents, true) });
  }
  for (const commit of listed) rows.push({ ...commit, graph: layout.next(commit.hash, connected ? commit.parents : [], commit.stash !== undefined) });

  return {
    root,
    name: path.basename(root),
    ...parts,
    rows,
    hasMore: commits.hasMore,
    changes: changes.length,
    filter: filter?.kind === 'path' && names.length > 1 ? { ...filter, renamedFrom: names.slice(1) } : filter,
    scope: shown,
    favorites: [],
    defaultPull: 'pullRebaseAutostash',
    repos: [],
    github: null,
    pulls: {},
    agentsWindow: false,
    undo: null,
    worktreesRoot: null,
    openBranches: 'checkout',
    showStashes: stashes,
    firstParent,
    graphColumns: GRAPH_COLUMNS,
    dateSeparators: true,
    combineUpstream: true,
    traceSelected: true,
    branchesFollowScope: true,
    sortOrder: 'name',
    resolveWithAI: false,
    home: null,
  };
}

/** The commits read, and whether the history has more. */
export interface GraphCommits {
  commits: Listed[];
  hasMore: boolean;
}

/** Which history a timeline lists, without how much of it. */
type History = Pick<Timeline, 'shown' | 'parents' | 'alone' | 'narrowing'>;

/** How a search of the extension looks at the commits, as the engine names it. */
const SEARCH_MODES: Record<Search['kind'], SearchMode> = {
  message: 'message',
  author: 'author',
  pickaxe: 'changes',
  regex: 'changesPattern',
};

/** The branches the graph shows, as the engine names them. With nothing checked out, what is checked out is everything. */
export function shownOf(scope: Scope | null, head: string | null): BranchesShown | null {
  if (!scope) return null;
  switch (scope.kind) {
    case 'current':
      if (!head) return null;
      return scope.upstream ? { kind: 'upstream' } : { kind: 'current' };
    case 'kinds':
      return { kind: 'kinds', local: scope.kinds.includes('branch'), remote: scope.kinds.includes('remote'), tags: scope.kinds.includes('tag') };
    case 'refs':
      return { kind: 'references', names: scope.refs };
  }
}

/** A commit of the engine as the graph lists it. */
function commitOf(commit: EngineCommit): Commit {
  return { hash: commit.id, parents: commit.parents, author: commit.authorName, email: commit.authorEmail, date: commit.authorDate, subject: commit.subject };
}

/** The rows of a page as the graph lists them: the commits, and the stashes by their names. */
export function listedOf(rows: EngineRow[]): Listed[] {
  return rows.flatMap((row): Listed[] => {
    if (row.kind === 'commit') return [commitOf(row.commit)];
    if (row.kind === 'stash') return [{ ...commitOf(row.commit), stash: `stash@{${row.index}}` }];
    // The graph adds the row of the uncommitted changes itself.
    return [];
  });
}

/**
 * The history the graph lists, as the engine is asked for it: the branches shown, the first parents, and what narrows
 * it. `names` are the names of the file of a `path` filter, its name now first, read when they are not given. A
 * branch, tag or commit of a `ref` filter is listed alone by the commit it stands for.
 */
export async function historyOf(
  client: EngineClient,
  repository: string,
  filter: Filter | null,
  scope: Scope | null,
  head: string | null,
  firstParent: boolean,
  names?: string[],
  signal?: AbortSignal,
): Promise<History> {
  const history: History = { shown: shownOf(scope, head), parents: firstParent ? 'first' : 'all', alone: null, narrowing: null };
  if (!filter) return history;
  switch (filter.kind) {
    case 'path': {
      if (filter.folder) return { ...history, narrowing: { kind: 'folder', path: filter.path } };
      const every = names ?? (await client.fileNames(repository, filter.path, signal));
      return { ...history, narrowing: { kind: 'file', path: filter.path, earlier: every.filter((name) => name !== filter.path) } };
    }
    case 'lines': {
      const { path, start, end, commit, editor } = filter;
      const narrowing: Narrowing = { kind: 'lines', path, lines: { start, end }, commit, inEditor: editor ?? null };
      return { ...history, narrowing };
    }
    case 'ref': {
      const commit = await client.searchRevision(repository, filter.ref, signal);
      if (!commit) throw new Error(`"${filter.ref}" is not a commit of this repository.`);
      return { ...history, alone: commit };
    }
    default:
      return { ...history, narrowing: { kind: 'search', mode: SEARCH_MODES[filter.kind], text: filter.query } };
  }
}

/** The rows of a page cut after the commit `limit`, with the stashes whose commit is cut too taken out. */
function cut(read: GraphCommits, limit: number): GraphCommits {
  let count = 0;
  const end = read.commits.findIndex((commit) => commit.stash === undefined && ++count > limit);
  if (end < 0) return read;
  const kept = read.commits.slice(0, end);
  const listed = new Set(kept.flatMap((commit) => (commit.stash === undefined ? [commit.hash] : [])));
  return { commits: kept.filter((commit) => commit.stash === undefined || listed.has(commit.parents[0])), hasMore: true };
}

/** The commits read last in each repository, and what they were read for, to ask the next pages from the last of them. */
interface Read extends GraphCommits {
  key: string;
}

/**
 * Reads the timeline in the engine. The rows read last are kept for each repository: when the graph asks for more of
 * the same history, of the same references, only the rows after the last of them are read (`skip`). Anything that
 * makes those rows not what a read of every page would give, such as a stash that would go above them, is read whole.
 */
export class EngineGraph {
  private readonly last = new Map<string, Read>();

  /** The commits of the graph, `limit` at most, read in pages of `pageSize`. */
  async commits(client: EngineClient, repository: string, asked: GraphAsked, pageSize: number, signal?: AbortSignal): Promise<GraphCommits> {
    const history = await historyOf(client, repository, asked.filter, asked.scope, asked.head, asked.firstParent, asked.names, signal);
    const timeline: Timeline = { ...history, pageSize, pages: Math.max(1, Math.ceil(asked.limit / pageSize)), skip: 0, stashes: asked.stashes };
    const key = createHash('sha1')
      .update(JSON.stringify([history, asked.stashes, asked.head, asked.refs, asked.stashList]))
      .digest('hex');
    const last = this.last.get(repository);
    const known = last?.key === key ? last : undefined;
    const wantsMore = known && known.commits.filter((commit) => commit.stash === undefined).length < asked.limit;
    const more = wantsMore ? await this.after(client, repository, timeline, known, signal) : undefined;
    const read = cut(more ?? this.page(await client.graphCommits(repository, timeline, signal)), asked.limit);
    this.last.set(repository, { ...read, key });
    return read;
  }

  /** Forgets the rows read in a repository, or in every one. */
  forget(repository?: string): void {
    if (repository === undefined) this.last.clear();
    else this.last.delete(repository);
  }

  /**
   * The rows read before with those after the last of their commits, read from it; undefined when the history is not
   * the same up to it, or when a stash of the new rows would go above it.
   */
  private async after(client: EngineClient, repository: string, timeline: Timeline, known: Read, signal?: AbortSignal): Promise<GraphCommits | undefined> {
    const commits = known.commits.filter((commit) => commit.stash === undefined);
    const lastKnown = commits.at(-1);
    if (!lastKnown) return undefined;
    const page = this.page(await client.graphCommits(repository, { ...timeline, skip: commits.length - 1 }, signal));
    const first = page.commits.findIndex((commit) => commit.stash === undefined);
    // The read starts from the last commit known: anything above it, or another commit there, is another history.
    if (first !== 0 || page.commits[0].hash !== lastKnown.hash) return undefined;
    const rows = page.commits.slice(1);
    // A stash goes where its date puts it among all the commits: above the rows known when one of them is not newer.
    const earlier = commits.slice(0, -1);
    const placedAbove = rows.some((row) => row.stash !== undefined && earlier.some((commit) => commit.date <= row.date));
    if (placedAbove) return undefined;
    return { commits: [...known.commits, ...rows], hasMore: page.hasMore };
  }

  /** A page of the engine as the graph lists it. */
  private page(page: CommitPage): GraphCommits {
    return { commits: listedOf(page.rows), hasMore: page.hasMore };
  }
}

/** Where a commit is in the history the graph lists: how many commits come before it, or -1 when it is not there. */
export async function positionOnEngine(
  client: EngineClient,
  repository: string,
  commit: string,
  history: History,
  signal?: AbortSignal,
): Promise<number> {
  const index = await client.graphPosition(repository, commit, { ...history, pageSize: 1000, pages: 1, skip: 0, stashes: false }, signal);
  return index ?? -1;
}

/** The commits of the history the graph lists that a search finds, newest first, and whether there are more. */
export async function matchesOnEngine(
  client: EngineClient,
  repository: string,
  search: Search,
  history: History,
  signal?: AbortSignal,
): Promise<{ hashes: string[]; more: boolean }> {
  const timeline: Timeline = { ...history, pageSize: 1000, pages: 1, skip: 0, stashes: false };
  const found = await client.searchMatches(repository, search.query, SEARCH_MODES[search.kind], timeline, signal);
  return { hashes: found.commits, more: found.hasMore };
}

/**
 * The graph read by Gitardo's engine, from stage 7 of the transition: the rows of the timeline, the position of a
 * commit and the matches of a search, asked with the timeline the graph shows (`read/graph.read-commits`,
 * `read/graph.read-position`, `read/search.read-matches`). From version 1.8 of the protocol the commits of the timeline
 * come from one read that sends its pages as git prints them, to the end of the history or to 100,000 commits (D54),
 * and the lines of the graph are laid out here from the parents of the rows, each page from the last row before it.
 */
import { type Hash, createHash } from 'node:crypto';
import * as path from 'node:path';
import { changedPaths } from '../shared/changes';
import { GraphLayout } from '../shared/graph';
import { EngineError, ErrorCode } from './connection';
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
 * the favorites, are left at their defaults for the caller to fill. `previous` is the layout of the rows shown: when
 * the commits are the same read, laid out the same way, it is kept, and only the rows read since are laid out, under
 * its last row (`graph.lanes`).
 */
export async function graphState(
  client: EngineClient,
  root: string,
  parts: RepoParts,
  asked: StateAsked,
  read: CommitsReader,
  signal?: AbortSignal,
  previous?: LaidOutRows,
): Promise<RepoState> {
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

  // Search results are unrelated commits: drawing lines between them would suggest a history that is not there.
  const connected = !filter || filter.kind === 'path' || filter.kind === 'lines' || filter.kind === 'ref';
  // Nor are the changes shown above a graph that leaves out the commit they are made on.
  const headListed = !shown || !head || listed.some((commit) => commit.stash === undefined && commit.hash === head);
  let worktree: Listed | null = null;
  if (changes.length && !filter && headListed) {
    // Before the first commit there is no HEAD to hang the changes on, and they are all there is to show.
    const parents = head ? [head] : [];
    worktree = { hash: WORKTREE, parents, author: '', email: '', date: 0, subject: `Uncommitted changes (${changes.length})` };
  }
  const prefix = `${root}\0${JSON.stringify(filter)}\0${firstParent}\0${connected}\0`;
  const laid = previous?.fits(commits, prefix, worktree) ? previous : new LaidOutRows(commits, prefix, connected, worktree);
  const rows = laid.catchUp();

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
    hosting: null,
    pulls: {},
    agentsWindow: false,
    undo: null,
    worktreesRoot: null,
    openBranches: 'checkout',
    showStashes: stashes,
    firstParent,
    oneLane: false,
    graphColumns: GRAPH_COLUMNS,
    dateSeparators: true,
    combineUpstream: true,
    islands: false,
    traceSelected: true,
    listed: null,
    sortOrder: 'name',
    resolveWithAI: false,
    home: null,
    keepOpen: false,
    signOff: false,
    tagMarks: null,
  };
}

/**
 * A state read, as the views are given it: with what else they show of the repository, read by `rest`, and with the
 * pages its read of the commits added while that was read laid out under its rows. The pages are looked at after the
 * rest, in the same turn the state is given: a read that ends while the rest is read tells nobody after, and its last
 * commits would wait for the list to be scrolled to its end, under a row that says more are loading.
 */
export async function completed<Rest extends Partial<RepoState>>(state: RepoState, rest: () => Promise<Rest>): Promise<RepoState> {
  const read = await rest();
  const laid = laidOutOf(state.rows);
  return laid ? { ...state, ...read, rows: laid.catchUp(), hasMore: laid.source.hasMore } : { ...state, ...read };
}

/**
 * The commits read, and whether the history has more, read or still to read. A read that goes on adds the commits of
 * each page to `commits` as it comes.
 */
export interface GraphCommits {
  readonly commits: Listed[];
  readonly hasMore: boolean;
  /**
   * How many times commits listed were taken out from the middle: a read of the same history that found other commits
   * from some row on. The rows laid out from those commits are laid out again.
   */
  readonly cuts?: number;
}

/** The layout of each array of rows given to the views, to lay out the rows of the next pages from its last row. */
const layouts = new WeakMap<Row[], LaidOutRows>();

/** The layout rows were laid out with by `graphState`, which lays out the next pages of the same read under them. */
export function laidOutOf(rows: Row[]): LaidOutRows | undefined {
  return layouts.get(rows);
}

/**
 * The rows of a read of the commits laid out, the row of the uncommitted changes first when there is one, with the
 * lanes as they stand under the last of them (`graph.lanes`, D54): each page the read adds is laid out alone from
 * there, which gives the cells a layout of every row gives. It also says what identifies the first rows given, so that
 * the views are sent only the rows they do not have.
 */
export class LaidOutRows {
  private layout = new GraphLayout();
  /** How many of the commits read are laid out. */
  private laid = 0;
  /** How many cuts of the commits read the rows are laid out after. */
  private cuts: number;
  private hash: Hash;
  /** What identifies the first rows, by how many they are. */
  private readonly keys = new Map<number, string>();
  /** The rows laid out until now: a new array each time rows are added. */
  rows: Row[] = [];

  constructor(
    /** The read whose commits are laid out. */
    readonly source: GraphCommits,
    /** The repository, the filter and what else changes the rows of the same commits. */
    private readonly prefix: string,
    /** Whether lines are drawn between the commits: not between those a search found. */
    private readonly connected: boolean,
    worktree: Listed | null,
  ) {
    this.cuts = source.cuts ?? 0;
    this.hash = createHash('sha1').update(prefix);
    if (worktree) this.add([{ ...worktree, graph: this.layout.next(WORKTREE, worktree.parents, true) }]);
  }

  /**
   * Whether these rows can stand for a read: the same read, laid out the same way, with the row of the uncommitted
   * changes above them as before, on the same commit. Its subject, which counts the files, is taken from the one given.
   */
  fits(source: GraphCommits, prefix: string, worktree: Listed | null): boolean {
    const top = this.rows[0]?.hash === WORKTREE ? this.rows[0] : null;
    if (source !== this.source || prefix !== this.prefix || !top !== !worktree) return false;
    if (top && worktree && top.parents[0] !== worktree.parents[0]) return false;
    if (top && worktree && top.subject !== worktree.subject) {
      this.rows = [{ ...top, subject: worktree.subject }, ...this.rows.slice(1)];
      layouts.set(this.rows, this);
    }
    return true;
  }

  /** The rows, with the commits read since the last call laid out under them. */
  catchUp(): Row[] {
    if ((this.source.cuts ?? 0) !== this.cuts) this.restart();
    const listed = this.source.commits;
    if (this.laid >= listed.length && this.rows.length) return this.rows;
    const added = listed.slice(this.laid);
    this.laid = listed.length;
    const cells = this.layout.page(added.map((commit) => ({ hash: commit.hash, parents: this.connected ? commit.parents : [], dashed: commit.stash !== undefined })));
    this.add(added.map((commit, index) => ({ ...commit, graph: cells[index] })));
    return this.rows;
  }

  /** What identifies the first `count` rows, as the views were last sent them. */
  keyAt(count: number): string {
    let key = this.keys.get(count);
    if (key === undefined) {
      const hash = createHash('sha1').update(this.prefix);
      for (const row of this.rows.slice(0, count)) hash.update(row.hash);
      key = hash.digest('hex');
      this.keys.set(count, key);
    }
    return key;
  }

  /** Lays the rows out again from the first, as the commits read are now: some were taken out from the middle. */
  private restart(): void {
    const top = this.rows[0]?.hash === WORKTREE ? this.rows[0] : null;
    this.cuts = this.source.cuts ?? 0;
    this.layout = new GraphLayout();
    this.laid = 0;
    this.hash = createHash('sha1').update(this.prefix);
    this.keys.clear();
    this.rows = [];
    if (top) this.add([{ ...top, graph: this.layout.next(WORKTREE, top.parents, true) }]);
  }

  private add(rows: Row[]): void {
    for (const row of rows) this.hash.update(row.hash);
    this.rows = this.rows.length ? this.rows.concat(rows) : rows;
    this.keys.set(this.rows.length, this.hash.copy().digest('hex'));
    layouts.set(this.rows, this);
  }
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

/**
 * The first `limit` commits of the graph, read at once in pages of `pageSize` and given together: for what needs a
 * few commits once, such as the subjects of the commits before a draft or a repository the views do not show.
 */
export async function readCommits(client: EngineClient, repository: string, asked: GraphAsked, pageSize: number, signal?: AbortSignal): Promise<GraphCommits> {
  const history = await historyOf(client, repository, asked.filter, asked.scope, asked.head, asked.firstParent, asked.names, signal);
  const timeline: Timeline = { ...history, pageSize, pages: Math.max(1, Math.ceil(asked.limit / pageSize)), skip: 0, stashes: asked.stashes };
  const page = await client.graphCommits(repository, timeline, signal);
  return cut({ commits: listedOf(page.rows), hasMore: page.hasMore }, asked.limit);
}

/** The error of a read of the commits stopped because another took its place, or because it was no longer wanted. */
function stopped(): EngineError {
  return new EngineError(ErrorCode.cancelled, 'The read was cancelled.');
}

/** Whether two rows read are the same: the same commit, the same stash. */
function same(one: Listed, other: Listed): boolean {
  return one.hash === other.hash && one.stash === other.stash;
}

/**
 * One read of the commits of a history as git prints them, and the reads past its 100,000 commits (D54). Read again,
 * after a commit, a fetch or **Refresh**, it keeps the commits it has while the pages of the new read are the same, as
 * the timeline of Gitardo Desktop does (Q781): from the first that is not, the commits listed are cut there and those
 * read follow.
 */
class StreamedRead implements GraphCommits {
  readonly commits: Listed[] = [];
  /** How many of them are commits, not stashes: where the read past them starts. */
  count = 0;
  /** How many times commits were cut from the middle of those listed. */
  cuts = 0;
  /** While a read of the same history goes over the commits listed, the row it compares next; null once it reads past them. */
  private checking: number | null = null;
  /** How many commits, not stashes, the read that goes over the commits listed found the same. */
  private checked = 0;
  /** Whether the history has more commits than those read, once the reads end. */
  more = false;
  /** Whether a read still adds pages. */
  reading = true;
  /** Whether it was stopped or failed: it gives nothing more, and is read again when asked. */
  over = false;
  /** Whether it is read again when asked, as **Refresh** wants, even for the same references and stashes. */
  stale = false;
  /** Stops the read under way. */
  stop = new AbortController();
  /** Who waits for some commits. */
  private waiting: { count: number; resolve: () => void; reject: (error: unknown) => void }[] = [];

  constructor(
    /** The history read, of which references and stashes. */
    public key: string,
    /** The history read, whatever the references and the stashes. */
    readonly history: string,
  ) {}

  get hasMore(): boolean {
    return this.reading || this.more;
  }

  /** How many commits the read under way has read: those it found the same among the commits listed, then all. */
  get reached(): number {
    return this.checking === null ? this.count : this.checked;
  }

  /**
   * The same history is read again, of other references or stashes, or on **Refresh**: the read under way is stopped,
   * who waits for it is told so, and the read that starts goes over the commits listed before it adds any.
   */
  again(key: string): void {
    this.stop.abort();
    for (const waiter of this.waiting.splice(0)) waiter.reject(stopped());
    this.stop = new AbortController();
    this.key = key;
    this.reading = true;
    this.over = false;
    this.stale = false;
    this.more = false;
    this.checking = 0;
    this.checked = 0;
  }

  /** Adds the commits of a page, past those that are the same as the commits listed while the read goes over them. */
  add(listed: Listed[]): void {
    let from = 0;
    while (this.checking !== null && from < listed.length) {
      if (this.checking < this.commits.length && same(this.commits[this.checking], listed[from])) {
        if (listed[from].stash === undefined) this.checked++;
        this.checking++;
        from++;
      } else this.cut();
    }
    for (const commit of listed.slice(from)) {
      this.commits.push(commit);
      if (commit.stash === undefined) this.count++;
    }
    this.settle();
  }

  /** The read under way ended, the history having more commits or not. */
  end(more: boolean): void {
    // A history that has fewer commits than were listed: those it no longer has are taken out.
    if (this.checking !== null && this.checking < this.commits.length) this.cut();
    this.checking = null;
    this.reading = false;
    this.more = more;
    this.settle();
  }

  /** Takes out the commits listed from the row where the read going over them found another commit. */
  private cut(): void {
    this.commits.length = this.checking ?? this.commits.length;
    this.count = this.checked;
    this.checking = null;
    this.cuts++;
  }

  /** The read under way failed or was stopped: who waits is given the error. */
  fail(error: unknown): void {
    this.reading = false;
    this.over = true;
    for (const waiter of this.waiting.splice(0)) waiter.reject(error);
  }

  /** Stops the read under way: no page of it is added after. */
  cancel(): void {
    if (this.over) return;
    this.stop.abort();
    this.fail(stopped());
  }

  /** Waits until `count` commits are read, or the read ends. When `signal` aborts first, the read is stopped. */
  until(count: number, signal?: AbortSignal): Promise<void> {
    if (this.over) return Promise.reject(stopped());
    if (this.reached >= count || !this.reading) return Promise.resolve();
    return new Promise<void>((resolve, reject) => {
      const abort = () => this.cancel();
      // Once its commits are there, the signal of who waited no longer stops the read: its next pages are wanted too.
      const done = () => signal?.removeEventListener('abort', abort);
      this.waiting.push({ count, resolve: () => (done(), resolve()), reject: (error) => (done(), reject(error)) });
      if (signal?.aborted) abort();
      else signal?.addEventListener('abort', abort, { once: true });
    });
  }

  private settle(): void {
    const ready = this.waiting.filter((waiter) => this.reached >= waiter.count || !this.reading);
    this.waiting = this.waiting.filter((waiter) => !ready.includes(waiter));
    for (const waiter of ready) waiter.resolve();
  }
}

/**
 * Reads the timeline in the engine, one read for each history, as git prints it (D54): the first page is given as soon
 * as it is read, and the commits of the next ones are added to the same read as they come, `onPage` told of each. The
 * read of each repository is kept: asked again for the same history, of the same references and stashes, it keeps its
 * commits and asks git nothing (`repo.schedule-reads`); of other references or stashes, or after `readAgain`, the
 * history is read again over the commits it has, which stay while they are the same (Q781); another history stops it
 * and starts a new one. Past 100,000 commits, the next read starts from the last commit read, when more are asked for
 * (`graph.load-more`).
 */
export class EngineGraph {
  private readonly reads = new Map<string, StreamedRead>();

  constructor(private readonly onPage: (repository: string) => void = () => {}) {}

  /**
   * The commits of the graph, as soon as `asked.limit` commits or a page of them is read, whichever is fewer, and the
   * commits after them as they come. A read under way of another history of the repository is stopped.
   */
  async commits(client: EngineClient, repository: string, asked: GraphAsked, pageSize: number, signal?: AbortSignal): Promise<GraphCommits> {
    const history = await historyOf(client, repository, asked.filter, asked.scope, asked.head, asked.firstParent, asked.names, signal);
    const timeline: Timeline = { ...history, pageSize, pages: 1, skip: 0, stashes: asked.stashes };
    const read = createHash('sha1').update(JSON.stringify([history, asked.stashes, pageSize]));
    const historyKey = read.copy().digest('hex');
    const key = read.update(JSON.stringify([asked.head, asked.refs, asked.stashList])).digest('hex');
    const known = this.reads.get(repository);
    if (known?.key === key && !known.over && !known.stale) {
      // The same history: its commits are kept, unless more are asked for than one read lists.
      if (known.reading || !known.more || known.count >= asked.limit) return known;
      const wanted = known.count + pageSize;
      this.more(client, repository, known, timeline);
      await known.until(Math.min(asked.limit, wanted), signal);
      return known;
    }
    if (known?.history === historyKey && known.commits.length) {
      // The same history, which may have moved: it is read again over the commits listed, which stay while they are the same.
      known.again(key);
      this.stream(client, repository, known, timeline, (listed) => known.add(listed));
      await known.until(Math.min(asked.limit, pageSize), signal);
      return known;
    }
    known?.cancel();
    const fresh = new StreamedRead(key, historyKey);
    this.reads.set(repository, fresh);
    this.stream(client, repository, fresh, timeline, (listed) => fresh.add(listed));
    await fresh.until(Math.min(asked.limit, pageSize), signal);
    return fresh;
  }

  /**
   * Has the commits of a repository, or of every one, read again the next time they are asked for, as **Refresh** and a
   * change of the settings want: the read goes over the commits listed, which stay while they are the same (Q781).
   */
  readAgain(repository?: string): void {
    for (const [root, read] of this.reads) if (repository === undefined || root === repository) read.stale = true;
  }

  /** Whether a read of the commits of a repository is still adding pages. */
  isReading(repository: string): boolean {
    return this.reads.get(repository)?.reading === true;
  }

  /** Waits until the read of a repository has `count` commits, or ends. */
  async reached(repository: string, count: number): Promise<void> {
    await this.reads.get(repository)?.until(count);
  }

  /** Stops the read of a repository that is not shown any more, when it still adds pages: it is read again once shown. */
  stop(repository: string): void {
    const read = this.reads.get(repository);
    if (!read?.reading) return;
    read.cancel();
    this.reads.delete(repository);
  }

  /** Forgets the commits read in a repository, or in every one, stopping their reads: the next read asks git again. */
  forget(repository?: string): void {
    for (const [root, read] of this.reads) {
      if (repository !== undefined && root !== repository) continue;
      read.cancel();
      this.reads.delete(root);
    }
  }

  /** Runs a read of the engine for a read of the commits, `take` given the rows of each page. */
  private stream(client: EngineClient, repository: string, read: StreamedRead, timeline: Timeline, take: (listed: Listed[]) => void): void {
    read.reading = true;
    const stop = read.stop;
    client
      .graphStream(
        repository,
        timeline,
        (rows) => {
          if (stop.signal.aborted) return;
          take(listedOf(rows));
          this.onPage(repository);
        },
        stop.signal,
      )
      .then(
        ({ hasMore }) => {
          if (stop.signal.aborted) return;
          read.end(hasMore);
          this.onPage(repository);
        },
        (error: unknown) => {
          if (stop.signal.aborted) return;
          read.fail(error);
          if (this.reads.get(repository) === read) this.reads.delete(repository);
          this.onPage(repository);
        },
      );
  }

  /**
   * Reads the commits after the 100,000 of a read, from the last of them: the read starts with it when the history is
   * the same up to it. When it does not, the history changed under the commits read, and the read is forgotten.
   */
  private more(client: EngineClient, repository: string, read: StreamedRead, timeline: Timeline): void {
    const last = read.commits.findLast((commit) => commit.stash === undefined);
    read.stop = new AbortController();
    let first = true;
    this.stream(client, repository, read, { ...timeline, skip: read.count - 1, stashes: false }, (listed) => {
      if (!first) return read.add(listed);
      first = false;
      if (listed[0]?.hash === last?.hash) return read.add(listed.slice(1));
      read.stop.abort();
      read.end(true);
      if (this.reads.get(repository) === read) this.reads.delete(repository);
    });
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

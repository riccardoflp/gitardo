import * as path from 'node:path';
import * as vscode from 'vscode';
import { EngineError, ErrorCode } from '../engine/connection';
import type { EngineClient } from '../engine/client';
import { type CommitsReader, EngineGraph, graphState, laidOutOf } from '../engine/graph';
import { type Repository, reads } from '../engine/reads';
import { pullWords, readsPullRequests, type Hosting } from '../shared/hosting';
import { samePath } from '../shared/paths';
import type { Filter, Loading, PullRequest, RepoInfo, RepoParts, RepoState, Scope, ToViews } from '../shared/protocol';
import type { Activity } from './activity';
import { githubSession } from './github';

/** How many commits the graph reads at a time, unless the settings ask for another number between the two after it. */
const PAGE_SIZE = 1000;
const SMALLEST_PAGE = 100;
const LARGEST_PAGE = 10_000;

export const REFRESH_DELAY = 400;
/** Builds and installs write many files at once, so those are given longer to settle. */
export const WORKTREE_REFRESH_DELAY = 1000;
/** How long the pages of a read wait for the next before the views get them together, so that they are drawn less often. */
const PAGES_DELAY = 150;
/** How old the last read is when the window getting the focus asks for one (`repo.watch-changes`, Q463). */
const FOCUS_READ_AFTER = 60_000;
/** Pull requests change on GitHub's side, so they are re-read at most this often unless something asks for it. */
const PULLS_MAX_AGE = 120_000;

export const errorText = (error: unknown) => (error instanceof Error ? error.message : String(error));

/** Whether a read failed because it was cancelled for another. */
export const isCancelled = (error: unknown) => error instanceof EngineError && error.code === ErrorCode.cancelled;

/** What git said when a read failed in the engine; the error itself otherwise. */
export function failureText(error: unknown): string {
  const output = error instanceof EngineError ? (error.data as { output?: string | null } | undefined)?.output?.trim() : undefined;
  return output || errorText(error);
}

/**
 * Whether a file of the folder of git that changed can mean that the repository did. What git writes for itself and
 * takes away again does not: the daemon of `core.fsmonitor` leaves a file in the folder of a worktree each time
 * `git status` runs there, as it does each time Gitardo reads, which would have it read again and again for ever, and on
 * Linux and macOS listens on a socket beside it; and a lock is followed by the file it locked.
 */
function meansChange(file: string): boolean {
  return !/(^|\/)fsmonitor--daemon(\.ipc)?(\/|$)/.test(file) && !file.endsWith('.lock');
}

/** What the status bar says of a read of the repository: what asked for it, and how many came one after another. */
function readDetail(reads: number, cause: string | undefined): string | undefined {
  const why = cause ? `Because ${cause}.` : '';
  if (reads === 1) return why || undefined;
  return `Read ${reads} times in a row: something changed during each read. ${why}`.trim();
}

/**
 * Whether a repository left keeps what the views showed of it, and is read again only once it is shown when it changed
 * meanwhile, as `gitardo.repositories.keepOpen` asks (`repo.keep-open`, `repo.watch-hidden`). Off by default.
 */
export function keepOpen(): boolean {
  return vscode.workspace.getConfiguration('gitardo').get('repositories.keepOpen') === true;
}

/** How many commits the graph reads at a time, as `gitardo.graph.pageSize` asks. */
export function pageSize(): number {
  const wanted = Math.trunc(Number(vscode.workspace.getConfiguration('gitardo').get('graph.pageSize')));
  return Number.isFinite(wanted) ? Math.min(Math.max(wanted, SMALLEST_PAGE), LARGEST_PAGE) : PAGE_SIZE;
}

/** Whether the graph follows only the first parent of each merge, as `gitardo.graph.firstParent` asks. */
export function firstParent(): boolean {
  return vscode.workspace.getConfiguration('gitardo').get('graph.firstParent') === true;
}

/** What a group of views asks of its window: to be sent what it shows, and what it does once the whole history is read. */
export interface GroupHost {
  /** Sends a message to the views of a group. */
  post(group: ViewGroup, message: ToViews): void;
  /** Sends the views of a group that list commits the rows its state has now, read since it was sent. */
  sendRows(group: ViewGroup): void;
  /** The read of the whole history of what a group shows ended. */
  historyRead(group: ViewGroup): void;
}

/**
 * The views that show one repository together, and the history they list (D89): the panel and the views of the side
 * bar, which follow the repository chosen, or a tab of the graph in the editor, which keeps its own. Each group has its
 * search, its branches shown and how far its history is read, and the state read for it; the repository is read once
 * for every group that shows it, by its session.
 */
export class ViewGroup {
  /** The session of the repository the group shows, if it shows one. */
  session?: RepositorySession;
  /** The state of the repository the views of the group were sent last, with the rows of their history. */
  state: RepoState | null = null;
  filter: Filter | null = null;
  /** Which branches the graph shows: all of them when null. */
  scope: Scope | null = null;
  /** How many commits the graph reads. */
  limit = pageSize();
  /** The repository of the workspace shown before a worktree of it that is not in the workspace. */
  home?: RepoInfo;
  /** What the views were told the graph is read for, and how many of those reads are not done yet. */
  waiting: { what: Loading; count: number } | null = null;
  /** A commit to show as soon as a graph view of the group is there to show it. */
  pendingReveal?: string;
  /**
   * The commits of the graph the engine reads, the pages of each read added as they come (stage 7, D54), and kept while
   * the history is the same.
   */
  readonly graph: EngineGraph;
  /** The read of the commits of the graph under way in the engine, to cancel when another takes its place. */
  private graphRead?: AbortController;
  /** When the pages read since the state was sent are sent to the views. */
  private pagesTimer?: ReturnType<typeof setTimeout>;

  constructor(
    private readonly host: GroupHost,
    /** The tab of the graph the group is, or none for the panel and the side bar. */
    readonly panel?: vscode.WebviewPanel,
  ) {
    this.graph = new EngineGraph((repository) => this.pageRead(repository));
  }

  /** The repository the group shows. */
  get repo(): Repository | undefined {
    return this.session?.repo;
  }

  /** Whether the views of the group search inside the changes, which the status bar says. */
  get searching(): boolean {
    return this.filter?.kind === 'pickaxe' || this.filter?.kind === 'regex';
  }

  /** The search, the branches shown and how far the history is read go back to what a repository just shown starts with. */
  reset(scope: Scope | null): void {
    this.limit = pageSize();
    this.cancelGraphRead();
    this.filter = null;
    this.scope = scope;
  }

  /**
   * What reads the commits of the graph in the engine: in pages of `gitardo.graph.pageSize`, the next ones from the last
   * row read. A read that something else takes the place of is cancelled.
   */
  graphReader(client: EngineClient, repo: Repository): CommitsReader {
    return async (asked) => {
      this.graphRead?.abort();
      const reading = new AbortController();
      this.graphRead = reading;
      try {
        return await this.graph.commits(client, repo.root, asked, pageSize(), reading.signal);
      } finally {
        if (this.graphRead === reading) this.graphRead = undefined;
      }
    };
  }

  /** Cancels the read of the commits under way, which a read of another history or of more commits takes the place of. */
  cancelGraphRead(): void {
    this.graphRead?.abort();
    this.graphRead = undefined;
  }

  /**
   * The state of the group from the parts of its repository read, with the commits of its history. A search git cannot
   * run, such as an invalid regular expression, is reported and dropped rather than taking the graph down. Null when the
   * read of the commits was cancelled, for a read that takes its place.
   */
  async readGraph(client: EngineClient, repo: Repository, parts: RepoParts, stashes: boolean, log: vscode.LogOutputChannel): Promise<RepoState | null> {
    const reader = this.graphReader(client, repo);
    const asked = (filter: Filter | null) => ({ limit: this.limit, filter, scope: this.scope, stashes, firstParent: firstParent() });
    // The rows shown are kept, and laid out further, when they are of the same read.
    const shown = this.state && samePath(this.state.root, repo.root) ? laidOutOf(this.state.rows) : undefined;
    try {
      if (!this.filter) return await graphState(client, repo.root, parts, asked(null), reader, undefined, shown);
      try {
        return await graphState(client, repo.root, parts, asked(this.filter), reader, undefined, shown);
      } catch (error) {
        if (isCancelled(error)) throw error;
        this.filter = null;
        const text = failureText(error);
        log.error(text);
        void vscode.window.showErrorMessage(`Gitardo: the search failed. ${text.split('\n')[0]}`);
        return await graphState(client, repo.root, parts, asked(null), reader, undefined, shown);
      }
    } catch (error) {
      if (isCancelled(error)) return null;
      throw error;
    }
  }

  /**
   * A page of the commits of a repository was read: once the pages stop coming for a moment, or the read ends, the
   * views get their rows, laid out under the rows they have (`graph.load-more`, `graph.lanes`).
   */
  private pageRead(repository: string): void {
    if (!this.state || !samePath(this.state.root, repository)) return;
    if (!this.graph.isReading(repository)) {
      this.showPages();
      // The read of the whole history ended: the commit-graph file is offered when it would make it quicker (M44).
      return this.host.historyRead(this);
    }
    this.pagesTimer ??= setTimeout(() => this.showPages(), PAGES_DELAY);
  }

  /** Sends the views the rows of the pages read since the state they have, without reading anything. */
  showPages(): void {
    clearTimeout(this.pagesTimer);
    this.pagesTimer = undefined;
    const state = this.state;
    const laid = state && laidOutOf(state.rows);
    if (!state || !laid) return;
    const rows = laid.catchUp();
    const hasMore = laid.source.hasMore;
    if (rows === state.rows && hasMore === state.hasMore) return;
    this.state = { ...state, rows, hasMore };
    // The rows of a page change nothing of what the other views show: only those that list commits are sent them.
    this.host.sendRows(this);
  }

  /**
   * Reads the graph again, or does what ends with that, telling the views what for while they wait. Reads that overlap
   * are one wait: the views are told it is over when the last of them is, and what for by the first.
   */
  async reload(what: Loading, work: () => Promise<void>): Promise<void> {
    if (this.waiting) this.waiting.count++;
    else {
      this.waiting = { what, count: 1 };
      this.host.post(this, { type: 'loading', what });
    }
    try {
      await work();
    } finally {
      if (!--this.waiting!.count) {
        this.waiting = null;
        this.host.post(this, { type: 'loading', what: null });
      }
    }
  }

  dispose(): void {
    clearTimeout(this.pagesTimer);
    this.cancelGraphRead();
    this.graph.forget();
  }
}

/** What a session asks of its window: the operation running, the groups on it, and what is done with what it reads. */
export interface SessionHost {
  readonly log: vscode.LogOutputChannel;
  readonly activity: Activity;
  /** Whether an operation runs: nothing is read meanwhile. */
  readonly running: boolean;
  /** How many operations were started, to tell a state that was read across one. */
  readonly operations: number;
  /** The groups of views that show the repository of a session. */
  groupsOn(session: RepositorySession): ViewGroup[];
  /** Whether a view is there to show what is read of a session: nothing is read for none. */
  hasViews(session: RepositorySession): boolean;
  /** The name of a repository as the list of the repositories has it, or its folder's. */
  repositoryNamed(root: string): string;
  /** Whether conflicts are resolved by the rules, which the state says of each file in conflict. */
  autoResolve(): boolean;
  /** The state read for a group, with what the window adds to it: its favorites, its settings, its pull requests. */
  complete(group: ViewGroup, read: RepoState): Promise<RepoState>;
  /** A group was read: its views are sent its state, or told that there is none. */
  stateRead(group: ViewGroup, state: RepoState | null): void;
  /** The branches shown as git took them, without those deleted since they were chosen, for a group. */
  settleScope(group: ViewGroup, root: string, scope: Scope | null): void;
  /** Something changed in a repository, whose session reads it again, or which is marked out of date when it is kept. */
  changedIn(root: string, cause: string, delay?: number): void;
  /** A file of the working tree of a repository changed. */
  fileChangedIn(root: string, file: string): void;
  /** The pull requests of a session were read again, and differ from what its groups were sent. */
  pullsRead(session: RepositorySession): void;
  /** Forgets the token of a GitLab that the GitLab refused, so that signing in asks for another (`hosting.token`). */
  forgetToken?(host: string): Promise<void>;
}

/**
 * What the window holds for one repository that a group of views shows (D89, `repo.tab-repository`): the reads of its
 * state and their loop, its watchers, the hosting service of its main remote and its pull requests. The repository is
 * read once for all the groups that show it, each with the commits of its own history, and watched once.
 */
export class RepositorySession {
  readonly repo: Repository;
  /** The hosting service of the main remote, as the engine found it (`hosting.detect-remote`). */
  hosting: Hosting | null = null;
  pulls: Record<string, PullRequest> = {};
  pullsAt = 0;
  /** The read of the repository under way, if any. */
  loading?: Promise<void>;
  /** Who waits for a read that starts after they asked: each is let go once one is done, or when reading stops. */
  private waiters: (() => void)[] = [];
  /** What changed last that the repository is read again for, as the status bar says it. */
  cause?: string;
  /** Something changed since that read started. */
  private dirty = false;
  /** Counts the changes heard of that ask for a read, so that a view that opens can tell whether the state held saw them all. */
  private heard = 0;
  /** How many changes had been heard of when the read under way started. */
  private readingHeard = 0;
  /** How many changes had been heard of when the state held was read; -1 for a state that no read of this window made. */
  stateHeard = -1;
  /** The next read is a **Refresh**, or the read of a repository chosen: every worktree is counted again, every commit read. */
  everything = false;
  /** When the last read ended, for the focus (`repo.watch-changes`). */
  private readAt?: number;
  /** A change came while no view could show it, so that the focus asks for a read. */
  private missed = false;
  /** The files of the working tree changed since the last read, which the engine is asked about a second after the first. */
  private readonly changedFiles = new Set<string>();
  /** When the files changed are asked about; none while none waits. */
  private filesTimer?: ReturnType<typeof setTimeout>;
  /** Counts the reads started, so that an answer about the files asked before one is not waited for any more. */
  private readsStarted = 0;
  private timer?: ReturnType<typeof setTimeout>;
  /** When `timer` fires. */
  private due = 0;
  /** The watchers of its folder of git and of its working tree. */
  watcher?: vscode.Disposable;
  /** Its session was let go: what it reads is no longer shown. */
  disposed = false;

  constructor(
    root: string,
    private readonly host: SessionHost,
  ) {
    this.repo = { root };
  }

  get root(): string {
    return this.repo.root;
  }

  /** The state the groups on it were sent last, for what all of them share: its references, its worktrees, its files. */
  get state(): RepoState | null {
    return this.host.groupsOn(this).find((group) => group.state && samePath(group.state.root, this.root))?.state ?? null;
  }

  /** What a group was shown is as fresh as a read: no change was heard of since, as for a repository kept that did not change. */
  markFresh(): void {
    this.stateHeard = this.heard;
  }

  /** Whether no change was heard of since the last read was made. */
  get isFresh(): boolean {
    return this.stateHeard === this.heard;
  }

  /** Looks for its folder of git to watch it and for the hosting service of its main remote, and reads it. */
  async open(): Promise<void> {
    const gitDir = await reads.commonDir(this.repo);
    // Let go of meanwhile: nothing is watched for it.
    if (this.disposed) return;
    this.watcher = this.watch(gitDir);
    await this.detectHosting();
    await this.refresh();
  }

  /**
   * **Refresh**: reads the repository again whole, counting what is not committed in every other worktree and reading
   * every commit again, which the reads that follow a change keep when nothing changed there (M15).
   */
  refreshAll(): Promise<void> {
    this.everything = true;
    return this.refresh();
  }

  refresh(): Promise<void> {
    // A read already under way may have started before what the caller just did: it is followed by another,
    // and the caller waits for that one too.
    this.dirty = true;
    // Nothing is read while an operation runs: it reads once it is done. Reading runs git in every worktree, and on
    // Windows a folder git runs in can be neither moved nor removed.
    if (this.host.running) return this.loading ?? Promise.resolve();
    // Only for that one: while files keep changing, a large repository is read again and again, and what the caller
    // waits for is already there after the first.
    const read = new Promise<void>((resolve) => this.waiters.push(resolve));
    this.loading ??= this.load();
    return read;
  }

  private async load(): Promise<void> {
    const task = this.host.activity.start(this.readTitle());
    let waiting: (() => void)[] = [];
    try {
      for (let reads = 1; this.dirty && !this.host.running && !this.disposed; reads++) {
        this.dirty = false;
        waiting = this.waiters.splice(0);
        const cause = this.cause;
        this.cause = undefined;
        // The read that starts now sees what the changes seen until now did: the read they had asked for is this one.
        clearTimeout(this.timer);
        this.timer = undefined;
        task.describe(readDetail(reads, cause), this.readTitle());
        // The read sees what changed until now: the files waiting to be asked about, and a change the focus missed.
        this.readStarted();
        const heard = (this.readingHeard = this.heard);
        const everything = this.everything;
        this.everything = false;
        const operations = this.host.operations;
        const groups = this.host.groupsOn(this);
        const scopes = groups.map((group) => group.scope);
        let read: (RepoState | null)[] | null;
        try {
          read = await this.readState(groups, everything);
        } catch (error) {
          // An operation that ran meanwhile can take from under the read what it was reading, such as a conflict
          // resolved while its versions were read: the read is dropped as below, and the one after it says what is there.
          if (!this.host.running && operations === this.host.operations && !this.disposed) throw error;
          read = null;
        }
        // Read while an operation ran, it can be half from before it and half from after, such as a new commit that
        // no branch points at yet. It is dropped: the operation is followed by a read of its own.
        // Unless the views were pointed at another repository in the meantime. Who waited for it waits for the next,
        // as for a read of the commits cancelled for another.
        if (read) this.readAt = Date.now();
        if (!read || read.includes(null) || this.host.running || operations !== this.host.operations || this.disposed) {
          // A repository left while it was read keeps what it showed before: it is read again once it is shown.
          if (this.disposed) this.host.changedIn(this.root, cause ?? 'it was left while it was read');
          this.waiters.unshift(...waiting.splice(0));
          continue;
        }
        for (const [index, group] of groups.entries()) {
          const state = read[index]!;
          // A group pointed at another repository while this one was read is read there.
          if (group.session !== this) continue;
          // Unless other branches were chosen while this one was read.
          if (scopes[index] === group.scope) this.host.settleScope(group, state.root, state.scope);
          const shown = await this.host.complete(group, state);
          if (group.session === this) this.host.stateRead(group, shown);
        }
        this.stateHeard = heard;
        for (const resolve of waiting.splice(0)) resolve();
      }
    } catch (error) {
      this.host.log.error(errorText(error));
      this.stateHeard = -1;
      for (const group of this.host.groupsOn(this)) this.host.stateRead(group, null);
    } finally {
      // In the same turn as the last look at `dirty`, so that no request can fall between the two.
      this.dirty = false;
      this.loading = undefined;
      for (const resolve of [...waiting, ...this.waiters.splice(0)]) resolve();
      task.end();
    }
    // Not for the read at the start of VS Code: GitHub is asked only once a view can show what it says.
    if (this.host.hasViews(this) && !this.disposed) void this.refreshPulls(false);
  }

  /**
   * Reads what the views show but the commits once, and then the commits of the history of each group from what it
   * says (stage 7). Null for a group whose read of the commits was cancelled, for a read that takes its place.
   */
  async readState(groups: ViewGroup[], everything = false): Promise<(RepoState | null)[]> {
    const client = reads.client();
    const stashes = vscode.workspace.getConfiguration('gitardo').get('graph.showStashes') !== false;
    // **Refresh** reads every commit again; the other reads keep them while the history is the same (Q442). The rows of
    // the commits it finds the same stay as they are, and the views are sent none of them (Q781).
    if (everything) for (const group of groups) group.graph.readAgain(this.root);
    const parts = await reads.parts(this.repo, this.host.autoResolve(), everything);
    return Promise.all(groups.map((group) => group.readGraph(client, this.repo, parts, stashes, this.host.log)));
  }

  /** What the status bar says the repository is read for, by the name the list of repositories gives it. */
  private readTitle(): string {
    const searching = this.host.groupsOn(this).some((group) => group.searching);
    return `${searching ? 'Searching' : 'Reading'} ${this.host.repositoryNamed(this.root)}`;
  }

  /**
   * What a view that opens waits for: nothing when the state its group holds was read after the last change heard of,
   * the read under way when it started after that change, and a read otherwise. Views that open together, as when VS
   * Code starts, make one read at most (D88). A first read still under way is sent to every view when it is done.
   */
  async readForView(group: ViewGroup): Promise<void> {
    const fresh = group.state !== null && samePath(group.state.root, this.root) && this.isFresh;
    if (fresh && !this.loading) {
      // Not read at the start of VS Code: GitHub is asked only once a view can show what it says.
      void this.refreshPulls(false);
      return;
    }
    if (this.loading && this.readingHeard === this.heard) return this.loading;
    return this.refresh();
  }

  /** Refreshes once, `delay` after the first of a burst of events, however long the burst lasts. `cause` is the last of them. */
  scheduleRefresh(cause: string, delay = REFRESH_DELAY): void {
    this.heard++;
    const views = this.host.hasViews(this);
    // With no view to show it, the change is remembered for when the window gets the focus.
    if (!views) this.missed = true;
    if (!views || this.host.running) return;
    this.cause = cause;
    const due = Date.now() + delay;
    if (this.timer && this.due <= due) return;
    clearTimeout(this.timer);
    this.due = due;
    this.timer = setTimeout(() => {
      this.timer = undefined;
      void this.refresh();
    }, delay);
  }

  /** A change seen just before it is left, which is its own and not the next one's; none when nothing waits to be read. */
  takePending(): string | undefined {
    const pending = this.timer !== undefined ? (this.cause ?? 'something changed') : undefined;
    clearTimeout(this.timer);
    this.timer = undefined;
    this.cause = undefined;
    return pending;
  }

  /**
   * The window got the focus: it asks for a read only when a change came while no view could show it, or when the last
   * read ended more than a minute ago, or it was never read (`repo.watch-changes`, Q463).
   */
  focusGot(): void {
    if (this.missed || this.readAt === undefined || Date.now() - this.readAt > FOCUS_READ_AFTER) this.scheduleRefresh('VS Code got the focus');
  }

  /**
   * A file of the working tree changed (`repo.watch-changes`, M15). One in a worktree nested in the repository asks for
   * nothing; the others wait to be asked of the engine together, a second after the first of them, which says which git
   * does not ignore: only those ask for a read.
   */
  fileChanged(file: string): void {
    if (!this.host.hasViews(this) || this.host.running) return this.scheduleRefresh(`${file} changed`, WORKTREE_REFRESH_DELAY);
    if (this.inNestedWorktree(file)) return;
    this.changedFiles.add(file);
    this.filesTimer ??= setTimeout(() => void this.askChangedFiles(), WORKTREE_REFRESH_DELAY);
  }

  /** Whether a file, from the top of the repository, is in the folder of another worktree inside it. */
  private inNestedWorktree(file: string): boolean {
    const state = this.state;
    const root = this.root;
    if (!state) return false;
    const inside = (folder: string) => {
      const relative = path.relative(root, folder).replace(/\\/g, '/');
      return relative && !relative.startsWith('..') && !path.isAbsolute(relative) ? relative : null;
    };
    const lower = (text: string) => (process.platform === 'win32' || process.platform === 'darwin' ? text.toLowerCase() : text);
    return state.worktrees.some((worktree) => {
      const folder = !worktree.isCurrent && inside(worktree.folder);
      return !!folder && (lower(file) === lower(folder) || lower(file).startsWith(`${lower(folder)}/`));
    });
  }

  /**
   * Asks the engine which of the files changed ask for a read, and reads the repository at once when one does: their
   * second is over. Files that change meanwhile wait for the next question; a read that starts meanwhile sees them all,
   * and the answer is not waited for. When the engine cannot say, every file asks for a read.
   */
  private async askChangedFiles(): Promise<void> {
    this.filesTimer = undefined;
    const files = [...this.changedFiles];
    this.changedFiles.clear();
    if (!files.length || this.disposed) return;
    const started = this.readsStarted;
    const toRead = await reads
      .client()
      .changesToRead(this.root, files)
      .catch((error: unknown) => {
        this.host.log.warn(`Could not ask which changed files git ignores: ${errorText(error)}`);
        return files;
      });
    if (this.changedFiles.size) this.filesTimer ??= setTimeout(() => void this.askChangedFiles(), WORKTREE_REFRESH_DELAY);
    if (!toRead.length || started !== this.readsStarted || this.disposed) return;
    this.scheduleRefresh(`${toRead[0]} changed`, 0);
  }

  /** A read starts: it sees every change until now, so that nothing waits to be asked about, nor for the focus. */
  private readStarted(): void {
    this.readsStarted++;
    this.missed = false;
    this.changedFiles.clear();
    clearTimeout(this.filesTimer);
    this.filesTimer = undefined;
  }

  /**
   * Asks the engine which hosting service the main remote is on (`hosting.detect-remote`), with the services of one's own
   * of `gitardo.hosting.selfHosted`; the pull requests known are forgotten when it is another.
   */
  async detectHosting(): Promise<void> {
    const hosting = await reads.hosting(this.root);
    if (JSON.stringify(hosting) === JSON.stringify(this.hosting)) return;
    this.hosting = hosting;
    this.pulls = {};
    this.pullsAt = 0;
  }

  /**
   * Reads the open pull requests from GitHub, or the merge requests of a GitLab (`hosting.read-merge-requests`). Without
   * `force` it reuses what it has for a while; it reads only with a token already there: the user signed in to GitHub in
   * VS Code, or the token of the GitLab kept in the secret storage.
   */
  async refreshPulls(force: boolean): Promise<void> {
    const hosting = this.hosting;
    if (!hosting || !readsPullRequests(hosting.service)) return;
    if (!force && Date.now() - this.pullsAt < PULLS_MAX_AGE) return;
    const gitLab = hosting.service === 'gitLab';
    const words = `${pullWords(hosting.service)}s`;
    try {
      // On GitHub only once the user signed in in VS Code: the engine asks it for the same session (`host/githubToken`).
      if (!reads.available || (!gitLab && !(await githubSession(false)))) return;
      this.pullsAt = Date.now();
      const what = gitLab ? 'Reading the merge requests from GitLab' : 'Reading the pull requests from GitHub';
      const pulls = await this.host.activity.track(what, () => reads.pullRequests(this.root, hosting.remote, hosting.service));
      if (hosting !== this.hosting || JSON.stringify(pulls) === JSON.stringify(this.pulls)) return;
      this.pulls = pulls;
      this.host.pullsRead(this);
    } catch (error) {
      // A token the GitLab refused is forgotten: the next sign-in asks for another.
      if (gitLab && (error as { isUnauthorized?: boolean }).isUnauthorized) await this.host.forgetToken?.(hosting.host);
      this.host.log.warn(`Could not read ${words}: ${errorText(error)}`);
    }
  }

  /**
   * Picks up changes made outside Gitardo: the terminal, another tool, a background fetch. The watchers go on with the
   * repository when it is kept while no view shows it, and a change there only marks it out of date (`repo.watch-hidden`).
   */
  private watch(gitDir: string): vscode.Disposable {
    const root = this.root;
    const pattern = new vscode.RelativePattern(
      vscode.Uri.file(gitDir),
      // The submodules keep their repositories in it too: what is checked out in them and their branches.
      '{HEAD,FETCH_HEAD,ORIG_HEAD,MERGE_HEAD,CHERRY_PICK_HEAD,REVERT_HEAD,BISECT_LOG,index,config,packed-refs,refs/**,logs/refs/stash,worktrees/**,modules/**/HEAD,modules/**/refs/**}',
    );
    const watcher = vscode.workspace.createFileSystemWatcher(pattern);
    const relative = (from: string, uri: vscode.Uri) => path.relative(from, uri.fsPath).replace(/\\/g, '/');
    const schedule = (uri: vscode.Uri) => {
      const file = relative(gitDir, uri);
      if (!meansChange(file)) return;
      this.host.changedIn(root, `.git/${file} changed`);
    };
    // Files written by something other than this editor, such as a command in the terminal. Git's own files have the watcher above.
    const files = vscode.workspace.createFileSystemWatcher(new vscode.RelativePattern(vscode.Uri.file(root), '**'));
    const fileChanged = (uri: vscode.Uri) => {
      if (/(^|\/)\.git(\/|$)/.test(uri.path)) return;
      this.host.fileChangedIn(root, relative(root, uri));
    };
    return vscode.Disposable.from(
      watcher,
      watcher.onDidChange(schedule),
      watcher.onDidCreate(schedule),
      watcher.onDidDelete(schedule),
      files,
      files.onDidChange(fileChanged),
      files.onDidCreate(fileChanged),
      files.onDidDelete(fileChanged),
    );
  }

  /**
   * Lets go of the session: nothing is read for it any more. Its watchers are stopped, unless `keepWatcher`, for a
   * repository kept while no view shows it, whose watchers then go with what it keeps.
   */
  dispose(keepWatcher = false): void {
    this.disposed = true;
    clearTimeout(this.timer);
    clearTimeout(this.filesTimer);
    this.timer = undefined;
    if (!keepWatcher) this.watcher?.dispose();
    this.watcher = undefined;
    for (const resolve of this.waiters.splice(0)) resolve();
  }
}

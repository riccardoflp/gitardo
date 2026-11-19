import * as path from 'node:path';
import * as vscode from 'vscode';
import { reads } from '../engine/reads';
import { samePath } from '../shared/paths';
import type { HostMessage, RepoState } from '../shared/protocol';
import { type FollowedFolders, type ListedRepository, repositoryRows, routeChange } from '../shared/repositories';
import type { Grouped } from '../shared/repositoryGroups';
import type { Worktree } from '../shared/state';
import type { FolderSummary } from '../shared/summaries';

/** How long after the first change of a burst in a folder the engine is told of it, in milliseconds. */
const SAY_AFTER = 300;

/** How many paths changed in a folder are kept for the engine; with more, it is told to read the folder whatever changed. */
const PATHS_KEPT = 200;

/** What the watchers saw change in a folder since the engine was last told: the paths, or null for more than are kept. */
type Seen = Set<string> | null;

/**
 * The host side of the Repositories view (`repo.tabs`, milestone M22): it has the engine follow the repositories of
 * the workspace, their worktrees and their submodules (`repo.worktree-summary`), watches the folder of each to tell the
 * engine what changed there, and sends the view its rows. Nothing is followed nor watched until the view is opened;
 * while it is hidden, what the watchers see is kept, and said to the engine once the view is seen again.
 */
export class RepositoriesView implements vscode.Disposable {
  /** The repositories of the workspace, grouped; undefined until they were looked for. */
  private repositories?: Grouped<ListedRepository>[];
  private summaries: FolderSummary[] = [];
  private state: RepoState | null = null;
  /** The folder an operation runs in. */
  private running: string | null = null;
  /** The pages of the view, and whether each is in sight. */
  private readonly pages = new Map<vscode.Webview, boolean>();
  /** The watcher of each folder followed, by the folder. */
  private readonly watchers = new Map<string, vscode.Disposable>();
  /** What changed in each folder that the engine was not told yet. */
  private readonly seen = new Map<string, Seen>();
  private timer?: ReturnType<typeof setTimeout>;
  /** The handler of what the engine reads again, once it is asked to follow. */
  private listening?: { dispose(): void };
  /** The folders the engine was last asked to follow, as one text. */
  private asked?: string;
  /** What the pages were last sent, as one text. */
  private sent?: string;

  constructor(private readonly log: vscode.LogOutputChannel) {}

  dispose(): void {
    clearTimeout(this.timer);
    this.listening?.dispose();
    for (const watcher of this.watchers.values()) watcher.dispose();
    this.watchers.clear();
  }

  /** A page of the view opened: the folders are followed from now on. Disposing what it gives says it closed. */
  open(page: vscode.Webview, isVisible = true): vscode.Disposable {
    this.pages.set(page, isVisible);
    void this.follow();
    return new vscode.Disposable(() => {
      this.pages.delete(page);
      if (!this.pages.size) this.unfollow();
    });
  }

  /** A page of the view came in sight, or left it: what changed while none was in sight is said to the engine now. */
  visible(page: vscode.Webview, isVisible: boolean): void {
    if (!this.pages.has(page)) return;
    this.pages.set(page, isVisible);
    if (isVisible) this.sayLater();
  }

  /** A page has just loaded: it is sent the rows as they are. */
  ready(page: vscode.Webview): void {
    const message = this.message();
    if (message) void page.postMessage(message);
  }

  /** The repositories of the workspace were looked for. */
  found(repositories: Grouped<ListedRepository>[]): void {
    this.repositories = repositories;
    void this.follow();
    this.send();
  }

  /**
   * What Gitardo shows was read. The folder shown is not read by the engine: its row says what this state says. One
   * that is no longer shown is read by the engine whatever changed there, since nothing told it meanwhile.
   */
  shown(state: RepoState | null): void {
    const before = this.state?.folder;
    this.state = state;
    if (state) this.forget(state.folder);
    if (before !== undefined && (!state || !samePath(before, state.folder)) && this.follows(before)) {
      this.forget(before);
      this.seen.set(before, null);
      this.sayLater();
    }
    // Its worktrees may be new ones, which the engine lists a moment later.
    this.watch();
    this.send();
  }

  /** Drops what was seen to change in a folder. */
  private forget(folder: string): void {
    for (const each of [...this.seen.keys()]) if (samePath(each, folder)) this.seen.delete(each);
  }

  /** An operation started in a folder, or the one that ran ended. */
  ran(folder: string | null): void {
    this.running = folder;
    this.send();
  }

  private get isVisible(): boolean {
    return [...this.pages.values()].some(Boolean);
  }

  private follows(folder: string): boolean {
    return [...this.watchers.keys()].some((each) => samePath(each, folder));
  }

  /** Asks the engine to follow the repositories of the workspace, when a page is open and they are not the ones asked. */
  private async follow(): Promise<void> {
    if (!this.pages.size || !this.repositories || !reads.available) return;
    const folders = this.repositories.map((each) => each.repository.folder);
    const asked = JSON.stringify(folders);
    if (asked === this.asked) return;
    this.asked = asked;
    try {
      this.listening ??= reads.onSummaries((summaries) => this.said(summaries));
      const summaries = await reads.summaries(folders);
      // The folders of the workspace changed, or the view closed, while the engine read them.
      if (this.asked !== asked) return;
      this.summaries = summaries;
      this.watch();
      this.send();
    } catch (error) {
      if (this.asked === asked) this.asked = undefined;
      this.log.warn(`Could not read what the repositories of the workspace are on: ${error instanceof Error ? error.message : String(error)}`);
    }
  }

  /** The view closed: nothing is watched, and the engine follows nothing. */
  private unfollow(): void {
    clearTimeout(this.timer);
    this.timer = undefined;
    for (const watcher of this.watchers.values()) watcher.dispose();
    this.watchers.clear();
    this.seen.clear();
    this.summaries = [];
    this.sent = undefined;
    if (this.asked === undefined) return;
    this.asked = undefined;
    if (reads.available) reads.summaries([]).catch(() => {});
  }

  /** The engine read some folders again: what it says of each takes the place of what it said before. */
  private said(summaries: FolderSummary[]): void {
    if (this.asked === undefined) return;
    for (const said of summaries) {
      const at = this.summaries.findIndex((each) => samePath(each.folder, said.folder));
      if (at >= 0) this.summaries[at] = said;
      else this.summaries.push(said);
    }
    this.watch();
    this.send();
  }

  /** The folders the engine follows: those asked, and the worktrees git lists for them whose folder is there. */
  private followed(): FollowedFolders {
    const repositories = this.repositories ?? [];
    // The state lists the worktrees of its repository last: what the engine listed for that one before is left out.
    const ofState = this.state?.worktrees ?? [];
    const inState = (worktrees: Worktree[]) => worktrees.some((each) => ofState.some((own) => samePath(own.folder, each.folder)));
    const lists = [...this.summaries.flatMap((each) => (each.worktrees && !inState(each.worktrees) ? [each.worktrees] : [])), ofState];
    const families = lists.map((worktrees) => worktrees.filter((each) => !each.isMissing).map((each) => each.folder)).filter((family) => family.length > 0);
    const folders: string[] = [];
    for (const folder of [...repositories.map((each) => each.repository.folder), ...families.flat()]) {
      if (!folders.some((each) => samePath(each, folder))) folders.push(folder);
    }
    const submodules = repositories.flatMap(({ repository }) =>
      repository.superproject ? [{ folder: repository.folder, superproject: repository.superproject }] : [],
    );
    return { folders, families, submodules };
  }

  /** Watches each folder followed, and no other. */
  private watch(): void {
    if (this.asked === undefined) return;
    const { folders } = this.followed();
    for (const [folder, watcher] of this.watchers) {
      if (folders.some((each) => samePath(each, folder))) continue;
      watcher.dispose();
      this.watchers.delete(folder);
      this.seen.delete(folder);
    }
    for (const folder of folders) {
      if (this.follows(folder)) continue;
      const watcher = vscode.workspace.createFileSystemWatcher(new vscode.RelativePattern(vscode.Uri.file(folder), '**'));
      const changed = (uri: vscode.Uri) => this.changed(folder, path.relative(folder, uri.fsPath).replace(/\\/g, '/'));
      this.watchers.set(folder, vscode.Disposable.from(watcher, watcher.onDidChange(changed), watcher.onDidCreate(changed), watcher.onDidDelete(changed)));
    }
  }

  /** The watcher of a folder saw a file change: it is kept for each folder it tells of, but the one shown. */
  private changed(folder: string, file: string): void {
    const shown = this.state?.folder;
    for (const change of routeChange(folder, file, this.followed())) {
      if (shown !== undefined && samePath(change.folder, shown)) continue;
      const key = [...this.seen.keys()].find((each) => samePath(each, change.folder)) ?? change.folder;
      const paths = this.seen.has(key) ? this.seen.get(key)! : new Set<string>();
      if (paths) paths.add(change.path);
      this.seen.set(key, paths && paths.size > PATHS_KEPT ? null : paths);
    }
    this.sayLater();
  }

  /** Tells the engine what changed, a moment after the first change of a burst, while a page is in sight. */
  private sayLater(): void {
    if (this.timer || !this.seen.size || !this.isVisible) return;
    this.timer = setTimeout(() => {
      this.timer = undefined;
      if (!this.isVisible) return;
      for (const [folder, paths] of this.seen) reads.folderChanged(folder, paths ? [...paths] : []);
      this.seen.clear();
    }, SAY_AFTER);
  }

  /** What the pages are sent: nothing until the repositories of the workspace were looked for. */
  private message(): HostMessage | undefined {
    if (!reads.available) return { type: 'repositories', rows: null, engineMissing: true };
    if (!this.repositories) return undefined;
    const state = this.state;
    const rows = repositoryRows({ repositories: this.repositories, summaries: this.summaries, state, running: this.running });
    // The menus of the rows of its worktrees and its submodules are made of these, as in the Branches view.
    const shown = state && {
      root: state.root,
      home: state.home,
      agentsWindow: state.agentsWindow,
      worktrees: state.worktrees,
      submodules: state.submodules,
      isLookedAfter: state.isLookedAfter,
    };
    return { type: 'repositories', rows, shown };
  }

  /** Sends the pages their rows, when they are not the ones sent last. */
  private send(): void {
    const message = this.message();
    if (!message || !this.pages.size) return;
    const sent = JSON.stringify(message);
    if (sent === this.sent) return;
    this.sent = sent;
    for (const page of this.pages.keys()) void page.postMessage(message);
  }
}

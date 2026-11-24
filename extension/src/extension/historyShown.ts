/**
 * What the graph of each group of views shows and selects: its search and its branches shown, the branches the
 * references list and the favorites, the history of a file, a folder or some lines, and the commit selected, brought
 * into sight, gone to or found by a search.
 */
import * as path from 'node:path';
import * as vscode from 'vscode';
import { EngineError, ErrorCode } from '../engine/connection';
import { historyOf, matchesOnEngine, positionOnEngine } from '../engine/graph';
import { type Repository, reads } from '../engine/reads';
import { samePath } from '../shared/paths';
import { type Filter, type RepoState, type Scope, type Search, showsGraph } from '../shared/protocol';
import { isHash } from '../shared/refs';
import { pruneScope } from '../shared/scope';
import type { ControllerHost } from './controllerHost';
import { type ViewGroup, failureText, firstParent, pageSize } from './repositorySession';
import { askRevision } from './revisions';

/** What the references of each repository list, by its folder, as their own filter says, apart from `gitardo.scope`. */
const LISTED_KEY = 'gitardo.branchesListed';
/** The favorite branches of each repository of the workspace. */
const FAVORITES_KEY = 'gitardo.favorites';

/** Narrows, widens and selects in the graph of a group: the panel's unless another is given. */
export class HistoryShown {
  constructor(
    private readonly host: Pick<
      ControllerHost,
      | 'followers'
      | 'groupOf'
      | 'groupsShowing'
      | 'log'
      | 'memento'
      | 'post'
      | 'postToGroup'
      | 'reload'
      | 'repos'
      | 'requireState'
      | 'selectRepo'
      | 'setState'
      | 'showRepoOf'
      | 'views'
      | 'whenLocated'
    >,
  ) {}

  /** Narrows the graph of a group to the history of a file or to a search, or shows everything again. */
  async setFilter(filter: Filter | null, on = this.host.followers): Promise<void> {
    await this.host.whenLocated();
    on.filter = filter;
    on.limit = pageSize();
    on.cancelGraphRead();
    // Searching inside the changes of every commit can take a while on a large repository, and so can following lines.
    const slow = filter?.kind === 'pickaxe' || filter?.kind === 'regex' || filter?.kind === 'lines';
    if (slow) this.host.postToGroup(on, { type: 'busy', title: 'Searching' });
    try {
      await this.host.reload(on, 'filter');
    } finally {
      if (slow) this.host.postToGroup(on, { type: 'busy', title: null });
    }
  }

  /** Narrows the graph of a group to the history of some branches, or shows them all again. What the filter does is left to it. */
  async setScope(scope: Scope | null, on = this.host.followers): Promise<void> {
    await this.host.whenLocated();
    // As it is given: reading the repository leaves out what is not a branch of it.
    on.scope = scope && typeof scope === 'object' ? scope : null;
    on.limit = pageSize();
    on.cancelGraphRead();
    await this.host.reload(on, 'filter');
  }

  /**
   * Chooses what the references list, in the repository a group shows, apart from the branches shown in the graph
   * (`refs.list-filter`). Nothing is read: the views of every group that shows it are told at once.
   */
  async setListed(listed: unknown, on = this.host.followers): Promise<void> {
    const state = await this.host.requireState(on);
    if (!state) return;
    const kept = pruneScope(listed, state);
    const others = Object.fromEntries(Object.entries(this.host.memento.get<Record<string, unknown>>(LISTED_KEY, {})).filter(([root]) => root !== state.root));
    await this.host.memento.update(LISTED_KEY, kept ? { ...others, [state.root]: kept } : others);
    for (const group of this.host.groupsShowing(state.root)) this.host.setState(group, { ...group.state!, listed: kept });
  }

  /** Marks a branch as a favorite, or no longer. Nothing of the repository changes: the views are told at once. */
  async setFavorite(ref: string, favorite: boolean, on = this.host.followers): Promise<void> {
    const state = await this.host.requireState(on);
    if (!state || typeof ref !== 'string') return;
    // The ones whose branch is gone are forgotten as soon as there is something to save.
    const others = this.favorites(state).filter((other) => other !== ref);
    const next = favorite ? [...others, ref] : others;
    await this.host.memento.update(FAVORITES_KEY, { ...this.host.memento.get<Record<string, unknown>>(FAVORITES_KEY, {}), [state.root]: next });
    for (const group of this.host.groupsShowing(state.root)) this.host.setState(group, { ...group.state!, favorites: this.favorites(group.state!) });
  }

  /** The favorites of a repository that are still branches of it, by their full names. */
  favorites(state: Pick<RepoState, 'root' | 'branches' | 'remoteBranches'>): string[] {
    const known = new Set([...state.branches.map((branch) => `refs/heads/${branch.name}`), ...state.remoteBranches.map((ref) => `refs/remotes/${ref.name}`)]);
    const stored: unknown = this.host.memento.get<Record<string, unknown>>(FAVORITES_KEY, {})[state.root];
    return Array.isArray(stored) ? stored.filter((ref): ref is string => typeof ref === 'string' && known.has(ref)) : [];
  }

  /**
   * What the references of a repository list, as their own filter was last set there (`refs.listed`), without the
   * references gone since: null, everything, once none of those ticked is left. What is kept is written again only when
   * the choice changes.
   */
  listedIn(state: Pick<RepoState, 'root' | 'head' | 'branches' | 'remoteBranches' | 'tags'>): Scope | null {
    return pruneScope(this.host.memento.get<Record<string, unknown>>(LISTED_KEY, {})[state.root], state);
  }

  /** Shows the commits that touched a file: the given one, or the one in the active editor. */
  async fileHistory(uri = vscode.window.activeTextEditor?.document.uri): Promise<void> {
    if (!(await this.host.requireState())) return;
    if (uri?.scheme === 'file') await this.host.showRepoOf(uri);
    if (!this.host.followers.repo) return;
    const file = uri?.scheme === 'file' ? path.relative(this.host.followers.repo.root, uri.fsPath).replace(/\\/g, '/') : '';
    if (!file || file.startsWith('..') || path.isAbsolute(file)) {
      void vscode.window.showWarningMessage('Gitardo: open a file of this repository to see its history.');
      return;
    }
    await vscode.commands.executeCommand('gitardo.graph.focus');
    await this.setFilter({ kind: 'path', path: file });
  }

  /**
   * Shows the commits that changed some lines of the file in the active editor: the ones selected, or the line of the
   * cursor. Git follows them back from the commit checked out, so they are first found there when the file in the
   * editor is not as committed.
   */
  async lineHistory(): Promise<void> {
    const editor = vscode.window.activeTextEditor;
    if (!(await this.host.requireState())) return;
    const uri = editor?.document.uri;
    if (uri?.scheme === 'file') await this.host.showRepoOf(uri);
    const repo = this.host.followers.repo;
    if (!repo) return;
    const file = editor && uri?.scheme === 'file' ? path.relative(repo.root, uri.fsPath).replace(/\\/g, '/') : '';
    if (!editor || !file || file.startsWith('..') || path.isAbsolute(file)) {
      void vscode.window.showWarningMessage('Gitardo: open a file of this repository to see the history of its lines.');
      return;
    }
    const { document, selection } = editor;
    // A selection that ends at the start of a line leaves that line out, as when whole lines are selected; and the empty
    // line after the last line break is no line for git.
    const last = selection.end.line > selection.start.line && selection.end.character === 0 ? selection.end.line - 1 : selection.end.line;
    const count = Math.max(1, document.lineAt(document.lineCount - 1).text === '' ? document.lineCount - 1 : document.lineCount);
    const selected = { start: Math.min(selection.start.line + 1, count), end: Math.min(last + 1, count) };
    // A file renamed in the index has its old name in the commit checked out.
    const committedAs = this.host.followers.state?.workingTree.staged.find((change) => change.path === file)?.originalPath ?? file;
    const found = await reads.committedLines(repo, committedAs, selected, document.getText());
    if (found === 'file') {
      void vscode.window.showInformationMessage(`Gitardo: ${path.posix.basename(file)} is in no commit yet, so its lines have no history.`);
      return;
    }
    if (found === 'lines') {
      const one = selected.start === selected.end;
      void vscode.window.showInformationMessage(
        `Gitardo: ${one ? 'this line was' : 'these lines were'} added after the last commit, and ${one ? 'has' : 'have'} no history yet.`,
      );
      return;
    }
    const { commit, ...lines } = found;
    const moved = lines.start !== selected.start || lines.end !== selected.end;
    await vscode.commands.executeCommand('gitardo.graph.focus');
    await this.setFilter({ kind: 'lines', path: committedAs, ...lines, commit, ...(moved ? { editor: selected } : {}) });
  }

  /** Shows the commits that changed anything in a folder. */
  async folderHistory(uri: vscode.Uri | undefined): Promise<void> {
    if (!(await this.host.requireState())) return;
    if (uri?.scheme === 'file') {
      // The root folder of a repository is not in the repository: it is the repository.
      const own = this.host.repos.find((repo) => samePath(repo.root, uri.fsPath));
      if (own) await this.host.selectRepo(own.root);
      else await this.host.showRepoOf(uri);
    }
    if (!this.host.followers.repo) return;
    const folder = uri?.scheme === 'file' ? path.relative(this.host.followers.repo.root, uri.fsPath).replace(/\\/g, '/') : '..';
    if (folder.startsWith('..') || path.isAbsolute(folder)) {
      void vscode.window.showWarningMessage('Gitardo: choose a folder of this repository to see its history.');
      return;
    }
    await vscode.commands.executeCommand('gitardo.graph.focus');
    // The root folder of a repository has all of its history.
    await this.setFilter(folder ? { kind: 'path', path: folder, folder: true } : null);
  }

  /**
   * Selects a commit in the graph of a group, bringing it into sight first: the panel's, or a tab's. `hash` is the full
   * one.
   */
  async reveal(hash: string, on = this.host.followers): Promise<void> {
    if (!isHash(hash)) return;
    if (on.panel) on.panel.reveal();
    else await vscode.commands.executeCommand('gitardo.graph.focus');
    await this.include(hash, on);
    // Only now: a view that loads in the meantime would be told of a commit it has not been sent yet.
    on.pendingReveal = hash;
    this.flushPending(on);
  }

  /**
   * Makes sure a commit is among the ones a group lists. The history is loaded as far back as it is; what narrows the
   * list gives way if it leaves the commit out; and a commit that no branch or tag reaches any more, such as one found
   * in the reflog, is shown with its own history.
   */
  private async include(hash: string, on: ViewGroup): Promise<void> {
    await this.host.whenLocated();
    const repo = on.repo;
    if (!repo || on.state?.rows.some((row) => row.hash === hash)) return;
    const shown = on.filter;
    const scope = on.scope;
    let index = await this.positionOf(on, repo, hash, shown, scope);
    if (index < 0 && shown) {
      on.filter = null;
      index = await this.positionOf(on, repo, hash, null, scope);
    }
    // On a branch that is not among the ones shown: they are all shown again.
    if (index < 0 && scope) {
      on.scope = null;
      index = await this.positionOf(on, repo, hash);
    }
    if (index < 0) {
      on.filter = { kind: 'ref', ref: hash, lost: true };
      index = 0;
    }
    const page = pageSize();
    const pages = Math.floor(index / page) + 1;
    const same = on.filter === shown && on.scope === scope;
    on.limit = Math.max(same ? on.limit : page, pages * page);
    await this.host.reload(on, same ? 'more' : 'filter');
    // The read gives its first page at once: the commit is shown once the pages that have it are read.
    if (on.repo === repo && on.state) {
      await on.graph.reached(repo.root, pages * page).catch(() => {});
      on.showPages();
    }
  }

  // The graph read by the engine (stage 7): the position of a commit and the matches of a search.

  /** How many commits come before one in the history listed with a filter and a scope, or -1 when it is not there. */
  private async positionOf(on: ViewGroup, repo: Repository, hash: string, filter: Filter | null = null, scope: Scope | null = null): Promise<number> {
    if (!isHash(hash)) return -1;
    const client = reads.client();
    try {
      const history = await historyOf(client, repo.root, filter, scope, on.state?.head.commit ?? null, firstParent());
      return await positionOnEngine(client, repo.root, hash, history);
    } catch (error) {
      // A history git cannot list has no commit in it.
      if (error instanceof EngineError && error.code === ErrorCode.gitFailed) return -1;
      throw error;
    }
  }

  /** The commits of the history listed that a search finds, newest first; it throws when git refuses the search. */
  private async matchesOf(on: ViewGroup, repo: Repository, search: Search): Promise<{ hashes: string[]; more: boolean }> {
    const client = reads.client();
    const history = await historyOf(client, repo.root, on.filter, on.scope, on.state?.head.commit ?? null, firstParent());
    return matchesOnEngine(client, repo.root, search, history);
  }

  /** Selects, in the view that asked, the commit that a hash or anything else git understands stands for. */
  async goTo(revision: unknown, asker: vscode.Webview, on: ViewGroup): Promise<void> {
    const repo = on.repo;
    if (!repo || typeof revision !== 'string' || !revision.trim()) return;
    const hash = await reads.resolve(repo, revision.trim());
    if (!hash) {
      // A few characters can be the start of more than one hash, which git refuses to choose between.
      void vscode.window.showWarningMessage(`Gitardo: "${revision.trim()}" is not a commit of this repository, or is the start of several.`);
      return;
    }
    await this.include(hash, on);
    this.host.post({ type: 'reveal', hash }, asker);
  }

  /** Asks for a hash or a revision, and shows its commit in the graph of a group. */
  async goToCommit(on = this.host.followers): Promise<void> {
    const repo = (await this.host.requireState(on)) && on.repo;
    if (!repo) return;
    const commit = await askRevision(repo, 'Go to commit');
    if (commit) await this.reveal(commit.hash, on);
  }

  /** Tells a view which commits a search finds in the history its group shows, for it to highlight them where they are. */
  async find(search: Search, asker: vscode.Webview, on: ViewGroup): Promise<void> {
    const repo = on.repo;
    const kinds = ['message', 'author', 'pickaxe', 'regex'];
    if (!repo || !kinds.includes(search?.kind) || typeof search.query !== 'string' || !search.query) return;
    try {
      // A list of results gives way to the graph, in which the new ones are to be shown.
      if (on.filter && 'query' in on.filter) await this.setFilter(null, on);
      const found = await this.matchesOf(on, repo, { kind: search.kind, query: search.query });
      this.host.post({ type: 'found', search, ...found }, asker);
    } catch (error) {
      const text = failureText(error);
      this.host.log.error(text);
      void vscode.window.showErrorMessage(`Gitardo: the search failed. ${text.split('\n')[0]}`);
      this.host.post({ type: 'found', search, hashes: [], more: false, failed: true }, asker);
    }
  }

  /** The views of a group that list commits. */
  private graphViews(on: ViewGroup): vscode.Webview[] {
    return [...this.host.views].filter(([view, kind]) => showsGraph(kind) && this.host.groupOf(view) === on).map(([view]) => view);
  }

  /** Selects a commit in the views of a group that list commits and are there, without opening any or taking the focus. */
  show(hash: string, on = this.host.followers): void {
    if (!isHash(hash)) return;
    for (const view of this.graphViews(on)) this.host.post({ type: 'reveal', hash }, view);
  }

  /**
   * Brings out the history of a commit in the views of the graph of a group, or no longer, while the pointer rests on
   * its branch.
   */
  trace(hash: string | null, on: ViewGroup): void {
    if (hash !== null && !isHash(hash)) return;
    for (const view of this.graphViews(on)) this.host.post({ type: 'trace', hash }, view);
  }

  /** Hands over what was waiting for a graph view of a group to be there. */
  flushPending(on: ViewGroup): void {
    const lists = this.graphViews(on);
    if (on.pendingReveal && lists.length) {
      for (const view of lists) this.host.post({ type: 'reveal', hash: on.pendingReveal }, view);
      on.pendingReveal = undefined;
    }
  }
}

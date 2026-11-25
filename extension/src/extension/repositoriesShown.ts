/**
 * Which repository each group of views shows: the panel and the side bar, chosen in Gitardo or in VS Code's Repositories
 * view and remembered for the workspace, and each tab of the graph in the editor, which keeps its own; a worktree or a
 * submodule of one, for as long as the window lasts; and what a repository left keeps of what it showed.
 */
import * as fs from 'node:fs';
import * as path from 'node:path';
import * as vscode from 'vscode';
import { reads } from '../engine/reads';
import { samePath } from '../shared/paths';
import type { Filter, RepoInfo, Scope } from '../shared/protocol';
import { submoduleFolder } from '../shared/submodules';
import type { ControllerHost } from './controllerHost';
import type { KeptRepository } from './keptRepositories';
import { type RepositorySession, ViewGroup, errorText, keepOpen } from './repositorySession';
import { sameHistory, sharesHistory, tabTitle } from './sessions';

/** The repository of the workspace the panel and the side bar show, which they show again when the window opens. */
export const REPO_KEY = 'gitardo.repository';
/** The branches chosen for the graph of each repository, by its folder. */
export const SCOPE_KEY = 'gitardo.scope';

/** The kind of the tabs of the graph in the editor, which VS Code brings back after a reload (`repo.keep-tabs`). */
export const GRAPH_TAB = 'gitardo.graphEditor';

/** What a tab of the graph showed, which it is opened with again after a reload: its search and its branches shown. */
export interface TabShown {
  filter?: Filter | null;
  scope?: Scope | null;
}

/** A folder as the repositories are told apart by, whatever its slashes and its case. */
const key = (root: string) => root.replace(/\\/g, '/').toLowerCase();

/** Points the groups of views at the repositories they show, and opens the tabs of the graph. */
export class RepositoriesShown {
  /** The folder the worktrees of each repository of the workspace share, as the engine said when it looked for them. */
  private readonly sharedFolders = new Map<string, string>();

  constructor(
    private readonly host: Pick<
      ControllerHost,
      | 'allGroups'
      | 'attach'
      | 'chosen'
      | 'followers'
      | 'goneTab'
      | 'groupHost'
      | 'homeOf'
      | 'kept'
      | 'located'
      | 'log'
      | 'memento'
      | 'reload'
      | 'repos'
      | 'running'
      | 'sessions'
      | 'setState'
      | 'views'
      | 'whenLocated'
    >,
  ) {}

  /**
   * The repositories of the workspace were looked for: the folder each shares with its worktrees is known from then on,
   * and is not asked again when the repository is first shown (`repo.read-first`).
   */
  found(repositories: readonly { folder: string; commonFolder: string | null }[]): void {
    this.sharedFolders.clear();
    for (const { folder, commonFolder } of repositories) if (commonFolder) this.sharedFolders.set(key(folder), path.resolve(commonFolder));
  }

  /**
   * Switches a group to another repository of the workspace: the panel and the side bar unless another is given, whose
   * choice is remembered for this workspace; or to another worktree of the repository it shows, for as long as the window
   * lasts: its changes, commits and branches are worked on from there.
   */
  async selectRepo(root: string, on = this.host.followers): Promise<void> {
    const shown = on.repo?.root;
    if (this.host.running || (shown && samePath(root, shown))) return;
    const repo = this.host.repos.find((each) => samePath(each.root, root));
    if (repo) {
      if (on === this.host.followers) await this.host.memento.update(REPO_KEY, repo.root);
      await this.point(on, repo.root);
      this.chose(on, repo.root);
      return;
    }
    const state = on.state;
    // A worktree of it, or one of its submodules initialized since the repositories of the workspace were looked for.
    const folder = !state
      ? undefined
      : (state.worktrees.find((each) => samePath(each.folder, root) && !each.isMissing)?.folder ??
        state.submodules.filter((each) => each.checkout?.commit).map((each) => submoduleFolder(each, state)).find((each) => samePath(each, root)));
    if (folder) {
      // The way back is the repository of the workspace that was shown, or the one that was before another worktree.
      on.home = this.host.repos.find((each) => shown && samePath(each.root, shown)) ?? on.home;
      await this.point(on, folder);
      this.chose(on, folder);
      return;
    }
    // A worktree of another repository of the workspace, such as one chosen in VS Code's Repositories view.
    const owner = await this.worktreeOwner(root);
    if (!owner || this.host.running) return;
    on.home = owner;
    await this.point(on, root);
    this.chose(on, root);
  }

  /** A group shows what was chosen: what the panel shows is selected in VS Code's Repositories view, a tab says it in its title. */
  private chose(on: ViewGroup, root: string): void {
    if (on === this.host.followers) this.host.chosen(root);
    else this.retitle(on);
  }

  async pickRepo(on: ViewGroup): Promise<void> {
    await this.host.whenLocated();
    const state = on.state;
    // Showing a worktree, there is always the repository of the workspace to go back to.
    if (this.host.repos.length < 2 && !state?.home) {
      void vscode.window.showInformationMessage(
        this.host.repos.length ? 'Gitardo: this workspace has a single repository.' : 'Gitardo: no Git repository in this workspace.',
      );
      return;
    }
    const current = on.repo?.root;
    const choice = await vscode.window.showQuickPick(
      [
        ...this.host.repos.map((repo) => ({ label: repo.name, description: repo.root === current ? 'current' : undefined, detail: repo.root, repo })),
        ...(state?.home && current ? [{ label: state.name, description: 'worktree, current', detail: current, repo: { root: current, name: state.name, depth: 0 } }] : []),
      ],
      { title: 'Repository shown in Gitardo', matchOnDetail: true },
    );
    if (choice) await this.selectRepo(choice.repo.root, on);
  }

  /**
   * Shows the repository or worktree chosen in VS Code's Repositories view, once Gitardo has looked for the
   * repositories of the workspace: until then, it reads nothing for it.
   */
  async followSourceControl(root: string): Promise<void> {
    if (!this.host.located) return;
    await this.host.whenLocated();
    await this.selectRepo(root);
  }

  /** In a workspace with several repositories a file decides which one to show: the innermost one that contains it. */
  async showRepoOf(uri: vscode.Uri): Promise<void> {
    const contains = (root: string) => {
      const relative = path.relative(root, uri.fsPath);
      return relative !== '' && !relative.startsWith('..') && !path.isAbsolute(relative);
    };
    const owner = this.host.repos.filter((repo) => contains(repo.root)).sort((a, b) => b.root.length - a.root.length)[0];
    if (owner) await this.selectRepo(owner.root);
  }

  /** The repository of the workspace that a folder is a worktree of, by the git directory they share. */
  private async worktreeOwner(root: string): Promise<RepoInfo | undefined> {
    const commonDir = (folder: string) => reads.commonDir({ root: folder }).catch(() => undefined);
    const shared = await commonDir(root);
    if (!shared) return undefined;
    const theirs = await Promise.all(this.host.repos.map((repo) => commonDir(repo.root)));
    return this.host.repos.find((_, index) => theirs[index] && samePath(theirs[index], shared));
  }

  /**
   * Points a group of views at a repository, a worktree or a submodule, or at none: it goes on the session of that
   * repository, made when no group showed it, and the session it leaves is let go when it was the last group there.
   * With `again`, a group already there starts again, as when the repositories of the workspace are looked for again.
   */
  async point(group: ViewGroup, root: string | undefined, again = false, shown: TabShown = {}): Promise<void> {
    const before = group.session;
    if (before && root && samePath(before.root, root)) {
      if (!again) return;
      this.host.sessions.leave(group);
    }
    // A worktree of the repository left, or that repository from one of its worktrees, shares its commits.
    const shares = !!before && !!root && sharesHistory(group.state, before.root, root);
    const narrowed = { filter: group.filter, scope: group.scope };
    // A repository left reads no more pages: it reads its commits again once it is shown (Q437).
    if (before && !shares) group.graph.stop(before.root);
    if (!root) {
      this.host.sessions.leave(group);
      group.session = undefined;
      group.reset(null);
      return this.host.setState(group, null);
    }
    const { session, made } = this.host.sessions.move(group, root);
    group.session = session;
    const kept = made && keepOpen() ? this.host.kept.take(root) : undefined;
    group.reset(this.host.memento.get<Record<string, Scope>>(SCOPE_KEY, {})[root] ?? null);
    // A tab brought back after a reload shows what its page kept, not what the repository kept when it was left.
    const ownShown = shown.filter !== undefined || shown.scope !== undefined;
    if (kept && !ownShown) return this.comeBack(group, session, kept);
    kept?.watcher?.dispose();
    if (shown.filter) group.filter = shown.filter;
    if (shown.scope !== undefined) group.scope = shown.scope;
    // A repository chosen that was not watched meanwhile counts every worktree again, as **Refresh** does (Q1744).
    if (made) session.everything = true;
    // The folder its worktrees share is the one of the worktree left, or the one found with the repositories.
    if (made) session.commonFolder ??= (shares ? before?.commonFolder : undefined) ?? this.sharedFolders.get(key(root));
    // A repository that shows nothing yet is read in two steps (`repo.read-first`): the views wait for the first, which
    // comes with the first rows, and the rest of its state comes behind them.
    let whole: Promise<void> | undefined;
    const read = () => {
      if (!made) return session.refresh();
      const steps = session.open();
      whole = steps.whole;
      return steps.first;
    };
    // A worktree of the repository left asks for the same history when nothing narrows either and both show the same
    // branches: it keeps the rows the views have, which are its own, and reads only what differs in it
    // (`repo.share-history`). The views are not told to wait for commits (Q1743).
    if (shares && sameHistory(narrowed, group)) {
      session.commitsTaken = made;
      await read();
      return whole;
    }
    // The commits of the repository before stay in the views until those of this one come.
    await this.host.reload(group, 'filter', read);
    return whole;
  }

  /**
   * A session no group shows any more. With `gitardo.repositories.keepOpen` on its repository keeps what the last group
   * showed of it, and its watchers, which from now on only mark it out of date (`repo.keep-open`, `repo.watch-hidden`);
   * else it is no longer watched.
   */
  letGo(session: RepositorySession, last: ViewGroup): void {
    // A change seen just before is the repository left's, not the next one's.
    const pending = session.takePending();
    const state = last.state;
    // A folder that is no longer there, as a worktree removed, has nothing to come back to.
    if (!state || !samePath(state.root, session.root) || !keepOpen() || !fs.existsSync(session.root)) return session.dispose();
    this.host.kept.keep(session.root, {
      state,
      filter: last.filter,
      scope: last.scope,
      limit: last.limit,
      hosting: session.hosting,
      pulls: session.pulls,
      pullsAt: session.pullsAt,
      commonFolder: session.commonFolder,
      watcher: session.watcher,
      changed: pending,
    });
    session.dispose(true);
  }

  /**
   * Shows at once what a repository showed when it was left, and reads it again behind that when it changed meanwhile,
   * as **Refresh** would, the status bar saying why (`repo.keep-open`, `repo.watch-hidden`).
   */
  private async comeBack(group: ViewGroup, session: RepositorySession, kept: KeptRepository): Promise<void> {
    group.filter = kept.filter;
    group.scope = kept.scope;
    group.limit = kept.limit;
    session.hosting = kept.hosting;
    session.pulls = kept.pulls;
    session.pullsAt = kept.pullsAt;
    session.commonFolder = kept.commonFolder;
    session.watcher = kept.watcher;
    // The repositories of the workspace may have changed while it was not shown.
    this.host.setState(group, { ...kept.state, repos: this.host.repos, home: this.host.homeOf(group, kept.state.root), keepOpen: true });
    if (kept.changed) {
      // It was watched while it was not shown: the worktrees are not all counted again, nor every commit read (Q1744).
      session.cause = kept.changed;
      await session.detectHosting().catch((error) => this.host.log.warn(`Could not read the remotes of ${session.root}: ${errorText(error)}`));
      await session.refresh();
    } else {
      session.markFresh();
      if (this.host.views.size) void session.refreshPulls(false);
    }
  }

  /**
   * Opens a tab of the graph in the editor on a repository, a worktree or a submodule: the one the panel shows unless
   * another is given (`repo.tab-repository`). The tab keeps it, whatever the other views show, and is read with them.
   * `panel` is a tab VS Code brings back after a reload, with what it showed (`repo.keep-tabs`).
   */
  async openTab(root?: string, panel?: vscode.WebviewPanel, shown: TabShown = {}): Promise<void> {
    await this.host.whenLocated();
    if (panel && root && !fs.existsSync(root)) return this.host.goneTab(panel, root);
    const tab =
      panel ??
      vscode.window.createWebviewPanel(GRAPH_TAB, 'Gitardo', vscode.ViewColumn.Active, {
        retainContextWhenHidden: true,
      });
    const group = new ViewGroup(this.host.groupHost, tab);
    const attachment = this.host.attach(tab.webview, 'graph', { tab: 'graph' }, group);
    // The panel and the side bar follow the tab that comes into focus, not one opened (`repo.follow-tab`): VS Code also
    // says a tab is active as it is first shown, which is not a change.
    let active = tab.active;
    tab.onDidChangeViewState(({ webviewPanel }) => {
      const came = webviewPanel.active && !active;
      active = webviewPanel.active;
      if (came) void this.followTab(group);
    });
    tab.onDidDispose(() => {
      attachment.dispose();
      this.host.sessions.leave(group);
      group.session = undefined;
      group.dispose();
    });
    root ??= this.host.followers.repo?.root;
    if (!root) return this.host.setState(group, null);
    group.home = await this.homeFor(root);
    // A choice of the panel's is the panel's: the tab starts with the history as the panel shows it, read for itself.
    await this.point(group, root, false, shown);
    this.retitle(group);
  }

  /**
   * The panel and the side bar show the repository of a tab of the graph that came into focus, as if it were chosen
   * there, and VS Code's Repositories view selects it (`repo.follow-tab`). The repository chosen stays the one kept in
   * the workspace state, which they show again when the window opens; while an operation runs they stay where they are,
   * as choosing would.
   */
  private async followTab(group: ViewGroup): Promise<void> {
    const root = group.session?.root;
    if (!root || this.host.running || (this.host.followers.session && samePath(this.host.followers.session.root, root))) return;
    this.host.followers.home = group.home;
    await this.point(this.host.followers, root);
    this.chose(this.host.followers, root);
  }

  /** Says in the title of the tab of a group what it shows: `Gitardo · api`, `Gitardo · api › feature/login` (Q1520). */
  retitle(group: ViewGroup): void {
    if (group.panel) group.panel.title = tabTitle(group.session?.root, this.host.repos, group.home);
  }

  /**
   * The repository of the workspace a worktree or a submodule outside its list is of, to go back to: the one a group
   * shows it from, or the one it shares its folder of git with; none for a folder of the list.
   */
  private async homeFor(root: string): Promise<RepoInfo | undefined> {
    if (this.host.repos.some((each) => samePath(each.root, root))) return undefined;
    for (const group of this.host.allGroups()) {
      const state = group.state;
      if (!state) continue;
      if (samePath(state.root, root)) return group.home;
      const member =
        state.worktrees.some((each) => samePath(each.folder, root)) ||
        state.submodules.some((each) => each.checkout?.commit && samePath(submoduleFolder(each, state), root));
      if (member) return this.host.repos.find((each) => samePath(each.root, state.root)) ?? group.home;
    }
    return this.worktreeOwner(root);
  }

  /**
   * **Gitardo: Open Repository in a New Tab**: asks which repository, worktree or submodule, as **Gitardo: Select
   * Repository** lists them, and opens a tab of the graph on it.
   */
  async pickTab(): Promise<void> {
    await this.host.whenLocated();
    if (!this.host.repos.length) {
      void vscode.window.showInformationMessage('Gitardo: no Git repository in this workspace.');
      return;
    }
    const choice = await vscode.window.showQuickPick(
      this.host.repos.map((repo) => ({ label: repo.name, detail: repo.root, repo })),
      { title: 'Open Repository in a New Tab', matchOnDetail: true },
    );
    if (choice) await this.openTab(choice.repo.root);
  }
  /**
   * The scope as git took it, without the branches deleted since they were chosen, is the one in use in the group and
   * the one remembered for the repository, which the next group to show it starts with.
   */
  settleScope(on: ViewGroup, root: string, scope: Scope | null): void {
    on.scope = scope;
    const all = this.host.memento.get<Record<string, Scope>>(SCOPE_KEY, {});
    if (JSON.stringify(all[root] ?? null) === JSON.stringify(scope)) return;
    const others = Object.fromEntries(Object.entries(all).filter(([other]) => other !== root));
    void this.host.memento.update(SCOPE_KEY, scope ? { ...others, [root]: scope } : others);
  }
}

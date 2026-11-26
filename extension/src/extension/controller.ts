import * as path from 'node:path';
import * as vscode from 'vscode';
import { ThemeService } from '../flow/theme';
import type { Hosting } from '../shared/hosting';
import { samePath } from '../shared/paths';
import { offerCommitGraph } from './commitGraphOffer';
import { GRAPH_COLUMNS, showsGraph } from '../shared/protocol';
import { sortRefs } from '../shared/refs';
import type { TagMarks } from '../shared/tagMarks';
import { groupRepositories } from '../shared/repositoryGroups';
import { Activity } from './activity';
import { KeptRepositories } from './keptRepositories';
import { RepositoriesView } from './repositoriesView';
import { FileIcons } from './fileIcons';
import type { Action, BranchSort, Filter, GraphColumn, ToViews, Loading, RepoInfo, RepoState, ViewKind, WebviewMessage } from '../shared/protocol';
import { AGENTS_WINDOW, LAST_INPUTS, hostAction, worktreesFolder } from './actions';
import { ConflictReviews, autoResolve, resolveWithAI } from './conflictReview';
import { setupGiven } from './worktreeSettings';
import { EngineError, ErrorCode } from '../engine/connection';
import { type EngineRun, engineRuns } from '../engine/actions';
import type { FollowUp, Target } from '../engine/messages';
import { type HostHooks, VscodeAsker, type WindowEngine, sayEngineMissing } from '../engine/vscode';
import { type Repository, reads, repositoryName } from '../engine/reads';
import type { NotDownloaded } from './content';
import { LargeFilesTabs } from './largeFiles';
import { MenuContext } from './menuContext';
import { LFS_WORDS, notDownloadedMessage } from '../shared/lfs';
import { completed, graphState, readCommits } from '../engine/graph';
import { RangeDiffTab } from './rangeDiffTab';
import { type GroupHost, REFRESH_DELAY, RepositorySession, type SessionHost, ViewGroup, errorText, firstParent, keepOpen, pageSize } from './repositorySession';
import { Sessions, tabTitle } from './sessions';
import { RepositoriesAhead } from './repositoriesAhead';
import { type Sent, rowsKey, stateFor } from './sentParts';
import { ROWS_PIECE, RowsBehind } from './rowsBehind';
import { partsOf } from '../shared/stateParts';
import { Commands, defaultPull } from './commands';
import { ChangesSeen } from './changesSeen';
import { GitFoldersHeard } from './gitFoldersHeard';
import { CommitTabs } from './commitTabs';
import { CompareTabs } from './compareTabs';
import type { ControllerHost } from './controllerHost';
import { FileDiffs } from './fileDiffs';
import { HistoryShown } from './historyShown';
import { REPO_KEY, RepositoriesShown } from './repositoriesShown';
import { RunFollowUps } from './runFollowUps';
import { UnsavedFiles } from './unsavedFiles';
import { ViewMessages } from './viewMessages';
import { WorktreeSetupTab } from './worktreeSetupTab';

/**
 * What is said of an operation asked while another runs, with its title when Gitardo knows it, as Gitardo Desktop says
 * it (`repo.switch-tab`).
 */
const runningElsewhere = (title: string | null, repository: string) =>
  `Gitardo: ${title ?? 'another operation'} is running in ${repository}. Try again once it ends.`;

const hasCommand = async (id: string) => (await vscode.commands.getCommands(true)).includes(id);

/** What the graph shows of each commit beside its message, as `gitardo.graph.columns` asks: all of it when that is not a list. */
function graphColumns(): GraphColumn[] {
  const wanted = vscode.workspace.getConfiguration('gitardo').get('graph.columns');
  return Array.isArray(wanted) ? GRAPH_COLUMNS.filter((column) => wanted.includes(column)) : GRAPH_COLUMNS;
}

/**
 * How branches and tags are listed, as `gitardo.branches.sortOrder` asks, or else as VS Code's `git.branchSortOrder`
 * does when it was set. Its default is by date, and Gitardo lists them by name unless asked otherwise.
 */
function branchSort(): BranchSort {
  const set = vscode.workspace.getConfiguration('gitardo').inspect<unknown>('branches.sortOrder');
  const ours = set?.workspaceFolderValue ?? set?.workspaceValue ?? set?.globalValue;
  if (ours === 'name' || ours === 'date') return ours;
  const git = vscode.workspace.getConfiguration('git').inspect<unknown>('branchSortOrder');
  return (git?.workspaceFolderValue ?? git?.workspaceValue ?? git?.globalValue) === 'committerdate' ? 'date' : 'name';
}

/**
 * Serves every Gitardo webview, wherever the user has placed it: the views, the commands, the operation running, and
 * which repository each group of views shows, whose session reads and watches it (D89).
 */
export class Controller implements vscode.Disposable {
  private located = false;
  /** The search for the repositories of the workspace and the first read of the one shown, while they run. */
  private locating?: Promise<void>;
  /** How many operations were started, to tell a state that was read across one. */
  private operations = 0;
  /** The operation `runOnEngine` runs, which another one asked meanwhile is refused for. */
  private operation: { feature: string; root: string; asker: VscodeAsker } | undefined;
  /** Whether an operation runs: nothing is read meanwhile. */
  private get running(): boolean {
    return this.operation !== undefined;
  }
  /** Gitardo's engine, which reads the repository and runs every operation on it; without it, the views say so (D49). */
  private engine?: WindowEngine;
  /** Forgets the token of a GitLab that the GitLab refused, so that signing in asks for another (`hosting.token`). */
  private forgetToken?: (host: string) => Promise<void>;
  /** Every repository of the workspace. */
  private repos: RepoInfo[] = [];
  /** The repositories left that keep what they showed, with `gitardo.repositories.keepOpen` on. */
  private readonly kept = new KeptRepositories();
  /**
   * The views of the panel and of the side bar, which show the repository chosen, and the views opened in the editor
   * that show what they do.
   */
  private readonly followers: ViewGroup;
  /** What every group of views asks of the controller. */
  private readonly groupHost: GroupHost;
  /** The session of each repository a group of views shows, and which one each group is on. */
  private readonly sessions: Sessions<RepositorySession, ViewGroup>;
  /** The group each view that shows a repository is in; the tabs of a commit, a comparison and the like are in none. */
  private readonly groups = new Map<vscode.Webview, ViewGroup>();
  /** The tabs of the graph brought back on a folder that is no longer there, which only say so (`repo.keep-tabs`). */
  private readonly goneTabs = new Map<vscode.Webview, vscode.WebviewPanel>();
  /** The rows of the Repositories view, and what the engine follows for them while the view is open (`repo.tabs`). */
  readonly repositoriesView: RepositoriesView;
  /** Whether this VS Code has the Agents window, which is looked up once: it comes and goes with VS Code itself. */
  private agentsWindow?: Promise<boolean>;
  private readonly views = new Map<vscode.Webview, ViewKind>();
  /** What the engine says of the git folders it watches, and which repositories it is told a view shows (`repo.watch-git`). */
  private readonly gitFolders: GitFoldersHeard;
  /** What the last **Check tags on `<remote>`** of each repository said, by its folder, until VS Code is closed (`tag.remote-marks`). */
  private readonly tagMarks = new Map<string, TagMarks>();
  /** Which repository each group of views shows, and the tabs of the graph in the editor. */
  readonly repositories: RepositoriesShown;
  /** What the graph of each group shows and selects: its search, its branches shown, the commit selected. */
  readonly history: HistoryShown;
  /** The commands that ask in VS Code before the engine runs them, or after. */
  readonly commands: Commands;
  /** What VS Code does around each operation the engine runs. */
  private readonly runFollowUps: RunFollowUps;
  /** What the views ask. */
  private readonly messages: ViewMessages;
  /** The tabs of the commits. */
  readonly commitTabs: CommitTabs;
  /** The tabs of the comparisons. */
  readonly compareTabs: CompareTabs;
  /** The diffs and the files the views open in an editor. */
  private readonly diffs: FileDiffs;
  private readonly pinned = new Map<vscode.Webview, Repository>();
  /** The pages that color code, which are sent the colors of the theme again when it changes. */
  private readonly colored = new Set<vscode.Webview>();
  private readonly themes = new ThemeService();
  /** The pages that show files, which are sent the icons of the file icon theme again when it changes. */
  private readonly iconed = new Set<vscode.Webview>();
  private readonly fileIcons: FileIcons;
  /** The folders the pages may load files from: the build, and the extensions with icon themes. */
  private roots: vscode.Uri[];
  /** The tab of what new worktrees get. */
  readonly worktreeSetup: WorktreeSetupTab;
  /** The tab of the review of the conflicts. */
  readonly conflictReviews: ConflictReviews;
  /** The views with a search box that have the keyboard: one at most, but each says so on its own. */
  private readonly focused = new Set<vscode.Webview>();
  /** What each view was last sent of the state: the key of each part, and how many commits (D88). */
  private readonly sent = new WeakMap<vscode.Webview, Sent>();
  /** The views that are sent the commits of a long history behind its first page, a piece at a time (QA6). */
  private readonly behind = new RowsBehind<vscode.Webview>((view) => {
    const state = this.views.has(view) ? this.groupOf(view)?.state : null;
    if (state) this.post({ type: 'state', state }, view);
  });
  /** The files with edits not saved in an editor. */
  private readonly unsaved: UnsavedFiles;
  /** The repositories of the list read ahead, while no view shows them (`repo.watch-hidden`). */
  private readonly ahead: RepositoriesAhead;
  /** What a change seen asks for: a read of a repository shown, or a repository kept marked out of date. */
  private readonly changesSeen: ChangesSeen;
  /** What the menus of VS Code are told of the repository the panel shows. */
  private readonly menus = new MenuContext();
  /** Has the tabs of the scheme `gitardo` said not downloaded read again once a download ran in a repository (M47). */
  readAgainAfterDownload: (root: string) => void = () => undefined;
  /** The Large files tabs, one for each repository (`lfs.page`). */
  private readonly largeFiles = new LargeFilesTabs({
    attach: (webview, root) => this.attach(webview, 'largeFiles', { root }),
    post: (message, webview) => this.post(message, webview),
    run: (runs, repo) => this.runOnEngine(runs, repo),
    name: (root) => this.repositoryNamed(root),
    held: (root) => this.heldFor(root),
    downloaded: (root) => this.readAgainAfterDownload(root),
    error: (text) => this.log.error(text),
  });
  private readonly stateEmitter = new vscode.EventEmitter<RepoState | null>();
  private readonly readEmitter = new vscode.EventEmitter<RepoState>();
  private readonly subscriptions: vscode.Disposable[];
  /** What Gitardo is doing that takes time, in the status bar. */
  readonly activity: Activity;
  /** The tab of what a rewrite changed, commit by commit (`range-diff.page`). */
  private readonly rangeDiffTab: RangeDiffTab;

  /** Fires whenever the state of the repository the panel and the side bar show has been reloaded. */
  readonly onDidChangeState = this.stateEmitter.event;
  /** Fires whenever a repository a view shows has been read, the panel's or a tab's. */
  readonly onDidReadRepository = this.readEmitter.event;

  /** The repository state the panel and the side bar were sent last, for a view that opens after it was read. */
  get shownState(): RepoState | null {
    return this.followers.state;
  }
  /** The repository the panel and the side bar show, which the commands of the Command Palette act on (Q1522). */
  private get repo(): Repository | undefined {
    return this.followers.repo;
  }
  /** The state of the repository the panel and the side bar show. */
  private get state(): RepoState | null {
    return this.followers.state;
  }
  // The search, how far the history is read, the hosting service and the read under way of what the panel shows, which
  // the end-to-end suite looks at.
  /** The search of the panel and the side bar. */
  private get filter(): Filter | null {
    return this.followers.filter;
  }
  /** How many commits the graph of the panel and the side bar reads. */
  private get limit(): number {
    return this.followers.limit;
  }
  private set limit(limit: number) {
    this.followers.limit = limit;
  }
  /** The hosting service of the main remote of the repository the panel and the side bar show. */
  private get hosting(): Hosting | null {
    return this.followers.session?.hosting ?? null;
  }
  private set hosting(hosting: Hosting | null) {
    if (this.followers.session) this.followers.session.hosting = hosting;
  }
  /** The read of the repository the panel and the side bar show, while there is one. */
  private get loading(): Promise<void> | undefined {
    return this.followers.session?.loading;
  }
  private readonly chosenEmitter = new vscode.EventEmitter<string>();
  /** Fires with the root of the repository or worktree chosen to be shown in the panel and the side bar, once they show it. */
  readonly onDidChooseRepo = this.chosenEmitter.event;

  constructor(
    private readonly extensionUri: vscode.Uri,
    private readonly log: vscode.LogOutputChannel,
    /** Remembers which repository of this workspace was chosen, and which branches of each are shown. */
    private readonly memento: vscode.Memento,
  ) {
    // eslint-disable-next-line @typescript-eslint/no-this-alias
    const controller = this;
    const unsaved = () => void this.unsaved.check();
    // Files created, deleted or renamed in the editor: every repository shown is read again, or looked for when none is.
    const schedule = (cause: string) => () => {
      if (!this.sessions.all().length) return this.scheduleRefresh(cause);
      for (const session of this.sessions.all()) session.scheduleRefresh(cause);
    };
    this.fileIcons = new FileIcons(log);
    this.roots = this.resourceRoots();
    this.activity = new Activity(log);
    this.groupHost = {
      post: (group, message) => this.postToGroup(group, message),
      sendRows: (group) => {
        const state = group.state;
        for (const [view, kind] of this.views) if (state && showsGraph(kind) && this.groupOf(view) === group) this.post({ type: 'state', state }, view);
      },
      historyRead: (group) => {
        // The history of a repository shown is read to its end: the others may be read ahead (`repo.watch-hidden`).
        this.ahead.turns.next();
        const ask = (message: string, ...buttons: string[]) => vscode.window.showInformationMessage(message, ...buttons);
        void offerCommitGraph(group.state, { memento: this.memento, ask, write: () => this.action({ kind: 'writeCommitGraph' }, group) });
      },
    };
    this.followers = new ViewGroup(this.groupHost);
    const sessionHost: SessionHost = {
      log,
      activity: this.activity,
      get running() {
        return controller.running;
      },
      get operations() {
        return controller.operations;
      },
      groupsOn: (session) => this.sessions.groupsOn(session),
      hasViews: (session) => this.gitFolders.hasViews(session),
      repositoryNamed: (root) => this.repos.find((each) => samePath(each.root, root))?.name ?? path.basename(root),
      autoResolve: () => autoResolve(),
      complete: (group, read, first) => this.complete(group, read, first),
      stateRead: (group, state) => this.setState(group, state),
      settleScope: (group, root, scope) => this.repositories.settleScope(group, root, scope),
      changedIn: (root, cause, delay) => this.changesSeen.changedIn(root, cause, delay),
      fileChangedIn: (root, file) => this.changesSeen.fileChangedIn(root, file),
      pullsRead: (session) => {
        for (const group of this.sessions.groupsOn(session)) if (group.state) this.setState(group, { ...group.state, pulls: session.pulls });
      },
      forgetToken: (host) => this.forgetToken?.(host) ?? Promise.resolve(),
      readStarts: () => this.ahead.turns.stop(),
      readEnded: () => this.ahead.turns.next(),
    };
    this.sessions = new Sessions<RepositorySession, ViewGroup>({
      make: (root) => new RepositorySession(root, sessionHost),
      letGo: (session, last) => this.repositories.letGo(session, last),
    });
    this.rangeDiffTab = new RangeDiffTab({
      attach: (webview, kind, data) => this.attach(webview, kind, data),
      pin: (webview, repo) => (repo ? this.pinned.set(webview, repo) : this.pinned.delete(webview)),
      track: (title, work) => this.activity.track(title, work),
      post: (message, to) => this.post(message, to),
    });
    this.repositoriesView = new RepositoriesView(log);
    this.gitFolders = new GitFoldersHeard({ log, views: this.views, groups: this.groups, followers: this.followers, sessions: this.sessions, kept: this.kept });
    const host: ControllerHost = {
      log,
      memento,
      activity: this.activity,
      views: this.views,
      sent: this.sent,
      behind: this.behind,
      pinned: this.pinned,
      colored: this.colored,
      iconed: this.iconed,
      themes: this.themes,
      fileIcons: this.fileIcons,
      goneTabs: this.goneTabs,
      repositoriesView: this.repositoriesView,
      get reviewTab() {
        return controller.conflictReviews.tab;
      },
      attach: (webview, kind, data, group) => this.attach(webview, kind, data, group),
      post: (message, to, group) => this.post(message, to, group),
      postToGroup: (group, message) => this.postToGroup(group, message),
      pin: (webview, repo) => (repo ? this.pinned.set(webview, repo) : this.pinned.delete(webview)),
      setFocused: (webview, focused) => this.setFocused(webview, focused),
      goneTab: (panel, root) => this.goneTab(panel, root),
      followers: this.followers,
      groupHost: this.groupHost,
      sessions: this.sessions,
      kept: this.kept,
      get repos() {
        return controller.repos;
      },
      get located() {
        return controller.located;
      },
      get locating() {
        return controller.locating;
      },
      get running() {
        return controller.running;
      },
      tagMarks: this.tagMarks,
      allGroups: () => this.allGroups(),
      groupOf: (webview) => this.groupOf(webview),
      groupShowing: (root) => this.groupShowing(root),
      groupsShowing: (root) => this.groupsShowing(root),
      homeOf: (group, root) => this.homeOf(group, root),
      heldFor: (root) => this.heldFor(root),
      stateOf: (repo) => this.stateOf(repo),
      requireState: (on) => this.requireState(on),
      setState: (group, state) => this.setState(group, state),
      gitFolders: this.gitFolders,
      makeSession: (root) => new RepositorySession(root, sessionHost),
      completeAhead: (session, read) => this.completeFor(session, null, read),
      hostingOf: (root) => this.sessions.find(root)?.hosting,
      markTags: (root, marks) => this.markTags(root, marks),
      whenLocated: () => this.whenLocated(),
      refresh: (on) => this.refresh(on),
      refreshAll: (on) => this.refreshAll(on),
      reload: (on, what, work) => this.reload(on, what, work),
      readForView: (on) => this.readForView(on),
      chosen: (root) => this.chosenEmitter.fire(root),
      scheduleRefresh: (cause, delay) => this.scheduleRefresh(cause, delay),
      run: (runs, repo, hooks, ran, on) => this.runOnEngine(runs, repo, hooks, ran, on),
      action: (action, on) => this.action(action, on),
      resolveWithAI: () => resolveWithAI(),
      showRepoOf: (uri) => this.repositories.showRepoOf(uri),
      selectRepo: (root, on) => this.repositories.selectRepo(root, on),
      pickRepo: (on) => this.repositories.pickRepo(on),
      reveal: (hash, on) => this.history.reveal(hash, on),
      show: (hash, on) => this.history.show(hash, on),
      fileHistory: (uri) => this.history.fileHistory(uri),
      goToCommit: (on) => this.history.goToCommit(on),
      openCommit: (hash, repo) => this.commitTabs.openCommit(hash, repo),
      openRangeDiff: (target, repo) => this.rangeDiffTab.open(target, repo),
      openWorktreeSetup: (on) => this.worktreeSetup.open(on),
      openLargeFiles: (on) => this.openLargeFiles(on),
      reviewConflicts: (paths, ai, on) => this.conflictReviews.reviewConflicts(paths, ai, on),
    };
    this.repositories = new RepositoriesShown(host);
    this.ahead = new RepositoriesAhead(host);
    this.history = new HistoryShown(host);
    this.commands = new Commands(host);
    this.runFollowUps = new RunFollowUps(host);
    this.commitTabs = new CommitTabs(host);
    this.compareTabs = new CompareTabs(host);
    this.diffs = new FileDiffs(host);
    this.worktreeSetup = new WorktreeSetupTab(host);
    this.unsaved = new UnsavedFiles(host);
    this.conflictReviews = new ConflictReviews(host);
    this.changesSeen = new ChangesSeen(host, this.unsaved);
    this.messages = new ViewMessages(host, {
      commands: this.commands,
      commitTabs: this.commitTabs,
      compareTabs: this.compareTabs,
      diffs: this.diffs,
      history: this.history,
      largeFiles: this.largeFiles,
      rangeDiffTab: this.rangeDiffTab,
      repositories: this.repositories,
      unsaved: this.unsaved,
      worktreeSetup: this.worktreeSetup,
    });
    this.subscriptions = [
      this.stateEmitter,
      this.readEmitter,
      this.chosenEmitter,
      this.activity,
      this.repositoriesView,
      this.fileIcons,
      this.fileIcons.onDidChange(() => this.sendFileIcons()),
      vscode.workspace.onDidChangeWorkspaceFolders(() => this.locate()),
      vscode.workspace.onDidSaveTextDocument((document) => {
        unsaved();
        this.changesSeen.saved(document.uri);
      }),
      // An edit, or one undone back to what is saved, and an editor closed without saving.
      vscode.workspace.onDidChangeTextDocument(unsaved),
      vscode.workspace.onDidCloseTextDocument(unsaved),
      vscode.workspace.onDidCreateFiles(schedule('files were created')),
      vscode.workspace.onDidDeleteFiles(schedule('files were deleted')),
      vscode.workspace.onDidRenameFiles(schedule('files were renamed')),
      vscode.window.onDidChangeWindowState((window) => window.focused && this.changesSeen.focusGot()),
      vscode.workspace.onDidChangeConfiguration((event) => {
        const sections = ['gitardo.conflicts', 'gitardo.pull', 'gitardo.worktrees', 'gitardo.branches', 'gitardo.graph', 'gitardo.commit', 'git.branchSortOrder'];
        if (sections.some((section) => event.affectsConfiguration(section))) {
          // They can change what is listed: the commits are read again (Q442), and those that are the same stay (Q781).
          for (const group of this.allGroups()) group.graph.readAgain();
          for (const session of this.sessions.all()) session.scheduleRefresh('the settings changed');
          // The repositories kept were read with the settings before: each is read again once it is shown.
          this.kept.changedAll('the settings changed');
        }
        if (event.affectsConfiguration('gitardo.repositories.keepOpen')) {
          this.changesSeen.keepOpenChanged();
          // Turned off, nothing is read ahead any more; turned on, the repositories of the list are.
          this.ahead.turns.stop();
          this.ahead.turns.next();
        }
        // After the engine is told of the settings, in a listener of its own that may come after this one.
        if (event.affectsConfiguration('gitardo.hosting')) setTimeout(() => void this.changesSeen.hostingChanged(), 0);
        if (event.affectsConfiguration('gitardo.conflicts')) void this.conflictReviews.tab?.reload();
      }),
      // The branches a comparison is of may have moved, and the conflicts of the review may be resolved.
      this.onDidReadRepository(() => {
        for (const tab of this.compareTabs.tabs.values()) void this.compareTabs.sendCompare(tab);
        void this.conflictReviews.tab?.reload();
      }),
      vscode.window.onDidChangeActiveColorTheme(async () => {
        if (!this.colored.size) return;
        const theme = (await this.themes.resolveActive()) ?? null;
        for (const webview of this.colored) this.post({ type: 'syntaxTheme', theme }, webview);
      }),
    ];
  }

  dispose(): void {
    this.ahead.turns.stop();
    for (const group of this.allGroups()) {
      this.sessions.leave(group);
      group.dispose();
    }
    this.kept.dispose();
    for (const subscription of this.subscriptions) subscription.dispose();
  }

  /** The followers and every tab of the graph. */
  private allGroups(): ViewGroup[] {
    return [this.followers, ...new Set(this.groups.values())].filter((group, index, all) => all.indexOf(group) === index);
  }

  /**
   * The group a view is in, whose repository what it asks acts on: the tab of a commit, of a comparison and the like
   * goes with the group that shows its repository, the panel's first.
   */
  private groupOf(webview: vscode.Webview): ViewGroup | undefined {
    const pinned = this.pinned.get(webview);
    if (!pinned) return this.groups.get(webview);
    const shows = (group: ViewGroup) => !!group.session && samePath(group.session.root, pinned.root);
    // With no group on its repository, it is sent nothing more, as the tab of a commit of another repository is.
    return shows(this.followers) ? this.followers : this.allGroups().find(shows);
  }

  /** Sends a message to the views of a group. */
  private postToGroup(group: ViewGroup, message: ToViews): void {
    this.post(message, undefined, group);
  }

  /** Loads the UI into a webview and wires it to this controller. `data` is written on the page for the view to read. */
  attach(webview: vscode.Webview, kind: ViewKind, data: Record<string, string> = {}, group = this.followers): vscode.Disposable {
    const dist = vscode.Uri.joinPath(this.extensionUri, 'dist');
    webview.options = { enableScripts: true, localResourceRoots: this.roots };
    // The session tells the page what it remembers from an earlier start of VS Code, for what is to start afresh each time.
    webview.html = this.html(webview, dist, { view: kind, session: vscode.env.sessionId, ...data });
    this.views.set(webview, kind);
    if (kind !== 'repositories') this.groups.set(webview, group);
    void this.gitFolders.tellShown();
    const listener = webview.onDidReceiveMessage((message: WebviewMessage) => this.onMessage(webview, message));
    // The Repositories view is sent its rows, which the engine follows the folders of while it is open.
    const rows = kind === 'repositories' ? this.repositoriesView.open(webview) : undefined;
    return new vscode.Disposable(() => {
      listener.dispose();
      rows?.dispose();
      this.views.delete(webview);
      this.groups.delete(webview);
      void this.gitFolders.tellShown();
      this.messages.signatureReads.delete(webview);
      this.colored.delete(webview);
      this.iconed.delete(webview);
      this.setFocused(webview, false);
    });
  }

  /**
   * VS Code is told when a view that searches has the keyboard, for the keybinding of package.json to take Ctrl+F
   * away from the search of the editor behind: nothing else says that the focus is inside a webview.
   */
  private setFocused(webview: vscode.Webview, focused: boolean): void {
    const before = this.focused.size;
    if (focused) this.focused.add(webview);
    else this.focused.delete(webview);
    if (!before !== !this.focused.size) void vscode.commands.executeCommand('setContext', 'gitardo.searchFocus', this.focused.size > 0);
  }

  /**
   * **Refresh**: reads the repository a group shows again whole, counting what is not committed in every other worktree
   * and reading every commit again, which the reads that follow a change keep when nothing changed there (M15).
   */
  async refreshAll(on = this.followers): Promise<void> {
    if (on.session) on.session.everything = true;
    return this.refresh(on);
  }

  /** Reads again the repository a group shows, the panel's unless another is given; with none yet, looks for one. */
  async refresh(on = this.followers): Promise<void> {
    if (!this.located) return this.locating ?? this.locate();
    // A tab that shows nothing waits for nothing: only the panel looks for the repositories again.
    if (!on.session) return on === this.followers ? (this.locating ?? this.locate()) : this.locating;
    return on.session.refresh();
  }

  /** The state read for a group, with what the window adds to it: its favorites, its settings, its pull requests. */
  private complete(group: ViewGroup, state: RepoState, first = false): Promise<RepoState> {
    return this.completeFor(group.session, this.homeOf(group, state.root), state, first);
  }

  /** A state read of a repository, for the session that read it, with what the window adds to it; the journal is not asked for a `first` step. */
  private async completeFor(session: RepositorySession | undefined, home: RepoInfo | null, state: RepoState, first = false): Promise<RepoState> {
    const sort = branchSort();
    // The pages of the read that come while the rest is read are laid out under its rows once it is: a read that
    // ended meanwhile would not be told to the views after.
    return completed(sortRefs(state, sort), async () => ({
      favorites: this.history.favorites(state),
      defaultPull: defaultPull(),
      repos: this.repos,
      hosting: session?.hosting ?? null,
      pulls: session?.pulls ?? {},
      agentsWindow: await (this.agentsWindow ??= hasCommand(AGENTS_WINDOW)),
      undo: first ? null : await this.newestInJournal(state.root),
      worktreesRoot: worktreesFolder(state),
      openBranches: vscode.workspace.getConfiguration('gitardo').get('branches.openIn') === 'worktree' ? ('worktree' as const) : ('checkout' as const),
      listed: this.history.listedIn(state),
      sortOrder: sort,
      traceSelected: vscode.workspace.getConfiguration('gitardo').get('graph.traceSelected') !== false,
      oneLane: vscode.workspace.getConfiguration('gitardo').get('graph.oneLane') === true,
      compactRows: vscode.workspace.getConfiguration('gitardo').get('graph.compactRows') !== false,
      graphColumns: graphColumns(),
      dateSeparators: vscode.workspace.getConfiguration('gitardo').get('graph.dateSeparators') !== false,
      combineUpstream: vscode.workspace.getConfiguration('gitardo').get('graph.combineUpstream') !== false,
      islands: vscode.workspace.getConfiguration('gitardo').get('graph.islands') === true,
      resolveWithAI: resolveWithAI(),
      home,
      keepOpen: keepOpen(),
      signOff: vscode.workspace.getConfiguration('gitardo').get('commit.signOff') === true,
      tagMarks: this.tagMarks.get(state.root) ?? null,
    }));
  }

  /** The repository of the workspace a group goes back to, while it shows a worktree of it that is not in the workspace. */
  private homeOf(group: ViewGroup, root: string): RepoInfo | null {
    return this.repos.some((each) => samePath(each.root, root)) ? null : (group.home ?? null);
  }

  /** The state of a repository the views may not show, with one commit only: for choosing among its branches. */
  private async stateOf(repo: Repository): Promise<RepoState> {
    const client = reads.client();
    const asked = { limit: 1, filter: null, scope: null, stashes: false, firstParent: false };
    return graphState(client, repo.root, await reads.parts(repo), asked, (graph) => readCommits(client, repo.root, graph, 1));
  }

  /** Reads the graph of a group again, or does what ends with that, telling its views what for while they wait. */
  private reload(on: ViewGroup, what: Loading, work = () => this.refresh(on)): Promise<void> {
    return on.reload(what, work);
  }

  /** The groups that show a repository now, each with its state. */
  private groupsShowing(root: string): ViewGroup[] {
    return this.allGroups().filter((group) => group.state && samePath(group.state.root, root));
  }

  // Milestone M47: the files stored with Git LFS.

  /** Opens the Large files tab of the repository a group shows, or shows the one open for it (`lfs.page`). */
  async openLargeFiles(on = this.followers): Promise<void> {
    const repo = (await this.requireState(on)) && on.repo;
    if (repo) this.largeFiles.open(repo);
  }

  /**
   * Says that a version of a file stored with Git LFS that a tab opened is not downloaded, with **Download**, which
   * downloads it with the engine (`lfs.download`): into the working tree for the commit checked out and the index, into
   * the store for another commit. The tabs of the scheme `gitardo` read it again once it ran.
   */
  sayNotDownloaded(side: NotDownloaded): void {
    void vscode.window.showInformationMessage(notDownloadedMessage(side.path, side.size), LFS_WORDS.download).then(async (choice) => {
      if (choice !== LFS_WORDS.download) return;
      const target: Target = { kind: 'lfsFiles', paths: [{ path: side.path, isFolder: false }], commit: side.commit };
      if (await this.runOnEngine([{ feature: 'lfs.download', target }], { root: side.root })) this.readAgainAfterDownload(side.root);
    });
  }

  /** Reads the repository and runs every operation on it with this engine from now on. */
  useEngine(engine: WindowEngine): void {
    this.engine = engine;
  }

  /** How a token of a GitLab that it refused is forgotten, in the secret storage of the extension (`hosting.token`). */
  useTokenForgetting(forget: (host: string) => Promise<void>): void {
    this.forgetToken = forget;
  }

  /** Runs an action of a view on the repository its group shows: the panel's unless another is given. */
  async action(action: Action, on = this.followers): Promise<void> {
    const state = await this.requireState(on);
    const repo = on.repo;
    if (!state || !repo) return;
    // Every operation on the repository is the engine's: what is VS Code's around it is done here (`hostAction`).
    // A key binding, or a view drawn before the setting changed, can still ask for it: said here, where the engine would
    // offer a page of settings the extension does not have.
    if (action.kind === 'resolveConflicts' && !autoResolve()) {
      void vscode.window
        .showInformationMessage('Gitardo: resolving conflicts automatically is experimental, and it is switched off in the settings.', 'Open Settings')
        .then((choice) => choice && vscode.commands.executeCommand('workbench.action.openSettings', 'gitardo.conflicts.autoResolve'));
      return;
    }
    if (action.kind === 'checkTags') return this.commands.checkTags(state, repo);
    try {
      if (await hostAction(action, state)) return;
    } catch (error) {
      void vscode.window.showErrorMessage(`Gitardo: ${errorText(error)}`);
      return;
    }
    const runs = engineRuns(action, state);
    if (!runs) return;
    if (!reads.available) return sayEngineMissing();
    // VS Code's Git lets go of what the engine is about to move, empty or delete.
    await this.runFollowUps.letGoInVSCodeGit(action, state);
    // The clipboard is VS Code's: the text of a patch in it is read here and given to the engine.
    const first = runs[0];
    if (action.kind === 'applyPatch' && first) first.answers = { clipboard: { kind: 'text', text: await vscode.env.clipboard.readText() } };
    // A bisect says once it ran whether the commit it was looking for is found, which the engine leaves to the window.
    if ((await this.runOnEngine(runs, repo, {}, undefined, on)) && action.kind === 'bisect') this.runFollowUps.reportBisect(on);
  }

  /**
   * Runs a feature of the engine in a repository, its questions asked with VS Code's input boxes and picks, its log in
   * the output channel, and the follow-ups it leaves carried out here; false when it did not run, for want of an engine
   * or because another operation runs, which is said.
   */
  private async runOnEngine(runs: EngineRun[], repo: Repository, hooks: HostHooks = {}, ran?: () => void, on = this.groupShowing(repo.root)): Promise<boolean> {
    const client = this.engine?.running;
    if (!client) {
      sayEngineMissing();
      return false;
    }
    for (const { feature, target, answers } of runs) {
      // One operation at a time, as in Gitardo Desktop: another one asked meanwhile is refused, and that is said.
      if (this.operation) {
        const { feature: running, root, asker } = this.operation;
        void vscode.window.showErrorMessage(runningElsewhere(asker.confirmed ?? running, this.repositoryNamed(root)));
        return false;
      }
      const asker = new VscodeAsker(client, repo.root, hooks);
      this.operation = { feature, root: repo.root, asker };
      this.operations++;
      // Nothing is read ahead while an operation runs: the read that follows it gives the others their turn again.
      this.ahead.turns.stop();
      this.post({ type: 'busy', title: feature });
      this.repositoriesView.ran(repo.root);
      let result: Awaited<ReturnType<typeof client.runFeature>> | undefined;
      let failed: unknown;
      let busy = false;
      try {
        const inputs = this.memento.get<Record<string, string>>(LAST_INPUTS, {});
        // What new worktrees get as VS Code resolves it in the repository's folder, for the features that save it.
        const setup = feature.startsWith('worktree-setup.') ? setupGiven(repo.root) : undefined;
        result = await client.runFeature({ repository: repo.root, feature, target, answers, inputs, setup }, asker);
      } catch (failure) {
        busy = failure instanceof EngineError && failure.code === ErrorCode.busy;
        if (!(failure instanceof EngineError && (failure.code === ErrorCode.busy || failure.code === ErrorCode.cancelled))) {
          const data = failure instanceof EngineError ? (failure.data as { output?: string | null; followUps?: FollowUp[] } | undefined) : undefined;
          const said = data?.output?.trim();
          if (said) this.log.error(said);
          failed = failure;
        }
      } finally {
        // The repository is read once the run is over: nothing is read while an operation runs.
        this.operation = undefined;
        this.post({ type: 'busy', title: null });
        this.repositoriesView.ran(null);
      }
      const { finished, followUps } = result ?? { finished: undefined, followUps: [] };
      if (finished?.kind === 'ran') {
        // What a new worktree runs and offers once made is VS Code's.
        await this.runFollowUps.beforeRead(this.runFollowUps.worktreeMade(feature, finished, followUps, repo), { feature, target, repo, title: finished.plan.title });
      }
      await this.refreshAfter(repo);
      // The engine runs one operation in a repository at a time, whoever asked for it.
      if (busy) {
        void vscode.window.showErrorMessage(runningElsewhere(null, this.repositoryNamed(repo.root)));
        return false;
      }
      // Said once the repository is read again, which tells whether it stopped on conflicts.
      if (failed !== undefined) this.runFollowUps.reportEngineFailure(failed, asker.confirmed, repo);
      if (finished?.kind === 'refused') this.runFollowUps.offerAfterRefusal(finished.message, finished.offer, target, repo);
      // **Compare First** in the warning of a force push: nothing of the plan ran, and what it offered instead runs now.
      if (asker.instead) return this.runOnEngine([{ feature: asker.instead.feature, target: asker.instead.target, answers: asker.instead.answers }], repo, {}, undefined, on);
      if (finished?.kind !== 'ran') return true;
      this.runFollowUps.afterRead(followUps, repo, on);
    }
    ran?.();
    return true;
  }

  /**
   * Reads again, once an operation ran, the repository it ran in, the one the panel shows, and every other repository
   * shown that heard of a change meanwhile: nothing is read while an operation runs.
   */
  private async refreshAfter(repo: Repository): Promise<void> {
    const ran = this.sessions.find(repo.root);
    const sessions = this.sessions.all().filter((session) => session === ran || session === this.followers.session || !session.isFresh);
    if (!sessions.length) return this.refresh();
    await Promise.all(sessions.map((session) => session.refresh()));
  }

  /** The group that shows a repository, for what an operation run there selects and says: the panel's first. */
  private groupShowing(root: string): ViewGroup {
    if (!this.followers.session || samePath(this.followers.session.root, root)) return this.followers;
    return this.allGroups().find((group) => group.session && samePath(group.session.root, root)) ?? this.followers;
  }

  /** The state held of a repository by a group that shows it, the panel's first; null when no group shows it. */
  private heldFor(root: string): RepoState | null {
    const shown = (group: ViewGroup) => group.state && samePath(group.state.root, root);
    if (shown(this.followers)) return this.followers.state;
    return this.allGroups().find(shown)?.state ?? null;
  }

  /** Keeps the marks of the tags of a repository, and shows them while it is the one shown. */
  private markTags(root: string, marks: TagMarks): void {
    this.tagMarks.set(root, marks);
    for (const group of this.groupsShowing(root)) this.setState(group, { ...group.state!, tagMarks: marks });
  }

  /** The state of the repository a group shows; without one, the user is told why: no engine, or no repository. */
  private async requireState(on = this.followers): Promise<RepoState | null> {
    await this.whenLocated();
    if (!reads.available) sayEngineMissing();
    else if (!on.state) void vscode.window.showWarningMessage('Gitardo: no Git repository in this workspace.');
    return on.state;
  }

  /** The state of a group, sent to its views; the panel's also says in the menus and the Repositories view what it shows. */
  private setState(group: ViewGroup, state: RepoState | null): void {
    group.state = state;
    if (group === this.followers) this.menus.showHiddenInMenus(state);
    if (group === this.followers) this.menus.showLfsInMenus(state);
    this.postToGroup(group, { type: 'state', state });
    if (group === this.followers) {
      this.repositoriesView.shown(state);
      this.stateEmitter.fire(state);
      void this.unsaved.check();
    }
    if (state) this.readEmitter.fire(state);
    if (state) this.largeFiles.repositoryRead(state.root);
  }

  /** Looks for the repositories of the workspace and reads the one to show. Who asks while that runs waits for it. */
  private locate(): Promise<void> {
    this.located = true;
    const locating = this.findRepos();
    this.locating = locating;
    const done = () => {
      if (this.locating === locating) this.locating = undefined;
      this.ahead.turns.next();
    };
    locating.then(done, done);
    return locating;
  }

  /** Once the repositories of the workspace are known and the one shown is read: at once when they already are. */
  private whenLocated(): Promise<void> {
    return this.locating ?? (this.located ? Promise.resolve() : this.locate());
  }

  private async findRepos(): Promise<void> {
    const folders = (vscode.workspace.workspaceFolders ?? []).map((folder) => folder.uri.fsPath);
    // Without the engine there is nothing to look for them with: the views say so.
    const found = reads.available
      ? await reads.discover(folders).catch((error) => {
          this.log.error(`Could not look for the repositories of the workspace: ${errorText(error)}`);
          return [];
        })
      : [];
    const roots = found.map((each) => each.folder);
    // Each worktree and submodule under its repository, as the list of the repositories shows them (`repo.group-tabs`).
    const grouped = groupRepositories(found).map(({ repository, depth }) => ({ repository: { ...repository, name: repositoryName(repository.folder, folders) }, depth }));
    this.repos = grouped.map(({ repository, depth }) => ({ root: repository.folder, name: repository.name, depth }));
    // The list is another: what was being read ahead may not be in it, and each repository gets its turn again.
    this.ahead.turns.stop();
    this.ahead.turns.again();
    this.repositoriesView.found(grouped);
    this.repositories.found(found);
    for (const group of this.allGroups()) this.repositories.retitle(group);
    // A repository that is no longer in the workspace keeps nothing.
    this.kept.forget((root) => !this.repos.some((repo) => samePath(repo.root, root)));
    // The repository chosen last time in this workspace, as long as it is still there.
    const remembered = this.memento.get<string>(REPO_KEY);
    await this.open(roots.find((root) => root === remembered) ?? roots[0]);
  }

  /**
   * Points the panel and the side bar at one repository of the workspace, or at none, from the start: with
   * `gitardo.repositories.keepOpen` on, the one left keeps what it showed, and one that kept what it showed is shown as
   * it was (`repo.keep-open`).
   */
  private async open(root: string | undefined): Promise<void> {
    return this.repositories.point(this.followers, root, true);
  }

  /**
   * A tab of the graph brought back on a folder that is no longer there, such as a worktree removed while VS Code was
   * closed: it says so, with **Close**, and reads nothing (`repo.keep-tabs`, Q1523).
   */
  private goneTab(panel: vscode.WebviewPanel, root: string): void {
    const webview = panel.webview;
    webview.options = { enableScripts: true, localResourceRoots: this.roots };
    webview.html = this.html(webview, vscode.Uri.joinPath(this.extensionUri, 'dist'), { view: 'graph', tab: 'graph', gone: root });
    panel.title = tabTitle(root, this.repos, undefined);
    this.goneTabs.set(webview, panel);
    const listener = webview.onDidReceiveMessage((message: WebviewMessage) => this.onMessage(webview, message));
    panel.onDidDispose(() => {
      listener.dispose();
      this.goneTabs.delete(webview);
    });
  }

  /**
   * Reads again the repository the panel shows, once, `delay` after the first of a burst of events; with none shown, it
   * is looked for again.
   */
  private scheduleRefresh(cause: string, delay = REFRESH_DELAY): void {
    const session = this.followers.session;
    if (session) return session.scheduleRefresh(cause, delay);
    if (!this.views.size || this.running) return;
    clearTimeout(this.lookAgain);
    this.lookAgain = setTimeout(() => void this.refresh(), delay);
  }

  /** When the repositories of the workspace are looked for again, for a change heard of while none was shown. */
  private lookAgain?: ReturnType<typeof setTimeout>;

  private post(message: ToViews, to?: vscode.Webview, group?: ViewGroup): void {
    // The Repositories view is sent its rows alone, by `repositoriesView`; a message for a group goes to its views.
    const targets = to
      ? [to]
      : [...this.views].filter(([view, kind]) => kind !== 'repositories' && (!group || this.groupOf(view) === group)).map(([view]) => view);
    if (message.type !== 'state') {
      for (const view of targets) void view.postMessage(message);
      return;
    }
    const { state } = message;
    for (const view of targets) {
      const kind = this.views.get(view) ?? 'graph';
      // The tabs that read what they show themselves are sent no state.
      if (!partsOf(kind).length) continue;
      if (!state) {
        // Without a state, the views say whether it is for want of the engine; the next state is sent them whole.
        this.sent.delete(view);
        this.behind.sent(view, false);
        void view.postMessage(reads.available ? { type: 'state', state: null } : { type: 'state', state: null, engineMissing: true });
        continue;
      }
      const keys = new Map<number, string>();
      const filter = this.groupOf(view)?.filter ?? null;
      const keyOf = (count: number) => keys.get(count) ?? keys.set(count, rowsKey(state, filter, count)).get(count)!;
      // A long history is not sent in one message: its first page, and the rest behind it in pieces (QA6).
      const { message: posted, sent, more } = stateFor(kind, state, this.sent.get(view), keyOf, { first: pageSize(), piece: ROWS_PIECE });
      this.sent.set(view, sent);
      this.behind.sent(view, more);
      if (!posted) continue;
      void view.postMessage(posted).then((delivered) => {
        // A view that is not there to receive it asks again when it comes back, and gets everything.
        if (this.sent.get(view) !== sent) return;
        if (!delivered) this.sent.delete(view);
        this.behind.reached(view, delivered);
      });
    }
  }

  /**
   * What a view that opens waits for: the search for the repositories of the workspace while it runs, and then what the
   * session of its group says (`RepositorySession.readForView`, D88). With no repository shown, they are looked for again.
   */
  private async readForView(on: ViewGroup): Promise<void> {
    if (this.locating) return this.locating;
    if (!this.located) return this.locate();
    if (!on.session) return on === this.followers ? this.refresh(on) : undefined;
    return on.session.readForView(on);
  }

  /** Hands what a view asks to the part that answers it. */
  private onMessage(webview: vscode.Webview, message: WebviewMessage): Promise<void> {
    return this.messages.onMessage(webview, message);
  }

  /**
   * Runs features of the engine that another part of Gitardo, such as the Flow Diff, asks for in a repository: not always
   * the one shown. True once they all ran.
   */
  async runIn(root: string, runs: EngineRun[]): Promise<boolean> {
    const repo = this.repo && samePath(this.repo.root, root) ? this.repo : { root };
    let ran = false;
    await this.runOnEngine(runs, repo, {}, () => (ran = true));
    return ran;
  }

  /** The name of a repository as the list of the repositories has it. */
  private repositoryNamed(root: string): string {
    const folders = (vscode.workspace.workspaceFolders ?? []).map((folder) => folder.uri.fsPath);
    return this.repos.find((repo) => samePath(repo.root, root))?.name ?? repositoryName(root, folders);
  }

  /** The newest entry of the engine's journal of a repository, for the label of **Undo**; null without an engine. */
  private async newestInJournal(root: string): Promise<RepoState['undo']> {
    const client = this.engine?.running;
    if (!client) return null;
    try {
      const entry = await client.journalLast(root);
      return entry && { title: entry.title, undoable: entry.undoable };
    } catch {
      return null;
    }
  }

  private resourceRoots(): vscode.Uri[] {
    return [vscode.Uri.joinPath(this.extensionUri, 'dist'), ...FileIcons.roots()];
  }

  /**
   * Sends the pages that show files the icons of the file icon theme, which changed. An extension with icon themes
   * added since they were opened is let in first: that loads them again, and they ask for the icons anew.
   */
  private sendFileIcons(): void {
    const roots = this.resourceRoots();
    if (roots.join('\n') !== this.roots.join('\n')) {
      this.roots = roots;
      for (const webview of this.views.keys()) webview.options = { ...webview.options, localResourceRoots: roots };
    }
    for (const webview of this.iconed) void this.fileIcons.for(webview).then((theme) => this.post({ type: 'fileIcons', theme }, webview));
  }

  /** `data` becomes the `data-` attributes of the body, which is where the page reads what view it is. */
  private html(webview: vscode.Webview, dist: vscode.Uri, data: Record<string, string>): string {
    const escape = (text: string) => text.replace(/[&<>"']/g, (character) => `&#${character.charCodeAt(0)};`);
    const attributes = Object.entries(data).map(([name, value]) => `data-${name}="${escape(value)}"`).join(' ');
    const nonce = Array.from({ length: 32 }, () => Math.floor(Math.random() * 36).toString(36)).join('');
    const script = webview.asWebviewUri(vscode.Uri.joinPath(dist, 'webview', 'index.js'));
    const style = webview.asWebviewUri(vscode.Uri.joinPath(dist, 'webview', 'index.css'));
    // The scripts of the extension itself, besides the page's own: the page of the view is imported by kind, and those
    // that color code once it shows some.
    // Images and fonts also for the icons of the file icon theme; `data:` for the images of a commit, in which nothing
    // of an SVG runs and nothing is fetched (`commit-page.images`).
    const csp = `default-src 'none'; style-src ${webview.cspSource}; font-src ${webview.cspSource}; img-src ${webview.cspSource} data:; script-src 'nonce-${nonce}' ${webview.cspSource};`;
    return `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta http-equiv="Content-Security-Policy" content="${csp}">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <link href="${style}" rel="stylesheet">
</head>
<body ${attributes}>
  <div id="root"></div>
  <script type="module" nonce="${nonce}" src="${script}"></script>
</body>
</html>`;
  }
}

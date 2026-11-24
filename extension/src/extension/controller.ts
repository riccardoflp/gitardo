import { createHash } from 'node:crypto';
import * as path from 'node:path';
import * as vscode from 'vscode';
import { ThemeService } from '../flow/theme';
import { CONFLICT_SIDES } from '../shared/conflicts';
import { pullWords, readsPullRequests, type Hosting } from '../shared/hosting';
import { openingOf } from '../shared/markdown';
import { samePath } from '../shared/paths';
import { offerCommitGraph } from './commitGraphOffer';
import { GRAPH_COLUMNS, WORKTREE, showsGraph } from '../shared/protocol';
import { isHash, sortRefs, worktreeLabel, worktreeStatus } from '../shared/refs';
import {
  cleanCommands,
  cleanPatterns,
  type CommandProposal,
  type IgnoredEntry,
  type SetupChoice,
  type SetupPatterns,
  type SetupTarget,
} from '../shared/worktreeSetup';
import { submoduleFolder } from '../shared/submodules';
import { subjectAndBody } from '../shared/message';
import { checkedOutBranch, operationName, worktreeBranch } from '../shared/state';
import { checkedMarks, followMarks, type TagMarks } from '../shared/tagMarks';
import { groupRepositories } from '../shared/repositoryGroups';
import { pruneScope } from '../shared/scope';
import { Activity } from './activity';
import { type KeptRepository, KeptRepositories } from './keptRepositories';
import { RepositoriesView } from './repositoriesView';
import { FileIcons } from './fileIcons';
import type {
  Action,
  CompareMode,
  CompareSide,
  ComparedRef,
  DiffSource,
  FileChange,
  FilesLayout,
  BranchSort,
  Filter,
  GraphColumn,
  ToViews,
  Loading,
  PullAction,
  RepoInfo,
  RepoState,
  Scope,
  Search,
  SyncAction,
  UiCommand,
  ViewKind,
  WebviewMessage,
} from '../shared/protocol';
import { openMergeEditor } from '../flow/merge';
import {
  AGENTS_WINDOW,
  LAST_INPUTS,
  SETUP_OFFERED,
  SET_UP,
  closeInVSCodeGit,
  hostAction,
  openIn,
  openWorktree,
  inVSCodeGit,
  refreshVSCodeGit,
  worktreesFolder,
} from './actions';
import { pickCoAuthors } from './coAuthors';
import { ConflictReviewTab } from './conflictReview';
import { INDEX, contentUri, isAbsent, proposalUri } from './content';
import { imagePair } from '../shared/images';
import { changeSetup, editSetupJson, savePatterns, saveSetup, setupChangeText, setupPatterns, worktreeSetupInfo } from './worktreeSettings';
import { EngineError, ErrorCode } from '../engine/connection';
import { type EngineRun, engineRuns, syncRuns } from '../engine/actions';
import type { FileResolutions, Finished, FollowUp, OfferedRun, Plan, Step, Target } from '../engine/messages';
import { type HostHooks, VscodeAsker, type WindowEngine, sayEngineMissing } from '../engine/vscode';
import { runSetupCommands } from './worktreeCommands';
import { type Repository, reads, repositoryName } from '../engine/reads';
import { completed, graphState, historyOf, laidOutOf, matchesOnEngine, positionOnEngine, readCommits } from '../engine/graph';
import type { ComparisonTarget, CompareSideTarget, WriteSettingFollowUp } from '../engine/messages';
import { extensionKey } from '../engine/settings';
import { RangeDiffTab } from './rangeDiffTab';
import { type GroupHost, REFRESH_DELAY, RepositorySession, type SessionHost, ViewGroup, errorText, failureText, firstParent, keepOpen, pageSize } from './repositorySession';
import { Sessions, tabTitle } from './sessions';
import { type Sent, stateFor } from './sentParts';
import { partsOf } from '../shared/stateParts';
import { COMPARE_REWRITE, SEE_WHAT_CHANGED, buttonWords, forcePushed, type RangeDiffTarget } from '../shared/rangeDiff';

const REFLOG_ENTRIES = 200;
/** The diffs VS Code's Git serves the sides of, once it has the repository. */
const WORKING_SOURCES = new Set<DiffSource['kind']>(['staged', 'unstaged', 'change']);
const REPO_KEY = 'gitardo.repository';
/** The branches chosen for the graph of each repository, by its folder. */
const SCOPE_KEY = 'gitardo.scope';
/** What the references of each repository list, by its folder, as their own filter says, apart from `SCOPE_KEY`. */
const LISTED_KEY = 'gitardo.branchesListed';
/** How the files of a commit are laid out in its tab: as a tree until a list is asked for. */
const FILES_KEY = 'gitardo.commitFiles';
/** The favorite branches of each repository of the workspace. */
const FAVORITES_KEY = 'gitardo.favorites';
/** The most commits the view of the commits of a branch is given, however many it asks for. */
const LOG_LIMIT = 10_000;
/** What fits of the message of a commit in the title of its tab. */
const TITLE_LENGTH = 32;
/** The comparisons opened last in each repository, by its folder. */
const RECENT_KEY = 'gitardo.recentComparisons';
/** How many of them are kept. */
const RECENT_COMPARISONS = 8;
/** How many subjects of the last commits a draft of a commit message is written with. */
const DRAFT_SUBJECTS = 10;

/** The kind of the tabs of the graph in the editor, which VS Code brings back after a reload (`repo.keep-tabs`). */
export const GRAPH_TAB = 'gitardo.graphEditor';

/** What a tab of the graph showed, which it is opened with again after a reload: its search and its branches shown. */
export interface TabShown {
  filter?: Filter | null;
  scope?: Scope | null;
}

/** The working tree, as a side of a comparison. */
const WORKTREE_SIDE: ComparedRef = { name: 'Working tree', hash: WORKTREE };

/** A comparison opened before, to open again: its sides by the names they had, and the file or folder it showed. */
interface RecentComparison {
  from: ComparedRef;
  to: ComparedRef;
  path?: string;
}

/** A comparison open in a tab: its two sides as they were chosen, and what the tab was sent last. */
interface CompareTab {
  panel: vscode.WebviewPanel;
  repo: Repository;
  from: CompareSide;
  to: CompareSide;
  mode: CompareMode;
  /** The file or the folder the comparison is narrowed to. */
  path: string | null;
  /** The commits and the mode the tab shows, to tell whether what it shows has to be read again. */
  shown?: string;
  /** Where the changes the tab shows start from. */
  since?: string;
}

/**
 * What is said of an operation asked while another runs, with its title when Gitardo knows it, as Gitardo Desktop says
 * it (`repo.switch-tab`).
 */
const runningElsewhere = (title: string | null, repository: string) =>
  `Gitardo: ${title ?? 'another operation'} is running in ${repository}. Try again once it ends.`;

/** The title of the tab of a comparison. */
const compareTitle = (from: CompareSide, to: CompareSide, path: string | null) =>
  `${from.name} ↔ ${to.name}${path ? ` · ${path.slice(path.lastIndexOf('/') + 1)}` : ''}`;

/**
 * How two sides are compared at first: two branches or tags from where they diverged, as a pull request shows them; a
 * commit, or the working tree, directly, for how the files differ from it.
 */
const compareMode = (from: CompareSide, to: CompareSide): CompareMode =>
  from.live && to.live && from.hash !== WORKTREE && to.hash !== WORKTREE ? 'diverged' : 'direct';

/** What can be a side of a comparison: a commit, or the working tree. */
const isSide = (hash: unknown): hash is string => typeof hash === 'string' && (hash === WORKTREE || isHash(hash));

/** A file or a folder as git names it, from the top of the repository, or null for all of it. */
const cleanPath = (path: unknown): string | null => {
  if (typeof path !== 'string') return null;
  const clean = path
    .trim()
    .replace(/\\/g, '/')
    .replace(/^(?:\.?\/)+|\/+$/g, '');
  return clean && clean !== '.' ? clean : null;
};

const hasCommand = async (id: string) => (await vscode.commands.getCommands(true)).includes(id);

/**
 * Identifies the first `count` rows of a state: two states with the same key have the same rows, as far as that.
 * A commit never changes, so its hash stands for all of it, and the lines of the graph of a row only depend on the rows
 * above it. The row of the uncommitted changes only counts for being there or not, which moves the lines of the graph:
 * how many changes it says there are is sent apart.
 * The filter is part of it because the history of a file gives the same commits other parents, and so is following
 * only the first parent of each merge, which leaves a merge one parent.
 */
function rowsKey(state: RepoState, filter: Filter | null, count = state.rows.length): string {
  // The rows of the engine's reads keep what identifies them as they are laid out, page after page.
  const laid = laidOutOf(state.rows);
  if (laid) return laid.keyAt(count);
  const hash = createHash('sha1');
  hash.update(`${state.root}\0${JSON.stringify(filter)}\0${state.firstParent}\0`);
  for (let index = 0; index < count; index++) hash.update(state.rows[index].hash);
  return hash.digest('hex');
}

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
  /** The rows of the Repositories view, and what the engine follows for them while the view is open (`repo.tabs`). */
  readonly repositoriesView: RepositoriesView;
  /** Whether this VS Code has the Agents window, which is looked up once: it comes and goes with VS Code itself. */
  private agentsWindow?: Promise<boolean>;
  private readonly views = new Map<vscode.Webview, ViewKind>();
  /** The read of the signature each view waits for, cancelled when it asks for another (`details.signature`). */
  private readonly signatureReads = new Map<vscode.Webview, AbortController>();
  /** What the last **Check tags on `<remote>`** of each repository said, by its folder, until VS Code is closed (`tag.remote-marks`). */
  private readonly tagMarks = new Map<string, TagMarks>();
  /** The tabs of the commits, by repository and hash, and the repository each is of: not always the one being shown. */
  private readonly commitTabs = new Map<string, vscode.WebviewPanel>();
  /** The tabs of the comparisons, by their webview. */
  private readonly compareTabs = new Map<vscode.Webview, CompareTab>();
  private readonly pinned = new Map<vscode.Webview, Repository>();
  /** The pages that color code, which are sent the colors of the theme again when it changes. */
  private readonly colored = new Set<vscode.Webview>();
  private readonly themes = new ThemeService();
  /** The pages that show files, which are sent the icons of the file icon theme again when it changes. */
  private readonly iconed = new Set<vscode.Webview>();
  private readonly fileIcons: FileIcons;
  /** The folders the pages may load files from: the build, and the extensions with icon themes. */
  private roots: vscode.Uri[];
  /**
   * The tab where what new worktrees get is chosen, the repository it is of, and what git ignores there and the commands
   * proposed for it, once looked at.
   */
  /** `saving` while the tab writes the settings one after the other, which it shows once they are all written. */
  private setupTab?: { panel: vscode.WebviewPanel; repo: Repository; entries: IgnoredEntry[] | null; proposed: CommandProposal[]; saving: boolean };
  /** The tab where what is proposed for the conflicts is looked at before it is written. */
  private reviewTab?: ConflictReviewTab;
  /** The views with a search box that have the keyboard: one at most, but each says so on its own. */
  private readonly focused = new Set<vscode.Webview>();
  /** What each view was last sent of the state: the key of each part, and how many commits (D88). */
  private readonly sent = new WeakMap<vscode.Webview, Sent>();
  /** The files of the repository shown that have edits not saved in an editor, by their paths from its root. */
  private unsaved: string[] = [];
  /**
   * Whether git ignores each file with edits not saved, by its repository and its path there, asked of the engine once
   * for each file while it stays unsaved, not at each read.
   */
  private readonly ignoredUnsaved = new Map<string, boolean>();
  /** What `unsaved` was last worked out from: the state, and the files of the editors with edits not saved. */
  private unsavedFrom?: { state: RepoState | null; files: string };
  /** The files hidden from git the Explorer's menu was last told of, as JSON. */
  private hiddenInMenus = '';
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
    const unsaved = () => void this.checkUnsaved();
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
      hasViews: (session) => this.hasViews(session),
      repositoryNamed: (root) => this.repos.find((each) => samePath(each.root, root))?.name ?? path.basename(root),
      autoResolve: () => this.autoResolve(),
      complete: (group, read) => this.complete(group, read),
      stateRead: (group, state) => this.setState(group, state),
      settleScope: (group, root, scope) => this.settleScope(group, root, scope),
      changedIn: (root, cause, delay) => this.changedIn(root, cause, delay),
      fileChangedIn: (root, file) => this.fileChangedIn(root, file),
      pullsRead: (session) => {
        for (const group of this.sessions.groupsOn(session)) if (group.state) this.setState(group, { ...group.state, pulls: session.pulls });
      },
      forgetToken: (host) => this.forgetToken?.(host) ?? Promise.resolve(),
    };
    this.sessions = new Sessions<RepositorySession, ViewGroup>({
      make: (root) => new RepositorySession(root, sessionHost),
      letGo: (session, last) => this.letGo(session, last),
    });
    this.rangeDiffTab = new RangeDiffTab({
      attach: (webview, kind, data) => this.attach(webview, kind, data),
      pin: (webview, repo) => (repo ? this.pinned.set(webview, repo) : this.pinned.delete(webview)),
      track: (title, work) => this.activity.track(title, work),
      post: (message, to) => this.post(message, to),
    });
    this.repositoriesView = new RepositoriesView(log);
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
        this.saved(document.uri);
      }),
      // An edit, or one undone back to what is saved, and an editor closed without saving.
      vscode.workspace.onDidChangeTextDocument(unsaved),
      vscode.workspace.onDidCloseTextDocument(unsaved),
      vscode.workspace.onDidCreateFiles(schedule('files were created')),
      vscode.workspace.onDidDeleteFiles(schedule('files were deleted')),
      vscode.workspace.onDidRenameFiles(schedule('files were renamed')),
      vscode.window.onDidChangeWindowState((window) => window.focused && this.focusGot()),
      vscode.workspace.onDidChangeConfiguration((event) => {
        const sections = ['gitardo.conflicts', 'gitardo.pull', 'gitardo.worktrees', 'gitardo.branches', 'gitardo.graph', 'gitardo.commit', 'git.branchSortOrder'];
        if (sections.some((section) => event.affectsConfiguration(section))) {
          // They can change what is listed: the commits are read again (Q442), and those that are the same stay (Q781).
          for (const group of this.allGroups()) group.graph.readAgain();
          for (const session of this.sessions.all()) session.scheduleRefresh('the settings changed');
          // The repositories kept were read with the settings before: each is read again once it is shown.
          this.kept.changedAll('the settings changed');
        }
        if (event.affectsConfiguration('gitardo.repositories.keepOpen')) this.keepOpenChanged();
        // After the engine is told of the settings, in a listener of its own that may come after this one.
        if (event.affectsConfiguration('gitardo.hosting')) setTimeout(() => void this.hostingChanged(), 0);
        if (event.affectsConfiguration('gitardo.conflicts')) void this.reviewTab?.reload();
      }),
      // The branches a comparison is of may have moved, and the conflicts of the review may be resolved.
      this.onDidReadRepository(() => {
        for (const tab of this.compareTabs.values()) void this.sendCompare(tab);
        void this.reviewTab?.reload();
      }),
      vscode.window.onDidChangeActiveColorTheme(async () => {
        if (!this.colored.size) return;
        const theme = (await this.themes.resolveActive()) ?? null;
        for (const webview of this.colored) this.post({ type: 'syntaxTheme', theme }, webview);
      }),
    ];
  }

  dispose(): void {
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

  /** Whether a view is there to show what is read of a session. */
  private hasViews(session: RepositorySession): boolean {
    const groups = this.sessions.groupsOn(session);
    // The panel's views include the tabs of commits and the Repositories view, as they did when one repository was shown.
    if (groups.includes(this.followers)) return this.views.size > 0;
    return [...this.groups].some(([view, group]) => groups.includes(group) && this.views.has(view));
  }

  /** Sends a message to the views of a group. */
  private postToGroup(group: ViewGroup, message: ToViews): void {
    for (const [view, kind] of this.views) if (kind !== 'repositories' && this.groupOf(view) === group) this.post(message, view);
  }

  /** Loads the UI into a webview and wires it to this controller. `data` is written on the page for the view to read. */
  attach(webview: vscode.Webview, kind: ViewKind, data: Record<string, string> = {}, group = this.followers): vscode.Disposable {
    const dist = vscode.Uri.joinPath(this.extensionUri, 'dist');
    webview.options = { enableScripts: true, localResourceRoots: this.roots };
    // The session tells the page what it remembers from an earlier start of VS Code, for what is to start afresh each time.
    webview.html = this.html(webview, dist, { view: kind, session: vscode.env.sessionId, ...data });
    this.views.set(webview, kind);
    if (kind !== 'repositories') this.groups.set(webview, group);
    const listener = webview.onDidReceiveMessage((message: WebviewMessage) => this.onMessage(webview, message));
    // The Repositories view is sent its rows, which the engine follows the folders of while it is open.
    const rows = kind === 'repositories' ? this.repositoriesView.open(webview) : undefined;
    return new vscode.Disposable(() => {
      listener.dispose();
      rows?.dispose();
      this.views.delete(webview);
      this.groups.delete(webview);
      this.signatureReads.delete(webview);
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
  private async complete(group: ViewGroup, state: RepoState): Promise<RepoState> {
    const sort = branchSort();
    const session = group.session;
    // The pages of the read that come while the rest is read are laid out under its rows once it is: a read that
    // ended meanwhile would not be told to the views after.
    return completed(sortRefs(state, sort), async () => ({
      favorites: this.favorites(state),
      defaultPull: this.defaultPull(),
      repos: this.repos,
      hosting: session?.hosting ?? null,
      pulls: session?.pulls ?? {},
      agentsWindow: await (this.agentsWindow ??= hasCommand(AGENTS_WINDOW)),
      undo: await this.newestInJournal(state.root),
      worktreesRoot: worktreesFolder(state),
      openBranches: vscode.workspace.getConfiguration('gitardo').get('branches.openIn') === 'worktree' ? ('worktree' as const) : ('checkout' as const),
      listed: this.listedIn(state),
      sortOrder: sort,
      traceSelected: vscode.workspace.getConfiguration('gitardo').get('graph.traceSelected') !== false,
      oneLane: vscode.workspace.getConfiguration('gitardo').get('graph.oneLane') === true,
      graphColumns: graphColumns(),
      dateSeparators: vscode.workspace.getConfiguration('gitardo').get('graph.dateSeparators') !== false,
      combineUpstream: vscode.workspace.getConfiguration('gitardo').get('graph.combineUpstream') !== false,
      islands: vscode.workspace.getConfiguration('gitardo').get('graph.islands') === true,
      resolveWithAI: this.resolveWithAI(),
      home: this.homeOf(group, state.root),
      keepOpen: keepOpen(),
      signOff: vscode.workspace.getConfiguration('gitardo').get('commit.signOff') === true,
      tagMarks: this.tagMarks.get(state.root) ?? null,
    }));
  }

  /** The repository of the workspace a group goes back to, while it shows a worktree of it that is not in the workspace. */
  private homeOf(group: ViewGroup, root: string): RepoInfo | null {
    return this.repos.some((each) => samePath(each.root, root)) ? null : (group.home ?? null);
  }

  /**
   * Turned off, the repositories kept forget at once what they showed, and the views forget what they kept of them;
   * turned on, the repository shown keeps what it shows once it is left.
   */
  private keepOpenChanged(): void {
    if (!keepOpen()) this.kept.forget();
    for (const group of this.allGroups()) if (group.state) this.setState(group, { ...group.state, keepOpen: keepOpen() });
  }

  /** Resolving conflicts without choosing a side is experimental: nothing of it is offered, or run, until it is switched on. */
  private autoResolve(): boolean {
    return vscode.workspace.getConfiguration('gitardo').get<boolean>('conflicts.autoResolve') === true;
  }

  /** Asking a language model how to resolve conflicts is experimental too, with a setting of its own. */
  private resolveWithAI(): boolean {
    return vscode.workspace.getConfiguration('gitardo').get<boolean>('conflicts.ai') === true;
  }

  /**
   * Opens the tab where what is proposed for the conflicts is looked at, of these files or of all of them, or shows the
   * one open. With `ai` the language model is asked about the conflicts the rules leave.
   */
  async reviewConflicts(paths?: string[], ai = false, on = this.followers): Promise<void> {
    const repo = (await this.requireState(on)) && on.repo;
    if (!repo) return;
    if (!this.autoResolve() && !this.resolveWithAI()) {
      void vscode.window
        .showInformationMessage('Gitardo: resolving conflicts automatically is experimental, and it is switched off in the settings.', 'Open Settings')
        .then((choice) => choice && vscode.commands.executeCommand('workbench.action.openSettings', 'gitardo.conflicts'));
      return;
    }
    const wanted = paths?.filter((file) => typeof file === 'string');
    if (this.reviewTab && samePath(this.reviewTab.repo.root, repo.root)) this.reviewTab.reveal(wanted);
    else {
      this.reviewTab?.dispose();
      const host = {
        attach: (webview: vscode.Webview, kind: ViewKind, data: Record<string, string>) => this.attach(webview, kind, data),
        post: (message: ToViews, webview: vscode.Webview) => this.post(message, webview),
        write: (on: Repository, resolutions: FileResolutions[]) => this.writeResolutions(on, resolutions),
        settings: () => ({ rules: this.autoResolve(), ai: this.resolveWithAI() }),
      };
      const tab: ConflictReviewTab = new ConflictReviewTab(host, repo, wanted, () => {
        this.pinned.delete(tab.webview);
        if (this.reviewTab === tab) this.reviewTab = undefined;
      });
      this.reviewTab = tab;
      this.pinned.set(tab.webview, repo);
      await tab.reload();
    }
    if (ai) await this.reviewTab?.ask();
  }

  /** Opens a file in conflict where it is resolved by hand: the Flow Merge, the merge editor of VS Code, or the file itself at a line. */
  private async openConflict(repo: Repository, file: string, line: number, editor: 'flowMerge' | 'mergeEditor' | 'text'): Promise<void> {
    if (typeof file !== 'string' || !(await reads.conflicted(repo)).includes(file)) return;
    const uri = vscode.Uri.file(path.join(repo.root, file));
    try {
      if (editor === 'flowMerge') return void (await vscode.commands.executeCommand('gitardo.openInFlowMerge', uri));
      const stages = await reads.conflictStages(repo, file);
      // The merge editor needs all three sides: a file both sides added has no base.
      if (editor === 'mergeEditor' && [1, 2, 3].every((stage) => stages.includes(stage))) {
        return await openMergeEditor(repo.root, file, await reads.operation(repo).catch(() => null));
      }
      if (editor !== 'text' && (await this.openDeletedConflict(repo, file, stages))) return;
    } catch (error) {
      this.log.warn(`Could not open ${file} to resolve it: ${errorText(error)}`);
    }
    const at = Math.max(0, (Number(line) || 1) - 1);
    await vscode.window.showTextDocument(uri, { selection: new vscode.Range(at, 0, at, 0), preview: false });
  }

  /**
   * A file one side deleted and the other changed, as what it was beside what the side that kept it made of it. False
   * when the file is not one of those.
   */
  private async openDeletedConflict(repo: Repository, file: string, stages: number[]): Promise<boolean> {
    if (!stages.includes(1) || stages.includes(2) === stages.includes(3)) return false;
    const [current, incoming] = CONFLICT_SIDES[(await reads.operation(repo).catch(() => null)) ?? 'none'];
    const [deleter, keeper] = stages.includes(2) ? [incoming, current] : [current, incoming];
    const lower = (label: string) => `${label[0].toLowerCase()}${label.slice(1)}`;
    const title = `${path.posix.basename(file)} (deleted by ${lower(deleter)}, changed by ${lower(keeper)})`;
    await vscode.commands.executeCommand('vscode.diff', contentUri(repo.root, ':1', file), vscode.Uri.file(path.join(repo.root, file)), title);
    return true;
  }

  /** Writes resolutions of conflicts, and says what is left: the engine's journal keeps how Undo puts the conflicts back. */
  private async writeResolutions(repo: Repository, resolutions: FileResolutions[]): Promise<boolean> {
    let written = false;
    const runs: EngineRun[] = [{ feature: 'review.write', target: { kind: 'resolutions', files: resolutions } }];
    await this.runOnEngine(runs, repo, {}, () => (written = true));
    return written;
  }

  /** The state of a repository the views may not show, with one commit only: for choosing among its branches. */
  private async stateOf(repo: Repository): Promise<RepoState> {
    const client = reads.client();
    const asked = { limit: 1, filter: null, scope: null, stashes: false, firstParent: false };
    return graphState(client, repo.root, await reads.parts(repo), asked, (graph) => readCommits(client, repo.root, graph, 1));
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

  /** Narrows the graph of a group to the history of some branches, or shows them all again. What the filter does is left to it. */
  async setScope(scope: Scope | null, on = this.followers): Promise<void> {
    await this.whenLocated();
    // As it is given: reading the repository leaves out what is not a branch of it.
    on.scope = scope && typeof scope === 'object' ? scope : null;
    on.limit = pageSize();
    on.cancelGraphRead();
    await this.reload(on, 'filter');
  }

  /** Reads the graph of a group again, or does what ends with that, telling its views what for while they wait. */
  private reload(on: ViewGroup, what: Loading, work = () => this.refresh(on)): Promise<void> {
    return on.reload(what, work);
  }

  /**
   * The scope as git took it, without the branches deleted since they were chosen, is the one in use in the group and
   * the one remembered for the repository, which the next group to show it starts with.
   */
  private settleScope(on: ViewGroup, root: string, scope: Scope | null): void {
    on.scope = scope;
    const all = this.memento.get<Record<string, Scope>>(SCOPE_KEY, {});
    if (JSON.stringify(all[root] ?? null) === JSON.stringify(scope)) return;
    const others = Object.fromEntries(Object.entries(all).filter(([other]) => other !== root));
    void this.memento.update(SCOPE_KEY, scope ? { ...others, [root]: scope } : others);
  }

  /** The favorites of a repository that are still branches of it, by their full names. */
  private favorites(state: Pick<RepoState, 'root' | 'branches' | 'remoteBranches'>): string[] {
    const known = new Set([...state.branches.map((branch) => `refs/heads/${branch.name}`), ...state.remoteBranches.map((ref) => `refs/remotes/${ref.name}`)]);
    const stored: unknown = this.memento.get<Record<string, unknown>>(FAVORITES_KEY, {})[state.root];
    return Array.isArray(stored) ? stored.filter((ref): ref is string => typeof ref === 'string' && known.has(ref)) : [];
  }

  /**
   * What the references of a repository list, as their own filter was last set there (`refs.listed`), without the
   * references gone since: null, everything, once none of those ticked is left. What is kept is written again only when
   * the choice changes.
   */
  private listedIn(state: Pick<RepoState, 'root' | 'head' | 'branches' | 'remoteBranches' | 'tags'>): Scope | null {
    return pruneScope(this.memento.get<Record<string, unknown>>(LISTED_KEY, {})[state.root], state);
  }

  /** The groups that show a repository now, each with its state. */
  private groupsShowing(root: string): ViewGroup[] {
    return this.allGroups().filter((group) => group.state && samePath(group.state.root, root));
  }

  /**
   * Chooses what the references list, in the repository a group shows, apart from the branches shown in the graph
   * (`refs.list-filter`). Nothing is read: the views of every group that shows it are told at once.
   */
  async setListed(listed: unknown, on = this.followers): Promise<void> {
    const state = await this.requireState(on);
    if (!state) return;
    const kept = pruneScope(listed, state);
    const others = Object.fromEntries(Object.entries(this.memento.get<Record<string, unknown>>(LISTED_KEY, {})).filter(([root]) => root !== state.root));
    await this.memento.update(LISTED_KEY, kept ? { ...others, [state.root]: kept } : others);
    for (const group of this.groupsShowing(state.root)) this.setState(group, { ...group.state!, listed: kept });
  }

  /** Marks a branch as a favorite, or no longer. Nothing of the repository changes: the views are told at once. */
  async setFavorite(ref: string, favorite: boolean, on = this.followers): Promise<void> {
    const state = await this.requireState(on);
    if (!state || typeof ref !== 'string') return;
    // The ones whose branch is gone are forgotten as soon as there is something to save.
    const others = this.favorites(state).filter((other) => other !== ref);
    const next = favorite ? [...others, ref] : others;
    await this.memento.update(FAVORITES_KEY, { ...this.memento.get<Record<string, unknown>>(FAVORITES_KEY, {}), [state.root]: next });
    for (const group of this.groupsShowing(state.root)) this.setState(group, { ...group.state!, favorites: this.favorites(group.state!) });
  }

  /** Narrows the graph of a group to the history of a file or to a search, or shows everything again. */
  async setFilter(filter: Filter | null, on = this.followers): Promise<void> {
    await this.whenLocated();
    on.filter = filter;
    on.limit = pageSize();
    on.cancelGraphRead();
    // Searching inside the changes of every commit can take a while on a large repository, and so can following lines.
    const slow = filter?.kind === 'pickaxe' || filter?.kind === 'regex' || filter?.kind === 'lines';
    if (slow) this.postToGroup(on, { type: 'busy', title: 'Searching' });
    try {
      await this.reload(on, 'filter');
    } finally {
      if (slow) this.postToGroup(on, { type: 'busy', title: null });
    }
  }

  /** Shows the commits that touched a file: the given one, or the one in the active editor. */
  async fileHistory(uri = vscode.window.activeTextEditor?.document.uri): Promise<void> {
    if (!(await this.requireState())) return;
    if (uri?.scheme === 'file') await this.showRepoOf(uri);
    if (!this.repo) return;
    const file = uri?.scheme === 'file' ? path.relative(this.repo.root, uri.fsPath).replace(/\\/g, '/') : '';
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
    if (!(await this.requireState())) return;
    const uri = editor?.document.uri;
    if (uri?.scheme === 'file') await this.showRepoOf(uri);
    const repo = this.repo;
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
    const committedAs = this.state?.workingTree.staged.find((change) => change.path === file)?.originalPath ?? file;
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
    if (!(await this.requireState())) return;
    if (uri?.scheme === 'file') {
      // The root folder of a repository is not in the repository: it is the repository.
      const own = this.repos.find((repo) => samePath(repo.root, uri.fsPath));
      if (own) await this.selectRepo(own.root);
      else await this.showRepoOf(uri);
    }
    if (!this.repo) return;
    const folder = uri?.scheme === 'file' ? path.relative(this.repo.root, uri.fsPath).replace(/\\/g, '/') : '..';
    if (folder.startsWith('..') || path.isAbsolute(folder)) {
      void vscode.window.showWarningMessage('Gitardo: choose a folder of this repository to see its history.');
      return;
    }
    await vscode.commands.executeCommand('gitardo.graph.focus');
    // The root folder of a repository has all of its history.
    await this.setFilter(folder ? { kind: 'path', path: folder, folder: true } : null);
  }

  /** In a workspace with several repositories a file decides which one to show: the innermost one that contains it. */
  private async showRepoOf(uri: vscode.Uri): Promise<void> {
    const contains = (root: string) => {
      const relative = path.relative(root, uri.fsPath);
      return relative !== '' && !relative.startsWith('..') && !path.isAbsolute(relative);
    };
    const owner = this.repos.filter((repo) => contains(repo.root)).sort((a, b) => b.root.length - a.root.length)[0];
    if (owner) await this.selectRepo(owner.root);
  }

  /**
   * Stashes from the list of changes VS Code keeps, in the repository these files are of: the files themselves, only
   * what is staged, or everything.
   */
  async stashFromScm(uris: vscode.Uri[], what: 'files' | 'staged' | 'all'): Promise<void> {
    if (!(await this.requireState())) return;
    const local = uris.filter((uri) => uri.scheme === 'file');
    if (local[0]) await this.showRepoOf(local[0]);
    const { state, repo } = this;
    if (!state || !repo) return;
    if (what !== 'files') return this.action({ kind: 'stashPush', staged: what === 'staged' || undefined });
    const files = local
      .map((uri) => path.relative(repo.root, uri.fsPath).replace(/\\/g, '/'))
      .filter((file) => file && !file.startsWith('..') && !path.isAbsolute(file));
    if (!files.length) return;
    // A renamed file is stashed with the name it had, or git would leave half of the rename behind.
    const before = new Map([...state.workingTree.staged, ...state.workingTree.unstaged].map((change) => [change.path, change.originalPath]));
    const paths = [...new Set(files.flatMap((file) => [file, before.get(file)]).filter((file) => typeof file === 'string'))];
    return this.action({ kind: 'stashPush', paths });
  }

  /**
   * Selects a commit in the graph of a group, bringing it into sight first: the panel's, or a tab's. `hash` is the full
   * one.
   */
  async reveal(hash: string, on = this.followers): Promise<void> {
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
    await this.whenLocated();
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
    await this.reload(on, same ? 'more' : 'filter');
    // The read gives its first page at once: the commit is shown once the pages that have it are read.
    if (on.repo === repo && on.state) {
      await on.graph.reached(repo.root, pages * page).catch(() => {});
      on.showPages();
    }
  }

  /** Selects, in the view that asked, the commit that a hash or anything else git understands stands for. */
  private async goTo(revision: unknown, asker: vscode.Webview, on: ViewGroup): Promise<void> {
    const repo = on.repo;
    if (!repo || typeof revision !== 'string' || !revision.trim()) return;
    const hash = await reads.resolve(repo, revision.trim());
    if (!hash) {
      // A few characters can be the start of more than one hash, which git refuses to choose between.
      void vscode.window.showWarningMessage(`Gitardo: "${revision.trim()}" is not a commit of this repository, or is the start of several.`);
      return;
    }
    await this.include(hash, on);
    this.post({ type: 'reveal', hash }, asker);
  }

  /**
   * Opens a commit in an editor tab of its own, with its message, its files and what it changed in them, or shows the
   * tab it has already. Without a hash it asks for one, or for anything else git understands.
   */
  async openCommit(hash?: unknown, repo = this.repo): Promise<void> {
    if (hash === undefined) {
      repo = ((await this.requireState()) && this.repo) || undefined;
      hash = repo && (await this.askRevision(repo, 'Open commit'))?.hash;
    }
    if (!repo || !isHash(hash)) return;
    const detail = await reads.commitDetail(repo, hash).catch(() => null);
    if (!detail) {
      void vscode.window.showWarningMessage(`Gitardo: ${hash.slice(0, 7)} is not a commit of this repository.`);
      return;
    }
    const commit = detail.id ?? hash;
    const key = `${repo.root}\n${commit}`;
    const open = this.commitTabs.get(key);
    if (open) return open.reveal();

    const { subject: whole } = subjectAndBody(detail.message);
    const subject = whole.length > TITLE_LENGTH ? `${whole.slice(0, TITLE_LENGTH).trimEnd()}…` : whole;
    const panel = vscode.window.createWebviewPanel('gitardo.commit', `${commit.slice(0, 7)} ${subject}`, vscode.ViewColumn.Active, {
      retainContextWhenHidden: true,
      // The search of VS Code, for a word among the changes.
      enableFindWidget: true,
    });
    const layout = this.memento.get<FilesLayout>(FILES_KEY, 'tree');
    // Kept at hand: a panel that is closed no longer says which webview was its own.
    const { webview } = panel;
    const attachment = this.attach(webview, 'commit', { commit, root: repo.root, files: layout });
    this.pinned.set(webview, repo);
    this.commitTabs.set(key, panel);
    panel.onDidDispose(() => {
      attachment.dispose();
      this.pinned.delete(webview);
      this.commitTabs.delete(key);
    });
  }

  /**
   * Opens the tab where what new worktrees get of what git ignores is chosen, for the repository the views show, or
   * shows the one open for it.
   */
  async openWorktreeSetup(on = this.followers): Promise<void> {
    const repo = (await this.requireState(on)) && on.repo;
    if (!repo) return;
    // Once it has been seen, creating a worktree no longer offers it.
    void this.memento.update(SETUP_OFFERED, true);
    if (this.setupTab && samePath(this.setupTab.repo.root, repo.root)) return this.setupTab.panel.reveal();
    this.setupTab?.panel.dispose();

    const panel = vscode.window.createWebviewPanel('gitardo.worktreeSetup', `New Worktrees · ${path.basename(repo.root)}`, vscode.ViewColumn.Active, {
      retainContextWhenHidden: true,
    });
    const { webview } = panel;
    const tab = { panel, repo, entries: null as IgnoredEntry[] | null, proposed: [] as CommandProposal[], saving: false };
    this.setupTab = tab;
    const attachment = this.attach(webview, 'worktreeSetup', { root: repo.root });
    this.pinned.set(webview, repo);
    // Settings changed elsewhere, in the Settings editor or settings.json, show here at once.
    const watching = vscode.workspace.onDidChangeConfiguration((event) => {
      const sections = [
        'gitardo.worktrees.linkFolders',
        'gitardo.worktrees.copyFiles',
        'gitardo.worktrees.setupCommands',
        'git.worktreeSymlinkFolders',
        'git.worktreeIncludeFiles',
      ];
      if (tab.saving) return;
      if (sections.some((section) => event.affectsConfiguration(section, vscode.Uri.file(repo.root)))) void this.sendSetup(webview, false);
    });
    panel.onDidDispose(() => {
      attachment.dispose();
      watching.dispose();
      this.pinned.delete(webview);
      if (this.setupTab === tab) this.setupTab = undefined;
    });
  }

  /** Sends the tab of the setup of new worktrees what it shows, looking again at what git ignores when `rescan`. */
  private async sendSetup(webview: vscode.Webview, rescan: boolean): Promise<void> {
    const tab = this.setupTab;
    if (!tab || tab.panel.webview !== webview) return;
    const { root } = tab.repo;
    try {
      if (rescan || !tab.entries) [tab.entries, tab.proposed] = await Promise.all([reads.setupIgnored(root), reads.setupCommands(root)]);
      // The other worktrees can be given what applies only while the views show this repository, which that runs in.
      const state = this.heldFor(root);
      const others = state?.worktrees.filter((worktree) => !worktree.isMissing && !samePath(worktree.folder, root)).length ?? 0;
      this.post({ type: 'worktreeSetup', info: { ...worktreeSetupInfo(root, others, tab.proposed), entries: tab.entries } }, webview);
    } catch (error) {
      this.log.error(errorText(error));
      this.post({ type: 'worktreeSetup', info: null, error: errorText(error) }, webview);
    }
  }

  /** Asks for a hash or a revision, and shows its commit in the graph of a group. */
  async goToCommit(on = this.followers): Promise<void> {
    const repo = (await this.requireState(on)) && on.repo;
    if (!repo) return;
    const commit = await this.askRevision(repo, 'Go to commit');
    if (commit) await this.reveal(commit.hash, on);
  }

  /** Tells a view which commits a search finds in the history its group shows, for it to highlight them where they are. */
  private async find(search: Search, asker: vscode.Webview, on: ViewGroup): Promise<void> {
    const repo = on.repo;
    const kinds = ['message', 'author', 'pickaxe', 'regex'];
    if (!repo || !kinds.includes(search?.kind) || typeof search.query !== 'string' || !search.query) return;
    try {
      // A list of results gives way to the graph, in which the new ones are to be shown.
      if (on.filter && 'query' in on.filter) await this.setFilter(null, on);
      const found = await this.matchesOf(on, repo, { kind: search.kind, query: search.query });
      this.post({ type: 'found', search, ...found }, asker);
    } catch (error) {
      const text = failureText(error);
      this.log.error(text);
      void vscode.window.showErrorMessage(`Gitardo: the search failed. ${text.split('\n')[0]}`);
      this.post({ type: 'found', search, hashes: [], more: false, failed: true }, asker);
    }
  }

  /** The views of a group that list commits. */
  private graphViews(on: ViewGroup): vscode.Webview[] {
    return [...this.views].filter(([view, kind]) => showsGraph(kind) && this.groupOf(view) === on).map(([view]) => view);
  }

  /** Selects a commit in the views of a group that list commits and are there, without opening any or taking the focus. */
  private show(hash: string, on = this.followers): void {
    if (!isHash(hash)) return;
    for (const view of this.graphViews(on)) this.post({ type: 'reveal', hash }, view);
  }

  /**
   * Brings out the history of a commit in the views of the graph of a group, or no longer, while the pointer rests on
   * its branch.
   */
  private trace(hash: string | null, on: ViewGroup): void {
    if (hash !== null && !isHash(hash)) return;
    for (const view of this.graphViews(on)) this.post({ type: 'trace', hash }, view);
  }

  /** Hands over what was waiting for a graph view of a group to be there. */
  private flushPending(on: ViewGroup): void {
    const lists = this.graphViews(on);
    if (on.pendingReveal && lists.length) {
      for (const view of lists) this.post({ type: 'reveal', hash: on.pendingReveal }, view);
      on.pendingReveal = undefined;
    }
  }

  /**
   * Compares two branches, tags or commits, or one with the working tree, in a tab, asking which. With `from`, only
   * the other side is asked. The first question also offers the comparisons opened last.
   */
  async compare(from?: ComparedRef, on = this.followers): Promise<void> {
    const state = await this.requireState(on);
    const repo = on.repo;
    if (!state || !repo) return;
    try {
      const given = from && isSide(from.hash) && typeof from.name === 'string' ? { name: from.name, hash: from.hash } : undefined;
      const first =
        given ??
        (await this.pickRevision(state, repo, 'Compare', 'The branch, tag or commit to start from, or a comparison opened before', {
          worktree: true,
          recent: this.recentComparisons(repo),
        }));
      if (!first) return;
      if ('to' in first) return await this.openRecent(first, repo);
      const second = await this.pickRevision(state, repo, `Compare ${first.name} with`, 'The branch, tag or commit to compare it with', {
        other: first,
        worktree: true,
      });
      if (!second || 'to' in second) return;
      await this.openCompare(first, second, repo);
    } catch (error) {
      void vscode.window.showErrorMessage(`Gitardo: ${errorText(error)}`);
    }
  }

  /**
   * Compares a file or a folder as it is in the working tree with a branch, tag or commit, asking which: the given
   * one, or the file in the active editor.
   */
  async compareWith(uri = vscode.window.activeTextEditor?.document.uri): Promise<void> {
    if (!(await this.requireState())) return;
    if (uri?.scheme === 'file') await this.showRepoOf(uri);
    const { state, repo } = this;
    if (!state || !repo) return;
    const file = uri?.scheme === 'file' ? path.relative(repo.root, uri.fsPath).replace(/\\/g, '/') : '';
    if (!uri || file.startsWith('..') || path.isAbsolute(file)) {
      void vscode.window.showWarningMessage('Gitardo: choose a file or a folder of this repository to compare.');
      return;
    }
    try {
      const name = file || path.basename(repo.root);
      const picked = await this.pickRevision(state, repo, `Compare ${name} with`, 'The branch, tag or commit to compare the working tree with');
      if (!picked || 'to' in picked) return;
      await this.openCompare(picked, WORKTREE_SIDE, repo, file || null);
    } catch (error) {
      void vscode.window.showErrorMessage(`Gitardo: ${errorText(error)}`);
    }
  }

  /**
   * Compares a file listed in a commit, in a comparison or in the changes not committed, with the branch, tag, commit or
   * working tree the user chooses, in the diff editor. A file the commit deleted is compared as it was before.
   */
  private async compareFile(source: DiffSource, file: FileChange, names: { hash?: string; parent?: string } = {}, repo = this.repo): Promise<void> {
    if (!repo || typeof file?.path !== 'string') return;
    // The branches offered are those of the repository the file is in, which no view may show.
    const state = this.heldFor(repo.root) ?? (await this.stateOf(repo).catch(() => null));
    if (!state) return;
    const deleted = file.kind === 'deleted';
    const head = state.head.commit;
    let at: ComparedRef | undefined;
    if (source?.kind === 'worktree') at = !deleted ? WORKTREE_SIDE : head ? { name: checkedOutBranch(state) ?? 'HEAD', hash: head } : undefined;
    else if (source?.kind === 'commit') {
      const hash = deleted ? source.parent : source.hash;
      const named = deleted ? names?.parent : names?.hash;
      if (hash === WORKTREE) at = WORKTREE_SIDE;
      else if (hash && isHash(hash)) at = { name: typeof named === 'string' && named ? named : hash.slice(0, 7), hash };
    }
    if (!at) return;
    const name = path.posix.basename(file.path);
    try {
      const picked = await this.pickRevision(state, repo, `Compare ${name} of ${at.name} with`, 'The branch, tag or commit to compare it with', {
        other: at,
        worktree: true,
      });
      if (!picked || 'to' in picked) return;
      // The working tree is on the right, where its file can be edited, as in every other diff.
      const [left, right] = picked.hash === WORKTREE ? [at, picked] : [picked, at];
      const uri = (side: ComparedRef) => (side.hash === WORKTREE ? vscode.Uri.file(path.join(repo.root, file.path)) : contentUri(repo.root, side.hash, file.path));
      await vscode.commands.executeCommand('vscode.diff', uri(left), uri(right), `${name} (${left.name} ↔ ${right.name})`);
    } catch (error) {
      void vscode.window.showErrorMessage(`Gitardo: ${errorText(error)}`);
    }
  }

  /**
   * Has new worktrees get the files and folders chosen in the Explorer linked, copied, or nothing by a pattern of their
   * own path, in the settings that apply to the repository each one is in, and says what became of each.
   */
  async setUpFromExplorer(choice: SetupChoice, chosen: vscode.Uri[]): Promise<void> {
    await this.whenLocated();
    const owner = (uri: vscode.Uri) => {
      const inside = this.repos.filter((repo) => {
        const relative = path.relative(repo.root, uri.fsPath);
        return relative !== '' && !relative.startsWith('..') && !path.isAbsolute(relative);
      });
      return inside.sort((a, b) => b.root.length - a.root.length)[0];
    };
    const said: string[] = [];
    let changed = 0;
    let saved: SetupTarget | undefined;
    const byRepo = new Map<string, vscode.Uri[]>();
    for (const uri of chosen.filter((each) => each.scheme === 'file')) {
      const repo = owner(uri);
      if (!repo) said.push(`${path.basename(uri.fsPath)} is in no repository of the workspace.`);
      else byRepo.set(repo.root, [...(byRepo.get(repo.root) ?? []), uri]);
    }
    try {
      await vscode.window.withProgress({ location: vscode.ProgressLocation.Window, title: 'Gitardo: looking at what git ignores' }, async () => {
        for (const [root, uris] of byRepo) {
          const { linkFrom, copyFrom, ...before } = setupPatterns(root);
          let patterns: SetupPatterns = before;
          for (const uri of uris) {
            const entry = { path: path.relative(root, uri.fsPath).split(path.sep).join('/'), folder: ((await vscode.workspace.fs.stat(uri)).type & vscode.FileType.Directory) !== 0 };
            const result = await changeSetup(root, entry, choice, patterns);
            patterns = result.patterns;
            if (result.change.kind === 'done' || result.change.kind === 'still') changed++;
            said.push(setupChangeText(entry.path, entry.folder, choice, result.change));
          }
          for (const [kind, from] of [['link', linkFrom], ['copy', copyFrom]] as const) {
            const same = patterns[kind].length === before[kind].length && patterns[kind].every((pattern, index) => pattern === before[kind][index]);
            if (!same) saved = await savePatterns(root, kind, patterns[kind], from);
          }
        }
      });
    } catch (error) {
      said.push(errorText(error));
    }
    if (!said.length) return;
    const where = saved && ` Saved in the settings of ${saved === 'user' ? 'the user' : 'the workspace folder'}.`;
    const text = `Gitardo: ${said.join(' ')}${where ?? ''}`;
    const open = 'Choose What New Worktrees Get…';
    const show = changed ? vscode.window.showInformationMessage : vscode.window.showWarningMessage;
    if ((await show(text, open)) !== open) return;
    const [first] = byRepo.values();
    if (first) await this.showRepoOf(first[0]);
    await this.openWorktreeSetup();
  }

  /** Opens a comparison opened before, its branches where they are now. */
  private async openRecent(recent: RecentComparison, repo: Repository): Promise<void> {
    const now = async (ref: ComparedRef): Promise<ComparedRef> =>
      ref.hash === WORKTREE || ref.hash.startsWith(ref.name) ? ref : { name: ref.name, hash: (await reads.resolve(repo, ref.name)) ?? ref.hash };
    await this.openCompare(await now(recent.from), await now(recent.to), repo, recent.path ?? null);
  }

  /**
   * Opens the comparison of two branches, tags or commits, or of one with the working tree, in a tab of its own, or
   * shows the tab it has. With `path`, only that file or folder.
   */
  async openCompare(from: ComparedRef, to: ComparedRef, repo = this.repo, path: string | null = null): Promise<void> {
    if (!repo || !isSide(from?.hash) || !isSide(to?.hash) || (from.hash === WORKTREE && to.hash === WORKTREE)) return;
    const narrowed = cleanPath(path);
    const [first, second] = await Promise.all([this.compareSide(repo, from), this.compareSide(repo, to)]);
    this.rememberComparison(repo, first, second, narrowed);
    const same = (a: CompareSide, b: CompareSide) => a.name === b.name && a.live === b.live && (a.live || a.hash === b.hash);
    for (const tab of this.compareTabs.values()) {
      if (samePath(tab.repo.root, repo.root) && same(tab.from, first) && same(tab.to, second) && tab.path === narrowed) return tab.panel.reveal();
    }

    const panel = vscode.window.createWebviewPanel('gitardo.compare', compareTitle(first, second, narrowed), vscode.ViewColumn.Active, {
      retainContextWhenHidden: true,
      // The search of VS Code, for a word among the changes.
      enableFindWidget: true,
    });
    const { webview } = panel;
    const tab: CompareTab = { panel, repo, from: first, to: second, mode: compareMode(first, second), path: narrowed };
    const attachment = this.attach(webview, 'compare', { root: repo.root, files: this.memento.get<FilesLayout>(FILES_KEY, 'tree') });
    this.pinned.set(webview, repo);
    this.compareTabs.set(webview, tab);
    panel.onDidDispose(() => {
      attachment.dispose();
      this.pinned.delete(webview);
      this.compareTabs.delete(webview);
    });
  }

  /**
   * A side of a comparison: followed when git gives its name the commit it was chosen at, fixed when it is a hash. The
   * working tree is followed as it changes.
   */
  private async compareSide(repo: Repository, ref: ComparedRef): Promise<CompareSide> {
    if (ref.hash === WORKTREE) return { name: WORKTREE_SIDE.name, hash: WORKTREE, live: true };
    const name = typeof ref.name === 'string' && ref.name ? ref.name : ref.hash.slice(0, 7);
    // A name that git reads as another commit, a tag that has the name of a branch, stands for the one chosen.
    const live = !ref.hash.startsWith(name) && (await reads.resolve(repo, name)) === ref.hash;
    return { name, hash: ref.hash, live };
  }

  /** The comparisons opened last in a repository, the latest first. */
  private recentComparisons(repo: Repository): RecentComparison[] {
    return this.memento.get<Record<string, RecentComparison[]>>(RECENT_KEY, {})[repo.root] ?? [];
  }

  private rememberComparison(repo: Repository, from: CompareSide, to: CompareSide, path: string | null): void {
    const all = this.memento.get<Record<string, RecentComparison[]>>(RECENT_KEY, {});
    const entry: RecentComparison = { from: { name: from.name, hash: from.hash }, to: { name: to.name, hash: to.hash }, ...(path ? { path } : {}) };
    // The two ways of a comparison are one: the last is kept.
    const same = (other: RecentComparison) =>
      (other.path ?? null) === path &&
      ((other.from.name === from.name && other.to.name === to.name) || (other.from.name === to.name && other.to.name === from.name));
    const list = [entry, ...(all[repo.root] ?? []).filter((other) => !same(other))].slice(0, RECENT_COMPARISONS);
    void this.memento.update(RECENT_KEY, { ...all, [repo.root]: list });
  }

  /**
   * Sends the tab of a comparison what it shows, once its sides are looked up again, and then the lines of its files.
   * Nothing is sent when it shows that already, unless `again`; a comparison with the working tree is read again
   * whenever the repository is.
   */
  private async sendCompare(tab: CompareTab, again = false): Promise<void> {
    const { panel, repo } = tab;
    const { webview } = panel;
    // Its repository is read again only while the other views show it.
    if (!again && !this.heldFor(repo.root)) return;
    // A tab closed while git answered is not there to be told.
    const open = () => this.compareTabs.get(webview) === tab;
    try {
      for (const side of [tab.from, tab.to]) {
        if (!side.live || side.hash === WORKTREE) continue;
        const hash = await reads.resolve(repo, side.name);
        if (!hash) side.gone = true;
        else {
          side.hash = hash;
          delete side.gone;
        }
      }
      const { from, to, mode, path } = tab;
      const changing = from.hash === WORKTREE || to.hash === WORKTREE;
      const key = `${from.hash} ${to.hash} ${mode} ${from.gone} ${to.gone} ${path}`;
      if (!open() || (!again && !changing && tab.shown === key)) return;
      tab.shown = key;
      panel.title = compareTitle(from, to, path);
      // What the user changed shows a wait until it is compared; the working tree compared again as it changes does not.
      if (again) this.post({ type: 'loading', what: 'filter' }, webview);
      const found = await this.activity.track(`Comparing ${panel.title}`, () => reads.compareSides(repo, from.hash, to.hash, mode, path));
      // The sides changed again meanwhile: the answer for them is the one to send, and the end of any wait.
      if (tab.shown !== key || !open()) return;
      this.post({ type: 'loading', what: null }, webview);
      if (!found) return this.post({ type: 'compareView', view: null, error: 'These are not commits of this repository.' }, webview);
      tab.since = found.since;
      this.post({ type: 'compareView', view: { from: { ...from }, to: { ...to }, mode, ...found } }, webview);
      const patch = await reads.comparePatch(repo, found.files, found.since, to.hash, path).catch(() => null);
      if (tab.shown === key && open()) this.post({ type: 'comparePatch', since: found.since, to: to.hash, patch }, webview);
    } catch (error) {
      tab.shown = undefined;
      if (!open()) return;
      this.log.error(errorText(error));
      this.post({ type: 'loading', what: null }, webview);
      this.post({ type: 'compareView', view: null, error: errorText(error) }, webview);
    }
  }

  /** Lets the user choose one side of a comparison again, in its tab. */
  private async pickCompareSide(tab: CompareTab, which: 'from' | 'to'): Promise<void> {
    // The branches to choose from are of the repository of the tab, which the other views may no longer show.
    const state = this.heldFor(tab.repo.root) ?? (await this.stateOf(tab.repo).catch(() => null));
    if (!state) return;
    const other = which === 'from' ? tab.to : tab.from;
    const title = which === 'from' ? `Compare with ${other.name}` : `Compare ${other.name} with`;
    const picked = await this.pickRevision(state, tab.repo, title, `The branch, tag or commit to put in place of ${tab[which].name}`, {
      other,
      worktree: other.hash !== WORKTREE,
    });
    if (!picked || 'to' in picked) return;
    tab[which] = await this.compareSide(tab.repo, picked);
    // As when the tab opened.
    tab.mode = compareMode(tab.from, tab.to);
    this.rememberComparison(tab.repo, tab.from, tab.to, tab.path);
    await this.sendCompare(tab, true);
  }

  /**
   * Makes a file in the working tree and the index what the second side of a comparison has: its version of the file,
   * or no file when it has none.
   */
  private async takeCompareFile(tab: CompareTab, file: FileChange): Promise<void> {
    const { to, repo } = tab;
    if (to.hash === WORKTREE || typeof file?.path !== 'string' || !file.path) return;
    // The engine asks before it overwrites what is not committed in the file.
    await this.runOnEngine([{ feature: 'compare.take-file', target: { ...this.comparisonTarget(tab), file } }], repo);
  }

  /** Copies the changes of a comparison as a patch that `git apply` takes. */
  private async copyComparePatch(tab: CompareTab): Promise<void> {
    if (!tab.since) return;
    const text = await reads.compareText(tab.repo, tab.since, tab.to.hash, tab.path).catch(() => '');
    if (!text) {
      void vscode.window.showInformationMessage('Gitardo: there are no changes to copy.');
      return;
    }
    await vscode.env.clipboard.writeText(text);
    const files = text.match(/^diff --git /gm)?.length ?? 0;
    void vscode.window.showInformationMessage(`Gitardo: copied the changes of ${files} ${files === 1 ? 'file' : 'files'} as a patch.`);
  }

  /**
   * Copies commits as one patch that `git am` takes, oldest first, each with its author, date and message, or saves it
   * to a file the user chooses. Git makes no patch of a merge commit.
   */
  private async formatPatch(repo: Repository, hashes: unknown, save: boolean): Promise<void> {
    if (!Array.isArray(hashes) || !hashes.length || !hashes.every(isHash)) return;
    // The engine makes the patch, and puts it on the clipboard or in the file the save dialog chose.
    await this.runOnEngine([{ feature: save ? 'patch.save' : 'patch.copy', target: { kind: 'commits', commits: hashes } }], repo);
  }

  /**
   * Opens the comparison on the hosting service of the main remote, each side by the name it has there: a branch by its
   * upstream there, a remote branch of that remote and a tag by their names, anything else by its commit, which the
   * service knows once it is pushed (`compare.open-on-service`, which is `compare.open-on-github` on GitHub).
   */
  private async openCompareOnService(tab: CompareTab): Promise<void> {
    const hosting = this.sessions.find(tab.repo.root)?.hosting;
    const { from, to } = tab;
    if (!hosting || from.hash === WORKTREE || to.hash === WORKTREE) return;
    // The engine names each side on the service and opens the address.
    await this.runOnEngine([{ feature: 'compare.open-on-service', target: this.comparisonTarget(tab) }], tab.repo);
  }

  /** Lets the user choose a branch or a tag, or type anything git can turn into a commit: a hash, `HEAD~3`, `v1.0^`. */
  private async pickRevision(
    state: RepoState,
    repo: Repository,
    title: string,
    placeHolder: string,
    options: {
      /** The side chosen already, which is not offered again. */
      other?: ComparedRef;
      /** Offers the working tree too. */
      worktree?: boolean;
      /** Comparisons to open again, offered first. */
      recent?: RecentComparison[];
    } = {},
  ): Promise<ComparedRef | RecentComparison | undefined> {
    const { other, worktree, recent = [] } = options;
    type Item = vscode.QuickPickItem & { ref?: ComparedRef; recent?: RecentComparison };
    const group = (label: string, refs: { name: string; commit: string }[], icon: string): Item[] =>
      refs.length
        ? [
            { label, kind: vscode.QuickPickItemKind.Separator },
            ...refs.map((ref) => ({ label: `$(${icon}) ${ref.name}`, description: ref.commit.slice(0, 7), ref: { name: ref.name, hash: ref.commit } })),
          ]
        : [];
    const items: Item[] = recent.length
      ? [
          { label: 'Opened before', kind: vscode.QuickPickItemKind.Separator },
          ...recent.map((entry) => ({ label: `$(history) ${entry.from.name} ↔ ${entry.to.name}`, description: entry.path, recent: entry })),
          { label: '', kind: vscode.QuickPickItemKind.Separator },
        ]
      : [];
    items.push({ label: '$(edit) Enter a hash or a revision…', description: 'such as 1a2b3c4 or HEAD~3', alwaysShow: true });
    const head = state.head.commit;
    if (head) {
      const name = checkedOutBranch(state) ?? 'HEAD';
      items.push({ label: `$(target) ${name}`, description: `${head.slice(0, 7)} · checked out`, ref: { name, hash: head } });
    }
    if (worktree && other?.hash !== WORKTREE) {
      items.push({ label: `$(files) ${WORKTREE_SIDE.name}`, description: 'with the changes not committed', ref: WORKTREE_SIDE });
    }
    items.push(
      ...group(
        'Branches',
        state.branches.filter((branch) => !branch.isCheckedOut),
        'git-branch',
      ),
      ...group('Remote branches', state.remoteBranches, 'cloud'),
      ...group('Tags', state.tags, 'tag'),
    );
    // Comparing something with itself shows nothing.
    const offered = items.filter((item) => !item.ref || item.ref.name !== other?.name || item.ref.hash !== other.hash);
    const choice = await vscode.window.showQuickPick(offered, { title, placeHolder, matchOnDescription: true });
    if (!choice) return;
    return choice.recent ?? choice.ref ?? this.askRevision(repo, title);
  }

  /** Lets the user type a hash, or anything else git can turn into a commit, and says which commit that is. */
  private async askRevision(repo: Repository, title: string): Promise<ComparedRef | undefined> {
    const revision = (
      await vscode.window.showInputBox({
        title,
        prompt: 'A commit hash, or anything else git understands: HEAD~3, v1.0^, main@{yesterday}',
        validateInput: async (value) =>
          !value.trim() || (await reads.resolve(repo, value.trim())) ? undefined : 'Git does not know a commit by this name in this repository.',
      })
    )?.trim();
    if (!revision) return;
    // The box can be accepted before the last check has come back.
    const hash = await reads.resolve(repo, revision);
    if (!hash) {
      void vscode.window.showWarningMessage(`Gitardo: "${revision}" is not a commit of this repository.`);
      return;
    }
    return { name: isHash(revision) ? hash.slice(0, 7) : revision, hash };
  }

  /** Runs a command of the toolbar or of the Command Palette, on the repository of a group: the panel's unless another is given. */
  async command(command: UiCommand, on = this.followers): Promise<void> {
    try {
      if (command === 'showLog') return this.log.show();
      if (command === 'activeFileHistory') return await this.fileHistory();
      if (command === 'signInGitHub') return await this.signIn(on);
      if (command === 'signInGitLab') return await this.runHostingFeature('hosting.load-merge-requests', on);
      if (command === 'checkOutPullRequest') return await this.runHostingFeature('hosting.checkout-pull-request', on);
      if (command === 'reflog') return await this.browseReflog(on);
      if (command === 'goToCommit') return await this.goToCommit(on);
      if (command === 'cleanupBranches') return await this.cleanupBranches(on);
      if (command === 'draftMessage') return await this.draftMessage(on);
      if (command === 'switchIdentity') return await this.switchIdentity(on);
      if (command === 'selectRepository') return await this.pickRepo(on);
      if (command === 'remotes') return await this.manageRemotes(false, on);
      if (command === 'switchWorktree') return await this.switchWorktree(on);
      if (command === 'worktreeSetup') return await this.openWorktreeSetup(on);
    } catch (error) {
      void vscode.window.showErrorMessage(`Gitardo: ${errorText(error)}`);
    }
  }

  /**
   * Lists the worktrees with how they stand: the branch against its upstream, the files not committed, the pull request.
   * The one picked opens where the settings say; the last entry creates another.
   */
  private async switchWorktree(on: ViewGroup): Promise<void> {
    const state = await this.requireState(on);
    if (!state) return;
    const create = { label: '$(add) Create worktree…', path: '', current: false };
    const worktrees = state.worktrees
      .filter((worktree) => !worktree.isMissing)
      .map((worktree) => {
        const { ahead, pull } = worktreeStatus(worktree, state);
        const description = [
          [worktreeBranch(worktree) ?? `detached at ${(worktree.commit ?? '0000000').slice(0, 7)}`, ahead].filter(Boolean).join(' '),
          worktree.uncommitted ? `$(edit) ${worktree.uncommitted}` : undefined,
          pull && `$(git-pull-request) #${pull.number}`,
          worktree.isCurrent && 'this window',
        ];
        return {
          label: `$(${worktree.isMain ? 'home' : 'worktree'}) ${worktreeLabel(worktree.folder, state.worktreesRoot)}`,
          description: description.filter(Boolean).join(' · '),
          detail: worktree.folder,
          path: worktree.folder,
          current: worktree.isCurrent,
        };
      });
    // The one open here is listed last: it is where the user already is.
    worktrees.sort((a, b) => Number(a.current) - Number(b.current));
    const picked = await vscode.window.showQuickPick([...worktrees, { label: '', kind: vscode.QuickPickItemKind.Separator, path: '', current: false }, create], {
      title: 'Switch Worktree',
      placeHolder: 'The worktree to open',
      matchOnDescription: true,
      matchOnDetail: true,
    });
    if (!picked || picked.current) return;
    await this.action(picked === create ? { kind: 'worktreeAdd' } : { kind: 'worktreeOpen', path: picked.path }, on);
  }

  /** Lists where HEAD has been, and offers what can be done with the chosen position, even if no branch reaches it any more. */
  private async browseReflog(on: ViewGroup): Promise<void> {
    const state = await this.requireState(on);
    if (!state || !on.repo) return;
    const entries = await reads.reflog(on.repo, 'HEAD', REFLOG_ENTRIES);
    const entry = await vscode.window.showQuickPick(
      entries.map((item) => ({
        label: item.subject,
        description: `${item.hash.slice(0, 7)} · ${item.selector}`,
        detail: new Date(item.date * 1000).toLocaleString(),
        item,
      })),
      { title: 'Reflog: where HEAD has been', placeHolder: 'Pick an entry to go back to it or recover it', matchOnDescription: true },
    );
    if (!entry) return;
    const { hash } = entry.item;
    const short = hash.slice(0, 7);
    const choice = await vscode.window.showQuickPick(
      [
        { label: 'Show in the graph', run: () => this.reveal(hash, on) },
        { label: 'Create branch here…', run: () => this.action({ kind: 'createBranch', at: hash }, on) },
        { label: `Reset ${checkedOutBranch(state) ?? 'HEAD'} to here…`, run: () => this.action({ kind: 'reset', hash }, on) },
        { label: 'Check out (detached)', run: () => this.action({ kind: 'checkout', target: 'detached', ref: hash }, on) },
        { label: 'Cherry-pick', run: () => this.action({ kind: 'cherryPick', hash, isMerge: false }, on) },
      ],
      { title: `${short}: ${entry.item.subject}` },
    );
    await choice?.run();
  }

  /** Offers to delete the local branches that are merged into the current one, or whose upstream is gone. */
  private async cleanupBranches(on: ViewGroup): Promise<void> {
    const state = await this.requireState(on);
    if (!state || !on.repo) return;
    // Once the plan is confirmed, VS Code's Git first lets go of the worktrees it removes.
    await this.runOnEngine([{ feature: 'branch.clean-up', target: { kind: 'head' } }], on.repo, { confirmed: (plan) => this.closeRemoved(plan) }, undefined, on);
  }

  /** Closes in VS Code's Git the worktrees a plan of the engine removes, which Windows would not let git delete. */
  private async closeRemoved(plan: Plan): Promise<void> {
    const removed = plan.steps.flatMap((step) => {
      const command = step.kind === 'git' && Array.isArray(step.arguments) ? (step.arguments as string[]) : [];
      return command[0] === 'worktree' && command[1] === 'remove' ? [command[command.length - 1]] : [];
    });
    if (removed.length) await closeInVSCodeGit(removed, this.log);
  }

  /** Asks the language model for a commit message and hands it to the Changes view. */
  private async draftMessage(on: ViewGroup): Promise<void> {
    const state = await this.requireState(on);
    const repo = on.repo;
    if (!state || !repo) return;
    this.post({ type: 'drafting', active: true });
    try {
      const message = await vscode.window.withProgress(
        { location: vscode.ProgressLocation.Window, title: 'Gitardo: drafting commit message', cancellable: true },
        (_progress, token) => this.draftWith(repo, state, token),
      );
      this.post({ type: 'drafting', active: false, message });
    } catch (error) {
      this.post({ type: 'drafting', active: false });
      throw error;
    }
  }

  /**
   * Runs an operation of the pull requests of the hosting service of the main remote on the engine, which refuses it,
   * saying why, when the remote is not on a service that has it (`hosting.load-merge-requests`,
   * `hosting.checkout-pull-request`).
   */
  private async runHostingFeature(feature: 'hosting.load-merge-requests' | 'hosting.checkout-pull-request', on: ViewGroup): Promise<void> {
    const repo = on.repo;
    if (!repo) return;
    await this.runOnEngine([{ feature, target: { kind: 'head' } }], repo, {}, undefined, on);
  }

  /**
   * Signs in to the hosting service of the main remote of a group's repository, and loads its pull requests with the
   * engine's `github.load-pull-requests` or `hosting.load-merge-requests`, which read them again.
   */
  private async signIn(on: ViewGroup): Promise<void> {
    const hosting = on.session?.hosting;
    const repo = on.repo;
    if (!hosting || !readsPullRequests(hosting.service)) {
      void vscode.window.showWarningMessage('Gitardo: the main remote of this repository is not on github.com or on a GitLab.');
      return;
    }
    if (!repo) return;
    const gitLab = hosting.service === 'gitLab';
    const words = `${pullWords(hosting.service)}s`;
    try {
      await this.runOnEngine([{ feature: gitLab ? 'hosting.load-merge-requests' : 'github.load-pull-requests', target: { kind: 'head' } }], repo, {}, undefined, on);
    } catch (error) {
      this.log.warn(`Could not read ${words}: ${errorText(error)}`);
      void vscode.window.showErrorMessage(`Gitardo: could not read ${words}. ${errorText(error)}`);
    }
  }

  // AI and GitHub through the engine.

  /**
   * The draft of a commit message, by the engine (`commit.draft-message`), from the subjects of the last commits the
   * timeline lists, without the draft and without stashes.
   */
  private async draftWith(repo: Repository, state: RepoState, token: vscode.CancellationToken): Promise<string> {
    const client = reads.client();
    const head = state.head.commit;
    const asked = { limit: DRAFT_SUBJECTS, filter: null, scope: null, names: [], stashes: false, firstParent: false, head, refs: null, stashList: null };
    const subjects = (await readCommits(client, repo.root, asked, DRAFT_SUBJECTS)).commits.map((commit) => commit.subject);
    const aborted = new AbortController();
    const listener = token.onCancellationRequested(() => aborted.abort());
    try {
      const drafted = await client.draftMessage(repo.root, subjects, aborted.signal);
      if (drafted.kind === 'failed') throw new Error(drafted.message);
      return drafted.message;
    } catch (error) {
      // Cancelled, the box keeps what it had, as when the extension drafted it.
      if (error instanceof EngineError && error.code === ErrorCode.cancelled) throw new vscode.CancellationError();
      throw error;
    } finally {
      listener.dispose();
    }
  }

  /**
   * Asks which language model resolves conflicts, and how much it thinks: `ai.choose-model` of the engine, which lists
   * VS Code's models and leaves the two settings to write.
   */
  async chooseConflictModel(on = this.followers): Promise<void> {
    const repo = (await this.requireState(on)) && on.repo;
    if (repo) await this.runOnEngine([{ feature: 'ai.choose-model', target: { kind: 'head' } }], repo, {}, undefined, on);
  }

  /** Writes a setting the engine left, in the user's settings, by the extension's name; null removes it. */
  private async writeSetting(followUp: WriteSettingFollowUp): Promise<void> {
    const value = followUp.value === null ? undefined : followUp.value;
    await vscode.workspace.getConfiguration('gitardo').update(extensionKey(followUp.key), value, vscode.ConfigurationTarget.Global);
  }

  /** A comparison tab as the target of `compare.open-on-service` and, with a file, of `compare.take-file`. */
  private comparisonTarget(tab: CompareTab): ComparisonTarget {
    const side = (compared: CompareSide): CompareSideTarget => ({
      name: compared.name,
      side: compared.hash === WORKTREE ? { kind: 'workingTree' } : { kind: 'commit', commit: compared.hash },
      isFollowed: compared.live,
      isGone: compared.gone ?? false,
    });
    return {
      kind: 'comparison',
      first: side(tab.from),
      second: side(tab.to),
      mode: tab.mode === 'diverged' ? 'sinceDiverged' : 'everything',
      path: tab.path,
      mergeBase: null,
    };
  }

  /**
   * Opens the pull request of a branch in the browser, or the page of the hosting service that creates one, whose
   * address the engine gives (`hosting.open-pull-request`, `hosting.web-links`).
   */
  private async openPullRequest(branch: string, on: ViewGroup): Promise<void> {
    const repo = on.repo;
    if (!on.session?.hosting || !repo) return;
    const known = on.session.pulls[branch]?.url;
    const url = known ?? (await reads.webLink(repo.root, { kind: 'create', branch }).catch((error) => void this.log.warn(`No address: ${errorText(error)}`)));
    if (url) await vscode.env.openExternal(vscode.Uri.parse(url));
  }

  /** Checks out the pull request a remote branch has on its label, by its number (`hosting.checkout-pull-request`). */
  private async checkOutPullRequest(number: number, remoteBranch: string, on: ViewGroup): Promise<void> {
    const repo = on.repo;
    if (!repo || !on.session?.hosting?.canCheckOut) return;
    const answers = { number: { kind: 'text' as const, text: String(number) } };
    await this.runOnEngine([{ feature: 'hosting.checkout-pull-request', target: { kind: 'remoteBranch', name: remoteBranch }, answers }], repo, {}, undefined, on);
  }

  /** What the Pull button runs: the settings add `--rebase` and `--autostash`, unless switched off. */
  private defaultPull(): PullAction {
    const config = vscode.workspace.getConfiguration('gitardo.pull');
    const rebase = config.get<boolean>('rebase') !== false;
    const autostash = config.get<boolean>('autostash') !== false;
    if (rebase) return autostash ? 'pullRebaseAutostash' : 'pullRebase';
    return autostash ? 'pullAutostash' : 'pull';
  }

  /** Pulls the way the settings choose. */
  pull(): Promise<void> {
    return this.sync(this.defaultPull());
  }

  /** Fetches, pulls, pushes or syncs the repository a group shows: the panel's unless another is given. */
  async sync(action: SyncAction, on = this.followers): Promise<void> {
    const state = await this.requireState(on);
    if (!state) return;
    // Fetch, pull, push and Sync are the engine's.
    const repo = on.repo;
    if (repo && (await this.runOnEngine(syncRuns(action, this.defaultPull()), repo, {}, undefined, on))) await this.afterSync(action, state, on);
  }

  /** What is said after a pull or a push, on either path, from the state before it and the state read after. */
  private async afterSync(action: SyncAction, before: RepoState, on: ViewGroup): Promise<void> {
    const after = on.state && samePath(on.state.root, before.root) ? on.state : null;
    // The changes that were set aside no longer fit on what came in: git leaves them in conflict in the files, and
    // keeps the stash it made, which nothing says unless its output is read.
    const kept = after?.stashes[0];
    if (kept && after.stashes.length > before.stashes.length && /autostash/i.test(kept.message)) {
      void vscode.window
        .showWarningMessage(
          `Gitardo: pulled, but your changes do not fit on what came in. They are in conflict in the files, and whole in the stash "${kept.message}": resolve the conflicts, then drop the stash.`,
          'Show Stashes',
        )
        .then((choice) => choice && vscode.commands.executeCommand('gitardo.openStashes'));
    }
    // A fetch that found the upstream of a branch force pushed says so, with what it changed (M36).
    const forced = after && forcePushed(before, after);
    if (forced && on.repo) {
      const repo = on.repo;
      void vscode.window.showInformationMessage(`Gitardo: ${forced} was force pushed.`, SEE_WHAT_CHANGED).then((choice) => {
        if (choice) void this.runOnEngine([{ feature: COMPARE_REWRITE, target: { kind: 'remoteBranch', name: forced } }], repo, {}, undefined, on);
      });
    }
    // The first push of a branch is the natural moment to open a pull request for it: it had no upstream, and has one now.
    const branch = checkedOutBranch(before);
    const hadUpstream = before.branches.some((each) => each.isCheckedOut && each.upstream);
    const hasUpstream = after?.branches.some((each) => each.isCheckedOut && each.name === branch && each.upstream);
    const hosting = on.session?.hosting;
    if ((action.startsWith('push') || action === 'sync') && branch && !hadUpstream && hasUpstream && hosting && !on.session?.pulls[branch]) {
      const button = hosting.service === 'gitLab' ? 'Create Merge Request' : 'Create Pull Request';
      const choice = await vscode.window.showInformationMessage(`Pushed ${branch}.`, button);
      if (choice) await this.openPullRequest(branch, on);
    }
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
    if (action.kind === 'resolveConflicts' && !this.autoResolve()) {
      void vscode.window
        .showInformationMessage('Gitardo: resolving conflicts automatically is experimental, and it is switched off in the settings.', 'Open Settings')
        .then((choice) => choice && vscode.commands.executeCommand('workbench.action.openSettings', 'gitardo.conflicts.autoResolve'));
      return;
    }
    if (action.kind === 'checkTags') return this.checkTags(state, repo);
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
    await this.letGoInVSCodeGit(action, state);
    // The clipboard is VS Code's: the text of a patch in it is read here and given to the engine.
    const first = runs[0];
    if (action.kind === 'applyPatch' && first) first.answers = { clipboard: { kind: 'text', text: await vscode.env.clipboard.readText() } };
    // A bisect says once it ran whether the commit it was looking for is found, which the engine leaves to the window.
    if ((await this.runOnEngine(runs, repo, {}, undefined, on)) && action.kind === 'bisect') this.reportBisect(on);
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
      this.post({ type: 'busy', title: feature });
      this.repositoriesView.ran(repo.root);
      let result: Awaited<ReturnType<typeof client.runFeature>> | undefined;
      let failed: unknown;
      let busy = false;
      try {
        const inputs = this.memento.get<Record<string, string>>(LAST_INPUTS, {});
        result = await client.runFeature({ repository: repo.root, feature, target, answers, inputs }, asker);
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
        await this.followUpsBeforeRead(this.worktreeMade(feature, finished, followUps, repo), { feature, target, repo, title: finished.plan.title });
      }
      await this.refreshAfter(repo);
      // The engine runs one operation in a repository at a time, whoever asked for it.
      if (busy) {
        void vscode.window.showErrorMessage(runningElsewhere(null, this.repositoryNamed(repo.root)));
        return false;
      }
      // Said once the repository is read again, which tells whether it stopped on conflicts.
      if (failed !== undefined) this.reportEngineFailure(failed, asker.confirmed, repo);
      if (finished?.kind === 'refused') this.offerAfterRefusal(finished.message, finished.offer, target, repo);
      // **Compare First** in the warning of a force push: nothing of the plan ran, and what it offered instead runs now.
      if (asker.instead) return this.runOnEngine([{ feature: asker.instead.feature, target: asker.instead.target, answers: asker.instead.answers }], repo, {}, undefined, on);
      if (finished?.kind !== 'ran') return true;
      this.followUpsAfterRead(followUps, repo, on);
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

  // Area 5: worktrees and what new worktrees get, submodules, run by the engine.

  /**
   * Has VS Code's Git extension close the worktree or the submodules, and those inside them, that the engine is about to
   * move, empty or delete, which Windows refuses while the extension watches them.
   */
  private async letGoInVSCodeGit(action: Action, state: RepoState): Promise<void> {
    if (action.kind === 'worktreeRemove' || action.kind === 'worktreeRename') {
      // Git never moves nor removes the main worktree, which the engine refuses: VS Code's Git keeps it open.
      const main = state.worktrees.some((worktree) => worktree.isMain && samePath(worktree.folder, action.path));
      if (!main) await closeInVSCodeGit([action.path], this.log);
    }
    if (action.kind === 'submoduleDeinit' || action.kind === 'submoduleRemove') {
      const path = action.path;
      const inside = state.submodules.filter((each) => each.checkout?.commit && (each.folder === path || each.folder.startsWith(`${path}/`)));
      await closeInVSCodeGit(
        inside.map((each) => submoduleFolder(each, state)),
        this.log,
      );
    }
  }

  /**
   * Carries out what follows a new worktree made by the engine as the extension did: the commands of
   * gitardo.worktrees.setupCommands, handed over by the engine, in a terminal of this window, and only once they all
   * succeeded the message that it was created, with its buttons to open it, or the worktree opened for **Open in
   * worktree** where gitardo.worktrees.openIn says; when a command fails, Gitardo says so and offers to open it. Gives the
   * follow-ups left to carry out.
   */
  private worktreeMade(feature: string, finished: Extract<Finished, { kind: 'ran' }>, followUps: FollowUp[], repo: Repository): FollowUp[] {
    const creates = ['worktree.create', 'worktree.create-from-branch', 'worktree.create-at-commit'].includes(feature);
    if (!creates && feature !== 'worktree.open-in-worktree') return followUps;
    // `git worktree add [<options>] <folder> <commit-ish>`.
    const argumentsOf = (step: Step) => (step.kind === 'git' && Array.isArray(step.arguments) ? (step.arguments as string[]) : []);
    const added = finished.plan.steps.map(argumentsOf).find((args) => args[0] === 'worktree' && args[1] === 'add') ?? [];
    const dir = added[added.length - 2];
    if (!dir) return followUps;
    const agents = this.heldFor(repo.root)?.agentsWindow ?? false;
    const says = followUps.flatMap((followUp) => (followUp.kind === 'say' ? [String(followUp.text)] : []));
    const offersSetup =
      followUps.some((followUp) => followUp.kind === 'offerRun' && followUp.feature === 'worktree-setup.page') && !this.memento.get(SETUP_OFFERED);
    const opens = [...(agents ? ['Open in Agents Window'] : []), 'Open in New Window'];
    const open = (choice: string | undefined) => {
      if (choice) void openWorktree(dir, choice === 'Open in Agents Window' ? 'agents' : 'window');
    };
    const done = () => {
      if (!creates) {
        void openWorktree(dir, openIn({ agentsWindow: agents }));
        for (const said of says) void vscode.window.showInformationMessage(`Gitardo: ${said}`);
        return;
      }
      if (offersSetup) void this.memento.update(SETUP_OFFERED, true);
      const [created, ...others] = says;
      for (const said of others) void vscode.window.showInformationMessage(`Gitardo: ${said}`);
      void vscode.window
        // Without the name of Gitardo before it, as the message of the extension had it.
        .showInformationMessage(created ?? `Worktree created in ${dir}`, ...opens, ...(offersSetup ? [SET_UP] : []))
        .then((choice) => (choice === SET_UP ? void vscode.commands.executeCommand('gitardo.worktreeSetup') : open(choice)));
    };
    const commands = finished.waiting.filter((step) => step.kind === 'task').map((step) => String(step.command));
    if (!commands.length) done();
    else {
      // Not waited for: the commands can take minutes, and other operations go on meanwhile.
      void runSetupCommands(repo.root, dir, commands, this.log).then((stopped) => {
        if (!stopped) return done();
        void vscode.window.showWarningMessage(`Gitardo: ${stopped}`, ...opens).then(open);
      });
    }
    return followUps.filter((followUp) => followUp.kind !== 'say' && followUp.kind !== 'openWindow');
  }

  /** Says why the engine refused, with the button of the feature it offers in its place, run on the same target. */
  private offerAfterRefusal(message: string, offer: { feature: string; label: string } | null, target: Target, repo: Repository): void {
    const items = offer ? [offer.label] : [];
    void vscode.window.showWarningMessage(`Gitardo: ${message}`, ...items).then((choice) => {
      if (offer && choice === offer.label) void this.runOnEngine([{ feature: offer.feature, target }], repo);
    });
  }

  /**
   * Says that a step of the engine failed: that a merge, a rebase, a cherry-pick, a revert or a patch stopped when files
   * are left in conflict, with the title of the plan confirmed, and with the engine's words otherwise, with a button for
   * each operation the plan offers once git refused it, such as **Delete anyway**.
   */
  private reportEngineFailure(failure: unknown, title: string | undefined, repo: Repository): void {
    const state = this.heldFor(repo.root);
    const operation = state?.operation;
    if (failure instanceof EngineError && failure.code === ErrorCode.stepFailed && state?.workingTree.conflicted.length) {
      this.reportFailure({ title: title ?? `The ${operation ? operationName(operation) : 'operation'}` }, errorText(failure), repo);
      return;
    }
    const data = failure instanceof EngineError ? (failure.data as { followUps?: FollowUp[] } | undefined) : undefined;
    const offers = (data?.followUps ?? []).filter((followUp): followUp is OfferedRun => followUp.kind === 'offerRun');
    void vscode.window.showErrorMessage(`Gitardo: ${errorText(failure)}`, ...offers.map((offer) => offer.label), 'Show Log').then((choice) => {
      const offer = offers.find((each) => each.label === choice);
      if (offer) void this.runOnEngine([{ feature: offer.feature, target: offer.target, answers: offer.answers }], repo);
      else if (choice) this.log.show();
    });
  }

  /**
   * **Check tags on `<remote>`**: the remote of `remote.default` asked for its tags by the engine, and the tags it lacks
   * marked in the references and on the labels until VS Code is closed (`tag.remote-marks`).
   */
  private async checkTags(state: RepoState, repo: Repository): Promise<void> {
    const remote = state.remotes.includes('origin') ? 'origin' : state.remotes[0];
    if (!remote) {
      void vscode.window.showWarningMessage('Gitardo: this repository has no remote.');
      return;
    }
    if (!reads.available) return sayEngineMissing();
    try {
      const title = `Gitardo: asking ${remote} for its tags`;
      const comparison = await vscode.window.withProgress({ location: vscode.ProgressLocation.Window, title }, () => reads.remoteTags(repo, remote));
      this.markTags(repo.root, checkedMarks(comparison, state.tags));
    } catch (error) {
      void vscode.window.showErrorMessage(`Gitardo: ${errorText(error)}`);
    }
  }

  /** Keeps the marks of the tags of a repository, and shows them while it is the one shown. */
  private markTags(root: string, marks: TagMarks): void {
    this.tagMarks.set(root, marks);
    for (const group of this.groupsShowing(root)) this.setState(group, { ...group.state!, tagMarks: marks });
  }

  /** Carries out what a plan of the engine leaves for the host before the repository is read again. */
  private async followUpsBeforeRead(followUps: FollowUp[], run: { feature: string; target: Target; repo: Repository; title?: string }): Promise<void> {
    for (const [index, followUp] of followUps.entries()) {
      switch (followUp.kind) {
        case 'say':
          this.say(String(followUp.text), followUps.slice(index + 1), run);
          break;
        // **See What Changed** after a rewrite that said nothing of its own (M36): said as the desktop says it.
        case 'offerRun':
          if (followUp.feature === COMPARE_REWRITE && !followUps.slice(0, index).some((before) => before.kind === 'say')) {
            this.say(`${run.title ?? 'The operation'}: done.`, followUps.slice(index), run);
          }
          break;
        case 'rememberInputs': {
          const last = this.memento.get<Record<string, string>>(LAST_INPUTS, {});
          await this.memento.update(LAST_INPUTS, { ...last, ...(followUp.values as Record<string, string>) });
          break;
        }
        case 'emptyCommitMessage':
          this.post({ type: 'committed' });
          break;
        case 'copyToClipboard':
          await vscode.env.clipboard.writeText(String(followUp.text));
          break;
        case 'openUrl':
          void vscode.env.openExternal(vscode.Uri.parse(String(followUp.url)));
          break;
        case 'openWindow':
          void vscode.commands.executeCommand('vscode.openFolder', vscode.Uri.file(String(followUp.folder)), { forceNewWindow: true });
          break;
        case 'showFolder':
          void vscode.commands.executeCommand('revealFileInOS', vscode.Uri.file(String(followUp.folder)));
          break;
        // Branches, tags and remotes (stage 4, area 2): a fetch, a pull, a push or a remote changed.
        case 'readPullRequests':
          void this.sessions.find(run.repo.root)?.refreshPulls(true);
          break;
        // AI and GitHub (stage 4, area 6): the model that resolves conflicts, chosen with `ai.choose-model`.
        case 'writeSetting':
          await this.writeSetting({ kind: 'writeSetting', key: String(followUp.key), value: followUp.value });
          break;
        // The commit box of the Changes view, after a squash merge and a split (area 3).
        case 'preparedMessage': {
          const prepared = (await reads.squashMessage(run.repo).catch(() => null))?.trim();
          if (prepared) this.post({ type: 'commitMessage', message: prepared });
          break;
        }
        case 'fillCommitMessage':
          if (String(followUp.message)) this.post({ type: 'commitMessage', message: String(followUp.message) });
          break;
        // A tag made, pushed or deleted from a remote changes its mark without asking the remote again (`tag.remote-marks`).
        case 'tagsOnRemote': {
          const marks = this.tagMarks.get(run.repo.root);
          const changed = { remote: (followUp.remote as string | null) ?? null, pushed: followUp.pushed as string[], notPushed: followUp.notPushed as string[] };
          if (marks) this.markTags(run.repo.root, followMarks(marks, changed, this.heldFor(run.repo.root)?.tags ?? []));
          break;
        }
        default:
          break;
      }
    }
  }

  /**
   * A message of a plan of the engine, with a button for each feature offered right after it, run on the same target.
   * Resolving automatically keeps the **Undo** it had beside them (stage 4, area 4).
   */
  private say(text: string, rest: FollowUp[], run: { feature: string; target: Target; repo: Repository }): void {
    const offers: Array<EngineRun & { label: string }> = [];
    for (const next of rest) {
      if (next.kind !== 'offer') break;
      offers.push({ feature: String(next.feature), label: String(next.label), target: run.target });
    }
    // **See What Changed** after a rewrite, until what is said next (M36), on its own target.
    const until = rest.findIndex((next) => next.kind === 'say');
    for (const next of until < 0 ? rest : rest.slice(0, until)) {
      if (next.kind !== 'offerRun' || next.feature !== COMPARE_REWRITE) continue;
      const offered = next as OfferedRun;
      offers.push({ feature: offered.feature, label: buttonWords(offered.label), target: offered.target, answers: offered.answers });
    }
    const undo = run.feature === 'conflicts.resolve-automatically' ? ['Undo'] : [];
    const words = text.startsWith('Gitardo ') ? text : `Gitardo: ${text}`;
    void vscode.window.showInformationMessage(words, ...offers.map((offer) => offer.label), ...undo).then((choice) => {
      const offer = offers.find((each) => each.label === choice);
      if (offer) void this.runOnEngine([{ feature: offer.feature, target: offer.target, answers: offer.answers }], run.repo);
      if (choice === 'Undo') void this.action({ kind: 'undo' }, this.groupShowing(run.repo.root));
    });
  }

  /** Carries out what a plan of the engine leaves for the host once the repository was read again: what to select. */
  private followUpsAfterRead(followUps: FollowUp[], repo: Repository, on: ViewGroup): void {
    // What to select is selected in the group the operation was asked in, when it still shows that repository.
    const state = on.state && samePath(on.state.root, repo.root) ? on.state : null;
    for (const followUp of followUps) {
      // The tab of what a rewrite changed (M36), in the repository the operation ran in.
      if (followUp.kind === 'openRangeDiff') void this.rangeDiffTab.open(followUp as RangeDiffTarget, repo);
      if (followUp.kind === 'selectHead' && state?.head.commit) this.show(state.head.commit, on);
      if (followUp.kind === 'selectCommit' && state) this.show(String(followUp.commit), on);
      if (followUp.kind === 'selectBeforeHead' && state?.head.commit) {
        const count = Number(followUp.count);
        void reads.resolve(repo, `HEAD~${count}`).then((hash) => hash && this.show(hash, on));
      }
      if (followUp.kind === 'openCommitPage') void this.openCommit(String(followUp.commit), repo);
      // Conflicts (stage 4, area 4): **Review** of resolving automatically.
      if (followUp.kind === 'openReview') void this.reviewConflicts(Array.isArray(followUp.paths) ? (followUp.paths as string[]) : undefined, false, on);
    }
  }

  /** Says so when the last answer left a single suspect: that is the commit the bisect was looking for. */
  private reportBisect(on: ViewGroup): void {
    const bisect = on.state?.bisect;
    const bad = bisect?.bad;
    if (!bad || bisect.remaining !== 1) return;
    const subject = on.state?.rows.find((row) => row.hash === bad)?.subject;
    void vscode.window
      .showInformationMessage(`Bisect: the first bad commit is ${bad.slice(0, 7)}${subject ? ` "${subject}"` : ''}.`, 'Show in the graph', 'End bisect')
      .then((choice) => {
        if (choice === 'Show in the graph') void this.reveal(bad, on);
        if (choice === 'End bisect') void this.action({ kind: 'bisectReset' }, on);
      });
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
    if (group === this.followers) this.showHiddenInMenus(state);
    this.postToGroup(group, { type: 'state', state });
    if (group === this.followers) {
      this.repositoriesView.shown(state);
      this.stateEmitter.fire(state);
      void this.checkUnsaved();
    }
    if (state) this.readEmitter.fire(state);
  }

  /**
   * Tells the context menu of the Explorer which files of the repository shown are hidden from git, for it to offer
   * **Show Its Changes to Git Again** on them in place of **Hide Its Changes from Git…**.
   */
  private showHiddenInMenus(state: RepoState | null): void {
    const hidden = Object.fromEntries((state?.skipped ?? []).map((file) => [vscode.Uri.file(path.join(state?.root ?? '', file.path)).fsPath, true]));
    const said = JSON.stringify(hidden);
    if (said === this.hiddenInMenus) return;
    this.hiddenInMenus = said;
    void vscode.commands.executeCommand('setContext', 'gitardo.hiddenFromGit', hidden);
  }

  /**
   * Ignores, stops tracking, hides from git or shows to git again the file or the folder the Explorer's menu was opened
   * on, in the repository it is in, which the views show first (`changes.ignore`, `changes.stop-tracking`,
   * `changes.skip`, `changes.unskip`). One path at a time, as in the desktop (Q1173).
   */
  async leaveAloneFromExplorer(kind: 'ignore' | 'stopTracking' | 'hideFromGit' | 'showToGit', uri: vscode.Uri | undefined): Promise<void> {
    if (uri?.scheme !== 'file' || !(await this.requireState())) return;
    await this.showRepoOf(uri);
    const relative = this.repo ? path.relative(this.repo.root, uri.fsPath).replace(/\\/g, '/') : '';
    if (!relative || relative.startsWith('..') || path.isAbsolute(relative)) {
      void vscode.window.showWarningMessage(`Gitardo: ${path.basename(uri.fsPath)} is in no repository of the workspace.`);
      return;
    }
    await this.action({ kind, path: relative });
  }

  /** Tells the Changes views which files of the repository have edits not saved in an editor, when that changed. */
  private async checkUnsaved(): Promise<void> {
    const { state, repo } = this;
    const documents = vscode.workspace.textDocuments.filter((document) => document.isDirty && document.uri.scheme === 'file');
    const files = documents.map((document) => document.uri.fsPath);
    const from = { state, files: JSON.stringify(files) };
    // It runs at every key typed in an editor: most of the time nothing it looks at has changed.
    if (this.unsavedFrom?.state === from.state && this.unsavedFrom.files === from.files) return;
    this.unsavedFrom = from;
    const paths = state && repo ? await this.unsavedIn(repo, state, files) : [];
    if (this.unsavedFrom !== from || JSON.stringify(paths) === JSON.stringify(this.unsaved)) return;
    this.unsaved = paths;
    for (const [view, kind] of this.views) if (kind === 'changes') this.post({ type: 'unsaved', paths }, view);
  }

  /**
   * Those of these files that are of the repository, by their paths from its root, in order. The files of its
   * submodules and of the worktrees inside it are of other repositories, and the ones git ignores are no change to it.
   */
  private async unsavedIn(repo: Repository, state: RepoState, files: string[]): Promise<string[]> {
    const inside = (folder: string) => {
      const relative = path.relative(repo.root, folder);
      return relative && !relative.startsWith('..') && !path.isAbsolute(relative) ? relative.replace(/\\/g, '/') : null;
    };
    const others = [
      ...state.submodules.map((submodule) => submodule.folder),
      ...state.worktrees.filter((worktree) => !worktree.isCurrent).flatMap((worktree) => inside(worktree.folder) ?? []),
    ];
    const paths = files
      .flatMap((file) => inside(file) ?? [])
      .filter((file) => !others.some((other) => file === other || file.startsWith(`${other}/`)))
      .sort();
    const key = (file: string) => `${repo.root}\0${file}`;
    const unknown = paths.filter((file) => !this.ignoredUnsaved.has(key(file)));
    if (unknown.length) {
      // When the engine cannot say, a file is a change, and is asked about again the next time.
      const ignored = await reads.ignored(repo, unknown).catch(() => null);
      for (const file of ignored ? unknown : []) this.ignoredUnsaved.set(key(file), ignored!.has(file));
    }
    // A file saved since, or of another repository, is asked about again once it has edits not saved there.
    const asked = new Set(paths.map(key));
    for (const known of [...this.ignoredUnsaved.keys()]) if (!asked.has(known)) this.ignoredUnsaved.delete(known);
    return paths.filter((file) => this.ignoredUnsaved.get(key(file)) !== true);
  }

  /** Saves the editors of these files, by their paths from the root of the repository. */
  private async save(paths: string[], repo = this.repo): Promise<void> {
    if (!repo || !Array.isArray(paths)) return;
    const wanted = paths.map((file) => path.join(repo.root, file));
    const documents = vscode.workspace.textDocuments.filter(
      (document) => document.isDirty && document.uri.scheme === 'file' && wanted.some((file) => samePath(file, document.uri.fsPath)),
    );
    await Promise.all(documents.map((document) => document.save()));
  }

  /** Looks for the repositories of the workspace and reads the one to show. Who asks while that runs waits for it. */
  private locate(): Promise<void> {
    this.located = true;
    const locating = this.findRepos();
    this.locating = locating;
    const done = () => {
      if (this.locating === locating) this.locating = undefined;
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
    this.repositoriesView.found(grouped);
    for (const group of this.allGroups()) this.retitle(group);
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
    return this.point(this.followers, root, true);
  }

  /**
   * Points a group of views at a repository, a worktree or a submodule, or at none: it goes on the session of that
   * repository, made when no group showed it, and the session it leaves is let go when it was the last group there.
   * With `again`, a group already there starts again, as when the repositories of the workspace are looked for again.
   */
  private async point(group: ViewGroup, root: string | undefined, again = false, shown: TabShown = {}): Promise<void> {
    const before = group.session;
    if (before && root && samePath(before.root, root)) {
      if (!again) return;
      this.sessions.leave(group);
    }
    // A repository left reads no more pages: it reads its commits again once it is shown (Q437).
    if (before) group.graph.stop(before.root);
    if (!root) {
      this.sessions.leave(group);
      group.session = undefined;
      group.reset(null);
      return this.setState(group, null);
    }
    const { session, made } = this.sessions.move(group, root);
    group.session = session;
    const kept = made && keepOpen() ? this.kept.take(root) : undefined;
    group.reset(this.memento.get<Record<string, Scope>>(SCOPE_KEY, {})[root] ?? null);
    if (kept) return this.comeBack(group, session, kept);
    if (shown.filter) group.filter = shown.filter;
    if (shown.scope !== undefined) group.scope = shown.scope;
    // A repository chosen counts every worktree again, as **Refresh** does.
    if (made) session.everything = true;
    // The commits of the repository before stay in the views until those of this one come.
    await this.reload(group, 'filter', () => (made ? session.open() : session.refresh()));
  }

  /**
   * A session no group shows any more. With `gitardo.repositories.keepOpen` on its repository keeps what the last group
   * showed of it, and its watchers, which from now on only mark it out of date (`repo.keep-open`, `repo.watch-hidden`);
   * else it is no longer watched.
   */
  private letGo(session: RepositorySession, last: ViewGroup): void {
    // A change seen just before is the repository left's, not the next one's.
    const pending = session.takePending();
    const state = last.state;
    if (!state || !samePath(state.root, session.root) || !keepOpen()) return session.dispose();
    this.kept.keep(session.root, {
      state,
      filter: last.filter,
      scope: last.scope,
      limit: last.limit,
      hosting: session.hosting,
      pulls: session.pulls,
      pullsAt: session.pullsAt,
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
    session.watcher = kept.watcher;
    // The repositories of the workspace may have changed while it was not shown.
    this.setState(group, { ...kept.state, repos: this.repos, home: this.homeOf(group, kept.state.root), keepOpen: true });
    if (kept.changed) {
      session.cause = kept.changed;
      session.everything = true;
      await session.detectHosting().catch((error) => this.log.warn(`Could not read the remotes of ${session.root}: ${errorText(error)}`));
      await session.refresh();
    } else {
      session.markFresh();
      if (this.views.size) void session.refreshPulls(false);
    }
  }

  /** The services of one's own changed: the main remote of each repository shown may be on another, and the views say so. */
  private async hostingChanged(): Promise<void> {
    if (!reads.available) return;
    for (const session of this.sessions.all()) {
      try {
        await session.detectHosting();
      } catch (error) {
        this.log.warn(`Could not read the remotes of ${session.root}: ${errorText(error)}`);
        continue;
      }
      for (const group of this.sessions.groupsOn(session)) {
        if (group.state) this.setState(group, { ...group.state, hosting: session.hosting, pulls: session.pulls });
      }
      void session.refreshPulls(true);
    }
  }

  /** Lists the remotes, to fetch, rename, repoint or remove one, or to add another. */
  private async manageRemotes(add = false, on = this.followers): Promise<void> {
    const state = await this.requireState(on);
    const repo = on.repo;
    if (!state || !repo) return;
    // The remote that pull requests are read from may be another one after.
    if (!(await this.runOnEngine([{ feature: add ? 'remote.add' : 'remote.manage', target: { kind: 'head' } }], repo, {}, undefined, on))) return;
    await on.session?.detectHosting();
    await this.refresh(on);
  }

  /**
   * Switches a group to another repository of the workspace: the panel and the side bar unless another is given, whose
   * choice is remembered for this workspace; or to another worktree of the repository it shows, for as long as the window
   * lasts: its changes, commits and branches are worked on from there.
   */
  async selectRepo(root: string, on = this.followers): Promise<void> {
    const shown = on.repo?.root;
    if (this.running || (shown && samePath(root, shown))) return;
    const repo = this.repos.find((each) => samePath(each.root, root));
    if (repo) {
      if (on === this.followers) await this.memento.update(REPO_KEY, repo.root);
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
      on.home = this.repos.find((each) => shown && samePath(each.root, shown)) ?? on.home;
      await this.point(on, folder);
      this.chose(on, folder);
      return;
    }
    // A worktree of another repository of the workspace, such as one chosen in VS Code's Repositories view.
    const owner = await this.worktreeOwner(root);
    if (!owner || this.running) return;
    on.home = owner;
    await this.point(on, root);
    this.chose(on, root);
  }

  /** A group shows what was chosen: what the panel shows is selected in VS Code's Repositories view, a tab says it in its title. */
  private chose(on: ViewGroup, root: string): void {
    if (on === this.followers) this.chosenEmitter.fire(root);
    else this.retitle(on);
  }

  /**
   * Opens a tab of the graph in the editor on a repository, a worktree or a submodule: the one the panel shows unless
   * another is given (`repo.tab-repository`). The tab keeps it, whatever the other views show, and is read with them.
   * `panel` is a tab VS Code brings back after a reload, with what it showed (`repo.keep-tabs`).
   */
  async openTab(root?: string, panel?: vscode.WebviewPanel, shown: TabShown = {}): Promise<void> {
    await this.whenLocated();
    const tab =
      panel ??
      vscode.window.createWebviewPanel(GRAPH_TAB, 'Gitardo', vscode.ViewColumn.Active, {
        retainContextWhenHidden: true,
      });
    const group = new ViewGroup(this.groupHost, tab);
    const attachment = this.attach(tab.webview, 'graph', { tab: 'graph' }, group);
    tab.onDidDispose(() => {
      attachment.dispose();
      this.sessions.leave(group);
      group.session = undefined;
      group.dispose();
    });
    root ??= this.followers.repo?.root;
    if (!root) return this.setState(group, null);
    group.home = await this.homeFor(root);
    // A choice of the panel's is the panel's: the tab starts with the history as the panel shows it, read for itself.
    await this.point(group, root, false, shown);
    this.retitle(group);
  }

  /** Says in the title of the tab of a group what it shows: `Gitardo · api`, `Gitardo · api › feature/login` (Q1520). */
  private retitle(group: ViewGroup): void {
    if (group.panel) group.panel.title = tabTitle(group.session?.root, this.repos, group.home);
  }

  /**
   * The repository of the workspace a worktree or a submodule outside its list is of, to go back to: the one a group
   * shows it from, or the one it shares its folder of git with; none for a folder of the list.
   */
  private async homeFor(root: string): Promise<RepoInfo | undefined> {
    if (this.repos.some((each) => samePath(each.root, root))) return undefined;
    for (const group of this.allGroups()) {
      const state = group.state;
      if (!state) continue;
      if (samePath(state.root, root)) return group.home;
      const member =
        state.worktrees.some((each) => samePath(each.folder, root)) ||
        state.submodules.some((each) => each.checkout?.commit && samePath(submoduleFolder(each, state), root));
      if (member) return this.repos.find((each) => samePath(each.root, state.root)) ?? group.home;
    }
    return this.worktreeOwner(root);
  }

  /**
   * **Gitardo: Open Repository in a New Tab**: asks which repository, worktree or submodule, as **Gitardo: Select
   * Repository** lists them, and opens a tab of the graph on it.
   */
  async pickTab(): Promise<void> {
    await this.whenLocated();
    if (!this.repos.length) {
      void vscode.window.showInformationMessage('Gitardo: no Git repository in this workspace.');
      return;
    }
    const choice = await vscode.window.showQuickPick(
      this.repos.map((repo) => ({ label: repo.name, detail: repo.root, repo })),
      { title: 'Open Repository in a New Tab', matchOnDetail: true },
    );
    if (choice) await this.openTab(choice.repo.root);
  }

  /** The repository of the workspace that a folder is a worktree of, by the git directory they share. */
  private async worktreeOwner(root: string): Promise<RepoInfo | undefined> {
    const commonDir = (folder: string) => reads.commonDir({ root: folder }).catch(() => undefined);
    const shared = await commonDir(root);
    if (!shared) return undefined;
    const theirs = await Promise.all(this.repos.map((repo) => commonDir(repo.root)));
    return this.repos.find((_, index) => theirs[index] && samePath(theirs[index], shared));
  }

  /**
   * Shows the repository or worktree chosen in VS Code's Repositories view, once Gitardo has looked for the
   * repositories of the workspace: until then, it reads nothing for it.
   */
  async followSourceControl(root: string): Promise<void> {
    if (!this.located) return;
    await this.whenLocated();
    await this.selectRepo(root);
  }

  private async pickRepo(on: ViewGroup): Promise<void> {
    await this.whenLocated();
    const state = on.state;
    // Showing a worktree, there is always the repository of the workspace to go back to.
    if (this.repos.length < 2 && !state?.home) {
      void vscode.window.showInformationMessage(
        this.repos.length ? 'Gitardo: this workspace has a single repository.' : 'Gitardo: no Git repository in this workspace.',
      );
      return;
    }
    const current = on.repo?.root;
    const choice = await vscode.window.showQuickPick(
      [
        ...this.repos.map((repo) => ({ label: repo.name, description: repo.root === current ? 'current' : undefined, detail: repo.root, repo })),
        ...(state?.home && current ? [{ label: state.name, description: 'worktree, current', detail: current, repo: { root: current, name: state.name, depth: 0 } }] : []),
      ],
      { title: 'Repository shown in Gitardo', matchOnDetail: true },
    );
    if (choice) await this.selectRepo(choice.repo.root, on);
  }

  /**
   * Keeps the message the commit box had in the repository it was pointed away from, and gives it back the one it had in
   * the repository shown, when it kept one (`repo.keep-open`).
   */
  private keepMessage(webview: vscode.Webview, root: unknown, message: unknown, shown: unknown): void {
    if (!keepOpen() || typeof root !== 'string' || typeof message !== 'string' || typeof shown !== 'string') return;
    this.kept.keepMessage(root, message);
    const back = this.kept.message(shown);
    if (back !== undefined) this.post({ type: 'keptMessage', root: shown, message: back }, webview);
  }

  /** Chooses who commits are made as in this repository: a profile from the settings, a new one, or the global identity again. */
  private async switchIdentity(on: ViewGroup): Promise<void> {
    const state = await this.requireState(on);
    const repo = on.repo;
    if (!state || !repo) return;
    // The engine's, with the entry that opens the setting at the end of its pick (B9).
    const editList = { label: 'Edit the list of identities…', description: 'gitardo.identities', run: () => this.openIdentities() };
    await this.runOnEngine([{ feature: 'identity.switch', target: { kind: 'head' } }], repo, { choices: { 'identity-pick': [editList] } }, undefined, on);
  }

  /** Asks for the co-authors of the next commit, the authors of the history read as the pick opens, and gives them to the box. */
  private async pickCoAuthors(repo: Repository, chosen: string[], webview: vscode.Webview): Promise<void> {
    const authors = reads.authors(repo);
    const email = this.heldFor(repo.root)?.identity.email ?? null;
    const picked = await pickCoAuthors(authors, email, Array.isArray(chosen) ? chosen : []);
    if (picked) this.post({ type: 'coAuthors', coAuthors: picked }, webview);
  }

  /** Opens the setting that lists the identities to commit as. */
  private async openIdentities(): Promise<void> {
    await vscode.commands.executeCommand('workbench.action.openSettings', 'gitardo.identities');
  }

  /**
   * A file of the working tree changed in a repository (`repo.watch-changes`, M15): its session asks the engine whether
   * git ignores it before it reads. A repository kept is only marked out of date (Q461).
   */
  private fileChangedIn(root: string, file: string): void {
    const session = this.sessions.find(root);
    if (!session) return this.kept.changed(root, `${file} changed`);
    session.fileChanged(file);
  }

  /**
   * A file was saved in an editor. One of the working tree of a repository shown asks for a read as a file changed on
   * the disk does, once the engine says git does not ignore it, and one of git's own, such as `.git/info/exclude`, at
   * once; one out of them asks for none, since the watchers of a repository kept see it. With no repository shown, it is
   * looked for again.
   */
  private saved(uri: vscode.Uri): void {
    if (uri.scheme !== 'file') return;
    const sessions = this.sessions.all();
    if (!sessions.length) return this.scheduleRefresh(`${vscode.workspace.asRelativePath(uri)} was saved`);
    for (const session of sessions) {
      const relative = path.relative(session.root, uri.fsPath).replace(/\\/g, '/');
      if (!relative || relative.startsWith('..') || path.isAbsolute(relative)) continue;
      // A file that tells git what to ignore may change which unsaved files are changes.
      if (path.posix.basename(relative) === '.gitignore' || relative === '.git/info/exclude') this.ignoredUnsaved.clear();
      if (/(^|\/)\.git(\/|$)/.test(relative)) session.scheduleRefresh(`${relative} was saved`);
      else session.fileChanged(relative);
    }
  }

  /** The window got the focus: each repository shown asks for a read when it missed a change or was read long ago. */
  private focusGot(): void {
    const sessions = this.sessions.all();
    if (!sessions.length) return this.scheduleRefresh('VS Code got the focus');
    for (const session of sessions) session.focusGot();
  }

  /** A change seen in a repository: one shown is read again for it, one kept is only marked out of date. */
  private changedIn(root: string, cause: string, delay?: number): void {
    const session = this.sessions.find(root);
    if (session) session.scheduleRefresh(cause, delay);
    else this.kept.changed(root, cause);
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

  private post(message: ToViews, to?: vscode.Webview): void {
    // The Repositories view is sent its rows alone, by `repositoriesView`.
    const targets = to ? [to] : [...this.views].filter(([, kind]) => kind !== 'repositories').map(([view]) => view);
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
        void view.postMessage(reads.available ? { type: 'state', state: null } : { type: 'state', state: null, engineMissing: true });
        continue;
      }
      const keys = new Map<number, string>();
      const filter = this.groupOf(view)?.filter ?? null;
      const keyOf = (count: number) => keys.get(count) ?? keys.set(count, rowsKey(state, filter, count)).get(count)!;
      const { message: posted, sent } = stateFor(kind, state, this.sent.get(view), keyOf);
      this.sent.set(view, sent);
      if (!posted) continue;
      void view.postMessage(posted).then((delivered) => {
        // A view that is not there to receive it asks again when it comes back, and gets everything.
        if (!delivered && this.sent.get(view) === sent) this.sent.delete(view);
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

  private async onMessage(webview: vscode.Webview, message: WebviewMessage): Promise<void> {
    // What a view asks is about the repository its group shows, and what the tab of a commit asks about its commit's.
    const on = this.groupOf(webview) ?? this.followers;
    const repo = this.pinned.get(webview) ?? on.repo;
    const compareTab = this.compareTabs.get(webview);
    const review = this.reviewTab?.webview === webview ? this.reviewTab : undefined;
    // The tab of the review reads the files in conflict by itself.
    if (review && message.type === 'ready') return review.reload();
    // The tab of what a rewrite changed is sent its pairs, which it does not read again with the repository.
    if (message.type === 'ready' && this.rangeDiffTab.shows(webview)) return this.rangeDiffTab.ready();
    switch (message.type) {
      case 'focus':
        return this.setFocused(webview, message.focused === true);
      case 'ready':
        // A view that has just loaded has nothing yet: it is sent every part it shows.
        this.sent.delete(webview);
        // The tab of a commit reads its commit by itself, and nothing in it goes stale: the repository is not read for it.
        if (this.views.get(webview) === 'commit') return this.post({ type: 'state', state: on.state }, webview);
        // The tab of a comparison is sent its comparison, and the state for what each side is.
        if (compareTab) {
          this.post({ type: 'state', state: on.state }, webview);
          return this.sendCompare(compareTab, true);
        }
        // The Repositories view is sent its rows as they are, and what the panel shows is read as for any view.
        if (this.views.get(webview) === 'repositories') {
          this.repositoriesView.ready(webview);
          return this.readForView(this.followers);
        }
        // What was read before it opened, as at the start of VS Code, is shown at once, and read again if it is not fresh.
        if (this.located && !this.locating && !on.session?.loading) this.post({ type: 'state', state: on.state }, webview);
        if (on.waiting) this.post({ type: 'loading', what: on.waiting.what }, webview);
        if (this.views.get(webview) === 'changes' && this.unsaved.length) this.post({ type: 'unsaved', paths: this.unsaved }, webview);
        await this.readForView(on);
        return this.flushPending(on);
      case 'setFilter':
        return this.setFilter(message.filter, on);
      case 'setScope':
        return this.setScope(message.scope, on);
      case 'setListed':
        return this.setListed(message.listed, on);
      case 'setShowStashes':
        return vscode.workspace.getConfiguration('gitardo').update('graph.showStashes', message.show === true, vscode.ConfigurationTarget.Global);
      case 'setDateSeparators':
        return vscode.workspace.getConfiguration('gitardo').update('graph.dateSeparators', message.show === true, vscode.ConfigurationTarget.Global);
      case 'setCombineUpstream':
        return vscode.workspace.getConfiguration('gitardo').update('graph.combineUpstream', message.combine === true, vscode.ConfigurationTarget.Global);
      case 'setBranchSort': {
        const sort = message.sort === 'date' ? 'date' : 'name';
        return vscode.workspace.getConfiguration('gitardo').update('branches.sortOrder', sort, vscode.ConfigurationTarget.Global);
      }
      case 'setGraphColumns': {
        const columns = GRAPH_COLUMNS.filter((column) => message.columns.includes(column));
        return vscode.workspace.getConfiguration('gitardo').update('graph.columns', columns, vscode.ConfigurationTarget.Global);
      }
      case 'setFirstParent':
        return vscode.workspace.getConfiguration('gitardo').update('graph.firstParent', message.only === true, vscode.ConfigurationTarget.Global);
      case 'setOneLane':
        return vscode.workspace.getConfiguration('gitardo').update('graph.oneLane', message.one === true, vscode.ConfigurationTarget.Global);
      case 'setTraceSelected':
        return vscode.workspace.getConfiguration('gitardo').update('graph.traceSelected', message.trace === true, vscode.ConfigurationTarget.Global);
      case 'setFavorite':
        return this.setFavorite(message.ref, message.favorite === true, on);
      case 'goTo':
        return this.goTo(message.revision, webview, on);
      case 'find':
        return this.find(message.search, webview, on);
      case 'command':
        return this.command(message.command, on);
      case 'pullRequest':
        return this.openPullRequest(message.branch, on);
      case 'checkOutPullRequest':
        return this.checkOutPullRequest(message.number, message.remoteBranch, on);
      case 'syntaxTheme':
        this.colored.add(webview);
        return this.post({ type: 'syntaxTheme', theme: (await this.themes.resolveActive()) ?? null }, webview);
      case 'fileIcons':
        this.iconed.add(webview);
        return this.post({ type: 'fileIcons', theme: await this.fileIcons.for(webview) }, webview);
      case 'getWorktreeSetup':
        return this.sendSetup(webview, message.rescan === true);
      case 'previewWorktreeSetup': {
        if (!repo) return;
        const patterns = { link: cleanPatterns(message.patterns?.link), copy: cleanPatterns(message.patterns?.copy) };
        try {
          const preview = await reads.setupPreview(repo.root, patterns);
          return this.post({ type: 'worktreeSetupPreview', patterns: message.patterns, preview }, webview);
        } catch (error) {
          return this.post({ type: 'worktreeSetupPreview', patterns: message.patterns, preview: null, error: errorText(error) }, webview);
        }
      }
      case 'saveWorktreeSetup': {
        if (!repo || (message.target !== 'user' && message.target !== 'workspace')) return;
        const tab = this.setupTab?.panel.webview === webview ? this.setupTab : undefined;
        if (tab) tab.saving = true;
        try {
          const commands = Array.isArray(message.commands) ? cleanCommands(message.commands) : undefined;
          await saveSetup(repo.root, message.target, message.patterns, message.alsoVSCode === true, commands);
        } catch (error) {
          void vscode.window.showErrorMessage(`Gitardo: the settings were not saved. ${errorText(error)}`);
        } finally {
          if (tab) tab.saving = false;
        }
        return this.sendSetup(webview, false);
      }
      case 'editWorktreeSetup':
        return repo && editSetupJson(repo.root, message.target === 'user' ? 'user' : 'workspace');
      case 'reviewConflicts':
        return this.reviewConflicts(Array.isArray(message.paths) ? message.paths : undefined, message.ai === true, on);
      case 'getReview':
        return review?.reload();
      case 'writeResolutions':
        return Array.isArray(message.accepted) ? review?.write(message.accepted) : undefined;
      case 'previewResolutions':
        return Array.isArray(message.accepted) ? review?.preview(message.accepted) : undefined;
      case 'chooseResolution':
        return message.conflict && review?.choose(message.conflict, message.choice ?? null);
      case 'askAI':
        return review?.ask(Array.isArray(message.conflicts) ? message.conflicts : undefined);
      case 'stopAsking':
        return review?.stop();
      case 'chooseConflictModel':
        return this.chooseConflictModel(on);
      case 'openConflict':
        return repo && this.openConflict(repo, message.path, message.line, message.editor);
      case 'refresh':
        return this.refreshAll(on);
      case 'loadMore': {
        // While a read adds its pages, the next ones come without asking (`graph.load-more`); and a list that asked while
        // the last page came has nothing left to ask for once the read ended at the end of the history.
        if (!on.repo || on.graph.isReading(on.repo.root) || on.state?.hasMore === false) return;
        on.cancelGraphRead();
        const listed = on.state?.rows.filter((row) => row.hash !== WORKTREE && row.stash === undefined).length ?? 0;
        on.limit = Math.max(on.limit, listed) + pageSize();
        return this.reload(on, 'more');
      }
      case 'getLog': {
        const limit = Math.min(Math.max(Math.trunc(Number(message.limit)) || 1, 1), LOG_LIMIT);
        const log = on.repo && (await reads.branchLog(on.repo, limit).catch(() => null));
        return log ? this.post({ type: 'log', log }, webview) : undefined;
      }
      case 'getDetail': {
        const detail = (repo && (await reads.commitDetail(repo, message.hash).catch(() => null))) ?? null;
        return this.post({ type: 'detail', hash: message.hash, detail }, webview);
      }
      case 'getSignature': {
        // The read of the commit selected before is no longer waited for: going down the commits reads the last alone.
        this.signatureReads.get(webview)?.abort();
        const reading = new AbortController();
        this.signatureReads.set(webview, reading);
        const signature = (repo && (await reads.signature(repo, message.hash, reading.signal).catch(() => null))) ?? null;
        if (reading.signal.aborted) return;
        this.signatureReads.delete(webview);
        return this.post({ type: 'signature', hash: message.hash, signature }, webview);
      }
      case 'pickCoAuthors':
        return repo && this.pickCoAuthors(repo, message.chosen, webview);
      case 'getPatch': {
        const detail = repo && (await reads.commitDetail(repo, message.hash).catch(() => null));
        const patch = (repo && detail && (await reads.commitPatch(repo, detail).catch(() => null))) ?? null;
        return this.post({ type: 'patch', hash: message.hash, patch }, webview);
      }
      case 'openCommit':
        return this.openCommit(message.hash, repo);
      case 'reveal':
        // In the graph of the group that asks; from a tab of a commit, in the panel, pointed at the repository the commit
        // is of when it shows another.
        if (this.pinned.has(webview)) {
          if (repo && repo.root !== this.repo?.root) await this.selectRepo(repo.root);
          return this.reveal(message.hash);
        }
        return this.reveal(message.hash, on);
      case 'setFilesLayout':
        return void this.memento.update(FILES_KEY, message.layout === 'list' ? 'list' : 'tree');
      case 'compare':
        return this.compare(message.from, on);
      case 'openCompare':
        return this.openCompare(message.from, message.to, repo);
      case 'pickCompareSide':
        return compareTab && this.pickCompareSide(compareTab, message.side === 'from' ? 'from' : 'to');
      case 'swapCompare':
        if (!compareTab) return;
        [compareTab.from, compareTab.to] = [compareTab.to, compareTab.from];
        this.rememberComparison(compareTab.repo, compareTab.from, compareTab.to, compareTab.path);
        return this.sendCompare(compareTab, true);
      case 'setCompareMode':
        if (!compareTab) return;
        compareTab.mode = message.mode === 'diverged' ? 'diverged' : 'direct';
        return this.sendCompare(compareTab, true);
      case 'setComparePath':
        if (!compareTab) return;
        compareTab.path = cleanPath(message.path);
        this.rememberComparison(compareTab.repo, compareTab.from, compareTab.to, compareTab.path);
        return this.sendCompare(compareTab, true);
      case 'takeCompareFile':
        return compareTab ? this.takeCompareFile(compareTab, message.file) : undefined;
      case 'copyComparePatch':
        return compareTab ? this.copyComparePatch(compareTab) : undefined;
      case 'formatPatch':
        return repo && this.formatPatch(repo, message.hashes, message.save === true);
      case 'openCompareOnService':
        return compareTab ? this.openCompareOnService(compareTab) : undefined;
      case 'show':
        return this.show(message.hash, on);
      case 'trace':
        return this.trace(message.hash, on);
      case 'getComparison': {
        const comparison = on.repo && (await reads.compare(on.repo, message.from, message.to).catch(() => null));
        return comparison ? this.post({ type: 'comparison', comparison }, webview) : undefined;
      }
      case 'selectRepo':
        return this.selectRepo(message.root, on);
      case 'openInTab':
        return typeof message.root === 'string' ? this.openTab(message.root) : undefined;
      case 'keepMessage':
        return this.keepMessage(webview, message.root, message.message, message.shown);
      case 'openDiff':
        return this.openDiff(message.source, message.file, repo);
      case 'readImages':
        return this.readImages(message.source, message.file, webview, repo);
      case 'compareFile':
        return this.compareFile(message.source, message.file, message.names, repo);
      case 'openFile':
        return this.openFile(message.file, repo);
      case 'openChange':
        return vscode.workspace.getConfiguration('gitardo').get('changes.openOnClick') === 'diff'
          ? this.openDiff({ kind: 'change' }, message.file, repo)
          : this.openFile(message.file, repo);
      case 'openChanges':
        return this.openChanges(message.title, message.source, message.files, repo);
      case 'save':
        return this.save(message.paths, repo);
      case 'compareWithSaved':
        if (!repo || typeof message.path !== 'string') return;
        return void vscode.commands.executeCommand('workbench.files.action.compareWithSaved', vscode.Uri.file(path.join(repo.root, message.path)));
      case 'sync':
        return this.sync(message.action, on);
      case 'action':
        return this.action(message.action, on);
      case 'copy':
        return void vscode.env.clipboard.writeText(message.text);
    }
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

  /** Says that an operation failed or stopped on conflicts, with what can be done next. */
  private reportFailure(op: { title: string }, error: string, repo: Repository): void {
    const showLog = (choice?: string) => choice && this.log.show();
    const state = this.heldFor(repo.root);
    const on = this.groupShowing(repo.root);
    const conflicts = state?.workingTree.conflicted.length ?? 0;
    if (state && conflicts) {
      const { operation } = state;
      const files = conflicts === 1 ? '1 file has conflicts' : `${conflicts} files have conflicts`;
      // Without an operation to continue, as after a stash that did not apply cleanly, staging the files is all there is to do.
      const then = operation ? `continue or abort the ${operationName(operation)}` : 'stage the files';
      const marked = state.conflictFiles.reduce((sum, file) => sum + file.conflicts, 0);
      const resolvable = state.conflictFiles.reduce((sum, file) => sum + file.resolvable.length, 0);
      // The ones that need no choice between the two sides are a click away.
      const which = marked === 1 ? 'the conflict' : resolvable === marked ? `all ${marked} conflicts` : `${resolvable} of the ${marked} conflicts`;
      const offer = resolvable ? ` Gitardo can resolve ${which} automatically.` : '';
      // The language model is offered for what the rules leave.
      const ai = this.resolveWithAI() && (!marked || resolvable < marked);
      const items = [...(resolvable ? ['Resolve Automatically'] : []), ...(ai ? ['Resolve with AI'] : []), 'Show Log'];
      void vscode.window.showWarningMessage(`${op.title} stopped: ${files}. Resolve them, then ${then}.${offer}`, ...items).then((choice) => {
        if (choice === 'Resolve Automatically') void this.action({ kind: 'resolveConflicts' }, on);
        if (choice === 'Resolve with AI') void this.reviewConflicts(undefined, true, on);
        if (choice === 'Show Log') this.log.show();
      });
      return;
    }
    // Git prefixes the line that matters; hints and progress come after it.
    const lines = error.split('\n').filter(Boolean);
    const summary = lines.find((line) => /^(error|fatal):/.test(line)) ?? lines[0] ?? 'unknown error';
    void vscode.window.showErrorMessage(`${op.title} failed. ${summary}`, 'Show Log').then(showLog);
  }

  /** The two versions of a file that a diff of `source` puts side by side, and what to call the pair. */
  private sides(repo: Repository, source: DiffSource, file: FileChange): { left: vscode.Uri; right: vscode.Uri; label: string } | null {
    const added = file.kind === 'added';
    const deleted = file.kind === 'deleted';
    const before = file.originalPath ?? file.path;
    const held = this.heldFor(repo.root);
    const head = held?.head.commit ?? '';
    const onDisk = vscode.Uri.file(path.join(repo.root, file.path));
    const at = (ref: string, filePath = file.path) => contentUri(repo.root, ref, filePath);
    // The index and HEAD as VS Code's Git extension serves them, when it has the repository: its own commands, such as
    // Stage Selected Ranges, then know the diff, as they only do for its URIs. Gitardo serves them otherwise.
    const vscodeGit = (source.kind === 'staged' || source.kind === 'unstaged') && inVSCodeGit(repo.root);
    const inGit = (ref: '~' | 'HEAD', filePath = file.path) => {
      if (!vscodeGit) return at(ref === '~' ? INDEX : head, filePath);
      const uri = vscode.Uri.file(path.join(repo.root, filePath));
      return uri.with({ scheme: 'git', query: JSON.stringify({ path: uri.fsPath, ref }) });
    };

    switch (source.kind) {
      case 'commit': {
        // Either side of a comparison can be the working tree.
        const { hash, parent } = source;
        if (!isSide(hash) || (parent !== null && !isSide(parent)) || hash === parent) return null;
        const name = (ref: string | null) => (ref === WORKTREE ? 'Working Tree' : ref ? ref.slice(0, 7) : 'empty');
        const left = added || !parent ? at('', before) : parent === WORKTREE ? vscode.Uri.file(path.join(repo.root, before)) : at(parent, before);
        const right = deleted ? at('') : hash === WORKTREE ? onDisk : at(hash);
        return { left, right, label: `${name(parent)} ↔ ${name(hash)}` };
      }
      case 'worktree':
        return { left: at(added ? '' : head, before), right: deleted ? at('') : onDisk, label: 'Working Tree' };
      case 'staged':
        return { left: added ? at('') : inGit('HEAD', before), right: deleted ? at('') : inGit('~'), label: 'Index' };
      case 'unstaged':
        return { left: added ? at('') : inGit('~'), right: deleted ? at('') : onDisk, label: 'Working Tree' };
      case 'change': {
        // The file as git lists it on that side: added to the index and changed since is `A` there, `M` here.
        const working = held?.workingTree;
        // An untracked file is all in the working tree.
        const unstaged = working?.unstaged.find((each) => each.path === file.path) ?? (working?.untracked.includes(file.path) ? file : undefined);
        if (unstaged || !working) return this.sides(repo, { kind: 'unstaged' }, unstaged ?? file);
        return this.sides(repo, { kind: 'staged' }, working.staged.find((each) => each.path === file.path) ?? file);
      }
      case 'conflict':
        return null;
    }
  }

  /** Opens the diffs of several files in a single editor, one under the other. */
  private async openChanges(title: string, source: DiffSource, files: FileChange[], repo = this.repo): Promise<void> {
    if (!repo) return;
    if (WORKING_SOURCES.has(source.kind)) await refreshVSCodeGit(repo.root);
    const resources = files.flatMap((file) => {
      const sides = this.sides(repo, source, file);
      return sides ? [[vscode.Uri.file(path.join(repo.root, file.path)), sides.left, sides.right]] : [];
    });
    if (resources.length) await vscode.commands.executeCommand('vscode.changes', title, resources);
  }

  /**
   * A file of the working tree in its editor, or a Markdown file in VS Code's preview of Markdown while
   * `gitardo.markdown.openAsPreview` is on. A deleted one is not there to open: its diff shows what it was.
   */
  private async openFile(file: FileChange, repo = this.repo): Promise<void> {
    if (!repo) return;
    if (file.kind === 'deleted') return this.openDiff({ kind: 'change' }, file, repo);
    const uri = vscode.Uri.file(path.join(repo.root, file.path));
    const openAsPreview = vscode.workspace.getConfiguration('gitardo').get<boolean>('markdown.openAsPreview') ?? false;
    if (openingOf(file.path, openAsPreview) === 'preview') {
      try {
        return void (await vscode.commands.executeCommand('markdown.showPreview', uri));
      } catch (error) {
        // VS Code's Markdown extension may be turned off: the file still opens, in its editor.
        this.log.warn(`Could not open the preview of ${file.path}: ${errorText(error)}`);
      }
    }
    await vscode.commands.executeCommand('vscode.open', uri);
  }

  /**
   * Sends a page the two versions of an image file of a commit or of a comparison, as the diff editor would compare
   * them, each as a `data:` URI unless it is over 10 MB (`commit-page.images`).
   */
  private async readImages(source: DiffSource, file: FileChange, webview: vscode.Webview, repo = this.repo): Promise<void> {
    const sides = repo && source.kind === 'commit' ? this.sides(repo, source, file) : null;
    const read = async (uri: vscode.Uri, filePath: string) => {
      if (isAbsent(uri)) return null;
      try {
        return { bytes: await vscode.workspace.fs.readFile(uri), path: filePath };
      } catch {
        return null;
      }
    };
    const [before, after] = sides
      ? await Promise.all([read(sides.left, file.originalPath ?? file.path), read(sides.right, file.path)])
      : [null, null];
    const images = imagePair(before, after, (bytes) => Buffer.from(bytes).toString('base64'));
    return this.post({ type: 'images', source, path: file.path, images }, webview);
  }

  private async openDiff(source: DiffSource, file: FileChange, repo = this.repo): Promise<void> {
    if (!repo) return;
    const onDisk = vscode.Uri.file(path.join(repo.root, file.path));

    switch (source.kind) {
      case 'commit':
      case 'worktree':
      case 'staged':
      case 'unstaged':
      case 'change': {
        if (WORKING_SOURCES.has(source.kind)) await refreshVSCodeGit(repo.root);
        const sides = this.sides(repo, source, file);
        if (!sides) return;
        const title = `${path.posix.basename(file.path)} (${sides.label})`;
        await vscode.commands.executeCommand('vscode.diff', sides.left, sides.right, title);
        return;
      }
      case 'conflict': {
        // The merge editor needs all three sides. A file one side deleted shows what the other changed of it; one both
        // sides added, or both deleted, opens as it is.
        const stages = await reads.conflictStages(repo, file.path);
        if ([1, 2, 3].every((stage) => stages.includes(stage))) {
          try {
            // Not part of the documented API, hence the fallback.
            await openMergeEditor(repo.root, file.path, this.heldFor(repo.root)?.operation ?? null);
            return;
          } catch (error) {
            this.log.warn(`Could not open the merge editor: ${errorText(error)}`);
          }
        }
        if (await this.openDeletedConflict(repo, file.path, stages)) return;
        await vscode.commands.executeCommand('vscode.open', onDisk);
        return;
      }
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

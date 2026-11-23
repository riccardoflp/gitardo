import { createHash } from 'node:crypto';
import * as path from 'node:path';
import * as vscode from 'vscode';
import { ThemeService } from '../flow/theme';
import { CONFLICT_SIDES } from '../shared/conflicts';
import { pullWords, readsPullRequests, type Hosting } from '../shared/hosting';
import { openingOf } from '../shared/markdown';
import { samePath } from '../shared/paths';
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
  PullRequest,
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
import { githubSession } from './github';
import { changeSetup, editSetupJson, savePatterns, saveSetup, setupChangeText, setupPatterns, worktreeSetupInfo } from './worktreeSettings';
import { EngineError, ErrorCode } from '../engine/connection';
import { type EngineRun, engineRuns, syncRuns } from '../engine/actions';
import type { FileResolutions, Finished, FollowUp, OfferedRun, Plan, Step, Target } from '../engine/messages';
import { type HostHooks, VscodeAsker, type WindowEngine, sayEngineMissing } from '../engine/vscode';
import { runSetupCommands } from './worktreeCommands';
import { type Repository, reads, repositoryName } from '../engine/reads';
import { type CommitsReader, EngineGraph, completed, graphState, historyOf, laidOutOf, matchesOnEngine, positionOnEngine, readCommits } from '../engine/graph';
import type { EngineClient } from '../engine/client';
import type { ComparisonTarget, CompareSideTarget, WriteSettingFollowUp } from '../engine/messages';
import { extensionKey } from '../engine/settings';
import { RangeDiffTab } from './rangeDiffTab';
import { type Sent, stateFor } from './sentParts';
import { partsOf } from '../shared/stateParts';
import { COMPARE_REWRITE, SEE_WHAT_CHANGED, buttonWords, forcePushed, type RangeDiffTarget } from '../shared/rangeDiff';

/** How many commits the graph reads at a time, unless the settings ask for another number between the two after it. */
const PAGE_SIZE = 1000;
const SMALLEST_PAGE = 100;
const LARGEST_PAGE = 10_000;

const REFRESH_DELAY = 400;
/** Builds and installs write many files at once, so those are given longer to settle. */
const WORKTREE_REFRESH_DELAY = 1000;
/** How long the pages of a read wait for the next before the views get them together, so that they are drawn less often. */
const PAGES_DELAY = 150;
/** How old the last read is when the window getting the focus asks for one (`repo.watch-changes`, Q463). */
const FOCUS_READ_AFTER = 60_000;
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
/** Pull requests change on GitHub's side, so they are re-read at most this often unless something asks for it. */
const PULLS_MAX_AGE = 120_000;
/** The comparisons opened last in each repository, by its folder. */
const RECENT_KEY = 'gitardo.recentComparisons';
/** How many of them are kept. */
const RECENT_COMPARISONS = 8;
/** How many subjects of the last commits a draft of a commit message is written with. */
const DRAFT_SUBJECTS = 10;

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

const errorText = (error: unknown) => (error instanceof Error ? error.message : String(error));

/** Whether a read failed because it was cancelled for another. */
const isCancelled = (error: unknown) => error instanceof EngineError && error.code === ErrorCode.cancelled;

/** What git said when a read failed in the engine; the error itself otherwise. */
function failureText(error: unknown): string {
  const output = error instanceof EngineError ? (error.data as { output?: string | null } | undefined)?.output?.trim() : undefined;
  return output || errorText(error);
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
 * Whether a repository left keeps what the views showed of it, and is read again only once it is shown when it changed
 * meanwhile, as `gitardo.repositories.keepOpen` asks (`repo.keep-open`, `repo.watch-hidden`). Off by default.
 */
function keepOpen(): boolean {
  return vscode.workspace.getConfiguration('gitardo').get('repositories.keepOpen') === true;
}

/** How many commits the graph reads at a time, as `gitardo.graph.pageSize` asks. */
function pageSize(): number {
  const wanted = Math.trunc(Number(vscode.workspace.getConfiguration('gitardo').get('graph.pageSize')));
  return Number.isFinite(wanted) ? Math.min(Math.max(wanted, SMALLEST_PAGE), LARGEST_PAGE) : PAGE_SIZE;
}

/** Owns the repository state and serves every Gitardo webview, wherever the user has placed it. */
export class Controller implements vscode.Disposable {
  private repo?: Repository;
  private state: RepoState | null = null;
  private located = false;
  /** The search for the repositories of the workspace and the first read of the one shown, while they run. */
  private locating?: Promise<void>;
  private limit = pageSize();
  /** The read of the repository under way, if any. */
  private loading?: Promise<void>;
  /** Who waits for a read that starts after they asked: each is let go once one is done, or when reading stops. */
  private waiters: (() => void)[] = [];
  /** What changed last that the repository is read again for, as the status bar says it. */
  private cause?: string;
  /** What the views were told the graph is read for, and how many of those reads are not done yet. */
  private waiting: { what: Loading; count: number } | null = null;
  /** How many operations were started, to tell a state that was read across one. */
  private operations = 0;
  /** Something changed since that read started. */
  private dirty = false;
  /** Counts the changes heard of that ask for a read, so that a view that opens can tell whether the state held saw them all. */
  private heard = 0;
  /** How many changes had been heard of when the read under way started. */
  private readingHeard = 0;
  /** How many changes had been heard of when the state held was read; -1 for a state that no read of this window made. */
  private stateHeard = -1;
  /** The operation `runOnEngine` runs, which another one asked meanwhile is refused for. */
  private operation: { feature: string; root: string; asker: VscodeAsker } | undefined;
  /** Whether an operation runs: nothing is read meanwhile. */
  private get running(): boolean {
    return this.operation !== undefined;
  }
  /** Gitardo's engine, which reads the repository and runs every operation on it; without it, the views say so (D49). */
  private engine?: WindowEngine;
  /**
   * The commits of the graph the engine reads, the pages of each read added as they come (stage 7, D54), and kept while
   * the history is the same.
   */
  private readonly engineGraph = new EngineGraph((repository) => this.pageRead(repository));
  /** When the pages read since the state was sent are sent to the views. */
  private pagesTimer?: ReturnType<typeof setTimeout>;
  /** The next read is a **Refresh**, or the read of a repository chosen: every worktree is counted again, every commit read. */
  private everything = false;
  /** When the last read of each repository ended, by its root, for the focus (`repo.watch-changes`). */
  private readonly readAt = new Map<string, number>();
  /** A change came while no view could show it, so that the focus asks for a read. */
  private missed = false;
  /** The files of the working tree changed since the last read, which the engine is asked about a second after the first. */
  private changedFiles = new Set<string>();
  /** When the files changed are asked about; none while none waits. */
  private filesTimer?: ReturnType<typeof setTimeout>;
  /** Counts the reads started, so that an answer about the files asked before one is not waited for any more. */
  private readsStarted = 0;
  /** The read of the commits of the graph under way in the engine, to cancel when another takes its place. */
  private graphRead?: AbortController;
  private timer?: ReturnType<typeof setTimeout>;
  /** When `timer` fires. */
  private due = 0;
  private watcher?: vscode.Disposable;
  private filter: Filter | null = null;
  /** Which branches the graph shows: all of them when null. */
  private scope: Scope | null = null;
  /** The hosting service of the main remote, as the engine found it (`hosting.detect-remote`). */
  private hosting: Hosting | null = null;
  /** Forgets the token of a GitLab that the GitLab refused, so that signing in asks for another (`hosting.token`). */
  private forgetToken?: (host: string) => Promise<void>;
  private pulls: Record<string, PullRequest> = {};
  private pullsAt = 0;
  /** A commit to show as soon as a graph view is there to show it. */
  private pendingReveal?: string;
  /** Every repository of the workspace; `repo` is the one being shown. */
  private repos: RepoInfo[] = [];
  /** The repository of the workspace shown before a worktree of it that is not in the workspace. */
  private home?: RepoInfo;
  /** The repositories left that keep what they showed, with `gitardo.repositories.keepOpen` on. */
  private readonly kept = new KeptRepositories();
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
  private readonly subscriptions: vscode.Disposable[];
  /** What Gitardo is doing that takes time, in the status bar. */
  readonly activity: Activity;
  /** The tab of what a rewrite changed, commit by commit (`range-diff.page`). */
  private readonly rangeDiffTab: RangeDiffTab;

  /** Fires whenever the repository state has been reloaded. */
  readonly onDidChangeState = this.stateEmitter.event;

  /** The repository state the views were sent last, for a view that opens after it was read. */
  get shownState(): RepoState | null {
    return this.state;
  }
  private readonly chosenEmitter = new vscode.EventEmitter<string>();
  /** Fires with the root of the repository or worktree chosen to be shown, once the views show it. */
  readonly onDidChooseRepo = this.chosenEmitter.event;

  constructor(
    private readonly extensionUri: vscode.Uri,
    private readonly log: vscode.LogOutputChannel,
    /** Remembers which repository of this workspace was chosen, and which branches of each are shown. */
    private readonly memento: vscode.Memento,
  ) {
    const schedule = (cause: string) => () => this.scheduleRefresh(cause);
    const unsaved = () => void this.checkUnsaved();
    this.fileIcons = new FileIcons(log);
    this.roots = this.resourceRoots();
    this.activity = new Activity(log);
    this.rangeDiffTab = new RangeDiffTab({
      attach: (webview, kind, data) => this.attach(webview, kind, data),
      pin: (webview, repo) => (repo ? this.pinned.set(webview, repo) : this.pinned.delete(webview)),
      track: (title, work) => this.activity.track(title, work),
      post: (message, to) => this.post(message, to),
    });
    this.repositoriesView = new RepositoriesView(log);
    this.subscriptions = [
      this.stateEmitter,
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
          this.engineGraph.readAgain();
          this.scheduleRefresh('the settings changed');
          // The repositories kept were read with the settings before: each is read again once it is shown.
          this.kept.changedAll('the settings changed');
        }
        if (event.affectsConfiguration('gitardo.repositories.keepOpen')) this.keepOpenChanged();
        // After the engine is told of the settings, in a listener of its own that may come after this one.
        if (event.affectsConfiguration('gitardo.hosting')) setTimeout(() => void this.hostingChanged(), 0);
        if (event.affectsConfiguration('gitardo.conflicts')) void this.reviewTab?.reload();
      }),
      // The branches a comparison is of may have moved, and the conflicts of the review may be resolved.
      this.onDidChangeState(() => {
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
    clearTimeout(this.timer);
    clearTimeout(this.pagesTimer);
    clearTimeout(this.filesTimer);
    this.engineGraph.forget();
    this.watcher?.dispose();
    this.kept.dispose();
    for (const subscription of this.subscriptions) subscription.dispose();
  }

  /** Loads the UI into a webview and wires it to this controller. `data` is written on the page for the view to read. */
  attach(webview: vscode.Webview, kind: ViewKind, data: Record<string, string> = {}): vscode.Disposable {
    const dist = vscode.Uri.joinPath(this.extensionUri, 'dist');
    webview.options = { enableScripts: true, localResourceRoots: this.roots };
    // The session tells the page what it remembers from an earlier start of VS Code, for what is to start afresh each time.
    webview.html = this.html(webview, dist, { view: kind, session: vscode.env.sessionId, ...data });
    this.views.set(webview, kind);
    const listener = webview.onDidReceiveMessage((message: WebviewMessage) => this.onMessage(webview, message));
    // The Repositories view is sent its rows, which the engine follows the folders of while it is open.
    const rows = kind === 'repositories' ? this.repositoriesView.open(webview) : undefined;
    return new vscode.Disposable(() => {
      listener.dispose();
      rows?.dispose();
      this.views.delete(webview);
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
   * **Refresh**: reads the repository shown again whole, counting what is not committed in every other worktree and
   * reading every commit again, which the reads that follow a change keep when nothing changed there (M15).
   */
  async refreshAll(): Promise<void> {
    this.everything = true;
    return this.refresh();
  }

  async refresh(): Promise<void> {
    if (!this.located || !this.repo) return this.locating ?? this.locate();
    // A read already under way may have started before what the caller just did: it is followed by another,
    // and the caller waits for that one too.
    this.dirty = true;
    // Nothing is read while an operation runs: it reads once it is done. Reading runs git in every worktree, and on
    // Windows a folder git runs in can be neither moved nor removed.
    if (this.running) return this.loading;
    // Only for that one: while files keep changing, a large repository is read again and again, and what the caller
    // waits for is already there after the first.
    const read = new Promise<void>((resolve) => this.waiters.push(resolve));
    this.loading ??= this.load(this.repo);
    return read;
  }

  private async load(first: Repository): Promise<void> {
    const task = this.activity.start(this.readTitle(first));
    let waiting: (() => void)[] = [];
    try {
      for (let repo: Repository | undefined = first, reads = 1; repo && this.dirty && !this.running; repo = this.repo, reads++) {
        this.dirty = false;
        waiting = this.waiters.splice(0);
        const cause = this.cause;
        this.cause = undefined;
        // The read that starts now sees what the changes seen until now did: the read they had asked for is this one.
        clearTimeout(this.timer);
        this.timer = undefined;
        task.describe(readDetail(reads, cause), this.readTitle(repo));
        // The read sees what changed until now: the files waiting to be asked about, and a change the focus missed.
        this.readStarted();
        const heard = (this.readingHeard = this.heard);
        const everything = this.everything;
        this.everything = false;
        const operations = this.operations;
        const scope = this.scope;
        let state: RepoState | null;
        try {
          state = await this.readState(repo, everything);
        } catch (error) {
          // An operation that ran meanwhile can take from under the read what it was reading, such as a conflict
          // resolved while its versions were read: the read is dropped as below, and the one after it says what is there.
          if (!this.running && operations === this.operations && repo === this.repo) throw error;
          state = null;
        }
        // Read while an operation ran, it can be half from before it and half from after, such as a new commit that
        // no branch points at yet. It is dropped: the operation is followed by a read of its own.
        // Unless the views were pointed at another repository in the meantime. Who waited for it waits for the next,
        // as for a read of the commits cancelled for another.
        if (state) this.readAt.set(repo.root, Date.now());
        if (!state || this.running || operations !== this.operations || repo !== this.repo) {
          // A repository left while it was read keeps what it showed before: it is read again once it is shown.
          if (repo !== this.repo) this.kept.changed(repo.root, cause ?? 'it was left while it was read');
          this.waiters.unshift(...waiting.splice(0));
          continue;
        }
        // Unless another was chosen while this one was read.
        if (scope === this.scope) this.settleScope(state.root, state.scope);
        const sort = branchSort();
        const read = state;
        // The pages of the read that come while the rest is read are laid out under its rows once it is: a read that
        // ended meanwhile would not be told to the views after.
        const shown = await completed(sortRefs(read, sort), async () => ({
          favorites: this.favorites(read),
          defaultPull: this.defaultPull(),
          repos: this.repos,
          hosting: this.hosting,
          pulls: this.pulls,
          agentsWindow: await (this.agentsWindow ??= hasCommand(AGENTS_WINDOW)),
          undo: await this.newestInJournal(read.root),
          worktreesRoot: worktreesFolder(read),
          openBranches: vscode.workspace.getConfiguration('gitardo').get('branches.openIn') === 'worktree' ? ('worktree' as const) : ('checkout' as const),
          listed: this.listedIn(read),
          sortOrder: sort,
          traceSelected: vscode.workspace.getConfiguration('gitardo').get('graph.traceSelected') !== false,
          oneLane: vscode.workspace.getConfiguration('gitardo').get('graph.oneLane') === true,
          graphColumns: graphColumns(),
          dateSeparators: vscode.workspace.getConfiguration('gitardo').get('graph.dateSeparators') !== false,
          combineUpstream: vscode.workspace.getConfiguration('gitardo').get('graph.combineUpstream') !== false,
          islands: vscode.workspace.getConfiguration('gitardo').get('graph.islands') === true,
          resolveWithAI: this.resolveWithAI(),
          home: this.homeOf(read.root),
          keepOpen: keepOpen(),
          signOff: vscode.workspace.getConfiguration('gitardo').get('commit.signOff') === true,
          tagMarks: this.tagMarks.get(read.root) ?? null,
        }));
        this.stateHeard = heard;
        this.setState(shown);
        for (const resolve of waiting.splice(0)) resolve();
      }
    } catch (error) {
      this.log.error(errorText(error));
      this.stateHeard = -1;
      this.setState(null);
    } finally {
      // In the same turn as the last look at `dirty`, so that no request can fall between the two.
      this.dirty = false;
      this.loading = undefined;
      for (const resolve of [...waiting, ...this.waiters.splice(0)]) resolve();
      task.end();
    }
    // Not for the read at the start of VS Code: GitHub is asked only once a view can show what it says.
    if (this.views.size) void this.refreshPulls(false);
  }

  /** The repository of the workspace the views go back to, while they show a worktree of it that is not in the workspace. */
  private homeOf(root: string): RepoInfo | null {
    return this.repos.some((each) => samePath(each.root, root)) ? null : (this.home ?? null);
  }

  /**
   * Turned off, the repositories kept forget at once what they showed, and the views forget what they kept of them;
   * turned on, the repository shown keeps what it shows once it is left.
   */
  private keepOpenChanged(): void {
    if (!keepOpen()) this.kept.forget();
    if (this.state) this.setState({ ...this.state, keepOpen: keepOpen() });
  }

  /** What the status bar says the repository is read for, by the name the list of repositories gives it. */
  private readTitle(repo: Repository): string {
    const name = this.repos.find((each) => samePath(each.root, repo.root))?.name ?? path.basename(repo.root);
    const searching = this.filter?.kind === 'pickaxe' || this.filter?.kind === 'regex';
    return `${searching ? 'Searching' : 'Reading'} ${name}`;
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
  async reviewConflicts(paths?: string[], ai = false): Promise<void> {
    const repo = (await this.requireState()) && this.repo;
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

  /**
   * A search git cannot run, such as an invalid regular expression, is reported and dropped rather than taking the graph
   * down. Null when the read of the commits was cancelled, for a read that takes its place.
   */
  private async readState(repo: Repository, everything = false): Promise<RepoState | null> {
    const client = reads.client();
    const stashes = vscode.workspace.getConfiguration('gitardo').get('graph.showStashes') !== false;
    const firstParent = this.firstParent();
    // **Refresh** reads every commit again; the other reads keep them while the history is the same (Q442). The rows of
    // the commits it finds the same stay as they are, and the views are sent none of them (Q781).
    if (everything) this.engineGraph.readAgain(repo.root);
    // What the views show but the commits, and then the commits of the graph from what it says (stage 7).
    const parts = await reads.parts(repo, this.autoResolve(), everything);
    const reader = this.graphReader(client, repo);
    const asked = (filter: Filter | null) => ({ limit: this.limit, filter, scope: this.scope, stashes, firstParent });
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
        this.log.error(text);
        void vscode.window.showErrorMessage(`Gitardo: the search failed. ${text.split('\n')[0]}`);
        return await graphState(client, repo.root, parts, asked(null), reader, undefined, shown);
      }
    } catch (error) {
      if (isCancelled(error)) return null;
      throw error;
    }
  }

  /** Whether the graph follows only the first parent of each merge, as `gitardo.graph.firstParent` asks. */
  private firstParent(): boolean {
    return vscode.workspace.getConfiguration('gitardo').get('graph.firstParent') === true;
  }

  /** The state of a repository the views may not show, with one commit only: for choosing among its branches. */
  private async stateOf(repo: Repository): Promise<RepoState> {
    const client = reads.client();
    const asked = { limit: 1, filter: null, scope: null, stashes: false, firstParent: false };
    return graphState(client, repo.root, await reads.parts(repo), asked, (graph) => readCommits(client, repo.root, graph, 1));
  }

  // The graph read by the engine (stage 7): its rows, the position of a commit and the matches of a search.

  /**
   * What reads the commits of the graph in the engine: in pages of `gitardo.graph.pageSize`, the next ones from the last
   * row read. A read that something else takes the place of is cancelled.
   */
  private graphReader(client: EngineClient, repo: Repository): CommitsReader {
    return async (asked) => {
      this.graphRead?.abort();
      const reading = new AbortController();
      this.graphRead = reading;
      try {
        return await this.engineGraph.commits(client, repo.root, asked, pageSize(), reading.signal);
      } finally {
        if (this.graphRead === reading) this.graphRead = undefined;
      }
    };
  }

  /** Cancels the read of the commits under way, which a read of another history or of more commits takes the place of. */
  private cancelGraphRead(): void {
    this.graphRead?.abort();
    this.graphRead = undefined;
  }

  /**
   * A page of the commits of a repository was read: once the pages stop coming for a moment, or the read ends, the
   * views get their rows, laid out under the rows they have (`graph.load-more`, `graph.lanes`).
   */
  private pageRead(repository: string): void {
    if (!this.state || !samePath(this.state.root, repository)) return;
    if (!this.engineGraph.isReading(repository)) return this.showPages();
    this.pagesTimer ??= setTimeout(() => this.showPages(), PAGES_DELAY);
  }

  /** Sends the views the rows of the pages read since the state they have, without reading anything. */
  private showPages(): void {
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
    for (const [view, kind] of this.views) if (showsGraph(kind)) this.post({ type: 'state', state: this.state }, view);
  }

  /** How many commits come before one in the history listed with a filter and a scope, or -1 when it is not there. */
  private async positionOf(repo: Repository, hash: string, filter: Filter | null = null, scope: Scope | null = null): Promise<number> {
    if (!isHash(hash)) return -1;
    const client = reads.client();
    try {
      const history = await historyOf(client, repo.root, filter, scope, this.state?.head.commit ?? null, this.firstParent());
      return await positionOnEngine(client, repo.root, hash, history);
    } catch (error) {
      // A history git cannot list has no commit in it.
      if (error instanceof EngineError && error.code === ErrorCode.gitFailed) return -1;
      throw error;
    }
  }

  /** The commits of the history listed that a search finds, newest first; it throws when git refuses the search. */
  private async matchesOf(repo: Repository, search: Search, filter: Filter | null, scope: Scope | null): Promise<{ hashes: string[]; more: boolean }> {
    const client = reads.client();
    const history = await historyOf(client, repo.root, filter, scope, this.state?.head.commit ?? null, this.firstParent());
    return matchesOnEngine(client, repo.root, search, history);
  }

  /** Narrows the graph to the history of some branches, or shows them all again. What the filter does is left to it. */
  async setScope(scope: Scope | null): Promise<void> {
    await this.whenLocated();
    // As it is given: reading the repository leaves out what is not a branch of it.
    this.scope = scope && typeof scope === 'object' ? scope : null;
    this.limit = pageSize();
    this.cancelGraphRead();
    await this.reload('filter');
  }

  /**
   * Reads the graph again, or does what ends with that, telling the views what for while they wait. Reads that overlap
   * are one wait: the views are told it is over when the last of them is, and what for by the first.
   */
  private async reload(what: Loading, work = () => this.refresh()): Promise<void> {
    if (this.waiting) this.waiting.count++;
    else {
      this.waiting = { what, count: 1 };
      this.post({ type: 'loading', what });
    }
    try {
      await work();
    } finally {
      if (!--this.waiting!.count) {
        this.waiting = null;
        this.post({ type: 'loading', what: null });
      }
    }
  }

  /** The scope as git took it, without the branches deleted since they were chosen, is the one in use and the one remembered. */
  private settleScope(root: string, scope: Scope | null): void {
    this.scope = scope;
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

  /**
   * Chooses what the references list, in the repository shown, apart from the branches shown in the graph
   * (`refs.list-filter`). Nothing is read: the views are told at once.
   */
  async setListed(listed: unknown): Promise<void> {
    const state = await this.requireState();
    if (!state) return;
    const kept = pruneScope(listed, state);
    const others = Object.fromEntries(Object.entries(this.memento.get<Record<string, unknown>>(LISTED_KEY, {})).filter(([root]) => root !== state.root));
    await this.memento.update(LISTED_KEY, kept ? { ...others, [state.root]: kept } : others);
    if (this.state?.root === state.root) this.setState({ ...this.state, listed: kept });
  }

  /** Marks a branch as a favorite, or no longer. Nothing of the repository changes: the views are told at once. */
  async setFavorite(ref: string, favorite: boolean): Promise<void> {
    const state = await this.requireState();
    if (!state || typeof ref !== 'string') return;
    // The ones whose branch is gone are forgotten as soon as there is something to save.
    const others = this.favorites(state).filter((other) => other !== ref);
    const next = favorite ? [...others, ref] : others;
    await this.memento.update(FAVORITES_KEY, { ...this.memento.get<Record<string, unknown>>(FAVORITES_KEY, {}), [state.root]: next });
    if (this.state?.root === state.root) this.setState({ ...this.state, favorites: this.favorites(this.state) });
  }

  /** Narrows the graph to the history of a file or to a search, or shows everything again. */
  async setFilter(filter: Filter | null): Promise<void> {
    await this.whenLocated();
    this.filter = filter;
    this.limit = pageSize();
    this.cancelGraphRead();
    // Searching inside the changes of every commit can take a while on a large repository, and so can following lines.
    const slow = filter?.kind === 'pickaxe' || filter?.kind === 'regex' || filter?.kind === 'lines';
    if (slow) this.post({ type: 'busy', title: 'Searching' });
    try {
      await this.reload('filter');
    } finally {
      if (slow) this.post({ type: 'busy', title: null });
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

  /** Selects a commit in the graph, opening the graph first if needed. `hash` is the full one. */
  async reveal(hash: string): Promise<void> {
    if (!isHash(hash)) return;
    await vscode.commands.executeCommand('gitardo.graph.focus');
    await this.include(hash);
    // Only now: a view that loads in the meantime would be told of a commit it has not been sent yet.
    this.pendingReveal = hash;
    this.flushPending();
  }

  /**
   * Makes sure a commit is among the ones listed. The history is loaded as far back as it is; what narrows the list
   * gives way if it leaves the commit out; and a commit that no branch or tag reaches any more, such as one found in
   * the reflog, is shown with its own history.
   */
  private async include(hash: string): Promise<void> {
    await this.whenLocated();
    const repo = this.repo;
    if (!repo || this.state?.rows.some((row) => row.hash === hash)) return;
    const shown = this.filter;
    const scope = this.scope;
    let index = await this.positionOf(repo, hash, shown, scope);
    if (index < 0 && shown) {
      this.filter = null;
      index = await this.positionOf(repo, hash, null, scope);
    }
    // On a branch that is not among the ones shown: they are all shown again.
    if (index < 0 && scope) {
      this.scope = null;
      index = await this.positionOf(repo, hash);
    }
    if (index < 0) {
      this.filter = { kind: 'ref', ref: hash, lost: true };
      index = 0;
    }
    const page = pageSize();
    const pages = Math.floor(index / page) + 1;
    const same = this.filter === shown && this.scope === scope;
    this.limit = Math.max(same ? this.limit : page, pages * page);
    await this.reload(same ? 'more' : 'filter');
    // The read gives its first page at once: the commit is shown once the pages that have it are read.
    if (this.repo === repo && this.state) {
      await this.engineGraph.reached(repo.root, pages * page).catch(() => {});
      this.showPages();
    }
  }

  /** Selects, in the view that asked, the commit that a hash or anything else git understands stands for. */
  private async goTo(revision: unknown, asker: vscode.Webview): Promise<void> {
    const repo = this.repo;
    if (!repo || typeof revision !== 'string' || !revision.trim()) return;
    const hash = await reads.resolve(repo, revision.trim());
    if (!hash) {
      // A few characters can be the start of more than one hash, which git refuses to choose between.
      void vscode.window.showWarningMessage(`Gitardo: "${revision.trim()}" is not a commit of this repository, or is the start of several.`);
      return;
    }
    await this.include(hash);
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
  async openWorktreeSetup(): Promise<void> {
    const repo = (await this.requireState()) && this.repo;
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
      const state = this.state && samePath(this.state.root, root) ? this.state : null;
      const others = state?.worktrees.filter((worktree) => !worktree.isMissing && !samePath(worktree.folder, root)).length ?? 0;
      this.post({ type: 'worktreeSetup', info: { ...worktreeSetupInfo(root, others, tab.proposed), entries: tab.entries } }, webview);
    } catch (error) {
      this.log.error(errorText(error));
      this.post({ type: 'worktreeSetup', info: null, error: errorText(error) }, webview);
    }
  }

  /** Asks for a hash or a revision, and shows its commit in the graph. */
  async goToCommit(): Promise<void> {
    const repo = (await this.requireState()) && this.repo;
    if (!repo) return;
    const commit = await this.askRevision(repo, 'Go to commit');
    if (commit) await this.reveal(commit.hash);
  }

  /** Tells a view which commits a search finds in the history that is shown, for it to highlight them where they are. */
  private async find(search: Search, asker: vscode.Webview): Promise<void> {
    const repo = this.repo;
    const kinds = ['message', 'author', 'pickaxe', 'regex'];
    if (!repo || !kinds.includes(search?.kind) || typeof search.query !== 'string' || !search.query) return;
    try {
      // A list of results gives way to the graph, in which the new ones are to be shown.
      if (this.filter && 'query' in this.filter) await this.setFilter(null);
      const found = await this.matchesOf(repo, { kind: search.kind, query: search.query }, this.filter, this.scope);
      this.post({ type: 'found', search, ...found }, asker);
    } catch (error) {
      const text = failureText(error);
      this.log.error(text);
      void vscode.window.showErrorMessage(`Gitardo: the search failed. ${text.split('\n')[0]}`);
      this.post({ type: 'found', search, hashes: [], more: false, failed: true }, asker);
    }
  }

  /** Selects a commit in the views that list commits and are there, without opening any or taking the focus. */
  private show(hash: string): void {
    if (!isHash(hash)) return;
    for (const [view, kind] of this.views) if (showsGraph(kind)) this.post({ type: 'reveal', hash }, view);
  }

  /** Brings out the history of a commit in the views of the graph, or no longer, while the pointer rests on its branch. */
  private trace(hash: string | null): void {
    if (hash !== null && !isHash(hash)) return;
    for (const [view, kind] of this.views) if (showsGraph(kind)) this.post({ type: 'trace', hash }, view);
  }

  /** Hands over what was waiting for a graph view to be there. */
  private flushPending(): void {
    const views = (wanted: (kind: ViewKind) => boolean) => [...this.views].filter(([, kind]) => wanted(kind)).map(([view]) => view);
    const lists = views(showsGraph);
    if (this.pendingReveal && lists.length) {
      for (const view of lists) this.post({ type: 'reveal', hash: this.pendingReveal }, view);
      this.pendingReveal = undefined;
    }
  }

  /**
   * Compares two branches, tags or commits, or one with the working tree, in a tab, asking which. With `from`, only
   * the other side is asked. The first question also offers the comparisons opened last.
   */
  async compare(from?: ComparedRef): Promise<void> {
    const state = await this.requireState();
    const repo = this.repo;
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
    // The branches offered are those of the repository the file is in.
    await this.selectRepo(repo.root);
    const { state } = this;
    if (!state || !samePath(state.root, repo.root)) return;
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
    if (!again && !(this.state && samePath(this.state.root, repo.root))) return;
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
    const state = this.state && samePath(this.state.root, tab.repo.root) ? this.state : await this.stateOf(tab.repo).catch(() => null);
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
    const { state, hosting } = this;
    const { from, to } = tab;
    if (!state || !hosting || !samePath(state.root, tab.repo.root) || from.hash === WORKTREE || to.hash === WORKTREE) return;
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

  async command(command: UiCommand): Promise<void> {
    try {
      if (command === 'showLog') return this.log.show();
      if (command === 'activeFileHistory') return await this.fileHistory();
      if (command === 'signInGitHub') return await this.refreshPulls(true, true);
      if (command === 'signInGitLab') return await this.runHostingFeature('hosting.load-merge-requests');
      if (command === 'checkOutPullRequest') return await this.runHostingFeature('hosting.checkout-pull-request');
      if (command === 'reflog') return await this.browseReflog();
      if (command === 'goToCommit') return await this.goToCommit();
      if (command === 'cleanupBranches') return await this.cleanupBranches();
      if (command === 'draftMessage') return await this.draftMessage();
      if (command === 'switchIdentity') return await this.switchIdentity();
      if (command === 'selectRepository') return await this.pickRepo();
      if (command === 'remotes') return await this.manageRemotes();
      if (command === 'switchWorktree') return await this.switchWorktree();
      if (command === 'worktreeSetup') return await this.openWorktreeSetup();
    } catch (error) {
      void vscode.window.showErrorMessage(`Gitardo: ${errorText(error)}`);
    }
  }

  /**
   * Lists the worktrees with how they stand: the branch against its upstream, the files not committed, the pull request.
   * The one picked opens where the settings say; the last entry creates another.
   */
  private async switchWorktree(): Promise<void> {
    const state = await this.requireState();
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
    await this.action(picked === create ? { kind: 'worktreeAdd' } : { kind: 'worktreeOpen', path: picked.path });
  }

  /** Lists where HEAD has been, and offers what can be done with the chosen position, even if no branch reaches it any more. */
  private async browseReflog(): Promise<void> {
    const state = await this.requireState();
    if (!state || !this.repo) return;
    const entries = await reads.reflog(this.repo, 'HEAD', REFLOG_ENTRIES);
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
        { label: 'Show in the graph', run: () => this.reveal(hash) },
        { label: 'Create branch here…', run: () => this.action({ kind: 'createBranch', at: hash }) },
        { label: `Reset ${checkedOutBranch(state) ?? 'HEAD'} to here…`, run: () => this.action({ kind: 'reset', hash }) },
        { label: 'Check out (detached)', run: () => this.action({ kind: 'checkout', target: 'detached', ref: hash }) },
        { label: 'Cherry-pick', run: () => this.action({ kind: 'cherryPick', hash, isMerge: false }) },
      ],
      { title: `${short}: ${entry.item.subject}` },
    );
    await choice?.run();
  }

  /** Offers to delete the local branches that are merged into the current one, or whose upstream is gone. */
  private async cleanupBranches(): Promise<void> {
    const state = await this.requireState();
    if (!state || !this.repo) return;
    // Once the plan is confirmed, VS Code's Git first lets go of the worktrees it removes.
    await this.runOnEngine([{ feature: 'branch.clean-up', target: { kind: 'head' } }], this.repo, { confirmed: (plan) => this.closeRemoved(plan) });
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
  private async draftMessage(): Promise<void> {
    const state = await this.requireState();
    const repo = this.repo;
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
  private async runHostingFeature(feature: 'hosting.load-merge-requests' | 'hosting.checkout-pull-request'): Promise<void> {
    const repo = this.repo;
    if (!repo) return;
    await this.runOnEngine([{ feature, target: { kind: 'head' } }], repo);
  }

  /**
   * Reads the open pull requests from GitHub, or the merge requests of a GitLab (`hosting.read-merge-requests`). Without
   * `force` it reuses what it has for a while; without `prompt` it reads only with a token already there: the user
   * signed in to GitHub in VS Code, or the token of the GitLab kept in the secret storage.
   */
  private async refreshPulls(force: boolean, prompt = false): Promise<void> {
    const hosting = this.hosting;
    if (!hosting || !readsPullRequests(hosting.service)) {
      if (prompt) void vscode.window.showWarningMessage('Gitardo: the main remote of this repository is not on github.com or on a GitLab.');
      return;
    }
    if (!force && Date.now() - this.pullsAt < PULLS_MAX_AGE) return;
    const repo = this.repo;
    if (!repo) return;
    const gitLab = hosting.service === 'gitLab';
    const words = `${pullWords(hosting.service)}s`;
    try {
      // Signing in loads them with the engine's `github.load-pull-requests` or `hosting.load-merge-requests`, which
      // read them again.
      if (prompt) {
        await this.runOnEngine([{ feature: gitLab ? 'hosting.load-merge-requests' : 'github.load-pull-requests', target: { kind: 'head' } }], repo);
        return;
      }
      // On GitHub only once the user signed in in VS Code: the engine asks it for the same session (`host/githubToken`).
      if (!reads.available || (!gitLab && !(await githubSession(false)))) return;
      this.pullsAt = Date.now();
      const what = gitLab ? 'Reading the merge requests from GitLab' : 'Reading the pull requests from GitHub';
      const pulls = await this.activity.track(what, () => reads.pullRequests(repo.root, hosting.remote, hosting.service));
      if (hosting !== this.hosting || JSON.stringify(pulls) === JSON.stringify(this.pulls)) return;
      this.pulls = pulls;
      if (this.state) this.setState({ ...this.state, pulls });
    } catch (error) {
      // A token the GitLab refused is forgotten: the next sign-in asks for another.
      if (gitLab && (error as { isUnauthorized?: boolean }).isUnauthorized) await this.forgetToken?.(hosting.host);
      this.log.warn(`Could not read ${words}: ${errorText(error)}`);
      if (prompt) void vscode.window.showErrorMessage(`Gitardo: could not read ${words}. ${errorText(error)}`);
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
  async chooseConflictModel(): Promise<void> {
    const repo = (await this.requireState()) && this.repo;
    if (repo) await this.runOnEngine([{ feature: 'ai.choose-model', target: { kind: 'head' } }], repo);
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
  private async openPullRequest(branch: string): Promise<void> {
    const repo = this.repo;
    if (!this.hosting || !repo) return;
    const known = this.pulls[branch]?.url;
    const url = known ?? (await reads.webLink(repo.root, { kind: 'create', branch }).catch((error) => void this.log.warn(`No address: ${errorText(error)}`)));
    if (url) await vscode.env.openExternal(vscode.Uri.parse(url));
  }

  /** Checks out the pull request a remote branch has on its label, by its number (`hosting.checkout-pull-request`). */
  private async checkOutPullRequest(number: number, remoteBranch: string): Promise<void> {
    const repo = this.repo;
    if (!repo || !this.hosting?.canCheckOut) return;
    const answers = { number: { kind: 'text' as const, text: String(number) } };
    await this.runOnEngine([{ feature: 'hosting.checkout-pull-request', target: { kind: 'remoteBranch', name: remoteBranch }, answers }], repo);
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

  async sync(action: SyncAction): Promise<void> {
    const state = await this.requireState();
    if (!state) return;
    // Fetch, pull, push and Sync are the engine's.
    if (this.repo && (await this.runOnEngine(syncRuns(action, this.defaultPull()), this.repo))) await this.afterSync(action, state);
  }

  /** What is said after a pull or a push, on either path, from the state before it and the state read after. */
  private async afterSync(action: SyncAction, before: RepoState): Promise<void> {
    // The changes that were set aside no longer fit on what came in: git leaves them in conflict in the files, and
    // keeps the stash it made, which nothing says unless its output is read.
    const kept = this.state?.stashes[0];
    if (kept && this.state!.stashes.length > before.stashes.length && /autostash/i.test(kept.message)) {
      void vscode.window
        .showWarningMessage(
          `Gitardo: pulled, but your changes do not fit on what came in. They are in conflict in the files, and whole in the stash "${kept.message}": resolve the conflicts, then drop the stash.`,
          'Show Stashes',
        )
        .then((choice) => choice && vscode.commands.executeCommand('gitardo.openStashes'));
    }
    // A fetch that found the upstream of a branch force pushed says so, with what it changed (M36).
    const forced = this.state && forcePushed(before, this.state);
    if (forced && this.repo) {
      const repo = this.repo;
      void vscode.window.showInformationMessage(`Gitardo: ${forced} was force pushed.`, SEE_WHAT_CHANGED).then((choice) => {
        if (choice) void this.runOnEngine([{ feature: COMPARE_REWRITE, target: { kind: 'remoteBranch', name: forced } }], repo);
      });
    }
    // The first push of a branch is the natural moment to open a pull request for it: it had no upstream, and has one now.
    const branch = checkedOutBranch(before);
    const hadUpstream = before.branches.some((each) => each.isCheckedOut && each.upstream);
    const hasUpstream = this.state?.branches.some((each) => each.isCheckedOut && each.name === branch && each.upstream);
    const hosting = this.hosting;
    if ((action.startsWith('push') || action === 'sync') && branch && !hadUpstream && hasUpstream && hosting && !this.pulls[branch]) {
      const button = hosting.service === 'gitLab' ? 'Create Merge Request' : 'Create Pull Request';
      const choice = await vscode.window.showInformationMessage(`Pushed ${branch}.`, button);
      if (choice) await this.openPullRequest(branch);
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

  async action(action: Action): Promise<void> {
    const state = await this.requireState();
    const repo = this.repo;
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
    if ((await this.runOnEngine(runs, repo)) && action.kind === 'bisect') this.reportBisect();
  }

  /**
   * Runs a feature of the engine in a repository, its questions asked with VS Code's input boxes and picks, its log in
   * the output channel, and the follow-ups it leaves carried out here; false when it did not run, for want of an engine
   * or because another operation runs, which is said.
   */
  private async runOnEngine(runs: EngineRun[], repo: Repository, hooks: HostHooks = {}, ran?: () => void): Promise<boolean> {
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
      await this.refresh();
      // The engine runs one operation in a repository at a time, whoever asked for it.
      if (busy) {
        void vscode.window.showErrorMessage(runningElsewhere(null, this.repositoryNamed(repo.root)));
        return false;
      }
      // Said once the repository is read again, which tells whether it stopped on conflicts.
      if (failed !== undefined) this.reportEngineFailure(failed, asker.confirmed, repo);
      if (finished?.kind === 'refused') this.offerAfterRefusal(finished.message, finished.offer, target, repo);
      // **Compare First** in the warning of a force push: nothing of the plan ran, and what it offered instead runs now.
      if (asker.instead) return this.runOnEngine([{ feature: asker.instead.feature, target: asker.instead.target, answers: asker.instead.answers }], repo);
      if (finished?.kind !== 'ran') return true;
      this.followUpsAfterRead(followUps, repo);
    }
    ran?.();
    return true;
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
    const agents = this.state?.agentsWindow ?? false;
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
    const operation = this.state?.operation;
    if (failure instanceof EngineError && failure.code === ErrorCode.stepFailed && this.state?.workingTree.conflicted.length) {
      this.reportFailure({ title: title ?? `The ${operation ? operationName(operation) : 'operation'}` }, errorText(failure));
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
    if (this.state && samePath(this.state.root, root)) this.setState({ ...this.state, tagMarks: marks });
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
          void this.refreshPulls(true);
          break;
        // AI and GitHub (stage 4, area 6): the model that resolves conflicts, chosen with `ai.choose-model`.
        case 'writeSetting':
          await this.writeSetting({ kind: 'writeSetting', key: String(followUp.key), value: followUp.value });
          break;
        // The commit box of the Changes view, after a squash merge and a split (area 3).
        case 'preparedMessage': {
          const prepared = (this.repo && (await reads.squashMessage(this.repo).catch(() => null)))?.trim();
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
          if (marks) this.markTags(run.repo.root, followMarks(marks, changed, this.state?.tags ?? []));
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
      if (choice === 'Undo') void this.action({ kind: 'undo' });
    });
  }

  /** Carries out what a plan of the engine leaves for the host once the repository was read again: what to select. */
  private followUpsAfterRead(followUps: FollowUp[], repo: Repository): void {
    for (const followUp of followUps) {
      // The tab of what a rewrite changed (M36), in the repository the operation ran in.
      if (followUp.kind === 'openRangeDiff') void this.rangeDiffTab.open(followUp as RangeDiffTarget, repo);
      if (followUp.kind === 'selectHead' && this.state?.head.commit) this.show(this.state.head.commit);
      if (followUp.kind === 'selectCommit') this.show(String(followUp.commit));
      if (followUp.kind === 'selectBeforeHead' && this.state?.head.commit && this.repo) {
        const count = Number(followUp.count);
        void reads.resolve(this.repo, `HEAD~${count}`).then((hash) => hash && this.show(hash));
      }
      if (followUp.kind === 'openCommitPage') void this.openCommit(String(followUp.commit));
      // Conflicts (stage 4, area 4): **Review** of resolving automatically.
      if (followUp.kind === 'openReview') void this.reviewConflicts(Array.isArray(followUp.paths) ? (followUp.paths as string[]) : undefined);
    }
  }

  /** Says so when the last answer left a single suspect: that is the commit the bisect was looking for. */
  private reportBisect(): void {
    const bisect = this.state?.bisect;
    const bad = bisect?.bad;
    if (!bad || bisect.remaining !== 1) return;
    const subject = this.state?.rows.find((row) => row.hash === bad)?.subject;
    void vscode.window
      .showInformationMessage(`Bisect: the first bad commit is ${bad.slice(0, 7)}${subject ? ` "${subject}"` : ''}.`, 'Show in the graph', 'End bisect')
      .then((choice) => {
        if (choice === 'Show in the graph') void this.reveal(bad);
        if (choice === 'End bisect') void this.action({ kind: 'bisectReset' });
      });
  }

  /** The state of the repository shown; without one, the user is told why: no engine, or no repository. */
  private async requireState(): Promise<RepoState | null> {
    await this.whenLocated();
    if (!reads.available) sayEngineMissing();
    else if (!this.state) void vscode.window.showWarningMessage('Gitardo: no Git repository in this workspace.');
    return this.state;
  }

  private setState(state: RepoState | null): void {
    this.state = state;
    this.showHiddenInMenus(state);
    this.post({ type: 'state', state });
    this.repositoriesView.shown(state);
    this.stateEmitter.fire(state);
    void this.checkUnsaved();
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
    // A repository that is no longer in the workspace keeps nothing.
    this.kept.forget((root) => !this.repos.some((repo) => samePath(repo.root, root)));
    // The repository chosen last time in this workspace, as long as it is still there.
    const remembered = this.memento.get<string>(REPO_KEY);
    await this.open(roots.find((root) => root === remembered) ?? roots[0]);
  }

  /**
   * Points every view at one repository of the workspace, or at none. With `gitardo.repositories.keepOpen` on, the one
   * left keeps what it showed, and one that kept what it showed is shown as it was (`repo.keep-open`).
   */
  private async open(root: string | undefined): Promise<void> {
    // A repository left reads no more pages: it reads its commits again once it is shown (Q437).
    if (this.repo) this.engineGraph.stop(this.repo.root);
    this.leave();
    this.repo = undefined;
    this.limit = pageSize();
    this.cancelGraphRead();
    this.filter = null;
    this.scope = null;
    this.hosting = null;
    this.pulls = {};
    this.pullsAt = 0;
    if (!root) return this.setState(null);
    const repo: Repository = { root };
    const kept = keepOpen() ? this.kept.take(root) : undefined;
    if (kept) return this.comeBack(repo, kept);
    this.scope = this.memento.get<Record<string, Scope>>(SCOPE_KEY, {})[root] ?? null;
    this.repo = repo;
    // A repository chosen counts every worktree again, as **Refresh** does.
    this.everything = true;
    // The commits of the repository before stay in the views until those of this one come.
    await this.reload('filter', async () => {
      const gitDir = await reads.commonDir(repo);
      // Another may have been chosen meanwhile, which has watchers of its own.
      if (this.repo !== repo) return;
      this.watcher = this.watch(root, gitDir);
      await this.detectHosting(repo);
      await this.refresh();
    });
  }

  /**
   * The repository shown is left. With `gitardo.repositories.keepOpen` on it keeps what it showed, and its watchers,
   * which from now on only mark it out of date (`repo.keep-open`, `repo.watch-hidden`); else it is no longer watched.
   */
  private leave(): void {
    const { repo, state, watcher } = this;
    // A change seen just before is the repository left's, not the next one's.
    const pending = this.timer !== undefined ? (this.cause ?? 'something changed') : undefined;
    clearTimeout(this.timer);
    this.timer = undefined;
    this.cause = undefined;
    this.watcher = undefined;
    if (!repo || !state || !samePath(state.root, repo.root) || !keepOpen()) return watcher?.dispose();
    this.kept.keep(repo.root, {
      state,
      filter: this.filter,
      scope: this.scope,
      limit: this.limit,
      hosting: this.hosting,
      pulls: this.pulls,
      pullsAt: this.pullsAt,
      watcher,
      changed: pending,
    });
  }

  /**
   * Shows at once what a repository showed when it was left, and reads it again behind that when it changed meanwhile,
   * as **Refresh** would, the status bar saying why (`repo.keep-open`, `repo.watch-hidden`).
   */
  private async comeBack(repo: Repository, kept: KeptRepository): Promise<void> {
    this.repo = repo;
    this.filter = kept.filter;
    this.scope = kept.scope;
    this.limit = kept.limit;
    this.hosting = kept.hosting;
    this.pulls = kept.pulls;
    this.pullsAt = kept.pullsAt;
    this.watcher = kept.watcher;
    // The repositories of the workspace may have changed while it was not shown.
    this.setState({ ...kept.state, repos: this.repos, home: this.homeOf(kept.state.root), keepOpen: true });
    if (kept.changed) {
      this.cause = kept.changed;
      this.everything = true;
      await this.detectHosting(repo).catch((error) => this.log.warn(`Could not read the remotes of ${repo.root}: ${errorText(error)}`));
      await this.refresh();
    } else if (this.views.size) void this.refreshPulls(false);
  }

  /**
   * Asks the engine which hosting service the main remote is on (`hosting.detect-remote`), with the services of one's own
   * of `gitardo.hosting.selfHosted`; the pull requests known are forgotten when it is another.
   */
  private async detectHosting(repo: Repository): Promise<void> {
    const hosting = await reads.hosting(repo.root);
    if (JSON.stringify(hosting) === JSON.stringify(this.hosting)) return;
    this.hosting = hosting;
    this.pulls = {};
    this.pullsAt = 0;
  }

  /** The services of one's own changed: the main remote may be on another, and the views say so. */
  private async hostingChanged(): Promise<void> {
    const repo = this.repo;
    if (!repo || !reads.available) return;
    try {
      await this.detectHosting(repo);
    } catch (error) {
      return this.log.warn(`Could not read the remotes of ${repo.root}: ${errorText(error)}`);
    }
    if (this.state && this.repo === repo) this.setState({ ...this.state, hosting: this.hosting, pulls: this.pulls });
    void this.refreshPulls(true);
  }

  /** Lists the remotes, to fetch, rename, repoint or remove one, or to add another. */
  private async manageRemotes(add = false): Promise<void> {
    const state = await this.requireState();
    const repo = this.repo;
    if (!state || !repo) return;
    // The remote that pull requests are read from may be another one after.
    if (!(await this.runOnEngine([{ feature: add ? 'remote.add' : 'remote.manage', target: { kind: 'head' } }], repo))) return;
    await this.detectHosting(repo);
    await this.refresh();
  }

  /**
   * Switches to another repository of the workspace, and remembers the choice for this workspace; or to another worktree
   * of the repository shown, for as long as the window lasts: its changes, commits and branches are worked on from here.
   */
  async selectRepo(root: string): Promise<void> {
    const shown = this.repo?.root;
    if (this.running || (shown && samePath(root, shown))) return;
    const repo = this.repos.find((each) => samePath(each.root, root));
    if (repo) {
      await this.memento.update(REPO_KEY, repo.root);
      await this.open(repo.root);
      this.chosenEmitter.fire(repo.root);
      return;
    }
    const state = this.state;
    // A worktree of it, or one of its submodules initialized since the repositories of the workspace were looked for.
    const folder = !state
      ? undefined
      : (state.worktrees.find((each) => samePath(each.folder, root) && !each.isMissing)?.folder ??
        state.submodules.filter((each) => each.checkout?.commit).map((each) => submoduleFolder(each, state)).find((each) => samePath(each, root)));
    if (folder) {
      // The way back is the repository of the workspace that was shown, or the one that was before another worktree.
      this.home = this.repos.find((each) => shown && samePath(each.root, shown)) ?? this.home;
      await this.open(folder);
      this.chosenEmitter.fire(folder);
      return;
    }
    // A worktree of another repository of the workspace, such as one chosen in VS Code's Repositories view.
    const owner = await this.worktreeOwner(root);
    if (!owner || this.running) return;
    this.home = owner;
    await this.open(root);
    this.chosenEmitter.fire(root);
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

  private async pickRepo(): Promise<void> {
    await this.whenLocated();
    // Showing a worktree, there is always the repository of the workspace to go back to.
    if (this.repos.length < 2 && !this.state?.home) {
      void vscode.window.showInformationMessage(
        this.repos.length ? 'Gitardo: this workspace has a single repository.' : 'Gitardo: no Git repository in this workspace.',
      );
      return;
    }
    const current = this.repo?.root;
    const choice = await vscode.window.showQuickPick(
      [
        ...this.repos.map((repo) => ({ label: repo.name, description: repo.root === current ? 'current' : undefined, detail: repo.root, repo })),
        ...(this.state?.home && current
          ? [{ label: this.state.name, description: 'worktree, current', detail: current, repo: { root: current, name: this.state.name, depth: 0 } }]
          : []),
      ],
      { title: 'Repository shown in Gitardo', matchOnDetail: true },
    );
    if (choice) await this.selectRepo(choice.repo.root);
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
  private async switchIdentity(): Promise<void> {
    const state = await this.requireState();
    const repo = this.repo;
    if (!state || !repo) return;
    // The engine's, with the entry that opens the setting at the end of its pick (B9).
    const editList = { label: 'Edit the list of identities…', description: 'gitardo.identities', run: () => this.openIdentities() };
    await this.runOnEngine([{ feature: 'identity.switch', target: { kind: 'head' } }], repo, { choices: { 'identity-pick': [editList] } });
  }

  /** Asks for the co-authors of the next commit, the authors of the history read as the pick opens, and gives them to the box. */
  private async pickCoAuthors(repo: Repository, chosen: string[], webview: vscode.Webview): Promise<void> {
    const authors = reads.authors(repo);
    const email = this.state?.root === repo.root ? this.state.identity.email : null;
    const picked = await pickCoAuthors(authors, email, Array.isArray(chosen) ? chosen : []);
    if (picked) this.post({ type: 'coAuthors', coAuthors: picked }, webview);
  }

  /** Opens the setting that lists the identities to commit as. */
  private async openIdentities(): Promise<void> {
    await vscode.commands.executeCommand('workbench.action.openSettings', 'gitardo.identities');
  }

  /**
   * Picks up changes made outside Gitardo: the terminal, another tool, a background fetch. A repository kept while
   * another is shown is watched too, and a change there only marks it out of date (`repo.watch-hidden`).
   */
  private watch(root: string, gitDir: string): vscode.Disposable {
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
      this.changedIn(root, `.git/${file} changed`);
    };
    // Files written by something other than this editor, such as a command in the terminal. Git's own files have the watcher above.
    const files = vscode.workspace.createFileSystemWatcher(new vscode.RelativePattern(vscode.Uri.file(root), '**'));
    const fileChanged = (uri: vscode.Uri) => {
      if (/(^|\/)\.git(\/|$)/.test(uri.path)) return;
      this.fileChangedIn(root, relative(root, uri));
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
   * A file of the working tree changed in a repository (`repo.watch-changes`, M15). One in a worktree nested in the
   * repository shown asks for nothing; the others wait to be asked of the engine together, a second after the first of
   * them, which says which git does not ignore: only those ask for a read. A repository kept is only marked out of
   * date (Q461).
   */
  private fileChangedIn(root: string, file: string): void {
    if (!this.repo || !samePath(this.repo.root, root)) return this.kept.changed(root, `${file} changed`);
    if (!this.views.size || this.running) return this.scheduleRefresh(`${file} changed`, WORKTREE_REFRESH_DELAY);
    if (this.inNestedWorktree(root, file)) return;
    this.changedFiles.add(file);
    this.filesTimer ??= setTimeout(() => void this.askChangedFiles(root), WORKTREE_REFRESH_DELAY);
  }

  /**
   * A file was saved in an editor. One of the working tree of the repository shown asks for a read as a file changed on
   * the disk does, once the engine says git does not ignore it, and one of git's own, such as `.git/info/exclude`, at
   * once; one out of it asks for none, since the watchers of a repository kept see it. With no repository shown, it is
   * looked for again.
   */
  private saved(uri: vscode.Uri): void {
    if (uri.scheme !== 'file') return;
    const root = this.repo?.root;
    if (root === undefined) return this.scheduleRefresh(`${vscode.workspace.asRelativePath(uri)} was saved`);
    const relative = path.relative(root, uri.fsPath).replace(/\\/g, '/');
    if (!relative || relative.startsWith('..') || path.isAbsolute(relative)) return;
    // A file that tells git what to ignore may change which unsaved files are changes.
    if (path.posix.basename(relative) === '.gitignore' || relative === '.git/info/exclude') this.ignoredUnsaved.clear();
    if (/(^|\/)\.git(\/|$)/.test(relative)) return this.scheduleRefresh(`${relative} was saved`);
    this.fileChangedIn(root, relative);
  }

  /** Whether a file, from the top of the repository shown, is in the folder of another worktree inside it. */
  private inNestedWorktree(root: string, file: string): boolean {
    const state = this.state;
    if (!state || !samePath(state.root, root)) return false;
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
  private async askChangedFiles(root: string): Promise<void> {
    this.filesTimer = undefined;
    const files = [...this.changedFiles];
    this.changedFiles.clear();
    if (!files.length || !this.repo || !samePath(this.repo.root, root)) return;
    const started = this.readsStarted;
    const toRead = await reads
      .client()
      .changesToRead(root, files)
      .catch((error: unknown) => {
        this.log.warn(`Could not ask which changed files git ignores: ${errorText(error)}`);
        return files;
      });
    if (this.changedFiles.size) this.filesTimer ??= setTimeout(() => void this.askChangedFiles(root), WORKTREE_REFRESH_DELAY);
    if (!toRead.length || started !== this.readsStarted || !this.repo || !samePath(this.repo.root, root)) return;
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
   * The window got the focus: it asks for a read only when a change came while no view could show it, or when the last
   * read of the repository shown ended more than a minute ago, or it was never read (`repo.watch-changes`, Q463).
   */
  private focusGot(): void {
    const root = this.repo?.root;
    const readAt = root === undefined ? undefined : this.readAt.get(root);
    if (this.missed || readAt === undefined || Date.now() - readAt > FOCUS_READ_AFTER) this.scheduleRefresh('VS Code got the focus');
  }

  /** A change seen in a repository: the one shown is read again for it, one kept is only marked out of date. */
  private changedIn(root: string, cause: string, delay?: number): void {
    if (this.repo && samePath(this.repo.root, root)) this.scheduleRefresh(cause, delay);
    else this.kept.changed(root, cause);
  }

  /** Refreshes once, `delay` after the first of a burst of events, however long the burst lasts. `cause` is the last of them. */
  private scheduleRefresh(cause: string, delay = REFRESH_DELAY): void {
    this.heard++;
    // With no view to show it, the change is remembered for when the window gets the focus.
    if (!this.views.size) this.missed = true;
    if (!this.views.size || this.running) return;
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
      const keyOf = (count: number) => keys.get(count) ?? keys.set(count, rowsKey(state, this.filter, count)).get(count)!;
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
   * What a view that opens waits for: nothing when the state held was read after the last change heard of, the read
   * under way when it started after that change, and a read otherwise. Views that open together, as when VS Code
   * starts, make one read at most (D88). A first read still under way is sent to every view when it is done.
   */
  private async readForView(): Promise<void> {
    if (this.locating) return this.locating;
    if (!this.located) return this.locate();
    const fresh = this.state !== null && this.stateHeard === this.heard;
    if (fresh && !this.loading) {
      // Not read at the start of VS Code: GitHub is asked only once a view can show what it says.
      void this.refreshPulls(false);
      return;
    }
    if (this.loading && this.readingHeard === this.heard) return this.loading;
    return this.refresh();
  }

  private async onMessage(webview: vscode.Webview, message: WebviewMessage): Promise<void> {
    // What the tab of a commit asks is about the repository of its commit.
    const repo = this.pinned.get(webview) ?? this.repo;
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
        if (this.views.get(webview) === 'commit') return this.post({ type: 'state', state: this.state }, webview);
        // The tab of a comparison is sent its comparison, and the state for what each side is.
        if (compareTab) {
          this.post({ type: 'state', state: this.state }, webview);
          return this.sendCompare(compareTab, true);
        }
        // The Repositories view is sent its rows as they are, and what Gitardo shows is read as for any view.
        if (this.views.get(webview) === 'repositories') {
          this.repositoriesView.ready(webview);
          return this.readForView();
        }
        // What was read before it opened, as at the start of VS Code, is shown at once, and read again if it is not fresh.
        if (this.located && !this.locating && !this.loading) this.post({ type: 'state', state: this.state }, webview);
        if (this.waiting) this.post({ type: 'loading', what: this.waiting.what }, webview);
        if (this.views.get(webview) === 'changes' && this.unsaved.length) this.post({ type: 'unsaved', paths: this.unsaved }, webview);
        await this.readForView();
        return this.flushPending();
      case 'setFilter':
        return this.setFilter(message.filter);
      case 'setScope':
        return this.setScope(message.scope);
      case 'setListed':
        return this.setListed(message.listed);
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
        return this.setFavorite(message.ref, message.favorite === true);
      case 'goTo':
        return this.goTo(message.revision, webview);
      case 'find':
        return this.find(message.search, webview);
      case 'command':
        return this.command(message.command);
      case 'pullRequest':
        return this.openPullRequest(message.branch);
      case 'checkOutPullRequest':
        return this.checkOutPullRequest(message.number, message.remoteBranch);
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
        return this.reviewConflicts(Array.isArray(message.paths) ? message.paths : undefined, message.ai === true);
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
        return this.chooseConflictModel();
      case 'openConflict':
        return repo && this.openConflict(repo, message.path, message.line, message.editor);
      case 'refresh':
        return this.refreshAll();
      case 'loadMore': {
        // While a read adds its pages, the next ones come without asking (`graph.load-more`); and a list that asked while
        // the last page came has nothing left to ask for once the read ended at the end of the history.
        if (!this.repo || this.engineGraph.isReading(this.repo.root) || this.state?.hasMore === false) return;
        this.cancelGraphRead();
        const listed = this.state?.rows.filter((row) => row.hash !== WORKTREE && row.stash === undefined).length ?? 0;
        this.limit = Math.max(this.limit, listed) + pageSize();
        return this.reload('more');
      }
      case 'getLog': {
        const limit = Math.min(Math.max(Math.trunc(Number(message.limit)) || 1, 1), LOG_LIMIT);
        const log = this.repo && (await reads.branchLog(this.repo, limit).catch(() => null));
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
        // In the repository the commit is of, when the tab that asks is of another than the one shown.
        if (repo && repo.root !== this.repo?.root) await this.selectRepo(repo.root);
        return this.reveal(message.hash);
      case 'setFilesLayout':
        return void this.memento.update(FILES_KEY, message.layout === 'list' ? 'list' : 'tree');
      case 'compare':
        return this.compare(message.from);
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
        return this.show(message.hash);
      case 'trace':
        return this.trace(message.hash);
      case 'getComparison': {
        const comparison = this.repo && (await reads.compare(this.repo, message.from, message.to).catch(() => null));
        return comparison ? this.post({ type: 'comparison', comparison }, webview) : undefined;
      }
      case 'selectRepo':
        return this.selectRepo(message.root);
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
        return this.sync(message.action);
      case 'action':
        return this.action(message.action);
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
  private reportFailure(op: { title: string }, error: string): void {
    const showLog = (choice?: string) => choice && this.log.show();
    const conflicts = this.state?.workingTree.conflicted.length ?? 0;
    if (this.state && conflicts) {
      const { operation } = this.state;
      const files = conflicts === 1 ? '1 file has conflicts' : `${conflicts} files have conflicts`;
      // Without an operation to continue, as after a stash that did not apply cleanly, staging the files is all there is to do.
      const then = operation ? `continue or abort the ${operationName(operation)}` : 'stage the files';
      const marked = this.state.conflictFiles.reduce((sum, file) => sum + file.conflicts, 0);
      const resolvable = this.state.conflictFiles.reduce((sum, file) => sum + file.resolvable.length, 0);
      // The ones that need no choice between the two sides are a click away.
      const which = marked === 1 ? 'the conflict' : resolvable === marked ? `all ${marked} conflicts` : `${resolvable} of the ${marked} conflicts`;
      const offer = resolvable ? ` Gitardo can resolve ${which} automatically.` : '';
      // The language model is offered for what the rules leave.
      const ai = this.resolveWithAI() && (!marked || resolvable < marked);
      const items = [...(resolvable ? ['Resolve Automatically'] : []), ...(ai ? ['Resolve with AI'] : []), 'Show Log'];
      void vscode.window.showWarningMessage(`${op.title} stopped: ${files}. Resolve them, then ${then}.${offer}`, ...items).then((choice) => {
        if (choice === 'Resolve Automatically') void this.action({ kind: 'resolveConflicts' });
        if (choice === 'Resolve with AI') void this.reviewConflicts(undefined, true);
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
    const head = this.state?.head.commit ?? '';
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
        const working = repo === this.repo ? this.state?.workingTree : undefined;
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
            await openMergeEditor(repo.root, file.path, this.state?.operation ?? null);
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
    const script = webview.asWebviewUri(vscode.Uri.joinPath(dist, 'webview.js'));
    const style = webview.asWebviewUri(vscode.Uri.joinPath(dist, 'webview.css'));
    // The scripts of the extension itself, besides the page's own: those that color code are imported once it shows some.
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
  <script nonce="${nonce}" src="${script}"></script>
</body>
</html>`;
  }
}

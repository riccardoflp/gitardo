/**
 * What the parts of the `Controller` use of it, each through the members it names: the views and what they are sent,
 * the groups of views and the repository each shows, the engine that runs every operation, and the other parts. The
 * controller gives them one object, which reads its members as they are at each call.
 */
import type * as vscode from 'vscode';
import type { ThemeService } from '../flow/theme';
import type { EngineRun } from '../engine/actions';
import type { Repository } from '../engine/reads';
import type { HostHooks } from '../engine/vscode';
import type { Hosting } from '../shared/hosting';
import type { Action, Loading, RepoInfo, RepoState, ToViews, ViewKind } from '../shared/protocol';
import type { RangeDiffTarget } from '../shared/rangeDiff';
import type { TagMarks } from '../shared/tagMarks';
import type { Activity } from './activity';
import type { Commands } from './commands';
import type { CommitTabs } from './commitTabs';
import type { CompareTabs } from './compareTabs';
import type { ConflictReviewTab } from './conflictReview';
import type { FileDiffs } from './fileDiffs';
import type { FileIcons } from './fileIcons';
import type { GitFoldersHeard } from './gitFoldersHeard';
import type { HistoryShown } from './historyShown';
import type { KeptRepositories } from './keptRepositories';
import type { LargeFilesTabs } from './largeFiles';
import type { RangeDiffTab } from './rangeDiffTab';
import type { RepositoriesShown } from './repositoriesShown';
import type { RepositoriesView } from './repositoriesView';
import type { GroupHost, RepositorySession, ViewGroup } from './repositorySession';
import type { RowsBehind } from './rowsBehind';
import type { Sent } from './sentParts';
import type { Sessions } from './sessions';
import type { UnsavedFiles } from './unsavedFiles';
import type { WorktreeSetupTab } from './worktreeSetupTab';

/** The members of the controller its parts use. */
export interface ControllerHost {
  readonly log: vscode.LogOutputChannel;
  /** The workspace state: which repository was chosen, the branches shown of each, the comparisons opened last. */
  readonly memento: vscode.Memento;
  /** What Gitardo is doing that takes time, in the status bar. */
  readonly activity: Activity;

  // The views.

  /** Every view and tab of Gitardo, by its webview, with its kind. */
  readonly views: ReadonlyMap<vscode.Webview, ViewKind>;
  /** What each view was last sent of the state (D88). */
  readonly sent: WeakMap<vscode.Webview, Sent>;
  /** The views that are still sent the commits of their state, behind its first page (QA6). */
  readonly behind: RowsBehind<vscode.Webview>;
  /** The repository of each tab that shows one of its own: a commit's, a comparison's and the like. */
  readonly pinned: ReadonlyMap<vscode.Webview, Repository>;
  /** The pages that color code, and those that show files with the icons of the file icon theme. */
  readonly colored: Set<vscode.Webview>;
  readonly iconed: Set<vscode.Webview>;
  readonly themes: ThemeService;
  readonly fileIcons: FileIcons;
  /** The tabs of the graph brought back on a folder that is no longer there (`repo.keep-tabs`). */
  readonly goneTabs: ReadonlyMap<vscode.Webview, vscode.WebviewPanel>;
  /** The rows of the Repositories view. */
  readonly repositoriesView: RepositoriesView;
  /** The tab of the review of the conflicts, while it is open. */
  readonly reviewTab: ConflictReviewTab | undefined;
  /** Loads the UI into a webview and wires it to the controller, in a group: the panel's unless another is given. */
  attach(webview: vscode.Webview, kind: ViewKind, data?: Record<string, string>, group?: ViewGroup): vscode.Disposable;
  /** Sends a message to a view, to the views of a group, or to every view. */
  post(message: ToViews, to?: vscode.Webview, group?: ViewGroup): void;
  postToGroup(group: ViewGroup, message: ToViews): void;
  /** Gives a tab the repository it shows, or takes it back with null once the tab is closed. */
  pin(webview: vscode.Webview, repo: Repository | null): void;
  /** Says whether a view that searches has the keyboard. */
  setFocused(webview: vscode.Webview, focused: boolean): void;
  /** Says that a tab of the graph shows a folder that is no longer there, and reads nothing for it. */
  goneTab(panel: vscode.WebviewPanel, root: string): void;

  // The groups of views and the repositories they show.

  /** The views of the panel and of the side bar, whose repository the commands of the Command Palette act on. */
  readonly followers: ViewGroup;
  /** What every group of views asks of the controller. */
  readonly groupHost: GroupHost;
  /** The session of each repository a group of views shows. */
  readonly sessions: Sessions<RepositorySession, ViewGroup>;
  /** The repositories left that keep what they showed. */
  readonly kept: KeptRepositories;
  /** Every repository of the workspace. */
  readonly repos: RepoInfo[];
  /** Whether the repositories of the workspace were looked for, and the search while it runs. */
  readonly located: boolean;
  readonly locating: Promise<void> | undefined;
  /** Whether an operation runs: nothing is read meanwhile. */
  readonly running: boolean;
  /** What the last **Check tags on `<remote>`** of each repository said (`tag.remote-marks`). */
  readonly tagMarks: ReadonlyMap<string, TagMarks>;
  /** The followers and every tab of the graph. */
  allGroups(): ViewGroup[];
  /** The group a view is in, whose repository what it asks acts on. */
  groupOf(webview: vscode.Webview): ViewGroup | undefined;
  /** The group that shows a repository, the panel's first. */
  groupShowing(root: string): ViewGroup;
  /** The groups that show a repository now, each with its state. */
  groupsShowing(root: string): ViewGroup[];
  /** The repository of the workspace a group goes back to, while it shows a worktree of it that is not in the workspace. */
  homeOf(group: ViewGroup, root: string): RepoInfo | null;
  /** The state held of a repository by a group that shows it; null when none does. */
  heldFor(root: string): RepoState | null;
  /** The state of a repository the views may not show, with one commit only. */
  stateOf(repo: Repository): Promise<RepoState>;
  /** The state of the repository a group shows; without one, the user is told why. */
  requireState(on?: ViewGroup): Promise<RepoState | null>;
  /** The state of a group, sent to its views. */
  setState(group: ViewGroup, state: RepoState | null): void;
  /** What the engine says of the git folders it watches, which is told the repositories a view shows (`repo.watch-git`). */
  readonly gitFolders: GitFoldersHeard;
  /** A session for a repository no group shows, which reads it ahead and is let go (`repo.watch-hidden`). */
  makeSession(root: string): RepositorySession;
  /** The state read ahead of a repository of the list, with what the window adds to it, as a group that shows it has it. */
  completeAhead(session: RepositorySession, read: RepoState): Promise<RepoState>;
  /** The hosting service of the main remote of a repository shown. */
  hostingOf(root: string): Hosting | null | undefined;
  /** Keeps the marks of the tags of a repository, and shows them while it is shown. */
  markTags(root: string, marks: TagMarks): void;
  /** Once the repositories of the workspace are known and the one shown is read. */
  whenLocated(): Promise<void>;
  /** Reads again the repository a group shows, the panel's unless another is given. */
  refresh(on?: ViewGroup): Promise<void>;
  /** **Refresh**: reads the repository a group shows again whole. */
  refreshAll(on?: ViewGroup): Promise<void>;
  /** Reads the graph of a group again, or does what ends with that, telling its views what for while they wait. */
  reload(on: ViewGroup, what: Loading, work?: () => Promise<void>): Promise<void>;
  /** Reads again the repository the panel shows, once, `delay` after the first of a burst of events. */
  scheduleRefresh(cause: string, delay?: number): void;
  /** What a view that opens waits for. */
  readForView(on: ViewGroup): Promise<void>;
  /** The panel and the side bar show a repository chosen: VS Code's Repositories view selects it. */
  chosen(root: string): void;

  // The operations.

  /** Runs features of the engine in a repository, as `Controller.runOnEngine` does; false when none ran. */
  run(runs: EngineRun[], repo: Repository, hooks?: HostHooks, ran?: () => void, on?: ViewGroup): Promise<boolean>;
  /** Runs an action of a view on the repository its group shows. */
  action(action: Action, on?: ViewGroup): Promise<void>;
  /** Whether asking a language model how to resolve conflicts is switched on. */
  resolveWithAI(): boolean;

  // What the other parts do, which one part asks of another.

  showRepoOf(uri: vscode.Uri): Promise<void>;
  selectRepo(root: string, on?: ViewGroup): Promise<void>;
  pickRepo(on: ViewGroup): Promise<void>;
  reveal(hash: string, on?: ViewGroup): Promise<void>;
  show(hash: string, on?: ViewGroup): void;
  fileHistory(uri?: vscode.Uri): Promise<void>;
  goToCommit(on?: ViewGroup): Promise<void>;
  openCommit(hash?: unknown, repo?: Repository): Promise<void>;
  openRangeDiff(target: RangeDiffTarget, repo: Repository): Promise<void>;
  openWorktreeSetup(on?: ViewGroup): Promise<void>;
  openLargeFiles(on?: ViewGroup): Promise<void>;
  reviewConflicts(paths?: string[], ai?: boolean, on?: ViewGroup): Promise<void>;
}

/** The parts of the controller, which the messages of the views are handed to. */
export interface ControllerParts {
  readonly commands: Commands;
  readonly commitTabs: CommitTabs;
  readonly compareTabs: CompareTabs;
  readonly diffs: FileDiffs;
  readonly history: HistoryShown;
  readonly largeFiles: LargeFilesTabs;
  readonly rangeDiffTab: RangeDiffTab;
  readonly repositories: RepositoriesShown;
  readonly unsaved: UnsavedFiles;
  readonly worktreeSetup: WorktreeSetupTab;
}

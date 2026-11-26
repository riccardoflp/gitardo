/**
 * The extension's side of `gitardo-engine`: it starts the binary, says `initialize`, opens the repositories, and
 * starts it again when it ends, as `docs/transition/01-engine.md` says: once more after each exit, until a third exit
 * within five minutes stops the restarts until the window is reloaded. It imports nothing of VS Code, so that the
 * tests drive it as the extension does.
 */
import type { LfsFile, LfsLocks, LfsStatus } from '../shared/lfs';
import { type ChildProcess, spawn } from 'node:child_process';
import { Connection, ENDED, EngineError, ErrorCode } from './connection';
import type { JournalEntry } from './journal';
import type { MermaidAnswer, MermaidTheme } from '../shared/mermaid';
import type { RangeDiffPairs, RangeDiffRanges } from '../shared/rangeDiff';
import type { FolderSummary } from '../shared/summaries';
import type { TagComparison } from '../shared/tagMarks';
import type { Author, CommitSignature, FileChange, RepositoryState } from '../shared/state';
import {
  AiMethod,
  type AskedConflict,
  type ConflictAnswerResult,
  type DiscoveredRepository,
  type DraftResult,
  type LanguageModelInfo,
  type HelpAnswerResult,
  type HelpAskParams,
  type LanguageModelRequest,
  type LanguageModelResult,
  type Answer,
  type AskParams,
  type BlameLine,
  type BranchLog,
  type Bytes,
  type CommitAsked,
  type CommitPage,
  type EngineRow,
  type CommitDetail,
  type CommittedLines,
  type CompareRequest,
  type CompareView,
  type ConfirmParams,
  type ConflictFile,
  type ConflictProposals,
  type FeatureRunParams,
  type Finished,
  type FollowUp,
  type FollowUpsParams,
  GRAPH_ROWS,
  type GraphRowsParams,
  HostMethod,
  type IgnoredEntry,
  type InitializeParams,
  type InitializeResult,
  type LfsContent,
  type LineRange,
  type LogLineParams,
  type Matches,
  type MergeSides,
  type OpenParams,
  type OpenResult,
  type ReadFirstParams,
  type GitFolderChangedParams,
  GitFoldersMethod,
  HelpMethod,
  MermaidMethod,
  RangeDiffMethod,
  Method,
  PROTOCOL,
  type Patch,
  type PathStatus,
  type Plan,
  type PullRequests,
  type Question,
  ReadMethod,
  type ReflogPosition,
  type SearchMode,
  type HostingFound,
  type HostingPage,
  type SetupPreview,
  type Settings,
  type SetupGiven,
  type ShownResult,
  type Side,
  type StateParams,
  type StateReason,
  type Submodule,
  SummariesMethod,
  type SummariesResult,
  type TakeRequest,
  type Target,
  type Timeline,
  type Validation,
  type Version,
} from './messages';

/** How long the exits are counted for. */
export const RESTART_WINDOW_MS = 5 * 60 * 1000;
/** The exit within that time that stops the restarts. */
export const EXITS_THAT_STOP = 3;

/** Where the client says what happens: the **Gitardo** output channel in the extension. */
export interface EngineLog {
  info(line: string): void;
  warn(line: string): void;
  error(line: string): void;
  debug(line: string): void;
}

/** What the client needs to start the engine. */
export interface EngineOptions {
  /** The binary to start. */
  binary: string;
  /** The extension's version, said in `initialize`. */
  clientVersion: string;
  /** The settings as they are now, read at each start. */
  settings: () => Settings;
  /** Whether the log says the reads too. */
  logLevel: () => 'info' | 'debug';
  log: EngineLog;
  /** The time now; the tests give their own. */
  now?: () => number;
  /**
   * The token of GitHub that VS Code keeps, for `read/github.read-pull-requests`; null when there is none. Without it the
   * engine is told the host does not answer `host/githubToken`, and reads no pull requests.
   */
  githubToken?: (ask: boolean, host: string) => Promise<string | null>;
  // Area 6 of stage 4: VS Code's language models, for the engine's questions to a model.
  /**
   * Asks a language model of VS Code; `onText` is given the whole answer so far as it comes, when a read of this client
   * asked for it. Without it the engine is told the host does not answer `host/languageModel`.
   */
  languageModel?: (request: LanguageModelRequest, onText: ((text: string) => void) | undefined, signal: AbortSignal) => Promise<LanguageModelResult>;
  /** The language models of VS Code, in its order. */
  languageModels?: () => Promise<LanguageModelInfo[]>;
}

/** Who answers the questions of a feature run: in the extension, VS Code's input boxes, picks and dialogs. */
export interface FeatureAsker {
  /** The answer to a question; null when the user backed out. */
  ask(question: Question): Promise<Answer | null>;
  /** Whether to run a plan. */
  confirm(plan: Plan): Promise<boolean>;
}

/** A feature to run, as `feature/run` takes it. */
export interface FeatureRequest {
  repository: string;
  feature: string;
  target: Target;
  /** Answers given before it asks. */
  answers?: Record<string, Answer>;
  /** The values of the inputs of `branches.format` used last. */
  inputs?: Record<string, string>;
  /** What new worktrees get in the repository's folder, for the features that save it. */
  setup?: SetupGiven;
}

/** How a feature ended, and the follow-ups the engine left the extension to carry out. */
export interface FeatureResult {
  finished: Finished;
  followUps: FollowUp[];
}

/**
 * The state of a repository read in two steps (`repo.read-first`, 1.14): `state` is its first step, and `rest` gives
 * the whole state once the engine has read it behind that answer. `rest` is null when `state` is whole already, as the
 * one of an opening that was under way.
 */
export interface StateInSteps {
  state: RepositoryState;
  rest: Promise<RepositoryState> | null;
}

/** Where the client stands. */
export type EngineStatus = 'stopped' | 'starting' | 'running' | 'restarting' | 'given up' | 'refused';

/** `gitardo-engine` for one window of VS Code. */
export class EngineClient {
  private process?: ChildProcess;
  private connection?: Connection;
  private started?: Promise<InitializeResult>;
  private readonly opened = new Set<string>();
  private readonly openings = new Map<string, Promise<StateInSteps>>();
  /** Who waits for the rest of a state whose first step was asked, by repository. */
  private readonly rests = new Map<string, Set<(state: RepositoryState) => void>>();
  private readonly exits: number[] = [];
  private readonly stateHandlers = new Set<(repository: string, state: RepositoryState, reason: StateReason) => void>();
  private readonly summaryHandlers = new Set<(summaries: FolderSummary[]) => void>();
  /** The folders the engine was last asked to follow for the Repositories view, which it is asked again after a restart. */
  private followed: string[] = [];
  private readonly gitFolderHandlers = new Set<(change: GitFolderChangedParams) => void>();
  /** The repositories the engine was last told a view shows, which it is told again after a restart. */
  private shownLast: string[] = [];
  /** The askers of the runs waiting for their answer, by the id of their `feature/run`. */
  private readonly askers = new Map<number, FeatureAsker>();
  /** The follow-ups of the runs, sent before their answers. */
  private readonly followUps = new Map<number, FollowUp[]>();
  /**
   * Where the answer of a language model is shown as it comes, by the id of the read that asked for it (area 6), given
   * the question too, whose `features` an answer of Ask Gitardo names (1.11).
   */
  private readonly modelText = new Map<number, (text: string, request: LanguageModelRequest) => void>();
  /** Who takes the pages of each read of the commits sent as they are read, by the id of its request (1.8). */
  private readonly graphRows = new Map<number, (rows: EngineRow[]) => void>();
  private stopping = false;
  private current: EngineStatus = 'stopped';

  constructor(private readonly options: EngineOptions) {}

  /** Where the client stands. */
  get status(): EngineStatus {
    return this.current;
  }

  /** The process id of the engine that runs, for the tests that end it. */
  get pid(): number | undefined {
    return this.process?.pid;
  }

  /** Starts the engine once, and gives what it answered to `initialize`. */
  start(): Promise<InitializeResult> {
    this.started ??= this.launch();
    return this.started;
  }

  /** Opens a repository, and gives its state; it is opened again after a restart. */
  async open(repository: string, signal?: AbortSignal): Promise<RepositoryState> {
    const connection = await this.ready();
    const { state } = await connection.request<OpenResult>(Method.open, { repository } satisfies OpenParams, signal);
    this.opened.add(repository);
    return state;
  }

  /**
   * The state of a repository in two steps (`repo.read-first`, 1.14): its first step as soon as it is read, and the whole
   * state behind it. A repository not open yet is opened by the same request, whose rest counts every other worktree;
   * one open is read again, counting them all only with `refresh`. Asked while the repository is being opened whole,
   * it gives that state, with no rest.
   */
  firstState(repository: string, signal?: AbortSignal, refresh = false): Promise<StateInSteps> {
    // Said before anything is awaited, so that the reads asked right after this one wait for this opening.
    if (!this.opened.has(repository)) return this.opening(repository, true);
    return this.inSteps(repository, async (connection) => {
      const state = await connection.request<RepositoryState>(ReadMethod.readFirst, { repository, refresh } satisfies ReadFirstParams, signal);
      return { state, first: true };
    });
  }

  /**
   * Asks for the first step of a state with `ask`, and gives it with the whole state the engine sends once it has read
   * the rest. The engine sends nothing when that read fails or a plan runs meanwhile: so the state is also asked for as
   * soon as the first step is answered, which waits for the rest under way and starts no git, and reads the state only
   * when there is no rest to wait for.
   */
  private async inSteps(repository: string, ask: (connection: Connection) => Promise<OpenResult>): Promise<StateInSteps> {
    const connection = await this.ready();
    let heard: RepositoryState | undefined;
    let tell: (state: RepositoryState) => void = (state) => {
      heard = state;
    };
    const listener = (state: RepositoryState) => tell(state);
    const listeners = this.rests.get(repository) ?? new Set();
    this.rests.set(repository, listeners.add(listener));
    const forget = () => {
      listeners.delete(listener);
      if (!listeners.size && this.rests.get(repository) === listeners) this.rests.delete(repository);
    };
    try {
      const { state, first } = await ask(connection);
      if (!first) {
        forget();
        return { state, rest: null };
      }
      const rest = heard
        ? Promise.resolve(heard)
        : new Promise<RepositoryState>((resolve, reject) => {
            tell = resolve;
            connection.request<RepositoryState>(Method.readState, { repository, refresh: false }).then(resolve, reject);
          });
      // Whoever shows the first step looks at the rest later: a failure meanwhile is theirs to see then.
      rest.then(forget, forget);
      return { state, rest };
    } catch (error) {
      forget();
      throw error;
    }
  }

  /**
   * The repositories the folders of a workspace give access to, in their order: the one each folder is in and those
   * directly inside it, each followed by its submodules checked out, with the git folder each shares with its worktrees
   * (`repository/discover`, from 1.7, `repo.read-repositories`). No repository is opened for it.
   */
  async discover(folders: string[], signal?: AbortSignal): Promise<DiscoveredRepository[]> {
    const connection = await this.ready();
    const { repositories } = await connection.request<{ repositories: DiscoveredRepository[] }>(Method.discover, { folders }, signal);
    return repositories;
  }

  /** Closes a repository opened. */
  async close(repository: string): Promise<void> {
    this.opened.delete(repository);
    const connection = await this.ready();
    await connection.request(Method.close, { repository });
  }

  /**
   * Reads the state of a repository opened. A `refresh` (**Refresh**, or a repository chosen) counts what is not
   * committed in every other worktree again; any other read keeps the count of a worktree whose git folder did not
   * change since (1.8).
   */
  async readState(repository: string, signal?: AbortSignal, refresh = false): Promise<RepositoryState> {
    const connection = await this.ready();
    return connection.request<RepositoryState>(Method.readState, { repository, refresh }, signal);
  }

  /** Opens a repository unless it is open, once for the reads that ask at the same time. */
  async ensureOpen(repository: string): Promise<void> {
    if (this.opened.has(repository)) return;
    await this.opening(repository);
  }

  /**
   * The state of a repository as it is now: the one read as it opens, when it was not open yet (`repo.read-state`).
   * `refresh` counts every other worktree again, as opening it does.
   */
  async currentState(repository: string, signal?: AbortSignal, refresh = false): Promise<RepositoryState> {
    if (this.opened.has(repository)) return this.readState(repository, signal, refresh);
    // An opening in two steps answered its first step: the whole state is the one that follows it.
    const { state, rest } = await this.opening(repository);
    return rest ?? state;
  }

  /**
   * The opening of a repository under way, shared by those who ask while it runs. With `first`, a repository that is not
   * being opened is opened in two steps (`repo.read-first`).
   */
  private opening(repository: string, first = false): Promise<StateInSteps> {
    let opening = this.openings.get(repository);
    if (!opening) {
      const asked = first
        ? this.inSteps(repository, async (connection) => {
            const answer = await connection.request<OpenResult>(Method.open, { repository, first: true } satisfies OpenParams);
            this.opened.add(repository);
            return answer;
          })
        : this.open(repository).then((state) => ({ state, rest: null }));
      opening = asked.finally(() => this.openings.delete(repository));
      this.openings.set(repository, opening);
    }
    return opening;
  }

  /**
   * Runs a feature of the engine in a repository, asking `asker` its questions, and gives how it ended. A failure is an
   * `EngineError`: `stepFailed` with what ran in its data, `busy` while another runs there, `cancelled` once `signal`
   * aborted it.
   */
  async runFeature(request: FeatureRequest, asker: FeatureAsker, signal?: AbortSignal): Promise<FeatureResult> {
    const connection = await this.ready();
    await this.ensureOpen(request.repository);
    const params: FeatureRunParams = {
      repository: request.repository,
      feature: request.feature,
      target: request.target,
      answers: request.answers ?? {},
      journal: true,
      inputs: request.inputs ?? {},
      ...(request.setup ? { setup: request.setup } : {}),
    };
    const { id, answer } = connection.send<Finished>(Method.featureRun, params, signal);
    this.askers.set(id, asker);
    try {
      const finished = await answer;
      return { finished, followUps: this.followUps.get(id) ?? [] };
    } finally {
      this.askers.delete(id);
      this.followUps.delete(id);
    }
  }

  /** The newest entry of the journal of a repository, which **Undo** would take back; null when Undo goes by the reflog. */
  async journalLast(repository: string): Promise<JournalEntry | null> {
    const connection = await this.ready();
    await this.ensureOpen(repository);
    const { entry } = await connection.request<{ entry: JournalEntry | null }>(Method.journalLast, { repository });
    return entry;
  }

  // The reads of version 1.4, from stage 5: one method for each, which opens the repository when it is not open. Each
  // runs in the engine at the same time as the others, and `signal` cancels it.

  /** The commit a name stands for, as a search reads it; null when it names none (`search.read-revision`). */
  async searchRevision(repository: string, revision: string, signal?: AbortSignal): Promise<string | null> {
    return (await this.read<{ commit: string | null }>(ReadMethod.searchRevision, repository, { revision }, signal)).commit;
  }

  /** The details of a commit by its full id, of a stash, or of the uncommitted changes; null for one not there. */
  details(repository: string, asked: CommitAsked, signal?: AbortSignal): Promise<CommitDetail | null> {
    return this.read(ReadMethod.details, repository, asked, signal);
  }

  /** The lines a commit changed in each of its files, or in one of them (`commit-page.read-patch`). */
  commitPatch(repository: string, commit: string, path: string | null, signal?: AbortSignal): Promise<Patch> {
    return this.read(ReadMethod.commitPatch, repository, { commit, path }, signal);
  }

  /** What is not committed (`changes.read-working-tree`). */
  workingTree(repository: string, signal?: AbortSignal): Promise<RepositoryState['workingTree']> {
    return this.read(ReadMethod.workingTree, repository, {}, signal);
  }

  /** Those of some paths that git ignores. */
  async ignored(repository: string, paths: string[], signal?: AbortSignal): Promise<string[]> {
    return (await this.read<{ ignored: string[] }>(ReadMethod.ignored, repository, { paths }, signal)).ignored;
  }

  /** The two sides of a file of a Diff page; null for a side that has no such file (`diff.read-sides`). */
  diffSides(repository: string, left: Version, right: Version, signal?: AbortSignal): Promise<{ left: Bytes | null; right: Bytes | null }> {
    return this.read(ReadMethod.diffSides, repository, { left, right }, signal);
  }

  /** A file as a commit has it; null when the name is no commit or the commit has no such file (`file-view.read-revision`). */
  fileRevision(repository: string, path: string, revision: string, signal?: AbortSignal): Promise<Bytes | null> {
    return this.read(ReadMethod.fileRevision, repository, { path, revision }, signal);
  }

  /** The versions of a file in conflict, and the file as it is now or as its editor has it (`merge-view.read-sides`). */
  mergeSides(repository: string, path: string, unsaved: Bytes | null, signal?: AbortSignal): Promise<MergeSides> {
    return this.read(ReadMethod.mergeSides, repository, { path, unsaved }, signal);
  }

  /** The files in conflict, with the stages the index has of each (`conflicts.read-files`). */
  conflictFiles(repository: string, signal?: AbortSignal): Promise<ConflictFile[]> {
    return this.read(ReadMethod.conflictFiles, repository, {}, signal);
  }

  /** What the two sides of the operation under way are (`conflicts.side-names`). */
  sideNames(repository: string, signal?: AbortSignal): Promise<{ current: Side; incoming: Side }> {
    return this.read(ReadMethod.sideNames, repository, {}, signal);
  }

  /** What the rules propose for the conflicts of the files in conflict, or of some of them (`conflicts.resolve-by-rules`). */
  conflictRules(repository: string, paths: string[] | null, signal?: AbortSignal): Promise<ConflictProposals[]> {
    return this.read(ReadMethod.conflictRules, repository, { paths }, signal);
  }

  /** What two sides of a comparison have that the other has not (`compare.read-sides`). */
  compareSides(repository: string, request: CompareRequest, signal?: AbortSignal): Promise<CompareView> {
    return this.read(ReadMethod.compareSides, repository, request, signal);
  }

  /** The files that differ between two commits (`compare.read-files`). */
  compareFiles(repository: string, from: string, to: string, signal?: AbortSignal): Promise<FileChange[]> {
    return this.read(ReadMethod.compareFiles, repository, { from, to }, signal);
  }

  /** The lines that differ in a comparison, in some of its files, as lines or as text (`compare.read-patch`). */
  comparePatch(
    repository: string,
    request: CompareRequest & { mergeBase: string | null; files: FileChange[]; form: 'lines' | 'text' },
    signal?: AbortSignal,
  ): Promise<Patch> {
    return this.read(ReadMethod.comparePatch, repository, request, signal);
  }

  /** The commit a side of a comparison names; null when it names none (`compare.resolve-revision`). */
  async compareRevision(repository: string, revision: string, signal?: AbortSignal): Promise<string | null> {
    return (await this.read<{ commit: string | null }>(ReadMethod.compareRevision, repository, { revision }, signal)).commit;
  }

  /** Who last changed a line of a file, from 1, with the text of its editor when it has edits not saved (`blame.read-line`). */
  blameLine(repository: string, path: string, line: number, contents: Bytes | null, signal?: AbortSignal): Promise<BlameLine | null> {
    return this.read(ReadMethod.blameLine, repository, { path, line, contents }, signal);
  }

  /** Who last changed each line of a file; null when git cannot blame it (`blame.read-file`). */
  blameFile(repository: string, path: string, revision: string | null, contents: Bytes | null, signal?: AbortSignal): Promise<Array<BlameLine | null> | null> {
    return this.read(ReadMethod.blameFile, repository, { path, revision, contents }, signal);
  }

  /** Every name a file had, the newest first (`file-history.read-names`). */
  async fileNames(repository: string, path: string, signal?: AbortSignal): Promise<string[]> {
    return (await this.read<{ names: string[] }>(ReadMethod.fileNames, repository, { path }, signal)).names;
  }

  /** Where some lines of a file are in the commit checked out (`file-history.read-committed-lines`). */
  committedLines(
    repository: string,
    request: { path: string; renamedFrom: string | null; text: string; lines: LineRange },
    signal?: AbortSignal,
  ): Promise<CommittedLines> {
    return this.read(ReadMethod.committedLines, repository, request, signal);
  }

  /** The places a reference was at, the last first (`reflog.read`). */
  reflog(repository: string, reference: string, count: number, signal?: AbortSignal): Promise<ReflogPosition[]> {
    return this.read(ReadMethod.reflog, repository, { reference, count }, signal);
  }

  /** The remotes, with their URLs (`remote.read-remotes`). */
  remotes(repository: string, signal?: AbortSignal): Promise<Array<{ name: string; url: string }>> {
    return this.read(ReadMethod.remotes, repository, {}, signal);
  }

  /** What git says of the signature of a commit by its full id, checked with the program git's configuration names (`details.read-signature`, 1.11). */
  signature(repository: string, commit: string, signal?: AbortSignal): Promise<CommitSignature> {
    return this.read(ReadMethod.signature, repository, { commit }, signal);
  }

  /**
   * The tags of a remote, asked of it over the network, compared with those of the repository (`tag.read-remote-tags`,
   * `tag.compare-remote`, 1.11).
   */
  remoteTags(repository: string, remote: string, signal?: AbortSignal): Promise<TagComparison> {
    return this.read(ReadMethod.remoteTags, repository, { remote }, signal);
  }

  /** Whether Git LFS is there with its version, whether the repository is set up, and its patterns (`lfs.read-status`, 1.13). */
  lfsStatus(repository: string, signal?: AbortSignal): Promise<LfsStatus> {
    return this.read(ReadMethod.lfsStatus, repository, {}, signal);
  }

  /** The files of the commit checked out stored with Git LFS, with the version `lfsStatus` read (`lfs.read-files`, 1.13). */
  lfsFiles(repository: string, version: string, signal?: AbortSignal): Promise<{ files: LfsFile[] }> {
    return this.read(ReadMethod.lfsFiles, repository, { version }, signal);
  }

  /** The locks of the server, asked over the network (`lfs.read-locks`, 1.13). */
  lfsLocks(repository: string, signal?: AbortSignal): Promise<LfsLocks> {
    return this.read(ReadMethod.lfsLocks, repository, { remote: null }, signal);
  }

  /**
   * A file of a commit, or of the index for a null revision, with the content in place of its pointer when the store
   * of the repository has it (`lfs.read-content`, 1.13).
   */
  lfsContent(repository: string, path: string, revision: string | null, signal?: AbortSignal): Promise<LfsContent> {
    return this.read(ReadMethod.lfsContent, repository, { path, revision }, signal);
  }

  /** The people who made commits in the repository, the one with the most first (`commit.read-authors`, 1.11). */
  authors(repository: string, signal?: AbortSignal): Promise<{ authors: Author[] }> {
    return this.read(ReadMethod.authors, repository, {}, signal);
  }

  /** The identity commits are made with, and the keys of it the repository sets (`identity.read`). */
  identity(repository: string, signal?: AbortSignal): Promise<{ identity: RepositoryState['identity']; localKeys: string[] }> {
    return this.read(ReadMethod.identity, repository, {}, signal);
  }

  /** The submodules, and theirs (`submodule.read-list`). */
  submodules(repository: string, signal?: AbortSignal): Promise<Submodule[]> {
    return this.read(ReadMethod.submodules, repository, {}, signal);
  }

  /** The links to folders that removing a worktree would follow (`worktree.read-folder-links`). */
  async folderLinks(repository: string, worktree: string, signal?: AbortSignal): Promise<string[]> {
    return (await this.read<{ paths: string[] }>(ReadMethod.folderLinks, repository, { worktree }, signal)).paths;
  }

  /** What git ignores in a checkout, with its size on disk (`worktree-setup.read-ignored`). */
  setupIgnored(repository: string, worktree: string, signal?: AbortSignal): Promise<IgnoredEntry[]> {
    return this.read(ReadMethod.setupIgnored, repository, { worktree }, signal);
  }

  /** What link and copy patterns take from each checkout asked (`worktree-setup.read-what-patterns-take`). */
  setupTakes(repository: string, requests: TakeRequest[], signal?: AbortSignal): Promise<SetupPreview[]> {
    return this.read(ReadMethod.setupTakes, repository, { requests }, signal);
  }

  /** The commands proposed for new worktrees, each with the file that calls for it (`worktree-setup.propose-commands`). */
  async setupCommands(repository: string, signal?: AbortSignal): Promise<Array<{ command: string; file: string }>> {
    const { commands } = await this.read<{ commands: Array<{ command: string; file: string }> }>(ReadMethod.setupCommands, repository, {}, signal);
    return commands;
  }

  /** What each path is for git, and whether it is a folder (`worktree-setup.explain-change`). */
  pathStatuses(repository: string, paths: string[], signal?: AbortSignal): Promise<PathStatus[]> {
    return this.read(ReadMethod.pathStatuses, repository, { paths }, signal);
  }

  /**
   * The open pull requests of the repository on GitHub of a remote, or of the main one (`github.read-pull-requests`);
   * on a GitLab, its merge requests (1.11).
   */
  pullRequests(repository: string, remote: string | null, signal?: AbortSignal): Promise<PullRequests> {
    return this.read(ReadMethod.pullRequests, repository, { remote }, signal);
  }

  /** The hosting service and the project of the main remote (`hosting.detect-remote`, 1.11). */
  hosting(repository: string, signal?: AbortSignal): Promise<HostingFound> {
    return this.read(ReadMethod.hosting, repository, {}, signal);
  }

  /** The address of a page of the project of the main remote (`hosting.web-links`, 1.11); null on no service known. */
  webLink(repository: string, page: HostingPage, signal?: AbortSignal): Promise<{ url: string | null }> {
    return this.read(ReadMethod.webLink, repository, { page }, signal);
  }

  /** The commits of what is checked out, at most `count`, and which are not pushed (`branch.read-log`). */
  branchLog(repository: string, count: number, signal?: AbortSignal): Promise<BranchLog> {
    return this.read(ReadMethod.branchLog, repository, { count }, signal);
  }

  /** The git folder the worktrees share, in full (`repo.common-folder`). */
  async commonFolder(repository: string, signal?: AbortSignal): Promise<string> {
    return (await this.read<{ folder: string }>(ReadMethod.commonFolder, repository, {}, signal)).folder;
  }

  /** The message git prepared for the commit of a squash merge; null when none waits (`commit.prepared-message`). */
  async preparedMessage(repository: string, signal?: AbortSignal): Promise<string | null> {
    return (await this.read<{ message: string | null }>(ReadMethod.preparedMessage, repository, {}, signal)).message;
  }

  // The reads of the graph, from stage 7 (version 1.5).

  /** The rows of the pages of the timeline asked for, after the `skip` the client has (`graph.read-commits`). */
  graphCommits(repository: string, timeline: Timeline, signal?: AbortSignal): Promise<CommitPage> {
    return this.read(ReadMethod.graphCommits, repository, timeline, signal);
  }

  /**
   * Reads the commits of the timeline after the `skip` the client has as git prints them, to the end of the history or
   * to 100,000 commits, giving each page to `onRows` as soon as it is read (`graph.read-commits` with `stream`, 1.8).
   * It ends once the read does, saying whether the history has more; cancelled, no page comes after it.
   */
  async graphStream(repository: string, timeline: Timeline, onRows: (rows: EngineRow[]) => void, signal?: AbortSignal): Promise<{ hasMore: boolean }> {
    const connection = await this.ready();
    await this.ensureOpen(repository);
    const { id, answer } = connection.send<CommitPage>(ReadMethod.graphCommits, { ...timeline, repository, stream: true }, signal);
    this.graphRows.set(id, onRows);
    try {
      return { hasMore: (await answer).hasMore };
    } finally {
      this.graphRows.delete(id);
    }
  }

  /**
   * Which files of the working tree a burst changed ask for a read, from the top with `/`: none in a worktree nested in
   * this one, a `.gitignore` always, and of the others those git does not ignore (`repo.watch-changes`, 1.8).
   */
  async changesToRead(repository: string, paths: string[], signal?: AbortSignal): Promise<string[]> {
    return (await this.read<{ paths: string[] }>(ReadMethod.watchChanges, repository, { paths }, signal)).paths;
  }

  /** How many commits come before a commit in the timeline; null when it is not there (`graph.read-position`). */
  async graphPosition(repository: string, commit: string, timeline: Timeline, signal?: AbortSignal): Promise<number | null> {
    return (await this.read<{ index: number | null }>(ReadMethod.graphPosition, repository, { commit, timeline }, signal)).index;
  }

  /** The commits of the history of the timeline that a search finds, newest first (`search.read-matches`). */
  searchMatches(repository: string, text: string, mode: SearchMode, timeline: Timeline, signal?: AbortSignal): Promise<Matches> {
    return this.read(ReadMethod.searchMatches, repository, { text, mode, timeline }, signal);
  }

  /** The pairs of what a rewrite changed, commit by commit, of the lines its follow-up gave (`range-diff.read-pairs`, 1.11). */
  rangeDiffPairs(repository: string, ranges: RangeDiffRanges, signal?: AbortSignal): Promise<RangeDiffPairs> {
    return this.read(RangeDiffMethod.pairs, repository, { ranges }, signal);
  }

  /**
   * A diagram of Mermaid as an SVG in the colors of a kind of VS Code's theme, or why it is not drawn
   * (`file-view.mermaid`, 1.9). It reads no repository, so none is opened for it.
   */
  async drawMermaid(source: string, theme: MermaidTheme, signal?: AbortSignal): Promise<MermaidAnswer> {
    const connection = await this.ready();
    return connection.request<MermaidAnswer>(MermaidMethod.draw, { source, theme }, signal);
  }

  /**
   * Follows these folders in place of those before, the repositories of the workspace, and gives what each of them and
   * each worktree git lists for them is on, once each was read (`repo.worktree-summary`, 1.10). From then on the engine
   * reads one again when `folderChanged` says it changed, and says what it found to the handlers of `onSummaries`. No
   * repository is opened for it.
   */
  async summaries(folders: string[], signal?: AbortSignal): Promise<FolderSummary[]> {
    this.followed = folders;
    const connection = await this.ready();
    const { summaries } = await connection.request<SummariesResult>(SummariesMethod.follow, { folders }, signal);
    return summaries;
  }

  /**
   * Says which paths the watchers saw change in a folder followed, from the top of its working tree, or in its git
   * folder with `.git/` before them. With no path the folder is read again whatever changed.
   */
  folderChanged(folder: string, paths: string[]): void {
    this.connection?.notify(SummariesMethod.folderChanged, { folder, paths });
  }

  /** Calls a handler with what the folders followed are on, each time the engine read some again; disposing it stops that. */
  onSummaries(handler: (summaries: FolderSummary[]) => void): { dispose(): void } {
    this.summaryHandlers.add(handler);
    return { dispose: () => this.summaryHandlers.delete(handler) };
  }

  /**
   * Says which repositories a view shows now, in place of those said before: the engine looks at the git folder of each
   * again every two seconds (`repo.watch-git`, 1.15). It gives those of them, not shown before, whose git folder changed
   * with nothing said of it, which what was kept of them is out of date for. An engine older than 1.15 answers with the
   * error of a method it does not have: it watches nothing, and VS Code's watchers go on alone.
   */
  async shown(repositories: string[], signal?: AbortSignal): Promise<string[]> {
    this.shownLast = repositories;
    const connection = await this.ready();
    const { changed } = await connection.request<ShownResult>(GitFoldersMethod.shown, { repositories }, signal);
    return changed;
  }

  /** Calls a handler with each change the engine itself sees in the git folder of a repository opened; disposing it stops that. */
  onGitFolderChanged(handler: (change: GitFolderChangedParams) => void): { dispose(): void } {
    this.gitFolderHandlers.add(handler);
    return { dispose: () => this.gitFolderHandlers.delete(handler) };
  }

  // Area 6 of stage 4: the reads that ask a language model of VS Code.

  /** The draft of a commit message from the changes, matching the style of these subjects (`commit.draft-message`). */
  draftMessage(repository: string, subjects: string[], signal?: AbortSignal): Promise<DraftResult> {
    return this.read(AiMethod.draftMessage, repository, { subjects }, signal);
  }

  /**
   * The question about one conflict of the review, asked of the model of `gitardo.conflicts.aiModel` (`ai.ask-conflict`);
   * `onText` is given the whole answer so far as it comes.
   */
  async askConflict(
    repository: string,
    question: { path: string; lines: string[]; conflict: AskedConflict },
    onText: (text: string) => void,
    signal?: AbortSignal,
  ): Promise<ConflictAnswerResult> {
    const connection = await this.ready();
    await this.ensureOpen(repository);
    const { id, answer } = connection.send<ConflictAnswerResult>(AiMethod.askConflict, { ...question, repository }, signal);
    this.modelText.set(id, onText);
    try {
      return await answer;
    } finally {
      this.modelText.delete(id);
    }
  }

  /**
   * A question of VS Code's Chat to Ask Gitardo, asked of the model of the Chat (`help.ask`, 1.11): `onText` is given
   * the whole answer so far as the model writes it, with the features it may name, and the answer read comes back. No
   * repository is read, so none is opened for it.
   */
  async askHelp(params: HelpAskParams, onText: (text: string, request: LanguageModelRequest) => void, signal?: AbortSignal): Promise<HelpAnswerResult> {
    const connection = await this.ready();
    const { id, answer } = connection.send<HelpAnswerResult>(HelpMethod.ask, params, signal);
    this.modelText.set(id, onText);
    try {
      return await answer;
    } finally {
      this.modelText.delete(id);
    }
  }

  /** A read of a repository, opened first when it is not. */
  private async read<T>(method: string, repository: string, params: object, signal?: AbortSignal): Promise<T> {
    const connection = await this.ready();
    await this.ensureOpen(repository);
    return connection.request<T>(method, { ...params, repository }, signal);
  }

  /** What is wrong with text typed for a question, as the engine says it; null when it is fine. */
  async validate(repository: string, validation: Validation, text: string): Promise<string | null> {
    const connection = await this.ready();
    const { problem } = await connection.request<{ problem: string | null }>(Method.validate, { repository, validation, text });
    return problem;
  }

  /** Sends the settings as they are now, after one changed. */
  settingsChanged(): void {
    this.connection?.notify(Method.settingsChanged, { settings: this.options.settings() });
  }

  /** Says which paths the watchers saw change in a repository opened. */
  changed(repository: string, paths: string[]): void {
    this.connection?.notify(Method.changed, { repository, paths });
  }

  /** Calls a handler with each state the engine sends after files changed; disposing it stops that. */
  onState(handler: (repository: string, state: RepositoryState, reason: StateReason) => void): {
    dispose(): void;
  } {
    this.stateHandlers.add(handler);
    return { dispose: () => this.stateHandlers.delete(handler) };
  }

  /**
   * Asks the engine to end, and ends it when it does not within a second. It is waited for either way, since it watches
   * the git folders of the repositories it opened, and Windows deletes no folder a process still holds.
   */
  async dispose(): Promise<void> {
    this.stopping = true;
    const child = this.process;
    const connection = this.connection;
    if (!child || !connection) return;
    const ended = new Promise<void>((resolve) => child.once('exit', () => resolve()));
    try {
      await Promise.race([connection.request(Method.shutdown, null), delay(1000)]);
      connection.notify(Method.exit, null);
    } catch {
      // An engine that cannot answer is ended below.
    }
    await Promise.race([ended, delay(1000)]);
    if (child.exitCode === null && child.signalCode === null) {
      child.kill();
      await Promise.race([ended, delay(2000)]);
    }
    this.current = 'stopped';
  }

  private async ready(): Promise<Connection> {
    await this.start();
    if (!this.connection) throw new EngineError(ENDED, "Gitardo's engine is not running.");
    return this.connection;
  }

  /** Starts the process and says `initialize`; a major version that differs ends it there. */
  private async launch(): Promise<InitializeResult> {
    this.current = this.exits.length ? 'restarting' : 'starting';
    const { log } = this.options;
    const child = spawn(this.options.binary, ['--stdio'], {
      stdio: ['pipe', 'pipe', 'pipe'],
      windowsHide: true,
    });
    this.process = child;
    const stdin = child.stdin;
    const stdout = child.stdout;
    if (!stdin || !stdout) throw new EngineError(ENDED, "Gitardo's engine could not be started.");
    const connection = new Connection(stdin, stdout, (error) => log.warn(error.message));
    this.connection = connection;
    child.stderr?.on('data', (chunk: Buffer) => {
      for (const line of chunk.toString('utf8').split(/\r?\n/)) if (line.trim()) log.warn(`[engine] ${line}`);
    });
    connection.on(Method.logLine, (params) => this.logLine(params as LogLineParams));
    connection.on(Method.followUps, (params) => {
      const { run, followUps } = params as FollowUpsParams;
      this.followUps.set(run, followUps);
    });
    connection.onRequest(Method.featureAsk, async (params) => {
      const { run, question } = params as AskParams;
      const asker = this.askers.get(run);
      return { answer: asker ? await asker.ask(question) : null };
    });
    connection.onRequest(Method.featureConfirm, async (params) => {
      const { run, plan } = params as ConfirmParams;
      const asker = this.askers.get(run);
      return { run: asker ? await asker.confirm(plan) : false };
    });
    connection.onRequest(HostMethod.githubToken, async (params) => {
      // Since 1.11 it names the host: github.com, or a GitLab whose token the extension keeps (`hosting.token`).
      const { ask, host } = params as { host: string; ask: boolean };
      return { token: this.options.githubToken ? await this.options.githubToken(ask, host ?? 'github.com') : null };
    });
    // Area 6 of stage 4: the engine's questions to VS Code's language models.
    connection.onRequest(AiMethod.languageModel, async (params, signal) => {
      const request = params as LanguageModelRequest;
      if (!this.options.languageModel) return { refused: 'This window of VS Code asks no language model.' };
      const shown = typeof request.read === 'number' ? this.modelText.get(request.read) : undefined;
      const onText = shown && ((text: string) => shown(text, request));
      return this.options.languageModel(request, onText, signal);
    });
    connection.onRequest(AiMethod.languageModels, async () => ({ models: this.options.languageModels ? await this.options.languageModels() : [] }));
    connection.on(GRAPH_ROWS, (params) => {
      const { read, rows } = params as GraphRowsParams;
      this.graphRows.get(read)?.(rows);
    });
    connection.on(Method.state, (params) => {
      const { repository, state, reason } = params as StateParams;
      // The whole state read behind a first step answered is for who asked that step (`repo.read-first`).
      if (reason === 'rest') for (const tell of [...(this.rests.get(repository) ?? [])]) tell(state);
      for (const handler of this.stateHandlers) handler(repository, state, reason);
    });
    connection.on(SummariesMethod.summaries, (params) => {
      const { summaries } = params as SummariesResult;
      for (const handler of this.summaryHandlers) handler(summaries);
    });
    connection.on(GitFoldersMethod.changed, (params) => {
      for (const handler of this.gitFolderHandlers) handler(params as GitFolderChangedParams);
    });
    const failed = new Promise<never>((_, reject) => {
      child.once('error', (error) => reject(new EngineError(ENDED, `Gitardo's engine could not be started: ${error.message}`)));
    });
    failed.catch(() => {});
    child.once('exit', (code, signal) => this.exited(child, code, signal));

    const params: InitializeParams = {
      protocol: PROTOCOL,
      client: { name: 'gitardo-vscode', version: this.options.clientVersion },
      settings: this.options.settings(),
      host: { terminal: true, githubToken: this.options.githubToken !== undefined, languageModel: this.options.languageModel !== undefined },
      logLevel: this.options.logLevel(),
    };
    try {
      const answer = await Promise.race([connection.request<InitializeResult>(Method.initialize, params), failed]);
      this.current = 'running';
      log.info(`Gitardo's engine ${answer.engine.version} started (protocol ${answer.protocol}, ${answer.engine.git ?? 'git not found'}).`);
      return answer;
    } catch (error) {
      if (error instanceof EngineError && error.code === ErrorCode.invalidParams && error.data) {
        const engine = error.data as InitializeResult;
        this.current = 'refused';
        this.stopping = true;
        log.error(
          `Gitardo's engine ${engine.engine.version} speaks version ${engine.protocol} of the protocol, and this extension ${PROTOCOL}: it is not used.`,
        );
        child.kill();
      }
      throw error;
    }
  }

  /** Counts an exit, and starts the engine again unless it was asked to end or ended three times in five minutes. */
  private exited(child: ChildProcess, code: number | null, signal: NodeJS.Signals | null): void {
    if (this.process !== child) return;
    this.connection?.end();
    this.connection = undefined;
    this.process = undefined;
    if (this.stopping) return;
    const now = (this.options.now ?? Date.now)();
    this.exits.push(now);
    while (this.exits.length && now - this.exits[0] > RESTART_WINDOW_MS) this.exits.shift();
    const how = signal ? `was ended by ${signal}` : `exited with code ${code}`;
    if (this.exits.length >= EXITS_THAT_STOP) {
      this.current = 'given up';
      this.started = Promise.reject(new EngineError(ENDED, "Gitardo's engine stopped too often and is not started again."));
      this.started.catch(() => {});
      this.options.log.error(
        `Gitardo's engine ${how}, the ${ordinal(this.exits.length)} time in five minutes: it is not started again until the window is reloaded.`,
      );
      return;
    }
    this.options.log.warn(`Gitardo's engine ${how}; starting it again.`);
    this.started = this.launch();
    void this.started.then(
      () => this.reopen(),
      (error: unknown) => this.options.log.error(error instanceof Error ? error.message : String(error)),
    );
  }

  /** Opens again the repositories that were open before a restart, and follows again the folders that were followed. */
  private async reopen(): Promise<void> {
    if (this.followed.length) {
      void this.summaries(this.followed).then(
        (summaries) => {
          for (const handler of this.summaryHandlers) handler(summaries);
        },
        (error: unknown) => this.options.log.warn(`Gitardo's engine could not follow the repositories again: ${error instanceof Error ? error.message : String(error)}`),
      );
    }
    for (const repository of [...this.opened]) {
      try {
        await this.open(repository);
      } catch (error) {
        // An engine that ended again says so itself, as its exit.
        if (error instanceof EngineError && error.code === ENDED) return;
        this.options.log.warn(`Gitardo's engine could not open ${repository} again: ${error instanceof Error ? error.message : String(error)}`);
      }
    }
    // The engine started again has nothing from before to compare with: it only looks again from now on.
    if (this.shownLast.length) await this.shown(this.shownLast).catch(() => []);
  }

  private logLine({ repository, entry }: LogLineParams): void {
    const where = repository ? ` (${repository})` : '';
    if (typeof entry.kind === 'object') this.options.log.debug(`[engine] ${entry.line}${where}, ${entry.kind.milliseconds} ms`);
    else if (entry.kind === 'failure') this.options.log.error(`[engine] ${entry.line}${where}`);
    else if (entry.kind === 'warning') this.options.log.warn(`[engine] ${entry.line}${where}`);
    // A step is logged as a command, `> ` before it, as the log has always shown them.
    else if (entry.kind === 'step') this.options.log.info(`> ${entry.line}`);
    // Under it, what it was given on its standard input, indented, and what git printed once it succeeded, as they are.
    else if (entry.kind === 'input' || entry.kind === 'output') this.options.log.info(entry.line);
    else this.options.log.info(`[engine] ${entry.line}${where}`);
  }
}

function delay(milliseconds: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, milliseconds));
}

function ordinal(count: number): string {
  return ['first', 'second', 'third'][count - 1] ?? `${count}th`;
}

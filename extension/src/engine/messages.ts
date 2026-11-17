/**
 * The messages of the protocol with `gitardo-engine` that stages 1 to 5 and 7 built, as the crate `protocol` writes them
 * (`rust/crates/protocol/src/messages.rs`, `state.rs`, `feature/`, `journal.rs` and `reads/`,
 * `docs/transition/02-protocol.md`). The state of a repository and the details of a commit are in `src/shared/state.ts`,
 * which the views take as they are (stage 6).
 */

import type { ChangeKind, FileChange, Operation, RepositoryState } from '../shared/state';

export type { CommitDetail, Signature, Submodule } from '../shared/state';

/**
 * The version of the protocol the extension speaks: 1.8 has the pages of the commits sent as they are read, `refresh` in
 * `read/repo.read-state` and `read/repo.watch-changes` (milestone M15); 1.9 has `read/file-view.mermaid`, a diagram of
 * Mermaid as an SVG for VS Code's preview of Markdown (milestone M17).
 */
export const PROTOCOL = '1.9';

/** The names of the methods. */
export const Method = {
  initialize: 'initialize',
  shutdown: 'shutdown',
  exit: 'exit',
  open: 'repository/open',
  close: 'repository/close',
  changed: 'repository/changed',
  state: 'repository/state',
  readState: 'read/repo.read-state',
  logLine: 'log/line',
  activity: 'activity/changed',
  featureRun: 'feature/run',
  featureAsk: 'feature/ask',
  featureConfirm: 'feature/confirm',
  followUps: 'feature/followUps',
  validate: 'question/validate',
  settingsChanged: 'settings/changed',
  journalLast: 'journal/last',
  discover: 'repository/discover',
} as const;

/** What the extension says of itself in `initialize`. */
export interface InitializeParams {
  protocol: string;
  client: { name: string; version: string };
  settings: Settings;
  /** The host questions the extension answers; in stage 1 it answers none. */
  host: Partial<Record<'clipboard' | 'githubToken' | 'languageModel' | 'dialogs' | 'secrets' | 'terminal', boolean>>;
  logLevel: 'info' | 'debug';
}

/** What the engine answers to `initialize`, and the data of the error of a major version that differs. */
export interface InitializeResult {
  protocol: string;
  engine: { version: string; git: string | null };
}

/** The settings, by the desktop's names: the extension's without `gitardo.`. */
export type Settings = Record<string, unknown>;

/** The notification of a state read again after files changed. */
export interface StateParams {
  repository: string;
  state: RepositoryState;
  reason: 'changed' | 'ran' | 'opened';
}

/** A line of the engine's log. */
export interface LogLineParams {
  repository: string | null;
  entry: {
    time: number;
    kind: 'step' | 'failure' | 'warning' | { kind: 'read'; milliseconds: number };
    line: string;
  };
}

/** What a feature is run on (`Target` of `model`): the kinds stage 2 built. */
export type Target =
  | { kind: 'head' }
  | { kind: 'commit'; commit: string }
  | { kind: 'branch'; name: string }
  | { kind: 'remoteBranch'; name: string }
  | { kind: 'tag'; name: string }
  | { kind: 'stash'; index: number }
  | { kind: 'remote'; name: string }
  | { kind: 'files'; paths: string[] }
  | { kind: 'file'; path: string }
  | { kind: 'changes' }
  | { kind: 'commits'; commits: string[] }
  | { kind: 'submodule'; path: string }
  | { kind: 'worktree'; folder: string }
  | { kind: 'path'; path: string }
  // The targets of the conflicts, from 1.3.
  | { kind: 'resolutions'; files: FileResolutions[] }
  | { kind: 'mergeResult'; path: string; unsaved: string | null; marked: number }
  // Area 6 of stage 4, from 1.6: a comparison page as a whole, or one of its files from 1.7.
  | ComparisonTarget
  // From 1.7: a change of a diff, which the Flow Diff stages or unstages.
  | DiffChangeTarget;

/** The resolutions accepted in a file of the review, and the text they were made from (`FileResolutions` of `model`). */
export interface FileResolutions {
  path: string;
  text: string;
  conflicts: number;
  accepted: AcceptedProposal[];
}

/** A proposal accepted: the lines `from` to `to` of the file, from 0 and `to` left out, with `text` in their place. */
export interface AcceptedProposal {
  source:
    | { kind: 'rules'; case: 'spacesOrLineEndings' | 'sameChange' | 'oneSide' | 'nextToEachOther' }
    | { kind: 'ai'; model: string; reason: string; confidence: 'high' | 'medium' | 'low' }
    | { kind: 'choice'; choice: 'currentOnly' | 'incomingOnly' | 'bothCurrentFirst' | 'bothIncomingFirst' | 'written' };
  from: number;
  to: number;
  text: string;
  newLines: number;
}

/** What a person answers to a question. */
export type Answer =
  | { kind: 'text'; text: string }
  | { kind: 'choice'; identifier: string }
  | { kind: 'choices'; identifiers: string[] }
  | { kind: 'confirmed' }
  | { kind: 'path'; path: string }
  /** Only in the answers of an operation offered (`offerRun`), sent back as they came. */
  | { kind: 'flag'; value: boolean };

/** What typed text must be; sent back as it came in `question/validate`. */
export type Validation = { kind: string } & Record<string, unknown>;

/** One choice of a pick. */
export interface Choice {
  identifier: string;
  label: string;
  description: string | null;
}

/** A file or folder to choose. */
export interface FileQuestion {
  folder: string | null;
  fileName: string | null;
  filters: Array<{ name: string; extensions: string[] }>;
}

/** How a question is answered. */
export type QuestionKind =
  | { kind: 'text'; placeholder: string | null; validation: Validation }
  | { kind: 'secret' }
  | { kind: 'pick'; choices: Choice[] }
  | { kind: 'search'; choices: Choice[] }
  | { kind: 'pickMany'; choices: Choice[] }
  | { kind: 'confirm'; accept: string }
  | ({ kind: 'openFile' } & FileQuestion)
  | ({ kind: 'saveFile' } & FileQuestion);

/** A question of a feature. */
export interface Question {
  key: string;
  kind: QuestionKind;
  title: string;
  detail: string | null;
  default: Answer | null;
}

/** One thing a plan does; the extension shows its `line`. */
export type Step = { kind: string; line: string } & Record<string, unknown>;

/** What a feature is about to do. */
export interface Plan {
  title: string;
  steps: Step[];
  danger: { kind: 'none' } | { kind: 'confirm'; reason: string } | { kind: 'destructive'; reason: string };
  undo: { kind: string } & Record<string, unknown>;
  after: FollowUp[];
}

/** What the host does once a plan ran. */
export type FollowUp =
  | { kind: 'say'; text: string }
  | { kind: 'selectHead' }
  | { kind: 'selectCommit'; commit: string }
  | { kind: 'selectBeforeHead'; count: number }
  | { kind: 'rememberInputs'; values: Record<string, string> }
  /** An operation offered with a button, run on its own target with the answers given (1.3). */
  | { kind: 'offerRun'; label: string; feature: string; target: Target; answers: Record<string, Answer> }
  | ({ kind: string } & Record<string, unknown>);

/** An operation a plan offers with a button. */
export type OfferedRun = Extract<FollowUp, { kind: 'offerRun' }>;

/** How a feature ended: the answer of `feature/run`. */
export type Finished =
  | { kind: 'ran'; plan: Plan; steps: string[]; wentOnAfter: Array<{ failed: string; said: string }>; waiting: Step[] }
  | { kind: 'refused'; message: string; offer: { feature: string; label: string } | null }
  | { kind: 'backedOut' }
  | { kind: 'notConfirmed'; plan: Plan };

/** The request to run a feature. */
export interface FeatureRunParams {
  repository: string;
  feature: string;
  target: Target;
  answers: Record<string, Answer>;
  journal: boolean;
  inputs: Record<string, string>;
}

/** The engine's question of a run. */
export interface AskParams {
  run: number;
  question: Question;
}

/** The engine's plan of a run to confirm. */
export interface ConfirmParams {
  run: number;
  plan: Plan;
}

/** The follow-ups of a run that ran, sent right before its answer. */
export interface FollowUpsParams {
  run: number;
  followUps: FollowUp[];
}

// The reads of version 1.4, from stage 5 (`rust/crates/protocol/src/reads/`): every read but the graph's.

/** The names of the reads of version 1.4, each `read/<card>`. */
export const ReadMethod = {
  searchRevision: 'read/search.read-revision',
  details: 'read/details.read-commit',
  commitPatch: 'read/commit-page.read-patch',
  workingTree: 'read/changes.read-working-tree',
  ignored: 'read/changes.read-ignored',
  diffSides: 'read/diff.read-sides',
  fileRevision: 'read/file-view.read-revision',
  mergeSides: 'read/merge-view.read-sides',
  conflictFiles: 'read/conflicts.read-files',
  sideNames: 'read/conflicts.side-names',
  conflictRules: 'read/conflicts.resolve-by-rules',
  compareSides: 'read/compare.read-sides',
  compareFiles: 'read/compare.read-files',
  comparePatch: 'read/compare.read-patch',
  compareRevision: 'read/compare.resolve-revision',
  blameLine: 'read/blame.read-line',
  blameFile: 'read/blame.read-file',
  fileNames: 'read/file-history.read-names',
  committedLines: 'read/file-history.read-committed-lines',
  reflog: 'read/reflog.read',
  remotes: 'read/remote.read-remotes',
  identity: 'read/identity.read',
  submodules: 'read/submodule.read-list',
  folderLinks: 'read/worktree.read-folder-links',
  setupIgnored: 'read/worktree-setup.read-ignored',
  setupTakes: 'read/worktree-setup.read-what-patterns-take',
  setupCommands: 'read/worktree-setup.propose-commands',
  pathStatuses: 'read/worktree-setup.explain-change',
  pullRequests: 'read/github.read-pull-requests',
  branchLog: 'read/branch.read-log',
  commonFolder: 'read/repo.common-folder',
  preparedMessage: 'read/commit.prepared-message',
  graphCommits: 'read/graph.read-commits',
  graphPosition: 'read/graph.read-position',
  searchMatches: 'read/search.read-matches',
  // Version 1.8, milestone M15.
  watchChanges: 'read/repo.watch-changes',
} as const;

/** The notification of a page of `read/graph.read-commits` asked with `stream`, as it is read (1.8). */
export const GRAPH_ROWS = 'read/graph.rows';

/** A page of a read of the commits sent as it is read: the id of the request, and its rows after those before. */
export interface GraphRowsParams {
  read: number;
  rows: EngineRow[];
}

/** The host question that `read/github.read-pull-requests` asks. */
export const HostMethod = { githubToken: 'host/githubToken' } as const;

/** Bytes, as they travel: base64, padded with `=`. */
export interface Bytes {
  base64: string;
}

/** Which commit `read/details.read-commit` is asked for: one by its full id, a stash by its place, or the changes. */
export type CommitAsked = { commit: string } | { stash: number } | { uncommitted: true };

/** A line of a hunk, numbered on each side it is on. */
export interface DiffLine {
  kind: 'context' | 'added' | 'removed' | 'noNewline';
  text: string;
  before: number | null;
  after: number | null;
}

/** The lines a file changed. */
export interface FilePatch {
  path: string;
  originalPath: string | null;
  kind: ChangeKind;
  hunks: Array<{ header: string; lines: DiffLine[] }>;
  added: number;
  removed: number;
  isBinary: boolean;
  isPastLimit: boolean;
}

/** The lines that changed, as lines of each file, as text, or not matched to the files asked. */
export type Patch = { kind: 'files'; files: FilePatch[] } | { kind: 'unmatched' } | { kind: 'text'; text: Bytes };

/** A side of a comparison. */
export type ComparisonSide = { kind: 'commit'; commit: string } | { kind: 'workingTree' };

/** What `read/compare.read-sides` and `read/compare.read-patch` compare. */
export interface CompareRequest {
  first: ComparisonSide;
  second: ComparisonSide;
  mode: 'sinceDiverged' | 'everything';
  path: string | null;
}

/** A commit of one side of a comparison. */
export interface ComparedCommit {
  side: 'first' | 'second';
  id: string;
  authorName: string;
  authorDate: number;
  subject: string;
  isAlsoInOtherSide: boolean;
}

/** What two sides of a comparison have that the other has not (`CompareView`). */
export interface CompareView {
  mergeBase: string | null;
  onlyInFirst: number;
  onlyInSecond: number;
  files: FileChange[];
  commits: ComparedCommit[];
  merge: { kind: 'notAsked' } | { kind: 'clean' } | { kind: 'conflicts'; files: string[] } | { kind: 'unknown' };
}

/** A version of a file, for a side of a Diff page. */
export interface Version {
  version: { kind: 'absent' } | { kind: 'index' } | { kind: 'commit'; commit: string } | { kind: 'base' } | { kind: 'workingTree'; unsaved: Bytes | null };
  path: string;
}

/** Who last changed a line; `notCommitted` for a line not committed yet. */
export type BlameLine =
  | { kind: 'notCommitted' }
  | { kind: 'committed'; commit: string; author: string; authorTime: number; summary: string; previous: { commit: string; path: string } | null };

/** Lines of a file, from 1, the end in. */
export interface LineRange {
  start: number;
  end: number;
}

/** Where some lines of a file are in the commit checked out. */
export type CommittedLines = { kind: 'inNoCommit' } | { kind: 'committed'; commit: string; path: string; lines: LineRange | null };

/** The versions of a file in conflict, each empty when its stage has none, and the file as it is now. */
export interface MergeSides {
  base: Bytes;
  current: Bytes;
  incoming: Bytes;
  result: Bytes;
  operation: Operation | null;
  hasBothSides: boolean;
}

/** A version the index keeps of a file in conflict. */
export interface StageEntry {
  mode: string;
  object: string;
}

/** A file in conflict, with the stages the index has of it. */
export interface ConflictFile {
  path: string;
  base: StageEntry | null;
  current: StageEntry | null;
  incoming: StageEntry | null;
}

/** What a side of a conflict is. */
export interface Side {
  role:
    | 'branchCheckedOut'
    | 'branchBeingMerged'
    | 'rebasedOnto'
    | 'commitBeingReplayed'
    | 'commitBeingPicked'
    | 'stateBeforeReverted'
    | 'patchBeingApplied'
    | 'yourFiles'
    | 'changesBeingApplied';
  name: string | null;
  subject: string | null;
}

/** Lines of a file, from 0, the end left out. */
export interface Span {
  start: number;
  end: number;
}

/** What a rule proposes for a conflict: lines in place of some lines of the file. */
export interface Proposal {
  case: 'spacesOrLineEndings' | 'sameChange' | 'oneSide' | 'nextToEachOther';
  replaced: Span;
  lines: Array<{ text: string; ending: string }>;
}

/** What the rules make of the conflicts of one file. */
export interface ConflictProposals {
  path: string;
  leftAlone: 'tooLarge' | 'noMarkers' | 'binary' | 'notUtf8' | 'tooManyLines' | 'markersDoNotPair' | 'tooComplex' | null;
  conflicts: Array<{ line: number; span: Span; proposal: Proposal | null }>;
}

/** A place a reference was at. */
export interface ReflogPosition {
  commit: string;
  subject: string;
  selector: string;
  time: number;
}

/** Something git ignores in a checkout, with its size on disk. */
export interface IgnoredEntry {
  path: string;
  kind: { kind: 'file'; bytes: number } | { kind: 'folder'; files: number; bytes: number; hasMore: boolean } | { kind: 'link' };
}

/** What link and copy patterns are asked of a checkout. */
export interface TakeRequest {
  folder: string;
  link: string[];
  copy: string[];
  commit: string | null;
}

/** How many paths a pattern takes, with the first of them. */
export interface PatternTake {
  pattern: string;
  count: number;
  paths: string[];
}

/** What the link and copy patterns take from a checkout. */
export interface SetupPreview {
  linked: string[];
  copied: string[];
  linkPatterns: PatternTake[];
  copyPatterns: PatternTake[];
  already: string[];
}

/** What a path is for git, and whether it is a folder on disk. */
export interface PathStatus {
  isFolder: boolean;
  tracked: 'nothing' | 'it' | 'filesIn';
  ignored: 'whole' | 'part' | 'nothing';
}

/** An open pull request. */
export interface PullRequest {
  number: number;
  title: string;
  url: string;
  isDraft: boolean;
  branch: string;
  checks: 'passed' | 'failed' | 'running' | null;
}

/** The open pull requests by the name of their branch, or why they were not read. */
export type PullRequests =
  | { kind: 'read'; byBranch: Record<string, PullRequest> }
  | { kind: 'notOnGitHub' }
  | { kind: 'noToken' }
  | { kind: 'failed'; message: string; isUnauthorized: boolean };

/** A commit as the log of what is checked out lists it. */
export interface EngineCommit {
  id: string;
  parents: string[];
  authorName: string;
  authorEmail: string;
  authorDate: number;
  subject: string;
}

/** The commits of what is checked out, whether there are more, and which are not pushed against which upstream. */
export interface BranchLog {
  commits: EngineCommit[];
  hasMore: boolean;
  unpushed: string[] | null;
  upstream: string | null;
}

// The reads of the graph, from stage 7 (version 1.5): `read/graph.read-commits`, `read/graph.read-position` and
// `read/search.read-matches`, each asked with the timeline the graph shows.

/** The branches the graph shows; null in a timeline is every branch. */
export type BranchesShown =
  | { kind: 'current' }
  | { kind: 'upstream' }
  | { kind: 'kinds'; local: boolean; remote: boolean; tags: boolean }
  | { kind: 'references'; names: string[] };

/** How a search of the whole history looks at the commits. */
export type SearchMode = 'message' | 'author' | 'changes' | 'changesPattern';

/** What narrows the history the graph lists: the history of a file, of some lines or of a folder, or a search shown as the list. */
export type Narrowing =
  | { kind: 'file'; path: string; earlier: string[] }
  | { kind: 'lines'; path: string; lines: LineRange; commit: string; inEditor: LineRange | null }
  | { kind: 'folder'; path: string }
  | { kind: 'search'; mode: SearchMode; text: string };

/** The timeline a read of the graph is about, and how much of it: `pages` of `pageSize` commits, after the `skip` the client has. */
export interface Timeline {
  shown: BranchesShown | null;
  parents: 'all' | 'first';
  alone: string | null;
  narrowing: Narrowing | null;
  pageSize: number;
  pages: number;
  skip: number;
  stashes: boolean;
}

/** A row of the graph as the engine reads it; it never lists the uncommitted changes itself. */
export type EngineRow =
  | { kind: 'commit'; commit: EngineCommit }
  | { kind: 'stash'; commit: EngineCommit; index: number }
  | { kind: 'uncommitted'; parent: string | null; files: number };

/** The rows of the pages asked, after those the client has, and whether the history has more. */
export interface CommitPage {
  rows: EngineRow[];
  hasMore: boolean;
}

/** The commits a search found, newest first, at most 5,000, and whether it found more. */
export interface Matches {
  commits: string[];
  hasMore: boolean;
}

// Area 6 of stage 4, AI and GitHub, from 1.6 (`rust/crates/protocol/src/ai.rs`): the questions to VS Code's language
// models, the two reads that ask them, the setting a feature leaves for the extension to write, and the comparison a
// feature is run on.

/** The names of the methods of area 6. */
export const AiMethod = {
  languageModel: 'host/languageModel',
  languageModels: 'host/languageModels',
  draftMessage: 'read/commit.draft-message',
  askConflict: 'read/ai.ask-conflict',
} as const;

/** A question of the engine to a language model of VS Code (`host/languageModel`). */
export interface LanguageModelRequest {
  /** What it is for, which says the justification VS Code shows. */
  purpose: 'commitMessage' | 'conflict';
  /** The id of the model; null for the first VS Code has. */
  model: string | null;
  /** How much the model thinks, as `ai.effort` names it; null for as the model is set up. */
  effort: string | null;
  messages: Array<{ role: 'user' | 'assistant'; text: string }>;
  /** The id of the extension's read it is asked for, where the answer is shown as it comes; null when a feature asks. */
  read: number | string | null;
}

/** The answer of `host/languageModel`: the whole text and the name of the model, or why it could not be asked. */
export type LanguageModelResult = { text: string; model: string } | { refused: string };

/** A language model of VS Code, as `host/languageModels` lists it. */
export interface LanguageModelInfo {
  id: string;
  name: string;
}

/** The answer of `read/commit.draft-message`. */
export type DraftResult = { kind: 'drafted'; message: string } | { kind: 'failed'; message: string };

/** A conflict of the review, as `read/ai.ask-conflict` takes it: its lines in the file from 0, `end` left out, and its sides. */
export interface AskedConflict {
  start: number;
  end: number;
  current: string[];
  incoming: string[];
  base: string[] | null;
}

/** The answer of `read/ai.ask-conflict`: the resolution read and trimmed, or why there is none. */
export type ConflictAnswerResult =
  | { kind: 'answered'; model: string; lines: string[]; reason: string; confidence: 'high' | 'medium' | 'low' }
  | { kind: 'failed'; message: string; stopsTheRest: boolean };

/** A side of a comparison page, as a target names it. */
export interface CompareSideTarget {
  name: string;
  side: { kind: 'commit'; commit: string } | { kind: 'workingTree' };
  isFollowed: boolean;
  isGone: boolean;
}

/** A comparison page as a whole (`compare.open-on-github`). */
export interface ComparisonTarget {
  kind: 'comparison';
  first: CompareSideTarget;
  second: CompareSideTarget;
  mode: 'sinceDiverged' | 'everything';
  path: string | null;
  mergeBase: string | null;
  /** The file of the comparison it was invoked on, from 1.7 (`compare.take-file`); the whole comparison without it. */
  file?: FileChange;
}

/** A side of a diff, as a change of it names it. */
export type DiffSideTarget = { kind: 'head' } | { kind: 'index' } | { kind: 'workingTree' } | { kind: 'commit'; commit: string } | { kind: 'empty' };

/** A change of a diff: the lines it replaces on the left, from 0 with the end out, and those it puts there from the right. */
export interface DiffChangeTarget {
  kind: 'diffChange';
  path: string;
  left: DiffSideTarget;
  right: DiffSideTarget;
  leftLines: { start: number; end: number };
  rightLines: { start: number; end: number };
  leftText: string[];
  rightText: string[];
}

/** A repository found in the folders of a workspace (`repository/discover`, from 1.7). */
export interface DiscoveredRepository {
  folder: string;
  /** The repository it is a submodule of, when it is one. */
  superproject: string | null;
  /** The git folder its worktrees share, where the references are; null when git could not say. */
  commonFolder: string | null;
}

/** A setting a feature leaves for the extension to write in the user's settings, by the desktop's name; null removes it. */
export interface WriteSettingFollowUp {
  kind: 'writeSetting';
  key: string;
  value: unknown;
}

// Milestone M17, from 1.9 (`rust/crates/protocol/src/mermaid.rs`): a diagram of Mermaid as an SVG, with no repository.

/** The name of the read of the diagrams of Mermaid. */
export const MermaidMethod = {
  draw: 'read/file-view.mermaid',
} as const;

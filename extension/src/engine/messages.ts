/**
 * The messages of the protocol with `gitardo-engine` that stages 1, 2 and 3 built, as the crate `protocol` writes them
 * (`rust/crates/protocol/src/messages.rs`, `state.rs`, `feature/` and `journal.rs`, `docs/transition/02-protocol.md`).
 */

/** The version of the protocol the extension speaks. */
export const PROTOCOL = '1.3';

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
  journalRecord: 'journal/record',
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

/** The state of a repository, as the engine reads it (`repo.read-state`). */
export interface EngineState {
  folder: string;
  gitFolder: string;
  head: { commit: string | null; branch: string | null };
  branches: Array<{
    name: string;
    commit: string;
    upstream: {
      reference: string;
      remote: string;
      remoteBranch: string;
      ahead: number;
      behind: number;
      isGone: boolean;
    } | null;
    isCheckedOut: boolean;
    date: number;
  }>;
  remoteBranches: Array<{
    name: string;
    remote: string;
    commit: string;
    date: number;
  }>;
  tags: Array<{ name: string; commit: string; date: number }>;
  stashes: Array<{
    commit: string;
    index: number;
    message: string;
    date: number;
  }>;
  worktrees: Array<{
    folder: string;
    commit: string | null;
    branch: string | null;
    isMain: boolean;
    isCurrent: boolean;
    isMissing: boolean;
    uncommitted: number | null;
  }>;
  remotes: string[];
  identity: { name: string | null; email: string | null };
  branchSort: string | null;
  reflogSubjects: string[];
  workingTree: {
    staged: EngineFileChange[];
    unstaged: EngineFileChange[];
    untracked: string[];
    conflicted: Array<{ path: string; kind: string }>;
  };
  operation: string | null;
  rebaseStop: unknown;
  bisect: unknown;
  submodules: unknown[];
}

/** A file changed in the working tree or the index. */
export interface EngineFileChange {
  path: string;
  originalPath: string | null;
  kind: 'added' | 'modified' | 'deleted' | 'renamed' | 'copied' | 'typeChanged';
}

/** The notification of a state read again after files changed. */
export interface StateParams {
  repository: string;
  state: EngineState;
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
  | { kind: 'mergeResult'; path: string; unsaved: string | null; marked: number };

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
  | { kind: 'path'; path: string };

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
  | ({ kind: string } & Record<string, unknown>);

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

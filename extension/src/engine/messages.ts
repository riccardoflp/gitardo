/**
 * The messages of stage 1 of the protocol with `gitardo-engine`, as the crate `protocol` writes them
 * (`rust/crates/protocol/src/messages.rs` and `state.rs`, `docs/transition/02-protocol.md`).
 */

/** The version of the protocol the extension speaks. */
export const PROTOCOL = '1.0';

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
  reason: 'changed';
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

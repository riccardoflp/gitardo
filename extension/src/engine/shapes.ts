/**
 * The engine's answers to the reads of stage 5 in the shapes the views take today (`src/shared/protocol.ts`), so that
 * nothing a user sees changes while the views are not moved to the engine's own shapes (stage 6). Where the two differ
 * is in `docs/transition/04-differences.md`, **Reads (stage 5)**.
 */
import type {
  BlameLine as EngineBlameLine,
  BranchLog as EngineBranchLog,
  CommitDetail as EngineCommitDetail,
  CompareView as EngineCompareView,
  ConflictProposals,
  EngineFileChange,
  EngineState,
  FilePatch as EngineFilePatch,
  IgnoredEntry as EngineIgnoredEntry,
  MergeSides,
  Patch,
  PullRequest as EnginePullRequest,
  ReflogPosition,
  SetupPreview as EngineSetupPreview,
  Submodule as EngineSubmodule,
} from './messages';
import type {
  BranchLog,
  CommitDetail,
  CompareMode,
  CompareView,
  FileChange,
  FilePatch,
  Operation,
  PullRequest,
  RepoParts,
  Resolution,
  Submodule,
  WorkingState,
} from '../shared/protocol';
import { subjectAndBody } from '../shared/message';
import { recentBranches } from '../shared/refs';
import { type IgnoredEntry, type SetupPreview, suggest } from '../shared/worktreeSetup';

/** Who last changed a line, as the editors of the extension show it (`BlameLine` of `src/git/parse.ts`). */
export interface ShownBlameLine {
  hash: string;
  author: string;
  date: number;
  summary: string;
  committed: boolean;
  previous?: { hash: string; file: string };
}

/** A place HEAD or a branch was at, as the extension lists it (`ReflogEntry` of `src/git/parse.ts`). */
export interface ShownReflogEntry {
  hash: string;
  subject: string;
  selector: string;
  date: number;
}

/** What the tab of a comparison shows but its sides and its mode, as `Repo.compareSides` reads it. */
export type ComparedSides = Omit<CompareView, 'from' | 'to' | 'mode'>;

/** The letter git gives each kind of change, which the views show. */
const LETTERS: Record<EngineFileChange['kind'], string> = {
  added: 'A',
  modified: 'M',
  deleted: 'D',
  renamed: 'R',
  copied: 'C',
  typeChanged: 'T',
};

/** The kind of change of a letter of git; an untracked file is added, a file in conflict changed. */
const KINDS: Record<string, EngineFileChange['kind']> = {
  A: 'added',
  M: 'modified',
  D: 'deleted',
  R: 'renamed',
  C: 'copied',
  T: 'typeChanged',
  '?': 'added',
  U: 'modified',
};

/** The id git gives the lines that are in no commit yet. */
const NOT_COMMITTED = '0'.repeat(40);

/** What the extension calls the operations the engine names. */
const OPERATIONS: Record<NonNullable<MergeSides['operation']>, Operation> = {
  merge: 'merge',
  rebase: 'rebase',
  cherryPick: 'cherry-pick',
  revert: 'revert',
  patch: 'am',
};

/** How the review of the conflicts calls what a rule of the engine proposes. */
const RESOLUTIONS: Record<NonNullable<ConflictProposals['conflicts'][number]['proposal']>['case'], Resolution> = {
  spacesOrLineEndings: 'whitespace',
  sameChange: 'same',
  oneSide: 'oneSide',
  nextToEachOther: 'separate',
};

/** The checks of a pull request as the views name them. */
const CHECKS: Record<NonNullable<EnginePullRequest['checks']>, PullRequest['checks']> = {
  passed: 'success',
  failed: 'failure',
  running: 'pending',
};

/** A file changed, as the views take it. */
export function fileChange(file: EngineFileChange): FileChange {
  return file.originalPath && (file.kind === 'renamed' || file.kind === 'copied')
    ? { status: LETTERS[file.kind], path: file.path, oldPath: file.originalPath }
    : { status: LETTERS[file.kind], path: file.path };
}

/** A file changed of the views, as the engine takes it. */
export function engineFileChange(file: FileChange): EngineFileChange {
  return { path: file.path, originalPath: file.oldPath ?? null, kind: KINDS[file.status[0]] ?? 'modified' };
}

/** The operation stopped half way, as the extension names it. */
export function operation(name: MergeSides['operation']): Operation | null {
  return name ? OPERATIONS[name] : null;
}

/** What is not committed, split as `git status` splits it: the untracked files are unstaged with the status `?`. */
export function workingState(tree: EngineState['workingTree']): WorkingState {
  return {
    staged: tree.staged.map(fileChange),
    unstaged: [...tree.unstaged.map((file) => ({ status: LETTERS[file.kind], path: file.path })), ...tree.untracked.map((file) => ({ status: '?', path: file }))],
    conflicts: tree.conflicted.map((file) => ({ status: 'U', path: file.path })),
  };
}

/** Every changed file once, the files in conflict first, as `allChanges` of `src/git/parse.ts` counts them. */
function changedCount(working: WorkingState): number {
  return new Set([...working.conflicts, ...working.staged, ...working.unstaged].map((file) => file.path)).size;
}

/** A submodule, with what is checked out in it, as the Submodules view shows it. */
export function submodule(read: EngineSubmodule): Submodule {
  const checkout = read.checkout;
  return {
    path: read.folder,
    parent: read.parent ?? '',
    name: read.name,
    url: read.url ?? '',
    branch: read.branch,
    recorded: read.recorded,
    staged: read.staged,
    head: checkout?.commit ?? null,
    headBranch: checkout?.branch ?? null,
    ahead: checkout?.distance?.ahead ?? null,
    behind: checkout?.distance?.behind ?? null,
    changes: checkout ? checkout.uncommitted : null,
  };
}

/** How many conflicts of each file in conflict the rules propose a resolution for, as the views list them. */
export function conflictFiles(proposals: ConflictProposals[]): RepoParts['conflictFiles'] {
  return proposals
    .filter((file) => file.leftAlone === null && file.conflicts.length > 0)
    .map((file) => ({
      path: file.path,
      conflicts: file.conflicts.length,
      resolvable: file.conflicts.flatMap((conflict) => (conflict.proposal ? [RESOLUTIONS[conflict.proposal.case]] : [])),
    }));
}

/** The state of the repository but its commits, from what the engine read (`repo.read-state`). */
export function repoParts(state: EngineState, conflicts: RepoParts['conflictFiles']): RepoParts {
  const branch = state.head.branch?.replace(/^refs\/heads\//, '') ?? null;
  const working = workingState(state.workingTree);
  const branches = state.branches.map((each) => ({
    name: each.name,
    hash: each.commit,
    date: each.date,
    current: each.isCheckedOut,
    ...(each.upstream && {
      upstream: {
        name: each.upstream.reference.replace(/^refs\/(remotes|heads)\//, ''),
        remote: each.upstream.remote,
        branch: each.upstream.remoteBranch.replace(/^refs\/heads\//, ''),
        ahead: each.upstream.ahead,
        behind: each.upstream.behind,
        gone: each.upstream.isGone,
      },
    }),
  }));
  const here = changedCount(working);
  return {
    head: state.head.commit,
    branch,
    branches,
    remoteBranches: state.remoteBranches.map((each) => ({ name: each.name, hash: each.commit, date: each.date })),
    tags: state.tags.map((each) => ({ name: each.name, hash: each.commit, date: each.date })),
    stashes: state.stashes.map((each) => ({ ref: `stash@{${each.index}}`, hash: each.commit, message: each.message, date: each.date })),
    worktrees: state.worktrees.map((each) => ({
      path: each.folder,
      // `git worktree list` says forty zeros before the first commit, where the engine says none.
      hash: each.commit ?? NOT_COMMITTED,
      branch: each.branch?.replace(/^refs\/heads\//, '') ?? null,
      current: each.isCurrent,
      main: each.isMain,
      prunable: each.isMissing,
      // The worktree shown counts its changes as the Changes view does.
      changes: each.isCurrent ? here : each.uncommitted,
    })),
    submodules: state.submodules.map(submodule),
    remotes: state.remotes,
    working,
    conflictFiles: conflicts,
    operation: operation(state.operation),
    rebaseStop: state.rebaseStop && { kind: state.rebaseStop.kind, hash: state.rebaseStop.commit, subject: state.rebaseStop.subject },
    bisect: state.bisect && { good: state.bisect.good, bad: state.bisect.bad, skipped: state.bisect.skipped, suspects: state.bisect.remaining },
    identity: { name: state.identity.name ?? '', email: state.identity.email ?? '' },
    recentBranches: recentBranches(
      state.reflogSubjects.join('\n'),
      branches.map((each) => each.name),
      branch,
    ),
  };
}

/** The details of a commit or of a stash, its message split into a subject and a body. */
export function commitDetail(read: EngineCommitDetail): CommitDetail | null {
  if (!read.id || !read.author || !read.committer) return null;
  const { subject, body } = subjectAndBody(read.message);
  // A stash keeps the untracked files it took in its third parent; the commit is there even when there were none.
  const third = read.parents[2];
  return {
    hash: read.id,
    parents: read.parents,
    author: read.author.name,
    email: read.author.email,
    date: read.author.date,
    committer: read.committer.name,
    committerEmail: read.committer.email,
    commitDate: read.committer.date,
    subject,
    body,
    files: read.files.map(fileChange),
    untracked: third && read.untrackedFiles.length ? { hash: third, files: read.untrackedFiles.map(fileChange) } : undefined,
  };
}

/** The lines of one file, each with its sign first, as the views draw them. */
function filePatch(read: EngineFilePatch): FilePatch {
  const signs = { context: ' ', added: '+', removed: '-', noNewline: '\\' } as const;
  return {
    file: fileChange(read),
    added: read.added,
    removed: read.removed,
    binary: read.isBinary,
    hunks: read.isPastLimit ? null : read.hunks.map((hunk) => ({ header: hunk.header, lines: hunk.lines.map((line) => signs[line.kind] + line.text) })),
  };
}

/** The lines of the files of a commit or of a comparison; null when they did not match the files listed. */
export function filePatches(patch: Patch): FilePatch[] | null {
  return patch.kind === 'files' ? patch.files.map(filePatch) : null;
}

/** A comparison as its tab shows it. `since` is where its files are compared from, as `Repo.compareSides` says it. */
export function comparedSides(view: EngineCompareView, from: string, mode: CompareMode, path: string | null, limit: number): ComparedSides {
  const ahead = view.onlyInSecond;
  const behind = view.onlyInFirst;
  return {
    base: view.mergeBase,
    since: mode === 'diverged' && view.mergeBase ? view.mergeBase : from,
    ahead,
    behind,
    commits: view.commits.map((commit) => ({
      hash: commit.id,
      subject: commit.subject,
      author: commit.authorName,
      date: commit.authorDate,
      side: commit.side === 'first' ? 'from' : 'to',
      equivalent: commit.isAlsoInOtherSide,
    })),
    matched: ahead + behind <= limit,
    conflicts: view.merge.kind === 'conflicts' ? view.merge.files : view.merge.kind === 'clean' ? [] : null,
    path,
    files: view.files.map(fileChange),
  };
}

/** The commits of what is checked out, for the Commits view. */
export function branchLog(root: string, log: EngineBranchLog): BranchLog {
  return {
    root,
    commits: log.commits.map((commit) => ({
      hash: commit.id,
      parents: commit.parents,
      author: commit.authorName,
      email: commit.authorEmail,
      date: commit.authorDate,
      subject: commit.subject,
    })),
    hasMore: log.hasMore,
    unpushed: log.unpushed,
    upstream: log.upstream,
  };
}

/** The lines of a file not committed yet share one blame, as the extension's own read gives them. */
const notCommitted = (): ShownBlameLine => ({ hash: NOT_COMMITTED, author: '', date: 0, summary: '', committed: false });

/** Who last changed one line. */
export function blameLine(line: EngineBlameLine): ShownBlameLine {
  if (line.kind === 'notCommitted') return notCommitted();
  return {
    hash: line.commit,
    author: line.author,
    date: line.authorTime,
    summary: line.summary,
    committed: true,
    ...(line.previous && { previous: { hash: line.previous.commit, file: line.previous.path } }),
  };
}

/** Who last changed each line of a file, the first at 0: the lines of one commit share one object, as the editors expect. */
export function blameLines(lines: Array<EngineBlameLine | null>): ShownBlameLine[] {
  const commits = new Map<string, ShownBlameLine>();
  const shown: ShownBlameLine[] = [];
  lines.forEach((line, index) => {
    if (!line) return;
    const key = line.kind === 'committed' ? line.commit : NOT_COMMITTED;
    let one = commits.get(key);
    if (!one) commits.set(key, (one = blameLine(line)));
    shown[index] = one;
  });
  return shown;
}

/** A place a reference was at. */
export function reflogEntry(position: ReflogPosition): ShownReflogEntry {
  return { hash: position.commit, subject: position.subject, selector: position.selector, date: position.time };
}

/** The open pull requests by the name of their branch, as the views show them. */
export function pullRequests(byBranch: Record<string, EnginePullRequest>): Record<string, PullRequest> {
  return Object.fromEntries(
    Object.entries(byBranch).map(([branch, pull]) => [
      branch,
      { number: pull.number, title: pull.title, url: pull.url, draft: pull.isDraft, checks: pull.checks ? CHECKS[pull.checks] : null },
    ]),
  );
}

/**
 * Something git ignores in a checkout, with what is proposed for it. `folder` says for a link whether it leads to a
 * folder, which the engine does not say.
 */
export function ignoredEntry(entry: EngineIgnoredEntry, linksToFolder: boolean): IgnoredEntry {
  const { kind } = entry;
  const folder = kind.kind === 'folder' || (kind.kind === 'link' && linksToFolder);
  const size =
    kind.kind === 'folder'
      ? { files: kind.files, bytes: kind.bytes, more: kind.hasMore }
      : kind.kind === 'file'
        ? { files: 1, bytes: kind.bytes, more: false }
        : { files: 0, bytes: 0, more: false };
  return { path: entry.path, folder, link: kind.kind === 'link', ...size, ...suggest({ path: entry.path, folder, bytes: size.bytes }) };
}

/** What link and copy patterns take from a checkout, with how many each takes by itself. */
export function setupPreview(preview: EngineSetupPreview): SetupPreview {
  const counts = (takes: EngineSetupPreview['linkPatterns']) => Object.fromEntries(takes.map((take) => [take.pattern, take.count]));
  return { link: preview.linked, copy: preview.copied, counts: { link: counts(preview.linkPatterns), copy: counts(preview.copyPatterns) } };
}

/**
 * The engine's answers to the reads of stage 5 in the shapes the views take (`src/shared/protocol.ts`), where those are
 * not the engine's own: the comparisons, the lines of a commit, the commits of the branch, the blame, the reflog, the
 * pull requests and what new worktrees get. The state of a repository and the details of a commit are the engine's
 * shapes from stage 6 (`src/shared/state.ts`), and are not converted. Where the two differ is in
 * `docs/transition/04-differences.md`, **Reads (stage 5)**.
 */
import type {
  BlameLine as EngineBlameLine,
  BranchLog as EngineBranchLog,
  CompareView as EngineCompareView,
  ConflictProposals,
  FilePatch as EngineFilePatch,
  IgnoredEntry as EngineIgnoredEntry,
  Patch,
  PullRequest as EnginePullRequest,
  ReflogPosition,
  SetupPreview as EngineSetupPreview,
} from './messages';
import type { BranchLog, CompareMode, CompareView, FilePatch, PullRequest, RepoParts, Resolution } from '../shared/protocol';
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

/** The id git gives the lines that are in no commit yet. */
const NOT_COMMITTED = '0'.repeat(40);

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

/** The lines of one file, each with its sign first, as the views draw them. */
function filePatch(read: EngineFilePatch): FilePatch {
  const signs = { context: ' ', added: '+', removed: '-', noNewline: '\\' } as const;
  return {
    file: { path: read.path, originalPath: read.originalPath, kind: read.kind },
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
    files: view.files,
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

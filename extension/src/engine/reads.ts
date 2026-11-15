/**
 * Where the extension's reads come from (stage 5 of `docs/transition/06-roadmap.md`): Gitardo's engine when it is
 * there, each read of version 1.4 of the protocol in the shape the views take, and the extension's own read in
 * TypeScript when the engine is not there or could not answer (D49). The state and the details of a commit are the
 * engine's shapes, which `Repo` reads too (stage 6): they come back as the engine wrote them. The callers, the controller and the providers, do
 * not ask which of the two answered. Each read is timed in the log: the engine's by the engine (`LogKind::Read`), the
 * TypeScript ones here.
 *
 * `src/engine` imports nothing of `src/git` (R1.5): the TypeScript reads are given by their shape, `TypeScriptReads`,
 * which `Repo` has, and the reads that are not `Repo`'s are given as functions by the caller.
 */
import * as fs from 'node:fs';
import * as path from 'node:path';
import type { EngineClient } from './client';
import { EngineError, ErrorCode } from './connection';
import type { Bytes, ComparisonSide } from './messages';
import {
  type ComparedSides,
  type ShownBlameLine,
  type ShownReflogEntry,
  blameLine,
  blameLines,
  branchLog,
  comparedSides,
  conflictFiles,
  filePatches,
  ignoredEntry,
  pullRequests,
  reflogEntry,
  setupPreview,
} from './shapes';
import {
  WORKTREE,
  type BranchLog,
  type CommitDetail,
  type CompareMode,
  type Comparison,
  type FileChange,
  type FilePatch,
  type Operation,
  type PullRequest,
  type RepoParts,
  type WorkingTree,
} from '../shared/protocol';
import { uncommittedDetail } from '../shared/state';
import type { CommandProposal, IgnoredEntry, SetupPatterns, SetupPreview } from '../shared/worktreeSetup';

/** The lines of a commit or of a comparison sent to a view at most, as the engine cuts them (`LINE_LIMIT`). */
const ENGINE_LINES = 20_000;
/** Past this many commits on the two sides of a comparison together, they are not matched with each other. */
const MATCHED_COMMITS = 2000;
/** A commit by its full id, which is how the engine is told one. */
const FULL_ID = /^(?:[0-9a-f]{40}|[0-9a-f]{64})$/;
/** A commit by its hash, whole or the start of it, as the extension takes one. */
const HASH = /^[0-9a-f]{7,64}$/;

/** What the reads write in the output channel. */
export interface ReadLog {
  debug(line: string): void;
  warn(line: string): void;
}

/** What the blame of a file is asked with: the text of its editor when it has edits not saved, and the commit to blame it in. */
export interface BlameAsked {
  contents?: string;
  revision?: string;
  ignoreWhitespace?: boolean;
}

/** Some lines of a file, from 1, the end in. */
export interface Lines {
  start: number;
  end: number;
}

/** The versions of a file in conflict, as the Flow Merge compares them. */
export interface ConflictVersions {
  /** Whether the index has a version of it on each side. */
  hasBothSides: boolean;
  base: string;
  ours: string;
  theirs: string;
  operation: Operation | null;
}

/** The reads of the extension in TypeScript that the engine answers in their place: those of `Repo` in `src/git/repo.ts`. */
export interface TypeScriptReads {
  readonly root: string;
  parts(resolve?: boolean): Promise<RepoParts>;
  resolve(revision: string): Promise<string | null>;
  commitDetail(hash: string): Promise<CommitDetail | null>;
  commitPatch(detail: CommitDetail, limit: number): Promise<FilePatch[] | null>;
  working(): Promise<WorkingTree>;
  ignored(paths: string[]): Promise<Set<string>>;
  conflicted(): Promise<string[]>;
  conflictStages(file: string): Promise<number[]>;
  conflictNames(): Promise<[ours: string | null, theirs: string | null]>;
  conflictSubjects(): Promise<[ours: string | null, theirs: string | null]>;
  operation(): Promise<Operation | null>;
  compare(from: string, to: string): Promise<Comparison | null>;
  compareSides(from: string, to: string, mode: CompareMode, path?: string | null): Promise<ComparedSides | null>;
  comparePatch(files: FileChange[], since: string, to: string, limit: number, path?: string | null): Promise<FilePatch[] | null>;
  compareText(since: string, to: string, path?: string | null): Promise<string>;
  branchLog(limit: number): Promise<BranchLog>;
  blameLine(file: string, line: number, options?: BlameAsked): Promise<ShownBlameLine | null>;
  blame(file: string, options?: BlameAsked): Promise<ShownBlameLine[] | null>;
  remotes(): Promise<Array<{ name: string; url: string }>>;
  reflog(reference: string, count: number): Promise<ShownReflogEntry[]>;
  committedLines(file: string, lines: Lines, contents: string): Promise<(Lines & { commit: string }) | 'file' | 'lines'>;
  show(revision: string, file: string): Promise<string>;
  squashMessage(): Promise<string | null>;
  commonDir(): Promise<string>;
}

/** Text as it travels. */
const bytesOf = (text: string): Bytes => ({ base64: Buffer.from(text, 'utf8').toString('base64') });

/** The text of bytes that travelled, as git's output is read in TypeScript. */
const textOf = (bytes: Bytes): string => Buffer.from(bytes.base64, 'base64').toString('utf8');

/** A side of a comparison, as the engine is told it: a commit by its full id, or the working tree. */
function comparisonSide(side: string): ComparisonSide | null {
  if (side === WORKTREE) return { kind: 'workingTree' };
  return FULL_ID.test(side) ? { kind: 'commit', commit: side } : null;
}

/** The engine of the window, or none, and the reads that ask it first. */
export class Reads {
  constructor(
    private engine: () => EngineClient | undefined = () => undefined,
    private log: ReadLog = { debug: () => {}, warn: () => {} },
  ) {}

  /** Asks this engine from now on, and writes in this log. */
  use(engine: () => EngineClient | undefined, log: ReadLog): void {
    this.engine = engine;
    this.log = log;
  }

  /**
   * One read: the engine's answer when it is there and `onEngine` asks it, the TypeScript read otherwise. A failure of
   * the engine is said in the log and read in TypeScript; `onEngine` answers undefined for what the engine cannot read.
   */
  private async read<T>(what: string, root: string, onEngine: ((client: EngineClient) => Promise<T | undefined>) | null, inTypeScript: () => Promise<T>): Promise<T> {
    const client = onEngine && this.engine();
    if (client) {
      try {
        const answer = await onEngine(client);
        if (answer !== undefined) return answer;
      } catch (error) {
        if (!(error instanceof EngineError)) throw error;
        if (error.code === ErrorCode.cancelled) throw error;
        this.log.warn(`Gitardo's engine could not read ${what} in ${root}, which is read without it: ${error.message}`);
      }
    }
    const started = Date.now();
    const answer = await inTypeScript();
    this.log.debug(`Read ${what} in TypeScript (${root}), ${Date.now() - started} ms`);
    return answer;
  }

  /**
   * The state of the repository but the commits of the graph (`repo.read-state`). With `resolve`, the files in
   * conflict say how many of their conflicts the rules resolve (`conflicts.resolve-by-rules`).
   */
  parts(repo: TypeScriptReads, resolve = false): Promise<RepoParts> {
    return this.read(
      'the state',
      repo.root,
      async (client) => {
        const state = await client.currentState(repo.root);
        const conflicts = resolve && state.workingTree.conflicted.length ? conflictFiles(await client.conflictRules(repo.root, null)) : [];
        return { ...state, conflictFiles: conflicts };
      },
      () => repo.parts(resolve),
    );
  }

  /** The commit a branch, a tag, a hash or anything else git understands stands for; null when there is none. */
  resolve(repo: TypeScriptReads, revision: string): Promise<string | null> {
    // What starts with a dash would be read as an option.
    if (!revision || revision.startsWith('-')) return Promise.resolve(null);
    return this.read('a revision', repo.root, (client) => client.searchRevision(repo.root, revision), () => repo.resolve(revision));
  }

  /** The details of a commit, a stash or the uncommitted changes (`WORKTREE`). */
  commitDetail(repo: TypeScriptReads, hash: string): Promise<CommitDetail | null> {
    if (hash === WORKTREE) {
      return this.read(
        'the uncommitted changes',
        repo.root,
        async (client) => uncommittedDetail(await client.workingTree(repo.root)),
        () => repo.commitDetail(hash),
      );
    }
    if (!HASH.test(hash)) return Promise.resolve(null);
    return this.read(
      'the details of a commit',
      repo.root,
      async (client) => {
        const commit = FULL_ID.test(hash) ? hash : await client.searchRevision(repo.root, hash);
        const read = commit ? await client.details(repo.root, { commit }) : null;
        return read?.id && read.author && read.committer ? read : null;
      },
      () => repo.commitDetail(hash),
    );
  }

  /** What a commit changed in each of its files, no more than `limit` lines of them; null when they do not match its files. */
  commitPatch(repo: TypeScriptReads, detail: CommitDetail, limit: number): Promise<FilePatch[] | null> {
    const commit = detail.id;
    const asked = commit !== null && FULL_ID.test(commit) && limit === ENGINE_LINES;
    return this.read(
      'the lines of a commit',
      repo.root,
      asked ? async (client) => filePatches(await client.commitPatch(repo.root, commit, null)) : null,
      () => repo.commitPatch(detail, limit),
    );
  }

  /** What is not committed, as `git status` splits it. */
  working(repo: TypeScriptReads): Promise<WorkingTree> {
    return this.read('the working tree', repo.root, (client) => client.workingTree(repo.root), () => repo.working());
  }

  /** Those of these files that git ignores. */
  ignored(repo: TypeScriptReads, paths: string[]): Promise<Set<string>> {
    if (!paths.length) return Promise.resolve(new Set());
    return this.read('the ignored files', repo.root, async (client) => new Set(await client.ignored(repo.root, paths)), () => repo.ignored(paths));
  }

  /** The files in conflict. */
  conflicted(repo: TypeScriptReads): Promise<string[]> {
    return this.read(
      'the files in conflict',
      repo.root,
      async (client) => [...new Set((await client.conflictFiles(repo.root)).map((file) => file.path))],
      () => repo.conflicted(),
    );
  }

  /** Which sides of a conflict the index has for a file: 1 is the common ancestor, 2 "ours", 3 "theirs". */
  conflictStages(repo: TypeScriptReads, file: string): Promise<number[]> {
    return this.read(
      'the stages of a file in conflict',
      repo.root,
      async (client) => {
        const found = (await client.conflictFiles(repo.root)).find((each) => each.path === file);
        return found ? [found.base, found.current, found.incoming].flatMap((stage, index) => (stage ? [index + 1] : [])) : [];
      },
      () => repo.conflictStages(file),
    );
  }

  /** What the two sides of the conflicts are called, and the subjects of their commits (`conflicts.side-names`). */
  conflictSides(repo: TypeScriptReads): Promise<{ names: [string | null, string | null]; subjects: [string | null, string | null] }> {
    return this.read(
      'the sides of the conflicts',
      repo.root,
      async (client) => {
        const { current, incoming } = await client.sideNames(repo.root);
        return { names: [current.name, incoming.name], subjects: [current.subject, incoming.subject] };
      },
      async () => {
        const [names, subjects] = await Promise.all([repo.conflictNames(), repo.conflictSubjects()]);
        return { names, subjects };
      },
    );
  }

  /** The versions of a file in conflict, each empty when the index has none, and the operation that stopped on it. */
  conflictVersions(repo: TypeScriptReads, file: string): Promise<ConflictVersions> {
    return this.read(
      'the versions of a file in conflict',
      repo.root,
      async (client) => {
        const sides = await client.mergeSides(repo.root, file, null);
        return {
          hasBothSides: sides.hasBothSides,
          base: textOf(sides.base),
          ours: textOf(sides.current),
          theirs: textOf(sides.incoming),
          operation: sides.operation,
        };
      },
      async () => {
        const stages = await repo.conflictStages(file).catch(() => [] as number[]);
        // Both sides added the file: there is no base, which is the same as an empty one.
        const stage = (number: number) => (stages.includes(number) ? repo.show(`:${number}`, file).catch(() => '') : Promise.resolve(''));
        const [base, ours, theirs, stopped] = await Promise.all([stage(1), stage(2), stage(3), repo.operation().catch(() => null)]);
        return { hasBothSides: stages.includes(2) && stages.includes(3), base, ours, theirs, operation: stopped };
      },
    );
  }

  /** The files that differ between two commits. */
  compare(repo: TypeScriptReads, from: string, to: string): Promise<Comparison | null> {
    if (!HASH.test(from) || !HASH.test(to)) return Promise.resolve(null);
    const asked = FULL_ID.test(from) && FULL_ID.test(to);
    return this.read(
      'the files of a comparison',
      repo.root,
      asked ? async (client) => ({ from, to, files: await client.compareFiles(repo.root, from, to) }) : null,
      () => repo.compare(from, to),
    );
  }

  /** Two commits, or a commit and the working tree, compared as the tab of a comparison shows them (`compare.read-sides`). */
  compareSides(repo: TypeScriptReads, from: string, to: string, mode: CompareMode, path: string | null = null): Promise<ComparedSides | null> {
    const first = comparisonSide(from);
    const second = comparisonSide(to);
    // From the working tree, the engine compares the files with the commit it is on whatever the mode, where the
    // extension compares them from where the two diverged; narrowed to a path, the engine counts only the commits that
    // touch it, and asks no merge when one side has none (R7). Those stay the extension's.
    const asked = first && second && path === null && !(from === WORKTREE && mode === 'diverged');
    return this.read(
      'a comparison',
      repo.root,
      asked
        ? async (client) => {
            const request = { first, second, mode: mode === 'diverged' ? 'sinceDiverged' : 'everything', path } as const;
            return comparedSides(await client.compareSides(repo.root, request), from, mode, path, MATCHED_COMMITS);
          }
        : null,
      () => repo.compareSides(from, to, mode, path),
    );
  }

  /** What changed in each of these files from one commit to another, either of which can be the working tree. */
  comparePatch(repo: TypeScriptReads, files: FileChange[], since: string, to: string, limit: number, path: string | null = null): Promise<FilePatch[] | null> {
    const first = comparisonSide(since);
    const second = comparisonSide(to);
    return this.read(
      'the lines of a comparison',
      repo.root,
      first && second && limit === ENGINE_LINES
        ? async (client) => {
            const request = { first, second, mode: 'everything', path, mergeBase: null, files, form: 'lines' } as const;
            return filePatches(await client.comparePatch(repo.root, request));
          }
        : null,
      () => repo.comparePatch(files, since, to, limit, path),
    );
  }

  /** The changes from one commit to another, either of which can be the working tree, as a patch `git apply` takes. */
  compareText(repo: TypeScriptReads, since: string, to: string, path: string | null = null): Promise<string> {
    const first = comparisonSide(since);
    const second = comparisonSide(to);
    return this.read(
      'the patch of a comparison',
      repo.root,
      first && second
        ? async (client) => {
            const request = { first, second, mode: 'everything' as const, path, mergeBase: null, files: [], form: 'text' as const };
            const patch = await client.comparePatch(repo.root, request);
            return patch.kind === 'text' ? textOf(patch.text) : undefined;
          }
        : null,
      () => repo.compareText(since, to, path),
    );
  }

  /** The commits of what is checked out, `limit` at most, and which of them are not pushed (`branch.read-log`). */
  branchLog(repo: TypeScriptReads, limit: number): Promise<BranchLog> {
    return this.read('the commits of the branch', repo.root, async (client) => branchLog(repo.root, await client.branchLog(repo.root, limit)), () => repo.branchLog(limit));
  }

  /** Who last changed a line, from 1. The engine blames the file as it is, and leaves whitespace to its setting. */
  blameLine(repo: TypeScriptReads, file: string, line: number, options: BlameAsked = {}): Promise<ShownBlameLine | null> {
    const contents = options.contents === undefined ? null : bytesOf(options.contents);
    return this.read(
      'the blame of a line',
      repo.root,
      options.revision
        ? null
        : async (client) => {
            const read = await client.blameLine(repo.root, file, line, contents);
            return read && blameLine(read);
          },
      () => repo.blameLine(file, line, options),
    );
  }

  /** Who last changed each line of a file, the first one at 0; null for a file that is in no commit. */
  blame(repo: TypeScriptReads, file: string, options: BlameAsked = {}): Promise<ShownBlameLine[] | null> {
    const contents = options.contents === undefined ? null : bytesOf(options.contents);
    return this.read(
      'the blame of a file',
      repo.root,
      async (client) => {
        const read = await client.blameFile(repo.root, file, options.revision ?? null, contents);
        return read && blameLines(read);
      },
      () => repo.blame(file, options),
    );
  }

  /** The remotes, with the URL each is fetched from. */
  remotes(repo: TypeScriptReads): Promise<Array<{ name: string; url: string }>> {
    return this.read('the remotes', repo.root, (client) => client.remotes(repo.root), () => repo.remotes());
  }

  /** Where a reference has been, the last first. */
  reflog(repo: TypeScriptReads, reference: string, count: number): Promise<ShownReflogEntry[]> {
    return this.read(
      'the reflog',
      repo.root,
      async (client) => (await client.reflog(repo.root, reference, count)).map(reflogEntry),
      () => repo.reflog(reference, count),
    );
  }

  /**
   * Some lines of a file as it is now, as the commit checked out has them: 'file' when that commit has no such file,
   * 'lines' when none of the lines are in it.
   */
  committedLines(repo: TypeScriptReads, file: string, lines: Lines, contents: string): Promise<(Lines & { commit: string }) | 'file' | 'lines'> {
    return this.read(
      'where lines were committed',
      repo.root,
      async (client) => {
        const found = await client.committedLines(repo.root, { path: file, renamedFrom: null, text: contents, lines });
        if (found.kind === 'inNoCommit') return 'file';
        return found.lines ? { ...found.lines, commit: found.commit } : 'lines';
      },
      () => repo.committedLines(file, lines, contents),
    );
  }

  /**
   * A file as a revision has it, as `git show <revision>:<file>` prints it: `:0` is the index, `:1` to `:3` the sides
   * of a conflict. Empty when there is no such file (R2).
   */
  async show(repo: TypeScriptReads, revision: string, file: string): Promise<string> {
    return (await this.fileAt(repo.root, revision, file, () => repo.show(revision, file))) ?? '';
  }

  /**
   * A file as a revision has it, or as the index has it for the revision `''` or `:0`; undefined when there is no such
   * file, which `inTypeScript` reads when the engine is not there.
   */
  fileAt(root: string, revision: string, file: string, inTypeScript: () => Promise<string | undefined>): Promise<string | undefined> {
    const stage = /^:([0-3])$/.exec(revision)?.[1];
    return this.read(
      'a file at a revision',
      root,
      async (client) => {
        if (revision === '' || stage === '0') {
          const { left } = await client.diffSides(root, { version: { kind: 'index' }, path: file }, { version: { kind: 'absent' }, path: file });
          return left ? textOf(left) : undefined;
        }
        if (stage) {
          const sides = await client.mergeSides(root, file, null);
          return textOf(stage === '1' ? sides.base : stage === '2' ? sides.current : sides.incoming);
        }
        const bytes = await client.fileRevision(root, file, revision);
        return bytes ? textOf(bytes) : undefined;
      },
      inTypeScript,
    );
  }

  /** The message git prepared for the commit of a squash merge, without the newlines at its end (R3); null otherwise. */
  squashMessage(repo: TypeScriptReads): Promise<string | null> {
    return this.read('the message prepared', repo.root, (client) => client.preparedMessage(repo.root), () => repo.squashMessage());
  }

  /** The git folder the worktrees share, where the references are. */
  commonDir(repo: TypeScriptReads): Promise<string> {
    return this.read('the common git folder', repo.root, async (client) => path.resolve(await client.commonFolder(repo.root)), () => repo.commonDir());
  }

  /**
   * The open pull requests of the repository on GitHub of a remote, by their branch. The engine asks VS Code for the
   * token; `inTypeScript` reads them when it is not there, or did not get one.
   */
  pullRequests(root: string, remote: string | null, inTypeScript: () => Promise<Record<string, PullRequest>>): Promise<Record<string, PullRequest>> {
    return this.read(
      'the pull requests',
      root,
      async (client) => {
        const read = await client.pullRequests(root, remote);
        // What GitHub answered is the answer: it is not asked twice.
        if (read.kind === 'failed') throw new Error(read.message);
        return read.kind === 'read' ? pullRequests(read.byBranch) : undefined;
      },
      inTypeScript,
    );
  }

  /** What git ignores in a checkout, with its size and what is proposed for it, for the page of what new worktrees get. */
  setupIgnored(root: string, inTypeScript: () => Promise<IgnoredEntry[]>): Promise<IgnoredEntry[]> {
    return this.read(
      'what git ignores',
      root,
      async (client) => {
        const entries = await client.setupIgnored(root, root);
        // The engine does not look where a link leads; the page proposes for a folder what it does not for a file.
        const toFolder = (entry: string) =>
          fs.promises.stat(path.join(root, entry)).then(
            (target) => target.isDirectory(),
            () => false,
          );
        return Promise.all(entries.map(async (entry) => ignoredEntry(entry, entry.kind.kind === 'link' && (await toFolder(entry.path)))));
      },
      inTypeScript,
    );
  }

  /** The commands proposed for new worktrees, by the lock files committed at the top of the checkout. */
  setupCommands(root: string, inTypeScript: () => Promise<CommandProposal[]>): Promise<CommandProposal[]> {
    return this.read('the commands proposed', root, (client) => client.setupCommands(root), inTypeScript);
  }

  /** What the patterns would take from the checkout for a new worktree, and what each pattern takes by itself. */
  setupPreview(root: string, patterns: SetupPatterns, inTypeScript: () => Promise<SetupPreview>): Promise<SetupPreview> {
    return this.read(
      'what the patterns take',
      root,
      async (client) => {
        const [preview] = await client.setupTakes(root, [{ folder: root, link: patterns.link, copy: patterns.copy, commit: null }]);
        return preview ? setupPreview(preview) : undefined;
      },
      inTypeScript,
    );
  }
}

/** The reads of this window: `extension.ts` gives them its engine when it starts. */
export const reads = new Reads();

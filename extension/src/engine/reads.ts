/**
 * Where the extension's reads come from: Gitardo's engine, each read of the protocol in the shape the views take. The
 * state and the details of a commit are the engine's shapes (stage 6), and come back as the engine wrote them. The
 * extension runs no git of its own (stage 10 of `docs/transition/06-roadmap.md`): without the engine, every read fails
 * with `EngineMissing`, whose message is the sentence the views show (D49). Each read is timed in the log by the engine
 * (`LogKind::Read`).
 */
import * as fs from 'node:fs';
import * as path from 'node:path';
import type { EngineClient } from './client';
import type { Bytes, ComparisonSide, DiscoveredRepository, PathStatus, SetupPreview as EngineSetupPreview, TakeRequest } from './messages';
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
  NO_ENGINE,
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
import type { MermaidAnswer, MermaidTheme } from '../shared/mermaid';
import { uncommittedDetail } from '../shared/state';
import type { CommandProposal, IgnoredEntry, SetupPatterns, SetupPreview } from '../shared/worktreeSetup';

/** A read, or a run, asked while there is no engine: its message is the sentence of the views. */
export class EngineMissing extends Error {
  constructor() {
    super(NO_ENGINE);
  }
}

/** A commit by its full id, which is how the engine is told one. */
const FULL_ID = /^(?:[0-9a-f]{40}|[0-9a-f]{64})$/;
/** A commit by its hash, whole or the start of it, as the extension takes one. */
const HASH = /^[0-9a-f]{7,64}$/;
/** Past this many commits on the two sides of a comparison together, they are not matched with each other. */
const MATCHED_COMMITS = 2000;

/** A repository or a worktree the extension reads, by its folder. */
export interface Repository {
  readonly root: string;
}

/** What the blame of a file is asked with: the text of its editor when it has edits not saved, and the commit to blame it in. */
export interface BlameAsked {
  contents?: string;
  revision?: string;
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

/** Text as it travels. */
const bytesOf = (text: string): Bytes => ({ base64: Buffer.from(text, 'utf8').toString('base64') });

/** The text of bytes that travelled. */
const textOf = (bytes: Bytes): string => Buffer.from(bytes.base64, 'base64').toString('utf8');

/** Two spellings of the same folder compare equal, whatever the slashes and, on Windows, the case. */
const folderKey = (folder: string) => (process.platform === 'win32' ? path.resolve(folder).toLowerCase() : path.resolve(folder));

/** Whether a file is in a folder, or is the folder. */
function contains(folder: string, file: string): boolean {
  const relative = path.relative(folder, file);
  return !relative.startsWith('..') && !path.isAbsolute(relative);
}

/** The engine of the window, or none, and the reads asked of it. */
export class Reads {
  constructor(private engine: () => EngineClient | undefined = () => undefined) {}

  /** Asks this engine from now on. */
  use(engine: () => EngineClient | undefined): void {
    this.engine = engine;
  }

  /** The engine, or `EngineMissing` when there is none. */
  client(): EngineClient {
    const client = this.engine();
    if (!client) throw new EngineMissing();
    return client;
  }

  /** Whether there is an engine to ask. */
  get available(): boolean {
    return this.engine() !== undefined;
  }

  /** A diagram of Mermaid drawn by the engine for a kind of VS Code's theme, or why it is not (`file-view.mermaid`). */
  mermaid(source: string, theme: MermaidTheme): Promise<MermaidAnswer> {
    return this.client().drawMermaid(source, theme);
  }

  /**
   * The repositories the folders of a workspace give access to, in their order, each with the git folder its worktrees
   * share (`repository/discover`).
   */
  discover(folders: string[]): Promise<DiscoveredRepository[]> {
    return this.client().discover(folders);
  }

  /**
   * The repository a file is in, the innermost one, and its path there with forward slashes. The folder of the file can
   * be gone with it: the repository is looked for higher up.
   */
  async repositoryOf(file: string): Promise<{ root: string; path: string } | undefined> {
    for (let folder = path.dirname(file); ; folder = path.dirname(folder)) {
      if (fs.existsSync(folder)) {
        const found = (await this.discover([folder])).filter((each) => contains(each.folder, file) && folderKey(each.folder) !== folderKey(file));
        const root = found.sort((one, other) => other.folder.length - one.folder.length)[0]?.folder;
        if (root) return { root, path: path.relative(root, file).split(path.sep).join('/') };
      }
      if (path.dirname(folder) === folder) return undefined;
    }
  }

  /**
   * The state of the repository but the commits of the graph (`repo.read-state`). With `resolve`, the files in
   * conflict say how many of their conflicts the rules resolve (`conflicts.resolve-by-rules`). With `refresh`, for
   * **Refresh** and a repository chosen, every other worktree is counted again; otherwise only those whose git folder
   * changed since the read before (M15).
   */
  async parts(repo: Repository, resolve = false, refresh = false): Promise<RepoParts> {
    const client = this.client();
    const state = await client.currentState(repo.root, undefined, refresh);
    const conflicts = resolve && state.workingTree.conflicted.length ? conflictFiles(await client.conflictRules(repo.root, null)) : [];
    return { ...state, conflictFiles: conflicts };
  }

  /** The commit a branch, a tag, a hash or anything else git understands stands for; null when there is none. */
  async resolve(repo: Repository, revision: string): Promise<string | null> {
    // What starts with a dash would be read as an option.
    if (!revision || revision.startsWith('-')) return null;
    return this.client().searchRevision(repo.root, revision);
  }

  /** A commit by its full id: as it is, or the one the start of a hash stands for. */
  private async fullId(repo: Repository, hash: string): Promise<string | null> {
    if (FULL_ID.test(hash)) return hash;
    return HASH.test(hash) ? this.resolve(repo, hash) : null;
  }

  /** A side of a comparison, as the engine is told it: a commit by its full id, or the working tree. */
  private async comparisonSide(repo: Repository, side: string): Promise<ComparisonSide | null> {
    if (side === WORKTREE) return { kind: 'workingTree' };
    const commit = await this.fullId(repo, side);
    return commit ? { kind: 'commit', commit } : null;
  }

  /** The details of a commit, a stash or the uncommitted changes (`WORKTREE`). */
  async commitDetail(repo: Repository, hash: string): Promise<CommitDetail | null> {
    const client = this.client();
    if (hash === WORKTREE) return uncommittedDetail(await client.workingTree(repo.root));
    const commit = await this.fullId(repo, hash);
    const read = commit ? await client.details(repo.root, { commit }) : null;
    return read?.id && read.author && read.committer ? read : null;
  }

  /** What a commit changed in each of its files, as far as the engine cuts them; null when they do not match its files. */
  async commitPatch(repo: Repository, detail: CommitDetail): Promise<FilePatch[] | null> {
    const commit = detail.id;
    if (commit === null || !FULL_ID.test(commit)) return null;
    return filePatches(await this.client().commitPatch(repo.root, commit, null));
  }

  /** What is not committed, as `git status` splits it. */
  working(repo: Repository): Promise<WorkingTree> {
    return this.client().workingTree(repo.root);
  }

  /** Those of these files that git ignores. */
  async ignored(repo: Repository, paths: string[]): Promise<Set<string>> {
    if (!paths.length) return new Set();
    return new Set(await this.client().ignored(repo.root, paths));
  }

  /** The files in conflict. */
  async conflicted(repo: Repository): Promise<string[]> {
    return [...new Set((await this.client().conflictFiles(repo.root)).map((file) => file.path))];
  }

  /** Which sides of a conflict the index has for a file: 1 is the common ancestor, 2 "ours", 3 "theirs". */
  async conflictStages(repo: Repository, file: string): Promise<number[]> {
    const found = (await this.client().conflictFiles(repo.root)).find((each) => each.path === file);
    return found ? [found.base, found.current, found.incoming].flatMap((stage, index) => (stage ? [index + 1] : [])) : [];
  }

  /** What the two sides of the conflicts are called, and the subjects of their commits (`conflicts.side-names`). */
  async conflictSides(repo: Repository): Promise<{ names: [string | null, string | null]; subjects: [string | null, string | null] }> {
    const { current, incoming } = await this.client().sideNames(repo.root);
    return { names: [current.name, incoming.name], subjects: [current.subject, incoming.subject] };
  }

  /** The versions of a file in conflict, each empty when the index has none, and the operation that stopped on it. */
  async conflictVersions(repo: Repository, file: string): Promise<ConflictVersions> {
    const sides = await this.client().mergeSides(repo.root, file, null);
    const { hasBothSides, operation } = sides;
    return { hasBothSides, base: textOf(sides.base), ours: textOf(sides.current), theirs: textOf(sides.incoming), operation };
  }

  /** The operation under way, which stopped on the conflicts or for the user: none when nothing is under way. */
  async operation(repo: Repository): Promise<Operation | null> {
    return (await this.client().currentState(repo.root)).operation;
  }

  /** The files that differ between two commits. */
  async compare(repo: Repository, from: string, to: string): Promise<Comparison | null> {
    const [first, second] = await Promise.all([this.fullId(repo, from), this.fullId(repo, to)]);
    if (!first || !second) return null;
    return { from, to, files: await this.client().compareFiles(repo.root, first, second) };
  }

  /**
   * Two commits, or a commit and the working tree, compared as the tab of a comparison shows them (`compare.read-sides`):
   * narrowed to a path, the commits and what a merge would do are still those of the whole sides (Q407).
   */
  async compareSides(repo: Repository, from: string, to: string, mode: CompareMode, path: string | null = null): Promise<ComparedSides | null> {
    const [first, second] = await Promise.all([this.comparisonSide(repo, from), this.comparisonSide(repo, to)]);
    if (!first || !second) return null;
    const request = { first, second, mode: mode === 'diverged' ? 'sinceDiverged' : 'everything', path } as const;
    return comparedSides(await this.client().compareSides(repo.root, request), from, mode, path, MATCHED_COMMITS);
  }

  /** What changed in each of these files from one commit to another, either of which can be the working tree. */
  async comparePatch(repo: Repository, files: FileChange[], since: string, to: string, path: string | null = null): Promise<FilePatch[] | null> {
    const [first, second] = await Promise.all([this.comparisonSide(repo, since), this.comparisonSide(repo, to)]);
    if (!first || !second) return null;
    const request = { first, second, mode: 'everything', path, mergeBase: null, files, form: 'lines' } as const;
    return filePatches(await this.client().comparePatch(repo.root, request));
  }

  /** The changes from one commit to another, either of which can be the working tree, as a patch `git apply` takes. */
  async compareText(repo: Repository, since: string, to: string, path: string | null = null): Promise<string> {
    const [first, second] = await Promise.all([this.comparisonSide(repo, since), this.comparisonSide(repo, to)]);
    if (!first || !second) return '';
    const request = { first, second, mode: 'everything' as const, path, mergeBase: null, files: [], form: 'text' as const };
    const patch = await this.client().comparePatch(repo.root, request);
    return patch.kind === 'text' ? textOf(patch.text) : '';
  }

  /** The commits of what is checked out, `limit` at most, and which of them are not pushed (`branch.read-log`). */
  async branchLog(repo: Repository, limit: number): Promise<BranchLog> {
    return branchLog(repo.root, await this.client().branchLog(repo.root, limit));
  }

  /**
   * Who last changed a line, from 1. The engine blames the file as it is, or as a commit has it, and leaves whitespace to
   * its setting.
   */
  async blameLine(repo: Repository, file: string, line: number, asked: BlameAsked = {}): Promise<ShownBlameLine | null> {
    if (asked.revision) return (await this.blame(repo, file, asked))?.[line - 1] ?? null;
    const contents = asked.contents === undefined ? null : bytesOf(asked.contents);
    const read = await this.client().blameLine(repo.root, file, line, contents);
    return read && blameLine(read);
  }

  /** Who last changed each line of a file, the first one at 0; null for a file that is in no commit. */
  async blame(repo: Repository, file: string, asked: BlameAsked = {}): Promise<ShownBlameLine[] | null> {
    const contents = asked.contents === undefined ? null : bytesOf(asked.contents);
    const read = await this.client().blameFile(repo.root, file, asked.revision ?? null, contents);
    return read && blameLines(read);
  }

  /** The remotes, with the URL each is fetched from. */
  remotes(repo: Repository): Promise<Array<{ name: string; url: string }>> {
    return this.client().remotes(repo.root);
  }

  /** Where a reference has been, the last first. */
  async reflog(repo: Repository, reference: string, count: number): Promise<ShownReflogEntry[]> {
    return (await this.client().reflog(repo.root, reference, count)).map(reflogEntry);
  }

  /**
   * Some lines of a file as it is now, as the commit checked out has them: 'file' when that commit has no such file,
   * 'lines' when none of the lines are in it.
   */
  async committedLines(repo: Repository, file: string, lines: Lines, contents: string): Promise<(Lines & { commit: string }) | 'file' | 'lines'> {
    const found = await this.client().committedLines(repo.root, { path: file, renamedFrom: null, text: contents, lines });
    if (found.kind === 'inNoCommit') return 'file';
    return found.lines ? { ...found.lines, commit: found.commit } : 'lines';
  }

  /**
   * A file as a revision has it, as `git show <revision>:<file>` prints it: `:0` is the index, `:1` to `:3` the sides
   * of a conflict. Empty when there is no such file (R2).
   */
  async show(repo: Repository, revision: string, file: string): Promise<string> {
    return (await this.fileAt(repo.root, revision, file)) ?? '';
  }

  /** A file as a revision has it, or as the index has it for the revision `''` or `:0`; undefined when there is no such file. */
  async fileAt(root: string, revision: string, file: string): Promise<string | undefined> {
    const client = this.client();
    const stage = /^:([0-3])$/.exec(revision)?.[1];
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
  }

  /** The message git prepared for the commit of a squash merge, without the newlines at its end (R3); null otherwise. */
  squashMessage(repo: Repository): Promise<string | null> {
    return this.client().preparedMessage(repo.root);
  }

  /** The git folder the worktrees share, where the references are. */
  async commonDir(repo: Repository): Promise<string> {
    return path.resolve(await this.client().commonFolder(repo.root));
  }

  /** The folder of git's files for this worktree, where its index and its HEAD are. */
  async gitFolder(repo: Repository): Promise<string> {
    return path.resolve((await this.client().currentState(repo.root)).gitFolder);
  }

  /** The open pull requests of the repository on GitHub of a remote, by their branch; the engine asks VS Code for the token. */
  async pullRequests(root: string, remote: string | null): Promise<Record<string, PullRequest>> {
    const read = await this.client().pullRequests(root, remote);
    // What GitHub answered is the answer: it is not asked twice.
    if (read.kind === 'failed') throw new Error(read.message);
    return read.kind === 'read' ? pullRequests(read.byBranch) : {};
  }

  /** What git ignores in a checkout, with its size and what is proposed for it, for the page of what new worktrees get. */
  async setupIgnored(root: string): Promise<IgnoredEntry[]> {
    const entries = await this.client().setupIgnored(root, root);
    // The engine does not look where a link leads; the page proposes for a folder what it does not for a file.
    const toFolder = (entry: string) =>
      fs.promises.stat(path.join(root, entry)).then(
        (target) => target.isDirectory(),
        () => false,
      );
    return Promise.all(entries.map(async (entry) => ignoredEntry(entry, entry.kind.kind === 'link' && (await toFolder(entry.path)))));
  }

  /** The commands proposed for new worktrees, by the lock files committed at the top of the checkout. */
  setupCommands(root: string): Promise<CommandProposal[]> {
    return this.client().setupCommands(root);
  }

  /** What the patterns would take from the checkout for a new worktree, and what each pattern takes by itself. */
  async setupPreview(root: string, patterns: SetupPatterns): Promise<SetupPreview> {
    const [preview] = await this.setupTakes(root, [{ folder: root, link: patterns.link, copy: patterns.copy, commit: null }]);
    return preview ? setupPreview(preview) : { link: [], copy: [], counts: { link: {}, copy: {} } };
  }

  /** What each of these requests of link and copy patterns takes, in the engine's shape (`worktree-setup.read-what-patterns-take`). */
  setupTakes(root: string, requests: TakeRequest[]): Promise<EngineSetupPreview[]> {
    return this.client().setupTakes(root, requests);
  }

  /** What these paths of the checkout are for git: tracked, ignored, folders (`worktree-setup.explain-change`). */
  pathStatuses(root: string, paths: string[]): Promise<PathStatus[]> {
    return this.client().pathStatuses(root, paths);
  }
}

/** What to call a repository in a list: its path from the folder of the workspace that contains it, or its folder's name. */
export function repositoryName(root: string, folders: string[]): string {
  for (const folder of folders) {
    const relative = path.relative(folder, root);
    if (relative && !relative.startsWith('..') && !path.isAbsolute(relative)) return `${path.basename(folder)}/${relative.replace(/\\/g, '/')}`;
  }
  return path.basename(root);
}

/** The reads of this window: `extension.ts` gives them its engine when it starts. */
export const reads = new Reads();

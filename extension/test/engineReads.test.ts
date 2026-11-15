/**
 * The reads of stage 5 of `docs/transition/06-roadmap.md`: each read of the engine asked through the client, and what
 * the extension's `Repo` (or the module that reads it in TypeScript) reads of the same repository. The two must say the
 * same, but where `docs/transition/04-differences.md` says why not, under **Reads (stage 5)**.
 */
import * as fs from 'node:fs';
import * as path from 'node:path';
import { describe, expect, it } from 'vitest';
import type { Bytes, DiffLine } from '../src/engine/messages';
import { resolveConflicts } from '../src/shared/conflicts';
import { git } from '../src/git/exec';
import { parseGitHubRemote } from '../src/git/github';
import { Repo } from '../src/git/repo';
import { previewSetup, proposedCommands, scanIgnored } from '../src/git/worktreeSetup';
import { WORKTREE } from '../src/shared/protocol';
import { built, cleanUpAfterEach, client, commitFile, folder, repository } from './engine/harness';

cleanUpAfterEach();

/** The text of bytes that travelled. */
const text = (bytes: Bytes | null) => (bytes ? Buffer.from(bytes.base64, 'base64').toString('utf8') : null);

/** Bytes as they travel. */
const bytesOf = (value: string): Bytes => ({ base64: Buffer.from(value, 'utf8').toString('base64') });

/** A line of a hunk of the engine as the extension keeps it: its sign, then its text. */
const signed = (line: DiffLine) => ({ context: ' ', added: '+', removed: '-', noNewline: '\\' })[line.kind] + line.text;

/** The full id of a revision. */
async function id(root: string, revision: string): Promise<string> {
  return (await git(root, ['rev-parse', revision])).stdout.trim();
}

/** A repository with `a.txt` changed twice on `main` and a branch `feature` at the first commit with a commit of its own. */
async function branched(): Promise<string> {
  const root = await repository();
  await git(root, ['branch', 'feature']);
  await commitFile(root, 'a.txt', 'one\ntwo\n', 'Add two');
  await commitFile(root, 'b.txt', 'b\n', 'Add b');
  await git(root, ['checkout', '--quiet', 'feature']);
  await commitFile(root, 'c.txt', 'c\n', 'Add c');
  await git(root, ['checkout', '--quiet', 'main']);
  return root;
}

/** A merge of `incoming` into `main` stopped on two conflicts in `c.txt`: one the sides differ in only by spaces, one real. */
async function conflicted(): Promise<string> {
  const root = await repository();
  const lines = ['one', 'two', 'three', 'four', 'five', 'six', 'seven', 'eight'];
  const file = (first: string, last: string) => [first, ...lines.slice(1, -1), last].join('\n') + '\n';
  await commitFile(root, 'c.txt', file('one', 'eight'), 'Add c');
  await git(root, ['branch', 'incoming']);
  await commitFile(root, 'c.txt', file('one  ,', 'eight here'), 'Change c here');
  await git(root, ['checkout', '--quiet', 'incoming']);
  await commitFile(root, 'c.txt', file('one ,', 'eight there'), 'Change c there');
  await git(root, ['checkout', '--quiet', 'main']);
  await git(root, ['merge', '--no-edit', 'incoming']).catch(() => undefined);
  return root;
}

describe.skipIf(!built)('the reads of the engine and of the extension', { timeout: 60_000 }, () => {
  it('search.read-revision and compare.resolve-revision give the commit that Repo.resolve gives', async () => {
    const root = await branched();
    const repo = new Repo(root);
    const engine = client();
    for (const revision of ['main', 'feature~1', 'HEAD', 'no-such-branch', '-x']) {
      expect(await engine.searchRevision(root, revision), revision).toBe(await repo.resolve(revision));
      expect(await engine.compareRevision(root, revision), revision).toBe(await repo.resolve(revision));
    }
  });

  it('details.read-commit has what Repo.commitDetail has of a commit, of a stash with its untracked files and of the changes', async () => {
    const root = await branched();
    const repo = new Repo(root);
    const engine = client();
    const head = await id(root, 'HEAD');
    // Both in the engine's shape (stage 6), field for field.
    const [detail, read] = await Promise.all([engine.details(root, { commit: head }), repo.commitDetail(head)]);
    expect(detail).toEqual(read);

    // The view lists the uncommitted changes in the order of the working tree, the engine by path: the same files.
    fs.writeFileSync(path.join(root, 'a.txt'), 'edited\n');
    fs.writeFileSync(path.join(root, 'new.txt'), 'new\n');
    const changes = await engine.details(root, { uncommitted: true });
    const working = await repo.commitDetail(WORKTREE);
    const byPath = <T extends { path: string }>(files: T[] = []) => [...files].sort((one, other) => one.path.localeCompare(other.path));
    expect({ ...changes, files: byPath(changes?.files) }).toEqual({ ...working, files: byPath(working?.files) });

    await git(root, ['stash', 'push', '--include-untracked', '-m', 'Both']);
    const stash = await id(root, 'stash@{0}');
    const [stashed, kept] = await Promise.all([engine.details(root, { stash: 0 }), repo.commitDetail(stash)]);
    expect(stashed?.id).toBe(stash);
    expect(stashed).toEqual(kept);
    expect(await engine.details(root, { stash: 1 })).toBeNull();
  });

  it('commit-page.read-patch has the lines Repo.commitPatch has, file by file', async () => {
    const root = await branched();
    const repo = new Repo(root);
    const engine = client();
    const commit = await id(root, 'HEAD~1');
    const patch = await engine.commitPatch(root, commit, null);
    const read = await repo.commitPatch((await repo.commitDetail(commit))!, 10_000);
    expect(patch.kind).toBe('files');
    if (patch.kind !== 'files' || !read) throw new Error('No patch');
    expect(patch.files.map((file) => [file.path, file.added, file.removed, file.isBinary])).toEqual(
      read.map((file) => [file.file.path, file.added, file.removed, file.binary]),
    );
    expect(patch.files.map((file) => file.hunks.map((hunk) => hunk.lines.filter((line) => line.kind !== 'noNewline').map(signed)))).toEqual(
      read.map((file) => file.hunks?.map((hunk) => hunk.lines.filter((line) => !line.startsWith('\\')))),
    );
  });

  it('changes.read-working-tree and changes.read-ignored read what Repo.working and Repo.ignored read', async () => {
    const root = await repository();
    await commitFile(root, '.gitignore', '*.log\n', 'Ignore the logs');
    fs.writeFileSync(path.join(root, 'kept.log'), 'tracked\n');
    await git(root, ['add', '--force', 'kept.log']);
    await git(root, ['commit', '--quiet', '-m', 'Keep a log']);
    fs.writeFileSync(path.join(root, 'a.txt'), 'edited\n');
    fs.writeFileSync(path.join(root, 'staged.txt'), 'staged\n');
    await git(root, ['add', 'staged.txt']);
    fs.writeFileSync(path.join(root, 'new.txt'), 'new\n');
    fs.writeFileSync(path.join(root, 'run.log'), 'log\n');
    const repo = new Repo(root);
    const engine = client();
    const [tree, working] = await Promise.all([engine.workingTree(root), repo.working()]);
    expect(tree).toEqual(working);
    const paths = ['a.txt', 'run.log', 'kept.log', 'missing.log'];
    expect(new Set(await engine.ignored(root, paths))).toEqual(await repo.ignored(paths));
  });

  it('diff.read-sides and file-view.read-revision give the bytes Repo.show and the disk give', async () => {
    const root = await branched();
    const repo = new Repo(root);
    const engine = client();
    const first = await id(root, 'HEAD~2');
    fs.writeFileSync(path.join(root, 'a.txt'), 'on disk\n');
    const sides = await engine.diffSides(
      root,
      { version: { kind: 'commit', commit: first }, path: 'a.txt' },
      { version: { kind: 'workingTree', unsaved: null }, path: 'a.txt' },
    );
    expect(text(sides.left)).toBe(await repo.show(first, 'a.txt'));
    expect(text(sides.right)).toBe(fs.readFileSync(path.join(root, 'a.txt'), 'utf8'));
    const typed = await engine.diffSides(
      root,
      { version: { kind: 'index' }, path: 'a.txt' },
      { version: { kind: 'workingTree', unsaved: bytesOf('typed\n') }, path: 'a.txt' },
    );
    expect([text(typed.left), text(typed.right)]).toEqual([await repo.show('', 'a.txt'), 'typed\n']);
    expect(text(await engine.fileRevision(root, 'a.txt', 'feature'))).toBe(await repo.show('feature', 'a.txt'));
    // R2: a file the commit does not have is null for the engine, and empty for Repo.show.
    expect(await engine.fileRevision(root, 'b.txt', 'feature')).toBeNull();
    expect(await repo.show('feature', 'b.txt')).toBe('');
  });

  it('merge-view.read-sides and conflicts.read-files read the stages Repo reads of a file in conflict', async () => {
    const root = await conflicted();
    const repo = new Repo(root);
    const engine = client();
    const sides = await engine.mergeSides(root, 'c.txt', null);
    expect([text(sides.base), text(sides.current), text(sides.incoming)]).toEqual([
      await repo.show(':1', 'c.txt'),
      await repo.show(':2', 'c.txt'),
      await repo.show(':3', 'c.txt'),
    ]);
    expect(text(sides.result)).toBe(fs.readFileSync(path.join(root, 'c.txt'), 'utf8'));
    expect(sides.operation).toBe(await repo.operation());
    const files = await engine.conflictFiles(root);
    expect(files.map((file) => file.path)).toEqual(await repo.conflicted());
    const stages = files.map((file) => [file.base, file.current, file.incoming].flatMap((stage, index) => (stage ? [index + 1] : [])));
    expect(stages).toEqual([await repo.conflictStages('c.txt')]);
  });

  it('conflicts.side-names names the sides as Repo.conflictNames does', async () => {
    const root = await conflicted();
    const names = await client().sideNames(root);
    const [ours, theirs] = await new Repo(root).conflictNames();
    expect([names.current.name, names.incoming.name]).toEqual([ours, theirs]);
    expect([names.current.role, names.incoming.role]).toEqual(['branchCheckedOut', 'branchBeingMerged']);
  });

  it('conflicts.resolve-by-rules resolves the conflicts that resolveConflicts resolves, with the same lines', async () => {
    const root = await conflicted();
    const repo = new Repo(root);
    const [proposals] = await client().conflictRules(root, null);
    const file = fs.readFileSync(path.join(root, 'c.txt'), 'utf8');
    const resolved = resolveConflicts(file, await repo.show(':1', 'c.txt'), { ours: await repo.show(':2', 'c.txt'), theirs: await repo.show(':3', 'c.txt') });
    expect(proposals.leftAlone).toBeNull();
    expect(proposals.conflicts.map((conflict) => [conflict.line, conflict.proposal !== null])).toEqual(
      resolved.conflicts.map((conflict) => [conflict.line, conflict.resolution !== null]),
    );
    const lines = file.split(/(?<=\n)/);
    const written = (proposal: (typeof proposals.conflicts)[number]['proposal']) =>
      proposal ? [lines.slice(0, proposal.replaced.start).length, proposal.lines.map((line) => line.text + line.ending).join('')] : null;
    expect(proposals.conflicts.map((conflict) => written(conflict.proposal))).toEqual(
      resolved.conflicts.map((conflict) => (conflict.patch ? [conflict.patch.from, conflict.patch.text] : null)),
    );
  });

  it('compare.read-sides, compare.read-files and compare.read-patch read what Repo.compareSides, compare and comparePatch read', async () => {
    const root = await branched();
    const repo = new Repo(root);
    const engine = client();
    const [main, feature] = [await id(root, 'main'), await id(root, 'feature')];
    const request = { first: { kind: 'commit', commit: feature }, second: { kind: 'commit', commit: main }, mode: 'sinceDiverged', path: null } as const;
    const [view, read] = await Promise.all([engine.compareSides(root, request), repo.compareSides(feature, main, 'diverged')]);
    expect(view.mergeBase).toBe(read?.base);
    expect(view.files).toEqual(read?.files);
    const sides = (commits: Array<{ side: string; id?: string; hash?: string }>) => commits.map((commit) => [commit.side, commit.id ?? commit.hash]);
    expect(sides(view.commits).map(([side, commit]) => [side === 'first' ? 'from' : 'to', commit])).toEqual(sides(read?.commits ?? []));
    expect([view.onlyInFirst, view.onlyInSecond]).toEqual([read?.behind, read?.ahead]);

    const files = await engine.compareFiles(root, feature, main);
    expect(files).toEqual((await repo.compare(feature, main))?.files);

    const patch = await engine.comparePatch(root, { ...request, mergeBase: view.mergeBase, files: view.files, form: 'lines' });
    const lines = await repo.comparePatch(read!.files, read!.since, main, 10_000);
    if (patch.kind !== 'files' || !lines) throw new Error('No patch');
    expect(patch.files.map((file) => [file.path, file.added, file.removed])).toEqual(lines.map((file) => [file.file.path, file.added, file.removed]));
    const textPatch = await engine.comparePatch(root, { ...request, mergeBase: view.mergeBase, files: view.files, form: 'text' });
    expect(textPatch.kind === 'text' && text(textPatch.text)).toBe(await repo.compareText(read!.since, main));
  });

  it('blame.read-line and blame.read-file blame the lines Repo.blameLine and Repo.blame blame', async () => {
    const root = await branched();
    await git(root, ['mv', 'a.txt', 'moved.txt']);
    await git(root, ['commit', '--quiet', '-m', 'Move a']);
    await commitFile(root, 'moved.txt', 'one\ntwo\nthree\n', 'Add three');
    const repo = new Repo(root);
    const engine = client();
    const typed = 'one\ntyped\nthree\n';
    const ofExtension = (line: { hash: string; author: string; summary: string; committed: boolean; previous?: { hash: string; file: string } } | null) =>
      line && (line.committed ? { commit: line.hash, author: line.author, summary: line.summary, previous: line.previous ?? null } : 'notCommitted');
    const ofEngine = (line: Awaited<ReturnType<typeof engine.blameLine>>) =>
      line &&
      (line.kind === 'committed'
        ? {
            commit: line.commit,
            author: line.author,
            summary: line.summary,
            previous: line.previous && { hash: line.previous.commit, file: line.previous.path },
          }
        : 'notCommitted');
    // R4: Repo.blameLine leaves out the version before, which the engine gives as blame.read-file does.
    const withoutPrevious = (line: ReturnType<typeof ofEngine>) => (line && typeof line === 'object' ? { ...line, previous: null } : line);
    for (const line of [1, 2, 3]) {
      expect(withoutPrevious(ofEngine(await engine.blameLine(root, 'moved.txt', line, bytesOf(typed)))), `line ${line}`).toEqual(
        ofExtension(await repo.blameLine('moved.txt', line, { contents: typed })),
      );
    }
    const third = await engine.blameLine(root, 'moved.txt', 3, null);
    expect(third?.kind === 'committed' && third.previous).toEqual({ commit: await id(root, 'HEAD~1'), path: 'moved.txt' });
    const [lines, read] = await Promise.all([engine.blameFile(root, 'moved.txt', null, null), repo.blame('moved.txt')]);
    expect(lines?.map(ofEngine)).toEqual(read?.map(ofExtension));
    const before = await id(root, 'HEAD~1');
    const [old, oldRead] = await Promise.all([engine.blameFile(root, 'moved.txt', before, null), repo.blame('moved.txt', { revision: before })]);
    expect(old?.map(ofEngine)).toEqual(oldRead?.map(ofExtension));
    expect(await engine.blameFile(root, 'never.txt', null, null)).toBe(await repo.blame('never.txt'));
  });

  it('file-history.read-names and file-history.read-committed-lines read what Repo.pathNames and Repo.committedLines read', async () => {
    const root = await repository();
    await commitFile(root, 'a.txt', 'one\ntwo\nthree\n', 'Grow a');
    await git(root, ['mv', 'a.txt', 'b.txt']);
    await git(root, ['commit', '--quiet', '-m', 'Rename a']);
    const repo = new Repo(root);
    const engine = client();
    expect(await engine.fileNames(root, 'b.txt')).toEqual(await repo.pathNames('b.txt'));
    const now = 'zero\none\ntwo\nthree\n';
    const lines = await engine.committedLines(root, { path: 'b.txt', renamedFrom: null, text: now, lines: { start: 3, end: 4 } });
    const read = await repo.committedLines('b.txt', { start: 3, end: 4 }, now);
    expect(lines).toEqual(
      typeof read === 'object' ? { kind: 'committed', commit: read.commit, path: 'b.txt', lines: { start: read.start, end: read.end } } : read,
    );
    const missing = await engine.committedLines(root, { path: 'never.txt', renamedFrom: null, text: now, lines: { start: 1, end: 1 } });
    expect([missing.kind, await repo.committedLines('never.txt', { start: 1, end: 1 }, now)]).toEqual(['inNoCommit', 'file']);
  });

  it('reflog.read, remote.read-remotes and identity.read read what Repo.reflog, Repo.remotes and Repo.localIdentityKeys read', async () => {
    const root = await branched();
    await git(root, ['remote', 'add', 'origin', 'https://example.com/app.git']);
    await git(root, ['remote', 'add', 'fork', 'git@example.com:me/app.git']);
    const repo = new Repo(root);
    const engine = client();
    const positions = await engine.reflog(root, 'HEAD', 5);
    expect(positions.map((position) => [position.commit, position.subject, position.selector, position.time])).toEqual(
      (await repo.reflog('HEAD', 5)).map((entry) => [entry.hash, entry.subject, entry.selector, entry.date]),
    );
    expect(await engine.remotes(root)).toEqual(await repo.remotes());
    const identity = await engine.identity(root);
    expect(identity.localKeys).toEqual(await repo.localIdentityKeys());
    expect(identity.identity).toEqual({ name: 'Gitardo Test', email: 'test@gitardo.invalid' });
  });

  it('submodule.read-list lists the submodules Repo.submodules lists', async () => {
    const inside = await repository();
    const root = await repository();
    await git(root, ['-c', 'protocol.file.allow=always', 'submodule', 'add', '--quiet', inside, 'libs/inside']);
    await git(root, ['commit', '--quiet', '-m', 'Add the submodule']);
    const [listed, read] = await Promise.all([client().submodules(root), new Repo(root).submodules()]);
    expect(listed).toEqual(read);
  });

  it('the reads of worktree-setup read what scanIgnored, previewSetup and proposedCommands read', async () => {
    const root = await repository();
    await commitFile(root, '.gitignore', 'node_modules/\n.env\n*.local\n', 'Ignore');
    await commitFile(root, 'package.json', '{ "packageManager": "pnpm@9.1.0" }\n', 'Add the package');
    await commitFile(root, 'pnpm-lock.yaml', 'lockfileVersion: 9\n', 'Lock');
    fs.mkdirSync(path.join(root, 'node_modules', 'left-pad'), { recursive: true });
    fs.writeFileSync(path.join(root, 'node_modules', 'left-pad', 'index.js'), 'module.exports = 1;\n');
    fs.writeFileSync(path.join(root, '.env'), 'KEY=1\n');
    fs.writeFileSync(path.join(root, 'settings.local'), 'x\n');
    const engine = client();
    const entries = await engine.setupIgnored(root, root);
    expect(
      entries.map((entry) => ({
        path: entry.path,
        folder: entry.kind.kind === 'folder',
        link: entry.kind.kind === 'link',
        bytes: entry.kind.kind === 'link' ? 0 : entry.kind.bytes,
        files: entry.kind.kind === 'folder' ? entry.kind.files : entry.kind.kind === 'file' ? 1 : 0,
      })),
    ).toEqual((await scanIgnored(root)).map((entry) => ({ path: entry.path, folder: entry.folder, link: entry.link, bytes: entry.bytes, files: entry.files })));
    const patterns = { link: ['node_modules'], copy: ['.env', '*.local'] };
    const [preview] = await engine.setupTakes(root, [{ folder: root, ...patterns, commit: null }]);
    const read = await previewSetup(root, patterns);
    expect([preview.linked, [...preview.copied].sort()]).toEqual([read.link, [...read.copy].sort()]);
    const counts = (takes: Array<{ pattern: string; count: number }>) => Object.fromEntries(takes.map((take) => [take.pattern, take.count]));
    expect({ link: counts(preview.linkPatterns), copy: counts(preview.copyPatterns) }).toEqual(read.counts);
    expect(await engine.setupCommands(root)).toEqual(await proposedCommands(root));
  });

  it('github.read-pull-requests finds the repository on GitHub that parseGitHubRemote finds, and reads nothing without a token', async () => {
    const root = await repository();
    const engine = client();
    await git(root, ['remote', 'add', 'origin', 'https://gitlab.com/owner/app.git']);
    expect([await engine.pullRequests(root, null), parseGitHubRemote('https://gitlab.com/owner/app.git')]).toEqual([{ kind: 'notOnGitHub' }, null]);
    await git(root, ['remote', 'add', 'hub', 'git@github.com:owner/app.git']);
    expect(parseGitHubRemote('git@github.com:owner/app.git')).toEqual({ owner: 'owner', repo: 'app' });
    expect(await engine.pullRequests(root, 'hub')).toEqual({ kind: 'noToken' });
  });

  it('branch.read-log lists the commits, what is more and what is not pushed as Repo.branchLog does', async () => {
    const root = await repository();
    await commitFile(root, 'a.txt', 'two\n', 'Second');
    const repo = new Repo(root);
    const engine = client();
    const compared = async (count: number) => {
      const [log, read] = await Promise.all([engine.branchLog(root, count), repo.branchLog(count)]);
      expect(log.commits.map((commit) => [commit.id, commit.parents, commit.authorName, commit.authorEmail, commit.authorDate, commit.subject])).toEqual(
        read.commits.map((commit) => [commit.hash, commit.parents, commit.author, commit.email, commit.date, commit.subject]),
      );
      expect([log.hasMore, log.unpushed, log.upstream]).toEqual([read.hasMore, read.unpushed, read.upstream]);
    };
    await compared(1);
    const remote = path.join(folder(), 'remote.git');
    await git(path.dirname(remote), ['init', '--quiet', '--bare', '--initial-branch=main', remote]);
    await git(root, ['remote', 'add', 'origin', remote]);
    await git(root, ['push', '--quiet', '--set-upstream', 'origin', 'main']);
    await commitFile(root, 'a.txt', 'three\n', 'Third');
    await compared(5);
    await git(root, ['branch', '--unset-upstream']);
    await compared(5);
  });

  it('repo.common-folder and commit.prepared-message read what Repo.commonDir and Repo.squashMessage read', async () => {
    const root = await branched();
    const repo = new Repo(root);
    const engine = client();
    const worktree = path.join(folder(), 'other');
    await git(root, ['worktree', 'add', '--quiet', worktree, 'feature']);
    for (const checkout of [root, worktree]) {
      expect(await engine.commonFolder(checkout)).toBe(await new Repo(checkout).commonDir());
    }
    expect(await engine.preparedMessage(root)).toBe(await repo.squashMessage());
    await git(root, ['merge', '--squash', 'feature']);
    // R3: the engine gives the message without the newlines at its end, which Repo.squashMessage keeps.
    expect(await engine.preparedMessage(root)).toBe((await repo.squashMessage())?.trimEnd());
  });
});

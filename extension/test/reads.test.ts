/**
 * The module the extension reads through (`src/engine/reads.ts`, stage 5): each read asked of Gitardo's engine, on
 * throwaway repositories, against what git itself says of them; and, with no engine, every read refused with
 * `EngineMissing`, as stage 10 of `docs/transition/06-roadmap.md` leaves the extension no other way to read.
 */
import * as fs from 'node:fs';
import * as path from 'node:path';
import { describe, expect, it } from 'vitest';
import { EngineClient } from '../src/engine/client';
import { EngineMissing, Reads } from '../src/engine/reads';
import { WORKTREE } from '../src/shared/protocol';
import { git } from './git';
import { built, cleanUpAfterEach, client, commitFile, folder, memoryLog, repository } from './engine/harness';

cleanUpAfterEach();

/** The full id of a revision. */
async function id(root: string, revision: string): Promise<string> {
  return (await git(root, ['rev-parse', revision])).stdout.trim();
}

/** What git says, line by line. */
async function lines(root: string, args: string[]): Promise<string[]> {
  return (await git(root, args)).stdout.split('\n').filter(Boolean);
}

/** The reads of a repository through the engine. */
function readsOf(root: string) {
  return { reads: new Reads(() => client()), repo: { root } };
}

/** A repository with branches that diverged, a tag of each kind, a remote, two stashes, a worktree and changes not committed. */
async function busy(): Promise<string> {
  const root = await repository();
  await git(root, ['branch', 'feature']);
  await commitFile(root, 'a.txt', 'one\ntwo\n', 'Add two');
  await commitFile(root, '.gitignore', '*.log\nnode_modules/\n', 'Ignore the logs');
  await git(root, ['tag', 'v1.0']);
  await git(root, ['tag', '-a', 'v1.1', '-m', 'Release 1.1']);
  await git(root, ['checkout', '--quiet', 'feature']);
  await commitFile(root, 'c.txt', 'c\n', 'Add c');
  await git(root, ['checkout', '--quiet', 'main']);
  await git(root, ['checkout', '--quiet', 'feature']);
  await git(root, ['checkout', '--quiet', 'main']);
  const remote = path.join(folder(), 'remote.git');
  await git(path.dirname(remote), ['init', '--quiet', '--bare', '--initial-branch=main', remote]);
  await git(root, ['remote', 'add', 'origin', remote]);
  await git(root, ['push', '--quiet', '--set-upstream', 'origin', 'main']);
  await commitFile(root, 'b.txt', 'b\n', 'Add b');
  fs.writeFileSync(path.join(root, 'a.txt'), 'stashed\n');
  fs.writeFileSync(path.join(root, 'taken.txt'), 'taken\n');
  await git(root, ['stash', 'push', '--include-untracked', '-m', 'With what is untracked']);
  fs.writeFileSync(path.join(root, 'a.txt'), 'stashed again\n');
  await git(root, ['stash', 'push']);
  await git(root, ['worktree', 'add', '--quiet', path.join(path.dirname(root), 'fix'), 'feature']);
  fs.writeFileSync(path.join(root, 'a.txt'), 'edited\n');
  fs.writeFileSync(path.join(root, 'staged.txt'), 'staged\n');
  await git(root, ['add', 'staged.txt']);
  fs.writeFileSync(path.join(root, 'new.txt'), 'new\n');
  fs.writeFileSync(path.join(root, 'run.log'), 'log\n');
  return root;
}

/** A merge of `incoming` into `main` stopped on a conflict in `c.txt` that the rules resolve, and one they do not. */
async function conflicted(): Promise<string> {
  const root = await repository();
  const lines = ['one', 'two', 'three', 'four', 'five', 'six', 'seven', 'eight'];
  const file = (first: string, last: string) => [first, ...lines.slice(1, -1), last].join('\n') + '\n';
  await commitFile(root, 'c.txt', file('one', 'eight'), 'Add c');
  await commitFile(root, 'gone.txt', 'gone\n', 'Add gone');
  await git(root, ['branch', 'incoming']);
  await commitFile(root, 'c.txt', file('one  ,', 'eight here'), 'Change c here');
  await commitFile(root, 'gone.txt', 'changed here\n', 'Change gone here');
  await git(root, ['checkout', '--quiet', 'incoming']);
  await commitFile(root, 'c.txt', file('one ,', 'eight there'), 'Change c there');
  await git(root, ['rm', '--quiet', 'gone.txt']);
  await git(root, ['commit', '--quiet', '-m', 'Remove gone']);
  await git(root, ['checkout', '--quiet', 'main']);
  await git(root, ['merge', '--no-edit', 'incoming']).catch(() => undefined);
  return root;
}

describe.skipIf(!built)('the reads of the extension through the engine', { timeout: 60_000 }, () => {
  it('read the state, the details, the lines and the comparisons git says', async () => {
    const root = await busy();
    const { reads, repo } = readsOf(root);
    const parts = await reads.parts(repo);
    expect(parts.head.commit).toBe(await id(root, 'HEAD'));
    expect(parts.branches.map((branch) => branch.name).sort()).toEqual(await lines(root, ['for-each-ref', '--format=%(refname:short)', 'refs/heads']));
    expect(parts.tags.map((tag) => tag.name).sort()).toEqual(['v1.0', 'v1.1']);
    expect(parts.stashes).toHaveLength(2);
    expect(parts.workingTree.untracked.sort()).toEqual(['new.txt']);
    for (const revision of ['main', 'feature~1', 'v1.1', 'HEAD', 'stash@{1}'])
      expect(await reads.resolve(repo, revision), revision).toBe(await id(root, `${revision}^{commit}`));
    for (const revision of ['stash@{5}', 'no-such-branch', '']) expect(await reads.resolve(repo, revision), revision).toBeNull();
    const [head, feature] = [await id(root, 'HEAD'), await id(root, 'feature')];
    expect((await reads.commitDetail(repo, head.slice(0, 8)))?.id).toBe(head);
    expect(await reads.commitDetail(repo, 'not-a-commit')).toBeNull();
    const detail = (await reads.commitDetail(repo, head))!;
    expect((await reads.commitPatch(repo, detail))?.map((file) => file.file.path)).toEqual(['b.txt']);
    expect([...(await reads.ignored(repo, ['a.txt', 'run.log', 'node_modules/x', 'missing.log']))].sort()).toEqual([
      'missing.log',
      'node_modules/x',
      'run.log',
    ]);
    expect((await reads.compare(repo, feature, head))?.files.map((file) => file.path).sort()).toEqual(
      await lines(root, ['diff', '--name-only', feature, head]),
    );
    const since = (await reads.compareSides(repo, feature, head, 'diverged'))!;
    expect(since.since).toBe((await git(root, ['merge-base', feature, head])).stdout.trim());
    expect(await reads.compareText(repo, since.since, head)).toBe((await git(root, ['diff', '--no-color', since.since, head])).stdout);
    // From the working tree, the patch names the sides a/ and b/ (R6).
    expect(await reads.compareText(repo, WORKTREE, feature)).toContain('diff --git a/');
    expect((await reads.branchLog(repo, 2)).commits.map((commit) => commit.hash)).toEqual(await lines(root, ['log', '-n', '2', '--format=%H']));
    expect((await reads.blame(repo, 'a.txt', { revision: feature }))?.map((line) => line.hash)).toEqual([await id(root, 'feature~1')]);
    expect(await reads.blame(repo, 'never.txt')).toBeNull();
    expect((await reads.blameLine(repo, 'a.txt', 2, { contents: 'one\ntyped\n' }))?.committed).toBe(false);
    expect((await reads.remotes(repo)).map((remote) => remote.name)).toEqual(['origin']);
    expect((await reads.reflog(repo, 'HEAD', 5)).map((entry) => entry.hash)).toEqual(await lines(root, ['reflog', '-n', '5', '--format=%H', 'HEAD']));
    expect(await reads.show(repo, feature, 'c.txt')).toBe('c\n');
    expect(await reads.show(repo, ':0', 'staged.txt')).toBe('staged\n');
    expect(await reads.squashMessage(repo)).toBeNull();
    const common = (await git(root, ['rev-parse', '--path-format=absolute', '--git-common-dir'])).stdout.trim();
    expect(path.resolve(await reads.commonDir(repo))).toBe(path.resolve(common));
  });

  it('read the bytes of an image at a revision as they are, for the preview of images (file-view.image)', async () => {
    const root = await repository();
    const bytes = Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0xff, 0x00, 0xfe]);
    fs.writeFileSync(path.join(root, 'logo.png'), bytes);
    await git(root, ['add', '--', 'logo.png']);
    await git(root, ['commit', '--quiet', '-m', 'Add the logo']);
    const { reads } = readsOf(root);
    const head = await id(root, 'HEAD');
    expect(Buffer.from((await reads.fileBytesAt(root, head, 'logo.png'))!)).toEqual(bytes);
    expect(Buffer.from((await reads.fileBytesAt(root, ':0', 'logo.png'))!)).toEqual(bytes);
    expect(await reads.fileBytesAt(root, head, 'none.png')).toBeUndefined();
  });

  it('read the conflicts, their sides and the stages git has', async () => {
    const root = await conflicted();
    const { reads, repo } = readsOf(root);
    expect((await reads.conflicted(repo)).sort()).toEqual(await lines(root, ['diff', '--name-only', '--diff-filter=U']));
    expect(await reads.conflictStages(repo, 'c.txt')).toEqual([1, 2, 3]);
    expect(await reads.conflictStages(repo, 'gone.txt')).toEqual([1, 2]);
    expect(await reads.conflictStages(repo, 'a.txt')).toEqual([]);
    expect((await reads.conflictSides(repo)).names).toEqual(['main', 'incoming']);
    for (const stage of [1, 2, 3]) expect(await reads.show(repo, `:${stage}`, 'c.txt')).toBe((await git(root, ['show', `:${stage}:c.txt`])).stdout);
  });

  it('read the submodules and the bisect of the state', async () => {
    const inside = await repository();
    const root = await repository();
    await git(root, ['-c', 'protocol.file.allow=always', 'submodule', 'add', '--quiet', inside, 'libs/inside']);
    await git(root, ['commit', '--quiet', '-m', 'Add the submodule']);
    await commitFile(root, 'a.txt', 'two\n', 'Second');
    await commitFile(root, 'a.txt', 'three\n', 'Third');
    await git(root, ['bisect', 'start']);
    await git(root, ['bisect', 'bad']);
    await git(root, ['bisect', 'good', 'HEAD~2']);
    const { reads, repo } = readsOf(root);
    const parts = await reads.parts(repo);
    expect(parts.submodules.map((submodule) => submodule.folder)).toEqual(['libs/inside']);
    expect(parts.bisect).not.toBeNull();
  });

  it('read the state before the first commit, and with a detached head and a worktree whose folder is gone', async () => {
    const empty = await repository(false);
    fs.writeFileSync(path.join(empty, 'new.txt'), 'new\n');
    const first = readsOf(empty);
    expect((await first.reads.parts(first.repo)).head.commit).toBeNull();
    const root = await repository();
    await git(root, ['worktree', 'add', '--quiet', '-b', 'gone', path.join(path.dirname(root), 'gone')]);
    fs.rmSync(path.join(path.dirname(root), 'gone'), { recursive: true, force: true });
    await git(root, ['checkout', '--quiet', '--detach']);
    const second = readsOf(root);
    const parts = await second.reads.parts(second.repo);
    expect(parts.head.branch).toBeNull();
    expect(parts.worktrees.some((worktree) => worktree.isMissing)).toBe(true);
  });

  it('read nothing of GitHub when the engine has no token', async () => {
    const root = await repository();
    await git(root, ['remote', 'add', 'origin', 'git@github.com:owner/app.git']);
    const { reads } = readsOf(root);
    expect(await reads.pullRequests(root, 'origin')).toEqual({});
  });
});

describe('the reads of the extension without the engine', () => {
  it('are refused with EngineMissing, which the views turn into the sentence that says so (D49)', async () => {
    const reads = new Reads();
    expect(reads.available).toBe(false);
    expect(() => reads.client()).toThrow(EngineMissing);
    await expect(reads.parts({ root: folder() })).rejects.toBeInstanceOf(EngineMissing);
    await expect(reads.resolve({ root: folder() }, 'main')).rejects.toBeInstanceOf(EngineMissing);
  });

  it('say the binary cannot be started when it is not there', async () => {
    const missing = new EngineClient({
      binary: path.join(folder(), 'no-such-engine'),
      clientVersion: '0.0.0-test',
      settings: () => ({}),
      logLevel: () => 'info',
      log: memoryLog(),
    });
    await expect(new Reads(() => missing).parts({ root: folder() })).rejects.toThrow();
  });
});

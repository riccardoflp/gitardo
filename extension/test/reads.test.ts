/**
 * The module the extension reads through (`src/engine/reads.ts`, stage 5): each read asked with Gitardo's engine and
 * without it, where `Repo` answers, on the same throwaway repositories. The views must get the same either way.
 */
import * as fs from 'node:fs';
import * as path from 'node:path';
import { describe, expect, it } from 'vitest';
import { EngineClient } from '../src/engine/client';
import { Reads } from '../src/engine/reads';
import { git } from '../src/git/exec';
import { Repo } from '../src/git/repo';
import { previewSetup, proposedCommands, scanIgnored } from '../src/git/worktreeSetup';
import { WORKTREE } from '../src/shared/protocol';
import { built, cleanUpAfterEach, client, commitFile, folder, memoryLog, repository } from './engine/harness';

cleanUpAfterEach();

/** The full id of a revision. */
async function id(root: string, revision: string): Promise<string> {
  return (await git(root, ['rev-parse', revision])).stdout.trim();
}

/** The reads with the engine, the reads without it, the repository read, and the log of the first. */
async function both(root: string) {
  const log = memoryLog();
  const engine = client();
  const repo = new Repo((await Repo.discover(root))!);
  return { withEngine: new Reads(() => engine, log), without: new Reads(), repo, log };
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

describe.skipIf(!built)('the reads of the extension with the engine and without it', { timeout: 60_000 }, () => {
  it('read the same state but the commits, the same details, lines and comparisons', async () => {
    const root = await busy();
    const { withEngine, without, repo, log } = await both(root);
    const same = async <T>(read: (reads: Reads) => Promise<T>) => {
      const [engine, typescript] = await Promise.all([read(withEngine), read(without)]);
      expect(engine).toEqual(typescript);
      return engine;
    };
    await same((reads) => reads.parts(repo));
    for (const revision of ['main', 'feature~1', 'v1.1', 'HEAD', 'stash@{1}', 'stash@{5}', 'no-such-branch', '']) await same((reads) => reads.resolve(repo, revision));
    const [head, stash, feature] = [await id(root, 'HEAD'), await id(root, 'stash@{1}'), await id(root, 'feature')];
    for (const hash of [head, head.slice(0, 8), stash, WORKTREE, 'not-a-commit']) await same((reads) => reads.commitDetail(repo, hash));
    const detail = (await repo.commitDetail(head))!;
    await same((reads) => reads.commitPatch(repo, detail, 20_000));
    await same((reads) => reads.working(repo));
    await same((reads) => reads.ignored(repo, ['a.txt', 'run.log', 'node_modules/x', 'missing.log']));
    await same((reads) => reads.compare(repo, feature, head));
    for (const [from, to, mode] of [
      [feature, head, 'diverged'],
      [feature, head, 'direct'],
      [head, WORKTREE, 'direct'],
      [WORKTREE, feature, 'direct'],
    ] as const) {
      const found = await same((reads) => reads.compareSides(repo, from, to, mode, null));
      await same((reads) => reads.comparePatch(repo, found!.files, found!.since, to, 20_000, null));
      if (from !== WORKTREE) await same((reads) => reads.compareText(repo, found!.since, to, null));
    }
    // R6: from the working tree, git's -R names the sides b/ and a/ in the extension's patch, a/ and b/ in the engine's.
    const unprefixed = (patch: string) => patch.replace(/^(diff --git|---|\+\+\+) [ab]\/(\S+)(?: [ab]\/(\S+))?/gm, '$1 $2 $3');
    const [engineText, typescriptText] = await Promise.all([withEngine.compareText(repo, WORKTREE, feature), without.compareText(repo, WORKTREE, feature)]);
    expect(unprefixed(engineText)).toEqual(unprefixed(typescriptText));
    expect(engineText).toContain('diff --git a/');
    await same((reads) => reads.compareSides(repo, feature, head, 'direct', 'c.txt'));
    await same((reads) => reads.branchLog(repo, 2));
    await same((reads) => reads.branchLog(repo, 10));
    // Of a line not committed, the editors show only that it is not: git's author, time and summary for it are not read.
    const shown = (lines: Awaited<ReturnType<Reads['blame']>>) => lines?.map((line) => (line.committed ? line : { hash: line.hash, committed: false }));
    await same(async (reads) => shown(await reads.blame(repo, 'a.txt', { contents: 'one\ntyped\n' })));
    await same((reads) => reads.blame(repo, 'a.txt', { revision: feature }));
    await same((reads) => reads.blame(repo, 'never.txt'));
    for (const line of [1, 2]) {
      const [engine, typescript] = await Promise.all([withEngine.blameLine(repo, 'a.txt', line), without.blameLine(repo, 'a.txt', line)]);
      // R4: the engine gives the version before for one line too, which Repo.blameLine leaves out.
      const [one] = shown(engine && [{ ...engine, previous: undefined }]) ?? [];
      expect(one).toEqual(shown(typescript && [{ ...typescript, previous: undefined }])?.[0]);
    }
    await same((reads) => reads.remotes(repo));
    await same((reads) => reads.reflog(repo, 'HEAD', 5));
    await same((reads) => reads.committedLines(repo, 'a.txt', { start: 2, end: 2 }, 'zero\none\ntwo\n'));
    await same((reads) => reads.committedLines(repo, 'a.txt', { start: 1, end: 1 }, 'new\none\ntwo\n'));
    await same((reads) => reads.committedLines(repo, 'never.txt', { start: 1, end: 1 }, 'x\n'));
    for (const [revision, file] of [
      [feature, 'c.txt'],
      [feature, 'b.txt'],
      [':0', 'staged.txt'],
      ['', 'a.txt'],
    ]) {
      await same((reads) => reads.show(repo, revision, file));
    }
    await same((reads) => reads.squashMessage(repo));
    await same((reads) => reads.commonDir(repo));
    expect(log.lines.filter((line) => line.startsWith('warn'))).toEqual([]);
  });

  it('read the same conflicts, their sides and the state that says what the rules resolve', async () => {
    const root = await conflicted();
    const { withEngine, without, repo, log } = await both(root);
    const same = async <T>(read: (reads: Reads) => Promise<T>) => expect(await read(withEngine)).toEqual(await read(without));
    await same((reads) => reads.parts(repo, true));
    await same((reads) => reads.conflicted(repo));
    for (const file of ['c.txt', 'gone.txt', 'a.txt']) await same((reads) => reads.conflictStages(repo, file));
    await same((reads) => reads.conflictSides(repo));
    await same((reads) => reads.conflictVersions(repo, 'c.txt'));
    for (const stage of [':1', ':2', ':3']) await same((reads) => reads.show(repo, stage, 'c.txt'));
    expect(log.lines.filter((line) => line.startsWith('warn'))).toEqual([]);
  });

  it('read the same submodules and bisect in the state', async () => {
    const inside = await repository();
    const root = await repository();
    await git(root, ['-c', 'protocol.file.allow=always', 'submodule', 'add', '--quiet', inside, 'libs/inside']);
    await git(root, ['commit', '--quiet', '-m', 'Add the submodule']);
    await commitFile(root, 'a.txt', 'two\n', 'Second');
    await commitFile(root, 'a.txt', 'three\n', 'Third');
    await git(root, ['bisect', 'start']);
    await git(root, ['bisect', 'bad']);
    await git(root, ['bisect', 'good', 'HEAD~2']);
    const { withEngine, without, repo } = await both(root);
    expect(await withEngine.parts(repo)).toEqual(await without.parts(repo));
  });

  it('read the same state before the first commit, and with a detached head and a worktree whose folder is gone', async () => {
    const empty = await repository(false);
    fs.writeFileSync(path.join(empty, 'new.txt'), 'new\n');
    const first = await both(empty);
    expect(await first.withEngine.parts(first.repo)).toEqual(await first.without.parts(first.repo));
    const root = await repository();
    await git(root, ['worktree', 'add', '--quiet', '-b', 'gone', path.join(path.dirname(root), 'gone')]);
    fs.rmSync(path.join(path.dirname(root), 'gone'), { recursive: true, force: true });
    await git(root, ['checkout', '--quiet', '--detach']);
    const second = await both(root);
    expect(await second.withEngine.parts(second.repo)).toEqual(await second.without.parts(second.repo));
  });

  it('read the same of what new worktrees get', async () => {
    const root = await repository();
    await commitFile(root, '.gitignore', 'node_modules/\n.env\n*.local\n', 'Ignore');
    await commitFile(root, 'package.json', '{ "packageManager": "pnpm@9.1.0" }\n', 'Add the package');
    await commitFile(root, 'pnpm-lock.yaml', 'lockfileVersion: 9\n', 'Lock');
    fs.mkdirSync(path.join(root, 'node_modules', 'left-pad'), { recursive: true });
    fs.writeFileSync(path.join(root, 'node_modules', 'left-pad', 'index.js'), 'module.exports = 1;\n');
    fs.writeFileSync(path.join(root, '.env'), 'KEY=1\n');
    fs.writeFileSync(path.join(root, 'settings.local'), 'x\n');
    const { withEngine, without } = await both(root);
    const same = async <T>(read: (reads: Reads) => Promise<T>) => expect(await read(withEngine)).toEqual(await read(without));
    await same((reads) => reads.setupIgnored(root, () => scanIgnored(root)));
    await same((reads) => reads.setupCommands(root, () => proposedCommands(root)));
    const patterns = { link: ['node_modules'], copy: ['.env', '*.local'] };
    await same((reads) => reads.setupPreview(root, patterns, () => previewSetup(root, patterns)));
  });

  it('read the pull requests in TypeScript when the engine has no token', async () => {
    const root = await repository();
    await git(root, ['remote', 'add', 'origin', 'git@github.com:owner/app.git']);
    const { withEngine } = await both(root);
    const pulls = { main: { number: 1, title: 'Fix', url: 'https://github.com/owner/app/pull/1', draft: false, checks: null } };
    expect(await withEngine.pullRequests(root, 'origin', async () => pulls)).toEqual(pulls);
  });

  it('read in TypeScript, and say so in the log, when the engine cannot be started', async () => {
    const root = await busy();
    const repo = new Repo((await Repo.discover(root))!);
    const log = memoryLog();
    const missing = new EngineClient({
      binary: path.join(folder(), 'no-such-engine'),
      clientVersion: '0.0.0-test',
      settings: () => ({}),
      logLevel: () => 'info',
      log: memoryLog(),
    });
    const reads = new Reads(() => missing, log);
    expect(await reads.parts(repo)).toEqual(await repo.parts());
    expect(await reads.resolve(repo, 'main')).toBe(await repo.resolve('main'));
    expect(log.lines.some((line) => line.startsWith('warn') && line.includes('could not read the state'))).toBe(true);
    expect(log.lines.some((line) => line.startsWith('debug Read the state in TypeScript'))).toBe(true);
  });
});

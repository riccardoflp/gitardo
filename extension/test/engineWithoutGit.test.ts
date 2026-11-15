/**
 * What version 1.7 of the protocol gives the extension once it runs no git of its own (stage 10 of
 * `docs/transition/06-roadmap.md`): the repositories of the workspace with the git folder each shares with its
 * worktrees, the file of a comparison as a target, and a change of a diff as a target, which the Flow Diff stages and
 * unstages.
 */
import * as fs from 'node:fs';
import * as path from 'node:path';
import { describe, expect, it } from 'vitest';
import type { ComparisonTarget, DiffChangeTarget } from '../src/engine/messages';
import { answering, built, cleanUpAfterEach, client, commitFile, folder, repository } from './engine/harness';
import { git } from './git';

cleanUpAfterEach();

/** A path as both sides can be compared by. */
const comparable = (named: string) => (process.platform === 'win32' ? path.resolve(named).toLowerCase() : path.resolve(named));

/** What git has of a file in the index, or null. */
async function inIndex(root: string, file: string): Promise<string | null> {
  return (await git(root, ['show', `:${file}`]).catch(() => null))?.stdout ?? null;
}

describe.skipIf(!built)('the engine without git in the extension', { timeout: 60_000 }, () => {
  it('finds the repositories of the folders of a workspace, each with the git folder its worktrees share', async () => {
    const workspace = folder();
    const app = path.join(workspace, 'app');
    const lib = path.join(workspace, 'lib');
    for (const each of [app, lib]) {
      fs.mkdirSync(each);
      await git(each, ['init', '--quiet', '--initial-branch=main']);
      await git(each, ['-c', 'user.name=Test', '-c', 'user.email=test@gitardo.invalid', 'commit', '--quiet', '--allow-empty', '-m', 'First']);
    }
    const worktree = path.join(folder(), 'feature');
    await git(app, ['worktree', 'add', '--quiet', '-b', 'feature', worktree]);

    const found = await client().discover([workspace, worktree]);

    const byFolder = new Map(found.map((each) => [comparable(each.folder), each]));
    expect([...byFolder.keys()].sort()).toEqual([app, lib, worktree].map(comparable).sort());
    const appGit = comparable(path.join(app, '.git'));
    expect(comparable(byFolder.get(comparable(app))!.commonFolder!)).toBe(appGit);
    expect(comparable(byFolder.get(comparable(worktree))!.commonFolder!)).toBe(appGit);
    expect(byFolder.get(comparable(lib))!.superproject).toBeNull();
  });

  it('takes a file of a comparison from its second side, and deletes the one the second side has not', async () => {
    const root = await repository();
    await commitFile(root, 'b.txt', 'b\n', 'Add b');
    await git(root, ['checkout', '--quiet', '-b', 'feature']);
    await commitFile(root, 'a.txt', 'feature\n', 'Change a');
    await git(root, ['rm', '--quiet', 'b.txt']);
    await git(root, ['commit', '--quiet', '-m', 'Delete b']);
    const feature = (await git(root, ['rev-parse', 'HEAD'])).stdout.trim();
    await git(root, ['checkout', '--quiet', 'main']);

    const comparison = (file: ComparisonTarget['file']): ComparisonTarget => ({
      kind: 'comparison',
      first: { name: 'Working tree', side: { kind: 'workingTree' }, isFollowed: true, isGone: false },
      second: { name: 'feature', side: { kind: 'commit', commit: feature }, isFollowed: true, isGone: false },
      mode: 'everything',
      path: null,
      mergeBase: null,
      file,
    });
    const engine = client();
    const asker = answering({});
    const taken = await engine.runFeature(
      { repository: root, feature: 'compare.take-file', target: comparison({ path: 'a.txt', originalPath: null, kind: 'modified' }) },
      asker,
    );
    expect(taken.finished.kind).toBe('ran');
    expect(fs.readFileSync(path.join(root, 'a.txt'), 'utf8')).toBe('feature\n');
    expect(await inIndex(root, 'a.txt')).toBe('feature\n');

    const deleted = await engine.runFeature(
      { repository: root, feature: 'compare.take-file', target: comparison({ path: 'b.txt', originalPath: null, kind: 'deleted' }) },
      asker,
    );
    expect(deleted.finished.kind).toBe('ran');
    expect(fs.existsSync(path.join(root, 'b.txt'))).toBe(false);
    expect(await inIndex(root, 'b.txt')).toBeNull();
  });

  it('stages one change of a diff of the working tree against HEAD, and unstages it from the diff of the index', async () => {
    const root = await repository(false);
    await commitFile(root, 'a.txt', 'one\ntwo\nthree\n', 'First commit');
    fs.writeFileSync(path.join(root, 'a.txt'), 'one\nTWO\nthree\nfour\n');
    const change = (right: DiffChangeTarget['right']): DiffChangeTarget => ({
      kind: 'diffChange',
      path: 'a.txt',
      left: { kind: 'head' },
      right,
      leftLines: { start: 1, end: 2 },
      rightLines: { start: 1, end: 2 },
      leftText: ['two'],
      rightText: ['TWO'],
    });
    const engine = client();

    const staged = await engine.runFeature({ repository: root, feature: 'diff.stage-change', target: change({ kind: 'workingTree' }) }, answering({}));
    expect(staged.finished.kind).toBe('ran');
    // Only that change: the line added after it stays in the working tree.
    expect(await inIndex(root, 'a.txt')).toBe('one\nTWO\nthree\n');

    const unstaged = await engine.runFeature({ repository: root, feature: 'diff.unstage-change', target: change({ kind: 'index' }) }, answering({}));
    expect(unstaged.finished.kind).toBe('ran');
    expect(await inIndex(root, 'a.txt')).toBe('one\ntwo\nthree\n');
    expect(fs.readFileSync(path.join(root, 'a.txt'), 'utf8')).toBe('one\nTWO\nthree\nfour\n');
  });

  it('refuses to stage a change the file no longer has where the diff showed it', async () => {
    const root = await repository(false);
    await commitFile(root, 'a.txt', 'one\ntwo\n', 'First commit');
    fs.writeFileSync(path.join(root, 'a.txt'), 'one\nchanged again\n');
    const target: DiffChangeTarget = {
      kind: 'diffChange',
      path: 'a.txt',
      left: { kind: 'head' },
      right: { kind: 'workingTree' },
      leftLines: { start: 1, end: 2 },
      rightLines: { start: 1, end: 2 },
      leftText: ['two'],
      rightText: ['TWO'],
    };
    const { finished } = await client().runFeature({ repository: root, feature: 'diff.stage-change', target }, answering({}));
    expect(finished.kind).toBe('refused');
    expect(await inIndex(root, 'a.txt')).toBe('one\ntwo\n');
  });
});

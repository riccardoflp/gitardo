/**
 * What the folders of the rows of the Repositories view are on, read through Gitardo's engine (`repo.worktree-summary`,
 * milestone M22): `read/repo.worktree-summary` follows the repositories of a workspace and answers what each of them
 * and each of their worktrees is on, `repository/folderChanged` has one read again, and `repository/summaries` says
 * what that read found.
 */
import * as fs from 'node:fs';
import * as path from 'node:path';
import { describe, expect, it } from 'vitest';
import type { EngineClient } from '../src/engine/client';
import { samePath } from '../src/shared/paths';
import { type FolderSummary, summaryWords } from '../src/shared/summaries';
import { git } from './git';
import { built, cleanUpAfterEach, client, commitFile, folder, repository } from './engine/harness';

cleanUpAfterEach();

/** What an answer or a notification says of a folder. */
function saidOf(summaries: FolderSummary[], wanted: string): FolderSummary {
  const said = summaries.find((each) => samePath(each.folder, wanted));
  if (!said) throw new Error(`${wanted} is not among ${summaries.map((each) => each.folder).join(', ')}`);
  return said;
}

/** The next summaries the engine says of a folder, or a failure after `within` milliseconds. */
function nextOf(engine: EngineClient, wanted: string, within = 5000): Promise<FolderSummary> {
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => {
      listening.dispose();
      reject(new Error(`The engine said nothing of ${wanted} in ${within} ms.`));
    }, within);
    const listening = engine.onSummaries((summaries) => {
      const said = summaries.find((each) => samePath(each.folder, wanted));
      if (!said) return;
      clearTimeout(timer);
      listening.dispose();
      resolve(said);
    });
  });
}

/** A repository with a branch that is one commit ahead of the remote it follows, and a worktree of it beside it. */
async function withWorktree(): Promise<{ root: string; worktree: string }> {
  const root = await repository();
  const remote = path.join(folder(), 'remote.git');
  await git(root, ['init', '--bare', '--quiet', remote]);
  await git(root, ['remote', 'add', 'origin', remote]);
  await git(root, ['push', '--quiet', '--set-upstream', 'origin', 'main']);
  await commitFile(root, 'b.txt', 'two\n', 'Second commit');
  const worktree = path.join(path.dirname(root), 'repo-login');
  await git(root, ['worktree', 'add', '--quiet', '-b', 'topic/login', worktree]);
  return { root, worktree };
}

describe.skipIf(!built)('what the folders of the Repositories view are on, through the engine', () => {
  it('answers what each repository asked and each of its worktrees is on', async () => {
    const { root, worktree } = await withWorktree();
    fs.writeFileSync(path.join(root, 'a.txt'), 'changed\n');
    fs.writeFileSync(path.join(root, 'new.txt'), 'new\n');
    const other = await repository(false);
    const engine = client();
    const summaries = await engine.summaries([root, other]);

    expect(summaries).toHaveLength(3);
    const main = saidOf(summaries, root);
    expect(main.summary).toEqual({ head: { kind: 'branch', name: 'main' }, changed: 2, ahead: 1, behind: 0 });
    expect(main.isStale).toBe(false);
    expect(main.worktrees?.map((each) => each.branch)).toEqual(['refs/heads/main', 'refs/heads/topic/login']);
    expect(summaryWords(main.summary!)).toEqual({ head: 'main', changed: '● 2', distance: '↑1' });
    const listed = saidOf(summaries, worktree);
    expect(listed.summary).toEqual({ head: { kind: 'branch', name: 'topic/login' }, changed: 0, ahead: null, behind: null });
    expect(listed.worktrees).toBeNull();
    expect(saidOf(summaries, other).summary?.head).toEqual({ kind: 'noCommits' });
  });

  it('says a worktree again once a file of it changed, and a worktree added once its repository lists it', async () => {
    const { root, worktree } = await withWorktree();
    const engine = client();
    await engine.summaries([root]);

    fs.writeFileSync(path.join(worktree, 'a.txt'), 'changed in the worktree\n');
    const changed = nextOf(engine, worktree);
    engine.folderChanged(worktree, ['a.txt']);
    expect((await changed).summary?.changed).toBe(1);

    const added = path.join(path.dirname(root), 'repo-fix');
    await git(root, ['worktree', 'add', '--quiet', '--detach', added]);
    const read = nextOf(engine, added);
    const relisted = nextOf(engine, root);
    engine.folderChanged(root, ['.git/worktrees/repo-fix/HEAD']);
    expect((await relisted).worktrees).toHaveLength(3);
    expect((await read).summary?.head.kind).toBe('detached');
  });

  it('follows the folders again after the engine was ended and started again', async () => {
    const { root, worktree } = await withWorktree();
    const engine = client();
    await engine.summaries([root]);
    const again = nextOf(engine, worktree, 15000);
    process.kill(engine.pid!);
    expect((await again).summary?.head).toEqual({ kind: 'branch', name: 'topic/login' });
  }, 20000);
});

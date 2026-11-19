/**
 * The scenarios of the first area moved to Gitardo's engine in stage 4 of `docs/transition/06-roadmap.md`: changes and
 * commit, and stashes. Each operation runs through the engine's feature, with the target and answers `engineRuns`
 * gives it, and must run the commands it ran when it was a `GitOp` of the extension, but where
 * `docs/transition/04-differences.md` says why not; what the feature leaves is the engine's tests' to check.
 */
import * as fs from 'node:fs';
import * as path from 'node:path';
import { describe, expect, it } from 'vitest';
import { engineRuns } from '../src/engine/actions';
import type { Answer, Question } from '../src/engine/messages';
import type { Action } from '../src/shared/protocol';
import { git } from './git';
import { built, choice, cleanUpAfterEach, confirmed, engineState, repository, runFeature, text } from './engine/harness';

cleanUpAfterEach();

const NO_REFS = { branches: [], remoteBranches: [], tags: [] };

/** Files changed in three ways: `a.txt` modified, `b.txt` staged, `c.txt` modified and partly staged, `new.txt` untracked. */
async function changed(root: string): Promise<void> {
  fs.writeFileSync(path.join(root, 'b.txt'), 'b\n');
  fs.writeFileSync(path.join(root, 'c.txt'), 'c\n');
  await git(root, ['add', 'b.txt', 'c.txt']);
  await git(root, ['commit', '--quiet', '-m', 'Add b and c']);
  fs.writeFileSync(path.join(root, 'a.txt'), 'changed\n');
  fs.writeFileSync(path.join(root, 'b.txt'), 'b staged\n');
  await git(root, ['add', 'b.txt']);
  fs.writeFileSync(path.join(root, 'c.txt'), 'c staged\n');
  await git(root, ['add', 'c.txt']);
  fs.writeFileSync(path.join(root, 'c.txt'), 'c on disk\n');
  fs.writeFileSync(path.join(root, 'new.txt'), 'new\n');
}

/** A merge of `other` waiting to be committed, with no conflict, and `a.txt` changed and staged too. */
async function merging(root: string): Promise<void> {
  await git(root, ['checkout', '--quiet', '-b', 'other']);
  fs.writeFileSync(path.join(root, 'b.txt'), 'other\n');
  await git(root, ['add', 'b.txt']);
  await git(root, ['commit', '--quiet', '-m', 'Other']);
  await git(root, ['checkout', '--quiet', 'main']);
  await git(root, ['merge', '--no-commit', '--no-ff', 'other']);
  fs.writeFileSync(path.join(root, 'a.txt'), 'changed\n');
  await git(root, ['add', 'a.txt']);
}

/**
 * Runs a scenario: `action` through the engine in a repository `setup` made, its questions answered by key; gives
 * the commands it ran and the repository.
 */
async function scenario(
  setup: (root: string) => Promise<void>,
  action: Action,
  answers: Record<string, (question: Question) => Answer | null> = {},
): Promise<{ engine: string[]; root: string }> {
  const root = await repository();
  await setup(root);
  const runs = engineRuns(action, NO_REFS);
  if (!runs) throw new Error(`${action.kind} does not run in the engine`);
  const engine: string[] = [];
  for (const run of runs) {
    const { finished, lines } = await runFeature(root, run.feature, run.target, {
      ...Object.fromEntries(Object.entries(run.answers ?? {}).map(([key, answer]) => [key, () => answer])),
      ...answers,
    });
    expect(finished.kind, JSON.stringify(finished)).toBe('ran');
    engine.push(...lines);
  }
  return { engine, root };
}

describe.skipIf(!built)('Changes and commit, through the engine', { timeout: 60_000 }, () => {
  it('stages a file, and the files ticked by one command', async () => {
    expect((await scenario(changed, { kind: 'stage', paths: ['a.txt'] })).engine).toEqual(['git add -- a.txt']);
    const both = await scenario(changed, { kind: 'stage', paths: ['a.txt', 'new.txt'] });
    expect(both.engine).toEqual(['git add -- a.txt new.txt']);
  });

  it('unstages a file, and the files ticked', async () => {
    const one = await scenario(changed, { kind: 'unstage', paths: ['b.txt'] });
    expect(one.engine).toEqual(['git restore --staged -- b.txt']);
    const both = await scenario(changed, { kind: 'unstage', paths: ['b.txt', 'c.txt'] });
    expect(both.engine).toEqual(['git restore --staged -- b.txt c.txt']);
  });

  it('stages all and unstages all, and while a merge waits names the staged files rather than end it with git reset (A1, A2)', async () => {
    const stage = await scenario(changed, { kind: 'stageAll' });
    expect(stage.engine).toEqual(['git add --all']);
    const unstage = await scenario(changed, { kind: 'unstageAll' });
    expect(unstage.engine).toEqual(['git reset']);

    const root = await repository();
    await merging(root);
    const { lines } = await runFeature(root, 'changes.unstage-all', { kind: 'changes' });
    expect(lines).toEqual(['git restore --staged -- a.txt b.txt']);
    expect(fs.existsSync(path.join(root, '.git', 'MERGE_HEAD'))).toBe(true);
  });

  it('discards a file with nothing staged, the files ticked with their staged changes, and all changes', async () => {
    const one = await scenario(changed, { kind: 'discard', paths: ['a.txt'] });
    expect(one.engine).toEqual(['git restore -- a.txt']);
    const ticked = await scenario(changed, { kind: 'discardChanges', paths: ['a.txt', 'c.txt', 'new.txt'] });
    expect((await git(ticked.root, ['status', '--porcelain=v1', '--untracked-files=all'])).stdout).toBe('M  b.txt\n');
    const all = await scenario(changed, { kind: 'discardChanges' });
    expect((await git(all.root, ['status', '--porcelain=v1', '--untracked-files=all'])).stdout).toBe('');
  });

  it('commits what is staged, asks to stage all when nothing is, and amends', async () => {
    const staged = await scenario(changed, { kind: 'commit', message: 'Change b and c', amend: false });
    expect(staged.engine).toEqual(['git commit -m "Change b and c"']);
    const modified = async (root: string) => fs.writeFileSync(path.join(root, 'a.txt'), 'changed\n');
    const all = await scenario(modified, { kind: 'commit', message: 'Change a', amend: false }, { 'stage-all': confirmed() });
    expect(all.engine).toEqual(['git add --all', 'git commit -m "Change a"']);
    const amend = await scenario(changed, { kind: 'commit', message: 'Better words', amend: true });
    expect(amend.engine).toEqual(['git commit --amend -m "Better words"']);
  });
});

describe.skipIf(!built)('Stashes, through the engine', { timeout: 60_000 }, () => {
  /** Two stashes on top of the changes. */
  const stashed = async (root: string) => {
    await changed(root);
    await git(root, ['stash', 'push', '--include-untracked', '-m', 'first']);
    fs.writeFileSync(path.join(root, 'a.txt'), 'second\n');
    await git(root, ['stash', 'push', '-m', 'second']);
  };

  it('stashes all changes, the staged ones, and a file, each with its message', async () => {
    const message = { message: text('wip') };
    const all = await scenario(changed, { kind: 'stashPush' }, message);
    expect(all.engine).toEqual(['git stash push --include-untracked -m wip']);
    // Git stashes the staged changes only of files that have no other change.
    const stagedOnly = async (root: string) => {
      fs.writeFileSync(path.join(root, 'a.txt'), 'staged\n');
      await git(root, ['add', 'a.txt']);
      fs.writeFileSync(path.join(root, 'new.txt'), 'new\n');
    };
    const staged = await scenario(stagedOnly, { kind: 'stashPush', staged: true }, message);
    expect(staged.engine).toEqual(['git stash push --staged -m wip']);
    const file = await scenario(changed, { kind: 'stashPush', paths: ['a.txt'] }, message);
    expect(file.engine).toEqual(['git stash push --include-untracked -m wip -- a.txt']);
  });

  it('applies, pops and drops a stash, and creates a branch from one', async () => {
    for (const verb of ['apply', 'pop', 'drop'] as const) {
      const kind = ({ apply: 'stashApply', pop: 'stashPop', drop: 'stashDrop' } as const)[verb];
      const ran = await scenario(stashed, { kind, ref: 'stash@{1}' });
      expect(ran.engine).toEqual([`git stash ${verb} stash@{1}`]);
    }
    const branch = await scenario(stashed, { kind: 'stashBranch', ref: 'stash@{0}' }, { name: text('from-stash') });
    expect(branch.engine).toEqual(['git stash branch from-stash stash@{0}']);
  });
});

describe.skipIf(!built)('What git leaves alone, through the engine', { timeout: 60_000 }, () => {
  /** `debug.log` untracked, and `config/local.json` committed then changed. */
  const leftAlone = async (root: string) => {
    fs.mkdirSync(path.join(root, 'config'));
    fs.writeFileSync(path.join(root, 'config', 'local.json'), '{}\n');
    await git(root, ['add', 'config/local.json']);
    await git(root, ['commit', '--quiet', '-m', 'Add the local settings']);
    fs.writeFileSync(path.join(root, 'config', 'local.json'), '{ "port": 8080 }\n');
    fs.writeFileSync(path.join(root, 'debug.log'), 'log\n');
  };
  const status = async (root: string) => (await git(root, ['status', '--porcelain=v1', '--untracked-files=all'])).stdout;

  it('changes.ignore: All .log files writes *.log at the end of the .gitignore, and debug.log leaves the changes', async () => {
    const { engine, root } = await scenario(leftAlone, { kind: 'ignore', path: 'debug.log' }, { 'ignore-what': choice('1'), 'ignore-where': choice('0') });
    expect(engine).toHaveLength(1);
    expect(engine[0]).toMatch(/^write .*\.gitignore$/);
    expect(fs.readFileSync(path.join(root, '.gitignore'), 'utf8')).toBe('*.log\n');
    expect(await status(root)).toBe(' M config/local.json\n?? .gitignore\n');
  });

  it('changes.ignore and changes.stop-tracking: a tracked file is ignored and taken out of the index, kept on disk', async () => {
    const answers = { 'ignore-what': choice('0'), 'ignore-where': choice('0'), 'ignore-tracked': choice('stop-tracking') };
    const ignored = await scenario(leftAlone, { kind: 'ignore', path: 'config/local.json' }, answers);
    expect(ignored.engine.slice(1)).toEqual(['git rm --cached -- config/local.json']);
    expect(fs.readFileSync(path.join(ignored.root, '.gitignore'), 'utf8')).toBe('/config/local.json\n');
    expect(fs.existsSync(path.join(ignored.root, 'config', 'local.json'))).toBe(true);
    const stopped = await scenario(leftAlone, { kind: 'stopTracking', path: 'config/local.json' });
    expect(stopped.engine).toEqual(['git rm --cached -- config/local.json']);
    expect(await status(stopped.root)).toBe('D  config/local.json\n?? config/local.json\n?? debug.log\n');
  });

  it('changes.skip and changes.unskip: hides the changes of a tracked file, lists it as skipped, and shows it to git again', async () => {
    const hidden = await scenario(leftAlone, { kind: 'hideFromGit', path: 'config/local.json' }, { 'skip-how': choice('skip-worktree') });
    expect(hidden.engine).toEqual(['git update-index --skip-worktree -- config/local.json']);
    expect(await status(hidden.root)).toBe('?? debug.log\n');
    expect((await engineState(hidden.root)).skipped).toEqual([{ path: 'config/local.json', skipWorktree: true, assumeUnchanged: false }]);
    const [one] = engineRuns({ kind: 'showToGit', path: 'config/local.json' }, NO_REFS) ?? [];
    expect((await runFeature(hidden.root, one.feature, one.target)).lines).toEqual(['git update-index --no-skip-worktree -- config/local.json']);
    expect(await status(hidden.root)).toBe(' M config/local.json\n?? debug.log\n');
    await git(hidden.root, ['update-index', '--assume-unchanged', '--', 'config/local.json']);
    const [all] = engineRuns({ kind: 'showToGit' }, NO_REFS) ?? [];
    expect(all.target).toEqual({ kind: 'changes' });
    expect((await runFeature(hidden.root, all.feature, all.target)).lines).toEqual(['git update-index --no-assume-unchanged -- config/local.json']);
  });
});

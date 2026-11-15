/**
 * The scenarios of the first area moved to Gitardo's engine in stage 4 of `docs/transition/06-roadmap.md`: changes and
 * commit, and stashes. Each operation runs as the `GitOp` the extension ran until now in one copy of a repository and as
 * the engine's feature, with the target and answers `engineRuns` gives it, in the other; the two repositories after
 * must be the same, and so must the commands, but where `docs/transition/04-differences.md` says why not.
 */
import * as fs from 'node:fs';
import * as path from 'node:path';
import { describe, expect, it } from 'vitest';
import { engineRuns } from '../src/engine/actions';
import type { Answer, Question } from '../src/engine/messages';
import {
  commitOp,
  discardChangesOp,
  discardOp,
  type GitOp,
  stageAllOp,
  stageOp,
  stashBranchOp,
  stashOp,
  stashPushOp,
  unstageAllOp,
  unstageOp,
} from '../src/git/ops';
import { Repo } from '../src/git/repo';
import type { Action, RepoState } from '../src/shared/protocol';
import { built, cleanUpAfterEach, confirmed, runFeature, runGitOp, snapshot, text, twins } from './engine/harness';

cleanUpAfterEach();

const NO_REFS = { branches: [], remoteBranches: [], tags: [] };

/** Files changed in three ways: `a.txt` modified, `b.txt` staged, `c.txt` modified and partly staged, `new.txt` untracked. */
async function changed(root: string): Promise<void> {
  fs.writeFileSync(path.join(root, 'b.txt'), 'b\n');
  fs.writeFileSync(path.join(root, 'c.txt'), 'c\n');
  const { git } = await import('../src/git/exec');
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
  const { git } = await import('../src/git/exec');
  await git(root, ['checkout', '--quiet', '-b', 'other']);
  fs.writeFileSync(path.join(root, 'b.txt'), 'other\n');
  await git(root, ['add', 'b.txt']);
  await git(root, ['commit', '--quiet', '-m', 'Other']);
  await git(root, ['checkout', '--quiet', 'main']);
  await git(root, ['merge', '--no-commit', '--no-ff', 'other']);
  fs.writeFileSync(path.join(root, 'a.txt'), 'changed\n');
  await git(root, ['add', 'a.txt']);
}

/** What the extension reads of a repository, for the `GitOp`s that need it. */
async function state(root: string): Promise<RepoState> {
  return new Repo((await Repo.discover(root))!).state(10);
}

/**
 * Runs a scenario: `op` as a `GitOp` in one copy, `action` through the engine in the other, its questions answered by
 * key; gives the commands of each, and checks that the two repositories after are the same.
 */
async function scenario(
  setup: (root: string) => Promise<void>,
  op: (state: RepoState) => GitOp,
  action: Action,
  answers: Record<string, (question: Question) => Answer | null> = {},
): Promise<{ typescript: string[]; engine: string[] }> {
  const [one, other] = await twins(setup);
  const typescript = await runGitOp(one, op(await state(one)));
  const runs = engineRuns(action, NO_REFS);
  if (!runs) throw new Error(`${action.kind} does not run in the engine`);
  const engine: string[] = [];
  for (const run of runs) {
    const { finished, lines } = await runFeature(other, run.feature, run.target, {
      ...Object.fromEntries(Object.entries(run.answers ?? {}).map(([key, answer]) => [key, () => answer])),
      ...answers,
    });
    expect(finished.kind, JSON.stringify(finished)).toBe('ran');
    engine.push(...lines);
  }
  expect(await snapshot(other)).toEqual(await snapshot(one));
  return { typescript, engine };
}

describe.skipIf(!built)('Changes and commit, through the engine and as GitOps', { timeout: 60_000 }, () => {
  it('stages a file, and the files ticked by one command', async () => {
    expect(await scenario(changed, () => stageOp(['a.txt']), { kind: 'stage', paths: ['a.txt'] })).toEqual({
      typescript: ['git add -- a.txt'],
      engine: ['git add -- a.txt'],
    });
    const both = await scenario(changed, () => stageOp(['a.txt', 'new.txt']), { kind: 'stage', paths: ['a.txt', 'new.txt'] });
    expect(both.engine).toEqual(both.typescript);
  });

  it('unstages a file, and the files ticked', async () => {
    const one = await scenario(changed, () => unstageOp(['b.txt'], true), { kind: 'unstage', paths: ['b.txt'] });
    expect(one.engine).toEqual(one.typescript);
    const both = await scenario(changed, () => unstageOp(['b.txt', 'c.txt'], true), { kind: 'unstage', paths: ['b.txt', 'c.txt'] });
    expect(both.engine).toEqual(both.typescript);
  });

  it('stages all and unstages all, and while a merge waits names the staged files rather than end it with git reset (A1, A2)', async () => {
    const stage = await scenario(changed, () => stageAllOp(), { kind: 'stageAll' });
    expect(stage.engine).toEqual(stage.typescript);
    const unstage = await scenario(changed, () => unstageAllOp(true), { kind: 'unstageAll' });
    expect(unstage.engine).toEqual(['git reset']);
    expect(unstage.engine).toEqual(unstage.typescript);

    const [root] = await twins(merging);
    const { lines } = await runFeature(root, 'changes.unstage-all', { kind: 'changes' });
    expect(lines).toEqual(['git restore --staged -- a.txt b.txt']);
    expect(fs.existsSync(path.join(root, '.git', 'MERGE_HEAD'))).toBe(true);
  });

  it('discards a file with nothing staged, the files ticked with their staged changes, and all changes', async () => {
    const one = await scenario(changed, (read) => discardOp(['a.txt'], read.working), { kind: 'discard', paths: ['a.txt'] });
    expect(one.engine).toEqual(one.typescript);
    await scenario(changed, (read) => discardChangesOp(['a.txt', 'c.txt', 'new.txt'], read.working, true), {
      kind: 'discardChanges',
      paths: ['a.txt', 'c.txt', 'new.txt'],
    });
    await scenario(changed, (read) => discardChangesOp(null, read.working, true), { kind: 'discardChanges' });
  });

  it('commits what is staged, asks to stage all when nothing is, and amends', async () => {
    const staged = await scenario(changed, () => commitOp('Change b and c', false, false), { kind: 'commit', message: 'Change b and c', amend: false });
    expect(staged.engine).toEqual(staged.typescript);
    const modified = async (root: string) => fs.writeFileSync(path.join(root, 'a.txt'), 'changed\n');
    const all = await scenario(modified, () => commitOp('Change a', false, true), { kind: 'commit', message: 'Change a', amend: false }, { 'stage-all': confirmed() });
    expect(all.engine).toEqual(all.typescript);
    const amend = await scenario(changed, () => commitOp('Better words', true, false), { kind: 'commit', message: 'Better words', amend: true });
    expect(amend.engine).toEqual(amend.typescript);
  });
});

describe.skipIf(!built)('Stashes, through the engine and as GitOps', { timeout: 60_000 }, () => {
  /** Two stashes on top of the changes. */
  const stashed = async (root: string) => {
    await changed(root);
    const { git } = await import('../src/git/exec');
    await git(root, ['stash', 'push', '--include-untracked', '-m', 'first']);
    fs.writeFileSync(path.join(root, 'a.txt'), 'second\n');
    await git(root, ['stash', 'push', '-m', 'second']);
  };

  it('stashes all changes, the staged ones, and a file, each with its message', async () => {
    const message = { message: text('wip') };
    const all = await scenario(changed, () => stashPushOp('wip'), { kind: 'stashPush' }, message);
    expect(all.engine).toEqual(all.typescript);
    // Git stashes the staged changes only of files that have no other change.
    const stagedOnly = async (root: string) => {
      fs.writeFileSync(path.join(root, 'a.txt'), 'staged\n');
      const { git } = await import('../src/git/exec');
      await git(root, ['add', 'a.txt']);
      fs.writeFileSync(path.join(root, 'new.txt'), 'new\n');
    };
    const staged = await scenario(stagedOnly, () => stashPushOp('wip', { staged: true }), { kind: 'stashPush', staged: true }, message);
    expect(staged.engine).toEqual(staged.typescript);
    const file = await scenario(changed, () => stashPushOp('wip', { paths: ['a.txt'] }), { kind: 'stashPush', paths: ['a.txt'] }, message);
    expect(file.engine).toEqual(file.typescript);
  });

  it('applies, pops and drops a stash, and creates a branch from one', async () => {
    for (const verb of ['apply', 'pop', 'drop'] as const) {
      const kind = ({ apply: 'stashApply', pop: 'stashPop', drop: 'stashDrop' } as const)[verb];
      const ran = await scenario(stashed, () => stashOp(verb, 'stash@{1}'), { kind, ref: 'stash@{1}' });
      expect(ran.engine).toEqual(ran.typescript);
    }
    const branch = await scenario(stashed, () => stashBranchOp('from-stash', 'stash@{0}'), { kind: 'stashBranch', ref: 'stash@{0}' }, { name: text('from-stash') });
    expect(branch.engine).toEqual(branch.typescript);
  });
});

/**
 * The operations of milestone M35 that the extension runs on Gitardo's engine: the upstream of a branch set and taken
 * away, a tag deleted from a remote, and a branch with no history. Each action of the views is given to `engineRuns`,
 * and what it gives is run through the engine on a repository made by the test, with a bare `origin` beside it.
 */
import * as fs from 'node:fs';
import * as path from 'node:path';
import { describe, expect, it } from 'vitest';
import { engineRuns } from '../src/engine/actions';
import type { Answer, Question } from '../src/engine/messages';
import type { Action } from '../src/shared/protocol';
import { git } from './git';
import { answering, built, choice, cleanUpAfterEach, client, engineState, repository, text } from './engine/harness';

cleanUpAfterEach();

/** A repository on `main` whose `origin`, a bare repository beside it, has `main`, `work` and the tag `v1`; `work` follows nothing. */
async function withOrigin(): Promise<string> {
  const root = await repository();
  const origin = path.join(path.dirname(root), 'origin.git');
  await git(path.dirname(root), ['init', '--quiet', '--bare', '--initial-branch=main', origin]);
  await git(root, ['remote', 'add', 'origin', '../origin.git']);
  await git(root, ['branch', 'work']);
  await git(root, ['tag', 'v1']);
  await git(root, ['push', '--quiet', 'origin', 'main', 'work', 'refs/tags/v1']);
  return root;
}

/** Runs what the action gives through the engine, answering by key, and gives the lines of the steps. */
async function run(root: string, action: Action, answers: Record<string, (question: Question) => Answer | null> = {}): Promise<string[]> {
  const runs = engineRuns(action, await engineState(root));
  if (!runs) throw new Error('This does not run in the engine');
  const lines: string[] = [];
  for (const { feature, target, answers: given } of runs) {
    const { finished } = await client().runFeature({ repository: root, feature, target, answers: given }, answering(answers));
    expect(finished.kind, JSON.stringify(finished)).toBe('ran');
    if (finished.kind === 'ran') lines.push(...finished.plan.steps.map((step) => step.line));
  }
  return lines;
}

describe.skipIf(!built)('Upstreams, tags on remotes and branches with no history, through the engine', { timeout: 120_000 }, () => {
  it('sets the upstream of work to origin/work, then stops tracking it', async () => {
    const root = await withOrigin();
    expect(await run(root, { kind: 'setUpstream', name: 'work' }, { upstream: choice('origin/work') })).toEqual([
      'git branch --set-upstream-to=origin/work work',
    ]);
    const work = (await engineState(root)).branches.find((branch) => branch.name === 'work');
    expect(work?.upstream).toMatchObject({ remote: 'origin', ahead: 0, behind: 0 });
    expect(await run(root, { kind: 'unsetUpstream', name: 'work' })).toEqual(['git branch --unset-upstream work']);
    expect((await engineState(root)).branches.find((branch) => branch.name === 'work')?.upstream ?? null).toBeNull();
  });

  it('deletes v1 from origin, and here and from origin, after the warning', async () => {
    const root = await withOrigin();
    expect(await run(root, { kind: 'deleteTagOnRemote', name: 'v1', here: false })).toEqual(['git push origin --delete refs/tags/v1']);
    expect((await git(root, ['tag', '--list'])).stdout).toBe('v1\n');
    await git(root, ['push', '--quiet', 'origin', 'refs/tags/v1']);
    expect(await run(root, { kind: 'deleteTagOnRemote', name: 'v1', here: true })).toEqual(['git push origin --delete refs/tags/v1', 'git tag -d v1']);
    expect((await git(root, ['tag', '--list'])).stdout).toBe('');
  });

  it('creates site with no history, starting with no files', async () => {
    const root = await withOrigin();
    fs.writeFileSync(path.join(root, 'notes.txt'), 'kept\n');
    expect(await run(root, { kind: 'createBranchWithNoHistory' }, { name: text('site'), keeps: choice('no-files') })).toEqual(['git switch --orphan site']);
    expect(fs.existsSync(path.join(root, 'a.txt'))).toBe(false);
    expect(fs.existsSync(path.join(root, 'notes.txt'))).toBe(true);
  });
});

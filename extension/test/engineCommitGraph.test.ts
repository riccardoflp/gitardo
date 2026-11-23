/**
 * Git's commit-graph file through the engine (milestone M44): the state says whether the repository has one
 * (`upkeep.read-commit-graph`, protocol 1.12), and **Speed up reading the history**, mapped by `engineRuns`, writes it
 * (`upkeep.write-commit-graph`), on throwaway repositories.
 */

import * as fs from 'node:fs';
import * as path from 'node:path';
import { describe, expect, it } from 'vitest';
import { engineRuns } from '../src/engine/actions';
import type { Finished } from '../src/engine/messages';
import { git } from './git';
import { built, cleanUpAfterEach, client, engineState, repository } from './engine/harness';

cleanUpAfterEach();

/** Runs **Speed up reading the history** through the engine, as the row menu and the notification ask it. */
async function writeCommitGraph(root: string): Promise<Finished> {
  const [run] = engineRuns({ kind: 'writeCommitGraph' }, await engineState(root)) ?? [];
  if (!run) throw new Error('writeCommitGraph does not run in the engine');
  const asker = { ask: async () => null, confirm: async () => true };
  return (await client().runFeature({ repository: root, feature: run.feature, target: run.target, answers: run.answers }, asker)).finished;
}

describe.skipIf(!built)('The commit-graph file, through the engine', { timeout: 60_000 }, () => {
  it('upkeep.read-commit-graph and upkeep.write-commit-graph: the state has no file, the operation writes one, and is then refused', async () => {
    const root = await repository();
    expect((await engineState(root)).commitGraph).toEqual({ hasFile: false, isIgnored: false });
    expect((await writeCommitGraph(root)).kind).toBe('ran');
    expect(fs.existsSync(path.join(root, '.git', 'objects', 'info', 'commit-graph'))).toBe(true);
    expect((await engineState(root)).commitGraph).toEqual({ hasFile: true, isIgnored: false });
    expect((await writeCommitGraph(root)).kind).toBe('refused');
  });

  it('upkeep.read-commit-graph: with core.commitGraph false, git does not read the file, and the state says so', async () => {
    const root = await repository();
    await git(root, ['config', 'core.commitGraph', 'false']);
    expect((await engineState(root)).commitGraph).toEqual({ hasFile: false, isIgnored: true });
  });
});

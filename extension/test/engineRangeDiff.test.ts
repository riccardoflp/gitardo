/**
 * Milestone M36 as the extension runs it on Gitardo's engine: a stack of branches rebased with `--update-refs` and taken
 * back by one Undo, and what a rewrite changed, from **What its last rewrite changed** to the pairs of its tab.
 */
import { describe, expect, it } from 'vitest';
import { engineRuns } from '../src/engine/actions';
import type { FollowUp } from '../src/engine/messages';
import type { RangeDiffTarget } from '../src/shared/rangeDiff';
import { git } from './git';
import { answering, built, cleanUpAfterEach, client, commitFile, engineState, repository } from './engine/harness';

cleanUpAfterEach();

/** `main` one commit further than where `feature` started; `feature` with a commit, `top` one more above it, checked out. */
async function stack(): Promise<string> {
  const root = await repository();
  await git(root, ['switch', '--quiet', '-c', 'feature']);
  await commitFile(root, 'b.txt', 'b\n', 'Add b');
  await git(root, ['switch', '--quiet', '-c', 'top']);
  await commitFile(root, 'c.txt', 'c\n', 'Add c');
  await git(root, ['switch', '--quiet', 'main']);
  await commitFile(root, 'd.txt', 'd\n', 'Add d');
  await git(root, ['switch', '--quiet', 'top']);
  return root;
}

const commitOf = async (root: string, ref: string) => (await git(root, ['rev-parse', ref])).stdout.trim();

describe.skipIf(!built)('What a rewrite changed, through the engine', { timeout: 120_000 }, () => {
  it('rebases a stack onto main with feature moved too, and one Undo puts both back', async () => {
    const root = await stack();
    const before = { feature: await commitOf(root, 'feature'), top: await commitOf(root, 'top') };
    const engine = client();
    const { finished } = await engine.runFeature(
      { repository: root, feature: 'rebase.onto', target: { kind: 'branch', name: 'main' }, answers: {} },
      answering({}),
    );
    expect(finished.kind, JSON.stringify(finished)).toBe('ran');
    expect(await commitOf(root, 'feature^')).toBe(await commitOf(root, 'main'));
    expect(await commitOf(root, 'top^')).toBe(await commitOf(root, 'feature'));
    const { finished: undone } = await engine.runFeature({ repository: root, feature: 'undo.last', target: { kind: 'head' }, answers: {} }, answering({}));
    expect(undone.kind, JSON.stringify(undone)).toBe('ran');
    expect({ feature: await commitOf(root, 'feature'), top: await commitOf(root, 'top') }).toEqual(before);
  });

  it('opens the tab of what the rebase of top changed with What its last rewrite changed, and reads its pairs', async () => {
    const root = await stack();
    await git(root, ['rebase', '--quiet', 'main']);
    const runs = engineRuns({ kind: 'rangeDiff', target: 'branch', name: 'top' }, await engineState(root));
    expect(runs).toHaveLength(1);
    const [{ feature, target, answers }] = runs!;
    const engine = client();
    const { finished, followUps } = await engine.runFeature({ repository: root, feature, target, answers }, answering({}));
    expect(finished.kind, JSON.stringify(finished)).toBe('ran');
    const opens = followUps.find((followUp: FollowUp) => followUp.kind === 'openRangeDiff') as (FollowUp & RangeDiffTarget) | undefined;
    expect(opens?.title).toBe('What the rebase of top changed');
    const pairs = await engine.rangeDiffPairs(root, opens!.ranges);
    expect(pairs.pairs.map((pair) => [pair.kind, pair.subject])).toEqual([
      ['same', 'Add b'],
      ['same', 'Add c'],
    ]);
    expect(pairs.counts).toBe('2 the same');
  });
});

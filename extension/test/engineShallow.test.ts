/**
 * The history of a shallow clone, through the engine (milestone M46): where it ends, in the state
 * (`repo.read-shallow`), and more of it fetched as **Load more history** and **Load the whole history** ask, from the
 * last row of the graph and the Command Palette (`sync.deepen`, `sync.unshallow`), on throwaway repositories.
 */

import * as path from 'node:path';
import { describe, expect, it } from 'vitest';
import { engineRuns } from '../src/engine/actions';
import type { Answer } from '../src/engine/messages';
import { git } from './git';
import { built, cleanUpAfterEach, commitFile, engineState, folder, repository, runFeature } from './engine/harness';

cleanUpAfterEach();

/** A history of ten commits, and a clone of its last `depth` commits, through a file:// URL, as git clones from a server. */
async function shallowClone(depth: number): Promise<{ origin: string; root: string }> {
  const origin = await repository();
  for (let index = 2; index <= 10; index++) await commitFile(origin, 'f.txt', `${index}\n`, `Commit ${index}`);
  const parent = folder();
  await git(parent, ['clone', '-q', `--depth=${depth}`, `file://${origin}`, 'clone']);
  return { origin, root: path.join(parent, 'clone') };
}

const count = (text: string) => (): Answer => ({ kind: 'text', text });
const listed = async (root: string) => (await engineState(root, 100)).rows.length;

describe.skipIf(!built)('The history of a shallow clone, through the engine', { timeout: 60_000 }, () => {
  it('repo.read-shallow: a clone with --depth 2 is shallow, ending at the older of its two commits; a whole one is not', async () => {
    const { origin, root } = await shallowClone(2);
    const state = await engineState(root);
    const older = (await git(root, ['rev-parse', 'HEAD~1'])).stdout.trim();
    expect(state.rows.length).toBe(2);
    expect(state.shallow).toEqual([older]);
    expect((await engineState(origin)).shallow ?? []).toEqual([]);
  });

  it('sync.deepen: Load more history with 3 lists five commits, still shallow; sync.unshallow lists the ten', async () => {
    const { root } = await shallowClone(2);
    const state = await engineState(root);
    const [deepen] = engineRuns({ kind: 'deepen' }, state) ?? [];
    expect(deepen).toEqual({ feature: 'sync.deepen', target: { kind: 'head' } });
    const deepened = await runFeature(root, deepen.feature, deepen.target, { count: count('3') });
    expect(deepened.finished.kind).toBe('ran');
    expect(deepened.lines).toEqual(['git fetch --deepen=3 origin']);
    expect(await listed(root)).toBe(5);
    expect((await engineState(root)).shallow?.length).toBe(1);

    const [unshallow] = engineRuns({ kind: 'unshallow' }, await engineState(root)) ?? [];
    expect(unshallow).toEqual({ feature: 'sync.unshallow', target: { kind: 'head' } });
    const whole = await runFeature(root, unshallow.feature, unshallow.target);
    expect(whole.lines).toEqual(['git fetch --unshallow origin']);
    expect(await listed(root)).toBe(10);
    expect((await engineState(root)).shallow ?? []).toEqual([]);
  });

  it('sync.deepen and sync.unshallow: a repository with its whole history is refused', async () => {
    const root = await repository();
    expect((await runFeature(root, 'sync.unshallow', { kind: 'head' })).finished.kind).toBe('refused');
    expect((await runFeature(root, 'sync.deepen', { kind: 'head' }, { count: count('3') })).finished.kind).toBe('refused');
  });
});

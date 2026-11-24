/**
 * What git remembers of conflicts, through the engine (milestone M46): whether it remembers them, in the state
 * (`rerere.read-state`, protocol 1.13), turned on and off as the menu of the row of the repository and the Command
 * Palette ask it (`rerere.turn-on`, `rerere.turn-off`); the files git resolved as before (`rerere.read-resolved`), as git
 * marked them (`rerere.read-marked`) and forgotten (`rerere.forget`), on throwaway repositories.
 */

import * as fs from 'node:fs';
import * as os from 'node:os';
import * as path from 'node:path';
import { describe, expect, it } from 'vitest';
import { engineRuns } from '../src/engine/actions';
import type { Answer, Finished } from '../src/engine/messages';
import { recordedConflicts } from '../src/engine/conflictPlans';
import { Reads } from '../src/engine/reads';
import { MARKED, bannerWords, stopWords } from '../src/shared/rerere';
import { git } from './git';
import { answering, built, cleanUpAfterEach, client, commitFile, confirmed, engineState, repository, runFeature } from './engine/harness';

cleanUpAfterEach();

// The user's own configuration may turn rerere on for every repository: git and the engine read an empty one instead.
const emptyConfig = path.join(fs.mkdtempSync(path.join(os.tmpdir(), 'gitardo-config-')), 'gitconfig');
fs.writeFileSync(emptyConfig, '');
process.env.GIT_CONFIG_GLOBAL = emptyConfig;

/** Runs **Remember how conflicts are resolved…**, ticked or not, through the engine, its question answered with `when`. */
async function remember(root: string, start: boolean, when = 'leave'): Promise<Finished> {
  const [run] = engineRuns({ kind: 'rememberResolutions', start }, await engineState(root)) ?? [];
  if (!run) throw new Error('rememberResolutions does not run in the engine');
  const asker = answering({ 'when-resolved-before': (): Answer => ({ kind: 'choice', identifier: when }) });
  return (await client().runFeature({ repository: root, feature: run.feature, target: run.target }, asker)).finished;
}

const config = async (root: string, key: string) => (await git(root, ['config', '--local', '--get', key]).catch(() => ({ stdout: '' }))).stdout.trim();

describe.skipIf(!built)('What git remembers of conflicts, through the engine', { timeout: 60_000 }, () => {
  it('rerere.read-state and rerere.turn-on: off at first, then on with rerere.enabled alone, and refused once on', async () => {
    const root = await repository();
    expect((await engineState(root)).rerere).toEqual({ isOn: false, stages: false, setsStages: false });
    const finished = await remember(root, true);
    expect(finished.kind).toBe('ran');
    expect(finished.kind === 'ran' && finished.plan.steps.map((step) => step.line)).toEqual(['git config --local rerere.enabled true']);
    expect(await config(root, 'rerere.enabled')).toBe('true');
    expect(await config(root, 'rerere.autoUpdate')).toBe('');
    expect((await engineState(root)).rerere).toEqual({ isOn: true, stages: false, setsStages: false });
    expect((await remember(root, true)).kind).toBe('refused');
  });

  it('rerere.turn-on with the second answer stages what git writes; rerere.turn-off sets it false and unsets autoUpdate', async () => {
    const root = await repository();
    expect((await remember(root, true, 'stage')).kind).toBe('ran');
    expect((await engineState(root)).rerere).toEqual({ isOn: true, stages: true, setsStages: true });
    const off = await remember(root, false);
    expect(off.kind === 'ran' && off.plan.steps.map((step) => step.line)).toEqual([
      'git config --local rerere.enabled false',
      'git config --local --unset rerere.autoUpdate',
    ]);
    expect((await engineState(root)).rerere?.isOn).toBe(false);
    expect((await remember(root, false)).kind).toBe('refused');
  });

  it('rerere.read-resolved, rerere.read-marked and rerere.forget: a merge resolved, undone and met again says m.txt resolved as before', async () => {
    const root = await repository();
    await git(root, ['config', 'rerere.enabled', 'true']);
    await commitFile(root, 'm.txt', 'one\nbase\nthree\n', 'Add m');
    await git(root, ['checkout', '-q', '-b', 'side']);
    await commitFile(root, 'm.txt', 'one\ntheirs\nthree\n', 'Theirs');
    await git(root, ['checkout', '-q', 'main']);
    await commitFile(root, 'm.txt', 'one\nours\nthree\n', 'Ours');
    await git(root, ['merge', 'side']).catch(() => undefined);
    fs.writeFileSync(path.join(root, 'm.txt'), 'one\nboth\nthree\n');
    await git(root, ['add', 'm.txt']);
    await git(root, ['commit', '-q', '--no-edit']);
    await git(root, ['reset', '-q', '--hard', 'HEAD~1']);
    await git(root, ['merge', 'side']).catch(() => undefined);
    expect(fs.readFileSync(path.join(root, 'm.txt'), 'utf8')).toBe('one\nboth\nthree\n');

    const state = await engineState(root);
    expect(state.resolvedAsBefore).toEqual([{ path: 'm.txt', isStaged: false }]);
    expect(bannerWords(state)).toBe('git resolved 1 as before');
    expect(stopWords(state)).toBe('Git resolved 1 file as it did before: look at them before you continue.');

    // Show what git wrote: the file as git marked it, read by the engine, beside what git wrote on disk.
    const reads = new Reads(() => client());
    const repo = { root } as Parameters<typeof recordedConflicts>[1];
    const marked = Buffer.from((await reads.fileBytesAt(root, MARKED, 'm.txt')) ?? new Uint8Array()).toString('utf8');
    expect(marked).toMatch(/^one\n<<<<<<< main\nours\n=======\ntheirs\n>>>>>>> side\nthree\n$/);
    expect(await recordedConflicts(reads, repo, ['m.txt'])).toEqual([{ path: 'm.txt', marked, written: 'one\nboth\nthree\n' }]);

    // Forget this resolution…: it asks, then the file has its markers again.
    const { finished, lines } = await runFeature(root, 'rerere.forget', { kind: 'file', path: 'm.txt' }, { confirm: confirmed() });
    expect(finished.kind).toBe('ran');
    expect(lines).toEqual(['git rerere forget m.txt', 'git checkout --merge -- m.txt']);
    expect(fs.readFileSync(path.join(root, 'm.txt'), 'utf8')).toContain('<<<<<<<');
    expect((await engineState(root)).resolvedAsBefore).toEqual([]);
  });
});

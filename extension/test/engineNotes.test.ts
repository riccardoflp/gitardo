/**
 * The notes of commits and the archives, run by Gitardo's engine as the menus of the views ask (milestone M40): the
 * actions mapped by `engineRuns`, run on throwaway repositories and a bare remote beside them, never on the network.
 */

import * as fs from 'node:fs';
import * as path from 'node:path';
import { describe, expect, it } from 'vitest';
import { engineRuns } from '../src/engine/actions';
import type { Answer, Question } from '../src/engine/messages';
import type { Action } from '../src/shared/protocol';
import { git } from './git';
import { built, cleanUpAfterEach, client, engineState, folder, repository } from './engine/harness';

cleanUpAfterEach();

/** Runs `action` through the engine as `engineRuns` maps it, its questions answered by key or by their default, each plan confirmed. */
async function runAction(root: string, action: Action, answers: Record<string, Answer> = {}): Promise<string> {
  const runs = engineRuns(action, await engineState(root));
  if (!runs) throw new Error(`${action.kind} does not run in the engine`);
  const engine = client();
  let finished = '';
  for (const run of runs) {
    const asker = {
      async ask(question: Question) {
        const answer = answers[question.key] ?? question.default;
        if (!answer) throw new Error(`No answer for ${question.key}`);
        return answer;
      },
      async confirm() {
        return true;
      },
    };
    const result = await engine.runFeature({ repository: root, feature: run.feature, target: run.target, answers: run.answers }, asker);
    finished = result.finished.kind;
  }
  return finished;
}

const text = (value: string): Answer => ({ kind: 'text', text: value });
const head = async (root: string) => (await git(root, ['rev-parse', 'HEAD'])).stdout.trim();

describe.skipIf(!built)('Notes and archives, through the engine', { timeout: 60_000 }, () => {
  it('note.edit and note.remove: Add note… writes the note the details then show, and Remove note takes it away', async () => {
    const root = await repository();
    const commit = await head(root);
    expect(await runAction(root, { kind: 'editNote', hash: commit }, { note: text('Reviewed.') })).toBe('ran');
    expect((await engineState(root)).notes?.commits).toEqual([commit]);
    expect((await client().details(root, { commit }))?.note).toBe('Reviewed.');
    expect(await runAction(root, { kind: 'removeNote', hash: commit })).toBe('ran');
    expect((await engineState(root)).notes?.commits).toEqual([]);
    expect((await client().details(root, { commit }))?.note ?? null).toBeNull();
  });

  it('note.push and note.fetch: Push notes sends them to the remote, and Fetch notes brings them into a clone', async () => {
    const root = await repository();
    const remote = path.join(path.dirname(root), 'remote.git');
    await git(path.dirname(root), ['init', '--quiet', '--bare', remote]);
    await git(root, ['remote', 'add', 'origin', remote]);
    await git(root, ['push', '--quiet', 'origin', 'main']);
    const commit = await head(root);
    await git(root, ['notes', 'add', '-m', 'Reviewed.', commit]);
    expect(await runAction(root, { kind: 'pushNotes', remote: 'origin' })).toBe('ran');
    const clone = path.join(path.dirname(root), 'clone');
    await git(path.dirname(root), ['clone', '--quiet', remote, clone]);
    expect((await engineState(clone)).notes?.commits).toEqual([]);
    expect(await runAction(clone, { kind: 'fetchNotes', remote: 'origin' })).toBe('ran');
    expect((await client().details(clone, { commit }))?.note).toBe('Reviewed.');
  });

  it('archive.save: Save as archive… on a tag writes a zip where it is asked to', async () => {
    const root = await repository();
    await git(root, ['tag', 'v1.0']);
    const saved = path.join(folder(), 'v1.0.zip');
    // The format starts on zip on Windows only, so it is chosen here.
    const answers: Record<string, Answer> = { format: { kind: 'choice', identifier: 'zip' }, path: { kind: 'path', path: saved } };
    expect(await runAction(root, { kind: 'saveArchive', at: 'refs/tags/v1.0' }, answers)).toBe('ran');
    expect(fs.readFileSync(saved).subarray(0, 2).toString('latin1')).toBe('PK');
  });
});

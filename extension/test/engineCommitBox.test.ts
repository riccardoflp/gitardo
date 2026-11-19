/**
 * The options of the commit box of **Gitardo Changes** and the signature of a commit, which milestone M34 brings to the
 * extension: each action of the view is given to `engineRuns`, run through the engine on a repository made by the
 * test, and the reads of 1.13 are asked of the engine as the views ask them.
 */
import * as fs from 'node:fs';
import * as path from 'node:path';
import { describe, expect, it } from 'vitest';
import { engineRuns } from '../src/engine/actions';
import { startingOptions } from '../src/shared/commitOptions';
import type { Action } from '../src/shared/protocol';
import { git } from './git';
import { answering, built, cleanUpAfterEach, client, engineState, repository } from './engine/harness';

cleanUpAfterEach();

/** Runs what the action gives through the engine and gives the lines of the steps. */
async function run(root: string, action: Action): Promise<string[]> {
  const runs = engineRuns(action, await engineState(root));
  if (!runs) throw new Error('This does not run in the engine');
  const lines: string[] = [];
  for (const { feature, target, answers } of runs) {
    const { finished } = await client().runFeature({ repository: root, feature, target, answers }, answering({}));
    expect(finished.kind, JSON.stringify(finished)).toBe('ran');
    if (finished.kind === 'ran') lines.push(...finished.plan.steps.map((step) => step.line));
  }
  return lines;
}

describe.skipIf(!built)('The commit box and the signature, through the engine', { timeout: 120_000 }, () => {
  it('commit.options: Add Signed-off-by, two co-authors and Skip the hooks commit past a failing hook, once', async () => {
    const root = await repository();
    const hooks = path.join(root, '.git', 'hooks');
    fs.mkdirSync(hooks, { recursive: true });
    fs.writeFileSync(path.join(hooks, 'pre-commit'), '#!/bin/sh\nexit 1\n', { mode: 0o755 });
    fs.writeFileSync(path.join(root, 'b.txt'), 'b\n');
    await git(root, ['add', 'b.txt']);
    const state = await engineState(root);
    const start = startingOptions(state.signing, false);
    const options = { ...start, signOff: true, skipHooks: true, coAuthors: ['Ben <ben@example.com>', 'Cy Two <cy@example.com>'] };
    const lines = await run(root, { kind: 'commit', message: 'Add b', amend: false, options, start });
    expect(lines.at(-1)).toContain('--signoff --no-verify --trailer');
    const message = (await git(root, ['log', '-1', '--format=%B'])).stdout.trim().split('\n');
    expect(message.slice(-3)).toEqual([
      'Signed-off-by: Gitardo Test <test@gitardo.invalid>',
      'Co-authored-by: Ben <ben@example.com>',
      'Co-authored-by: Cy Two <cy@example.com>',
    ]);
    // The next commit, with the options back where they start, runs the hook again, which refuses it.
    fs.writeFileSync(path.join(root, 'c.txt'), 'c\n');
    await git(root, ['add', 'c.txt']);
    const runs = engineRuns({ kind: 'commit', message: 'Add c', amend: false, options: start, start }, await engineState(root));
    expect(runs?.[0].answers).toEqual({ message: { kind: 'text', text: 'Add c' } });
  });

  it('commit.read-signing: the state says git signs every commit when commit.gpgSign is on, and Sign this commit unticked sends --no-gpg-sign', async () => {
    const root = await repository();
    await git(root, ['config', 'commit.gpgsign', 'true']);
    const state = await engineState(root);
    expect(state.signing).toEqual({ signsEveryCommit: true, kind: 'openpgp', key: null });
    fs.writeFileSync(path.join(root, 'b.txt'), 'b\n');
    await git(root, ['add', 'b.txt']);
    const start = startingOptions(state.signing, false);
    expect(start.sign).toBe(true);
    const lines = await run(root, { kind: 'commit', message: 'Add b', amend: false, options: { ...start, sign: false }, start });
    expect(lines.at(-1)).toBe('git commit --no-gpg-sign -m "Add b"');
  });

  it('details.read-signature and commit.read-authors: a commit not signed says so, and the authors come with their counts', async () => {
    const root = await repository();
    await git(root, ['commit', '--quiet', '--allow-empty', '--author=Ben <ben@example.com>', '-m', "Ben's commit"]);
    const head = (await git(root, ['rev-parse', 'HEAD'])).stdout.trim();
    expect(await client().signature(root, head)).toMatchObject({ verdict: 'notSigned', kind: null });
    const { authors } = await client().authors(root);
    expect(authors.map((author) => [author.email, author.commits])).toEqual([
      ['ben@example.com', 1],
      ['test@gitardo.invalid', 1],
    ]);
  });
});

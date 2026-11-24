/**
 * The lines selected in the Flow Diff, staged and unstaged by the engine (milestone M46): the rows of a selection
 * mapped to a change (`diff.select-lines`) and sent as **Stage lines** and **Unstage lines** send them
 * (`diff.stage-lines`, `diff.unstage-lines`), on throwaway repositories.
 */

import * as fs from 'node:fs';
import * as path from 'node:path';
import { describe, expect, it } from 'vitest';
import { computeDiff } from '../src/flow/diff/computeDiff';
import { rowsOfSelection } from '../src/flow/diff/lineMap';
import { linesTarget } from '../src/flow/lineTargets';
import type { DiffDescriptor } from '../src/flow/sides';
import { git } from './git';
import { built, cleanUpAfterEach, commitFile, repository, runFeature } from './engine/harness';

cleanUpAfterEach();

/** The model of the Flow Diff of two texts, as the page computes it. */
const modelOf = (oldText: string, newText: string) =>
  computeDiff({ oldText, newText, leftLabel: 'HEAD', rightLabel: 'Working Tree', languageId: 'plaintext', filePath: 'f.txt' });

describe.skipIf(!built)('The lines selected in the Flow Diff, through the engine', { timeout: 60_000 }, () => {
  it('diff.stage-lines and diff.unstage-lines: of five lines added, the second and third are staged alone, then unstaged', async () => {
    const root = await repository();
    await commitFile(root, 'f.txt', 'a\nb\n', 'Add f');
    const added = 'a\none\ntwo\nthree\nfour\nfive\nb\n';
    fs.writeFileSync(path.join(root, 'f.txt'), added);
    const descriptor: DiffDescriptor = { repoRoot: root, path: 'f.txt', left: { kind: 'commit', ref: 'HEAD' }, right: { kind: 'worktree' } };
    const model = modelOf('a\nb\n', added);
    const rows = rowsOfSelection(model, { side: 'right', startLine: 3, endLine: 4, endColumn: 6 });
    if (!rows) throw new Error('the selection is not in the model');

    const staged = await runFeature(root, 'diff.stage-lines', linesTarget(descriptor, model, rows));
    expect(staged.finished.kind).toBe('ran');
    expect(staged.lines).toEqual(['git apply --cached --unidiff-zero --whitespace=nowarn -']);
    expect((await git(root, ['show', ':f.txt'])).stdout).toBe('a\ntwo\nthree\nb\n');
    expect(fs.readFileSync(path.join(root, 'f.txt'), 'utf8')).toBe(added);

    // The same rows again: the page shows them staged now, and Unstage lines takes them out.
    const unstaged = await runFeature(root, 'diff.unstage-lines', linesTarget(descriptor, model, rows));
    expect(unstaged.finished.kind).toBe('ran');
    expect((await git(root, ['diff', '--cached', '--', 'f.txt'])).stdout).toBe('');
  });

  it('diff.stage-lines: rows with no line left to stage are refused with the engine\'s words', async () => {
    const root = await repository();
    await commitFile(root, 'f.txt', 'a\nb\n', 'Add f');
    fs.writeFileSync(path.join(root, 'f.txt'), 'a\nx\nb\n');
    await git(root, ['add', '--', 'f.txt']);
    const descriptor: DiffDescriptor = { repoRoot: root, path: 'f.txt', left: { kind: 'commit', ref: 'HEAD' }, right: { kind: 'worktree' } };
    const model = modelOf('a\nb\n', 'a\nx\nb\n');
    const rows = rowsOfSelection(model, { side: 'right', startLine: 2, endLine: 2, endColumn: 2 })!;
    const { finished } = await runFeature(root, 'diff.stage-lines', linesTarget(descriptor, model, rows));
    expect(finished.kind).toBe('refused');
    expect(JSON.stringify(finished)).toContain('These lines are staged already.');
  });
});

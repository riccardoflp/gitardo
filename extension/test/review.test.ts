/**
 * The review of conflicts (`src/shared/review.ts`): what it proposes for each conflict of a merge that git stopped, read
 * by the engine as the review reads it. Writing what is accepted is the engine's, in `engineConflicts.test.ts`.
 */
import * as fs from 'node:fs';
import * as path from 'node:path';
import { describe, expect, it } from 'vitest';
import { conflictPlans } from '../src/engine/conflictPlans';
import { Reads } from '../src/engine/reads';
import type { Choice } from '../src/shared/resolution';
import { conflictChanges, reviewFiles } from '../src/shared/review';
import { git } from './git';
import { built, cleanUpAfterEach, client, repository } from './engine/harness';

cleanUpAfterEach();

const text = (...lines: string[]) => lines.map((line) => `${line}\n`).join('');
const BASE = text('one', 'two', 'three', 'four', 'five');

/** A repository whose merge of `other` into `main` stopped on the conflicts of what each side made of the files. */
async function stopped(base: Record<string, string>, other: Record<string, string>, main: Record<string, string>): Promise<string> {
  const root = await repository(false);
  await git(root, ['config', 'core.autocrlf', 'false']);
  const commit = async (files: Record<string, string>, message: string) => {
    for (const [file, content] of Object.entries(files)) fs.writeFileSync(path.join(root, file), content);
    await git(root, ['add', '--all']);
    await git(root, ['commit', '--quiet', '-m', message]);
  };
  await commit(base, 'base');
  await git(root, ['checkout', '--quiet', '-b', 'other']);
  await commit(other, 'other change');
  await git(root, ['checkout', '--quiet', 'main']);
  await commit(main, 'main change');
  await expect(git(root, ['merge', '--quiet', 'other'])).rejects.toThrow();
  return root;
}

/** The conflicts of the files git left conflicted, as the review reads them through the engine. */
async function plans(root: string) {
  const conflicted = (await git(root, ['diff', '--name-only', '--diff-filter=U'])).stdout.trim().split('\n');
  return conflictPlans(new Reads(() => client()), { root }, conflicted);
}

describe('what the review shows of a conflict', () => {
  it('finds in the file the lines that the changes of a conflict take around its markers, for the lines around to follow them', () => {
    // Both sides changed "b" to "X" the same way, next to the conflict: what each side changed starts there.
    const file = text('a', 'X', '<<<<<<< ours', 'c1', '=======', 'c2', '>>>>>>> theirs', 'd');
    const conflict = {
      line: 3,
      resolution: null,
      from: 2,
      to: 7,
      ours: text('c1'),
      theirs: text('c2'),
      patch: null,
      stretch: { base: text('b', 'c'), ours: text('X', 'c1'), theirs: text('X', 'c2') },
    };
    expect(conflictChanges(file.split(/(?<=\n)/), conflict)).toEqual({
      base: text('b', 'c'),
      ours: text('X', 'c1'),
      theirs: text('X', 'c2'),
      before: text('a'),
      beforeLine: 1,
      after: text('d'),
    });
    expect(conflictChanges(file.split(/(?<=\n)/), { ...conflict, stretch: null })).toBeNull();
  });
});

describe.skipIf(!built)('the review of the conflicts of a merge, read through the engine', { timeout: 60_000 }, () => {
  it('proposes a resolution for each conflict, by the rules or by the AI', async () => {
    const root = await stopped(
      { 'b.txt': BASE, 'c.txt': BASE },
      { 'b.txt': BASE.replace('two', 'TWO'), 'c.txt': BASE.replace('one', 'theirs') },
      { 'b.txt': BASE.replace('three', 'THREE'), 'c.txt': BASE.replace('one', 'ours') },
    );
    const answered = { text: 'ours and theirs\n', reason: 'Both sides renamed the first line.', confidence: 'medium' as const };
    const asked = { answer: (file: string) => (file === 'c.txt' ? answered : undefined), asking: () => undefined };
    const [b, c] = reviewFiles(await plans(root), true, asked);
    expect(b.conflicts).toMatchObject([
      { line: 2, ours: text('two', 'THREE'), theirs: text('TWO', 'three'), proposal: { source: 'rules', text: text('TWO', 'THREE'), newLines: 0 } },
    ]);
    expect(b.conflicts[0].proposal?.reason).toBe('Changes to lines next to each other');
    expect(c.conflicts).toMatchObject([{ ours: text('ours'), theirs: text('theirs'), base: text('one'), after: text('two', 'three', 'four') }]);
    expect(c.conflicts[0].proposal).toMatchObject({ source: 'ai', from: 0, to: 5, text: 'ours and theirs\n', confidence: 'medium', newLines: 1 });
    // What each proposal takes the place of, as each side has it, for the columns.
    expect(b.conflicts[0].region).toEqual({ ours: text('two', 'THREE'), theirs: text('TWO', 'three') });
    expect(c.conflicts[0].region).toEqual({ ours: text('ours'), theirs: text('theirs') });
    // What each side changed of the base, for the cards, with the lines of the file around it and their numbers.
    expect(b.conflicts[0].changes).toEqual({
      base: text('two', 'three'),
      ours: text('two', 'THREE'),
      theirs: text('TWO', 'three'),
      before: text('one'),
      beforeLine: 1,
      after: text('four', 'five'),
    });
    expect(c.conflicts[0].changes).toMatchObject({
      base: text('one'),
      ours: text('ours'),
      theirs: text('theirs'),
      before: '',
      beforeLine: 1,
      after: text('two', 'three', 'four'),
    });
    // Without the rules, only the language model proposes.
    expect(reviewFiles(await plans(root), false, asked).map((file) => file.conflicts[0].proposal?.source ?? null)).toEqual([null, 'ai']);
  });

  it('puts what the user chose before what is proposed, and warns of a proposal that keeps a setting twice or leaves a file that is not JSON', async () => {
    const json = (...lines: string[]) => text('{', '  "name": "shop",', ...lines, '}');
    const root = await stopped(
      { 'config.json': json('  "version": "1.2.0"'), 'b.txt': BASE },
      { 'config.json': json('  "version": "1.3.0",', '  "discounts": true'), 'b.txt': BASE.replace('two', 'TWO') },
      { 'config.json': json('  "version": "1.2.1",', '  "shipping": true'), 'b.txt': BASE.replace('three', 'THREE') },
    );
    const both = {
      text: text('  "version": "1.2.1",', '  "shipping": true', '  "version": "1.3.0",', '  "discounts": true'),
      reason: 'Both.',
      confidence: 'low' as const,
    };
    const chosen = new Map<string, Choice>();
    const asked = {
      answer: (file: string) => (file === 'config.json' ? both : undefined),
      asking: () => undefined,
      choice: (file: string) => chosen.get(file),
    };
    const review = async () => reviewFiles(await plans(root), true, asked);

    // Both sides one after the other: the version twice, and a comma missing.
    const [b, config] = await review();
    expect(b.conflicts[0].warnings).toEqual([]);
    expect(config.conflicts[0].proposal?.source).toBe('ai');
    expect(config.conflicts[0].warnings).toMatchObject([
      { kind: 'repeated', key: 'version', lines: [0, 2] },
      { kind: 'invalid', format: 'JSON' },
    ]);

    // A side chosen comes before the language model and the rules, and leaves the file JSON.
    chosen.set('config.json', { kind: 'incoming' });
    chosen.set('b.txt', { kind: 'current-first' });
    const [b2, config2] = await review();
    expect(config2.conflicts[0].proposal).toMatchObject({
      source: 'choice',
      text: text('  "version": "1.3.0",', '  "discounts": true'),
      reason: 'Only the incoming side',
      newLines: 0,
    });
    expect(config2.conflicts[0].warnings).toEqual([]);
    expect(b2.conflicts[0].proposal).toMatchObject({
      source: 'choice',
      text: text('two', 'THREE', 'TWO', 'three'),
      reason: 'Both sides, the current one first',
    });

    // Lines written by hand: what neither side has is counted as new.
    chosen.set('config.json', { kind: 'edited', text: '  "version": "1.3.0",\n  "shipping": true,\n  "discounts": true' });
    const [, config3] = await review();
    expect(config3.conflicts[0].proposal).toMatchObject({ source: 'choice', reason: 'Written by you', newLines: 1 });
  });
});

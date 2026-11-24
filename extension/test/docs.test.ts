/**
 * `npm run check:docs` as part of `npm test` (Q378, risk RS19): the cards, the identifiers, the words of the glossary
 * and the tests each card says it is tested by, checked by `docs/catalog.py` with the arguments of the script of
 * `package.json`. It needs Python 3, which the project does not otherwise need: without it the check is skipped, and
 * said to be.
 */

import { execFileSync, spawnSync } from 'node:child_process';
import * as fs from 'node:fs';
import * as path from 'node:path';
import { describe, expect, it } from 'vitest';

const folder = path.resolve(__dirname, '..');

/** The first of the names Python 3 goes by that answers on this computer, or null. */
function findPython(): string | null {
  for (const name of ['python3', 'python', 'py']) {
    try {
      const version = execFileSync(name, ['--version'], { encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] });
      if (/^Python 3\./.test(version.trim())) return name;
    } catch {
      // Not this name; the next one.
    }
  }
  return null;
}

const python = findPython();
// Written past Vitest, which keeps what a file whose tests are all skipped logs to itself.
if (!python) process.stderr.write('The check of the documents (npm run check:docs) is skipped: Python 3 is not on the PATH.\n');

/** The arguments `npm run check:docs` gives Python, read from `package.json` so that both stay one. */
function checkArguments(): string[] {
  const script: string = JSON.parse(fs.readFileSync(path.join(folder, 'package.json'), 'utf8')).scripts['check:docs'];
  const [program, ...rest] = script.split(/\s+/);
  expect(program).toBe('python');
  return rest;
}

describe.skipIf(!python)('the documents, as npm run check:docs checks them', { timeout: 30_000 }, () => {
  it('finds no problem in the cards, the identifiers, the words or the tests the cards name', () => {
    const run = spawnSync(python!, checkArguments(), { cwd: folder, encoding: 'utf8' });
    expect(run.error).toBeUndefined();
    expect(run.status, `${run.stdout}${run.stderr}`).toBe(0);
    expect(run.stdout).toMatch(/no problems/);
  });
});

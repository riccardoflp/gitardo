/**
 * The length of the files of `src` (rule R5.2, risk RS1): none of TypeScript is over 1,000 lines, so that a file too
 * long is split along a line the reader sees when it is changed, before it grows further. The stylesheets are left
 * out (Q1700).
 */

import * as fs from 'node:fs';
import * as path from 'node:path';
import { describe, expect, it } from 'vitest';

const source = path.resolve(__dirname, '..', 'src');

/** The most lines a file of `src` may have. */
const MOST_LINES = 1_000;

/** Every TypeScript file of `src`, by its path from there, with `/` between folders. */
function files(folder = source): string[] {
  return fs.readdirSync(folder, { withFileTypes: true }).flatMap((entry) => {
    const full = path.join(folder, entry.name);
    if (entry.isDirectory()) return files(full);
    return /\.tsx?$/.test(entry.name) ? [path.relative(source, full).split(path.sep).join('/')] : [];
  });
}

/** How many lines a file has, as `wc -l` counts them. */
function lines(file: string): number {
  return fs.readFileSync(path.join(source, file), 'utf8').split('\n').length - 1;
}

describe('the length of the files of src', () => {
  it('finds the files of src', () => {
    expect(files()).toContain('extension/controller.ts');
  });

  it(`R5.2: no file of TypeScript is over ${MOST_LINES.toLocaleString('en')} lines`, () => {
    const long = files()
      .map((file) => ({ file, lines: lines(file) }))
      .filter((each) => each.lines > MOST_LINES)
      .map((each) => `${each.file}: ${each.lines} lines`);
    expect(long).toEqual([]);
  });
});

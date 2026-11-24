import * as assert from 'node:assert/strict';
import { test } from 'vitest';
import { computeDiff } from '../../src/flow/diff/computeDiff';
import { hasChangedRow, revertedRows, rightLineForLeft, rowsOfSelection, sideLinesOfRows } from '../../src/flow/diff/lineMap';
import { linesTarget } from '../../src/flow/lineTargets';

function model(oldText: string, newText: string) {
  return computeDiff({
    oldText,
    newText,
    leftLabel: 'HEAD',
    rightLabel: 'Working Tree',
    languageId: 'plaintext',
    filePath: 'f.txt',
  });
}

test('context and modified lines map to their own row', () => {
  const m = model('a\nb\nc\nd\n', 'x\na\nB\nc\nd\n');
  assert.equal(rightLineForLeft(m, 1), 2); // a, shifted by the insertion
  assert.equal(rightLineForLeft(m, 2), 3); // b → B
  assert.equal(rightLineForLeft(m, 4), 5);
});

test('a removed line maps to the line above the gap', () => {
  const m = model('a\nb\nc\nd\n', 'a\nd\n');
  assert.equal(rightLineForLeft(m, 2), 1);
  assert.equal(rightLineForLeft(m, 3), 1);
  assert.equal(rightLineForLeft(m, 4), 2);
});

test('removed lines at the top map to line 1', () => {
  assert.equal(rightLineForLeft(model('a\nb\nc\n', 'c\n'), 1), 1);
});

const FIVE_ADDED = model('a\nb\n', 'a\none\ntwo\nthree\nfour\nfive\nb\n');
const right = (startLine: number, endLine: number, endColumn = 2) => ({ side: 'right' as const, startLine, endLine, endColumn });

test('diff.select-lines: the second and third of five lines added take their two rows, and are staged as two lines of the working tree', () => {
  const rows = rowsOfSelection(FIVE_ADDED, right(3, 4));
  assert.deepEqual(rows, { start: 2, end: 4 });
  assert.ok(rows && hasChangedRow(FIVE_ADDED, rows));
  const target = linesTarget({ repoRoot: '/r', path: 'f.txt', left: { kind: 'commit', ref: 'HEAD' }, right: { kind: 'worktree' } }, FIVE_ADDED, rows!);
  assert.deepEqual(target.left, { kind: 'head' });
  assert.deepEqual(target.right, { kind: 'workingTree' });
  // No line of HEAD in those rows: none, at the place of the next line of HEAD, `b`.
  assert.deepEqual(target.leftLines, { start: 1, end: 1 });
  assert.deepEqual(target.leftText, []);
  assert.deepEqual(target.rightLines, { start: 2, end: 4 });
  assert.deepEqual(target.rightText, ['two', 'three']);
});

test('diff.select-lines: a selection that ends at the start of a line does not take it, and a caret takes the row of its line', () => {
  assert.deepEqual(rowsOfSelection(FIVE_ADDED, right(3, 5, 1)), { start: 2, end: 4 });
  assert.deepEqual(rowsOfSelection(FIVE_ADDED, right(4, 4, 1)), { start: 3, end: 4 });
});

test('diff.select-lines: rows with no changed line among them leave the three items greyed out', () => {
  const rows = rowsOfSelection(FIVE_ADDED, right(1, 1));
  assert.ok(rows && !hasChangedRow(FIVE_ADDED, rows));
});

test('diff.select-lines: selecting the new text of a line changed into another selects the row where the two face each other', () => {
  const m = model('a\nb\nc\n', 'a\nB\nc\n');
  const rows = rowsOfSelection(m, right(2, 2, 2))!;
  assert.deepEqual(sideLinesOfRows(m, rows, 'left'), { start: 1, end: 2, text: ['b'] });
  assert.deepEqual(sideLinesOfRows(m, rows, 'right'), { start: 1, end: 2, text: ['B'] });
});

test('diff.select-lines: lines removed are selected on the left side, and have no line on the right at the place of the next', () => {
  const m = model('a\nb\nc\nd\n', 'a\nd\n');
  const rows = rowsOfSelection(m, { side: 'left', startLine: 2, endLine: 3, endColumn: 2 })!;
  assert.deepEqual(sideLinesOfRows(m, rows, 'left'), { start: 1, end: 3, text: ['b', 'c'] });
  assert.deepEqual(sideLinesOfRows(m, rows, 'right'), { start: 1, end: 1, text: [] });
  // Removed at the end: no line after, the place is the end of the side.
  const atEnd = model('a\nb\n', 'a\n');
  assert.deepEqual(sideLinesOfRows(atEnd, rowsOfSelection(atEnd, { side: 'left', startLine: 2, endLine: 2, endColumn: 2 })!, 'right'), {
    start: 1,
    end: 1,
    text: [],
  });
});

test('diff.revert-lines: reverting the row of the second of three lines changed gives it its old text, the other two left as they are', () => {
  const m = model('a\nb\nc\n', 'A\nB\nC\n');
  const reverted = revertedRows(m, rowsOfSelection(m, right(2, 2))!);
  assert.deepEqual(reverted, { right: { start: 1, end: 2, text: ['B'] }, text: ['b'] });
});

test('diff.revert-lines: rows that cross two changes revert both, and keep the line that does not change between them', () => {
  const m = model('a\nb\nc\n', 'A\nb\nC\n');
  const reverted = revertedRows(m, rowsOfSelection(m, right(1, 3))!);
  assert.deepEqual(reverted, { right: { start: 0, end: 3, text: ['A', 'b', 'C'] }, text: ['a', 'b', 'c'] });
});

test('diff.revert-lines: reverting the rows of lines removed puts them back after the line before them', () => {
  const m = model('a\nb\nc\nd\n', 'a\nd\n');
  const reverted = revertedRows(m, rowsOfSelection(m, { side: 'left', startLine: 2, endLine: 3, endColumn: 2 })!);
  assert.deepEqual(reverted, { right: { start: 1, end: 1, text: [] }, text: ['b', 'c'] });
});

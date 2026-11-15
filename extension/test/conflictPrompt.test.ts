import { describe, expect, it } from 'vitest';
import { answerProposal, fingerprint } from '../src/shared/review';
import { resolveConflicts } from '../src/shared/conflicts';
import { partialResolution } from '../src/shared/conflictPrompt';
import { newLineCount } from '../src/shared/resolution';

// The question and the reading of the answer are the engine's; these are what the review does with an answer.

describe('an answer of the language model while it is written', () => {
  it('shows what is written so far, without the start of the closing tag', () => {
    expect(partialResolution('Thinking')).toBe('');
    expect(partialResolution('<resolution>\n  sum += item')).toBe('  sum += item');
    expect(partialResolution('<resolution>\n  sum;\n</reso')).toBe('  sum;\n');
    expect(partialResolution('<resolution>\n  sum;\n</resolution>\n<reason>')).toBe('  sum;\n');
  });
});

describe('an answer of the language model in its file', () => {
  const file = (...lines: string[]) => lines.map((line) => `${line}\n`).join('');
  const marked = file('top', '<<<<<<< HEAD', 'ours', '=======', 'theirs', '>>>>>>> feature', 'end');
  const answer = { text: 'ours\nnew line\n', reason: 'Both.', confidence: 'low' as const };

  it('replaces the markers and what is between them, and counts the lines neither side has', () => {
    const [conflict] = resolveConflicts(marked, file('top', 'old', 'end')).conflicts;
    expect(answerProposal(marked, conflict, answer)).toEqual({
      source: 'ai',
      from: 1,
      to: 6,
      text: 'ours\nnew line\n',
      reason: 'Both.',
      confidence: 'low',
      newLines: 1,
    });
    expect(newLineCount('  a\n\nb\n', 'a\n', null)).toBe(1);
  });

  it('takes the line endings of the file, also at its very end', () => {
    const windows = marked.replaceAll('\n', '\r\n');
    const [conflict] = resolveConflicts(windows, '').conflicts;
    expect(answerProposal(windows, conflict, answer).text).toBe('ours\r\nnew line\r\n');
    const last = file('top', '<<<<<<< HEAD', 'ours', '=======', 'theirs').concat('>>>>>>> feature');
    const [atEnd] = resolveConflicts(last, '').conflicts;
    expect(answerProposal(last, atEnd, answer).text).toBe('ours\nnew line');
  });

  it('knows a conflict by its text, wherever it is in the file', () => {
    const [conflict] = resolveConflicts(marked, '').conflicts;
    const moved = `${file('added above')}${marked}`;
    const [again] = resolveConflicts(moved, '').conflicts;
    expect(fingerprint(moved, again)).toBe(fingerprint(marked, conflict));
    expect(fingerprint(marked.replaceAll('\n', '\r\n'), conflict)).toBe(fingerprint(marked, conflict));
    expect(fingerprint(marked.replace('ours', 'edited'), conflict)).not.toBe(fingerprint(marked, conflict));
  });

  it('knows a conflict again when git names its sides otherwise, as after Undo', () => {
    const [conflict] = resolveConflicts(marked, '').conflicts;
    const renamed = marked.replace(/^<{7} .*$/m, '<<<<<<< ours').replace(/^>{7} .*$/m, '>>>>>>> theirs');
    expect(renamed).not.toBe(marked);
    expect(fingerprint(renamed, conflict)).toBe(fingerprint(marked, conflict));
  });
});

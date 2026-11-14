import { describe, expect, it } from 'vitest';
import { sideChanges } from '../src/webview/sideChanges';

const text = (...lines: string[]) => lines.map((line) => `${line}\n`).join('');

describe('what a side of a conflict changed of the base', () => {
  it('lists the lines kept, then in each change the lines taken out before the ones written', () => {
    const base = text('if (!item) throw new Error();', 'return item;');
    const side = text('if (!item) {', '  return null;', '}', 'return item;');
    expect(sideChanges(base, side)).toEqual([
      { kind: 'removed', index: 0, text: 'if (!item) throw new Error();', words: [[11, 29]] },
      { kind: 'added', index: 0, text: 'if (!item) {', words: [[11, 12]] },
      { kind: 'added', index: 1, text: '  return null;' },
      { kind: 'added', index: 2, text: '}' },
      { kind: 'same', index: 3, text: 'return item;' },
    ]);
  });

  it('marks the words that changed in a line, and not in a line written all anew', () => {
    const [removed, added] = sideChanges(text('const total = price * count;'), text('const total = price * amount;'))!;
    expect(removed).toMatchObject({ kind: 'removed', words: [[22, 27]] });
    expect(added).toMatchObject({ kind: 'added', words: [[22, 28]] });
    expect(sideChanges(text('alpha beta'), text('gamma delta'))!.every((line) => !line.words)).toBe(true);
  });

  it('has only lines written when there was nothing, and only lines taken out when nothing is left', () => {
    expect(sideChanges('', text('new'))).toEqual([{ kind: 'added', index: 0, text: 'new' }]);
    expect(sideChanges(text('old'), '')).toEqual([{ kind: 'removed', index: 0, text: 'old' }]);
    expect(sideChanges(text('same'), text('same'))).toEqual([{ kind: 'same', index: 0, text: 'same' }]);
  });
});

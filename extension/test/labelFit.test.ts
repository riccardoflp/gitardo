import { describe, expect, it } from 'vitest';
import { fitLabels, roomBesideAuthor, roomBesideMessage } from '../src/shared/labelFit';

/** The labels of the desktop's test of the same rule, each ten pixels a letter and ten more. */
const NAMES = ['main', 'feature', 'origin/main', 'origin/feature', 'v1.0'];
const widthOf = (text: string) => 10 * text.length + 10;
const room = (width: number) => ({ width, least: 60, gap: 10 });
const countWidth = (count: number) => widthOf(`+${count}`);

/** The words of the labels shown, `+N` last. */
const words = (names: string[], fitted: ReturnType<typeof fitLabels>) => [...names.slice(0, fitted.named), ...(fitted.counted ? [`+${fitted.counted}`] : [])];

describe('the labels that do not fit (graph.label-overflow)', () => {
  it('keeps room for the message and counts the labels that do not fit, as Gitardo Desktop does', () => {
    const widths = NAMES.map(widthOf);
    const all = fitLabels(widths, countWidth, room(1000));
    expect(words(NAMES, all)).toEqual(NAMES);
    expect(all.width).toBe(500);
    expect(all.hidesSome).toBe(false);

    const some = fitLabels(widths, countWidth, room(200));
    expect(words(NAMES, some)).toEqual(['main', 'feature', '+3']);
    expect(some.width).toBeLessThanOrEqual(200);
    expect(some.hidesSome).toBe(true);

    const rest = NAMES.slice(2);
    const shortened = fitLabels(rest.map(widthOf), countWidth, room(120));
    expect(words(rest, shortened)).toEqual(['origin/main', '+2']);
    expect(shortened.widths[0]).toBe(70);
    expect(shortened.shortened).toBe(true);
    expect(some.shortened).toBe(false);
    expect(shortened.hidesSome).toBe(true);

    const count = fitLabels(widths, countWidth, room(50));
    expect(words(NAMES, count)).toEqual(['+5']);

    const none = fitLabels(widths, countWidth, room(30));
    expect(none.widths).toEqual([]);
    expect(none.width).toBe(0);
    expect(none.hidesSome).toBe(true);
  });

  it('shows nothing and hides nothing for a commit with no label', () => {
    expect(fitLabels([], countWidth, room(100))).toEqual({ named: 0, counted: 0, widths: [], width: 0, hidesSome: false, shortened: false });
  });

  it('leaves the labels two fifths of a compact line at least, and all of what a short message leaves', () => {
    expect(roomBesideMessage(1000, 900)).toBe(400);
    expect(roomBesideMessage(1000, 300)).toBe(700);
  });

  it('leaves the labels two thirds of the line under the message at least, and all of what a short author and date leave', () => {
    expect(roomBesideAuthor(300, 200)).toBe(200);
    expect(roomBesideAuthor(300, 60)).toBe(240);
  });
});

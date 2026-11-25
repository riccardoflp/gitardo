/**
 * The commits a view is sent behind the first page of a long history (QA6): the next piece once the last one reached
 * the view, none for a view that is not there, and none once the view has them all.
 */
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { RowsBehind } from '../src/extension/rowsBehind';

describe('the commits sent behind the first page', () => {
  beforeEach(() => void vi.useFakeTimers());
  afterEach(() => void vi.useRealTimers());

  /** A view that is sent `pieces` pieces after its first page, and what it was asked to be sent again. */
  function sending(pieces: number) {
    const view = {};
    const asked: number[] = [];
    let left = pieces;
    const behind: RowsBehind<object> = new RowsBehind((to) => {
      asked.push(Date.now());
      left--;
      behind.sent(to, left > 0);
      behind.reached(to, true);
    }, 10);
    return { view, asked, behind };
  }

  it('sends the next piece a moment after the last one reached the view, until it has every commit', () => {
    const { view, asked, behind } = sending(3);
    behind.sent(view, true);
    expect(behind.has(view)).toBe(true);
    // Not before the first page reached the view.
    vi.advanceTimersByTime(100);
    expect(asked).toEqual([]);
    behind.reached(view, true);
    vi.advanceTimersByTime(9);
    expect(asked).toHaveLength(0);
    vi.advanceTimersByTime(1);
    expect(asked).toHaveLength(1);
    vi.advanceTimersByTime(100);
    expect(asked).toHaveLength(3);
    expect(behind.has(view)).toBe(false);
    vi.advanceTimersByTime(100);
    expect(asked).toHaveLength(3);
  });

  it('makes one piece at a time, however many states the view is sent meanwhile', () => {
    const { view, asked, behind } = sending(2);
    behind.sent(view, true);
    behind.reached(view, true);
    // A read of the repository sends the view its state again, with its next piece, while the wait runs.
    behind.sent(view, true);
    behind.reached(view, true);
    vi.advanceTimersByTime(10);
    expect(asked).toHaveLength(1);
  });

  it('sends nothing more to a view that what it was sent did not reach, nor to one that has every commit', () => {
    const { view, asked, behind } = sending(3);
    behind.sent(view, true);
    behind.reached(view, false);
    expect(behind.has(view)).toBe(false);
    vi.advanceTimersByTime(100);
    expect(asked).toEqual([]);

    // Pointed at a short history while a piece waited: it has every commit, and the wait is over.
    behind.sent(view, true);
    behind.reached(view, true);
    behind.sent(view, false);
    vi.advanceTimersByTime(100);
    expect(asked).toEqual([]);
    expect(behind.has(view)).toBe(false);
  });
});

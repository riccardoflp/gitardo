import { describe, expect, it } from 'vitest';
import { WORKTREE, type Row } from '../src/shared/protocol';
import { addedUnder, held, listingOf, type Held } from '../src/webview/pages';

const row = (hash: string, subject = hash): Row => ({ hash, parents: [], author: '', email: '', date: 0, subject, graph: { col: 0, color: 0, top: [], bottom: [] } });

describe('the commits listed, as the pages of a read come', () => {
  const listed = [row('c'), row('b'), row('a')];

  it('are the same commits with more under them when a page is added, and when nothing is', () => {
    expect(addedUnder(listed, [...listed, row('9'), row('8')])).toBe(true);
    expect(addedUnder(listed, [...listed])).toBe(true);
    expect(addedUnder([], listed)).toBe(true);
  });

  it('are others when a commit was made on top, when the read starts again from its first page, and when they come anew', () => {
    expect(addedUnder(listed, [row('d'), ...listed])).toBe(false);
    expect(addedUnder(listed, listed.slice(0, 2))).toBe(false);
    // The same hashes in rows sent again are a history read again: what it lists may have changed above them.
    expect(addedUnder(listed, listed.map((each) => row(each.hash)))).toBe(false);
  });

  it('are the same under the row of the uncommitted changes, which comes again with its number of files', () => {
    const before = [row(WORKTREE, 'Uncommitted changes (1)'), ...listed];
    expect(addedUnder(before, [row(WORKTREE, 'Uncommitted changes (2)'), ...listed, row('9')])).toBe(true);
    // Without it, the lines of the graph are others from the first row.
    expect(addedUnder(before, [...listed, row('9')])).toBe(false);
  });
});

describe('what the list keeps while its commits are read again', () => {
  const listing = listingOf({ root: '/repository', filter: null, scope: null, firstParent: false, showStashes: true });
  /** The list after each state of a read, from what it kept before: how many rows it has loaded, the most lanes, whether more come. */
  const after = (before: Held | null, states: [loaded: number, lanes: number, more: boolean][]) =>
    states.reduce<Held | null>((kept, [loaded, lanes, more]) => held(kept, listing, loaded, lanes, more), before);

  it('is as long as its commits, and its graph as wide as their lanes, while a history is read for the first time', () => {
    expect(after(null, [[1000, 3, true]])).toEqual({ listing, loaded: 1000, lanes: 3 });
    expect(after(null, [[1000, 3, true], [15000, 4, true], [56000, 4, false]])).toEqual({ listing, loaded: 56000, lanes: 4 });
  });

  it('keeps its length and the width of its graph when Refresh reads its commits again from the first page, until they are back', () => {
    const read = after(null, [[1000, 3, true], [56000, 4, false]]);
    // The first page of the read that starts again: 1,000 rows of three lanes, more to come.
    const again = after(read, [[1000, 3, true]]);
    expect(again).toEqual({ listing, loaded: 56000, lanes: 4 });
    expect(after(again, [[27000, 4, true]])).toEqual({ listing, loaded: 56000, lanes: 4 });
    // A commit was made meanwhile: one row more than before, and the list is as long as its commits again.
    expect(after(again, [[27000, 4, true], [56001, 4, false]])).toEqual({ listing, loaded: 56001, lanes: 4 });
  });

  it('is as long as the commits of a read that ends with fewer, as after a reset to an older commit', () => {
    const read = after(null, [[56000, 4, false]]);
    expect(after(read, [[1000, 3, true], [1200, 3, false]])).toEqual({ listing, loaded: 1200, lanes: 3 });
  });

  it('starts from the first page of another history: another filter, other branches, another repository', () => {
    const read = after(null, [[56000, 4, false]]);
    const other = listingOf({ root: '/repository', filter: null, scope: { kind: 'current', upstream: false }, firstParent: false, showStashes: true });
    expect(held(read, other, 1000, 2, true)).toEqual({ listing: other, loaded: 1000, lanes: 2 });
    const elsewhere = listingOf({ root: '/other', filter: null, scope: null, firstParent: false, showStashes: true });
    expect(held(read, elsewhere, 1000, 2, true)).toEqual({ listing: elsewhere, loaded: 1000, lanes: 2 });
    expect(other).not.toBe(listing);
  });
});

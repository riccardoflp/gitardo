import { describe, expect, it } from 'vitest';
import { WORKTREE, type Row } from '../src/shared/protocol';
import { addedUnder } from '../src/webview/pages';

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

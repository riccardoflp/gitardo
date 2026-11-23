/**
 * The notification of git's commit-graph file (`upkeep.offer-commit-graph`), with the workspace state and the buttons
 * of VS Code faked: which histories are offered the file, **Write it**, and **Not for this repository** kept under
 * `gitardo.commitGraphDeclined`.
 */

import { describe, expect, it } from 'vitest';
import { DECLINED_KEY, NOT_FOR_THIS_REPOSITORY, type OfferHost, WRITE_IT, noticeOf, offerCommitGraph, offersCommitGraph } from '../src/extension/commitGraphOffer';
import type { RepoState, Row } from '../src/shared/protocol';
import type { CommitGraph } from '../src/shared/state';

/** As many rows of commits as asked, and one stash. */
function rows(count: number): Row[] {
  const row = (index: number): Row => ({
    hash: index.toString(16).padStart(40, '0'),
    parents: [],
    author: 'Test Author',
    email: 'author@example.com',
    date: 0,
    subject: `Change ${index}`,
    graph: { col: 0, color: 0, top: [], bottom: [] },
  });
  const all = Array.from({ length: count }, (_, index) => row(index));
  return [{ ...row(count), stash: 'stash@{0}' }, ...all];
}

const missing: CommitGraph = { hasFile: false, isIgnored: false };

/** The state of a repository read whole, as the offer looks at it. */
const stateOf = (root: string, count: number, commitGraph: CommitGraph | undefined = missing) => ({ root, name: 'app', rows: rows(count), commitGraph }) as unknown as RepoState;

/** A workspace state, a notification answered with a button, and the writes asked, recorded. */
function host(button: string | undefined): OfferHost & { asked: string[]; written: number; kept: Map<string, unknown> } {
  const kept = new Map<string, unknown>();
  const faked = {
    asked: [] as string[],
    written: 0,
    kept,
    memento: {
      get: <T>(key: string, fallback?: T) => (kept.has(key) ? kept.get(key) : fallback) as T,
      update: async (key: string, value: unknown) => void kept.set(key, value),
    } as OfferHost['memento'],
    ask: async (message: string, ...buttons: string[]) => {
      faked.asked.push(message, ...buttons);
      return button;
    },
    write: async () => void faked.written++,
  };
  return faked;
}

describe('upkeep.offer-commit-graph in the extension', () => {
  it('offers the file to a history of 10,000 commits with none, and not to 9,999, nor with the file, nor with core.commitGraph false', () => {
    expect(offersCommitGraph(stateOf('C:/prj/a', 10_000))).toBe(true);
    expect(offersCommitGraph(stateOf('C:/prj/a', 9_999))).toBe(false);
    expect(offersCommitGraph(stateOf('C:/prj/a', 56_000, { hasFile: true, isIgnored: false }))).toBe(false);
    expect(offersCommitGraph(stateOf('C:/prj/a', 56_000, { hasFile: false, isIgnored: true }))).toBe(false);
    expect(offersCommitGraph({ ...stateOf('C:/prj/a', 56_000), commitGraph: undefined })).toBe(false);
  });

  it('says the notice with Write it and Not for this repository, and Write it writes the file, once', async () => {
    const faked = host(WRITE_IT);
    expect(await offerCommitGraph(stateOf('C:/prj/write', 10_000), faked)).toBe(WRITE_IT);
    expect(faked.asked).toEqual([noticeOf('app'), WRITE_IT, NOT_FOR_THIS_REPOSITORY]);
    expect(faked.written).toBe(1);
    expect(await offerCommitGraph(stateOf('C:/prj/write', 10_000), faked)).toBeUndefined();
    expect(faked.written).toBe(1);
  });

  it('keeps Not for this repository in gitardo.commitGraphDeclined, and says nothing for that repository after', async () => {
    const faked = host(NOT_FOR_THIS_REPOSITORY);
    expect(await offerCommitGraph(stateOf('C:/prj/declined', 10_000), faked)).toBe(NOT_FOR_THIS_REPOSITORY);
    expect(faked.kept.get(DECLINED_KEY)).toEqual(['C:/prj/declined']);
    expect(faked.written).toBe(0);
    // As after VS Code started again: the notice was said for no repository, and the workspace state keeps the answer.
    const again = host(WRITE_IT);
    await again.memento.update(DECLINED_KEY, ['c:\\prj\\declined']);
    expect(await offerCommitGraph(stateOf('C:/prj/declined2', 9_999), again)).toBeUndefined();
    expect(await offerCommitGraph(stateOf('C:/prj/Declined', 10_000), again)).toBeUndefined();
    expect(again.asked).toEqual([]);
  });

  it('says it again for a repository whose notice was closed without a button only once VS Code starts again', async () => {
    const faked = host(undefined);
    expect(await offerCommitGraph(stateOf('C:/prj/closed', 10_000), faked)).toBeUndefined();
    expect(faked.asked).toHaveLength(3);
    expect(faked.kept.has(DECLINED_KEY)).toBe(false);
    await offerCommitGraph(stateOf('C:/prj/closed', 10_000), faked);
    expect(faked.asked).toHaveLength(3);
  });
});

import { describe, expect, it } from 'vitest';
import type { EngineClient } from '../src/engine/client';
import { EngineGraph, completed, graphState } from '../src/engine/graph';
import type { EngineRow } from '../src/engine/messages';
import type { RepoParts } from '../src/shared/protocol';
import { headAt, workingTreeOf } from './states';

/** A line of commits, newest first, each the parent of the one before. */
function line(count: number): EngineRow[] {
  const id = (number: number) => number.toString(16).padStart(40, '0');
  return Array.from({ length: count }, (_, index) => ({
    kind: 'commit' as const,
    commit: { id: id(count - index), parents: index + 1 < count ? [id(count - index - 1)] : [], authorName: 'Test Author', authorEmail: 'author@example.com', authorDate: count - index, subject: `Commit ${count - index}` },
  }));
}

/** An engine whose read of the commits gives its pages, and ends, when the test says. */
function engine() {
  let give: (rows: EngineRow[]) => void = () => {};
  let end: (answer: { hasMore: boolean }) => void = () => {};
  const client = {
    graphStream: (_repository: string, _timeline: unknown, onRows: (rows: EngineRow[]) => void) => {
      give = onRows;
      return new Promise<{ hasMore: boolean }>((resolve) => (end = resolve));
    },
  } as unknown as EngineClient;
  return { client, page: (rows: EngineRow[]) => give(rows), end: (hasMore: boolean) => end({ hasMore }) };
}

/** What `repo.read-state` reads of a repository with nothing to commit, as far as the graph looks at it. */
const parts = { head: headAt('f'.repeat(40), 'main'), workingTree: workingTreeOf(), branches: [], remoteBranches: [], tags: [], stashes: [] } as unknown as RepoParts;
const asked = { limit: 1000, filter: null, scope: null, stashes: true, firstParent: false };
/** Lets what waits for a promise already settled go on. */
const turn = () => new Promise<void>((resolve) => setTimeout(resolve, 0));

describe('the pages of a read of the commits, as the views are given them', () => {
  it('has every commit, and no more to come, when the read ends while the rest of the state is read', async () => {
    const { client, page, end } = engine();
    const commits = line(1250);
    const graph = new EngineGraph();
    const reading = graphState(client, '/repository', parts, asked, (wanted) => graph.commits(client, '/repository', wanted, 1000));
    await turn();
    page(commits.slice(0, 1000));
    const state = await reading;
    // The first page is given as soon as it is read, the read going on.
    expect(state.rows).toHaveLength(1000);
    expect(state.hasMore).toBe(true);

    // The last page comes, and the read ends, while the newest entry of the journal is asked for.
    const shown = await completed(state, async () => {
      page(commits.slice(1000));
      end(false);
      await turn();
      return { undo: { title: 'Commit', undoable: true } };
    });
    expect(shown.rows).toHaveLength(1250);
    expect(shown.rows.at(-1)?.subject).toBe('Commit 1');
    expect(shown.hasMore).toBe(false);
    expect(shown.undo).toEqual({ title: 'Commit', undoable: true });
    // The rows of the first page are the same rows, laid out once.
    expect(shown.rows[999]).toBe(state.rows[999]);
  });

  it('keeps the rows it has while the read still goes on, and says there are more', async () => {
    const { client, page } = engine();
    const commits = line(2500);
    const graph = new EngineGraph();
    const reading = graphState(client, '/repository', parts, asked, (wanted) => graph.commits(client, '/repository', wanted, 1000));
    await turn();
    page(commits.slice(0, 1000));
    const state = await reading;
    const shown = await completed(state, async () => {
      page(commits.slice(1000, 2000));
      return {};
    });
    expect(shown.rows).toHaveLength(2000);
    expect(shown.hasMore).toBe(true);
    expect(graph.isReading('/repository')).toBe(true);
  });
});

import { describe, expect, it } from 'vitest';
import type { EngineClient } from '../src/engine/client';
import { EngineGraph, completed, graphState, laidOutOf } from '../src/engine/graph';
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

/** An engine whose read of the commits gives its pages, and ends, when the test says; it counts the reads asked of it. */
function engine() {
  let give: (rows: EngineRow[]) => void = () => {};
  let end: (answer: { hasMore: boolean }) => void = () => {};
  let reads = 0;
  const client = {
    graphStream: (_repository: string, _timeline: unknown, onRows: (rows: EngineRow[]) => void) => {
      reads++;
      give = onRows;
      return new Promise<{ hasMore: boolean }>((resolve) => (end = resolve));
    },
  } as unknown as EngineClient;
  return { client, page: (rows: EngineRow[]) => give(rows), end: (hasMore: boolean) => end({ hasMore }), reads: () => reads };
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

  /** The whole history read once, in pages of 1,000, as the views have it, and what reads it again. */
  async function listed(commits: EngineRow[]) {
    const { client, page, end, reads } = engine();
    const graph = new EngineGraph();
    const reader = (wanted: Parameters<EngineGraph['commits']>[2]) => graph.commits(client, '/repository', wanted, 1000);
    const reading = graphState(client, '/repository', parts, asked, reader);
    await turn();
    const shown = await completed(await (page(commits.slice(0, 1000)), reading), async () => {
      for (let from = 1000; from < commits.length; from += 1000) page(commits.slice(from, from + 1000));
      end(false);
      await turn();
      return {};
    });
    /** Reads the history again with the references of `again`, as the rows shown were laid out, until its first page. */
    const readAgain = async (again: RepoParts, first: EngineRow[]) => {
      const read = graphState(client, '/repository', again, asked, reader, undefined, laidOutOf(shown.rows));
      await turn();
      page(first);
      return read;
    };
    return { graph, page, end, reads, shown, readAgain };
  }

  it('keeps every row, and gives the views none, when a read after a branch was made finds the same commits', async () => {
    const commits = line(2500);
    const { page, end, reads, shown, readAgain } = await listed(commits);
    expect(shown.rows).toHaveLength(2500);
    const branched = { ...parts, branches: [{ name: 'topic', commit: 'f'.repeat(40) }] } as unknown as RepoParts;
    const state = await readAgain(branched, commits.slice(0, 1000));
    expect(reads()).toBe(2);
    // The rows are those the views have: the same array, so that none of them is sent again.
    expect(state.rows).toBe(shown.rows);
    page(commits.slice(1000, 2000));
    page(commits.slice(2000));
    end(false);
    await turn();
    const laid = laidOutOf(state.rows)!;
    expect(laid.catchUp()).toBe(shown.rows);
    expect(laid.source.hasMore).toBe(false);
  });

  it('reads every commit again on Refresh, and keeps the rows of those that are the same', async () => {
    const commits = line(1500);
    const { graph, end, reads, shown, readAgain } = await listed(commits);
    graph.readAgain('/repository');
    const state = await readAgain(parts, commits.slice(0, 1000));
    expect(reads(), 'Refresh asks git again').toBe(2);
    expect(state.rows).toBe(shown.rows);
    end(false);
  });

  it('cuts the rows where a read finds other commits, as after a commit made on top, and lays them out as a first read would', async () => {
    const commits = line(2500);
    const { page, end, shown, readAgain } = await listed(commits);
    const made = line(2501).slice(0, 1);
    const moved = [...made, ...commits];
    const state = await readAgain({ ...parts, head: headAt(made[0].kind === 'commit' ? made[0].commit.id : '', 'main') }, moved.slice(0, 1000));
    expect(state.rows).toHaveLength(1000);
    expect(state.rows[0].subject).toBe('Commit 2501');
    expect(state.rows[1]).not.toBe(shown.rows[0]);
    page(moved.slice(1000, 2000));
    page(moved.slice(2000));
    end(false);
    await turn();
    const rows = laidOutOf(state.rows)!.catchUp();
    expect(rows.map((row) => row.hash)).toEqual(moved.map((row) => (row.kind === 'commit' ? row.commit.id : '')));
    // The lines of the graph are those of the same commits read once.
    const fresh = await listed(moved);
    expect(rows.map((row) => row.graph)).toEqual(fresh.shown.rows.map((row) => row.graph));
  });

  it('takes out the commits the history no longer has when a read of it ends with fewer', async () => {
    const commits = line(2500);
    const { graph, page, end, shown, readAgain } = await listed(commits);
    graph.readAgain('/repository');
    const state = await readAgain(parts, commits.slice(0, 1000));
    expect(state.rows).toBe(shown.rows);
    page(commits.slice(1000, 1200));
    end(false);
    await turn();
    const rows = laidOutOf(state.rows)!.catchUp();
    expect(rows).toHaveLength(1200);
    expect(rows.at(-1)?.hash).toBe(shown.rows[1199].hash);
  });
});

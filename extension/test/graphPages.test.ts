import { describe, expect, it } from 'vitest';
import type { EngineClient } from '../src/engine/client';
import { EngineGraph, completed, graphState, laidOutOf } from '../src/engine/graph';
import type { EngineRow } from '../src/engine/messages';
import { WORKTREE, type RepoParts, type RepoState, type Scope } from '../src/shared/protocol';
import { branchAt, headAt, workingTreeOf } from './states';

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

  /** The state of a worktree of `/api`, in `root`, read with the graph of the group that shows it, as its session reads it. */
  const worktreeState = (client: EngineClient, graph: EngineGraph, root: string, read: RepoParts, shown?: RepoState, scope: Scope | null = null) => {
    graph.keepBy(root, '/api/.git');
    const reader = (wanted: Parameters<EngineGraph['commits']>[2]) => graph.commits(client, root, wanted, 1000);
    return graphState(client, root, read, { ...asked, scope }, reader, undefined, shown && laidOutOf(shown.rows), graph.placeOf(root));
  };
  const tip = (commits: EngineRow[], at: number) => (commits[at].kind === 'commit' ? commits[at].commit.id : '');
  /** What is read of `/api`, on `main`, and of its worktree on `fix`, a few commits before: the same references, another one checked out. */
  const worktrees = (commits: EngineRow[]) => {
    const refs = (checkedOut: string) => [branchAt('fix', tip(commits, 3), { isCheckedOut: checkedOut === 'fix' }), branchAt('main', tip(commits, 0), { isCheckedOut: checkedOut === 'main' })];
    const of = (branch: string, at: number) => ({ ...parts, head: headAt(tip(commits, at), branch), branches: refs(branch) }) as unknown as RepoParts;
    return { main: of('main', 0), fix: of('fix', 3) };
  };

  it('gives a worktree the rows its repository has read, with no read of the commits, when its HEAD is on a branch', async () => {
    const { client, page, end, reads } = engine();
    const commits = line(1200);
    const { main, fix } = worktrees(commits);
    const graph = new EngineGraph();
    const reading = worktreeState(client, graph, '/api', main);
    await turn();
    page(commits.slice(0, 1000));
    const api = await reading;
    const taken = await worktreeState(client, graph, '/api-fix', fix, api);
    expect(reads(), 'no read of the commits for the worktree').toBe(1);
    // The rows are those the views have: the same array, so that none of them is sent again.
    expect(taken.rows).toBe(api.rows);
    expect(taken.root).toBe('/api-fix');
    expect(taken.head).toEqual(fix.head);
    expect(graph.shares('/api', '/api-fix')).toBe(true);
    // The pages the read of the repository still adds are the worktree's too, and choosing the repository back takes them.
    page(commits.slice(1000));
    end(false);
    await turn();
    expect(graph.isReading('/api-fix')).toBe(false);
    const back = await worktreeState(client, graph, '/api', main, taken);
    expect(reads()).toBe(1);
    expect(back.rows).toHaveLength(1200);
    expect(back.rows[999]).toBe(api.rows[999]);
  });

  it('lays the rows out again under the row of the uncommitted changes of the worktree, with no read of the commits', async () => {
    const { client, page, end, reads } = engine();
    const commits = line(500);
    const { main, fix } = worktrees(commits);
    const graph = new EngineGraph();
    const reading = worktreeState(client, graph, '/api', main);
    await turn();
    page(commits);
    end(false);
    const api = await reading;
    const changed = { ...fix, workingTree: workingTreeOf({ unstaged: [{ path: 'a.txt', kind: 'modified', originalPath: null }] }) } as unknown as RepoParts;
    const taken = await worktreeState(client, graph, '/api-fix', changed, api);
    expect(reads()).toBe(1);
    expect(taken.rows[0].hash).toBe(WORKTREE);
    expect(taken.rows[0].parents).toEqual([tip(commits, 3)]);
    expect(taken.rows.slice(1).map((row) => row.hash)).toEqual(api.rows.map((row) => row.hash));
  });

  it('reads the commits of a worktree detached at a commit no reference is on, over the rows of its repository', async () => {
    const { client, page, end, reads } = engine();
    const commits = line(300);
    const { main } = worktrees(commits);
    const graph = new EngineGraph();
    const reading = worktreeState(client, graph, '/api', main);
    await turn();
    page(commits);
    end(false);
    const api = await reading;
    const own = { kind: 'commit' as const, commit: { id: 'd'.repeat(40), parents: [tip(commits, 0)], authorName: 'Test Author', authorEmail: 'author@example.com', authorDate: 999, subject: 'Tried apart' } };
    const detached = { ...main, head: headAt(own.commit.id), branches: main.branches.map((branch) => ({ ...branch, isCheckedOut: false })) } as unknown as RepoParts;
    const taking = worktreeState(client, graph, '/api-tried', detached, api);
    await turn();
    expect(reads(), 'one read for the commit of its own').toBe(2);
    page([own, ...commits]);
    end(false);
    const taken = await taking;
    const rows = laidOutOf(taken.rows)!.catchUp();
    expect(rows[0].subject).toBe('Tried apart');
    expect(rows.slice(1).map((row) => row.hash)).toEqual(api.rows.map((row) => row.hash));
  });

  it('reads a worktree as a repository chosen when it shows what is checked out, which is its own', async () => {
    const { client, page, end, reads } = engine();
    const commits = line(300);
    const { main, fix } = worktrees(commits);
    const graph = new EngineGraph();
    const current: Scope = { kind: 'current' };
    const reading = worktreeState(client, graph, '/api', main, undefined, current);
    await turn();
    page(commits);
    end(false);
    const api = await reading;
    const taking = worktreeState(client, graph, '/api-fix', fix, api, current);
    await turn();
    expect(reads()).toBe(2);
    page(commits.slice(3));
    end(false);
    const taken = await taking;
    expect(taken.rows).not.toBe(api.rows);
    expect(taken.rows).toHaveLength(297);
  });

  it('keeps the commits of a submodule, or of any repository with a git folder of its own, apart', async () => {
    const { client, page, end, reads } = engine();
    const commits = line(300);
    const { main } = worktrees(commits);
    const graph = new EngineGraph();
    const reading = worktreeState(client, graph, '/api', main);
    await turn();
    page(commits);
    end(false);
    const api = await reading;
    graph.keepBy('/api/vendor/lib', '/api/.git/modules/lib');
    expect(graph.shares('/api', '/api/vendor/lib')).toBe(false);
    const reader = (wanted: Parameters<EngineGraph['commits']>[2]) => graph.commits(client, '/api/vendor/lib', wanted, 1000);
    const submodule = graphState(client, '/api/vendor/lib', main, asked, reader, undefined, undefined, graph.placeOf('/api/vendor/lib'));
    await turn();
    expect(reads()).toBe(2);
    page(commits);
    end(false);
    expect((await submodule).rows).not.toBe(api.rows);
    // The read of the repository is still there, by the folder its worktrees share.
    expect((await worktreeState(client, graph, '/api', main, api)).rows).toBe(api.rows);
    expect(reads()).toBe(2);
  });

  it('moves the read a repository has to the folder its worktrees share once it is known', async () => {
    const { client, page, end, reads } = engine();
    const commits = line(300);
    const { main, fix } = worktrees(commits);
    const graph = new EngineGraph();
    const reading = graphState(client, '/api', main, asked, (wanted) => graph.commits(client, '/api', wanted, 1000));
    await turn();
    page(commits);
    end(false);
    await reading;
    graph.keepBy('/api', '/api/.git');
    await worktreeState(client, graph, '/api-fix', fix);
    expect(reads()).toBe(1);
  });
});

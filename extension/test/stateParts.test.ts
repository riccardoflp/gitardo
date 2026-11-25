import { describe, expect, it } from 'vitest';
import { stateFor, type Most, type Sent } from '../src/extension/sentParts';
import { WORKTREE, type HostMessage, type RepoState, type Row, type ViewKind } from '../src/shared/protocol';
import { NO_STATE, partsOf, withParts, type ShownState } from '../src/shared/stateParts';
import { branchAt, headAt, tagAt, workingTreeOf, worktreeAt } from './states';

const cell = { col: 0, color: 0, top: [], bottom: [] };
const row = (hash: string, parents: string[] = []): Row => ({ hash, parents, author: 'A', email: 'a@example.com', date: 0, subject: hash, graph: cell });
const draft = (count: number, on = 'c3'): Row => ({ ...row(WORKTREE, [on]), subject: `Uncommitted changes (${count})` });

/** A state read of a repository: every part made anew, as a read of the engine makes it. */
function read(extra: Partial<RepoState> = {}): RepoState {
  const { draft: _draft, commits: _commits, ...empty } = NO_STATE;
  return {
    ...structuredClone(empty),
    root: '/work/app',
    name: 'app',
    head: headAt('c3', 'main'),
    branches: [branchAt('main', 'c3', { isCheckedOut: true }), branchAt('topic', 'c2')],
    tags: [tagAt('v1', 'c1')],
    worktrees: [worktreeAt('/work/app', 'main', { isMain: true, isCurrent: true })],
    rows: [row('c3', ['c2']), row('c2', ['c1']), row('c1')],
    ...extra,
  };
}

/** What identifies the first rows of a state: their hashes and how each is drawn, as far as that. */
const keyOf = (state: RepoState) => (count: number) => state.rows.slice(0, count).map((each) => `${each.hash}:${each.graph.col}:${each.graph.top.length}`).join();

type StateMessage = Extract<HostMessage, { type: 'state' }>;

/**
 * Sends one view the states one after the other, as `Controller.post` does, and keeps what the view has of them. With
 * `most`, no message carries more commits than it says, and `more` tells whether the view has not got them all.
 */
function viewOf(kind: ViewKind, most?: Most) {
  let sent: Sent | undefined;
  let shown: ShownState | null = null;
  let more = false;
  return {
    send(state: RepoState): StateMessage | undefined {
      const { message, sent: now, more: left } = stateFor(kind, state, sent, keyOf(state), most);
      sent = now;
      more = left;
      const posted = message as StateMessage | undefined;
      if (posted?.state) shown = withParts(shown, posted.state, posted.parts ?? [], posted.kept, posted.keptLast);
      return posted;
    },
    get more() {
      return more;
    },
    get shown() {
      return shown!;
    },
  };
}

describe('the parts of the state each view is sent (D88)', () => {
  it('sends the graphs every part, the other views what they show, and the tabs of conflicts and ranges nothing', () => {
    expect(partsOf('graph')).toContain('rows');
    expect(partsOf('sideGraph')).toContain('draft');
    for (const kind of ['changes', 'commits', 'branches', 'stashes', 'commit', 'compare'] as const) expect(partsOf(kind)).not.toContain('rows');
    expect(partsOf('branches')).not.toContain('workingTree');
    expect(partsOf('commit')).toEqual(['repository', 'refs', 'stashes', 'worktrees', 'pulls']);
    for (const kind of ['conflicts', 'rangeDiff', 'worktreeSetup', 'repositories'] as const) expect(partsOf(kind)).toEqual([]);
  });

  it('sends a view every part it shows first, then only the parts that changed, and nothing when none did', () => {
    const branches = viewOf('branches');
    const first = branches.send(read())!;
    expect(first.parts).toEqual(partsOf('branches'));
    expect(first.state?.branches?.map((branch) => branch.name)).toEqual(['main', 'topic']);
    expect(first.state).not.toHaveProperty('rows');
    expect(first.state).not.toHaveProperty('workingTree');

    // A read that found the same: nothing is sent.
    expect(branches.send(read())).toBeUndefined();
    // A file changed, which the Branches view does not show.
    expect(branches.send(read({ workingTree: workingTreeOf({ untracked: ['new.txt'] }), changes: 1 }))).toBeUndefined();

    const tagged = branches.send(read({ tags: [tagAt('v1', 'c1'), tagAt('v2', 'c3')] }))!;
    expect(tagged.parts).toEqual(['refs']);
    expect(Object.keys(tagged.state!).sort()).toEqual(['branchSort', 'branches', 'favorites', 'listed', 'reflogSubjects', 'remoteBranches', 'sortOrder', 'tagMarks', 'tags']);
  });

  it('sends the Changes view what is not committed when it changed, and the tab of a commit only what its labels need', () => {
    const changes = viewOf('changes');
    const commit = viewOf('commit');
    changes.send(read());
    commit.send(read());
    const changed = read({ workingTree: workingTreeOf({ untracked: ['new.txt'] }), changes: 1 });
    expect(changes.send(changed)?.parts).toEqual(['workingTree']);
    expect(commit.send(changed)).toBeUndefined();
    // The settings the views follow are not the labels of a commit.
    expect(commit.send(read({ ...changed, oneLane: true }))).toBeUndefined();
    expect(changes.send(read({ ...changed, oneLane: true }))?.parts).toEqual(['rest']);
  });

  it('keeps the parts that were not sent again the same objects, and drops a field a part sent again no longer has', () => {
    const graph = viewOf('graph');
    graph.send(read({ notes: { reference: 'refs/notes/commits', tip: 'n1', commits: ['c1'] } }));
    const before = graph.shown;
    graph.send(read({ notes: { reference: 'refs/notes/commits', tip: 'n1', commits: ['c1'] }, workingTree: workingTreeOf({ untracked: ['new.txt'] }), changes: 1 }));
    const after = graph.shown;
    expect(after.changes).toBe(1);
    expect(after.branches).toBe(before.branches);
    expect(after.tags).toBe(before.tags);
    expect(after.rows).toBe(before.rows);
    expect(after.head).toBe(before.head);

    graph.send(read({ workingTree: workingTreeOf({ untracked: ['new.txt'] }), changes: 1 }));
    expect(graph.shown).not.toHaveProperty('notes');
  });

  it('sends the commits a graph has not got yet, after those it has, and none when they are the same', () => {
    const graph = viewOf('graph');
    const first = graph.send(read())!;
    expect(first.kept).toBeUndefined();
    expect(first.state?.rows?.map((each) => each.hash)).toEqual(['c3', 'c2', 'c1']);

    const more = graph.send(read({ rows: [row('c3', ['c2']), row('c2', ['c1']), row('c1', ['c0']), row('c0')], hasMore: false }))!;
    expect(more.kept).toBe(3);
    expect(more.state?.rows?.map((each) => each.hash)).toEqual(['c0']);
    expect(graph.shown.rows.map((each) => each.hash)).toEqual(['c3', 'c2', 'c1', 'c0']);

    // A new commit on top, with the rows under it drawn as they were: it is sent alone, before the commits the graph has.
    const before = graph.shown.rows;
    const committed = graph.send(read({ rows: [row('c4', ['c3']), row('c3', ['c2']), row('c2', ['c1']), row('c1', ['c0']), row('c0')] }))!;
    expect(committed.kept).toBe(0);
    expect(committed.keptLast).toBe(4);
    expect(committed.state?.rows?.map((each) => each.hash)).toEqual(['c4']);
    expect(graph.shown.rows.map((each) => each.hash)).toEqual(['c4', 'c3', 'c2', 'c1', 'c0']);
    expect(graph.shown.rows[1]).toBe(before[0]);

    // The same commits with other lines, as when a branch above them moves the lanes: those are sent again.
    const moved = { ...cell, col: 1 };
    const laid = graph.send(read({ rows: [row('c4', ['c3']), { ...row('c3', ['c2']), graph: moved }, { ...row('c2', ['c1']), graph: moved }, row('c1', ['c0']), row('c0')] }))!;
    expect(laid.kept).toBe(1);
    expect(laid.keptLast).toBe(2);
    expect(laid.state?.rows?.map((each) => each.hash)).toEqual(['c3', 'c2']);
    expect(graph.shown.rows.map((each) => each.graph.col)).toEqual([0, 1, 1, 0, 0]);

    // Another history: nothing is kept, and every commit is sent.
    const other = graph.send(read({ rows: [row('d2', ['d1']), row('d1')] }))!;
    expect(other.kept).toBeUndefined();
    expect(other.keptLast).toBeUndefined();
    expect(graph.shown.rows.map((each) => each.hash)).toEqual(['d2', 'd1']);
  });

  it('sends a graph the first page of a long history, and the rest behind it in pieces, never in one message (QA6)', () => {
    const graph = viewOf('graph', { first: 5, piece: 3 });
    const hashes = Array.from({ length: 12 }, (_, index) => `c${12 - index}`);
    const long = read({ rows: hashes.map((hash) => row(hash)), hasMore: false });
    const first = graph.send(long)!;
    expect(first.kept).toBeUndefined();
    expect(first.state?.rows?.map((each) => each.hash)).toEqual(hashes.slice(0, 5));
    // More are to come, though the history is read to its end: the list says so under its rows.
    expect(first.state?.hasMore).toBe(true);
    expect(first.parts).toEqual(partsOf('graph'));
    expect(graph.more).toBe(true);

    // The same state again brings the next piece, and nothing else of it.
    const pieces: StateMessage[] = [];
    while (graph.more) pieces.push(graph.send(long)!);
    expect(pieces.map((piece) => [piece.kept, piece.state?.rows?.length, piece.state?.hasMore])).toEqual([
      [5, 3, true],
      [8, 3, true],
      [11, 1, false],
    ]);
    for (const piece of pieces) expect(piece.parts).toEqual(['rows']);
    expect(graph.shown.rows.map((each) => each.hash)).toEqual(hashes);
    expect(graph.shown.hasMore).toBe(false);
    expect(graph.send(long)).toBeUndefined();

    // Pointed at another repository whose history is read: its first page only, whatever the view had.
    const others = Array.from({ length: 9 }, (_, index) => `d${9 - index}`);
    const pointed = graph.send(read({ root: '/work/web', name: 'web', rows: others.map((hash) => row(hash)) }))!;
    expect(pointed.kept).toBeUndefined();
    expect(pointed.state?.rows?.map((each) => each.hash)).toEqual(others.slice(0, 5));
    expect(graph.more).toBe(true);

    // A page read meanwhile is sent in its turn: the pieces follow the state as it is when each is made.
    const grown = read({ root: '/work/web', name: 'web', rows: [...others, 'd0'].map((hash) => row(hash)) });
    expect(graph.send(grown)?.state?.rows?.map((each) => each.hash)).toEqual(['d4', 'd3', 'd2']);
    expect(graph.send(grown)?.state?.rows?.map((each) => each.hash)).toEqual(['d1', 'd0']);
    expect(graph.more).toBe(false);
    expect(graph.shown.rows).toHaveLength(10);
  });

  it('sends the commits that differ in pieces too, and the ones kept at the end only when one message brings the view to them', () => {
    const graph = viewOf('graph', { first: 4, piece: 2 });
    const rows = ['c6', 'c5', 'c4', 'c3', 'c2', 'c1'].map((hash) => row(hash));
    graph.send(read({ rows }));
    while (graph.more) graph.send(read({ rows }));
    expect(graph.shown.rows).toHaveLength(6);

    // One row drawn otherwise: it is sent alone, between the others.
    const moved = { ...cell, col: 1 };
    const one = rows.map((each, index) => (index === 2 ? { ...each, graph: moved } : each));
    const patched = graph.send(read({ rows: one }))!;
    expect([patched.kept, patched.keptLast, patched.state?.rows?.length]).toEqual([2, 3, 1]);
    expect(graph.more).toBe(false);

    // More rows drawn otherwise than a piece carries: those after the ones kept at the start, a piece at a time.
    const many = rows.map((each, index) => (index >= 1 && index <= 4 ? { ...each, graph: { ...cell, col: 2 } } : each));
    const started = graph.send(read({ rows: many }))!;
    expect([started.kept, started.keptLast, started.state?.rows?.length, started.state?.hasMore]).toEqual([1, undefined, 2, true]);
    while (graph.more) graph.send(read({ rows: many }));
    expect(graph.shown.rows.map((each) => each.graph.col)).toEqual([0, 2, 2, 2, 2, 0]);
    expect(graph.shown.hasMore).toBe(false);
  });

  it('sends the row of the uncommitted changes apart from the commits, which are not sent again for another count of files', () => {
    const graph = viewOf('graph');
    const commits = [row('c3', ['c2']), row('c2', ['c1']), row('c1')];
    const first = graph.send(read({ rows: [draft(1), ...commits], changes: 1 }))!;
    expect(first.state?.draft?.subject).toBe('Uncommitted changes (1)');
    expect(first.state?.rows?.map((each) => each.hash)).toEqual(['c3', 'c2', 'c1']);
    expect(graph.shown.rows.map((each) => each.hash)).toEqual([WORKTREE, 'c3', 'c2', 'c1']);
    const rows = graph.shown.rows;

    const second = graph.send(read({ rows: [draft(2), ...commits], changes: 2 }))!;
    expect(second.parts).not.toContain('rows');
    expect(second.state?.draft?.subject).toBe('Uncommitted changes (2)');
    expect(graph.shown.draft?.subject).toBe('Uncommitted changes (2)');
    // The rows are the same list: nothing worked out from them is worked out again.
    expect(graph.shown.rows).toBe(rows);

    // More commits under the same: what is kept counts the commits alone.
    const more = graph.send(read({ rows: [draft(2), ...commits.slice(0, 2), row('c1', ['c0']), row('c0')], changes: 2 }))!;
    expect(more.kept).toBe(3);
    expect(graph.shown.rows.map((each) => each.hash)).toEqual([WORKTREE, 'c3', 'c2', 'c1', 'c0']);
  });

  it('sends a view that loaded again, and has nothing, every part again', () => {
    const stashes = viewOf('stashes');
    stashes.send(read());
    expect(stashes.send(read())).toBeUndefined();
    const again = viewOf('stashes');
    expect(again.send(read())?.parts).toEqual(partsOf('stashes'));
  });

  it('sends a graph moved to a worktree of its repository what is its own, and no commit it has (repo.share-history)', () => {
    const graph = viewOf('graph');
    const api = read();
    graph.send(api);
    const rows = graph.shown.rows;
    // The worktree lists the same commits: the same rows, another folder, another branch checked out.
    const worktree = read({
      root: '/work/app-fix',
      name: 'app-fix',
      head: headAt('c2', 'topic'),
      branches: [branchAt('main', 'c3'), branchAt('topic', 'c2', { isCheckedOut: true })],
      worktrees: [worktreeAt('/work/app', 'main', { isMain: true }), worktreeAt('/work/app-fix', 'topic', { isCurrent: true })],
      rows: api.rows,
    });
    const moved = graph.send(worktree)!;
    expect(moved.parts).toEqual(['repository', 'refs', 'worktrees']);
    expect(moved.state?.rows).toBeUndefined();
    expect(graph.shown.root).toBe('/work/app-fix');
    expect(graph.shown.head).toEqual(headAt('c2', 'topic'));
    // The rows are the same list: the graph draws none of them again.
    expect(graph.shown.rows).toBe(rows);
  });

  it('sends a graph moved to a worktree with other uncommitted changes only the row they are made on, between the rows it keeps (repo.share-history)', () => {
    const graph = viewOf('graph');
    const commits = [row('c3', ['c2']), row('c2', ['c1']), row('c1')];
    // The repository has changes not committed on `c3`: their line comes down into the row of `c3`.
    const under = { ...cell, top: [{ from: 0, to: 0, color: 0, dashed: true }] };
    graph.send(read({ rows: [draft(1), { ...commits[0], graph: under }, commits[1], commits[2]], changes: 1 }));
    const before = graph.shown.rows;
    expect(before.map((each) => each.hash)).toEqual([WORKTREE, 'c3', 'c2', 'c1']);

    // The worktree has none: the rows are laid out again without that line, and only the row of `c3` differs.
    const clean = graph.send(read({ root: '/work/app-fix', name: 'app-fix', rows: [row('c3', ['c2']), row('c2', ['c1']), row('c1')] }))!;
    expect([clean.kept, clean.keptLast]).toEqual([0, 2]);
    expect(clean.state?.rows?.map((each) => each.hash)).toEqual(['c3']);
    expect(clean.state?.draft).toBeNull();
    expect(graph.shown.rows.map((each) => each.hash)).toEqual(['c3', 'c2', 'c1']);
    expect(graph.shown.rows[0].graph.top).toEqual([]);
    expect(graph.shown.rows[1]).toBe(before[2]);

    // And back to the repository, whose row of the uncommitted changes comes with the row under it.
    const back = graph.send(read({ rows: [draft(1), { ...commits[0], graph: under }, commits[1], commits[2]], changes: 1 }))!;
    expect([back.kept, back.keptLast]).toEqual([0, 2]);
    expect(back.state?.rows?.map((each) => each.hash)).toEqual(['c3']);
    expect(graph.shown.rows.map((each) => each.hash)).toEqual([WORKTREE, 'c3', 'c2', 'c1']);
  });

  it('sends a graph the first step of a state whole, and of the rest only the parts it changed and no row (repo.read-first)', () => {
    const graph = viewOf('graph');
    const branches = viewOf('branches');
    // The first step: no remote read, and no count of what is not committed in the other worktree.
    const worktrees = (uncommitted: number | null) => [worktreeAt('/work/app', 'main', { isMain: true, isCurrent: true }), worktreeAt('/work/app-fix', 'topic', { uncommitted })];
    const first = read({ worktrees: worktrees(null) });
    expect(graph.send(first)?.parts).toEqual(partsOf('graph'));
    branches.send(first);
    const rows = graph.shown.rows;
    expect(graph.shown.worktrees[1].uncommitted).toBeNull();

    // The rest is read anew, every part of it: the remotes, the identity and the count of the other worktree differ.
    const whole = read({ worktrees: worktrees(2), remotes: ['origin'], identity: { name: 'Test Author', email: 'author@example.com' }, rows: first.rows });
    const rest = graph.send(whole)!;
    expect(rest.parts).toEqual(['repository', 'rest', 'worktrees']);
    expect(rest.state?.rows).toBeUndefined();
    expect(graph.shown.rows).toBe(rows);
    expect(graph.shown.remotes).toEqual(['origin']);
    expect(graph.shown.worktrees[1].uncommitted).toBe(2);
    // What the first step read stays the same objects: nothing made from it is made again.
    expect(graph.shown.branches).toBe(first.branches);
    expect(branches.send(whole)?.parts).toEqual(['repository', 'rest', 'worktrees']);
    // A rest that changes nothing, as in a repository with no remote and no other worktree, sends nothing.
    expect(graph.send(read({ worktrees: worktrees(2), remotes: ['origin'], identity: whole.identity, rows: first.rows }))).toBeUndefined();
  });
});

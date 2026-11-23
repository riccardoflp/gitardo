import { describe, expect, it } from 'vitest';
import { stateFor, type Sent } from '../src/extension/sentParts';
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

/** What identifies the first rows of a state: their hashes, as far as that. */
const keyOf = (state: RepoState) => (count: number) => state.rows.slice(0, count).map((each) => each.hash).join();

type StateMessage = Extract<HostMessage, { type: 'state' }>;

/** Sends one view the states one after the other, as `Controller.post` does, and keeps what the view has of them. */
function viewOf(kind: ViewKind) {
  let sent: Sent | undefined;
  let shown: ShownState | null = null;
  return {
    send(state: RepoState): StateMessage | undefined {
      const { message, sent: now } = stateFor(kind, state, sent, keyOf(state));
      sent = now;
      const posted = message as StateMessage | undefined;
      if (posted?.state) shown = withParts(shown, posted.state, posted.parts ?? [], posted.kept);
      return posted;
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

    // A new commit on top: the rows are others from the first, and are sent whole.
    const committed = graph.send(read({ rows: [row('c4', ['c3']), row('c3', ['c2']), row('c2', ['c1']), row('c1', ['c0']), row('c0')] }))!;
    expect(committed.kept).toBeUndefined();
    expect(committed.state?.rows).toHaveLength(5);
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

    // More commits under the same: what is kept counts the commits alone.
    const more = graph.send(read({ rows: [draft(2), ...commits.slice(0, 2), row('c1', ['c0']), row('c0')], changes: 2 }))!;
    expect(more.kept).toBe(3);
    expect(graph.shown.rows.map((each) => each.hash)).toEqual([WORKTREE, 'c3', 'c2', 'c1', 'c0']);
    expect(rows).toHaveLength(4);
  });

  it('sends a view that loaded again, and has nothing, every part again', () => {
    const stashes = viewOf('stashes');
    stashes.send(read());
    expect(stashes.send(read())).toBeUndefined();
    const again = viewOf('stashes');
    expect(again.send(read())?.parts).toEqual(partsOf('stashes'));
  });
});

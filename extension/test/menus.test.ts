import { describe, expect, it, vi } from 'vitest';
import type { Branch, RepoState, Row, Submodule, Worktree } from '../src/shared/protocol';
import type { ShownParts } from '../src/shared/repositories';
import type { Hosting, HostingService } from '../src/shared/hosting';
import { changesText, pullTitle, upstreamHint, worktreeOf, worktreeStatus } from '../src/shared/refs';
import { branchAt, headAt, remoteAt, tagAt, upstreamOf, workingTreeOf } from './states';

// The menus post what is chosen to the extension; here they are only read.
const posted = vi.hoisted(() => [] as unknown[]);
vi.mock('../src/webview/vscode', () => ({ post: (message: unknown) => posted.push(message) }));
const {
  branchMenu,
  canFormatPatch,
  canReword,
  canSquash,
  changedFileMenu,
  changesMenu,
  commitMenu,
  deleteOnRemoteEntries,
  dropMenu,
  hiddenFileMenu,
  hiddenGroupMenu,
  lockTitle,
  remoteNotesEntries,
  openBranch,
  openRemoteBranch,
  remoteBranchMenu,
  repositoryRowMenu,
  selectionMenu,
  tagMenu,
  trackingEntries,
  worktreeMenu,
} = await import('../src/webview/menus');

const worktree = (overrides: Partial<Worktree>): Worktree => ({
  folder: 'C:/prj/app',
  commit: 'aaa',
  branch: 'refs/heads/main',
  isMain: true,
  isCurrent: true,
  isMissing: false,
  uncommitted: 0,
  ...overrides,
});

const state = (worktrees: Worktree[], branches: Branch[], agentsWindow = true, openBranches: RepoState['openBranches'] = 'checkout') =>
  ({
    head: headAt('aaa', 'main'),
    branches,
    remoteBranches: [],
    tags: [],
    worktrees,
    remotes: [],
    favorites: [],
    pulls: {},
    hosting: null,
    scope: null,
    operation: null,
    agentsWindow,
    openBranches,
  }) as Partial<RepoState> as RepoState;

const labels = (entries: ReturnType<typeof branchMenu>) =>
  entries.flatMap((entry) => (entry && entry !== 'separator' ? [entry.label] : []));

describe('menus of branches that a worktree has', () => {
  const main: Branch = branchAt('main', 'aaa', { isCheckedOut: true });
  const login: Branch = branchAt('feature/login', 'bbb', { isCheckedOut: false });
  const plain: Branch = branchAt('plain', 'ccc', { isCheckedOut: false });
  const here = worktree({});
  const there = worktree({ folder: 'C:/prj/app.worktrees/feature/login', commit: 'bbb', branch: 'refs/heads/feature/login', isCurrent: false, isMain: false });
  const repo = state([here, there], [main, login, plain]);

  it('finds the other worktree that has a branch, and not this one', () => {
    expect(worktreeOf('feature/login', repo)).toBe(there);
    expect(worktreeOf('main', repo)).toBeUndefined();
    expect(worktreeOf('plain', repo)).toBeUndefined();
  });

  it('opens the worktree of a branch instead of checking it out, and removes it instead of deleting the branch', () => {
    const menu = labels(branchMenu(login, repo, () => {}));
    expect(menu.slice(0, 4)).toEqual(['Open worktree in Agents Window', 'Open worktree in new window', 'Open terminal in worktree', expect.stringMatching(/worktree/)]);
    expect(menu).not.toContain('Check out feature/login');
    expect(menu).not.toContain('Delete');
    expect(menu).not.toContain('Create worktree…');
    expect(menu).toEqual(expect.arrayContaining(['Rename branch…', 'Rename worktree…', 'Remove worktree…', 'Remove worktree and delete branch…']));

    posted.length = 0;
    const entries = branchMenu(login, repo, () => {});
    for (const entry of entries) if (entry && entry !== 'separator' && entry.label.startsWith('Remove worktree')) entry.run();
    expect(posted).toEqual([
      { type: 'action', action: { kind: 'worktreeRemove', path: there.folder, deleteBranch: false } },
      { type: 'action', action: { kind: 'worktreeRemove', path: there.folder, deleteBranch: true } },
    ]);
  });

  it('checks out a branch without a worktree, or opens it in one made for it, first as a double click does', () => {
    const menu = labels(branchMenu(plain, repo, () => {}));
    expect(menu.slice(0, 2)).toEqual(['Check out plain', 'Open in worktree']);
    expect(menu).toEqual(expect.arrayContaining(['Create worktree…', 'Rename…', 'Delete']));
    expect(menu.some((label) => /worktree in|Remove worktree/.test(label))).toBe(false);

    const worktrees = state([here, there], [main, login, plain], true, 'worktree');
    expect(labels(branchMenu(plain, worktrees, () => {})).slice(0, 2)).toEqual(['Open in worktree', 'Check out plain']);
    posted.length = 0;
    const open = branchMenu(plain, worktrees, () => {}).find((entry) => entry && entry !== 'separator' && entry.label === 'Open in worktree');
    if (open && open !== 'separator') open.run();
    expect(posted).toEqual([{ type: 'action', action: { kind: 'openInWorktree', name: 'plain' } }]);
    // The branch checked out here has no worktree to open.
    expect(labels(branchMenu(main, worktrees, () => {}))).not.toContain('Open in worktree');
  });

  it('opens the main worktree from another one, which cannot be removed', () => {
    const elsewhere = state([worktree({ isCurrent: false }), worktree({ ...there, isCurrent: true })], [{ ...main, isCheckedOut: false }, { ...login, isCheckedOut: true }], false);
    const menu = labels(branchMenu({ ...main, isCheckedOut: false }, elsewhere, () => {}));
    expect(menu[0]).toBe('Open worktree in new window');
    expect(menu.some((label) => /Remove worktree|Rename worktree/.test(label))).toBe(false);
  });

  it('offers no rebase of a branch another worktree has, by dragging it onto a commit', () => {
    const row = { hash: 'ddd', parents: [], subject: '', graph: {} } as unknown as Row;
    expect(dropMenu({ kind: 'branch', name: 'feature/login', hash: 'bbb', local: true, current: false }, row, repo)).toEqual([]);
    expect(labels(dropMenu({ kind: 'branch', name: 'plain', hash: 'ccc', local: true, current: false }, row, repo))).toEqual(['Rebase plain onto ddd']);
  });

  it('says what a worktree menu opens without naming the worktree', () => {
    expect(labels(worktreeMenu(there, repo)).slice(0, 3)).toEqual(['Open in Agents Window', 'Open in new window', 'Open in terminal']);
  });

  it('shows another worktree in the views of this window, and keeps the folder of the window from being removed there', () => {
    expect(labels(worktreeMenu(there, repo))).toContain('Show in Gitardo');
    expect(labels(worktreeMenu(here, repo))).not.toContain('Show in Gitardo');
    expect(labels(branchMenu(login, repo, () => {}))).toContain('Show worktree in Gitardo');
    posted.length = 0;
    const show = worktreeMenu(there, repo).find((entry) => entry && entry !== 'separator' && entry.label === 'Show in Gitardo');
    if (show && show !== 'separator') show.run();
    expect(posted).toEqual([{ type: 'selectRepo', root: there.folder }]);

    // Seen from the worktree of feature/login, the folder of the window is a worktree like another, but its own.
    const window = worktree({ folder: 'C:/prj/feature', branch: 'refs/heads/feature', isCurrent: false, isMain: false });
    const shown = { ...state([worktree({}), { ...there, isCurrent: true }, window], [main, login]), home: { root: 'C:\\prj\\feature', name: 'feature' } } as RepoState;
    expect(labels(worktreeMenu(window, shown))).not.toEqual(expect.arrayContaining(['Rename…']));
    expect(labels(worktreeMenu(window, shown))).not.toContain('Remove…');
    expect(labels(branchMenu(branchAt('feature', 'aaa', { isCheckedOut: false }), shown, () => {}))).not.toContain('Remove worktree…');
  });

  it('worktree.lock: offers Lock… on a worktree, Unlock on a locked one, and no prune while the only missing one is locked', () => {
    expect(labels(worktreeMenu(there, repo))).toContain('Lock…');
    expect(labels(worktreeMenu(worktree({}), repo))).not.toContain('Lock…');
    const locked = { ...there, isMissing: true, locked: 'On the USB drive' };
    const lockedRepo = state([worktree({}), locked], [main, login]);
    const menu = labels(worktreeMenu(locked, lockedRepo));
    expect(menu).toContain('Unlock');
    expect(menu).not.toContain('Lock…');
    expect(menu).not.toContain('Prune missing worktrees');
    posted.length = 0;
    const unlock = worktreeMenu(locked, lockedRepo).find((entry) => entry && entry !== 'separator' && entry.label === 'Unlock');
    if (unlock && unlock !== 'separator') unlock.run();
    expect(posted).toEqual([{ type: 'action', action: { kind: 'worktreeUnlock', path: there.folder } }]);
    expect(lockTitle(locked)).toBe('Locked: On the USB drive');
    expect(lockTitle({ ...there, locked: '' })).toBe('Locked');
  });
});

describe('menus of remote branches', () => {
  const upstream = upstreamOf('origin/feature', { ahead: 1, behind: 2 });
  const main: Branch = branchAt('main', 'aaa', { isCheckedOut: true });
  const feature: Branch = branchAt('feature', 'bbb', { isCheckedOut: false, upstream });
  const ref = remoteAt('origin/feature', 'ccc');
  const repo = (branches: Branch[], openBranches: RepoState['openBranches'] = 'checkout') =>
    ({ ...state([worktree({})], branches, true, openBranches), remoteBranches: [ref], remotes: ['origin'] }) as RepoState;

  it('checks out the local branch of its name, and offers to rebase it when each has commits the other has not', () => {
    expect(labels(remoteBranchMenu(ref, repo([main]), () => {})).slice(0, 2)).toEqual(['Check out as local branch', 'Open in worktree']);
    expect(labels(remoteBranchMenu(ref, repo([main, { ...feature, upstream: { ...upstream, ahead: 0 } }]), () => {})).slice(0, 3)).toEqual([
      'Check out feature',
      'Open in worktree',
      'Merge into main',
    ]);
    const menu = labels(remoteBranchMenu(ref, repo([main, feature]), () => {}));
    expect(menu.slice(0, 3)).toEqual(['Check out feature', 'Open in worktree', 'Check out feature and rebase it onto this']);

    posted.length = 0;
    const rebase = remoteBranchMenu(ref, repo([main, feature]), () => {}).find((entry) => entry && entry !== 'separator' && entry.label.includes('rebase it'));
    if (rebase && rebase !== 'separator') rebase.run();
    expect(posted).toEqual([{ type: 'action', action: { kind: 'checkout', target: 'remote', ref: 'origin/feature', rebase: true } }]);
  });

  it('checks out a branch on a double click, or opens it in a worktree, and does nothing for the one checked out', () => {
    posted.length = 0;
    openRemoteBranch(ref, repo([main]))?.();
    openRemoteBranch(ref, repo([main], 'worktree'))?.();
    openBranch(feature, repo([main, feature]))?.();
    expect(posted).toEqual([
      { type: 'action', action: { kind: 'checkout', target: 'remote', ref: 'origin/feature' } },
      { type: 'action', action: { kind: 'openInWorktree', name: 'origin/feature', remote: true } },
      { type: 'action', action: { kind: 'checkout', target: 'branch', ref: 'feature' } },
    ]);
    expect(openBranch(main, repo([main]))).toBeUndefined();
  });

  it('opens the worktree that has the local branch of its name, instead of checking it out', () => {
    const there = worktree({ folder: 'C:/prj/app.worktrees/feature', commit: 'bbb', branch: 'refs/heads/feature', isCurrent: false, isMain: false });
    const repoThere = { ...repo([main, feature]), worktrees: [worktree({}), there] } as RepoState;
    const menu = labels(remoteBranchMenu(ref, repoThere, () => {}));
    expect(menu.slice(0, 2)).toEqual(['Open worktree in Agents Window', 'Open worktree in new window']);
    expect(menu.some((label) => /^Check out|rebase it/.test(label))).toBe(false);

    posted.length = 0;
    openRemoteBranch(ref, repoThere)?.();
    openRemoteBranch(ref, { ...repoThere, openBranches: 'worktree' })?.();
    expect(posted).toEqual([
      { type: 'action', action: { kind: 'worktreeOpen', path: there.folder } },
      { type: 'action', action: { kind: 'worktreeOpen', path: there.folder } },
    ]);
    // Its folder is gone: there is nothing to open, and the double click opens the commit.
    const missing = { ...repoThere, worktrees: [worktree({}), { ...there, isMissing: true }] } as RepoState;
    expect(openRemoteBranch(ref, missing)).toBeUndefined();
  });
});

describe('upstreams and tags on remotes', () => {
  it('sets the upstream of a branch that follows nothing, and changes or stops tracking one that follows a remote branch', () => {
    const upstream = upstreamOf('origin/work', { ahead: 0, behind: 0 });
    expect(labels(trackingEntries(branchAt('work', 'bbb', { isCheckedOut: false }), { remotes: ['origin'] }))).toEqual(['Set upstream…']);
    expect(labels(trackingEntries(branchAt('work', 'bbb', { isCheckedOut: false }), { remotes: [] }))).toEqual([]);
    expect(labels(trackingEntries(branchAt('work', 'bbb', { isCheckedOut: false, upstream }), { remotes: ['origin'] }))).toEqual([
      'Change upstream…',
      'Stop tracking origin/work',
    ]);
    posted.length = 0;
    const stop = trackingEntries(branchAt('work', 'bbb', { isCheckedOut: false, upstream }), { remotes: ['origin'] })[1];
    if (stop && stop !== 'separator') stop.run();
    expect(posted).toEqual([{ type: 'action', action: { kind: 'unsetUpstream', name: 'work' } }]);
  });

  it('deletes a tag from the one remote by its name, or from a remote asked which, and here too', () => {
    expect(labels(deleteOnRemoteEntries('v1', { remotes: ['origin'] }))).toEqual(['Delete from origin', 'Delete here and from origin']);
    expect(labels(deleteOnRemoteEntries('v1', { remotes: ['origin', 'upstream'] }))).toEqual(['Delete from a remote…', 'Delete here and from a remote…']);
    expect(deleteOnRemoteEntries('v1', { remotes: [] })).toEqual([]);
    posted.length = 0;
    const both = deleteOnRemoteEntries('v1', { remotes: ['origin'] })[1];
    if (both && both !== 'separator') both.run();
    expect(posted).toEqual([{ type: 'action', action: { kind: 'deleteTagOnRemote', name: 'v1', here: true } }]);
  });
});

describe('the status of a worktree', () => {
  const upstream = upstreamOf('origin/feature/login', { ahead: 2, behind: 1 });
  const login: Branch = branchAt('feature/login', 'bbb', { isCheckedOut: false, upstream });
  const there = worktree({ folder: 'C:/prj/app.worktrees/feature/login', commit: 'bbb', branch: 'refs/heads/feature/login', isCurrent: false, isMain: false, uncommitted: 3 });
  const pull = { number: 12, title: 'Login', url: '', draft: false, checks: null };

  it('says how its branch stands against its upstream and what is not committed', () => {
    expect(upstreamHint(login)).toBe('↑2 ↓1');
    expect(upstreamHint({ ...login, upstream: { ...upstream, ahead: 0, behind: 0 } })).toBeUndefined();
    expect(upstreamHint({ ...login, upstream: { ...upstream, isGone: true } })).toBe('gone');
    expect(upstreamHint({ ...login, upstream: null })).toBeUndefined();
    expect([changesText(null), changesText(0), changesText(1), changesText(3)]).toEqual([
      undefined,
      'Nothing to commit',
      '1 file not committed',
      '3 files not committed',
    ]);
  });

  it('finds its branch and the pull request of the branch', () => {
    const hosting = { service: 'gitHub', host: 'github.com', remote: 'origin', canCheckOut: true, comparesSinceDiverged: false } as const;
    const repo = { ...state([worktree({}), there], [login]), remotes: ['origin'], hosting, pulls: { 'feature/login': pull } } as RepoState;
    expect(worktreeStatus(there, repo)).toEqual({ branch: login, ahead: '↑2 ↓1', pull });
    expect(worktreeStatus({ ...there, branch: null }, repo)).toEqual({});
  });
});

describe('menu of the changes that are not committed', () => {
  const file = (path: string) => ({ path, originalPath: null, kind: 'modified' as const });
  const changes = (workingTree: RepoState['workingTree'], more: Partial<RepoState> = {}) =>
    labels(changesMenu({ ...state([], []), workingTree, ...more } as RepoState));

  it('stashes, stages, unstages and discards them all, as far as there is something to', () => {
    expect(changes(workingTreeOf({ staged: [file('a.txt')], untracked: ['b.txt'] }))).toEqual([
      'Stash changes…',
      'Stash staged changes…',
      'Stage all changes',
      'Unstage all changes',
      'Discard all changes…',
    ]);
    expect(changes(workingTreeOf({ unstaged: [file('a.txt')] }))).toEqual(['Stash changes…', 'Stage all changes', 'Discard all changes…']);
  });

  it('leaves out stashing before the first commit, staging over conflicts, and discarding during a merge', () => {
    expect(changes(workingTreeOf({ untracked: ['a.txt'] }), { head: headAt(null) })).toEqual(['Stage all changes', 'Discard all changes…']);
    expect(changes(workingTreeOf({ staged: [file('a.txt')], unstaged: [file('c.txt')], conflicted: [{ path: 'c.txt', kind: 'bothModified' }] }), { operation: 'merge' })).toEqual([
      'Stash changes…',
      'Stash staged changes…',
      'Unstage all changes',
    ]);
  });
});

describe('changing the message of a commit', () => {
  const row = (hash: string, parents: string[]): Row => ({ hash, parents, author: '', email: '', date: 0, subject: hash, graph: [] as never });
  const rows = [row('d', ['c']), row('c', ['b']), row('b', ['a', 'x']), row('a', [])];
  const repo = (more: Partial<RepoState> = {}) => ({ ...state([], []), head: headAt('d', 'main'), rows, workingTree: workingTreeOf(), bisect: null, ...more }) as RepoState;
  const tools = { selected: null, onInteractiveRebase: () => {}, onCompare: () => {}, onEditMessage: () => {} };

  it('is offered on the commits down to the first merge, and not while something else is going on', () => {
    expect(['d', 'c', 'b', 'a'].map((hash) => canReword(hash, repo()))).toEqual([true, true, false, false]);
    expect(canReword('d', repo({ operation: 'merge' }))).toBe(false);
    // The last commit is amended, merge or not.
    expect(canReword('b', repo({ head: headAt('b', 'main'), rows: rows.slice(2) }))).toBe(true);
    expect(labels(commitMenu(rows[1], repo(), tools))).toContain('Edit message…');
    expect(labels(commitMenu(rows[2], repo(), tools))).not.toContain('Edit message…');
  });
});

describe('notes and archives', () => {
  const row = (hash: string, parents: string[]): Row => ({ hash, parents, author: '', email: '', date: 0, subject: hash, graph: [] as never });
  const rows = [row('d', ['c']), row('c', [])];
  const repo = (more: Partial<RepoState> = {}) => ({ ...state([], []), head: headAt('d', 'main'), rows, workingTree: workingTreeOf(), bisect: null, ...more }) as RepoState;
  const tools = { selected: null, onInteractiveRebase: () => {}, onCompare: () => {}, onEditMessage: () => {} };
  const chosen = (entries: ReturnType<typeof branchMenu>, label: string) => {
    posted.length = 0;
    for (const entry of entries) if (entry && entry !== 'separator' && entry.label === label) entry.run();
    return [...posted];
  };

  it('note.edit and note.remove: offers Add note… on a commit with none, Edit note… and Remove note on one with a note', () => {
    const plain = labels(commitMenu(rows[0], repo(), tools));
    expect(plain).toContain('Add note…');
    expect(plain).not.toContain('Remove note');
    const noted = repo({ notes: { reference: 'refs/notes/commits', tip: 'n1', commits: ['d'] } });
    expect(labels(commitMenu(rows[0], noted, tools))).toEqual(expect.arrayContaining(['Edit note…', 'Remove note']));
    expect(chosen(commitMenu(rows[0], noted, tools), 'Edit note…')).toEqual([{ type: 'action', action: { kind: 'editNote', hash: 'd' } }]);
    expect(chosen(commitMenu(rows[0], noted, tools), 'Remove note')).toEqual([{ type: 'action', action: { kind: 'removeNote', hash: 'd' } }]);
  });

  it('note.fetch and note.push: fetches the notes of each remote, and pushes them only while the repository has some', () => {
    expect(labels(remoteNotesEntries({ remotes: ['origin'], notes: undefined }))).toEqual(['Fetch notes from origin']);
    const entries = remoteNotesEntries({ remotes: ['origin'], notes: { reference: 'refs/notes/commits', tip: 'n1', commits: ['d'] } });
    expect(labels(entries)).toEqual(['Fetch notes from origin', 'Push notes to origin']);
    expect(chosen(entries, 'Push notes to origin')).toEqual([{ type: 'action', action: { kind: 'pushNotes', remote: 'origin' } }]);
    expect(chosen(entries, 'Fetch notes from origin')).toEqual([{ type: 'action', action: { kind: 'fetchNotes', remote: 'origin' } }]);
  });

  it('archive.save: saves a commit, a branch, a remote branch and a tag by their full names', () => {
    const main = branchAt('main', 'd', { isCheckedOut: true });
    const shown = repo({ branches: [main] });
    expect(chosen(commitMenu(rows[1], shown, tools), 'Save as archive…')).toEqual([{ type: 'action', action: { kind: 'saveArchive', at: 'c' } }]);
    expect(chosen(branchMenu(main, shown, () => {}), 'Save as archive…')).toEqual([
      { type: 'action', action: { kind: 'saveArchive', at: 'refs/heads/main' } },
    ]);
    const remote = remoteAt('origin/main', 'd');
    expect(chosen(remoteBranchMenu(remote, shown, () => {}), 'Save as archive…')).toEqual([
      { type: 'action', action: { kind: 'saveArchive', at: 'refs/remotes/origin/main' } },
    ]);
    expect(chosen(tagMenu(tagAt('v1.0', 'c'), shown, () => {}), 'Save as archive…')).toEqual([
      { type: 'action', action: { kind: 'saveArchive', at: 'refs/tags/v1.0' } },
    ]);
  });
});

describe('squashing commits', () => {
  const row = (hash: string, parents: string[]): Row => ({ hash, parents, author: '', email: '', date: 0, subject: hash, graph: [] as never });
  const rows = [row('e', ['d']), row('d', ['c']), row('c', ['b']), row('b', ['a', 'x']), row('a', [])];
  const repo = (more: Partial<RepoState> = {}) => ({ ...state([], []), head: headAt('e', 'main'), rows, workingTree: workingTreeOf(), bisect: null, ...more }) as RepoState;
  const tools = { selected: null, onInteractiveRebase: () => {}, onCompare: () => {}, onEditMessage: () => {} };
  const pick = (...hashes: string[]) => rows.filter((each) => hashes.includes(each.hash));

  it('is offered for commits that follow one another before HEAD, down to the first merge', () => {
    expect(canSquash(pick('e', 'd'), repo())).toBe(true);
    expect(canSquash(pick('d', 'c'), repo())).toBe(true);
    expect(canSquash(pick('e', 'c'), repo())).toBe(false);
    expect(canSquash(pick('c', 'b'), repo())).toBe(false);
    expect(canSquash(pick('e'), repo())).toBe(false);
    expect(canSquash(pick('e', 'd'), repo({ operation: 'rebase' }))).toBe(false);
    expect(labels(selectionMenu(pick('e', 'd', 'c'), repo(), () => {}))).toContain('Squash 3 commits into one…');
  });

  it('squashes a commit into the one before it when that one is on the same line', () => {
    expect(labels(commitMenu(rows[1], repo(), tools))).toContain('Squash into the commit before it…');
    expect(labels(commitMenu(rows[2], repo(), tools))).not.toContain('Squash into the commit before it…');
    posted.length = 0;
    const entry = commitMenu(rows[0], repo(), tools).find((each) => each && each !== 'separator' && each.label === 'Squash into the commit before it…');
    if (entry && entry !== 'separator') entry.run();
    expect(posted).toEqual([{ type: 'action', action: { kind: 'squash', hashes: ['d', 'e'] } }]);
  });
});

describe('ways to merge', () => {
  const main: Branch = branchAt('main', 'aaa', { isCheckedOut: true });
  const feature: Branch = branchAt('feature', 'bbb', { isCheckedOut: false });
  const repo = { ...state([worktree({})], [main, feature]), rows: [] } as unknown as RepoState;
  const choose = (entries: ReturnType<typeof branchMenu>, label: string) => {
    posted.length = 0;
    const entry = entries.find((each) => each && each !== 'separator' && each.label === label);
    if (entry && entry !== 'separator') entry.run();
    return posted;
  };

  it('merges with one click as git does by default, and asks for the other ways with the entry after it', () => {
    const menu = branchMenu(feature, repo, () => {});
    expect(labels(menu)).toEqual(expect.arrayContaining(['Merge into main', 'Merge into main…']));
    expect(labels(menu).indexOf('Merge into main…')).toBe(labels(menu).indexOf('Merge into main') + 1);
    expect(choose(menu, 'Merge into main')).toEqual([{ type: 'action', action: { kind: 'merge', ref: 'feature' } }]);
    expect(choose(menu, 'Merge into main…')).toEqual([{ type: 'action', action: { kind: 'merge', ref: 'feature', mode: 'ask' } }]);
  });

  it('offers the same when a branch is dropped on what is checked out', () => {
    const head = { hash: 'aaa', parents: [], subject: '', graph: {} } as unknown as Row;
    const menu = dropMenu({ kind: 'branch', name: 'feature', hash: 'bbb', local: true, current: false }, head, repo);
    expect(labels(menu)).toEqual(['Merge feature into main', 'Merge feature into main…', 'Rebase main onto feature']);
    expect(choose(menu, 'Merge feature into main…')).toEqual([{ type: 'action', action: { kind: 'merge', ref: 'feature', mode: 'ask' } }]);
  });
});

describe('commits as a patch', () => {
  const row = (hash: string, parents: string[]): Row => ({ hash, parents, author: '', email: '', date: 0, subject: hash, graph: [] as never });
  const rows = [row('d', ['c']), row('c', ['b', 'x']), row('b', ['a']), row('a', [])];
  const repo = { ...state([], []), head: headAt('d', 'main'), rows, workingTree: workingTreeOf(), bisect: null } as RepoState;
  const tools = { selected: null, onInteractiveRebase: () => {}, onCompare: () => {}, onEditMessage: () => {} };

  it('is offered for commits that are not merges, the first one included', () => {
    expect(labels(commitMenu(rows[0], repo, tools))).toEqual(expect.arrayContaining(['Copy as patch', 'Save as patch…']));
    expect(labels(commitMenu(rows[3], repo, tools))).toContain('Copy as patch');
    expect(labels(commitMenu(rows[1], repo, tools))).not.toContain('Copy as patch');
    expect(canFormatPatch([rows[0], rows[2]])).toBe(true);
    expect(canFormatPatch([rows[0], rows[1]])).toBe(false);
    // A stash keeps only its first parent in the graph, and is a merge all the same.
    expect(canFormatPatch([{ ...row('s', ['d']), stash: 'stash@{0}' }])).toBe(false);
  });

  it('asks for the selected commits oldest first', () => {
    posted.length = 0;
    const entries = selectionMenu([rows[0], rows[2], rows[3]], repo, () => {});
    for (const entry of entries) if (entry && entry !== 'separator' && / as patch/.test(entry.label)) entry.run();
    expect(posted).toEqual([
      { type: 'formatPatch', hashes: ['a', 'b', 'd'], save: false },
      { type: 'formatPatch', hashes: ['a', 'b', 'd'], save: true },
    ]);
  });
});

describe('the menu of a row of the Repositories view', () => {
  const main = worktree({});
  const login = worktree({ folder: 'C:/prj/app-login', commit: 'bbb', branch: 'refs/heads/feature/login', isCurrent: false, isMain: false });
  const core: Submodule = {
    name: 'lib/core',
    folder: 'lib/core',
    parent: null,
    url: 'https://example.com/core.git',
    branch: null,
    recorded: 'ccc',
    staged: null,
    checkout: { commit: 'ccc', branch: null, uncommitted: 0, distance: { ahead: 0, behind: 0 } },
  };
  const shown: ShownParts = { root: 'C:\\prj\\app', home: null, agentsWindow: false, worktrees: [main, login], submodules: [core] };
  const run = (entries: ReturnType<typeof repositoryRowMenu>, label: string | RegExp) => {
    posted.length = 0;
    for (const entry of entries) if (entry && entry !== 'separator' && (typeof label === 'string' ? entry.label === label : label.test(entry.label))) entry.run();
    return [...posted];
  };

  it('reveals the folder of a row of another repository, copies its path, opens a terminal there and shows it', () => {
    const menu = repositoryRowMenu({ folder: 'C:/prj/web', isShown: false, kind: 'repository' }, shown);
    expect(labels(menu)).toEqual(['Open in a new tab', 'Open in terminal', expect.stringMatching(/^Reveal in|^Open containing folder$/), 'Show in Gitardo', 'Copy path']);
    expect(run(menu, 'Open in terminal')).toEqual([{ type: 'action', action: { kind: 'worktreeTerminal', path: 'C:/prj/web' } }]);
    expect(run(menu, /^Reveal in|^Open containing folder$/)).toEqual([{ type: 'action', action: { kind: 'worktreeReveal', path: 'C:/prj/web' } }]);
    expect(run(menu, 'Show in Gitardo')).toEqual([{ type: 'selectRepo', root: 'C:/prj/web' }]);
    expect(run(menu, 'Copy path')).toEqual([{ type: 'copy', text: 'C:/prj/web' }]);
    // Before the repository shown is read, every row has these.
    expect(labels(repositoryRowMenu({ folder: 'C:/prj/app-login', isShown: true, kind: 'worktree' }, null))).toEqual([
      'Open in a new tab',
      'Open in terminal',
      expect.any(String),
      'Copy path',
    ]);
  });

  it('has what the menu of a worktree has in the Branches view, for a worktree of the repository shown', () => {
    const menu = repositoryRowMenu({ folder: 'c:\\prj\\app-login', isShown: false, kind: 'worktree' }, shown);
    expect(labels(menu)).toEqual(['Open in a new tab', ...labels(worktreeMenu(login, shown))]);
    expect(labels(menu)).toEqual(expect.arrayContaining(['Open in new window', 'Show in Gitardo', 'Rename…', 'Copy path', 'Bring ignored files…', 'Remove…']));
    expect(run(menu, 'Remove…')).toEqual([{ type: 'action', action: { kind: 'worktreeRemove', path: login.folder } }]);
    // The one shown is not shown again, nor removed from here.
    const own = labels(repositoryRowMenu({ folder: 'C:/prj/app', isShown: true, kind: 'repository' }, shown));
    expect(own).toContain('Copy path');
    expect(own).not.toEqual(expect.arrayContaining(['Show in Gitardo']));
    expect(own).not.toContain('Remove…');
  });

  it('has what the menu of a submodule has in the Branches view, for a submodule of the repository shown', () => {
    const menu = repositoryRowMenu({ folder: 'C:/prj/app/lib/core', isShown: false, kind: 'submodule' }, shown);
    expect(labels(menu)).toEqual(expect.arrayContaining(['Open in new window', 'Show in Gitardo', 'Update to the latest of its remote', 'Copy URL', 'Deinitialize…']));
    expect(run(menu, 'Show in Gitardo')).toEqual([{ type: 'selectRepo', root: 'C:\\prj\\app/lib/core' }]);
    expect(run(menu, 'Deinitialize…')).toEqual([{ type: 'action', action: { kind: 'submoduleDeinit', path: 'lib/core' } }]);
    // A submodule of another repository is a folder like another.
    expect(labels(repositoryRowMenu({ folder: 'C:/prj/web/lib/core', isShown: false, kind: 'submodule' }, shown))).toHaveLength(5);
  });

  it('repo.tab-repository: opens any row in a new tab of the graph, the one shown too', () => {
    for (const row of [
      { folder: 'C:/prj/web', isShown: false, kind: 'repository' },
      { folder: 'C:/prj/app', isShown: true, kind: 'repository' },
      { folder: 'c:\\prj\\app-login', isShown: false, kind: 'worktree' },
      { folder: 'C:/prj/app/lib/core', isShown: false, kind: 'submodule' },
    ] as const) {
      const menu = repositoryRowMenu(row, shown);
      expect(labels(menu)[0]).toBe('Open in a new tab');
      expect(run(menu, 'Open in a new tab')).toEqual([{ type: 'openInTab', root: row.folder }]);
    }
  });

  it('upkeep.clean-up and upkeep.start-background: the row of the repository shown alone cleans up and has git look after it', () => {
    const own = repositoryRowMenu({ folder: 'C:/prj/app', isShown: true, kind: 'repository' }, shown);
    expect(labels(own).slice(-2)).toEqual(['Clean up…', 'Look after it in the background…']);
    expect(run(own, 'Clean up…')).toEqual([{ type: 'action', action: { kind: 'cleanUp' } }]);
    expect(run(own, 'Look after it in the background…')).toEqual([{ type: 'action', action: { kind: 'lookAfter', start: true } }]);
    // Ticked while git looks after it, it stops.
    const lookedAfter = repositoryRowMenu({ folder: 'C:/prj/app', isShown: true, kind: 'repository' }, { ...shown, isLookedAfter: true });
    const entry = lookedAfter.find((each) => each && each !== 'separator' && each.label === 'Look after it in the background…');
    expect(entry && entry !== 'separator' && entry.checked).toBe(true);
    expect(run(lookedAfter, 'Look after it in the background…')).toEqual([{ type: 'action', action: { kind: 'lookAfter', start: false } }]);
    expect(labels(repositoryRowMenu({ folder: 'C:/prj/app-login', isShown: false, kind: 'worktree' }, shown))).not.toContain('Clean up…');
    expect(labels(repositoryRowMenu({ folder: 'C:/prj/web', isShown: false, kind: 'repository' }, shown))).not.toContain('Clean up…');
  });

  it('upkeep.write-commit-graph: Speed up reading the history comes after Clean up… while git keeps no commit-graph file and would read one', () => {
    const row = { folder: 'C:/prj/app', isShown: true, kind: 'repository' } as const;
    const missing = repositoryRowMenu(row, { ...shown, commitGraph: { hasFile: false, isIgnored: false } });
    const at = labels(missing).indexOf('Clean up…');
    expect(labels(missing)[at + 1]).toBe('Speed up reading the history');
    expect(run(missing, 'Speed up reading the history')).toEqual([{ type: 'action', action: { kind: 'writeCommitGraph' } }]);
    expect(labels(repositoryRowMenu(row, { ...shown, commitGraph: { hasFile: true, isIgnored: false } }))).not.toContain('Speed up reading the history');
    expect(labels(repositoryRowMenu(row, { ...shown, commitGraph: { hasFile: false, isIgnored: true } }))).not.toContain('Speed up reading the history');
    expect(labels(repositoryRowMenu(row, shown))).not.toContain('Speed up reading the history');
  });
});

describe('the pull requests of the hosting services in the menus of branches (hosting.open-pull-request, hosting.checkout-pull-request)', () => {
  const on = (service: HostingService): Hosting => {
    const canCheckOut = service !== 'bitbucket' && service !== 'azureDevOps';
    return { service, host: 'example.test', remote: 'origin', canCheckOut, comparesSinceDiverged: false };
  };
  const feature: Branch = branchAt('feature', 'bbb', { isCheckedOut: false, upstream: upstreamOf('origin/feature') });
  const ref = remoteAt('origin/feature', 'bbb');
  const merge = { number: 12, title: 'Login', url: 'https://gitlab.com/group/app/-/merge_requests/12', draft: false, checks: null, service: 'gitLab' as const };
  const repo = (hosting: Hosting | null, pulls: RepoState['pulls'] = {}) =>
    ({ ...state([worktree({})], [feature]), remoteBranches: [ref], remotes: ['origin'], hosting, pulls }) as RepoState;

  it('opens the merge request of a branch on GitLab by its number, or creates one', () => {
    expect(labels(branchMenu(feature, repo(on('gitLab'), { feature: merge }), () => {}))).toContain('Open merge request !12');
    expect(labels(branchMenu(feature, repo(on('gitLab')), () => {}))).toContain('Create merge request…');
    expect(pullTitle(merge)).toBe('Merge request !12: Login');
  });

  it('creates a pull request on Bitbucket, opening none, and has neither entry for a branch never pushed or on no service', () => {
    const menu = labels(branchMenu(feature, repo(on('bitbucket')), () => {}));
    expect(menu).toContain('Create pull request…');
    expect(menu.some((label) => label.startsWith('Open pull request'))).toBe(false);
    const unpushed = { ...feature, upstream: null };
    expect(labels(branchMenu(unpushed, repo(on('gitLab')), () => {})).some((label) => /request/.test(label))).toBe(false);
    expect(labels(branchMenu(feature, repo(null), () => {})).some((label) => /request/.test(label))).toBe(false);
  });

  it('checks out the pull request a remote branch has on its label, where the service keeps a reference of each', () => {
    posted.length = 0;
    const entries = remoteBranchMenu(ref, repo(on('gitLab'), { feature: merge }), () => {});
    const entry = entries.find((each) => each && each !== 'separator' && each.label === 'Check out merge request !12');
    if (!entry || entry === 'separator') throw new Error('no entry');
    entry.run();
    expect(posted).toEqual([{ type: 'checkOutPullRequest', number: 12, remoteBranch: 'origin/feature' }]);
    const pull = { ...merge, service: 'gitHub' as const };
    expect(labels(remoteBranchMenu(ref, repo(on('gitHub'), { feature: pull }), () => {}))).toContain('Check out pull request #12');
    expect(labels(remoteBranchMenu(ref, repo(on('gitHub')), () => {})).some((label) => label.startsWith('Check out pull'))).toBe(false);
  });
});

describe('menus of the files of the Gitardo Changes view', () => {
  const file = (kind: 'modified' | 'deleted' | 'added' = 'modified') => ({ path: 'config/local.json', originalPath: null, kind });
  const chosen = (entries: ReturnType<typeof branchMenu>, label: string) => {
    posted.length = 0;
    const entry = entries.find((each) => each && each !== 'separator' && each.label === label);
    if (entry && entry !== 'separator') entry.run();
    return posted;
  };

  it('changes.row-menu: a tracked file offers what its buttons do, then Ignore…, Stop tracking and Hide its changes from git…', () => {
    const menu = changedFileMenu({ file: file(), staged: 'none', untracked: false, unsaved: false }, false);
    expect(labels(menu)).toEqual([
      'Open changes',
      'Open file',
      'Stage',
      'Discard changes',
      'Stash this file…',
      'Ignore…',
      'Stop tracking, keep on disk…',
      'Hide its changes from git…',
    ]);
    expect(chosen(menu, 'Ignore…')).toEqual([{ type: 'action', action: { kind: 'ignore', path: 'config/local.json' } }]);
    expect(chosen(menu, 'Stop tracking, keep on disk…')).toEqual([{ type: 'action', action: { kind: 'stopTracking', path: 'config/local.json' } }]);
    expect(chosen(menu, 'Hide its changes from git…')).toEqual([{ type: 'action', action: { kind: 'hideFromGit', path: 'config/local.json' } }]);
  });

  it('changes.row-menu: an untracked file can only be ignored, a deleted one not even that, and nothing changes while an operation runs', () => {
    const untracked = labels(changedFileMenu({ file: file('added'), staged: 'none', untracked: true, unsaved: true }, false));
    expect(untracked).toContain('Ignore…');
    expect(untracked).not.toContain('Stop tracking, keep on disk…');
    expect(untracked).not.toContain('Hide its changes from git…');
    expect(untracked).toContain('Save');
    const deleted = labels(changedFileMenu({ file: file('deleted'), staged: 'all', untracked: false, unsaved: false }, false));
    expect(deleted).toContain('Unstage');
    expect(deleted).not.toContain('Open file');
    expect(deleted).not.toContain('Ignore…');
    const busy = changedFileMenu({ file: file(), staged: 'none', untracked: false, unsaved: false }, true);
    const disabled = busy.flatMap((entry) => (entry && entry !== 'separator' && entry.disabled ? [entry.label] : []));
    expect(disabled).toEqual(['Stage', 'Discard changes', 'Stash this file…', 'Ignore…', 'Stop tracking, keep on disk…', 'Hide its changes from git…']);
    const onlyEdited = labels(changedFileMenu({ file: file(), staged: null, untracked: false, unsaved: true }, false));
    expect(onlyEdited).toEqual(['Save', 'Compare with the saved file']);
  });

  it('changes.skipped-list: a hidden file opens its diff with the last commit and shows itself to git again, and the heading all of them', () => {
    const menu = hiddenFileMenu('config/local.json', false);
    expect(labels(menu)).toEqual(['Open changes', 'Open file', 'Show its changes to git again']);
    expect(chosen(menu, 'Open changes')).toEqual([
      { type: 'openDiff', source: { kind: 'worktree' }, file: { path: 'config/local.json', originalPath: null, kind: 'modified' } },
    ]);
    expect(chosen(menu, 'Show its changes to git again')).toEqual([{ type: 'action', action: { kind: 'showToGit', path: 'config/local.json' } }]);
    expect(chosen(hiddenGroupMenu(false), 'Show all to git again')).toEqual([{ type: 'action', action: { kind: 'showToGit' } }]);
  });
});

import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { RepoState } from '../src/shared/protocol';
import type { MenuEntry, MenuItem } from '../src/webview/ContextMenu';

// The menus post what is chosen to the extension; here it is kept to be read.
const posted = vi.hoisted(() => [] as unknown[]);
vi.mock('../src/webview/vscode', () => ({ post: (message: unknown) => posted.push(message) }));
const { moreMenu, viewMenu } = await import('../src/webview/toolbarMenus');

/** A repository with a merge loaded, a remote on GitHub and no submodule, with the switches of the graph as they start. */
const state = (overrides: Partial<RepoState> = {}) =>
  ({
    scope: null,
    showStashes: true,
    firstParent: false,
    oneLane: false,
    compactRows: true,
    traceSelected: true,
    dateSeparators: true,
    combineUpstream: true,
    graphColumns: ['author', 'date', 'hash'],
    rows: [{ parents: ['aaa', 'bbb'] }, { parents: ['aaa'] }],
    submodules: [],
    repos: [{}, {}],
    home: null,
    hosting: { service: 'gitHub', host: 'github.com', remote: 'origin', canCheckOut: true, comparesSinceDiverged: false },
    ...overrides,
  }) as Partial<RepoState> as RepoState;

/** The words of a menu, with `-` for a separator, without the entries its condition left out. */
const words = (entries: MenuEntry[]) => entries.flatMap((entry) => (entry ? [entry === 'separator' ? '-' : entry.label] : []));

const item = (entries: MenuEntry[], label: string): MenuItem => {
  const found = entries.find((entry) => entry && entry !== 'separator' && entry.label === label);
  if (!found || found === 'separator') throw new Error(`no ${label} in the menu`);
  return found;
};

const VIEW = [
  'Show local branches',
  'Show remote branches',
  'Show tags',
  'Show stashes',
  'Show only the first parent of merges',
  '-',
  'Show all commits on one line',
  'Compact rows',
  'Bring out the history of the selected commit',
  'Separate the days, weeks and months',
  'Put a branch and its upstream on one label',
  'Fold all merges',
  'Unfold all merges',
  '-',
  'Show the author',
  'Show the date',
  'Show the hash',
];

const MORE = [
  'Compare branches, tags or commits…',
  'Go to commit…',
  'History of the active file',
  'Browse reflog…',
  'Clean up branches…',
  'Apply patch…',
  'Manage remotes…',
  'Add submodule…',
  'Select repository…',
  '-',
  'Switch identity…',
  '-',
  'Load pull requests from GitHub',
  'Check out a pull request…',
];

beforeEach(() => {
  posted.length = 0;
});

describe('the menu of the view of the toolbar', () => {
  const folds = { onFoldAll: () => {}, folded: 'none' as const };

  it('has every switch of the graph, in the groups and the order of Gitardo Desktop', () => {
    expect(words(viewMenu(state(), folds))).toEqual(VIEW);
  });

  it('ticks each switch while it is on', () => {
    const menu = viewMenu(state({ firstParent: true, graphColumns: ['author', 'date'], scope: { kind: 'kinds', kinds: ['branch', 'tag'] } }), folds);
    const ticked = menu.flatMap((entry) => (entry && entry !== 'separator' && entry.checked ? [entry.label] : []));
    expect(ticked).toEqual([
      'Show local branches',
      'Show tags',
      'Show stashes',
      'Show only the first parent of merges',
      'Compact rows',
      'Bring out the history of the selected commit',
      'Separate the days, weeks and months',
      'Put a branch and its upstream on one label',
      'Show the author',
      'Show the date',
    ]);
  });

  it('has the folds only where merges can be folded and the graph is not on one line', () => {
    const none = words(viewMenu(state(), { folded: 'none' }));
    expect(none).not.toContain('Fold all merges');
    expect(none).not.toContain('Unfold all merges');
    const oneLane = words(viewMenu(state({ oneLane: true }), folds));
    expect(oneLane).toContain('Show all commits on one line');
    expect(oneLane).not.toContain('Fold all merges');
    expect(oneLane).not.toContain('Unfold all merges');
    expect(item(viewMenu(state(), folds), 'Unfold all merges').disabled).toBe(true);
    expect(item(viewMenu(state({ rows: [] }), folds), 'Fold all merges').disabled).toBe(true);
    expect(item(viewMenu(state(), { ...folds, folded: 'all' }), 'Fold all merges').checked).toBe(true);
  });

  it('greys the kinds out with a hint while the filter chose the references one by one', () => {
    const menu = viewMenu(state({ scope: { kind: 'refs', refs: ['refs/heads/main'] } }), folds);
    for (const label of ['Show local branches', 'Show remote branches', 'Show tags']) {
      expect(item(menu, label)).toMatchObject({ disabled: true, checked: false, hint: 'Chosen in the filter' });
    }
  });

  it('writes the columns left when Show the hash is unticked, and turns a switch the other way', () => {
    const menu = viewMenu(state(), folds);
    item(menu, 'Show the hash').run();
    item(menu, 'Show stashes').run();
    item(menu, 'Show all commits on one line').run();
    item(menu, 'Compact rows').run();
    item(menu, 'Show tags').run();
    expect(posted).toEqual([
      { type: 'setGraphColumns', columns: ['author', 'date'] },
      { type: 'setShowStashes', show: false },
      { type: 'setOneLane', one: true },
      { type: 'setCompactRows', compact: false },
      { type: 'setScope', scope: { kind: 'kinds', kinds: ['branch', 'remote'] } },
    ]);
  });
});

describe('the … menu of the toolbar', () => {
  it('has the actions alone, in their order, and no switch of the graph', () => {
    const menu = words(moreMenu(state(), false));
    expect(menu).toEqual(['Refresh', '-', ...MORE]);
    for (const label of VIEW.filter((each) => each !== '-')) expect(menu).not.toContain(label);
  });

  it('starts with Refresh but in the side bar, where the header of the view has it', () => {
    expect(words(moreMenu(state(), false)).slice(0, 2)).toEqual(['Refresh', '-']);
    expect(words(moreMenu(state(), true))).toEqual(MORE);
  });

  it('has Branch, Stash and Undo after Refresh while the toolbar is too narrow for their buttons, greyed out as the buttons are', () => {
    const narrow = moreMenu(state({ head: { commit: 'aaa' } as RepoState['head'], changes: 0 }), false, true);
    expect(words(narrow)).toEqual(['Refresh', '-', 'Branch', 'Stash', 'Undo', '-', ...MORE]);
    expect(item(narrow, 'Branch').disabled).toBe(false);
    // Nothing to stash.
    expect(item(narrow, 'Stash').disabled).toBe(true);
    item(narrow, 'Undo').run();
    expect(posted).toContainEqual(expect.objectContaining({ action: { kind: 'undo' } }));
    // Before the first commit none of the three can be chosen.
    const empty = moreMenu(state({ head: { commit: null } as RepoState['head'], changes: 0 }), false, true);
    expect(['Branch', 'Stash', 'Undo'].map((label) => item(empty, label).disabled)).toEqual([true, true, true]);
    expect(words(moreMenu(state({ head: { commit: 'aaa' } as RepoState['head'] }), false))).not.toContain('Stash');
  });

  it('leaves out what its condition does not hold for', () => {
    const menu = words(moreMenu(state({ submodules: [{}] as RepoState['submodules'], repos: [{}] as RepoState['repos'], hosting: null }), false));
    expect(menu).not.toContain('Add submodule…');
    expect(menu).not.toContain('Select repository…');
    expect(menu).not.toContain('Load pull requests from GitHub');
    expect(menu).toContain('Switch identity…');
  });

  it('loads the merge requests of a GitLab and checks one out, and has neither on Bitbucket (hosting.load-merge-requests)', () => {
    const on = (service: 'gitLab' | 'bitbucket', canCheckOut: boolean) => ({
      service,
      host: 'example.test',
      remote: 'origin',
      canCheckOut,
      comparesSinceDiverged: false,
    });
    const gitLab = words(moreMenu(state({ hosting: on('gitLab', true) }), false));
    expect(gitLab).toEqual(expect.arrayContaining(['Load merge requests from GitLab', 'Check out a merge request…']));
    expect(gitLab).not.toContain('Load pull requests from GitHub');
    const bitbucket = words(moreMenu(state({ hosting: on('bitbucket', false) }), false));
    expect(bitbucket.filter((label) => /request/.test(label))).toEqual([]);
  });

  it('posts the command of the entry chosen', () => {
    const menu = moreMenu(state(), false);
    item(menu, 'Refresh').run();
    item(menu, 'Compare branches, tags or commits…').run();
    item(menu, 'Go to commit…').run();
    item(menu, 'Apply patch…').run();
    expect(posted).toEqual([
      { type: 'refresh' },
      { type: 'compare' },
      { type: 'command', command: 'goToCommit' },
      { type: 'action', action: { kind: 'applyPatch' } },
    ]);
  });
});

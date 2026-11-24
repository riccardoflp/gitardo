import { readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, it, vi } from 'vitest';
import type { Branch, FileChange, RepoState, Row, Stash, Submodule, Worktree } from '../src/shared/protocol';
import type { ShownParts } from '../src/shared/repositories';
import type { MenuEntry, MenuItem } from '../src/webview/ContextMenu';
import { MEANING_ICONS, OPERATION_ICONS, itemIcon } from '../src/webview/menuIcons';
import { branchAt, headAt, remoteAt, tagAt, upstreamOf, worktreeAt, workingTreeOf } from './states';

// The menus post what is chosen to the extension; here they are only read.
vi.mock('../src/webview/vscode', () => ({ post: () => {} }));
const menus = await import('../src/webview/menus');
const { moreMenu, viewMenu } = await import('../src/webview/toolbarMenus');
const { commitOptionsMenu } = await import('../src/webview/commitOptions');

const ROOT = join(__dirname, '..', '..');
const ICONS = join(ROOT, 'rust', 'crates', 'views', 'src', 'icons');
const read = (file: string) => readFileSync(join(ICONS, file), 'utf8');

/** The meanings of the desktop's table of icons, in its order. */
const desktopMeanings = () => [...read('mod.rs').matchAll(/^pub\(crate\) const (\w+): Icon = Icon::\w+;/gm)].map((match) => match[1]);

/** The meaning of each operation as the desktop's `operation_icon` gives it: the arms of its three matches. */
function desktopOperations(): Record<string, string> {
  const found: Record<string, string> = {};
  const code = read('menu.rs').slice(0, read('menu.rs').indexOf('pub(crate) fn action'));
  for (const arm of code.matchAll(/((?:"[^"]+"\s*\|?\s*)+)=>\s*\{?\s*super::(\w+)/g)) {
    for (const feature of arm[1].matchAll(/"([^"]+)"/g)) found[feature[1]] = arm[2];
  }
  return found;
}

/** The operations the desktop offers in a menu with no icon, for they rewrite the history or no icon says them. */
const desktopWithoutIcon = () => {
  const code = read('menu_tests.rs');
  const list = code.slice(code.indexOf('const OPERATIONS_WITHOUT_ICON'), code.indexOf('];', code.indexOf('const OPERATIONS_WITHOUT_ICON')));
  return new Set([...list.matchAll(/"([^"]+)"/g)].map((match) => match[1]));
};

/** The identifiers of the cards of the catalog. */
const cards = () =>
  new Set(
    readdirSync(join(ROOT, 'docs', 'features'))
      .filter((file) => file.endsWith('.md'))
      .flatMap((file) => [...readFileSync(join(ROOT, 'docs', 'features', file), 'utf8').matchAll(/^### `([^`]+)`/gm)].map((match) => match[1])),
  );

/**
 * The items of the extension's menus that run an operation the desktop offers in no menu, or offers with an icon it
 * draws elsewhere: none draws an icon, and each says why.
 */
const EXTENSION_WITHOUT_ICON: Record<string, string> = {
  'rebase.interactive': 'the interactive rebase page: a rewrite of the history',
  'lfs.page': 'the page of what Git LFS stores: no icon says it',
};

const file = (path: string): FileChange => ({ path, originalPath: null, kind: 'modified' });
const row = (hash: string, parents: string[], more: Partial<Row> = {}): Row => ({
  hash,
  parents,
  author: '',
  email: '',
  date: 0,
  subject: hash,
  graph: [] as never,
  ...more,
});

const main = worktreeAt('C:/prj/app', 'main', { commit: 'd', isMain: true, isCurrent: true });
const login = worktreeAt('C:/prj/app-login', 'feature/login', { commit: 'b', isMain: false, isCurrent: false, uncommitted: 2 });
const locked = { ...login, folder: 'C:/prj/app-locked', branch: null, locked: 'kept' } as Worktree;
const core: Submodule = {
  name: 'lib/core',
  folder: 'lib/core',
  parent: null,
  url: 'https://example.com/core.git',
  branch: 'main',
  recorded: 'ccc',
  staged: 'eee',
  checkout: { commit: 'ddd', branch: null, uncommitted: 1, distance: { ahead: 1, behind: 1 } },
};
const raw: Submodule = { ...core, name: 'lib/raw', folder: 'lib/raw', checkout: null };
const rows = [row('d', ['c']), row('c', ['b']), row('b', ['a', 'x']), row('a', [])];
const branches: Branch[] = [
  branchAt('main', 'd', { isCheckedOut: true, upstream: upstreamOf('origin/main', { ahead: 1, behind: 1 }) }),
  branchAt('feature/login', 'b', { isCheckedOut: false, upstream: upstreamOf('origin/feature/login', { ahead: 2, behind: 1 }) }),
  branchAt('plain', 'c', { isCheckedOut: false }),
];
const state = {
  root: 'C:/prj/app',
  home: null,
  head: headAt('d', 'main'),
  rows,
  branches,
  remoteBranches: [remoteAt('origin/main', 'c'), remoteAt('origin/feature/login', 'a'), remoteAt('origin/other', 'b')],
  tags: [tagAt('v1.0', 'c'), tagAt('v0.9', 'a')],
  tagMarks: { 'v1.0': 'notOnRemote' },
  worktrees: [main, login, locked],
  submodules: [core, raw],
  remotes: ['origin', 'upstream'],
  favorites: ['feature/login'],
  pulls: { 'feature/login': { number: 12, title: 'Login', url: 'https://example.com/12', draft: false, checks: null } },
  hosting: { service: 'gitHub', host: 'github.com', remote: 'origin', canCheckOut: true, comparesSinceDiverged: false },
  scope: null,
  operation: null,
  agentsWindow: true,
  openBranches: 'checkout',
  workingTree: workingTreeOf({ staged: [file('a.txt')], unstaged: [file('b.txt')], untracked: ['c.txt'] }),
  bisect: null,
  showStashes: true,
  firstParent: false,
  oneLane: false,
  compactRows: true,
  traceSelected: true,
  dateSeparators: true,
  combineUpstream: true,
  graphColumns: ['author', 'date', 'hash'],
  repos: [{}, {}],
  notes: { commits: ['c'], remotes: ['origin'] },
  stashes: [],
} as unknown as RepoState;
const stash: Stash = { commit: 's', index: 0, message: 'WIP on main', date: 0 };
const shown: ShownParts = { root: 'C:/prj/app', home: null, agentsWindow: true, worktrees: [main, login, locked], submodules: [core, raw] };
const tools = { selected: 'b', onInteractiveRebase: () => {}, onCompare: () => {}, onEditMessage: () => {}, onFold: () => {} };

/** Every menu the webviews draw, each built for the states that show the most of its items. */
function everyMenu(): Record<string, MenuEntry[]> {
  const drop = (source: Parameters<typeof menus.dropMenu>[0], target: Row) => menus.dropMenu(source, target, state);
  return {
    changes: menus.changesMenu(state),
    'changed file': [
      ...menus.changedFileMenu({ file: file('a.txt'), staged: 'some', untracked: false, unsaved: false }, false),
      ...menus.changedFileMenu({ file: file('b.txt'), staged: null, untracked: false, unsaved: true }, false),
      ...menus.changedFileMenu({ file: file('c.txt'), staged: 'none', untracked: true, unsaved: false }, false),
    ],
    'resolved as before': [...menus.resolvedAsBeforeMenu('a.txt', true, false), ...menus.resolvedAsBeforeMenu('a.txt', false, false)],
    'hidden file': menus.hiddenFileMenu('a.txt', false),
    'hidden group': menus.hiddenGroupMenu(false),
    commit: rows.flatMap((each) => [
      ...menus.commitMenu(each, state, tools),
      ...menus.commitMenu(each, { ...state, bisect: { good: ['a'], bad: null, skipped: [], remaining: null } } as unknown as RepoState, tools),
    ]),
    log: menus.logMenu({ ...rows[1], body: '' } as never, state),
    selection: menus.selectionMenu([rows[0], rows[1]], state, () => {}),
    branch: state.branches.flatMap((branch) =>
      menus.branchMenu(
        branch,
        state,
        () => {},
        () => {},
      ),
    ),
    'remote branch': state.remoteBranches.flatMap((ref) =>
      menus.remoteBranchMenu(
        ref,
        state,
        () => {},
        () => {},
      ),
    ),
    tag: state.tags.flatMap((tag) =>
      menus.tagMenu(
        tag,
        state,
        () => {},
        () => {},
      ),
    ),
    stash: menus.stashMenu(stash),
    file: [...menus.fileMenu(file('a.txt'), { kind: 'commit', hash: 'c', parent: 'b' }, { hash: 'c', parent: 'b' })],
    worktree: state.worktrees.flatMap((worktree) => menus.worktreeMenu(worktree, state, () => {})),
    worktrees: menus.worktreesMenu(state),
    submodule: [core, raw].flatMap((submodule) => menus.submoduleMenu(submodule, state)),
    'repository row': [
      ...menus.repositoryRowMenu({ folder: 'C:/prj/web', isShown: false, kind: 'repository' }, shown),
      ...menus.repositoryRowMenu({ folder: 'C:/prj/app-login', isShown: false, kind: 'worktree' }, shown),
      ...menus.repositoryRowMenu({ folder: 'C:/prj/app/lib/core', isShown: false, kind: 'submodule' }, shown),
    ],
    drop: [
      ...drop({ kind: 'branch', name: 'plain', hash: 'c', local: true, current: false }, rows[0]),
      ...drop({ kind: 'branch', name: 'feature/login', hash: 'b', local: true, current: false }, rows[3]),
      ...drop({ kind: 'commit', hash: 'b', isMerge: false }, rows[0]),
      ...drop({ kind: 'branch', name: 'main', hash: 'd', local: true, current: true }, rows[3]),
    ],
    view: viewMenu(state, { onFoldAll: () => {}, folded: 'some' }),
    more: [...moreMenu(state, false), ...moreMenu(state, true)],
    'commit options': [false, true].flatMap((signs) =>
      commitOptionsMenu({
        options: { sign: signs, signOff: false, skipHooks: false, coAuthors: ['Ada <ada@example.com>'] },
        signing: { signsEveryCommit: false, kind: 'openpgp', key: signs ? 'KEY' : null },
        identity: { name: 'Ada', email: 'ada@example.com' },
        change: () => {},
        pickCoAuthors: () => {},
        switchIdentity: () => {},
      }),
    ),
  };
}

const items = (entries: MenuEntry[]) => entries.filter((entry): entry is MenuItem => !!entry && entry !== 'separator');

describe('the icons of the items of the menus (ui.menu-icons)', () => {
  it('gives each meaning of the icons of Gitardo Desktop a codicon, in the order of its table', () => {
    expect(Object.keys(MEANING_ICONS)).toEqual(desktopMeanings());
  });

  it('gives each operation the meaning operation_icon of Gitardo Desktop gives it', () => {
    expect(OPERATION_ICONS).toEqual(desktopOperations());
  });

  it('draws in every menu the icon of the desktop for each item that has one there, and says why each other item has none', () => {
    const known = cards();
    const withoutIcon = desktopWithoutIcon();
    const wrong: string[] = [];
    let counted = 0;
    for (const [menu, entries] of Object.entries(everyMenu())) {
      for (const item of items(entries)) {
        counted++;
        const where = `${menu}: ${item.label}`;
        if (item.feature && !known.has(item.feature)) wrong.push(`${where} names ${item.feature}, which is no card`);
        if (item.checked !== undefined) {
          expect(itemIcon(item), where).toBe(item.checked ? MEANING_ICONS.CHOSEN : undefined);
          continue;
        }
        if (item.icon) continue;
        if (!item.feature) {
          wrong.push(`${where} names neither its operation nor its meaning`);
          continue;
        }
        const meaning = OPERATION_ICONS[item.feature];
        if (meaning) expect(itemIcon(item), where).toBe(MEANING_ICONS[meaning]);
        else if (!withoutIcon.has(item.feature) && !EXTENSION_WITHOUT_ICON[item.feature])
          wrong.push(`${where} runs ${item.feature}, which has an icon nowhere`);
      }
    }
    expect(wrong).toEqual([]);
    // The menus were built with most of their items.
    expect(counted).toBeGreaterThan(150);
  });
});

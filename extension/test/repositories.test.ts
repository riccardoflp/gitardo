/**
 * The rows of the Repositories view (`repo.tabs`, `repo.group-tabs`): which rows a workspace gives, what each says it
 * is on, which are drawn while a repository is folded, and which folders a change the watchers saw tells of.
 */
import { describe, expect, it } from 'vitest';
import { type FollowedFolders, type ListedRepository, type RepositoryRow, drawnRows, repositoryRows, routeChange } from '../src/shared/repositories';
import { type FoundRepository, groupRepositories } from '../src/shared/repositoryGroups';
import type { RepositoryState } from '../src/shared/state';
import type { FolderSummary, WorktreeSummary } from '../src/shared/summaries';
import { branchAt, headAt, upstreamOf, workingTreeOf, worktreeAt } from './states';

const commit = '3f2a1c9'.padEnd(40, '0');

/** The repositories of a workspace as the rows take them, named after their folders. */
const listed = (found: FoundRepository[]) =>
  groupRepositories(found.map((each): ListedRepository & FoundRepository => ({ ...each, name: each.folder.split('/').pop()! })));

const repository = (folder: string, extra: Partial<FoundRepository> = {}): FoundRepository => ({ folder, superproject: null, commonFolder: `${folder}/.git`, ...extra });

const on = (branch: string, extra: Partial<WorktreeSummary> = {}): WorktreeSummary => ({ head: { kind: 'branch', name: branch }, changed: 0, ahead: null, behind: null, ...extra });

const said = (folder: string, summary: WorktreeSummary | null, extra: Partial<FolderSummary> = {}): FolderSummary => ({ folder, summary, isStale: false, worktrees: null, ...extra });

/** The state of a repository shown, on a branch, with these worktrees. */
const stateOf = (folder: string, branch: string, extra: Partial<RepositoryState> = {}) =>
  ({
    folder,
    head: headAt(commit, branch),
    branches: [branchAt(branch, commit, { upstream: upstreamOf(`origin/${branch}`, { ahead: 1, behind: 2 }) })],
    workingTree: workingTreeOf({ untracked: ['a.txt', 'b.txt', 'c.txt'] }),
    worktrees: [],
    ...extra,
  }) as unknown as RepositoryState;

/** What the rows say, one line each: the indent, the kind, the name and the marks. */
const lines = (rows: RepositoryRow[]) =>
  rows.map((row) => `${'  '.repeat(row.depth)}${row.kind} ${row.name}${row.isShown ? ' *' : ''}${row.isRunning ? ' …' : ''}${row.isStale ? ' ?' : ''}`);

describe('the rows of the Repositories view', () => {
  const api = repository('/work/api');
  const login = repository('/work/api-login', { commonFolder: '/work/api/.git' });
  const core = repository('/work/api/lib/core', { superproject: '/work/api', commonFolder: '/work/api/.git/modules/lib/core' });
  const vendor = repository('/work/api/lib/core/vendor', { superproject: '/work/api/lib/core', commonFolder: null });
  const web = repository('/work/web');

  it('lists each repository, then its worktrees, then its submodules, a submodule of a submodule under it', () => {
    const rows = repositoryRows({ repositories: listed([web, core, login, vendor, api]), summaries: [], state: null, running: null });
    expect(lines(rows)).toEqual(['repository web', 'repository api', '  worktree api-login', '  submodule lib/core', '    submodule vendor']);
    expect(rows.map((row) => row.group)).toEqual(['/work/web', '/work/api', '/work/api', '/work/api', '/work/api']);
  });

  it('lists the worktrees git lists for a repository that are not folders of the workspace, but those whose folder is gone', () => {
    const worktrees = [
      worktreeAt('/work/api', 'main', { isMain: true }),
      worktreeAt('/elsewhere/api-fix', 'fix'),
      worktreeAt('/work/api-login', 'feature/login'),
      worktreeAt('/elsewhere/gone', 'old', { isMissing: true }),
    ];
    const summaries = [said('/work/api', on('main'), { worktrees }), said('/elsewhere/api-fix', on('fix', { changed: 2 })), said('/work/api-login', null)];
    const rows = repositoryRows({ repositories: listed([api, login, core]), summaries, state: null, running: null });
    expect(lines(rows)).toEqual(['repository api', '  worktree api-login', '  worktree api-fix', '  submodule lib/core']);
    expect(rows[2].summary).toEqual(on('fix', { changed: 2 }));
    // Until a worktree is read, its row says the branch git listed for it.
    expect(rows[1].summary).toBeNull();
    expect(rows[1].listed).toBe('feature/login');
    expect(rows[1].member).toBe('feature/login');
  });

  it('takes what the one shown is on from its own read, and its worktrees from it', () => {
    const state = stateOf('/elsewhere/api-fix', 'fix', {
      worktrees: [worktreeAt('/work/api', 'main', { isMain: true }), worktreeAt('/elsewhere/api-fix', 'fix', { isCurrent: true })],
    });
    const summaries = [said('/work/api', on('main'), { worktrees: [worktreeAt('/work/api', 'main')] }), said('/elsewhere/api-fix', on('before'), { isStale: true })];
    const rows = repositoryRows({ repositories: listed([api, web]), summaries, state, running: '/work/api' });
    expect(lines(rows)).toEqual(['repository api …', '  worktree api-fix *', 'repository web']);
    expect(rows[1].summary).toEqual({ head: { kind: 'branch', name: 'fix' }, changed: 3, ahead: 1, behind: 2 });
    expect(rows[1].isStale).toBe(false);
  });

  it('marks a folder whose last read failed, and says the commit a worktree listed is detached at', () => {
    const worktrees = [worktreeAt('/work/api', 'main'), worktreeAt('/elsewhere/api-fix', null)];
    const rows = repositoryRows({ repositories: listed([api]), summaries: [said('/work/api', on('main'), { isStale: true, worktrees })], state: null, running: null });
    expect(lines(rows)).toEqual(['repository api ?', '  worktree api-fix']);
    expect(rows[1].listed).toBe('detached at aaaaaaa');
    expect(rows[1].member).toBe('api-fix');
  });
});

describe('the rows drawn while repositories are folded', () => {
  const row = (folder: string, kind: RepositoryRow['kind'], group: string, extra: Partial<RepositoryRow> = {}): RepositoryRow => ({
    folder,
    kind,
    name: folder.split('/').pop()!,
    member: folder.split('/').pop()!,
    depth: kind === 'repository' ? 0 : 1,
    group,
    summary: on(kind === 'repository' ? 'main' : 'feature/login'),
    listed: null,
    isStale: false,
    isShown: false,
    isRunning: false,
    ...extra,
  });
  const rows = [
    row('/work/api', 'repository', '/work/api'),
    row('/work/api-login', 'worktree', '/work/api', { member: 'feature/login' }),
    row('/work/web', 'repository', '/work/web', { isShown: true }),
  ];

  it('draws every row unfolded, a chevron only on a repository with rows under it', () => {
    const drawn = drawnRows(rows, []);
    expect(drawn.map((each) => [each.name, each.hasRows, each.isFolded])).toEqual([
      ['api', true, false],
      ['api-login', false, false],
      ['web', false, false],
    ]);
  });

  it('leaves out the rows under a folded repository, which stands for its own folder while none of them is shown', () => {
    const drawn = drawnRows(rows, ['/WORK/api', '/work/web']);
    expect(drawn.map((each) => [each.name, each.folder, each.isFolded])).toEqual([
      ['api', '/work/api', true],
      ['web', '/work/web', false],
    ]);
  });

  it('says the worktree shown after the name of its folded repository, with what it is on, and marks it', () => {
    const shown = rows.map((each) => ({ ...each, isShown: each.folder === '/work/api-login', isRunning: each.folder === '/work/api-login' }));
    const [api] = drawnRows(shown, ['/work/api']);
    expect(api).toMatchObject({ name: 'api › feature/login', repository: 'api', folder: '/work/api-login', kind: 'repository', depth: 0, isShown: true, isRunning: true });
    expect(api.isFolded).toBe(true);
    expect(api.summary).toEqual(on('feature/login'));
    // Unfolded, the row of the repository says its own folder, and the worktree is the one marked.
    expect(drawnRows(shown, []).map((each) => each.isShown)).toEqual([false, true, false]);
  });
});

describe('the folders a change the watchers saw tells of', () => {
  const followed: FollowedFolders = {
    folders: ['/work/api', '/work/api-login', '/work/api/.claude/worktrees/fix', '/work/api/lib/core', '/work/api/lib/core/vendor', '/work/web'],
    families: [['/work/api', '/work/api-login', '/work/api/.claude/worktrees/fix']],
    submodules: [
      { folder: '/work/api/lib/core', superproject: '/work/api' },
      { folder: '/work/api/lib/core/vendor', superproject: '/work/api/lib/core' },
    ],
  };
  const told = (folder: string, file: string) => routeChange(folder, file, followed).map((change) => `${change.folder} ${change.path}`);

  it('tells of its folder for a file of the working tree, and of the repository around a submodule too', () => {
    expect(told('/work/api', 'src/a.rs')).toEqual(['/work/api src/a.rs']);
    expect(told('/work/api', 'lib/core/a.rs')).toEqual(['/work/api lib/core/a.rs']);
    expect(told('/work/api/lib/core', 'a.rs')).toEqual(['/work/api/lib/core a.rs']);
  });

  it('tells of nothing for a file in a worktree nested in the folder, nor for the file that names a git folder', () => {
    expect(told('/work/api', '.claude/worktrees/fix/src/a.rs')).toEqual([]);
    expect(told('/work/api/.claude/worktrees/fix', 'src/a.rs')).toEqual(['/work/api/.claude/worktrees/fix src/a.rs']);
    expect(told('/work/api-login', '.git')).toEqual([]);
  });

  it('tells every worktree of a repository of what they share, and only the repository of its own index', () => {
    expect(told('/work/api', '.git/refs/heads/main')).toEqual([
      '/work/api .git/refs/heads/main',
      '/work/api-login .git/refs/heads/main',
      '/work/api/.claude/worktrees/fix .git/refs/heads/main',
    ]);
    expect(told('/work/web', '.git/index')).toEqual(['/work/web .git/index']);
  });

  it('tells a worktree of what changed in the folder git keeps for it, as a path of its own git folder', () => {
    expect(told('/work/api', '.git/worktrees/api-login/index')).toEqual([
      '/work/api .git/worktrees/api-login/index',
      '/work/api-login .git/index',
      '/work/api/.claude/worktrees/fix .git/index',
    ]);
    // The folder itself, added or removed, lists the worktrees again and is nobody's own.
    expect(told('/work/api', '.git/worktrees/api-login')).toEqual(['/work/api .git/worktrees/api-login']);
  });

  it('tells a submodule, and a submodule of it, of what changed in the git folder kept for it', () => {
    expect(told('/work/api', '.git/modules/lib/core/HEAD')).toEqual(['/work/api .git/modules/lib/core/HEAD', '/work/api/lib/core .git/HEAD']);
    expect(told('/work/api', '.git/modules/lib/core/modules/vendor/index')).toEqual([
      '/work/api .git/modules/lib/core/modules/vendor/index',
      '/work/api/lib/core .git/modules/vendor/index',
      '/work/api/lib/core/vendor .git/index',
    ]);
  });
});

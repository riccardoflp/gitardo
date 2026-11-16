/**
 * The list of the repositories of a workspace, which puts each worktree and each submodule under the repository it is
 * of, as `repo.group-tabs` groups them: on the folders `repository/discover` would answer, and on real repositories
 * the engine finds.
 */
import * as fs from 'node:fs';
import * as path from 'node:path';
import { describe, expect, it } from 'vitest';
import { repositoryName } from '../src/engine/reads';
import { groupRepositories, type FoundRepository } from '../src/shared/repositoryGroups';
import { built, cleanUpAfterEach, client, folder } from './engine/harness';
import { git } from './git';

cleanUpAfterEach();

const found = (folder: string, superproject: string | null = null, commonFolder: string | null = `${folder}/.git`): FoundRepository => ({
  folder,
  superproject,
  commonFolder,
});

/** The list as it reads: each folder after as many dots as it is indented. */
const listed = (repositories: FoundRepository[]) => groupRepositories(repositories).map(({ repository, depth }) => `${'.'.repeat(depth)}${repository.folder}`);

describe('the list of the repositories', () => {
  it('puts a worktree and a submodule of a repository under it, the worktree first', () => {
    const repositories = [found('/work/api'), found('/work/api/lib/core', '/work/api', '/work/api/.git/modules/lib/core'), found('/work/web'), found('/work/api-login', null, '/work/api/.git')];
    expect(listed(repositories)).toEqual(['/work/api', './work/api-login', './work/api/lib/core', '/work/web']);
  });

  it('puts the submodules of a submodule under it, one step further', () => {
    const repositories = [found('/work/api'), found('/work/api/lib', '/work/api'), found('/work/api/lib/deep', '/work/api/lib')];
    expect(listed(repositories)).toEqual(['/work/api', './work/api/lib', '../work/api/lib/deep']);
  });

  it('puts the worktrees under the repository whose git folder they share, even when it is found after them', () => {
    const repositories = [found('/work/api-login', null, '/work/api/.git'), found('/work/web'), found('/work/api')];
    expect(listed(repositories)).toEqual(['/work/web', '/work/api', './work/api-login']);
  });

  it('keeps worktrees whose repository is not listed together, under the first of them', () => {
    const repositories = [found('/work/one', null, '/elsewhere/api/.git'), found('/work/two', null, '/elsewhere/api/.git')];
    expect(listed(repositories)).toEqual(['/work/one', './work/two']);
  });

  it('keeps apart the repositories that share no git folder and are not inside one another, in the order they were found', () => {
    const repositories = [found('/work/web'), found('/work/api'), found('/work/docs', null, null)];
    expect(listed(repositories)).toEqual(['/work/web', '/work/api', '/work/docs']);
  });

  it('tells the folders apart whatever their slashes and their case', () => {
    const repositories = [found('C:\\Work\\Api'), found('c:/work/api/lib', 'C:\\Work\\Api', 'C:/Work/Api/.git/modules/lib'), found('C:\\work\\api-login', null, 'c:\\work\\api\\.git\\')];
    expect(listed(repositories)).toEqual(['C:\\Work\\Api', '.C:\\work\\api-login', '.c:/work/api/lib']);
  });

  it('lists every repository once, even when what they say of one another goes round', () => {
    const repositories = [found('/work/one', '/work/two'), found('/work/two', '/work/one')];
    expect(listed(repositories).map((line) => line.replace(/^\.+/, '')).sort()).toEqual(['/work/one', '/work/two']);
  });
});

describe.skipIf(!built)('the list of the repositories the engine finds', { timeout: 60_000 }, () => {
  /** A repository with a commit, made as the test author. */
  async function repository(root: string): Promise<void> {
    fs.mkdirSync(root, { recursive: true });
    await git(root, ['init', '--quiet', '--initial-branch=main']);
    await git(root, ['config', 'user.name', 'Test Author']);
    await git(root, ['config', 'user.email', 'author@example.com']);
    await git(root, ['config', 'commit.gpgsign', 'false']);
    fs.writeFileSync(path.join(root, 'a.txt'), 'one\n');
    await git(root, ['add', 'a.txt']);
    await git(root, ['commit', '--quiet', '-m', 'Add a']);
  }

  it('puts a worktree of api and a submodule of api under api, indented, and web apart', async () => {
    const workspace = folder();
    const api = path.join(workspace, 'api');
    const web = path.join(workspace, 'web');
    const core = path.join(folder(), 'core');
    const login = path.join(workspace, 'api-login');
    for (const root of [api, web, core]) await repository(root);
    await git(api, ['-c', 'protocol.file.allow=always', 'submodule', 'add', '--quiet', core, 'lib/core']);
    await git(api, ['commit', '--quiet', '-m', 'Add the submodule']);
    await git(api, ['worktree', 'add', '--quiet', '-b', 'feature/login', login]);

    const discovered = await client().discover([workspace]);
    const names = groupRepositories(discovered).map(({ repository, depth }) => `${'  '.repeat(depth)}${repositoryName(repository.folder, [workspace])}`);

    const own = path.basename(workspace);
    expect(names).toEqual([`${own}/api`, `  ${own}/api-login`, `  ${own}/api/lib/core`, `${own}/web`]);
  });
});

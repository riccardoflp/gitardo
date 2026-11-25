// Makes what choosing a repository is timed on: node test/perf/switch-fixture.mjs <folder> [commits on main] [changed files]
// In <folder>/workspace, the folder VS Code opens, the large repository of big-repo.mjs as `large` and a small one as
// `beside`, which comes first in the list and so is the one shown when the window opens; in <folder>/worktrees, three
// worktrees of the large one, each on one of its branches. The large one is looked after as `git gc` leaves it, as the
// pipeline does for the times of docs/extension/12-quality.md. Keep the folder away from any repository, such as under
// ~/.cache/gitardo-perf: test/perf/vscode.mjs <folder> --switch measures on it.

import { execFileSync } from 'node:child_process';
import * as fs from 'node:fs';
import * as path from 'node:path';
import { fileURLToPath } from 'node:url';

const [folder, commitsGiven, changedGiven] = process.argv.slice(2);
if (!folder) {
  console.error('Usage: node test/perf/switch-fixture.mjs <folder> [commits on main] [changed files]');
  process.exit(1);
}
/** How many worktrees the large repository gets: one for each window the measure opens. */
const WORKTREES = 3;
/** The commits of the small repository. */
const SMALL_COMMITS = 40;

const root = path.resolve(folder);
const { large, beside, worktrees } = places(root);
if (fs.existsSync(large) || fs.existsSync(beside) || fs.existsSync(path.join(root, 'worktrees'))) {
  console.error(`${root} already has a workspace: remove it to make it again.`);
  process.exit(1);
}
const git = (cwd, ...args) => execFileSync('git', args, { cwd, stdio: ['ignore', 'pipe', 'inherit'] }).toString();

/** Where each part of what is made is, under the folder given. */
function places(under) {
  const workspace = path.join(under, 'workspace');
  return {
    workspace,
    large: path.join(workspace, 'large'),
    beside: path.join(workspace, 'beside'),
    worktrees: Array.from({ length: WORKTREES }, (_, index) => path.join(under, 'worktrees', `worktree-${index + 1}`)),
  };
}

fs.mkdirSync(path.dirname(large), { recursive: true });
const maker = path.join(path.dirname(fileURLToPath(import.meta.url)), 'big-repo.mjs');
execFileSync(process.execPath, [maker, large, commitsGiven ?? '50000', changedGiven ?? '1779'], { stdio: 'inherit' });
git(large, 'pack-refs', '--all');
git(large, 'commit-graph', 'write', '--reachable');

// The newest branches never merged, one for each worktree.
const branches = git(large, 'for-each-ref', '--sort=-committerdate', `--count=${WORKTREES}`, '--format=%(refname:short)', 'refs/heads/feature').trim().split('\n');
if (branches.length < WORKTREES) throw new Error(`The large repository has ${branches.length} branches to check out, fewer than ${WORKTREES}: give it more commits.`);
branches.forEach((branch, index) => git(large, 'worktree', 'add', '--quiet', worktrees[index], branch));

fs.mkdirSync(beside, { recursive: true });
git(beside, 'init', '--quiet', '--initial-branch=main');
git(beside, 'config', 'user.name', 'Gitardo Test');
git(beside, 'config', 'user.email', 'test@gitardo.invalid');
git(beside, 'config', 'commit.gpgsign', 'false');
git(beside, 'config', 'core.autocrlf', 'false');
for (let index = 1; index <= SMALL_COMMITS; index++) {
  fs.writeFileSync(path.join(beside, 'notes.txt'), `line ${index}\n`, { flag: 'a' });
  git(beside, 'add', '--all');
  git(beside, 'commit', '--quiet', '-m', `Add note ${index}`);
  if (index === SMALL_COMMITS / 2) git(beside, 'branch', 'earlier');
}

console.log(`${root}: workspace/large with ${WORKTREES} worktrees on ${branches.join(', ')}, and workspace/beside with ${SMALL_COMMITS} commits`);

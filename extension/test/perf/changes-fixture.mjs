// Makes what changes made by git outside are timed on: node test/perf/changes-fixture.mjs <folder> [repositories] [folders of a build in each] [files in each folder]
// In <folder>/workspace, the folder VS Code opens, thirty small repositories, `repository-01` to `repository-30`, each
// with a few commits and, ignored by it, a folder `build` of many folders of small files, as a build leaves them:
// what the watcher of the workspace has to go through before it says anything. It is the folders that count: 7,000 in
// each repository by default, 210,000 in all with a file each, about 800 MB made in some seconds, with which a window
// on a fast disk hears nothing for its first fourteen to eighteen seconds; with 2,000 it hears of everything from its
// first rows. On Linux each folder takes one of the watches of `fs.inotify.max_user_watches`. Beside the repositories
// `written`, a folder that is no repository, where the measure writes files without end. The first repository is the
// one shown when the window opens and the one the measure changes; its tag `fixture` is where each measure starts
// from. The worktrees the measure adds go in <folder>/worktrees. Keep the folder away from any repository, such as
// under ~/.cache/gitardo-perf: test/perf/vscode.mjs <folder> --changes measures on it.

import { execFileSync } from 'node:child_process';
import * as fs from 'node:fs';
import * as path from 'node:path';

const [folder, repositoriesGiven, foldersGiven, filesGiven] = process.argv.slice(2);
if (!folder) {
  console.error('Usage: node test/perf/changes-fixture.mjs <folder> [repositories] [folders of a build in each] [files in each folder]');
  process.exit(1);
}
/** How many repositories the workspace has. */
const REPOSITORIES = Number(repositoriesGiven ?? 30);
/** How many folders the build of each repository has, and how many files each of them. */
const BUILD_FOLDERS = Number(foldersGiven ?? 7000);
const FILES_IN_EACH = Number(filesGiven ?? 1);
/** The folders of a build are in groups of this many, so that no folder lists thousands. */
const GROUP = 50;
/** The commits of each repository. */
const COMMITS = 5;

const root = path.resolve(folder);
const workspace = path.join(root, 'workspace');
if (fs.existsSync(workspace) || fs.existsSync(path.join(root, 'worktrees'))) {
  console.error(`${root} already has a workspace: remove it to make it again.`);
  process.exit(1);
}
const git = (cwd, ...args) => execFileSync('git', args, { cwd, stdio: ['ignore', 'pipe', 'inherit'] }).toString();
const numbered = (index, width) => String(index).padStart(width, '0');

const started = Date.now();
for (let index = 1; index <= REPOSITORIES; index++) {
  const repository = path.join(workspace, `repository-${numbered(index, 2)}`);
  fs.mkdirSync(repository, { recursive: true });
  // Its own identity, so that nothing depends on the machine's git configuration.
  git(repository, 'init', '--quiet', '--initial-branch=main');
  git(repository, 'config', 'user.name', 'Gitardo Test');
  git(repository, 'config', 'user.email', 'test@gitardo.invalid');
  git(repository, 'config', 'commit.gpgsign', 'false');
  git(repository, 'config', 'core.autocrlf', 'false');
  fs.writeFileSync(path.join(repository, '.gitignore'), 'build/\n');
  for (let commit = 1; commit <= COMMITS; commit++) {
    fs.writeFileSync(path.join(repository, 'notes.txt'), `line ${commit}\n`, { flag: 'a' });
    git(repository, 'add', '--all');
    git(repository, 'commit', '--quiet', '-m', `Add note ${commit}`);
  }
  git(repository, 'tag', 'fixture');
  for (let made = 0; made < BUILD_FOLDERS; made++) {
    const unit = path.join(repository, 'build', `group-${numbered(Math.floor(made / GROUP), 4)}`, `unit-${numbered(made % GROUP, 2)}`);
    fs.mkdirSync(unit, { recursive: true });
    for (let file = 1; file <= FILES_IN_EACH; file++) fs.writeFileSync(path.join(unit, `object-${file}.o`), `${index} ${made} ${file}\n`);
  }
}
fs.mkdirSync(path.join(workspace, 'written'));
fs.mkdirSync(path.join(root, 'worktrees'));

const folders = REPOSITORIES * BUILD_FOLDERS;
console.log(
  `${root}: workspace with ${REPOSITORIES} repositories of ${COMMITS} commits, ${folders} folders of builds with ${folders * FILES_IN_EACH} files, and the folder written, in ${Math.round((Date.now() - started) / 1000)} s`,
);

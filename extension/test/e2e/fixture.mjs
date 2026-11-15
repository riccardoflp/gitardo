// What the scripts that start a real VS Code share: finding it, and the throwaway workspace they open in it.

import { execFileSync } from 'node:child_process';
import * as fs from 'node:fs';
import * as path from 'node:path';
import { fileURLToPath } from 'node:url';

/** The folder of the extension. */
export const source = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..');

export function findCode(given) {
  const candidates = [
    given,
    process.env.VSCODE_PATH,
    process.env.LOCALAPPDATA && path.join(process.env.LOCALAPPDATA, 'Programs', 'Microsoft VS Code', 'Code.exe'),
    'C:\\Program Files\\Microsoft VS Code\\Code.exe',
    '/Applications/Visual Studio Code.app/Contents/MacOS/Electron',
    '/usr/share/code/code',
    '/usr/bin/code',
  ];
  const found = candidates.find((candidate) => candidate && fs.existsSync(candidate));
  if (!found) throw new Error('VS Code not found. Pass the path to its executable, or set VSCODE_PATH.');
  return found;
}

export const git = (cwd, ...args) => execFileSync('git', args, { cwd, stdio: 'pipe' });

/** A repository with its own identity, so that nothing depends on the machine's git configuration. */
function init(dir) {
  fs.mkdirSync(dir, { recursive: true });
  git(dir, 'init', '--initial-branch=main');
  git(dir, 'config', 'user.name', 'Gitardo Test');
  git(dir, 'config', 'user.email', 'test@gitardo.invalid');
  git(dir, 'config', 'commit.gpgsign', 'false');
  git(dir, 'config', 'core.autocrlf', 'false');
}

function commit(dir, file, content, message) {
  fs.writeFileSync(path.join(dir, file), content);
  git(dir, 'add', '--all');
  git(dir, 'commit', '-m', message);
}

/**
 * Makes the workspace inside `temp`: a plain folder with two repositories, `app` and `lib`.
 * `app` has a bare remote next to the workspace, a merged branch and a `feature` branch one commit ahead.
 */
export function createWorkspace(temp) {
  const workspace = path.join(temp, 'workspace');
  const app = path.join(workspace, 'app');
  const lib = path.join(workspace, 'lib');
  const remote = path.join(temp, 'remote.git');
  init(app);
  commit(app, 'a.txt', 'one\n', 'Add a');
  commit(app, 'b.txt', 'bee\n', 'Add b');
  commit(app, 'a.txt', 'one\ntwo\n', 'Change a');
  fs.mkdirSync(remote);
  git(remote, 'init', '--bare', '--initial-branch=main');
  git(app, 'remote', 'add', 'origin', remote);
  git(app, 'push', '--set-upstream', 'origin', 'main');
  git(app, 'branch', 'merged');
  git(app, 'checkout', '-b', 'feature');
  // It also changes a file of `main`, so that a later change to that file there conflicts with it.
  fs.writeFileSync(path.join(app, 'b.txt'), 'bee\nfeature\n');
  commit(app, 'feature.txt', 'feature\n', 'Add feature');
  git(app, 'checkout', 'main');
  init(lib);
  commit(lib, 'lib.txt', 'lib\n', 'Add lib');
  return { workspace, app, lib, remote };
}

/** The arguments that start VS Code on its own, away from the user's settings, extensions and windows. */
export function isolated(temp) {
  return [
    `--user-data-dir=${path.join(temp, 'user-data')}`,
    `--extensions-dir=${path.join(temp, 'extensions')}`,
    '--disable-extensions',
    '--disable-workspace-trust',
    '--disable-updates',
    '--skip-welcome',
    '--skip-release-notes',
    '--new-window',
  ];
}

/**
 * The setting that points the throwaway profile at Gitardo's engine, which is read from the user settings only: the
 * debug binary of `../rust/target/debug` (or `GITARDO_ENGINE`), unless `--no-engine` asks for a file that is not there,
 * which checks the fallback to the TypeScript operations. `said` is what was chosen, to be printed.
 */
export function engineSetting(temp, options) {
  const name = process.platform === 'win32' ? 'gitardo-engine.exe' : 'gitardo-engine';
  const built = process.env.GITARDO_ENGINE ?? path.resolve(source, '..', 'rust', 'target', 'debug', name);
  const engine = options.includes('--no-engine') || !fs.existsSync(built) ? path.join(temp, 'no-engine', name) : built;
  return { settings: { 'gitardo.engine.path': engine }, said: fs.existsSync(engine) ? `with the engine ${engine}` : 'without the engine' };
}

/** The environment for VS Code, without the variable that would make it run as plain Node when started from an Electron app. */
export function environment(extra = {}) {
  const env = { ...process.env, ...extra };
  delete env.ELECTRON_RUN_AS_NODE;
  return env;
}

export function remove(temp) {
  try {
    fs.rmSync(temp, { recursive: true, force: true, maxRetries: 5, retryDelay: 500 });
  } catch {
    console.log(`Could not remove ${temp}`);
  }
}

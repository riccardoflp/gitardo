/**
 * The extension's client against `gitardo-engine`, built by `cargo build -p server` in `../rust`: the acceptance lines of
 * stage 1 of `docs/transition/06-roadmap.md` that need the extension. The state the engine reads is compared with the
 * one `Repo.state` reads in the same throwaway repositories. Skipped, with the reason, when the binary is not built;
 * `GITARDO_ENGINE` names another one.
 */
import { spawn } from 'node:child_process';
import * as fs from 'node:fs';
import * as os from 'node:os';
import * as path from 'node:path';
import { afterEach, describe, expect, it } from 'vitest';
import { binaryName } from '../src/engine/binary';
import { EngineClient, type EngineLog } from '../src/engine/client';
import { Connection, EngineError, ErrorCode } from '../src/engine/connection';
import type { EngineState } from '../src/engine/messages';
import { engineSettings } from '../src/engine/settings';
import { git } from '../src/git/exec';
import { Repo } from '../src/git/repo';
import type { RepoState } from '../src/shared/protocol';

const binary = process.env.GITARDO_ENGINE ?? path.resolve(__dirname, '../../rust/target/debug', binaryName());
const built = fs.existsSync(binary);
if (!built) console.warn(`The engine's tests are skipped: ${binary} is not built. Run cargo build -p server in rust/.`);

const folders: string[] = [];
const clients: EngineClient[] = [];

afterEach(async () => {
  await Promise.all(clients.splice(0).map((client) => client.dispose()));
  for (const folder of folders.splice(0)) fs.rmSync(folder, { recursive: true, force: true });
});

/** A folder of its own for a test, by its real name, as git names it. */
function folder(): string {
  const made = fs.realpathSync.native(fs.mkdtempSync(path.join(os.tmpdir(), 'gitardo-engine-')));
  folders.push(made);
  return made;
}

/** A new repository with a commit on `main`, or none. */
async function repository(commit = true): Promise<string> {
  const root = path.join(folder(), 'repo');
  fs.mkdirSync(root);
  await git(root, ['init', '--initial-branch=main']);
  await git(root, ['config', 'user.name', 'Gitardo Test']);
  await git(root, ['config', 'user.email', 'test@gitardo.invalid']);
  await git(root, ['config', 'commit.gpgsign', 'false']);
  if (commit) await commitFile(root, 'a.txt', 'one\n', 'First commit');
  return root;
}

async function commitFile(root: string, file: string, content: string, message: string): Promise<void> {
  fs.writeFileSync(path.join(root, file), content);
  await git(root, ['add', '--', file]);
  await git(root, ['commit', '--quiet', '-m', message]);
}

/** The lines the tests read of the log. */
function memoryLog(): EngineLog & { lines: string[] } {
  const lines: string[] = [];
  const keep = (level: string) => (line: string) => lines.push(`${level} ${line}`);
  return {
    lines,
    info: keep('info'),
    warn: keep('warn'),
    error: keep('error'),
    debug: keep('debug'),
  };
}

function client(log = memoryLog()): EngineClient {
  const started = new EngineClient({
    binary,
    clientVersion: '0.0.0-test',
    settings: () => engineSettings({}, new Set(), {}, 0),
    logLevel: () => 'info',
    log,
  });
  clients.push(started);
  return started;
}

/** A path as both sides can be compared by. */
function samePath(named: string): string {
  const resolved = path.resolve(named);
  return process.platform === 'win32' ? resolved.toLowerCase() : resolved;
}

const LETTERS: Record<string, string> = {
  added: 'A',
  modified: 'M',
  deleted: 'D',
  renamed: 'R',
  copied: 'C',
  typeChanged: 'T',
};
const byPath = <T extends { path: string }>(files: T[]) => [...files].sort((one, other) => one.path.localeCompare(other.path));
const byName = <T extends { name: string }>(refs: T[]) => [...refs].sort((one, other) => one.name.localeCompare(other.name));

/** What the engine read, in the shape `Repo.state` gives it. */
function asRepoState(state: EngineState) {
  return {
    head: state.head.commit,
    branch: state.head.branch?.replace(/^refs\/heads\//, '') ?? null,
    branches: byName(
      state.branches.map((branch) => ({
        name: branch.name,
        hash: branch.commit,
        current: branch.isCheckedOut,
        ...(branch.upstream && {
          upstream: {
            name: branch.upstream.reference.replace(/^refs\/remotes\//, ''),
            remote: branch.upstream.remote,
            // git's configuration names it in full, and Repo.state by its short name.
            branch: branch.upstream.remoteBranch.replace(/^refs\/heads\//, ''),
            ahead: branch.upstream.ahead,
            behind: branch.upstream.behind,
            gone: branch.upstream.isGone,
          },
        }),
      })),
    ),
    remoteBranches: byName(
      state.remoteBranches.map((branch) => ({
        name: branch.name,
        hash: branch.commit,
      })),
    ),
    tags: byName(state.tags.map((tag) => ({ name: tag.name, hash: tag.commit }))),
    stashes: state.stashes.map((stash) => ({
      ref: `stash@{${stash.index}}`,
      hash: stash.commit,
      message: stash.message,
      date: stash.date,
    })),
    worktrees: state.worktrees.map((worktree) => ({
      path: samePath(worktree.folder),
      // `git worktree list` says forty zeros before the first commit, where the engine says none.
      hash: worktree.commit ?? '0'.repeat(40),
      branch: worktree.branch?.replace(/^refs\/heads\//, '') ?? null,
      current: worktree.isCurrent,
      main: worktree.isMain,
      prunable: worktree.isMissing,
      changes: worktree.uncommitted,
    })),
    working: {
      staged: byPath(
        state.workingTree.staged.map((file) => ({
          status: LETTERS[file.kind],
          path: file.path,
          ...(file.originalPath && { oldPath: file.originalPath }),
        })),
      ),
      unstaged: byPath([
        ...state.workingTree.unstaged.map((file) => ({
          status: LETTERS[file.kind],
          path: file.path,
        })),
        ...state.workingTree.untracked.map((file) => ({
          status: '?',
          path: file,
        })),
      ]),
      conflicts: byPath(
        state.workingTree.conflicted.map((file) => ({
          status: 'U',
          path: file.path,
        })),
      ),
    },
  };
}

/** What `Repo.state` read, cut to what the engine reads in stage 1. */
function cut(state: RepoState) {
  return {
    head: state.head,
    branch: state.branch,
    branches: byName(
      state.branches.map(({ name, hash, current, upstream }) => ({
        name,
        hash,
        current,
        ...(upstream && { upstream }),
      })),
    ),
    remoteBranches: byName(state.remoteBranches.map(({ name, hash }) => ({ name, hash }))),
    tags: byName(state.tags.map(({ name, hash }) => ({ name, hash }))),
    stashes: state.stashes,
    worktrees: state.worktrees.map((worktree) => ({
      ...worktree,
      path: samePath(worktree.path),
    })),
    working: {
      staged: byPath(state.working.staged),
      unstaged: byPath(state.working.unstaged),
      conflicts: byPath(state.working.conflicts),
    },
  };
}

/** Opens a repository in the engine and reads it with `Repo.state`, and says whether the two read the same. */
async function compare(root: string): Promise<void> {
  const engine = client();
  const read = await engine.open(root);
  const discovered = (await Repo.discover(root))!;
  const extension = await new Repo(discovered).state(10);
  expect(asRepoState(read)).toEqual(cut(extension));
}

describe.skipIf(!built)('gitardo-engine from the extension', { timeout: 60_000 }, () => {
  describe('reads the same state as Repo.state', () => {
    it('in a repository with no commits yet and a file not tracked', async () => {
      const root = await repository(false);
      fs.writeFileSync(path.join(root, 'new.txt'), 'new\n');
      await compare(root);
    });

    it('with branches, a tag of each kind and two stashes', async () => {
      const root = await repository();
      await git(root, ['branch', 'feature']);
      await commitFile(root, 'a.txt', 'two\n', 'Second commit');
      await git(root, ['tag', 'v1.0']);
      await git(root, ['tag', '-a', 'v1.1', '-m', 'Release 1.1']);
      fs.writeFileSync(path.join(root, 'a.txt'), 'three\n');
      await git(root, ['stash', 'push', '-m', 'half a fix']);
      fs.writeFileSync(path.join(root, 'a.txt'), 'four\n');
      await git(root, ['stash', 'push']);
      await compare(root);
    });

    it('with branches that track a remote, ahead, behind and gone', async () => {
      const remote = await repository();
      await git(remote, ['branch', 'old']);
      const root = path.join(path.dirname(remote), 'clone');
      await git(path.dirname(remote), ['clone', '--quiet', remote, root]);
      await git(root, ['config', 'user.name', 'Gitardo Test']);
      await git(root, ['config', 'user.email', 'test@gitardo.invalid']);
      await git(root, ['config', 'commit.gpgsign', 'false']);
      await git(root, ['branch', '--track', 'old', 'origin/old']);
      await commitFile(remote, 'b.txt', 'b\n', 'On the remote');
      await commitFile(root, 'c.txt', 'c\n', 'Here');
      await git(remote, ['branch', '-D', 'old']);
      await git(root, ['fetch', '--quiet', '--prune']);
      await compare(root);
    });

    it('with a worktree, one whose folder is gone, and a detached head', async () => {
      const root = await repository();
      const parent = path.dirname(root);
      await git(root, ['worktree', 'add', '--quiet', '-b', 'fix', path.join(parent, 'fix')]);
      fs.writeFileSync(path.join(parent, 'fix', 'x.txt'), 'x\n');
      await git(root, ['worktree', 'add', '--quiet', '-b', 'gone', path.join(parent, 'gone')]);
      fs.rmSync(path.join(parent, 'gone'), { recursive: true, force: true });
      await git(root, ['checkout', '--quiet', '--detach']);
      await compare(root);
    });

    it('with files staged, renamed, changed, deleted and not tracked', async () => {
      const root = await repository();
      await commitFile(root, 'b.txt', 'b\n', 'Add b');
      await commitFile(root, 'old.txt', 'a long enough line to be found again\n', 'Add old');
      fs.writeFileSync(path.join(root, 'added.txt'), 'added\n');
      await git(root, ['add', 'added.txt']);
      await git(root, ['mv', 'old.txt', 'new.txt']);
      fs.writeFileSync(path.join(root, 'a.txt'), 'changed\n');
      await git(root, ['add', 'a.txt']);
      fs.writeFileSync(path.join(root, 'a.txt'), 'changed again\n');
      fs.rmSync(path.join(root, 'b.txt'));
      fs.writeFileSync(path.join(root, 'untracked.txt'), 'u\n');
      await compare(root);
    });

    it('in a merge that stopped with conflicts', async () => {
      const root = await repository();
      await git(root, ['checkout', '--quiet', '-b', 'other']);
      await commitFile(root, 'a.txt', 'other\n', 'Other change');
      await git(root, ['checkout', '--quiet', 'main']);
      await commitFile(root, 'a.txt', 'main\n', 'Main change');
      await git(root, ['merge', 'other']).catch(() => {});
      await compare(root);
    });
  });

  it('answers initialize with protocol 1.0, and an initialize with 2.0 with -32602 and its own version', async () => {
    const child = spawn(binary, ['--stdio'], {
      stdio: ['pipe', 'pipe', 'inherit'],
    });
    const connection = new Connection(child.stdin, child.stdout);
    try {
      const answer = await connection.request<{ protocol: string }>('initialize', { protocol: '1.0', client: { name: 'test', version: '0' } });
      expect(answer.protocol).toBe('1.0');
      const refused = await connection
        .request('initialize', {
          protocol: '2.0',
          client: { name: 'test', version: '0' },
        })
        .catch((error: unknown) => error);
      expect(refused).toBeInstanceOf(EngineError);
      expect((refused as EngineError).code).toBe(ErrorCode.invalidParams);
      expect((refused as EngineError).data).toMatchObject({
        protocol: '1.0',
        engine: { version: expect.any(String) },
      });

      const unknown = await connection.request('graph/teleport', {}).catch((error: unknown) => error);
      expect((unknown as EngineError).code).toBe(ErrorCode.methodNotFound);
      await expect(connection.request('shutdown', null)).resolves.toBeNull();
    } finally {
      child.kill();
    }
  });

  it('sends one state for a burst of ten changes', async () => {
    const root = await repository();
    const engine = client();
    await engine.open(root);
    const states: EngineState[] = [];
    engine.onState((repository, state) => repository === root && states.push(state));
    for (let round = 0; round < 10; round++) {
      fs.writeFileSync(path.join(root, 'b.txt'), `${round}\n`);
      engine.changed(root, ['b.txt']);
      await new Promise((resolve) => setTimeout(resolve, 20));
    }
    await new Promise((resolve) => setTimeout(resolve, 4000));
    expect(states).toHaveLength(1);
    expect(states[0].workingTree.untracked).toEqual(['b.txt']);
  });

  it('cancels a read that runs when its signal is aborted', async () => {
    const root = await repository();
    // git asks this hook before it lists the changes, and the hook takes ten seconds.
    await git(root, ['config', 'core.fsmonitor', 'sleep 10; true']);
    const engine = client();
    await engine.start();
    const abort = new AbortController();
    const reading = engine.open(root, abort.signal).catch((error: unknown) => error);
    await new Promise((resolve) => setTimeout(resolve, 500));
    const aborted = Date.now();
    abort.abort();
    const error = await reading;
    expect((error as EngineError).code).toBe(ErrorCode.cancelled);
    expect(Date.now() - aborted).toBeLessThan(5000);
  });

  it('starts the engine again once it is killed and opens the repositories again, and gives up at the third exit in five minutes', async () => {
    const root = await repository();
    const log = memoryLog();
    const engine = client(log);
    await engine.open(root);
    const killAndWait = async () => {
      const before = engine.pid!;
      process.kill(before);
      for (let tries = 0; tries < 100 && (engine.pid === before || engine.status !== 'running') && engine.status !== 'given up'; tries++) {
        await new Promise((resolve) => setTimeout(resolve, 100));
      }
    };

    await killAndWait();
    expect(engine.status).toBe('running');
    // Opened again by the client: the engine just started answers for it without an open of the test.
    await new Promise((resolve) => setTimeout(resolve, 1000));
    expect((await engine.readState(root)).head.branch).toBe('refs/heads/main');
    expect(log.lines.filter((line) => line.includes('starting it again'))).toHaveLength(1);

    await killAndWait();
    expect(engine.status).toBe('running');
    await killAndWait();
    expect(engine.status).toBe('given up');
    expect(
      log.lines.filter((line) => /^error .*the third time in five minutes: it is not started again until the window is reloaded\.$/.test(line)),
    ).toHaveLength(1);
    await expect(engine.readState(root)).rejects.toBeInstanceOf(EngineError);
  });
});

describe('the settings sent to the engine', () => {
  it('drops the prefix, maps the three the desktop names otherwise, keeps what is only VS Code', () => {
    const values = {
      'graph.pageSize': 500,
      'conflicts.aiModel': 'model',
      'conflicts.aiEffort': 'high',
      'flowDiff.collapseUnchanged': true,
      'scm.stashButtons': true,
      'engine.path': 'x',
      'branches.sortOrder': 'name',
      'worktrees.linkFolders': [],
    };
    const settings = engineSettings(
      values,
      new Set(['branches.sortOrder', 'worktrees.linkFolders']),
      {
        branchSortOrder: 'committerdate',
        worktreeSymlinkFolders: ['node_modules'],
      },
      60,
    );
    expect(settings).toEqual({
      'graph.pageSize': 500,
      'ai.model': 'model',
      'ai.effort': 'high',
      'diff.collapseUnchanged': true,
      'branches.sortOrder': 'date',
      'worktrees.linkFolders': ['node_modules'],
      host: 'vscode',
      'ai.provider': 'vscode',
      trust: 'vscode',
      utcOffsetMinutes: 60,
    });
  });
});

/**
 * The extension's client against `gitardo-engine`, built by `cargo build -p server` in `../rust`: the acceptance lines of
 * stages 1 and 2 of `docs/transition/06-roadmap.md` that need the extension. The state the engine reads is compared with the
 * one `Repo.state` reads in the same throwaway repositories. Skipped, with the reason, when the binary is not built;
 * `GITARDO_ENGINE` names another one.
 */
import { execFileSync, spawn } from 'node:child_process';
import * as fs from 'node:fs';
import * as os from 'node:os';
import * as path from 'node:path';
import { afterEach, describe, expect, it } from 'vitest';
import { binaryName } from '../src/engine/binary';
import { EngineClient, type EngineLog, type FeatureAsker } from '../src/engine/client';
import { Connection, EngineError, ErrorCode } from '../src/engine/connection';
import { type Answer, type EngineState, PROTOCOL, type Plan, type Question, type Settings } from '../src/engine/messages';
import { engineSettings } from '../src/engine/settings';
import { headOf, recordOf, type UndoRecord } from '../src/engine/journal';
import { targetOf } from '../src/engine/target';
import { git } from '../src/git/exec';
import { commitOp, deleteBranchOp, type GitOp, mergeOp, splitHeadOp, stashOp } from '../src/git/ops';
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

function client(log = memoryLog(), settings: Settings = engineSettings({}, new Set(), {}, 0)): EngineClient {
  const started = new EngineClient({
    binary,
    clientVersion: '0.0.0-test',
    settings: () => settings,
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

  it('answers initialize with its version of the protocol, also to 1.0, and an initialize with 2.0 with -32602 and its own version', async () => {
    const child = spawn(binary, ['--stdio'], {
      stdio: ['pipe', 'pipe', 'inherit'],
    });
    const connection = new Connection(child.stdin, child.stdout);
    try {
      const answer = await connection.request<{ protocol: string }>('initialize', { protocol: '1.0', client: { name: 'test', version: '0' } });
      expect(answer.protocol).toBe(PROTOCOL);
      const refused = await connection
        .request('initialize', {
          protocol: '2.0',
          client: { name: 'test', version: '0' },
        })
        .catch((error: unknown) => error);
      expect(refused).toBeInstanceOf(EngineError);
      expect((refused as EngineError).code).toBe(ErrorCode.invalidParams);
      expect((refused as EngineError).data).toMatchObject({
        protocol: PROTOCOL,
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

/** An asker that answers by the key of each question, and keeps what it was asked. */
function answering(answers: Record<string, (question: Question) => Answer | null | Promise<Answer | null>>): FeatureAsker & {
  asked: Question[];
  confirmed: Plan[];
} {
  const asked: Question[] = [];
  const confirmed: Plan[] = [];
  return {
    asked,
    confirmed,
    async ask(question) {
      asked.push(question);
      const answer = answers[question.key];
      if (!answer) throw new Error(`No answer for ${question.key}`);
      return answer(question);
    },
    async confirm(plan) {
      confirmed.push(plan);
      return true;
    },
  };
}

const text = (value: string) => (): Answer => ({ kind: 'text', text: value });
const choice = (identifier: string) => (): Answer => ({ kind: 'choice', identifier });

describe.skipIf(!built)('Create branch through gitardo-engine', { timeout: 60_000 }, () => {
  it("asks the name on a commit, refuses feature x and a name taken with the engine's sentences while it is typed, and creates the branch with the commands the engine logs", async () => {
    const root = await repository();
    await commitFile(root, 'b.txt', 'two\n', 'Second commit');
    const first = (await git(root, ['rev-parse', 'HEAD~1'])).stdout.trim();
    const log = memoryLog();
    const engine = client(log);
    const said: Array<string | null> = [];
    const asker = answering({
      name: async (question) => {
        if (question.kind.kind !== 'text') throw new Error('the name is typed');
        for (const typed of ['feature x', 'main', 'topic']) said.push(await engine.validate(root, question.kind.validation, typed));
        return { kind: 'text', text: 'topic' };
      },
      mode: choice('checkout'),
    });
    const target = targetOf(first, { branches: [], remoteBranches: [], tags: [] });
    const { finished, followUps } = await engine.runFeature({ repository: root, feature: 'branch.create', target }, asker);
    expect(finished.kind).toBe('ran');
    expect(asker.asked.map((question) => question.key)).toEqual(['name', 'mode']);
    expect(said[0]).toEqual(expect.any(String));
    expect(said.slice(1)).toEqual(['A branch with that name already exists', null]);
    expect(followUps).toEqual([{ kind: 'selectHead' }]);
    expect((await git(root, ['symbolic-ref', '--short', 'HEAD'])).stdout.trim()).toBe('topic');
    expect((await git(root, ['rev-parse', 'HEAD'])).stdout.trim()).toBe(first);
    expect(log.lines).toContain(`info [engine] git checkout -b topic ${first} (${root})`);
  });

  it('proposes the name from gitardo.branches.format, with the inputs used last, and gives them back to remember', async () => {
    const root = await repository();
    const settings = engineSettings(
      { 'branches.format': '${input:ticket}-${base}', 'branches.inputs': [{ id: 'ticket', type: 'promptString', description: 'Ticket' }] },
      new Set(),
      {},
      0,
    );
    const engine = client(memoryLog(), settings);
    let proposed: Answer | null = null;
    const asker = answering({
      way: choice('format'),
      input1: (question) => question.default ?? { kind: 'text', text: '' },
      name: (question) => (proposed = question.default),
      mode: choice('only'),
    });
    const request = { repository: root, feature: 'branch.create', target: { kind: 'head' } as const, inputs: { ticket: 'T-1' } };
    const { finished, followUps } = await engine.runFeature(request, asker);
    expect(finished.kind).toBe('ran');
    expect(proposed).toEqual({ kind: 'text', text: 'T-1-main' });
    expect(followUps).toContainEqual({ kind: 'rememberInputs', values: { ticket: 'T-1' } });
    expect((await git(root, ['branch', '--list', 'T-1-main'])).stdout.trim()).toBe('T-1-main');
  });

  it('runs branch.create with the same answers through gitardo-cli and through the extension, with the same plan and the same repository after', async () => {
    const root = await repository();
    await commitFile(root, 'b.txt', 'two\n', 'Second commit');
    const copy = path.join(folder(), 'repo');
    fs.cpSync(root, copy, { recursive: true });
    const first = (await git(root, ['rev-parse', 'HEAD~1'])).stdout.trim();
    const cli = path.join(path.dirname(binary), process.platform === 'win32' ? 'gitardo-cli.exe' : 'gitardo-cli');
    const arguments_ = ['--repository', copy, '--target', `commit:${first}`, 'branch.create', 'name=topic', 'mode=checkout'];
    const printed = execFileSync(cli, arguments_, { encoding: 'utf8' });

    const engine = client();
    const { finished } = await engine.runFeature(
      { repository: root, feature: 'branch.create', target: { kind: 'commit', commit: first } },
      answering({ name: text('topic'), mode: choice('checkout') }),
    );
    if (finished.kind !== 'ran') throw new Error(`branch.create ended ${finished.kind}`);
    const plan = [finished.plan.title, ...finished.plan.steps.map((step) => `  > ${step.line}`)];
    expect(printed.trim().split(/\r?\n/)).toEqual(plan);
    const refs = async (where: string) => [
      (await git(where, ['symbolic-ref', 'HEAD'])).stdout.trim(),
      (await git(where, ['for-each-ref', '--format=%(refname) %(objectname)'])).stdout.trim(),
      (await git(where, ['status', '--porcelain'])).stdout.trim(),
    ];
    expect(await refs(root)).toEqual(await refs(copy));
  });
});

describe('the target of a feature from what a menu names', () => {
  const refs = { branches: [{ name: 'main' }], remoteBranches: [{ name: 'origin/main' }], tags: [{ name: 'v1' }] };
  it('takes HEAD, a full name, a short name of each kind, and a commit', () => {
    expect(targetOf('HEAD', refs)).toEqual({ kind: 'head' });
    expect(targetOf('refs/tags/main', refs)).toEqual({ kind: 'tag', name: 'main' });
    expect(targetOf('main', refs)).toEqual({ kind: 'branch', name: 'main' });
    expect(targetOf('origin/main', refs)).toEqual({ kind: 'remoteBranch', name: 'origin/main' });
    expect(targetOf('v1', refs)).toEqual({ kind: 'tag', name: 'v1' });
    expect(targetOf('a'.repeat(40), refs)).toEqual({ kind: 'commit', commit: 'a'.repeat(40) });
  });
});

describe('the settings sent to the engine', () => {
  it('sends the format of branch names of the worktrees where gitardo.branches.format is empty, as it was before', () => {
    const inputs = [{ id: 'ticket', type: 'promptString' }];
    const values = { 'branches.format': '', 'branches.inputs': [], 'worktrees.branchFormat': '${input:ticket}', 'worktrees.inputs': inputs };
    const settings = engineSettings(values, new Set(), {}, 0);
    expect(settings['branches.format']).toBe('${input:ticket}');
    expect(settings['branches.inputs']).toEqual(inputs);
  });

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

/**
 * Runs a `GitOp` as `Controller.run` does, then tells the engine's journal about it as `Controller.recordInJournal`
 * does: with `record` when the operation says what Undo needs of it, else with what the states before and after tell.
 */
async function ranInTypeScript(engine: EngineClient, root: string, op: GitOp, record?: UndoRecord): Promise<number | null> {
  const read = async () => new Repo((await Repo.discover(root))!).state(10);
  const before = await read();
  let failure: unknown;
  for (const [index, args] of op.commands.entries()) {
    try {
      await git(root, args, { input: index === op.commands.length - 1 ? op.input : undefined });
    } catch (stopped) {
      failure = stopped;
      break;
    }
  }
  const after = await read();
  const told = record ?? recordOf(before, after);
  const number = told ? await engine.journalRecord(root, op.title, told, headOf(before), headOf(after)) : null;
  if (failure && !record) return null;
  return number;
}

/** Runs **Undo** as the toolbar does once the engine runs: `feature/run` of `undo.last`, its plan confirmed. */
async function undo(engine: EngineClient, root: string) {
  const { finished } = await engine.runFeature({ repository: root, feature: 'undo.last', target: { kind: 'head' } }, answering({}));
  return finished;
}

const head = async (root: string) => (await git(root, ['rev-parse', 'HEAD'])).stdout.trim();

describe.skipIf(!built)('Undo through gitardo-engine, of operations run by either', { timeout: 60_000 }, () => {
  it('takes back a branch created by the engine, then one deleted in TypeScript, from the journal', async () => {
    const root = await repository();
    await git(root, ['branch', 'old']);
    const commit = await head(root);
    const engine = client();
    const number = await ranInTypeScript(engine, root, deleteBranchOp('old'), { kind: 'branchDeleted', name: 'old', commit });
    expect(number).toEqual(expect.any(Number));
    const asker = answering({ name: text('spike'), mode: choice('only') });
    expect((await engine.runFeature({ repository: root, feature: 'branch.create', target: { kind: 'head' } }, asker)).finished.kind).toBe('ran');
    expect(await engine.journalLast(root)).toMatchObject({ title: 'Create branch spike at HEAD', undoable: true });

    expect((await undo(engine, root)).kind).toBe('ran');
    expect((await git(root, ['branch', '--list', 'spike'])).stdout.trim()).toBe('');
    expect((await undo(engine, root)).kind).toBe('ran');
    expect((await git(root, ['rev-parse', 'old'])).stdout.trim()).toBe(commit);
  });

  it('takes back a commit made in TypeScript, then the stash dropped in TypeScript before it', async () => {
    const root = await repository();
    fs.writeFileSync(path.join(root, 'a.txt'), 'changed\n');
    await git(root, ['stash', 'push', '-m', 'half a fix']);
    const stash = (await git(root, ['rev-parse', 'stash@{0}'])).stdout.trim();
    const engine = client();
    const { message } = (await new Repo((await Repo.discover(root))!).state(10)).stashes[0];
    await ranInTypeScript(engine, root, stashOp('drop', 'stash@{0}'), { kind: 'stashDropped', message, commit: stash });
    const before = await head(root);
    fs.writeFileSync(path.join(root, 'b.txt'), 'b\n');
    await ranInTypeScript(engine, root, commitOp('Add b', false, true));

    expect((await undo(engine, root)).kind).toBe('ran');
    expect(await head(root)).toBe(before);
    expect((await undo(engine, root)).kind).toBe('ran');
    expect((await git(root, ['rev-parse', 'stash@{0}'])).stdout.trim()).toBe(stash);
  });

  it('says right after a push that it cannot take it back, and takes back nothing', async () => {
    const remote = await repository();
    const root = path.join(folder(), 'clone');
    await git(path.dirname(root), ['clone', '--quiet', remote, root]);
    await git(root, ['config', 'user.name', 'Gitardo Test']);
    await git(root, ['config', 'user.email', 'test@gitardo.invalid']);
    await git(root, ['config', 'commit.gpgsign', 'false']);
    await git(remote, ['checkout', '--quiet', '--detach']);
    await commitFile(root, 'b.txt', 'b\n', 'Add b');
    const engine = client();
    await ranInTypeScript(engine, root, { title: 'Push main to origin', commands: [['push', 'origin', 'main']] });
    const pushed = await head(root);
    expect(await engine.journalLast(root)).toMatchObject({ title: 'Push main to origin', undoable: false });

    expect(await undo(engine, root)).toMatchObject({
      kind: 'refused',
      message: '"Push main to origin" cannot be undone. It changed the remote branches, which Undo does not take back.',
    });
    expect(await head(root)).toBe(pushed);
  });

  it('refuses during a merge stopped on conflicts with the sentence of the engine', async () => {
    const root = await repository();
    await git(root, ['checkout', '--quiet', '-b', 'other']);
    await commitFile(root, 'a.txt', 'other\n', 'Other');
    await git(root, ['checkout', '--quiet', 'main']);
    await commitFile(root, 'a.txt', 'main\n', 'Main');
    const engine = client();
    await ranInTypeScript(engine, root, mergeOp('other', 'main'));
    expect(await undo(engine, root)).toMatchObject({ kind: 'refused', message: 'Finish or abort the merge first.' });
  });

  it('takes back whole a squash merge, a split and a resolution of conflicts', async () => {
    const root = await repository();
    await git(root, ['checkout', '--quiet', '-b', 'other']);
    await commitFile(root, 'b.txt', 'b\n', 'Add b');
    await commitFile(root, 'c.txt', 'c\n', 'Add c');
    await git(root, ['checkout', '--quiet', 'main']);
    const engine = client();
    await ranInTypeScript(engine, root, mergeOp('other', 'main', 'squash'), { kind: 'squashMerge' });
    expect((await undo(engine, root)).kind).toBe('ran');
    expect((await git(root, ['status', '--porcelain'])).stdout.trim()).toBe('');

    await commitFile(root, 'd.txt', 'd\n', 'Add d');
    const split = await head(root);
    await ranInTypeScript(engine, root, splitHeadOp(split), { kind: 'splitLast', commit: split });
    expect((await undo(engine, root)).kind).toBe('ran');
    expect(await head(root)).toBe(split);
    expect((await git(root, ['status', '--porcelain'])).stdout.trim()).toBe('');

    await commitFile(root, 'a.txt', 'main\n', 'Main');
    await git(root, ['checkout', '--quiet', 'other']);
    await commitFile(root, 'a.txt', 'other\n', 'Other');
    await git(root, ['checkout', '--quiet', 'main']);
    await git(root, ['merge', 'other']).catch(() => null);
    fs.writeFileSync(path.join(root, 'a.txt'), 'both\n');
    const resolve: GitOp = { title: 'Resolve the conflicts of a.txt', commands: [['add', '--', 'a.txt']] };
    await ranInTypeScript(engine, root, resolve, { kind: 'conflictsResolved', files: ['a.txt'] });
    expect((await undo(engine, root)).kind).toBe('ran');
    expect(fs.readFileSync(path.join(root, 'a.txt'), 'utf8')).toContain('<<<<<<<');
  });

  it('goes by the reflog once the engine restarted, its journal empty', async () => {
    const root = await repository();
    const first = await head(root);
    const before = client();
    fs.writeFileSync(path.join(root, 'b.txt'), 'b\n');
    await ranInTypeScript(before, root, commitOp('Add b', false, true));
    await ranInTypeScript(before, root, { title: 'Create tag v1', commands: [['tag', 'v1']] });
    expect(await before.journalLast(root)).toMatchObject({ undoable: false });
    await before.dispose();

    const engine = client();
    expect(await engine.journalLast(root)).toBeNull();
    expect((await undo(engine, root)).kind).toBe('ran');
    expect(await head(root)).toBe(first);
  });
});

describe('what the journal is told of an operation run in TypeScript', () => {
  const state = (changes: Partial<RepoState>) =>
    ({
      head: 'a'.repeat(40),
      branch: 'main',
      branches: [],
      remoteBranches: [],
      tags: [],
      stashes: [],
      worktrees: [],
      remotes: [],
      submodules: [],
      ...changes,
    }) as unknown as RepoState;

  it('is a move of HEAD for the reflog, the references it changed with a reason, and nothing when only files changed', () => {
    expect(recordOf(state({}), state({ head: 'b'.repeat(40) }))).toEqual({ kind: 'reflog' });
    expect(recordOf(state({}), state({ branch: null }))).toEqual({ kind: 'reflog' });
    expect(recordOf(state({}), state({ tags: [{ name: 'v1', hash: 'a'.repeat(40) }] }))).toEqual({
      kind: 'notUndoable',
      reason: 'It changed the tags, which Undo does not take back.',
    });
    const stashed = { ref: 'stash@{0}', hash: 'c'.repeat(40), message: 'm', date: 0 };
    expect(recordOf(state({}), state({ remotes: ['origin'], stashes: [stashed] }))).toEqual({
      kind: 'notUndoable',
      reason: 'It changed the stashes and remotes, which Undo does not take back.',
    });
    expect(recordOf(state({}), state({ changes: 3 }))).toBeNull();
  });

  it('names the branch by its full name, and none when detached', () => {
    expect(headOf({ head: 'a'.repeat(40), branch: 'main' })).toEqual({ commit: 'a'.repeat(40), branch: 'refs/heads/main' });
    expect(headOf({ head: 'a'.repeat(40), branch: null })).toEqual({ commit: 'a'.repeat(40), branch: null });
  });
});

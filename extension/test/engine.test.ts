/**
 * The extension's client against `gitardo-engine`, built by `cargo build -p server` in `../rust`: the acceptance lines of
 * stages 1 and 2 of `docs/transition/06-roadmap.md` that need the extension. The state the engine reads is compared with
 * what git itself says of the same throwaway repositories. Skipped, with the reason, when the binary is not built;
 * `GITARDO_ENGINE` names another one.
 */
import { execFileSync, spawn } from 'node:child_process';
import * as fs from 'node:fs';
import * as path from 'node:path';
import { describe, expect, it } from 'vitest';
import { Connection, EngineError, ErrorCode } from '../src/engine/connection';
import { type Answer, PROTOCOL } from '../src/engine/messages';
import { engineSettings } from '../src/engine/settings';
import { answering, built, binary, choice, cleanUpAfterEach, client, commitFile, folder, memoryLog, repository, text } from './engine/harness';
import { targetOf } from '../src/engine/target';
import type { RepositoryState } from '../src/shared/protocol';
import { git } from './git';

cleanUpAfterEach();

/** A path as both sides can be compared by. */
function samePath(named: string): string {
  const resolved = path.resolve(named);
  return process.platform === 'win32' ? resolved.toLowerCase() : resolved;
}

const byName = <T extends { name: string }>(refs: T[]) => [...refs].sort((one, other) => one.name.localeCompare(other.name));

/** The paths of a list, in one order. */
const paths = (files: Array<{ path: string }>) => files.map((file) => file.path).sort();

/**
 * The parts of a state that git says too, as both sides can be compared by: the folders as the system names them, the
 * references by their names and commits, and the files changed by their paths, in one order.
 */
function comparable(state: RepositoryState) {
  return {
    folder: samePath(state.folder),
    gitFolder: samePath(state.gitFolder),
    head: state.head,
    branches: byName(state.branches).map((branch) => ({
      name: branch.name,
      commit: branch.commit,
      upstream: branch.upstream && {
        reference: branch.upstream.reference,
        ahead: branch.upstream.ahead,
        behind: branch.upstream.behind,
        isGone: branch.upstream.isGone,
      },
    })),
    remoteBranches: byName(state.remoteBranches).map(({ name, commit }) => ({ name, commit })),
    tags: byName(state.tags).map(({ name, commit }) => ({ name, commit })),
    stashes: state.stashes.map(({ commit, index }) => ({ commit, index })),
    worktrees: state.worktrees.map((worktree) => ({
      folder: samePath(worktree.folder),
      commit: worktree.commit,
      branch: worktree.branch,
      isMissing: worktree.isMissing,
    })),
    remotes: [...state.remotes].sort(),
    staged: paths(state.workingTree.staged),
    unstaged: paths(state.workingTree.unstaged),
    untracked: [...state.workingTree.untracked].sort(),
    conflicted: paths(state.workingTree.conflicted),
    merging: state.operation === 'merge',
  };
}

/** What git itself says of the parts of a repository that `comparable` keeps of a state. */
async function fromGit(root: string): Promise<ReturnType<typeof comparable>> {
  const out = async (args: string[]) => (await git(root, args)).stdout.trim();
  const lines = async (args: string[]) => (await out(args)).split('\n').filter(Boolean);
  const fields = async (args: string[]) => (await lines(args)).map((line) => line.split('\u0001'));
  const commit = await out(['rev-parse', '-q', '--verify', 'HEAD']).catch(() => '');
  const branch = await out(['symbolic-ref', '-q', 'HEAD']).catch(() => '');
  const branches = await fields([
    'for-each-ref',
    '--format=%(refname:short)\u0001%(objectname)\u0001%(upstream)\u0001%(upstream:track,nobracket)',
    'refs/heads',
  ]);
  const count = (track: string, word: string) => Number(new RegExp(`${word} (\\d+)`).exec(track)?.[1] ?? 0);
  const remoteBranches = await fields(['for-each-ref', '--format=%(refname:short)\u0001%(objectname)\u0001%(symref)', 'refs/remotes']);
  const tags = await fields(['for-each-ref', '--format=%(refname:short)\u0001%(objectname)\u0001%(*objectname)', 'refs/tags']);
  const stashes = await lines(['stash', 'list', '--format=%H']);
  const worktrees: Array<ReturnType<typeof comparable>['worktrees'][number]> = [];
  for (const block of (await out(['worktree', 'list', '--porcelain'])).split('\n\n')) {
    const entry = Object.fromEntries(block.split('\n').map((line) => [line.split(' ')[0], line.slice(line.indexOf(' ') + 1)]));
    const folder = entry.worktree.split('/').join(path.sep);
    worktrees.push({
      folder: samePath(folder),
      // A worktree on a branch with no commit yet is at no commit, which git writes as zeros.
      commit: entry.HEAD && !/^0+$/.test(entry.HEAD) ? entry.HEAD : null,
      branch: entry.branch ?? null,
      isMissing: 'prunable' in entry || !fs.existsSync(folder),
    });
  }
  const status = (await git(root, ['status', '--porcelain=v1', '-z', '--untracked-files=all'])).stdout.split('\0');
  const staged: string[] = [];
  const unstaged: string[] = [];
  const untracked: string[] = [];
  const conflicted: string[] = [];
  for (let index = 0; index < status.length; index++) {
    const entry = status[index];
    if (!entry) continue;
    const [x, y, file] = [entry[0], entry[1], entry.slice(3)];
    // A rename names the path it had next.
    if (x === 'R' || x === 'C') index++;
    if (x === '?') untracked.push(file);
    else if (x === 'U' || y === 'U' || (x === 'A' && y === 'A') || (x === 'D' && y === 'D')) conflicted.push(file);
    else {
      if (x !== ' ') staged.push(file);
      if (y !== ' ') unstaged.push(file);
    }
  }
  return {
    folder: samePath(await out(['rev-parse', '--show-toplevel'])),
    gitFolder: samePath(await out(['rev-parse', '--absolute-git-dir'])),
    head: { commit: commit || null, branch: branch || null },
    branches: branches
      .map(([name, hash, upstream, track]) => ({
        name,
        commit: hash,
        upstream: upstream ? { reference: upstream, ahead: count(track, 'ahead'), behind: count(track, 'behind'), isGone: track === 'gone' } : null,
      }))
      .sort((one, other) => one.name.localeCompare(other.name)),
    remoteBranches: remoteBranches
      .filter(([, , symref]) => !symref)
      .map(([name, hash]) => ({ name, commit: hash }))
      .sort((one, other) => one.name.localeCompare(other.name)),
    tags: tags.map(([name, hash, peeled]) => ({ name, commit: peeled || hash })).sort((one, other) => one.name.localeCompare(other.name)),
    stashes: stashes.map((hash, index) => ({ commit: hash, index })),
    worktrees,
    remotes: (await lines(['remote'])).sort(),
    staged: staged.sort(),
    unstaged: unstaged.sort(),
    untracked: untracked.sort(),
    conflicted: conflicted.sort(),
    merging: fs.existsSync(path.join(await out(['rev-parse', '--absolute-git-dir']), 'MERGE_HEAD')),
  };
}

/** Opens a repository in the engine, and says whether what it read is what git says. */
async function compare(root: string): Promise<void> {
  const read = await client().open(root);
  expect(comparable(read)).toEqual(await fromGit(root));
}

describe.skipIf(!built)('gitardo-engine from the extension', { timeout: 60_000 }, () => {
  describe('reads the state git says', () => {
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
    const states: RepositoryState[] = [];
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
    expect(log.lines).toContain(`info > git checkout -b topic ${first}`);
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

/** Runs **Undo** as the toolbar does: `feature/run` of `undo.last`, its plan confirmed. */
async function undo(engine: ReturnType<typeof client>, root: string) {
  const { finished } = await engine.runFeature({ repository: root, feature: 'undo.last', target: { kind: 'head' } }, answering({}));
  return finished;
}

const head = async (root: string) => (await git(root, ['rev-parse', 'HEAD'])).stdout.trim();

describe.skipIf(!built)('Undo through gitardo-engine', { timeout: 60_000 }, () => {
  it('takes back a branch the engine created, from its journal', async () => {
    const root = await repository();
    const engine = client();
    const asker = answering({ name: text('spike'), mode: choice('only') });
    expect((await engine.runFeature({ repository: root, feature: 'branch.create', target: { kind: 'head' } }, asker)).finished.kind).toBe('ran');
    expect(await engine.journalLast(root)).toMatchObject({ title: 'Create branch spike at HEAD', undoable: true });
    expect((await undo(engine, root)).kind).toBe('ran');
    expect((await git(root, ['branch', '--list', 'spike'])).stdout.trim()).toBe('');
  });

  it('refuses during a merge stopped on conflicts with the sentence of the engine', async () => {
    const root = await repository();
    await git(root, ['checkout', '--quiet', '-b', 'other']);
    await commitFile(root, 'a.txt', 'other\n', 'Other');
    await git(root, ['checkout', '--quiet', 'main']);
    await commitFile(root, 'a.txt', 'main\n', 'Main');
    await git(root, ['merge', 'other']).catch(() => null);
    expect(await undo(client(), root)).toMatchObject({ kind: 'refused', message: 'Finish or abort the merge first.' });
  });

  it('goes by the reflog for a commit made outside Gitardo, its journal empty', async () => {
    const root = await repository();
    const first = await head(root);
    await commitFile(root, 'b.txt', 'b\n', 'Add b');
    const engine = client();
    expect(await engine.journalLast(root)).toBeNull();
    expect((await undo(engine, root)).kind).toBe('ran');
    expect(await head(root)).toBe(first);
  });
});

/**
 * The scenarios of the fifth area moved to Gitardo's engine in stage 4 of `docs/transition/06-roadmap.md`: worktrees,
 * what new worktrees get, and submodules. Each operation runs through the engine's feature, with the target and
 * answers `engineRuns` gives it, and must run the commands it ran when it was a `GitOp` of the extension, but where
 * `docs/transition/04-differences.md` says why not. A worktree's folder is beside the repository, so its paths are
 * written from the folder that holds the repository.
 */
import * as fs from 'node:fs';
import * as path from 'node:path';
import { afterAll, beforeAll, describe, expect, it } from 'vitest';
import type { EngineClient } from '../src/engine/client';
import type { EngineError } from '../src/engine/connection';
import { engineRuns } from '../src/engine/actions';
import type { Answer, Finished, Plan, Question, Step, Target } from '../src/engine/messages';
import { engineSettings } from '../src/engine/settings';
import type { Action, RepoState } from '../src/shared/protocol';
import { git } from './git';
import { built, cleanUpAfterEach, client, commitFile, engineState, folder, memoryLog, repository, snapshot } from './engine/harness';

cleanUpAfterEach();

// Git clones submodules from folders on disk only when told it may; the tests tell it so, and the engine's retry with
// the file protocol allowed has tests of its own.
const allowed = process.env.GIT_ALLOW_PROTOCOL;
beforeAll(() => {
  process.env.GIT_ALLOW_PROTOCOL = 'file';
});
afterAll(() => {
  if (allowed === undefined) delete process.env.GIT_ALLOW_PROTOCOL;
  else process.env.GIT_ALLOW_PROTOCOL = allowed;
});

/** What the extension reads of a repository through the engine, for `engineRuns`. */
async function state(root: string): Promise<RepoState> {
  return engineState(root);
}

/** The folder a new worktree named `name` gets in the folder of the worktrees beside `root`, as the engine writes it. */
const worktreeOf = (root: string, name: string) => `${path.dirname(root).split(path.sep).join('/')}/${path.basename(root)}.worktrees/${name}`;

/** A repository made by `setup`, with the worktrees it made beside it. */
async function made(setup: (root: string) => Promise<void>): Promise<string> {
  const root = await repository();
  await setup(root);
  return root;
}

/** A path, or an argument that is one, from the folder that holds `root`, with forward slashes. */
function fromParent(value: string, root: string): string {
  const parent = path.dirname(root);
  const forms = [parent, parent.split(path.sep).join('/')];
  for (const form of forms) {
    const at = value.toLowerCase().indexOf(form.toLowerCase());
    if (at >= 0) return `${value.slice(0, at)}<parent>${value.slice(at + form.length).split('\\').join('/')}`;
  }
  return value;
}

/** A command, from the folder that holds `root`. */
const commandLine = (args: string[], root: string) => ['git', ...args.map((argument) => fromParent(argument, root))].join(' ');

/** The steps of a plan, the commands among them by their arguments, from the folder that holds `root`. */
function planLines(plan: Plan, root: string): string[] {
  return plan.steps.map((step: Step) => (step.kind === 'git' && Array.isArray(step.arguments) ? commandLine(step.arguments as string[], root) : fromParent(step.line, root)));
}

/** The worktrees of a repository, by their folders from the one that holds it, with what each has checked out. */
async function worktreesOf(root: string): Promise<string[]> {
  const listed = (await git(root, ['worktree', 'list', '--porcelain'])).stdout;
  return listed
    .split('\n')
    .filter((line) => line.startsWith('worktree ') || line.startsWith('branch ') || line === 'detached')
    .map((line) => fromParent(line, root));
}

/** What is in the folder of the worktrees beside a repository, but what git keeps in each worktree. */
function worktreeFolders(root: string): string[] | null {
  const worktrees = path.join(path.dirname(root), 'repo.worktrees');
  const folders = fs.existsSync(worktrees)
    ? fs.readdirSync(worktrees, { recursive: true, encoding: 'utf8' }).filter((entry) => !entry.split(/[\\/]/).includes('.git'))
    : null;
  return folders?.map((entry) => entry.split(path.sep).join('/')).sort() ?? null;
}

/**
 * Runs `action` through the engine as `engineRuns` maps it, its questions answered by key or else by their default,
 * each plan confirmed; gives the lines of the steps of each plan run, from the folder that holds `root`.
 */
async function runAction(
  root: string,
  action: Action,
  answers: Record<string, Answer> = {},
  engine: EngineClient = client(),
): Promise<{ lines: string[]; finished: Finished[] }> {
  const runs = engineRuns(action, await state(root));
  if (!runs) throw new Error(`${action.kind} does not run in the engine`);
  const lines: string[] = [];
  const finished: Finished[] = [];
  for (const run of runs) {
    const asker = {
      async ask(question: Question) {
        const answer = answers[question.key] ?? question.default;
        if (!answer) throw new Error(`No answer for ${question.key}`);
        return answer;
      },
      async confirm() {
        return true;
      },
    };
    const result = await engine.runFeature({ repository: root, feature: run.feature, target: run.target, answers: run.answers }, asker);
    expect(result.finished.kind, JSON.stringify(result.finished)).toBe('ran');
    if (result.finished.kind === 'ran') lines.push(...planLines(result.finished.plan, root));
    finished.push(result.finished);
  }
  return { lines, finished };
}

const text = (value: string): Answer => ({ kind: 'text', text: value });
const choice = (identifier: string): Answer => ({ kind: 'choice', identifier });

/** A branch `topic` with a commit of its own, and `main` checked out. */
async function topic(root: string): Promise<void> {
  await git(root, ['checkout', '--quiet', '-b', 'topic']);
  await commitFile(root, 'b.txt', 'b\n', 'Topic');
  await git(root, ['checkout', '--quiet', 'main']);
}

/** A worktree `feature` on a new branch of that name, beside the repository. */
async function withWorktree(root: string): Promise<void> {
  await git(root, ['worktree', 'add', '--quiet', '-b', 'feature', worktreeOf(root, 'feature'), 'main']);
}

/** A repository of its own, with two commits, for a submodule to be cloned from. */
async function library(): Promise<string> {
  const root = path.join(folder(), 'lib');
  fs.mkdirSync(root);
  await git(root, ['init', '--quiet', '--initial-branch=main']);
  await git(root, ['config', 'user.name', 'Gitardo Test']);
  await git(root, ['config', 'user.email', 'test@gitardo.invalid']);
  await commitFile(root, 'lib.txt', 'one\n', 'Library');
  return root;
}

/** The submodule `lib`, cloned from `source`, committed. */
const withSubmodule = (source: string) => async (root: string) => {
  await git(root, ['submodule', 'add', '--quiet', '--', source, 'lib']);
  await git(root, ['commit', '--quiet', '-m', 'Add lib']);
};

describe.skipIf(!built)('Worktrees, through the engine', { timeout: 60_000 }, () => {
  it('creates a worktree on a new branch from the branch checked out', async () => {
    const root = await made(async () => {});
    const engine = await runAction(root, { kind: 'worktreeAdd' }, { name: text('feature') });
    expect(engine.lines).toEqual(['git worktree add --no-track -b feature <parent>/repo.worktrees/feature main']);
    expect(await worktreesOf(root)).toEqual(expect.arrayContaining(['worktree <parent>/repo.worktrees/feature', 'branch refs/heads/feature']));
  });

  it('creates a worktree for a branch, named after it', async () => {
    const root = await made(topic);
    const engine = await runAction(root, { kind: 'worktreeAdd', ref: 'topic', isBranch: true });
    expect(engine.lines).toEqual(['git worktree add <parent>/repo.worktrees/topic topic']);
    expect(await worktreesOf(root)).toEqual(expect.arrayContaining(['worktree <parent>/repo.worktrees/topic', 'branch refs/heads/topic']));
  });

  it('creates a worktree at a commit, on a new branch', async () => {
    const root = await made(topic);
    const commit = (await git(root, ['rev-parse', 'topic~1'])).stdout.trim();
    const engine = await runAction(root, { kind: 'worktreeAdd', ref: commit, isBranch: false }, { name: text('spike') });
    expect(engine.lines).toEqual([`git worktree add --no-track -b spike <parent>/repo.worktrees/spike ${commit}`]);
    expect((await git(root, ['rev-parse', 'spike'])).stdout.trim()).toBe(commit);
  });

  it('removes a worktree and its branch, and deletes the folders it leaves empty before the branch (W8)', async () => {
    const root = await made(async (setup) => {
      await git(setup, ['worktree', 'add', '--quiet', '-b', 'team/feature', worktreeOf(setup, 'team/feature'), 'main']);
    });
    const engine = await runAction(root, { kind: 'worktreeRemove', path: worktreeOf(root, 'team/feature'), deleteBranch: true });
    expect(engine.lines).toEqual(['git worktree remove <parent>/repo.worktrees/team/feature', 'delete <parent>/repo.worktrees/team', 'git branch -d team/feature']);
    expect(worktreeFolders(root)).toEqual([]);
    expect((await git(root, ['branch', '--list', 'team/feature'])).stdout).toBe('');
  });

  it('renames a worktree and its branch', async () => {
    const root = await made(withWorktree);
    const engine = await runAction(
      root,
      { kind: 'worktreeRename', path: worktreeOf(root, 'feature') },
      { 'new name': text('better'), 'rename branch': choice('rename'), 'branch name': text('better') },
    );
    expect(engine.lines).toEqual(['git worktree move <parent>/repo.worktrees/feature <parent>/repo.worktrees/better', 'git branch -m feature better']);
    expect(await worktreesOf(root)).toEqual(expect.arrayContaining(['worktree <parent>/repo.worktrees/better', 'branch refs/heads/better']));
  });

  it('prunes the worktrees whose folders are gone', async () => {
    const root = await made(async (setup) => {
      await withWorktree(setup);
      fs.rmSync(worktreeOf(setup, 'feature'), { recursive: true, force: true });
    });
    const engine = await runAction(root, { kind: 'worktreePrune' });
    expect(engine.lines).toEqual(['git worktree prune']);
    expect((await worktreesOf(root)).filter((line) => line.startsWith('worktree '))).toHaveLength(1);
  });

  it('worktree.lock and worktree.unlock: locks a worktree with the reason asked, then unlocks it, in the engine', async () => {
    const root = await made(withWorktree);
    const folder = worktreeOf(root, 'feature');
    const locked = await runAction(root, { kind: 'worktreeLock', path: folder }, { reason: text('On the USB drive') });
    expect(locked.lines).toEqual(['git worktree lock --reason On the USB drive <parent>/repo.worktrees/feature']);
    const listed = async () => (await git(root, ['worktree', 'list', '--porcelain'])).stdout.split(/\r?\n/);
    expect(await listed()).toContain('locked On the USB drive');
    const unlocked = await runAction(root, { kind: 'worktreeUnlock', path: folder });
    expect(unlocked.lines).toEqual(['git worktree unlock <parent>/repo.worktrees/feature']);
    expect((await listed()).some((line) => line.startsWith('locked'))).toBe(false);
  });

  it('opens a remote branch in a new worktree, on a local branch that tracks it', async () => {
    const root = await made(async (setup) => {
      await git(setup, ['branch', 'fix']);
      await git(setup, ['remote', 'add', 'origin', setup]);
      await git(setup, ['fetch', '--quiet', 'origin']);
      await git(setup, ['branch', '-D', 'fix']);
    });
    const engine = await runAction(root, { kind: 'openInWorktree', name: 'origin/fix', remote: true });
    expect(engine.lines).toEqual(['git worktree add --track -b fix <parent>/repo.worktrees/fix origin/fix']);
    expect((await git(root, ['rev-parse', '--abbrev-ref', 'fix@{upstream}'])).stdout.trim()).toBe('origin/fix');
  });

  it('brings what git ignores into a worktree: the folders linked, the files copied', async () => {
    const ignored = async (root: string) => {
      fs.writeFileSync(path.join(root, '.gitignore'), 'node_modules\n.env\n');
      await git(root, ['add', '.gitignore']);
      await git(root, ['commit', '--quiet', '-m', 'Ignore']);
      fs.mkdirSync(path.join(root, 'node_modules', 'left-pad'), { recursive: true });
      fs.writeFileSync(path.join(root, 'node_modules', 'left-pad', 'index.js'), 'pad\n');
      fs.writeFileSync(path.join(root, '.env'), 'KEY=value\n');
      await withWorktree(root);
    };
    const root = await made(ignored);
    const settings = engineSettings({ 'worktrees.linkFolders': ['node_modules'], 'worktrees.copyFiles': ['.env'] }, new Set(), {}, 0);
    await runAction(root, { kind: 'worktreeSetUp', path: worktreeOf(root, 'feature') }, {}, client(memoryLog(), settings));
    const worktree = worktreeOf(root, 'feature');
    expect({
      linked: fs.lstatSync(path.join(worktree, 'node_modules')).isSymbolicLink(),
      package: fs.readFileSync(path.join(worktree, 'node_modules', 'left-pad', 'index.js'), 'utf8'),
      env: fs.readFileSync(path.join(worktree, '.env'), 'utf8'),
    }).toEqual({ linked: true, package: 'pad\n', env: 'KEY=value\n' });
  });

  it('offers Remove anyway when git refuses a worktree with changes, and removes it forced when clicked (W7)', async () => {
    const root = await repository();
    await withWorktree(root);
    const worktree = worktreeOf(root, 'feature');
    fs.writeFileSync(path.join(worktree, 'a.txt'), 'changed\n');
    const engine = client();
    const failure = await runAction(root, { kind: 'worktreeRemove', path: worktree, deleteBranch: false }, {}, engine).then(
      () => null,
      (error: unknown) => error as EngineError,
    );
    const offers = ((failure?.data as { followUps?: Array<Record<string, unknown>> } | undefined)?.followUps ?? []).filter((each) => each.kind === 'offerRun');
    expect(offers).toEqual([
      { kind: 'offerRun', label: 'Remove anyway', feature: 'worktree.remove', target: { kind: 'worktree', folder: expect.any(String) }, answers: expect.objectContaining({ force: { kind: 'flag', value: true } }) },
    ]);
    const [offer] = offers as Array<{ feature: string; target: Target; answers: Record<string, Answer> }>;
    const { finished } = await engine.runFeature({ repository: root, feature: offer.feature, target: offer.target, answers: offer.answers }, { ask: async () => null, confirm: async () => true });
    expect(finished.kind === 'ran' && planLines(finished.plan, root)).toEqual(['git worktree remove --force <parent>/repo.worktrees/feature']);
    expect(fs.existsSync(worktree)).toBe(false);
  });

  it('hands the setup commands of a new worktree over to be run in a terminal of VS Code, in the worktree', async () => {
    const root = await repository();
    const settings = engineSettings({ 'worktrees.setupCommands': ['npm ci'] }, new Set(), {}, 0);
    const { finished } = await runAction(root, { kind: 'worktreeAdd' }, { name: text('feature') }, client(memoryLog(), settings));
    const [made] = finished;
    expect(made.kind === 'ran' && made.waiting.map((step) => [step.kind, step.command])).toEqual([['task', 'npm ci']]);
    // Said in the plan, but not run by the engine.
    expect(made.kind === 'ran' && made.steps.filter((line) => line.startsWith('run '))).toEqual([]);
  });

  it.runIf(process.platform === 'win32')(
    'removes a worktree with a folder linked into it and leaves the folder of the checkout it links to whole',
    async () => {
      const root = await repository();
      fs.writeFileSync(path.join(root, '.gitignore'), 'node_modules\n');
      await git(root, ['add', '.gitignore']);
      await git(root, ['commit', '--quiet', '-m', 'Ignore']);
      fs.mkdirSync(path.join(root, 'node_modules', 'left-pad'), { recursive: true });
      fs.writeFileSync(path.join(root, 'node_modules', 'left-pad', 'index.js'), 'pad\n');
      const settings = engineSettings({ 'worktrees.linkFolders': ['node_modules'] }, new Set(), {}, 0);
      const engine = client(memoryLog(), settings);
      await runAction(root, { kind: 'worktreeAdd' }, { name: text('feature') }, engine);
      const worktree = worktreeOf(root, 'feature');
      expect(fs.lstatSync(path.join(worktree, 'node_modules')).isSymbolicLink()).toBe(true);
      const removed = await runAction(root, { kind: 'worktreeRemove', path: worktree, deleteBranch: false }, {}, engine);
      expect(removed.lines.findIndex((line) => line.startsWith('unlink '))).toBeGreaterThanOrEqual(0);
      expect(removed.lines.findIndex((line) => line.startsWith('unlink '))).toBeLessThan(removed.lines.findIndex((line) => line.startsWith('git worktree remove')));
      expect(fs.existsSync(worktree)).toBe(false);
      expect(fs.readFileSync(path.join(root, 'node_modules', 'left-pad', 'index.js'), 'utf8')).toBe('pad\n');
    },
  );
});

describe.skipIf(!built)('Submodules, through the engine', { timeout: 60_000 }, () => {
  it('adds a submodule and checks out its own submodules', async () => {
    const source = await library();
    const root = await made(async () => {});
    const engine = await runAction(root, { kind: 'submoduleAdd' }, { url: text(source), folder: text('lib'), branch: text('') });
    expect(engine.lines).toEqual([commandLine(['submodule', 'add', '--', source, 'lib'], root), 'git submodule update --init --recursive -- lib']);
    expect(fs.readFileSync(path.join(root, 'lib', 'lib.txt'), 'utf8')).toBe('one\n');
  });

  it('updates a submodule to the recorded commit, initializes one that is not, and to the last commit of its branch', async () => {
    const source = await library();
    const moved = async (root: string) => {
      await withSubmodule(source)(root);
      await commitFile(source, 'lib.txt', 'two\n', 'Library again');
      await git(path.join(root, 'lib'), ['pull', '--quiet', 'origin', 'main']);
    };
    const root = await made(moved);
    const recorded = (await git(root, ['ls-tree', 'HEAD', 'lib'])).stdout.split(/\s+/)[2];
    const at = async () => (await git(path.join(root, 'lib'), ['rev-parse', 'HEAD'])).stdout.trim();
    const engineRecorded = await runAction(root, { kind: 'submoduleUpdate', path: 'lib' });
    expect(engineRecorded.lines).toEqual(['git submodule update --init --recursive -- lib']);
    expect(await at()).toBe(recorded);

    const engineLatest = await runAction(root, { kind: 'submoduleUpdate', path: 'lib', remote: true });
    expect(engineLatest.lines).toEqual(['git submodule update --init --remote -- lib', 'git -C lib submodule update --init --recursive']);
    expect(await at()).toBe((await git(source, ['rev-parse', 'HEAD'])).stdout.trim());

    await git(root, ['submodule', 'deinit', '--quiet', '--force', '--', 'lib']);
    const engineInitialized = await runAction(root, { kind: 'submoduleUpdate', path: 'lib' });
    expect(engineInitialized.finished.map((each) => each.kind === 'ran' && each.plan.title)).toEqual([expect.stringMatching(/^Initialize/)]);
    expect(engineInitialized.lines).toEqual(['git submodule update --init --recursive -- lib']);
    expect(await at()).toBe(recorded);
  });

  it('stages the commit a submodule is on as the one recorded', async () => {
    const source = await library();
    const root = await made(async (setup) => {
      await withSubmodule(source)(setup);
      await commitFile(path.join(setup, 'lib'), 'lib.txt', 'local\n', 'Library here');
    });
    const engine = await runAction(root, { kind: 'submoduleRecord', path: 'lib' });
    expect(engine.lines).toEqual(['git add -- lib']);
    const on = (await git(path.join(root, 'lib'), ['rev-parse', 'HEAD'])).stdout.trim();
    expect((await git(root, ['ls-files', '--stage', '--', 'lib'])).stdout).toContain(on);
  });

  it('deinitializes a submodule', async () => {
    const source = await library();
    const root = await made(withSubmodule(source));
    const before = await snapshot(root);
    const engine = await runAction(root, { kind: 'submoduleDeinit', path: 'lib' });
    expect(engine.lines).toEqual(['git submodule deinit -- lib']);
    expect(fs.readdirSync(path.join(root, 'lib'))).toEqual([]);
    expect((await snapshot(root)).index).toEqual(before.index);
  });

  it('removes a submodule, and deletes its clone in the git directory after (S6)', async () => {
    const source = await library();
    const root = await made(withSubmodule(source));
    const engine = await runAction(root, { kind: 'submoduleRemove', path: 'lib' });
    expect(engine.lines).toEqual(['git submodule deinit --force -- lib', 'git rm --force -- lib']);
    expect(fs.existsSync(path.join(root, '.git', 'modules', 'lib'))).toBe(false);
    expect(fs.existsSync(path.join(root, 'lib'))).toBe(false);
  });

  it('synchronizes the URLs of the submodules', async () => {
    const source = await library();
    const root = await made(async (setup) => {
      await withSubmodule(source)(setup);
      await git(setup, ['config', '-f', '.gitmodules', 'submodule.lib.url', `${source}/`]);
      await git(setup, ['commit', '--quiet', '-am', 'Move lib']);
    });
    const engine = await runAction(root, { kind: 'submoduleSync' });
    expect(engine.lines).toEqual(['git submodule sync --recursive']);
    expect((await git(root, ['config', 'submodule.lib.url'])).stdout.trim()).toBe(`${source}/`);
  });
});

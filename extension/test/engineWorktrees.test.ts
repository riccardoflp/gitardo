/**
 * The scenarios of the fifth area moved to Gitardo's engine in stage 4 of `docs/transition/06-roadmap.md`: worktrees,
 * what new worktrees get, and submodules. Each operation runs as the `GitOp` the extension ran until now in one copy of
 * a repository and as the engine's feature, with the target and answers `engineRuns` gives it, in the other; the two
 * repositories after must be the same, and so must the commands, but where `docs/transition/04-differences.md` says why
 * not. A worktree's folder is beside the repository, so the paths of the two copies are compared from the folder that
 * holds each.
 */
import * as fs from 'node:fs';
import * as path from 'node:path';
import { afterAll, beforeAll, describe, expect, it } from 'vitest';
import type { EngineClient } from '../src/engine/client';
import type { EngineError } from '../src/engine/connection';
import { engineRuns } from '../src/engine/actions';
import type { Answer, Finished, Plan, Question, Step, Target } from '../src/engine/messages';
import { engineSettings } from '../src/engine/settings';
import { git } from '../src/git/exec';
import {
  type GitOp,
  submoduleAddOp,
  submoduleDeinitOp,
  submoduleRecordOp,
  submoduleRemoveOp,
  submoduleSyncOp,
  submoduleUpdateOp,
  worktreeAddOp,
  worktreeDir,
  worktreeMoveOp,
  worktreePruneOp,
  worktreeRemoveOp,
  worktreeTrackOp,
} from '../src/git/ops';
import { Repo } from '../src/git/repo';
import { setUpWorktree } from '../src/git/worktreeSetup';
import type { Action, RepoState } from '../src/shared/protocol';
import { built, cleanUpAfterEach, client, commitFile, folder, memoryLog, repository, snapshot } from './engine/harness';

cleanUpAfterEach();

// Git clones submodules from folders on disk only when told it may; both paths are told so here, and the engine's
// retry with the file protocol allowed has tests of its own.
const allowed = process.env.GIT_ALLOW_PROTOCOL;
beforeAll(() => {
  process.env.GIT_ALLOW_PROTOCOL = 'file';
});
afterAll(() => {
  if (allowed === undefined) delete process.env.GIT_ALLOW_PROTOCOL;
  else process.env.GIT_ALLOW_PROTOCOL = allowed;
});

/** What the extension reads of a repository, for the `GitOp`s and for `engineRuns`. */
async function state(root: string): Promise<RepoState> {
  return new Repo((await Repo.discover(root))!).state(10);
}

/** The folder a new worktree named `name` gets in the folder of the worktrees beside `root`, as the extension writes it. */
const worktreeOf = (root: string, name: string) => worktreeDir(root, '', name);

/**
 * Two copies of a repository made by `setup`, with the worktrees it made beside it: the folder that holds the
 * repository is copied whole, and git is told where the worktrees of the copy are.
 */
async function twins(setup: (root: string) => Promise<void>): Promise<[string, string]> {
  const root = await repository();
  await setup(root);
  const copy = path.join(folder(), 'repo');
  fs.cpSync(path.dirname(root), path.dirname(copy), { recursive: true, verbatimSymlinks: true });
  const listed = (await git(root, ['worktree', 'list', '--porcelain'])).stdout
    .split('\n')
    .filter((line) => line.startsWith('worktree '))
    .map((line) => path.join(path.dirname(copy), path.relative(path.dirname(root), line.slice('worktree '.length))))
    .filter((worktree) => path.resolve(worktree) !== path.resolve(copy));
  const present = listed.filter((worktree) => fs.existsSync(worktree));
  if (present.length) await git(copy, ['worktree', 'repair', ...present]);
  return [root, copy];
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

/** What a scenario compares: the repository, its worktrees, and what is in the folder of the worktrees. */
async function everything(root: string): Promise<Record<string, unknown>> {
  const worktrees = path.join(path.dirname(root), 'repo.worktrees');
  const folders = fs.existsSync(worktrees) ? fs.readdirSync(worktrees, { recursive: true, encoding: 'utf8' }).filter((entry) => !entry.split(/[\\/]/).includes('.git')) : null;
  return { ...(await snapshot(root)), worktrees: await worktreesOf(root), folders: folders?.map((entry) => entry.split(path.sep).join('/')).sort() ?? null };
}

/** Runs a `GitOp` as `Controller.run` does, but for its hooks, and gives its commands from the folder that holds `root`. */
async function runOp(root: string, op: GitOp): Promise<string[]> {
  for (const args of op.commands) await git(root, args);
  return op.commands.map((args) => commandLine(args, root));
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

/** The submodule `lib` as the `GitOp`s of submodules take it. */
async function lib(root: string) {
  const submodule = (await state(root)).submodules.find((each) => each.path === 'lib');
  if (!submodule) throw new Error('lib is not a submodule');
  return submodule;
}

describe.skipIf(!built)('Worktrees, through the engine and as GitOps', { timeout: 60_000 }, () => {
  it('creates a worktree on a new branch from the branch checked out', async () => {
    const [one, other] = await twins(async () => {});
    const typescript = await runOp(one, worktreeAddOp(worktreeOf(one, 'feature'), 'main', 'feature', false));
    const engine = await runAction(other, { kind: 'worktreeAdd' }, { name: text('feature') });
    expect(engine.lines).toEqual(typescript);
    expect(typescript).toEqual(['git worktree add --no-track -b feature <parent>/repo.worktrees/feature main']);
    expect(await everything(other)).toEqual(await everything(one));
  });

  it('creates a worktree for a branch, named after it', async () => {
    const [one, other] = await twins(topic);
    const typescript = await runOp(one, worktreeAddOp(worktreeOf(one, 'topic'), 'topic', null, false));
    const engine = await runAction(other, { kind: 'worktreeAdd', ref: 'topic', isBranch: true });
    expect(engine.lines).toEqual(typescript);
    expect(await everything(other)).toEqual(await everything(one));
  });

  it('creates a worktree at a commit, on a new branch', async () => {
    const [one, other] = await twins(topic);
    const commit = (await git(one, ['rev-parse', 'topic~1'])).stdout.trim();
    const typescript = await runOp(one, worktreeAddOp(worktreeOf(one, 'spike'), commit, 'spike', true));
    const engine = await runAction(other, { kind: 'worktreeAdd', ref: commit, isBranch: false }, { name: text('spike') });
    expect(engine.lines).toEqual(typescript);
    expect(await everything(other)).toEqual(await everything(one));
  });

  it('removes a worktree and its branch, and deletes the folders it leaves empty before the branch (W8)', async () => {
    const [one, other] = await twins(async (root) => {
      await git(root, ['worktree', 'add', '--quiet', '-b', 'team/feature', worktreeOf(root, 'team/feature'), 'main']);
    });
    const dir = (root: string) => worktreeOf(root, 'team/feature');
    const typescript = await runOp(one, worktreeRemoveOp(dir(one), false, 'team/feature'));
    // The `GitOp`'s hook deletes the folders inside the folder of the worktrees that are left empty.
    fs.rmdirSync(path.dirname(dir(one)));
    const engine = await runAction(other, { kind: 'worktreeRemove', path: dir(other), deleteBranch: true });
    expect(engine.lines).toEqual([typescript[0], 'delete <parent>/repo.worktrees/team', typescript[1]]);
    expect(await everything(other)).toEqual(await everything(one));
  });

  it('renames a worktree and its branch', async () => {
    const [one, other] = await twins(withWorktree);
    const typescript = await runOp(one, worktreeMoveOp(worktreeOf(one, 'feature'), worktreeOf(one, 'better'), { from: 'feature', to: 'better' }));
    const engine = await runAction(
      other,
      { kind: 'worktreeRename', path: worktreeOf(other, 'feature') },
      { 'new name': text('better'), 'rename branch': choice('rename'), 'branch name': text('better') },
    );
    expect(engine.lines).toEqual(typescript);
    expect(await everything(other)).toEqual(await everything(one));
  });

  it('prunes the worktrees whose folders are gone', async () => {
    const [one, other] = await twins(async (root) => {
      await withWorktree(root);
      fs.rmSync(worktreeOf(root, 'feature'), { recursive: true, force: true });
    });
    const typescript = await runOp(one, worktreePruneOp());
    const engine = await runAction(other, { kind: 'worktreePrune' });
    expect(engine.lines).toEqual(typescript);
    expect(await everything(other)).toEqual(await everything(one));
  });

  it('opens a remote branch in a new worktree, on a local branch that tracks it', async () => {
    const [one, other] = await twins(async (root) => {
      await git(root, ['branch', 'fix']);
      await git(root, ['remote', 'add', 'origin', root]);
      await git(root, ['fetch', '--quiet', 'origin']);
      await git(root, ['branch', '-D', 'fix']);
    });
    const typescript = await runOp(one, worktreeTrackOp(worktreeOf(one, 'fix'), 'fix', 'origin/fix'));
    const engine = await runAction(other, { kind: 'openInWorktree', name: 'origin/fix', remote: true });
    expect(engine.lines).toEqual(typescript);
    expect(await everything(other)).toEqual(await everything(one));
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
    const [one, other] = await twins(ignored);
    const patterns = { link: ['node_modules'], copy: ['.env'] };
    const failures = await setUpWorktree(one, worktreeOf(one, 'feature'), patterns, { info: () => {}, warn: () => {}, error: () => {} } as never);
    expect(failures).toEqual([]);
    const settings = engineSettings({ 'worktrees.linkFolders': patterns.link, 'worktrees.copyFiles': patterns.copy }, new Set(), {}, 0);
    await runAction(other, { kind: 'worktreeSetUp', path: worktreeOf(other, 'feature') }, {}, client(memoryLog(), settings));
    const inWorktree = (root: string) => {
      const worktree = worktreeOf(root, 'feature');
      return {
        linked: fs.lstatSync(path.join(worktree, 'node_modules')).isSymbolicLink(),
        package: fs.readFileSync(path.join(worktree, 'node_modules', 'left-pad', 'index.js'), 'utf8'),
        env: fs.readFileSync(path.join(worktree, '.env'), 'utf8'),
      };
    };
    expect(inWorktree(other)).toEqual(inWorktree(one));
    expect(inWorktree(other)).toEqual({ linked: true, package: 'pad\n', env: 'KEY=value\n' });
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

describe.skipIf(!built)('Submodules, through the engine and as GitOps', { timeout: 60_000 }, () => {
  it('adds a submodule and checks out its own submodules', async () => {
    const source = await library();
    const [one, other] = await twins(async () => {});
    const typescript = await runOp(one, submoduleAddOp(source, 'lib', ''));
    const engine = await runAction(other, { kind: 'submoduleAdd' }, { url: text(source), folder: text('lib'), branch: text('') });
    expect(engine.lines).toEqual(typescript);
    expect(await snapshot(other)).toEqual(await snapshot(one));
  });

  it('updates a submodule to the recorded commit, initializes one that is not, and to the last commit of its branch', async () => {
    const source = await library();
    const moved = async (root: string) => {
      await withSubmodule(source)(root);
      await commitFile(source, 'lib.txt', 'two\n', 'Library again');
      await git(path.join(root, 'lib'), ['pull', '--quiet', 'origin', 'main']);
    };
    const [one, other] = await twins(moved);
    const recorded = await runOp(one, submoduleUpdateOp(await lib(one), false));
    const engineRecorded = await runAction(other, { kind: 'submoduleUpdate', path: 'lib' });
    expect(engineRecorded.lines).toEqual(recorded);
    expect(await snapshot(other)).toEqual(await snapshot(one));

    const latest = await runOp(one, submoduleUpdateOp(await lib(one), true));
    const engineLatest = await runAction(other, { kind: 'submoduleUpdate', path: 'lib', remote: true });
    expect(engineLatest.lines).toEqual(latest);
    expect(await snapshot(other)).toEqual(await snapshot(one));

    for (const root of [one, other]) await git(root, ['submodule', 'deinit', '--quiet', '--force', '--', 'lib']);
    const initialized = await runOp(one, submoduleUpdateOp(await lib(one), false));
    const engineInitialized = await runAction(other, { kind: 'submoduleUpdate', path: 'lib' });
    expect(engineInitialized.finished.map((each) => each.kind === 'ran' && each.plan.title)).toEqual([expect.stringMatching(/^Initialize/)]);
    expect(engineInitialized.lines).toEqual(initialized);
    expect(await snapshot(other)).toEqual(await snapshot(one));
  });

  it('stages the commit a submodule is on as the one recorded', async () => {
    const source = await library();
    const [one, other] = await twins(async (root) => {
      await withSubmodule(source)(root);
      await commitFile(path.join(root, 'lib'), 'lib.txt', 'local\n', 'Library here');
    });
    const typescript = await runOp(one, submoduleRecordOp(await lib(one), (await lib(one)).head!));
    const engine = await runAction(other, { kind: 'submoduleRecord', path: 'lib' });
    expect(engine.lines).toEqual(typescript);
    expect(await snapshot(other)).toEqual(await snapshot(one));
  });

  it('deinitializes a submodule', async () => {
    const source = await library();
    const [one, other] = await twins(withSubmodule(source));
    const typescript = await runOp(one, submoduleDeinitOp(await lib(one)));
    const engine = await runAction(other, { kind: 'submoduleDeinit', path: 'lib' });
    expect(engine.lines).toEqual(typescript);
    expect(await snapshot(other)).toEqual(await snapshot(one));
  });

  it('removes a submodule, and deletes its clone in the git directory after (S6)', async () => {
    const source = await library();
    const [one, other] = await twins(withSubmodule(source));
    const clone = (root: string) => path.join(root, '.git', 'modules', 'lib');
    const typescript = await runOp(one, submoduleRemoveOp(await lib(one), clone(one)));
    // The `GitOp`'s hook deletes the clone once git removed the submodule.
    fs.rmSync(clone(one), { recursive: true, force: true });
    const engine = await runAction(other, { kind: 'submoduleRemove', path: 'lib' });
    expect(engine.lines).toEqual(typescript);
    expect(fs.existsSync(clone(other))).toBe(false);
    expect(await snapshot(other)).toEqual(await snapshot(one));
  });

  it('synchronizes the URLs of the submodules', async () => {
    const source = await library();
    const [one, other] = await twins(async (root) => {
      await withSubmodule(source)(root);
      await git(root, ['config', '-f', '.gitmodules', 'submodule.lib.url', `${source}/`]);
      await git(root, ['commit', '--quiet', '-am', 'Move lib']);
    });
    const typescript = await runOp(one, submoduleSyncOp());
    const engine = await runAction(other, { kind: 'submoduleSync' });
    expect(engine.lines).toEqual(typescript);
    const url = async (root: string) => (await git(root, ['config', 'submodule.lib.url'])).stdout.trim();
    expect(await url(other)).toEqual(await url(one));
    expect(await snapshot(other)).toEqual(await snapshot(one));
  });
});

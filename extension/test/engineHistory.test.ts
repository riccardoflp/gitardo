/**
 * The scenarios of the third area moved to Gitardo's engine in stage 4 of `docs/transition/06-roadmap.md`: merge,
 * rebase, cherry-pick, revert and reset; reword, squash, fixup, split and the rebase editor; bisect; patches. Each
 * operation runs as the `GitOp` the extension ran until now in one copy of a repository and as the engine's feature,
 * with the target and answers `engineRuns` gives it, in the other; the two repositories after must be the same, and so
 * must the commands where neither writes files of its own, but where `docs/transition/04-differences.md` says why not.
 * Undo of what the engine runs as one operation of several commits is taken back whole by its journal.
 */
import * as fs from 'node:fs';
import * as path from 'node:path';
import { describe, expect, it } from 'vitest';
import { type EngineRun, engineRuns, todoRows } from '../src/engine/actions';
import type { Answer, FollowUp, Question } from '../src/engine/messages';
import { git } from '../src/git/exec';
import {
  applyPatchOp,
  bisectOp,
  bisectResetOp,
  cherryPickManyOp,
  cherryPickOp,
  fixupOp,
  type GitOp,
  mergeOp,
  rebaseInteractiveOp,
  rebaseOp,
  rebaseTodo,
  resetOp,
  revertOp,
  rewordHeadOp,
  splitHeadOp,
  splitRebaseOp,
  squashMessage,
} from '../src/git/ops';
import { Repo } from '../src/git/repo';
import type { Action, RebaseStep, RepoState } from '../src/shared/protocol';
import { answering, built, choice, cleanUpAfterEach, client, commitFile, folder, snapshot, stepLine, text, twins } from './engine/harness';

cleanUpAfterEach();

/** What the extension reads of a repository, for the `GitOp`s and for `engineRuns`. */
async function state(root: string): Promise<RepoState> {
  return new Repo((await Repo.discover(root))!).state(50);
}

const head = async (root: string, revision = 'HEAD') => (await git(root, ['rev-parse', revision])).stdout.trim();
const subjects = async (root: string, count = 5) => (await git(root, ['log', '--format=%s', '-n', String(count)])).stdout.trim().split('\n');

/**
 * Runs a `GitOp` as `Controller.run` does, with what it prepares and finishes itself; a command that fails, as one that
 * stops on conflicts, ends it there. Gives its commands as the engine would write their lines.
 */
async function runOp(root: string, op: GitOp): Promise<string[]> {
  const log = { info: () => undefined, warn: () => undefined };
  await op.prepare?.(log);
  try {
    for (const [index, args] of op.commands.entries()) await git(root, args, { input: index === op.commands.length - 1 ? op.input : undefined });
    await op.finish?.(log);
  } catch {
    // Stopped, as git leaves it.
  } finally {
    op.cleanup?.();
  }
  return op.commands.map(stepLine);
}

/**
 * Runs the runs of an action through the engine as the controller does, the answers it gives before asking given with
 * `feature/run`, and the questions answered by key; gives how the last ended, the lines of the steps and the follow-ups.
 */
async function runEngine(
  root: string,
  runs: EngineRun[],
  answers: Record<string, (question: Question) => Answer | null> = {},
): Promise<{ kind: string; lines: string[]; followUps: FollowUp[]; message?: string }> {
  const engine = engineOf(root);
  let ended: { kind: string; lines: string[]; followUps: FollowUp[]; message?: string } = { kind: 'none', lines: [], followUps: [] };
  for (const run of runs) {
    try {
      const { finished, followUps } = await engine.runFeature(
        { repository: root, feature: run.feature, target: run.target, answers: run.answers },
        answering(answers),
      );
      const lines = finished.kind === 'ran' ? finished.plan.steps.map((step) => step.line) : [];
      ended = { kind: finished.kind, lines, followUps, message: finished.kind === 'refused' ? finished.message : undefined };
    } catch (failure) {
      ended = { kind: 'failed', lines: [], followUps: [], message: failure instanceof Error ? failure.message : String(failure) };
    }
    if (ended.kind !== 'ran') break;
  }
  return ended;
}

/** One engine for each repository of a test, as one window has one: its journal holds from one run to the next. */
const engines = new Map<string, ReturnType<typeof client>>();
function engineOf(root: string): ReturnType<typeof client> {
  const known = engines.get(root);
  if (known) return known;
  const started = client();
  engines.set(root, started);
  return started;
}

/** The runs of an action, which must be the engine's. */
async function runsOf(root: string, action: Action): Promise<EngineRun[]> {
  const runs = engineRuns(action, await state(root));
  if (!runs) throw new Error(`${action.kind} does not run in the engine`);
  return runs;
}

/**
 * Runs a scenario: the `GitOp` and the action `build` makes of a copy, the commits named by their hashes there, which
 * are the same in both copies; the `GitOp` in one copy, the action through the engine in the other. Gives the commands
 * of each, and checks that the two repositories after are the same.
 */
async function scenario(
  setup: (root: string) => Promise<void>,
  build: (root: string, state: RepoState) => Promise<{ op: GitOp; action: Action }>,
  answers: Record<string, (question: Question) => Answer | null> = {},
): Promise<{ typescript: string[]; engine: string[]; followUps: FollowUp[]; roots: [string, string] }> {
  const [one, other] = await twins(setup);
  const { op, action } = await build(one, await state(one));
  const typescript = await runOp(one, op);
  const ran = await runEngine(other, await runsOf(other, action), answers);
  expect(ran.kind, JSON.stringify(ran)).toBe('ran');
  expect(await snapshot(other)).toEqual(await snapshot(one));
  return { typescript, engine: ran.lines, followUps: ran.followUps, roots: [one, other] };
}

/** A scenario whose `GitOp` and action name no commit. */
const plain = (op: GitOp, action: Action) => async () => ({ op, action });

/** Undo through the engine, as **Undo** runs it. */
async function undo(root: string): Promise<string> {
  const ran = await runEngine(root, [{ feature: 'undo.last', target: { kind: 'head' } }]);
  expect(ran.kind, JSON.stringify(ran)).toBe('ran');
  return ran.lines.join('\n');
}

/** `feature` with a commit of its own, and `main` with another, each changing a file of its own. */
async function diverged(root: string): Promise<void> {
  await git(root, ['checkout', '--quiet', '-b', 'feature']);
  await commitFile(root, 'feature.txt', 'feature\n', 'Add feature');
  await git(root, ['checkout', '--quiet', 'main']);
  await commitFile(root, 'main.txt', 'main\n', 'Add main');
}

/** `feature` one commit ahead of `main`. */
async function ahead(root: string): Promise<void> {
  await git(root, ['checkout', '--quiet', '-b', 'feature']);
  await commitFile(root, 'feature.txt', 'feature\n', 'Add feature');
  await git(root, ['checkout', '--quiet', 'main']);
}

/** `feature` and `main` changing the same line of `a.txt`. */
async function clashing(root: string): Promise<void> {
  await git(root, ['checkout', '--quiet', '-b', 'feature']);
  await commitFile(root, 'a.txt', 'feature\n', 'Change a on feature');
  await git(root, ['checkout', '--quiet', 'main']);
  await commitFile(root, 'a.txt', 'main\n', 'Change a on main');
}

/** Three commits after the first, with messages that have bodies, and a change not committed. */
async function line(root: string): Promise<void> {
  await commitFile(root, 'one.txt', 'one\n', 'Add one\n\nIts body.');
  await commitFile(root, 'two.txt', 'two\n', 'Add two\n\nKept under the new subject.');
  await commitFile(root, 'three.txt', 'three\n', 'Add three');
}

/** The `GitOp` of an interactive rebase as `interactiveRebase` of `src/extension/actions.ts` builds it, its files in a folder of its own. */
async function interactiveOp(root: string, steps: RebaseStep[], base: string | null, autostash: boolean): Promise<GitOp> {
  const dir = folder();
  const file = (name: string) => path.join(dir, name).replace(/\\/g, '/');
  const repo = new Repo((await Repo.discover(root))!);
  for (const [index, step] of steps.entries()) {
    if (step.action !== 'reword') continue;
    let message = (step.message ?? '').trim();
    if (step.keepBody) {
      const original = await repo.commitMessage(step.hash);
      const end = original.search(/\n\s*\n/);
      if (end !== -1) message = `${message}\n\n${original.slice(end).trim()}`;
    }
    fs.writeFileSync(file(`message-${index}.txt`), `${message}\n`);
  }
  const todo = rebaseTodo(steps, (index) => file(`message-${index}.txt`));
  fs.writeFileSync(file('todo'), todo);
  return rebaseInteractiveOp(base, file('todo'), todo, 'main', autostash);
}

describe.skipIf(!built)('Merge, rebase, cherry-pick, revert and reset, through the engine and as GitOps', { timeout: 60_000 }, () => {
  it('merges as git says, and in each of the ways asked', async () => {
    const merged = await scenario(diverged, plain(mergeOp('feature', 'main'), { kind: 'merge', ref: 'feature' }));
    expect(merged.engine).toEqual(['git merge feature']);
    expect(merged.engine).toEqual(merged.typescript);
    const noFastForward = await scenario(ahead, plain(mergeOp('feature', 'main', 'noFastForward'), { kind: 'merge', ref: 'feature', mode: 'noFastForward' }));
    expect(noFastForward.engine).toEqual(noFastForward.typescript);
    const fastForward = await scenario(ahead, plain(mergeOp('feature', 'main', 'fastForwardOnly'), { kind: 'merge', ref: 'feature', mode: 'fastForwardOnly' }));
    expect(fastForward.engine).toEqual(fastForward.typescript);
    const asked = await scenario(diverged, plain(mergeOp('feature', 'main', 'noFastForward'), { kind: 'merge', ref: 'feature', mode: 'ask' }), {
      way: choice('no-ff'),
    });
    expect(asked.engine).toEqual(['git merge --no-ff feature']);
  });

  it('refuses a fast-forward only that cannot be one before git runs', async () => {
    const [root] = await twins(diverged);
    const before = await head(root);
    const ran = await runEngine(root, await runsOf(root, { kind: 'merge', ref: 'feature', mode: 'fastForwardOnly' }));
    expect(ran).toMatchObject({ kind: 'refused', message: expect.stringMatching(/^main cannot be fast-forwarded to feature/) });
    expect(await head(root)).toBe(before);
  });

  it('squash merges with the changes staged and the message git prepared, and Undo takes it back whole, also once it stopped on conflicts', async () => {
    const squash = await scenario(diverged, plain(mergeOp('feature', 'main', 'squash'), { kind: 'merge', ref: 'feature', mode: 'squash' }));
    expect(squash.engine).toEqual(squash.typescript);
    expect(squash.followUps.map((followUp) => followUp.kind)).toEqual(['say', 'preparedMessage']);
    const [, root] = squash.roots;
    // Staging a file moves no HEAD: the squash merge is still what Undo takes back.
    fs.writeFileSync(path.join(root, 'other.txt'), 'other\n');
    await git(root, ['add', 'other.txt']);
    expect(await undo(root)).toBe('git reset --merge');
    expect((await git(root, ['status', '--porcelain'])).stdout.trim()).toBe('');

    const [stopped] = await twins(clashing);
    const before = await head(stopped);
    const ran = await runEngine(stopped, await runsOf(stopped, { kind: 'merge', ref: 'feature', mode: 'squash' }));
    expect(ran.kind).toBe('failed');
    expect((await git(stopped, ['diff', '--name-only', '--diff-filter=U'])).stdout.trim()).toBe('a.txt');
    expect(await undo(stopped)).toBe('git reset --merge');
    expect(await head(stopped)).toBe(before);
    expect((await git(stopped, ['status', '--porcelain'])).stdout.trim()).toBe('');
  });

  it('rebases what is checked out onto a branch, and a branch not checked out onto a commit', async () => {
    const onto = await scenario(diverged, plain(rebaseOp('feature', 'main'), { kind: 'rebase', onto: 'feature' }));
    expect(onto.engine).toEqual(onto.typescript);
    const other = async (root: string) => {
      await diverged(root);
      await git(root, ['checkout', '--quiet', '-b', 'side', 'HEAD~1']);
      await commitFile(root, 'side.txt', 'side\n', 'Add side');
      await git(root, ['checkout', '--quiet', 'main']);
    };
    const branch = await scenario(other, async (root) => {
      const tip = await head(root, 'main');
      return { op: rebaseOp(tip, 'main', 'side'), action: { kind: 'rebase', onto: tip, branch: 'side' } };
    });
    expect(branch.engine).toEqual(branch.typescript);
  });

  it('refuses to rebase a branch another worktree has checked out, before git runs (C4)', async () => {
    const [root] = await twins(diverged);
    await git(root, ['worktree', 'add', '--quiet', path.join(folder(), 'feature'), 'feature']);
    const ran = await runEngine(root, await runsOf(root, { kind: 'rebase', onto: await head(root), branch: 'feature' }));
    expect(ran).toMatchObject({ kind: 'refused', message: 'feature is checked out in another worktree: rebase it there.' });
  });

  it('cherry-picks a commit, and refuses while a merge is stopped (C2)', async () => {
    const one = await scenario(diverged, async (root) => {
      const picked = await head(root, 'feature');
      return { op: cherryPickOp(picked, false), action: { kind: 'cherryPick', hash: picked, isMerge: false } };
    });
    expect(one.engine).toEqual(one.typescript);

    const [stopped] = await twins(clashing);
    await git(stopped, ['merge', 'feature']).catch(() => undefined);
    const ran = await runEngine(stopped, await runsOf(stopped, { kind: 'cherryPick', hash: await head(stopped, 'feature'), isMerge: false }));
    expect(ran).toMatchObject({ kind: 'refused', message: 'Finish or abort the merge first.' });
  });

  it('cherry-picks several commits oldest first whatever order they were selected in, and Undo takes back the series whole', async () => {
    const two = async (root: string) => {
      await git(root, ['checkout', '--quiet', '-b', 'picks']);
      await commitFile(root, 'pick-one.txt', 'one\n', 'Pick one');
      await commitFile(root, 'pick-two.txt', 'two\n', 'Pick two');
      await git(root, ['checkout', '--quiet', 'main']);
      // The graph shows them: they are among the rows of the state.
      await git(root, ['merge', '--quiet', '--no-ff', '-m', 'Show picks', 'picks']);
      await git(root, ['reset', '--quiet', '--hard', 'HEAD~1']);
    };
    let picked: string[] = [];
    const series = await scenario(two, async (root) => {
      picked = [await head(root, 'picks~1'), await head(root, 'picks')];
      return { op: cherryPickManyOp(picked), action: { kind: 'cherryPickMany', hashes: picked } };
    });
    expect(series.engine).toEqual([`git cherry-pick ${picked[0]} ${picked[1]}`]);
    expect(series.engine).toEqual(series.typescript);
    const [, root] = series.roots;
    const before = await head(root, 'HEAD~2');
    expect(await undo(root)).toBe(`git reset --keep ${before}`);
    expect(await head(root)).toBe(before);
  });

  it('carries a series stopped on a conflict through Continue, and Undo still takes it back whole', async () => {
    const [root] = await twins(async (made) => {
      await git(made, ['checkout', '--quiet', '-b', 'picks']);
      await commitFile(made, 'a.txt', 'theirs\n', 'Pick clash');
      await commitFile(made, 'after.txt', 'after\n', 'Pick after');
      await git(made, ['checkout', '--quiet', 'main']);
      await commitFile(made, 'a.txt', 'ours\n', 'Clash on main');
    });
    const before = await head(root);
    const hashes = [await head(root, 'picks'), await head(root, 'picks~1')];
    expect((await runEngine(root, await runsOf(root, { kind: 'cherryPickMany', hashes }))).kind).toBe('failed');
    fs.writeFileSync(path.join(root, 'a.txt'), 'both\n');
    await git(root, ['add', 'a.txt']);
    const continued = await runEngine(root, [{ feature: 'conflicts.continue-operation', target: { kind: 'head' } }]);
    expect(continued.kind, JSON.stringify(continued)).toBe('ran');
    expect(await subjects(root, 3)).toEqual(['Pick after', 'Pick clash', 'Clash on main']);
    expect(await undo(root)).toBe(`git reset --keep ${before}`);
    expect(await head(root)).toBe(before);
  });

  it('reverts a commit, and fails as git does on a commit whose changes are taken back already, leaving nothing to skip (C1)', async () => {
    const reverted = await scenario(line, async (root) => {
      const last = await head(root);
      return { op: revertOp(last, false), action: { kind: 'revert', hash: last, isMerge: false } };
    });
    expect(reverted.engine).toEqual(reverted.typescript);

    const [one, other] = await twins(async (root) => {
      await line(root);
      await git(root, ['revert', '--no-edit', 'HEAD']);
    });
    const hash = await head(one, 'HEAD~1');
    await runOp(one, revertOp(hash, false));
    const ran = await runEngine(other, await runsOf(other, { kind: 'revert', hash, isMerge: false }));
    expect(ran).toMatchObject({ kind: 'failed', message: expect.stringContaining('nothing to commit') });
    expect(fs.existsSync(path.join(other, '.git', 'REVERT_HEAD'))).toBe(false);
    expect(await snapshot(other)).toEqual(await snapshot(one));
  });

  it('resets soft, mixed and hard, as asked', async () => {
    for (const mode of ['soft', 'mixed', 'hard'] as const) {
      const reset = await scenario(
        line,
        async (root) => {
          const to = await head(root, 'HEAD~2');
          return { op: resetOp(to, mode, 'main'), action: { kind: 'reset', hash: to } };
        },
        { mode: choice(mode) },
      );
      expect(reset.engine).toEqual(reset.typescript);
    }
  });
});

describe.skipIf(!built)('Rewriting, through the engine and as GitOps', { timeout: 60_000 }, () => {
  it('gives the last commit a new message, and an older one by a rebase whose files the engine deletes once it ended', async () => {
    const amended = await scenario(line, async (root) => ({
      op: rewordHeadOp('Add three, new'),
      action: { kind: 'reword', hash: await head(root), message: 'Add three, new' },
    }));
    expect(amended.engine).toEqual(amended.typescript);
    expect(amended.followUps).toEqual([{ kind: 'selectHead' }]);

    const older = await scenario(line, async (root) => {
      const [one, two, three, base] = await Promise.all(['HEAD~2', 'HEAD~1', 'HEAD', 'HEAD~3'].map((revision) => head(root, revision)));
      const message = 'Add one, new\n\nIts body.';
      const steps: RebaseStep[] = [
        { action: 'reword', hash: one, message },
        { action: 'pick', hash: two },
        { action: 'pick', hash: three },
      ];
      return { op: await interactiveOp(root, steps, base, true), action: { kind: 'reword', hash: one, message } };
    });
    expect(older.followUps).toEqual([{ kind: 'selectBeforeHead', count: 2 }]);
    expect(older.engine.at(-1)).toMatch(/rebase -i --autostash [0-9a-f]{40}$/);
    expect(fs.existsSync(path.join(older.roots[1], '.git', 'gitardo-rebase'))).toBe(false);
  });

  it('runs the rows of the rebase editor, a new subject keeping the body', async () => {
    const edited = await scenario(line, async (root) => {
      const [one, two, three, base] = await Promise.all(['HEAD~2', 'HEAD~1', 'HEAD', 'HEAD~3'].map((revision) => head(root, revision)));
      const steps: RebaseStep[] = [
        { action: 'pick', hash: one },
        { action: 'reword', hash: two, message: 'Add two, new', keepBody: true },
        { action: 'fixup', hash: three },
      ];
      expect(todoRows(steps)).toBe(`pick ${one}\nreword ${two} Add two, new\nfixup ${three}\n`);
      return { op: await interactiveOp(root, steps, base, false), action: { kind: 'rebaseInteractive', base, steps } };
    });
    const [, root] = edited.roots;
    expect((await git(root, ['log', '-1', '--format=%B'])).stdout.trim()).toBe('Add two, new\n\nKept under the new subject.');
    expect(fs.existsSync(path.join(root, '.git', 'gitardo-rebase'))).toBe(false);
  });

  it('refuses the rebase editor while a bisect is under way (C8)', async () => {
    const [root] = await twins(line);
    await git(root, ['bisect', 'start']);
    await git(root, ['bisect', 'bad']);
    const ran = await runEngine(root, await runsOf(root, { kind: 'rebaseInteractive', base: null, steps: [{ action: 'pick', hash: await head(root) }] }));
    expect(ran).toMatchObject({ kind: 'refused', message: 'End the bisect first.' });
  });

  it('squashes commits into one under the subject typed, with the messages of them all', async () => {
    const message = squashMessage('Add one and two', ['Add one\n\nIts body.', 'Add two\n\nKept under the new subject.']);
    const squashed = await scenario(
      line,
      async (root) => {
        const [one, two, three, base] = await Promise.all(['HEAD~2', 'HEAD~1', 'HEAD', 'HEAD~3'].map((revision) => head(root, revision)));
        const steps: RebaseStep[] = [
          { action: 'reword', hash: one, message },
          { action: 'fixup', hash: two },
          { action: 'pick', hash: three },
        ];
        return { op: await interactiveOp(root, steps, base, true), action: { kind: 'squash', hashes: [two, one] } };
      },
      { subject: text('Add one and two') },
    );
    expect(squashed.followUps).toEqual([{ kind: 'selectBeforeHead', count: 1 }]);
    expect((await git(squashed.roots[1], ['log', '-1', '--format=%B', 'HEAD~1'])).stdout.trim()).toBe(message);
  });

  it('adds the changes to an earlier commit', async () => {
    const changed = async (root: string) => {
      await line(root);
      fs.writeFileSync(path.join(root, 'one.txt'), 'one, fixed\n');
    };
    const fixed = await scenario(changed, async (root) => {
      const one = await head(root, 'HEAD~2');
      return { op: fixupOp(one, false, true), action: { kind: 'fixup', hash: one, isRoot: false } };
    });
    expect(fixed.engine).toEqual(fixed.typescript);
  });

  it('splits the last commit, which Undo takes back whole, and an older one, which stops marked as a split with its message in the box', async () => {
    let last = '';
    const split = await scenario(line, async (root) => {
      last = await head(root);
      return { op: splitHeadOp(last), action: { kind: 'split', hash: last } };
    });
    expect(split.engine).toEqual(split.typescript);
    expect(split.followUps).toEqual([
      { kind: 'fillCommitMessage', message: 'Add three' },
      { kind: 'say', text: 'The changes of "Add three" are in the working tree, unstaged: commit them in as many commits as you like.' },
    ]);
    const [, root] = split.roots;
    expect(await undo(root)).toBe(`git reset --mixed ${last}`);
    expect(await head(root)).toBe(last);
    expect((await git(root, ['status', '--porcelain'])).stdout.trim()).toBe('');

    let two = '';
    const older = await scenario(line, async (made) => {
      const [one, three] = [await head(made, 'HEAD~2'), await head(made)];
      two = await head(made, 'HEAD~1');
      const steps: RebaseStep[] = [
        { action: 'edit', hash: two },
        { action: 'pick', hash: three },
      ];
      return { op: splitRebaseOp(await interactiveOp(made, steps, one, true), two, path.join(made, '.git')), action: { kind: 'split', hash: two } };
    });
    expect(older.followUps).toEqual([{ kind: 'fillCommitMessage', message: 'Add two\n\nKept under the new subject.' }]);
    for (const made of older.roots) expect(fs.readFileSync(path.join(made, '.git', 'rebase-merge', 'gitardo-split'), 'utf8').trim()).toBe(two);
  });
});

describe.skipIf(!built)('Bisect, through the engine and as GitOps', { timeout: 60_000 }, () => {
  it('starts on the first mark, marks a commit of the graph, answers for the one tested, refuses once it is found (C9), and ends', async () => {
    let good = '';
    const started = await scenario(line, async (root) => {
      const bad = await head(root);
      good = await head(root, 'HEAD~3');
      return { op: bisectOp('bad', bad, false), action: { kind: 'bisect', mark: 'bad', hash: bad } };
    });
    expect(started.engine).toEqual(started.typescript);
    const [one, other] = started.roots;
    await runOp(one, bisectOp('good', good, true));
    const marked = await runEngine(other, await runsOf(other, { kind: 'bisect', mark: 'good', hash: good }));
    expect(marked.lines).toEqual([`git bisect good ${good}`]);
    expect(await head(other)).toBe(await head(one));
    for (let guard = 0; guard < 5 && (await state(other)).bisect?.remaining !== 1; guard++) {
      await runOp(one, bisectOp('bad', undefined, true));
      const answered = await runEngine(other, await runsOf(other, { kind: 'bisect', mark: 'bad' }));
      expect(answered.lines, JSON.stringify([answered, (await state(other)).bisect])).toEqual(['git bisect bad']);
      expect(await head(other)).toBe(await head(one));
    }
    const found = await runEngine(other, await runsOf(other, { kind: 'bisect', mark: 'good' }));
    expect(found).toMatchObject({ kind: 'refused', message: 'The bisect found the first bad commit already.' });
    await runOp(one, bisectResetOp());
    expect((await runEngine(other, await runsOf(other, { kind: 'bisectReset' }))).lines).toEqual(['git bisect reset']);
    expect(await snapshot(other)).toEqual(await snapshot(one));
    const again = await runEngine(other, await runsOf(other, { kind: 'bisectReset' }));
    expect(again).toMatchObject({ kind: 'refused', message: 'No bisect is in progress.' });
  });
});

describe.skipIf(!built)('Patches, through the engine and as GitOps', { timeout: 60_000 }, () => {
  /** Two commits on `patched` made into a patch, `main` without them. */
  const patched = async (root: string) => {
    await git(root, ['checkout', '--quiet', '-b', 'patched']);
    await commitFile(root, 'patched.txt', 'one\n', 'Patch one');
    await commitFile(root, 'patched.txt', 'one\ntwo\n', 'Patch two');
    await git(root, ['checkout', '--quiet', 'main']);
  };
  const patchOf = async (root: string) => (await git(root, ['format-patch', '--stdout', 'main..patched'])).stdout;

  it('copies commits as one patch oldest first, and saves it to the file chosen', async () => {
    const [root] = await twins(patched);
    const hashes = [await head(root, 'patched~1'), await head(root, 'patched')];
    const copied = await runEngine(root, [{ feature: 'patch.copy', target: { kind: 'commits', commits: hashes } }]);
    expect(copied.followUps).toEqual([
      { kind: 'copyToClipboard', text: await patchOf(root) },
      { kind: 'say', text: 'Copied 2 commits as a patch.' },
    ]);
    const file = path.join(folder(), 'picks.patch');
    const saved = await runEngine(root, [{ feature: 'patch.save', target: { kind: 'commits', commits: hashes } }], {
      path: () => ({ kind: 'path', path: file }),
    });
    expect(saved.kind).toBe('ran');
    expect(fs.readFileSync(file, 'utf8')).toBe(await patchOf(root));
  });

  it('applies a patch from a file as commits, and from the clipboard the controller read, which Undo takes back whole', async () => {
    const file = path.join(folder(), 'two.patch');
    const fromFile = await scenario(
      patched,
      async (root) => {
        const text = await patchOf(root);
        fs.writeFileSync(file, text);
        return { op: applyPatchOp({ name: 'two.patch', file, text }, true, 'main'), action: { kind: 'applyPatch' } };
      },
      { from: choice('file'), file: () => ({ kind: 'path', path: file }), as: choice('commits') },
    );
    expect(fromFile.engine).toEqual(fromFile.typescript);

    const [one, other] = await twins(patched);
    const text = await patchOf(other);
    const before = await head(other);
    await runOp(one, applyPatchOp({ name: 'the patch in the clipboard', text }, true, 'main'));
    const runs = await runsOf(other, { kind: 'applyPatch' });
    runs[0].answers = { clipboard: { kind: 'text', text } };
    const ran = await runEngine(other, runs, { from: choice('clipboard'), as: choice('commits') });
    expect(ran.kind, JSON.stringify(ran)).toBe('ran');
    expect(ran.lines[0]).toMatch(/^write .*gitardo-[\w-]+\.patch"?$/);
    expect(await snapshot(other)).toEqual(await snapshot(one));
    expect(await undo(other)).toBe(`git reset --keep ${before}`);
    expect(await head(other)).toBe(before);
  });

  it('applies a patch to the working tree only', async () => {
    const file = path.join(folder(), 'two.patch');
    const applied = await scenario(
      patched,
      async (root) => {
        const text = await patchOf(root);
        fs.writeFileSync(file, text);
        return { op: applyPatchOp({ name: 'two.patch', file, text }, false, 'main'), action: { kind: 'applyPatch' } };
      },
      { from: choice('file'), file: () => ({ kind: 'path', path: file }), as: choice('working-tree') },
    );
    expect(applied.engine).toEqual(applied.typescript);
  });
});

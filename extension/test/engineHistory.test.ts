/**
 * The scenarios of the third area moved to Gitardo's engine in stage 4 of `docs/transition/06-roadmap.md`: merge,
 * rebase, cherry-pick, revert and reset; reword, squash, fixup, split and the rebase editor; bisect; patches. Each
 * operation runs through the engine's feature, with the target and answers `engineRuns` gives it, and must run the
 * commands it ran when it was a `GitOp` of the extension where it writes no files of its own, but where
 * `docs/transition/04-differences.md` says why not. Undo of what the engine runs as one operation of several commits is
 * taken back whole by its journal.
 */
import * as fs from 'node:fs';
import * as path from 'node:path';
import { describe, expect, it } from 'vitest';
import { type EngineRun, engineRuns, todoRows } from '../src/engine/actions';
import type { Answer, FollowUp, Question } from '../src/engine/messages';
import type { Action, RebaseStep, RepoState } from '../src/shared/protocol';
import { git } from './git';
import { answering, built, choice, cleanUpAfterEach, client, commitFile, engineState, folder, repository, snapshot, stepLine, text } from './engine/harness';

cleanUpAfterEach();

/** What the extension reads of a repository through the engine, for `engineRuns`. */
async function state(root: string): Promise<RepoState> {
  return engineState(root, 50);
}

/** A repository made by `setup`. */
async function made(setup: (root: string) => Promise<void>): Promise<string> {
  const root = await repository();
  await setup(root);
  return root;
}

const head = async (root: string, revision = 'HEAD') => (await git(root, ['rev-parse', revision])).stdout.trim();
const subjects = async (root: string, count = 5) => (await git(root, ['log', '--format=%s', '-n', String(count)])).stdout.trim().split('\n');

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
 * Runs a scenario: the action `build` makes of a repository `setup` made, the commits named by their hashes there,
 * through the engine. Gives the commands, the follow-ups and the repository.
 */
async function scenario(
  setup: (root: string) => Promise<void>,
  build: (root: string, state: RepoState) => Promise<Action>,
  answers: Record<string, (question: Question) => Answer | null> = {},
): Promise<{ engine: string[]; followUps: FollowUp[]; root: string }> {
  const root = await made(setup);
  const action = await build(root, await state(root));
  const ran = await runEngine(root, await runsOf(root, action), answers);
  expect(ran.kind, JSON.stringify(ran)).toBe('ran');
  return { engine: ran.lines, followUps: ran.followUps, root };
}

/** A scenario whose action names no commit. */
const plain = (action: Action) => async () => action;

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

describe.skipIf(!built)('Merge, rebase, cherry-pick, revert and reset, through the engine', { timeout: 60_000 }, () => {
  it('merges as git says, and in each of the ways asked', async () => {
    const merged = await scenario(diverged, plain({ kind: 'merge', ref: 'feature' }));
    expect(merged.engine).toEqual(['git merge feature']);
    const noFastForward = await scenario(ahead, plain({ kind: 'merge', ref: 'feature', mode: 'noFastForward' }));
    expect(noFastForward.engine).toEqual(['git merge --no-ff feature']);
    const fastForward = await scenario(ahead, plain({ kind: 'merge', ref: 'feature', mode: 'fastForwardOnly' }));
    expect(fastForward.engine).toEqual(['git merge --ff-only feature']);
    const asked = await scenario(diverged, plain({ kind: 'merge', ref: 'feature', mode: 'ask' }), { way: choice('no-ff') });
    expect(asked.engine).toEqual(['git merge --no-ff feature']);
  });

  it('refuses a fast-forward only that cannot be one before git runs', async () => {
    const root = await made(diverged);
    const before = await head(root);
    const ran = await runEngine(root, await runsOf(root, { kind: 'merge', ref: 'feature', mode: 'fastForwardOnly' }));
    expect(ran).toMatchObject({ kind: 'refused', message: expect.stringMatching(/^main cannot be fast-forwarded to feature/) });
    expect(await head(root)).toBe(before);
  });

  it('squash merges with the changes staged and the message git prepared, and Undo takes it back whole, also once it stopped on conflicts', async () => {
    const squash = await scenario(diverged, plain({ kind: 'merge', ref: 'feature', mode: 'squash' }));
    expect(squash.engine).toEqual(['git merge --squash feature']);
    expect(squash.followUps.map((followUp) => followUp.kind)).toEqual(['say', 'preparedMessage']);
    const { root } = squash;
    // Staging a file moves no HEAD: the squash merge is still what Undo takes back.
    fs.writeFileSync(path.join(root, 'other.txt'), 'other\n');
    await git(root, ['add', 'other.txt']);
    expect(await undo(root)).toBe('git reset --merge');
    expect((await git(root, ['status', '--porcelain'])).stdout.trim()).toBe('');

    const stopped = await made(clashing);
    const before = await head(stopped);
    const ran = await runEngine(stopped, await runsOf(stopped, { kind: 'merge', ref: 'feature', mode: 'squash' }));
    expect(ran.kind).toBe('failed');
    expect((await git(stopped, ['diff', '--name-only', '--diff-filter=U'])).stdout.trim()).toBe('a.txt');
    expect(await undo(stopped)).toBe('git reset --merge');
    expect(await head(stopped)).toBe(before);
    expect((await git(stopped, ['status', '--porcelain'])).stdout.trim()).toBe('');
  });

  it('rebases what is checked out onto a branch, and a branch not checked out onto a commit', async () => {
    const onto = await scenario(diverged, plain({ kind: 'rebase', onto: 'feature' }));
    expect(onto.engine).toEqual(['git rebase feature']);
    const other = async (root: string) => {
      await diverged(root);
      await git(root, ['checkout', '--quiet', '-b', 'side', 'HEAD~1']);
      await commitFile(root, 'side.txt', 'side\n', 'Add side');
      await git(root, ['checkout', '--quiet', 'main']);
    };
    let tip = '';
    const branch = await scenario(other, async (root) => {
      tip = await head(root, 'main');
      return { kind: 'rebase', onto: tip, branch: 'side' };
    });
    expect(branch.engine).toEqual([`git rebase ${tip} side`]);
  });

  it('refuses to rebase a branch another worktree has checked out, before git runs (C4)', async () => {
    const root = await made(diverged);
    await git(root, ['worktree', 'add', '--quiet', path.join(folder(), 'feature'), 'feature']);
    const ran = await runEngine(root, await runsOf(root, { kind: 'rebase', onto: await head(root), branch: 'feature' }));
    expect(ran).toMatchObject({ kind: 'refused', message: 'feature is checked out in another worktree: rebase it there.' });
  });

  it('cherry-picks a commit, and refuses while a merge is stopped (C2)', async () => {
    let picked = '';
    const one = await scenario(diverged, async (root) => {
      picked = await head(root, 'feature');
      return { kind: 'cherryPick', hash: picked, isMerge: false };
    });
    expect(one.engine).toEqual([`git cherry-pick ${picked}`]);

    const stopped = await made(clashing);
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
      return { kind: 'cherryPickMany', hashes: picked };
    });
    expect(series.engine).toEqual([`git cherry-pick ${picked[0]} ${picked[1]}`]);
    const { root } = series;
    const before = await head(root, 'HEAD~2');
    expect(await undo(root)).toBe(`git reset --keep ${before}`);
    expect(await head(root)).toBe(before);
  });

  it('carries a series stopped on a conflict through Continue, and Undo still takes it back whole', async () => {
    const root = await made(async (setup) => {
      await git(setup, ['checkout', '--quiet', '-b', 'picks']);
      await commitFile(setup, 'a.txt', 'theirs\n', 'Pick clash');
      await commitFile(setup, 'after.txt', 'after\n', 'Pick after');
      await git(setup, ['checkout', '--quiet', 'main']);
      await commitFile(setup, 'a.txt', 'ours\n', 'Clash on main');
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
    let last = '';
    const reverted = await scenario(line, async (root) => {
      last = await head(root);
      return { kind: 'revert', hash: last, isMerge: false };
    });
    expect(reverted.engine).toEqual([`git revert --no-edit ${last}`]);

    const other = await made(async (root) => {
      await line(root);
      await git(root, ['revert', '--no-edit', 'HEAD']);
    });
    const hash = await head(other, 'HEAD~1');
    const before = await snapshot(other);
    const ran = await runEngine(other, await runsOf(other, { kind: 'revert', hash, isMerge: false }));
    expect(ran).toMatchObject({ kind: 'failed', message: expect.stringContaining('nothing to commit') });
    expect(fs.existsSync(path.join(other, '.git', 'REVERT_HEAD'))).toBe(false);
    expect(await snapshot(other)).toEqual(before);
  });

  it('resets soft, mixed and hard, as asked', async () => {
    for (const mode of ['soft', 'mixed', 'hard'] as const) {
      let to = '';
      const reset = await scenario(
        line,
        async (root) => {
          to = await head(root, 'HEAD~2');
          return { kind: 'reset', hash: to };
        },
        { mode: choice(mode) },
      );
      expect(reset.engine).toEqual([`git reset --${mode} ${to}`]);
    }
  });
});

describe.skipIf(!built)('Rewriting, through the engine', { timeout: 60_000 }, () => {
  it('gives the last commit a new message, and an older one by a rebase whose files the engine deletes once it ended', async () => {
    const amended = await scenario(line, async (root) => ({ kind: 'reword', hash: await head(root), message: 'Add three, new' }));
    expect(amended.engine).toEqual(['git commit --amend --only --allow-empty -F -']);
    expect(amended.followUps).toEqual([{ kind: 'selectHead' }]);

    const older = await scenario(line, async (root) => ({ kind: 'reword', hash: await head(root, 'HEAD~2'), message: 'Add one, new\n\nIts body.' }));
    expect(older.followUps).toEqual([{ kind: 'selectBeforeHead', count: 2 }]);
    expect(older.engine.at(-1)).toMatch(/rebase -i --autostash [0-9a-f]{40}$/);
    expect(await subjects(older.root, 3)).toEqual(['Add three', 'Add two', 'Add one, new']);
    expect(fs.existsSync(path.join(older.root, '.git', 'gitardo-rebase'))).toBe(false);
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
      return { kind: 'rebaseInteractive', base, steps };
    });
    const { root } = edited;
    expect((await git(root, ['log', '-1', '--format=%B'])).stdout.trim()).toBe('Add two, new\n\nKept under the new subject.');
    expect(fs.existsSync(path.join(root, '.git', 'gitardo-rebase'))).toBe(false);
  });

  it('refuses the rebase editor while a bisect is under way (C8)', async () => {
    const root = await made(line);
    await git(root, ['bisect', 'start']);
    await git(root, ['bisect', 'bad']);
    const ran = await runEngine(root, await runsOf(root, { kind: 'rebaseInteractive', base: null, steps: [{ action: 'pick', hash: await head(root) }] }));
    expect(ran).toMatchObject({ kind: 'refused', message: 'End the bisect first.' });
  });

  it('squashes commits into one under the subject typed, with the messages of them all', async () => {
    const message = 'Add one and two\n\nAdd one\n\nIts body.\n\nAdd two\n\nKept under the new subject.';
    const squashed = await scenario(
      line,
      async (root) => {
        const [one, two] = await Promise.all(['HEAD~2', 'HEAD~1'].map((revision) => head(root, revision)));
        return { kind: 'squash', hashes: [two, one] };
      },
      { subject: text('Add one and two') },
    );
    expect(squashed.followUps).toEqual([{ kind: 'selectBeforeHead', count: 1 }]);
    expect((await git(squashed.root, ['log', '-1', '--format=%B', 'HEAD~1'])).stdout.trim()).toBe(message);
    expect(await subjects(squashed.root, 3)).toEqual(['Add three', 'Add one and two', 'First commit']);
  });

  it('adds the changes to an earlier commit', async () => {
    const changed = async (root: string) => {
      await line(root);
      fs.writeFileSync(path.join(root, 'one.txt'), 'one, fixed\n');
    };
    let one = '';
    const fixed = await scenario(changed, async (root) => {
      one = await head(root, 'HEAD~2');
      return { kind: 'fixup', hash: one, isRoot: false };
    });
    expect(fixed.engine).toEqual(['git add --all', `git commit --fixup=${one}`, `git -c sequence.editor=true rebase -i --autosquash --autostash ${one}^`]);
    expect((await git(fixed.root, ['show', 'HEAD~2:one.txt'])).stdout).toBe('one, fixed\n');
  });

  it('splits the last commit, which Undo takes back whole, and an older one, which stops marked as a split with its message in the box', async () => {
    let last = '';
    const split = await scenario(line, async (root) => {
      last = await head(root);
      return { kind: 'split', hash: last };
    });
    expect(split.engine).toEqual(['git reset --mixed HEAD~']);
    expect(split.followUps).toEqual([
      { kind: 'fillCommitMessage', message: 'Add three' },
      { kind: 'say', text: 'The changes of "Add three" are in the working tree, unstaged: commit them in as many commits as you like.' },
    ]);
    const { root } = split;
    expect(await undo(root)).toBe(`git reset --mixed ${last}`);
    expect(await head(root)).toBe(last);
    expect((await git(root, ['status', '--porcelain'])).stdout.trim()).toBe('');

    let two = '';
    const older = await scenario(line, async (root) => {
      two = await head(root, 'HEAD~1');
      return { kind: 'split', hash: two };
    });
    expect(older.followUps).toEqual([{ kind: 'fillCommitMessage', message: 'Add two\n\nKept under the new subject.' }]);
    expect(fs.readFileSync(path.join(older.root, '.git', 'rebase-merge', 'gitardo-split'), 'utf8').trim()).toBe(two);
  });
});

describe.skipIf(!built)('Bisect, through the engine', { timeout: 60_000 }, () => {
  it('starts on the first mark, marks a commit of the graph, answers for the one tested, refuses once it is found (C9), and ends', async () => {
    let good = '';
    let bad = '';
    const started = await scenario(line, async (root) => {
      bad = await head(root);
      good = await head(root, 'HEAD~3');
      return { kind: 'bisect', mark: 'bad', hash: bad };
    });
    expect(started.engine).toEqual(['git bisect start', `git bisect bad ${bad}`]);
    const other = started.root;
    const before = await snapshot(other);
    const marked = await runEngine(other, await runsOf(other, { kind: 'bisect', mark: 'good', hash: good }));
    expect(marked.lines).toEqual([`git bisect good ${good}`]);
    for (let guard = 0; guard < 5 && (await state(other)).bisect?.remaining !== 1; guard++) {
      const answered = await runEngine(other, await runsOf(other, { kind: 'bisect', mark: 'bad' }));
      expect(answered.lines, JSON.stringify([answered, (await state(other)).bisect])).toEqual(['git bisect bad']);
    }
    const found = await runEngine(other, await runsOf(other, { kind: 'bisect', mark: 'good' }));
    expect(found).toMatchObject({ kind: 'refused', message: 'The bisect found the first bad commit already.' });
    expect((await runEngine(other, await runsOf(other, { kind: 'bisectReset' }))).lines).toEqual(['git bisect reset']);
    expect(await snapshot(other)).toEqual(before);
    const again = await runEngine(other, await runsOf(other, { kind: 'bisectReset' }));
    expect(again).toMatchObject({ kind: 'refused', message: 'No bisect is in progress.' });
  });
});

describe.skipIf(!built)('Patches, through the engine', { timeout: 60_000 }, () => {
  /** Two commits on `patched` made into a patch, `main` without them. */
  const patched = async (root: string) => {
    await git(root, ['checkout', '--quiet', '-b', 'patched']);
    await commitFile(root, 'patched.txt', 'one\n', 'Patch one');
    await commitFile(root, 'patched.txt', 'one\ntwo\n', 'Patch two');
    await git(root, ['checkout', '--quiet', 'main']);
  };
  const patchOf = async (root: string) => (await git(root, ['format-patch', '--stdout', 'main..patched'])).stdout;

  it('copies commits as one patch oldest first, and saves it to the file chosen', async () => {
    const root = await made(patched);
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
        fs.writeFileSync(file, await patchOf(root));
        return { kind: 'applyPatch' };
      },
      { from: choice('file'), file: () => ({ kind: 'path', path: file }), as: choice('commits') },
    );
    expect(fromFile.engine).toEqual([stepLine(['am', '--3way', file])]);
    expect(await subjects(fromFile.root, 3)).toEqual(['Patch two', 'Patch one', 'First commit']);

    const other = await made(patched);
    const text = await patchOf(other);
    const before = await head(other);
    const runs = await runsOf(other, { kind: 'applyPatch' });
    runs[0].answers = { clipboard: { kind: 'text', text } };
    const ran = await runEngine(other, runs, { from: choice('clipboard'), as: choice('commits') });
    expect(ran.kind, JSON.stringify(ran)).toBe('ran');
    expect(ran.lines[0]).toMatch(/^write .*gitardo-[\w-]+\.patch"?$/);
    expect(await subjects(other, 3)).toEqual(['Patch two', 'Patch one', 'First commit']);
    expect(await undo(other)).toBe(`git reset --keep ${before}`);
    expect(await head(other)).toBe(before);
  });

  it('applies a patch to the working tree only', async () => {
    const file = path.join(folder(), 'two.patch');
    const applied = await scenario(
      patched,
      async (root) => {
        fs.writeFileSync(file, await patchOf(root));
        return { kind: 'applyPatch' };
      },
      { from: choice('file'), file: () => ({ kind: 'path', path: file }), as: choice('working-tree') },
    );
    expect(applied.engine).toEqual([stepLine(['apply', '--3way', file])]);
    expect(fs.readFileSync(path.join(applied.root, 'patched.txt'), 'utf8')).toBe('one\ntwo\n');
  });
});

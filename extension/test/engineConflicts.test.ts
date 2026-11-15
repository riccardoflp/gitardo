/**
 * The scenarios of the fourth area moved to Gitardo's engine in stage 4 of `docs/transition/06-roadmap.md`: the
 * conflicts. Each operation runs as the `GitOp` the extension ran until now in one copy of a repository stopped on
 * conflicts and as the engine's feature, with the target and answers `engineRuns` or the review gives it, in the other;
 * the two repositories after must be the same, and so must the commands of git, but where
 * `docs/transition/04-differences.md` says why not.
 */
import * as fs from 'node:fs';
import * as path from 'node:path';
import { describe, expect, it } from 'vitest';
import { engineRuns } from '../src/engine/actions';
import type { Answer, Finished, Plan, Question, Target } from '../src/engine/messages';
import { engineSettings } from '../src/engine/settings';
import { engineResolutions } from '../src/engine/resolutions';
import { git } from '../src/git/exec';
import { abortOp, type AcceptedFile, continueOp, type GitOp, resolveConflictsOp, skipOp, stageOp, takeSideOp, writeResolutionsOp } from '../src/git/ops';
import { Repo } from '../src/git/repo';
import type { Action, RepoState } from '../src/shared/protocol';
import { answering, built, choice, cleanUpAfterEach, client, commitFile, memoryLog, runGitOp, snapshot, twins } from './engine/harness';

cleanUpAfterEach();

/** Twelve lines, some of them changed. */
const lines = (changes: Record<number, string> = {}) => Array.from({ length: 12 }, (_, index) => `${changes[index] ?? `line ${index}`}\n`).join('');

/**
 * A merge of `aside` stopped on `e.txt`, which has two conflicts: one the rules resolve (lines next to each other) and
 * one that needs a choice; and on `d.txt`, which has one the rules resolve.
 */
async function merging(root: string): Promise<void> {
  await commitFile(root, 'e.txt', lines(), 'Add e');
  await commitFile(root, 'd.txt', 'one\ntwo\n', 'Add d');
  await git(root, ['checkout', '--quiet', '-b', 'aside']);
  fs.writeFileSync(path.join(root, 'd.txt'), 'one\nTWO\n');
  await git(root, ['add', '--', 'd.txt']);
  await commitFile(root, 'e.txt', lines({ 2: 'two, theirs', 9: 'nine, theirs' }), 'Aside, theirs');
  await git(root, ['checkout', '--quiet', 'main']);
  fs.writeFileSync(path.join(root, 'd.txt'), 'ONE\ntwo\n');
  await git(root, ['add', '--', 'd.txt']);
  await commitFile(root, 'e.txt', lines({ 3: 'three, ours', 9: 'nine, ours' }), 'Aside, ours');
  await git(root, ['merge', 'aside']).catch(() => undefined);
}

/** The merge with every conflict resolved and staged, waiting to be committed. */
async function resolved(root: string): Promise<void> {
  await merging(root);
  fs.writeFileSync(path.join(root, 'e.txt'), lines({ 2: 'two, theirs', 3: 'three, ours', 9: 'nine, both' }));
  fs.writeFileSync(path.join(root, 'd.txt'), 'ONE\nTWO\n');
  await git(root, ['add', '--', 'e.txt', 'd.txt']);
}

/** A cherry-pick of `aside`'s commit stopped on `e.txt`. */
async function picking(root: string): Promise<void> {
  await commitFile(root, 'e.txt', lines(), 'Add e');
  await git(root, ['checkout', '--quiet', '-b', 'aside']);
  await commitFile(root, 'e.txt', lines({ 9: 'nine, theirs' }), 'Aside, theirs');
  await git(root, ['checkout', '--quiet', 'main']);
  await commitFile(root, 'e.txt', lines({ 9: 'nine, ours' }), 'Aside, ours');
  await git(root, ['cherry-pick', 'aside']).catch(() => undefined);
}

/** What the extension reads of a repository, for the `GitOp`s that need it. */
async function state(root: string): Promise<RepoState> {
  return new Repo((await Repo.discover(root))!).state(10);
}

/** The settings of the scenarios: resolving conflicts automatically is on, as the extension needs it for its command. */
const ON = engineSettings({ 'conflicts.autoResolve': true }, new Set(), {}, 0);

/** Runs a feature through the engine with resolving automatically on, its questions answered by key. */
async function runFeature(
  root: string,
  feature: string,
  target: Target,
  answers: Record<string, (question: Question) => Answer | null> = {},
): Promise<{ finished: Finished }> {
  return client(memoryLog(), ON).runFeature({ repository: root, feature, target }, answering(answers));
}

/** The git commands among the lines of the steps of the engine: its warnings and the files it writes are not commands. */
const commands = (lines: string[]) => lines.filter((line) => line.startsWith('git '));

/**
 * Runs `op` as a `GitOp` in one copy and the engine's runs in the other, their questions answered by key; gives the
 * commands of each and how the last run ended, and checks that the two repositories after are the same.
 */
async function scenario(
  setup: (root: string) => Promise<void>,
  op: (state: RepoState, repo: Repo) => Promise<GitOp>,
  runs: (state: RepoState, repo: Repo) => Promise<{ feature: string; target: Target; answers?: Record<string, Answer> }[] | null>,
  answers: Record<string, (question: Question) => Answer | null> = {},
): Promise<{ typescript: string[]; engine: string[]; finished: Finished }> {
  const [one, other] = await twins(setup);
  const oneRepo = new Repo((await Repo.discover(one))!);
  const typescript = await runGitOp(one, await op(await state(one), oneRepo));
  const otherRepo = new Repo((await Repo.discover(other))!);
  const planned = await runs(await state(other), otherRepo);
  if (!planned) throw new Error('This does not run in the engine');
  const engine: string[] = [];
  let finished: Finished = { kind: 'backedOut' };
  for (const run of planned) {
    const given = Object.fromEntries(Object.entries(run.answers ?? {}).map(([key, answer]) => [key, () => answer]));
    ({ finished } = await runFeature(other, run.feature, run.target, { ...given, ...answers }));
    expect(finished.kind, JSON.stringify(finished)).toBe('ran');
    if (finished.kind === 'ran') engine.push(...finished.plan.steps.map((step) => step.line));
  }
  expect(await snapshot(other)).toEqual(await snapshot(one));
  return { typescript, engine, finished };
}

/** The runs `engineRuns` gives an action, on the state read. */
const mapped = (action: Action) => async (read: RepoState) => engineRuns(action, read);

/** The danger of the plan a run ended with. */
const danger = (finished: Finished): Plan['danger'] | null => (finished.kind === 'ran' ? finished.plan.danger : null);

describe.skipIf(!built)('Conflicts, through the engine and as GitOps', { timeout: 60_000 }, () => {
  it('resolves automatically what needs no choice, and its question is its confirmation (D3)', async () => {
    const files = (read: RepoState) => read.workingTree.conflicted.map((file) => file.path);
    const ran = await scenario(merging, async (read, repo) => resolveConflictsOp(await repo.conflictPlans(files(read))), mapped({ kind: 'resolveConflicts' }), {
      resolve: choice('resolve'),
    });
    expect(commands(ran.engine)).toEqual(ran.typescript);
    expect(ran.typescript).toEqual(['git add -- d.txt']);
    expect(ran.engine.filter((line) => line.startsWith('write '))).toHaveLength(2);
  });

  it('asks nothing more than its own question with every plan shown (D3)', async () => {
    const [root] = await twins(merging);
    const shown = client(memoryLog(), { ...ON, commandPreview: 'all' });
    const asker = answering({ resolve: choice('resolve') });
    const { finished } = await shown.runFeature({ repository: root, feature: 'conflicts.resolve-automatically', target: { kind: 'head' } }, asker);
    expect(finished.kind).toBe('ran');
    expect(asker.asked.map((question) => question.key)).toEqual(['resolve']);
    expect(asker.confirmed).toEqual([]);
  });

  it('takes a whole file from one side, and warns when lines typed in it would be lost (D6)', async () => {
    for (const side of ['ours', 'theirs'] as const) {
      const ran = await scenario(
        merging,
        async (read, repo) => takeSideOp('e.txt', side, await repo.conflictStages('e.txt'), read.operation),
        mapped({ kind: 'takeSide', path: 'e.txt', side }),
      );
      expect(ran.engine).toEqual(ran.typescript);
      expect(danger(ran.finished)?.kind).toBe('confirm');
    }
    const typed = async (root: string) => {
      await merging(root);
      fs.appendFileSync(path.join(root, 'e.txt'), 'a line typed by hand\n');
    };
    const warned = await scenario(
      typed,
      async (read, repo) => takeSideOp('e.txt', 'theirs', await repo.conflictStages('e.txt'), read.operation),
      mapped({ kind: 'takeSide', path: 'e.txt', side: 'theirs' }),
    );
    expect(danger(warned.finished)?.kind).toBe('destructive');
  });

  it('marks a file resolved, from the Changes view and from the Flow Merge', async () => {
    const edited = async (root: string) => {
      await merging(root);
      fs.writeFileSync(path.join(root, 'd.txt'), 'ONE\nTWO\n');
    };
    const row = await scenario(edited, async () => stageOp(['d.txt']), mapped({ kind: 'stage', paths: ['d.txt'] }));
    expect(row.engine).toEqual(['git add -- d.txt']);
    expect(row.engine).toEqual(row.typescript);
    const page = await scenario(
      edited,
      async () => stageOp(['d.txt']),
      async () => [{ feature: 'merge-view.mark-resolved', target: { kind: 'mergeResult', path: 'd.txt', unsaved: null, marked: 0 } }],
    );
    expect(page.engine).toEqual(page.typescript);
  });

  it('writes the resolutions accepted in the review, the rules and the AI alike, and stages the file left with none', async () => {
    const accepted = async (repo: Repo): Promise<AcceptedFile[]> => {
      const [d, e] = await repo.conflictPlans(['d.txt', 'e.txt']);
      const conflict = (plan: typeof d, index: number) => plan.conflicts[index];
      return [
        {
          path: d.path,
          text: d.text,
          conflicts: d.conflicts.length,
          proposals: [
            {
              source: 'rules',
              from: conflict(d, 0).from,
              to: conflict(d, 0).to,
              text: 'ONE\nTWO\n',
              reason: 'Changes to lines next to each other',
              confidence: null,
              newLines: 0,
            },
          ],
        },
        {
          path: e.path,
          text: e.text,
          conflicts: e.conflicts.length,
          proposals: [
            {
              source: 'ai',
              from: conflict(e, 1).from,
              to: conflict(e, 1).to,
              text: 'nine, both\n',
              reason: 'Both renamed it.',
              confidence: 'high',
              newLines: 1,
              model: 'A model',
            },
          ],
        },
      ];
    };
    const ran = await scenario(
      merging,
      async (_read, repo) => writeResolutionsOp(await accepted(repo)),
      async (_read, repo) => [{ feature: 'review.write', target: { kind: 'resolutions', files: engineResolutions(await accepted(repo)) } }],
    );
    expect(commands(ran.engine)).toEqual(ran.typescript);
    expect(ran.typescript).toEqual(['git add -- d.txt']);
    // The log says what each file got and by whom (D5).
    expect(ran.engine).toContain('d.txt: 1 of 1 conflict (by the rules)');
    expect(ran.engine).toContain('e.txt: 1 of 2 conflicts (by the AI)');
  });

  it('continues once every conflict is resolved and staged, and refuses before then (D1)', async () => {
    const ran = await scenario(resolved, async (read) => continueOp(read.operation!), mapped({ kind: 'continue' }));
    expect(ran.engine).toEqual(ran.typescript);
    const [root] = await twins(merging);
    const { finished } = await runFeature(root, 'conflicts.continue-operation', { kind: 'head' });
    expect(finished).toEqual({ kind: 'refused', message: 'Resolve the conflicts and stage the files first.', offer: null });
  });

  it('skips the step a cherry-pick stopped on, asking first since what was resolved in it is thrown away (D2)', async () => {
    const ran = await scenario(picking, async (read) => skipOp(read.operation!), mapped({ kind: 'skip' }));
    expect(ran.engine).toEqual(ran.typescript);
    expect(danger(ran.finished)).toEqual({ kind: 'confirm', reason: 'What was resolved in this step is thrown away.' });
  });

  it('aborts the merge, with the warning', async () => {
    const ran = await scenario(merging, async (read) => abortOp(read.operation!), mapped({ kind: 'abort' }));
    expect(ran.engine).toEqual(ran.typescript);
    expect(danger(ran.finished)?.kind).toBe('destructive');
  });
});

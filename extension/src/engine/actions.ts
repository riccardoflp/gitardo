/**
 * Which feature of Gitardo's engine an action of the views runs, with its target and the answers the view already
 * gave, for the areas moved to the engine ([docs/transition/06-roadmap.md](../../../docs/transition/06-roadmap.md),
 * stage 4). An action that has none here still runs as a `GitOp` of `src/git/ops.ts`, and so does every action when the
 * engine is not there (D49).
 */

import type { Action, MergeMode, RebaseStep, RepoState } from '../shared/protocol';
import type { Answer, Target } from './messages';
import { targetOf } from './target';

/** A feature of the engine to run, on a target, with the answers given before it asks. */
export interface EngineRun {
  feature: string;
  target: Target;
  answers?: Record<string, Answer>;
}

/** The index of a stash from its name in the list, `stash@{2}`. */
export function stashIndex(ref: string): number | null {
  const found = /^stash@\{(\d+)\}$/.exec(ref);
  return found ? Number(found[1]) : null;
}

/** The target of files: the one file, or the files ticked. */
function files(paths: string[]): Target {
  return paths.length === 1 ? { kind: 'file', path: paths[0] } : { kind: 'files', paths };
}

/** The text of an answer. */
const text = (value: string): Answer => ({ kind: 'text', text: value });

/**
 * The runs of the engine that take the place of an action, in order, each ended before the next starts; none when the
 * action is not one the engine runs yet.
 */
export function engineRuns(
  action: Action,
  state: Pick<RepoState, 'branches' | 'remoteBranches' | 'tags'> & Partial<Pick<RepoState, 'bisect' | 'rows'>>,
): EngineRun[] | null {
  switch (action.kind) {
    // Branches.
    case 'createBranch':
      return [{ feature: 'branch.create', target: targetOf(action.at, state) }];

    // Undo.
    case 'undo':
      return [{ feature: 'undo.last', target: { kind: 'head' } }];

    // Changes and commit.
    case 'stage':
      return action.paths.length ? [{ feature: 'changes.stage', target: files(action.paths) }] : null;
    case 'unstage':
      return action.paths.length ? [{ feature: 'changes.unstage', target: files(action.paths) }] : null;
    case 'discard':
      return action.paths.length ? [{ feature: 'changes.discard', target: files(action.paths) }] : null;
    case 'stageAll':
      return [{ feature: 'changes.stage-all', target: { kind: 'changes' } }];
    case 'unstageAll':
      return [{ feature: 'changes.unstage-all', target: { kind: 'changes' } }];
    case 'discardAll':
      return [{ feature: 'changes.discard-all', target: { kind: 'changes' } }];
    case 'discardChanges':
      // From the row of the changes in the graph: the files ticked, or the whole working tree.
      return action.paths?.length
        ? [{ feature: 'changes.discard', target: files(action.paths) }]
        : [{ feature: 'changes.discard-all', target: { kind: 'head' } }];
    case 'commit':
      return [{ feature: action.amend ? 'commit.amend' : 'commit.create', target: { kind: 'head' }, answers: { message: text(action.message) } }];

    // Stashes.
    case 'stashPush':
      if (action.paths?.length) return [{ feature: 'stash.push-file', target: { kind: 'files', paths: action.paths } }];
      return [{ feature: action.staged ? 'stash.push-staged' : 'stash.push', target: { kind: 'head' } }];
    case 'stashApply':
    case 'stashPop':
    case 'stashDrop':
    case 'stashBranch': {
      const index = stashIndex(action.ref);
      if (index === null) return null;
      const feature = { stashApply: 'stash.apply', stashPop: 'stash.pop', stashDrop: 'stash.drop', stashBranch: 'stash.branch' }[action.kind];
      return [{ feature, target: { kind: 'stash', index } }];
    }

    // Merge, rebase, cherry-pick, revert, reset, rewriting, bisect and patches (area 3).
    case 'merge':
      if (!action.mode || action.mode === 'default') return [{ feature: 'merge.into-current', target: targetOf(action.ref, state) }];
      if (action.mode === 'ask') return [{ feature: 'merge.choose-way', target: targetOf(action.ref, state) }];
      return [{ feature: 'merge.choose-way', target: targetOf(action.ref, state), answers: { way: choice(MERGE_WAYS[action.mode]) } }];
    case 'rebase':
      // A branch dropped on a commit is rebased onto that commit; git checks it out first.
      if (action.branch) return [{ feature: 'rebase.branch-onto', target: targetOf(action.onto, state), answers: { branch: choice(action.branch) } }];
      return [{ feature: 'rebase.onto', target: targetOf(action.onto, state) }];
    case 'rebaseInteractive': {
      const todo = todoRows(action.steps);
      return todo === null ? null : [{ feature: 'rebase.interactive', target: { kind: 'head' }, answers: { todo: text(todo) } }];
    }
    case 'reword':
      return [{ feature: 'message.edit', target: { kind: 'commit', commit: action.hash }, answers: { message: text(action.message) } }];
    case 'squash':
      return [{ feature: 'squash.commits', target: { kind: 'commits', commits: action.hashes } }];
    case 'split':
      return [{ feature: 'split.commit', target: { kind: 'commit', commit: action.hash } }];
    case 'fixup':
      return [{ feature: 'fixup.commit', target: { kind: 'commit', commit: action.hash } }];
    case 'cherryPick':
      return [{ feature: 'cherry-pick.commit', target: { kind: 'commit', commit: action.hash } }];
    case 'cherryPickMany':
      return [{ feature: 'selection.cherry-pick', target: { kind: 'commits', commits: newestFirst(action.hashes, state.rows) } }];
    case 'revert':
      return [{ feature: 'revert.commit', target: { kind: 'commit', commit: action.hash } }];
    case 'reset':
      return [{ feature: 'reset.to-commit', target: { kind: 'commit', commit: action.hash } }];
    case 'applyPatch':
      // The text of the clipboard is given by the controller, which reads it: the engine cannot.
      return [{ feature: 'patch.apply', target: { kind: 'head' } }];
    case 'bisect':
      // The banner marks the commit being tested; a menu marks the commit it is on, starting a bisect when none is.
      if (!action.hash) return [{ feature: 'bisect.answer', target: { kind: 'head' }, answers: { answer: choice(action.mark) } }];
      if (action.mark === 'skip') return null;
      return [
        { feature: state.bisect ? 'bisect.mark' : 'bisect.start', target: { kind: 'commit', commit: action.hash }, answers: { mark: choice(action.mark) } },
      ];
    case 'bisectReset':
      return [{ feature: 'bisect.end', target: { kind: 'head' } }];

    default:
      return null;
  }
}

/** The choice of an answer. */
const choice = (identifier: string): Answer => ({ kind: 'choice', identifier });

/** The way of `merge.choose-way` for each way the views name. */
const MERGE_WAYS: Record<Exclude<MergeMode, 'default'>, string> = { noFastForward: 'no-ff', fastForwardOnly: 'ff-only', squash: 'squash' };

/**
 * The rows of the rebase editor as `rebase.interactive` reads them, oldest first: the action, the full hash, and for a
 * reword the new subject, under which the body of the commit stays. None when a reword gives a whole message, which
 * the engine's rows cannot say.
 */
export function todoRows(steps: RebaseStep[]): string | null {
  if (steps.some((step) => step.action === 'reword' && !step.keepBody)) return null;
  const row = (step: RebaseStep) =>
    step.action === 'reword' ? `reword ${step.hash} ${(step.message ?? '').replace(/[\r\n]+/g, ' ').trim()}` : `${step.action} ${step.hash}`;
  return `${steps.map(row).join('\n')}\n`;
}

/** Commits in the order of the graph, newest first, as `selection.cherry-pick` takes them, whatever order they were selected in. */
function newestFirst(hashes: string[], rows: RepoState['rows'] | undefined): string[] {
  const order = new Map((rows ?? []).map((row, index) => [row.hash, index]));
  if (!hashes.every((hash) => order.has(hash))) return hashes;
  return [...hashes].sort((one, other) => order.get(one)! - order.get(other)!);
}

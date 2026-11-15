/**
 * Which feature of Gitardo's engine an action of the views runs, with its target and the answers the view already
 * gave, for the areas moved to the engine ([docs/transition/06-roadmap.md](../../../docs/transition/06-roadmap.md),
 * stage 4). An action that has none here still runs as a `GitOp` of `src/git/ops.ts`, and so does every action when the
 * engine is not there (D49).
 */

import type { Action, RepoState } from '../shared/protocol';
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
  state: Pick<RepoState, 'branches' | 'remoteBranches' | 'tags'> & Partial<Pick<RepoState, 'working' | 'rebaseStop'>>,
): EngineRun[] | null {
  // Conflicts (stage 4, area 4): staging the one file in conflict, from its row in the Changes view, marks it resolved.
  if (action.kind === 'stage' && action.paths.length === 1 && state.working?.conflicts.some((file) => file.path === action.paths[0])) {
    return [{ feature: 'conflicts.mark-resolved', target: { kind: 'file', path: action.paths[0] } }];
  }
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

    // Conflicts (stage 4, area 4).
    case 'resolveConflicts': {
      // The engine resolves every file in conflict: some of them only stay with TypeScript.
      const conflicted = state.working?.conflicts.map((file) => file.path) ?? [];
      if (action.paths && conflicted.some((path) => !action.paths?.includes(path))) return null;
      return [{ feature: 'conflicts.resolve-automatically', target: { kind: 'head' } }];
    }
    case 'takeSide':
      return [
        {
          feature: 'conflicts.take-side',
          target: { kind: 'file', path: action.path },
          answers: { side: { kind: 'choice', identifier: action.side === 'ours' ? 'current' : 'incoming' } },
        },
      ];
    case 'continue':
      // A rebase stopped to edit or to split a commit goes on as its stop says, which is the rewriting area's.
      return state.rebaseStop ? null : [{ feature: 'conflicts.continue-operation', target: { kind: 'head' } }];
    case 'skip':
      return [{ feature: 'conflicts.skip-step', target: { kind: 'head' } }];
    case 'abort':
      return [{ feature: 'conflicts.abort-operation', target: { kind: 'head' } }];

    default:
      return null;
  }
}

/**
 * Which feature of Gitardo's engine an action of the views runs, with its target and the answers the view already
 * gave, for the areas moved to the engine ([docs/transition/06-roadmap.md](../../../docs/transition/06-roadmap.md),
 * stage 4). An action that has none here still runs as a `GitOp` of `src/git/ops.ts`, and so does every action when the
 * engine is not there (D49).
 */

import type { Action, RepoState } from '../shared/protocol';
import { isHome } from '../shared/refs';
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
  state: Pick<RepoState, 'branches' | 'remoteBranches' | 'tags'> & Partial<Pick<RepoState, 'worktrees' | 'submodules' | 'home'>>,
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

    // Worktrees and what new worktrees get, submodules (area 5). Opening a worktree or a submodule in a window, a
    // terminal or the file manager stays VS Code's, and so does the folder of this window, which only the extension
    // knows and refuses to remove or rename.
    case 'worktreeAdd':
      if (!action.ref) return [{ feature: 'worktree.create', target: { kind: 'head' } }];
      return action.isBranch
        ? [{ feature: 'worktree.create-from-branch', target: { kind: 'branch', name: action.ref } }]
        : [{ feature: 'worktree.create-at-commit', target: { kind: 'commit', commit: action.ref } }];
    case 'worktreeRemove': {
      if (isHome(action.path, state)) return null;
      const answers = action.deleteBranch === undefined ? undefined : { branch: choiceOf(action.deleteBranch ? 'delete' : 'keep') };
      return [{ feature: 'worktree.remove', target: { kind: 'worktree', folder: action.path }, answers }];
    }
    case 'worktreeRename':
      return isHome(action.path, state) ? null : [{ feature: 'worktree.rename', target: { kind: 'worktree', folder: action.path } }];
    case 'worktreePrune':
      return [{ feature: 'worktree.prune', target: { kind: 'head' } }];
    case 'worktreeSetUp':
      return action.path
        ? [{ feature: 'worktree-setup.bring-into-worktree', target: { kind: 'worktree', folder: action.path } }]
        : [{ feature: 'worktree-setup.bring-into-other-worktrees', target: { kind: 'head' } }];
    case 'openInWorktree': {
      // A branch with a worktree only opens it, where gitardo.worktrees.openIn says, the Agents window included.
      const branch = action.remote ? null : action.name;
      if (branch && state.worktrees?.some((worktree) => worktree.branch === branch && !worktree.current)) return null;
      const target: Target = action.remote ? { kind: 'remoteBranch', name: action.name } : { kind: 'branch', name: action.name };
      return [{ feature: 'worktree.open-in-worktree', target }];
    }
    case 'submoduleAdd':
      return [{ feature: 'submodule.add', target: { kind: 'head' } }];
    case 'submoduleUpdate': {
      const submodule = state.submodules?.find((each) => each.path === action.path);
      const feature = action.remote ? 'submodule.update-to-latest' : submodule && !submodule.head ? 'submodule.initialize' : 'submodule.update-to-recorded';
      return [{ feature, target: { kind: 'submodule', path: action.path } }];
    }
    case 'submoduleRecord':
      return [{ feature: 'submodule.stage-recorded', target: { kind: 'submodule', path: action.path } }];
    case 'submoduleDeinit':
      return [{ feature: 'submodule.deinitialize', target: { kind: 'submodule', path: action.path } }];
    case 'submoduleRemove':
      return [{ feature: 'submodule.remove', target: { kind: 'submodule', path: action.path } }];
    case 'submoduleSync':
      return [{ feature: 'submodule.sync-urls', target: { kind: 'head' } }];

    default:
      return null;
  }
}

/** The answer that picks a choice. */
const choiceOf = (identifier: string): Answer => ({ kind: 'choice', identifier });

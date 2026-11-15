/**
 * Which feature of Gitardo's engine an action of the views runs, with its target and the answers the view already
 * gave, for the areas moved to the engine ([docs/transition/06-roadmap.md](../../../docs/transition/06-roadmap.md),
 * stage 4). An action that has none here still runs as a `GitOp` of `src/git/ops.ts`, and so does every action when the
 * engine is not there (D49).
 */

import type { Action, PullAction, RepoState, SyncAction } from '../shared/protocol';
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
export function engineRuns(action: Action, state: Pick<RepoState, 'branches' | 'remoteBranches' | 'tags'>): EngineRun[] | null {
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

    // Branches, tags and remotes (stage 4, area 2).
    case 'checkout':
      return checkoutRuns(action.target, action.ref, action.rebase ?? false, state);
    case 'renameBranch':
      return [{ feature: 'branch.rename', target: { kind: 'branch', name: action.name } }];
    case 'deleteBranch':
      return [{ feature: 'branch.delete', target: { kind: 'branch', name: action.name } }];
    case 'fastForward':
      return [{ feature: 'branch.fast-forward', target: { kind: 'branch', name: action.name } }];
    case 'deleteRemoteBranch':
      return [{ feature: 'remote.delete-branch', target: { kind: 'remoteBranch', name: action.name } }];
    case 'createTag':
      return [{ feature: 'tag.create', target: targetOf(action.at, state) }];
    case 'deleteTag':
      return [{ feature: 'tag.delete', target: { kind: 'tag', name: action.name } }];
    case 'pushTag':
      return [{ feature: 'tag.push', target: { kind: 'tag', name: action.name } }];

    default:
      return null;
  }
}

/**
 * The run of a checkout, by what it checks out: a branch; a remote branch, in the local branch of its name, rebased onto
 * it when asked; a tag or a commit on a detached `HEAD`. None for a name that is not of the kind the menu said.
 */
function checkoutRuns(
  kind: 'branch' | 'remote' | 'detached',
  ref: string,
  rebase: boolean,
  state: Pick<RepoState, 'branches' | 'remoteBranches' | 'tags'>,
): EngineRun[] | null {
  const target = targetOf(ref, state);
  if (kind === 'branch') return target.kind === 'branch' ? [{ feature: 'branch.checkout', target }] : null;
  if (kind === 'remote') {
    if (target.kind !== 'remoteBranch') return null;
    return [{ feature: rebase ? 'branch.checkout-remote-and-rebase' : 'branch.checkout-remote', target }];
  }
  if (target.kind === 'tag') return [{ feature: 'tag.checkout', target }];
  return target.kind === 'commit' ? [{ feature: 'branch.checkout-detached', target }] : null;
}

/** The ways to pull of the toolbar, by the identifiers of `sync.pull-with`. */
const PULL_WAYS: Record<PullAction, string> = {
  pull: 'plain',
  pullRebase: 'rebase',
  pullAutostash: 'autostash',
  pullRebaseAutostash: 'rebase-autostash',
};

/**
 * The run of Fetch, of a way to pull, of a way to push, or of Sync, on what is checked out: the way to pull the
 * settings choose is **Pull** (`sync.pull`), the others are pulled in their way (`sync.pull-with`).
 */
export function syncRuns(action: SyncAction, defaultPull: PullAction): EngineRun[] {
  const head: Target = { kind: 'head' };
  switch (action) {
    case 'fetch':
      return [{ feature: 'sync.fetch', target: head }];
    case 'push':
      return [{ feature: 'sync.push', target: head }];
    case 'pushForceWithLease':
      return [{ feature: 'sync.push-force-with-lease', target: head }];
    case 'pushForce':
      return [{ feature: 'sync.push-force', target: head }];
    case 'sync':
      return [{ feature: 'sync.sync', target: head }];
    default:
      if (action === defaultPull) return [{ feature: 'sync.pull', target: head }];
      return [{ feature: 'sync.pull-with', target: head, answers: { way: { kind: 'choice', identifier: PULL_WAYS[action] } } }];
  }
}

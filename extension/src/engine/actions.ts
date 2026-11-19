/**
 * Which feature of Gitardo's engine an action of the views runs, with its target and the answers the view already
 * gave ([docs/transition/06-roadmap.md](../../../docs/transition/06-roadmap.md), stage 4). An action that has none here
 * is VS Code's own (`hostAction` of `src/extension/actions.ts`), or one there is nothing to run for.
 */

import type { Action, MergeMode, PullAction, RebaseStep, RepoState, SyncAction } from '../shared/protocol';
import { isHome } from '../shared/refs';
import { worktreeBranch } from '../shared/state';
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
  state: Pick<RepoState, 'branches' | 'remoteBranches' | 'tags'> & Partial<Pick<RepoState, 'bisect' | 'rows' | 'workingTree' | 'rebaseStop' | 'worktrees' | 'submodules' | 'home'>>,
): EngineRun[] | null {
  // Conflicts (stage 4, area 4): staging the one file in conflict, from its row in the Changes view, marks it resolved.
  if (action.kind === 'stage' && action.paths.length === 1 && state.workingTree?.conflicted.some((file) => file.path === action.paths[0])) {
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
    // Conflicts (stage 4, area 4).
    case 'resolveConflicts': {
      // The engine resolves every file in conflict: some of them only are looked at in the review.
      const conflicted = state.workingTree?.conflicted.map((file) => file.path) ?? [];
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
      // A rebase stopped to edit or to split a commit goes on as its stop says, which the engine hands over to.
      return [{ feature: 'conflicts.continue-operation', target: { kind: 'head' } }];
    case 'skip':
      return [{ feature: 'conflicts.skip-step', target: { kind: 'head' } }];
    case 'abort':
      return [{ feature: 'conflicts.abort-operation', target: { kind: 'head' } }];
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
      const answers = action.deleteBranch === undefined ? undefined : { branch: choice(action.deleteBranch ? 'delete' : 'keep') };
      return [{ feature: 'worktree.remove', target: { kind: 'worktree', folder: action.path }, answers }];
    }
    case 'worktreeRename':
      return isHome(action.path, state) ? null : [{ feature: 'worktree.rename', target: { kind: 'worktree', folder: action.path } }];
    case 'worktreePrune':
      return [{ feature: 'worktree.prune', target: { kind: 'head' } }];
    case 'worktreeLock':
      return [{ feature: 'worktree.lock', target: { kind: 'worktree', folder: action.path } }];
    case 'worktreeUnlock':
      return [{ feature: 'worktree.unlock', target: { kind: 'worktree', folder: action.path } }];
    case 'worktreeSetUp':
      return action.path
        ? [{ feature: 'worktree-setup.bring-into-worktree', target: { kind: 'worktree', folder: action.path } }]
        : [{ feature: 'worktree-setup.bring-into-other-worktrees', target: { kind: 'head' } }];
    case 'openInWorktree': {
      // A branch with a worktree only opens it, where gitardo.worktrees.openIn says, the Agents window included.
      const branch = action.remote ? null : action.name;
      if (branch && state.worktrees?.some((worktree) => worktreeBranch(worktree) === branch && !worktree.isCurrent)) return null;
      const target: Target = action.remote ? { kind: 'remoteBranch', name: action.name } : { kind: 'branch', name: action.name };
      return [{ feature: 'worktree.open-in-worktree', target }];
    }
    case 'submoduleAdd':
      return [{ feature: 'submodule.add', target: { kind: 'head' } }];
    case 'submoduleUpdate': {
      const submodule = state.submodules?.find((each) => each.folder === action.path);
      const feature = action.remote ? 'submodule.update-to-latest' : submodule && !submodule.checkout?.commit ? 'submodule.initialize' : 'submodule.update-to-recorded';
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

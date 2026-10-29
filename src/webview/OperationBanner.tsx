import type { RepoState } from '../shared/protocol';
import { Icon } from './Icon';
import { act } from './menus';
import { post } from './vscode';

const NAMES = { merge: 'Merge', rebase: 'Rebase', 'cherry-pick': 'Cherry-pick', revert: 'Revert', am: 'Patch (git am)' };

const RESOLVE_TITLE =
  'Resolve the conflicts that need no choice between the two sides: changes to lines next to each other, the same change made by both, ' +
  'differences in spaces or line endings only. Gitardo shows each resolution first, to accept all of them or one by one; the other conflicts stay as they are.';

const AI_TITLE =
  'Ask the language model of VS Code how to resolve the conflicts that the rules leave. Gitardo shows each resolution first, ' +
  'to accept all of them or one by one, or to resolve by hand.';

/** How many of the conflicts marked in the files need no choice between their two sides, and how many there are. */
export function resolvableConflicts(state: RepoState, path?: string): [resolvable: number, marked: number] {
  const files = state.conflictFiles.filter((file) => path === undefined || file.path === path);
  return [files.reduce((sum, file) => sum + file.resolvable.length, 0), files.reduce((sum, file) => sum + file.conflicts, 0)];
}

/**
 * Shown while a merge, rebase, cherry-pick, revert or patch has stopped and waits for the user, and while files are in
 * conflict without one, as after a stash was applied. A rebase that stopped to edit or split a commit says so.
 */
export function OperationBanner({ state, busy }: { state: RepoState; busy: boolean }) {
  const { operation, rebaseStop } = state;
  const conflicts = state.working.conflicts.length;
  if (!operation && !conflicts) return null;
  const [resolvable, marked] = resolvableConflicts(state);
  // Nothing is counted while the rules are switched off.
  const leftByRules = !marked || resolvable < marked;
  const files = `${conflicts} ${conflicts === 1 ? 'file has' : 'files have'} conflicts`;
  return (
    <div className="banner" role="status">
      <Icon name={rebaseStop ? 'edit' : 'warning'} />
      <span className="grow">
        {rebaseStop?.kind === 'split'
          ? `Splitting "${rebaseStop.subject}": commit its changes in as many commits as you like, then Continue.`
          : rebaseStop
            ? `Stopped at "${rebaseStop.subject}" to edit it. Amend it or commit more, then Continue.`
            : operation
              ? `${NAMES[operation]} in progress${conflicts > 0 ? ` · ${files}` : ''}`
              : files}
      </span>
      {resolvable > 0 && (
        <button className="button resolve" disabled={busy} title={RESOLVE_TITLE} onClick={() => post({ type: 'reviewConflicts' })}>
          <Icon name="wand" />
          Resolve {resolvable === marked ? (marked === 1 ? 'it' : `all ${marked}`) : `${resolvable} of ${marked}`} automatically
        </button>
      )}
      {state.resolveWithAI && conflicts > 0 && leftByRules && (
        <button className="button resolve ai" disabled={busy} title={AI_TITLE} onClick={() => post({ type: 'reviewConflicts', ai: true })}>
          <Icon name="sparkle" />
          Resolve with AI
        </button>
      )}
      {operation && (
        <button
          className="button"
          disabled={busy || conflicts > 0}
          title={
            conflicts > 0
              ? 'Resolve the conflicts and stage the files first'
              : rebaseStop
                ? 'Go on with the rebase: the commits after it are made again on top'
                : undefined
          }
          onClick={() => act({ kind: 'continue' })}
        >
          Continue
        </button>
      )}
      {/* Skipping here would throw away what is not committed, and the commit stays anyway. */}
      {operation && operation !== 'merge' && !rebaseStop && (
        <button className="button" disabled={busy} onClick={() => act({ kind: 'skip' })}>
          Skip
        </button>
      )}
      {operation && (
        <button className="button" disabled={busy} onClick={() => act({ kind: 'abort' })}>
          Abort
        </button>
      )}
    </div>
  );
}

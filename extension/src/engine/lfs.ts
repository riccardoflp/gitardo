/**
 * What the **Large files** tab asks of the engine (`lfs.page`, milestone M47): the operation each of its buttons and
 * entries runs with `feature/run`, and the remote its note of the locks names.
 */
import type { LargeFilesAsk } from '../shared/lfs';
import type { RepoState } from '../shared/protocol';
import type { EngineRun } from './actions';
import type { Target } from './messages';

/**
 * The remote the note of the locks names, as the desktop names it (`remote.default`): `origin`, or the first remote;
 * the engine asks the locks of the remote Git LFS chooses.
 */
export function locksRemote(state: Pick<RepoState, 'remotes'> | null): string | null {
  if (!state) return null;
  return state.remotes.includes('origin') ? 'origin' : (state.remotes[0] ?? null);
}

/** The operation an ask of the tab runs, and whether the locks are read again after it; none for what runs no git. */
export function lfsRun(ask: LargeFilesAsk): { run: EngineRun; readLocks: boolean; downloads: boolean } | null {
  const head: Target = { kind: 'head' };
  const operation = (feature: string, target: Target = head, answers?: EngineRun['answers']) => ({ feature, target, answers });
  switch (ask.kind) {
    case 'setUp':
      return { run: operation('lfs.set-up'), readLocks: false, downloads: false };
    case 'track':
      return { run: operation('lfs.track'), readLocks: false, downloads: false };
    case 'untrack':
      return { run: operation('lfs.untrack', head, { pattern: { kind: 'text', text: ask.pattern } }), readLocks: false, downloads: false };
    case 'storeCommitted':
      return { run: operation('lfs.store-committed'), readLocks: false, downloads: false };
    case 'download':
    case 'downloadAll': {
      const paths = ask.kind === 'download' ? ask.paths.map((file) => ({ path: file, isFolder: false })) : [];
      return { run: operation('lfs.download', { kind: 'lfsFiles', paths, commit: null }), readLocks: false, downloads: true };
    }
    case 'lock':
      return { run: operation('lfs.lock', { kind: 'file', path: ask.path }), readLocks: true, downloads: false };
    case 'unlock':
      return { run: operation('lfs.unlock', { kind: 'lfsLock', ...ask.lock }), readLocks: true, downloads: false };
    default:
      return null;
  }
}

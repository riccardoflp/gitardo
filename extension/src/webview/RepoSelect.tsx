import type { RepoState } from '../shared/protocol';
import { post } from './vscode';

/** The spaces that indent a name in the list, which cannot pad its options: no-break spaces, which it keeps. */
const INDENT = '\u00a0'.repeat(4);

/** A name in the list, under the repository of its group as far as `depth` says (`repo.group-tabs`). */
export const indented = (name: string, depth: number) => `${INDENT.repeat(Math.max(0, depth || 0))}${name}`;

/**
 * Chooses which repository of the workspace every Gitardo view shows. Nothing is rendered when there is only one. While
 * the views show a worktree that is not in the workspace, the repository it is of stays chosen: the worktrees are
 * chosen beside the branch. The worktrees and the submodules of a repository are listed under it, indented.
 */
export function RepoSelect({ state, disabled }: { state: RepoState; disabled: boolean }) {
  if (state.repos.length < 2) return null;
  const chosen = state.home ?? state.repos.find((repo) => samePath(repo.root, state.root));
  return (
    <select
      className="repo"
      value={chosen?.root ?? state.root}
      disabled={disabled}
      aria-label="Repository"
      title={state.root}
      onChange={(event) => post({ type: 'selectRepo', root: event.target.value })}
    >
      {state.repos.map((repo) => (
        <option key={repo.root} value={repo.root}>
          {indented(repo.name, repo.depth)}
        </option>
      ))}
      {!chosen && <option value={state.root}>{state.name}</option>}
    </select>
  );
}

function samePath(a: string, b: string): boolean {
  const same = (path: string) => path.replace(/\\/g, '/').toLowerCase();
  return same(a) === same(b);
}

import type { RepoState } from '../shared/protocol';
import { post } from './vscode';

/**
 * Chooses which repository of the workspace every Gitardo view shows. Nothing is rendered when there is only one. While
 * the views show a worktree that is not in the workspace, the repository it is of stays chosen: the worktrees are
 * chosen beside the branch.
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
          {repo.name}
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

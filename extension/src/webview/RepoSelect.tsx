import type { RepoState } from '../shared/protocol';
import { useMenu } from './ContextMenu';
import { Icon } from './Icon';
import { post } from './vscode';

/**
 * Chooses which repository of the workspace every Gitardo view shows: a button that names the one shown and opens a
 * menu of them all, as the button of the worktree beside it does. Nothing is rendered when there is only one. While
 * the views show a worktree that is not in the workspace, the repository it is of stays chosen: the worktrees are
 * chosen beside the branch. The worktrees and the submodules of a repository are listed under it, indented
 * (`repo.group-tabs`).
 */
export function RepoSelect({ state, disabled }: { state: RepoState; disabled: boolean }) {
  const openMenu = useMenu();
  if (state.repos.length < 2) return null;
  const chosen = state.home ?? state.repos.find((repo) => samePath(repo.root, state.root));
  const name = chosen?.name ?? state.name;
  const show = (root: string) => {
    if (!samePath(root, chosen?.root ?? state.root)) post({ type: 'selectRepo', root });
  };
  return (
    <button
      className="place pick repo"
      disabled={disabled}
      aria-haspopup="menu"
      aria-label={`Repository: ${name}`}
      title={`${state.root}\nShow another repository of the workspace`}
      onClick={(event) =>
        openMenu(event, [
          ...state.repos.map((repo) => ({
            label: repo.name,
            indent: Math.max(0, repo.depth || 0),
            title: repo.root,
            checked: !!chosen && samePath(repo.root, chosen.root),
            run: () => show(repo.root),
          })),
          // The one shown is not of the workspace: it is listed after them, and ticked.
          !chosen && { label: state.name, title: state.root, checked: true, run: () => {} },
        ])
      }
    >
      <Icon name="repo" />
      <span className="name">{name}</span>
      <Icon name="chevron-down" />
    </button>
  );
}

function samePath(a: string, b: string): boolean {
  const same = (path: string) => path.replace(/\\/g, '/').toLowerCase();
  return same(a) === same(b);
}

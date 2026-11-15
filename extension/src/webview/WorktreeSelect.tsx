import type { RepoState } from '../shared/protocol';
import { worktreeLabel } from '../shared/refs';
import { checkedOutBranch, worktreeBranch, type Worktree } from '../shared/state';
import { useMenu } from './ContextMenu';
import { Icon } from './Icon';
import { post } from './vscode';

/** What a worktree has checked out, as the toolbar says it. */
function checkedOut(worktree: Worktree): string {
  return worktreeBranch(worktree) ?? (worktree.commit ? `detached at ${worktree.commit.slice(0, 7)}` : 'no commits yet');
}

/**
 * The branch checked out in the worktree the views show and, when the repository has other worktrees, the way to show
 * one of them instead: a menu of the worktrees, each by its branch and its folder.
 */
export function WorktreeSelect({ state, disabled }: { state: RepoState; disabled: boolean }) {
  const openMenu = useMenu();
  // Those whose folder is there.
  const worktrees = state.worktrees.filter((worktree) => !worktree.isMissing);
  const head = state.head.commit;
  const name = checkedOutBranch(state) ?? (head ? `detached at ${head.slice(0, 7)}` : 'no commits yet');
  if (worktrees.length < 2) {
    return (
      <span className="current" title={state.root}>
        <Icon name="git-branch" />
        <span className="name">{name}</span>
      </span>
    );
  }
  const shown = worktrees.find((worktree) => worktree.isCurrent);
  return (
    <button
      className="current pick"
      disabled={disabled}
      aria-haspopup="menu"
      aria-label={`Worktree: ${name}`}
      title={`${state.root}\nShow another worktree of this repository`}
      onClick={(event) =>
        openMenu(
          event,
          worktrees.map((worktree) => {
            const folder = worktreeLabel(worktree.folder, state.worktreesRoot);
            return {
              label: checkedOut(worktree),
              // The folder, unless it is named as the branch is, as the worktrees Gitardo makes are.
              hint: folder === worktreeBranch(worktree) ? undefined : folder,
              checked: worktree === shown,
              run: () => {
                if (worktree !== shown) post({ type: 'selectRepo', root: worktree.folder });
              },
            };
          }),
        )
      }
    >
      <Icon name={shown?.isMain === false ? 'worktree' : 'git-branch'} />
      <span className="name">{name}</span>
      <Icon name="chevron-down" />
    </button>
  );
}

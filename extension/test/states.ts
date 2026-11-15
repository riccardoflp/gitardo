/**
 * The parts of the state of a repository that the tests of the views write by hand, in the engine's shapes
 * (`src/shared/state.ts`): a name of a branch is given short, as the tests say it, and made full where the shape wants it.
 */
import type { Branch, Head, RemoteBranch, Tag, Upstream, WorkingTree, Worktree } from '../src/shared/state';

/** What is checked out: a commit, on the branch named, or detached. */
export const headAt = (commit: string | null, branch: string | null = null): Head => ({ commit, branch: branch === null ? null : `refs/heads/${branch}` });

/** The upstream `origin/main` of a branch, which is `main` on the remote `origin`. */
export function upstreamOf(name: string, extra: Partial<Upstream> = {}): Upstream {
  const slash = name.indexOf('/');
  return {
    reference: `refs/remotes/${name}`,
    remote: name.slice(0, slash),
    remoteBranch: `refs/heads/${name.slice(slash + 1)}`,
    ahead: 0,
    behind: 0,
    isGone: false,
    ...extra,
  };
}

export const branchAt = (name: string, commit: string, extra: Partial<Branch> = {}): Branch => ({
  name,
  commit,
  upstream: null,
  isCheckedOut: false,
  date: 0,
  ...extra,
});

export const remoteAt = (name: string, commit: string): RemoteBranch => ({ name, remote: name.slice(0, name.indexOf('/')), commit, date: 0 });

export const tagAt = (name: string, commit: string): Tag => ({ name, commit, date: 0 });

/** A worktree in `folder` with the branch named checked out, or detached. */
export const worktreeAt = (folder: string, branch: string | null, extra: Partial<Worktree> = {}): Worktree => ({
  folder,
  commit: 'a'.repeat(40),
  branch: branch === null ? null : `refs/heads/${branch}`,
  isMain: false,
  isCurrent: false,
  isMissing: false,
  uncommitted: 0,
  ...extra,
});

/** A working tree with nothing in it but what is given. */
export const workingTreeOf = (extra: Partial<WorkingTree> = {}): WorkingTree => ({ staged: [], unstaged: [], untracked: [], conflicted: [], ...extra });

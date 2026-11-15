import type { RepoState, Submodule } from './protocol';

/**
 * Where a submodule stands against the commit recorded for it: on it, with commits of its own after it, behind it, or
 * both. `uninitialized` has no repository in its folder, `added` has nothing recorded yet, and `unknown` does not have
 * the recorded commit, so cannot be compared with it.
 */
export type SubmoduleStatus = 'uninitialized' | 'added' | 'unknown' | 'synced' | 'ahead' | 'behind' | 'diverged';

export function submoduleStatus(submodule: Submodule): SubmoduleStatus {
  // One whose checkout has no commit yet is as good as not initialized: there is nothing to compare.
  if (!submodule.checkout?.commit) return 'uninitialized';
  if (!submodule.recorded) return 'added';
  const distance = submodule.checkout.distance;
  if (!distance) return 'unknown';
  if (distance.ahead && distance.behind) return 'diverged';
  if (distance.ahead) return 'ahead';
  if (distance.behind) return 'behind';
  return 'synced';
}

const short = (hash: string) => hash.slice(0, 7);
const commits = (count: number) => (count === 1 ? '1 commit' : `${count} commits`);

/** What its status icon says, in a sentence. */
export function submoduleStatusText(submodule: Submodule): string {
  const recorded = submodule.recorded ? short(submodule.recorded) : '';
  const ahead = submodule.checkout?.distance?.ahead ?? 0;
  const behind = submodule.checkout?.distance?.behind ?? 0;
  switch (submoduleStatus(submodule)) {
    case 'uninitialized':
      return 'Not initialized: its folder has no checkout';
    case 'added':
      return 'Added, not committed yet';
    case 'unknown':
      return `The recorded commit ${recorded} is not in it: fetch in it, or update it`;
    case 'synced':
      return `On the recorded commit ${recorded}`;
    case 'ahead':
      return `${commits(ahead)} ahead of the recorded commit ${recorded}`;
    case 'behind':
      return `${commits(behind)} behind the recorded commit ${recorded}`;
    case 'diverged':
      return `${commits(ahead)} ahead of and ${commits(behind)} behind the recorded commit ${recorded}`;
  }
}

/** Its folder on disk, with forward slashes as the root has them. */
export const submoduleFolder = (submodule: Submodule, state: Pick<RepoState, 'root'>) => `${state.root.replace(/[\\/]+$/, '')}/${submodule.folder}`;

/** How many submodules it is inside: 0 for one of the repository itself. */
export function submoduleDepth(submodule: Submodule, all: Submodule[]): number {
  let depth = 0;
  for (let parent = submodule.parent; parent; parent = all.find((each) => each.folder === parent)?.parent ?? null) depth++;
  return depth;
}

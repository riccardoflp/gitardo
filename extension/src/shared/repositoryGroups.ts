import { samePath } from './paths';

/** A repository found in the folders of the workspace, as `repository/discover` gives it. */
export interface FoundRepository {
  folder: string;
  /** The repository it is a submodule of, when it is one. */
  superproject: string | null;
  /** The git folder it shares with its worktrees; null when git could not say. */
  commonFolder: string | null;
}

/** A repository in the list of the repositories, and how far under the repository of its group it is. */
export interface Grouped<T> {
  repository: T;
  depth: number;
}

/** A path without the separators at its end, to be compared with `samePath`. */
const trimmed = (folder: string) => folder.replace(/[\\/]+$/, '');

/**
 * The repositories of the workspace in the order the list of `repo.select` shows them, as `repo.group-tabs` groups
 * them: a worktree under the repository whose common git folder it shares, a submodule under the repository around it,
 * and a submodule's own submodules under it. In a group the worktrees come first, then the submodules, each in the order
 * they were found; a repository that is neither is a group of its own, where it was found.
 */
export function groupRepositories<T extends FoundRepository>(found: T[]): Grouped<T>[] {
  const index = (folder: string) => found.findIndex((each) => samePath(trimmed(each.folder), trimmed(folder)));
  // Of the repositories that share a git folder, the one that git folder is the folder `.git` of, when it is listed:
  // that is the repository the others are worktrees of. Else the first of them listed.
  const owner = (at: number): number => {
    const common = found[at].commonFolder;
    if (!common) return -1;
    const sharing = found.flatMap((each, other) => (each.commonFolder && samePath(trimmed(each.commonFolder), trimmed(common)) ? [other] : []));
    const main = sharing.find((other) => samePath(`${trimmed(found[other].folder)}/.git`, trimmed(common)));
    const head = main ?? sharing[0];
    return head === at ? -1 : head;
  };
  // What each repository is listed under, and whether as a worktree or as a submodule.
  const parent = found.map((each, at) => {
    const around = each.superproject ? index(each.superproject) : -1;
    if (around >= 0 && around !== at) return { at: around, worktree: false };
    const head = owner(at);
    return head >= 0 ? { at: head, worktree: true } : null;
  });
  // A repository whose chain of parents comes back to it is a group of its own, so that the list ends.
  const top = (at: number) => {
    const seen = new Set<number>();
    for (let current = parent[at]?.at; current !== undefined; current = parent[current]?.at) {
      if (current === at) return true;
      if (seen.has(current)) return false;
      seen.add(current);
    }
    return parent[at] === null;
  };
  const listed: Grouped<T>[] = [];
  const done = new Set<number>();
  const add = (at: number, depth: number) => {
    if (done.has(at)) return;
    done.add(at);
    listed.push({ repository: found[at], depth });
    const children = (worktree: boolean) => found.flatMap((_, child) => (parent[child]?.at === at && parent[child]?.worktree === worktree && !top(child) ? [child] : []));
    for (const child of [...children(true), ...children(false)]) add(child, depth + 1);
  };
  for (let at = 0; at < found.length; at++) if (top(at)) add(at, 0);
  // None is left out, whatever its parents say.
  for (let at = 0; at < found.length; at++) add(at, 0);
  return listed;
}

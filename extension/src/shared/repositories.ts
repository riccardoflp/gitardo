/**
 * The rows of the Repositories view (`repo.tabs`, milestone M22): a row for each repository Gitardo found in the
 * workspace, the rows of its worktrees and its submodules under it (`repo.group-tabs`), and what each is on
 * (`repo.worktree-summary`), as the engine reads it or as the read of the repository shown says it.
 */

import { samePath } from './paths';
import type { RepoState } from './protocol';
import type { Grouped } from './repositoryGroups';
import { type RepositoryState, type Worktree, worktreeBranch } from './state';
import { type FolderSummary, type WorktreeSummary, summaryOfState } from './summaries';

/** What a row of the view is of. */
export type RowKind = 'repository' | 'worktree' | 'submodule';

/** A row of the Repositories view. */
export interface RepositoryRow {
  /** The folder a click on it shows. */
  folder: string;
  kind: RowKind;
  /** The name of a repository as the lists have it, of a worktree its folder's, of a submodule its path. */
  name: string;
  /** What is said of it after ` › ` on the row of its repository while that is folded: the branch of a worktree. */
  member: string;
  /** 0 for a repository, 1 for one of its worktrees or submodules, 2 for a submodule of one of those. */
  depth: number;
  /** The folder of the repository whose row it is under; its own for a repository. */
  group: string;
  /** What it is on; null until it is read. */
  summary: WorktreeSummary | null;
  /** The branch git listed for a worktree, said until it is read. */
  listed: string | null;
  /** The last read of it failed: what it says is from the one before. */
  isStale: boolean;
  /** It is the one Gitardo shows. */
  isShown: boolean;
  /** An operation runs in it. */
  isRunning: boolean;
}

/**
 * What the view is sent of the repository Gitardo shows, for the menus of its rows: its worktrees and its submodules as
 * the Branches view has them, and what their menus ask of the window.
 */
export type ShownParts = Pick<RepoState, 'root' | 'home' | 'agentsWindow' | 'worktrees' | 'submodules' | 'isLookedAfter' | 'commitGraph'>;

/** A repository of the workspace as the rows take it: what `repository/discover` found, and its name in the lists. */
export interface ListedRepository {
  folder: string;
  name: string;
  superproject: string | null;
}

/** What the rows are made from. */
export interface RowsFrom {
  /** The repositories of the workspace, grouped as `groupRepositories` orders them. */
  repositories: Grouped<ListedRepository>[];
  /** What the engine said of the folders it follows. */
  summaries: FolderSummary[];
  /** The state of what Gitardo shows, which says its own summary and the worktrees of its repository. */
  state: RepositoryState | null;
  /** The folder an operation runs in, if one does. */
  running: string | null;
}

const forward = (folder: string) => folder.replace(/\\/g, '/').replace(/\/+$/, '');
const lastName = (folder: string) => forward(folder).split('/').pop() ?? folder;

/** A folder from another it is inside, with forward slashes; its last name when it is not inside. */
function pathFrom(around: string, folder: string): string {
  const [outer, inner] = [forward(around), forward(folder)];
  return inner.toLowerCase().startsWith(`${outer.toLowerCase()}/`) ? inner.slice(outer.length + 1) : lastName(folder);
}

/**
 * The rows of the view, in their order: each repository of the workspace, then its other worktrees, those git lists
 * for it whether they are folders of the workspace or not, then its submodules, a submodule's own after it. A worktree
 * whose folder is gone has no row.
 */
export function repositoryRows({ repositories, summaries, state, running }: RowsFrom): RepositoryRow[] {
  const said = (folder: string) => summaries.find((each) => samePath(forward(each.folder), forward(folder)));
  const shown = state?.folder;
  const row = (folder: string, kind: RowKind, name: string, depth: number, group: string, listed: Worktree | undefined): RepositoryRow => {
    const isShown = shown !== undefined && samePath(forward(shown), forward(folder));
    const read = said(folder);
    const summary = isShown && state ? summaryOfState(state) : (read?.summary ?? null);
    const branch = summary?.head.kind === 'branch' ? summary.head.name : listed ? worktreeBranch(listed) : null;
    return {
      folder,
      kind,
      name,
      member: kind === 'worktree' ? (branch ?? name) : name,
      depth,
      group,
      summary,
      listed: listed ? (worktreeBranch(listed) ?? (listed.commit ? `detached at ${listed.commit.slice(0, 7)}` : null)) : null,
      isStale: !isShown && read?.isStale === true,
      isShown,
      isRunning: running !== null && samePath(forward(running), forward(folder)),
    };
  };
  const rows: RepositoryRow[] = [];
  for (let at = 0; at < repositories.length; ) {
    const head = repositories[at];
    let end = at + 1;
    while (end < repositories.length && repositories[end].depth > head.depth) end++;
    const members = repositories.slice(at + 1, end);
    at = end;
    const group = head.repository.folder;
    rows.push(row(group, 'repository', head.repository.name, 0, group, undefined));
    // The worktrees and the submodules found in the workspace, each with what is under it.
    const found: { isSubmodule: boolean; rows: Grouped<ListedRepository>[] }[] = [];
    for (const member of members) {
      if (member.depth === head.depth + 1) found.push({ isSubmodule: member.repository.superproject !== null, rows: [member] });
      else found[found.length - 1]?.rows.push(member);
    }
    // The worktrees git lists: those of the state when it is of this repository, which is read last, else the engine's.
    const ofState = state?.worktrees.some((each) => samePath(forward(each.folder), forward(group))) ? state.worktrees : undefined;
    const listed = (ofState ?? said(group)?.worktrees ?? []).filter((each) => !each.isMissing && !samePath(forward(each.folder), forward(group)));
    const gitLists = (folder: string) => listed.find((each) => samePath(forward(each.folder), forward(folder)));
    const under = (each: Grouped<ListedRepository>, top: Grouped<ListedRepository>) => {
      const { folder, superproject } = each.repository;
      const depth = each.depth - head.depth;
      if (each === top && superproject === null) return row(folder, 'worktree', lastName(folder), depth, group, gitLists(folder));
      return row(folder, 'submodule', pathFrom(each === top ? group : top.repository.folder, folder), depth, group, undefined);
    };
    const inWorkspace = found.filter((each) => !each.isSubmodule);
    for (const each of inWorkspace) rows.push(...each.rows.map((member) => under(member, each.rows[0])));
    for (const worktree of listed) {
      if (members.some((member) => samePath(forward(member.repository.folder), forward(worktree.folder)))) continue;
      rows.push(row(worktree.folder, 'worktree', lastName(worktree.folder), 1, group, worktree));
    }
    for (const each of found.filter((one) => one.isSubmodule)) rows.push(...each.rows.map((member) => under(member, each.rows[0])));
  }
  return rows;
}

/** A row as the view draws it. */
export interface DrawnRow extends RepositoryRow {
  /** It is a repository with rows under it, and so has a chevron, which is named after `repository`. */
  hasRows: boolean;
  /** The name of the repository its chevron folds, which its own name has more than while it is folded. */
  repository: string;
  /** Its rows are folded. */
  isFolded: boolean;
}

/**
 * The rows the view draws, with the rows under the repositories in `folded` left out. Folded, the row of a repository
 * stands for what its group shows: it says that worktree or submodule after ` › ` and what that is on, is marked as the
 * one shown, and a click on it shows that one; with none of them shown, it stands for its own folder.
 */
export function drawnRows(rows: RepositoryRow[], folded: readonly string[]): DrawnRow[] {
  const isFolded = (group: string) => folded.some((each) => samePath(each, group));
  const drawn: DrawnRow[] = [];
  for (const row of rows) {
    if (row.kind !== 'repository') {
      if (!isFolded(row.group)) drawn.push({ ...row, hasRows: false, repository: row.name, isFolded: false });
      continue;
    }
    const members = rows.filter((each) => each.kind !== 'repository' && samePath(each.group, row.group));
    const closed = members.length > 0 && isFolded(row.group);
    const shown = closed ? members.find((each) => each.isShown) : undefined;
    drawn.push({
      ...(shown ? { ...shown, kind: row.kind, name: `${row.name} › ${shown.member}`, depth: 0 } : row),
      isRunning: closed ? row.isRunning || members.some((each) => each.isRunning) : row.isRunning,
      hasRows: members.length > 0,
      repository: row.name,
      isFolded: closed,
    });
  }
  return drawn;
}

/** The folders the engine follows for the rows, as the watchers need them to say a change to the right ones. */
export interface FollowedFolders {
  /** Every folder followed: the repositories of the workspace, and the worktrees git lists for them. */
  folders: string[];
  /** The worktrees of each repository, which share its branches. */
  families: string[][];
  /** The submodules followed, each with the folder of the repository around it. */
  submodules: { folder: string; superproject: string }[];
}

/** A change to say to the engine: a folder followed, and a path as `repository/folderChanged` takes it. */
export interface FolderChange {
  folder: string;
  path: string;
}

/** Whether a folder is inside another. */
const isInside = (around: string, folder: string) => forward(folder).toLowerCase().startsWith(`${forward(around).toLowerCase()}/`);

/**
 * Which folders followed a change tells of, when the watcher of `folder` saw `file` change, a path from it with
 * forward slashes. A file of the working tree tells of its folder, unless it is in a worktree or a repository nested
 * there, whose own watcher saw it. A file of the git folder tells of its folder too, and of those that keep their own
 * git folder inside it: a worktree is told what is under `.git/worktrees/<name>/` as a path of its own git folder, a
 * submodule what is under `.git/modules/<path>/`, and every worktree of the repository what they share, its references.
 * The engine reads none of them for it unless what it looks at in their git folder changed.
 */
export function routeChange(folder: string, file: string, followed: FollowedFolders): FolderChange[] {
  if (file === '' || file === '.git') return [];
  if (file.startsWith('.git/')) return gitChange(folder, file.slice(5), followed, false);
  const isSubmodule = (other: string) => followed.submodules.some((each) => samePath(forward(each.folder), forward(other)));
  const nested = followed.folders.some((other) => {
    if (!isInside(folder, other) || isSubmodule(other)) return false;
    const inside = pathFrom(folder, other);
    return file === inside || file.startsWith(`${inside}/`);
  });
  return nested ? [] : [{ folder, path: file }];
}

/** The folders told of a change in the git folder of `owner`; `own` when no other worktree shares what changed. */
function gitChange(owner: string, file: string, followed: FollowedFolders, own: boolean): FolderChange[] {
  const told: FolderChange[] = [{ folder: owner, path: `.git/${file}` }];
  const family = (followed.families.find((each) => each.some((member) => samePath(forward(member), forward(owner)))) ?? []).filter(
    (member) => !samePath(forward(member), forward(owner)),
  );
  const parts = file.split('/');
  if (parts[0] === 'modules') {
    for (const submodule of followed.submodules) {
      if (!samePath(forward(submodule.superproject), forward(owner))) continue;
      const prefix = `modules/${pathFrom(owner, submodule.folder)}/`;
      if (file.startsWith(prefix)) told.push(...gitChange(submodule.folder, file.slice(prefix.length), followed, true));
    }
  } else if (own) {
    return told;
  } else if (parts[0] === 'worktrees') {
    const rest = parts.slice(2).join('/');
    if (rest) for (const member of family) told.push(...gitChange(member, rest, followed, true));
  } else {
    for (const member of family) told.push({ folder: member, path: `.git/${file}` });
  }
  return told;
}

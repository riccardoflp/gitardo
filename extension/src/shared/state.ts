/**
 * The state of a repository and the details of a commit as Gitardo's engine writes them (`repo.read-state` and
 * `details.read-commit`, the JSON of `rust/crates/protocol/src/state.rs` and `reads/commits.rs`), field for field: what
 * the views are sent, what `Repo` reads when the engine is not there, and what the engine's messages carry. From stage 6
 * of `docs/transition/06-roadmap.md`. `test/engineShapes.ts` checks these types against the JSON the engine writes.
 *
 * A commit is its full id, a time is in seconds since 1970, a path is a string, a name of a branch is its short name
 * unless the field says it is a full one (`refs/heads/main`).
 */

/** How a file changed. */
export type ChangeKind = 'added' | 'modified' | 'deleted' | 'renamed' | 'copied' | 'typeChanged';

/** A file changed, in a commit, in a comparison, in the index or in the working tree. */
export interface FileChange {
  /** Its path, from the top of the working tree, with forward slashes. */
  path: string;
  /** The path it had, for a rename or a copy. */
  originalPath: string | null;
  kind: ChangeKind;
}

/** What each side of a conflict did to a file, as `git status` says it: `bothModified` is `UU`. */
export type ConflictKind = 'bothDeleted' | 'addedByUs' | 'deletedByThem' | 'addedByThem' | 'deletedByUs' | 'bothAdded' | 'bothModified';

/** A file in conflict. */
export interface ConflictedFile {
  path: string;
  kind: ConflictKind;
}

/** What is not committed, split the way `git status` reports it. A file changed in both the index and the working tree is in both lists. */
export interface WorkingTree {
  staged: FileChange[];
  unstaged: FileChange[];
  /** The files git does not track, by their paths. */
  untracked: string[];
  conflicted: ConflictedFile[];
}

/** What is checked out. */
export interface Head {
  /** The commit; null before the first commit. */
  commit: string | null;
  /** The full name of the branch, `refs/heads/main`; null when `HEAD` is detached. */
  branch: string | null;
}

/** The branch a local branch follows. */
export interface Upstream {
  /** Its full name: `refs/remotes/origin/main`, or `refs/heads/main` for a local branch followed. */
  reference: string;
  /** The remote, `.` for a local branch followed. */
  remote: string;
  /** The full name of the branch on the remote, `refs/heads/main`. */
  remoteBranch: string;
  /** How many commits the local branch has that it does not. */
  ahead: number;
  /** How many commits it has that the local branch does not. */
  behind: number;
  /** Whether it is gone from the remote. */
  isGone: boolean;
}

export interface Branch {
  name: string;
  /** The commit it points at. */
  commit: string;
  upstream: Upstream | null;
  /** Whether it is checked out in the worktree read. */
  isCheckedOut: boolean;
  /** When the commit it points at was committed. */
  date: number;
}

export interface RemoteBranch {
  /** Its name with its remote, `origin/main`. */
  name: string;
  /** The remote it belongs to: the longest remote its name starts with. */
  remote: string;
  commit: string;
  date: number;
}

export interface Tag {
  name: string;
  /** The commit it points at, also for an annotated tag. */
  commit: string;
  /** When the commit it points at was committed, or for an annotated tag when the tag was made. */
  date: number;
}

export interface Stash {
  /** Its commit. */
  commit: string;
  /** Its place in the list, the newest at 0: `stash@{index}`. */
  index: number;
  message: string;
  /** When it was made. */
  date: number;
}

export interface Worktree {
  /** Its folder, as `git worktree list` writes it. */
  folder: string;
  /** The commit checked out in it; null before its first commit. */
  commit: string | null;
  /** The full name of the branch checked out in it, `refs/heads/main`; null when it is detached. */
  branch: string | null;
  /** The one the repository was cloned or created in, which git cannot move or remove. */
  isMain: boolean;
  /** The one read: the views show it. */
  isCurrent: boolean;
  /** Its folder is gone, so `git worktree prune` would forget it. */
  isMissing: boolean;
  /**
   * How many of its files are not committed: staged, not staged, untracked or in conflict, each once. Null when they
   * could not be read.
   */
  uncommitted: number | null;
}

/** Who git will record as the author of a commit made here; null for what is not configured. */
export interface Identity {
  name: string | null;
  email: string | null;
}

/** The kind of signature git makes, from `gpg.format`. */
export type SigningKind = 'openpgp' | 'x509' | 'ssh';

/** How git's configuration signs commits: `commit.gpgSign`, `gpg.format` and `user.signingKey`. */
export interface Signing {
  signsEveryCommit: boolean;
  kind: SigningKind;
  /** Null when `user.signingKey` is not set. */
  key: string | null;
}

/** What git says of the signature of a commit (`details.read-signature`), from the letter of `%G?`. */
export type SignatureVerdict =
  | 'good'
  | 'goodUnknownValidity'
  | 'goodExpired'
  | 'goodExpiredKey'
  | 'goodRevokedKey'
  | 'bad'
  | 'cannotCheck'
  | 'signedNotChecked'
  | 'notSigned';

/** The signature of a commit, as git says of it; each text empty when git cannot say it. */
export interface CommitSignature {
  verdict: SignatureVerdict;
  /** Null for a commit that carries none. */
  kind: SigningKind | null;
  /** Who signed, as git names them: `Ada <ada@example.com>`. */
  signer: string;
  key: string;
  fingerprint: string;
  trust: string;
}

/** Someone who made commits in the repository (`commit.read-authors`). */
export interface Author {
  name: string;
  email: string;
  /** How many commits they made, among those read. */
  commits: number;
}

/** A multi-step git operation that stopped and waits for the user; `patch` is `git am`. */
export type Operation = 'merge' | 'rebase' | 'cherryPick' | 'revert' | 'patch';

/** The git command of an operation, which continues, skips or aborts it, and which the messages name it by. */
export const OPERATION_COMMANDS: Record<Operation, string> = { merge: 'merge', rebase: 'rebase', cherryPick: 'cherry-pick', revert: 'revert', patch: 'am' };

/** How the messages name an operation: as git does, `cherry-pick` or `am`. */
export const operationName = (operation: Operation): string => OPERATION_COMMANDS[operation];

/**
 * Where an interactive rebase stopped for the user to change a commit rather than on a conflict: to edit it, or to
 * split it, with its changes taken out of it into the working tree. `commit` is the commit as the rebase made it, or
 * the commit being split.
 */
export interface RebaseStop {
  kind: 'edit' | 'split';
  commit: string;
  subject: string;
}

/** A `git bisect` in progress. HEAD is the commit git wants tested next. */
export interface Bisect {
  bad: string | null;
  good: string[];
  skipped: string[];
  /** How many commits can still be the first bad one, counting `bad` itself. Null until both a good and a bad commit are known. */
  remaining: number | null;
}

/** How far the commit checked out in a submodule is from the one recorded for it. */
export interface Distance {
  /** The commits it has checked out that the recorded one has not. */
  ahead: number;
  /** The commits the recorded one has that it has not checked out. */
  behind: number;
}

/** What is checked out in a submodule that is initialized. */
export interface SubmoduleCheckout {
  /** The commit; null before its first. */
  commit: string | null;
  /** The branch, by its short name; null on a detached HEAD. */
  branch: string | null;
  /** How many of its files are not committed. */
  uncommitted: number;
  /** Null when that cannot be told: the recorded commit is not in it, which a fetch in it would bring. */
  distance: Distance | null;
}

/** A submodule of the repository, or of one of its submodules. */
export interface Submodule {
  /** Its name in `.gitmodules`, which is usually its folder. */
  name: string;
  /** Its folder from the top of the repository shown, with forward slashes. */
  folder: string;
  /** The folder of the submodule it is in, from the same top; null for one of the repository itself. */
  parent: string | null;
  url: string | null;
  /** The branch `.gitmodules` says it follows, which an update from its remote goes to. */
  branch: string | null;
  /** The commit the repository that has it records for it in HEAD. Null when it is added but not committed yet. */
  recorded: string | null;
  /** The commit staged for it, when that is not the one recorded. */
  staged: string | null;
  /** What is checked out in it; null when it is not initialized: its folder has no repository. */
  checkout: SubmoduleCheckout | null;
}

/** What one read of a repository found (`repo.read-state`). */
export interface RepositoryState {
  /** The top folder of the working tree that was read. */
  folder: string;
  /** The folder of git's files for this worktree. */
  gitFolder: string;
  head: Head;
  branches: Branch[];
  remoteBranches: RemoteBranch[];
  tags: Tag[];
  /** The newest first. */
  stashes: Stash[];
  /** The main one first. */
  worktrees: Worktree[];
  /** The names of the remotes. */
  remotes: string[];
  identity: Identity;
  /** How git's configuration signs commits (`commit.read-signing`, 1.13). */
  signing: Signing;
  /** Git's `branch.sort`, when it is set. */
  branchSort: string | null;
  /** The subjects of the last entries of the reflog of `HEAD`, newest first, for the branches checked out last. */
  reflogSubjects: string[];
  workingTree: WorkingTree;
  operation: Operation | null;
  /** Set while a rebase waits for the user to edit a commit, or to split one. */
  rebaseStop: RebaseStop | null;
  bisect: Bisect | null;
  /** The submodules and theirs, each after the one it is in. */
  submodules: Submodule[];
}

/** Who made a commit, or committed it, and when. */
export interface Signature {
  name: string;
  email: string;
  date: number;
}

/** The details of a commit, of a stash or of the uncommitted changes (`details.read-commit`). */
export interface CommitDetail {
  /** Its full id; null for the uncommitted changes. */
  id: string | null;
  /** The first one first. */
  parents: string[];
  /** Null for the uncommitted changes. */
  author: Signature | null;
  committer: Signature | null;
  /** Its message, its subject and its body, without the line break at its end. */
  message: string;
  /**
   * The files it changed against its first parent, every file it added when it has none. For the uncommitted changes,
   * those of the index and of the working tree together, the untracked ones added and those in conflict modified.
   */
  files: FileChange[];
  /** For a stash made with its untracked files, those files, which its third parent holds. */
  untrackedFiles: FileChange[];
}

/** The letter git gives each kind of change, which the views show. */
export const CHANGE_LETTERS: Record<ChangeKind, string> = {
  added: 'A',
  modified: 'M',
  deleted: 'D',
  renamed: 'R',
  copied: 'C',
  typeChanged: 'T',
};

/** `main` for `refs/heads/main`: the short name of a branch named in full, or the name as it is. */
export const shortBranch = (name: string): string => name.replace(/^refs\/heads\//, '');

/** The short name of the branch checked out, or null when HEAD is detached. */
export const checkedOutBranch = (state: { head: Head }): string | null => (state.head.branch === null ? null : shortBranch(state.head.branch));

/** The short name of the branch checked out in a worktree, or null when it is detached. */
export const worktreeBranch = (worktree: Worktree): string | null => (worktree.branch === null ? null : shortBranch(worktree.branch));

/** `origin/main` for `refs/remotes/origin/main`, `main` for a local branch followed: the upstream as git names it short. */
export const upstreamName = (upstream: Upstream): string => upstream.reference.replace(/^refs\/(remotes|heads)\//, '');

/** The name of the branch an upstream is on its remote, `main`. */
export const upstreamBranch = (upstream: Upstream): string => shortBranch(upstream.remoteBranch);

/** `stash@{0}`: how git names a stash. */
export const stashName = (stash: Pick<Stash, 'index'>): string => `stash@{${stash.index}}`;

/**
 * The changes not committed as the details of the row above the commits list them, each file once, in the shape of the
 * details of a commit with no id and no author: the files in conflict first, then what the index has, what the working
 * tree has and the files git does not track. A file in conflict is modified and an untracked one added, as the engine
 * says them in `details.read-commit`; the views mark them `U` and `?` from the working tree. The engine's own details of
 * the uncommitted changes list them by path, which is not the order of the view, so they are made from the working tree.
 */
export function uncommittedDetail(working: WorkingTree): CommitDetail {
  const files = new Map<string, FileChange>();
  for (const file of working.conflicted) files.set(file.path, { path: file.path, originalPath: null, kind: 'modified' });
  for (const file of [...working.staged, ...working.unstaged]) if (!files.has(file.path)) files.set(file.path, file);
  for (const path of working.untracked) if (!files.has(path)) files.set(path, { path, originalPath: null, kind: 'added' });
  return { id: null, parents: [], author: null, committer: null, message: 'Uncommitted changes', files: [...files.values()], untrackedFiles: [] };
}

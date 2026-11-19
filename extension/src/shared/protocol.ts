import type { ImagePair } from './images';
import type { SyntaxTheme } from '../flow/diff/protocol';
import type { FileIconTheme } from './fileIcons';
import type { Hosting, HostingService } from './hosting';
import type { RangeDiffPairs, RangeDiffTarget } from './rangeDiff';
import type { RepositoryRow, ShownParts } from './repositories';
import type { TagMarks } from './tagMarks';
import type { Choice, ConflictRef, ConflictReview } from './resolution';
import type { CommitOptions } from './commitOptions';
import type { CommitDetail, CommitSignature, FileChange, RepositoryState } from './state';
import type { SetupPatterns, SetupPreview, SetupTarget, WorktreeSetupInfo } from './worktreeSetup';

// The state and the details of a commit are the engine's shapes, from stage 6.
export type * from './state';

/** Pseudo-hash of the row that stands for uncommitted changes. */
export const WORKTREE = 'WORKTREE';

export interface GraphLine {
  from: number;
  to: number;
  color: number;
  /** A line that is not history: from the uncommitted changes or a stash down to the commit they are made on. */
  dashed?: boolean;
}

/** What to draw in the graph cell of one row: `top` runs from the row's top edge to its middle, `bottom` from the middle down. */
export interface GraphRow {
  col: number;
  color: number;
  top: GraphLine[];
  bottom: GraphLine[];
}

export interface Commit {
  hash: string;
  parents: string[];
  author: string;
  email: string;
  date: number;
  subject: string;
}

export interface Row extends Commit {
  graph: GraphRow;
  /** The name of the stash the row is, `stash@{0}`, when it is one rather than a commit of the history. */
  stash?: string;
}

/**
 * Narrows the graph to some commits. A path or lines filter keeps the graph connected, because git can skip over
 * the commits in between; the search filters return unrelated commits, shown as a plain list.
 */
export type Filter =
  /**
   * `renamedFrom`: the names the file had before, which the extension finds and adds to the state it sends back.
   * `folder`: the path is of a folder, whose history is that of everything in it, with no renames followed.
   */
  | { kind: 'path'; path: string; renamedFrom?: string[]; folder?: true }
  /**
   * Some lines of a file, `start` to `end` from 1, as `commit` has them: git follows them back from there, through
   * moves too. `editor`: the same lines as the editor numbers them, when the file there is not as committed.
   */
  | { kind: 'lines'; path: string; start: number; end: number; commit: string; editor?: { start: number; end: number } }
  /** Only what is reachable from one branch, tag or commit. `lost` when it is a commit that no branch or tag reaches any more. */
  | { kind: 'ref'; ref: string; lost?: boolean }
  /** `message` and `author` match text; `pickaxe` finds commits that add or remove the text; `regex` matches changed lines. */
  | { kind: 'message' | 'author' | 'pickaxe' | 'regex'; query: string };

/** The kinds of references whose histories the graph can show: local branches, remote branches and tags. */
export type RefKind = 'branch' | 'remote' | 'tag';

/** What the graph can show of a commit beside its message. */
export type GraphColumn = 'author' | 'date' | 'hash';

/** Every one of them, in the order they are shown. */
export const GRAPH_COLUMNS: GraphColumn[] = ['author', 'date', 'hash'];

/** How branches and tags are listed: by name, or the one committed to last first. */
export type BranchSort = 'name' | 'date';

/**
 * Which histories the graph shows, when not those of every branch, remote branch and tag: of what is checked out, of
 * some kinds of references, or of the references chosen one by one, by their full names (`refs/heads/main`). It is
 * apart from the filter, which narrows what is left of it. `upstream` with what is checked out shows its upstream too,
 * to see what is ahead of it and what is behind.
 */
export type Scope = { kind: 'current'; upstream?: true } | { kind: 'kinds'; kinds: RefKind[] } | { kind: 'refs'; refs: string[] };

/** What the graph waits for: the commits of another filter, scope or repository, or the ones after the last that is loaded. */
export type Loading = 'filter' | 'more';

/** A search that git runs, in the whole history rather than in the commits that are loaded. */
export type Search = Extract<Filter, { query: string }>;

/** One of the repositories of the workspace. */
export interface RepoInfo {
  root: string;
  name: string;
  /**
   * How far under the repository of its group the list of the repositories puts it: 0 for a repository, 1 for one of
   * its worktrees or submodules, 2 for a submodule of one of those (`repo.group-tabs`).
   */
  depth: number;
}

export interface PullRequest {
  number: number;
  title: string;
  url: string;
  draft: boolean;
  /** Combined result of the checks on its last commit, or null when there are none. */
  checks: 'success' | 'failure' | 'pending' | null;
  /** The hosting service it is on, which says how it is named and numbered: `!12` on GitLab; GitHub when not given. */
  service?: HostingService;
}

/**
 * Why a conflict needs no choice between its two sides: they changed lines next to each other (`separate`), they made
 * the same change (`same`), they differ in spaces or line endings only (`whitespace`), or only one of them changed
 * the lines after all (`oneSide`).
 */
export type Resolution = 'separate' | 'same' | 'whitespace' | 'oneSide';

/** A file in conflict whose text has the markers of git: how many conflicts it has, and how the ones that need no choice resolve. */
export interface ConflictFile {
  path: string;
  conflicts: number;
  resolvable: Resolution[];
}

/**
 * What the views show of a repository but its commits: the state as `repo.read-state` of Gitardo's engine reads it, or
 * as `Repo` reads it in its shape when the engine is not there, with the files in conflict whose conflicts the rules
 * resolve. The commits of the graph are read after it, from what it says.
 */
export interface RepoParts extends RepositoryState {
  /** The files in conflict that have the markers of git in them, with what can be resolved of each without a choice. */
  conflictFiles: ConflictFile[];
}

/**
 * What the views are sent of a repository: its state as the engine reads it (`RepositoryState`, field for field), and
 * what the extension adds to it: the rows of the graph, what it is narrowed to, the settings that the views follow,
 * the pull requests, the repositories of the workspace and what Undo takes back.
 */
export interface RepoState extends RepoParts {
  /** The repository, as the extension knows it: the folder of `git rev-parse --show-toplevel`. */
  root: string;
  name: string;
  rows: Row[];
  hasMore: boolean;
  /** How many files are not committed, each once. */
  changes: number;
  filter: Filter | null;
  /** The branches the graph is narrowed to, without the ones that are no longer there. Null when it shows them all. */
  scope: Scope | null;
  /** The branches and remote branches the user marked as favorites, by their full names, in the order they were marked. */
  favorites: string[];
  /** What the Pull button runs, which the settings choose. The menu beside it has every way to pull. */
  defaultPull: PullAction;
  /** Every repository found in the workspace, when there is more than this one to choose from. */
  repos: RepoInfo[];
  /** The hosting service of the main remote, as Gitardo's engine finds it (`hosting.detect-remote`); null on none known. */
  hosting: Hosting | null;
  /** Open pull requests, or the merge requests of a GitLab, by the name of their branch. Empty until the user signs in. */
  pulls: Record<string, PullRequest>;
  /** Whether this VS Code has the Agents window, where a worktree can be opened for agent sessions. */
  agentsWindow: boolean;
  /**
   * The newest operation **Undo** would take back, as the journal of Gitardo's engine keeps it, and whether it can: null
   * when Undo goes by the reflog, or the engine is not there.
   */
  undo: { title: string; undoable: boolean } | null;
  /** Where new worktrees go, as the settings say: the ones in it are named by their path from there. */
  worktreesRoot: string | null;
  /** What a double click on a branch does, as the settings say: check it out here, or open its worktree. */
  openBranches: 'checkout' | 'worktree';
  /** Whether the graph shows the stashes, each above the commit it was made on, as the settings say. */
  showStashes: boolean;
  /** Whether the history of the commit selected in the graph stands out, as the settings say. */
  traceSelected: boolean;
  /** Whether the graph follows only the first parent of each merge, as the settings say. */
  firstParent: boolean;
  /** Whether every commit of the graph is on one line, without lanes, its merges rings, as the settings say. */
  oneLane: boolean;
  /** What the graph shows of each commit beside its message, as the settings say. */
  graphColumns: GraphColumn[];
  /** Whether the graph names the days, weeks and months further back on a line before their first commit, as the settings say. */
  dateSeparators: boolean;
  /** Whether a branch and its upstream of the same name share one label while they are at the same commit, as the settings say. */
  combineUpstream: boolean;
  /**
   * Whether the references, the commits and the details of the Graph view are each a rounded island on a darker
   * ground, as the settings say. The graph in the side bar keeps its look.
   */
  islands: boolean;
  /**
   * What the references list, beside the graph and in the Branches view, as their own filter says, apart from the
   * branches shown (`refs.list-filter`): the choice kept for this repository, without the references gone since. Null
   * when they list everything.
   */
  listed: Scope | null;
  /**
   * How the branches, remote branches and tags are listed everywhere, as the settings say: by name, or the latest first.
   * `gitardo.branches.sortOrder`; `branchSort` is git's own setting, as the engine reads it.
   */
  sortOrder: BranchSort;
  /** Whether a language model can be asked how to resolve the conflicts, as the settings say. */
  resolveWithAI: boolean;
  /**
   * The repository of the workspace the views go back to, while they show a worktree of it that is not in the
   * workspace. Null when they show a repository of the workspace.
   */
  home: RepoInfo | null;
  /**
   * Whether each repository keeps what the views showed of it while another is shown, as `gitardo.repositories.keepOpen`
   * says (`repo.keep-open`).
   */
  keepOpen: boolean;
  /** Whether **Add Signed-off-by** starts ticked for each commit, as `gitardo.commit.signOff` says. */
  signOff: boolean;
  /** What the last **Check tags on `<remote>`** said of the tags, as Gitardo's operations changed it; null before it (`tag.remote-marks`). */
  tagMarks: TagMarks | null;
}

/** The commits of what is checked out, for the view that lists them apart from the graph. */
export interface BranchLog {
  /** The repository they are of. */
  root: string;
  /** Newest first. */
  commits: Commit[];
  hasMore: boolean;
  /** The ones among them that are not pushed. Null where there is no remote to push to. */
  unpushed: string[] | null;
  /** What they are not in: the upstream of the branch, by its short name. Null when it is every remote branch. */
  upstream: string | null;
}

export type SyncAction =
  | 'fetch'
  | 'pull'
  | 'pullRebase'
  /** The two ways to pull, with the changes that are not committed set aside first and put back after. */
  | 'pullAutostash'
  | 'pullRebaseAutostash'
  | 'push'
  | 'pushForceWithLease'
  | 'pushForce'
  /** Fetch, the pull of the settings, and the push when the branch is ahead, in one click: only with the engine. */
  | 'sync';

/** The ways to pull. */
export type PullAction = Extract<SyncAction, `pull${string}`>;

/**
 * The ways to merge: as the settings of git say, which is a fast-forward when one is possible; always with a merge
 * commit; only as a fast-forward; or as changes staged for the user to commit, with no merge recorded.
 */
export type MergeMode = 'default' | 'noFastForward' | 'fastForwardOnly' | 'squash';

/** The files that differ between two commits, going from `from` to `to`. */
export interface Comparison {
  from: string;
  to: string;
  files: FileChange[];
}

/** Two commits to compare, with the names to show for them. */
export interface Comparing {
  from: string;
  to: string;
  fromName: string;
  toName: string;
}

/** One side of a comparison: a commit, or the working tree (`WORKTREE`), and the branch, tag or short hash to call it by. */
export interface ComparedRef {
  name: string;
  hash: string;
}

/**
 * How the tab of a comparison compares: `diverged` shows what `to` changed since it left `from`, from the commit the two
 * have in common, as a pull request does (`git diff from...to`); `direct` everything that differs (`git diff from to`).
 */
export type CompareMode = 'diverged' | 'direct';

/** One side of a comparison in its tab. */
export interface CompareSide {
  name: string;
  hash: string;
  /** Named by a branch, a tag or a revision: its commit is looked up again when the repository changes. */
  live: boolean;
  /** The name no longer stands for a commit, a branch that was deleted: the side stays where it was. */
  gone?: boolean;
}

/** A commit that one side of a comparison has and the other has not. */
export interface CompareCommit {
  hash: string;
  subject: string;
  author: string;
  date: number;
  side: 'from' | 'to';
  /** The other side has the same change in a commit of its own: it was cherry-picked, or rebased. */
  equivalent: boolean;
}

/** What the tab of a comparison shows, but the lines of its files. */
export interface CompareView {
  from: CompareSide;
  to: CompareSide;
  mode: CompareMode;
  /** The commit the two have in common, null when their histories never met. */
  base: string | null;
  /** The commit the files are compared from: `base` when `diverged`, `from` when `direct` or when there is no base. */
  since: string;
  /** The commits `to` has that `from` has not, and the other way round. */
  ahead: number;
  behind: number;
  /** The commits each side has that the other has not, newest first: no more than a few thousand of them. */
  commits: CompareCommit[];
  /** Whether the commits were matched with those of the other side, which is not done when there are too many. */
  matched: boolean;
  /**
   * The files that would conflict if `to` were merged into `from`. Null when nothing would be merged with a merge
   * commit, when a side is the working tree, or when git cannot tell.
   */
  conflicts: string[] | null;
  /** The file or the folder the comparison is narrowed to, from the top of the repository. */
  path: string | null;
  files: FileChange[];
}

/**
 * What a view shows. `graph` is everything, in the panel or in an editor tab. `changes`, `commits`, `sideGraph`,
 * `branches` and `stashes` are the views of the side bar: `commits` lists the commits of what is checked out, each
 * with its files, apart from the graph, and `sideGraph` is the graph without the references beside it. `commit` is
 * one commit in an editor tab, with its message, its files and what it changed in them. `compare` is two branches,
 * tags or commits compared in an editor tab. `worktreeSetup` is the tab where what new worktrees get of what git
 * ignores is chosen. `conflicts` is the tab where what is proposed for the conflicts is looked at before it is written.
 * `repositories` is the view of the side bar that lists the repositories of the workspace, with their worktrees and
 * their submodules under them and what each is on: it is sent its rows, and no state.
 */
export type ViewKind =
  | 'graph'
  | 'changes'
  | 'commits'
  | 'sideGraph'
  | 'branches'
  | 'stashes'
  | 'commit'
  | 'compare'
  | 'worktreeSetup'
  | 'conflicts'
  | 'repositories'
  | 'rangeDiff';

/** The pairs of the tab of what a rewrite changed: being read, read, or what git said. */
export type RangeDiffRead =
  | { kind: 'comparing' }
  | { kind: 'read'; target: RangeDiffTarget; pairs: RangeDiffPairs }
  | { kind: 'failed'; target: RangeDiffTarget; message: string };

/** The views that show the graph, and so are sent its commits. */
export const showsGraph = (kind: ViewKind) => kind === 'graph' || kind === 'sideGraph';

/** How the files of a commit are laid out in its editor tab: under their directories, or one after the other. */
export type FilesLayout = 'tree' | 'list';

/** One `@@` block of a diff. `header` is the `@@ -a,b +c,d @@` line, `lines` the context, `+` and `-` lines after it. */
export interface Hunk {
  header: string;
  lines: string[];
}

/** The diff of one file, cut up so that each hunk can be applied on its own: `prelude + header + lines` is a valid patch. */
export interface FileHunks {
  prelude: string;
  hunks: Hunk[];
  binary: boolean;
}

/** What a commit changed in one of its files. */
export interface FilePatch {
  file: FileChange;
  /** Lines added and removed: both 0 for a binary file, whose lines are not counted. */
  added: number;
  removed: number;
  binary: boolean;
  /** Null when the commit changes more lines than are sent to a view: the diff editor shows those. */
  hunks: Hunk[] | null;
}

/** What an interactive rebase does with a commit. `edit` stops right after making it, for the user to change it. */
export type RebaseAction = 'pick' | 'reword' | 'edit' | 'squash' | 'fixup' | 'drop';

export interface RebaseStep {
  action: RebaseAction;
  hash: string;
  /** The new message of a reworded commit. */
  message?: string;
  /** Set when `message` is only a new subject: the body the commit has stays under it. */
  keepBody?: boolean;
}

/** Which two versions of a file a diff compares. */
export type DiffSource =
  | { kind: 'commit'; hash: string; parent: string | null }
  /** HEAD against the working tree. */
  | { kind: 'worktree' }
  /** HEAD against the index. */
  | { kind: 'staged' }
  /** The index against the working tree. */
  | { kind: 'unstaged' }
  /** A file of the Changes view, as VS Code's list of changes opens it: `unstaged` when it has unstaged changes, else `staged`. */
  | { kind: 'change' }
  | { kind: 'conflict' };

/** Something the user asked for in the UI. The extension asks for any missing input, then turns it into git commands. */
export type Action =
  /**
   * A remote branch is checked out in the local branch of its name. One there already with commits of its own has them
   * rebased onto the remote branch with `rebase`; without it, the user is asked.
   */
  | { kind: 'checkout'; target: 'branch' | 'remote' | 'detached'; ref: string; rebase?: boolean }
  | { kind: 'createBranch'; at: string }
  | { kind: 'renameBranch'; name: string }
  | { kind: 'deleteBranch'; name: string }
  /** Moves a branch that is not checked out up to its upstream, which is fetched first. */
  | { kind: 'fastForward'; name: string }
  /**
   * What the last rewrite of a branch or the last force push of a remote branch changed, commit by commit, or with
   * `withUpstream` a branch against its upstream (`range-diff.compare`).
   */
  | { kind: 'rangeDiff'; target: 'branch' | 'remote'; name: string; withUpstream?: boolean }
  | { kind: 'deleteRemoteBranch'; name: string }
  /** As the settings of git say when `mode` is left out; `ask` asks which way. */
  | { kind: 'merge'; ref: string; mode?: MergeMode | 'ask' }
  /** Rebases `branch`, or the current branch when it is left out. */
  | { kind: 'rebase'; onto: string; branch?: string }
  /** `steps` are oldest first; `base` is the commit they sit on, or null when they start at the root. */
  | { kind: 'rebaseInteractive'; base: string | null; steps: RebaseStep[] }
  /**
   * Gives a commit a new message: the last one by amending it, an older one by an interactive rebase that makes the
   * commits after it again.
   */
  | { kind: 'reword'; hash: string; message: string }
  /**
   * Makes one commit of several that follow one another on the line before HEAD, by an interactive rebase that makes
   * the commits after them again. The message is asked for.
   */
  | { kind: 'squash'; hashes: string[] }
  /**
   * Takes the changes of a commit out of it into the working tree, to commit them again in several commits: the last
   * one by a reset, an older one by an interactive rebase that stops there and makes the commits after it again on
   * Continue.
   */
  | { kind: 'split'; hash: string }
  | { kind: 'undo' }
  /** On an existing branch or at a commit, or, without `ref`, on a new branch from a base asked for. */
  | { kind: 'worktreeAdd'; ref?: string; isBranch?: boolean }
  /**
   * In a new window, or in the Agents window of VS Code, which starts agent sessions in it. Without `where`, where
   * `gitardo.worktrees.openIn` says.
   */
  | { kind: 'worktreeOpen'; path: string; where?: 'window' | 'agents' }
  /** Opens the worktree of a branch, made for it first in the folder of the worktrees when it has none. */
  | { kind: 'openInWorktree'; name: string; remote?: boolean }
  /** Asks whether to delete its branch too, unless `deleteBranch` says. */
  | { kind: 'worktreeRemove'; path: string; deleteBranch?: boolean }
  | { kind: 'worktreeRename' | 'worktreeTerminal' | 'worktreeReveal'; path: string }
  | { kind: 'worktreePrune' }
  /** Locks a worktree with the reason asked, or unlocks it (`worktree.lock`, `worktree.unlock`). */
  | { kind: 'worktreeLock' | 'worktreeUnlock'; path: string }
  /** Brings into the worktree in `path`, or into every other worktree, what new worktrees get of what git ignores here. */
  | { kind: 'worktreeSetUp'; path?: string }
  /** Asks for the URL, the folder and the branch of a new submodule. */
  | { kind: 'submoduleAdd' }
  /**
   * Checks out in a submodule, and in the ones inside it, the commit that is recorded for it, cloning it first when it
   * is not initialized. With `remote`, the last commit of the branch it follows instead.
   */
  | { kind: 'submoduleUpdate'; path: string; remote?: boolean }
  /** Stages the commit checked out in a submodule as the one recorded for it. */
  | { kind: 'submoduleRecord'; path: string }
  /** Removes the folder of a submodule and forgets it locally, keeping it in `.gitmodules` and in the commits. */
  | { kind: 'submoduleDeinit'; path: string }
  /** Removes a submodule from the repository: its folder, its entry in `.gitmodules` and its clone in the git directory. */
  | { kind: 'submoduleRemove'; path: string }
  /** Copies the URLs of `.gitmodules` to the configuration of the submodules, after they changed there. */
  | { kind: 'submoduleSync' }
  /** Opens the folder of a submodule in a new window. */
  | { kind: 'submoduleOpen'; path: string }
  | { kind: 'cherryPick'; hash: string; isMerge: boolean }
  /** Several commits in one go, in any order: they are applied oldest first. */
  | { kind: 'cherryPickMany'; hashes: string[] }
  /** Asks for a patch, from a file or from the clipboard, and whether to make its commits or only to apply its changes. */
  | { kind: 'applyPatch' }
  | { kind: 'revert'; hash: string; isMerge: boolean }
  | { kind: 'reset'; hash: string }
  | { kind: 'createTag'; at: string }
  | { kind: 'deleteTag'; name: string }
  | { kind: 'deleteTagOnRemote'; name: string; here: boolean }
  | { kind: 'setUpstream'; name: string }
  | { kind: 'unsetUpstream'; name: string }
  | { kind: 'createBranchWithNoHistory' }
  | { kind: 'pushTag'; name: string }
  /** **Push tags…**: the tags the remote lacks, ticked in a pick (`tag.push-all`). */
  | { kind: 'pushTags' }
  /** **Check tags on `<remote>`**: asks the remote of `remote.default` for its tags and marks those it lacks (`tag.remote-marks`). */
  | { kind: 'checkTags' }
  /** Writes the note of a commit, proposing the one it has, or removes it (`note.edit`, `note.remove`). */
  | { kind: 'editNote' | 'removeNote'; hash: string }
  /** Fetches the notes of a remote, or pushes the notes to it (`note.fetch`, `note.push`). */
  | { kind: 'fetchNotes' | 'pushNotes'; remote: string }
  /** Saves the files of a commit, a branch, a remote branch or a tag as a zip or a tar.gz: `at` is a hash or a full name (`archive.save`). */
  | { kind: 'saveArchive'; at: string }
  /** Cleans up the repository shown with `git gc` (`upkeep.clean-up`). */
  | { kind: 'cleanUp' }
  /** Has git look after the repository shown in the background, or stop (`upkeep.start-background`, `upkeep.stop-background`). */
  | { kind: 'lookAfter'; start: boolean }
  /** Everything, or only these files, or only what is staged. */
  | { kind: 'stashPush'; paths?: string[]; staged?: boolean }
  | { kind: 'stashApply' | 'stashPop' | 'stashDrop' | 'stashBranch'; ref: string }
  | { kind: 'stage' | 'unstage' | 'discard'; paths: string[] }
  /**
   * Has git ignore a file or a folder by a pattern asked for, takes it out of the index while it stays on disk, or hides
   * the changes of a tracked file from git (`changes.ignore`, `changes.stop-tracking`, `changes.skip`); `path` is from
   * the top of the repository.
   */
  | { kind: 'ignore' | 'stopTracking' | 'hideFromGit'; path: string }
  /** Shows the changes of a file hidden from git to git again, or of every file hidden when `path` is left out (`changes.unskip`). */
  | { kind: 'showToGit'; path?: string }
  /** Resolves the conflicts that need no choice between the two sides, in these files or in all of them. */
  | { kind: 'resolveConflicts'; paths?: string[] }
  /** Makes a file in conflict what one side has, whole: `ours` is what is checked out, `theirs` what is coming in. */
  | { kind: 'takeSide'; path: string; side: 'ours' | 'theirs' }
  | { kind: 'stageAll' | 'unstageAll' | 'discardAll' }
  /**
   * Takes files back to the last commit, staged or not, and deletes the untracked ones: all of them without `paths`.
   * `discard` and `discardAll` leave what is staged alone.
   */
  | { kind: 'discardChanges'; paths?: string[] }
  /** With the options of the commit box, as they were ticked for this commit (`commit.options`). */
  | { kind: 'commit'; message: string; amend: boolean; options?: CommitOptions; start?: CommitOptions }
  /** Folds the staged changes, or all changes when nothing is staged, into an earlier commit. `isRoot`: that commit has no parent. */
  | { kind: 'fixup'; hash: string; isRoot: boolean }
  /** Marks a commit for `git bisect`, starting one if needed. Without `hash` it marks the commit being tested. */
  | { kind: 'bisect'; mark: 'good' | 'bad' | 'skip'; hash?: string }
  | { kind: 'bisectReset' }
  | { kind: 'continue' | 'skip' | 'abort' };

/** What every view and every command says when there is no engine to read or to run anything (D49). */
export const NO_ENGINE = "Gitardo's engine is not available on this system.";

export type HostMessage =
  /**
   * With `kept` the first `kept` rows are the ones sent before, and are not sent again: `state.rows` only has the rows
   * after them, which are none when the commits are the same, and before those the row of the uncommitted changes, if
   * there is one, because the number of changes it tells may be another. Without a state, `engineMissing` says that
   * there is no engine to read one with (D49).
   */
  | { type: 'state'; state: RepoState | null; kept?: number; engineMissing?: boolean }
  /** The commits of what is checked out, for the view that asked. */
  | { type: 'log'; log: BranchLog }
  | { type: 'detail'; hash: string; detail: CommitDetail | null }
  /** What git says of the signature of a commit shown, read after its details; null when it could not be read. */
  | { type: 'signature'; hash: string; signature: CommitSignature | null }
  /** The co-authors chosen in the quick pick of **Co-authors…**, each `Name <email>`. */
  | { type: 'coAuthors'; coAuthors: string[] }
  /** What a commit changed, file by file in the order of its details. Null when git and the list of files disagree. */
  | { type: 'patch'; hash: string; patch: FilePatch[] | null }
  | { type: 'comparison'; comparison: Comparison }
  /** What the tab of a comparison shows. Null, with `error`, when git could not compare the two. */
  | { type: 'compareView'; view: CompareView | null; error?: string }
  /**
   * The lines of the files of the comparison from `since` to `to`, in the order of its files. Null when git and the
   * list of files disagree.
   */
  | { type: 'comparePatch'; since: string; to: string; patch: FilePatch[] | null }
  /**
   * The two versions of an image file of the page of a commit or of a comparison, as `readImages` asked for them; null
   * when neither side is an image (`commit-page.images`).
   */
  | { type: 'images'; source: DiffSource; path: string; images: ImagePair | null }
  /**
   * The colors the theme of VS Code gives the code, once the page asked for them, and again whenever the theme changes.
   * Null when the theme cannot be read: the page then takes those of Dark+ or Light+.
   */
  | { type: 'syntaxTheme'; theme: SyntaxTheme | null }
  /**
   * The icons of the file icon theme of VS Code, once the page asked for them, and again whenever they change. Null when
   * the user chose none, or when it cannot be read: the files are then shown with no icons, as in VS Code.
   */
  | { type: 'fileIcons'; theme: FileIconTheme | null }
  | { type: 'busy'; title: string | null }
  /**
   * The graph is being read again: for a filter, a scope or a repository just chosen (`filter`), or for the commits
   * after the last (`more`). Null once the state it was read for has been sent.
   */
  | { type: 'loading'; what: Loading | null }
  | { type: 'committed' }
  /** The files of the repository shown that have edits not saved in an editor, by their paths from its root. */
  | { type: 'unsaved'; paths: string[] }
  /** Select a commit and scroll to it. */
  | { type: 'reveal'; hash: string }
  /** Bring out the history of a commit in the graph while the pointer rests on its branch in another view; null when it leaves. */
  | { type: 'trace'; hash: string | null }
  /**
   * The commits a search found in the history that is shown, newest first. `more` when there are others after the last,
   * `failed` when git refused the search.
   */
  | { type: 'found'; search: Search; hashes: string[]; more: boolean; failed?: boolean }
  /** A commit message is being drafted, or has been (`message`). */
  | { type: 'drafting'; active: boolean; message?: string }
  /** A message for the commit box to start from when it is empty, such as the one git prepared for a squash merge, or that of a commit being split. */
  | { type: 'commitMessage'; message: string }
  /** The message the commit box had when the repository shown was left, kept while another was shown (`repo.keep-open`). */
  | { type: 'keptMessage'; root: string; message: string }
  /** What the tab of the setup of new worktrees shows. Null, with `error`, when it could not be read. */
  | { type: 'worktreeSetup'; info: WorktreeSetupInfo | null; error?: string }
  /** What the patterns take from the checkout. Null when git could not say. */
  | { type: 'worktreeSetupPreview'; patterns: SetupPatterns; preview: SetupPreview | null; error?: string }
  /** What the tab of the review of the conflicts shows. Null, with `error`, when the files could not be read. */
  | { type: 'review'; review: ConflictReview | null; error?: string }
  /**
   * The rows of the Repositories view (`repo.tabs`), sent to it alone, once the repositories of the workspace were
   * looked for and whenever a row says something else, with what the menus of the rows need of the repository shown.
   * Null, with `engineMissing`, when there is no engine to read them with (D49).
   */
  | { type: 'repositories'; rows: RepositoryRow[] | null; shown?: ShownParts | null; engineMissing?: boolean }
  /** What the tab of what a rewrite changed shows (`range-diff.page`): the pairs while they are read, once read, or why not. */
  | { type: 'rangeDiff'; read: RangeDiffRead };

/** Things the UI can ask for that need the extension to prompt, pick or open something. */
export type UiCommand =
  | 'reflog'
  /** The output channel, which says why there is no engine. */
  | 'showLog'
  | 'cleanupBranches'
  | 'activeFileHistory'
  | 'signInGitHub'
  /** `hosting.load-merge-requests` on the engine. */
  | 'signInGitLab'
  /** `hosting.checkout-pull-request` on the engine. */
  | 'checkOutPullRequest'
  | 'draftMessage'
  | 'switchIdentity'
  | 'selectRepository'
  | 'remotes'
  | 'goToCommit'
  | 'switchWorktree'
  | 'worktreeSetup';

export type WebviewMessage =
  | { type: 'ready' }
  /** The page colors code: it is sent the colors of the theme, now and whenever they change. */
  | { type: 'syntaxTheme' }
  /** The page shows files: it is sent the icons of the file icon theme, now and whenever they change. */
  | { type: 'fileIcons' }
  /** A view with a search box has the keyboard, or no longer: while it does, the key that searches is its own. */
  | { type: 'focus'; focused: boolean }
  | { type: 'refresh' }
  | { type: 'loadMore' }
  /** Asks for the commits of what is checked out, that many at most, and which of them are pushed. */
  | { type: 'getLog'; limit: number }
  | { type: 'getDetail'; hash: string }
  | { type: 'getPatch'; hash: string }
  /** Asks what git says of the signature of a commit shown; a read asked before it is cancelled. */
  | { type: 'getSignature'; hash: string }
  /** Opens the quick pick of the co-authors of the next commit, those chosen already ticked. */
  | { type: 'pickCoAuthors'; chosen: string[] }
  /** Opens a commit in an editor tab of its own, or shows the tab it already has. */
  | { type: 'openCommit'; hash: string }
  /** Selects a commit in the graph, opening the graph for it. */
  | { type: 'reveal'; hash: string }
  /** The layout the tabs of the commits open with from now on. */
  | { type: 'setFilesLayout'; layout: FilesLayout }
  | { type: 'getComparison'; from: string; to: string }
  /** Asks the user what to compare, or what to compare `from` with, and opens the comparison in a tab. */
  | { type: 'compare'; from?: ComparedRef }
  /** Opens the comparison of these two in a tab, or shows the tab it has. */
  | { type: 'openCompare'; from: ComparedRef; to: ComparedRef }
  /** In the tab of a comparison: asks for the user to choose one of its sides again. */
  | { type: 'pickCompareSide'; side: 'from' | 'to' }
  | { type: 'swapCompare' }
  | { type: 'setCompareMode'; mode: CompareMode }
  /** Narrows the comparison of the tab to a file or a folder, or no longer when null. */
  | { type: 'setComparePath'; path: string | null }
  /** Writes the version of a file that the second side of the comparison has in the working tree and the index. */
  | { type: 'takeCompareFile'; file: FileChange }
  /** Copies the changes of the comparison as a patch. */
  | { type: 'copyComparePatch' }
  /** Copies these commits as one patch that `git am` takes, oldest first, or with `save` saves it to a file the user chooses. */
  | { type: 'formatPatch'; hashes: string[]; save?: boolean }
  /** Opens the comparison on the hosting service of the main remote (`compare.open-on-service`). */
  | { type: 'openCompareOnService' }
  /** Selects a commit in the views that list commits and are open, without opening one. */
  | { type: 'show'; hash: string }
  /** The pointer came to a branch, with its commit, whose history the views of the graph bring out; or left it, with null. */
  | { type: 'trace'; hash: string | null }
  | { type: 'selectRepo'; root: string }
  /**
   * The commit box was pointed at another repository: the message it had in the one left, to keep, and the one shown,
   * whose message it asks for (`repo.keep-open`).
   */
  | { type: 'keepMessage'; root: string; message: string; shown: string }
  | { type: 'openDiff'; source: DiffSource; file: FileChange }
  /** Reads the two versions of an image file of the page of a commit or of a comparison, to draw them (`commit-page.images`). */
  | { type: 'readImages'; source: DiffSource; file: FileChange }
  /**
   * Asks what to compare a file listed in a commit, in a comparison or in the changes not committed with, a branch,
   * tag, commit or the working tree, and opens the two versions in the diff editor. `names` are the branches or tags
   * the two commits of the source stand for, when they are not to be called by their short hashes.
   */
  | { type: 'compareFile'; source: DiffSource; file: FileChange; names?: { hash?: string; parent?: string } }
  /** Opens the file as it is in the working tree; one that is deleted opens its diff instead. */
  | { type: 'openFile'; file: FileChange }
  /** A click on a file of the Changes view: it opens the file or its diff against HEAD, as `gitardo.changes.openOnClick` says. */
  | { type: 'openChange'; file: FileChange }
  /** Opens the diffs of all these files in one editor. */
  | { type: 'openChanges'; source: DiffSource; files: FileChange[]; title: string }
  /** Saves the edits of these files, by their paths from the root of the repository, that are not saved in an editor. */
  | { type: 'save'; paths: string[] }
  /** Opens what an editor has of the file, not saved yet, against the file on disk. */
  | { type: 'compareWithSaved'; path: string }
  | { type: 'sync'; action: SyncAction }
  | { type: 'action'; action: Action }
  | { type: 'setFilter'; filter: Filter | null }
  | { type: 'setScope'; scope: Scope | null }
  /** What the references list, chosen in their own filter: null for everything (`refs.list-filter`). */
  | { type: 'setListed'; listed: Scope | null }
  /** Shows the stashes in the graph, or no longer. */
  | { type: 'setShowStashes'; show: boolean }
  | { type: 'setTraceSelected'; trace: boolean }
  | { type: 'setFirstParent'; only: boolean }
  | { type: 'setOneLane'; one: boolean }
  | { type: 'setGraphColumns'; columns: GraphColumn[] }
  | { type: 'setDateSeparators'; show: boolean }
  | { type: 'setCombineUpstream'; combine: boolean }
  | { type: 'setBranchSort'; sort: BranchSort }
  | { type: 'setFavorite'; ref: string; favorite: boolean }
  /** Selects the commit a hash or another revision stands for, loading the history as far back as it is. */
  | { type: 'goTo'; revision: string }
  /** Asks for the commits a search finds, to highlight them in the graph rather than replace it with them. */
  | { type: 'find'; search: Search }
  | { type: 'command'; command: UiCommand }
  /** Opens the pull request of a branch, or the page to create one. */
  | { type: 'pullRequest'; branch: string }
  /** Checks out the pull request of this number that a remote branch has on its label (`hosting.checkout-pull-request`). */
  | { type: 'checkOutPullRequest'; number: number; remoteBranch: string }
  /** Asks for what the tab of the setup of new worktrees shows, looking again at what git ignores when `rescan`. */
  | { type: 'getWorktreeSetup'; rescan?: boolean }
  | { type: 'previewWorktreeSetup'; patterns: SetupPatterns }
  /**
   * Saves the patterns there, with the commands run in new worktrees when given, and in VS Code's settings too when
   * `alsoVSCode`; null takes Gitardo's out of there, the commands too.
   */
  | { type: 'saveWorktreeSetup'; target: SetupTarget; patterns: SetupPatterns | null; alsoVSCode?: boolean; commands?: string[] }
  /** Opens the file of the settings there. */
  | { type: 'editWorktreeSetup'; target: SetupTarget }
  /**
   * Opens the tab of the review of the conflicts, of these files or of all of them, or shows the one open. With `ai`
   * the language model is asked about the conflicts the rules leave.
   */
  | { type: 'reviewConflicts'; paths?: string[]; ai?: boolean }
  | { type: 'getReview' }
  /** Writes the proposals accepted in the review. */
  | { type: 'writeResolutions'; accepted: ConflictRef[] }
  /** Shows each file in conflict beside what the proposals accepted would make of it, in a multi-file diff editor. */
  | { type: 'previewResolutions'; accepted: ConflictRef[] }
  /** Puts what the user chose in place of what is proposed for a conflict, or takes the choice back with null. */
  | { type: 'chooseResolution'; conflict: ConflictRef; choice: Choice | null }
  /** Asks the language model about these conflicts, or about all the ones with nothing proposed. */
  | { type: 'askAI'; conflicts?: ConflictRef[] }
  | { type: 'stopAsking' }
  /** Asks which language model resolves conflicts, and how much it thinks. */
  | { type: 'chooseConflictModel' }
  /** Opens a file in conflict in the Flow Merge, or in the merge editor, at the line of a conflict. */
  | { type: 'openConflict'; path: string; line: number; editor: 'flowMerge' | 'mergeEditor' | 'text' }
  | { type: 'copy'; text: string };

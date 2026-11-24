import { WORKTREE, type RepoState, type Row, type SentState, type StatePart, type ViewKind } from './protocol';

/**
 * The parts of the state sent to the views (D88), and how a view puts the parts it is sent in place of the ones it has,
 * so that what did not change keeps its identity and is not worked out or drawn again.
 */

/** The fields of each part but the rows, the draft and the rest, which is every field none of these names. */
const FIELDS = {
  repository: ['root', 'name', 'folder', 'gitFolder', 'head', 'remotes', 'bisect', 'notes', 'combineUpstream'],
  refs: ['branches', 'remoteBranches', 'tags', 'favorites', 'listed', 'sortOrder', 'branchSort', 'reflogSubjects', 'tagMarks'],
  stashes: ['stashes'],
  worktrees: ['worktrees'],
  workingTree: ['workingTree', 'changes', 'conflictFiles', 'skipped'],
  submodules: ['submodules'],
  pulls: ['pulls', 'hosting'],
} as const satisfies Partial<Record<StatePart, readonly (keyof RepoState)[]>>;

type Named = keyof typeof FIELDS;

/** The fields that are not of the rest: those of the named parts, and the rows with what is said of them. */
const NOT_REST = new Set<string>([...Object.values(FIELDS).flat(), 'rows', 'hasMore', 'draft', 'commits']);

/** Every part, the order they are sent in. */
export const ALL_PARTS: readonly StatePart[] = ['repository', 'rest', 'refs', 'stashes', 'worktrees', 'workingTree', 'submodules', 'pulls', 'draft', 'rows'];

/** What the labels of a commit need, which a tab of a commit or of a comparison is sent and no more. */
const LABELS: readonly StatePart[] = ['repository', 'refs', 'stashes', 'worktrees', 'pulls'];

/**
 * The parts each kind of view shows. The tabs of a commit and of a comparison show the labels of their commits; the
 * tabs of the conflicts, of a range, of what new worktrees get and of the large files, and the Repositories view, read what they show
 * themselves, and are sent no state.
 */
const PARTS: Record<ViewKind, readonly StatePart[]> = {
  graph: ALL_PARTS,
  sideGraph: ALL_PARTS,
  changes: ['repository', 'rest', 'refs', 'worktrees', 'workingTree', 'submodules'],
  commits: ['repository', 'rest', 'refs', 'worktrees', 'pulls'],
  branches: ['repository', 'rest', 'refs', 'stashes', 'worktrees', 'submodules', 'pulls'],
  stashes: ['repository', 'rest', 'stashes'],
  commit: LABELS,
  compare: LABELS,
  conflicts: [],
  rangeDiff: [],
  worktreeSetup: [],
  largeFiles: [],
  repositories: [],
};

/** The parts of the state a kind of view is sent. */
export const partsOf = (kind: ViewKind): readonly StatePart[] => PARTS[kind];

/** The row of the uncommitted changes the state has above its commits, or null. */
export const draftOf = (rows: readonly Row[]): Row | null => (rows[0]?.hash === WORKTREE ? rows[0] : null);

/** The commits of the rows, without the row of the uncommitted changes. */
export const commitsOf = (rows: Row[]): Row[] => (rows[0]?.hash === WORKTREE ? rows.slice(1) : rows);

/** The fields of one part of a state, as a message carries them; the rows whole, without the draft. */
export function partOf(state: RepoState, part: StatePart): SentState {
  const fields: Record<string, unknown> = {};
  switch (part) {
    case 'rows':
      return { rows: commitsOf(state.rows), hasMore: state.hasMore };
    case 'draft':
      return { draft: draftOf(state.rows) };
    case 'rest':
      for (const [field, value] of Object.entries(state)) if (!NOT_REST.has(field)) fields[field] = value;
      return fields as SentState;
    default:
      for (const field of FIELDS[part]) if (field in state) fields[field] = state[field];
      return fields as SentState;
  }
}

/** The parts of a state, all of those named, in one object. */
export function partsIn(state: RepoState, parts: readonly StatePart[]): SentState {
  return Object.assign({}, ...parts.map((part) => partOf(state, part))) as SentState;
}

/**
 * The state as a view keeps it: the parts it was sent, and, for what was never sent to it, nothing. `rows` has the row
 * of the uncommitted changes first when there is one, as the graph draws them.
 */
export interface ShownState extends RepoState {
  /**
   * The row of the uncommitted changes as it was last sent, with the count of its files. The one first in `rows` is
   * laid out the same, and is not put there again when only its count changed, so that the rows stay the same list.
   */
  draft: Row | null;
  /** The commits of `rows`, without the row of the uncommitted changes. */
  commits: Row[];
}

/** A state with nothing in it, under the parts a view is sent. */
export const NO_STATE: ShownState = {
  folder: '',
  gitFolder: '',
  head: { commit: null, branch: null },
  branches: [],
  remoteBranches: [],
  tags: [],
  stashes: [],
  worktrees: [],
  remotes: [],
  identity: { name: null, email: null },
  signing: { signsEveryCommit: false, kind: 'openpgp', key: null },
  branchSort: null,
  reflogSubjects: [],
  workingTree: { staged: [], unstaged: [], untracked: [], conflicted: [] },
  operation: null,
  rebaseStop: null,
  bisect: null,
  submodules: [],
  conflictFiles: [],
  root: '',
  name: '',
  rows: [],
  hasMore: false,
  changes: 0,
  filter: null,
  scope: null,
  favorites: [],
  defaultPull: 'pullRebaseAutostash',
  repos: [],
  hosting: null,
  pulls: {},
  agentsWindow: false,
  undo: null,
  worktreesRoot: null,
  openBranches: 'checkout',
  showStashes: false,
  traceSelected: false,
  firstParent: false,
  oneLane: false,
  graphColumns: [],
  dateSeparators: false,
  combineUpstream: false,
  islands: false,
  listed: null,
  sortOrder: 'name',
  resolveWithAI: false,
  home: null,
  keepOpen: false,
  signOff: false,
  tagMarks: null,
  draft: null,
  commits: [],
};

/** Whether two rows of the uncommitted changes are drawn the same in the graph, whatever count of files they say. */
function laidOutAlike(one: Row | null, other: Row | null): boolean {
  if (!one || !other) return one === other;
  return one.parents[0] === other.parents[0] && JSON.stringify(one.graph) === JSON.stringify(other.graph);
}

/**
 * The state of a view once it is sent some parts: those take the place of the ones it had, whole, and every other part
 * stays the same object. With `kept`, the commits sent come after the first `kept` it has.
 */
export function withParts(previous: ShownState | null, sent: SentState, parts: readonly StatePart[], kept?: number): ShownState {
  const base = previous ?? NO_STATE;
  const next: Record<string, unknown> = { ...base };
  for (const part of parts) {
    // A field a part does not have any more, such as the notes of a repository that has none, goes with it.
    const gone = part === 'rest' ? Object.keys(next).filter((field) => !NOT_REST.has(field)) : part === 'rows' || part === 'draft' ? [] : FIELDS[part as Named];
    for (const field of gone) delete next[field];
  }
  const { rows: sentRows, draft: sentDraft, ...fields } = sent;
  Object.assign(next, fields);
  let commits = base.commits;
  if (parts.includes('rows') && sentRows) {
    if (kept === undefined) commits = sentRows;
    // The same commits stay the same list, for what is worked out from them.
    else if (kept !== commits.length || sentRows.length) commits = commits.slice(0, kept).concat(sentRows);
  }
  const draft = parts.includes('draft') ? (sentDraft ?? null) : base.draft;
  const sameRows = commits === base.commits && laidOutAlike(draft, draftOf(base.rows));
  next.commits = commits;
  next.draft = draft;
  next.rows = sameRows ? base.rows : draft ? [draft, ...commits] : commits;
  return next as unknown as ShownState;
}

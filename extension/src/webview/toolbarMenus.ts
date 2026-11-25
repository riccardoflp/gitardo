import type { GraphColumn, RefKind, RepoState, UiCommand } from '../shared/protocol';
import { pullWords } from '../shared/hosting';
import { kindsScope, scopeKinds } from '../shared/scope';
import type { MenuEntry } from './ContextMenu';
import { act } from './menus';
import { post } from './vscode';

/** How the folds of the merges stand, and what folds or unfolds them all; no `onFoldAll` where nothing can be folded. */
export interface FoldAll {
  /** Folds every merge, those loaded later too, or shows all their commits again. */
  onFoldAll?(fold: boolean): void;
  /** Whether every merge is folded, but those unfolded one by one, or only some merges are folded now, or none. */
  folded: 'all' | 'some' | 'none';
}

const run = (command: UiCommand) => () => post({ type: 'command', command });

/**
 * The menu of the view of the toolbar (`graph.view-menu`): every switch of the graph, each ticked while it is on, in the
 * groups of Gitardo Desktop: what the graph shows, how it draws it, and what each row shows beside its message.
 */
export function viewMenu(state: RepoState, { onFoldAll, folded }: FoldAll): MenuEntry[] {
  // The kinds of references the graph shows, as the list of branches has them. Not while some are chosen one by one there.
  const kinds = scopeKinds(state.scope);
  const showKind = (kind: RefKind, label: string): MenuEntry => {
    const on = !!kinds?.includes(kind);
    return {
      label,
      checked: on,
      // With no kind left the graph would have nothing to show.
      disabled: !kinds || (on && kinds.length === 1),
      hint: kinds ? undefined : 'Chosen in the filter',
      run: () => kinds && post({ type: 'setScope', scope: kindsScope(on ? kinds.filter((each) => each !== kind) : [...kinds, kind]) }),
    };
  };
  const showColumn = (column: GraphColumn, label: string): MenuEntry => {
    const on = state.graphColumns.includes(column);
    const columns = on ? state.graphColumns.filter((each) => each !== column) : [...state.graphColumns, column];
    return { label, checked: on, run: () => post({ type: 'setGraphColumns', columns }) };
  };
  return [
    showKind('branch', 'Show local branches'),
    showKind('remote', 'Show remote branches'),
    showKind('tag', 'Show tags'),
    { label: 'Show stashes', checked: state.showStashes, run: () => post({ type: 'setShowStashes', show: !state.showStashes }) },
    {
      label: 'Show only the first parent of merges',
      checked: state.firstParent,
      run: () => post({ type: 'setFirstParent', only: !state.firstParent }),
    },
    'separator',
    { label: 'Show all commits on one line', checked: state.oneLane, run: () => post({ type: 'setOneLane', one: !state.oneLane }) },
    // On, the Graph view in the panel has a commit on one line; the one of the side bar keeps two lines either way.
    { label: 'Compact rows', checked: state.compactRows, run: () => post({ type: 'setCompactRows', compact: !state.compactRows }) },
    {
      label: 'Bring out the history of the selected commit',
      checked: state.traceSelected,
      run: () => post({ type: 'setTraceSelected', trace: !state.traceSelected }),
    },
    {
      label: 'Separate the days, weeks and months',
      checked: state.dateSeparators,
      run: () => post({ type: 'setDateSeparators', show: !state.dateSeparators }),
    },
    {
      label: 'Put a branch and its upstream on one label',
      checked: state.combineUpstream,
      run: () => post({ type: 'setCombineUpstream', combine: !state.combineUpstream }),
    },
    // On one line, the merges are not folded from the menu.
    onFoldAll &&
      !state.oneLane && {
        // Checked, it folds the merges loaded later too; unchecked, it shows all their commits again.
        label: 'Fold all merges',
        checked: folded === 'all',
        // A stash keeps only its first parent in the graph: a row with more is a merge.
        disabled: folded !== 'all' && !state.rows.some((row) => row.parents.length > 1),
        run: () => onFoldAll(folded !== 'all'),
      },
    onFoldAll && !state.oneLane && { label: 'Unfold all merges', icon: 'UNFOLD', disabled: folded === 'none', run: () => onFoldAll(false) },
    'separator',
    showColumn('author', 'Show the author'),
    showColumn('date', 'Show the date'),
    showColumn('hash', 'Show the hash'),
  ];
}

/**
 * The **…** menu of the toolbar (`graph.more-menu`): the actions alone, the switches being in the menu of the view.
 * **Refresh** is in it but where the toolbar is `compact`, in the side bar, where the header of the view has it; and
 * **Branch**, **Stash** and **Undo** while the toolbar is too `narrow` for their buttons.
 */
export function moreMenu(state: RepoState, compact: boolean, narrow = false): MenuEntry[] {
  // Asked only where the buttons are in the menu: before the first commit none of the three can be chosen.
  const committed = narrow && !!state.head.commit;
  return [
    // Its button is the first to go when the toolbar is short of room.
    !compact && { label: 'Refresh', icon: 'READ_AGAIN', run: () => post({ type: 'refresh' }) },
    !compact && 'separator',
    narrow && { label: 'Branch', feature: 'branch.create', disabled: !committed, title: 'Create a branch at HEAD', run: () => act({ kind: 'createBranch', at: 'HEAD' }) },
    narrow && {
      label: 'Stash',
      feature: 'stash.push',
      disabled: !committed || !state.changes,
      title: 'git stash push --include-untracked',
      run: () => act({ kind: 'stashPush' }),
    },
    narrow && { label: 'Undo', feature: 'undo.last', disabled: !committed, run: () => act({ kind: 'undo' }) },
    narrow && 'separator',
    { label: 'Compare branches, tags or commits…', feature: 'compare.choose', run: () => post({ type: 'compare' }) },
    { label: 'Go to commit…', feature: 'search.go-to-commit', icon: 'SEARCH', run: run('goToCommit') },
    { label: 'History of the active file', feature: 'file-history.file', run: run('activeFileHistory') },
    { label: 'Browse reflog…', feature: 'reflog.browse', run: run('reflog') },
    { label: 'Clean up branches…', feature: 'branch.clean-up', run: run('cleanupBranches') },
    { label: 'Apply patch…', feature: 'patch.apply', run: () => act({ kind: 'applyPatch' }) },
    { label: 'Manage remotes…', feature: 'remote.manage', run: run('remotes') },
    // With submodules, their section has the same button.
    !state.submodules.length && { label: 'Add submodule…', feature: 'submodule.add', run: () => act({ kind: 'submoduleAdd' }) },
    (state.repos.length > 1 || state.home) && { label: 'Select repository…', icon: 'CHOOSE', run: run('selectRepository') },
    'separator',
    { label: 'Switch identity…', feature: 'identity.switch', run: run('switchIdentity') },
    ...hostingEntries(state),
  ];
}

/**
 * The entries of the hosting service of the main remote: loading the pull requests of GitHub or the merge requests of a
 * GitLab, and checking one out where the service keeps a reference of each (`hosting.checkout-pull-request`).
 */
function hostingEntries(state: RepoState): MenuEntry[] {
  const hosting = state.hosting;
  if (!hosting) return [];
  const service = hosting.service;
  return [
    (service === 'gitHub' || service === 'gitLab' || hosting.canCheckOut) && 'separator',
    service === 'gitHub' && { label: 'Load pull requests from GitHub', feature: 'github.load-pull-requests', run: run('signInGitHub') },
    service === 'gitLab' && { label: 'Load merge requests from GitLab', feature: 'hosting.load-merge-requests', run: run('signInGitLab') },
    hosting.canCheckOut && { label: `Check out a ${pullWords(service)}…`, feature: 'hosting.checkout-pull-request', run: run('checkOutPullRequest') },
  ];
}

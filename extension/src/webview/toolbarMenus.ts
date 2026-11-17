import type { GraphColumn, RefKind, RepoState, UiCommand } from '../shared/protocol';
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
    onFoldAll && !state.oneLane && { label: 'Unfold all merges', disabled: folded === 'none', run: () => onFoldAll(false) },
    'separator',
    showColumn('author', 'Show the author'),
    showColumn('date', 'Show the date'),
    showColumn('hash', 'Show the hash'),
  ];
}

/**
 * The **…** menu of the toolbar (`graph.more-menu`): the actions alone, the switches being in the menu of the view.
 * **Refresh** is in it but where the toolbar is `compact`, in the side bar, where the header of the view has it.
 */
export function moreMenu(state: RepoState, compact: boolean): MenuEntry[] {
  return [
    // Its button is the first to go when the toolbar is short of room.
    !compact && { label: 'Refresh', run: () => post({ type: 'refresh' }) },
    !compact && 'separator',
    { label: 'Compare branches, tags or commits…', run: () => post({ type: 'compare' }) },
    { label: 'Go to commit…', run: run('goToCommit') },
    { label: 'History of the active file', run: run('activeFileHistory') },
    { label: 'Browse reflog…', run: run('reflog') },
    { label: 'Clean up branches…', run: run('cleanupBranches') },
    { label: 'Apply patch…', run: () => act({ kind: 'applyPatch' }) },
    { label: 'Manage remotes…', run: run('remotes') },
    // With submodules, their section has the same button.
    !state.submodules.length && { label: 'Add submodule…', run: () => act({ kind: 'submoduleAdd' }) },
    (state.repos.length > 1 || state.home) && { label: 'Select repository…', run: run('selectRepository') },
    'separator',
    { label: 'Switch identity…', run: run('switchIdentity') },
    state.github && 'separator',
    state.github && { label: 'Load pull requests from GitHub', run: run('signInGitHub') },
  ];
}

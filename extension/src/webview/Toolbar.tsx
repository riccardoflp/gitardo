import { useEffect, useRef, useState, type MouseEvent } from 'react';
import type { PullAction, RepoState, Search, SyncAction } from '../shared/protocol';
import { hasDiverged } from '../shared/rangeDiff';
import { type MenuEntry, useMenu } from './ContextMenu';
import { Icon } from './Icon';
import { GIVE, given, useGive } from './give';
import { RepoSelect } from './RepoSelect';
import { ScopeButton } from './ScopeButton';
import { WorktreeSelect } from './WorktreeSelect';
import { act, rewriteEntries } from './menus';
import { useKept } from './kept';
import { moreMenu, viewMenu } from './toolbarMenus';
import { loadUiState, post, saveUiState } from './vscode';

interface Props {
  state: RepoState;
  busy: string | null;
  /** Only the search and the menu: in the side bar the header of the view has the buttons, and the room is for the commits. */
  compact?: boolean;
  /**
   * The commits the search matches: how many, which of them is selected, counting from 0, and whether there are more
   * than were counted. Null when nothing is searched for.
   */
  matches: { count: number; index: number | null; more: boolean } | null;
  /** Git is looking for them. */
  searching: boolean;
  /** Whether the references beside the commits are put away, and what the button that says so does. */
  refsHidden?: boolean;
  onToggleRefs?(): void;
  /** Highlights the loaded commits that contain the text. */
  onQuery(query: string): void;
  /** Folds every merge into one row, those loaded later too, or shows all their commits again; missing where nothing can be folded. */
  onFoldAll?(fold: boolean): void;
  /** Whether every merge is folded, but those unfolded one by one, or only some merges are folded now, or none. */
  folded?: 'all' | 'some' | 'none';
  /** A search of the whole history is asked for, or again to go on from its results; null when it no longer holds. */
  onSearch(search: Search | null, by: 1 | -1): void;
  /** To the match after the selected one, or to the one before it. */
  onStep(by: 1 | -1): void;
}

const PULL_VARIANTS: { action: PullAction; label: string; command: string; feature: string }[] = [
  { action: 'pull', label: 'Pull', command: 'git pull', feature: 'sync.pull' },
  { action: 'pullRebase', label: 'Pull (rebase)', command: 'git pull --rebase', feature: 'sync.pull-with' },
  { action: 'pullAutostash', label: 'Pull (autostash)', command: 'git pull --autostash', feature: 'sync.pull-with' },
  { action: 'pullRebaseAutostash', label: 'Pull (rebase, autostash)', command: 'git pull --rebase --autostash', feature: 'sync.pull-with' },
];

const PUSH_VARIANTS: { action: SyncAction; label: string; feature: string }[] = [
  { action: 'push', label: 'Push', feature: 'sync.push' },
  { action: 'pushForceWithLease', label: 'Push (force with lease)', feature: 'sync.push-force-with-lease' },
  { action: 'pushForce', label: 'Push (force)', feature: 'sync.push-force' },
  // Fetch, pull and push in one click, as Gitardo's desktop has it (stage 4, area 2).
  { action: 'sync', label: 'Sync: fetch, pull, push', feature: 'sync.sync' },
];

/** `loaded` highlights matches among the commits on screen; the others ask git to search the whole history. */
type SearchMode = 'loaded' | Search['kind'];

/** What may be the hash of a commit, or the start of one. */
const HASH = /^[0-9a-f]{4,64}$/i;

const SEARCH_MODES: Record<SearchMode, { label: string; placeholder: string; title: string }> = {
  loaded: {
    label: 'Loaded',
    placeholder: 'Find by message, author or hash',
    title: 'Highlight matches among the loaded commits. Enter on a hash goes to that commit, however far back it is',
  },
  message: { label: 'Message', placeholder: 'Text in the message, then Enter', title: 'git log --grep: search every commit message' },
  author: { label: 'Author', placeholder: 'Author name or email, then Enter', title: 'git log --author: search every commit author' },
  pickaxe: {
    label: 'Changes',
    placeholder: 'Text added or removed, then Enter',
    title: 'git log -S: commits that add or remove this text',
  },
  regex: {
    label: 'Changes (regex)',
    placeholder: 'Regex on changed lines, then Enter',
    title: 'git log -G: commits whose changed lines match this POSIX extended regular expression ([0-9], not \\d)',
  },
};

export function Toolbar({ state, busy, compact, matches, searching, refsHidden, onToggleRefs, onFoldAll, folded = 'none', onQuery, onSearch, onStep }: Props) {
  const [mode, setMode] = useState<SearchMode>('loaded');
  const [text, setText] = useState('');
  // Whether a search of the whole history replaces the graph with what it finds, rather than highlight it there.
  const [only, setOnly] = useState(() => loadUiState({ searchOnly: false }).searchOnly);
  const input = useRef<HTMLInputElement>(null);
  const bar = useRef<HTMLDivElement>(null);
  useGive(bar);
  // Coming back to a repository that kept what it showed, its search is in the box again.
  useKept(state.root, state.keepOpen, 'search', { text, mode }, (kept) => {
    setText(kept.text);
    setMode(kept.mode);
    onQuery(kept.mode === 'loaded' ? kept.text : '');
  });
  const openMenu = useMenu();
  const upstream = state.branches.find((branch) => branch.isCheckedOut)?.upstream;
  const sync = (action: SyncAction) => post({ type: 'sync', action });
  // What the button runs, among the ways to pull: the settings choose it.
  const pull = PULL_VARIANTS.find(({ action }) => action === state.defaultPull) ?? PULL_VARIANTS[0];
  /** A menu of the ways to pull or push, which says which one the button beside it runs. */
  // The branch checked out against its upstream, commit by commit, while each has commits the other has not (M36).
  const checkedOut = state.branches.find((branch) => branch.isCheckedOut);
  const compare = checkedOut && hasDiverged(checkedOut) ? rewriteEntries(checkedOut).slice(1) : [];
  const variants = (list: typeof PUSH_VARIANTS, button: SyncAction) => (event: MouseEvent) =>
    openMenu(event, [
      ...list.map(({ action, label, feature }) => ({ label, feature, hint: action === button ? 'default' : undefined, run: () => sync(action) })),
      ...(button === 'push' && compare.length ? (['separator', ...compare] as MenuEntry[]) : []),
    ]);

  const typed = text.trim();
  // Not among the loaded commits, it may still be the hash of one further back, or of one no branch reaches.
  const lookup = mode === 'loaded' && matches?.count === 0 && HASH.test(typed);

  const changeText = (value: string) => {
    setText(value);
    if (mode === 'loaded') onQuery(value);
    else onSearch(null, 1);
  };
  const changeMode = (next: SearchMode) => {
    setMode(next);
    onQuery(next === 'loaded' ? text : '');
    onSearch(null, 1);
  };
  const submit = (by: 1 | -1) => {
    if (mode === 'loaded') {
      if (matches?.count) onStep(by);
      else if (lookup) post({ type: 'goTo', revision: typed.toLowerCase() });
    } else if (typed) {
      if (only) post({ type: 'setFilter', filter: { kind: mode, query: typed } });
      else onSearch({ kind: mode, query: typed }, by);
    }
  };
  /** Shows the search in the box the other way: as a list of what it finds, or highlighted in the graph. */
  const toggleOnly = () => {
    if (mode === 'loaded') return;
    saveUiState({ searchOnly: !only });
    setOnly(!only);
    if (only) {
      // The list gives way to the graph when it is searched; with nothing to search, it is left here.
      if (typed) onSearch({ kind: mode, query: typed }, 1);
      else if (state.filter && 'query' in state.filter) post({ type: 'setFilter', filter: null });
    } else {
      onSearch(null, 1);
      if (typed) post({ type: 'setFilter', filter: { kind: mode, query: typed } });
    }
  };

  // The keys of a search work wherever the focus is in the view.
  const onKey = useRef<(event: KeyboardEvent) => void>(() => {});
  onKey.current = (event) => {
    if (event.key === 'F3') {
      event.preventDefault();
      submit(event.shiftKey ? -1 : 1);
    } else if ((event.ctrlKey || event.metaKey) && !event.altKey && !event.shiftKey && event.key.toLowerCase() === 'f') {
      event.preventDefault();
      input.current?.focus();
      input.current?.select();
    }
  };
  useEffect(() => {
    const listener = (event: KeyboardEvent) => onKey.current(event);
    // The extension is told while the view has the keyboard, to keep the key that searches from the editor behind it.
    const focus = () => post({ type: 'focus', focused: true });
    const blur = () => post({ type: 'focus', focused: false });
    window.addEventListener('keydown', listener);
    window.addEventListener('focus', focus);
    window.addEventListener('blur', blur);
    if (document.hasFocus()) focus();
    return () => {
      window.removeEventListener('keydown', listener);
      window.removeEventListener('focus', focus);
      window.removeEventListener('blur', blur);
    };
  }, []);

  /** A click on a button of the search leaves the focus in the box, where Enter and Shift+Enter go on from it. */
  const keepFocus = (event: MouseEvent) => event.preventDefault();

  const search = (
    <div className="search">
      <Icon name="search" />
      <button
        className="mode"
        aria-haspopup="menu"
        aria-label={`Where to search: ${SEARCH_MODES[mode].label}`}
        title={SEARCH_MODES[mode].title}
        onClick={(event) =>
          openMenu(
            event,
            (Object.keys(SEARCH_MODES) as SearchMode[]).map((key) => ({
              label: SEARCH_MODES[key].label,
              title: SEARCH_MODES[key].title,
              checked: key === mode,
              run: () => changeMode(key),
            })),
          )
        }
      >
        <span className="name">{SEARCH_MODES[mode].label}</span>
        <Icon name="chevron-down" />
      </button>
      <input
        ref={input}
        type="text"
        value={text}
        aria-label="Search commits"
        placeholder={SEARCH_MODES[mode].placeholder}
        onChange={(event) => changeText(event.target.value)}
        onKeyDown={(event) => {
          if (event.key === 'Enter') submit(event.shiftKey ? -1 : 1);
          else if (event.key === 'Escape' && text) changeText('');
        }}
      />
      {searching && <Icon name="loading" spin />}
      {lookup ? (
        <span className="hint" title="Press Enter to go to the commit with this hash, wherever it is in the history">
          Enter ↵
        </span>
      ) : (
        matches && (
          <span className="count" role="status" title={matches.more ? 'Only the first matches were counted' : undefined}>
            {matches.index !== null && `${matches.index + 1}/`}
            {matches.count}
            {matches.more && '+'}
          </span>
        )
      )}
      {!!matches?.count && (
        <>
          <button
            className="step"
            onMouseDown={keepFocus}
            onClick={() => onStep(-1)}
            aria-label="Previous match"
            title="Previous match (Shift+Enter, Shift+F3)"
          >
            <Icon name="arrow-up" />
          </button>
          <button className="step" onMouseDown={keepFocus} onClick={() => onStep(1)} aria-label="Next match" title="Next match (Enter, F3)">
            <Icon name="arrow-down" />
          </button>
        </>
      )}
      {mode !== 'loaded' && (
        <button
          className="only"
          onMouseDown={keepFocus}
          onClick={toggleOnly}
          aria-pressed={only}
          aria-label="Show only the commits found"
          title="Show only the commits found, as a list, instead of highlighting them in the graph"
        >
          <Icon name="filter" />
        </button>
      )}
    </div>
  );
  // The switches of the graph and the actions are two menus side by side, as in Gitardo Desktop (D68).
  const menuButtons = (
    <>
      <button className="button icon" onClick={(event) => openMenu(event, viewMenu(state, { onFoldAll, folded }))} aria-label="View" title="View">
        <Icon name="settings" />
      </button>
      <button className="button icon" onClick={(event) => openMenu(event, moreMenu(state, !!compact, given(bar.current) >= GIVE.actions))} aria-label="More actions" title="More actions">
        <Icon name="ellipsis" />
      </button>
    </>
  );
  const working = busy && (
    <span className="busy" title={`${busy}…`}>
      <Icon name="loading" spin />
      <span className="label">{busy}…</span>
    </span>
  );

  if (compact) {
    return (
      <div className="toolbar compact">
        {search}
        <ScopeButton state={state} compact />
        {working}
        {menuButtons}
      </div>
    );
  }

  return (
    <div className="toolbar" ref={bar}>
      <div className="line first">
        <div className="group where">
          {onToggleRefs && (
            <button
              className={`button icon refs-toggle${refsHidden ? '' : ' on'}`}
              aria-pressed={!refsHidden}
              aria-label={refsHidden ? 'Show the branches' : 'Hide the branches'}
              title={refsHidden ? 'Show the branches, tags and stashes beside the commits' : 'Hide the branches, tags and stashes beside the commits'}
              onClick={onToggleRefs}
            >
              <Icon name={refsHidden ? 'layout-sidebar-left-off' : 'layout-sidebar-left'} />
            </button>
          )}
          <RepoSelect state={state} disabled={!!busy} />
          <WorktreeSelect state={state} disabled={!!busy} />
        </div>

        {/* The words are hidden when the toolbar is short of room, so each button also says what it is in its tooltip. */}
        <div className="group actions">
          <button className="button" disabled={!!busy} onClick={() => sync('fetch')} title="Fetch: git fetch --all --prune" aria-label="Fetch">
            <Icon name="repo-fetch" />
            <span className="label">Fetch</span>
          </button>
          <div className="split">
            <button className="button" disabled={!!busy} onClick={() => sync(pull.action)} title={`Pull: ${pull.command}`} aria-label="Pull">
              <Icon name="repo-pull" />
              <span className="label">Pull</span>
              {!!upstream?.behind && <span className="count">{upstream.behind}</span>}
            </button>
            <button className="button" disabled={!!busy} onClick={variants(PULL_VARIANTS, pull.action)} aria-label="More pull options">
              <Icon name="chevron-down" />
            </button>
          </div>
          <div className="split">
            <button className="button" disabled={!!busy} onClick={() => sync('push')} title="Push: git push" aria-label="Push">
              <Icon name="repo-push" />
              <span className="label">Push</span>
              {!!upstream?.ahead && <span className="count">{upstream.ahead}</span>}
            </button>
            <button className="button" disabled={!!busy} onClick={variants(PUSH_VARIANTS, 'push')} aria-label="More push options">
              <Icon name="chevron-down" />
            </button>
          </div>

          <button
            className="button later"
            disabled={!!busy || !state.head.commit}
            onClick={() => act({ kind: 'createBranch', at: 'HEAD' })}
            title="Branch: create a branch at HEAD"
            aria-label="Branch"
          >
            <Icon name="git-branch" />
            <span className="label">Branch</span>
          </button>
          <button
            className="button later"
            disabled={!!busy || !state.changes || !state.head.commit}
            onClick={() => act({ kind: 'stashPush' })}
            title="Stash: git stash push --include-untracked"
            aria-label="Stash"
          >
            <Icon name="git-stash" />
            <span className="label">Stash</span>
          </button>

          <button
            className="button later"
            disabled={!!busy || !state.head.commit}
            onClick={() => act({ kind: 'undo' })}
            title={
              state.undo
                ? state.undo.undoable
                  ? `Undo "${state.undo.title}"`
                  : `"${state.undo.title}" cannot be undone`
                : 'Undo the last commit, checkout, merge, rebase, reset, branch deletion or stash drop'
            }
            aria-label="Undo"
          >
            <Icon name="discard" />
            <span className="label">Undo</span>
          </button>

          {working}
        </div>
      </div>

      <span className="spacer" />

      <div className="line">
        <ScopeButton state={state} />
        {search}
        <button className="button icon refresh" onClick={() => post({ type: 'refresh' })} aria-label="Refresh" title="Refresh">
          <Icon name="refresh" />
        </button>
        {menuButtons}
      </div>
    </div>
  );
}

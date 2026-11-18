import { type KeyboardEvent, useEffect, useMemo, useState } from 'react';
import { samePath } from '../shared/paths';
import { type DrawnRow, type RepositoryRow, type ShownParts, drawnRows } from '../shared/repositories';
import { summarySpoken, summaryWords } from '../shared/summaries';
import { useMenu } from './ContextMenu';
import { Icon } from './Icon';
import { repositoryRowMenu } from './menus';
import { type Loaded, Placeholder } from './Placeholder';
import { loadUiState, onMessage, post, saveUiState } from './vscode';

/** The icon of a row: a repository, a worktree or a submodule. */
const ICONS = { repository: 'repo', worktree: 'worktree', submodule: 'file-submodule' } as const;

/** The width of an icon and a gap: the room of the chevron, and how far a row is indented under the one it belongs to. */
const INDENT = 20;

/** What a row is on, on its right: the branch, the files changed in the color of changes, the commits to pull and to push. */
function Summary({ row }: { row: DrawnRow }) {
  // Until a worktree is read, its row says the branch git listed for it.
  if (!row.summary) return row.listed ? <span className="on">{row.listed}</span> : null;
  const words = summaryWords(row.summary);
  return (
    <span className="on">
      <span className="head">{words.head}</span>
      {words.changed && <span className="changed">{words.changed}</span>}
      {words.distance && <span className="distance">{words.distance}</span>}
    </span>
  );
}

/** What a row says to a screen reader: what it is, its name, what it is on, and that an operation runs in it. */
function spoken(row: DrawnRow): string {
  const on = row.summary ? summarySpoken(row.summary) : (row.listed ?? '');
  return [`${row.kind} ${row.name}`, on, row.isRunning ? 'an operation is running' : ''].filter(Boolean).join(', ');
}

/**
 * The Repositories view (`repo.tabs`): a row for each repository of the workspace, the rows of its worktrees and its
 * submodules under it, folded and unfolded by the chevron at its left (`repo.group-tabs`), what each is on at its right
 * (`repo.worktree-summary`), and the one Gitardo shows marked. A click on a row shows it in every view (`repo.select`),
 * and a right click opens its menu.
 */
export function RepositoriesApp() {
  const [rows, setRows] = useState<RepositoryRow[] | null>(null);
  const [shown, setShown] = useState<ShownParts | null>(null);
  const [loaded, setLoaded] = useState<Loaded>(false);
  const [folded, setFolded] = useState(() => loadUiState({ foldedRepositories: [] as string[] }).foldedRepositories);

  useEffect(() => {
    const stop = onMessage((message) => {
      if (message.type !== 'repositories') return;
      setRows(message.rows);
      setShown(message.shown ?? null);
      setLoaded(message.engineMissing ? 'noEngine' : true);
    });
    post({ type: 'ready' });
    return stop;
  }, []);

  const openMenu = useMenu();
  const drawn = useMemo(() => drawnRows(rows ?? [], folded), [rows, folded]);
  if (!rows?.length) return <Placeholder loaded={loaded} />;

  // Nothing is chosen while an operation runs, as in the list of the toolbar.
  const busy = rows.some((row) => row.isRunning);
  const hasChevrons = drawn.some((row) => row.hasRows);
  const fold = (row: DrawnRow, closed: boolean) => {
    if (!row.hasRows || row.isFolded === closed) return;
    const next = closed ? [...folded, row.group] : folded.filter((each) => !samePath(each, row.group));
    setFolded(next);
    saveUiState({ foldedRepositories: next });
  };
  const show = (row: DrawnRow) => {
    if (!busy && !row.isShown) post({ type: 'selectRepo', root: row.folder });
  };
  // The arrows move between the rows and fold them, as in a tree of VS Code; Enter and Space are a click.
  const onKey = (event: KeyboardEvent<HTMLDivElement>, row: DrawnRow) => {
    if (event.target !== event.currentTarget) return;
    const beside = (step: -1 | 1) => {
      const all = [...(event.currentTarget.parentElement?.querySelectorAll<HTMLElement>('.repository') ?? [])];
      all[all.indexOf(event.currentTarget) + step]?.focus();
    };
    if (event.key === 'Enter' || event.key === ' ') show(row);
    else if (event.key === 'ArrowDown') beside(1);
    else if (event.key === 'ArrowUp') beside(-1);
    else if (event.key === 'ArrowLeft') fold(row, true);
    else if (event.key === 'ArrowRight') fold(row, false);
    else return;
    event.preventDefault();
  };

  return (
    <div className="side repositories" role="tree" aria-label="Repositories">
      {drawn.map((row) => (
        <div
          key={`${row.group}\n${row.kind === 'repository' ? '' : row.folder}`}
          className={['repository', row.isShown && 'shown', row.isStale && 'stale', busy && 'busy'].filter(Boolean).join(' ')}
          role="treeitem"
          tabIndex={0}
          aria-level={row.depth + 1}
          aria-selected={row.isShown}
          aria-expanded={row.hasRows ? !row.isFolded : undefined}
          aria-label={spoken(row)}
          aria-disabled={busy || undefined}
          title={row.folder}
          style={{ paddingLeft: 8 + (row.depth > 0 ? (row.depth + 1) * INDENT : 0) }}
          onClick={() => show(row)}
          onContextMenu={(event) => openMenu(event, repositoryRowMenu(row, shown))}
          onKeyDown={(event) => onKey(event, row)}
        >
          {hasChevrons &&
            row.depth === 0 &&
            (row.hasRows ? (
              <button
                className="fold"
                aria-label={`${row.isFolded ? 'Unfold' : 'Fold'} ${row.repository}`}
                aria-expanded={!row.isFolded}
                onClick={(event) => {
                  event.stopPropagation();
                  fold(row, !row.isFolded);
                }}
              >
                <Icon name={row.isFolded ? 'chevron-right' : 'chevron-down'} />
              </button>
            ) : (
              <span className="fold" />
            ))}
          <Icon name={ICONS[row.kind]} />
          <span className="name">
            <bdi>{row.name}</bdi>
          </span>
          <Summary row={row} />
          {row.isRunning && (
            <span className="running" role="img" aria-label="An operation is running" title="An operation is running">
              <Icon name="loading" spin />
            </span>
          )}
        </div>
      ))}
    </div>
  );
}

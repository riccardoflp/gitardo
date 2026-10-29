import { Fragment, type ReactNode } from 'react';
import { WORKTREE, type Filter, type RepoState, type Scope } from '../shared/protocol';
import { scopeRefName } from '../shared/scope';
import { Icon } from './Icon';
import { post } from './vscode';

/** No more names than this are spelled out: the rest are counted. */
const NAMED = 3;

/** "line 4", "lines 4–9". */
const lines = ({ start, end }: { start: number; end: number }) => (start === end ? `line ${start}` : `lines ${start}–${end}`);

function describe(filter: Filter, head: string | null): ReactNode {
  if (filter.kind === 'lines') {
    return (
      <>
        History of <b>{lines(filter)}</b> of <b>{filter.path}</b>
        {filter.editor && <span className="muted"> · {lines(filter.editor)} in the editor</span>}
        {filter.commit !== head && <span className="muted"> · as committed in {filter.commit.slice(0, 7)}</span>}
      </>
    );
  }
  if (filter.kind === 'path') {
    if (filter.folder) {
      return (
        <>
          History of the folder <b>{filter.path}</b>
        </>
      );
    }
    return (
      <>
        History of <b>{filter.path}</b>
        {filter.renamedFrom?.length ? <span className="muted"> · earlier {filter.renamedFrom.join(', ')}</span> : null}
      </>
    );
  }
  if (filter.kind === 'ref') {
    if (filter.lost) {
      return (
        <>
          Only the history of <b className="mono">{filter.ref.slice(0, 7)}</b>
          <span className="muted"> · on no branch or tag</span>
        </>
      );
    }
    return (
      <>
        Only the history of <b>{scopeRefName(filter.ref)}</b>
      </>
    );
  }
  const what = {
    message: 'Commits whose message contains',
    author: 'Commits by authors matching',
    pickaxe: 'Commits that add or remove',
    regex: 'Commits whose changed lines match',
  }[filter.kind];
  return (
    <>
      {what} <b>{filter.query}</b>
    </>
  );
}

const KIND_NAMES = { branch: 'the local branches', remote: 'the remote branches', tag: 'the tags' };

/** The branches a scope is made of: "the current branch", "the local branches and the tags", "main, feature and 2 more". */
function branches(scope: Scope): ReactNode {
  if (scope.kind === 'current') {
    return scope.upstream ? (
      <>
        <b>the current branch</b> and <b>its upstream</b>
      </>
    ) : (
      <b>the current branch</b>
    );
  }
  if (scope.kind === 'kinds') {
    return scope.kinds.map((kind, index) => (
      <Fragment key={kind}>
        {index > 0 && ' and '}
        <b>{KIND_NAMES[kind]}</b>
      </Fragment>
    ));
  }
  const names = scope.refs.map(scopeRefName);
  return (
    <>
      {names.slice(0, NAMED).map((name, index) => (
        <Fragment key={name}>
          {index > 0 && ', '}
          <b>{name}</b>
        </Fragment>
      ))}
      {names.length > NAMED && ` and ${names.length - NAMED} more`}
    </>
  );
}

/**
 * Shown while the graph is narrowed: to the history of some branches, to that of a file, a folder or some lines, or to
 * the results of a search.
 */
export function FilterBar({ state }: { state: RepoState }) {
  const { filter } = state;
  // The history of a commit that no branch reaches is not narrowed to branches, nor are lines, followed from one commit.
  const scope = filter?.kind === 'ref' || filter?.kind === 'lines' ? null : state.scope;
  if (!filter && !scope) return null;
  const count = state.rows.filter((row) => row.hash !== WORKTREE).length;
  return (
    <div className="filterbar" role="status">
      <Icon name={filter ? 'filter' : 'list-filter'} />
      <span className="grow">
        {filter ? describe(filter, state.head) : <>Only the history of {branches(scope!)}</>}
        {filter && scope && <span className="muted"> · in {branches(scope)}</span>}
        <span className="muted">
          {' · '}
          {count}
          {state.hasMore ? '+' : ''} {count === 1 ? 'commit' : 'commits'}
        </span>
      </span>
      {/* One thing at a time: what was narrowed last is what is undone first. */}
      {filter ? (
        <button className="button" onClick={() => post({ type: 'setFilter', filter: null })}>
          <Icon name="close" />
          Show all commits
        </button>
      ) : (
        <button className="button" onClick={() => post({ type: 'setScope', scope: null })}>
          <Icon name="close" />
          Show all branches
        </button>
      )}
    </div>
  );
}

import { useEffect, useLayoutEffect, useRef, useState } from 'react';
import type { BranchLog, Commit, CommitDetail, DiffSource, RepoState } from '../shared/protocol';
import { checkedOutBranch, upstreamName } from '../shared/state';
import { useMenu } from './ContextMenu';
import { OVERSCAN } from './CommitList';
import { ActionButtons, FileRow } from './FileRow';
import { Icon } from './Icon';
import { type Loaded, Placeholder, loadedOf } from './Placeholder';
import { logMenu } from './menus';
import { fullDate, relativeTime, shortHash } from './util';
import { receive } from './received';
import { onMessage, post } from './vscode';

/** How many commits are asked for at first, and how many more each time. */
const PAGE = 100;
/** How high the row of a commit is, as `.file` sets it; one unfolded is as high as it was measured. */
const ROW = 22;

/** What the unfolded commits changed, by the hash of each. Null while it is being read. */
type Open = Record<string, CommitDetail | null>;

/**
 * What tells that the commits may be others, or pushed since: what is checked out, what it is compared with, and where
 * the remote branches are.
 */
const logKey = (state: RepoState) =>
  [
    state.root,
    state.head.commit,
    checkedOutBranch(state),
    state.branches.find((branch) => branch.isCheckedOut)?.upstream?.reference,
    ...state.remoteBranches.map((branch) => `${branch.name} ${branch.commit}`),
  ].join('\n');

const count = (number: number, more: boolean) => `${number}${more ? '+' : ''} ${number === 1 && !more ? 'commit' : 'commits'}`;

/** What the line above the commits says: how many are left to push, how many to pull, and against what. */
function summary(state: RepoState, log: BranchLog): string {
  if (!log.unpushed) return 'No remote to push to';
  const upstream = state.branches.find((branch) => branch.isCheckedOut)?.upstream;
  // As many as were asked for: there may be others further back.
  const waiting = count(log.unpushed.length, log.hasMore && log.unpushed.length === log.commits.length);
  if (!log.upstream) {
    const branch = checkedOutBranch(state);
    const why = branch ? `${branch} has no upstream` : 'no branch is checked out';
    return log.unpushed.length ? `${waiting} on no remote branch: ${why}` : `Everything is on a remote branch: ${why}`;
  }
  const behind = upstream && !upstream.isGone && upstream.behind ? `, ${count(upstream.behind, false)} to pull` : '';
  return log.unpushed.length ? `${waiting} to push to ${log.upstream}${behind}` : `Everything is pushed to ${log.upstream}${behind}`;
}

/**
 * The commits of what is checked out, in the side bar, apart from the graph: nothing chosen there narrows them. Each
 * says whether it is pushed, and unfolds to the files it changed, which open as diffs.
 */
export function CommitsApp() {
  const [state, setState] = useState<RepoState | null>(null);
  const [loaded, setLoaded] = useState<Loaded>(false);
  const [log, setLog] = useState<BranchLog | null>(null);
  const [limit, setLimit] = useState(PAGE);
  const [open, setOpen] = useState<Open>({});
  const openRef = useRef(open);
  openRef.current = open;
  const rootRef = useRef<string | null>(null);
  const openMenu = useMenu();
  // Only the commits in sight are drawn, as in the graph: with thousands loaded, scrolling would otherwise draw them all.
  const scroller = useRef<HTMLDivElement>(null);
  const list = useRef<HTMLDivElement>(null);
  const [sight, setSight] = useState({ from: 0, to: 0 });
  /** How high each unfolded commit was, with its files, the last time it was drawn. */
  const measured = useRef(new Map<string, number>());
  const [, setMeasuredAgain] = useState(0);
  const look = () => {
    const element = scroller.current;
    if (!element || !list.current) return;
    // Where the list starts in what scrolls: under the line that says what is left to push.
    const offset = list.current.getBoundingClientRect().top - element.getBoundingClientRect().top + element.scrollTop;
    const from = element.scrollTop - offset;
    setSight((current) => (current.from === from && current.to === from + element.clientHeight ? current : { from, to: from + element.clientHeight }));
  };
  const shown = state !== null && log !== null;
  useLayoutEffect(() => {
    const element = scroller.current;
    if (!shown || !element) return;
    const observer = new ResizeObserver(look);
    observer.observe(element);
    look();
    return () => observer.disconnect();
  }, [shown]);
  // An unfolded commit is as high as its files, which come later: it is measured each time it is drawn.
  useLayoutEffect(() => {
    let changed = false;
    for (const section of list.current?.children ?? []) {
      const hash = (section as HTMLElement).dataset.hash;
      if (!hash || !(hash in open)) continue;
      const height = (section as HTMLElement).offsetHeight;
      if (measured.current.get(hash) !== height) {
        measured.current.set(hash, height);
        changed = true;
      }
    }
    if (changed) setMeasuredAgain((count) => count + 1);
  });

  useEffect(() => {
    const stop = onMessage((message) => {
      if (message.type === 'state') {
        const next = receive(message);
        setState(next);
        setLoaded(loadedOf(message));
        const root = next?.root ?? null;
        if (root !== rootRef.current) {
          // Another repository: its commits are others, and are asked for from the start.
          rootRef.current = root;
          setLog(null);
          setLimit(PAGE);
          setOpen({});
        }
      } else if (message.type === 'log') {
        // One asked of the repository shown before is of no use.
        if (message.log.root !== rootRef.current) return;
        setLog(message.log);
        // A commit never changes, so what was read of the ones still listed holds; the others are gone.
        const hashes = new Set(message.log.commits.map((commit) => commit.hash));
        setOpen((current) => Object.fromEntries(Object.entries(current).filter(([hash]) => hashes.has(hash))));
      } else if (message.type === 'detail') {
        if (message.hash in openRef.current) setOpen((current) => ({ ...current, [message.hash]: message.detail }));
      }
    });
    post({ type: 'ready' });
    return stop;
  }, []);

  const key = state ? logKey(state) : null;
  useEffect(() => {
    if (key !== null) post({ type: 'getLog', limit });
  }, [key, limit]);

  if (!state) return <Placeholder loaded={loaded} />;
  if (!log) return <p className="none muted">Loading…</p>;

  const unpushed = log.unpushed && new Set(log.unpushed);
  const against = log.upstream ?? 'a remote branch';

  const toggle = (commit: Commit) => {
    if (commit.hash in open) {
      const { [commit.hash]: _closed, ...rest } = open;
      setOpen(rest);
    } else {
      setOpen({ ...open, [commit.hash]: null });
      post({ type: 'getDetail', hash: commit.hash });
    }
  };

  /** Whether a commit is pushed, as an icon: nothing is said where there is no remote to push to. */
  const pushState = (commit: Commit) => {
    if (!unpushed) return { name: 'circle-outline', kind: 'local', title: '' };
    return unpushed.has(commit.hash)
      ? { name: 'cloud-upload', kind: 'unpushed', title: log.upstream ? `Not pushed to ${against}` : 'On no remote branch' }
      : { name: 'cloud', kind: 'pushed', title: `Pushed: it is in ${against}` };
  };

  const files = (commit: Commit, detail: CommitDetail) => {
    const source: DiffSource = { kind: 'commit', hash: commit.hash, parent: detail.parents[0] ?? null };
    return (
      <div className="commit-files">
        {detail.files.map((file) => (
          <FileRow key={file.path} file={file} onOpen={() => post({ type: 'openDiff', source, file })} />
        ))}
        {!detail.files.length && <p className="none muted">No files</p>}
      </div>
    );
  };

  // The commits from `start` to `end` are drawn; the room of those above and below is kept empty.
  const heightOf = (commit: Commit) => (commit.hash in open ? (measured.current.get(commit.hash) ?? ROW * 2) : ROW);
  const top = sight.from - OVERSCAN * ROW;
  const bottom = sight.to + OVERSCAN * ROW;
  let start = 0;
  let end = 0;
  let above = 0;
  let below = 0;
  let at = 0;
  for (const commit of log.commits) {
    const height = heightOf(commit);
    if (at + height <= top) {
      start++;
      end++;
      above += height;
    } else if (at < bottom) end++;
    else below += height;
    at += height;
  }

  return (
    <div className="side log" ref={scroller} onScroll={look}>
      {log.commits.length ? <p className="log-summary muted">{summary(state, log)}</p> : <p className="none muted">No commits yet.</p>}
      <div ref={list} style={{ paddingTop: above, paddingBottom: below }}>
        {log.commits.slice(start, end).map((commit) => {
          const unfolded = commit.hash in open;
          const detail = open[commit.hash];
          const pushed = pushState(commit);
          const about = `${commit.subject}\n${shortHash(commit.hash)} · ${commit.author} · ${fullDate(commit.date)}`;
          return (
            <section key={commit.hash} data-hash={commit.hash}>
              <div className={`file commit ${pushed.kind}`} onContextMenu={(event) => openMenu(event, logMenu(commit, state))}>
                <button
                  className="twist"
                  onClick={() => toggle(commit)}
                  aria-expanded={unfolded}
                  aria-label={`${unfolded ? 'Hide' : 'Show'} the files of ${shortHash(commit.hash)}`}
                >
                  <Icon name={unfolded ? 'chevron-down' : 'chevron-right'} />
                </button>
                <button className="file-open" title={pushed.title ? `${about}\n${pushed.title}` : about} onClick={() => toggle(commit)}>
                  <span className="push-state" aria-label={pushed.title || undefined}>
                    <Icon name={pushed.name} />
                  </span>
                  <span className="name">{commit.subject}</span>
                  <span className="meta">
                    {commit.author}, {relativeTime(commit.date)}
                  </span>
                </button>
                <ActionButtons
                  actions={[
                    { icon: 'link-external', title: 'Open in an editor tab, with its changes', run: () => post({ type: 'openCommit', hash: commit.hash }) },
                    { icon: 'git-commit', title: 'Show in the graph', run: () => post({ type: 'reveal', hash: commit.hash }) },
                  ]}
                />
              </div>
              {unfolded && (detail ? files(commit, detail) : <p className="none muted">Loading…</p>)}
            </section>
          );
        })}
      </div>
      {/* Fewer than were asked for, and more to come: they are on their way. */}
      {log.hasMore && log.commits.length < limit ? (
        <button className="file more" disabled>
          <Icon name="loading" spin />
          Loading more commits…
        </button>
      ) : (
        log.hasMore && (
          <button className="file more" onClick={() => setLimit(limit + PAGE)}>
            Load more commits…
          </button>
        )
      )}
    </div>
  );
}

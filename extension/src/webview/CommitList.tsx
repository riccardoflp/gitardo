import { createPortal } from 'react-dom';
import { memo, useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState, type CSSProperties, type KeyboardEvent, type MouseEvent } from 'react';
import { fitGraph, foldMerges, laneCount, laneTips, mergedBranch, mergedCommits, oneLane, ownCommits, reachedFrom, reaching, traceGraph, type ShownRow } from '../shared/graph';
import { WORKTREE, type GraphColumn, type Loading, type Row } from '../shared/protocol';
import { Icon } from './Icon';
import { Progress } from './Progress';
import { Sash } from './Sash';
import type { DragSource } from './drag';
import { datePeriods, laneColor, relativeTime, shortHash } from './util';
import { post } from './vscode';
import { useKept } from './kept';
import { held, type Held } from './pages';
import { rowFonts, type RowFonts } from './labelWidths';
import { LabelList, labelListHover, type Label, type RowActions, type SelectMode } from './CommitLabels';
import { CommitRow, FoldedRow, Graph, ShallowEnd } from './CommitRows';

interface Props {
  /** The repository the commits are of, which keeps where they were scrolled to while another is shown. */
  root: string;
  /** The history the commits are of, the repository and what the graph is narrowed to: read again, the list keeps its length (D72). */
  listing: string;
  /** Whether it does, as `gitardo.repositories.keepOpen` says (`repo.keep-open`). */
  keepOpen: boolean;
  rows: Row[];
  /**
   * The row of the uncommitted changes as last sent, which says how many files they are: the one first in `rows` is
   * laid out the same, and stays when only that count changed, so that the rows are not laid out again for it.
   */
  draft?: Row | null;
  /** What to say in their place when there are none. */
  empty: string;
  hasMore: boolean;
  /** What the extension is reading the commits for, while it does. */
  loading: Loading | null;
  labels: Map<string, Label[]>;
  /** Hashes matching the search, or null when there is no search. */
  matches: string[] | null;
  /** The width the graph was dragged to, or null while it chooses its own. */
  graphWidth: number | null;
  onGraphWidth(width: number | null): void;
  selected: string | null;
  /** The commits selected together, when there is more than one. */
  marked: ReadonlySet<string>;
  /** A new object each time a row should be scrolled into view. */
  reveal: { hash: string } | null;
  onSelect(hash: string, mode: SelectMode): void;
  /** A commit was double-clicked, to see it whole. */
  onOpen(hash: string): void;
  /** What a double click on a label does instead, when it does something of its own: a branch is checked out. */
  labelOpen(label: Label): (() => void) | undefined;
  /** Two lines to a commit: who made it and when go under its message, where a narrow list has the room for them. */
  tall?: boolean;
  onRowMenu(event: MouseEvent, row: Row): void;
  onLabelMenu(event: MouseEvent, label: Label, row: Row): void;
  /** What dragging a row, or one of its labels, carries. Null when it cannot be dragged. */
  dragSource(label: Label | null, row: Row): DragSource | null;
  canDrop(row: Row): boolean;
  onDrop(event: MouseEvent, row: Row): void;
  /** The commit whose history stands out, while the pointer rests on a label here or on a branch beside the graph. */
  traced: string | null;
  /** The pointer came to a label, with its commit, or left it, with null. */
  onTrace(hash: string | null): void;
  /** The commits a branch has that its upstream has not yet, with the upstream they would be pushed to. */
  unpushed: ReadonlyMap<string, string>;
  /** Whether the history of the selected commit stands out, what it comes from and what was made on top of it. */
  traceSelected: boolean;
  /** What is shown of each commit beside its message. */
  columns: GraphColumn[];
  /** The commit checked out, which `Home` goes back to. */
  head: string | null;
  /** Whether a line with the name of a time further back, a day, a week or a month, comes before its first commit. */
  dateSeparators: boolean;
  /** Whether every commit is on one line, without lanes, its merges rings (`graph.one-lane`). */
  oneLane: boolean;
  /** The merges whose commits are folded into one row. */
  folded: ReadonlySet<string>;
  /** The row of the commits of a merge was clicked, or one of them is to be shown: they are shown again. */
  onUnfold(merge: string): void;
  /** Folds the commits a merge brought in into one row. Missing where nothing can be folded, among what a search finds. */
  onFold?(merge: string): void;
  /** The commits whose parents were not fetched, where the history of a shallow clone ends: empty in a whole clone. */
  shallow: readonly string[];
}

const ROW_HEIGHT = 24;
/** With the author and the date under the message, as in the side bar. */
const TALL_ROW_HEIGHT = 42;
/** What names a line of the graph after the tip it comes from: not the marks of a bisect. */
const LANE_NAMES = new Set<Label['kind']>(['head', 'branch', 'remote', 'tag', 'stash']);
/** The room the row keeps after its message and at its right end, and between the cell of the graph and the message. */
const ROW_PADDING = 8;
const ROW_GAP = 8;
/** The columns at the right end of a row on one line, and how narrow the list is when the author and the hash leave (`styles.css`). */
const AUTHOR_WIDTH = 120;
const DATE_WIDTH = 84;
const HASH_WIDTH = 56;
const NARROW_LIST = 560;
/** How many rows are drawn above and below those in sight, so that scrolling a little shows no empty space. */
export const OVERSCAN = 8;
/**
 * How many commits are kept loaded below the last one on screen. Scrolling fast covers a few hundred commits in a
 * second or two, about as long as reading the next ones takes in a large repository: it should never catch up.
 */
const AHEAD = 500;
/** How long the pointer rests on a label before its history stands out, so that passing over labels does not flash the graph. */
const TRACE_DELAY = 150;
/** How long the history stays after the pointer leaves, so that going from one label to the next does not flash it either. */
const UNTRACE_DELAY = 100;
/** Whether Shift is held down, wherever the focus is in the page. */
function useShiftKey(): boolean {
  const [held, setHeld] = useState(false);
  useEffect(() => {
    const key = (event: globalThis.KeyboardEvent) => setHeld(event.shiftKey);
    const pointer = (event: PointerEvent) => setHeld(event.shiftKey);
    // Let go of while the page had not the focus, it would otherwise stay held.
    const release = () => setHeld(false);
    window.addEventListener('keydown', key);
    window.addEventListener('keyup', key);
    window.addEventListener('pointerover', pointer);
    window.addEventListener('blur', release);
    return () => {
      window.removeEventListener('keydown', key);
      window.removeEventListener('keyup', key);
      window.removeEventListener('pointerover', pointer);
      window.removeEventListener('blur', release);
    };
  }, []);
  return held;
}

/**
 * The commit whose history stands out in the graph, from a label or a branch the pointer rests on, and what to call
 * when the pointer comes to one or leaves it.
 */
export function useTrace(): [string | null, (hash: string | null) => void] {
  const [traced, setTraced] = useState<string | null>(null);
  const shown = useRef<string | null>(null);
  const timer = useRef<ReturnType<typeof setTimeout>>(undefined);
  useEffect(() => () => clearTimeout(timer.current), []);
  const traceLater = useCallback((hash: string | null) => {
    const show = (next: string | null) => {
      shown.current = next;
      setTraced(next);
    };
    clearTimeout(timer.current);
    if (hash && shown.current) show(hash);
    else timer.current = setTimeout(() => show(hash), hash ? TRACE_DELAY : UNTRACE_DELAY);
  }, []);
  return [traced, traceLater];
}

interface MarksProps {
  rows: Row[];
  /** How many rows high the list is, the one that says more are loading included. */
  total: number;
  /** How high the list is on the screen, and so its scroll bar. */
  height: number;
  head: string | null;
  selected: string | null;
  matches: ReadonlySet<string> | null;
}

/**
 * Beside the scroll bar, where in the list the commit checked out is, in the color of its line, the one selected,
 * and what a search found. Marks closer than a pixel are one.
 */
const ScrollMarks = memo(function ScrollMarks({ rows, total, height, head, selected, matches }: MarksProps) {
  const at = (index: number) => Math.round(((index + 0.5) / total) * height);
  const found = useMemo(() => {
    const tops = new Set<number>();
    if (matches) rows.forEach((row, index) => matches.has(row.hash) && tops.add(at(index)));
    return [...tops];
  }, [rows, matches, total, height]);
  const headAt = rows.findIndex((row) => row.hash === head);
  const selectedAt = selected === null ? -1 : rows.findIndex((row) => row.hash === selected);
  return (
    <div className="scroll-marks" aria-hidden="true">
      <div style={{ height }}>
        {found.map((top) => (
          <span key={top} className="found" style={{ top }} />
        ))}
        {headAt >= 0 && <span className="head" style={{ top: at(headAt), background: laneColor(rows[headAt].graph.color) }} />}
        {selectedAt >= 0 && <span className="selected" style={{ top: at(selectedAt) }} />}
      </div>
    </div>
  );
});

/** The first row from `start` on, going down or up, that is what is looked for, not counting the one at `start`; -1 when none is. */
function nearest(rows: Row[], start: number, step: number, wanted: (row: Row) => boolean): number {
  for (let index = start + step; index >= 0 && index < rows.length; index += step) if (wanted(rows[index])) return index;
  return -1;
}

export function CommitList(props: Props) {
  const { empty, hasMore, loading, labels, matches, selected, marked, reveal, onSelect, onUnfold } = props;
  // The commits a folded merge brought in have one row for them all.
  const { rows: folded, hiddenBy } = useMemo(() => foldMerges(props.rows, props.folded), [props.rows, props.folded]);
  const rows = useMemo(() => (props.oneLane ? oneLane(folded) : folded), [folded, props.oneLane]);
  const foldedNow = useMemo(() => new Set(hiddenBy.values()), [hiddenBy]);
  // Whether a merge brought in commits among those loaded, found out for each merge as it comes into sight.
  const foldable = useMemo(() => new Map<string, boolean>(), [props.rows]);
  const canFold = (row: Row) => {
    if (!props.onFold || row.parents.length < 2) return false;
    let known = foldable.get(row.hash);
    if (known === undefined) foldable.set(row.hash, (known = mergedCommits(props.rows, row.hash).size > 0));
    return known;
  };
  const { onOpen, tall, traced, onTrace, unpushed, traceSelected, columns } = props;
  const shows = (column: GraphColumn) => columns.includes(column);
  const [showAuthor, showDate, showHash] = [shows('author'), shows('date'), shows('hash')];
  const rowHeight = tall ? TALL_ROW_HEIGHT : ROW_HEIGHT;
  const [dropTarget, setDropTarget] = useState<string | null>(null);
  // The rows are given actions that stay the same, and call what the list was last given.
  const latest = useRef(props);
  latest.current = props;
  const actions = useMemo<RowActions>(
    () => ({
      select: (hash, mode) => latest.current.onSelect(hash, mode),
      open: (hash) => latest.current.onOpen(hash),
      rowMenu: (event, row) => latest.current.onRowMenu(event, row),
      labelMenu: (event, label, row) => latest.current.onLabelMenu(event, label, row),
      labelOpen: (label) => latest.current.labelOpen(label),
      dragSource: (label, row) => latest.current.dragSource(label, row),
      canDrop: (row) => latest.current.canDrop(row),
      drop: (event, row) => latest.current.onDrop(event, row),
      trace: (hash) => latest.current.onTrace(hash),
      fold: (merge) => latest.current.onFold?.(merge),
      unfold: (merge) => latest.current.onUnfold(merge),
      setDropTarget,
      restOnLabels: (hash, label) => labelsHover.current.rest(hash, label),
      leaveLabels: (label) => labelsHover.current.leave(label),
      closeLabels: () => labelsHover.current.close(false),
    }),
    [],
  );
  // The list of all the labels of a commit whose row does not show them all, under the label the pointer rested on.
  const [labelList, setLabelList] = useState<{ hash: string; anchor: DOMRect } | null>(null);
  const labelsHover = useRef(labelListHover(setLabelList));
  useEffect(() => () => labelsHover.current.dispose(), []);
  useEffect(() => {
    if (!labelList) return;
    // `Escape` or a press elsewhere closes it, and it does not open again while the pointer stays on its label.
    const escape = (event: globalThis.KeyboardEvent) => event.key === 'Escape' && labelsHover.current.close(true);
    const press = (event: Event) => {
      if (!(event.target as Element | null)?.closest?.('.label-list')) labelsHover.current.close(true);
    };
    const blur = () => labelsHover.current.close(false);
    window.addEventListener('keydown', escape, true);
    window.addEventListener('mousedown', press, true);
    window.addEventListener('blur', blur);
    return () => {
      window.removeEventListener('keydown', escape, true);
      window.removeEventListener('mousedown', press, true);
      window.removeEventListener('blur', blur);
    };
  }, [labelList]);
  // Where each commit shown is in the list, and each commit loaded, so that none is looked for row by row.
  const shownAt = useMemo(() => new Map(rows.map((row, index) => [row.hash, index])), [rows]);
  const loadedByHash = useMemo(() => new Map(props.rows.map((row) => [row.hash, row])), [props.rows]);
  const scroller = useRef<HTMLDivElement>(null);
  const [scrollTop, setScrollTop] = useState(0);
  const [{ width, height }, setSize] = useState({ width: 0, height: 0 });

  // The fonts of the page, which the labels, the messages and the authors are measured in; they change with the zoom of VS Code.
  const [fonts, setFonts] = useState<RowFonts | null>(null);
  useLayoutEffect(() => {
    const element = scroller.current!;
    const measure = () => {
      setSize({ width: element.clientWidth, height: element.clientHeight });
      const now = rowFonts(element);
      setFonts((before) => (before && before.text === now.text && before.small === now.small ? before : now));
    };
    const observer = new ResizeObserver(measure);
    observer.observe(element);
    measure();
    return () => observer.disconnect();
  }, []);

  const lanes = useMemo(() => {
    let most = 1;
    for (const row of rows) most = Math.max(most, laneCount(row.graph));
    return most;
  }, [rows]);
  // While the commits of the same history are read again from their first page, after a commit, a fetch or Refresh, the
  // list keeps the length it had, so that it stays where it is scrolled to, and the graph its width (D72).
  const kept = useRef<Held | null>(null);
  kept.current = held(kept.current, props.listing, props.rows.length, lanes, hasMore);
  // Coming back to a repository that kept what it showed, the commits are where they were scrolled to, in a list as
  // long as it was: its first page comes first and the rest of its commits behind it (QA6), each where it was.
  const comingBack = useRef<number | null>(null);
  const [, drawAgain] = useState(0);
  // Before the layout effect of `useKept`, so that the list is scrolled once it is drawn as long as it was.
  useLayoutEffect(() => {
    if (comingBack.current === null || !scroller.current) return;
    scroller.current.scrollTop = comingBack.current;
    comingBack.current = null;
  });
  useKept(props.root, props.keepOpen, 'scroll', { top: scrollTop, held: kept.current }, (back) => {
    kept.current = back.held;
    comingBack.current = back.top;
    setScrollTop(back.top);
    drawAgain((count) => count + 1);
  });
  // The rows not read again yet, under those that are.
  const awaited = kept.current.loaded - props.rows.length;
  // Once the last commits of a shallow clone are read, one more row says the history was not all fetched.
  const shallow = useMemo(() => new Set(props.shallow), [props.shallow]);
  const shallowEnd = shallow.size > 0 && !hasMore && !awaited && rows.length > 0;
  const length = rows.length + awaited + (hasMore || shallowEnd ? 1 : 0);
  // The rows the list had before the read started again: they are drawn, dimmed, where its own are not read yet.
  const before = useRef<ShownRow[]>(rows);
  if (!awaited) before.current = rows;
  const fit = fitGraph(kept.current.lanes, props.graphWidth, width);
  // Where the message, the labels and the columns of a row go, and how wide those columns are, as `styles.css` lays them out.
  const lineWidth = width > 0 ? width - ROW_PADDING - fit.width - ROW_GAP : 0;
  const narrow = width <= NARROW_LIST;
  const columnsWidth =
    (showAuthor && !narrow ? AUTHOR_WIDTH + ROW_GAP : 0) + (showDate ? DATE_WIDTH + ROW_GAP : 0) + (showHash && !narrow ? HASH_WIDTH + ROW_GAP : 0);
  const matchSet = useMemo(() => (matches ? new Set(matches) : null), [matches]);
  const periods = useMemo(() => (props.dateSeparators ? datePeriods(rows) : null), [rows, props.dateSeparators]);
  // The branches whose tips are above the top of the list, on the lines that go on down it.
  const laneName = (row: Row) => labels.get(row.hash)?.find((label) => LANE_NAMES.has(label.kind))?.name ?? row.stash ?? null;
  const tips = useMemo(() => laneTips(rows, (index) => laneName(rows[index]) !== null), [rows, labels]);
  // What the history of the commit traced does not reach fades. Without one, what the selected commit comes from and
  // what was made on top of it stand out, less: it stays while the commits are read.
  const hovered = traced !== null && shownAt.has(traced);
  // With Shift held, only what it would bring to what is checked out.
  const shift = useShiftKey();
  const own = useMemo(() => (hovered && shift ? ownCommits(rows, traced, props.head) : null), [rows, traced, hovered, shift, props.head]);
  const chosen = traceSelected && !hovered && marked.size < 2 && selected !== null && selected !== WORKTREE ? selected : null;
  const trace = useMemo(() => {
    if (hovered) return traceGraph(rows, own?.size ? own : reachedFrom(rows, traced));
    if (!chosen || !shownAt.has(chosen)) return null;
    return traceGraph(rows, new Set([...reachedFrom(rows, chosen), ...reaching(rows, chosen)]));
  }, [rows, traced, hovered, chosen, own]);

  const start = Math.max(0, Math.floor(scrollTop / rowHeight) - OVERSCAN);
  const end = Math.min(rows.length, Math.ceil((scrollTop + height) / rowHeight) + OVERSCAN);
  // Under the row that says more are loading, the rows of before that are in sight, each where it was.
  const awaitedFrom = Math.max(rows.length + 1, start);
  const awaitedRows = awaited ? before.current.slice(awaitedFrom, Math.min(length, Math.ceil((scrollTop + height) / rowHeight) + OVERSCAN)) : [];

  // Ask for the next page once per loaded length, well before the end of the list comes into sight. With small pages
  // one follows the other until there are enough ahead.
  const requestedAt = useRef(0);
  useEffect(() => {
    // Counted in the commits loaded: those of a folded merge are not in the rows.
    if (hasMore && end >= rows.length - AHEAD && requestedAt.current !== props.rows.length) {
      requestedAt.current = props.rows.length;
      post({ type: 'loadMore' });
    }
  }, [hasMore, end, rows.length, props.rows.length]);

  const scrollTo = (index: number, center: boolean) => {
    const element = scroller.current;
    if (!element || index < 0) return;
    const top = index * rowHeight;
    if (center) element.scrollTop = top - element.clientHeight / 2 + rowHeight / 2;
    else if (top < element.scrollTop) element.scrollTop = top;
    else if (top + rowHeight > element.scrollTop + element.clientHeight) {
      element.scrollTop = top + rowHeight - element.clientHeight;
    }
  };

  // A commit to show that a folded merge put away is shown once the merge is unfolded.
  const revealing = useRef<string | null>(null);
  useEffect(() => {
    if (!reveal) return;
    const merge = hiddenBy.get(reveal.hash);
    if (merge) {
      revealing.current = reveal.hash;
      onUnfold(merge);
    } else scrollTo(shownAt.get(reveal.hash) ?? -1, true);
    // Only a new reveal request should move the scroll position, not every refresh of the rows.
  }, [reveal]);
  useEffect(() => {
    const index = revealing.current ? (shownAt.get(revealing.current) ?? -1) : -1;
    if (index < 0) return;
    revealing.current = null;
    scrollTo(index, true);
  }, [rows]);

  const first = Math.floor(scrollTop / rowHeight);
  const offscreen: { tip: number; name: string; color: number }[] = [];
  if (first > 0 && first < rows.length) {
    rows[first].graph.top.forEach((line, index) => {
      const tip = tips[first][index];
      const name = tip === null ? null : laneName(rows[tip]);
      if (tip !== null && name && !offscreen.some((other) => other.tip === tip)) offscreen.push({ tip, name, color: line.color });
    });
  }

  const go = (index: number) => {
    if (index < 0 || rows[index].folded) return;
    onSelect(rows[index].hash, 'single');
    scrollTo(index, false);
  };
  const onKeyDown = (event: KeyboardEvent) => {
    // Enter does what a double click does.
    if (event.key === 'Enter' && selected && selected !== WORKTREE) return onOpen(selected);
    if (!rows.length) return;
    const current = selected === null ? -1 : (shownAt.get(selected) ?? -1);
    // On a merge, the arrow to the left folds what it brought in and the one to the right shows it again.
    if ((event.key === 'ArrowLeft' || event.key === 'ArrowRight') && current >= 0 && !event.altKey) {
      const row = rows[current];
      if (event.key === 'ArrowLeft' && !foldedNow.has(row.hash) && canFold(row)) props.onFold!(row.hash);
      else if (event.key === 'ArrowRight' && foldedNow.has(row.hash)) onUnfold(row.hash);
      else return;
      return event.preventDefault();
    }
    if (event.key === 'Home' && !event.ctrlKey && !event.metaKey) {
      event.preventDefault();
      return go(props.head === null ? -1 : (shownAt.get(props.head) ?? -1));
    }
    const step = event.key === 'ArrowDown' ? 1 : event.key === 'ArrowUp' ? -1 : 0;
    if (!step) return;
    event.preventDefault();
    if (event.altKey && current >= 0) {
      // With Shift, the next merge or the one before; without, the first parent, or the commit made on top of this one:
      // on its line when there is one, or else the merge that took it.
      const row = rows[current];
      if (event.shiftKey) return go(nearest(rows, current, step, (other) => other.parents.length > 1));
      if (step > 0) return go(row.parents.length ? (shownAt.get(row.parents[0]) ?? -1) : -1);
      const child = nearest(rows, current, -1, (other) => other.parents[0] === row.hash);
      return go(child >= 0 ? child : nearest(rows, current, -1, (other) => other.parents.includes(row.hash)));
    }
    // The row of a folded merge is no commit to select: it is passed over.
    let next = current + step;
    while (rows[next]?.folded) next += step;
    if (next >= 0 && next < rows.length) go(next);
    else if (current < 0) go(0);
  };

  // The commit of the list of labels, and its labels, as they are now.
  const listedAt = labelList ? shownAt.get(labelList.hash) : undefined;
  const listedRow = listedAt === undefined ? undefined : rows[listedAt];
  const listedLabels = labelList && listedRow && labels.get(labelList.hash) ? { row: listedRow, labels: labels.get(labelList.hash)!, anchor: labelList.anchor } : null;

  const listClasses = ['commits'];
  if (fit.cut) listClasses.push('cut');
  // The commits of the filter before stay until those of the new one come, but they are no longer what is asked for.
  if (loading === 'filter') listClasses.push('stale');
  if (trace && !hovered) listClasses.push('selection-traced');

  return (
    <div
      className={listClasses.join(' ')}
      ref={scroller}
      tabIndex={0}
      role="listbox"
      aria-label="Commits"
      aria-multiselectable="true"
      onScroll={(event) => {
        setScrollTop(event.currentTarget.scrollTop);
        // The list of the labels of a row would no longer be under it.
        labelsHover.current.close(false);
      }}
      onKeyDown={onKeyDown}
    >
      {loading && <Progress label={loading === 'more' ? 'Loading more commits' : 'Loading commits'} />}
      {!rows.length && <div className="empty">{loading === 'filter' ? 'Loading commits…' : empty}</div>}
      {/* It stays where it is while the commits scroll under it. */}
      {rows.length > 0 && (
        <div className="graph-edge" style={{ '--graph': `${fit.width}px`, '--list-height': `${height}px` } as CSSProperties}>
          <Sash
            pane="graph"
            width={fit.width}
            // Wider than all of its lanes at their ease, the graph would only be empty.
            onResize={(wanted) => props.onGraphWidth(Math.min(wanted, fit.natural))}
            onReset={() => props.onGraphWidth(null)}
          />
        </div>
      )}
      {rows.length > 0 && height > 0 && (
        <ScrollMarks rows={rows} total={length} height={height} head={props.head} selected={selected} matches={matchSet} />
      )}
      {offscreen.length > 0 && (
        <div className="lane-names" style={{ '--graph': `${fit.width}px` } as CSSProperties}>
          <div>
            {offscreen.map(({ tip, name, color }) => (
              <button
                key={tip}
                className="lane-name"
                style={{ '--lane': laneColor(color) } as CSSProperties}
                title={`Go up to ${name}`}
                onClick={() => {
                  onSelect(rows[tip].hash, 'single');
                  scrollTo(tip, true);
                }}
                onMouseEnter={() => onTrace(rows[tip].hash)}
                onMouseLeave={() => onTrace(null)}
              >
                <Icon name="arrow-up" />
                <span>{name}</span>
              </button>
            ))}
          </div>
        </div>
      )}
      {/* With more to load, one more row says so at the end, where scrolling gets before they come. */}
      <div className="spacer" style={{ height: length * rowHeight }}>
        {rows.slice(start, end).map((listed, offset) => {
          const index = start + offset;
          const row = listed.hash === WORKTREE && props.draft ? { ...listed, subject: props.draft.subject } : listed;
          if (row.folded) {
            const { merge, count } = row.folded;
            const mergeRow = loadedByHash.get(merge);
            const tip = mergeRow && labels.get(mergeRow.parents[1])?.find((label) => label.kind === 'branch' || label.kind === 'remote');
            const name = tip?.name ?? (mergeRow && mergedBranch(mergeRow.subject));
            const what = `${count} ${count === 1 ? 'commit' : 'commits'}${name ? ` of ${name}` : ' merged'}`;
            return (
              <FoldedRow
                key={row.hash}
                row={row}
                merge={merge}
                index={index}
                height={rowHeight}
                graphWidth={fit.width}
                lane={fit.lane}
                tall={!!tall}
                what={what}
                trace={trace?.[index]}
                actions={actions}
              />
            );
          }
          return (
            <CommitRow
              key={row.hash}
              row={row}
              index={index}
              height={rowHeight}
              graphWidth={fit.width}
              lane={fit.lane}
              tall={!!tall}
              selected={row.hash === selected || marked.has(row.hash)}
              dimmed={matchSet !== null && !matchSet.has(row.hash)}
              trace={trace?.[index]}
              dropTarget={row.hash === dropTarget}
              labels={labels.get(row.hash)}
              unpushed={unpushed.get(row.hash)}
              shallow={shallow.has(row.hash)}
              period={periods?.get(index)}
              fold={foldedNow.has(row.hash) ? 'folded' : canFold(row) ? 'foldable' : null}
              showAuthor={showAuthor}
              showDate={showDate}
              showHash={showHash}
              oneLane={props.oneLane}
              lineWidth={lineWidth}
              columnsWidth={columnsWidth}
              fonts={fonts}
              actions={actions}
            />
          );
        })}
        {awaitedRows.map((row, offset) => (
          <div
            key={`awaited:${row.hash}`}
            className={tall ? 'row tall awaited' : 'row awaited'}
            style={{ top: (awaitedFrom + offset) * rowHeight, height: rowHeight }}
            aria-hidden="true"
          >
            <Graph graph={row.graph} width={fit.width} height={rowHeight} lane={fit.lane} node={row.folded ? 'folded' : row.hash === WORKTREE ? 'worktree' : row.stash ? 'stash' : props.oneLane && row.parents.length > 1 ? 'merge' : 'commit'} />
            {!row.folded && tall && (
              <div className="lines">
                <span className="subject">{row.subject}</span>
                <div className="message">
                  {shows('author') && <span className="who">{row.author}</span>}
                  {shows('date') && <span className="when">{relativeTime(row.date)}</span>}
                </div>
              </div>
            )}
            {!row.folded && !tall && (
              <>
                <div className="message">
                  <span className="subject">{row.subject}</span>
                </div>
                {shows('author') && <div className="author">{row.author}</div>}
                {shows('date') && <div className="date">{relativeTime(row.date)}</div>}
                {shows('hash') && <div className="hash">{shortHash(row.hash)}</div>}
              </>
            )}
          </div>
        ))}
        {hasMore && rows.length > 0 && (
          // Scrolled past the commits read and the rows of before, while the others are read again, it is said where the list is.
          <div
            className="row more"
            style={{ top: (first < before.current.length ? rows.length : Math.max(rows.length, Math.min(first, length - 1))) * rowHeight, height: rowHeight }}
            aria-hidden="true"
          >
            <Icon name="loading" spin />
            Loading more commits…
          </div>
        )}
        {shallowEnd && <ShallowEnd top={rows.length * rowHeight} height={rowHeight} />}
      </div>
      {listedLabels && createPortal(<LabelList {...listedLabels} actions={actions} />, document.body)}
    </div>
  );
}

import { createPortal } from 'react-dom';
import {
  memo,
  useCallback,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
  type CSSProperties,
  type DragEvent,
  type KeyboardEvent,
  type MouseEvent,
} from 'react';
import { fitGraph, foldMerges, laneCount, laneTips, mergedBranch, mergedCommits, oneLane, ownCommits, reachedFrom, reaching, traceGraph, type GraphTrace, type ShownRow } from '../shared/graph';
import { WORKTREE, type GraphColumn, type GraphRow, type Loading, type PullRequest, type Row } from '../shared/protocol';
import { serviceName } from '../shared/hosting';
import { pullBadgeText, pullTitle } from '../shared/refs';
import { SHORTENED_LEAST, fitLabels, roomBesideAuthor, roomBesideMessage, type FittedLabels } from '../shared/labelFit';
import { Icon } from './Icon';
import { Progress } from './Progress';
import { Sash } from './Sash';
import { endDrag, startDrag, type DragSource } from './drag';
import { datePeriods, fullDate, laneColor, relativeTime, shortHash } from './util';
import { post } from './vscode';
import { useKept } from './kept';
import { held, type Held } from './pages';
import { countWidth, labelWidth, rowFonts, textWidth, type RowFonts } from './labelWidths';

export interface Label {
  /** `good`, `bad` and `skipped` are the marks of a bisect in progress. */
  kind: 'head' | 'branch' | 'remote' | 'tag' | 'stash' | 'good' | 'bad' | 'skipped';
  name: string;
  current?: boolean;
  /** For a branch: the folder of the other worktree that has it checked out. */
  worktree?: string;
  /** For a branch: its upstream, when it has the same name there and is at the same commit, so the one label stands for both. */
  remote?: string;
  /** For a branch: its open pull request, and the name of the branch on its hosting service. */
  pull?: { request: PullRequest; branch: string };
  /** For a tag: the mark of the remote checked, which lacks it or has another tag of its name (`tag.remote-marks`). */
  mark?: { icon: string; title: string };
}

/** A plain click selects one commit; with Ctrl or Cmd it adds or removes one; with Shift it selects up to the clicked one. */
export type SelectMode = 'single' | 'toggle' | 'range';

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
/** What the dashed line under the dot of a commit whose parents were not fetched says (`graph.shallow-end`). */
export const SHALLOW_LINE = 'The commits before this one were not fetched: this repository is a shallow clone.';
/** What the last row of the commits of a shallow clone says, once they are all read (`graph.shallow-end`). */
export const SHALLOW_END = 'The history ends here: this is a shallow clone.';

/** How tall a label is, on one line and on the line under a message, as `styles.css` draws it. */
const LABEL_HEIGHT = 18;
const TALL_LABEL_HEIGHT = 16;
/** The gap between the labels of a row, on one line and on the line under a message. */
const LABEL_GAP = 4;
const TALL_LABEL_GAP = 6;
/** The room the row keeps after its message and at its right end, and between the cell of the graph and the message. */
const ROW_PADDING = 8;
const ROW_GAP = 8;
/** The arrow that folds the commits of a merge, after its message, and the gap before it. */
const FOLD_WIDTH = 18 + LABEL_GAP;
/** The columns at the right end of a row on one line, and how narrow the list is when the author and the hash leave (`styles.css`). */
const AUTHOR_WIDTH = 120;
const DATE_WIDTH = 84;
const HASH_WIDTH = 56;
const NARROW_LIST = 560;
/** How long the pointer rests on a label of a row that does not show all of them before their list opens (Q642). */
const LIST_AFTER = 300;
/** How long the list stays once the pointer left it and the labels, for the pointer to go from one to the other. */
const LIST_CLOSE_AFTER = 120;
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
export const LABEL_ICONS = {
  head: 'target',
  branch: 'git-branch',
  remote: 'cloud',
  tag: 'tag',
  stash: 'git-stash',
  good: 'check',
  bad: 'close',
  skipped: 'debug-step-over',
} as const;

/** The icons of a label: of a branch, also the worktree that has it checked out and the upstream it is the same as. */
export function LabelIcons({ label }: { label: Label }) {
  if (!label.worktree && !label.remote && !label.mark) return <Icon name={LABEL_ICONS[label.kind]} />;
  return (
    <span className="icons">
      {label.worktree && <Icon name="worktree" />}
      <Icon name={LABEL_ICONS[label.kind]} />
      {label.remote && <Icon name={LABEL_ICONS.remote} />}
      {label.mark && <Icon name={label.mark.icon} />}
    </span>
  );
}

/** The name of a label, with the worktree and the upstream its icons stand for. */
export function labelTitle(label: Label): string {
  const lines = [label.name];
  if (label.worktree) lines.push(`In the worktree ${label.worktree}`);
  if (label.remote) lines.push(`Up to date with ${label.remote}`);
  if (label.mark) lines.push(label.mark.title);
  return lines.join('\n');
}

interface GraphProps {
  graph: GraphRow;
  width: number;
  height: number;
  /** The room each lane has: less than usual when the graph is squeezed. */
  lane: number;
  /** A dot for a commit, a ring for the uncommitted changes, a square for a stash. */
  node: 'commit' | 'merge' | 'worktree' | 'stash' | 'folded';
  /** While the history of a label stands out: which parts of the cell are in it. The others fade. */
  trace?: GraphTrace;
  /** For a commit not pushed yet, the upstream it would be pushed to: its dot has a ring around it. */
  unpushed?: string;
  /** For a commit whose parents were not fetched, in a shallow clone: its line goes on under its dot, dashed. */
  shallow?: boolean;
}

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

const Graph = memo(function Graph({ graph, width, height, lane, node, trace, unpushed, shallow }: GraphProps) {
  const mid = height / 2;
  const laneX = (index: number) => lane / 2 + 3 + index * lane;
  const x = laneX(graph.col);
  // Smaller dots where the lines are close, or they would sit on the lines beside them.
  const radius = lane < 11 ? 3 : 4;
  const nodeClass = trace && !trace.node ? ' faded' : '';
  return (
    <svg className="graph" width={width} height={height} aria-hidden="true">
      {graph.top.map((line, index) => {
        const from = laneX(line.from);
        const to = laneX(line.to);
        const d = from === to ? `M${from} 0V${mid}` : `M${from} 0C${from} ${mid} ${to} 0 ${to} ${mid}`;
        return <path key={`t${index}`} className={trace && !trace.top[index] ? 'faded' : undefined} d={d} stroke={laneColor(line.color)} strokeDasharray={line.dashed ? '3 3' : undefined} />;
      })}
      {graph.bottom.map((line, index) => {
        const from = laneX(line.from);
        const to = laneX(line.to);
        const d =
          from === to
            ? `M${from} ${mid}V${height}`
            : `M${from} ${mid}C${from} ${height} ${to} ${mid} ${to} ${height}`;
        return <path key={`b${index}`} className={trace && !trace.bottom[index] ? 'faded' : undefined} d={d} stroke={laneColor(line.color)} strokeDasharray={line.dashed ? '3 3' : undefined} />;
      })}
      {shallow && (
        <path className={nodeClass.trim() || undefined} d={`M${x} ${mid}V${height}`} stroke={laneColor(graph.color)} strokeDasharray="3 3">
          <title>{SHALLOW_LINE}</title>
        </path>
      )}
      {node === 'worktree' && <circle className={`hollow${nodeClass}`} cx={x} cy={mid} r={radius - 0.5} stroke={laneColor(graph.color)} />}
      {node === 'folded' && (
        <circle className={`hollow${nodeClass}`} cx={x} cy={mid} r={radius - 0.5} stroke={laneColor(graph.color)} strokeDasharray="2 1.5" />
      )}
      {node === 'stash' && (
        <rect className={`hollow${nodeClass}`} x={x - radius + 0.5} y={mid - radius + 0.5} width={radius * 2 - 1} height={radius * 2 - 1} rx={1} stroke={laneColor(graph.color)} />
      )}
      {node === 'commit' && <circle className={nodeClass.trim() || undefined} cx={x} cy={mid} r={radius} fill={laneColor(graph.color)} />}
      {node === 'merge' && <circle className={`hollow${nodeClass}`} cx={x} cy={mid} r={radius - 0.75} stroke={laneColor(graph.color)} strokeWidth={1.5} />}
      {(node === 'commit' || node === 'merge') && unpushed && (
        <circle className={`unpushed${nodeClass}`} cx={x} cy={mid} r={radius + 2.5} stroke={laneColor(graph.color)}>
          <title>{`Not pushed to ${unpushed}`}</title>
        </circle>
      )}
    </svg>
  );
});

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

/** The open pull request of a branch, on its label: its number, in the color of its checks. A click opens it. */
function PullBadge({ request, branch }: { request: PullRequest; branch: string }) {
  return (
    <span
      className={`pull ${request.checks ?? ''}`}
      role="link"
      title={`${pullTitle(request)}\nClick to open it on ${serviceName(request.service)}`}
      onClick={(event) => {
        // Not the selection of the commit as well.
        event.stopPropagation();
        post({ type: 'pullRequest', branch });
      }}
      onDoubleClick={(event) => event.stopPropagation()}
    >
      <Icon name={request.draft ? 'git-pull-request-draft' : 'git-pull-request'} />
      {pullBadgeText(request)}
    </span>
  );
}

/**
 * What a row of the list can do, the same object for the whole life of the list, so that a row draws again only when
 * what it shows changed. Each calls what the list was last given.
 */
interface RowActions {
  select(hash: string, mode: SelectMode): void;
  open(hash: string): void;
  rowMenu(event: MouseEvent, row: Row): void;
  labelMenu(event: MouseEvent, label: Label, row: Row): void;
  labelOpen(label: Label): (() => void) | undefined;
  /** Answers from the label and the row alone, since a row asks it only when it draws again. */
  dragSource(label: Label | null, row: Row): DragSource | null;
  canDrop(row: Row): boolean;
  drop(event: MouseEvent, row: Row): void;
  trace(hash: string | null): void;
  fold(merge: string): void;
  unfold(merge: string): void;
  /** The row a drag is over, which shows where it would be dropped. */
  setDropTarget(update: (current: string | null) => string | null): void;
  /** The pointer came to a label of a row that does not show all of them, or to their list: it opens after a while. */
  restOnLabels(hash: string, label: HTMLElement | null): void;
  /** The pointer left a label of such a row, or their list: it closes unless the pointer comes back. */
  leaveLabels(label: HTMLElement | null): void;
  /** Closes the list of the labels of a row, after a label of it was opened or its menu. */
  closeLabels(): void;
}

/** What a click with the keys held does to the selection. */
const selectMode = (event: MouseEvent): SelectMode => (event.shiftKey ? 'range' : event.ctrlKey || event.metaKey ? 'toggle' : 'single');

interface LabelChipProps {
  label: Label;
  row: Row;
  actions: RowActions;
  /** The width it is given when it is shortened beside `+N`. */
  width?: number;
  /** Whether the row does not show all its labels, and the pointer resting on this one opens their list. */
  opensList: boolean;
  /** In the list of all the labels of the commit, where a click selects the commit and a double click or a menu closes the list. */
  inList?: boolean;
}

/** A label of a commit, on its row or in the list of all of them, with what the pointer does on it. */
function LabelChip({ label, row, actions, width, opensList, inList }: LabelChipProps) {
  return (
    <span
      role="listitem"
      className={`label ${label.kind}${label.current ? ' current' : ''}`}
      title={labelTitle(label)}
      style={width === undefined ? undefined : { maxWidth: width }}
      onContextMenu={(event) => {
        actions.labelMenu(event, label, row);
        if (inList) actions.closeLabels();
      }}
      onMouseEnter={(event) => {
        actions.trace(row.hash);
        if (opensList) actions.restOnLabels(row.hash, event.currentTarget);
      }}
      onMouseLeave={(event) => {
        actions.trace(null);
        if (opensList) actions.leaveLabels(event.currentTarget);
      }}
      onClick={inList ? (event) => actions.select(row.hash, selectMode(event)) : undefined}
      onDoubleClick={(event) => {
        const open = actions.labelOpen(label);
        if (!open && !inList) return;
        // Not the commit's own double click as well.
        event.stopPropagation();
        if (open) open();
        else actions.open(row.hash);
        if (inList) actions.closeLabels();
      }}
      {...dragProps(actions, label, row)}
    >
      <LabelIcons label={label} />
      <span className="text">{label.name}</span>
      {label.pull && <PullBadge {...label.pull} />}
    </span>
  );
}

interface LabelListProps {
  row: Row;
  labels: Label[];
  /** The label the pointer rested on, which the list never covers. */
  anchor: DOMRect;
  actions: RowActions;
}

/**
 * The list of all the labels of a commit whose row does not show them all, under the label the pointer rested on, or
 * above it when there is no room under it and more above; taller than the room, it scrolls (`graph.label-overflow`).
 */
function LabelList({ row, labels, anchor, actions }: LabelListProps) {
  const element = useRef<HTMLDivElement>(null);
  const [place, setPlace] = useState<CSSProperties>({ left: anchor.left, top: anchor.bottom + 2, visibility: 'hidden' });
  useLayoutEffect(() => {
    const list = element.current!;
    const below = window.innerHeight - anchor.bottom - 6;
    const above = anchor.top - 6;
    const height = list.scrollHeight;
    const under = height <= below || below >= above;
    const room = under ? below : above;
    setPlace({
      left: Math.max(0, Math.min(anchor.left, window.innerWidth - list.offsetWidth - 4)),
      top: under ? anchor.bottom + 2 : anchor.top - 2 - Math.min(height, room),
      maxHeight: room,
    });
  }, [anchor]);
  return (
    <div
      ref={element}
      className="label-list"
      role="list"
      aria-label="Labels"
      style={{ ...place, '--lane': laneColor(row.graph.color) } as CSSProperties}
      onMouseEnter={() => actions.restOnLabels(row.hash, null)}
      onMouseLeave={() => actions.leaveLabels(null)}
    >
      {labels.map((label) => (
        <div key={`${label.kind}:${label.name}`} className="message">
          <LabelChip label={label} row={row} actions={actions} opensList={false} inList />
        </div>
      ))}
    </div>
  );
}

/** What makes a row or one of its labels draggable, and what it carries. */
function dragProps(actions: RowActions, label: Label | null, row: Row) {
  const source = actions.dragSource(label, row);
  return {
    draggable: source !== null,
    onDragStart: (event: DragEvent) => source && startDrag(event, source),
    onDragEnd: () => {
      endDrag();
      actions.setDropTarget(() => null);
    },
  };
}

/** What the row of a commit shows; each is the same from one drawing of the list to the next while it is unchanged. */
interface CommitRowProps {
  row: Row;
  index: number;
  height: number;
  graphWidth: number;
  lane: number;
  tall: boolean;
  selected: boolean;
  /** A search is shown and did not find this commit. */
  dimmed: boolean;
  /** While a history stands out, which parts of this row's cell of the graph are in it. */
  trace: GraphTrace | undefined;
  dropTarget: boolean;
  labels: Label[] | undefined;
  /** The upstream the commit would be pushed to, when it is not pushed yet. */
  unpushed: string | undefined;
  /** Whether the parents of the commit were not fetched, in a shallow clone. */
  shallow: boolean;
  /** The name of the time further back that starts at this commit. */
  period: string | undefined;
  /** Whether the commits of this merge are folded, or could be. */
  fold: 'folded' | 'foldable' | null;
  showAuthor: boolean;
  showDate: boolean;
  showHash: boolean;
  oneLane: boolean;
  /** How wide the row is but for the cell of the graph: where its message, its labels and its columns go. None before the list is measured. */
  lineWidth: number;
  /** How wide the columns at the right end of a row on one line are, with their gaps. */
  columnsWidth: number;
  /** The fonts the labels and the words are measured in. */
  fonts: RowFonts | null;
  actions: RowActions;
}

/**
 * Which labels of a commit its row shows whole (`graph.label-overflow`): on one line, in the room the message leaves,
 * which keeps up to three fifths of it; on two lines, in the room the author and the date leave on the line under the
 * message, which keep up to a third of it. All of them while the row is not measured yet.
 */
function rowLabels(props: CommitRowProps, labels: Label[], foldable: boolean): FittedLabels | null {
  const { row, tall, fonts } = props;
  if (!labels.length || !fonts || props.lineWidth <= 0) return null;
  const worktree = row.hash === WORKTREE;
  let room: number;
  if (tall) {
    const words = worktree ? ['Not committed yet'] : [props.showAuthor ? row.author : '', props.showDate ? relativeTime(row.date) : ''].filter(Boolean);
    const kept = words.reduce((sum, word) => sum + textWidth(word, fonts.small), 0) + Math.max(0, words.length - 1) * TALL_LABEL_GAP;
    room = roomBesideAuthor(props.lineWidth, kept);
  } else {
    const line = props.lineWidth - (worktree ? 0 : props.columnsWidth) - (foldable ? FOLD_WIDTH : 0);
    room = roomBesideMessage(line, textWidth(row.subject, fonts.text));
  }
  const height = tall ? TALL_LABEL_HEIGHT : LABEL_HEIGHT;
  return fitLabels(
    labels.map((label) => labelWidth(label, fonts)),
    (count) => countWidth(count, fonts),
    { width: room, least: SHORTENED_LEAST * height, gap: tall ? TALL_LABEL_GAP : LABEL_GAP },
  );
}

/** The button of a merge that folds the commits it brought in into one row, or shows them again. */
function FoldButton({ hash, folded, actions }: { hash: string; folded: boolean; actions: RowActions }) {
  return (
    <button
      className={folded ? 'fold folded' : 'fold'}
      title={folded ? 'Show the commits it merged again' : 'Fold the commits it merged into one row'}
      aria-label={folded ? 'Show the commits it merged again' : 'Fold the commits it merged into one row'}
      onClick={(event) => {
        // Not the selection of the commit as well.
        event.stopPropagation();
        if (folded) actions.unfold(hash);
        else actions.fold(hash);
      }}
      onDoubleClick={(event) => event.stopPropagation()}
    >
      <Icon name={folded ? 'chevron-right' : 'chevron-down'} />
    </button>
  );
}

/** The row of one commit, or of the uncommitted changes, drawn again only when what it shows changed. */
const CommitRow = memo(function CommitRow(props: CommitRowProps) {
  const { row, index, height, tall, actions, trace, fold } = props;
  const worktree = row.hash === WORKTREE;
  const classes = tall ? ['row', 'tall'] : ['row'];
  if (props.selected) classes.push('selected');
  if (worktree) classes.push('worktree');
  if (row.stash) classes.push('stash');
  if (props.dimmed) classes.push('dimmed');
  if (trace && !trace.node) classes.push('unreached');
  if (props.dropTarget) classes.push('drop-target');
  const all = props.labels ?? [];
  // As many as fit whole, in their order, then `+N` for the others, whose list opens under the pointer.
  const fitted = rowLabels(props, all, !!fold);
  const opensList = !!fitted?.hidesSome;
  const shown = fitted ? all.slice(0, fitted.named) : all;
  const chips = all.length > 0 && (
    <span className="labels" role="list" aria-label="Labels">
      {shown.map((label, place) => (
        <LabelChip
          key={`${label.kind}:${label.name}`}
          label={label}
          row={row}
          actions={actions}
          // Shortened beside `+N` when not even the first fits whole.
          width={fitted?.shortened && place === 0 ? fitted.widths[0] : undefined}
          opensList={opensList}
        />
      ))}
      {fitted && fitted.counted > 0 && (
        // No tooltip: the list names them (Q646).
        <span
          role="listitem"
          className="label more"
          onMouseEnter={(event) => actions.restOnLabels(row.hash, event.currentTarget)}
          onMouseLeave={(event) => actions.leaveLabels(event.currentTarget)}
        >
          <span className="text">+{fitted.counted}</span>
        </span>
      )}
    </span>
  );
  const foldButton = fold && <FoldButton hash={row.hash} folded={fold === 'folded'} actions={actions} />;
  return (
    <div
      className={classes.join(' ')}
      style={{ top: index * height, height }}
      role="option"
      aria-selected={props.selected}
      onClick={(event) => actions.select(row.hash, event.shiftKey ? 'range' : event.ctrlKey || event.metaKey ? 'toggle' : 'single')}
      onDoubleClick={worktree ? undefined : () => actions.open(row.hash)}
      onContextMenu={(event) => actions.rowMenu(event, row)}
      {...dragProps(actions, null, row)}
      onDragOver={(event) => {
        if (!actions.canDrop(row)) return;
        event.preventDefault();
        event.dataTransfer.dropEffect = 'copy';
        actions.setDropTarget(() => row.hash);
      }}
      onDragLeave={() => actions.setDropTarget((current) => (current === row.hash ? null : current))}
      onDrop={(event) => {
        actions.setDropTarget(() => null);
        actions.drop(event, row);
        endDrag();
      }}
    >
      {props.period !== undefined && (
        <span className="period" style={{ left: props.graphWidth }}>
          <span>{props.period}</span>
        </span>
      )}
      <Graph
        graph={row.graph}
        width={props.graphWidth}
        height={height}
        lane={props.lane}
        node={worktree ? 'worktree' : row.stash ? 'stash' : props.oneLane && row.parents.length > 1 ? 'merge' : 'commit'}
        trace={trace}
        unpushed={props.unpushed}
        shallow={props.shallow}
      />
      {tall ? (
        <div className="lines">
          <div className="subject-line">
            <span className="subject">{row.subject}</span>
            {foldButton}
          </div>
          {/* The labels start the line under the message, before the author and the date (D86). */}
          <div className="message" style={{ '--lane': laneColor(row.graph.color) } as CSSProperties}>
            {chips}
            {worktree ? (
              <span className="who">Not committed yet</span>
            ) : (
              <>
                {props.showAuthor && <span className="who">{row.author}</span>}
                {props.showDate && (
                  <span className="when" title={fullDate(row.date)}>
                    {relativeTime(row.date)}
                  </span>
                )}
              </>
            )}
          </div>
        </div>
      ) : (
        <div className="message" style={{ '--lane': laneColor(row.graph.color) } as CSSProperties}>
          {chips}
          <span className="subject">{row.subject}</span>
          {foldButton}
        </div>
      )}
      {!worktree && !tall && (
        <>
          {props.showAuthor && <div className="author">{row.author}</div>}
          {props.showDate && (
            <div className="date" title={fullDate(row.date)}>
              {relativeTime(row.date)}
            </div>
          )}
          {props.showHash && <div className="hash">{shortHash(row.hash)}</div>}
        </>
      )}
    </div>
  );
});

interface FoldedRowProps {
  row: ShownRow;
  /** The merge whose commits it stands for. */
  merge: string;
  index: number;
  height: number;
  graphWidth: number;
  lane: number;
  tall: boolean;
  /** How many commits it stands for, and of which branch. */
  what: string;
  trace: GraphTrace | undefined;
  actions: RowActions;
}

/** The one row of the commits a folded merge brought in; a click shows them again. */
const FoldedRow = memo(function FoldedRow({ row, merge, index, height, graphWidth, lane, tall, what, trace, actions }: FoldedRowProps) {
  const classes = ['row', 'folded'];
  if (tall) classes.push('tall');
  if (trace && !trace.node) classes.push('unreached');
  return (
    <div
      className={classes.join(' ')}
      style={{ top: index * height, height }}
      role="option"
      aria-selected={false}
      title="Click to show them again"
      onClick={() => actions.unfold(merge)}
      onContextMenu={(event) => event.preventDefault()}
    >
      <Graph graph={row.graph} width={graphWidth} height={height} lane={lane} node="folded" trace={trace} />
      <div className="message">
        <Icon name="unfold" />
        <span className="subject">{what}</span>
      </div>
    </div>
  );
});

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
  // Coming back to a repository that kept what it showed, the commits are where they were scrolled to.
  useKept(props.root, props.keepOpen, 'scroll', scrollTop, (top) => {
    if (scroller.current) scroller.current.scrollTop = top;
    setScrollTop(top);
  });

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
        {shallowEnd && (
          <div className="row shallow-end" style={{ top: rows.length * rowHeight, height: rowHeight }}>
            <span className="said">{SHALLOW_END}</span>
            <button className="link" onClick={() => post({ type: 'action', action: { kind: 'deepen' } })}>
              Load more history
            </button>
            <button className="link" onClick={() => post({ type: 'action', action: { kind: 'unshallow' } })}>
              Load the whole history
            </button>
          </div>
        )}
      </div>
      {listedLabels && createPortal(<LabelList {...listedLabels} actions={actions} />, document.body)}
    </div>
  );
}

/** What the list of the labels of a row is: where it opens, and when it closes, from where the pointer rests (`graph.label-overflow`). */
function labelListHover(show: (list: { hash: string; anchor: DOMRect } | null) => void) {
  let timer: ReturnType<typeof setTimeout> | undefined;
  let open: string | null = null;
  // The label the pointer is on, and the one the list was closed on with `Escape` or a press: that one opens nothing until the pointer leaves it.
  let over: HTMLElement | null = null;
  let blocked: HTMLElement | null = null;
  const set = (list: { hash: string; anchor: DOMRect } | null) => {
    open = list?.hash ?? null;
    show(list);
  };
  return {
    rest(hash: string, label: HTMLElement | null) {
      clearTimeout(timer);
      // On the list itself, or on another label of the row it is of, it stays.
      if (!label || open === hash) return;
      over = label;
      if (open) set(null);
      if (label !== blocked) timer = setTimeout(() => set({ hash, anchor: label.getBoundingClientRect() }), LIST_AFTER);
    },
    leave(label: HTMLElement | null) {
      clearTimeout(timer);
      if (label) {
        over = null;
        if (label === blocked) blocked = null;
      }
      if (open) timer = setTimeout(() => set(null), LIST_CLOSE_AFTER);
    },
    close(blocking: boolean) {
      clearTimeout(timer);
      if (blocking) blocked = over;
      if (open) set(null);
    },
    dispose() {
      clearTimeout(timer);
    },
  };
}

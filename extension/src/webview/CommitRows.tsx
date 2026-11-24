/**
 * How a row of the list of commits is drawn: its cell of the graph, its message, its labels and its columns, the row
 * of the commits of a folded merge, and the last row of a shallow clone (`graph.shallow-end`).
 */
import { memo, type CSSProperties } from 'react';
import type { GraphTrace, ShownRow } from '../shared/graph';
import { WORKTREE, type GraphRow, type Row } from '../shared/protocol';
import { SHORTENED_LEAST, fitLabels, roomBesideAuthor, roomBesideMessage, type FittedLabels } from '../shared/labelFit';
import { Icon } from './Icon';
import { endDrag } from './drag';
import { fullDate, laneColor, relativeTime, shortHash } from './util';
import { post } from './vscode';
import { countWidth, labelWidth, textWidth, type RowFonts } from './labelWidths';
import { LabelChip, dragProps, type Label, type RowActions } from './CommitLabels';

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
/** The arrow that folds the commits of a merge, after its message, and the gap before it. */
const FOLD_WIDTH = 18 + LABEL_GAP;

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

export const Graph = memo(function Graph({ graph, width, height, lane, node, trace, unpushed, shallow }: GraphProps) {
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
export const CommitRow = memo(function CommitRow(props: CommitRowProps) {
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
export const FoldedRow = memo(function FoldedRow({ row, merge, index, height, graphWidth, lane, tall, what, trace, actions }: FoldedRowProps) {
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

/** The last row of the commits of a shallow clone, once they are all read, with what loads more of its history (`graph.shallow-end`). */
export function ShallowEnd({ top, height }: { top: number; height: number }) {
  return (
    <div className="row shallow-end" style={{ top, height }}>
      <span className="said">{SHALLOW_END}</span>
      <button className="link" onClick={() => post({ type: 'action', action: { kind: 'deepen' } })}>
        Load more history
      </button>
      <button className="link" onClick={() => post({ type: 'action', action: { kind: 'unshallow' } })}>
        Load the whole history
      </button>
    </div>
  );
}

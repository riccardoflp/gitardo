import { useEffect, useLayoutEffect, useMemo, useRef, useState, type ReactNode } from 'react';
import { Columns as Resizer } from '../flow/webview/columns';
import type { LineOrigin, ProposalLines } from '../shared/proposalLines';
import type { ReviewConflict } from '../shared/resolution';
import { Code, PART } from './conflictCode';
import type { Token } from './syntax';

/** What a side of the conflicts is: its place in the operation, and the branch or commit it comes from. */
export interface Side {
  role: 'Current' | 'Incoming';
  label: string;
  name: string | null;
  subject: string | null;
}

/** The label over the lines of a side: Current or Incoming, the branch or commit, and the subject of the commit. */
export function SideLabel({ side }: { side: Side }) {
  const title = `${side.label}${side.name ? `: ${side.name}` : ''}${side.subject ? `, “${side.subject}”` : ''}`;
  return (
    <div className="side-label" title={title}>
      {side.role} · {side.name ? <span className="side-name">{side.name}</span> : side.label}
      {side.subject && <span className="side-subject"> {side.subject}</span>}
    </div>
  );
}

/**
 * The widths of the columns, the same for every conflict of the page: dragging the gutter of one conflict moves the
 * gutters of all of them.
 */
export const widths: { now?: number[]; columns: Set<Resizer> } = { columns: new Set() };

/** Where a line is drawn, from the top of the columns. */
interface Place {
  top: number;
  bottom: number;
}

/** The bands of a gutter: each joins lines of the column at its left to the same lines in the column at its right. */
function Bands(props: { pairs: [number, number][]; left: Place[]; right: Place[]; side: 'ours' | 'theirs' }) {
  const { pairs, left, right, side } = props;
  const width = 24;
  const runs: [number, number, number, number][] = [];
  for (const [i, j] of pairs) {
    const last = runs[runs.length - 1];
    if (last && last[1] === i - 1 && last[3] === j - 1) {
      last[1] = i;
      last[3] = j;
    } else runs.push([i, i, j, j]);
  }
  const middle = width / 2;
  // The bands take the height of the row, and never set it: the lines they join are measured in that row.
  return (
    <div className="bands">
      <svg width={width} aria-hidden="true">
        {runs.map(([fromLeft, toLeft, fromRight, toRight]) => {
          const [a, b, c, d] = [left[fromLeft]?.top, left[toLeft]?.bottom, right[fromRight]?.top, right[toRight]?.bottom];
          if ([a, b, c, d].some((y) => y === undefined)) return null;
          return (
            <path
              key={`${fromLeft}:${fromRight}`}
              className={side}
              d={`M0 ${a} C${middle} ${a} ${middle} ${c} ${width} ${c} L${width} ${d} C${middle} ${d} ${middle} ${b} 0 ${b} Z`}
            />
          );
        })}
      </svg>
    </div>
  );
}

/** The class a line of a proposal gets for where it comes from, which the cards and the columns color the same way. */
export const ORIGIN_CLASS: Record<LineOrigin, string> = {
  ours: 'from-ours',
  theirs: 'from-theirs',
  both: 'from-both',
  base: 'from-base',
  new: 'new',
  blank: '',
};

/** The column a line under the pointer is in, and which line of it. */
type Pointed = { column: 'ours' | 'proposed' | 'theirs'; index: number } | null;

/**
 * A conflict in three columns, as the Flow Merge shows a file: the current side, what is proposed, and the incoming
 * side, with bands that join each line proposed to the side it comes from, between the lines of the file around them.
 * The bands are also the sashes that make a column wider. The lines wrap in a narrow column, and the line under the
 * pointer lights up with the lines it is joined to.
 */
export function Columns(props: {
  conflict: ReviewConflict;
  sides: [Side, Side];
  accepted: boolean;
  colors?: (Token[] | null)[][];
  /** Where each proposed line comes from, when something is proposed. */
  lines: ProposalLines | null;
  /** The class and the tooltip of each proposed line, as the card has them. */
  kind: (line: string, index: number) => string;
  title: (line: string, index: number) => string | undefined;
  middle: ReactNode;
  /** Keeps the widths once a gutter is let go. */
  onWidths(weights: number[]): void;
}) {
  const { conflict, sides, accepted, colors, lines, kind, title, middle, onWidths } = props;
  const { proposal } = conflict;
  const fromOurs = lines?.fromOurs ?? [];
  const toTheirs = lines?.toTheirs ?? [];
  const taken = useMemo(() => ({ ours: new Set(fromOurs.map(([i]) => i)), theirs: new Set(toTheirs.map(([, j]) => j)) }), [lines]);
  const [pointed, setPointed] = useState<Pointed>(null);
  // The lines joined to the one under the pointer, in each column.
  const linked = useMemo(() => {
    const found = { ours: new Set<number>(), proposed: new Set<number>(), theirs: new Set<number>() };
    if (!pointed) return found;
    found[pointed.column].add(pointed.index);
    const middle =
      pointed.column === 'proposed'
        ? [pointed.index]
        : pointed.column === 'ours'
          ? fromOurs.filter(([i]) => i === pointed.index).map(([, j]) => j)
          : toTheirs.filter(([, j]) => j === pointed.index).map(([i]) => i);
    for (const index of middle) {
      found.proposed.add(index);
      fromOurs.filter(([, j]) => j === index).forEach(([i]) => found.ours.add(i));
      toTheirs.filter(([i]) => i === index).forEach(([, j]) => found.theirs.add(j));
    }
    return found;
  }, [pointed, lines]);

  const grid = useRef<HTMLDivElement>(null);
  const [places, setPlaces] = useState<{ ours: Place[]; proposed: Place[]; theirs: Place[] }>({ ours: [], proposed: [], theirs: [] });
  useLayoutEffect(() => {
    const element = grid.current;
    if (!element) return;
    // Lines wrap in a narrow column: where each one is drawn is measured, again whenever the columns change size.
    const measure = () => {
      const top = element.getBoundingClientRect().top;
      const of = (column: string) =>
        [...element.querySelectorAll(`:scope > .side.${column} .code-lines.body .code-line`)].map((line) => {
          const box = line.getBoundingClientRect();
          return { top: box.top - top, bottom: box.bottom - top };
        });
      const next = { ours: of('ours'), proposed: of('proposed'), theirs: of('theirs') };
      setPlaces((now) => (JSON.stringify(now) === JSON.stringify(next) ? now : next));
    };
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(element);
    return () => observer.disconnect();
  }, [conflict.region.ours, conflict.region.theirs, proposal?.text]);

  const kept = useRef(onWidths);
  kept.current = onWidths;
  useEffect(() => {
    const element = grid.current;
    if (!element) return;
    const columns = ['ours', 'proposed', 'theirs'].map((column) => {
      const host = element.querySelector<HTMLElement>(`:scope > .side.${column}`)!;
      return { host, label: host.querySelector<HTMLElement>('.side-label')! };
    });
    const resizer: Resizer = new Resizer(columns, [...element.querySelectorAll<HTMLElement>(':scope > .bands')], widths.now, (weights, done) => {
      widths.now = weights;
      for (const other of widths.columns) if (other !== resizer) other.set(weights);
      if (done) kept.current(weights);
    });
    widths.columns.add(resizer);
    return () => {
      widths.columns.delete(resizer);
      resizer.dispose();
    };
  }, []);

  const lit = (column: keyof typeof linked, index: number) => (linked[column].has(index) ? ' linked' : '');
  const sideKind = (column: 'ours' | 'theirs') => (_line: string, index: number) =>
    `${!proposal ? '' : taken[column].has(index) ? 'taken' : 'left-out'}${lit(column, index)}`;
  const hover = (column: 'ours' | 'proposed' | 'theirs') => (index: number | null) => setPointed(index === null ? null : { column, index });
  // The lines of the file around the conflict, the same in each column, for where the lines go.
  const around = (text: string, part: number) => <Code text={text} colors={colors?.[part]} className="around" />;
  return (
    <div className="columns" ref={grid}>
      <div className="side ours">
        <SideLabel side={sides[0]} />
        {around(conflict.before, PART.before)}
        <Code text={conflict.region.ours} colors={colors?.[PART.regionOurs]} className="body" kind={sideKind('ours')} onHover={proposal ? hover('ours') : undefined} />
        {around(conflict.after, PART.after)}
      </div>
      <Bands pairs={fromOurs} left={places.ours} right={places.proposed} side="ours" />
      <div className={`side proposed${proposal ? ` ${proposal.source}` : ''}`}>
        <div className="side-label">{proposal ? (accepted ? 'Accepted' : 'Proposed') : 'Nothing proposed yet'}</div>
        {proposal ? (
          <>
            {around(conflict.before, PART.before)}
            <Code
              text={proposal.text}
              colors={colors?.[PART.proposal]}
              className="body"
              kind={(line, index) => `${kind(line, index)}${lit('proposed', index)}`}
              title={title}
              onHover={hover('proposed')}
            />
            {around(conflict.after, PART.after)}
          </>
        ) : (
          middle
        )}
      </div>
      <Bands pairs={toTheirs} left={places.proposed} right={places.theirs} side="theirs" />
      <div className="side theirs">
        <SideLabel side={sides[1]} />
        {around(conflict.before, PART.before)}
        <Code text={conflict.region.theirs} colors={colors?.[PART.regionTheirs]} className="body" kind={sideKind('theirs')} onHover={proposal ? hover('theirs') : undefined} />
        {around(conflict.after, PART.after)}
      </div>
    </div>
  );
}

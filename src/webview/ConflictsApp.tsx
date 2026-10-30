import { useEffect, useLayoutEffect, useMemo, useRef, useState, type CSSProperties, type ReactNode } from 'react';
import { Columns as Resizer } from '../flow/webview/columns';
import type { Hunk } from '../shared/protocol';
import type { Asking, ConflictRef, ConflictReview, Proposal, ReviewConflict, ReviewFile } from '../shared/resolution';
import { Piece } from './FileChanges';
import { FileIcon } from './FileIcon';
import { Icon } from './Icon';
import { useColors, type Token } from './syntax';
import { loadUiState, onMessage, post, saveUiState } from './vscode';

/**
 * What the page remembers while it is hidden: the conflicts whose proposal was accepted, by file and key, whether
 * they are shown in columns, and how wide the columns were made.
 */
interface Remembered {
  accepted: string[];
  columns?: boolean;
  widths?: number[];
  /** The file chosen in the list, or none for all of them, and which of their conflicts are shown. */
  file?: string | null;
  filter?: Filter;
}

/** Which conflicts are shown: all of them, those not accepted yet, those with nothing proposed, or those from the AI. */
type Filter = 'all' | 'open' | 'nothing' | 'ai';

const FILTERS: { filter: Filter; label: string; title: string }[] = [
  { filter: 'all', label: 'All', title: 'Show every conflict' },
  { filter: 'open', label: 'Not accepted', title: 'Show the conflicts whose resolution is not accepted yet' },
  { filter: 'nothing', label: 'Nothing proposed', title: 'Show the conflicts with nothing proposed, to resolve by hand or to ask the AI about' },
  { filter: 'ai', label: 'From the AI', title: 'Show the conflicts the AI proposed a resolution for, to look at before you accept them' },
];

/** Whether a conflict is one the filter shows. */
function shows(filter: Filter, conflict: ReviewConflict, accepted: boolean): boolean {
  if (filter === 'open') return !accepted;
  if (filter === 'nothing') return !conflict.proposal;
  if (filter === 'ai') return conflict.proposal?.source === 'ai';
  return true;
}

/** What a side of the conflicts is: its place in the operation, and the branch or commit it comes from. */
interface Side {
  role: 'Current' | 'Incoming';
  label: string;
  name: string | null;
  subject: string | null;
}

/**
 * The widths of the columns, the same for every conflict of the page: dragging the gutter of one conflict moves the
 * gutters of all of them.
 */
const widths: { now?: number[]; columns: Set<Resizer> } = { columns: new Set() };

const idOf = (path: string, key: string) => `${path}\0${key}`;
const linesOf = (text: string) => (text ? text.replace(/\r?\n$/, '').split(/\r?\n/) : []);
const plural = (count: number, one: string, many = `${one}s`) => `${count} ${count === 1 ? one : many}`;

/** Lines of code, in the colors of the theme once they are known, each with the class `kind` gives it. */
function Code(props: { text: string; colors?: (Token[] | null)[]; kind?: (line: string, index: number) => string; className?: string; caret?: boolean }) {
  const { text, colors, kind, className, caret } = props;
  const lines = linesOf(text);
  if (!lines.length && !caret) return <div className={`code-lines empty-lines ${className ?? ''}`}>Nothing: the lines are deleted</div>;
  // The lines are as wide as the longest of them, for their colors to go all the way when the columns scroll sideways.
  return (
    <div className={`code-lines ${className ?? ''}`}>
      <div className="code-width">
        {lines.map((line, index) => {
          const pieces = colors?.[index];
          return (
            <div key={index} className={`code-line ${kind?.(line, index) ?? ''}`}>
              {pieces ? pieces.map((token, piece) => <Piece key={piece} token={token} />) : line || ' '}
            </div>
          );
        })}
        {caret && <span className="caret" aria-hidden="true" />}
      </div>
    </div>
  );
}

/** The label over the lines of a side: Current or Incoming, the branch or commit, and the subject of the commit. */
function SideLabel({ side }: { side: Side }) {
  const title = `${side.label}${side.name ? `: ${side.name}` : ''}${side.subject ? `, “${side.subject}”` : ''}`;
  return (
    <div className="side-label" title={title}>
      {side.role} · {side.name ? <span className="side-name">{side.name}</span> : side.label}
      {side.subject && <span className="side-subject"> {side.subject}</span>}
    </div>
  );
}

/** Says where a proposal comes from, and how sure the language model is of it. */
function Source({ proposal }: { proposal: Proposal }) {
  if (proposal.source === 'rules') {
    return (
      <span className="source rules" title="Every line of it is a line of one of the two sides">
        <Icon name="wand" />
        Rules
      </span>
    );
  }
  return (
    <span className={`source ai ${proposal.confidence ?? ''}`} title="Written by the language model: look at it before you accept it">
      <Icon name="sparkle" />
      AI{proposal.model ? ` · ${proposal.model}` : ''}
      {proposal.confidence ? ` · ${proposal.confidence} confidence` : ''}
    </span>
  );
}

/** What the page shows while the language model is asked about a conflict. */
function AskingState({ asking, onRetry }: { asking: Asking; onRetry(): void }) {
  if (asking.state === 'waiting') {
    return (
      <div className="asking waiting" role="status">
        <div className="asking-label">
          <Icon name="sparkle" />
          Waiting for its turn…
        </div>
        <div className="skeleton" aria-hidden="true">
          <span />
          <span />
          <span />
        </div>
      </div>
    );
  }
  if (asking.state === 'thinking') {
    return (
      <div className="asking thinking" role="status">
        <div className="asking-label">
          <span className="orbit" aria-hidden="true">
            <Icon name="sparkle" />
          </span>
          {asking.text ? 'The AI is writing a resolution…' : 'The AI is reading the conflict…'}
        </div>
        {asking.text ? (
          <Code text={asking.text} className="streaming" caret />
        ) : (
          <div className="skeleton" aria-hidden="true">
            <span />
            <span />
            <span />
          </div>
        )}
      </div>
    );
  }
  return (
    <div className="asking failed" role="alert">
      <Icon name="error" />
      <span className="grow">The AI could not propose a resolution: {asking.error}</span>
      <button className="button" onClick={onRetry}>
        <Icon name="refresh" />
        Ask again
      </button>
    </div>
  );
}

/** Which lines of `a` are lines of `b`, in the same order, as pairs of their indexes: as many as can be. */
function sameLines(a: string[], b: string[]): [number, number][] {
  // Past this, a conflict is too large to be worth pairing line by line: the columns show it without bands.
  if (!a.length || !b.length || a.length * b.length > 250_000) return [];
  const same = (one: string, other: string) => one.trim() === other.trim();
  const longest = Array.from({ length: a.length + 1 }, () => new Uint32Array(b.length + 1));
  for (let i = a.length - 1; i >= 0; i--) {
    for (let j = b.length - 1; j >= 0; j--) longest[i][j] = same(a[i], b[j]) ? longest[i + 1][j + 1] + 1 : Math.max(longest[i + 1][j], longest[i][j + 1]);
  }
  const pairs: [number, number][] = [];
  for (let i = 0, j = 0; i < a.length && j < b.length; ) {
    if (same(a[i], b[j])) pairs.push([i++, j++]);
    else if (longest[i + 1][j] >= longest[i][j + 1]) i++;
    else j++;
  }
  return pairs;
}

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

/**
 * A conflict in three columns, as the Flow Merge shows a file: the current side, what is proposed, and the incoming
 * side, with bands that join each line proposed to the side it comes from. The bands are also the sashes that make a
 * column wider, and the lines do not wrap: the three columns scroll sideways together.
 */
function Columns(props: {
  conflict: ReviewConflict;
  sides: [Side, Side];
  accepted: boolean;
  colors?: (Token[] | null)[][];
  middle: ReactNode;
  /** Keeps the widths once a gutter is let go. */
  onWidths(weights: number[]): void;
}) {
  const { conflict, sides, accepted, colors, middle, onWidths } = props;
  const { proposal } = conflict;
  const ours = linesOf(conflict.region.ours);
  const theirs = linesOf(conflict.region.theirs);
  const proposed = linesOf(proposal?.text ?? '');
  const fromOurs = useMemo(() => (proposal ? sameLines(ours, proposed) : []), [conflict.region.ours, proposal?.text]);
  const toTheirs = useMemo(() => (proposal ? sameLines(proposed, theirs) : []), [conflict.region.theirs, proposal?.text]);
  const taken = {
    ours: new Set(fromOurs.map(([i]) => i)),
    theirs: new Set(toTheirs.map(([, j]) => j)),
    byOurs: new Set(fromOurs.map(([, j]) => j)),
    byTheirs: new Set(toTheirs.map(([i]) => i)),
  };
  const grid = useRef<HTMLDivElement>(null);
  const [places, setPlaces] = useState<{ ours: Place[]; proposed: Place[]; theirs: Place[] }>({ ours: [], proposed: [], theirs: [] });
  useLayoutEffect(() => {
    const element = grid.current;
    if (!element) return;
    // Lines wrap in a narrow column: where each one is drawn is measured, again whenever the columns change size.
    const measure = () => {
      const top = element.getBoundingClientRect().top;
      const of = (column: string) =>
        [...element.querySelectorAll(`:scope > .side.${column} .code-line`)].map((line) => {
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
    // A column scrolled sideways takes the other two along. Each of those then says it scrolled too, which is not
    // passed on: it would bring back a column that could go further than they can.
    const following = new Set<Element>();
    const scrolled = (event: Event) => {
      const from = event.target;
      if (!(from instanceof HTMLElement) || !from.classList.contains('code-lines') || from.parentElement?.parentElement !== element) return;
      if (following.delete(from)) return;
      for (const other of element.querySelectorAll<HTMLElement>(':scope > .side > .code-lines')) {
        if (other === from || other.scrollLeft === from.scrollLeft) continue;
        const before = other.scrollLeft;
        other.scrollLeft = from.scrollLeft;
        if (other.scrollLeft !== before) following.add(other);
      }
    };
    element.addEventListener('scroll', scrolled, true);
    return () => {
      element.removeEventListener('scroll', scrolled, true);
      widths.columns.delete(resizer);
      resizer.dispose();
    };
  }, []);

  const kindOfProposed = (line: string, index: number) => {
    const [byOurs, byTheirs] = [taken.byOurs.has(index), taken.byTheirs.has(index)];
    if (byOurs && byTheirs) return 'from-both';
    if (byOurs) return 'from-ours';
    if (byTheirs) return 'from-theirs';
    return line.trim() ? 'new' : '';
  };
  return (
    <div className="columns" ref={grid}>
      <div className="side ours">
        <SideLabel side={sides[0]} />
        <Code text={conflict.region.ours} colors={colors?.[6]} kind={(_line, index) => (!proposal ? '' : taken.ours.has(index) ? 'taken' : 'left-out')} />
      </div>
      <Bands pairs={fromOurs} left={places.ours} right={places.proposed} side="ours" />
      <div className={`side proposed${proposal ? ` ${proposal.source}` : ''}`}>
        <div className="side-label">{proposal ? (accepted ? 'Accepted' : 'Proposed') : 'Nothing proposed yet'}</div>
        {proposal ? <Code text={proposal.text} colors={colors?.[3]} kind={kindOfProposed} /> : middle}
      </div>
      <Bands pairs={toTheirs} left={places.proposed} right={places.theirs} side="theirs" />
      <div className="side theirs">
        <SideLabel side={sides[1]} />
        <Code text={conflict.region.theirs} colors={colors?.[7]} kind={(_line, index) => (!proposal ? '' : taken.theirs.has(index) ? 'taken' : 'left-out')} />
      </div>
    </div>
  );
}

/** One conflict: what each side has, what is proposed in its place, and what can be done with it. */
function Conflict(props: {
  file: ReviewFile;
  conflict: ReviewConflict;
  sides: [Side, Side];
  ai: boolean;
  accepted: boolean;
  busy: boolean;
  /** Whether it is shown in three columns rather than as the two sides over what is proposed. */
  columns: boolean;
  /** Whether only its first line is shown, as for a conflict whose proposal is accepted. */
  folded: boolean;
  onAccept(accepted: boolean): void;
  onFold(folded: boolean): void;
  onWidths(weights: number[]): void;
}) {
  const { file, conflict, sides, ai, accepted, busy, columns, folded, onAccept, onFold, onWidths } = props;
  const { proposal, asking } = conflict;
  const ref: ConflictRef = { path: file.path, key: conflict.key };
  // Each part is colored on its own: the lines before, the two sides, the proposal, the lines after, the base, and the
  // lines the proposal replaces on each side, which the columns show.
  const parts = [conflict.before, conflict.ours, conflict.theirs, proposal?.text ?? '', conflict.after, conflict.base ?? '', conflict.region.ours, conflict.region.theirs];
  const hunks = useMemo<Hunk[]>(() => parts.map((text) => ({ header: '', lines: linesOf(text).map((line) => ` ${line}`) })), parts);
  const colors = useColors(file.path, hunks, !folded);
  const [showBase, setShowBase] = useState(false);
  const known = useMemo(
    () => new Set([conflict.ours, conflict.theirs, conflict.base ?? ''].flatMap((text) => linesOf(text).map((line) => line.trim()))),
    [conflict.ours, conflict.theirs, conflict.base],
  );
  const open = (editor: 'flowMerge' | 'mergeEditor' | 'text') => post({ type: 'openConflict', path: file.path, line: conflict.line, editor });
  const nothing = asking ? (
    <AskingState asking={asking} onRetry={() => post({ type: 'askAI', conflicts: [ref] })} />
  ) : (
    <p className="muted nothing">Nothing is proposed: the two sides changed the same lines in different ways.</p>
  );
  const accept = proposal && (
    <button
      className={`button${accepted ? ' accept on' : ' primary accept'}`}
      disabled={busy}
      title={accepted ? 'Take back the acceptance: the conflict is left as it is' : 'Put this in place of the conflict when the accepted resolutions are written'}
      onClick={() => onAccept(!accepted)}
    >
      <Icon name={accepted ? 'pass-filled' : 'check'} />
      {accepted ? 'Accepted' : 'Accept'}
    </button>
  );
  const className = `conflict${accepted ? ' accepted' : ''}${proposal ? ` proposed ${proposal.source}` : ''}${folded ? ' folded' : ''}`;
  const head = (
    <>
      <button className="button icon" title={folded ? 'Show this conflict' : 'Show only the first line of this conflict'} aria-expanded={!folded} onClick={() => onFold(!folded)}>
        <Icon name={folded ? 'chevron-right' : 'chevron-down'} />
      </button>
      <button className="link" title="Open the file at this line" onClick={() => open('text')}>
        Line {conflict.line}
      </button>
    </>
  );

  if (folded) {
    const first = linesOf(proposal?.text ?? conflict.ours).find((line) => line.trim()) ?? '';
    return (
      <article className={className} aria-label={`Conflict at line ${conflict.line}`}>
        <header>
          {head}
          <code className="grow first-line" title={proposal?.text}>
            {first.trim()}
          </code>
          {proposal && <Source proposal={proposal} />}
          {accept}
        </header>
      </article>
    );
  }

  return (
    <article className={className} aria-label={`Conflict at line ${conflict.line}`}>
      <header>
        {head}
        <span className="grow" />
        {conflict.base !== null && (
          <button className={`button icon${showBase ? ' toggled' : ''}`} title={showBase ? 'Hide the base' : 'Show what the lines were before either side changed them'} onClick={() => setShowBase(!showBase)}>
            <Icon name="history" />
          </button>
        )}
        {proposal && <Source proposal={proposal} />}
      </header>
      {showBase && conflict.base !== null && (
        <div className="side base">
          <div className="side-label">Base · before either side changed it</div>
          <Code text={conflict.base} colors={colors?.[5]} />
        </div>
      )}
      {columns ? (
        <>
          <Columns conflict={conflict} sides={sides} accepted={accepted} colors={colors} middle={nothing} onWidths={onWidths} />
          {proposal && (
            <p className="columns-reason">
              <span className="muted">{proposal.reason}</span>
              {proposal.newLines > 0 && (
                <span className="warning">
                  <Icon name="warning" />
                  {plural(proposal.newLines, 'line is', 'lines are')} new: neither side has {proposal.newLines === 1 ? 'it' : 'them'}.
                </span>
              )}
            </p>
          )}
        </>
      ) : (
        <>
          <div className="sides">
            <div className="side ours">
              <SideLabel side={sides[0]} />
              <Code text={conflict.ours} colors={colors?.[1]} />
            </div>
            <div className="side theirs">
              <SideLabel side={sides[1]} />
              <Code text={conflict.theirs} colors={colors?.[2]} />
            </div>
          </div>

          {proposal && (
            <div className={`proposal ${proposal.source}`} key={`${proposal.source}:${proposal.text}`}>
              <div className="side-label">
                {accepted ? 'Accepted' : 'Proposed'}
                <span className="muted"> · {proposal.reason}</span>
              </div>
              <Code text={conflict.before} colors={colors?.[0]} className="around" />
              <Code
                text={proposal.text}
                colors={colors?.[3]}
                className="resolved"
                kind={(line) => (proposal.source === 'ai' && line.trim() && !known.has(line.trim()) ? 'new' : '')}
              />
              <Code text={conflict.after} colors={colors?.[4]} className="around" />
              {proposal.newLines > 0 && (
                <p className="note">
                  <Icon name="warning" />
                  {plural(proposal.newLines, 'line is', 'lines are')} new: neither side has {proposal.newLines === 1 ? 'it' : 'them'}.
                </p>
              )}
            </div>
          )}
          {!proposal && asking && <AskingState asking={asking} onRetry={() => post({ type: 'askAI', conflicts: [ref] })} />}
          {!proposal && !asking && <p className="muted nothing">Nothing is proposed: the two sides changed the same lines in different ways.</p>}
        </>
      )}

      <footer>
        {accept}
        <button className="button" title="Resolve it in the Flow Merge, choosing for each change" onClick={() => open('flowMerge')}>
          <Icon name="git-merge" />
          Resolve by hand
        </button>
        <button className="button icon" title="Open it in the merge editor of VS Code" onClick={() => open('mergeEditor')}>
          <Icon name="split-horizontal" />
        </button>
        {ai && proposal?.source !== 'rules' && asking?.state !== 'waiting' && asking?.state !== 'thinking' && (
          <button
            className="button"
            disabled={busy}
            title={proposal ? 'Ask the language model again: its answer takes the place of this one' : 'Ask the language model how to resolve it'}
            onClick={() => post({ type: 'askAI', conflicts: [ref] })}
          >
            <Icon name="sparkle" />
            {proposal ? 'Ask again' : 'Ask the AI'}
          </button>
        )}
      </footer>
    </article>
  );
}

/** What the last resolutions written did, with the way back and the way on. */
function Report({ review }: { review: ConflictReview }) {
  const report = review.report;
  if (!report) return null;
  const left = review.files.reduce((sum, file) => sum + file.conflicts.length, 0);
  const done = !review.conflicted;
  return (
    <div className="report" role="status">
      <Icon name="pass" />
      <div className="grow">
        <strong>
          Wrote the {report.resolved === 1 ? 'resolution of 1 conflict' : `resolutions of ${report.resolved} conflicts`} in {plural(report.files, 'file')}
          {report.fromAI ? `, ${report.fromAI === report.resolved ? 'all' : report.fromAI} from the AI` : ''}.
        </strong>{' '}
        {report.staged ? `${plural(report.staged, 'file was', 'files were')} left with no conflict, and staged. ` : ''}
        {done
          ? `No file is left in conflict${review.operation ? `: the ${review.operation} can continue` : ''}.`
          : left
            ? `${plural(left, 'conflict is', 'conflicts are')} left to resolve.`
            : `${plural(review.conflicted, 'file is', 'files are')} still in conflict.`}
      </div>
      <button className="button" title="Put back the conflicts of the files written, as git left them" onClick={() => post({ type: 'action', action: { kind: 'undo' } })}>
        <Icon name="discard" />
        Undo
      </button>
      {done && review.operation && (
        <button className="button primary" title={`Commit what is staged and go on with the ${review.operation}`} onClick={() => post({ type: 'action', action: { kind: 'continue' } })}>
          <Icon name="debug-continue" />
          Continue the {review.operation}
        </button>
      )}
    </div>
  );
}

/** The bar that says how far the language model has gone, while it is asked. */
function Progress({ review }: { review: ConflictReview }) {
  const all = review.files.flatMap((file) => file.conflicts);
  const asked = all.filter((conflict) => conflict.asking && conflict.asking.state !== 'failed').length;
  if (!review.asking) return null;
  const answered = all.filter((conflict) => conflict.proposal?.source === 'ai').length;
  const total = asked + answered;
  return (
    <div className="ai-progress" role="status">
      <span className="orbit" aria-hidden="true">
        <Icon name="sparkle" />
      </span>
      <span className="grow">
        Asking the AI: {answered} of {plural(total, 'conflict')} answered
      </span>
      <button className="button" onClick={() => post({ type: 'stopAsking' })}>
        <Icon name="debug-stop" />
        Stop
      </button>
      <div className="bar" style={{ '--done': total ? answered / total : 0 } as CSSProperties} aria-hidden="true" />
    </div>
  );
}

/** Where a file of the review stands: how many of its conflicts are accepted, and whether some have nothing proposed. */
function FileState({ file, accepted }: { file: ReviewFile; accepted: (conflict: ReviewConflict) => boolean }) {
  const total = file.conflicts.length;
  const done = file.conflicts.filter(accepted).length;
  const nothing = file.conflicts.filter((conflict) => !conflict.proposal).length;
  const [icon, state, title] =
    done === total
      ? ['pass-filled', 'done', 'Every conflict of it is accepted']
      : nothing
        ? ['circle-filled', 'nothing', `${plural(nothing, 'conflict has', 'conflicts have')} nothing proposed`]
        : ['circle-outline', 'open', 'Something is proposed for each conflict: not all of it is accepted yet'];
  return (
    <span className={`file-state ${state}`} title={title}>
      {done}/{total}
      <Icon name={icon} />
    </span>
  );
}

/**
 * The files of the review, beside their conflicts: which ones are shown, and where each file stands. A file chosen
 * here is shown alone; the first entry shows them all again.
 */
function FileList(props: {
  review: ConflictReview;
  filter: Filter;
  selected: string | null;
  /** How many conflicts each filter shows, in the order of `FILTERS`. */
  counts: number[];
  accepted: (file: ReviewFile, conflict: ReviewConflict) => boolean;
  /** Whether a file has a conflict the filter shows. */
  listed: (file: ReviewFile) => boolean;
  onFilter(filter: Filter): void;
  onSelect(path: string | null): void;
}) {
  const { review, filter, selected, counts, accepted, listed, onFilter, onSelect } = props;
  const files = review.files.filter(listed);
  const total = counts[FILTERS.findIndex((each) => each.filter === filter)];
  return (
    <nav className="conflicts-nav" aria-label="Files in conflict">
      <div className="filters" role="radiogroup" aria-label="Which conflicts are shown">
        {FILTERS.map((each, index) => (
          <button key={each.filter} className={`filter${filter === each.filter ? ' on' : ''}`} role="radio" aria-checked={filter === each.filter} title={each.title} onClick={() => onFilter(each.filter)}>
            <span className="grow">{each.label}</span>
            <span className="count">{counts[index]}</span>
          </button>
        ))}
      </div>
      <div className="file-entries">
        <button className={`file-entry${selected === null ? ' on' : ''}`} aria-current={selected === null} title="Show the conflicts of every file" onClick={() => onSelect(null)}>
          <Icon name="files" />
          <span className="grow name">All files</span>
          <span className="file-state">{plural(total, 'conflict')}</span>
        </button>
        {files.map((file) => {
          const slash = file.path.lastIndexOf('/');
          return (
            <button key={file.path} className={`file-entry${selected === file.path ? ' on' : ''}`} aria-current={selected === file.path} title={file.path} onClick={() => onSelect(file.path)}>
              <FileIcon path={file.path} />
              <span className="grow name">
                {file.path.slice(slash + 1)}
                {slash > 0 && <span className="folder"> {file.path.slice(0, slash)}</span>}
              </span>
              <FileState file={file} accepted={(conflict) => accepted(file, conflict)} />
            </button>
          );
        })}
        {!files.length && <p className="muted none">No file has a conflict to show here.</p>}
      </div>
      {review.others.length > 0 && (
        <div className="file-entries others">
          <div className="others-label" title="Binary files, files a side deleted, and very large files are resolved in an editor, or by taking one side">
            Also in conflict
          </div>
          {review.others.map((file) => (
            <button key={file} className="file-entry" title={`${file}: open it in the merge editor`} onClick={() => post({ type: 'openConflict', path: file, line: 1, editor: 'mergeEditor' })}>
              <FileIcon path={file} />
              <span className="grow name">{file.slice(file.lastIndexOf('/') + 1)}</span>
              <Icon name="go-to-file" />
            </button>
          ))}
        </div>
      )}
    </nav>
  );
}

/** The tab where what is proposed for the conflicts is looked at, accepted and written. */
export function ConflictsApp() {
  // Undefined while it is read.
  const [review, setReview] = useState<ConflictReview | null>();
  const [error, setError] = useState<string>();
  const [busy, setBusy] = useState(false);
  const [accepted, setAccepted] = useState(() => new Set(loadUiState<Remembered>({ accepted: [] }).accepted));
  const [columns, setColumns] = useState(() => loadUiState<Remembered>({ accepted: [] }).columns === true);
  const [kept, setKept] = useState(() => loadUiState<Remembered>({ accepted: [] }).widths);
  widths.now ??= kept;
  /**
   * The conflicts and files folded or unfolded by hand, by id and by path. The others are folded once accepted: a
   * conflict when its proposal is, a file when all of its conflicts are.
   */
  const [folds, setFolds] = useState(() => new Map<string, boolean>());
  const [selected, setSelected] = useState(() => loadUiState<Remembered>({ accepted: [] }).file ?? null);
  const [filter, setFilter] = useState<Filter>(() => loadUiState<Remembered>({ accepted: [] }).filter ?? 'all');
  /**
   * The conflicts accepted or taken back since the filter was chosen: they stay in view, folded, until it changes,
   * rather than leaving under the pointer the moment they are accepted.
   */
  const [lingering, setLingering] = useState(() => new Set<string>());
  const list = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const stop = onMessage((message) => {
      if (message.type === 'review') {
        setReview(message.review);
        setError(message.error);
      }
      if (message.type === 'busy') setBusy(message.title !== null);
    });
    post({ type: 'ready' });
    return stop;
  }, []);

  // What was accepted and is no longer there, written or edited, is forgotten.
  const present = useMemo(() => new Set(review?.files.flatMap((file) => file.conflicts.filter((each) => each.proposal).map((each) => idOf(file.path, each.key)))), [review]);
  const chosen = useMemo(() => [...accepted].filter((id) => present.has(id)), [accepted, present]);
  useEffect(() => saveUiState({ accepted: chosen, columns, widths: kept, file: selected, filter }), [chosen, columns, kept, selected, filter]);
  // Another file, or other conflicts, are read from their top.
  useEffect(() => {
    if (list.current) list.current.scrollTop = 0;
  }, [selected, filter]);

  if (review === undefined) return <div className="empty">Reading the files in conflict…</div>;
  if (review === null) return <div className="empty">Gitardo could not read the files in conflict. {error}</div>;

  const conflicts = review.files.flatMap((file) => file.conflicts.map((conflict) => ({ file, conflict })));
  const byRules = conflicts.filter(({ conflict }) => conflict.proposal?.source === 'rules').length;
  const byAI = conflicts.filter(({ conflict }) => conflict.proposal?.source === 'ai').length;
  const unanswered = conflicts.filter(({ conflict }) => !conflict.proposal);
  const askable = unanswered.filter(({ conflict }) => conflict.asking?.state !== 'waiting' && conflict.asking?.state !== 'thinking').length;
  const refs = (ids: string[]): ConflictRef[] => ids.map((id) => ({ path: id.slice(0, id.indexOf('\0')), key: id.slice(id.indexOf('\0') + 1) }));
  const accept = (ids: string[], on: boolean) => {
    const next = new Set(accepted);
    const unfolded = new Map(folds);
    for (const id of ids) {
      if (on) next.add(id);
      else next.delete(id);
      // What is accepted or taken back folds or unfolds again as it would by itself, and so does its file.
      unfolded.delete(id);
      unfolded.delete(id.slice(0, id.indexOf('\0')));
    }
    setAccepted(next);
    setFolds(unfolded);
    if (filter !== 'all') setLingering(new Set([...lingering, ...ids]));
  };
  const fold = (id: string, folded: boolean) => setFolds(new Map(folds).set(id, folded));
  const choose = (next: Filter) => {
    setFilter(next);
    setLingering(new Set());
  };
  const isAccepted = (file: ReviewFile, conflict: ReviewConflict) => accepted.has(idOf(file.path, conflict.key)) && !!conflict.proposal;
  const shown = (file: ReviewFile, conflict: ReviewConflict) =>
    shows(filter, conflict, isAccepted(file, conflict)) || lingering.has(idOf(file.path, conflict.key));
  const listed = (file: ReviewFile) => file.conflicts.some((conflict) => shown(file, conflict));
  const counts = FILTERS.map((each) => conflicts.filter(({ file, conflict }) => shows(each.filter, conflict, isAccepted(file, conflict))).length);
  // A file no longer in conflict, or no longer shown, gives way to all of them.
  const current = review.files.find((file) => file.path === selected && listed(file));
  const files = (current ? [current] : review.files).filter(listed);
  const allProposed = [...present];
  const sides: [Side, Side] = [
    { role: 'Current', label: review.oursLabel, name: review.oursName, subject: review.oursSubject },
    { role: 'Incoming', label: review.theirsLabel, name: review.theirsName, subject: review.theirsSubject },
  ];
  const summary = [
    byRules && `${byRules} proposed by the rules`,
    byAI && `${byAI} by the AI`,
    unanswered.length && `${unanswered.length} with nothing proposed`,
  ].filter(Boolean);

  return (
    <main className={`conflicts-page${columns ? ' wide' : ''}`}>
      <header className="conflicts-head">
        <h1>
          Conflicts
          {review.operation && <span className="muted"> of the {review.operation}</span>}
        </h1>
        <p className="muted">
          {conflicts.length
            ? `${plural(conflicts.length, 'conflict')} in ${plural(review.files.length, 'file')}${summary.length ? `: ${summary.join(', ')}` : ''}. Nothing is written until you write what you accepted.`
            : 'No conflict with markers to look at here.'}
        </p>
        {!review.rules && !review.ai && <p className="note">Neither the rules nor the AI are switched on in the settings: nothing is proposed.</p>}
        <div className="actions">
          <button
            className="button"
            disabled={!allProposed.length || chosen.length === allProposed.length}
            title="Accept every resolution proposed, from the rules and from the AI"
            onClick={() => accept(allProposed, true)}
          >
            <Icon name="check-all" />
            Accept all proposed
          </button>
          {chosen.length > 0 && (
            <button className="button" title="Take back every acceptance" onClick={() => accept(chosen, false)}>
              <Icon name="clear-all" />
              Clear
            </button>
          )}
          {review.ai && (
            <button
              className="button ask-all"
              disabled={!askable || review.asking}
              title="Ask the language model about each conflict that has nothing proposed"
              onClick={() => post({ type: 'askAI' })}
            >
              <Icon name="sparkle" />
              Ask the AI about the rest{askable ? ` (${askable})` : ''}
            </button>
          )}
          {review.ai && (
            <button
              className="button model"
              title="Choose the language model that resolves the conflicts, and how much it thinks"
              onClick={() => post({ type: 'chooseConflictModel' })}
            >
              <Icon name="hubot" />
              <span className="name">{review.model ?? 'No language model'}</span>
            </button>
          )}
          <span className="grow" />
          <div className="segmented" role="group" aria-label="How each conflict is shown">
            <button
              className={`button${columns ? '' : ' on'}`}
              aria-pressed={!columns}
              title="Show the two sides of each conflict over what is proposed, with the lines around it"
              onClick={() => setColumns(false)}
            >
              <Icon name="list-flat" />
              Cards
            </button>
            <button
              className={`button${columns ? ' on' : ''}`}
              aria-pressed={columns}
              title="Show each conflict in three columns, as the Flow Merge does: the current side, what is proposed and the incoming side, joined by bands"
              onClick={() => setColumns(true)}
            >
              <Icon name="editor-layout" />
              Columns
            </button>
          </div>
          <button
            className="button"
            disabled={!chosen.length}
            title="Show each file beside what the accepted resolutions make of it, in the diff editor"
            onClick={() => post({ type: 'previewResolutions', accepted: refs(chosen) })}
          >
            <Icon name="diff-multiple" />
            Show the result
          </button>
          <button
            className="button primary"
            disabled={!chosen.length || busy}
            title="Write the accepted resolutions in the files, and stage the files left with no conflict"
            onClick={() => post({ type: 'writeResolutions', accepted: refs(chosen) })}
          >
            <Icon name="save-all" />
            {chosen.length ? `Write ${plural(chosen.length, 'resolution')}` : 'Write the accepted'}
          </button>
        </div>
        <Progress review={review} />
      </header>

      <Report review={review} />

      <div className="conflicts-body">
        {(review.files.length > 0 || review.others.length > 0) && (
          <FileList
            review={review}
            filter={filter}
            selected={current?.path ?? null}
            counts={counts}
            accepted={isAccepted}
            listed={listed}
            onFilter={choose}
            onSelect={setSelected}
          />
        )}
        <div className="conflicts-list" ref={list}>
          {files.map((file) => {
            const ids = file.conflicts.filter((conflict) => conflict.proposal).map((conflict) => idOf(file.path, conflict.key));
            const all = ids.length > 0 && ids.every((id) => accepted.has(id));
            const settled = all && ids.length === file.conflicts.length;
            // A file chosen alone is not folded unless asked: it is there to be looked at.
            const folded = folds.get(file.path) ?? (settled && !current);
            return (
              <section key={file.path} className={`conflict-file${folded ? ' folded' : ''}`} aria-label={file.path}>
                <h2>
                  <button
                    className="button icon"
                    title={folded ? 'Show the conflicts of this file' : 'Hide the conflicts of this file'}
                    aria-expanded={!folded}
                    onClick={() => fold(file.path, !folded)}
                  >
                    <Icon name={folded ? 'chevron-right' : 'chevron-down'} />
                  </button>
                  <FileIcon path={file.path} />
                  <span className="grow path" title={file.path}>
                    {file.path}
                  </span>
                  <span className="muted">
                    {settled
                      ? `${file.conflicts.length === 1 ? 'Its conflict is' : `All ${file.conflicts.length} conflicts are`} accepted`
                      : `${ids.length} of ${plural(file.conflicts.length, 'conflict')} proposed`}
                  </span>
                  <button className="button" disabled={!ids.length || all} title="Accept every resolution proposed in this file" onClick={() => accept(ids, true)}>
                    <Icon name="check-all" />
                    Accept the file
                  </button>
                </h2>
                {!folded &&
                  file.conflicts.filter((conflict) => shown(file, conflict)).map((conflict) => {
                    const id = idOf(file.path, conflict.key);
                    const on = accepted.has(id);
                    return (
                      <Conflict
                        key={conflict.key}
                        file={file}
                        conflict={conflict}
                        sides={sides}
                        ai={review.ai}
                        busy={busy}
                        columns={columns}
                        accepted={on}
                        folded={folds.get(id) ?? (on && !!conflict.proposal)}
                        onAccept={(yes) => accept([id], yes)}
                        onFold={(yes) => fold(id, yes)}
                        onWidths={setKept}
                      />
                    );
                  })}
              </section>
            );
          })}

          {review.files.length > 0 && !files.length && (
            <div className="empty-filter">
              <p className="muted">No conflict is left to show here.</p>
              <button className="button" onClick={() => choose('all')}>
                Show every conflict
              </button>
            </div>
          )}

          {review.others.length > 0 && !current && filter === 'all' && (
            <section className="conflict-file others" aria-label="Other files in conflict">
              <h2>
                <Icon name="file-binary" />
                <span className="grow">Also in conflict, with no markers to look at here</span>
              </h2>
              <p className="muted">Binary files, files a side deleted, and very large files are resolved in an editor, or by taking one side.</p>
              <ul>
                {review.others.map((file) => (
                  <li key={file}>
                    <FileIcon path={file} />
                    <button className="link" onClick={() => post({ type: 'openConflict', path: file, line: 1, editor: 'mergeEditor' })}>
                      {file}
                    </button>
                  </li>
                ))}
              </ul>
            </section>
          )}

          {!review.files.length && !review.others.length && !review.report && (
            <div className="empty-review">
              <Icon name="pass" />
              <p>No file is in conflict{review.operation ? `: the ${review.operation} can continue` : ''}.</p>
              {review.operation && (
                <button className="button primary" onClick={() => post({ type: 'action', action: { kind: 'continue' } })}>
                  <Icon name="debug-continue" />
                  Continue the {review.operation}
                </button>
              )}
            </div>
          )}
          <p className="muted footnote">
            {review.rules ? 'The rules resolve a conflict only where no choice is needed, and every line they write is a line of one of the two sides. ' : ''}
            {review.ai ? 'The AI proposes the rest from the lines around each conflict: look at what it writes before you accept it.' : ''}
          </p>
        </div>
      </div>
    </main>
  );
}

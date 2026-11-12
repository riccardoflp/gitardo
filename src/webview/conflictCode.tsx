import type { ReactNode } from 'react';
import type { ConflictChanges } from '../shared/resolution';
import { linesOf } from '../shared/proposalLines';
import { Piece } from './FileChanges';
import type { ChangedLine } from './sideChanges';
import type { Token } from './syntax';

// The lines of code of the review of the conflicts: as they are, or as what a side changed of the base.

export const plural = (count: number, one: string, many = `${one}s`) => `${count} ${count === 1 ? one : many}`;

/**
 * Lines of code, in the colors of the theme once they are known, each with the class `kind` gives it and the tooltip
 * `title` gives it.
 */
export function Code(props: {
  text: string;
  colors?: (Token[] | null)[];
  kind?: (line: string, index: number) => string;
  title?: (line: string, index: number) => string | undefined;
  className?: string;
  caret?: boolean;
  /** What is done when the pointer comes onto a line, or leaves the lines with null. */
  onHover?: (index: number | null) => void;
}) {
  const { text, colors, kind, title, className, caret, onHover } = props;
  const lines = linesOf(text);
  if (!lines.length && !caret) return <div className={`code-lines empty-lines ${className ?? ''}`}>Nothing: the lines are deleted</div>;
  // The lines are as wide as the longest of them, for their colors to go all the way when the lines scroll sideways.
  return (
    <div className={`code-lines ${className ?? ''}`} onMouseLeave={onHover && (() => onHover(null))}>
      <div className="code-width">
        {lines.map((line, index) => {
          const pieces = colors?.[index];
          return (
            <div key={index} className={`code-line ${kind?.(line, index) ?? ''}`} title={title?.(line, index)} onMouseEnter={onHover && (() => onHover(index))}>
              {pieces ? pieces.map((token, piece) => <Piece key={piece} token={token} />) : line || ' '}
            </div>
          );
        })}
        {caret && <span className="caret" aria-hidden="true" />}
      </div>
    </div>
  );
}

/** A line in its colors, with the words that changed in it marked. */
function Marked({ text, tokens, words }: { text: string; tokens?: Token[] | null; words?: [number, number][] }) {
  const pieces = tokens ?? [{ text }];
  if (!words?.length) return pieces.map((token, index) => <Piece key={index} token={token} />);
  const marked: ReactNode[] = [];
  let at = 0;
  for (const token of pieces) {
    // A piece of the colors can start or end inside a word that changed: it is cut where the word starts and ends.
    for (let start = 0; start < token.text.length; ) {
      const where = at + start;
      const inside = words.find(([from, to]) => where >= from && where < to);
      const next = inside ? inside[1] : Math.min(...words.map(([from]) => from).filter((from) => from > where), at + token.text.length);
      const end = Math.min(token.text.length, next - at);
      const piece = <Piece key={marked.length} token={{ ...token, text: token.text.slice(start, end) }} />;
      marked.push(
        inside ? (
          <mark key={marked.length} className="word">
            {piece}
          </mark>
        ) : (
          piece
        ),
      );
      start = end;
    }
    at += token.text.length;
  }
  return marked;
}

/**
 * A line of a side shown as what it changed: what it is, what is in its gutter, and its words that changed. A row
 * skipped stands for lines of the file that neither changed nor are near a change.
 */
interface ChangeRow {
  kind: ChangedLine['kind'] | 'around' | 'skipped';
  gutter: string;
  text: string;
  tokens?: Token[] | null;
  words?: [number, number][];
}

/** Lines of the file around what a side changed, numbered as they are in the file. */
function aroundLines(text: string, first: number, colors?: (Token[] | null)[]): ChangeRow[] {
  return linesOf(text).map((line, index) => ({ kind: 'around', gutter: String(first + index), text: line, tokens: colors?.[index] }));
}

/**
 * The parts of a conflict colored on their own, by their place in the list `colors` is given for: the lines before,
 * the two sides, the proposal, the lines after, the base, the lines the proposal replaces on each side, and what each
 * side changed of the base with the lines around it.
 */
export const PART = {
  before: 0,
  ours: 1,
  theirs: 2,
  proposal: 3,
  after: 4,
  base: 5,
  regionOurs: 6,
  regionTheirs: 7,
  changesBase: 8,
  changesOurs: 9,
  changesTheirs: 10,
  changesBefore: 11,
  changesAfter: 12,
} as const;

/**
 * A side as what it changed of the base: the lines it took out, with a minus, over the ones it wrote in their place,
 * with a plus, and the words that changed marked in both, between the lines of the file around them.
 */
export function Changes(props: { changes: ConflictChanges; lines: ChangedLine[]; side: 'ours' | 'theirs'; colors?: (Token[] | null)[][] }) {
  const { changes, lines, side, colors } = props;
  const rows: ChangeRow[] = [
    ...aroundLines(changes.before, changes.beforeLine, colors?.[PART.changesBefore]),
    ...lines.map((line) => ({
      kind: line.kind,
      gutter: line.kind === 'removed' ? '−' : line.kind === 'added' ? '+' : '',
      text: line.text,
      tokens: (line.kind === 'removed' ? colors?.[PART.changesBase] : colors?.[side === 'ours' ? PART.changesOurs : PART.changesTheirs])?.[line.index],
      words: line.words,
    })),
    ...aroundLines(changes.after, changes.afterLine, colors?.[PART.changesAfter]),
  ];
  return <ChangeLines rows={rows} titles={{ removed: 'Taken out of the base by this side', added: 'Written by this side', around: 'Line of the file around the conflict' }} />;
}

/** Rows of changes, each with its gutter, and what the gutter of each kind of row says when the pointer is on it. */
export function ChangeLines({ rows, titles }: { rows: ChangeRow[]; titles: Partial<Record<ChangeRow['kind'], string>> }) {
  return (
    <div className="code-lines changes">
      <div className="code-width">
        {rows.map((row, index) => (
          <div key={index} className={`code-line ${row.kind}`}>
            <span className="gutter" title={titles[row.kind]}>
              {row.gutter}
            </span>
            <span className="text">
              <Marked text={row.text || ' '} tokens={row.tokens} words={row.words} />
            </span>
          </div>
        ))}
      </div>
    </div>
  );
}

/** How many lines that did not change are shown before and after each change of a file, at most. */
const AROUND_CHANGES = 3;

/**
 * What a side changed of a whole file, as a unified diff shows it: the changes with the lines around them, numbered as
 * the file has them now, and the lines between them left out.
 */
export function fileRows(lines: ChangedLine[], colors?: (Token[] | null)[][]): ChangeRow[] {
  const near = lines.map(() => false);
  lines.forEach((line, at) => {
    if (line.kind === 'same') return;
    for (let each = Math.max(0, at - AROUND_CHANGES); each <= Math.min(lines.length - 1, at + AROUND_CHANGES); each++) near[each] = true;
  });
  const rows: ChangeRow[] = [];
  for (let at = 0; at < lines.length; ) {
    const line = lines[at];
    if (near[at]) {
      const gutter = line.kind === 'removed' ? '−' : line.kind === 'added' ? '+' : String(line.index + 1);
      rows.push({ kind: line.kind === 'same' ? 'around' : line.kind, gutter, text: line.text, tokens: colors?.[line.kind === 'removed' ? 0 : 1]?.[line.index], words: line.words });
      at++;
      continue;
    }
    let end = at;
    while (end < lines.length && !near[end]) end++;
    rows.push({ kind: 'skipped', gutter: '⋯', text: `${plural(end - at, 'line')} that did not change` });
    at = end;
  }
  return rows;
}

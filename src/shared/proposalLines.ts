import type { ProposalWarning, SideChoice } from './resolution';

// Where each line of a proposal comes from, what the proposal does with the two sides as a whole, and what in it looks
// like a mistake. The review shows all three on each conflict, so that a proposal can be understood without reading
// the two sides line by line.

/**
 * Where a line of a proposal comes from: one side, both (the two sides have it), the base (what was there before either
 * side changed it), none of them, or an empty line.
 */
export type LineOrigin = 'ours' | 'theirs' | 'both' | 'base' | 'new' | 'blank';

/** What a proposal does with the two sides: one of them, both one after the other, a mix, or none of their lines. */
export type ProposalShape = SideChoice | 'mixed' | 'nothing';

export interface ProposalLines {
  /** The origin of each line of the proposal. */
  origins: LineOrigin[];
  /** Pairs of a line of the current side and the same line of the proposal, by their indexes. */
  fromOurs: [number, number][];
  /** Pairs of a line of the proposal and the same line of the incoming side, by their indexes. */
  toTheirs: [number, number][];
}

export interface ProposalSummary {
  shape: ProposalShape;
  /** How many lines that are not empty come from the current side only, the incoming side only, both, the base, or none. */
  ours: number;
  theirs: number;
  both: number;
  base: number;
  fresh: number;
}

/** The lines of a text, without what ends them. */
export const linesOf = (text: string) => (text ? text.replace(/\r?\n$/, '').split(/\r?\n/) : []);

/** Two lines are the same when they differ only in the spaces at their ends. */
const same = (one: string, other: string) => one.trim() === other.trim();

/** Past this, a conflict is too large to be worth pairing line by line: its lines are said to come from neither side. */
const MOST_PAIRS = 250_000;

/** Which lines of `a` are lines of `b`, in the same order, as pairs of their indexes: as many as can be. */
export function sameLines(a: string[], b: string[]): [number, number][] {
  if (!a.length || !b.length || a.length * b.length > MOST_PAIRS) return [];
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

/**
 * Where each line of what is proposed comes from, given what the two sides have in the lines it takes the place of,
 * and what the base had there when it is known. A line is first paired with the same line of a side in the same order;
 * one that cannot be, as when a proposal changes the order of the lines, still comes from the side that has it.
 */
export function proposalLines(ours: string, theirs: string, proposed: string, base: string | null = null): ProposalLines {
  const lines = linesOf(proposed);
  const fromOurs = sameLines(linesOf(ours), lines);
  const toTheirs = sameLines(lines, linesOf(theirs));
  const [byOurs, byTheirs] = [new Set(fromOurs.map(([, j]) => j)), new Set(toTheirs.map(([i]) => i))];
  const [inOurs, inTheirs, inBase] = [ours, theirs, base ?? ''].map((text) => new Set(linesOf(text).map((line) => line.trim())));
  const origins = lines.map((line, index): LineOrigin => {
    const trimmed = line.trim();
    if (!trimmed) return 'blank';
    if (byOurs.has(index) && byTheirs.has(index)) return 'both';
    if (byOurs.has(index)) return 'ours';
    if (byTheirs.has(index)) return 'theirs';
    if (inOurs.has(trimmed) && inTheirs.has(trimmed)) return 'both';
    if (inOurs.has(trimmed)) return 'ours';
    if (inTheirs.has(trimmed)) return 'theirs';
    return inBase.has(trimmed) ? 'base' : 'new';
  });
  return { origins, fromOurs, toTheirs };
}

/** What a proposal does with the two sides, and how many of its lines come from where. */
export function summarize(ours: string, theirs: string, proposed: string, origins: LineOrigin[]): ProposalSummary {
  const count = (origin: LineOrigin) => origins.filter((each) => each === origin).length;
  const counts = { ours: count('ours'), theirs: count('theirs'), both: count('both'), base: count('base'), fresh: count('new') };
  // The empty lines and the spaces at the ends are left aside: a proposal that has them in other places still takes a side.
  const kept = (text: string) => linesOf(text).map((line) => line.trim()).filter(Boolean);
  const [mine, current, incoming] = [kept(proposed), kept(ours), kept(theirs)];
  const equal = (one: string[], other: string[]) => one.length === other.length && one.every((line, index) => line === other[index]);
  const shape: ProposalShape = !mine.length
    ? 'nothing'
    : equal(mine, current)
      ? 'current'
      : equal(mine, incoming)
        ? 'incoming'
        : equal(mine, [...current, ...incoming])
          ? 'current-first'
          : equal(mine, [...incoming, ...current])
            ? 'incoming-first'
            : 'mixed';
  return { shape, ...counts };
}

/**
 * The name a line gives a value, when it gives one: a key of JSON or YAML, a property, a variable, a setting. The
 * words that can come before a variable are left out of the name.
 */
const KEYED = /^(\s*)(?:(?:export|const|let|var|val|final|readonly|public|private|protected|static)\s+)*(["']?)([\w$.-]+)\2\s*\??\s*(?::(?!:)|=(?![=>]))/;

/**
 * The names the proposal gives a value twice, once with a line of each side, at the same depth and with values that
 * differ: what a proposal that keeps both sides does to a setting both of them changed.
 */
export function repeatedKeys(proposed: string, origins: LineOrigin[]): ProposalWarning[] {
  const found = new Map<string, { key: string; lines: number[]; sides: Set<LineOrigin>; texts: Set<string> }>();
  linesOf(proposed).forEach((line, index) => {
    const origin = origins[index];
    if (origin !== 'ours' && origin !== 'theirs') return;
    const match = KEYED.exec(line);
    if (!match) return;
    const [, indent, , key] = match;
    const named = found.get(`${indent}\0${key}`) ?? { key, lines: [], sides: new Set(), texts: new Set() };
    named.lines.push(index);
    named.sides.add(origin);
    named.texts.add(line.trim());
    found.set(`${indent}\0${key}`, named);
  });
  return [...found.values()]
    .filter((named) => named.sides.size === 2 && named.texts.size > 1)
    .map((named): ProposalWarning => ({ kind: 'repeated', key: named.key, lines: named.lines }));
}

/** What a choice of the two sides puts in place of a conflict, from what each side has between the markers. */
export function choiceText(choice: SideChoice, ours: string, theirs: string): string {
  const ended = (text: string) => (text && !/\n$/.test(text) ? `${text}\n` : text);
  if (choice === 'current') return ours;
  if (choice === 'incoming') return theirs;
  return choice === 'current-first' ? ended(ours) + theirs : ended(theirs) + ours;
}

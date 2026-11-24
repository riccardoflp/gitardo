import { pullBadgeText } from '../shared/refs';
import type { Label } from './CommitLabels';

/**
 * How wide the labels of a commit, its message, its author and its date are drawn, measured in the page with the fonts
 * of the page, so that the row chooses which labels it shows whole (`graph.label-overflow`). A label is laid out by
 * `styles.css`: these are its sizes there.
 */

/** The padding and the border of a label, on both sides. */
const LABEL_EDGES = 2 * 6 + 2 * 1;
/** An icon of a label. */
const ICON = 12;
/** Between the icons of one label. */
const ICONS_GAP = 1;
/** Between the icons, the name and the pull request of a label. */
const LABEL_GAP = 3;
/** The words of a label, and of the line under a message, are smaller than those of the message. */
const SMALLER = 0.9;
/** Past this many texts measured, they are measured again rather than kept: a long history has many messages. */
const KEPT = 5000;

/** The fonts the page draws a row with, as the canvas names them. */
export interface RowFonts {
  /** The message. */
  text: string;
  /** A label, the author and the date under a message. */
  small: string;
  /** The label of what is checked out, which is bolder. */
  strong: string;
}

/** The fonts of the rows, from the font of an element of the list. */
export function rowFonts(element: Element): RowFonts {
  const style = getComputedStyle(element);
  const size = parseFloat(style.fontSize) || 13;
  const family = style.fontFamily || 'sans-serif';
  const small = size * SMALLER;
  return { text: `${size}px ${family}`, small: `${small}px ${family}`, strong: `600 ${small}px ${family}` };
}

let context: CanvasRenderingContext2D | null | undefined;
const measured = new Map<string, number>();

/** How wide a text is drawn in a font, or 0 where the page cannot measure. */
export function textWidth(text: string, font: string): number {
  const key = `${font}\n${text}`;
  const known = measured.get(key);
  if (known !== undefined) return known;
  context ??= document.createElement('canvas').getContext('2d');
  if (!context) return 0;
  context.font = font;
  const width = context.measureText(text).width;
  if (measured.size >= KEPT) measured.clear();
  measured.set(key, width);
  return width;
}

/** How wide a label is drawn: its icons, its name and the number of its pull request. */
export function labelWidth(label: Label, fonts: RowFonts): number {
  const icons = 1 + Number(!!label.worktree) + Number(!!label.remote) + Number(!!label.mark);
  const font = label.current ? fonts.strong : fonts.small;
  let width = LABEL_EDGES + icons * ICON + (icons - 1) * ICONS_GAP + LABEL_GAP + textWidth(label.name, font);
  if (label.pull) width += LABEL_GAP + ICON + textWidth(pullBadgeText(label.pull.request), font);
  return Math.ceil(width);
}

/** How wide `+N` is drawn. */
export function countWidth(count: number, fonts: RowFonts): number {
  return Math.ceil(LABEL_EDGES + textWidth(`+${count}`, fonts.small));
}

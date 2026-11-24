/**
 * Which labels of a commit its row shows whole, as Gitardo Desktop fits them (`graph.label-overflow`,
 * `fitted_pills` of `rust/crates/views/src/graph/labels.rs`): as many as fit whole in the room the message, or the
 * author and the date, leave them, in their order, then one label `+N` for the others; when not even the first fits
 * whole beside `+N`, the first shortened, when it keeps three times its height; else `+N` alone. The widths are
 * measured in the page; this only chooses.
 */

/**
 * The share of the line of a commit that its message keeps, when it is that long, before the labels take the rest,
 * on the one line of a compact row: the labels have at least the other two fifths of it (Q640).
 */
export const MESSAGE_SHARE = 0.6;

/** The share of the line under a message that the author and the date keep, when they are that long, before the labels take the rest (D86). */
export const AUTHOR_AND_DATE_SHARE = 1 / 3;

/** How many times its own height a label shortened beside `+N` is at least wide, so that its icon and a few letters are seen. */
export const SHORTENED_LEAST = 3;

/** The room labels are fitted in. */
export interface LabelRoom {
  /** How wide it is. */
  width: number;
  /** How wide a label shortened beside `+N` is at least. */
  least: number;
  /** The gap after each label. */
  gap: number;
}

/** The labels of a commit fitted in the room of its row. */
export interface FittedLabels {
  /** How many of the labels are shown, from the first, before `+N`. */
  named: number;
  /** How many `+N` counts: the labels not shown. None when every label is shown. */
  counted: number;
  /** The width each label shown is given, in their order, then that of `+N` when there is one: the first may be less than it needs, shortened. */
  widths: number[];
  /** The width they take, with the gap after each. */
  width: number;
  /** Whether some labels are not shown whole: counted in `+N`, or the first one shortened. */
  hidesSome: boolean;
  /** Whether the first label is shortened beside `+N`, to the width it is given. */
  shortened: boolean;
}

/**
 * The labels of a commit in a room, each as wide as `widths` says, and `+N` as wide as `countWidth` says of N: as many
 * whole as fit, in their order, and the count of the others; when not even the first fits whole beside the count, the
 * first shortened to the room left beside it, when that is at least `least`; else the count alone; none when not even
 * that fits.
 */
export function fitLabels(widths: readonly number[], countWidth: (count: number) => number, room: LabelRoom): FittedLabels {
  const fit = (named: number): FittedLabels => {
    const counted = widths.length - named;
    const shown = [...widths.slice(0, named), ...(counted ? [countWidth(counted)] : [])];
    return { named, counted, widths: shown, width: shown.reduce((sum, width) => sum + width + room.gap, 0), hidesSome: counted > 0, shortened: false };
  };
  for (let named = widths.length; named >= 1; named--) {
    const fitted = fit(named);
    if (fitted.width <= room.width) return fitted;
  }
  if (!widths.length) return fit(0);
  const first = fit(1);
  const beside = first.widths.slice(1).reduce((sum, width) => sum + width + room.gap, 0);
  const left = room.width - beside - room.gap;
  if (left >= room.least) {
    first.widths[0] = Math.min(left, first.widths[0]);
    return { ...first, width: left + room.gap + beside, hidesSome: true, shortened: true };
  }
  const count = fit(0);
  if (count.width <= room.width) return count;
  return { named: 0, counted: widths.length, widths: [], width: 0, hidesSome: true, shortened: false };
}

/** The room of the labels on the one line of a compact row: what the message leaves, which keeps up to three fifths of the line. */
export function roomBesideMessage(line: number, message: number): number {
  return Math.max(0, line - Math.min(message, line * MESSAGE_SHARE));
}

/** The room of the labels on the line under the message: what the author and the date leave, which keep up to a third of the line. */
export function roomBesideAuthor(line: number, authorAndDate: number): number {
  return Math.max(0, line - Math.min(authorAndDate, line * AUTHOR_AND_DATE_SHARE));
}

/**
 * The commits a view is sent behind its first page (QA6, D92): a view pointed at a repository whose whole history is
 * read, or opened on one, is sent the first page at once, which it draws, and the rest in pieces of bounded size, each
 * once the one before it reached the view. Nothing here knows what a piece is: `next` sends a view the state of its
 * group again, of which it gets the commits after those it has (`stateFor`).
 */

/** How many commits a piece sent behind the first page carries at most: about three megabytes of rows. */
export const ROWS_PIECE = 5000;
/** How long after a piece reached a view the next one is made, so that the view draws and answers in between. */
const PIECE_DELAY = 16;

/** Which views wait for more of the commits of their state, and when each is sent its next piece. */
export class RowsBehind<View extends object> {
  /** The views that have not got every commit of their state, each with the wait for its next piece once the last reached it. */
  private readonly waiting = new WeakMap<View, ReturnType<typeof setTimeout> | null>();

  constructor(
    /** Sends a view the commits after those it has, by sending it the state of its group again. */
    private readonly next: (view: View) => void,
    private readonly delay = PIECE_DELAY,
  ) {}

  /** A view was sent a state: `more` says whether it has not got every commit of it yet. */
  sent(view: View, more: boolean): void {
    if (more) {
      if (!this.waiting.has(view)) this.waiting.set(view, null);
      return;
    }
    clearTimeout(this.waiting.get(view) ?? undefined);
    this.waiting.delete(view);
  }

  /** What a view was sent reached it, or did not, as for a view that is hidden: it then asks for everything once it is back. */
  reached(view: View, delivered: boolean): void {
    if (!delivered) return this.sent(view, false);
    if (this.waiting.get(view) !== null) return;
    this.waiting.set(
      view,
      setTimeout(() => {
        this.waiting.delete(view);
        this.next(view);
      }, this.delay),
    );
  }

  /** Whether a view still waits for commits of the state it was sent: it asks for no more of the history meanwhile. */
  has(view: View): boolean {
    return this.waiting.has(view);
  }
}

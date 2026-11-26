/**
 * One call for a burst of events, whoever says them: the watcher of VS Code and the engine's saying the same change of
 * a git folder ask for one read (`repo.schedule-reads`, `repo.watch-git`).
 */

/** Runs once, a delay after the first event of a burst, however long the burst lasts; sooner when a later event asks for less. */
export class Burst {
  private timer?: ReturnType<typeof setTimeout>;
  /** When `timer` fires. */
  private due = 0;

  constructor(private readonly run: () => void) {}

  /** Whether an event waits for its call. */
  get waits(): boolean {
    return this.timer !== undefined;
  }

  /** An event came: the call is `delay` from now, unless one waits that comes no later. */
  heard(delay: number): void {
    const due = Date.now() + delay;
    if (this.timer && this.due <= due) return;
    clearTimeout(this.timer);
    this.due = due;
    this.timer = setTimeout(() => {
      this.timer = undefined;
      this.run();
    }, delay);
  }

  /** Drops the call that waits, as when what it asked for is done already. */
  drop(): void {
    clearTimeout(this.timer);
    this.timer = undefined;
  }
}

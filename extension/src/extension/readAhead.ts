/**
 * The turns of the repositories read ahead (`repo.watch-hidden`, D92): once the repository shown has its first read,
 * the others of the list are read one after the other, so that each is there when it is first chosen. None is read
 * while an operation runs or a repository shown is read: one being read when they start is stopped, and waits for its
 * turn again. Nothing here reads anything: it says whose turn it is, and when.
 */

/** A folder as the repositories are told apart by, whatever its slashes and its case. */
const key = (root: string) => root.replace(/\\/g, '/').toLowerCase();

/** What the turns ask of the window. */
export interface AheadHost {
  /** The repositories that wait to be read ahead, in the order of the list: those no group shows and that keep nothing. */
  waiting(): string[];
  /** Whether one may be read now: the setting is on, the repository shown has its first read, and nothing else runs or is read. */
  isFree(): boolean;
  /**
   * Reads a repository and keeps what was read, as if it had been shown and left. False when nothing was kept: it was
   * stopped, which `stop` says, or it has nothing to read ahead, such as a worktree of a repository shown.
   */
  read(root: string, stop: AbortSignal): Promise<boolean>;
  /** A read failed: it is said in the log, and the repository is read when it is chosen. */
  failed(root: string, error: unknown): void;
}

/** Gives each repository that waits its turn to be read ahead, one at a time. */
export class ReadAhead {
  /** The read under way, and what stops it. */
  private under?: { root: string; stop: AbortController };
  /** The repositories whose turn gave nothing, which get no other until the list is read again. */
  private readonly passed = new Set<string>();

  constructor(private readonly host: AheadHost) {}

  /** The repository being read ahead, if one is. */
  get reading(): string | undefined {
    return this.under?.root;
  }

  /**
   * Gives the next repository its turn, when none is being read and nothing stands in the way. Called whenever that
   * may have changed: a read or an operation ended, the list was read, the setting was turned on.
   */
  next(): void {
    if (this.under || !this.host.isFree()) return;
    const root = this.host.waiting().find((each) => !this.passed.has(key(each)));
    if (root === undefined) return;
    const under = { root, stop: new AbortController() };
    this.under = under;
    const stopped = () => under.stop.signal.aborted;
    void this.host
      .read(root, under.stop.signal)
      .then(
        (kept) => {
          if (!kept && !stopped()) this.passed.add(key(root));
        },
        (error: unknown) => {
          if (stopped()) return;
          this.passed.add(key(root));
          this.host.failed(root, error);
        },
      )
      .then(() => {
        if (this.under === under) this.under = undefined;
        this.next();
      });
  }

  /**
   * Stops the read under way, as when an operation or a read of a repository shown starts: what it read is thrown
   * away, and its repository waits for its turn again.
   */
  stop(): void {
    const under = this.under;
    this.under = undefined;
    under?.stop.abort();
  }

  /** The list of the repositories was read again: those whose turn gave nothing get another. */
  again(): void {
    this.passed.clear();
  }
}

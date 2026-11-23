/**
 * The blame of the lines of documents, kept for the version of each document it was asked for: the line under the
 * cursor is asked of git once for each version of its text and each line, not each time the cursor moves along it.
 * The text of a document not saved goes to git with the first line asked of each version, and not again for it.
 */
export class LineBlames<T> {
  /** For each document, by its URI, the version last asked and what was asked of its lines, the first line at 0. */
  private readonly documents = new Map<string, { version: number; lines: Map<number, Promise<T>> }>();

  /**
   * The blame of a line of a version of a document: the one kept, or the one `read` asks for, which is kept. Asking for
   * another version forgets the ones before. A read that fails is not kept, so that the next time asks again.
   */
  get(document: string, version: number, line: number, read: () => Promise<T>): Promise<T> {
    let kept = this.documents.get(document);
    if (kept?.version !== version) this.documents.set(document, (kept = { version, lines: new Map() }));
    const lines = kept.lines;
    const known = lines.get(line);
    if (known) return known;
    const asked = read();
    lines.set(line, asked);
    asked.catch(() => {
      if (lines.get(line) === asked) lines.delete(line);
    });
    return asked;
  }

  /** Whether the blame of that line of that version is kept or being asked. */
  has(document: string, version: number, line: number): boolean {
    const kept = this.documents.get(document);
    return kept?.version === version && kept.lines.has(line);
  }

  /** Forgets a document that was closed, or every document when the commits under them or how they are blamed changed. */
  forget(document?: string): void {
    if (document === undefined) this.documents.clear();
    else this.documents.delete(document);
  }
}

/**
 * The commit `HEAD` was at when each repository was last read, to tell a read after which the blame may say other
 * commits (a commit made, a checkout, a reset) from one after which it says the same.
 */
export class HeadsSeen {
  private readonly seen = new Map<string, string | null>();

  /**
   * Whether `HEAD` of a repository is at another commit than when it was last seen, and from now on where it is: true
   * the first time a repository is seen, since what was blamed before may be of any commit. Its folder is compared
   * whatever its slashes and case.
   */
  moved(root: string, commit: string | null): boolean {
    const key = root.replace(/\\/g, '/').toLowerCase();
    const moved = !this.seen.has(key) || this.seen.get(key) !== commit;
    this.seen.set(key, commit);
    return moved;
  }
}

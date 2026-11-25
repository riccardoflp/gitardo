import type * as vscode from 'vscode';
import type { Hosting } from '../shared/hosting';
import type { Filter, PullRequest, RepoState, Scope } from '../shared/protocol';

/**
 * What a repository left keeps while another is shown, with `gitardo.repositories.keepOpen` on (`repo.keep-open`): the
 * state the views were last sent, with its commits, and what the graph was narrowed to and how far it was read.
 */
export interface KeptRepository {
  state: RepoState;
  filter: Filter | null;
  scope: Scope | null;
  /** How many commits the graph was read to. */
  limit: number;
  /** The hosting service of its main remote (`hosting.detect-remote`). */
  hosting: Hosting | null;
  pulls: Record<string, PullRequest>;
  pullsAt: number;
  /** The folder its worktrees share, which the commits read of it are kept by (`repo.share-history`). */
  commonFolder?: string;
  /** The watchers of its files, kept with it: while it is not shown they only mark it out of date (`repo.watch-hidden`). */
  watcher?: vscode.Disposable;
  /** The last change seen in it since it was left, which it is read again for once it is shown; none when nothing changed. */
  changed?: string;
}

/** A folder as the repositories are told apart by, whatever its slashes and its case. */
const key = (root: string) => root.replace(/\\/g, '/').toLowerCase();

/**
 * The repositories left that keep what they showed, and the message the commit box had in each, in memory: they go with
 * the window, and all of them are forgotten once the setting is turned off.
 */
export class KeptRepositories {
  private readonly kept = new Map<string, KeptRepository & { root: string }>();
  private readonly messages = new Map<string, string>();
  /** The repositories being read ahead, each with the last change seen in it since its read started (`repo.watch-hidden`). */
  private readonly ahead = new Map<string, string | undefined>();

  /** Keeps what a repository showed as it is left, in place of what it kept before. */
  keep(root: string, kept: KeptRepository): void {
    const before = this.kept.get(key(root));
    if (before && before.watcher !== kept.watcher) before.watcher?.dispose();
    this.kept.set(key(root), { ...kept, root });
  }

  /** What a repository kept, which it no longer keeps once it is shown again. */
  take(root: string): KeptRepository | undefined {
    const kept = this.kept.get(key(root));
    this.kept.delete(key(root));
    return kept;
  }

  /** Whether a repository keeps what it showed. */
  has(root: string): boolean {
    return this.kept.has(key(root));
  }

  /** The last change seen in a repository kept since it was left; none when nothing changed, or when it is not kept. */
  outOfDate(root: string): string | undefined {
    return this.kept.get(key(root))?.changed;
  }

  /**
   * Marks a repository kept as out of date, with the change seen last, and reads nothing (`repo.watch-hidden`). In one
   * being read ahead the change is noted, for what it keeps once it is read.
   */
  changed(root: string, cause: string): void {
    const kept = this.kept.get(key(root));
    if (kept) kept.changed = cause;
    else if (this.ahead.has(key(root))) this.ahead.set(key(root), cause);
  }

  /** Marks every repository kept as out of date, as when the settings they were read with change. */
  changedAll(cause: string): void {
    for (const kept of this.kept.values()) kept.changed = cause;
    for (const folder of this.ahead.keys()) this.ahead.set(folder, cause);
  }

  /** A repository no view shows starts being read ahead: a change seen in it from now on is noted. */
  readingAhead(root: string): void {
    this.ahead.set(key(root), undefined);
  }

  /** The read ahead of a repository is over: the last change seen in it meanwhile, which what was read is out of date for. */
  readAhead(root: string): string | undefined {
    const changed = this.ahead.get(key(root));
    this.ahead.delete(key(root));
    return changed;
  }

  /** Keeps the message the commit box had in a repository it was pointed away from. */
  keepMessage(root: string, message: string): void {
    this.messages.set(key(root), message);
  }

  /** The message the commit box had in a repository when it was pointed away from it, if it was. */
  message(root: string): string | undefined {
    return this.messages.get(key(root));
  }

  /** Forgets what the repositories kept, all of them or those `which` says, and stops watching them. */
  forget(which: (root: string) => boolean = () => true): void {
    for (const [folder, kept] of this.kept) {
      if (!which(kept.root)) continue;
      kept.watcher?.dispose();
      this.kept.delete(folder);
    }
    for (const folder of this.messages.keys()) if (which(folder)) this.messages.delete(folder);
  }

  dispose(): void {
    this.forget();
  }
}

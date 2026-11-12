import * as vscode from 'vscode';
import type { Answer } from '../git/review';
import type { Choice } from '../shared/resolution';

/**
 * Something known of each conflict, for as long as VS Code runs: kept by the file and the fingerprint of its conflict,
 * and so held as long as nobody edits that conflict.
 */
export class ByConflict<T> {
  private readonly values = new Map<string, T>();
  private readonly changed = new vscode.EventEmitter<string>();

  /** Fires with the root of the repository whose values changed. */
  readonly onDidChange = this.changed.event;

  get(root: string, path: string, fingerprint: string): T | undefined {
    return this.values.get(key(root, path, fingerprint));
  }

  set(root: string, path: string, fingerprint: string, value: T): void {
    this.values.set(key(root, path, fingerprint), value);
    this.changed.fire(root);
  }

  delete(root: string, path: string, fingerprint: string): void {
    if (this.values.delete(key(root, path, fingerprint))) this.changed.fire(root);
  }
}

/** The same for a repository however its folder is written: with either slash, in either case. */
const key = (root: string, path: string, fingerprint: string) => `${root.replace(/\\/g, '/').toLowerCase()}\0${path}\0${fingerprint}`;

/** What the language model answered for the conflicts: the tab of the review and the Flow Merge show the same. */
export const answers = new ByConflict<Answer>();

/**
 * What the user chose in the tab of the review in place of what was proposed: a side, both, or lines written by hand.
 * It stays when the tab is closed, and comes back with the conflict when **Undo** marks it again.
 */
export const choices = new ByConflict<Choice>();

/**
 * The files with edits not saved in an editor: which of them are of the repository the panel shows and are changes to
 * it, for the Changes views to mark them, and saving them from there.
 */
import * as path from 'node:path';
import * as vscode from 'vscode';
import { type Repository, reads } from '../engine/reads';
import { samePath } from '../shared/paths';
import type { RepoState } from '../shared/protocol';
import type { ControllerHost } from './controllerHost';

/** Follows the files with edits not saved, and tells the Changes views which they are. */
export class UnsavedFiles {
  /** The files of the repository shown that have edits not saved in an editor, by their paths from its root. */
  paths: string[] = [];
  /**
   * Whether git ignores each file with edits not saved, by its repository and its path there, asked of the engine once
   * for each file while it stays unsaved, not at each read.
   */
  readonly ignored = new Map<string, boolean>();
  /** What `paths` was last worked out from: the state, and the files of the editors with edits not saved. */
  private from?: { state: RepoState | null; files: string };

  constructor(private readonly host: Pick<ControllerHost, 'followers' | 'post' | 'views'>) {}

  /** Tells the Changes views which files of the repository have edits not saved in an editor, when that changed. */
  async check(): Promise<void> {
    const { state, repo } = this.host.followers;
    const documents = vscode.workspace.textDocuments.filter((document) => document.isDirty && document.uri.scheme === 'file');
    const files = documents.map((document) => document.uri.fsPath);
    const from = { state, files: JSON.stringify(files) };
    // It runs at every key typed in an editor: most of the time nothing it looks at has changed.
    if (this.from?.state === from.state && this.from.files === from.files) return;
    this.from = from;
    const paths = state && repo ? await this.unsavedIn(repo, state, files) : [];
    if (this.from !== from || JSON.stringify(paths) === JSON.stringify(this.paths)) return;
    this.paths = paths;
    for (const [view, kind] of this.host.views) if (kind === 'changes') this.host.post({ type: 'unsaved', paths }, view);
  }

  /**
   * Those of these files that are of the repository, by their paths from its root, in order. The files of its
   * submodules and of the worktrees inside it are of other repositories, and the ones git ignores are no change to it.
   */
  private async unsavedIn(repo: Repository, state: RepoState, files: string[]): Promise<string[]> {
    const inside = (folder: string) => {
      const relative = path.relative(repo.root, folder);
      return relative && !relative.startsWith('..') && !path.isAbsolute(relative) ? relative.replace(/\\/g, '/') : null;
    };
    const others = [
      ...state.submodules.map((submodule) => submodule.folder),
      ...state.worktrees.filter((worktree) => !worktree.isCurrent).flatMap((worktree) => inside(worktree.folder) ?? []),
    ];
    const paths = files
      .flatMap((file) => inside(file) ?? [])
      .filter((file) => !others.some((other) => file === other || file.startsWith(`${other}/`)))
      .sort();
    const key = (file: string) => `${repo.root}\0${file}`;
    const unknown = paths.filter((file) => !this.ignored.has(key(file)));
    if (unknown.length) {
      // When the engine cannot say, a file is a change, and is asked about again the next time.
      const ignored = await reads.ignored(repo, unknown).catch(() => null);
      for (const file of ignored ? unknown : []) this.ignored.set(key(file), ignored!.has(file));
    }
    // A file saved since, or of another repository, is asked about again once it has edits not saved there.
    const asked = new Set(paths.map(key));
    for (const known of [...this.ignored.keys()]) if (!asked.has(known)) this.ignored.delete(known);
    return paths.filter((file) => this.ignored.get(key(file)) !== true);
  }

  /** Saves the editors of these files, by their paths from the root of the repository. */
  async save(paths: string[], repo = this.host.followers.repo): Promise<void> {
    if (!repo || !Array.isArray(paths)) return;
    const wanted = paths.map((file) => path.join(repo.root, file));
    const documents = vscode.workspace.textDocuments.filter(
      (document) => document.isDirty && document.uri.scheme === 'file' && wanted.some((file) => samePath(file, document.uri.fsPath)),
    );
    await Promise.all(documents.map((document) => document.save()));
  }
}

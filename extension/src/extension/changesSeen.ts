/**
 * What a change seen asks for: a file of a repository changed on the disk or saved in an editor, the window getting the
 * focus, a change heard of in a repository shown or kept, and the settings of the hosting services and of keeping
 * repositories open changed. A repository shown is read again for it; one kept is only marked out of date.
 */
import * as path from 'node:path';
import * as vscode from 'vscode';
import { reads } from '../engine/reads';
import type { ControllerHost } from './controllerHost';
import { errorText, keepOpen } from './repositorySession';
import type { UnsavedFiles } from './unsavedFiles';

/** Has the repositories shown read again, or the ones kept marked out of date, for what changed. */
export class ChangesSeen {
  constructor(
    private readonly host: Pick<ControllerHost, 'allGroups' | 'kept' | 'log' | 'scheduleRefresh' | 'sessions' | 'setState'>,
    private readonly unsaved: UnsavedFiles,
  ) {}

  /**
   * Turned off, the repositories kept forget at once what they showed, and the views forget what they kept of them;
   * turned on, the repository shown keeps what it shows once it is left.
   */
  keepOpenChanged(): void {
    if (!keepOpen()) this.host.kept.forget();
    for (const group of this.host.allGroups()) if (group.state) this.host.setState(group, { ...group.state, keepOpen: keepOpen() });
  }

  /** The services of one's own changed: the main remote of each repository shown may be on another, and the views say so. */
  async hostingChanged(): Promise<void> {
    if (!reads.available) return;
    for (const session of this.host.sessions.all()) {
      try {
        await session.detectHosting();
      } catch (error) {
        this.host.log.warn(`Could not read the remotes of ${session.root}: ${errorText(error)}`);
        continue;
      }
      for (const group of this.host.sessions.groupsOn(session)) {
        if (group.state) this.host.setState(group, { ...group.state, hosting: session.hosting, pulls: session.pulls });
      }
      void session.refreshPulls(true);
    }
  }

  /**
   * A file of the working tree changed in a repository (`repo.watch-changes`, M15): its session asks the engine whether
   * git ignores it before it reads. A repository kept is only marked out of date (Q461).
   */
  fileChangedIn(root: string, file: string): void {
    const session = this.host.sessions.find(root);
    if (!session) return this.host.kept.changed(root, `${file} changed`);
    session.fileChanged(file);
  }

  /**
   * A file was saved in an editor. One of the working tree of a repository shown asks for a read as a file changed on
   * the disk does, once the engine says git does not ignore it, and one of git's own, such as `.git/info/exclude`, at
   * once; one out of them asks for none, since the watchers of a repository kept see it. With no repository shown, it is
   * looked for again.
   */
  saved(uri: vscode.Uri): void {
    if (uri.scheme !== 'file') return;
    const sessions = this.host.sessions.all();
    if (!sessions.length) return this.host.scheduleRefresh(`${vscode.workspace.asRelativePath(uri)} was saved`);
    for (const session of sessions) {
      const relative = path.relative(session.root, uri.fsPath).replace(/\\/g, '/');
      if (!relative || relative.startsWith('..') || path.isAbsolute(relative)) continue;
      // A file that tells git what to ignore may change which unsaved files are changes.
      if (path.posix.basename(relative) === '.gitignore' || relative === '.git/info/exclude') this.unsaved.ignored.clear();
      if (/(^|\/)\.git(\/|$)/.test(relative)) session.scheduleRefresh(`${relative} was saved`);
      else session.fileChanged(relative);
    }
  }

  /** The window got the focus: each repository shown asks for a read when it missed a change or was read long ago. */
  focusGot(): void {
    const sessions = this.host.sessions.all();
    if (!sessions.length) return this.host.scheduleRefresh('VS Code got the focus');
    for (const session of sessions) session.focusGot();
  }

  /** A change seen in a repository: one shown is read again for it, one kept is only marked out of date. */
  changedIn(root: string, cause: string, delay?: number): void {
    const session = this.host.sessions.find(root);
    if (session) session.scheduleRefresh(cause, delay);
    else this.host.kept.changed(root, cause);
  }
}

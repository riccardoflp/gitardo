/**
 * What the menus of VS Code are told of the repository the panel and the side bar show, through context keys: which
 * files are hidden from git, whether it is a shallow clone, and whether it stores files with Git LFS. Each is told only
 * when it changed.
 */
import * as path from 'node:path';
import * as vscode from 'vscode';
import type { RepoState } from '../shared/protocol';

/** The context keys of the menus, and what they were last told. */
export class MenuContext {
  /** The files hidden from git the Explorer's menu was last told of, as JSON. */
  private hiddenInMenus = '';
  /** Whether the repository shown is a shallow clone, as the Command Palette was last told. */
  private shallowInMenus: boolean | undefined;
  /** Whether the Explorer's menu was last told that the repository stores files with Git LFS (`lfs.track`). */
  private lfsInMenus = false;

  /**
   * Tells the context menu of the Explorer which files of the repository shown are hidden from git, for it to offer
   * **Show Its Changes to Git Again** on them in place of **Hide Its Changes from Git…**; and the Command Palette
   * whether the repository shown is a shallow clone, for it to offer **Load More History** (`sync.deepen`).
   */
  showHiddenInMenus(state: RepoState | null): void {
    const shallow = !!state?.shallow?.length;
    if (shallow !== this.shallowInMenus) {
      this.shallowInMenus = shallow;
      void vscode.commands.executeCommand('setContext', 'gitardo.shallow', shallow);
    }
    const hidden = Object.fromEntries((state?.skipped ?? []).map((file) => [vscode.Uri.file(path.join(state?.root ?? '', file.path)).fsPath, true]));
    const said = JSON.stringify(hidden);
    if (said === this.hiddenInMenus) return;
    this.hiddenInMenus = said;
    void vscode.commands.executeCommand('setContext', 'gitardo.hiddenFromGit', hidden);
  }

  /**
   * Tells the context menu of the Explorer whether the repository shown stores files with Git LFS, for it to offer
   * **Store with Git LFS…** there alone, as the desktop's Files page does: Git for Windows sets Git LFS up for every
   * repository, and it would be offered everywhere (`lfs.track`).
   */
  showLfsInMenus(state: RepoState | null): void {
    const stores = !!state?.lfs?.patterns.length;
    if (stores === this.lfsInMenus) return;
    this.lfsInMenus = stores;
    void vscode.commands.executeCommand('setContext', 'gitardo.storesWithLfs', stores);
  }
}

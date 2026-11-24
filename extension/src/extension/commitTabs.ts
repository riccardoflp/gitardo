/** The tabs of the commits: a commit in an editor tab of its own, with its message, its files and what it changed in them. */
import * as vscode from 'vscode';
import { reads } from '../engine/reads';
import { subjectAndBody } from '../shared/message';
import type { FilesLayout } from '../shared/protocol';
import { isHash } from '../shared/refs';
import type { ControllerHost } from './controllerHost';
import { askRevision } from './revisions';

/** How the files of a commit are laid out in its tab: as a tree until a list is asked for. */
export const FILES_KEY = 'gitardo.commitFiles';
/** What fits of the message of a commit in the title of its tab. */
const TITLE_LENGTH = 32;

/** Opens the tabs of the commits, one for each commit. */
export class CommitTabs {
  /** The tabs of the commits, by repository and hash, and the repository each is of: not always the one being shown. */
  private readonly tabs = new Map<string, vscode.WebviewPanel>();

  constructor(private readonly host: Pick<ControllerHost, 'attach' | 'followers' | 'memento' | 'pin' | 'requireState'>) {}

  /**
   * Opens a commit in an editor tab of its own, with its message, its files and what it changed in them, or shows the
   * tab it has already. Without a hash it asks for one, or for anything else git understands.
   */
  async openCommit(hash?: unknown, repo = this.host.followers.repo): Promise<void> {
    if (hash === undefined) {
      repo = ((await this.host.requireState()) && this.host.followers.repo) || undefined;
      hash = repo && (await askRevision(repo, 'Open commit'))?.hash;
    }
    if (!repo || !isHash(hash)) return;
    const detail = await reads.commitDetail(repo, hash).catch(() => null);
    if (!detail) {
      void vscode.window.showWarningMessage(`Gitardo: ${hash.slice(0, 7)} is not a commit of this repository.`);
      return;
    }
    const commit = detail.id ?? hash;
    const key = `${repo.root}\n${commit}`;
    const open = this.tabs.get(key);
    if (open) return open.reveal();

    const { subject: whole } = subjectAndBody(detail.message);
    const subject = whole.length > TITLE_LENGTH ? `${whole.slice(0, TITLE_LENGTH).trimEnd()}…` : whole;
    const panel = vscode.window.createWebviewPanel('gitardo.commit', `${commit.slice(0, 7)} ${subject}`, vscode.ViewColumn.Active, {
      retainContextWhenHidden: true,
      // The search of VS Code, for a word among the changes.
      enableFindWidget: true,
    });
    const layout = this.host.memento.get<FilesLayout>(FILES_KEY, 'tree');
    // Kept at hand: a panel that is closed no longer says which webview was its own.
    const { webview } = panel;
    const attachment = this.host.attach(webview, 'commit', { commit, root: repo.root, files: layout });
    this.host.pin(webview, repo);
    this.tabs.set(key, panel);
    panel.onDidDispose(() => {
      attachment.dispose();
      this.host.pin(webview, null);
      this.tabs.delete(key);
    });
  }
}

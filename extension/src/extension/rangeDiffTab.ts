/**
 * The tab of what a rewrite changed, commit by commit (`range-diff.page`, milestone M36), which the follow-up
 * `openRangeDiff` of `range-diff.compare` opens. There is one, as on the desktop: opening another shows the new pairs
 * in it. The pairs are read by the engine (`read/range-diff.read-pairs`) in the repository the operation ran in, and a
 * click on a hash opens the tab of that commit there.
 */
import * as vscode from 'vscode';
import { type Repository, reads } from '../engine/reads';
import type { HostMessage, RangeDiffRead, ViewKind } from '../shared/protocol';
import { type RangeDiffTarget, rangeDiffTabTitle } from '../shared/rangeDiff';

/** What the tab asks of the controller: its page, the repository its messages are about, the activity, and its posts. */
export interface RangeDiffHost {
  attach(webview: vscode.Webview, kind: ViewKind, data?: Record<string, string>): vscode.Disposable;
  pin(webview: vscode.Webview, repo: Repository | null): void;
  track<T>(title: string, work: () => Promise<T>): Promise<T>;
  post(message: HostMessage, to: vscode.Webview): void;
}

/** What git or the engine said of a read that failed. */
const errorText = (error: unknown) => (error instanceof Error ? error.message : String(error));

export class RangeDiffTab {
  private panel: vscode.WebviewPanel | undefined;
  private read: RangeDiffRead = { kind: 'comparing' };
  /** Counts the openings, for the pairs of an earlier one read late to be dropped. */
  private opening = 0;

  constructor(private readonly host: RangeDiffHost) {}

  /** Whether a webview is that of this tab. */
  shows(webview: vscode.Webview): boolean {
    return this.panel?.webview === webview;
  }

  /** The tab has loaded: it is sent what it shows. */
  ready(): void {
    if (this.panel) this.host.post({ type: 'rangeDiff', read: this.read }, this.panel.webview);
  }

  /** Opens the tab, or shows the one open, on the pairs of two lines of a repository, read now. */
  async open(target: RangeDiffTarget, repo: Repository): Promise<void> {
    const panel = this.panel ?? this.create();
    panel.title = rangeDiffTabTitle(target.branch);
    panel.reveal();
    this.host.pin(panel.webview, repo);
    const opening = ++this.opening;
    this.show({ kind: 'comparing' });
    let read: RangeDiffRead;
    try {
      const pairs = await this.host.track(`Comparing the commits of ${target.branch}`, () => reads.rangeDiffPairs(repo, target.ranges));
      read = { kind: 'read', target, pairs };
    } catch (failure) {
      read = { kind: 'failed', target, message: errorText(failure) };
    }
    if (opening === this.opening) this.show(read);
  }

  private show(read: RangeDiffRead): void {
    this.read = read;
    this.ready();
  }

  private create(): vscode.WebviewPanel {
    const panel = vscode.window.createWebviewPanel('gitardo.rangeDiff', 'What changed', vscode.ViewColumn.Active, {
      retainContextWhenHidden: true,
      // The search of VS Code, for a word among the pairs.
      enableFindWidget: true,
    });
    const { webview } = panel;
    const attachment = this.host.attach(webview, 'rangeDiff');
    this.panel = panel;
    panel.onDidDispose(() => {
      attachment.dispose();
      this.host.pin(webview, null);
      if (this.panel === panel) this.panel = undefined;
    });
    return panel;
  }
}

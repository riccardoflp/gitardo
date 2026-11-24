/**
 * The **Large files** tab (`lfs.page`, milestone M47): an editor tab for each repository, as the tab of what new
 * worktrees get is, with whether Git LFS is there and set up, the patterns, the files of the commit checked out and
 * whether each is downloaded, and the locks of the server. Everything is read by the engine (`read/lfs.*`) and every
 * operation sent to it with `feature/run`: Download, Set up, Track a pattern, Stop storing, Lock and Unlock.
 *
 * The locks talk to the server: they are read when the tab opens, on **Refresh** and once **Lock** or **Unlock** ran,
 * never when the repository is read again nor when the tab comes back into sight (Q1195, Q1528). The tab keeps nothing
 * across a reload of the window.
 */
import * as path from 'node:path';
import * as vscode from 'vscode';
import type { EngineRun } from '../engine/actions';
import { lfsRun, locksRemote } from '../engine/lfs';
import { type Repository, reads } from '../engine/reads';
import { LFS_SITE, type LargeFilesAsk, type LargeFilesInfo, type LfsFile, type LfsStatus, type LocksShown } from '../shared/lfs';
import type { RepoState, ToViews } from '../shared/protocol';

/** What the tab needs of the controller. */
export interface LargeFilesHost {
  /** Loads the page of the tab into its webview. */
  attach(webview: vscode.Webview, root: string): vscode.Disposable;
  post(message: ToViews, webview: vscode.Webview): void;
  /** Runs operations of the engine in the repository, as the rest of the extension runs them; false when none ran. */
  run(runs: EngineRun[], repo: Repository): Promise<boolean>;
  /** The name of the repository, as the views say it. */
  name(root: string): string;
  /** The state held of the repository, for the remote whose server has the locks. */
  held(root: string): RepoState | null;
  /** Has the tabs of the scheme `gitardo` said not downloaded in the repository read again, once a download ran. */
  downloaded(root: string): void;
  error(text: string): void;
}

/** A tab open, with what it read. */
interface Tab {
  panel: vscode.WebviewPanel;
  repo: Repository;
  status: LfsStatus | null;
  files: LfsFile[];
  locks: LocksShown | null;
  /** The reads asked, the last one alone being shown. */
  reading: number;
}

/** Two spellings of the same folder are one key, whatever the slashes and, on Windows, the case. */
const keyOf = (root: string) => (process.platform === 'win32' ? path.resolve(root).toLowerCase() : path.resolve(root));

/** The **Large files** tabs, one for each repository. */
export class LargeFilesTabs {
  private readonly tabs = new Map<string, Tab>();

  constructor(private readonly host: LargeFilesHost) {}

  /** Opens the tab of a repository, or shows the one open for it. */
  open(repo: Repository): void {
    const open = this.tabs.get(keyOf(repo.root));
    if (open) return open.panel.reveal();
    const panel = vscode.window.createWebviewPanel('gitardo.largeFiles', `Large Files · ${this.host.name(repo.root)}`, vscode.ViewColumn.Active, {
      retainContextWhenHidden: true,
    });
    const tab: Tab = { panel, repo, status: null, files: [], locks: null, reading: 0 };
    this.tabs.set(keyOf(repo.root), tab);
    const attachment = this.host.attach(panel.webview, repo.root);
    panel.onDidDispose(() => {
      attachment.dispose();
      if (this.tabs.get(keyOf(repo.root)) === tab) this.tabs.delete(keyOf(repo.root));
    });
  }

  /** The tab of a webview, if it is one of these. */
  private tabOf(webview: vscode.Webview): Tab | undefined {
    return [...this.tabs.values()].find((tab) => tab.panel.webview === webview);
  }

  /** Whether a webview is the page of one of these tabs. */
  owns(webview: vscode.Webview): boolean {
    return !!this.tabOf(webview);
  }

  /** The tab of a repository read again, once its repository was: its files, not its locks, which talk to the server. */
  repositoryRead(root: string): void {
    const tab = this.tabs.get(keyOf(root));
    if (tab?.status) void this.read(tab, false);
  }

  /** The page asks for what it shows, as it opens. */
  async opened(webview: vscode.Webview): Promise<void> {
    const tab = this.tabOf(webview);
    if (tab) await this.read(tab, true);
  }

  /** What the page asks: a read, an operation of the engine, or a file or a site opened. */
  async ask(webview: vscode.Webview, ask: LargeFilesAsk): Promise<void> {
    const tab = this.tabOf(webview);
    if (!tab || !ask) return;
    const { root } = tab.repo;
    const file = (relative: string) => vscode.Uri.file(path.join(root, ...relative.split('/')));
    switch (ask.kind) {
      case 'refresh':
        return this.read(tab, true);
      case 'openAttributes':
      case 'openFile':
        return void vscode.commands.executeCommand('vscode.open', file(ask.kind === 'openFile' ? ask.path : ask.file));
      case 'revealFile':
        return void vscode.commands.executeCommand('revealInExplorer', file(ask.path));
      case 'openSite':
        return void vscode.env.openExternal(vscode.Uri.parse(LFS_SITE));
    }
    const operation = lfsRun(ask);
    if (!operation) return;
    const ran = await this.host.run([operation.run], tab.repo);
    if (ran && operation.downloads) this.host.downloaded(root);
    await this.read(tab, ran && operation.readLocks);
  }

  /** Reads Git LFS and the files, and the locks when asked, and sends the page what it shows. */
  private async read(tab: Tab, withLocks: boolean): Promise<void> {
    const reading = ++tab.reading;
    const { root } = tab.repo;
    const current = () => reading === tab.reading && this.tabs.get(keyOf(root)) === tab;
    try {
      const client = reads.client();
      tab.status = await client.lfsStatus(root);
      const version = tab.status.version;
      tab.files = version ? (await client.lfsFiles(root, version)).files : [];
      if (!version) tab.locks = null;
      else if (withLocks) tab.locks = { kind: 'reading' };
      if (!current()) return;
      this.send(tab);
      if (!version || !withLocks) return;
      const locks: LocksShown = await client.lfsLocks(root).catch((error: unknown) => ({ kind: 'failed', message: errorMessage(error) }));
      if (!current()) return;
      tab.locks = locks;
      this.send(tab);
    } catch (error) {
      this.host.error(errorMessage(error));
      if (current()) this.host.post({ type: 'largeFiles', info: null, error: errorMessage(error) }, tab.panel.webview);
    }
  }

  /** Sends the page what its tab read. */
  private send(tab: Tab): void {
    if (!tab.status) return;
    const info: LargeFilesInfo = {
      name: this.host.name(tab.repo.root),
      status: tab.status,
      files: tab.files,
      locks: tab.locks,
      remote: locksRemote(this.host.held(tab.repo.root)),
    };
    this.host.post({ type: 'largeFiles', info }, tab.panel.webview);
  }
}

/** What an error says, without the name of its class. */
const errorMessage = (error: unknown) => (error instanceof Error ? error.message : String(error));

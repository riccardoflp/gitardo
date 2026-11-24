/**
 * The tab of what new worktrees get (`worktree-setup.page`): what git ignores in the repository the views show, linked or
 * copied into each new worktree, and the commands run there; saved by the engine's plan, which VS Code writes the
 * settings of (M48). And the four entries of the Explorer's context menu that do the same for the files chosen there.
 */
import * as path from 'node:path';
import * as vscode from 'vscode';
import type { SetupSettingsTarget } from '../engine/messages';
import { type Repository, reads } from '../engine/reads';
import { samePath } from '../shared/paths';
import type { WebviewMessage } from '../shared/protocol';
import { type CommandProposal, type IgnoredEntry, cleanCommands, cleanPatterns } from '../shared/worktreeSetup';
import { SETUP_OFFERED } from './actions';
import type { ControllerHost } from './controllerHost';
import { errorText } from './repositorySession';
import { worktreeSetupInfo } from './worktreeSettings';

/** The page of what new worktrees get, offered once settings of new worktrees are saved. */
export const SETUP_PAGE = 'worktree-setup.page';

/** An entry of the context menu of VS Code's Explorer that gives new worktrees a path, or stops it. */
export type ExplorerSetup = 'link' | 'copy' | 'stopLinking' | 'stopCopying';

/** The engine's feature of each entry of the Explorer (M48). */
const EXPLORER_SETUP: Record<ExplorerSetup, string> = {
  link: 'worktree-setup.link-into-new-worktrees',
  copy: 'worktree-setup.copy-into-new-worktrees',
  stopLinking: 'worktree-setup.stop-linking',
  stopCopying: 'worktree-setup.stop-copying',
};

/** The tab of what new worktrees get, one at a time, for the repository it was opened on. */
export class WorktreeSetupTab {
  /**
   * The tab where what new worktrees get is chosen, the repository it is of, and what git ignores there and the commands
   * proposed for it, once looked at.
   */
  /** `saving` while the tab writes the settings one after the other, which it shows once they are all written. */
  private tab?: { panel: vscode.WebviewPanel; repo: Repository; entries: IgnoredEntry[] | null; proposed: CommandProposal[]; saving: boolean };

  constructor(
    private readonly host: Pick<ControllerHost, 'attach' | 'followers' | 'heldFor' | 'log' | 'memento' | 'pin' | 'post' | 'repos' | 'requireState' | 'run' | 'whenLocated'>,
  ) {}

  /**
   * Opens the tab where what new worktrees get of what git ignores is chosen, for the repository the views show, or
   * shows the one open for it.
   */
  async open(on = this.host.followers): Promise<void> {
    const repo = (await this.host.requireState(on)) && on.repo;
    if (!repo) return;
    // Once it has been seen, creating a worktree no longer offers it.
    void this.host.memento.update(SETUP_OFFERED, true);
    if (this.tab && samePath(this.tab.repo.root, repo.root)) return this.tab.panel.reveal();
    this.tab?.panel.dispose();

    const panel = vscode.window.createWebviewPanel('gitardo.worktreeSetup', `New Worktrees · ${path.basename(repo.root)}`, vscode.ViewColumn.Active, {
      retainContextWhenHidden: true,
    });
    const { webview } = panel;
    const tab = { panel, repo, entries: null as IgnoredEntry[] | null, proposed: [] as CommandProposal[], saving: false };
    this.tab = tab;
    const attachment = this.host.attach(webview, 'worktreeSetup', { root: repo.root });
    this.host.pin(webview, repo);
    // Settings changed elsewhere, in the Settings editor or settings.json, show here at once.
    const watching = vscode.workspace.onDidChangeConfiguration((event) => {
      const sections = [
        'gitardo.worktrees.linkFolders',
        'gitardo.worktrees.copyFiles',
        'gitardo.worktrees.setupCommands',
        'git.worktreeSymlinkFolders',
        'git.worktreeIncludeFiles',
      ];
      if (tab.saving) return;
      if (sections.some((section) => event.affectsConfiguration(section, vscode.Uri.file(repo.root)))) void this.send(webview, false);
    });
    panel.onDidDispose(() => {
      attachment.dispose();
      watching.dispose();
      this.host.pin(webview, null);
      if (this.tab === tab) this.tab = undefined;
    });
  }

  /** Sends the tab of the setup of new worktrees what it shows, looking again at what git ignores when `rescan`. */
  async send(webview: vscode.Webview, rescan: boolean): Promise<void> {
    const tab = this.tab;
    if (!tab || tab.panel.webview !== webview) return;
    const { root } = tab.repo;
    try {
      if (rescan || !tab.entries) [tab.entries, tab.proposed] = await Promise.all([reads.setupIgnored(root), reads.setupCommands(root)]);
      // The other worktrees can be given what applies only while the views show this repository, which that runs in.
      const state = this.host.heldFor(root);
      const others = state?.worktrees.filter((worktree) => !worktree.isMissing && !samePath(worktree.folder, root)).length ?? 0;
      this.host.post({ type: 'worktreeSetup', info: { ...worktreeSetupInfo(root, others, tab.proposed), entries: tab.entries } }, webview);
    } catch (error) {
      this.host.log.error(errorText(error));
      this.host.post({ type: 'worktreeSetup', info: null, error: errorText(error) }, webview);
    }
  }

  /**
   * Has new worktrees get the files and folders chosen in the Explorer linked or copied by a pattern of their own path,
   * or stop it, with the engine's `worktree-setup.*` entry in each repository they are in: a plan that VS Code's
   * settings are written by, where each list is set, shown first under `all` and logged like any other (M48).
   */
  async fromExplorer(entry: ExplorerSetup, chosen: vscode.Uri[]): Promise<void> {
    await this.host.whenLocated();
    const owner = (uri: vscode.Uri) => {
      const inside = this.host.repos.filter((repo) => {
        const relative = path.relative(repo.root, uri.fsPath);
        return relative !== '' && !relative.startsWith('..') && !path.isAbsolute(relative);
      });
      return inside.sort((a, b) => b.root.length - a.root.length)[0];
    };
    const outside: string[] = [];
    const byRepo = new Map<string, { repo: Repository; paths: string[] }>();
    for (const uri of chosen.filter((each) => each.scheme === 'file')) {
      const repo = owner(uri);
      if (!repo) {
        outside.push(`${path.basename(uri.fsPath)} is in no repository of the workspace.`);
        continue;
      }
      const relative = path.relative(repo.root, uri.fsPath).split(path.sep).join('/');
      const known = byRepo.get(repo.root) ?? { repo, paths: [] };
      byRepo.set(repo.root, { repo, paths: [...known.paths, relative] });
    }
    if (outside.length) void vscode.window.showWarningMessage(`Gitardo: ${outside.join(' ')}`);
    for (const { repo, paths } of byRepo.values()) {
      if (!(await this.host.run([{ feature: EXPLORER_SETUP[entry], target: { kind: 'files', paths } }], repo))) return;
    }
  }

  /** Sends the tab what new worktrees would get with the patterns it shows, before they are saved. */
  async preview(webview: vscode.Webview, repo: Repository, message: Extract<WebviewMessage, { type: 'previewWorktreeSetup' }>): Promise<void> {
    const patterns = { link: cleanPatterns(message.patterns?.link), copy: cleanPatterns(message.patterns?.copy) };
    try {
      const preview = await reads.setupPreview(repo.root, patterns);
      return this.host.post({ type: 'worktreeSetupPreview', patterns: message.patterns, preview }, webview);
    } catch (error) {
      return this.host.post({ type: 'worktreeSetupPreview', patterns: message.patterns, preview: null, error: errorText(error) }, webview);
    }
  }

  /** Saves what the tab shows, or removes it from where it was set, and sends the tab what it shows then. */
  async save(webview: vscode.Webview, repo: Repository, message: Extract<WebviewMessage, { type: 'saveWorktreeSetup' }>): Promise<void> {
    if (message.target !== 'user' && message.target !== 'workspace') return;
    const tab = this.tab?.panel.webview === webview ? this.tab : undefined;
    if (tab) tab.saving = true;
    // The engine's plan, shown first under `all` and logged; VS Code writes the settings it leaves (M48).
    const target: SetupSettingsTarget = {
      kind: 'setupSettings',
      place: message.target === 'user' ? 'user' : 'repository',
      link: cleanPatterns(message.patterns?.link),
      copy: cleanPatterns(message.patterns?.copy),
      commands: Array.isArray(message.commands) ? cleanCommands(message.commands) : [],
      alsoVsCode: message.alsoVSCode === true,
    };
    try {
      await this.host.run([{ feature: message.patterns ? 'worktree-setup.save' : 'worktree-setup.remove-from', target }], repo);
    } finally {
      if (tab) tab.saving = false;
    }
    return this.send(webview, false);
  }
}

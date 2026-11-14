import * as vscode from 'vscode';
import { samePath } from '../git/parse';

/** The little of the API of VS Code's Git extension that following its selection needs. */
interface GitRepository {
  rootUri: vscode.Uri;
  kind?: 'repository' | 'submodule' | 'worktree';
  /**
   * Whether it is the repository Source Control is on: the one last clicked in the Repositories view, or whose changes
   * or commit box were last clicked. One at a time, whichever of them are listed in the view.
   */
  ui: { readonly selected: boolean; onDidChange: vscode.Event<void> };
}

interface GitApi {
  repositories: GitRepository[];
  onDidOpenRepository: vscode.Event<GitRepository>;
  onDidCloseRepository: vscode.Event<GitRepository>;
}

/**
 * The command of the Repositories view that lists a repository in Source Control, or stops listing it, one for each
 * repository the view has, by the number VS Code gave its source control. While the view lists one repository at a
 * time, listing another moves Source Control onto it; VS Code has no other way for an extension to do that.
 */
const TOGGLE_PREFIX = 'workbench.scm.action.toggleRepositoryVisibility.scm';

/** How long a repository just opened is left to settle: VS Code may move Source Control onto it a moment after. */
const SETTLE_MS = 1500;

/** How long a selection asked of VS Code is waited for before it is taken as not done. */
const CONFIRM_MS = 1000;

/**
 * Keeps the repository Gitardo shows and the one selected in VS Code's Repositories view the same: choosing one in the
 * view shows it in Gitardo, and, while the view selects one repository at a time, choosing one in Gitardo selects it in
 * the view.
 */
export class SourceControlSelection implements vscode.Disposable {
  private api?: GitApi;
  private readonly subscriptions: vscode.Disposable[] = [];
  // The Git extension hands out a new object for a repository each time it is asked, so they are kept by their roots.
  private readonly watched = new Map<string, vscode.Disposable>();
  /** When each repository was opened, for those still settling. */
  private readonly opened = new Map<string, number>();
  /** The repository selected when the view was last looked at. */
  private selected?: string;
  private timer?: ReturnType<typeof setTimeout>;
  /** Set while Gitardo selects a repository in the view, whose change is its own and not followed. */
  private selecting = false;
  /** Set once selecting in the view did not do what was asked: the commands are not what they were taken for. */
  private broken = false;

  constructor(
    /** Shows a repository in Gitardo, as chosen in the view. */
    private readonly show: (root: string) => Promise<void>,
    private readonly log: vscode.LogOutputChannel,
  ) {}

  /** Starts following, once VS Code's Git extension is there. Nothing happens without it. */
  async start(): Promise<void> {
    const extension = vscode.extensions.getExtension<{ getAPI(version: 1): GitApi }>('vscode.git');
    if (!extension) return;
    try {
      const exports = extension.isActive ? extension.exports : await extension.activate();
      this.api = exports.getAPI(1);
    } catch (error) {
      // With `git.enabled` off the extension gives no API: there is no selection to follow.
      this.log.debug(`VS Code's Git extension gives no API, so the Repositories view is not followed: ${errorText(error)}`);
      return;
    }
    const api = this.api;
    for (const repository of api.repositories) this.watch(repository, false);
    this.selected = this.selectedNow();
    this.subscriptions.push(
      api.onDidOpenRepository((repository) => this.watch(repository, true)),
      api.onDidCloseRepository((repository) => {
        const key = keyOf(repository);
        this.watched.get(key)?.dispose();
        this.watched.delete(key);
        this.opened.delete(key);
      }),
    );
  }

  private watch(repository: GitRepository, justOpened: boolean): void {
    const key = keyOf(repository);
    if (this.watched.has(key)) return;
    if (justOpened) this.opened.set(key, Date.now());
    this.watched.set(
      key,
      repository.ui.onDidChange(() => this.changed()),
    );
  }

  private selectedNow(): string | undefined {
    const repository = this.api?.repositories.find((each) => each.ui.selected);
    return repository && keyOf(repository);
  }

  /** The repository left and the one selected tell of their change apart: they are looked at together, once both have. */
  private changed(): void {
    clearTimeout(this.timer);
    this.timer = setTimeout(() => void this.follow(), 50);
  }

  private async follow(): Promise<void> {
    const before = this.selected;
    const now = this.selectedNow();
    this.selected = now;
    if (this.selecting || !now || now === before) return;
    const at = this.opened.get(now);
    if (at !== undefined) {
      if (Date.now() - at < SETTLE_MS) return;
      this.opened.delete(now);
    }
    const repository = this.api?.repositories.find((each) => keyOf(each) === now);
    if (repository) await this.show(repository.rootUri.fsPath);
  }

  /**
   * Selects the repository in the view, as a click on it would, while the view lists one repository at a time. With
   * several listed at once, VS Code gives no way to move Source Control onto one without taking others out of the
   * view, so nothing is done. Nothing either where VS Code does not have the repository open.
   */
  async select(root: string): Promise<void> {
    const api = this.api;
    if (!api || this.broken) return;
    const target = api.repositories.find((repository) => samePath(repository.rootUri.fsPath, root));
    if (!target || target.ui.selected) return;
    if (vscode.workspace.getConfiguration('scm').get('repositories.selectionMode') !== 'single') return;
    const command = await this.toggleCommand(api, target);
    if (!command) return;
    this.selecting = true;
    try {
      this.log.debug(`${command}: selects ${target.rootUri.fsPath} in the Repositories view`);
      await vscode.commands.executeCommand(command);
      if (!(await this.confirm(target))) {
        this.broken = true;
        this.log.warn(`Selecting ${root} in the Repositories view did not select it: Gitardo no longer selects there.`);
      }
    } catch (error) {
      this.log.warn(`Could not select ${root} in the Repositories view: ${errorText(error)}`);
    } finally {
      this.selected = this.selectedNow();
      this.selecting = false;
    }
  }

  /**
   * The command that lists the repository in the view. The commands are named after the numbers VS Code gives source
   * controls as they are made, in the order the Git extension lists its repositories; that holds only while the view
   * has the repositories of Git alone, so there is none when it has more or fewer of them.
   */
  private async toggleCommand(api: GitApi, target: GitRepository): Promise<string | undefined> {
    const ids = (await vscode.commands.getCommands(true))
      .filter((id) => id.startsWith(TOGGLE_PREFIX) && /^\d+$/.test(id.slice(TOGGLE_PREFIX.length)))
      .sort((a, b) => Number(a.slice(TOGGLE_PREFIX.length)) - Number(b.slice(TOGGLE_PREFIX.length)));
    // VS Code keeps out of the view the worktrees its agents make, which are named so.
    const listed = api.repositories.filter((repository) => !(repository.kind === 'worktree' && /^(copilot|agents)-/.test(baseName(repository))));
    if (ids.length !== listed.length) {
      this.log.debug(`The Repositories view has ${ids.length} repositories and the Git extension ${listed.length}: Gitardo does not select there.`);
      return undefined;
    }
    return ids[listed.findIndex((repository) => keyOf(repository) === keyOf(target))];
  }

  /** Whether the repository becomes the one selected. */
  private confirm(repository: GitRepository): Promise<boolean> {
    if (repository.ui.selected) return Promise.resolve(true);
    return new Promise((resolve) => {
      const done = (selected: boolean) => {
        clearTimeout(timer);
        listener.dispose();
        resolve(selected);
      };
      const timer = setTimeout(() => done(repository.ui.selected), CONFIRM_MS);
      const listener = repository.ui.onDidChange(() => repository.ui.selected && done(true));
    });
  }

  dispose(): void {
    clearTimeout(this.timer);
    for (const each of [...this.subscriptions, ...this.watched.values()]) each.dispose();
  }
}

function keyOf(repository: GitRepository): string {
  return repository.rootUri.toString();
}

function baseName(repository: GitRepository): string {
  return repository.rootUri.path.slice(repository.rootUri.path.lastIndexOf('/') + 1);
}

function errorText(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

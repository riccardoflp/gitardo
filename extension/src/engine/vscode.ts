/**
 * The engine in a window of VS Code: the binary found from `gitardo.engine.path` or the package, the settings read
 * from VS Code, and the log in the **Gitardo** output channel. In stage 1 nothing the user sees uses it: it is started
 * the first time a repository is found, and the output channel says when it cannot be.
 */
import * as vscode from 'vscode';
import { findBinary } from './binary';
import { EngineClient } from './client';
import type { Settings } from './messages';
import { engineSettings } from './settings';

/** The keys of every `gitardo.*` setting the extension contributes, without the prefix. */
function settingKeys(extension: vscode.Extension<unknown>): string[] {
  const json = extension.packageJSON as {
    contributes?: { configuration?: { properties?: Record<string, unknown> } };
  };
  return Object.keys(json.contributes?.configuration?.properties ?? {})
    .filter((key) => key.startsWith('gitardo.'))
    .map((key) => key.slice('gitardo.'.length));
}

/** The settings as VS Code resolves them now, by the desktop's names. */
export function readSettings(extension: vscode.Extension<unknown>): Settings {
  const ours = vscode.workspace.getConfiguration('gitardo');
  const values: Record<string, unknown> = {};
  const unset = new Set<string>();
  for (const key of settingKeys(extension)) {
    values[key] = ours.get(key);
    const set = ours.inspect(key);
    if (set?.workspaceFolderValue === undefined && set?.workspaceValue === undefined && set?.globalValue === undefined) unset.add(key);
  }
  const git = vscode.workspace.getConfiguration('git');
  const setValue = (key: string) => {
    const set = git.inspect(key);
    return set?.workspaceFolderValue ?? set?.workspaceValue ?? set?.globalValue;
  };
  const fallbacks = {
    branchSortOrder: setValue('branchSortOrder'),
    worktreeSymlinkFolders: setValue('worktreeSymlinkFolders'),
    worktreeIncludeFiles: setValue('worktreeIncludeFiles'),
  };
  return engineSettings(values, unset, fallbacks, -new Date().getTimezoneOffset());
}

/** The engine of this window, started on the first call to `start`; none when its binary is missing. */
export class WindowEngine implements vscode.Disposable {
  private client?: EngineClient;
  private looked = false;

  constructor(
    private readonly context: vscode.ExtensionContext,
    private readonly log: vscode.LogOutputChannel,
  ) {}

  /** Starts the engine the first time; the output channel says once when there is no binary for this system. */
  start(): EngineClient | undefined {
    if (this.looked) return this.client;
    this.looked = true;
    // Only the user's own settings may name another binary: a workspace's .vscode/settings.json comes with the repository.
    const setting = vscode.workspace.getConfiguration('gitardo').inspect<string>('engine.path')?.globalValue;
    const binary = findBinary(this.context.extensionPath, setting);
    if (!binary) {
      this.log.info(
        setting?.trim()
          ? `Gitardo's engine was not found at ${setting}, which gitardo.engine.path names; Gitardo works without it.`
          : `Gitardo's engine has no binary for ${process.platform}-${process.arch} in this package; Gitardo works without it.`,
      );
      return undefined;
    }
    const client = new EngineClient({
      binary,
      clientVersion: String((this.context.extension.packageJSON as { version?: string }).version ?? '0.0.0'),
      settings: () => readSettings(this.context.extension),
      logLevel: () => (this.log.logLevel <= vscode.LogLevel.Debug ? 'debug' : 'info'),
      log: this.log,
    });
    this.client = client;
    client.start().catch((error: unknown) => this.log.warn(error instanceof Error ? error.message : String(error)));
    return client;
  }

  dispose(): void {
    void this.client?.dispose();
  }
}

import * as os from 'node:os';
import * as vscode from 'vscode';
import type { Action, RepoState } from '../shared/protocol';
import { samePath } from '../shared/paths';
import { isHome } from '../shared/refs';
import { submoduleFolder } from '../shared/submodules';
import { worktreeBranch } from '../shared/state';

/*
 * What VS Code does of the actions of the views, around the engine, which runs every operation on the repository:
 * opening a worktree or a submodule in a window, a terminal or the file manager, the folder of this window, which only
 * the extension knows, and VS Code's Git extension, which has to let go of what the engine moves or deletes.
 */

/** Where the worktrees made last remember the inputs of their branch names, in the workspace state. */
export const LAST_INPUTS = 'worktreeInputs';

/** What is written in the output channel. */
export interface HostLog {
  info(line: string): void;
  warn(line: string): void;
}

/**
 * Where new worktrees go. `folder` is the setting: `${repo}` in it is the name of the main worktree's folder, and a
 * relative path starts from the folder that holds the main worktree.
 */
function worktreesRoot(main: string, folder: string): string {
  // With forward slashes, as git lists worktrees on every system.
  const slashes = (text: string) => text.replace(/\\/g, '/').replace(/\/+$/, '');
  const home = slashes(main);
  const repo = home.slice(home.lastIndexOf('/') + 1);
  const where = slashes((folder.trim() || '${repo}.worktrees').replace(/\$\{repo\}/g, repo));
  const absolute = /^([a-zA-Z]:)?\//.test(where);
  return absolute ? where : `${home.slice(0, home.lastIndexOf('/'))}/${where.replace(/^\.\//, '')}`;
}

/** Where new worktrees go, for the views to name the ones there by their path from it, as `gitardo.worktrees.folder` says. */
export function worktreesFolder(state: RepoState): string {
  const main = state.worktrees.find((worktree) => worktree.isMain)?.folder ?? state.root;
  const setting = vscode.workspace.getConfiguration('gitardo').get<string>('worktrees.folder', '');
  return worktreesRoot(main, setting.trim().replace(/^~(?=[\\/]|$)/, os.homedir()));
}

/** The command that opens the Agents window of VS Code. It is not part of the documented API, hence the fallback. */
export const AGENTS_WINDOW = 'workbench.action.openAgentsWindow';

/** Where a worktree opens when nothing says: as the settings choose, in the Agents window only where VS Code has one. */
export function openIn(state: Pick<RepoState, 'agentsWindow'>): 'window' | 'agents' {
  const setting = vscode.workspace.getConfiguration('gitardo').get<string>('worktrees.openIn', 'window');
  return setting === 'agents' && state.agentsWindow ? 'agents' : 'window';
}

/** Opens a worktree in a window of its own, or in the Agents window for agent sessions in it. */
export async function openWorktree(dir: string, where: 'window' | 'agents'): Promise<void> {
  const folder = vscode.Uri.file(dir);
  if (where === 'agents') {
    try {
      await vscode.commands.executeCommand(AGENTS_WINDOW, { folderUri: folder });
      return;
    } catch (error) {
      const reason = error instanceof Error ? error.message : String(error);
      void vscode.window.showWarningMessage(`The Agents window did not open (${reason}), so the worktree opens in a new window.`);
    }
  }
  await vscode.commands.executeCommand('vscode.openFolder', folder, { forceNewWindow: true });
}

/** Whether creating a worktree has offered once to choose what new worktrees get, and the button that offers it. */
export const SETUP_OFFERED = 'worktreeSetupOffered';
export const SET_UP = 'Choose What It Gets…';

/**
 * Does what of an action is VS Code's own: true when it did, or when it refused it with a message; false for the
 * actions the engine runs.
 */
export async function hostAction(action: Action, state: RepoState): Promise<boolean> {
  switch (action.kind) {
    case 'worktreeOpen':
      await openWorktree(action.path, action.where ?? openIn(state));
      return true;
    case 'worktreeTerminal':
      vscode.window.createTerminal({ name: action.path.replace(/\\/g, '/').split('/').pop(), cwd: action.path }).show();
      return true;
    case 'worktreeReveal':
      await vscode.commands.executeCommand('revealFileInOS', vscode.Uri.file(action.path));
      return true;
    case 'worktreeRemove':
    case 'worktreeRename':
      // The engine does not know which folder this window has open.
      if (!isHome(action.path, state)) return false;
      throw new Error(`This worktree is the folder open in this window: ${action.kind === 'worktreeRemove' ? 'remove' : 'rename'} it from another one.`);
    case 'openInWorktree': {
      // A branch with a worktree only opens it, where gitardo.worktrees.openIn says, the Agents window included.
      const existing = action.remote ? undefined : state.worktrees.find((worktree) => worktreeBranch(worktree) === action.name);
      // The engine says so of the one checked out in this window.
      if (!existing || existing.isCurrent) return false;
      if (existing.isMissing) throw new Error(`The worktree of ${action.name} is missing from ${existing.folder}: prune the worktrees to free the branch.`);
      await openWorktree(existing.folder, openIn(state));
      return true;
    }
    case 'submoduleOpen': {
      const submodule = state.submodules.find((each) => each.folder === action.path);
      if (!submodule) throw new Error(`${action.path} is not a submodule of this repository.`);
      if (!submodule.checkout?.commit) throw new Error(`${submodule.folder} is not initialized: update it first.`);
      await openWorktree(submoduleFolder(submodule, state), 'window');
      return true;
    }
    default:
      return false;
  }
}

/** The little of the API of VS Code's Git extension that Gitardo uses. */
interface VSCodeGit {
  getAPI(version: 1): { repositories: { rootUri: vscode.Uri; status(): Promise<void> }[] };
}

/** Whether VS Code's Git extension has this repository open, and so can serve the versions of its files. */
export function inVSCodeGit(root: string): boolean {
  const extension = vscode.extensions.getExtension<VSCodeGit>('vscode.git');
  if (!extension?.isActive) return false;
  try {
    return extension.exports.getAPI(1).repositories.some((repository) => samePath(repository.rootUri.fsPath, root));
  } catch {
    return false;
  }
}

/**
 * Has VS Code's Git extension read the status of the repository again, when it has it open. It serves the index of a
 * file only while it knows the file is staged, and HEAD otherwise: after Gitardo staged it, it may not know that yet.
 */
export async function refreshVSCodeGit(root: string): Promise<void> {
  const extension = vscode.extensions.getExtension<VSCodeGit>('vscode.git');
  if (!extension?.isActive) return;
  try {
    await extension.exports.getAPI(1).repositories.find((repository) => samePath(repository.rootUri.fsPath, root))?.status();
  } catch {
    // the diff opens all the same
  }
}

/**
 * VS Code's Git extension opens the worktrees of the repositories it has as repositories of their own, runs git in
 * them and watches their folders: on Windows a folder that is watched, or that git runs in, can be neither moved nor
 * removed. Before the engine moves or removes worktrees, the extension closes the ones it has open.
 */
export async function closeInVSCodeGit(dirs: string[], log: HostLog): Promise<void> {
  const extension = vscode.extensions.getExtension<VSCodeGit>('vscode.git');
  if (!extension?.isActive) return;
  let repositories: { rootUri: vscode.Uri }[];
  try {
    ({ repositories } = extension.exports.getAPI(1));
  } catch {
    return;
  }
  for (const dir of dirs) {
    // Only one open as that very folder: given another, the command closes the repository that holds it, or asks which.
    const open = repositories.find((repository) => samePath(repository.rootUri.fsPath, dir));
    if (!open) continue;
    log.info(`> git.close ${dir}`);
    try {
      await vscode.commands.executeCommand('git.close', open);
    } catch (error) {
      log.warn(`VS Code's Git extension did not close ${dir}: ${error instanceof Error ? error.message : String(error)}`);
    }
  }
}

import * as path from 'node:path';
import * as vscode from 'vscode';
import type { SetupGiven } from '../engine/messages';
import {
  cleanCommands,
  cleanPatterns,
  namedPaths,
  type CommandProposal,
  type SetupLayer,
  type SetupPatterns,
  type SetupSource,
  type SetupTarget,
  type WorktreeSetupInfo,
} from '../shared/worktreeSetup';

/** Gitardo's settings, under `gitardo`, and VS Code's, under `git`, which apply where Gitardo's are not set. */
const OURS = { link: 'worktrees.linkFolders', copy: 'worktrees.copyFiles' } as const;
const THEIRS = { link: 'worktreeSymlinkFolders', copy: 'worktreeIncludeFiles' } as const;

type Kind = keyof typeof OURS;

const config = (section: 'gitardo' | 'git', root: string) => vscode.workspace.getConfiguration(section, vscode.Uri.file(root));

/**
 * The patterns that new worktrees of the repository in `root` get, and where each of the two comes from: Gitardo's
 * setting wherever it is set, even empty, or else VS Code's.
 */
export function setupPatterns(root: string): SetupPatterns & { linkFrom: SetupSource; copyFrom: SetupSource } {
  const one = (kind: Kind): [string[], SetupSource] => {
    const set = config('gitardo', root).inspect<unknown>(OURS[kind]);
    const workspace = set?.workspaceFolderValue ?? set?.workspaceValue;
    if (workspace !== undefined) return [cleanPatterns(workspace), 'workspace'];
    if (set?.globalValue !== undefined) return [cleanPatterns(set.globalValue), 'user'];
    const theirs = cleanPatterns(config('git', root).get(THEIRS[kind]));
    return [theirs, theirs.length ? 'vscode' : 'none'];
  };
  const [link, linkFrom] = one('link');
  const [copy, copyFrom] = one('copy');
  return { link, copy, linkFrom, copyFrom };
}

/** Gitardo's setting of the commands run in new worktrees, which VS Code has none of. */
const COMMANDS = 'worktrees.setupCommands';

/**
 * The commands run in new worktrees of the repository in `root`, and where they come from: the workspace folder's
 * setting wherever it is set, even empty, or else the user's.
 */
export function setupCommands(root: string): { commands: string[]; from: SetupSource } {
  const set = config('gitardo', root).inspect<unknown>(COMMANDS);
  const workspace = set?.workspaceFolderValue ?? set?.workspaceValue;
  if (workspace !== undefined) return { commands: cleanCommands(workspace), from: 'workspace' };
  if (set?.globalValue !== undefined) return { commands: cleanCommands(set.globalValue), from: 'user' };
  return { commands: [], from: 'none' };
}

/** What is set in one place, each of the three missing where it is not set there. */
function layer(root: string, place: 'globalValue' | 'workspaceFolderValue'): SetupLayer {
  const found = (key: string) => {
    const set = config('gitardo', root).inspect<unknown>(key);
    return place === 'globalValue' ? set?.globalValue : (set?.workspaceFolderValue ?? set?.workspaceValue);
  };
  const value = (kind: Kind) => {
    const patterns = found(OURS[kind]);
    return patterns === undefined ? undefined : cleanPatterns(patterns);
  };
  const commands = found(COMMANDS);
  return { link: value('link'), copy: value('copy'), commands: commands === undefined ? undefined : cleanCommands(commands) };
}

/**
 * What the page shows of the settings of the repository in `root`, whose other worktrees are `worktrees`, with the
 * commands `proposed` for it: all but what git ignores, which takes longer to look at.
 */
export function worktreeSetupInfo(root: string, worktrees: number, proposed: CommandProposal[]): Omit<WorktreeSetupInfo, 'entries'> {
  const folder = vscode.workspace.getWorkspaceFolder(vscode.Uri.file(root));
  const { link, copy, linkFrom, copyFrom } = setupPatterns(root);
  const { commands, from } = setupCommands(root);
  return {
    root,
    user: layer(root, 'globalValue'),
    workspace: folder
      ? { ...layer(root, 'workspaceFolderValue'), name: folder.name, file: path.join(folder.uri.fsPath, '.vscode', 'settings.json') }
      : null,
    vscode: { link: cleanPatterns(config('git', root).get(THEIRS.link)), copy: cleanPatterns(config('git', root).get(THEIRS.copy)) },
    effective: { link, copy, linkFrom, copyFrom },
    worktrees,
    commands: { effective: commands, from, proposed },
  };
}

/**
 * What new worktrees get in the folder of the repository in `root`, with where each list is set, as the engine's
 * features that save it are given it (`setup` of `feature/run`): they plan the settings VS Code then writes.
 */
export function setupGiven(root: string): SetupGiven {
  const { link, copy, linkFrom, copyFrom } = setupPatterns(root);
  const { commands, from } = setupCommands(root);
  return {
    linkFolders: { values: link, from: linkFrom },
    copyFiles: { values: copy, from: copyFrom },
    setupCommands: { values: commands, from },
    inWorkspaceFolder: vscode.workspace.getWorkspaceFolder(vscode.Uri.file(root)) !== undefined,
  };
}

/** Opens the file of the settings of the user or of the workspace folder, at Gitardo's setting of the folders to link. */
export async function editSetupJson(root: string, target: SetupTarget): Promise<void> {
  const folder = vscode.workspace.getWorkspaceFolder(vscode.Uri.file(root));
  if (target === 'user' || !folder) {
    await vscode.commands.executeCommand('workbench.action.openSettingsJson', { revealSetting: { key: `gitardo.${OURS.link}` } });
    return;
  }
  const file = vscode.Uri.joinPath(folder.uri, '.vscode', 'settings.json');
  try {
    await vscode.workspace.fs.stat(file);
    await vscode.window.showTextDocument(file);
  } catch {
    // Not there yet: VS Code makes it, as it does from its own Settings editor.
    await vscode.commands.executeCommand('workbench.action.openFolderSettingsFile', folder.uri);
  }
}

/**
 * Tells the context menu of the Explorer which files and folders of the repositories in `roots` new worktrees get by a
 * pattern of their own path, for it to offer to stop it.
 */
export function showSetupInMenus(roots: string[]): void {
  const named = { link: {} as Record<string, true>, copy: {} as Record<string, true> };
  for (const root of roots) {
    const patterns = setupPatterns(root);
    for (const kind of ['link', 'copy'] as const) {
      for (const relative of namedPaths(patterns[kind])) named[kind][vscode.Uri.file(path.join(root, relative)).fsPath] = true;
    }
  }
  void vscode.commands.executeCommand('setContext', 'gitardo.linkedIntoWorktrees', named.link);
  void vscode.commands.executeCommand('setContext', 'gitardo.copiedIntoWorktrees', named.copy);
}

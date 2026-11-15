import * as path from 'node:path';
import * as vscode from 'vscode';
import { reads } from '../engine/reads';
import {
  chooseByPath,
  cleanCommands,
  cleanPatterns,
  namedPaths,
  statusOf,
  type CommandProposal,
  type IgnoredEntry,
  type SetupChoice,
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

const targetOf = (target: SetupTarget) =>
  target === 'user' ? vscode.ConfigurationTarget.Global : vscode.ConfigurationTarget.WorkspaceFolder;

/**
 * Saves the patterns in the settings of the user or of the workspace folder that has the repository, and in VS Code's
 * own settings there too when `alsoVSCode`, for the worktrees VS Code creates. Null patterns take the settings out of
 * that place, the commands too, so that what is set elsewhere applies again. The commands, when given, are saved with
 * the patterns; an empty list of them is written only where one is set somewhere, as it would change nothing else.
 */
export async function saveSetup(
  root: string,
  target: SetupTarget,
  patterns: SetupPatterns | null,
  alsoVSCode: boolean,
  commands?: string[],
): Promise<void> {
  const where = targetOf(target);
  for (const kind of ['link', 'copy'] as const) {
    const value = patterns ? cleanPatterns(patterns[kind]) : undefined;
    await config('gitardo', root).update(OURS[kind], value, where);
    if (alsoVSCode && value) await config('git', root).update(THEIRS[kind], value, where);
  }
  if (!patterns) await config('gitardo', root).update(COMMANDS, undefined, where);
  else if (commands) {
    const value = cleanCommands(commands);
    if (value.length || setupCommands(root).from !== 'none') await config('gitardo', root).update(COMMANDS, value, where);
  }
}

/**
 * Saves the patterns of the folders to link, or of the files to copy, where what applies now comes from: the settings
 * of the user or of the workspace folder that has the repository. Where VS Code's setting applies, or none, Gitardo's
 * goes in the workspace folder, or in the user's settings when the repository is in no folder of the workspace. Returns
 * where it went.
 */
export async function savePatterns(root: string, kind: Kind, patterns: string[], from: SetupSource): Promise<SetupTarget> {
  const target = from === 'user' || !vscode.workspace.getWorkspaceFolder(vscode.Uri.file(root)) ? 'user' : 'workspace';
  await config('gitardo', root).update(OURS[kind], cleanPatterns(patterns), targetOf(target));
  return target;
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

/** What became of a file or folder of the checkout, given to new worktrees, or taken from them, from the Explorer. */
export type SetupChange =
  /** The patterns now give it what was chosen; `partly`, only what git ignores in it. */
  | { kind: 'done'; partly: boolean }
  /** Taken out of the patterns, though another pattern, `by`, still gives it something. */
  | { kind: 'still'; choice: 'link' | 'copy'; by: string }
  /** Another pattern, `by`, gives it what was chosen already. */
  | { kind: 'already'; by: string }
  /** Git tracks it, or files in it: each worktree has it from its commit. */
  | { kind: 'tracked' }
  /** Git does not ignore it, or not all of it. */
  | { kind: 'notIgnored'; partly: boolean }
  /** It is the folder `folder`, or inside it, which the pattern `by` links: nothing in a linked folder is copied. */
  | { kind: 'linked'; folder: string; by: string };

/** Whether one of these paths is the entry, or a folder it is in. */
const covers = (taken: string[], entry: string) => taken.some((each) => entry === each || entry.startsWith(`${each}/`));

/** What the patterns take from the checkout in `root`, as the engine works it out: the folders linked and the paths copied. */
async function taken(root: string, patterns: SetupPatterns): Promise<SetupPatterns> {
  const [preview] = await reads.setupTakes(root, [{ folder: root, link: patterns.link, copy: patterns.copy, commit: null }]);
  return { link: preview?.linked ?? [], copy: preview?.copied ?? [] };
}

/** The first of the patterns of `choice` that gives `entry` something by itself, that is, takes it or a folder it is in. */
async function takenBy(root: string, entry: string, choice: 'link' | 'copy', patterns: SetupPatterns): Promise<string> {
  // A path is copied only outside the folders linked.
  const requests = patterns[choice].map((pattern) => ({
    folder: root,
    link: choice === 'link' ? [pattern] : patterns.link,
    copy: choice === 'copy' ? [pattern] : [],
    commit: null,
  }));
  const previews = await reads.setupTakes(root, requests);
  const found = previews.map((preview) => covers(choice === 'link' ? preview.linked : preview.copied, entry));
  return patterns[choice][found.indexOf(true)] ?? '';
}

/**
 * Sets what new worktrees get of the file or folder `entry` of the checkout in `root` by the pattern of its path, as
 * the context menu of the Explorer does: linked, copied, or nothing by a pattern of its own. Returns the patterns to
 * save, the same ones when nothing is to change, and what became of it, as the engine works the patterns out
 * (`worktree-setup.read-what-patterns-take`, `worktree-setup.explain-change`).
 */
export async function changeSetup(
  root: string,
  entry: Pick<IgnoredEntry, 'path' | 'folder'>,
  choice: SetupChoice,
  patterns: SetupPatterns,
): Promise<{ patterns: SetupPatterns; change: SetupChange }> {
  const next = chooseByPath(patterns, entry, choice);
  if (choice === 'none') {
    const still = statusOf(entry, await taken(root, next));
    if (still.choice === 'none' || still.partly) return { patterns: next, change: { kind: 'done', partly: false } };
    return { patterns: next, change: { kind: 'still', choice: still.choice, by: await takenBy(root, entry.path, still.choice, next) } };
  }

  const was = statusOf(entry, await taken(root, patterns));
  if (was.choice === choice && !was.partly) return { patterns, change: { kind: 'already', by: await takenBy(root, entry.path, choice, patterns) } };

  const after = await taken(root, next);
  const now = statusOf(entry, after);
  if (now.choice === choice && (!now.partly || choice === 'copy')) return { patterns: next, change: { kind: 'done', partly: now.partly } };
  if (choice === 'copy' && now.choice === 'link' && !now.partly) {
    const folder = after.link.find((each) => covers([each], entry.path))!;
    return { patterns, change: { kind: 'linked', folder, by: await takenBy(root, entry.path, 'link', next) } };
  }
  const [status] = await reads.pathStatuses(root, [entry.path]);
  const tracked = status !== undefined && status.tracked !== 'nothing';
  const ignored = status !== undefined && status.ignored !== 'nothing';
  if (tracked && (!entry.folder || choice === 'link' || !ignored)) return { patterns, change: { kind: 'tracked' } };
  return { patterns, change: { kind: 'notIgnored', partly: ignored } };
}

/** What became of `name`, a file or a folder of the checkout, given to new worktrees or taken from them, said for the user. */
export function setupChangeText(name: string, folder: boolean, choice: SetupChoice, change: SetupChange): string {
  const done = (taken: 'link' | 'copy') => (taken === 'link' ? 'linked' : 'copied');
  switch (change.kind) {
    case 'done':
      if (choice === 'none') return `New worktrees will no longer get ${name}.`;
      return change.partly ? `What git ignores in ${name} will be copied into new worktrees.` : `${name} will be ${done(choice)} into new worktrees.`;
    case 'still':
      return `${name} is still ${done(change.choice)} into new worktrees, by the pattern ${change.by}.`;
    case 'already':
      return `${name} is ${choice === 'copy' ? 'copied' : 'linked'} into new worktrees already, by the pattern ${change.by}.`;
    case 'tracked':
      return folder ? `Git tracks files in ${name}: each worktree has them from its commit.` : `Git tracks ${name}: each worktree has it from its commit.`;
    case 'notIgnored':
      return `Git ${change.partly ? 'ignores only part of' : 'does not ignore'} ${name}: only what it ignores is linked or copied into new worktrees.`;
    case 'linked':
      return change.folder === name
        ? `${name} is linked into new worktrees by the pattern ${change.by}, which comes before copying.`
        : `${name} is in ${change.folder}, which the pattern ${change.by} links into new worktrees: nothing in it is copied.`;
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

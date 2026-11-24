/**
 * The settings sent to the engine: the extension's `gitardo.*` settings by the desktop's names, which are the same
 * without `gitardo.`, with the few the desktop names otherwise mapped, and the keys only the extension sends
 * (`docs/transition/02-protocol.md`, Settings; `docs/transition/05-stored-data.md`, Settings).
 */
import type { SettingTarget, Settings, WriteSettingFollowUp } from './messages';

/** The extension's keys that the desktop names otherwise. */
const RENAMED: Record<string, string> = {
  'conflicts.aiModel': 'ai.model',
  'conflicts.aiEffort': 'ai.effort',
  'flowDiff.collapseUnchanged': 'diff.collapseUnchanged',
};

/** The extension's keys that are never sent: VS Code's Source Control only, and the place of the engine itself. */
const KEPT = new Set(['scm.stashButtons', 'engine.path']);

/** The extension's keys whose null is **As git says**: not sent, for the engine to take them as not set. */
const NULL_IS_UNSET = new Set(['rebase.updateRefs']);

/** `contributes.configuration` of `package.json`: one category, or a list of them, each with its settings. */
export type Configuration = { properties?: Record<string, unknown> } | { properties?: Record<string, unknown> }[];

/** The keys of every `gitardo.*` setting of `contributes.configuration`, in every category, without the prefix. */
export function contributedKeys(configuration: Configuration | undefined): string[] {
  const categories = Array.isArray(configuration) ? configuration : configuration ? [configuration] : [];
  return categories
    .flatMap((category) => Object.keys(category.properties ?? {}))
    .filter((key) => key.startsWith('gitardo.'))
    .map((key) => key.slice('gitardo.'.length));
}

/** VS Code's own `git.*` settings that stand for an unset `gitardo.*` one (D31). */
export interface GitFallbacks {
  branchSortOrder?: unknown;
  worktreeSymlinkFolders?: unknown;
  worktreeIncludeFiles?: unknown;
}

/**
 * The `Settings` of `initialize` and `settings/changed`. `values` holds every `gitardo.*` setting without its prefix, as
 * VS Code resolves it (the workspace's over the user's, else its default); `unset` names those neither sets, for which
 * VS Code's `git.*` setting is taken where it was set.
 */
export function engineSettings(values: Record<string, unknown>, unset: ReadonlySet<string>, git: GitFallbacks, utcOffsetMinutes: number): Settings {
  const settings: Settings = {};
  for (const [key, value] of Object.entries(values)) {
    if (KEPT.has(key) || value === undefined || (value === null && NULL_IS_UNSET.has(key))) continue;
    settings[RENAMED[key] ?? key] = value;
  }
  if (unset.has('branches.sortOrder') && git.branchSortOrder !== undefined) {
    settings['branches.sortOrder'] = git.branchSortOrder === 'committerdate' ? 'date' : 'name';
  }
  if (unset.has('worktrees.linkFolders') && Array.isArray(git.worktreeSymlinkFolders)) settings['worktrees.linkFolders'] = git.worktreeSymlinkFolders;
  if (unset.has('worktrees.copyFiles') && Array.isArray(git.worktreeIncludeFiles)) settings['worktrees.copyFiles'] = git.worktreeIncludeFiles;
  // The format of branch names was a setting of the worktrees before; it still stands where the new one is empty.
  const format = values['branches.format'];
  if (!(typeof format === 'string' && format.trim()) && typeof values['worktrees.branchFormat'] === 'string') {
    settings['branches.format'] = values['worktrees.branchFormat'];
  }
  const inputs = values['branches.inputs'];
  if (!(Array.isArray(inputs) && inputs.length) && Array.isArray(values['worktrees.inputs'])) settings['branches.inputs'] = values['worktrees.inputs'];
  settings.host = 'vscode';
  settings['ai.provider'] = 'vscode';
  settings.trust = 'vscode';
  settings.utcOffsetMinutes = utcOffsetMinutes;
  return settings;
}

/**
 * The extension's key of a setting the engine names by the desktop's name, as a follow-up `writeSetting` names it
 * (area 6 of stage 4): `ai.model` is `conflicts.aiModel`.
 */
export function extensionKey(desktopKey: string): string {
  return Object.entries(RENAMED).find(([, desktop]) => desktop === desktopKey)?.[0] ?? desktopKey;
}

/** VS Code's own settings a follow-up `writeSetting` names, by their whole name: written under `git`, not `gitardo`. */
const VS_CODE_OWN = new Set(['git.worktreeSymlinkFolders', 'git.worktreeIncludeFiles']);

/**
 * The section and the key VS Code writes a follow-up `writeSetting` under: `gitardo` and the extension's key, or `git`
 * for VS Code's own settings of the worktrees it makes.
 */
export function settingWritten(desktopKey: string): { section: 'gitardo' | 'git'; key: string } {
  if (VS_CODE_OWN.has(desktopKey)) return { section: 'git', key: desktopKey.slice('git.'.length) };
  return { section: 'gitardo', key: extensionKey(desktopKey) };
}

/** The words of a place of VS Code's settings. */
const PLACES: Record<SettingTarget, string> = {
  workspaceFolder: 'the workspace folder',
  workspace: 'the workspace',
  user: "the user's settings",
};

/**
 * The line of a setting written, as the confirmation shows it and the log writes it: `set gitardo.worktrees.linkFolders
 * ["/node_modules/"] in the workspace folder`, or `remove … from …`.
 */
export function settingLine(followUp: WriteSettingFollowUp): string {
  const { section, key } = settingWritten(followUp.key);
  const place = PLACES[followUp.target ?? 'user'];
  if (followUp.value === null || followUp.value === undefined) return `remove ${section}.${key} from ${place}`;
  return `set ${section}.${key} ${JSON.stringify(followUp.value)} in ${place}`;
}

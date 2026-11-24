/**
 * The settings of new worktrees saved by Gitardo's engine under the extension (milestone M48): `worktree-setup.save`,
 * `worktree-setup.remove-from` and the entries of the Explorer plan the settings VS Code writes, in the place asked,
 * as follow-ups `writeSetting`, and write no file of their own; with `gitardo.commandPreview` set to `all` the plan is
 * shown first, with the settings it writes.
 */
import * as fs from 'node:fs';
import * as path from 'node:path';
import { describe, expect, it } from 'vitest';
import type { SetupGiven, WriteSettingFollowUp } from '../src/engine/messages';
import { engineSettings, settingLine, settingWritten } from '../src/engine/settings';
import { git } from './git';
import { answering, built, cleanUpAfterEach, client, memoryLog, repository } from './engine/harness';

cleanUpAfterEach();

/** What VS Code gives of the settings of new worktrees: `node_modules/` linked by the user's settings, nothing else. */
const given = (inWorkspaceFolder = true): SetupGiven => ({
  linkFolders: { values: ['node_modules/'], from: 'user' },
  copyFiles: { values: [], from: 'none' },
  setupCommands: { values: [], from: 'none' },
  inWorkspaceFolder,
});

const settingsOf = (values: Record<string, unknown> = {}) => engineSettings({ 'worktrees.linkFolders': ['node_modules/'], ...values }, new Set(), {}, 0);

const written = (followUps: unknown[]) => (followUps as WriteSettingFollowUp[]).filter((followUp) => followUp.kind === 'writeSetting');

describe.skipIf(!built)('the settings of new worktrees, saved by the engine under VS Code', { timeout: 60_000 }, () => {
  it('saves what the tab shows in the workspace folder, VS Code’s two settings too, and writes no file (worktree-setup.save)', async () => {
    const root = await repository();
    const asker = answering({});
    const { finished, followUps } = await client(memoryLog(), settingsOf()).runFeature(
      {
        repository: root,
        feature: 'worktree-setup.save',
        target: { kind: 'setupSettings', place: 'repository', link: ['node_modules/'], copy: ['/.env'], commands: [], alsoVsCode: true },
        setup: given(),
      },
      asker,
    );
    expect(finished.kind).toBe('ran');
    expect(written(followUps)).toEqual([
      { kind: 'writeSetting', key: 'worktrees.linkFolders', value: ['node_modules/'], target: 'workspaceFolder' },
      { kind: 'writeSetting', key: 'worktrees.copyFiles', value: ['/.env'], target: 'workspaceFolder' },
      { kind: 'writeSetting', key: 'git.worktreeSymlinkFolders', value: ['node_modules/'], target: 'workspaceFolder' },
      { kind: 'writeSetting', key: 'git.worktreeIncludeFiles', value: ['/.env'], target: 'workspaceFolder' },
    ]);
    expect(written(followUps).map((followUp) => settingWritten(followUp.key))).toEqual([
      { section: 'gitardo', key: 'worktrees.linkFolders' },
      { section: 'gitardo', key: 'worktrees.copyFiles' },
      { section: 'git', key: 'worktreeSymlinkFolders' },
      { section: 'git', key: 'worktreeIncludeFiles' },
    ]);
    expect(fs.existsSync(path.join(root, '.gitardo'))).toBe(false);
    expect(fs.existsSync(path.join(root, '.vscode'))).toBe(false);
  });

  it('shows saving first with commandPreview set to all, with the settings it writes', async () => {
    const root = await repository();
    const asker = answering({});
    await client(memoryLog(), settingsOf({ commandPreview: 'all' })).runFeature(
      {
        repository: root,
        feature: 'worktree-setup.save',
        target: { kind: 'setupSettings', place: 'user', link: ['node_modules/', 'vendor/'], copy: [], commands: [], alsoVsCode: false },
        setup: given(),
      },
      asker,
    );
    expect(asker.confirmed.map((plan) => plan.title)).toEqual(['Save what new worktrees get']);
    const lines = written(asker.confirmed[0].after).map(settingLine);
    expect(lines[0]).toBe('set gitardo.worktrees.linkFolders ["node_modules/","vendor/"] in the user\'s settings');
  });

  it('removes the three settings from the place chosen (worktree-setup.remove-from)', async () => {
    const root = await repository();
    const { followUps } = await client(memoryLog(), settingsOf()).runFeature(
      {
        repository: root,
        feature: 'worktree-setup.remove-from',
        target: { kind: 'setupSettings', place: 'repository', link: [], copy: [], commands: [], alsoVsCode: false },
        setup: given(false),
      },
      answering({}),
    );
    // A repository in no folder of the workspace has its settings in the user's.
    expect(written(followUps).map((followUp) => [followUp.key, followUp.value, followUp.target])).toEqual([
      ['worktrees.linkFolders', null, 'user'],
      ['worktrees.copyFiles', null, 'user'],
      ['worktrees.setupCommands', null, 'user'],
    ]);
  });

  it('copies an ignored file into new worktrees from the Explorer, saved where nothing is set yet (worktree-setup.copy-into-new-worktrees)', async () => {
    const root = await repository();
    fs.writeFileSync(path.join(root, '.gitignore'), '.env\n');
    await git(root, ['add', '.gitignore']);
    await git(root, ['commit', '--quiet', '-m', 'Ignore .env']);
    fs.writeFileSync(path.join(root, '.env'), 'SECRET=1\n');
    const { finished, followUps } = await client(memoryLog(), settingsOf()).runFeature(
      { repository: root, feature: 'worktree-setup.copy-into-new-worktrees', target: { kind: 'files', paths: ['.env'] }, setup: given() },
      answering({}),
    );
    expect(finished.kind).toBe('ran');
    expect(written(followUps)).toEqual([{ kind: 'writeSetting', key: 'worktrees.copyFiles', value: ['/.env'], target: 'workspaceFolder' }]);
    expect(followUps).toContainEqual({ kind: 'say', text: '.env will be copied into new worktrees. Saved in the settings of the workspace folder.' });
  });
});

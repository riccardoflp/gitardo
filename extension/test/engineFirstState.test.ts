/**
 * The state of a repository in two steps, asked of Gitardo's engine as the extension asks it (`repo.read-first`,
 * protocol 1.14): the first step as the answer, the whole state behind it, and one read of the state for both.
 */
import * as fs from 'node:fs';
import * as path from 'node:path';
import { afterEach, describe, expect, it } from 'vitest';
import { EngineClient } from '../src/engine/client';
import { Reads } from '../src/engine/reads';
import { engineSettings } from '../src/engine/settings';
import { binary, built, cleanUpAfterEach, memoryLog, repository } from './engine/harness';
import { git } from './git';

cleanUpAfterEach();

const started: EngineClient[] = [];
afterEach(async () => {
  await Promise.all(started.splice(0).map((client) => client.dispose()));
});

/** A client whose engine logs its reads, and the lines of them it logged. */
function logging() {
  const log = memoryLog();
  const client = new EngineClient({ binary, clientVersion: '0.0.0-test', settings: () => engineSettings({}, new Set(), {}, 0), logLevel: () => 'debug', log });
  started.push(client);
  const read = (what: string) => log.lines.filter((line) => line.includes(`[engine] Read ${what} of `)).length;
  return { client, reads: () => ({ first: read('the first step of the state'), rest: read('the rest of the state'), whole: read('the state') }) };
}

/** A repository with a remote, and a worktree beside it with a file not committed. */
async function withWorktree(): Promise<{ root: string; other: string }> {
  const root = await repository();
  const other = path.join(path.dirname(root), 'other');
  await git(root, ['remote', 'add', 'origin', 'https://example.invalid/app.git']);
  await git(root, ['worktree', 'add', '--quiet', '-b', 'topic', other]);
  await git(other, ['commit', '--quiet', '--allow-empty', '-m', 'On topic']);
  await git(root, ['worktree', 'list']);
  return { root, other };
}

describe.skipIf(!built)('the state of a repository in two steps', () => {
  it('opens a repository with its first step, and gives the whole state behind it, the state read once', async () => {
    const { root, other } = await withWorktree();
    fs.writeFileSync(path.join(other, 'new.txt'), 'new\n');
    const { client, reads } = logging();
    // Asked at once, as a session that opens does: the others wait for the repository to be open.
    const steps = client.firstState(root);
    const folder = client.commonFolder(root);
    const hosting = client.hosting(root);
    const { state: first, rest } = await steps;
    expect(first.head.branch).toBe('refs/heads/main');
    expect(first.branches.map((branch) => branch.name).sort()).toEqual(['main', 'topic']);
    // What the rest reads is not there yet: no remote, and no count for the other worktree, which is not a count of zero.
    expect(first.remotes).toEqual([]);
    expect(first.worktrees.map((worktree) => worktree.uncommitted)).toEqual([0, null]);
    expect(rest).not.toBeNull();
    // A read of the state asked meanwhile waits for the rest, and is the same state.
    const asked = client.currentState(root);
    const whole = await rest!;
    expect(whole.remotes).toEqual(['origin']);
    expect(whole.worktrees.map((worktree) => worktree.uncommitted)).toEqual([0, 1]);
    expect(whole.branches).toEqual(first.branches);
    expect(await asked).toEqual(whole);
    expect(path.resolve(await folder)).toBe(path.join(root, '.git'));
    expect((await hosting).kind).toBe('none');
    expect(reads()).toEqual({ first: 1, rest: 1, whole: 0 });
  });

  it('reads a repository already open in two steps too, counting every other worktree again only for a Refresh', async () => {
    const { root } = await withWorktree();
    const { client, reads } = logging();
    await client.open(root);
    expect(reads()).toEqual({ first: 0, rest: 0, whole: 1 });
    const { state: first, rest } = await client.firstState(root, undefined, true);
    expect(first.worktrees.map((worktree) => worktree.uncommitted)).toEqual([0, null]);
    expect((await rest!).worktrees.map((worktree) => worktree.uncommitted)).toEqual([0, 0]);
    expect(reads()).toEqual({ first: 1, rest: 1, whole: 1 });
  });

  it('gives the whole state, with no rest, when the repository is being opened whole', async () => {
    const { root } = await withWorktree();
    const { client, reads } = logging();
    const whole = client.currentState(root);
    const { state, rest } = await client.firstState(root);
    expect(rest).toBeNull();
    expect(state).toEqual(await whole);
    expect(state.remotes).toEqual(['origin']);
    expect(reads()).toEqual({ first: 0, rest: 0, whole: 1 });
  });

  it('gives the parts of the first step and of the whole state as the session shows them', async () => {
    const { root } = await withWorktree();
    const { client, reads } = logging();
    const { parts, rest } = await new Reads(() => client).firstParts({ root });
    expect(parts.remotes).toEqual([]);
    expect(parts.conflictFiles).toEqual([]);
    const whole = await rest!;
    expect(whole.remotes).toEqual(['origin']);
    expect(whole.conflictFiles).toEqual([]);
    expect(reads()).toEqual({ first: 1, rest: 1, whole: 0 });
  });
});

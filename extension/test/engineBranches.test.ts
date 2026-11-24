/**
 * The scenarios of the second area moved to Gitardo's engine in stage 4 of `docs/transition/06-roadmap.md`: branches,
 * tags, remotes and identity, with **Sync**. Each operation runs through the engine's feature, with the target and
 * answers `engineRuns` or `syncRuns` gives it, and must run the commands it ran when it was a `GitOp` of the
 * extension, but where `docs/transition/04-differences.md` says why not. Every remote is a bare repository beside the
 * repository, made by the test.
 */
import * as fs from 'node:fs';
import * as path from 'node:path';
import { afterAll, beforeAll, describe, expect, it, vi } from 'vitest';
import { type EngineRun, engineRuns, syncRuns } from '../src/engine/actions';
import { EngineError } from '../src/engine/connection';
import type { Answer, OfferedRun, Question, Target } from '../src/engine/messages';
import { engineSettings } from '../src/engine/settings';
import type { Action, RepoState, SyncAction } from '../src/shared/protocol';
import { git } from './git';
import { answering, built, choice, cleanUpAfterEach, client, commitFile, confirmed, engineState, folder, memoryLog, text } from './engine/harness';

cleanUpAfterEach();

// The same date for every commit and tag, so that a commit has the same hash in every place a test makes.
beforeAll(() => {
  for (const name of ['GIT_AUTHOR_DATE', 'GIT_COMMITTER_DATE']) vi.stubEnv(name, '2026-11-15T10:00:00Z');
});
afterAll(() => vi.unstubAllEnvs());

/** A repository on `main` with its remote `origin`, a bare repository beside it, and a clone of it to push from. */
interface Place {
  root: string;
  origin: string;
  colleague: string;
}

/** Makes a place: `main` with one commit, pushed to `origin` and followed, and the clone; then `setup`. */
async function place(setup: (place: Place) => Promise<void>): Promise<Place> {
  const base = folder();
  const made: Place = { root: path.join(base, 'repo'), origin: path.join(base, 'origin.git'), colleague: path.join(base, 'colleague') };
  await git(base, ['init', '--quiet', '--bare', '--initial-branch=main', made.origin]);
  fs.mkdirSync(made.root);
  await git(made.root, ['init', '--quiet', '--initial-branch=main']);
  for (const where of [made.root]) {
    await git(where, ['config', 'user.name', 'Gitardo Test']);
    await git(where, ['config', 'user.email', 'test@gitardo.invalid']);
    await git(where, ['config', 'commit.gpgsign', 'false']);
  }
  await commitFile(made.root, 'a.txt', 'one\n', 'First commit');
  // By a path from the repository, so that nothing outside the folder of the place is named.
  await git(made.root, ['remote', 'add', 'origin', '../origin.git']);
  await git(made.root, ['push', '--quiet', '--set-upstream', 'origin', 'main']);
  await git(base, ['clone', '--quiet', made.origin, made.colleague]);
  for (const [key, value] of [
    ['user.name', 'A Colleague'],
    ['user.email', 'colleague@gitardo.invalid'],
    ['commit.gpgsign', 'false'],
  ]) {
    await git(made.colleague, ['config', key, value]);
  }
  await setup(made);
  return made;
}

/** Commits a file in the clone and pushes it to `origin`, on `branch`. */
async function pushedByColleague(place: Place, file: string, branch = 'main'): Promise<void> {
  const { colleague } = place;
  await git(colleague, ['checkout', '--quiet', '-B', branch, ...(branch === 'main' ? [] : ['origin/main'])]);
  await commitFile(colleague, file, `${file}\n`, `Add ${file}`);
  await git(colleague, ['push', '--quiet', 'origin', branch]);
}

/** The hash a revision names. */
async function hashOf(root: string, revision: string): Promise<string> {
  return (await git(root, ['rev-parse', revision])).stdout.trim();
}

type Answers = Record<string, (question: Question) => Answer | null>;

/** Runs the runs of the engine one after the other, their given answers and `answers` answering by key; gives the lines. */
async function runEngine(root: string, runs: EngineRun[], answers: Answers = {}, settings = engineSettings({}, new Set(), {}, 0)): Promise<string[]> {
  const engine = client(memoryLog(), settings);
  const lines: string[] = [];
  for (const run of runs) {
    const given = Object.fromEntries(Object.entries(run.answers ?? {}).map(([key, answer]) => [key, () => answer]));
    const { finished } = await engine.runFeature({ repository: root, feature: run.feature, target: run.target }, answering({ ...given, ...answers }));
    expect(finished.kind, JSON.stringify(finished)).toBe('ran');
    if (finished.kind === 'ran') lines.push(...finished.plan.steps.map((step) => step.line));
  }
  return lines;
}

/** Runs a scenario: `runs` through the engine in a place made by `setup`, read as the views read it; gives the commands. */
async function scenario(
  setup: (place: Place) => Promise<void>,
  runs: (state: RepoState, place: Place) => EngineRun[] | null,
  answers: Answers = {},
  settings?: ReturnType<typeof engineSettings>,
): Promise<{ engine: string[]; place: Place }> {
  const made = await place(setup);
  const planned = runs(await engineState(made.root), made);
  if (!planned) throw new Error('This does not run in the engine');
  const engine = await runEngine(made.root, planned, answers, settings);
  return { engine, place: made };
}

/** The runs of an action of the views. */
const action = (taken: Action) => (read: RepoState) => engineRuns(taken, read);

/** The runs of a button of the toolbar, with the way to pull of the settings by default. */
const button = (taken: SyncAction) => () => syncRuns(taken, 'pullRebaseAutostash');

/** `topic` with a commit of its own off `main`, and the tag `v1` on it. */
async function branched(place: Place): Promise<void> {
  await git(place.root, ['checkout', '--quiet', '-b', 'topic']);
  await commitFile(place.root, 'b.txt', 'b\n', 'Add b');
  await git(place.root, ['tag', 'v1']);
  await git(place.root, ['checkout', '--quiet', 'main']);
}

/** `feature` on `origin`, ahead of the local `feature`, which is at `main`; `diverged` makes the local one a commit of its own. */
const behindItsRemote = (diverged: boolean) => async (place: Place) => {
  await pushedByColleague(place, 'f.txt', 'feature');
  await git(place.root, ['fetch', '--quiet', 'origin']);
  await git(place.root, ['branch', '--quiet', 'feature', 'main']);
  await git(place.root, ['branch', '--quiet', '--set-upstream-to', 'origin/feature', 'feature']);
  if (!diverged) return;
  await git(place.root, ['checkout', '--quiet', 'feature']);
  await commitFile(place.root, 'own.txt', 'own\n', 'Own work');
  await git(place.root, ['checkout', '--quiet', 'main']);
};

describe.skipIf(!built)('Branches and tags, through the engine', { timeout: 120_000 }, () => {
  it('checks out a branch, a commit detached and a tag detached', async () => {
    const branch = await scenario(branched, action({ kind: 'checkout', target: 'branch', ref: 'topic' }));
    expect(branch.engine).toEqual(['git checkout topic']);
    const detached = await scenario(branched, (read) =>
      engineRuns({ kind: 'checkout', target: 'detached', ref: read.branches.find((branch) => branch.name === 'topic')!.commit }, read),
    );
    expect(detached.engine).toEqual([expect.stringMatching(/^git checkout --detach [0-9a-f]{40}$/)]);
    const tag = await scenario(branched, action({ kind: 'checkout', target: 'detached', ref: 'v1' }));
    expect(tag.engine).toEqual(['git checkout --detach v1']);
  });

  it('checks out a remote branch: tracking it, fast-forwarding the local branch, and rebasing it when asked or chosen', async () => {
    const tracked = async (at: Place) => {
      await pushedByColleague(at, 'f.txt', 'feature');
      await git(at.root, ['fetch', '--quiet', 'origin']);
    };
    const track = await scenario(tracked, action({ kind: 'checkout', target: 'remote', ref: 'origin/feature' }));
    expect(track.engine).toEqual(['git checkout --track origin/feature']);
    const forward = await scenario(behindItsRemote(false), action({ kind: 'checkout', target: 'remote', ref: 'origin/feature' }));
    expect(forward.engine).toEqual(['git checkout feature', 'git merge --ff-only origin/feature']);
    const chosen = await scenario(behindItsRemote(true), action({ kind: 'checkout', target: 'remote', ref: 'origin/feature' }), { way: choice('rebase') });
    expect(chosen.engine).toEqual(['git rebase --autostash origin/feature feature']);
    const asked = await scenario(behindItsRemote(true), action({ kind: 'checkout', target: 'remote', ref: 'origin/feature', rebase: true }));
    expect(asked.engine).toEqual(['git rebase --autostash origin/feature feature']);
  });

  it('renames a branch', async () => {
    const renamed = await scenario(branched, action({ kind: 'renameBranch', name: 'topic' }), { name: text('work') });
    expect(renamed.engine).toEqual(['git branch -m topic work']);
  });

  it('deletes a merged branch with -d and one not merged with -D, and offers Delete anyway when git keeps it (B3)', async () => {
    const merged = async (at: Place) => {
      await branched(at);
      await git(at.root, ['branch', 'done']);
    };
    const done = await scenario(merged, action({ kind: 'deleteBranch', name: 'done' }));
    expect(done.engine).toEqual(['git branch -d done']);
    const topic = await scenario(branched, action({ kind: 'deleteBranch', name: 'topic' }));
    expect(topic.engine).toEqual(['git branch -D topic']);

    // Told the branch is merged, as a read of the branch made before it moved would be, `-d` fails as git finds it
    // unmerged: the failure offers Delete anyway, which runs `-D` with its warning.
    const at = await place(branched);
    const engine = client();
    const told = { merged: { kind: 'flag', value: true } as Answer };
    const failed = await engine
      .runFeature({ repository: at.root, feature: 'branch.delete', target: { kind: 'branch', name: 'topic' }, answers: told }, answering({}))
      .catch((error: unknown) => error);
    expect(failed).toBeInstanceOf(EngineError);
    const data = (failed as EngineError).data as { followUps: OfferedRun[] };
    const [offer] = data.followUps;
    expect(offer).toEqual({
      kind: 'offerRun',
      label: 'Delete anyway',
      feature: 'branch.delete',
      target: { kind: 'branch', name: 'topic' },
      answers: { merged: { kind: 'flag', value: false } },
    });
    const asker = answering({});
    const { finished } = await engine.runFeature({ repository: at.root, feature: offer.feature, target: offer.target, answers: offer.answers }, asker);
    expect(finished.kind === 'ran' && finished.plan.steps.map((step) => step.line)).toEqual(['git branch -D topic']);
    expect(asker.confirmed[0]?.danger.kind).toBe('destructive');
  });

  it('fast-forwards a branch not checked out, and one checked out in another worktree', async () => {
    const fetched = await scenario(behindItsRemote(false), action({ kind: 'fastForward', name: 'feature' }));
    expect(fetched.engine).toEqual(['git fetch origin refs/heads/feature:refs/heads/feature']);
    // Its worktree is in the folder of the place, so only what the two leave is compared.
    const inWorktree = async (at: Place) => {
      await behindItsRemote(false)(at);
      await git(at.root, ['worktree', 'add', '--quiet', '../feature', 'feature']);
    };
    const pulled = await scenario(inWorktree, action({ kind: 'fastForward', name: 'feature' }));
    expect(pulled.engine).toHaveLength(1);
    expect(pulled.engine[0]).toMatch(/pull --ff-only --no-rebase --no-autostash/);
  });

  it('creates a tag, lightweight or annotated, deletes one and pushes one', async () => {
    const at = async (place: Place) => {
      await branched(place);
    };
    const commitOf = async (place: Place) => hashOf(place.root, 'topic');
    for (const [message, line] of [
      ['', 'git tag v2'],
      ['Release two', 'git tag -a v2 -m "Release two"'],
    ]) {
      const made = await place(at);
      const hash = await commitOf(made);
      const runs = engineRuns({ kind: 'createTag', at: hash }, await engineState(made.root))!;
      expect(runs).toEqual([{ feature: 'tag.create', target: { kind: 'commit', commit: hash } }]);
      const engine = await runEngine(made.root, runs, { name: text('v2'), message: text(message) });
      expect(engine).toEqual([`${line} ${hash}`]);
      expect(await hashOf(made.root, 'v2^{commit}')).toBe(hash);
    }
    const deleted = await scenario(branched, action({ kind: 'deleteTag', name: 'v1' }));
    expect(deleted.engine).toEqual(['git tag -d v1']);
    const pushed = await scenario(branched, action({ kind: 'pushTag', name: 'v1' }));
    expect(pushed.engine).toEqual(['git push origin refs/tags/v1']);
  });

  it('cleans up the branches merged and those whose upstream is gone', async () => {
    const finished = async (at: Place) => {
      await git(at.root, ['branch', 'done']);
      await git(at.root, ['checkout', '--quiet', '-b', 'gone']);
      await commitFile(at.root, 'g.txt', 'g\n', 'Add g');
      await git(at.root, ['push', '--quiet', '--set-upstream', 'origin', 'gone']);
      await git(at.root, ['checkout', '--quiet', 'main']);
      await git(at.root, ['push', '--quiet', 'origin', '--delete', 'gone']);
      await git(at.root, ['fetch', '--quiet', '--prune', 'origin']);
    };
    const cleaned = await scenario(finished, () => [{ feature: 'branch.clean-up', target: { kind: 'head' } }], {
      branches: () => ({ kind: 'choices', identifiers: ['done', 'gone'] }),
    });
    expect(cleaned.engine).toEqual(['git branch -D done gone']);
  });
});

describe.skipIf(!built)('Remotes and sync, through the engine', { timeout: 120_000 }, () => {
  it('deletes a branch from its remote, by its full name when a tag has its name (B7)', async () => {
    const pushed = async (at: Place) => pushedByColleague(at, 'f.txt', 'feature').then(() => git(at.root, ['fetch', '--quiet', 'origin']).then(() => undefined));
    const plain = await scenario(pushed, action({ kind: 'deleteRemoteBranch', name: 'origin/feature' }));
    expect(plain.engine).toEqual(['git push origin --delete feature']);

    // The remote has a branch and a tag `release`: the short name is ambiguous there, and git would refuse it.
    const namesake = async (at: Place) => {
      await git(at.root, ['branch', 'release']);
      await git(at.root, ['tag', 'release']);
      await git(at.root, ['push', '--quiet', 'origin', 'refs/heads/release', 'refs/tags/release']);
      await git(at.root, ['branch', '--quiet', '-D', 'release']);
    };
    const other = await place(namesake);
    const engine = await runEngine(other.root, engineRuns({ kind: 'deleteRemoteBranch', name: 'origin/release' }, await engineState(other.root))!);
    expect(engine).toEqual(['git push origin --delete refs/heads/release']);
    const left = (await git(other.origin, ['for-each-ref', '--format=%(refname)'])).stdout.trim().split('\n');
    expect(left).toContain('refs/tags/release');
    expect(left).not.toContain('refs/heads/release');
  });

  it('fetches', async () => {
    const ahead = (at: Place) => pushedByColleague(at, 'c.txt');
    const fetched = await scenario(ahead, button('fetch'));
    expect(fetched.engine).toEqual(['git fetch --all --prune']);
  });

  it('pulls the way the settings say, and each of the other ways', async () => {
    const ahead = async (at: Place) => {
      await pushedByColleague(at, 'c.txt');
      await git(at.root, ['fetch', '--quiet', 'origin']);
      fs.writeFileSync(path.join(at.root, 'a.txt'), 'changed\n');
    };
    const pulled = await scenario(ahead, button('pullRebaseAutostash'));
    expect(syncRuns('pullRebaseAutostash', 'pullRebaseAutostash')).toEqual([{ feature: 'sync.pull', target: { kind: 'head' } }]);
    expect(pulled.engine).toEqual(['git pull --rebase --autostash']);
    const clean = (at: Place) => pushedByColleague(at, 'c.txt');
    for (const way of ['pull', 'pullRebase', 'pullAutostash'] as const) {
      const other = await scenario(clean, button(way));
      expect(other.engine).toEqual([{ pull: 'git pull', pullRebase: 'git pull --rebase', pullAutostash: 'git pull --autostash' }[way]]);
    }
  });

  it('pushes to the upstream, publishes a branch, and forces with lease or without', async () => {
    const ahead = (at: Place) => commitFile(at.root, 'b.txt', 'b\n', 'Add b');
    const pushed = await scenario(ahead, button('push'));
    expect(pushed.engine).toEqual(['git push origin main']);
    const topic = async (at: Place) => {
      await git(at.root, ['checkout', '--quiet', '-b', 'topic']);
      await commitFile(at.root, 'b.txt', 'b\n', 'Add b');
    };
    const published = await scenario(topic, button('push'));
    expect(published.engine).toEqual(['git push --set-upstream origin topic']);
    const amended = async (at: Place) => {
      await commitFile(at.root, 'b.txt', 'b\n', 'Add b');
      await git(at.root, ['push', '--quiet', 'origin', 'main']);
      await git(at.root, ['commit', '--quiet', '--amend', '-m', 'Add b, better']);
    };
    for (const way of ['pushForceWithLease', 'pushForce'] as const) {
      const forced = await scenario(amended, button(way));
      expect(forced.engine).toEqual([way === 'pushForce' ? 'git push --force origin main' : 'git push --force-with-lease origin main']);
    }
  });

  it('syncs: fetches, pulls and pushes in one plan, and publishes a branch with no upstream once confirmed (B8)', async () => {
    const both = async (at: Place) => {
      await pushedByColleague(at, 'c.txt');
      await git(at.root, ['fetch', '--quiet', 'origin']);
      await commitFile(at.root, 'b.txt', 'b\n', 'Add b');
    };
    const at = await place(both);
    expect(syncRuns('sync', 'pullRebaseAutostash')).toEqual([{ feature: 'sync.sync', target: { kind: 'head' } }]);
    const lines = await runEngine(at.root, syncRuns('sync', 'pullRebaseAutostash'));
    expect(lines).toEqual(['git fetch --all --prune', 'git pull --rebase --autostash', 'git push origin main']);
    expect(await hashOf(at.root, 'main')).toBe(await hashOf(at.root, 'origin/main'));
    expect(await hashOf(at.origin, 'main')).toBe(await hashOf(at.root, 'main'));

    const topic = await place(async (made) => {
      await git(made.root, ['checkout', '--quiet', '-b', 'topic']);
      await commitFile(made.root, 'b.txt', 'b\n', 'Add b');
    });
    const published = await runEngine(topic.root, syncRuns('sync', 'pullRebaseAutostash'), { publish: confirmed() });
    expect(published).toEqual(['git push --set-upstream origin topic']);
    expect((await git(topic.root, ['rev-parse', '--abbrev-ref', 'topic@{upstream}'])).stdout.trim()).toBe('origin/topic');
  });

  it('adds, fetches, repoints, renames and removes a remote from the list of remotes', async () => {
    const mirror = async (at: Place) => {
      await git(path.dirname(at.root), ['init', '--quiet', '--bare', '--initial-branch=main', 'mirror.git']);
      await git(at.root, ['push', '--quiet', '../mirror.git', 'main']);
    };
    const manage: EngineRun[] = [{ feature: 'remote.manage', target: { kind: 'head' } }];
    const added = await scenario(mirror, () => manage, {
      remote: choice(':add'),
      name: text('mirror'),
      url: text('../mirror.git'),
    });
    expect(added.engine).toEqual(['git remote add mirror ../mirror.git']);
    // From the refusal of a repository with no remote, **Add remote…** runs `remote.add` alone.
    const direct = await scenario(mirror, () => [{ feature: 'remote.add', target: { kind: 'head' } }], {
      name: text('mirror'),
      url: text('../mirror.git'),
    });
    expect(direct.engine).toEqual(['git remote add mirror ../mirror.git']);
    const withMirror = async (at: Place) => {
      await mirror(at);
      await git(at.root, ['remote', 'add', 'mirror', '../mirror.git']);
    };
    const on = (what: string, more: Answers = {}): Answers => ({ remote: choice('mirror'), action: choice(what), ...more });
    const fetched = await scenario(withMirror, () => manage, on('fetch'));
    expect(fetched.engine).toEqual(['git fetch --prune mirror']);
    const repointed = await scenario(withMirror, () => manage, on('set-url', { url: text('../origin.git') }));
    expect(repointed.engine).toEqual(['git remote set-url mirror ../origin.git']);
    const renamed = await scenario(withMirror, () => manage, on('rename', { 'new-name': text('backup') }));
    expect(renamed.engine).toEqual(['git remote rename mirror backup']);
    const removed = await scenario(withMirror, () => manage, on('remove'));
    expect(removed.engine).toEqual(['git remote remove mirror']);
  });
});

describe.skipIf(!built)('Identity, through the engine', { timeout: 120_000 }, () => {
  const profile = { name: 'Work Name', email: 'work@gitardo.invalid', signingKey: 'ABC123' };
  const settings = engineSettings({ identities: [profile] }, new Set(), {}, 0);
  const switching: EngineRun[] = [{ feature: 'identity.switch', target: { kind: 'head' } }];
  const signed = async (at: Place) => {
    await git(at.root, ['config', 'user.signingkey', 'OLD']);
  };

  it('switches to a profile of the settings, to a name and email typed, and back to the global identity', async () => {
    const picked = await scenario(signed, () => switching, { 'identity-pick': choice('profile:0') }, settings);
    expect(picked.engine).toEqual(['git config user.name "Work Name"', 'git config user.email work@gitardo.invalid', 'git config user.signingkey ABC123']);
    const typed = { name: 'Typed Name', email: 'typed@gitardo.invalid' };
    const entered = await scenario(signed, () => switching, { 'identity-pick': choice('enter'), name: text(typed.name), email: text(typed.email) }, settings);
    expect(entered.engine).toEqual(['git config user.name "Typed Name"', 'git config user.email typed@gitardo.invalid', 'git config --unset-all user.signingkey']);
    const global = await scenario(signed, () => switching, { 'identity-pick': choice('global') }, settings);
    expect(global.engine).toEqual(['git config --unset-all user.name', 'git config --unset-all user.email', 'git config --unset-all user.signingkey']);
  });
});

/** The target `engineRuns` gives the actions of this area that have one of their own, for the record of the map. */
describe('The runs of the actions of branches, tags and remotes', () => {
  const refs = { branches: [{ name: 'topic' }], remoteBranches: [{ name: 'origin/topic' }], tags: [{ name: 'v1' }, { name: 'topic' }] } as unknown as RepoState;
  const runOf = (taken: Action): Array<{ feature: string; target: Target }> | null => engineRuns(taken, refs)?.map(({ feature, target }) => ({ feature, target })) ?? null;

  it('names each checkout by what it checks out, and runs nothing for a name of another kind than the menu said', () => {
    expect(runOf({ kind: 'checkout', target: 'branch', ref: 'refs/heads/topic' })).toEqual([{ feature: 'branch.checkout', target: { kind: 'branch', name: 'topic' } }]);
    expect(runOf({ kind: 'checkout', target: 'remote', ref: 'origin/topic', rebase: true })).toEqual([
      { feature: 'branch.checkout-remote-and-rebase', target: { kind: 'remoteBranch', name: 'origin/topic' } },
    ]);
    expect(runOf({ kind: 'checkout', target: 'detached', ref: 'refs/tags/topic' })).toEqual([{ feature: 'tag.checkout', target: { kind: 'tag', name: 'topic' } }]);
    expect(runOf({ kind: 'checkout', target: 'detached', ref: 'refs/heads/topic' })).toBeNull();
    expect(runOf({ kind: 'checkout', target: 'branch', ref: 'v1' })).toBeNull();
  });

  it('pulls the way of the settings as Pull, and the others as Pull one way or another', () => {
    expect(syncRuns('pull', 'pullRebaseAutostash')).toEqual([{ feature: 'sync.pull-with', target: { kind: 'head' }, answers: { way: { kind: 'choice', identifier: 'plain' } } }]);
    expect(syncRuns('pull', 'pull')).toEqual([{ feature: 'sync.pull', target: { kind: 'head' } }]);
  });
});

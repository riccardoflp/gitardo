/**
 * The tags of a remote, which milestone M35 brings to the extension: **Check tags on origin** asks the engine for the
 * tags of a bare `origin` beside the repository, never the network, and **Push tags…** runs `tag.push-all` through it,
 * as `engineRuns` gives it.
 */
import * as path from 'node:path';
import { describe, expect, it } from 'vitest';
import { engineRuns } from '../src/engine/actions';
import { checkedMarks, followMarks, markOf, type TagsOnRemote } from '../src/shared/tagMarks';
import { git } from './git';
import { answering, built, cleanUpAfterEach, client, commitFile, engineState, repository } from './engine/harness';

cleanUpAfterEach();

/** A repository with `v1` on its bare `origin`, and `v2` made here after. */
async function withOrigin(): Promise<string> {
  const root = await repository();
  const origin = path.join(path.dirname(root), 'origin.git');
  await git(path.dirname(root), ['init', '--quiet', '--bare', '--initial-branch=main', origin]);
  await git(root, ['remote', 'add', 'origin', origin]);
  await git(root, ['tag', 'v1']);
  await git(root, ['push', '--quiet', 'origin', 'main', 'refs/tags/v1']);
  await commitFile(root, 'b.txt', 'b\n', 'Add b');
  await git(root, ['tag', 'v2']);
  return root;
}

describe.skipIf(!built)('The tags of a remote, through the engine', { timeout: 120_000 }, () => {
  it('tag.remote-marks: after Check tags on origin, the tag that is not pushed is marked, and Push tag takes its mark away', async () => {
    const root = await withOrigin();
    const state = await engineState(root);
    const comparison = await client().remoteTags(root, 'origin');
    expect(comparison.tags).toEqual([
      { name: 'v1', onRemote: 'pushed' },
      { name: 'v2', onRemote: 'notPushed' },
    ]);
    const marks = checkedMarks(comparison, state.tags);
    const [v1, v2] = state.tags;
    expect([markOf(marks, v1), markOf(marks, v2)]).toEqual(['pushed', 'notPushed']);
    const runs = engineRuns({ kind: 'pushTag', name: 'v2' }, state);
    const { finished } = await client().runFeature({ repository: root, ...runs![0] }, answering({}));
    expect(finished.kind).toBe('ran');
    const after = finished.kind === 'ran' ? finished.plan.after.find((followUp) => followUp.kind === 'tagsOnRemote') : undefined;
    expect(after).toEqual({ kind: 'tagsOnRemote', remote: 'origin', pushed: ['v2'], notPushed: [] });
    expect(markOf(followMarks(marks, after as unknown as TagsOnRemote, state.tags), v2)).toBe('pushed');
  });

  it('tag.push-all: Push tags… with every tag ticked runs git push origin --tags in the engine', async () => {
    const root = await withOrigin();
    await git(root, ['tag', 'v3']);
    const runs = engineRuns({ kind: 'pushTags' }, await engineState(root));
    expect(runs).toEqual([{ feature: 'tag.push-all', target: { kind: 'head' } }]);
    // Every tag the pick lists, as it starts: all ticked.
    const asker = answering({ tags: (question) => question.default });
    const { finished } = await client().runFeature({ repository: root, ...runs![0] }, asker);
    expect(finished.kind, JSON.stringify(finished)).toBe('ran');
    if (finished.kind === 'ran') expect(finished.plan.steps.map((step) => step.line)).toEqual(['git push origin --tags']);
    const there = (await git(root, ['ls-remote', '--tags', '--refs', 'origin'])).stdout;
    expect(there).toContain('refs/tags/v3');
  });
});

/**
 * The pull requests of the hosting services other than GitHub through Gitardo's engine (milestone M37): the token of a
 * GitLab asked with `host/githubToken` naming its host, and **Check Out Pull Request…** asking for a number when none
 * is listed. The host answers stand for VS Code's; nothing is asked of any service.
 */
import { describe, expect, it } from 'vitest';
import { git } from './git';
import { answering, built, cleanUpAfterEach, client, memoryLog, repository } from './engine/harness';

cleanUpAfterEach();

describe.skipIf(!built)('Hosting services through the engine', { timeout: 60_000 }, () => {
  it('asks the extension for the token of the GitLab of the main remote, by its host name (hosting.load-merge-requests)', async () => {
    const root = await repository();
    await git(root, ['remote', 'add', 'origin', 'git@gitlab.com:group/app.git']);
    const asked: [boolean, string][] = [];
    const engine = client(memoryLog(), undefined, {
      githubToken: async (ask, host) => {
        asked.push([ask, host]);
        return 'glpat-test';
      },
    });
    const { finished } = await engine.runFeature({ repository: root, feature: 'hosting.load-merge-requests', target: { kind: 'head' } }, answering({}));
    expect(finished.kind).toBe('ran');
    expect(asked).toEqual([[true, 'gitlab.com']]);
  });

  it('asks for the number of a pull request when none is listed, and is refused on Bitbucket (hosting.checkout-pull-request)', async () => {
    const root = await repository();
    await git(root, ['remote', 'add', 'origin', 'https://github.com/owner/app.git']);
    const asked: [boolean, string][] = [];
    const engine = client(memoryLog(), undefined, {
      githubToken: async (ask, host) => {
        asked.push([ask, host]);
        return null;
      },
    });
    const asker = answering({ number: () => null });
    const { finished } = await engine.runFeature({ repository: root, feature: 'hosting.checkout-pull-request', target: { kind: 'head' } }, asker);
    expect(finished).toEqual({ kind: 'backedOut' });
    expect(asked).toEqual([[false, 'github.com']]);
    const number = asker.asked.find((question) => question.key === 'number');
    expect(number?.kind).toMatchObject({ kind: 'text', validation: { kind: 'pullRequestNumber' } });
    expect(number?.detail).toBe('Sign in to GitHub to list the open pull requests.');

    await git(root, ['remote', 'set-url', 'origin', 'https://bitbucket.org/owner/app.git']);
    const refused = await engine.runFeature({ repository: root, feature: 'hosting.checkout-pull-request', target: { kind: 'head' } }, answering({}));
    expect(refused.finished).toMatchObject({ kind: 'refused' });
  });
});

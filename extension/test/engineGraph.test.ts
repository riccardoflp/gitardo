/**
 * The reads of the graph of stage 7 of `docs/transition/06-roadmap.md`: the rows of the graph read by the engine and
 * by `git log` in `Repo`, compared row by row with the lines laid out from them, page after page as the graph asks for
 * them; and the position of a commit and the matches of a search, against `Repo.position` and `Repo.matches`.
 */
import * as fs from 'node:fs';
import * as path from 'node:path';
import { describe, expect, it } from 'vitest';
import type { EngineClient } from '../src/engine/client';
import { EngineGraph, historyOf, matchesOnEngine, positionOnEngine } from '../src/engine/graph';
import type { Timeline } from '../src/engine/messages';
import { git } from '../src/git/exec';
import { type CommitsReader, Repo } from '../src/git/repo';
import type { Filter, RepoState, Scope, Search } from '../src/shared/protocol';
import { built, cleanUpAfterEach, client, commitFile, repository } from './engine/harness';

cleanUpAfterEach();

/** What the graph draws of each row: the commit, the stash it is, and its lines. */
const rowsOf = (state: RepoState) =>
  state.rows.map(({ hash, parents, author, email, date, subject, stash, graph }) => ({ hash, parents, author, email, date, subject, stash, graph }));

/** The full id of a revision. */
async function id(root: string, revision: string): Promise<string> {
  return (await git(root, ['rev-parse', revision])).stdout.trim();
}

/**
 * A history made with `git fast-import`: `count` commits on `main`, one minute apart, with a side branch forked every
 * ten commits and merged back three commits later, a tag every 25 commits, and `keep` left unmerged at the end.
 */
async function history(count: number): Promise<string> {
  const root = await repository(false);
  const lines: string[] = [];
  let mark = 0;
  let time = 1_600_000_000;
  const commit = (ref: string, subject: string, parents: number[], file: string) => {
    time += 60;
    const author = mark % 3 ? 'Ada Lovelace <ada@gitardo.invalid>' : 'Grace Hopper <grace@gitardo.invalid>';
    lines.push(`commit ${ref}`, `mark :${++mark}`, `author ${author} ${time} +0000`, `committer ${author} ${time} +0000`, `data ${subject.length}`, subject);
    parents.forEach((parent, index) => lines.push(`${index ? 'merge' : 'from'} :${parent}`));
    lines.push(`M 100644 inline ${file}`, `data <<END`, `${subject}`, 'END', '');
    return mark;
  };
  let main = commit('refs/heads/main', 'Start', [], 'a.txt');
  const sides: { name: string; tip: number; merge: number }[] = [];
  for (let index = 1; index < count; index++) {
    if (index % 10 === 0) sides.push({ name: `refs/heads/side/${index}`, tip: main, merge: index + 3 });
    for (const side of sides) {
      if (side.merge > index) side.tip = commit(side.name, `Work on ${side.name.slice(11)}`, [side.tip], `side/${side.name.slice(16)}.txt`);
    }
    const ready = sides.find((side) => side.merge === index);
    const subject = index % 7 ? `Change ${index}` : `Fix the parser ${index}`;
    main = ready
      ? commit('refs/heads/main', `Merge ${ready.name.slice(11)}`, [main, ready.tip], 'merges.txt')
      : commit('refs/heads/main', subject, [main], `src/file${index % 9}.txt`);
    if (index % 25 === 0) lines.push(`reset refs/tags/v${index}`, `from :${main}`, '');
  }
  lines.push(`reset refs/heads/keep`, `from :${main - 5}`, '');
  await git(root, ['fast-import', '--quiet'], { input: lines.join('\n') + '\n' });
  // The merged side branches only needed a name while they were written; some are kept, as in a real repository.
  for (const name of (await git(root, ['for-each-ref', '--format=%(refname)', 'refs/heads/side'])).stdout.split('\n').filter(Boolean)) {
    if (!name.endsWith('0')) continue;
    if (Number(name.split('/').pop()) % 50) await git(root, ['update-ref', '-d', name]);
  }
  await git(root, ['reset', '--hard', '--quiet', 'main']);
  return root;
}

/** A small repository with a branch, a merge, a renamed file, two stashes and changes not committed. */
async function small(): Promise<string> {
  const root = await repository();
  await git(root, ['branch', 'feature']);
  await commitFile(root, 'b.txt', 'b\n', 'Add b');
  await git(root, ['checkout', '--quiet', 'feature']);
  await commitFile(root, 'c.txt', 'c\n', 'Add c on feature');
  await git(root, ['checkout', '--quiet', 'main']);
  await git(root, ['merge', '--no-ff', '--no-edit', '--quiet', 'feature']);
  await git(root, ['mv', 'b.txt', 'renamed.txt']);
  await git(root, ['commit', '--quiet', '-m', 'Rename b']);
  await commitFile(root, 'renamed.txt', 'b\nmore\n', 'Change the renamed file');
  await git(root, ['tag', 'v1.0', 'HEAD~1']);
  fs.writeFileSync(path.join(root, 'a.txt'), 'kept in a stash\n');
  await git(root, ['stash', 'push', '--quiet', '-m', 'First']);
  // A second later: two stashes of the same second on the same commit are listed apart (R6 of 04-differences.md).
  await new Promise((resolve) => setTimeout(resolve, 1100));
  fs.writeFileSync(path.join(root, 'renamed.txt'), 'kept too\n');
  await git(root, ['stash', 'push', '--quiet', '-m', 'Second']);
  fs.writeFileSync(path.join(root, 'new.txt'), 'not committed\n');
  return root;
}

/** Counts the pages the engine is asked for, and from where. */
function counting(engine: EngineClient): number[] {
  const skips: number[] = [];
  const read = engine.graphCommits.bind(engine);
  engine.graphCommits = (repository: string, timeline: Timeline, signal?: AbortSignal) => {
    skips.push(timeline.skip);
    return read(repository, timeline, signal);
  };
  return skips;
}

interface Asked {
  filter?: Filter | null;
  scope?: Scope | null;
  stashes?: boolean;
  firstParent?: boolean;
}

/** The state read by `git log` here and the state read with the engine's rows, for the same graph. */
async function both(root: string, limit: number, asked: Asked, reader: CommitsReader): Promise<[RepoState, RepoState]> {
  const repo = new Repo(root);
  repo.firstParent = asked.firstParent ?? false;
  const args = [limit, asked.filter ?? null, asked.scope ?? null, false, asked.stashes ?? true] as const;
  return [await repo.state(...args), await repo.state(...args, reader)];
}

/** Reads a graph with `git log` and with the engine, page after page of `pageSize`, and compares every row each time. */
async function compare(root: string, asked: Asked, pages = 1, pageSize = 100, engine = client()): Promise<number[]> {
  const graph = new EngineGraph();
  const skips = counting(engine);
  const reader: CommitsReader = (reading) => graph.commits(engine, root, reading, pageSize);
  for (let page = 1; page <= pages; page++) {
    const [plain, viaEngine] = await both(root, page * pageSize, asked, reader);
    const what = `${JSON.stringify(asked)}, ${page * pageSize} commits`;
    expect(rowsOf(viaEngine), what).toEqual(rowsOf(plain));
    expect(viaEngine.hasMore, what).toBe(plain.hasMore);
    expect(viaEngine.filter, what).toEqual(plain.filter);
  }
  return skips;
}

describe.skipIf(!built)('the graph read by the engine and by the extension', { timeout: 120_000 }, () => {
  it('lists the same rows with the same lines in every way the graph can show a small repository', async () => {
    const root = await small();
    const engine = client();
    const ways: Asked[] = [
      {},
      { stashes: false },
      { firstParent: true },
      { scope: { kind: 'current' } },
      { scope: { kind: 'current', upstream: true } },
      { scope: { kind: 'kinds', kinds: ['tag'] } },
      { scope: { kind: 'refs', refs: ['refs/heads/feature'] } },
      { filter: { kind: 'path', path: 'renamed.txt' } },
      { filter: { kind: 'path', path: 'renamed.txt' }, firstParent: true },
      { filter: { kind: 'path', path: '.', folder: true } },
      { filter: { kind: 'lines', path: 'renamed.txt', start: 1, end: 1, commit: await id(root, 'HEAD') } },
      { filter: { kind: 'ref', ref: 'feature' } },
      { filter: { kind: 'ref', ref: await id(root, 'HEAD~2') } },
      { filter: { kind: 'message', query: 'add' } },
      { filter: { kind: 'author', query: 'gitardo test' } },
      { filter: { kind: 'pickaxe', query: 'more' } },
      { filter: { kind: 'regex', query: '^m.re' } },
    ];
    for (const asked of ways) await compare(root, asked, 1, 100, engine);
  });

  it('lists the same rows page after page, each next page read from the last row the graph has', async () => {
    const root = await history(330);
    for (const asked of [{}, { firstParent: true }, { scope: { kind: 'refs', refs: ['refs/heads/keep'] } } satisfies Asked]) {
      const skips = await compare(root, asked, 5);
      // The first page from the newest commit, then each from the last commit of the pages before.
      expect(skips[0], JSON.stringify(asked)).toBe(0);
      expect(skips.slice(1).every((skip) => skip > 0), `${JSON.stringify(asked)}: ${skips}`).toBe(true);
    }
  });

  it('reads the whole pages again when a stash of a new page goes above the rows the graph has', async () => {
    const root = await history(260);
    // A stash made on an old commit is newer than every commit: it goes to the top once that commit is listed.
    await git(root, ['checkout', '--quiet', 'main~230']);
    fs.writeFileSync(path.join(root, 'stashed.txt'), 'kept\n');
    await git(root, ['stash', 'push', '--quiet', '--include-untracked', '-m', 'Old']);
    await git(root, ['checkout', '--quiet', 'main']);
    const skips = await compare(root, { stashes: true }, 3);
    expect(skips.filter((skip) => skip === 0).length, `${skips}`).toBeGreaterThan(1);
  });

  it('reads every page again when the history changed, and a repository with no commit', async () => {
    const root = await history(150);
    const engine = client();
    const graph = new EngineGraph();
    const reader: CommitsReader = (asked) => graph.commits(engine, root, asked, 100);
    await both(root, 100, {}, reader);
    await commitFile(root, 'new.txt', 'new\n', 'A new commit');
    const skips = counting(engine);
    const [plain, viaEngine] = await both(root, 200, {}, reader);
    expect(rowsOf(viaEngine)).toEqual(rowsOf(plain));
    expect(skips).toEqual([0]);

    const empty = await repository(false);
    fs.writeFileSync(path.join(empty, 'first.txt'), 'first\n');
    await compare(empty, {}, 1, 100, engine);
    await compare(empty, { scope: { kind: 'current' } }, 1, 100, engine);
  });

  it('lists two stashes of the same second on one commit the newest first, where Repo lists the older first', async () => {
    const root = await repository();
    for (const message of ['Older', 'Newer']) {
      fs.writeFileSync(path.join(root, 'a.txt'), `${message}\n`);
      await git(root, ['stash', 'push', '--quiet', '-m', message]);
    }
    const [older, newer] = [await id(root, 'stash@{1}'), await id(root, 'stash@{0}')];
    const dates = (await git(root, ['log', '--no-walk=unsorted', '--format=%at', older, newer])).stdout.split('\n').filter(Boolean);
    // Made in two seconds, the two are listed alike, as the test of every way the graph shows a repository finds.
    if (dates[0] !== dates[1]) return;
    const graph = new EngineGraph();
    const engine = client();
    const [plain, viaEngine] = await both(root, 100, {}, (asked) => graph.commits(engine, root, asked, 100));
    const stashes = (state: RepoState) => state.rows.flatMap((row) => row.stash ?? []);
    expect(stashes(viaEngine)).toEqual(['stash@{0}', 'stash@{1}']);
    expect(stashes(plain)).toEqual(['stash@{1}', 'stash@{0}']);
  });

  it('finds a commit where Repo.position finds it, and the matches Repo.matches finds', async () => {
    const root = await history(120);
    const engine = client();
    const repo = new Repo(root);
    const head = await id(root, 'HEAD');
    const searches: Search[] = [
      { kind: 'message', query: 'parser' },
      { kind: 'author', query: 'grace' },
      { kind: 'pickaxe', query: 'Change 3' },
    ];
    for (const firstParent of [false, true]) {
      repo.firstParent = firstParent;
      for (const [filter, scope] of [
        [null, null],
        [null, { kind: 'current' }],
        [{ kind: 'path', path: 'src/file3.txt' }, null],
        [{ kind: 'ref', ref: 'keep' }, null],
      ] as [Filter | null, Scope | null][]) {
        const listed = historyOf(engine, root, filter, scope, head, firstParent);
        for (const revision of ['HEAD~40', 'side/50', 'keep~2', 'main~100']) {
          const commit = await id(root, revision);
          const what = `${revision} in ${JSON.stringify([filter, scope, firstParent])}`;
          expect(await positionOnEngine(engine, root, commit, await listed), what).toBe(await repo.position(commit, filter, scope));
        }
        for (const search of searches) {
          const what = `${JSON.stringify(search)} in ${JSON.stringify([filter, scope, firstParent])}`;
          expect(await matchesOnEngine(engine, root, search, await listed), what).toEqual(await repo.matches(search, filter, 5000, scope));
        }
      }
    }
  });
});

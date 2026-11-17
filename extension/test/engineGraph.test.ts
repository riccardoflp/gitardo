/**
 * The reads of the graph of stage 7 of `docs/transition/06-roadmap.md`: the rows of the graph read by the engine,
 * compared with what `git log` lists of the same history, read as git prints them and laid out page after page (D54);
 * and the position of a commit and the matches of a search, against where `git log` lists the commit and what it finds.
 */
import * as fs from 'node:fs';
import * as path from 'node:path';
import { describe, expect, it } from 'vitest';
import type { EngineClient } from '../src/engine/client';
import { type CommitsReader, EngineGraph, graphState, historyOf, laidOutOf, matchesOnEngine, positionOnEngine } from '../src/engine/graph';
import type { Timeline } from '../src/engine/messages';
import { Reads } from '../src/engine/reads';
import { GraphLayout } from '../src/shared/graph';
import { type Filter, type RepoState, type Scope, type Search, WORKTREE } from '../src/shared/protocol';
import { git } from './git';
import { built, cleanUpAfterEach, client, commitFile, repository } from './engine/harness';

cleanUpAfterEach();

/** What a commit of the graph is, as `git log` says it too. */
const commitsOf = (state: RepoState) =>
  state.rows
    .filter((row) => row.author && row.stash === undefined)
    .map(({ hash, parents, author, email, date, subject }) => ({ hash, parents, author, email, date, subject }));

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
  // A second later: two stashes of the same second on the same commit are listed apart (R8 of 04-differences.md).
  await new Promise((resolve) => setTimeout(resolve, 1100));
  fs.writeFileSync(path.join(root, 'renamed.txt'), 'kept too\n');
  await git(root, ['stash', 'push', '--quiet', '-m', 'Second']);
  fs.writeFileSync(path.join(root, 'new.txt'), 'not committed\n');
  return root;
}

/** Counts the reads of the commits the engine is asked for, sent as they are read. */
function counting(engine: EngineClient): Timeline[] {
  const asked: Timeline[] = [];
  const read = engine.graphStream.bind(engine);
  engine.graphStream = (repository, timeline, onRows, signal) => {
    asked.push(timeline);
    return read(repository, timeline, onRows, signal);
  };
  return asked;
}

interface Asked {
  filter?: Filter | null;
  scope?: Scope | null;
  stashes?: boolean;
  firstParent?: boolean;
}

/**
 * What `git log` is told to list the history a graph shows: by date, from the branches, remote branches and tags and
 * what is checked out, or from what the scope or a `ref` filter names; a search and the history of a file narrow it.
 */
async function gitSelection(root: string, asked: Asked, search?: Search): Promise<string[]> {
  const { filter, scope } = asked;
  const args = ['--date-order'];
  if (asked.firstParent) args.push('--first-parent');
  for (const text of [filter, search]) {
    if (text?.kind === 'message') args.push('--fixed-strings', '--regexp-ignore-case', `--grep=${text.query}`);
    if (text?.kind === 'author') args.push('--fixed-strings', '--regexp-ignore-case', `--author=${text.query}`);
    if (text?.kind === 'pickaxe') args.push(`-S${text.query}`);
    if (text?.kind === 'regex') args.push(`-G${text.query}`);
  }
  // The history of a file or of lines is connected by the nearest commits listed, which git gives as their parents.
  if (filter?.kind === 'path' || filter?.kind === 'lines') args.push('--parents');
  const head = (await git(root, ['rev-parse', '--verify', '--quiet', 'HEAD']).catch(() => null))?.stdout.trim();
  if (filter?.kind === 'ref') args.push(filter.ref);
  else if (filter?.kind === 'lines') args.push('--no-patch', `-L${filter.start},${filter.end}:${filter.path}`, filter.commit);
  else if (scope?.kind === 'current') {
    if (head) args.push('HEAD');
    const upstream = scope.upstream ? (await git(root, ['rev-parse', '--symbolic-full-name', '@{upstream}']).catch(() => null))?.stdout.trim() : '';
    if (upstream?.startsWith('refs/')) args.push(upstream);
  } else if (scope?.kind === 'refs') args.push('--ignore-missing', ...scope.refs);
  else {
    const kinds = scope?.kind === 'kinds' ? scope.kinds : ['branch', 'remote', 'tag'];
    if (kinds.includes('branch')) args.push('--branches');
    if (kinds.includes('remote')) args.push('--remotes');
    if (kinds.includes('tag')) args.push('--tags');
    if (head) args.push('HEAD');
  }
  if (filter?.kind === 'path') {
    // A file is followed under the names it had before; a folder is not.
    const names = filter.folder
      ? [filter.path]
      : [
          filter.path,
          ...(await git(root, ['log', '--follow', '--name-only', '--format=', '--', filter.path])).stdout
            .split('\n')
            .filter((name) => name && name !== filter.path),
        ];
    args.push('--', ...new Set(names.map((name) => `:(literal)${name}`)));
  }
  return args;
}

/** The commits `git log` lists of a graph, as many as `limit`, as the graph shows them, and whether there are more. */
async function gitLog(root: string, limit: number, asked: Asked) {
  const format = '%H%x01%P%x01%an%x01%ae%x01%at%x01%s%x02';
  const selection = await gitSelection(root, asked);
  const out = (await git(root, ['log', `--format=${format}`, `--max-count=${limit + 1}`, ...selection]).catch(() => ({ stdout: '' }))).stdout;
  const commits = out
    .split('\x02')
    .map((entry) => entry.trim())
    .filter(Boolean)
    .map((entry) => {
      const [hash, parents, author, email, date, subject] = entry.split('\x01');
      // With --parents git writes the parents after the commit too.
      const listed = parents ? parents.split(' ') : [];
      // The other parents of a merge are not followed when the graph follows the first parent only.
      return { hash, parents: asked.firstParent ? listed.slice(0, 1) : listed, author, email, date: Number(date), subject };
    });
  return { commits: commits.slice(0, limit), hasMore: commits.length > limit };
}

/** The state of a graph read with the engine's rows, as the controller reads it. */
async function engineGraphState(engine: EngineClient, root: string, limit: number, asked: Asked, reader: CommitsReader): Promise<RepoState> {
  const parts = await new Reads(() => engine).parts({ root });
  const state = { limit, filter: asked.filter ?? null, scope: asked.scope ?? null, stashes: asked.stashes ?? true, firstParent: asked.firstParent ?? false };
  return graphState(engine, root, parts, state, reader);
}

/** The rows of a graph once its read ended, every page laid out under the rows before it. */
async function wholeRows(graph: EngineGraph, root: string, state: RepoState): Promise<RepoState> {
  await graph.reached(root, Number.POSITIVE_INFINITY);
  const laid = laidOutOf(state.rows)!;
  return { ...state, rows: laid.catchUp(), hasMore: laid.source.hasMore };
}

/** Reads a graph with the engine as git prints it, in pages of `pageSize`, and compares every commit with `git log`. */
async function compare(root: string, asked: Asked, pageSize = 100, engine = client()): Promise<RepoState> {
  const graph = new EngineGraph();
  const reader: CommitsReader = (reading) => graph.commits(engine, root, reading, pageSize);
  const viaEngine = await wholeRows(graph, root, await engineGraphState(engine, root, pageSize, asked, reader));
  const plain = await gitLog(root, 100_000, asked);
  const what = JSON.stringify(asked);
  expect(commitsOf(viaEngine), what).toEqual(plain.commits);
  expect(viaEngine.hasMore, what).toBe(false);
  return viaEngine;
}

describe.skipIf(!built)('the graph read by the engine', { timeout: 120_000 }, () => {
  it('lists the commits git lists in every way the graph can show a small repository', async () => {
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
    for (const asked of ways) await compare(root, asked, 100, engine);
  });

  it('places the stashes above the commits they were made on, the newest first', async () => {
    const root = await small();
    const engine = client();
    const state = await engineGraphState(engine, root, 100, {}, (reading) => new EngineGraph().commits(engine, root, reading, 100));
    const stashes = state.rows.filter((row) => row.stash !== undefined);
    expect(stashes.map((row) => [row.stash, row.subject])).toEqual([
      ['stash@{0}', 'On main: Second'],
      ['stash@{1}', 'On main: First'],
    ]);
    for (const stash of stashes) {
      const above = state.rows.findIndex((row) => row.hash === stash.parents[0]);
      expect(above).toBeGreaterThan(state.rows.indexOf(stash));
    }
  });

  it('reads the whole history in one read, each page laid out under the rows before it as all the rows at once', async () => {
    const root = await history(330);
    for (const asked of [{}, { firstParent: true }, { scope: { kind: 'refs', refs: ['refs/heads/keep'] } } satisfies Asked]) {
      const engine = client();
      const reads = counting(engine);
      const viaEngine = await compare(root, asked, 100, engine);
      expect(reads.map((timeline) => timeline.skip), JSON.stringify(asked)).toEqual([0]);
      // The lanes of each page went on from the last row of the page before: they are those of every row at once.
      const layout = new GraphLayout();
      const atOnce = viaEngine.rows.map((row) => layout.next(row.hash, row.parents, row.hash === WORKTREE || row.stash !== undefined));
      expect(
        viaEngine.rows.map((row) => row.graph),
        JSON.stringify(asked),
      ).toEqual(atOnce);
    }
  });

  it('gives the first page as soon as it is read, and the next ones as they come', async () => {
    const root = await history(330);
    const engine = client();
    let pages = 0;
    const graph = new EngineGraph(() => pages++);
    const state = await engineGraphState(engine, root, 100, {}, (reading) => graph.commits(engine, root, reading, 100));
    const first = state.rows.filter((row) => row.stash === undefined && row.hash !== WORKTREE).length;
    expect(first).toBeGreaterThanOrEqual(100);
    const whole = await wholeRows(graph, root, state);
    expect(commitsOf(whole).length).toBe((await gitLog(root, 100_000, {})).commits.length);
    expect(pages).toBeGreaterThan(1);
  });

  it('places a stash of a later page where all the commits read at once put it', async () => {
    const root = await history(260);
    // A stash made on an old commit is newer than every commit: it goes to the top once that commit is listed.
    await git(root, ['checkout', '--quiet', 'main~230']);
    fs.writeFileSync(path.join(root, 'stashed.txt'), 'kept\n');
    await git(root, ['stash', 'push', '--quiet', '--include-untracked', '-m', 'Old']);
    await git(root, ['checkout', '--quiet', 'main']);
    const viaEngine = await compare(root, { stashes: true });
    const stash = viaEngine.rows.findIndex((row) => row.stash === 'stash@{0}');
    const made = viaEngine.rows.findIndex((row) => row.hash === viaEngine.rows[stash]?.parents[0] && row.stash === undefined);
    expect(stash).toBeGreaterThanOrEqual(0);
    expect(made).toBeGreaterThan(stash);
  });

  it('keeps its commits while the history is the same, reads them again when it changed, and a repository with no commit', async () => {
    const root = await history(150);
    const engine = client();
    const graph = new EngineGraph();
    const reader: CommitsReader = (asked) => graph.commits(engine, root, asked, 100);
    const reads = counting(engine);
    const before = await wholeRows(graph, root, await engineGraphState(engine, root, 100, {}, reader));
    fs.writeFileSync(path.join(root, 'changed.txt'), 'not committed\n');
    const same = await wholeRows(graph, root, await engineGraphState(engine, root, 100, {}, reader));
    expect(reads.length, 'a change to the files reads no commits').toBe(1);
    expect(commitsOf(same)).toEqual(commitsOf(before));
    await commitFile(root, 'new.txt', 'new\n', 'A new commit');
    const viaEngine = await wholeRows(graph, root, await engineGraphState(engine, root, 100, {}, reader));
    expect(commitsOf(viaEngine)).toEqual((await gitLog(root, 100_000, {})).commits);
    expect(reads.map((timeline) => timeline.skip)).toEqual([0, 0]);

    const empty = await repository(false);
    fs.writeFileSync(path.join(empty, 'first.txt'), 'first\n');
    await compare(empty, {}, 100, engine);
    await compare(empty, { scope: { kind: 'current' } }, 100, engine);
  });

  it('lists two stashes of the same second on one commit the newest first', async () => {
    const root = await repository();
    for (const message of ['Older', 'Newer']) {
      fs.writeFileSync(path.join(root, 'a.txt'), `${message}\n`);
      await git(root, ['stash', 'push', '--quiet', '-m', message]);
    }
    const [older, newer] = [await id(root, 'stash@{1}'), await id(root, 'stash@{0}')];
    const dates = (await git(root, ['log', '--no-walk=unsorted', '--format=%at', older, newer])).stdout.split('\n').filter(Boolean);
    // Made in two seconds, the two are listed apart by their dates, as the test of the stashes finds.
    if (dates[0] !== dates[1]) return;
    const engine = client();
    const viaEngine = await engineGraphState(engine, root, 100, {}, (asked) => new EngineGraph().commits(engine, root, asked, 100));
    expect(viaEngine.rows.flatMap((row) => row.stash ?? [])).toEqual(['stash@{0}', 'stash@{1}']);
  });

  it('finds a commit where git lists it, and the matches git finds', async () => {
    const root = await history(120);
    const engine = client();
    const head = await id(root, 'HEAD');
    const searches: Search[] = [
      { kind: 'message', query: 'parser' },
      { kind: 'author', query: 'grace' },
      { kind: 'pickaxe', query: 'Change 3' },
    ];
    for (const firstParent of [false, true]) {
      for (const [filter, scope] of [
        [null, null],
        [null, { kind: 'current' }],
        [{ kind: 'path', path: 'src/file3.txt' }, null],
        [{ kind: 'ref', ref: 'keep' }, null],
      ] as [Filter | null, Scope | null][]) {
        const asked = { filter, scope, firstParent };
        const listed = historyOf(engine, root, filter, scope, head, firstParent);
        const all = (await git(root, ['log', '--format=%H', ...(await gitSelection(root, asked))])).stdout.split('\n').filter(Boolean);
        for (const revision of ['HEAD~40', 'side/50', 'keep~2', 'main~100']) {
          const commit = await id(root, revision);
          const what = `${revision} in ${JSON.stringify([filter, scope, firstParent])}`;
          expect(await positionOnEngine(engine, root, commit, await listed), what).toBe(all.indexOf(commit));
        }
        for (const search of searches) {
          const what = `${JSON.stringify(search)} in ${JSON.stringify([filter, scope, firstParent])}`;
          const found = (await git(root, ['log', '--format=%H', '--max-count=5001', ...(await gitSelection(root, asked, search))])).stdout
            .split('\n')
            .filter(Boolean);
          expect(await matchesOnEngine(engine, root, search, await listed), what).toEqual({ hashes: found.slice(0, 5000), more: found.length > 5000 });
        }
      }
    }
  });
});

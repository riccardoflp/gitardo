// Times what Gitardo reads from a repository through its engine: node dist/perf.js <repository> [--engine=<binary>]
// Make a large one to try it on with test/perf/big-repo.mjs. The engine is the debug build of ../rust unless named.

import * as path from 'node:path';
import { binaryName } from '../../src/engine/binary';
import { EngineClient } from '../../src/engine/client';
import { EngineGraph, graphState, laidOutOf, readCommits, type StateAsked } from '../../src/engine/graph';
import { Reads } from '../../src/engine/reads';
import { engineSettings } from '../../src/engine/settings';
import type { Filter } from '../../src/shared/protocol';
import { laneCount } from '../../src/shared/graph';

const [root, ...options] = process.argv.slice(2);
if (!root) {
  console.error('Usage: node dist/perf.js <repository> [--engine=<binary>]');
  process.exit(1);
}
const binary =
  options.find((option) => option.startsWith('--engine='))?.slice(9) ??
  process.env.GITARDO_ENGINE ??
  path.resolve(__dirname, '../../rust/target/debug', binaryName());

async function timed<T>(what: string, run: () => Promise<T>, describe: (result: T) => string = () => ''): Promise<T> {
  const start = performance.now();
  const result = await run();
  console.log(`${`${Math.round(performance.now() - start)} ms`.padStart(9)}  ${what}  ${describe(result)}`);
  return result;
}

const kb = (value: unknown) => `${Math.round(JSON.stringify(value).length / 1024)} KB`;

async function main() {
  const settings = engineSettings({}, new Set(), {}, 0);
  const log = { info: () => {}, warn: console.warn, error: console.error, debug: () => {} };
  const client = new EngineClient({ binary, clientVersion: '0.0.0-perf', settings: () => settings, logLevel: () => 'info', log });
  const reads = new Reads(() => client);
  const repo = { root };
  const state = async (limit: number, filter: Filter | null = null) => {
    const asked: StateAsked = { limit, filter, scope: null, stashes: true, firstParent: false };
    return graphState(client, root, await reads.parts(repo), asked, (graph) => readCommits(client, root, graph, limit));
  };

  // The whole history as the controller reads it (D54): the state, then one read whose pages are laid out as they come.
  for (const run of [1, 2]) {
    const started = performance.now();
    let pages = 0;
    let firstPage = 0;
    let rowsLaid = 0;
    const graph = new EngineGraph(() => {
      pages++;
      firstPage ||= performance.now() - started;
    });
    const asked: StateAsked = { limit: 1000, filter: null, scope: null, stashes: true, firstParent: false };
    const shown = await graphState(client, root, await reads.parts(repo, false, true), asked, (wanted) => graph.commits(client, root, wanted, 1000));
    const atFirst = performance.now() - started;
    const laid = laidOutOf(shown.rows)!;
    await graph.reached(root, Number.MAX_SAFE_INTEGER);
    rowsLaid = laid.catchUp().length;
    const total = performance.now() - started;
    console.log(
      `${`${Math.round(total)} ms`.padStart(9)}  whole history, read ${run}: ${rowsLaid} rows laid out in ${pages} pages, ` +
        `the first page in ${Math.round(firstPage)} ms, the state with it in ${Math.round(atFirst)} ms, ` +
        `widest ${Math.max(...laid.rows.map((row) => laneCount(row.graph)))} lanes`,
    );
    graph.forget();
  }

  for (const limit of [1000, 5000, 20000]) {
    const read = await timed(
      `state, ${limit} commits`,
      () => state(limit),
      (result) =>
        `${result.rows.length} rows, ${kb(result)} (rows ${kb(result.rows)}), ` +
        `widest ${Math.max(...result.rows.map((row) => laneCount(row.graph)))} lanes, ` +
        `${result.branches.length} branches, ${result.tags.length} tags, ${result.workingTree.unstaged.length + result.workingTree.staged.length + result.workingTree.untracked.length} changes`,
    );
    if (limit === 1000) {
      const commit = read.rows.find((row) => row.parents.length === 1)!;
      await timed('details of a commit', () => reads.commitDetail(repo, commit.hash), (detail) => `${detail?.files.length} files`);
      const merge = read.rows.find((row) => row.parents.length === 2);
      if (merge) await timed('details of a merge', () => reads.commitDetail(repo, merge.hash), (detail) => `${detail?.files.length} files`);
    }
  }
  await timed('working tree', () => reads.working(repo), (working) => `${working.unstaged.length} unstaged, ${working.staged.length} staged`);
  await timed('search by message', () => state(1000, { kind: 'message', query: 'tooltip' }), (result) => `${result.rows.length} rows`);
  await timed('search by author', () => state(1000, { kind: 'author', query: 'hopper' }), (result) => `${result.rows.length} rows`);
  await timed('search by change', () => state(1000, { kind: 'pickaxe', query: 'line 4242' }), (result) => `${result.rows.length} rows`);
  const tracked = (await reads.fileAt(root, 'HEAD', 'src/mod00/part0/file0.txt')) !== undefined ? 'src/mod00/part0/file0.txt' : null;
  if (tracked) {
    await timed('history of a file', () => state(1000, { kind: 'path', path: tracked }), (result) => `${result.rows.length} rows`);
    await timed('blame of a line', () => reads.blameLine(repo, tracked, 1), (line) => line?.summary ?? 'none');
  }
  await client.dispose();
}

void main();

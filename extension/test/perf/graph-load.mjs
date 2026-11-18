// Opens a repository in a real VS Code and records how its graph loads, as the eye sees it:
// node test/perf/graph-load.mjs <repository> [--out=<folder>] [--commit] [--profile] [path to VS Code]
// Meant for a large repository, such as test/perf/big-repo.mjs makes, and for one of a few thousand commits. It
// records, every 50 ms, how many commits the list has, where it is scrolled, what says that more are loading, how wide
// the graph is and which messages with commits the view got, while the graph opens, on Refresh, while the list is
// scrolled to its end, and, with --commit, when a commit is made outside while the list is scrolled far down: the only
// thing it changes in the repository, an empty commit on what is checked out.

import { execFileSync } from 'node:child_process';
import * as fs from 'node:fs';
import * as os from 'node:os';
import * as path from 'node:path';
import { sleep, until } from '../e2e/cdp.mjs';
import { engineSetting, remove, source } from '../e2e/fixture.mjs';
import { startWindow } from '../e2e/window.mjs';

const options = process.argv.slice(2).filter((argument) => argument.startsWith('--'));
const [repository, codePath] = process.argv.slice(2).filter((argument) => !argument.startsWith('--'));
if (!repository) {
  console.error('Usage: node test/perf/graph-load.mjs <repository> [--out=<folder>] [--commit] [--profile] [path to VS Code]');
  process.exit(1);
}
const out = path.resolve(options.find((option) => option.startsWith('--out='))?.slice(6) ?? path.join(source, 'dist', 'graph-load'));
const profiled = options.includes('--profile');
const temp = fs.mkdtempSync(path.join(os.tmpdir(), 'gitardo-load-'));
const engine = engineSetting(temp, options);
console.log(`Measuring ${engine.said}`);
const window = await startWindow({ workspace: path.resolve(repository), temp, out, codePath, settings: engine.settings });
const { shot, command, webview, problems } = window;
const timer = setTimeout(() => finish(new Error('Timed out.')), 240_000);

async function finish(error) {
  clearTimeout(timer);
  if (error) console.error(error.stack ?? String(error));
  if (problems.length) console.log(`\nErrors inside the views:\n${problems.join('\n')}`);
  console.log(`\nScreenshots are in ${out}`);
  await window.close();
  remove(temp);
  process.exit(error || problems.length ? 1 : 0);
}

/** Runs inside the graph: keeps what the list shows every 50 ms, each message with commits, and each frame that took long. */
const RECORD = `(() => {
  if (window.__load) return;
  const samples = [];
  const messages = [];
  const frames = [];
  setInterval(() => {
    const list = document.querySelector('.commits');
    const spacer = list?.querySelector('.spacer');
    const rows = spacer ? Math.round(spacer.offsetHeight / 24) : 0;
    samples.push({
      at: Math.round(performance.now()),
      rows,
      top: list ? Math.round(list.scrollTop / 24) : 0,
      drawn: document.querySelectorAll('.commits .row').length,
      bar: !!document.querySelector('.commits .progress'),
      more: !!document.querySelector('.commits .row.more'),
      stale: !!list?.classList.contains('stale'),
      graph: Number(document.querySelector('.commits svg.graph')?.getAttribute('width') ?? 0),
      first: document.querySelector('.commits .row:not(.worktree) .subject')?.textContent ?? '',
    });
  }, 50);
  addEventListener('message', (event) => {
    const message = event.data;
    if (message?.type === 'state' && message.state) {
      messages.push({ at: Math.round(performance.now()), sent: message.state.rows.length, kept: message.kept ?? 0, more: message.state.hasMore });
    } else if (message?.type === 'loading') messages.push({ at: Math.round(performance.now()), loading: message.what });
  });
  let last = performance.now();
  const frame = (now) => {
    if (now - last > 50) frames.push({ at: Math.round(now), took: Math.round(now - last) });
    last = now;
    requestAnimationFrame(frame);
  };
  requestAnimationFrame(frame);
  window.__load = { samples, messages, frames };
})()`;

/** What was recorded since the last time it was asked. */
const TAKE = `(() => {
  const taken = { now: Math.round(performance.now()), samples: window.__load.samples.splice(0), messages: window.__load.messages.splice(0), frames: window.__load.frames.splice(0) };
  return taken;
})()`;

/** Says what was recorded: the messages, the frames that took long, and each time what the list shows changed. */
function report(title, { samples, messages, frames }, from) {
  console.log(`\n== ${title}`);
  const since = (at) => `${String(at - from).padStart(6)} ms`;
  for (const message of messages) {
    if ('loading' in message) console.log(`${since(message.at)}  told: loading ${message.loading ?? 'is over'}`);
    else console.log(`${since(message.at)}  message: ${message.sent} rows sent, ${message.kept} kept, ${message.more ? 'more to come' : 'the last'}`);
  }
  if (frames.length) console.log(`  frames over 50 ms: ${frames.map((frame) => `${frame.took} ms at ${frame.at - from}`).join(', ')}`);
  let before = '';
  let changes = 0;
  for (const sample of samples) {
    const { at, ...shown } = sample;
    const text = `rows ${shown.rows}, at row ${shown.top}, ${shown.drawn} drawn, graph ${shown.graph}px${shown.bar ? ', bar' : ''}${shown.more ? ', "Loading more commits…"' : ''}${shown.stale ? ', dimmed' : ''}, first "${shown.first.slice(0, 40)}"`;
    if (text === before) continue;
    before = text;
    changes++;
    console.log(`${since(at)}  ${text}`);
  }
  console.log(`  ${changes} changes of what the list shows, ${messages.filter((message) => 'sent' in message).length} messages with commits`);
}

/** Says where the view spent its time, from a profile of it: the functions that ran longest themselves. */
function busiest(profile) {
  const spent = new Map();
  const nodes = new Map(profile.nodes.map((node) => [node.id, node]));
  profile.samples.forEach((id, index) => {
    const { functionName, url, lineNumber } = nodes.get(id).callFrame;
    const name = `${functionName || '(anonymous)'} ${url.split('/').pop()}:${lineNumber + 1}`;
    spent.set(name, (spent.get(name) ?? 0) + (profile.timeDeltas[index] ?? 0) / 1000);
  });
  console.log('\n== Where the view spent the Refresh, the 30 functions that ran longest themselves');
  for (const [name, time] of [...spent].sort((one, other) => other[1] - one[1]).slice(0, 30)) console.log(`${String(Math.round(time)).padStart(6)} ms  ${name}`);
}

/**
 * Looks at the list while its commits are read again: screenshots from the first page of the read, when the last row
 * says more are loading, then a wait until it no longer does. A read with nothing new to show has no such row.
 */
async function watch(graph, name) {
  const loading = () => graph.evaluate(`!!document.querySelector('.commits .row.more')`);
  const started = await until('the first page', loading, 8000).catch(() => false);
  for (let index = 0; started && index < 6; index++) {
    await shot(`${name}-${index}`);
    await sleep(80);
  }
  await until('the last page', async () => !(await loading()), 30_000);
  await sleep(1500);
}

try {
  await window.ready();
  await command('Gitardo: Show Graph');
  // What the window shows while the graph opens, since the view cannot be asked before it is there.
  const opening = (async () => {
    for (let index = 0; index < 12; index++) {
      await shot(`opening-${index}`);
      await sleep(150);
    }
  })();
  const graph = await webview('graph');
  await graph.evaluate(RECORD);
  await opening;
  await until('the commits', () => graph.evaluate(`document.querySelectorAll('.commits .row').length > 5`), 60_000);
  await sleep(6000);
  let taken = await graph.evaluate(TAKE);
  report('Opening the graph (from when the view could be asked)', taken, taken.samples[0]?.at ?? 0);
  await shot('opened');

  // Left alone for a while: whether what says that more commits are loading goes away by itself.
  await sleep(4000);
  taken = await graph.evaluate(TAKE);
  const settled = taken.samples.at(-1);
  console.log(`\n== Left alone for ten seconds: ${settled.rows} rows${settled.more ? ', and "Loading more commits…" still at the end' : ', all of them'}`);

  // Refresh reads every commit again: the read from its first page to its last.
  let from = (await graph.evaluate(TAKE)).now;
  if (profiled) {
    await graph.session.send('Profiler.enable');
    await graph.session.send('Profiler.setSamplingInterval', { interval: 500 });
    await graph.session.send('Profiler.start');
  }
  await graph.evaluate(`document.querySelector('button[aria-label="Refresh"]').click()`);
  await watch(graph, 'refresh');
  if (profiled) busiest((await graph.session.send('Profiler.stop')).profile);
  report('Refresh, at the top of the list', await graph.evaluate(TAKE), from);

  // Scrolled to the end as fast as the wheel goes, again and again until nothing more comes.
  from = (await graph.evaluate(TAKE)).now;
  for (let index = 0; index < 40; index++) {
    await graph.evaluate(`(() => { const list = document.querySelector('.commits'); list.scrollTop = list.scrollHeight; })()`);
    await sleep(150);
    if (index === 3 || index === 12) await shot(`scrolled-${index}`);
  }
  report('Scrolling to the end', await graph.evaluate(TAKE), from);
  await shot('end');

  // In the middle of the list, Refresh: whether the list stays where it was scrolled to.
  await graph.evaluate(`(() => { const list = document.querySelector('.commits'); list.scrollTop = Math.round(list.scrollHeight / 2); })()`);
  await sleep(500);
  from = (await graph.evaluate(TAKE)).now;
  await shot('middle-before');
  await graph.evaluate(`document.querySelector('button[aria-label="Refresh"]').click()`);
  await watch(graph, 'middle-refresh');
  report('Refresh, scrolled to the middle of the list', await graph.evaluate(TAKE), from);
  await shot('middle-after');

  if (options.includes('--commit')) {
    await graph.evaluate(`(() => { const list = document.querySelector('.commits'); list.scrollTop = Math.round(list.scrollHeight / 2); })()`);
    await sleep(500);
    from = (await graph.evaluate(TAKE)).now;
    execFileSync('git', ['commit', '--quiet', '--allow-empty', '--no-verify', '-m', 'Made while the graph was scrolled'], { cwd: path.resolve(repository) });
    await watch(graph, 'commit');
    report('A commit made outside, scrolled to the middle of the list', await graph.evaluate(TAKE), from);
    await shot('commit-after');
  }
} catch (error) {
  await finish(error);
}
await finish();

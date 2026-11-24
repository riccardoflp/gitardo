// Opens a repository in a real VS Code and times what a person waits for: node test/perf/vscode.mjs <repository> [--out=<folder>] [--limits] [path to VS Code]
// Meant for a large one, such as test/perf/big-repo.mjs makes. The only thing it changes there is a file it adds and removes.
// With --limits it fails when a time is over the one docs/extension/12-quality.md gives its scenario, times one and a
// half (D91, Q1519): the pipeline runs it so on Linux, whose machines are the steadiest.

import * as fs from 'node:fs';
import * as os from 'node:os';
import * as path from 'node:path';
import { sleep, until } from '../e2e/cdp.mjs';
import { engineSetting, remove, source } from '../e2e/fixture.mjs';
import { startWindow } from '../e2e/window.mjs';

const options = process.argv.slice(2).filter((argument) => argument.startsWith('--'));
const [repository, codePath] = process.argv.slice(2).filter((argument) => !argument.startsWith('--'));
if (!repository) {
  console.error('Usage: node test/perf/vscode.mjs <repository> [--out=<folder>] [path to VS Code]');
  process.exit(1);
}
const LIMITS = options.includes('--limits');
/** How far past the time of its scenario a measure may go before it fails (Q1519). */
const MARGIN = 1.5;
/** The time the working tree's changes wait before a refresh reads them, `WORKTREE_REFRESH_DELAY` of the extension. */
const WORKTREE_REFRESH_DELAY = 1000;

/**
 * The times of the scenarios of docs/extension/12-quality.md, in milliseconds, read from its table so that the page
 * stays the one place they are written: the first time of the column Measure, "about 1.5 s" or "about 300 ms a page".
 * A scenario with no time there, such as QA4's "no key waits", has no limit.
 */
function qualityTimes() {
  const page = fs.readFileSync(path.join(source, '..', 'docs', 'extension', '12-quality.md'), 'utf8');
  const times = {};
  for (const line of page.split('\n')) {
    const cells = line.split('|').map((cell) => cell.trim());
    if (!/^QA\d+$/.test(cells[1] ?? '')) continue;
    const time = /(\d+(?:\.\d+)?)\s*(ms|s)\b/.exec(cells[4] ?? '');
    if (time) times[cells[1]] = Number(time[1]) * (time[2] === 's' ? 1000 : 1);
  }
  for (const scenario of ['QA1', 'QA2', 'QA3']) if (!times[scenario]) throw new Error(`12-quality.md gives no time for ${scenario}`);
  return times;
}
const times = qualityTimes();
/** The measures over their limit, said at the end. */
const over = [];
/** Says a measure of a scenario against its limit and, with --limits, keeps it when it is over. */
function limit(scenario, what, measured) {
  const allowed = Math.round(times[scenario] * MARGIN);
  const passed = measured <= allowed;
  console.log(`${passed ? '' : 'OVER '}${scenario}: ${what} ${measured} ms, at most ${allowed} ms (${times[scenario]} ms × ${MARGIN})`);
  if (!passed) over.push(`${scenario}: ${what} took ${measured} ms, over ${allowed} ms`);
}

const out = path.resolve(options.find((option) => option.startsWith('--out='))?.slice(6) ?? path.join(source, 'dist', 'perf'));
const temp = fs.mkdtempSync(path.join(os.tmpdir(), 'gitardo-perf-'));
// The engine the end-to-end suite uses: without the setting, a checkout has none packed to find.
const engine = engineSetting(temp, options);
console.log(`Measuring ${engine.said}`);
const window = await startWindow({ workspace: path.resolve(repository), temp, out, codePath, settings: engine.settings });
const { shot, command, webview, problems } = window;
const timer = setTimeout(() => finish(new Error('Timed out.')), 240_000);
const probe = path.join(path.resolve(repository), 'gitardo-perf.txt');

const report = (what, value) => console.log(`${String(value).padStart(10)}  ${what}`);

async function finish(error) {
  clearTimeout(timer);
  fs.rmSync(probe, { force: true });
  if (error) console.error(error.stack ?? String(error));
  if (problems.length) console.log(`\nErrors inside the views:\n${problems.join('\n')}`);
  if (over.length) console.log(`\nOver the times of 12-quality.md times ${MARGIN}:\n${over.join('\n')}`);
  console.log(`\nScreenshots are in ${out}`);
  await window.close();
  remove(temp);
  process.exit(error || problems.length || (LIMITS && over.length) ? 1 : 0);
}

/** Runs inside a view: does something, and times the state that comes back and how long the view takes to show it. */
const TIMED = `(act) => new Promise((resolve) => {
  const started = performance.now();
  const listener = (event) => {
    if (event.data?.type !== 'state') return;
    removeEventListener('message', listener);
    const received = performance.now();
    requestAnimationFrame(() => setTimeout(() => resolve({
      wait: Math.round(received - started),
      draw: Math.round(performance.now() - received),
      sent: event.data.state.rows.length,
      kept: event.data.kept ?? 0,
    }), 0));
  };
  addEventListener('message', listener);
  act();
})`;

try {
  await window.ready();
  // From the command chosen in the palette, not from its name typed there, which is the script's time, not Gitardo's.
  let started = Date.now();
  await command('Gitardo: Show Graph', () => (started = Date.now()));
  const graph = await webview('graph');
  await until('the commits', () => graph.evaluate(`document.querySelectorAll('.row').length > 5`), 60_000);
  const opened = Date.now() - started;
  report('to open the graph once chosen in the command palette', `${opened} ms`);
  limit('QA1', 'the graph opened from the Command Palette', opened);
  const count = (selector) => graph.evaluate(`document.querySelectorAll(${JSON.stringify(selector)}).length`);
  report('elements in the graph view', await count('*'));
  report('references listed', `${await count('.refs .ref')} of ${await graph.evaluate(`[...document.querySelectorAll('.refs .count')].reduce((sum, badge) => sum + Number(badge.textContent), 0)`)}`);
  await shot('graph');

  const describe = (timing) => `${timing.wait} ms + ${timing.draw} ms to draw (${timing.kept ? `${timing.kept} rows not sent again, ${timing.sent} sent` : `${timing.sent} commits sent`})`;
  const refresh = () => graph.evaluate(`(${TIMED})(() => document.querySelector('button[aria-label="Refresh"]').click())`);
  await refresh();
  report('refresh', describe(await refresh()));

  const subject = () => graph.evaluate(`document.querySelector('.row .subject')?.textContent ?? ''`);
  const before = await subject();
  const changed = Date.now();
  fs.writeFileSync(probe, 'changed\n');
  await until('the view to notice the new file', async () => (await subject()) !== before, 30_000);
  const shown = Date.now() - changed;
  report(`for a new file to show: "${await subject()}"`, `${shown} ms`);
  // The refresh of QA2 starts once the wait for more changes of the working tree is over.
  limit('QA2', 'a new file shown, after the wait for more changes', Math.max(0, shown - WORKTREE_REFRESH_DELAY));

  // The commits come a page after the other without being asked for (D54): there is a next page to wait for only while
  // the last row says more are loading, during a read or past the 100,000 commits of one.
  for (let page = 0; page < 4; page++) {
    if (!(await graph.evaluate(`!!document.querySelector('.commits .row.more')`))) {
      report('commits loaded, every one of the history', (await graph.evaluate(`Math.round(document.querySelector('.commits .spacer').offsetHeight / 24)`)));
      break;
    }
    const timing = await graph.evaluate(`(${TIMED})(() => { const list = document.querySelector('.commits'); list.scrollTop = list.scrollHeight; })`);
    report(`next page of commits, ${timing.sent} loaded`, describe(timing));
    limit('QA3', 'the next page of commits', timing.wait + timing.draw);
  }
  report('refresh with those loaded', describe(await refresh()));
  const selected = await graph.evaluate(`new Promise((resolve) => {
    const started = performance.now();
    document.querySelectorAll('.row')[4].click();
    requestAnimationFrame(() => setTimeout(() => resolve(Math.round(performance.now() - started)), 0));
  })`);
  report('to select a commit', `${selected} ms`);
  await until('the details', () => graph.evaluate(`!!document.querySelector('.detail h2')`));
  await shot('details');

  // The filter of the references, typed the way a person does.
  const filterAt = await graph.locate('.refs-filter input');
  await window.mouse(filterAt.x, filterAt.y);
  await window.type('feature/00');
  await until('the filtered references', async () => (await count('.refs .ref')) < 30);
  report('references matching "feature/00"', await count('.refs .ref'));
  await shot('filter');

  await command('Gitardo: Show Changes');
  const changes = await webview('changes');
  await until('the changed files', () => changes.evaluate(`document.querySelectorAll('.group .file').length > 0`), 30_000);
  report('changed files listed', `${await changes.evaluate(`document.querySelectorAll('.group .file').length`)} of ${await changes.evaluate(`[...document.querySelectorAll('.group .count')].reduce((sum, badge) => sum + Number(badge.textContent), 0)`)}`);
  const box = await changes.locate('.commit-box textarea');
  await window.mouse(box.x, box.y);
  const typing = Date.now();
  for (const letter of 'A message typed letter by letter') await window.type(letter);
  await until('the typed message', () => changes.evaluate(`document.querySelector('.commit-box textarea').value === 'A message typed letter by letter'`));
  report('to type a message of 32 letters', `${Date.now() - typing} ms`);
  await shot('changes');
  await sleep(300);
} catch (error) {
  await finish(error);
}
await finish();

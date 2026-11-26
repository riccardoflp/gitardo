// Times how long the graph takes to show what git changed outside VS Code, for test/perf/vscode.mjs --changes, on the
// workspace test/perf/changes-fixture.mjs makes: thirty repositories with their builds, and a folder beside them where
// files are written. In a real VS Code, with the graph of the panel on the first repository, a commit, a branch and a
// worktree are made by git in turn, each timed until the graph shows it: from the first rows of the graph after the
// window starts, while a process writes files without end in the folder beside the repositories, and after it stopped.
// It changes the first repository and leaves it as the fixture made it.

import { execFile, spawn } from 'node:child_process';
import * as fs from 'node:fs';
import * as os from 'node:os';
import * as path from 'node:path';
import { promisify } from 'node:util';
import { sleep, until } from '../e2e/cdp.mjs';
import { engineSetting, remove } from '../e2e/fixture.mjs';
import { startWindow } from '../e2e/window.mjs';

/**
 * The moments the changes are made in. Each makes `changes` of them, one every `every` milliseconds whether those
 * before were shown or not, and gives up on one not shown after `giveUp`: a change not shown yet when the next is made
 * may be shown by the read that one asks for, as it would be for a person. `scenario` is the one of
 * docs/extension/12-quality.md whose time the worst of the moment may not pass with --limits: none yet (QA18 and QA19
 * come with the last phase of milestone M52).
 */
const MOMENTS = {
  start: { what: 'from the first rows of the graph, one a second for 25 seconds', changes: 25, every: 1000, giveUp: 5000, scenario: null },
  written: { what: 'while files are written beside the repositories, one every 4 seconds', changes: 9, every: 4000, giveUp: 6000, scenario: null },
  after: { what: 'in the 15 seconds after the writing stopped, one every 2.5 seconds', changes: 6, every: 2500, giveUp: 6000, scenario: null },
};
/** The kinds of change, made in turn. */
const KINDS = ['commit', 'branch', 'worktree'];
/** How long files are written before the first change of the moment `written` is made. */
const WRITTEN_FIRST = 8000;
/** How often the graph is looked at for what it shows. */
const LOOK_EVERY = 50;
/** What the log of the watcher of the window says when it drops the changes it is told of. */
const IGNORING = 'started ignoring events due to too many file change events at once';

/** The button of the toolbar that names the repository shown and opens the menu of them all. */
const REPOSITORIES = 'button.place.pick.repo';

/** How many files the process that writes is to write a second, if the machine lets it: a build of many processes. */
const WRITTEN_A_SECOND = 5000;

/**
 * What the process that writes runs, in the folder it is started in: ten files in one of 400 folders at each turn, and
 * the ten written there 400 turns before removed, as a build does, waiting only when it is ahead of the files it is to
 * write a second. The number of the turn is in the names of the files, which is how the measure knows how many were
 * written.
 */
const WRITER = `
const fs = require('node:fs');
const still = new Int32Array(new SharedArrayBuffer(4));
const started = Date.now();
for (let turn = 0; ; turn++) {
  const folder = 'folder-' + (turn % 400);
  fs.mkdirSync(folder, { recursive: true });
  for (let file = 1; file <= 10; file++) {
    fs.writeFileSync(folder + '/file-' + file + '.' + turn + '.o', String(turn));
    if (turn >= 400) fs.rmSync(folder + '/file-' + file + '.' + (turn - 400) + '.o', { force: true });
  }
  const ahead = ((turn + 1) * 10 * 1000) / ${WRITTEN_A_SECOND} - (Date.now() - started);
  if (ahead > 1) Atomics.wait(still, 0, 0, ahead);
}`;

/** Runs inside the graph view: which of the names given it shows, in a row of the commits or in the list of the references. */
const SHOWN = `(names) => { const text = document.body.textContent; return names.filter((name) => text.includes(name)); }`;

const run = promisify(execFile);
const median = (values) => [...values].sort((one, other) => one - other)[Math.floor(values.length / 2)];
const seconds = (milliseconds) => `${(milliseconds / 1000).toFixed(1)} s`;

/** The files the writer wrote so far, from the highest turn in the names of those in its folder. */
function filesWritten(folder) {
  let turn = -1;
  for (const each of fs.readdirSync(folder)) {
    for (const file of fs.readdirSync(path.join(folder, each))) turn = Math.max(turn, Number(file.split('.')[1]));
  }
  return (turn + 1) * 10;
}

/**
 * How many times each log of the window says the watcher dropped changes, by the name of the log: `fileWatcher.log` is
 * the watcher's own, and the window says it again in `renderer.log`. None when the logs are not there to read.
 */
function ignoredInLogs(temp) {
  const logs = path.join(temp, 'user-data', 'logs');
  if (!fs.existsSync(logs)) return null;
  const times = {};
  for (const file of fs.readdirSync(logs, { recursive: true }).map(String)) {
    if (!file.endsWith('.log')) continue;
    const said = fs.readFileSync(path.join(logs, file), 'utf8').split(IGNORING).length - 1;
    if (said) times[path.basename(file)] = (times[path.basename(file)] ?? 0) + said;
  }
  return times;
}

/**
 * Times the changes on the fixture in `folder`, prints each and what each moment comes to, and gives the problems the
 * views had. `limit` is called with the scenario, the moment and its worst time for each moment that has a scenario.
 */
export async function measureChanges({ folder, out, codePath, options, limit }) {
  const workspace = path.join(folder, 'workspace');
  const worktrees = path.join(folder, 'worktrees');
  const written = path.join(workspace, 'written');
  const repositories = fs.existsSync(workspace) ? fs.readdirSync(workspace).filter((name) => fs.existsSync(path.join(workspace, name, '.git'))).sort() : [];
  if (!repositories.length || !fs.existsSync(written)) throw new Error(`${folder} has no workspace: make it with test/perf/changes-fixture.mjs`);
  const repository = path.join(workspace, repositories[0]);
  /** Git as the measure runs it, without the optional locks, as Gitardo's engine reads. */
  const git = (...args) => run('git', args, { cwd: repository, env: { ...process.env, GIT_OPTIONAL_LOCKS: '0' } });

  /** Leaves the repository as the fixture made it, whatever a measure before left in it. */
  const asMade = async () => {
    fs.rmSync(worktrees, { recursive: true, force: true });
    fs.mkdirSync(worktrees);
    await git('worktree', 'prune');
    await git('checkout', '--quiet', 'main');
    await git('reset', '--quiet', '--hard', 'fixture');
    const branches = (await git('for-each-ref', '--format=%(refname:short)', 'refs/heads')).stdout.split('\n').filter((branch) => branch && branch !== 'main');
    if (branches.length) await git('branch', '--quiet', '-D', ...branches);
    for (const each of fs.readdirSync(written)) fs.rmSync(path.join(written, each), { recursive: true, force: true });
  };
  await asMade();

  const temp = fs.mkdtempSync(path.join(os.tmpdir(), 'gitardo-perf-'));
  const engine = engineSetting(temp, options);
  console.log(`Measuring ${engine.said}, on ${repositories.length} repositories in ${workspace}`);
  const windowStarted = Date.now();
  const window = await startWindow({ workspace, temp, out, codePath, settings: engine.settings });
  const sinceWindow = () => `${seconds(Date.now() - windowStarted)} after the window started`;
  const results = {};
  let writer;
  try {
    await window.ready();
    await window.command('Gitardo: Show Graph');
    const graph = await window.webview('graph');
    const shown = () => graph.evaluate(`document.querySelector('${REPOSITORIES}')?.title.split('\\n')[0] ?? ''`);
    await until('the commits', () => graph.evaluate(`document.querySelectorAll('.commits .row').length > 2`), 180_000);
    if (path.resolve(await shown()) !== path.resolve(repository)) {
      await graph.evaluate(`document.querySelector('${REPOSITORIES}').click()`);
      await until('the menu of the repositories', () => graph.evaluate(`[...document.querySelectorAll('[role="menuitemcheckbox"]')].some((item) => item.title === ${JSON.stringify(repository)})`));
      await graph.evaluate(`[...document.querySelectorAll('[role="menuitemcheckbox"]')].find((item) => item.title === ${JSON.stringify(repository)}).click()`);
      await until(`the graph on ${repository}`, async () => path.resolve(await shown()) === path.resolve(repository), 60_000);
    }
    // The branches are looked for in the list of the references, whose sections are closed when VS Code starts.
    await graph.evaluate(`document.querySelector('.refs [data-section="branches"] button.section[aria-expanded="false"]')?.click()`);
    console.log(`The graph has its first rows ${sinceWindow()}`);

    /** Makes one change, named so that the graph showing the name is the graph showing the change. */
    const change = async (kind, name) => {
      if (kind === 'commit') await git('commit', '--quiet', '--allow-empty', '-m', name);
      else if (kind === 'branch') await git('branch', name);
      else await git('worktree', 'add', '--quiet', path.join(worktrees, name), '-b', name);
    };

    /** One moment: its changes made at their times, each timed from when git has made it until the graph shows it. */
    const moment = async (id) => {
      const { what, changes, every, giveUp } = MOMENTS[id];
      console.log(`\n${what}, each given up after ${giveUp / 1000} s:`);
      const waiting = [];
      const timed = [];
      const started = Date.now();
      let made = 0;
      /** How many changes git has not made yet: on a machine that is busy git falls behind the times of the changes. */
      let unmade = 0;
      let failure;
      let making = Promise.resolve();
      while (made < changes || unmade || waiting.length) {
        if (failure) throw failure;
        if (made < changes && Date.now() - started >= made * every) {
          const kind = KINDS[made % KINDS.length];
          const name = `${id}-${String(made + 1).padStart(2, '0')}-${kind}`;
          made++;
          unmade++;
          // One git at a time, and the graph looked at meanwhile, also while the last changes are still being made.
          making = making
            .then(async () => {
              await change(kind, name);
              waiting.push({ kind, name, at: Date.now(), since: sinceWindow() });
              unmade--;
            })
            .catch((error) => {
              failure ??= error;
            });
        }
        const there = waiting.length ? await graph.evaluate(`(${SHOWN})(${JSON.stringify(waiting.map((each) => each.name))})`).catch(() => []) : [];
        const now = Date.now();
        for (const each of [...waiting]) {
          const seen = there.includes(each.name);
          if (!seen && now - each.at < giveUp) continue;
          waiting.splice(waiting.indexOf(each), 1);
          const took = seen ? now - each.at : null;
          timed.push({ ...each, took });
          console.log(`${(took === null ? 'not seen' : `${took} ms`).padStart(10)}  a ${each.kind}, made ${each.since}`);
        }
        await sleep(LOOK_EVERY);
      }
      results[id] = timed;
    };

    await moment('start');

    writer = spawn(process.execPath, ['-e', WRITER], { cwd: written, stdio: ['ignore', 'ignore', 'pipe'] });
    // It is to write until it is stopped: one that ends before leaves a moment in which nothing was written.
    let complaint = '';
    let ended = null;
    writer.stderr.on('data', (data) => (complaint += data));
    writer.on('exit', (code, signal) => (ended ??= code ?? signal));
    const writingStarted = Date.now();
    await sleep(WRITTEN_FIRST);
    await moment('written');
    if (ended !== null) throw new Error(`The process that writes files ended by itself, with ${ended}:\n${complaint}`);
    ended = 'stopped';
    writer.kill('SIGKILL');
    writer = undefined;
    const files = filesWritten(written);
    console.log(`\nThe writing stopped ${sinceWindow()}: ${files} files written, ${Math.round(files / ((Date.now() - writingStarted) / 1000))} a second`);
    await moment('after');
    await window.shot('changes');
  } finally {
    writer?.kill('SIGKILL');
    await window.close();
    const ignored = ignoredInLogs(temp);
    remove(temp);
    await asMade().catch((error) => console.log(`Could not leave ${repository} as it was made: ${error.message}`));
    results.ignored = ignored;
  }

  console.log('\nA commit, a branch and a worktree made by git outside, until the graph shows them:');
  for (const [id, each] of Object.entries(MOMENTS)) {
    const timed = results[id];
    if (!timed) continue;
    const seen = timed.filter((one) => one.took !== null).map((one) => one.took);
    const missed = timed.length - seen.length;
    const lastMissed = timed.filter((one) => one.took === null).sort((one, other) => other.at - one.at)[0];
    const worst = missed ? `not seen in ${each.giveUp / 1000} s, the last of them made ${lastMissed.since}` : `${Math.max(...seen)} ms`;
    const byKind = KINDS.map((kind) => {
      const ofKind = timed.filter((one) => one.kind === kind);
      const seenOfKind = ofKind.filter((one) => one.took !== null).map((one) => one.took);
      return `${kind} ${seenOfKind.length ? `${median(seenOfKind)} ms` : 'never'}${seenOfKind.length < ofKind.length ? `, ${ofKind.length - seenOfKind.length} of ${ofKind.length} not seen` : ''}`;
    });
    console.log(`${(seen.length ? `${median(seen)} ms` : 'never').padStart(10)}  ${each.what}: the median of those seen; the worst ${worst}; ${missed} of ${timed.length} not seen (${byKind.join('; ')})`);
    // A change not seen is over any time.
    if (each.scenario) limit(each.scenario, each.what, missed ? Number.MAX_SAFE_INTEGER : Math.max(...seen));
  }
  const { ignored } = results;
  const inWatcher = ignored?.['fileWatcher.log'] ?? 0;
  const elsewhere = Object.entries(ignored ?? {}).filter(([log]) => log !== 'fileWatcher.log').map(([log, times]) => `${log} ${times}`);
  if (!ignored) console.log('The logs of the window were not found.');
  else console.log(`fileWatcher.log of the window ${inWatcher ? `says "${IGNORING}" ${inWatcher} ${inWatcher === 1 ? 'time' : 'times'}` : `never says "${IGNORING}"`}${elsewhere.length ? `; said also in ${elsewhere.join(', ')}` : ''}.`);
  return window.problems;
}

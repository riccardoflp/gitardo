// Times what a person waits for after choosing a repository in a real VS Code, for test/perf/vscode.mjs --switch, on
// the workspace test/perf/switch-fixture.mjs makes: a small repository, shown when the window opens, a large one beside
// it and three worktrees of the large one. Each time is from the choice until the graph has drawn the first rows of
// what was chosen, in a window of its own for each run, and the median of the runs is said.

import { execFileSync } from 'node:child_process';
import * as fs from 'node:fs';
import * as os from 'node:os';
import * as path from 'node:path';
import { sleep, until } from '../e2e/cdp.mjs';
import { engineSetting, remove, source } from '../e2e/fixture.mjs';
import { startWindow } from '../e2e/window.mjs';

/** The windows opened, each measuring every choice once. The fixture has a worktree never shown for each. */
const RUNS = 3;

/**
 * The choices timed. `scenario` is the one of docs/extension/12-quality.md whose time, times the margin, the median
 * may not pass with --limits.
 */
const CHOICES = {
  never: { what: 'a repository never shown in this window, chosen for the first time', scenario: 'QA17' },
  worktree: { what: 'a worktree of the repository shown, chosen for the first time', scenario: 'QA16' },
  again: { what: 'a repository shown before, chosen again', scenario: 'QA15' },
  againKept: { what: 'a repository shown before, chosen again, with gitardo.repositories.keepOpen on', scenario: 'QA15' },
};

/**
 * Runs inside the graph view: makes a choice, and waits for the first state of the folder chosen that leaves the view
 * with commits of it, and for the view to have drawn it. That state carries the first page of the commits, the rest
 * coming behind it in pieces; or the commits that differ from those the view has, between the ones it keeps; or no
 * commit at all, for a worktree whose rows are the ones the view has. The commits of the repository left stay in the
 * view until then. `sent` is how many commits that first state carried. `from` and `to` are the clock's times of the
 * choice and of the rows drawn, to count what the engine ran between them. With `own`, it waits for the state after
 * that one too, which for a worktree of the repository shown is its first step, with what it has checked out: `own`
 * is then the same times for it.
 */
const CHOSEN = `(root, act, own) => new Promise((resolve) => {
  const same = (one, other) => one.replace(/\\\\/g, '/').toLowerCase() === other.replace(/\\\\/g, '/').toLowerCase();
  let chosen = false;
  let first = null;
  const drawn = (state, then) => {
    const received = performance.now();
    requestAnimationFrame(() => setTimeout(() => then({
      wait: Math.round(received - started),
      draw: Math.round(performance.now() - received),
      sent: state.rows?.length ?? 0,
      from,
      to: Date.now(),
    }), 0));
  };
  const listener = (event) => {
    const state = event.data?.type === 'state' ? event.data.state : null;
    if (!state) return;
    if (typeof state.root === 'string') chosen = same(state.root, root);
    if (first) {
      removeEventListener('message', listener);
      return drawn(state, (timing) => first.then((rows) => resolve({ ...rows, own: timing })));
    }
    // A state that names no rows leaves the view the ones it has, which are those of the folder chosen.
    const keeps = Array.isArray(event.data.parts) && !event.data.parts.includes('rows');
    if (!chosen || !(keeps || (state.rows?.length ?? 0) + (event.data.kept ?? 0) + (event.data.keptLast ?? 0))) return;
    if (!own) removeEventListener('message', listener);
    first = new Promise((timed) => drawn(state, timed));
    if (!own) first.then(resolve);
  };
  addEventListener('message', listener);
  const from = Date.now();
  const started = performance.now();
  act();
})`;

/** Chooses a repository in the menu of the repositories already open, by its folder, as a person picks it there. */
const IN_LIST = `(root) => () => [...document.querySelectorAll('[role="menuitemcheckbox"]')].find((item) => item.title === root).click()`;

/** The button of the toolbar that names the repository shown and opens the menu of them all, and the one of the worktrees. */
const REPOSITORIES = 'button.place.pick.repo';
const WORKTREES = 'button.place.pick:not(.repo)';

/** Chooses, in the menu of the worktrees already open, the one that has a branch checked out. */
const IN_MENU = `(branch) => () => [...document.querySelectorAll('[role="menuitemcheckbox"]')].find((item) => item.textContent.includes(branch)).click()`;

const median = (values) => [...values].sort((one, other) => one - other)[Math.floor(values.length / 2)];

/**
 * An engine that is the one given with every git it starts writing its events of trace2 to a file, so that the
 * processes of git of a choice can be counted; none where there is no shell to start it with, or no engine to name.
 */
function tracedEngine(temp, engine) {
  const real = engine.settings['gitardo.engine.path'];
  if (process.platform === 'win32' || !real || !fs.existsSync(real)) return null;
  const trace = path.join(temp, 'git-trace.json');
  const starter = path.join(temp, 'traced', 'gitardo-engine');
  const quoted = (text) => `'${text.replaceAll("'", "'\\''")}'`;
  fs.mkdirSync(path.dirname(starter), { recursive: true });
  fs.writeFileSync(starter, `#!/bin/sh\nGIT_TRACE2_EVENT=${quoted(trace)} exec ${quoted(real)} "$@"\n`, { mode: 0o755 });
  return { trace, settings: { ...engine.settings, 'gitardo.engine.path': starter } };
}

/**
 * The processes of git the engine started between two times of the clock, by command, from their `start` events of
 * trace2: a process that another git started has a `sid` with a `/` and is not counted.
 */
function processesBetween(trace, from, to) {
  const commands = {};
  for (const line of fs.readFileSync(trace, 'utf8').split('\n')) {
    if (!line.includes('"event":"start"')) continue;
    const event = JSON.parse(line);
    const time = Date.parse(event.time);
    if (event.sid.includes('/') || time < from || time > to) continue;
    const words = event.argv.slice(1);
    let command = 'git';
    for (let index = 0; index < words.length; index++) {
      if (words[index] === '-c' || words[index] === '-C') index++;
      else if (!words[index].startsWith('-')) {
        command = words[index];
        break;
      }
    }
    commands[command] = (commands[command] ?? 0) + 1;
  }
  return commands;
}

const total = (commands) => Object.values(commands).reduce((sum, count) => sum + count, 0);
const said = (commands) =>
  Object.keys(commands)
    .sort()
    .map((command) => `${command} ${commands[command]}`)
    .join(', ');

/** One window: every choice once, in the order that leaves each as its name says. Gives the timing of each. */
async function oneWindow({ places, worktree, out, codePath, options, run }) {
  const temp = fs.mkdtempSync(path.join(os.tmpdir(), 'gitardo-perf-'));
  const engine = engineSetting(temp, options);
  const traced = tracedEngine(temp, engine);
  if (run === 0) console.log(`Measuring ${engine.said}${traced ? '' : ', the processes of git not counted'}`);
  const window = await startWindow({ workspace: places.workspace, temp, out, codePath, settings: traced?.settings ?? engine.settings });
  const timings = {};
  try {
    await window.ready();
    await window.command('Gitardo: Show Graph');
    const graph = await window.webview('graph');
    const shown = () => graph.evaluate(`document.querySelector('${REPOSITORIES}')?.title.split('\\n')[0] ?? ''`);
    /** Waits until nothing is read any more: every commit of the history has come, and no choice is refused. */
    const quiet = async () => {
      await until('the commits', () => graph.evaluate(`document.querySelectorAll('.commits .row').length > 5`), 60_000);
      await until('the whole history', () => graph.evaluate(`!document.querySelector('.commits .row.more') && !document.querySelector('${REPOSITORIES}')?.disabled`), 120_000);
      await sleep(2000);
    };
    const choose = async (root, act, own = false) => {
      const timing = await graph.evaluate(`(${CHOSEN})(${JSON.stringify(root)}, (${act}), ${own})`);
      if (path.resolve(await shown()) !== path.resolve(root)) throw new Error(`The graph shows ${await shown()}, not ${root}`);
      if (traced) timing.processes = processesBetween(traced.trace, timing.from, timing.to);
      if (traced && timing.own) timing.own.processes = processesBetween(traced.trace, timing.own.from, timing.own.to);
      await quiet();
      return timing;
    };
    const inList = async (root) => {
      await graph.evaluate(`document.querySelector('${REPOSITORIES}').click()`);
      await until('the menu of the repositories', () => graph.evaluate(`[...document.querySelectorAll('[role="menuitemcheckbox"]')].some((item) => item.title === ${JSON.stringify(root)})`));
      return choose(root, `(${IN_LIST})(${JSON.stringify(root)})`);
    };

    await quiet();
    if (path.resolve(await shown()) !== path.resolve(places.beside)) throw new Error(`The window opened on ${await shown()}, not on ${places.beside}`);
    timings.never = await inList(places.large);

    await graph.evaluate(`document.querySelector('${WORKTREES}').click()`);
    await until('the menu of the worktrees', () => graph.evaluate(`document.querySelectorAll('[role="menuitemcheckbox"]').length > 1`));
    timings.worktree = await choose(worktree.folder, `(${IN_MENU})(${JSON.stringify(worktree.branch)})`, true);

    // The large repository shown again, so that it is left as a repository and not as one of its worktrees.
    await inList(places.beside);
    await inList(places.large);
    await inList(places.beside);
    timings.again = await inList(places.large);

    window.setting('gitardo.repositories.keepOpen', true);
    // VS Code reads the settings file again on its own, and the views are told.
    await sleep(1500);
    await inList(places.beside);
    timings.againKept = await inList(places.large);
    await window.shot(`switch-${run + 1}`);
  } finally {
    await window.close();
    remove(temp);
  }
  return { timings, problems: window.problems };
}

/**
 * Times the choices on the fixture in `folder`, prints each run and the medians, and gives the problems the views had.
 * `limit` is called with the scenario, the choice and the median of each choice that has a scenario.
 */
export async function measureSwitches({ folder, out, codePath, options, limit }) {
  const workspace = path.join(folder, 'workspace');
  const places = { workspace, large: path.join(workspace, 'large'), beside: path.join(workspace, 'beside') };
  if (!fs.existsSync(places.large) || !fs.existsSync(places.beside)) throw new Error(`${folder} has no workspace: make it with test/perf/switch-fixture.mjs`);
  // The worktrees besides the folder of the repository itself, each with the branch it has checked out.
  const worktrees = execFileSync('git', ['worktree', 'list', '--porcelain'], { cwd: places.large })
    .toString()
    .split('\n\n')
    .map((entry) => ({ folder: /^worktree (.*)$/m.exec(entry)?.[1], branch: /^branch refs\/heads\/(.*)$/m.exec(entry)?.[1] }))
    .filter((entry) => entry.folder && entry.branch && path.resolve(entry.folder) !== path.resolve(places.large));
  if (worktrees.length < RUNS) throw new Error(`${places.large} has ${worktrees.length} worktrees, fewer than the ${RUNS} runs`);
  const byDefault = JSON.parse(fs.readFileSync(path.join(source, 'package.json'), 'utf8')).contributes.configuration.flatMap((section) => Object.entries(section.properties ?? {})).find(([name]) => name === 'gitardo.repositories.keepOpen')?.[1].default;

  const runs = [];
  const problems = [];
  for (let run = 0; run < RUNS; run++) {
    const window = await oneWindow({ places, worktree: worktrees[run], out, codePath, options, run });
    runs.push(window.timings);
    problems.push(...window.problems);
  }

  console.log(`\nChoosing a repository, until the graph has drawn its first rows: the median of ${RUNS} windows, gitardo.repositories.keepOpen ${byDefault ? 'on' : 'off'} by default`);
  for (const [name, choice] of Object.entries(CHOICES)) {
    const each = runs.map((timings) => timings[name]);
    const times = each.map((timing) => timing.wait + timing.draw);
    console.log(`${`${median(times)} ms`.padStart(10)}  ${choice.what} (${each.map((timing) => `${timing.wait} + ${timing.draw} ms`).join(', ')}; ${each.map((timing) => timing.sent).join(', ')} commits in the first state sent)`);
    if (each[0].processes) {
      const counts = each.map((timing) => total(timing.processes));
      console.log(`${String(median(counts)).padStart(10)}  processes of git before those rows (${counts.join(', ')}; in the first window: ${said(each[0].processes) || 'none'})`);
    }
    // The worktree shows the rows of its repository first, and then what it has checked out, once its first step is read.
    const own = each.map((timing) => timing.own).filter(Boolean);
    if (own.length) {
      const marked = own.map((timing) => timing.wait + timing.draw);
      console.log(`${`${median(marked)} ms`.padStart(10)}  until it shows what it has checked out, after its first step (${own.map((timing) => `${timing.wait} + ${timing.draw} ms`).join(', ')}; ${own.map((timing) => timing.sent).join(', ')} commits in that state)`);
      if (own[0].processes) {
        const counts = own.map((timing) => total(timing.processes));
        console.log(`${String(median(counts)).padStart(10)}  processes of git before that (${counts.join(', ')}; in the first window: ${said(own[0].processes) || 'none'})`);
      }
    }
    if (choice.scenario) limit(choice.scenario, choice.what, median(times));
  }
  return problems;
}

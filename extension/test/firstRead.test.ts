/**
 * A repository that shows nothing yet read in two steps (`repo.read-first`, D92): what is asked in which order, and
 * what the views are sent at each step, with the engine played by the test.
 */
import { describe, expect, it } from 'vitest';
import { type StepsRead, readInSteps } from '../src/extension/firstRead';

/** Lets what waits for a promise already settled go on. */
const turn = () => new Promise<void>((resolve) => setTimeout(resolve, 0));

/** Something asked, which the test answers when it says. */
function asked<Value>() {
  let answer!: (value: Value) => void;
  let fail!: (error: unknown) => void;
  const promise = new Promise<Value>((resolve, reject) => {
    answer = resolve;
    fail = reject;
  });
  promise.catch(() => {});
  return { promise, answer, fail };
}

type Parts = { head: string; remotes: string[] };
type State = Parts & { rows: string[] };

/** A read in two steps of two groups, each answer of the engine given by the test, and what happened in its order. */
function steps(options: { whole?: boolean } = {}) {
  const events: string[] = [];
  const first = asked<Parts>();
  const rest = asked<Parts>();
  const folder = asked<void>();
  const commits = asked<(State | null)[]>();
  const hosting = asked<void>();
  const held = { value: true };
  const read: StepsRead<Parts, State> = {
    first: async () => {
      events.push('ask the first step');
      const parts = await first.promise;
      return { parts, rest: options.whole ? null : rest.promise };
    },
    beforeCommits: () => folder.promise,
    commits: (parts) => {
      events.push(`ask the commits of ${parts.head}`);
      return commits.promise;
    },
    holds: () => held.value,
    showFirst: async (states) =>
      void events.push(`show the first step: ${states.map((state) => `${state.rows.join()} ${state.remotes.join() || 'no remote'}`).join('; ')}`),
    beforeWhole: () => hosting.promise,
    withRest: (state, whole) => ({ ...state, ...whole }),
  };
  const done = readInSteps(read);
  done.then(
    () => events.push('read'),
    () => events.push('failed'),
  );
  return { events, first, rest, folder, commits, hosting, held, done };
}

const FIRST: Parts = { head: 'main', remotes: [] };
const WHOLE: Parts = { head: 'main', remotes: ['origin'] };

describe('a repository read in two steps', () => {
  it('asks the commits as soon as the first step and the folder the worktrees share are there, and shows the first step with the first rows before the rest', async () => {
    const read = steps();
    await turn();
    expect(read.events).toEqual(['ask the first step']);
    read.first.answer(FIRST);
    await turn();
    // The reads of the commits are kept by the folder the worktrees share: they wait for it when it is not known.
    expect(read.events).toEqual(['ask the first step']);
    read.folder.answer();
    await turn();
    expect(read.events).toEqual(['ask the first step', 'ask the commits of main']);
    read.commits.answer([
      { ...FIRST, rows: ['c2', 'c1'] },
      { ...FIRST, rows: ['c2'] },
    ]);
    await turn();
    // The rest and the hosting service are not waited for: the views have their rows.
    expect(read.events).toEqual(['ask the first step', 'ask the commits of main', 'show the first step: c2,c1 no remote; c2 no remote']);
    read.rest.answer(WHOLE);
    await turn();
    expect(read.events).toHaveLength(3);
    read.hosting.answer();
    // The whole state of each group has the rest in its place, and the rows of its first step.
    expect(await read.done).toEqual([
      { ...WHOLE, rows: ['c2', 'c1'] },
      { ...WHOLE, rows: ['c2'] },
    ]);
  });

  it('shows the rest that came before the first page of the commits once, whole, after the first step', async () => {
    const read = steps();
    read.folder.answer();
    read.hosting.answer();
    read.first.answer(FIRST);
    read.rest.answer(WHOLE);
    await turn();
    expect(read.events).toEqual(['ask the first step', 'ask the commits of main']);
    read.commits.answer([{ ...FIRST, rows: ['c1'] }, null]);
    // A group whose commits were cancelled for another read is shown nothing of this one, nor is the other apart.
    expect(await read.done).toEqual([{ ...WHOLE, rows: ['c1'] }, null]);
    expect(read.events).toEqual(['ask the first step', 'ask the commits of main', 'read']);
  });

  it('shows a first step that is whole already once, as the state of any read', async () => {
    const read = steps({ whole: true });
    read.folder.answer();
    read.hosting.answer();
    read.first.answer(WHOLE);
    read.commits.answer([{ ...WHOLE, rows: ['c1'] }]);
    expect(await read.done).toEqual([{ ...WHOLE, rows: ['c1'] }]);
    expect(read.events).toEqual(['ask the first step', 'ask the commits of main', 'read']);
  });

  it('shows nothing, and waits for no rest, when an operation ran or the repository was left meanwhile', async () => {
    const read = steps();
    read.folder.answer();
    read.first.answer(FIRST);
    await turn();
    read.held.value = false;
    read.commits.answer([{ ...FIRST, rows: ['c1'] }]);
    // Neither the rest nor the hosting service ever answer.
    expect(await read.done).toBeNull();
    expect(read.events).toEqual(['ask the first step', 'ask the commits of main', 'read']);
  });

  it('fails when the rest cannot be read, after the first step was shown', async () => {
    const read = steps();
    read.folder.answer();
    read.hosting.answer();
    read.first.answer(FIRST);
    read.commits.answer([{ ...FIRST, rows: ['c1'] }]);
    await turn();
    read.rest.fail(new Error('git ended'));
    await expect(read.done).rejects.toThrow('git ended');
    expect(read.events).toEqual(['ask the first step', 'ask the commits of main', 'show the first step: c1 no remote', 'failed']);
  });
});

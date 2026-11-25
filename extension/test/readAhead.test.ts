/**
 * The turns of the repositories read ahead (`repo.watch-hidden`, D92): one after the other once nothing stands in the
 * way, stopped by an operation or a read of a repository shown, and given their turn again after it.
 */
import { describe, expect, it } from 'vitest';
import { ReadAhead } from '../src/extension/readAhead';

/** Lets what waits for a promise already settled go on. */
const turn = () => new Promise<void>((resolve) => setTimeout(resolve, 0));

/** A window with repositories that wait, whose reads end when the test says. */
function window(roots: string[]) {
  const waiting = [...roots];
  const started: string[] = [];
  const failures: string[] = [];
  const ends = new Map<string, { kept: (kept: boolean) => void; fail: (error: unknown) => void; stop: AbortSignal }>();
  const state = { free: true };
  const ahead = new ReadAhead({
    waiting: () => [...waiting],
    isFree: () => state.free,
    read: (root, stop) => {
      started.push(root);
      return new Promise<boolean>((resolve, reject) => {
        ends.set(root, {
          kept: (kept) => {
            // What was read is kept only when the read was not stopped: the repository then waits no more.
            if (kept && !stop.aborted) waiting.splice(waiting.indexOf(root), 1);
            resolve(kept && !stop.aborted);
          },
          fail: reject,
          stop,
        });
      });
    },
    failed: (root) => failures.push(root),
  });
  return { ahead, started, failures, state, end: (root: string, kept = true) => ends.get(root)!.kept(kept), fail: (root: string) => ends.get(root)!.fail(new Error('no')), stopOf: (root: string) => ends.get(root)!.stop };
}

describe('the repositories read ahead', () => {
  it('are read one after the other, in the order of the list', async () => {
    const { ahead, started, end } = window(['/work/web', '/work/docs', '/work/tools']);
    ahead.next();
    ahead.next();
    expect(started).toEqual(['/work/web']);
    expect(ahead.reading).toBe('/work/web');
    end('/work/web');
    await turn();
    expect(started).toEqual(['/work/web', '/work/docs']);
    end('/work/docs');
    await turn();
    end('/work/tools');
    await turn();
    expect(started).toEqual(['/work/web', '/work/docs', '/work/tools']);
    expect(ahead.reading).toBeUndefined();
  });

  it('are not read while an operation runs or a repository shown is read, nor with the setting off', async () => {
    const { ahead, started, state } = window(['/work/web']);
    state.free = false;
    ahead.next();
    await turn();
    expect(started).toEqual([]);
    state.free = true;
    ahead.next();
    expect(started).toEqual(['/work/web']);
  });

  it('stop the one being read when an operation or a read of a repository shown starts, which waits for its turn again', async () => {
    const { ahead, started, state, end, stopOf } = window(['/work/web', '/work/docs']);
    ahead.next();
    state.free = false;
    ahead.stop();
    expect(stopOf('/work/web').aborted).toBe(true);
    expect(ahead.reading).toBeUndefined();
    // The read stopped ends, and nothing is read while the operation runs.
    end('/work/web');
    await turn();
    expect(started).toEqual(['/work/web']);
    state.free = true;
    ahead.next();
    expect(started).toEqual(['/work/web', '/work/web']);
    end('/work/web');
    await turn();
    expect(started).toEqual(['/work/web', '/work/web', '/work/docs']);
  });

  it('pass over a repository whose read failed or kept nothing, until the list is read again', async () => {
    const { ahead, started, failures, end, fail } = window(['/work/web', '/work/docs', '/work/tools']);
    ahead.next();
    fail('/work/web');
    await turn();
    expect(failures).toEqual(['/work/web']);
    // A worktree of a repository shown has nothing to read ahead: it is passed over, and said nowhere.
    end('/work/docs', false);
    await turn();
    end('/work/tools');
    await turn();
    expect(started).toEqual(['/work/web', '/work/docs', '/work/tools']);
    expect(failures).toEqual(['/work/web']);
    ahead.again();
    ahead.next();
    expect(started.at(-1)).toBe('/work/web');
  });
});

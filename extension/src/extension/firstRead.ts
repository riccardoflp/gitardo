/**
 * A repository that shows nothing yet, read in two steps (`repo.read-first`, D92): the views are sent its first step
 * with the first rows of its history as soon as those are read, and the rest of its state when the engine has read it
 * behind them. Nothing here asks the engine anything itself: `StepsRead` does, so that the order can be tested alone.
 */

/** What a read in two steps asks of the session it reads for. `Parts` is the state without its commits, `State` with them. */
export interface StepsRead<Parts, State> {
  /** Asks the first step of the state; `rest` gives the whole state, and is null when the first step is whole already. */
  first(): Promise<{ parts: Parts; rest: Promise<Parts> | null }>;
  /** What the commits wait for beside the first step: the folder the worktrees share, which their reads are kept by. */
  beforeCommits(): Promise<void>;
  /** The state of each group from the parts read, with the first commits of its history; null for a read cancelled. */
  commits(parts: Parts): Promise<(State | null)[]>;
  /** Whether what was read may be shown: no operation ran meanwhile, and the repository is still the one shown. */
  holds(): boolean;
  /** Sends the views of each group its first step, with its first rows. */
  showFirst(states: State[]): Promise<void>;
  /** What the whole state waits for beside the rest: the hosting service of the main remote. */
  beforeWhole(): Promise<void>;
  /** A state made from the first step with the rest in its place, its rows being the same. */
  withRest(state: State, rest: Parts): State;
}

/**
 * Reads a repository in two steps, and gives the whole state of each group, to be shown as the state of any read is:
 * the views then get only the parts the rest changed, and no row. The first step is asked first; the commits are asked
 * as soon as it ends, and the first step is shown with their first page while the rest is read. A first step that is
 * whole already, as when the repository was being opened whole, is not shown apart. Null when an operation ran or the
 * repository was left meanwhile: nothing is shown of it, and the rest is not waited for.
 */
export async function readInSteps<Parts, State>(read: StepsRead<Parts, State>): Promise<(State | null)[] | null> {
  const { parts, rest } = await read.first();
  await read.beforeCommits();
  const states = await read.commits(parts);
  if (!read.holds()) return null;
  // A group whose commits were cancelled for another read shows nothing of this one: the read after it does.
  if (rest && states.every((state) => state !== null)) await read.showFirst(states);
  const whole = rest && (await rest);
  await read.beforeWhole();
  return whole ? states.map((state) => state && read.withRest(state, whole)) : states;
}

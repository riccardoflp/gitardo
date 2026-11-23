/**
 * The notice of git's commit-graph file (`upkeep.offer-commit-graph`): once the read of the whole history of the
 * repository shown ends with 10,000 commits or more, while git keeps no commit-graph file of it and would read one, a
 * notification says that git can read the history about twice as fast with one, with **Write it**, which runs
 * `upkeep.write-commit-graph` on the engine, and **Not for this repository**, which the workspace state keeps under
 * `gitardo.commitGraphDeclined`, the roots of the repositories ([11-data.md](../../../docs/extension/11-data.md)). It is
 * said once each time the extension starts, for each repository.
 */

import type * as vscode from 'vscode';
import type { RepoState } from '../shared/protocol';
import { samePath } from '../shared/paths';

/** The key of the workspace state that keeps the roots of the repositories answered **Not for this repository**. */
export const DECLINED_KEY = 'gitardo.commitGraphDeclined';

/** How many commits the history has, at least, for the file to be offered (Q1515). */
export const OFFERED_FROM = 10_000;

/** The button of the notification that writes the file. */
export const WRITE_IT = 'Write it';

/** The button of the notification that says it no more for the repository. */
export const NOT_FOR_THIS_REPOSITORY = 'Not for this repository';

/** What the notification says of a repository. */
export const noticeOf = (name: string) => `Git can read the history of ${name} about twice as fast with its commit-graph file.`;

/**
 * Whether a repository whose history was read whole is offered the file: 10,000 commits or more, the stashes left out,
 * and no commit-graph file where git would read one.
 */
export function offersCommitGraph(state: Pick<RepoState, 'rows' | 'commitGraph'>): boolean {
  const graph = state.commitGraph;
  if (!graph || graph.hasFile || graph.isIgnored) return false;
  let commits = 0;
  for (const row of state.rows) if (!row.stash) commits++;
  return commits >= OFFERED_FROM;
}

/** What the offer needs of VS Code and of the controller. */
export interface OfferHost {
  /** The workspace state. */
  memento: Pick<vscode.Memento, 'get' | 'update'>;
  /** Shows the notification with its buttons, and gives the one chosen. */
  ask: (message: string, ...buttons: string[]) => Thenable<string | undefined>;
  /** Runs `upkeep.write-commit-graph` on the repository shown, through `Controller.runOnEngine`. */
  write: () => Promise<unknown>;
}

/** The repositories the notification was said for since the extension started, by their roots. */
const said: string[] = [];

/**
 * Says the notification for a repository once the read of its whole history ended, when it is offered the file, it was
 * not said for it since the extension started, and it was not answered **Not for this repository**. Gives the button
 * chosen, for the tests.
 */
export async function offerCommitGraph(state: RepoState | null, host: OfferHost): Promise<string | undefined> {
  if (!state || !offersCommitGraph(state)) return undefined;
  const root = state.root;
  const declined = host.memento.get<string[]>(DECLINED_KEY, []);
  if ([...said, ...declined].some((each) => samePath(each, root))) return undefined;
  said.push(root);
  const choice = await host.ask(noticeOf(state.name), WRITE_IT, NOT_FOR_THIS_REPOSITORY);
  if (choice === WRITE_IT) await host.write();
  if (choice === NOT_FOR_THIS_REPOSITORY) await host.memento.update(DECLINED_KEY, [...host.memento.get<string[]>(DECLINED_KEY, []), root]);
  return choice;
}

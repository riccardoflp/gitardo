import type { RepositoryState, ResolvedAsBefore } from './state';

/*
 * The files in conflict git resolved by itself from a resolution it recorded (`rerere.resolved-as-before`): the words
 * the banner, the rows of the Gitardo Changes view and the message of a stop say of them, as Gitardo Desktop says them.
 */

/** Stands for a file in conflict as git marked it when it stopped (`rerere.read-marked`), as a revision of a file. */
export const MARKED = ':marked';

/** What the row of such a file says, muted, after its name. */
export const RESOLVED_AS_BEFORE = 'Resolved as before';

/** What the left side of the diff of **Show what git wrote** is called. */
export const AS_GIT_MARKED_IT = 'As git marked it';

/** The file git resolved as before at this path, if it did. */
export function resolvedAsBefore(state: Pick<RepositoryState, 'resolvedAsBefore'>, path: string): ResolvedAsBefore | undefined {
  return state.resolvedAsBefore?.find((file) => file.path === path);
}

/** What the banner adds after the count of the files in conflict: `git resolved 2 as before`; nothing for none. */
export function bannerWords(state: Pick<RepositoryState, 'resolvedAsBefore'>): string | undefined {
  const count = state.resolvedAsBefore?.length ?? 0;
  return count ? `git resolved ${count} as before` : undefined;
}

/** What the message of an operation that stopped adds; nothing when git resolved none as before. */
export function stopWords(state: Pick<RepositoryState, 'resolvedAsBefore'>): string | undefined {
  const count = state.resolvedAsBefore?.length ?? 0;
  if (!count) return undefined;
  return `Git resolved ${count === 1 ? '1 file' : `${count} files`} as it did before: look at them before you continue.`;
}

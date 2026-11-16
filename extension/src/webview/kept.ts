import { useLayoutEffect, useRef } from 'react';

/**
 * What a view showed of each repository it was pointed away from, while `gitardo.repositories.keepOpen` is on
 * (`repo.keep-open`): each part of the view keeps its own, under its name, by the folder of the repository. In memory,
 * in the page of the view: it goes with the view, and it is all forgotten once the setting is turned off.
 */
export class KeptViews {
  private readonly kept = new Map<string, Map<string, unknown>>();

  /** Keeps a part of what the view showed of a repository, as it is left. */
  leave(root: string, part: string, value: unknown): void {
    const key = folderKey(root);
    const parts = this.kept.get(key) ?? new Map<string, unknown>();
    parts.set(part, value);
    this.kept.set(key, parts);
  }

  /** What a part of the view kept of a repository, if it kept anything. */
  back<T>(root: string, part: string): T | undefined {
    return this.kept.get(folderKey(root))?.get(part) as T | undefined;
  }

  forget(): void {
    this.kept.clear();
  }
}

/** A folder as the repositories are told apart by, whatever its slashes and its case. */
const folderKey = (root: string) => root.replace(/\\/g, '/').toLowerCase();

/** What this view kept of the repositories it was pointed away from. */
export const keptViews = new KeptViews();

/**
 * Keeps a part of what the view shows for each repository: once the view is pointed at another repository, the value
 * it had is kept for the one left, and the one shown gets back what it kept, before the view is drawn. A repository that
 * kept nothing, or any repository while the setting is off, leaves the value as it is.
 */
export function useKept<T>(root: string, keepOpen: boolean, part: string, value: T, restore: (kept: T) => void): void {
  const shown = useRef(root);
  // The value as the repository shown last had it, before the view was pointed at another.
  const current = useRef(value);
  useLayoutEffect(() => {
    if (!keepOpen) keptViews.forget();
    if (shown.current !== root) {
      const left = shown.current;
      shown.current = root;
      if (!keepOpen) return;
      keptViews.leave(left, part, current.current);
      const back = keptViews.back<T>(root, part);
      if (back !== undefined) {
        current.current = back;
        return restore(back);
      }
    }
    current.current = value;
  });
}

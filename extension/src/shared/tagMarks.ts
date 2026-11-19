/**
 * The marks of the tags a remote lacks (`tag.remote-marks`), kept for each repository while VS Code is open: what the
 * last **Check tags on `<remote>`** said of each tag, which the engine reads (`tag.read-remote-tags`,
 * `tag.compare-remote`), and what Gitardo's own operations on tags changed since, which the engine says with the
 * follow-up `tagsOnRemote`. As the desktop keeps them (`model::tag::TagMarks`).
 */
import type { Tag } from './state';

/** Where a tag of the repository stands on a remote. */
export type TagOnRemote = 'pushed' | 'differentThere' | 'notPushed';

/** The answer of `read/tag.read-remote-tags`: the tags of the repository sorted against those of the remote. */
export interface TagComparison {
  remote: string;
  tags: { name: string; onRemote: TagOnRemote }[];
  /** The tags of the remote with no tag of their name here. */
  onlyThere: string[];
}

/** What is known of a tag against the remote checked, with the commit it was at here then; null when it was not here. */
export interface TagMark {
  name: string;
  onRemote: TagOnRemote;
  commit: string | null;
}

/** What the last check of a remote said of the tags, as Gitardo's operations changed it since. */
export interface TagMarks {
  remote: string;
  tags: TagMark[];
}

/** The follow-up `tagsOnRemote`: the tags an operation pushed to a remote, and those the remote does not have after it. */
export interface TagsOnRemote {
  /** Null for a tag made here, which no remote has. */
  remote: string | null;
  pushed: string[];
  notPushed: string[];
}

/** The marks of a check: each tag of the comparison, at the commit the repository has it at. */
export function checkedMarks(comparison: TagComparison, tags: Pick<Tag, 'name' | 'commit'>[]): TagMarks {
  const commitOf = (name: string) => tags.find((tag) => tag.name === name)?.commit ?? null;
  return { remote: comparison.remote, tags: comparison.tags.map(({ name, onRemote }) => ({ name, onRemote, commit: commitOf(name) })) };
}

/**
 * Where a tag stands on the remote checked: not pushed when the check did not know it, made since; different there when
 * it was pushed at another commit than the one it is at now, moved since. Null before any check.
 */
export function markOf(marks: TagMarks | null | undefined, tag: Pick<Tag, 'name' | 'commit'>): TagOnRemote | null {
  if (!marks) return null;
  const mark = marks.tags.find((known) => known.name === tag.name);
  if (!mark) return 'notPushed';
  if (mark.onRemote === 'pushed' && mark.commit !== null && mark.commit !== tag.commit) return 'differentThere';
  return mark.onRemote;
}

/**
 * The mark drawn after a tag in the references and among the icons of its label: `cloud-upload` with `Not on origin`,
 * `warning` with `origin has another tag of this name`, since the Codicons have neither `cloud-off` nor `cloud-alert`;
 * none for a tag the remote has as here, or before any check.
 */
export function remoteMark(marks: TagMarks | null | undefined, tag: Pick<Tag, 'name' | 'commit'>): { icon: string; title: string } | null {
  switch (markOf(marks, tag)) {
    case 'notPushed':
      return { icon: 'cloud-upload', title: `Not on ${marks!.remote}` };
    case 'differentThere':
      return { icon: 'warning', title: `${marks!.remote} has another tag of this name` };
    default:
      return null;
  }
}

/** How many tags of the repository the remote checked lacks. */
export function notPushedCount(marks: TagMarks | null | undefined, tags: Pick<Tag, 'name' | 'commit'>[]): number {
  return marks ? tags.filter((tag) => markOf(marks, tag) === 'notPushed').length : 0;
}

/** What follows the count of the Tags list while the remote checked lacks some tags: `· 2 not on origin`. */
export function notPushedWords(marks: TagMarks | null | undefined, tags: Pick<Tag, 'name' | 'commit'>[]): string | null {
  const count = notPushedCount(marks, tags);
  return marks && count > 0 ? `· ${count} not on ${marks.remote}` : null;
}

/** The marks after an operation changed the tags on a remote: those of another remote than the one checked stay. */
export function followMarks(marks: TagMarks, changed: TagsOnRemote, tags: Pick<Tag, 'name' | 'commit'>[]): TagMarks {
  if (changed.remote !== null && changed.remote !== marks.remote) return marks;
  const set = new Map(marks.tags.map((mark) => [mark.name, mark]));
  for (const name of changed.pushed) set.set(name, { name, onRemote: 'pushed', commit: tags.find((tag) => tag.name === name)?.commit ?? null });
  for (const name of changed.notPushed) set.set(name, { name, onRemote: 'notPushed', commit: null });
  return { remote: marks.remote, tags: [...set.values()] };
}

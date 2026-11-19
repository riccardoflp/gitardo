/**
 * The marks of the tags a remote lacks (`tag.remote-marks`) and the entries of the **…** of the Tags list
 * (`tag.push-all`), from marks made by hand.
 */
import { describe, expect, it, vi } from 'vitest';
import type { MenuEntry } from '../src/webview/ContextMenu';
import { checkedMarks, followMarks, markOf, notPushedWords, remoteMark, type TagMarks } from '../src/shared/tagMarks';

// The menus post what is chosen to the extension; here they are only read.
const posted = vi.hoisted(() => [] as unknown[]);
vi.mock('../src/webview/vscode', () => ({ post: (message: unknown) => posted.push(message) }));
const { remoteTagsEntries } = await import('../src/webview/menus');

const tags = [
  { name: 'v1', commit: 'aaa', date: 0 },
  { name: 'v2', commit: 'bbb', date: 0 },
];

/** A check of `origin`, which has `v1` as here and lacks `v2`. */
const checked = (): TagMarks =>
  checkedMarks(
    {
      remote: 'origin',
      tags: [
        { name: 'v1', onRemote: 'pushed' },
        { name: 'v2', onRemote: 'notPushed' },
      ],
      onlyThere: [],
    },
    tags,
  );

const labels = (entries: MenuEntry[]) => entries.flatMap((entry) => (entry && entry !== 'separator' ? [entry.label] : []));

describe('tag.remote-marks', () => {
  it('marks nothing before a check, then the tag not on origin alone, and counts it after the count of the list', () => {
    expect(remoteMark(null, tags[1])).toBeNull();
    expect(notPushedWords(null, tags)).toBeNull();
    const marks = checked();
    expect(remoteMark(marks, tags[0])).toBeNull();
    expect(remoteMark(marks, tags[1])).toEqual({ icon: 'cloud-upload', title: 'Not on origin' });
    expect(notPushedWords(marks, tags)).toBe('· 1 not on origin');
  });

  it('a tag made after the check is not pushed, and one moved after it was pushed is different there', () => {
    const marks = checked();
    expect(markOf(marks, { name: 'v3', commit: 'ccc' })).toBe('notPushed');
    expect(remoteMark(marks, { name: 'v1', commit: 'ddd' })).toEqual({ icon: 'warning', title: 'origin has another tag of this name' });
  });

  it('the operations of Gitardo change the marks of the remote checked alone', () => {
    const marks = checked();
    expect(followMarks(marks, { remote: 'upstream', pushed: ['v2'], notPushed: [] }, tags)).toBe(marks);
    const pushed = followMarks(marks, { remote: 'origin', pushed: ['v2'], notPushed: [] }, tags);
    expect(markOf(pushed, tags[1])).toBe('pushed');
    const deleted = followMarks(pushed, { remote: 'origin', pushed: [], notPushed: ['v1'] }, tags);
    expect(markOf(deleted, tags[0])).toBe('notPushed');
  });

  it('the … of the Tags list checks origin, and offers Push tags… before a check and while a tag is not on it', () => {
    expect(labels(remoteTagsEntries({ remotes: [], tags, tagMarks: null }))).toEqual([]);
    const remotes = ['upstream', 'origin'];
    expect(labels(remoteTagsEntries({ remotes, tags, tagMarks: null }))).toEqual(['Check tags on origin', 'Push tags…']);
    expect(labels(remoteTagsEntries({ remotes, tags, tagMarks: checked() }))).toEqual(['Check tags on origin', 'Push tags…']);
    const every = followMarks(checked(), { remote: 'origin', pushed: ['v2'], notPushed: [] }, tags);
    expect(labels(remoteTagsEntries({ remotes, tags, tagMarks: every }))).toEqual(['Check tags on origin']);
  });
});

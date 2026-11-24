/**
 * What the extension shows of the files stored with Git LFS, from a given state and without Git LFS (milestone M47):
 * the text of a side not downloaded, the head and the counts of the **Large files** tab, and the menus of its patterns
 * and its files.
 */
import { describe, expect, it } from 'vitest';
import {
  type LargeFilesInfo,
  type LfsFile,
  type LfsLock,
  type LfsStatus,
  downloadAllText,
  fileEntries,
  filesText,
  locksNote,
  notDownloadedMessage,
  notDownloadedText,
  ownerText,
  patternEntries,
  showsNotSetUp,
  versionText,
} from '../src/shared/lfs';

const installed: LfsStatus = { version: '3.7.1', isSetUp: true, patterns: [{ pattern: '*.psd', file: '.gitattributes', line: 1 }] };
const MB = 1024 * 1024;

const file = (path: string, size: number, downloaded: boolean): LfsFile => ({
  path,
  size,
  oid: 'b'.repeat(64),
  isDownloaded: downloaded,
  isCheckedOut: downloaded,
});

const lock = (path: string, isOurs: boolean | null): LfsLock => ({ id: '7', path, owner: 'reviewer', lockedAt: '2026-11-24T10:00:00Z', isOurs });

const labels = (entries: ReturnType<typeof fileEntries>) => entries.map((entry) => (entry === 'separator' ? '—' : entry.label));

describe('A version stored with Git LFS and not downloaded', () => {
  it('lfs.show-in-pages: opens as a text that says so, with its size and the first digits of its id, never its pointer', () => {
    const text = notDownloadedText('art/logo.psd', { size: 2 * MB, oid: 'a'.repeat(64) });
    expect(text.split('\n').slice(0, 4)).toEqual(['Stored with Git LFS, not downloaded', '', 'art/logo.psd', `2.0 MB · ${'a'.repeat(10)}`]);
    expect(text).not.toContain('oid sha256');
    expect(notDownloadedMessage('art/logo.psd', 2 * MB)).toBe('art/logo.psd is stored with Git LFS and not downloaded (2.0 MB).');
  });
});

describe('The Large files tab', () => {
  it('lfs.page: two files, one not downloaded, give their count and size and Download all (1, 2.0 MB)', () => {
    const files = [file('art/logo.psd', 2 * MB, false), file('art/icon.psd', MB, true)];
    expect(filesText(files)).toBe('2 files, 3.0 MB');
    expect(downloadAllText(files)).toBe('Download all (1, 2.0 MB)');
    expect(downloadAllText([files[1]])).toBe('Download all');
    expect(filesText([files[1]])).toBe('1 file, 1.0 MB');
  });

  it('lfs.page: the head says the version, or that Git LFS is not installed, and not set up only with patterns', () => {
    expect(versionText('3.7.1')).toBe('Git LFS 3.7.1');
    expect(versionText(null)).toBe('Git LFS is not installed');
    expect(showsNotSetUp({ ...installed, isSetUp: false })).toBe(true);
    expect(showsNotSetUp({ ...installed, isSetUp: false, patterns: [] })).toBe(false);
    expect(showsNotSetUp({ ...installed, version: null, isSetUp: false })).toBe(false);
  });

  it('lfs.untrack: a pattern of the .gitattributes at the top can stop being stored; one of another file only opens it', () => {
    expect(labels(patternEntries(installed.patterns[0], installed))).toEqual([
      'Stop storing with Git LFS',
      'Store them with Git LFS in the next commit',
      '—',
      'Open .gitattributes',
    ]);
    expect(labels(patternEntries({ pattern: '*.png', file: 'assets/.gitattributes', line: 2 }, installed))).toEqual(['Open assets/.gitattributes']);
    expect(labels(patternEntries(installed.patterns[0], { ...installed, version: null }))).toEqual(['Open .gitattributes']);
  });

  it('lfs.read-locks: the note says the locks are read, that the server has none, or what git said', () => {
    expect(locksNote({ kind: 'reading' }, 'origin')).toBe('Reading the locks…');
    expect(locksNote({ kind: 'unsupported', message: 'locking API not supported' }, 'origin')).toBe('The server of origin does not support locks.');
    expect(locksNote({ kind: 'failed', message: 'no remote' }, null)).toBe('The locks could not be read: no remote');
    expect(locksNote({ kind: 'read', locks: [] }, 'origin')).toBeNull();
  });

  it('lfs.lock, lfs.unlock: a file offers Download when it is not, Lock while the server gave its locks, Unlock once locked', () => {
    const info: Pick<LargeFilesInfo, 'status' | 'locks'> = { status: installed, locks: { kind: 'read', locks: [lock('art/logo.psd', false)] } };
    expect(labels(fileEntries(file('art/icon.psd', MB, false), info))).toEqual(['Download', 'Lock', '—', 'Open file', 'Reveal in Explorer View']);
    const locked = fileEntries(file('art/logo.psd', MB, true), info);
    expect(labels(locked)).toEqual(['Unlock', '—', 'Open file', 'Reveal in Explorer View']);
    expect(locked[0]).toMatchObject({ ask: { kind: 'unlock', lock: { id: '7', owner: 'reviewer' } } });
    expect(labels(fileEntries(file('a.psd', MB, true), { status: installed, locks: { kind: 'unsupported', message: '' } }))).toEqual([
      'Open file',
      'Reveal in Explorer View',
    ]);
    expect(labels(fileEntries(file('a.psd', MB, false), { status: { ...installed, version: null }, locks: null }))).toEqual([
      'Open file',
      'Reveal in Explorer View',
    ]);
  });

  it('lfs.read-locks: a lock of this user is said You, someone else’s by their name', () => {
    expect(ownerText(lock('a.psd', true))).toBe('You');
    expect(ownerText(lock('a.psd', false))).toBe('reviewer');
    expect(ownerText(lock('a.psd', null))).toBe('reviewer');
  });
});

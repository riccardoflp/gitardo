/**
 * Files a repository stores with Git LFS (milestone M47, `docs/features/21-large-files.md`): what the engine reads of
 * them, the text a side of a diff opens as when its content is not downloaded (`lfs.show-in-pages`), and what the
 * **Large files** tab shows and offers (`lfs.page`). Nothing here runs git: the engine reads and runs everything.
 */
import { sizeText } from './worktreeSetup';

/** A pattern of a file of attributes whose attributes have `filter=lfs`. */
export interface LfsPattern {
  /** The pattern: `*.psd`. */
  pattern: string;
  /** The file of attributes it is in, from the top of the working tree: `.gitattributes`. */
  file: string;
  /** Its line in that file, from 1. */
  line: number;
}

/** Whether Git LFS is installed and set up for a repository, and the patterns of the files it stores with it. */
export interface LfsStatus {
  /** The version of Git LFS; null when it is not installed, or, in the state, when nothing asked for it yet. */
  version: string | null;
  isSetUp: boolean;
  patterns: LfsPattern[];
}

/** A file of the commit checked out stored with Git LFS. */
export interface LfsFile {
  path: string;
  /** The size of its content, in bytes. */
  size: number;
  oid: string;
  /** Whether its content is in the store of the repository. */
  isDownloaded: boolean;
  /** Whether the file on disk is its content rather than its pointer. */
  isCheckedOut: boolean;
}

/** A lock of a file, held on the server. */
export interface LfsLock {
  id: string;
  path: string;
  /** The name of whoever holds it, as the server says it. */
  owner: string;
  lockedAt: string;
  /** Whether this user holds it; null when the server did not say whose it is. */
  isOurs: boolean | null;
}

/** The locks a server gave, or that it does not support them. */
export type LfsLocks = { kind: 'read'; locks: LfsLock[] } | { kind: 'unsupported'; message: string };

/** The locks as the tab knows them: being read, read, or what git said when they could not be. */
export type LocksShown = { kind: 'reading' } | { kind: 'failed'; message: string } | LfsLocks;

/** What the **Large files** tab of a repository shows. */
export interface LargeFilesInfo {
  /** The name of the repository. */
  name: string;
  status: LfsStatus;
  /** The files, none without Git LFS. */
  files: LfsFile[];
  /** The locks, none without Git LFS. */
  locks: LocksShown | null;
  /** The remote whose server the locks are asked of, as the engine chooses it; null when there is none. */
  remote: string | null;
}

/** What the tab asks of the extension. */
export type LargeFilesAsk =
  | { kind: 'refresh' }
  | { kind: 'setUp' }
  | { kind: 'track' }
  | { kind: 'untrack'; pattern: string }
  | { kind: 'storeCommitted' }
  | { kind: 'openAttributes'; file: string }
  | { kind: 'download'; paths: string[] }
  | { kind: 'downloadAll' }
  | { kind: 'lock'; path: string }
  | { kind: 'unlock'; lock: LfsLock }
  | { kind: 'openFile'; path: string }
  | { kind: 'revealFile'; path: string }
  | { kind: 'openSite' };

/** The words the extension says of Git LFS, those of the desktop (`views::text::lfs`). */
export const LFS_WORDS = {
  storedNotDownloaded: 'Stored with Git LFS, not downloaded',
  download: 'Download',
  downloaded: 'Downloaded',
  notDownloaded: 'Not downloaded',
  notInstalled: 'Git LFS is not installed',
  openSite: 'Open git-lfs.com',
  notSetUp: 'Git LFS is not set up in this repository: its files stay pointers.',
  setUp: 'Set up',
  patterns: 'Patterns',
  track: 'Track a pattern…',
  stopStoring: 'Stop storing with Git LFS',
  storeCommitted: 'Store them with Git LFS in the next commit',
  files: 'Files',
  refresh: 'Refresh',
  openFile: 'Open file',
  revealFile: 'Reveal in Explorer View',
  nothingStored: 'This repository stores no file with Git LFS.',
  lock: 'Lock',
  unlock: 'Unlock',
  readingLocks: 'Reading the locks…',
  you: 'You',
} as const;

/** The site of Git LFS, which **Open git-lfs.com** opens. */
export const LFS_SITE = 'https://git-lfs.com';

/** How many digits of the id of a content are shown. */
const SHORT_OID = 10;

/** Whether the content of a file is on this computer and in the working tree, which **Download all** leaves alone. */
export const isDownloaded = (file: LfsFile): boolean => file.isDownloaded && file.isCheckedOut;

/**
 * The text a version of a file opens as when it is stored with Git LFS and its content is not on this computer, in
 * place of the lines of its pointer: what it is, its size and the first digits of its id, and how to download it.
 */
export function notDownloadedText(path: string, stored: { size: number; oid: string }): string {
  return [
    `${LFS_WORDS.storedNotDownloaded}`,
    '',
    `${path}`,
    `${sizeText(stored.size)} · ${stored.oid.slice(0, SHORT_OID)}`,
    '',
    'Download it with Download in the message of VS Code, or from the Large files tab.',
    '',
  ].join('\n');
}

/** The message of VS Code that offers **Download** for a version not downloaded. */
export function notDownloadedMessage(path: string, size: number): string {
  return `${path} is stored with Git LFS and not downloaded (${sizeText(size)}).`;
}

/** The head of the tab: **Git LFS 3.7.1**, or that it is not installed. */
export const versionText = (version: string | null): string => (version ? `Git LFS ${version}` : LFS_WORDS.notInstalled);

/** The count of the files and their size: **2 files, 4.1 MB**. */
export function filesText(files: readonly LfsFile[]): string {
  const size = files.reduce((sum, file) => sum + file.size, 0);
  return `${files.length} ${files.length === 1 ? 'file' : 'files'}, ${sizeText(size)}`;
}

/** **Download all** with the count and the size of the files not downloaded: **Download all (1, 2.0 MB)**. */
export function downloadAllText(files: readonly LfsFile[]): string {
  const missing = files.filter((file) => !isDownloaded(file));
  if (!missing.length) return 'Download all';
  return `Download all (${missing.length}, ${sizeText(missing.reduce((sum, file) => sum + file.size, 0))})`;
}

/** Whether the tab says the repository is not set up: it has patterns, Git LFS is there, and it is not set up. */
export const showsNotSetUp = (status: LfsStatus): boolean => !!status.version && status.patterns.length > 0 && !status.isSetUp;

/** The lock of a file, when the server gave one. */
export function lockOf(locks: LocksShown | null, path: string): LfsLock | undefined {
  return locks?.kind === 'read' ? locks.locks.find((lock) => lock.path === path) : undefined;
}

/** Who holds a lock, as the tab says it: **You** for this user's when the server verifies. */
export const ownerText = (lock: LfsLock): string => (lock.isOurs ? LFS_WORDS.you : lock.owner);

/** The note under **Files** about the locks: being read, a server without them, or what git said. */
export function locksNote(locks: LocksShown | null, remote: string | null): string | null {
  if (!locks) return null;
  if (locks.kind === 'reading') return LFS_WORDS.readingLocks;
  if (locks.kind === 'unsupported') return `The server of ${remote ?? 'this repository'} does not support locks.`;
  if (locks.kind === 'failed') return `The locks could not be read: ${locks.message}`;
  return null;
}

/** An entry of a menu of the tab: its label and what it asks. */
export interface LargeFilesEntry {
  label: string;
  ask: LargeFilesAsk;
}

/**
 * The menu of a pattern: **Stop storing with Git LFS** and **Store them with Git LFS in the next commit** for a pattern
 * of the `.gitattributes` at the top, which Git LFS writes, then **Open `<file>`** (`lfs.untrack`, `lfs.store-committed`).
 */
export function patternEntries(pattern: LfsPattern, status: LfsStatus): (LargeFilesEntry | 'separator')[] {
  const open: LargeFilesEntry = { label: `Open ${pattern.file}`, ask: { kind: 'openAttributes', file: pattern.file } };
  if (pattern.file !== '.gitattributes' || !status.version) return [open];
  return [
    { label: LFS_WORDS.stopStoring, ask: { kind: 'untrack', pattern: pattern.pattern } },
    { label: LFS_WORDS.storeCommitted, ask: { kind: 'storeCommitted' } },
    'separator',
    open,
  ];
}

/**
 * The menu of a file: **Download** when it is not, **Lock** or **Unlock** while the server gave its locks, then
 * **Open file** and **Reveal in Explorer View** (`lfs.download`, `lfs.lock`, `lfs.unlock`).
 */
export function fileEntries(file: LfsFile, info: Pick<LargeFilesInfo, 'status' | 'locks'>): (LargeFilesEntry | 'separator')[] {
  const entries: (LargeFilesEntry | 'separator')[] = [];
  const installed = !!info.status.version;
  if (installed && !isDownloaded(file)) entries.push({ label: LFS_WORDS.download, ask: { kind: 'download', paths: [file.path] } });
  const lock = lockOf(info.locks, file.path);
  if (lock) entries.push({ label: LFS_WORDS.unlock, ask: { kind: 'unlock', lock } });
  else if (installed && info.locks?.kind === 'read') entries.push({ label: LFS_WORDS.lock, ask: { kind: 'lock', path: file.path } });
  if (entries.length) entries.push('separator');
  entries.push({ label: LFS_WORDS.openFile, ask: { kind: 'openFile', path: file.path } });
  entries.push({ label: LFS_WORDS.revealFile, ask: { kind: 'revealFile', path: file.path } });
  return entries;
}

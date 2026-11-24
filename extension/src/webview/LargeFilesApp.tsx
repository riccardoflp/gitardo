import { useEffect, useState, type MouseEvent } from 'react';
import {
  LFS_WORDS,
  downloadAllText,
  fileEntries,
  filesText,
  isDownloaded,
  lockOf,
  locksNote,
  ownerText,
  patternEntries,
  showsNotSetUp,
  versionText,
  type LargeFilesAsk,
  type LargeFilesEntry,
  type LargeFilesInfo,
} from '../shared/lfs';
import { sizeText } from '../shared/worktreeSetup';
import { type MenuEntry, useMenu } from './ContextMenu';
import type { Meaning } from './menuIcons';
import { EntryIcon } from './FileIcon';
import { Icon } from './Icon';
import { onMessage, post } from './vscode';

/** Sends the extension what the tab asks. */
const ask = (asked: LargeFilesAsk) => post({ type: 'largeFiles', ask: asked });

/** The meaning of the icon of each entry of the menus of the tab, as the desktop's page draws it; none for a lock, which no icon says (`ui.menu-icons`). */
const ENTRY_ICONS: Partial<Record<LargeFilesAsk['kind'], Meaning>> = {
  untrack: 'DELETE',
  storeCommitted: 'ADD',
  openAttributes: 'OPEN_FILE',
  download: 'PULL',
  openFile: 'OPEN_FILE',
  revealFile: 'FILES',
};

/** The entries of a menu of the tab as the context menu takes them. */
const menuOf = (entries: (LargeFilesEntry | 'separator')[]): MenuEntry[] =>
  entries.map((entry) => (entry === 'separator' ? entry : { label: entry.label, icon: ENTRY_ICONS[entry.ask.kind], run: () => ask(entry.ask) }));

/**
 * The **Large files** tab of a repository (`lfs.page`): whether Git LFS is there and set up, the patterns of the files
 * it stores with it, the files of the commit checked out with their sizes, whether each is downloaded and who locked it,
 * and what each offers in its menu. The extension reads everything with the engine and sends it here.
 */
export function LargeFilesApp() {
  // Undefined while it is read.
  const [info, setInfo] = useState<LargeFilesInfo | null>();
  const [error, setError] = useState<string>();
  const openMenu = useMenu();

  useEffect(() => {
    const stop = onMessage((message) => {
      if (message.type !== 'largeFiles') return;
      setInfo(message.info);
      setError(message.error);
    });
    post({ type: 'getLargeFiles' });
    return stop;
  }, []);

  if (info === undefined) return <div className="empty">Reading the files stored with Git LFS…</div>;
  if (info === null) return <div className="empty">Gitardo could not read the files stored with Git LFS. {error}</div>;

  const { status, files, locks } = info;
  const installed = !!status.version;
  const note = locksNote(locks, info.remote);
  const missing = files.filter((file) => !isDownloaded(file));
  const menu = (entries: (LargeFilesEntry | 'separator')[]) => (event: MouseEvent) => {
    event.preventDefault();
    openMenu(event, menuOf(entries));
  };

  return (
    <main className="setup-page lfs-page">
      <header>
        <h1>Large files</h1>
        <p className="lfs-head">
          <span>{versionText(status.version)}</span>
          {!installed && (
            <button className="button" onClick={() => ask({ kind: 'openSite' })}>
              <Icon name="link-external" />
              {LFS_WORDS.openSite}
            </button>
          )}
        </p>
        {showsNotSetUp(status) && (
          <p className="note">
            <Icon name="warning" />
            {LFS_WORDS.notSetUp}
            <button className="button" onClick={() => ask({ kind: 'setUp' })}>
              {LFS_WORDS.setUp}
            </button>
          </p>
        )}
      </header>

      {!status.patterns.length && !files.length ? (
        <section aria-label={LFS_WORDS.patterns}>
          <p className="muted lfs-nothing">
            {LFS_WORDS.nothingStored}
            <button className="button" disabled={!installed} onClick={() => ask({ kind: 'track' })}>
              <Icon name="add" />
              {LFS_WORDS.track}
            </button>
          </p>
        </section>
      ) : (
        <>
          <section aria-label={LFS_WORDS.patterns}>
            <h2>
              <span className="grow">
                {LFS_WORDS.patterns} <span className="count">{status.patterns.length}</span>
              </span>
              <button className="button" disabled={!installed} onClick={() => ask({ kind: 'track' })}>
                <Icon name="add" />
                {LFS_WORDS.track}
              </button>
            </h2>
            <ul className="lfs-rows" role="list">
              {status.patterns.map((pattern) => (
                <li
                  key={`${pattern.file}:${pattern.line}`}
                  className="lfs-row"
                  tabIndex={0}
                  onContextMenu={menu(patternEntries(pattern, status))}
                  title={`${pattern.file}, line ${pattern.line}`}
                >
                  <span className="mono grow">{pattern.pattern}</span>
                  <span className="muted">{pattern.file}</span>
                  <button className="button icon" aria-label={`More for ${pattern.pattern}`} onClick={menu(patternEntries(pattern, status))}>
                    <Icon name="ellipsis" />
                  </button>
                </li>
              ))}
            </ul>
          </section>

          <section aria-label={LFS_WORDS.files}>
            <h2>
              <span className="grow">
                {LFS_WORDS.files} <span className="muted lfs-summary">{filesText(files)}</span>
              </span>
              <button className="button" disabled={!installed || !missing.length} onClick={() => ask({ kind: 'downloadAll' })}>
                <Icon name="cloud-download" />
                {downloadAllText(files)}
              </button>
              <button className="button" onClick={() => ask({ kind: 'refresh' })}>
                <Icon name="refresh" />
                {LFS_WORDS.refresh}
              </button>
            </h2>
            {note && <p className="muted lfs-locks-note">{note}</p>}
            {!installed && <p className="muted">Without Git LFS, the files it stores are not listed.</p>}
            <ul className="lfs-rows" role="list">
              {files.map((file) => {
                const lock = lockOf(locks, file.path);
                const entries = fileEntries(file, info);
                const downloaded = isDownloaded(file);
                return (
                  <li
                    key={file.path}
                    className="lfs-row"
                    tabIndex={0}
                    onContextMenu={menu(entries)}
                    onDoubleClick={() => ask({ kind: 'openFile', path: file.path })}
                  >
                    <EntryIcon path={file.path} folder={false} />
                    <span className="grow lfs-path">{file.path}</span>
                    <span className="muted lfs-owner" title={lock ? `Locked by ${lock.owner}` : undefined}>
                      {lock && (
                        <>
                          <Icon name="lock" /> {ownerText(lock)}
                        </>
                      )}
                    </span>
                    <span className="muted lfs-size">{sizeText(file.size)}</span>
                    <span className={downloaded ? 'muted lfs-state' : 'note lfs-state'}>{downloaded ? LFS_WORDS.downloaded : LFS_WORDS.notDownloaded}</span>
                    <button className="button icon" aria-label={`More for ${file.path}`} onClick={menu(entries)}>
                      <Icon name="ellipsis" />
                    </button>
                  </li>
                );
              })}
            </ul>
          </section>
        </>
      )}
    </main>
  );
}

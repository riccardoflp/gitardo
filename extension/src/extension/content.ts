import * as vscode from 'vscode';
import { reads } from '../engine/reads';

export const SCHEME = 'gitardo';

/** Stands for the staged version of a file: `git show :0:<path>`. */
export const INDEX = ':0';

/** The three sides of a file in conflict, as the index keeps them: `git show :1:<path>` and so on. */
export const CONFLICT = { base: ':1', ours: ':2', theirs: ':3' } as const;

/** Stands for what Gitardo proposes to write in a file, shown beside what the file has before anything is written. */
const PROPOSAL = 'proposal';
const proposals = new Map<string, string>();

/** URI of the text proposed for a file. The latest proposal for a file is the one kept. */
export function proposalUri(root: string, file: string, text: string): vscode.Uri {
  proposals.set(`${root}\n${file}`, text);
  return vscode.Uri.from({ scheme: SCHEME, path: `/${file}`, query: JSON.stringify({ root, ref: PROPOSAL, at: Date.now() }) });
}

/**
 * URI of a file as it was at `ref`. An empty ref stands for "the file does not exist on this side of the diff".
 * The index changes over time, so its URIs carry a timestamp to keep VS Code from showing a cached copy.
 */
export function contentUri(root: string, ref: string, file: string): vscode.Uri {
  const query = ref.startsWith(':') ? { root, ref, at: Date.now() } : { root, ref };
  return vscode.Uri.from({ scheme: SCHEME, path: `/${file}`, query: JSON.stringify(query) });
}

/**
 * URI of a file as it was in a commit, for an editor of its own rather than a side of a diff: its tab says which
 * commit, before the extension that tells VS Code what language the file is in.
 */
export function revisionUri(root: string, hash: string, file: string): vscode.Uri {
  const name = file.slice(file.lastIndexOf('/') + 1);
  const dot = name.lastIndexOf('.');
  const short = hash.slice(0, 7);
  const label = dot > 0 ? `${name.slice(0, dot)} (${short})${name.slice(dot)}` : `${name} (${short})`;
  const directory = file.slice(0, file.length - name.length);
  return vscode.Uri.from({ scheme: SCHEME, path: `/${directory}${label}`, query: JSON.stringify({ root, ref: hash, file }) });
}

/** What a URI of a file as it was in a commit stands for, or nothing for one of the index or of a proposal. */
export function revisionOf(uri: vscode.Uri): { root: string; revision: string; file: string } | undefined {
  if (uri.scheme !== SCHEME) return undefined;
  try {
    const { root, ref, file } = JSON.parse(uri.query) as { root: string; ref: string; file?: string };
    return /^[0-9a-f]{7,64}$/.test(ref) ? { root, revision: ref, file: file ?? uri.path.slice(1) } : undefined;
  } catch {
    return undefined;
  }
}

/** Whether a URI stands for the side of a diff that has no file. */
export function isAbsent(uri: vscode.Uri): boolean {
  if (uri.scheme !== SCHEME) return false;
  try {
    return !(JSON.parse(uri.query) as { ref?: string }).ref;
  } catch {
    return false;
  }
}

/** A version of a file stored with Git LFS whose content is not on this computer, as a tab opened it. */
export interface NotDownloaded {
  root: string;
  /** The file, from the top of the working tree. */
  path: string;
  /** The commit of the version, by its full id; null for the index, whose file **Download** pulls into the working tree. */
  commit: string | null;
  /** The size of its content, in bytes. */
  size: number;
}

/**
 * The files of the scheme `gitardo` as bytes, which can only be read: VS Code opens an image there in its preview of
 * images, and the two sides of a diff of an image in its diff editor as two pictures; a text file opens as before
 * (`file-view.image`, `diff.image`).
 */
export class GitFileSystem implements vscode.FileSystemProvider {
  private readonly changed = new vscode.EventEmitter<vscode.FileChangeEvent[]>();
  readonly onDidChangeFile = this.changed.event;
  /** The versions said not downloaded, by their URIs, which are read again once a download ran. */
  private readonly notDownloaded = new Map<string, vscode.Uri>();

  /** `said` is told of each version stored with Git LFS whose content is not downloaded, once until it is read again. */
  constructor(private readonly said: (side: NotDownloaded) => void = () => undefined) {}

  async readFile(uri: vscode.Uri): Promise<Uint8Array> {
    const { root, ref, file } = JSON.parse(uri.query) as { root: string; ref: string; file?: string };
    if (!ref) return new Uint8Array();
    if (ref === PROPOSAL) return Buffer.from(proposals.get(`${root}\n${uri.path.slice(1)}`) ?? '', 'utf8');
    // The path is the name of the file, but for the URIs whose tab says more than the name.
    const path = file ?? uri.path.slice(1);
    const shown = await reads.fileShownAt(root, ref, path);
    const key = uri.toString();
    if (shown.stored && !shown.stored.isDownloaded) {
      // `stat` reads the file too: the message is said once for a version opened.
      if (!this.notDownloaded.has(key)) this.said({ root, path, commit: shown.commit, size: shown.stored.size });
      this.notDownloaded.set(key, uri);
    } else this.notDownloaded.delete(key);
    return shown.bytes ?? new Uint8Array();
  }

  /** Has VS Code read again the versions of a repository said not downloaded, once **Download** ran there. */
  readAgain(root: string): void {
    const again = [...this.notDownloaded.values()].filter((uri) => (JSON.parse(uri.query) as { root: string }).root === root);
    for (const uri of again) this.notDownloaded.delete(uri.toString());
    if (again.length) this.changed.fire(again.map((uri) => ({ type: vscode.FileChangeType.Changed, uri })));
  }

  async stat(uri: vscode.Uri): Promise<vscode.FileStat> {
    const { at } = JSON.parse(uri.query) as { at?: number };
    const size = (await this.readFile(uri)).length;
    return { type: vscode.FileType.File, ctime: 0, mtime: at ?? 0, size, permissions: vscode.FilePermission.Readonly };
  }

  watch(): vscode.Disposable {
    // A version of a commit never changes; the index and a proposal carry the time they were asked at.
    return new vscode.Disposable(() => undefined);
  }

  readDirectory(): never {
    throw vscode.FileSystemError.FileNotADirectory();
  }

  createDirectory(): never {
    throw vscode.FileSystemError.NoPermissions();
  }

  writeFile(): never {
    throw vscode.FileSystemError.NoPermissions();
  }

  delete(): never {
    throw vscode.FileSystemError.NoPermissions();
  }

  rename(): never {
    throw vscode.FileSystemError.NoPermissions();
  }
}

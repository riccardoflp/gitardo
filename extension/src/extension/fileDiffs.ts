/**
 * The diffs and the files a view opens in an editor of VS Code: a file of a commit, of a comparison or of the changes
 * beside the version it is compared with, several in one editor, a file as it is in the working tree, the two versions
 * of an image sent to a page, and a file in conflict where it is resolved by hand.
 */
import * as path from 'node:path';
import * as vscode from 'vscode';
import { openMergeEditor } from '../flow/merge';
import { type Repository, reads } from '../engine/reads';
import { CONFLICT_SIDES } from '../shared/conflicts';
import { imagePair } from '../shared/images';
import { openingOf } from '../shared/markdown';
import { type DiffSource, type FileChange, WORKTREE } from '../shared/protocol';
import { AS_GIT_MARKED_IT, MARKED, resolvedAsBefore } from '../shared/rerere';
import { inVSCodeGit, refreshVSCodeGit } from './actions';
import { INDEX, contentUri, isAbsent } from './content';
import type { ControllerHost } from './controllerHost';
import { errorText } from './repositorySession';
import { isSide } from './revisions';

/** The diffs VS Code's Git serves the sides of, once it has the repository. */
const WORKING_SOURCES = new Set<DiffSource['kind']>(['staged', 'unstaged', 'change']);

/** Opens the diffs and the files the views ask for, in the repository given, or else in the one the panel shows. */
export class FileDiffs {
  constructor(private readonly host: Pick<ControllerHost, 'followers' | 'heldFor' | 'log' | 'post'>) {}

  /** Opens a file in conflict where it is resolved by hand: the Flow Merge, the merge editor of VS Code, or the file itself at a line. */
  async openConflict(repo: Repository, file: string, line: number, editor: 'flowMerge' | 'mergeEditor' | 'text'): Promise<void> {
    if (typeof file !== 'string' || !(await reads.conflicted(repo)).includes(file)) return;
    const uri = vscode.Uri.file(path.join(repo.root, file));
    try {
      if (editor === 'flowMerge') return void (await vscode.commands.executeCommand('gitardo.openInFlowMerge', uri));
      const stages = await reads.conflictStages(repo, file);
      // The merge editor needs all three sides: a file both sides added has no base.
      if (editor === 'mergeEditor' && [1, 2, 3].every((stage) => stages.includes(stage))) {
        return await openMergeEditor(repo.root, file, await reads.operation(repo).catch(() => null));
      }
      if (editor !== 'text' && (await this.openDeletedConflict(repo, file, stages))) return;
    } catch (error) {
      this.host.log.warn(`Could not open ${file} to resolve it: ${errorText(error)}`);
    }
    const at = Math.max(0, (Number(line) || 1) - 1);
    await vscode.window.showTextDocument(uri, { selection: new vscode.Range(at, 0, at, 0), preview: false });
  }

  /**
   * A file one side deleted and the other changed, as what it was beside what the side that kept it made of it. False
   * when the file is not one of those.
   */
  private async openDeletedConflict(repo: Repository, file: string, stages: number[]): Promise<boolean> {
    if (!stages.includes(1) || stages.includes(2) === stages.includes(3)) return false;
    const [current, incoming] = CONFLICT_SIDES[(await reads.operation(repo).catch(() => null)) ?? 'none'];
    const [deleter, keeper] = stages.includes(2) ? [incoming, current] : [current, incoming];
    const lower = (label: string) => `${label[0].toLowerCase()}${label.slice(1)}`;
    const title = `${path.posix.basename(file)} (deleted by ${lower(deleter)}, changed by ${lower(keeper)})`;
    await vscode.commands.executeCommand('vscode.diff', contentUri(repo.root, ':1', file), vscode.Uri.file(path.join(repo.root, file)), title);
    return true;
  }

  /** The two versions of a file that a diff of `source` puts side by side, and what to call the pair. */
  private sides(repo: Repository, source: DiffSource, file: FileChange): { left: vscode.Uri; right: vscode.Uri; label: string } | null {
    const added = file.kind === 'added';
    const deleted = file.kind === 'deleted';
    const before = file.originalPath ?? file.path;
    const held = this.host.heldFor(repo.root);
    const head = held?.head.commit ?? '';
    const onDisk = vscode.Uri.file(path.join(repo.root, file.path));
    const at = (ref: string, filePath = file.path) => contentUri(repo.root, ref, filePath);
    // The index and HEAD as VS Code's Git extension serves them, when it has the repository: its own commands, such as
    // Stage Selected Ranges, then know the diff, as they only do for its URIs. Gitardo serves them otherwise.
    const vscodeGit = (source.kind === 'staged' || source.kind === 'unstaged') && inVSCodeGit(repo.root);
    const inGit = (ref: '~' | 'HEAD', filePath = file.path) => {
      if (!vscodeGit) return at(ref === '~' ? INDEX : head, filePath);
      const uri = vscode.Uri.file(path.join(repo.root, filePath));
      return uri.with({ scheme: 'git', query: JSON.stringify({ path: uri.fsPath, ref }) });
    };

    switch (source.kind) {
      case 'commit': {
        // Either side of a comparison can be the working tree.
        const { hash, parent } = source;
        if (!isSide(hash) || (parent !== null && !isSide(parent)) || hash === parent) return null;
        const name = (ref: string | null) => (ref === WORKTREE ? 'Working Tree' : ref ? ref.slice(0, 7) : 'empty');
        const left = added || !parent ? at('', before) : parent === WORKTREE ? vscode.Uri.file(path.join(repo.root, before)) : at(parent, before);
        const right = deleted ? at('') : hash === WORKTREE ? onDisk : at(hash);
        return { left, right, label: `${name(parent)} ↔ ${name(hash)}` };
      }
      case 'worktree':
        return { left: at(added ? '' : head, before), right: deleted ? at('') : onDisk, label: 'Working Tree' };
      case 'staged':
        return { left: added ? at('') : inGit('HEAD', before), right: deleted ? at('') : inGit('~'), label: 'Index' };
      case 'unstaged':
        return { left: added ? at('') : inGit('~'), right: deleted ? at('') : onDisk, label: 'Working Tree' };
      case 'change': {
        // The file as git lists it on that side: added to the index and changed since is `A` there, `M` here.
        const working = held?.workingTree;
        // An untracked file is all in the working tree.
        const unstaged = working?.unstaged.find((each) => each.path === file.path) ?? (working?.untracked.includes(file.path) ? file : undefined);
        if (unstaged || !working) return this.sides(repo, { kind: 'unstaged' }, unstaged ?? file);
        return this.sides(repo, { kind: 'staged' }, working.staged.find((each) => each.path === file.path) ?? file);
      }
      case 'resolvedAsBefore':
        // What git wrote from a recorded resolution: beside the markers it would have left, or, staged, in the index.
        if (held && resolvedAsBefore(held, file.path)?.isStaged) return this.sides(repo, { kind: 'staged' }, file);
        return { left: at(MARKED), right: onDisk, label: `${AS_GIT_MARKED_IT} ↔ Working Tree` };
      case 'conflict':
        return null;
    }
  }

  /** Opens the diffs of several files in a single editor, one under the other. */
  async openChanges(title: string, source: DiffSource, files: FileChange[], repo = this.host.followers.repo): Promise<void> {
    if (!repo) return;
    if (WORKING_SOURCES.has(source.kind)) await refreshVSCodeGit(repo.root);
    const resources = files.flatMap((file) => {
      const sides = this.sides(repo, source, file);
      return sides ? [[vscode.Uri.file(path.join(repo.root, file.path)), sides.left, sides.right]] : [];
    });
    if (resources.length) await vscode.commands.executeCommand('vscode.changes', title, resources);
  }

  /**
   * A file of the working tree in its editor, or a Markdown file in VS Code's preview of Markdown while
   * `gitardo.markdown.openAsPreview` is on. A deleted one is not there to open: its diff shows what it was.
   */
  async openFile(file: FileChange, repo = this.host.followers.repo): Promise<void> {
    if (!repo) return;
    if (file.kind === 'deleted') return this.openDiff({ kind: 'change' }, file, repo);
    const uri = vscode.Uri.file(path.join(repo.root, file.path));
    const openAsPreview = vscode.workspace.getConfiguration('gitardo').get<boolean>('markdown.openAsPreview') ?? false;
    if (openingOf(file.path, openAsPreview) === 'preview') {
      try {
        return void (await vscode.commands.executeCommand('markdown.showPreview', uri));
      } catch (error) {
        // VS Code's Markdown extension may be turned off: the file still opens, in its editor.
        this.host.log.warn(`Could not open the preview of ${file.path}: ${errorText(error)}`);
      }
    }
    await vscode.commands.executeCommand('vscode.open', uri);
  }

  /**
   * Sends a page the two versions of an image file of a commit or of a comparison, as the diff editor would compare
   * them, each as a `data:` URI unless it is over 10 MB (`commit-page.images`).
   */
  async readImages(source: DiffSource, file: FileChange, webview: vscode.Webview, repo = this.host.followers.repo): Promise<void> {
    const sides = repo && source.kind === 'commit' ? this.sides(repo, source, file) : null;
    const read = async (uri: vscode.Uri, filePath: string) => {
      if (isAbsent(uri)) return null;
      try {
        return { bytes: await vscode.workspace.fs.readFile(uri), path: filePath };
      } catch {
        return null;
      }
    };
    const [before, after] = sides
      ? await Promise.all([read(sides.left, file.originalPath ?? file.path), read(sides.right, file.path)])
      : [null, null];
    const images = imagePair(before, after, (bytes) => Buffer.from(bytes).toString('base64'));
    return this.host.post({ type: 'images', source, path: file.path, images }, webview);
  }

  async openDiff(source: DiffSource, file: FileChange, repo = this.host.followers.repo): Promise<void> {
    if (!repo) return;
    const onDisk = vscode.Uri.file(path.join(repo.root, file.path));

    switch (source.kind) {
      case 'commit':
      case 'worktree':
      case 'staged':
      case 'unstaged':
      case 'change':
      case 'resolvedAsBefore': {
        if (WORKING_SOURCES.has(source.kind)) await refreshVSCodeGit(repo.root);
        const sides = this.sides(repo, source, file);
        if (!sides) return;
        const title = `${path.posix.basename(file.path)} (${sides.label})`;
        await vscode.commands.executeCommand('vscode.diff', sides.left, sides.right, title);
        return;
      }
      case 'conflict': {
        // The merge editor needs all three sides. A file one side deleted shows what the other changed of it; one both
        // sides added, or both deleted, opens as it is.
        const stages = await reads.conflictStages(repo, file.path);
        if ([1, 2, 3].every((stage) => stages.includes(stage))) {
          try {
            // Not part of the documented API, hence the fallback.
            await openMergeEditor(repo.root, file.path, this.host.heldFor(repo.root)?.operation ?? null);
            return;
          } catch (error) {
            this.host.log.warn(`Could not open the merge editor: ${errorText(error)}`);
          }
        }
        if (await this.openDeletedConflict(repo, file.path, stages)) return;
        await vscode.commands.executeCommand('vscode.open', onDisk);
        return;
      }
    }
  }
}

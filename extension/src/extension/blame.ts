import * as path from 'node:path';
import * as vscode from 'vscode';
import { type BlameAsked, type Repository, reads } from '../engine/reads';
import type { ShownBlameLine } from '../engine/shapes';
import { LineBlames } from '../shared/blameCache';
import { ago } from '../shared/blameFormat';
import { revisionOf } from './content';

const DELAY = 200;

/** Where a document is in a repository: the name the file has there, and the commit it is shown as it was in, if any. */
export interface Blamable {
  repo: Repository;
  file: string;
  revision?: string;
}

/** Whether a line is left to who wrote it when only its whitespace or its place in the file changed since. */
export const ignoresWhitespace = () => vscode.workspace.getConfiguration('gitardo').get<boolean>('blame.ignoreWhitespaceAndMoves') === true;

/** Finds the repository a file of the editor is in, by the engine. It works before any Gitardo view is opened. */
export class BlameSource {
  /** The repository of each folder a file was opened from, found once. */
  private readonly found = new Map<string, Promise<Repository | undefined>>();

  constructor(private readonly log: vscode.LogOutputChannel) {}

  private repoOf(file: string): Promise<Repository | undefined> {
    const folder = path.dirname(file);
    let repo = this.found.get(folder);
    if (!repo) {
      repo = reads.repositoryOf(file).then(
        (found) => (found ? { root: found.root } : undefined),
        (error: unknown) => {
          this.log.debug(`No repository found for ${folder}: ${error instanceof Error ? error.message : String(error)}`);
          return undefined;
        },
      );
      this.found.set(folder, repo);
    }
    return repo;
  }

  /** The repository of a document, when it is a file on disk inside one or a file as it was in a commit. */
  async of(document: vscode.TextDocument): Promise<Blamable | undefined> {
    const past = revisionOf(document.uri);
    if (past) return { repo: { root: past.root }, file: past.file, revision: past.revision };
    if (document.uri.scheme !== 'file') return undefined;
    const repo = await this.repoOf(document.uri.fsPath);
    if (!repo) return undefined;
    const file = path.relative(repo.root, document.uri.fsPath).replace(/\\/g, '/');
    return file.startsWith('..') || path.isAbsolute(file) ? undefined : { repo, file };
  }

  /**
   * What the engine is told to blame a document with: the text of the editor when it is not saved, and the commit it is
   * shown as it was in. Whitespace is left to the setting, which the engine reads.
   */
  options(document: vscode.TextDocument, found: Blamable): BlameAsked {
    return { contents: !found.revision && document.isDirty ? document.getText() : undefined, revision: found.revision };
  }
}

/** The document, its version and the line under the cursor of an editor, or nothing when more than one line is selected. */
const keyOf = (editor: vscode.TextEditor): string | undefined =>
  editor.selection.isSingleLine ? `${editor.document.uri.toString()}\n${editor.document.version}\n${editor.selection.active.line}` : undefined;

/**
 * Shows who last changed the line under the cursor, dimmed at the end of that line. The blame of a line is kept for
 * the version of the document and the line, and git is asked again only when either changes, or when the commits
 * under it may have (`refresh`).
 */
export class BlameAnnotator implements vscode.Disposable {
  private readonly decoration = vscode.window.createTextEditorDecorationType({
    after: { color: new vscode.ThemeColor('editorCodeLens.foreground'), margin: '0 0 0 3em' },
    rangeBehavior: vscode.DecorationRangeBehavior.ClosedOpen,
  });
  private readonly subscriptions: vscode.Disposable[];
  private readonly kept = new LineBlames<ShownBlameLine | null>();
  private timer?: ReturnType<typeof setTimeout>;
  /** Bumped on every request, so that a slow answer for a line the cursor already left is dropped. */
  private request = 0;
  /**
   * The editor and the line of a version of its text the decoration was last drawn for: a cursor that moves along it
   * draws nothing again.
   */
  private drawn?: { editor: vscode.TextEditor; key: string };

  constructor(private readonly source: BlameSource) {
    const schedule = () => this.schedule();
    this.subscriptions = [
      this.decoration,
      vscode.window.onDidChangeActiveTextEditor(schedule),
      vscode.window.onDidChangeTextEditorSelection(schedule),
      vscode.workspace.onDidChangeConfiguration((event) => event.affectsConfiguration('gitardo.blame') && this.refresh()),
      vscode.workspace.onDidCloseTextDocument((document) => this.kept.forget(document.uri.toString())),
    ];
    schedule();
  }

  dispose(): void {
    clearTimeout(this.timer);
    for (const subscription of this.subscriptions) subscription.dispose();
  }

  /**
   * Asks git again for the line under the cursor, and forgets the others: the commits under them, or how they are
   * blamed, changed.
   */
  refresh(): void {
    this.kept.forget();
    this.drawn = undefined;
    this.schedule();
  }

  private schedule(): void {
    clearTimeout(this.timer);
    const editor = vscode.window.activeTextEditor;
    if (editor && this.drawn?.editor === editor && this.drawn.key === keyOf(editor)) {
      // Back on the line drawn, in the same text: an answer for a line left on the way is dropped.
      this.request++;
      return;
    }
    this.timer = setTimeout(() => void this.update(), DELAY);
  }

  private async update(): Promise<void> {
    const request = ++this.request;
    const editor = vscode.window.activeTextEditor;
    if (!editor) return;
    const clear = () => {
      this.drawn = undefined;
      editor.setDecorations(this.decoration, []);
    };

    const enabled = vscode.workspace.getConfiguration('gitardo').get<boolean>('blame.enabled');
    const { document, selection } = editor;
    if (!enabled || document.uri.scheme !== 'file' || !selection.isSingleLine) return clear();

    const found = await this.source.of(document);
    if (request !== this.request) return;
    if (!found) return clear();

    // Read after the wait, so that the text sent with a document not saved is the text of the version it is kept for.
    const line = editor.selection.active.line;
    const key = keyOf(editor);
    if (!key) return clear();
    const blame = await this.kept
      .get(document.uri.toString(), document.version, line, () => reads.blameLine(found.repo, found.file, line + 1, this.source.options(document, found)))
      .catch(() => null);
    if (request !== this.request) return;
    if (!blame) return clear();

    const text = blame.committed ? `${blame.author}, ${ago(blame.date)} • ${blame.summary}` : 'Not committed yet';
    const hover = new vscode.MarkdownString();
    if (blame.committed) {
      hover.isTrusted = { enabledCommands: ['gitardo.revealCommit'] };
      const link = `command:gitardo.revealCommit?${encodeURIComponent(JSON.stringify([blame.hash]))}`;
      hover.appendText(`${blame.author}, ${new Date(blame.date * 1000).toLocaleString()}\n\n${blame.summary}`);
      hover.appendMarkdown(`\n\n[Show ${blame.hash.slice(0, 7)} in the Gitardo graph](${link})`);
    }
    const end = document.lineAt(line).range.end;
    editor.setDecorations(this.decoration, [
      { range: new vscode.Range(end, end), hoverMessage: blame.committed ? hover : undefined, renderOptions: { after: { contentText: text } } },
    ]);
    this.drawn = { editor, key };
  }
}

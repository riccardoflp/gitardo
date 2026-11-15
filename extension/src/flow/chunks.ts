import * as vscode from 'vscode';
import type { EngineRun } from '../engine/actions';
import type { DiffChangeTarget, DiffSideTarget } from '../engine/messages';
import type { AlignedDiffModel, DiffChunk } from './diff/model';
import type { DiffHost } from './panel';
import { type DiffDescriptor, type Version, canStage, fileUri } from './sides';

export type ChunkActionKind = 'revertChunk' | 'stageChunk' | 'unstageChunk';

/**
 * Runs features of the engine in a repository the way Gitardo runs its own: logged, asked first when the user wants
 * that, and followed by a refresh. Resolves to whether they all ran; a failure or a refusal has already been said.
 */
export type RunOp = (root: string, runs: EngineRun[]) => Promise<boolean>;
/** Host-side handlers for the per-chunk gutter buttons. */
export class ChunkActions {
  constructor(
    private readonly runOp: RunOp,
    private readonly refresh: (panel: DiffHost) => Promise<void>,
  ) {}

  async handle(panel: DiffHost, action: ChunkActionKind, chunkId: number): Promise<void> {
    const model = panel.currentModel;
    const chunk = model?.chunks.find((c) => c.id === chunkId);
    if (!model || !chunk) {
      return;
    }
    if (action !== 'revertChunk' && !canStage(panel.descriptor)) {
      return; // the webview never offers these when the index has no part in the diff
    }
    try {
      switch (action) {
        case 'revertChunk':
          await this.revertChunk(panel, model, chunk);
          break;
        case 'stageChunk':
          await this.stageChunk(panel, model, chunk);
          break;
        case 'unstageChunk':
          await this.unstageChunk(panel, model, chunk);
          break;
      }
      await this.refresh(panel);
    } catch (error) {
      void vscode.window.showWarningMessage(
        `Flow Diff: ${actionLabel(action)} failed — ${String((error as Error).message ?? error)}`,
      );
    }
  }

  /** Restores the left side's version of the chunk in the working tree (undoable edit + save). */
  private async revertChunk(panel: DiffHost, model: AlignedDiffModel, chunk: DiffChunk): Promise<void> {
    if (panel.descriptor.right.kind !== 'worktree') {
      return;
    }
    const uri = fileUri(panel.descriptor);
    const document = await vscode.workspace.openTextDocument(uri);
    if (!rightSideMatches(document, model, chunk)) {
      throw new Error('the file changed since the diff was computed, retry');
    }

    const leftLines = sideLines(model, chunk, 'left');
    const edit = new vscode.WorkspaceEdit();
    if (chunk.rightCount > 0) {
      const startLine = chunk.rightStart - 1;
      const endLineExclusive = startLine + chunk.rightCount;
      if (endLineExclusive < document.lineCount) {
        edit.replace(uri, new vscode.Range(startLine, 0, endLineExclusive, 0), leftLines.map((l) => l + '\n').join(''));
      } else {
        // chunk reaches a final line without trailing newline
        const lastLine = document.lineCount - 1;
        edit.replace(uri, new vscode.Range(startLine, 0, lastLine, document.lineAt(lastLine).text.length), leftLines.join('\n'));
      }
    } else {
      // pure deletion: re-insert the left lines (rightStart = line before, 1-based)
      if (chunk.rightStart < document.lineCount) {
        edit.insert(uri, new vscode.Position(chunk.rightStart, 0), leftLines.join('\n') + '\n');
      } else {
        const lastLine = document.lineCount - 1;
        edit.insert(uri, new vscode.Position(lastLine, document.lineAt(lastLine).text.length), '\n' + leftLines.join('\n'));
      }
    }
    if (!(await vscode.workspace.applyEdit(edit))) {
      throw new Error('the edit could not be applied');
    }
    await document.save();
  }

  /**
   * Moves the index towards the working tree on the change: `diff.stage-change` of the engine, which makes the patch,
   * checks the file is as the diff shows it, and stages a file the index does not have whole.
   */
  private async stageChunk(panel: DiffHost, model: AlignedDiffModel, chunk: DiffChunk): Promise<void> {
    const { descriptor } = panel;
    await this.runOp(descriptor.repoRoot, [{ feature: 'diff.stage-change', target: changeTarget(descriptor, model, chunk) }]);
  }

  /**
   * Moves the index back towards HEAD on the change: `diff.unstage-change` of the engine. In the diff of the index the
   * change's right side is the index; in the diff of the working tree (a staged change) its left side is HEAD.
   */
  private async unstageChunk(panel: DiffHost, model: AlignedDiffModel, chunk: DiffChunk): Promise<void> {
    const { descriptor } = panel;
    await this.runOp(descriptor.repoRoot, [{ feature: 'diff.unstage-change', target: changeTarget(descriptor, model, chunk) }]);
  }
}

/** A side of a Flow Diff as the engine names it. */
function sideTarget(version: Version): DiffSideTarget {
  switch (version.kind) {
    case 'worktree':
      return { kind: 'workingTree' };
    case 'index':
      return { kind: 'index' };
    case 'commit':
      return version.ref === 'HEAD' ? { kind: 'head' } : { kind: 'commit', commit: version.ref };
    case 'none':
      return { kind: 'empty' };
  }
}

/**
 * A change of a Flow Diff as the engine takes it: its lines on each side from 0, the end left out, and their text. A
 * change that has no lines on a side starts there after the line it follows, which the diff counts from 1.
 */
export function changeTarget(descriptor: DiffDescriptor, model: AlignedDiffModel, chunk: DiffChunk): DiffChangeTarget {
  const lines = (start: number, count: number) => {
    const first = count ? start - 1 : start;
    return { start: first, end: first + count };
  };
  return {
    kind: 'diffChange',
    path: descriptor.path,
    left: sideTarget(descriptor.left),
    right: sideTarget(descriptor.right),
    leftLines: lines(chunk.leftStart, chunk.leftCount),
    rightLines: lines(chunk.rightStart, chunk.rightCount),
    leftText: sideLines(model, chunk, 'left'),
    rightText: sideLines(model, chunk, 'right'),
  };
}

function sideLines(model: AlignedDiffModel, chunk: DiffChunk, side: 'left' | 'right'): string[] {
  const lines: string[] = [];
  for (let i = chunk.rowStart; i < chunk.rowEnd; i++) {
    const cell = side === 'left' ? model.rows[i].left : model.rows[i].right;
    if (cell.kind !== 'filler' && cell.text !== undefined) {
      lines.push(cell.text);
    }
  }
  return lines;
}

/** Race guard: the document must still contain what the model says it does. */
function rightSideMatches(document: vscode.TextDocument, model: AlignedDiffModel, chunk: DiffChunk): boolean {
  if (chunk.rightCount === 0) {
    return true;
  }
  const expected = sideLines(model, chunk, 'right');
  const startLine = chunk.rightStart - 1;
  if (startLine + chunk.rightCount > document.lineCount) {
    return false;
  }
  for (let j = 0; j < expected.length; j++) {
    if (document.lineAt(startLine + j).text !== expected[j]) {
      return false;
    }
  }
  return true;
}

function actionLabel(action: ChunkActionKind): string {
  switch (action) {
    case 'revertChunk':
      return 'revert chunk';
    case 'stageChunk':
      return 'stage chunk';
    case 'unstageChunk':
      return 'unstage chunk';
  }
}

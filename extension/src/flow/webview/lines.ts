import type * as monaco from 'monaco-editor/esm/vs/editor/editor.api.js';
import { MenuId, MenuRegistry } from 'monaco-editor/esm/vs/platform/actions/common/actions.js';
import { CommandsRegistry } from 'monaco-editor/esm/vs/platform/commands/common/commands.js';
import { ContextKeyExpr } from 'monaco-editor/esm/vs/platform/contextkey/common/contextkey.js';
import { type RowSpan, hasChangedRow, rowsOfSelection } from '../diff/lineMap';
import type { AlignedDiffModel } from '../diff/model';
import type { LinesAction } from '../diff/protocol';

/** The context key of an editor that says some row of a change is among the rows its selection takes. */
const HAS_CHANGED_ROW = 'gitardo.linesHaveChange';

const LABELS: Record<LinesAction, string> = { stage: 'Stage lines', unstage: 'Unstage lines', revert: 'Revert lines' };

/**
 * The lines selected in either side of the Flow Diff (`diff.select-lines`): the selection is Monaco's own, mapped to
 * the rows of the aligned model, and the context menu of both sides has **Stage lines**, **Unstage lines** and
 * **Revert lines**, each where the page offers it, greyed out while no row of a change is selected.
 */
export class LineSelection {
  private model: AlignedDiffModel | undefined;
  /** The side whose menu was opened last, or which had the keyboard last: the one its items act on. */
  private last: monaco.editor.IStandaloneCodeEditor | undefined;
  private readonly keys = new Map<monaco.editor.IStandaloneCodeEditor, monaco.editor.IContextKey<boolean>>();

  constructor(
    private readonly sides: { left: monaco.editor.IStandaloneCodeEditor; right: monaco.editor.IStandaloneCodeEditor },
    offered: LinesAction[],
    private readonly act: (action: LinesAction, rows: RowSpan) => void,
  ) {
    for (const editor of [sides.left, sides.right]) {
      this.keys.set(editor, editor.createContextKey(HAS_CHANGED_ROW, false));
      editor.onDidChangeCursorSelection(() => this.update(editor));
      editor.onDidFocusEditorWidget(() => (this.last = editor));
      editor.onContextMenu(() => (this.last = editor));
    }
    offered.forEach((action, order) => {
      const id = `gitardo.flowDiff.${action}Lines`;
      CommandsRegistry.registerCommand(id, () => this.run(action));
      MenuRegistry.appendMenuItem(MenuId.EditorContext, {
        command: { id, title: LABELS[action], precondition: ContextKeyExpr.has(HAS_CHANGED_ROW) },
        group: '1_gitardoLines',
        order,
      });
    });
  }

  /** The model the page shows now: the rows a selection takes are found again in it. */
  setModel(model: AlignedDiffModel): void {
    this.model = model;
    this.update(this.sides.left);
    this.update(this.sides.right);
  }

  /** The rows the selection of a side takes, if it is in the model. */
  rowsOf(editor: monaco.editor.IStandaloneCodeEditor): RowSpan | undefined {
    const selection = editor.getSelection();
    if (!this.model || !selection) return undefined;
    const side = editor === this.sides.left ? 'left' : 'right';
    return rowsOfSelection(this.model, {
      side,
      startLine: selection.startLineNumber,
      endLine: selection.endLineNumber,
      endColumn: selection.endColumn,
    });
  }

  private update(editor: monaco.editor.IStandaloneCodeEditor): void {
    const rows = this.rowsOf(editor);
    this.keys.get(editor)?.set(!!rows && !!this.model && hasChangedRow(this.model, rows));
  }

  /** Acts on the rows of the side the menu was opened on, and clears its selection, as the desktop does. */
  private run(action: LinesAction): void {
    const editor = this.last;
    if (!editor || !this.model) return;
    const rows = this.rowsOf(editor);
    if (!rows || !hasChangedRow(this.model, rows)) return;
    this.act(action, rows);
    const start = editor.getSelection()?.getStartPosition();
    if (start) editor.setSelection({ startLineNumber: start.lineNumber, startColumn: start.column, endLineNumber: start.lineNumber, endColumn: start.column });
  }
}

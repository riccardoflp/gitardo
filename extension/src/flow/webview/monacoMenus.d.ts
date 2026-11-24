/**
 * The few parts of Monaco's own menus the Flow Diff uses, which its package gives no types for: an item of the context
 * menu of the editors that is greyed out, rather than left out, while its condition is false (`diff.select-lines`).
 */

declare module 'monaco-editor/esm/vs/platform/actions/common/actions.js' {
  interface Disposable {
    dispose(): void;
  }
  export const MenuId: { EditorContext: unknown };
  export const MenuRegistry: {
    appendMenuItem(
      menu: unknown,
      item: { command: { id: string; title: string; precondition?: unknown }; when?: unknown; group?: string; order?: number },
    ): Disposable;
  };
}

declare module 'monaco-editor/esm/vs/platform/commands/common/commands.js' {
  export const CommandsRegistry: { registerCommand(id: string, handler: () => void): { dispose(): void } };
}

declare module 'monaco-editor/esm/vs/platform/contextkey/common/contextkey.js' {
  export const ContextKeyExpr: { has(key: string): unknown };
}

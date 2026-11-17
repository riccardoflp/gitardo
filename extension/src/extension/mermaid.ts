/**
 * The diagrams of Mermaid in VS Code's preview of Markdown (`file-view.mermaid`): the plugin of `src/shared/mermaid.ts`
 * given to VS Code's Markdown, which asks it of the extensions that contribute `markdown.markdownItPlugins`, with the
 * kind of VS Code's theme, the engine's reads, and a refresh of the previews when a diagram comes or the theme changes.
 */
import * as vscode from 'vscode';
import { reads } from '../engine/reads';
import { type MarkdownItRules, MermaidDiagrams, type MermaidTheme, useMermaid } from '../shared/mermaid';

/** The kind of a theme of VS Code, as the engine draws for it. */
function themeOf(kind: vscode.ColorThemeKind): MermaidTheme {
  switch (kind) {
    case vscode.ColorThemeKind.Light:
      return 'light';
    case vscode.ColorThemeKind.HighContrast:
      return 'highContrastDark';
    case vscode.ColorThemeKind.HighContrastLight:
      return 'highContrastLight';
    case vscode.ColorThemeKind.Dark:
      return 'dark';
  }
}

/**
 * Another extension that draws Mermaid in the preview, whose plugin of the Markdown is left the blocks of Mermaid (Q592):
 * one whose id names Mermaid and that contributes a plugin of the Markdown.
 */
function otherMermaid(own: string): vscode.Extension<unknown> | undefined {
  return vscode.extensions.all.find((extension) => {
    if (extension.id === own || !/mermaid/i.test(extension.id)) return false;
    const contributes = (extension.packageJSON as { contributes?: Record<string, unknown> }).contributes;
    return !!contributes?.['markdown.markdownItPlugins'];
  });
}

/** What the extension gives VS Code's Markdown, and the refresh of its previews when the theme changes. */
export function mermaidPreview(context: vscode.ExtensionContext, log: vscode.LogOutputChannel): <T extends MarkdownItRules>(markdown: T) => T {
  const refresh = () => void vscode.commands.executeCommand('markdown.preview.refresh');
  const diagrams = new MermaidDiagrams({
    theme: () => themeOf(vscode.window.activeColorTheme.kind),
    draw: (source, theme) =>
      reads.mermaid(source, theme).catch((error: unknown) => {
        log.debug(`A diagram of Mermaid was not drawn: ${error instanceof Error ? error.message : String(error)}`);
        throw error;
      }),
    refresh,
  });
  context.subscriptions.push(vscode.window.onDidChangeActiveColorTheme(refresh));
  return (markdown) => {
    const other = otherMermaid(context.extension.id);
    if (other) {
      log.info(`The diagrams of Mermaid in the preview of Markdown are left to ${other.id}.`);
      return markdown;
    }
    return useMermaid(markdown, diagrams);
  };
}

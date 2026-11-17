/** Whether a file is Markdown by its name: one ending in `.md` or `.markdown`, whatever the case, as Gitardo Desktop says. */
export function isMarkdown(file: string): boolean {
  const name = file.slice(Math.max(file.lastIndexOf('/'), file.lastIndexOf('\\')) + 1);
  return /^.+\.(md|markdown)$/i.test(name);
}

/** Where a file Gitardo opens is shown: VS Code's preview of Markdown, or its editor. */
export type Opening = 'preview' | 'editor';

/**
 * Where a file Gitardo opens is shown: a Markdown file in VS Code's preview of Markdown while
 * `gitardo.markdown.openAsPreview` is on, every other file in its editor (`file-view.markdown-preview`).
 */
export function openingOf(file: string, openAsPreview: boolean): Opening {
  return openAsPreview && isMarkdown(file) ? 'preview' : 'editor';
}

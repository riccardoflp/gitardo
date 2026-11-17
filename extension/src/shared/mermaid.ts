/**
 * The diagrams of Mermaid in VS Code's preview of Markdown (`file-view.mermaid`, D60): a plugin of its Markdown that
 * shows a block fenced as `mermaid` as the diagram Gitardo's engine drew for the kind of VS Code's theme, as an image,
 * so that nothing in it runs. Until the engine answers, and without the engine, the block keeps its code; once it
 * answers, the preview is drawn again. What the engine answered is kept for the text and the theme.
 */

/** The kind of VS Code's theme a diagram is drawn for. */
export type MermaidTheme = 'dark' | 'light' | 'highContrastDark' | 'highContrastLight';

/** What the engine answers for a block of Mermaid (`read/file-view.mermaid`, 1.9). */
export type MermaidAnswer =
  | { kind: 'drawn'; svg: string; width: number; height: number; title: string | null; description: string | null }
  | { kind: 'invalid'; reason: string }
  | { kind: 'unsupported' }
  | { kind: 'tooLarge' };

/** What the plugin asks of VS Code and of the engine. */
export interface MermaidHost {
  /** The kind of VS Code's theme now. */
  theme(): MermaidTheme;
  /** The engine's answer for a block in a theme; rejected when there is no engine to ask. */
  draw(source: string, theme: MermaidTheme): Promise<MermaidAnswer>;
  /** Draws the previews again, once a diagram they show came. */
  refresh(): void;
}

/** A block of the Markdown as markdown-it gives it to the rule of fenced code. */
export interface FenceToken {
  info: string;
  content: string;
}

/** The rule of markdown-it that writes the HTML of a block of fenced code. */
export type FenceRule = (tokens: FenceToken[], index: number, options: unknown, env: unknown, renderer: unknown) => string;

/** What the plugin changes of markdown-it: the rule of fenced code. */
export interface MarkdownItRules {
  renderer: { rules: { fence?: FenceRule } };
}

/** What the screen reader says of a diagram without a title. */
export const DIAGRAM = 'Mermaid diagram';
/** What a diagram of a kind not drawn says under its code. */
export const DIAGRAM_UNSUPPORTED = 'Gitardo does not draw this kind of diagram.';
/** What a diagram too large to draw says under its code. */
export const DIAGRAM_TOO_LARGE = 'This diagram is too large to draw.';

/** What a diagram with an error says under its code, in the words of the library. */
export function diagramError(reason: string): string {
  return `This diagram has an error: ${reason}.`;
}

/** How many answers are kept: more than a few documents of diagrams, in two themes. */
const KEPT = 200;

/** Whether the fence of a block names `mermaid`, in any case, as its language. */
export function isMermaid(info: string): boolean {
  return (info.trim().split(/\s+/)[0] ?? '').toLowerCase() === 'mermaid';
}

/** The answers of the engine for the blocks of the previews, by their text and their theme, and those asked. */
export class MermaidDiagrams {
  private readonly kept = new Map<string, MermaidAnswer>();
  private readonly asked = new Set<string>();
  private refreshing = false;

  constructor(private readonly host: MermaidHost) {}

  /** What the preview shows of a block now: the engine's answer in the theme of VS Code, or nothing while it is asked. */
  shown(source: string): MermaidAnswer | undefined {
    const theme = this.host.theme();
    const key = `${theme}\n${source}`;
    const answer = this.kept.get(key);
    if (answer || this.asked.has(key)) return answer;
    this.asked.add(key);
    this.host.draw(source, theme).then(
      (drawn) => {
        this.asked.delete(key);
        this.keep(key, drawn);
        this.refreshSoon();
      },
      () => {
        // Without the engine, or when it failed, the block keeps its code, and is asked again the next time the
        // preview is drawn; the client has logged why.
        this.asked.delete(key);
      },
    );
    return undefined;
  }

  /** Keeps an answer, the oldest one forgotten past the most kept. */
  private keep(key: string, answer: MermaidAnswer): void {
    this.kept.set(key, answer);
    if (this.kept.size > KEPT) {
      const oldest = this.kept.keys().next().value;
      if (oldest !== undefined) this.kept.delete(oldest);
    }
  }

  /** Draws the previews again once, after the answers that come together. */
  private refreshSoon(): void {
    if (this.refreshing) return;
    this.refreshing = true;
    setTimeout(() => {
      this.refreshing = false;
      this.host.refresh();
    }, 0);
  }
}

/** The HTML of a text, safe between tags and in an attribute. */
function escaped(text: string): string {
  return text.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;').replace(/'/g, '&#39;');
}

/** What the screen reader says of a diagram drawn: its title and its description, or that it is a diagram of Mermaid. */
function said(answer: { title: string | null; description: string | null }): string {
  const parts = [answer.title ?? (answer.description ? DIAGRAM : null), answer.description].filter((part): part is string => !!part);
  return parts.length ? parts.join('. ') : DIAGRAM;
}

/** The HTML of a diagram drawn: an image of its SVG, as wide as it is or as the page, named for the screen reader. */
function image(answer: Extract<MermaidAnswer, { kind: 'drawn' }>): string {
  const source = `data:image/svg+xml;charset=utf-8,${encodeURIComponent(answer.svg)}`;
  const size = answer.width > 0 && answer.height > 0 ? ` width="${Math.round(answer.width)}" height="${Math.round(answer.height)}"` : '';
  return `<p class="gitardo-mermaid"><img src="${source}" alt="${escaped(said(answer))}"${size} style="max-width: 100%; height: auto;"></p>\n`;
}

/** The line under the code of a diagram not drawn, which says why. */
function why(answer: Exclude<MermaidAnswer, { kind: 'drawn' }>): string {
  const text = answer.kind === 'invalid' ? diagramError(answer.reason) : answer.kind === 'unsupported' ? DIAGRAM_UNSUPPORTED : DIAGRAM_TOO_LARGE;
  return `<p class="gitardo-mermaid-error"><em>${escaped(text)}</em></p>\n`;
}

/**
 * Makes the preview show a block fenced as `mermaid` as its diagram: the image of the engine's SVG once it answered,
 * its code until then and without the engine, its code and why when the engine did not draw it. Every other block is
 * written as before.
 *
 * Feature: file-view.mermaid
 */
export function useMermaid<T extends MarkdownItRules>(markdown: T, diagrams: MermaidDiagrams): T {
  const before = markdown.renderer.rules.fence;
  const code: FenceRule = (tokens, index, options, env, renderer) =>
    before ? before(tokens, index, options, env, renderer) : `<pre><code>${escaped(tokens[index]?.content ?? '')}</code></pre>\n`;
  markdown.renderer.rules.fence = (tokens, index, options, env, renderer) => {
    const token = tokens[index];
    if (!token || !isMermaid(token.info)) return code(tokens, index, options, env, renderer);
    const answer = diagrams.shown(token.content);
    if (!answer) return code(tokens, index, options, env, renderer);
    if (answer.kind === 'drawn') return image(answer);
    return code(tokens, index, options, env, renderer) + why(answer);
  };
  return markdown;
}

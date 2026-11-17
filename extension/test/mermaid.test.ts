import { describe, expect, it, vi } from 'vitest';
import {
  DIAGRAM,
  DIAGRAM_UNSUPPORTED,
  type FenceRule,
  type FenceToken,
  type MermaidAnswer,
  MermaidDiagrams,
  type MermaidTheme,
  diagramError,
  isMermaid,
  useMermaid,
} from '../src/shared/mermaid';
import { built, cleanUpAfterEach, client } from './engine/harness';

cleanUpAfterEach();

const FLOWCHART = 'flowchart LR\n  A[Edit files] --> B[Commit]\n';
const DRAWN: MermaidAnswer = {
  kind: 'drawn',
  svg: '<svg viewBox="0 0 370 61"><text>Edit files</text></svg>',
  width: 370,
  height: 61,
  title: 'How a commit is made',
  description: null,
};

/** A preview of VS Code: its Markdown with the rule of fenced code it has, and the engine and the theme it asks. */
function preview(answer: (source: string, theme: MermaidTheme) => Promise<MermaidAnswer>) {
  const asked: Array<[string, MermaidTheme]> = [];
  const refresh = vi.fn();
  let theme: MermaidTheme = 'dark';
  const host = {
    theme: () => theme,
    draw: (source: string, kind: MermaidTheme) => {
      asked.push([source, kind]);
      return answer(source, kind);
    },
    refresh,
  };
  const before: FenceRule = (tokens, index) => `<pre><code class="language-${tokens[index]?.info}">${tokens[index]?.content}</code></pre>`;
  const markdown = useMermaid({ renderer: { rules: { fence: before } } }, new MermaidDiagrams(host));
  const render = (token: FenceToken) => markdown.renderer.rules.fence?.([token], 0, {}, {}, {}) ?? '';
  return { asked, refresh, render, setTheme: (kind: MermaidTheme) => (theme = kind) };
}

/** Lets the answers of the engine and the refresh after them come. */
const settled = () => new Promise((resolve) => setTimeout(resolve, 5));

describe('isMermaid', () => {
  it('is true for a fence that names mermaid in any case, with more words after it', () => {
    for (const info of ['mermaid', 'Mermaid', ' MERMAID ', 'mermaid title']) expect(isMermaid(info), info).toBe(true);
    for (const info of ['', 'rust', 'mermaidjs', 'text mermaid']) expect(isMermaid(info), info).toBe(false);
  });
});

describe('useMermaid', () => {
  it("file_view_mermaid: with a mermaid block, VS Code's preview shows the diagram the engine drew for the theme of VS Code; without the engine it shows the code", async () => {
    const drawn = preview(async () => DRAWN);
    const block = { info: 'mermaid', content: FLOWCHART };
    expect(drawn.render(block)).toContain('<pre><code class="language-mermaid">');
    await settled();
    expect(drawn.refresh).toHaveBeenCalledTimes(1);
    const html = drawn.render(block);
    expect(html).toMatch(/^<p class="gitardo-mermaid"><img src="data:image\/svg\+xml;charset=utf-8,%3Csvg/);
    expect(html).toContain('alt="How a commit is made"');
    expect(html).toContain('width="370" height="61"');
    expect(html).not.toContain('<svg');
    expect(drawn.asked).toEqual([[FLOWCHART, 'dark']]);

    const missing = preview(() => Promise.reject(new Error("Gitardo's engine is not running.")));
    expect(missing.render(block)).toContain('<pre><code class="language-mermaid">');
    await settled();
    expect(missing.refresh).not.toHaveBeenCalled();
    expect(missing.render(block)).toContain('<pre><code class="language-mermaid">');
  });

  it('asks each text once for each theme, and again when the theme changes', async () => {
    const drawn = preview(async () => DRAWN);
    const block = { info: 'mermaid', content: FLOWCHART };
    drawn.render(block);
    drawn.render(block);
    await settled();
    drawn.render(block);
    drawn.setTheme('highContrastLight');
    expect(drawn.render(block)).toContain('<pre>');
    expect(drawn.asked).toEqual([
      [FLOWCHART, 'dark'],
      [FLOWCHART, 'highContrastLight'],
    ]);
  });

  it('shows the code of a diagram not drawn, with why under it', async () => {
    const answers: Record<string, MermaidAnswer> = {
      'flowchart LR\n  A[Start --> B\n': { kind: 'invalid', reason: 'Unterminated node label (missing `]`)' },
      'notADiagram\n': { kind: 'unsupported' },
    };
    const shown = preview(async (source) => answers[source] ?? DRAWN);
    for (const content of Object.keys(answers)) shown.render({ info: 'mermaid', content });
    await settled();
    const invalid = shown.render({ info: 'mermaid', content: 'flowchart LR\n  A[Start --> B\n' });
    expect(invalid).toContain('A[Start --> B');
    expect(invalid).toContain(`<em>${diagramError('Unterminated node label (missing `]`)')}</em>`);
    expect(shown.render({ info: 'mermaid', content: 'notADiagram\n' })).toContain(`<em>${DIAGRAM_UNSUPPORTED}</em>`);
  });

  it('names a diagram without a title Mermaid diagram, and escapes what the title says', async () => {
    const untitled = preview(async () => ({ ...DRAWN, title: null }));
    untitled.render({ info: 'mermaid', content: FLOWCHART });
    await settled();
    expect(untitled.render({ info: 'mermaid', content: FLOWCHART })).toContain(`alt="${DIAGRAM}"`);
    const quoted = preview(async () => ({ ...DRAWN, title: '"><script>' }));
    quoted.render({ info: 'mermaid', content: FLOWCHART });
    await settled();
    expect(quoted.render({ info: 'mermaid', content: FLOWCHART })).toContain('alt="&quot;&gt;&lt;script&gt;"');
  });

  it('leaves every other block to the rule before it, and asks nothing', () => {
    const shown = preview(async () => DRAWN);
    expect(shown.render({ info: 'rust', content: 'fn main() {}' })).toBe('<pre><code class="language-rust">fn main() {}</code></pre>');
    expect(shown.asked).toEqual([]);
  });
});

describe.skipIf(!built)('read/file-view.mermaid', { timeout: 60_000 }, () => {
  it('draws a flowchart as an SVG for the kind of the theme, with no repository, and says why another is not drawn', async () => {
    const engine = client();
    const drawn = await engine.drawMermaid('flowchart LR\n  accTitle: How a commit is made\n  A[Edit files] --> B[Commit]\n', 'light');
    expect(drawn.kind).toBe('drawn');
    if (drawn.kind !== 'drawn') return;
    expect(drawn.svg.startsWith('<svg')).toBe(true);
    expect(drawn.svg).toContain('Edit files');
    expect(drawn.title).toBe('How a commit is made');
    expect(drawn.width).toBeGreaterThan(drawn.height);
    expect(await engine.drawMermaid('notADiagram\n', 'dark')).toEqual({ kind: 'unsupported' });
  });
});

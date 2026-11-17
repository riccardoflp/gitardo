import { describe, expect, it } from 'vitest';
import { isMarkdown, openingOf } from '../src/shared/markdown';

describe('isMarkdown', () => {
  it('is true for a file ending in .md or .markdown, whatever the case and the folder', () => {
    for (const file of ['README.md', 'docs/guide.markdown', 'docs\\NOTES.MD', 'a.Markdown']) expect(isMarkdown(file), file).toBe(true);
  });

  it('is false for other files, a name that is only the extension, and a folder named so', () => {
    for (const file of ['src/app.rs', '.md', 'docs/.md', 'notes.md/readme.txt', 'README', 'guide.mdx']) expect(isMarkdown(file), file).toBe(false);
  });
});

describe('openingOf', () => {
  it('opens README.md in the preview with the setting on, in the editor with it off', () => {
    expect(openingOf('README.md', true)).toBe('preview');
    expect(openingOf('README.md', false)).toBe('editor');
  });

  it('opens a file that is not Markdown in the editor whatever the setting', () => {
    expect(openingOf('src/app.rs', true)).toBe('editor');
  });
});

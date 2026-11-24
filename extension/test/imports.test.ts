/**
 * The imports between the folders of `src`, as the architecture draws them (docs/extension/02-architecture.md, "Who
 * may use whom"; rules R1.4 and R1.5; risk RS3): which folder uses which, that only `src/engine/vscode.ts` of the
 * engine's folder speaks to VS Code, that only `src/engine/client.ts` starts a program, and that nothing bundled into
 * a page of a webview imports Node or VS Code, which would break that bundle or bloat it.
 */

import * as fs from 'node:fs';
import { builtinModules } from 'node:module';
import * as path from 'node:path';
import { describe, expect, it } from 'vitest';

const source = path.resolve(__dirname, '..', 'src');

/** The folders of the drawing, the deepest first, so that a file is in the most precise one. */
const FOLDERS = ['flow/webview', 'flow/diff', 'webview', 'flow', 'engine', 'shared', 'extension', 'dev'];

/** Which folders each folder may import from, besides its own. */
const USES: Record<string, string[]> = {
  shared: [],
  webview: ['shared'],
  engine: ['shared'],
  flow: ['engine', 'shared', 'flow/diff', 'flow/webview'],
  'flow/diff': ['shared'],
  'flow/webview': ['shared', 'flow/diff'],
  extension: ['engine', 'shared', 'flow', 'flow/diff', 'flow/webview'],
  dev: ['engine', 'shared'],
};

/**
 * The files one folder imports from another the drawing does not join, each marked on the page of the architecture:
 * a type of the Flow Diff that `src/shared` uses, the pure code and two types of the Flow Diff bundled into the
 * webview's pages,
 * the two modules of the extension the Flow Diff hands its operations through, and the file icons of the preview.
 */
const MARKED: Record<string, string[]> = {
  'shared/protocol.ts': ['flow/diff/protocol.ts'],
  webview: ['flow/diff/computeDiff.ts', 'flow/diff/model.ts', 'flow/diff/protocol.ts', 'flow/webview/columns.ts', 'flow/webview/languages.ts'],
  flow: ['extension/content.ts', 'extension/answers.ts'],
  dev: ['extension/fileIconTheme.ts'],
};

/** The modules of the pages of the webviews, as `esbuild.mjs` bundles them for the browser. */
const PAGES = ['webview/main.tsx', 'webview/highlight/index.ts', 'flow/webview/main.ts', 'flow/webview/merge.ts'];

const node = new Set(builtinModules);

/** Every TypeScript file of `src`, by its path from there, with `/` between folders. */
function files(folder = source): string[] {
  return fs.readdirSync(folder, { withFileTypes: true }).flatMap((entry) => {
    const full = path.join(folder, entry.name);
    if (entry.isDirectory()) return files(full);
    return /\.tsx?$/.test(entry.name) && !entry.name.endsWith('.d.ts') ? [path.relative(source, full).split(path.sep).join('/')] : [];
  });
}

/** What a file imports: `import … from`, `export … from`, a bare `import '…'` and `import('…')`, comments left out. */
function specifiers(file: string): string[] {
  const text = fs
    .readFileSync(path.join(source, file), 'utf8')
    .replace(/\/\*[\s\S]*?\*\//g, '')
    .replace(/^\s*\/\/.*$/gm, '');
  const found: string[] = [];
  const pattern = /\b(?:import|export)\s[^'";]*?\bfrom\s*['"]([^'"]+)['"]|\bimport\s*\(\s*['"]([^'"]+)['"]\s*\)|^\s*import\s*['"]([^'"]+)['"]/gm;
  for (const match of text.matchAll(pattern)) found.push(match[1] ?? match[2] ?? match[3]);
  return found;
}

/** The file of `src` a relative import names, or null for a package, Node or a file that is not TypeScript. */
function resolve(from: string, specifier: string): string | null {
  if (!specifier.startsWith('.')) return null;
  const base = path.posix.normalize(path.posix.join(path.posix.dirname(from), specifier)).replace(/\.js$/, '');
  for (const candidate of [base, `${base}.ts`, `${base}.tsx`, `${base}/index.ts`, `${base}/index.tsx`]) {
    if (/\.tsx?$/.test(candidate) && fs.existsSync(path.join(source, candidate))) return candidate;
  }
  return null;
}

function folderOf(file: string): string {
  return FOLDERS.find((folder) => file.startsWith(`${folder}/`)) ?? file.split('/')[0];
}

function isNode(specifier: string): boolean {
  return specifier.startsWith('node:') || node.has(specifier.split('/')[0]);
}

const all = files();
const imports = new Map(all.map((file) => [file, specifiers(file)]));

describe('the imports between the folders of src', () => {
  it('finds the files of every folder of the drawing, and what they import', () => {
    for (const folder of FOLDERS) expect(all.some((file) => folderOf(file) === folder), folder).toBe(true);
    expect(imports.get('extension/controller.ts')).toContain('vscode');
  });

  it('R1.4: each folder imports only from the folders the architecture lets it use, and the files it marks', () => {
    const wrong: string[] = [];
    for (const [file, named] of imports) {
      const from = folderOf(file);
      for (const specifier of named) {
        const target = resolve(file, specifier);
        if (!target) continue;
        const to = folderOf(target);
        if (to === from || USES[from]?.includes(to)) continue;
        if (MARKED[file]?.includes(target) || MARKED[from]?.includes(target)) continue;
        wrong.push(`src/${file} imports src/${target}: ${from} does not use ${to}`);
      }
    }
    expect(wrong).toEqual([]);
  });

  it('R1.5: of src/engine, only vscode.ts imports VS Code, and only src/engine/client.ts starts a program', () => {
    const wrong: string[] = [];
    for (const [file, named] of imports) {
      if (folderOf(file) === 'engine' && file !== 'engine/vscode.ts' && named.includes('vscode')) wrong.push(`src/${file} imports vscode`);
      if (file !== 'engine/client.ts' && named.some((specifier) => /^(node:)?child_process$/.test(specifier))) wrong.push(`src/${file} imports child_process`);
    }
    expect(wrong).toEqual([]);
  });

  it('RS3: nothing bundled into the page of a webview imports Node or VS Code', () => {
    const wrong: string[] = [];
    const seen = new Set<string>();
    const visit = (file: string, from: string[]) => {
      if (seen.has(file)) return;
      seen.add(file);
      for (const specifier of imports.get(file) ?? []) {
        if (specifier === 'vscode' || isNode(specifier)) wrong.push(`src/${file} imports ${specifier}, through ${[...from, file].join(' → ')}`);
        const target = resolve(file, specifier);
        if (target) visit(target, [...from, file]);
      }
    };
    for (const page of PAGES) {
      expect(imports.has(page), page).toBe(true);
      visit(page, []);
    }
    expect(seen.size).toBeGreaterThan(PAGES.length);
    expect(wrong).toEqual([]);
  });
});

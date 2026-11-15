// Packs the minified build that `node esbuild.mjs --production` left in dist/ into the vsix files of the extension:
//   node scripts/package.mjs [--target <target>]... [--binaries <folder>] [--every-target]
// With no --target, one vsix for each target whose engine is found: there is no universal vsix, since without the
// engine for its system the extension does nothing but say so (D49), and a release waits for the vsix of every target.
// A target's engine is looked for in <folder>/<target>/ when --binaries (or GITARDO_BINARIES) names a folder, as the
// pipeline lays its artifacts out, then in ../rust/target/<triple>/release/, and, for the target of this computer, in
// ../rust/target/release/. --every-target fails when an engine is missing.
// Each vsix holds bin/<target>/gitardo-engine[.exe] only, and its size is printed for the release notes.

import { execFileSync } from 'node:child_process';
import * as fs from 'node:fs';
import { createRequire } from 'node:module';
import * as path from 'node:path';
import { fileURLToPath } from 'node:url';

const extension = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const rust = path.resolve(extension, '..', 'rust');

/** The targets of the Marketplace the extension is published for, with the target of Rust that builds each. */
export const TARGETS = {
  'win32-x64': 'x86_64-pc-windows-msvc',
  'win32-arm64': 'aarch64-pc-windows-msvc',
  'linux-x64': 'x86_64-unknown-linux-gnu',
  'linux-arm64': 'aarch64-unknown-linux-gnu',
  'darwin-x64': 'x86_64-apple-darwin',
  'darwin-arm64': 'aarch64-apple-darwin',
  'alpine-x64': 'x86_64-unknown-linux-musl',
};

/** The links of the README and the CHANGELOG, which the Marketplace shows from the repository on GitHub (D47). */
const LINKS = [
  '--baseContentUrl',
  'https://github.com/riccardoflp/gitardo/blob/HEAD',
  '--baseImagesUrl',
  'https://github.com/riccardoflp/gitardo/raw/HEAD/extension',
];

/** The target of this computer, as the Marketplace names it. */
export function hostTarget() {
  if (process.platform === 'linux' && fs.existsSync('/etc/alpine-release')) return `alpine-${process.arch}`;
  return `${process.platform}-${process.arch}`;
}

/** The name of the engine's binary for a target. */
const binaryName = (target) => (target.startsWith('win32-') ? 'gitardo-engine.exe' : 'gitardo-engine');

/** Where the engine of a target is, or nothing when it was not built. */
function engineOf(target, binaries) {
  const name = binaryName(target);
  const candidates = [
    binaries && path.join(binaries, target, name),
    path.join(rust, 'target', TARGETS[target], 'release', name),
    target === hostTarget() && path.join(rust, 'target', 'release', name),
  ].filter(Boolean);
  return candidates.find((candidate) => fs.existsSync(candidate));
}

/** Runs vsce, one of the extension's dependencies, in the folder of the extension. */
function vsce(args) {
  const manifest = createRequire(import.meta.url).resolve('@vscode/vsce/package.json');
  const { bin } = JSON.parse(fs.readFileSync(manifest, 'utf8'));
  const cli = path.join(path.dirname(manifest), typeof bin === 'string' ? bin : bin.vsce);
  execFileSync(process.execPath, [cli, 'package', '--no-dependencies', ...LINKS, ...args], { cwd: extension, stdio: 'inherit' });
}

/** The size of a file in megabytes, as the release notes say it. */
const megabytes = (file) => `${(fs.statSync(file).size / 1024 / 1024).toFixed(1)} MB`;

function main() {
  const args = process.argv.slice(2);
  const valueOf = (flag) => {
    const index = args.indexOf(flag);
    return index >= 0 ? args[index + 1] : undefined;
  };
  const asked = args.flatMap((argument, index) => (args[index - 1] === '--target' ? [argument] : []));
  for (const target of asked) if (!TARGETS[target]) throw new Error(`${target} is not a target: ${Object.keys(TARGETS).join(', ')}.`);
  const binaries = valueOf('--binaries') ?? process.env.GITARDO_BINARIES;
  const every = args.includes('--every-target');
  const { name, version } = JSON.parse(fs.readFileSync(path.join(extension, 'package.json'), 'utf8'));
  const bin = path.join(extension, 'bin');
  const made = [];

  // A bin/ left by a run that failed would go into the next vsix.
  fs.rmSync(bin, { recursive: true, force: true });

  const missing = [];
  for (const target of asked.length ? asked : Object.keys(TARGETS)) {
    const engine = engineOf(target, binaries);
    if (!engine) {
      missing.push(target);
      continue;
    }
    const folder = path.join(bin, target);
    fs.mkdirSync(folder, { recursive: true });
    const packed = path.join(folder, binaryName(target));
    fs.copyFileSync(engine, packed);
    fs.chmodSync(packed, 0o755);
    try {
      vsce(['--target', target]);
    } finally {
      fs.rmSync(bin, { recursive: true, force: true });
    }
    made.push(path.join(extension, `${name}-${target}-${version}.vsix`));
    console.log(`${target}: the engine from ${engine}`);
  }

  for (const file of made) console.log(`${path.basename(file)}  ${megabytes(file)}`);
  if (missing.length) {
    const said = `No engine for ${missing.join(', ')}: build it with cargo build --release -p server --target <triple> in rust/.`;
    if (every || asked.length) throw new Error(said);
    console.log(`${said} Their vsix files were not made, and a release waits for every target.`);
  }
  if (!made.length) throw new Error('No vsix was made: no engine was found for any target.');
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) main();

/** Where the extension finds `gitardo-engine`: the setting for development, or the binary its vsix holds (stage 8). */
import * as fs from 'node:fs';
import * as os from 'node:os';
import * as path from 'node:path';
import { afterEach, describe, expect, it } from 'vitest';
import { binaryName, findBinary } from '../src/engine/binary';

const folders: string[] = [];
afterEach(() => {
  for (const folder of folders.splice(0)) fs.rmSync(folder, { recursive: true, force: true });
});

/** An extension folder with the binaries given, each by its path from the folder. */
function extensionWith(...files: string[]): string {
  const folder = fs.mkdtempSync(path.join(os.tmpdir(), 'gitardo-binary-'));
  folders.push(folder);
  for (const file of files) {
    fs.mkdirSync(path.dirname(path.join(folder, file)), { recursive: true });
    fs.writeFileSync(path.join(folder, file), '');
  }
  return folder;
}

describe('findBinary', () => {
  it('names the binary with .exe on Windows only', () => {
    expect(binaryName('win32')).toBe('gitardo-engine.exe');
    expect(binaryName('linux')).toBe('gitardo-engine');
    expect(binaryName('darwin')).toBe('gitardo-engine');
  });

  it('takes the binary the vsix of the system holds', () => {
    const folder = extensionWith('bin/win32-x64/gitardo-engine.exe', 'bin/darwin-arm64/gitardo-engine');
    expect(findBinary(folder, undefined, 'win32', 'x64')).toBe(path.join(folder, 'bin', 'win32-x64', 'gitardo-engine.exe'));
    expect(findBinary(folder, '', 'darwin', 'arm64')).toBe(path.join(folder, 'bin', 'darwin-arm64', 'gitardo-engine'));
    expect(findBinary(folder, undefined, 'win32', 'arm64')).toBeUndefined();
  });

  it('on Linux takes the binary of the vsix for Alpine too, which is built for musl', () => {
    const alpine = extensionWith('bin/alpine-x64/gitardo-engine');
    expect(findBinary(alpine, undefined, 'linux', 'x64')).toBe(path.join(alpine, 'bin', 'alpine-x64', 'gitardo-engine'));
    const linux = extensionWith('bin/linux-x64/gitardo-engine');
    expect(findBinary(linux, undefined, 'linux', 'x64')).toBe(path.join(linux, 'bin', 'linux-x64', 'gitardo-engine'));
  });

  it('takes the binary the setting names, with .exe added on Windows when it was left out, and none that is not there', () => {
    const folder = extensionWith('build/gitardo-engine.exe');
    expect(findBinary(folder, 'build/gitardo-engine', 'win32', 'x64')).toBe(path.join(folder, 'build', 'gitardo-engine.exe'));
    expect(findBinary(folder, 'missing/gitardo-engine', 'win32', 'x64')).toBeUndefined();
  });

  it('has no binary in the universal vsix', () => {
    expect(findBinary(extensionWith(), undefined, 'linux', 'arm64')).toBeUndefined();
  });
});

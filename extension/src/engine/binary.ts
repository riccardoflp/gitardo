/**
 * Where `gitardo-engine` is: `gitardo.engine.path` when it is set, for development, or else the binary packed for this
 * system at `bin/<platform>-<arch>/` of the extension (`docs/transition/01-engine.md`, The process).
 */
import * as fs from 'node:fs';
import * as path from 'node:path';

/** The name of the binary on a system. */
export function binaryName(platform: NodeJS.Platform = process.platform): string {
  return platform === 'win32' ? 'gitardo-engine.exe' : 'gitardo-engine';
}

/**
 * The binary to start, or none when there is none: the path the setting names, with `.exe` added on Windows when it was
 * left out, else the one in the package.
 */
export function findBinary(
  extensionFolder: string,
  setting: string | undefined,
  platform: NodeJS.Platform = process.platform,
  arch = process.arch,
): string | undefined {
  const named = setting?.trim();
  if (named) {
    const absolute = path.resolve(extensionFolder, named);
    const candidates = platform === 'win32' && !absolute.toLowerCase().endsWith('.exe') ? [absolute, `${absolute}.exe`] : [absolute];
    return candidates.find((candidate) => fs.existsSync(candidate));
  }
  const packed = path.join(extensionFolder, 'bin', `${platform}-${arch}`, binaryName(platform));
  return fs.existsSync(packed) ? packed : undefined;
}

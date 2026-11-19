/**
 * The images of a commit in its page and in a comparison (`commit-page.images`, D74): which files are read as images,
 * and the type of an image by its first bytes, as the engine tells it (`file-view.read-image`).
 */

/** A side over this many bytes is not sent to the page. */
export const MOST_SENT = 10 * 1024 * 1024;

/** The endings of the names of the files read as images in a page, in any case (Q1301). */
const IMAGE_ENDINGS = ['.png', '.jpg', '.jpeg', '.gif', '.webp', '.bmp', '.ico', '.svg'];

/** Whether the name of a file ends as an image does, in any case. */
export function isImageName(path: string): boolean {
  const lower = path.toLowerCase();
  return IMAGE_ENDINGS.some((ending) => lower.endsWith(ending));
}

/** Whether the name of a file ends in `.svg`, in any case. */
export function isSvgName(path: string): boolean {
  return path.toLowerCase().endsWith('.svg');
}

/** The first bytes of each format, and its type. */
const SIGNATURES: [number[], string][] = [
  [[0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a], 'image/png'],
  [[0xff, 0xd8, 0xff], 'image/jpeg'],
  [[0x47, 0x49, 0x46, 0x38], 'image/gif'],
  [[0x42, 0x4d], 'image/bmp'],
  [[0x00, 0x00, 0x01, 0x00], 'image/x-icon'],
];

/** The type of an image by its first bytes, or for an SVG by its name; undefined for a file that is not an image. */
export function imageType(bytes: Uint8Array, path: string): string | undefined {
  const starts = (signature: number[], at = 0) => signature.every((byte, index) => bytes[at + index] === byte);
  const found = SIGNATURES.find(([signature]) => starts(signature));
  if (found) return found[1];
  const ascii = (text: string) => [...text].map((character) => character.charCodeAt(0));
  if (starts(ascii('RIFF')) && starts(ascii('WEBP'), 8)) return 'image/webp';
  return isSvgName(path) ? 'image/svg+xml' : undefined;
}

/** One version of an image a page draws. */
export type ImageSide =
  /** Its bytes as a `data:` URI, and its weight in bytes. */
  | { uri: string; weight: number }
  /** A version over 10 MB, which is not sent. */
  | { tooLarge: true; weight: number };

/** The two versions of an image file: null on a side that has none. */
export interface ImagePair {
  before: ImageSide | null;
  after: ImageSide | null;
}

/**
 * The two versions of a file as a page draws them, from their bytes and their paths, each as a `data:` URI unless it
 * is over 10 MB; null when neither side is an image. `encode` gives the base64 of bytes.
 */
export function imagePair(
  before: { bytes: Uint8Array; path: string } | null,
  after: { bytes: Uint8Array; path: string } | null,
  encode: (bytes: Uint8Array) => string,
): ImagePair | null {
  const side = (version: { bytes: Uint8Array; path: string } | null): ImageSide | null => {
    if (!version) return null;
    const weight = version.bytes.length;
    if (weight > MOST_SENT) return { tooLarge: true, weight };
    const type = imageType(version.bytes, version.path) ?? 'application/octet-stream';
    return { uri: `data:${type};base64,${encode(version.bytes)}`, weight };
  };
  const isImage = (version: { bytes: Uint8Array; path: string } | null) => !!version && imageType(version.bytes, version.path) !== undefined;
  if (!isImage(before) && !isImage(after)) return null;
  return { before: side(before), after: side(after) };
}

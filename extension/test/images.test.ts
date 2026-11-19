/** The images of a commit in its page and in a comparison (`commit-page.images`): which files, their types, and the 10 MB a side may weigh. */
import { describe, expect, it } from 'vitest';
import { MOST_SENT, imagePair, imageType, isImageName, isSvgName } from '../src/shared/images';

const base64 = (bytes: Uint8Array) => Buffer.from(bytes).toString('base64');
const png = new Uint8Array([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0, 0, 0, 13]);

describe('commit-page.images', () => {
  it('reads as images the files whose name ends as an image does, in any case', () => {
    expect(isImageName('media/Logo.PNG')).toBe(true);
    expect(isImageName('photo.jpeg')).toBe(true);
    expect(isImageName('data.bin')).toBe(false);
    expect(isSvgName('icons/check.SVG')).toBe(true);
  });

  it('tells an image by its first bytes, and an SVG by its name', () => {
    expect(imageType(png, 'logo.dat')).toBe('image/png');
    expect(imageType(new Uint8Array([0xff, 0xd8, 0xff, 0xe0]), 'photo')).toBe('image/jpeg');
    expect(imageType(new TextEncoder().encode('RIFF\0\0\0\0WEBPVP8 '), 'a')).toBe('image/webp');
    expect(imageType(new TextEncoder().encode('<svg/>'), 'check.svg')).toBe('image/svg+xml');
    expect(imageType(new TextEncoder().encode('hello'), 'notes.txt')).toBeUndefined();
  });

  it('a commit that changed a PNG gives its two versions as data URIs, with their weights', () => {
    const pair = imagePair({ bytes: png, path: 'logo.png' }, { bytes: png, path: 'logo.png' }, base64);
    expect(pair?.before).toEqual({ uri: `data:image/png;base64,${base64(png)}`, weight: png.length });
    expect(pair?.after).toEqual(pair?.before);
  });

  it('a PNG added has no version before', () => {
    const pair = imagePair(null, { bytes: png, path: 'new.png' }, base64);
    expect(pair?.before).toBeNull();
    expect(pair?.after).toMatchObject({ weight: png.length });
  });

  it('a side of 12 MB is not sent: it is too large to show here', () => {
    const large = new Uint8Array(12 * 1024 * 1024);
    large.set(png);
    expect(large.length).toBeGreaterThan(MOST_SENT);
    const pair = imagePair({ bytes: png, path: 'photo.png' }, { bytes: large, path: 'photo.png' }, base64);
    expect(pair?.after).toEqual({ tooLarge: true, weight: large.length });
  });

  it('a file that is an image on neither side gives nothing', () => {
    expect(imagePair({ bytes: new Uint8Array([1, 2]), path: 'a.png' }, null, base64)).toBeNull();
  });
});

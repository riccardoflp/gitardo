import { describe, expect, it } from 'vitest';
import { FrameReader, frame } from '../src/engine/framing';

/** The bytes cut in pieces of this many bytes, as a pipe would give them. */
function pieces(bytes: Buffer, size: number): Buffer[] {
  const cut: Buffer[] = [];
  for (let at = 0; at < bytes.length; at += size) cut.push(bytes.subarray(at, at + size));
  return cut;
}

describe('FrameReader', () => {
  it('gives a message of 4 MB that comes in pieces of 64 KB once, when its last piece comes', () => {
    const message = { jsonrpc: '2.0', id: 1, result: { rows: 'x'.repeat(4 * 1024 * 1024) } };
    const reader = new FrameReader();
    const given: unknown[] = [];
    const cut = pieces(frame(message), 64 * 1024);
    for (const [index, piece] of cut.entries()) {
      const messages = reader.push(piece);
      if (index < cut.length - 1) expect(messages).toEqual([]);
      given.push(...messages);
    }
    expect(given).toEqual([message]);
  });

  it('gives every message of one piece, in order, and keeps the start of the next', () => {
    const reader = new FrameReader();
    const bytes = Buffer.concat([frame({ id: 1 }), frame({ id: 2 }), frame({ id: 3 })]);
    const split = bytes.length - 5;
    expect(reader.push(bytes.subarray(0, split))).toEqual([{ id: 1 }, { id: 2 }]);
    expect(reader.push(bytes.subarray(split))).toEqual([{ id: 3 }]);
  });

  it('reads a header that comes in pieces of one byte', () => {
    const reader = new FrameReader();
    const given: unknown[] = [];
    for (const piece of pieces(frame({ text: 'àé ✓' }), 1)) given.push(...reader.push(piece));
    expect(given).toEqual([{ text: 'àé ✓' }]);
  });

  it('reads a body whose characters are cut between pieces', () => {
    const reader = new FrameReader();
    const message = { text: '✓'.repeat(1000) };
    const given: unknown[] = [];
    for (const piece of pieces(frame(message), 7)) given.push(...reader.push(piece));
    expect(given).toEqual([message]);
  });

  it('gives a message with an empty body as an error, and reads the next', () => {
    const reader = new FrameReader();
    const messages = reader.push(Buffer.concat([Buffer.from('Content-Length: 0\r\n\r\n'), frame({ id: 2 })]));
    expect(messages[0]).toBeInstanceOf(Error);
    expect(messages[1]).toEqual({ id: 2 });
  });

  it('gives a body that is not JSON as an error, and reads the next', () => {
    const reader = new FrameReader();
    const messages = reader.push(Buffer.concat([Buffer.from('Content-Length: 3\r\n\r\n{x}'), frame({ id: 2 })]));
    expect(messages).toHaveLength(2);
    expect(messages[0]).toBeInstanceOf(Error);
    expect(messages[1]).toEqual({ id: 2 });
  });

  it('gives a header with no Content-Length as an error, drops what came with it, and reads what comes after', () => {
    const reader = new FrameReader();
    const messages = reader.push(Buffer.from('Content-Type: text\r\n\r\n{"id":1}'));
    expect(messages).toHaveLength(1);
    expect(String(messages[0])).toContain('has no Content-Length');
    expect(reader.push(frame({ id: 2 }))).toEqual([{ id: 2 }]);
  });

  it('reads the header whatever the case of its name, with other headers beside it', () => {
    const reader = new FrameReader();
    const body = Buffer.from('{"id":4}');
    const bytes = Buffer.concat([Buffer.from(`content-type: application/json\r\ncontent-length: ${body.length}\r\n\r\n`), body]);
    expect(reader.push(bytes)).toEqual([{ id: 4 }]);
  });
});

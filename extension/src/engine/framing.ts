/**
 * The framing of the messages between the extension and `gitardo-engine`, as the Language Server Protocol frames them:
 * a `Content-Length` header, an empty line, then the JSON in UTF-8 (`docs/transition/02-protocol.md`, Transport).
 */

/** The bytes of one message with its header. */
export function frame(message: unknown): Buffer {
  const body = Buffer.from(JSON.stringify(message), 'utf8');
  return Buffer.concat([Buffer.from(`Content-Length: ${body.length}\r\n\r\n`, 'ascii'), body]);
}

/**
 * Takes the bytes of the engine's output as they come, and gives each message once all of it has come. The pieces of a
 * long message are kept as they came and joined once, so that a state of several megabytes read in pieces of 64 KB is
 * copied once, not once for every piece.
 */
export class FrameReader {
  /** The bytes that came and are not yet given, in the order they came. */
  private pieces: Buffer[] = [];
  /** How many bytes `pieces` holds. */
  private buffered = 0;
  /** The length of the body of the message that is coming, once its header has been read. */
  private bodySize: number | undefined;

  /** The messages the bytes so far complete, in order; a message that cannot be read as JSON is given as an error. */
  push(chunk: Buffer): Array<unknown | Error> {
    if (chunk.length > 0) {
      this.pieces.push(chunk);
      this.buffered += chunk.length;
    }
    const messages: Array<unknown | Error> = [];
    for (;;) {
      if (this.bodySize === undefined) {
        // A header is a few bytes, so joining what came to find its end costs little.
        const head = this.joined();
        const end = head.indexOf('\r\n\r\n');
        if (end < 0) break;
        const headers = head.subarray(0, end).toString('ascii');
        const length = /^content-length:\s*(\d+)\s*$/im.exec(headers);
        if (!length) {
          // A header with no length cannot be framed; what is left is dropped with it, and the error says so.
          this.keep(Buffer.alloc(0));
          messages.push(new Error(`A message of Gitardo's engine has no Content-Length: ${headers}`));
          break;
        }
        this.bodySize = Number(length[1]);
        this.keep(head.subarray(end + 4));
      }
      if (this.buffered < this.bodySize) break;
      const bytes = this.joined();
      const body = bytes.subarray(0, this.bodySize).toString('utf8');
      this.keep(bytes.subarray(this.bodySize));
      this.bodySize = undefined;
      try {
        messages.push(JSON.parse(body));
      } catch (error) {
        messages.push(error instanceof Error ? error : new Error(String(error)));
      }
    }
    return messages;
  }

  /** What came and is not yet given, as one buffer, kept as the only piece. */
  private joined(): Buffer {
    if (this.pieces.length === 1) return this.pieces[0];
    const bytes = Buffer.concat(this.pieces, this.buffered);
    this.pieces = [bytes];
    return bytes;
  }

  /** Keeps these bytes alone, as what came and is not yet given. */
  private keep(bytes: Buffer): void {
    this.pieces = bytes.length > 0 ? [bytes] : [];
    this.buffered = bytes.length;
  }
}

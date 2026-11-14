/**
 * The framing of the messages between the extension and `gitardo-engine`, as the Language Server Protocol frames them:
 * a `Content-Length` header, an empty line, then the JSON in UTF-8 (`docs/transition/02-protocol.md`, Transport).
 */

/** The bytes of one message with its header. */
export function frame(message: unknown): Buffer {
  const body = Buffer.from(JSON.stringify(message), 'utf8');
  return Buffer.concat([Buffer.from(`Content-Length: ${body.length}\r\n\r\n`, 'ascii'), body]);
}

/** Takes the bytes of the engine's output as they come, and gives each message once all of it has come. */
export class FrameReader {
  private pending = Buffer.alloc(0);

  /** The messages the bytes so far complete, in order; a message that cannot be read as JSON is given as an error. */
  push(chunk: Buffer): Array<unknown | Error> {
    this.pending = Buffer.concat([this.pending, chunk]);
    const messages: Array<unknown | Error> = [];
    for (;;) {
      const end = this.pending.indexOf('\r\n\r\n');
      if (end < 0) break;
      const headers = this.pending.subarray(0, end).toString('ascii');
      const length = /^content-length:\s*(\d+)\s*$/im.exec(headers);
      if (!length) {
        // A header with no length cannot be framed; what is left is dropped with it, and the error says so.
        this.pending = Buffer.alloc(0);
        messages.push(new Error(`A message of Gitardo's engine has no Content-Length: ${headers}`));
        break;
      }
      const start = end + 4;
      const size = Number(length[1]);
      if (this.pending.length < start + size) break;
      const body = this.pending.subarray(start, start + size).toString('utf8');
      this.pending = this.pending.subarray(start + size);
      try {
        messages.push(JSON.parse(body));
      } catch (error) {
        messages.push(error instanceof Error ? error : new Error(String(error)));
      }
    }
    return messages;
  }
}

/**
 * JSON-RPC 2.0 over the standard input and output of `gitardo-engine`: requests with their answers, notifications both
 * ways, and `$/cancel` for a request no longer wanted (`docs/transition/02-protocol.md`).
 */
import type { Readable, Writable } from 'node:stream';
import { FrameReader, frame } from './framing';

/** The codes of the errors the engine answers with. */
export const ErrorCode = {
  parseError: -32700,
  invalidRequest: -32600,
  methodNotFound: -32601,
  invalidParams: -32602,
  internalError: -32603,
  notOpen: 1,
  gitFailed: 2,
  stepFailed: 3,
  answerRefused: 4,
  endlessFeature: 5,
  busy: 6,
  cancelled: 7,
  unknownFeature: 8,
} as const;

/** An error the engine answered a request with, or the end of the engine while the request waited. */
export class EngineError extends Error {
  constructor(
    readonly code: number,
    message: string,
    readonly data?: unknown,
  ) {
    super(message);
    this.name = 'EngineError';
  }
}

/** The code an `EngineError` has when the engine ended before it answered. */
export const ENDED = -1;

type Waiting = {
  resolve: (result: unknown) => void;
  reject: (error: EngineError) => void;
};

/** One conversation with the engine, over its standard input and output. */
export class Connection {
  private nextId = 1;
  private readonly waiting = new Map<number, Waiting>();
  private readonly handlers = new Map<string, (params: unknown) => void>();
  private readonly reader = new FrameReader();
  private ended = false;

  constructor(
    private readonly input: Writable,
    output: Readable,
    private readonly onUnreadable: (error: Error) => void = () => {},
  ) {
    output.on('data', (chunk: Buffer) => {
      for (const message of this.reader.push(chunk)) {
        if (message instanceof Error) this.onUnreadable(message);
        else this.take(message);
      }
    });
    output.on('close', () => this.end());
    input.on('error', () => this.end());
  }

  /**
   * Sends a request and gives its answer. When `signal` is aborted before the answer, the engine is sent `$/cancel`,
   * and the request ends with the engine's answer, `cancelled` for a read.
   */
  request<T>(method: string, params: unknown, signal?: AbortSignal): Promise<T> {
    if (this.ended) return Promise.reject(new EngineError(ENDED, "Gitardo's engine is not running."));
    const id = this.nextId++;
    const answer = new Promise<T>((resolve, reject) => {
      this.waiting.set(id, {
        resolve: (result) => resolve(result as T),
        reject,
      });
    });
    this.write({ jsonrpc: '2.0', id, method, params });
    if (signal) {
      const cancel = () => this.notify('$/cancel', { id });
      if (signal.aborted) cancel();
      else signal.addEventListener('abort', cancel, { once: true });
      void answer.finally(() => signal.removeEventListener('abort', cancel)).catch(() => {});
    }
    return answer;
  }

  /** Sends a notification. */
  notify(method: string, params: unknown): void {
    if (!this.ended) this.write({ jsonrpc: '2.0', method, params });
  }

  /** Gives each notification of a method to a handler, in place of the one before. */
  on(method: string, handler: (params: unknown) => void): void {
    this.handlers.set(method, handler);
  }

  /** Fails every request still waiting: the engine ended. */
  end(): void {
    if (this.ended) return;
    this.ended = true;
    for (const waiting of this.waiting.values()) waiting.reject(new EngineError(ENDED, "Gitardo's engine ended before it answered."));
    this.waiting.clear();
  }

  private write(message: unknown): void {
    this.input.write(frame(message));
  }

  private take(message: unknown): void {
    if (typeof message !== 'object' || message === null) return;
    const { id, method, params, result, error } = message as {
      id?: number | string | null;
      method?: string;
      params?: unknown;
      result?: unknown;
      error?: { code: number; message: string; data?: unknown };
    };
    if (typeof method === 'string') {
      // The engine of stage 1 asks the extension nothing; a request of it would be answered here.
      if (id === undefined || id === null) this.handlers.get(method)?.(params);
      return;
    }
    if (typeof id !== 'number') {
      if (error) this.onUnreadable(new Error(`Gitardo's engine could not read a message: ${error.message}`));
      return;
    }
    const waiting = this.waiting.get(id);
    if (!waiting) return;
    this.waiting.delete(id);
    if (error) waiting.reject(new EngineError(error.code, error.message, error.data));
    else waiting.resolve(result);
  }
}

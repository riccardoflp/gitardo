/**
 * The extension's side of `gitardo-engine`: it starts the binary, says `initialize`, opens the repositories, and
 * starts it again when it ends, as `docs/transition/01-engine.md` says: once more after each exit, until a third exit
 * within five minutes stops the restarts until the window is reloaded. It imports nothing of VS Code, so that the
 * tests drive it as the extension does.
 */
import { type ChildProcess, spawn } from 'node:child_process';
import { Connection, ENDED, EngineError, ErrorCode } from './connection';
import {
  type EngineState,
  type InitializeParams,
  type InitializeResult,
  type LogLineParams,
  Method,
  PROTOCOL,
  type Settings,
  type StateParams,
} from './messages';

/** How long the exits are counted for. */
export const RESTART_WINDOW_MS = 5 * 60 * 1000;
/** The exit within that time that stops the restarts. */
export const EXITS_THAT_STOP = 3;

/** Where the client says what happens: the **Gitardo** output channel in the extension. */
export interface EngineLog {
  info(line: string): void;
  warn(line: string): void;
  error(line: string): void;
  debug(line: string): void;
}

/** What the client needs to start the engine. */
export interface EngineOptions {
  /** The binary to start. */
  binary: string;
  /** The extension's version, said in `initialize`. */
  clientVersion: string;
  /** The settings as they are now, read at each start. */
  settings: () => Settings;
  /** Whether the log says the reads too. */
  logLevel: () => 'info' | 'debug';
  log: EngineLog;
  /** The time now; the tests give their own. */
  now?: () => number;
}

/** Where the client stands. */
export type EngineStatus = 'stopped' | 'starting' | 'running' | 'restarting' | 'given up' | 'refused';

/** `gitardo-engine` for one window of VS Code. */
export class EngineClient {
  private process?: ChildProcess;
  private connection?: Connection;
  private started?: Promise<InitializeResult>;
  private readonly opened = new Set<string>();
  private readonly exits: number[] = [];
  private readonly stateHandlers = new Set<(repository: string, state: EngineState) => void>();
  private stopping = false;
  private current: EngineStatus = 'stopped';

  constructor(private readonly options: EngineOptions) {}

  /** Where the client stands. */
  get status(): EngineStatus {
    return this.current;
  }

  /** The process id of the engine that runs, for the tests that end it. */
  get pid(): number | undefined {
    return this.process?.pid;
  }

  /** Starts the engine once, and gives what it answered to `initialize`. */
  start(): Promise<InitializeResult> {
    this.started ??= this.launch();
    return this.started;
  }

  /** Opens a repository, and gives its state; it is opened again after a restart. */
  async open(repository: string, signal?: AbortSignal): Promise<EngineState> {
    const connection = await this.ready();
    const { state } = await connection.request<{ state: EngineState }>(Method.open, { repository }, signal);
    this.opened.add(repository);
    return state;
  }

  /** Closes a repository opened. */
  async close(repository: string): Promise<void> {
    this.opened.delete(repository);
    const connection = await this.ready();
    await connection.request(Method.close, { repository });
  }

  /** Reads the state of a repository opened. */
  async readState(repository: string, signal?: AbortSignal): Promise<EngineState> {
    const connection = await this.ready();
    return connection.request<EngineState>(Method.readState, { repository }, signal);
  }

  /** Says which paths the watchers saw change in a repository opened. */
  changed(repository: string, paths: string[]): void {
    this.connection?.notify(Method.changed, { repository, paths });
  }

  /** Calls a handler with each state the engine sends after files changed; disposing it stops that. */
  onState(handler: (repository: string, state: EngineState) => void): {
    dispose(): void;
  } {
    this.stateHandlers.add(handler);
    return { dispose: () => this.stateHandlers.delete(handler) };
  }

  /** Asks the engine to end, and ends it when it does not within a second. */
  async dispose(): Promise<void> {
    this.stopping = true;
    const child = this.process;
    const connection = this.connection;
    if (!child || !connection) return;
    const ended = new Promise<void>((resolve) => child.once('exit', () => resolve()));
    try {
      await Promise.race([connection.request(Method.shutdown, null), delay(1000)]);
      connection.notify(Method.exit, null);
    } catch {
      // An engine that cannot answer is ended below.
    }
    await Promise.race([ended, delay(1000)]);
    if (child.exitCode === null && child.signalCode === null) child.kill();
    this.current = 'stopped';
  }

  private async ready(): Promise<Connection> {
    await this.start();
    if (!this.connection) throw new EngineError(ENDED, "Gitardo's engine is not running.");
    return this.connection;
  }

  /** Starts the process and says `initialize`; a major version that differs ends it there. */
  private async launch(): Promise<InitializeResult> {
    this.current = this.exits.length ? 'restarting' : 'starting';
    const { log } = this.options;
    const child = spawn(this.options.binary, ['--stdio'], {
      stdio: ['pipe', 'pipe', 'pipe'],
      windowsHide: true,
    });
    this.process = child;
    const stdin = child.stdin;
    const stdout = child.stdout;
    if (!stdin || !stdout) throw new EngineError(ENDED, "Gitardo's engine could not be started.");
    const connection = new Connection(stdin, stdout, (error) => log.warn(error.message));
    this.connection = connection;
    child.stderr?.on('data', (chunk: Buffer) => {
      for (const line of chunk.toString('utf8').split(/\r?\n/)) if (line.trim()) log.warn(`[engine] ${line}`);
    });
    connection.on(Method.logLine, (params) => this.logLine(params as LogLineParams));
    connection.on(Method.state, (params) => {
      const { repository, state } = params as StateParams;
      for (const handler of this.stateHandlers) handler(repository, state);
    });
    const failed = new Promise<never>((_, reject) => {
      child.once('error', (error) => reject(new EngineError(ENDED, `Gitardo's engine could not be started: ${error.message}`)));
    });
    failed.catch(() => {});
    child.once('exit', (code, signal) => this.exited(child, code, signal));

    const params: InitializeParams = {
      protocol: PROTOCOL,
      client: { name: 'gitardo-vscode', version: this.options.clientVersion },
      settings: this.options.settings(),
      host: {},
      logLevel: this.options.logLevel(),
    };
    try {
      const answer = await Promise.race([connection.request<InitializeResult>(Method.initialize, params), failed]);
      this.current = 'running';
      log.info(`Gitardo's engine ${answer.engine.version} started (protocol ${answer.protocol}, ${answer.engine.git ?? 'git not found'}).`);
      return answer;
    } catch (error) {
      if (error instanceof EngineError && error.code === ErrorCode.invalidParams && error.data) {
        const engine = error.data as InitializeResult;
        this.current = 'refused';
        this.stopping = true;
        log.error(
          `Gitardo's engine ${engine.engine.version} speaks version ${engine.protocol} of the protocol, and this extension ${PROTOCOL}: it is not used.`,
        );
        child.kill();
      }
      throw error;
    }
  }

  /** Counts an exit, and starts the engine again unless it was asked to end or ended three times in five minutes. */
  private exited(child: ChildProcess, code: number | null, signal: NodeJS.Signals | null): void {
    if (this.process !== child) return;
    this.connection?.end();
    this.connection = undefined;
    this.process = undefined;
    if (this.stopping) return;
    const now = (this.options.now ?? Date.now)();
    this.exits.push(now);
    while (this.exits.length && now - this.exits[0] > RESTART_WINDOW_MS) this.exits.shift();
    const how = signal ? `was ended by ${signal}` : `exited with code ${code}`;
    if (this.exits.length >= EXITS_THAT_STOP) {
      this.current = 'given up';
      this.started = Promise.reject(new EngineError(ENDED, "Gitardo's engine stopped too often and is not started again."));
      this.started.catch(() => {});
      this.options.log.error(
        `Gitardo's engine ${how}, the ${ordinal(this.exits.length)} time in five minutes: it is not started again until the window is reloaded.`,
      );
      return;
    }
    this.options.log.warn(`Gitardo's engine ${how}; starting it again.`);
    this.started = this.launch();
    void this.started.then(
      () => this.reopen(),
      (error: unknown) => this.options.log.error(error instanceof Error ? error.message : String(error)),
    );
  }

  /** Opens again the repositories that were open before a restart. */
  private async reopen(): Promise<void> {
    for (const repository of [...this.opened]) {
      try {
        await this.open(repository);
      } catch (error) {
        // An engine that ended again says so itself, as its exit.
        if (error instanceof EngineError && error.code === ENDED) return;
        this.options.log.warn(`Gitardo's engine could not open ${repository} again: ${error instanceof Error ? error.message : String(error)}`);
      }
    }
  }

  private logLine({ repository, entry }: LogLineParams): void {
    const where = repository ? ` (${repository})` : '';
    if (typeof entry.kind === 'object') this.options.log.debug(`[engine] ${entry.line}${where}, ${entry.kind.milliseconds} ms`);
    else if (entry.kind === 'failure') this.options.log.error(`[engine] ${entry.line}${where}`);
    else if (entry.kind === 'warning') this.options.log.warn(`[engine] ${entry.line}${where}`);
    else this.options.log.info(`[engine] ${entry.line}${where}`);
  }
}

function delay(milliseconds: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, milliseconds));
}

function ordinal(count: number): string {
  return ['first', 'second', 'third'][count - 1] ?? `${count}th`;
}

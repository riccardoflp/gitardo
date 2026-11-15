import { NO_ENGINE, type HostMessage } from '../shared/protocol';
import { post } from './vscode';

/** What a view has been told: nothing yet, a state or none, or that there is no engine to read one with. */
export type Loaded = boolean | 'noEngine';

/** What a message with the state tells a view it has been told. */
export function loadedOf(message: Extract<HostMessage, { type: 'state' }>): Loaded {
  return !message.state && message.engineMissing ? 'noEngine' : true;
}

/**
 * What a view shows in place of a repository: that it is being read, that Gitardo's engine is not there to read it
 * (D49), with the log that says why, or that there is none.
 */
export function Placeholder({ loaded }: { loaded: Loaded }) {
  if (!loaded) return <div className="empty">Loading repository…</div>;
  if (loaded === 'noEngine') {
    return (
      <div className="empty">
        <p>{NO_ENGINE}</p>
        <button className="button" onClick={() => post({ type: 'command', command: 'showLog' })}>
          Show the log
        </button>
      </div>
    );
  }
  return (
    <div className="empty">
      <p>No Git repository in this workspace</p>
      <button className="button" onClick={() => post({ type: 'refresh' })}>
        Check again
      </button>
    </div>
  );
}

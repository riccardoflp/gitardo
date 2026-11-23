import type { HostMessage } from '../shared/protocol';
import { ALL_PARTS, type ShownState, withParts } from '../shared/stateParts';

/** The state this page was last sent, the parts it was sent put together: each page is one view. */
let held: ShownState | null = null;

/**
 * The state of the repository as this view has it once it is sent this message: the parts the message carries in place
 * of the ones it had, and the others as they were, the same objects (D88). A message that names no parts is a state whole.
 */
export function receive(message: Extract<HostMessage, { type: 'state' }>): ShownState | null {
  held = message.state ? withParts(held, message.state, message.parts ?? ALL_PARTS, message.kept) : null;
  return held;
}

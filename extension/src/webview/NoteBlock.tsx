import type { ReactNode } from 'react';
import { Icon } from './Icon';
import { act } from './menus';

/**
 * The note of a commit under its message (`note.show`): **Note**, with the pencil of **Edit note…** and the bin of
 * **Remove note** at its right, run by the engine, then its text as it was written. Where the commit is of another
 * repository than the one the views show, the note is only shown.
 */
export function NoteBlock({ hash, editable, children }: { hash: string; editable: boolean; children: ReactNode }) {
  return (
    <section className="note" aria-label="Note">
      <div className="note-head">
        <h3>Note</h3>
        {editable && (
          <>
            <button className="button icon" onClick={() => act({ kind: 'editNote', hash })} aria-label="Edit note" title="Edit note…">
              <Icon name="edit" />
            </button>
            <button className="button icon" onClick={() => act({ kind: 'removeNote', hash })} aria-label="Remove note" title="Remove note">
              <Icon name="trash" />
            </button>
          </>
        )}
      </div>
      {children}
    </section>
  );
}

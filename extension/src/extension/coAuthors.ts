import * as vscode from 'vscode';
import { coAuthorEntries, type CoAuthorEntry } from '../shared/commitOptions';
import type { Author } from '../shared/state';

type Item = vscode.QuickPickItem & { entry: CoAuthorEntry };

/**
 * The quick pick of many of **Co-authors of this commit** (`commit.co-authors`): the authors of the history, given as
 * they are read, those chosen already ticked, and someone typed as `Name <email>` offered first. The co-authors ticked
 * when it is accepted; undefined when it is closed with `Escape`, which leaves them as they were.
 */
export function pickCoAuthors(authors: Promise<Author[]>, identityEmail: string | null, chosen: string[]): Promise<string[] | undefined> {
  const pick = vscode.window.createQuickPick<Item>();
  pick.title = 'Co-authors of this commit';
  pick.placeholder = 'Type a name, or Name <email> to add someone';
  pick.canSelectMany = true;
  pick.matchOnDescription = true;
  pick.busy = true;
  let read: Author[] = [];
  let ticked = new Set(chosen);
  const show = () => {
    const items = coAuthorEntries(read, identityEmail, [...ticked], pick.value).map((entry) => ({
      label: entry.label,
      description: entry.description,
      // The one typed is listed whatever matches, so that it is not filtered out by its own words.
      alwaysShow: entry.typed,
      entry,
    }));
    pick.items = items;
    pick.selectedItems = items.filter((item) => ticked.has(item.entry.coAuthor));
  };
  show();
  return new Promise((resolve) => {
    let accepted = false;
    pick.onDidChangeSelection((selected) => {
      ticked = new Set(selected.map((item) => item.entry.coAuthor));
    });
    pick.onDidChangeValue(show);
    pick.onDidAccept(() => {
      accepted = true;
      pick.hide();
    });
    pick.onDidHide(() => {
      pick.dispose();
      resolve(accepted ? [...ticked] : undefined);
    });
    authors.then(
      (found) => {
        read = found;
        pick.busy = false;
        show();
      },
      () => {
        // Without the authors of the history, someone typed can still be added.
        pick.busy = false;
      },
    );
    pick.show();
  });
}

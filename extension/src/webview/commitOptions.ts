import { canSign, type CommitOptions, signsWith } from '../shared/commitOptions';
import type { Identity, Signing } from '../shared/state';
import type { MenuEntry } from './ContextMenu';

/** What the menu of **Commit options** is drawn from, and what its entries do. */
export interface CommitOptionsMenu {
  options: CommitOptions;
  signing: Signing;
  identity: Identity;
  /** Gives the options as they are after an entry was chosen. */
  change(options: CommitOptions): void;
  /** Opens the quick pick of the co-authors. */
  pickCoAuthors(): void;
  /** Opens **Switch identity…**, where a profile sets a key to sign with. */
  switchIdentity(): void;
}

/**
 * The menu of **Commit options** beside **Commit** (`commit.options`): **Sign this commit**, **Add Signed-off-by**,
 * **Co-authors…** with the co-authors chosen under it, each taken away by a click, and **Skip the hooks**. What each
 * signs with, adds or does is its tooltip. When git has no key to sign with, **Sign this commit** cannot be ticked and
 * **Switch identity…** is under it.
 */
export function commitOptionsMenu(menu: CommitOptionsMenu): MenuEntry[] {
  const { options, signing, identity, change } = menu;
  const signer = identity.name || identity.email ? `${identity.name ?? ''} <${identity.email ?? ''}>` : 'your identity';
  return [
    {
      label: 'Sign this commit',
      checked: options.sign,
      disabled: !canSign(signing),
      title: signsWith(signing),
      run: () => change({ ...options, sign: !options.sign }),
    },
    !canSign(signing) && { label: 'Switch identity…', title: 'No key to sign with: set one with Switch identity…', run: menu.switchIdentity },
    {
      label: 'Add Signed-off-by',
      checked: options.signOff,
      title: `Signed-off-by: ${signer}`,
      run: () => change({ ...options, signOff: !options.signOff }),
    },
    'separator',
    { label: 'Co-authors…', title: 'Choose who made this commit with you', run: menu.pickCoAuthors },
    ...options.coAuthors.map((coAuthor) => ({
      label: coAuthor,
      hint: '×',
      title: 'Remove from the co-authors',
      run: () => change({ ...options, coAuthors: options.coAuthors.filter((other) => other !== coAuthor) }),
    })),
    'separator',
    {
      label: 'Skip the hooks',
      checked: options.skipHooks,
      title: 'Commit without running the pre-commit and commit-msg hooks',
      run: () => change({ ...options, skipHooks: !options.skipHooks }),
    },
  ];
}

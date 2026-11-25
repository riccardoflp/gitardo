import { useLayoutEffect, type RefObject } from 'react';

/**
 * What the toolbar of the graph gives up when it is short of room, in this order, each step with the ones before it:
 * the words of the button of the branches shown; the words of the other buttons; **Refresh**, which **…** has too; one
 * line, the search and what follows it going to a second; **Branch**, **Stash** and **Undo**, which go into **…**; and
 * last the first line, which breaks between where one is and the buttons.
 */
export const GIVE = { scopeWords: 1, words: 2, refresh: 3, secondLine: 4, actions: 5, firstLine: 6 } as const;

const LAST = GIVE.firstLine;

/** Whether what a line holds is wider than the line. */
const overflows = (line: Element) => line.scrollWidth > line.clientWidth + 1;

/**
 * Gives up as little as makes the toolbar fit, and says what in its `data-give`, as the steps taken: `1 2` for the
 * words alone. It starts from nothing given up each time, so that the words come back as soon as they fit.
 */
export function fit(bar: HTMLElement): number {
  for (let step = 0; ; step += 1) {
    bar.dataset.give = Array.from({ length: step }, (_, index) => index + 1).join(' ');
    if (step === LAST) return step;
    // On one line the toolbar itself is measured; on two, each line.
    const lines = step < GIVE.secondLine ? [bar] : [...bar.querySelectorAll(':scope > .line')];
    if (!lines.some(overflows)) return step;
  }
}

/** How many steps a toolbar has given up, as `fit` left it. */
export const given = (bar: HTMLElement | null) => (bar?.dataset.give ? bar.dataset.give.split(' ').length : 0);

/**
 * Keeps a toolbar fitted: after each time it is drawn, since a name, a count or what is running changes what it holds,
 * when its width changes, and once the font of the icons is loaded.
 */
export function useGive(bar: RefObject<HTMLElement | null>): void {
  useLayoutEffect(() => {
    if (bar.current) fit(bar.current);
  });
  useLayoutEffect(() => {
    const element = bar.current;
    if (!element) return;
    let width = element.clientWidth;
    // Fitting changes the height, which is nothing to fit again for.
    const observer = new ResizeObserver(() => {
      if (element.clientWidth === width) return;
      width = element.clientWidth;
      fit(element);
    });
    observer.observe(element);
    let live = true;
    void document.fonts?.ready.then(() => live && fit(element));
    return () => {
      live = false;
      observer.disconnect();
    };
  }, [bar]);
}

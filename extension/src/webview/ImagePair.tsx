import { useEffect, useState, type ReactNode } from 'react';
import type { ImagePair as Pair, ImageSide } from '../shared/images';
import type { DiffSource, FileChange } from '../shared/protocol';
import { sizeText } from '../shared/worktreeSetup';
import { onMessage, post } from './vscode';

/** Whether two sources of diffs are the same. */
const sameSource = (one: DiffSource, other: DiffSource) => JSON.stringify(one) === JSON.stringify(other);

/** One version of an image: its picture with its size and weight under it, **Empty**, or that it is too large. */
function Side({ side, onOpen }: { side: ImageSide | null; onOpen(): void }) {
  const [size, setSize] = useState<string>();
  if (!side) return <div className="image-side muted">Empty</div>;
  if ('tooLarge' in side) {
    return (
      <div className="image-side muted">
        Too large to show here.{' '}
        <button className="link" onClick={onOpen}>
          Open it in the diff editor
        </button>
      </div>
    );
  }
  return (
    <figure className="image-side">
      <button className="image-picture" title="Open in the diff editor" onClick={onOpen}>
        <img src={side.uri} alt="" onLoad={(event) => setSize(`${event.currentTarget.naturalWidth} × ${event.currentTarget.naturalHeight}`)} />
      </button>
      <figcaption className="muted">{[size, sizeText(side.weight)].filter(Boolean).join(' · ')}</figcaption>
    </figure>
  );
}

interface Props {
  source: DiffSource;
  file: FileChange;
  /** Opens the file in the diff editor. */
  onOpen(): void;
  /** Said when neither version is an image after all. */
  notImages: ReactNode;
}

/**
 * The version before and the version after of an image file, side by side under the header of its diff, read once it
 * is shown (`commit-page.images`): as pictures of `data:` URIs, in which nothing of an SVG runs and nothing is fetched.
 */
export function ImagePair({ source, file, onOpen, notImages }: Props) {
  // Undefined while they are read; null when neither is an image.
  const [images, setImages] = useState<Pair | null>();
  // The page makes its source anew at each drawing: the images are asked for again only when it says another.
  const key = JSON.stringify(source);
  useEffect(() => {
    const stop = onMessage((message) => {
      if (message.type === 'images' && message.path === file.path && sameSource(message.source, source)) setImages(message.images);
    });
    post({ type: 'readImages', source, file });
    return stop;
  }, [key, file]);
  if (images === undefined) return <p className="muted diff-note">Reading the images…</p>;
  if (images === null) return notImages;
  return (
    <div className="image-pair" aria-label={`The images of ${file.path}`}>
      <Side side={images.before} onOpen={onOpen} />
      <Side side={images.after} onOpen={onOpen} />
    </div>
  );
}

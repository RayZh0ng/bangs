import type { CSSProperties } from "react";

import type { LyricWord } from "../lib/native";

/**
 * The line being sung, filled in from left to right. QRC gives a timing per
 * character; plain LRC only marks the start of the line, so the sweep is then
 * spread evenly across it.
 *
 * The fill is painted through the glyphs themselves rather than layered over a
 * second copy of the text: a copy needs a box per character, and Chromium
 * rounds those boxes to whole pixels, which pushed the characters apart on
 * Windows.
 */
export function KaraokeLine({
  text,
  words,
  from,
  to,
  elapsed,
  className = "",
}: {
  text: string;
  words?: LyricWord[];
  /** Seconds at which this line starts. */
  from: number;
  /** Seconds at which the next line starts. */
  to: number;
  elapsed: number;
  className?: string;
}) {
  if (words?.length) {
    return (
      <span className={`karaoke ${className}`}>
        {words.map((word, index) => (
          <Sung key={index} text={word.text} sung={clamp((elapsed - word.at) / Math.max(word.duration, 0.05))} />
        ))}
      </span>
    );
  }

  const span = Math.max(0.3, to - from);
  return (
    <span className={`karaoke ${className}`}>
      <Sung text={text} sung={clamp((elapsed - from) / span)} />
    </span>
  );
}

/** Text filled from the left to `sung`, a fraction of its own width. */
function Sung({ text, sung }: { text: string; sung: number }) {
  return (
    <span className="karaoke__sung" style={{ "--sung": `${sung * 100}%` } as CSSProperties}>
      {text}
    </span>
  );
}

const clamp = (value: number) => Math.min(1, Math.max(0, value));

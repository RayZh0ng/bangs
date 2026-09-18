import type { LyricWord } from "../lib/native";

/**
 * The line being sung, filled in from left to right. QRC gives a timing per
 * character; plain LRC only marks the start of the line, so the sweep is then
 * spread evenly across it.
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
          <Word key={index} word={word} elapsed={elapsed} />
        ))}
      </span>
    );
  }

  const span = Math.max(0.3, to - from);
  const sung = clamp((elapsed - from) / span);

  return (
    <span className={`karaoke ${className}`}>
      <span className="karaoke__base">{text}</span>
      <span className="karaoke__sung" style={{ width: `${sung * 100}%` }} aria-hidden>
        {text}
      </span>
    </span>
  );
}

function Word({ word, elapsed }: { word: LyricWord; elapsed: number }) {
  const sung = clamp((elapsed - word.at) / Math.max(word.duration, 0.05));
  if (sung >= 1) return <span>{word.text}</span>;
  if (sung <= 0) return <span className="karaoke__base">{word.text}</span>;

  return (
    <span className="karaoke__word">
      <span className="karaoke__base">{word.text}</span>
      <span className="karaoke__sung" style={{ width: `${sung * 100}%` }} aria-hidden>
        {word.text}
      </span>
    </span>
  );
}

const clamp = (value: number) => Math.min(1, Math.max(0, value));

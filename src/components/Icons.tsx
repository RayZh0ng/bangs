import type { SVGProps } from "react";

type IconProps = SVGProps<SVGSVGElement>;

function Stroke({ children, ...props }: IconProps) {
  return (
    <svg
      viewBox="0 0 24 24"
      width={16}
      height={16}
      fill="none"
      stroke="currentColor"
      strokeWidth={2}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden
      {...props}
    >
      {children}
    </svg>
  );
}

function Solid({ children, ...props }: IconProps) {
  return (
    <svg viewBox="0 0 24 24" width={16} height={16} fill="currentColor" aria-hidden {...props}>
      {children}
    </svg>
  );
}

// The tab glyphs are drawn for 13px: few strokes, nothing smaller than three units.
export const MusicIcon = (props: IconProps) => (
  <Stroke {...props}>
    <path d="M4.5 13.5v-3M9.5 17V7M14.5 19.5v-15M19.5 14.5v-5" />
  </Stroke>
);

export const ShelfIcon = (props: IconProps) => (
  <Stroke {...props}>
    <path d="M12 3.5v9m0 0 3.3-3.3M12 12.5 8.7 9.2" />
    <path d="M4 15v3.2A1.8 1.8 0 0 0 5.8 20h12.4a1.8 1.8 0 0 0 1.8-1.8V15" />
  </Stroke>
);

export const PinIcon = ({ filled, ...props }: IconProps & { filled?: boolean }) => (
  <Stroke {...props} fill={filled ? "currentColor" : "none"}>
    <path d="M12 16.4V21" />
    <path d="M8.4 3.2h7.2" />
    <path d="M9.9 3.2v6.4l-2.1 3.1a1 1 0 0 0 .83 1.56h6.74a1 1 0 0 0 .83-1.56L14.1 9.6V3.2" />
  </Stroke>
);

export const PlayIcon = (props: IconProps) => (
  <Solid {...props}>
    <path d="M7 4.8v14.4a1 1 0 0 0 1.52.85l11.5-7.2a1 1 0 0 0 0-1.7L8.52 3.95A1 1 0 0 0 7 4.8z" />
  </Solid>
);

export const PauseIcon = (props: IconProps) => (
  <Solid {...props}>
    <rect x="5.5" y="4" width="4.5" height="16" rx="1.2" />
    <rect x="14" y="4" width="4.5" height="16" rx="1.2" />
  </Solid>
);

export const NextIcon = (props: IconProps) => (
  <Solid {...props}>
    <path d="M3 5.7v12.6a1 1 0 0 0 1.55.83L13 13.5V18.3a1 1 0 0 0 1.55.83l8.44-5.6a1.8 1.8 0 0 0 0-3.06l-8.44-5.6A1 1 0 0 0 13 5.7v4.8L4.55 4.87A1 1 0 0 0 3 5.7z" />
  </Solid>
);

export const PreviousIcon = (props: IconProps) => (
  <NextIcon {...props} style={{ transform: "scaleX(-1)", ...props.style }} />
);

export const CloseIcon = (props: IconProps) => (
  <Stroke {...props}>
    <path d="M18 6 6 18M6 6l12 12" />
  </Stroke>
);

export const FolderIcon = (props: IconProps) => (
  <Stroke {...props}>
    <path d="M4 20h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.93a2 2 0 0 1-1.66-.9l-.82-1.2A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13c0 1.1.9 2 2 2z" />
  </Stroke>
);

// A shell prompt: the dev panel is a list of CLI sessions.
export const CodeIcon = (props: IconProps) => (
  <Stroke {...props}>
    <path d="m5 8.5 4 3.5-4 3.5" />
    <path d="M12.5 16H19" />
  </Stroke>
);

export const ClipboardIcon = (props: IconProps) => (
  <Stroke {...props}>
    <rect x="4.7" y="5" width="14.6" height="15.4" rx="3.2" />
    <rect x="9" y="2.6" width="6" height="4.2" rx="1.6" fill="currentColor" stroke="none" />
  </Stroke>
);

export const CheckIcon = (props: IconProps) => (
  <Stroke {...props}>
    <path d="M20 6 9 17l-5-5" />
  </Stroke>
);

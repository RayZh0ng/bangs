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

export const MusicIcon = (props: IconProps) => (
  <Stroke {...props}>
    <path d="M9 18V5l12-2v13" />
    <circle cx="6" cy="18" r="3" />
    <circle cx="18" cy="16" r="3" />
  </Stroke>
);

export const TimerIcon = (props: IconProps) => (
  <Stroke {...props}>
    <circle cx="12" cy="13" r="8" />
    <path d="M12 9v4l2.5 2.5M9 2h6" />
  </Stroke>
);

export const ShelfIcon = (props: IconProps) => (
  <Stroke {...props}>
    <path d="M22 12h-6l-2 3h-4l-2-3H2" />
    <path d="M5.45 5.11 2 12v6a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2v-6l-3.45-6.89A2 2 0 0 0 16.76 4H7.24a2 2 0 0 0-1.79 1.11z" />
  </Stroke>
);

export const PinIcon = ({ filled, ...props }: IconProps & { filled?: boolean }) => (
  <Stroke {...props} fill={filled ? "currentColor" : "none"}>
    <path d="M12 17v5M9 10.76a2 2 0 0 1-1.11 1.79l-1.78.9A2 2 0 0 0 5 15.24V17h14v-1.76a2 2 0 0 0-1.11-1.79l-1.78-.9A2 2 0 0 1 15 10.76V7a1 1 0 0 1 1-1 2 2 0 0 0 0-4H8a2 2 0 0 0 0 4 1 1 0 0 1 1 1z" />
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

export const CodeIcon = (props: IconProps) => (
  <Stroke {...props}>
    <path d="m8 8-4 4 4 4M16 8l4 4-4 4M13.5 5l-3 14" />
  </Stroke>
);

export const ClipboardIcon = (props: IconProps) => (
  <Stroke {...props}>
    <rect x="8" y="3" width="8" height="4" rx="1.4" />
    <path d="M16 5h2a2 2 0 0 1 2 2v12a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V7a2 2 0 0 1 2-2h2" />
  </Stroke>
);

export const CheckIcon = (props: IconProps) => (
  <Stroke {...props}>
    <path d="M20 6 9 17l-5-5" />
  </Stroke>
);

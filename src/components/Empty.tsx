import type { ReactNode } from "react";

export function Empty({ icon, title, hint }: { icon: ReactNode; title: string; hint: string }) {
  return (
    <div className="empty">
      {icon}
      <div className="empty__title">{title}</div>
      <div className="empty__hint">{hint}</div>
    </div>
  );
}

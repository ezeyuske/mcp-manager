import type { ReactNode } from "react";

interface ScreenShellProps {
  title: string;
  subtitle: string;
  actions?: ReactNode;
  children: ReactNode;
}

/** Marco de pantalla: título grande + subtítulo gris, y cuerpo scrollable. */
export function ScreenShell({
  title,
  subtitle,
  actions,
  children,
}: ScreenShellProps) {
  return (
    <div className="ds-rise flex h-full flex-col">
      <header className="flex items-start justify-between gap-4 px-8 pt-8 pb-5">
        <div>
          <h1 className="text-[26px] font-semibold leading-tight tracking-tight text-ink">
            {title}
          </h1>
          <p className="mt-1 text-[14px] text-muted">{subtitle}</p>
        </div>
        {actions}
      </header>
      <div className="flex-1 overflow-y-auto px-8 pb-10">{children}</div>
    </div>
  );
}

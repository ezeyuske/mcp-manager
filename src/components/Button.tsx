import type { ButtonHTMLAttributes, ReactNode } from "react";

type Variant = "primary" | "ghost" | "danger";

interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: Variant;
  children: ReactNode;
}

const BASE =
  "inline-flex items-center justify-center gap-2 rounded-[var(--radius-sm)] px-4 py-2.5 text-[13px] font-semibold transition-all duration-[var(--dur)] ease-[var(--ease-out)] disabled:cursor-not-allowed disabled:opacity-45 focus-visible:outline focus-visible:outline-2";

/** Botón del design system. primary = acento + glow; ghost = sutil; danger = estado. */
export function Button({
  variant = "primary",
  children,
  className = "",
  ...rest
}: ButtonProps) {
  const style =
    variant === "primary"
      ? {
          background: "var(--accent)",
          color: "var(--accent-contrast)",
          boxShadow: "0 0 18px var(--accent-glow)",
        }
      : variant === "danger"
        ? {
            background: "var(--state-danger)",
            color: "var(--state-danger-contrast)",
            boxShadow: "0 0 16px var(--state-danger-glow)",
          }
        : {
            background: "var(--color-surface-2)",
            color: "var(--color-ink-soft)",
            border: "1px solid var(--line)",
          };

  const hover =
    variant === "ghost"
      ? "hover:!border-[var(--line-strong)] hover:!text-ink"
      : "hover:scale-[1.03]";

  return (
    <button className={`${BASE} ${hover} ${className}`} style={style} {...rest}>
      {children}
    </button>
  );
}

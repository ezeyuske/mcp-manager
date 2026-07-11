import type { ReactNode } from "react";

interface CardProps {
  children: ReactNode;
  /** Título opcional del panel */
  title?: string;
  /** Subtítulo gris bajo el título */
  subtitle?: string;
  className?: string;
}

/** Tarjeta glass flotante del design system. */
export function Card({ children, title, subtitle, className = "" }: CardProps) {
  return (
    <section className={`ds-card p-5 ${className}`}>
      {(title || subtitle) && (
        <header className="mb-4">
          {title && (
            <h3 className="text-[15px] font-semibold tracking-tight text-ink">
              {title}
            </h3>
          )}
          {subtitle && (
            <p className="mt-0.5 text-[13px] leading-snug text-muted">
              {subtitle}
            </p>
          )}
        </header>
      )}
      {children}
    </section>
  );
}

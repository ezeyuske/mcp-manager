import type { LucideIcon } from "lucide-react";

interface EmptyStateProps {
  icon: LucideIcon;
  title: string;
  description: string;
  /** Fase en la que llega la funcionalidad real. */
  phase: string;
}

/** Placeholder consistente para pantallas aún no implementadas. */
export function EmptyState({
  icon: Icon,
  title,
  description,
  phase,
}: EmptyStateProps) {
  return (
    <div className="flex h-full min-h-[360px] flex-col items-center justify-center text-center">
      <div
        className="mb-5 flex h-16 w-16 items-center justify-center rounded-[20px] border border-[var(--line)]"
        style={{
          background: "var(--accent-soft)",
          boxShadow: "0 0 28px -6px var(--accent-glow)",
        }}
      >
        <Icon size={28} style={{ color: "var(--accent-strong)" }} />
      </div>
      <h2 className="text-[18px] font-semibold text-ink">{title}</h2>
      <p className="mt-1.5 max-w-sm text-[13.5px] leading-relaxed text-muted">
        {description}
      </p>
      <span
        className="mt-5 rounded-full border border-[var(--line)] px-3 py-1 text-[11.5px] font-medium text-ink-soft"
        style={{ background: "var(--color-surface-2)" }}
      >
        {phase}
      </span>
    </div>
  );
}

import { CheckCircle2, AlertTriangle, Info, X } from "lucide-react";
import { useToast, type ToastKind } from "../store/toast";

const ICON: Record<ToastKind, typeof Info> = {
  success: CheckCircle2,
  error: AlertTriangle,
  info: Info,
};

const ACCENT_VAR: Record<ToastKind, string> = {
  success: "var(--state-ok)",
  error: "var(--state-danger)",
  info: "var(--accent)",
};

/** Host de toasts (montar una vez en App). Consume el store useToast. */
export function ToastHost() {
  const toasts = useToast((s) => s.toasts);
  const dismiss = useToast((s) => s.dismiss);

  return (
    <div className="pointer-events-none fixed bottom-5 right-5 z-[60] flex w-80 flex-col gap-2.5">
      {toasts.map((t) => {
        const Icon = ICON[t.kind];
        const color = ACCENT_VAR[t.kind];
        return (
          <div
            key={t.id}
            className="ds-card ds-rise pointer-events-auto flex items-start gap-3 !rounded-[var(--radius-md)] px-3.5 py-3"
            style={{ borderLeft: `3px solid ${color}` }}
          >
            <Icon size={17} style={{ color }} className="mt-0.5 shrink-0" />
            <p className="flex-1 text-[13px] leading-snug text-ink-soft">
              {t.message}
            </p>
            <button
              onClick={() => dismiss(t.id)}
              aria-label="Cerrar"
              className="shrink-0 text-faint transition-colors hover:text-ink"
            >
              <X size={14} />
            </button>
          </div>
        );
      })}
    </div>
  );
}

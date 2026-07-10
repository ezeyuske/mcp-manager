import { useEffect, type ReactNode } from "react";
import { X } from "lucide-react";

interface ModalProps {
  open: boolean;
  onClose: () => void;
  title: string;
  subtitle?: string;
  children: ReactNode;
  /** Contenido del footer (botones). */
  footer?: ReactNode;
  /** ancho máximo del panel (px). */
  width?: number;
}

/** Modal glass centrado. Cierra por overlay, botón X o Esc. */
export function Modal({
  open,
  onClose,
  title,
  subtitle,
  children,
  footer,
  width = 520,
}: ModalProps) {
  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    document.addEventListener("keydown", onKey);
    return () => document.removeEventListener("keydown", onKey);
  }, [open, onClose]);

  if (!open) return null;

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center p-6"
      style={{ background: "var(--overlay)", backdropFilter: "blur(3px)" }}
      onMouseDown={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div
        role="dialog"
        aria-modal="true"
        aria-label={title}
        className="ds-card ds-rise flex max-h-[85vh] w-full flex-col overflow-hidden"
        style={{ maxWidth: width }}
      >
        <header className="flex items-start justify-between gap-4 border-b border-[var(--line)] px-5 py-4">
          <div>
            <h2 className="text-[16px] font-semibold tracking-tight text-ink">
              {title}
            </h2>
            {subtitle && (
              <p className="mt-0.5 text-[12.5px] text-muted">{subtitle}</p>
            )}
          </div>
          <button
            onClick={onClose}
            aria-label="Cerrar"
            className="flex h-8 w-8 shrink-0 items-center justify-center rounded-[10px] text-muted transition-colors duration-[var(--dur)] hover:bg-[rgba(255,255,255,0.06)] hover:text-ink"
          >
            <X size={17} />
          </button>
        </header>

        <div className="flex-1 overflow-y-auto px-5 py-4">{children}</div>

        {footer && (
          <footer className="flex items-center justify-end gap-2.5 border-t border-[var(--line)] px-5 py-3.5">
            {footer}
          </footer>
        )}
      </div>
    </div>
  );
}

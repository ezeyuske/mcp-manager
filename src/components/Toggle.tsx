interface ToggleProps {
  checked: boolean;
  onChange: (checked: boolean) => void;
  disabled?: boolean;
  /** Etiqueta accesible cuando no hay texto visible asociado */
  label?: string;
}

/** Toggle redondeado: on = acento + glow, off = gris oscuro (--color-track). */
export function Toggle({ checked, onChange, disabled, label }: ToggleProps) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      aria-label={label}
      disabled={disabled}
      onClick={() => onChange(!checked)}
      className="relative inline-flex h-6 w-11 shrink-0 items-center rounded-full transition-colors duration-[var(--dur)] ease-[var(--ease-out)] disabled:opacity-40"
      style={{
        background: checked ? "var(--accent)" : "var(--color-track)",
        boxShadow: checked ? "0 0 14px var(--accent-glow)" : "none",
      }}
    >
      <span
        className="inline-block h-[18px] w-[18px] transform rounded-full bg-white shadow-sm transition-transform duration-[var(--dur)] ease-[var(--ease-out)]"
        style={{ transform: checked ? "translateX(22px)" : "translateX(3px)" }}
      />
    </button>
  );
}

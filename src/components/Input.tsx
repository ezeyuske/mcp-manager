import type { InputHTMLAttributes } from "react";

interface InputProps extends InputHTMLAttributes<HTMLInputElement> {
  label?: string;
  help?: string;
  /** Usar fuente monoespaciada (comandos, urls). */
  mono?: boolean;
}

/** Campo de texto oscuro elevado, borde sutil, focus con acento. */
export function Input({
  label,
  help,
  mono = false,
  className = "",
  id,
  ...rest
}: InputProps) {
  const field = (
    <input
      id={id}
      className={`w-full rounded-[var(--radius-sm)] border border-[var(--line)] bg-surface-2 px-3 py-2 text-[13px] text-ink placeholder:text-faint transition-colors duration-[var(--dur)] hover:border-[var(--line-strong)] focus:border-[var(--accent)] focus:outline-none ${
        mono ? "font-mono" : ""
      } ${className}`}
      {...rest}
    />
  );

  if (!label && !help) return field;

  return (
    <label className="flex flex-col gap-1.5" htmlFor={id}>
      {label && (
        <span className="text-[12.5px] font-medium text-ink-soft">{label}</span>
      )}
      {field}
      {help && <span className="text-[11.5px] text-faint">{help}</span>}
    </label>
  );
}

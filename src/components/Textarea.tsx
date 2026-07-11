import type { TextareaHTMLAttributes } from "react";

interface TextareaProps extends TextareaHTMLAttributes<HTMLTextAreaElement> {
  label?: string;
  help?: string;
}

/** Textarea oscuro elevado (ej. args uno por línea). */
export function Textarea({
  label,
  help,
  className = "",
  id,
  ...rest
}: TextareaProps) {
  const field = (
    <textarea
      id={id}
      className={`w-full resize-y rounded-[var(--radius-sm)] border border-[var(--line)] bg-surface-2 px-3 py-2 font-mono text-[13px] text-ink placeholder:text-faint transition-colors duration-[var(--dur)] hover:border-[var(--line-strong)] focus:border-[var(--accent)] focus:outline-none ${className}`}
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

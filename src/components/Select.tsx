import { ChevronDown } from "lucide-react";

export interface SelectOption<T extends string> {
  value: T;
  label: string;
}

interface SelectProps<T extends string> {
  options: SelectOption<T>[];
  value: T;
  onChange: (value: T) => void;
  label?: string;
}

/** Select oscuro con borde sutil y chevron. */
export function Select<T extends string>({
  options,
  value,
  onChange,
  label,
}: SelectProps<T>) {
  return (
    <div className="relative inline-flex">
      <select
        aria-label={label}
        value={value}
        onChange={(e) => onChange(e.currentTarget.value as T)}
        className="appearance-none rounded-[var(--radius-sm)] border border-[var(--line)] bg-surface-2 py-2 pl-3 pr-9 text-[13px] font-medium text-ink transition-colors duration-[var(--dur)] hover:border-[var(--line-strong)] focus:border-[var(--accent)] focus:outline-none"
      >
        {options.map((opt) => (
          <option key={opt.value} value={opt.value} className="bg-surface-2">
            {opt.label}
          </option>
        ))}
      </select>
      <ChevronDown
        size={15}
        className="pointer-events-none absolute right-3 top-1/2 -translate-y-1/2 text-muted"
      />
    </div>
  );
}

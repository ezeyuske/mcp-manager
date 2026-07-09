export interface SegmentOption<T extends string> {
  value: T;
  label: string;
}

interface SegmentedControlProps<T extends string> {
  options: SegmentOption<T>[];
  value: T;
  onChange: (value: T) => void;
  label?: string;
}

/** Segmented control tipo pill; opción activa en acento con glow. */
export function SegmentedControl<T extends string>({
  options,
  value,
  onChange,
  label,
}: SegmentedControlProps<T>) {
  return (
    <div
      role="tablist"
      aria-label={label}
      className="inline-flex items-center gap-1 rounded-full border border-[var(--line)] bg-surface-2 p-1"
    >
      {options.map((opt) => {
        const active = opt.value === value;
        return (
          <button
            key={opt.value}
            role="tab"
            aria-selected={active}
            onClick={() => onChange(opt.value)}
            className="rounded-full px-3.5 py-1.5 text-[13px] font-medium transition-all duration-[var(--dur)] ease-[var(--ease-out)]"
            style={
              active
                ? {
                    background: "var(--accent)",
                    color: "var(--accent-contrast)",
                    boxShadow: "0 0 16px var(--accent-glow)",
                  }
                : { color: "var(--color-muted)" }
            }
          >
            {opt.label}
          </button>
        );
      })}
    </div>
  );
}

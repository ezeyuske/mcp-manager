import { Check } from "lucide-react";

interface ColorSwatchProps {
  color: string;
  /** rgba del glow propio del color */
  glow: string;
  selected: boolean;
  onClick: () => void;
  label: string;
}

/** Swatch circular con anillo y glow del propio color (pantalla Themes). */
export function ColorSwatch({
  color,
  glow,
  selected,
  onClick,
  label,
}: ColorSwatchProps) {
  return (
    <button
      type="button"
      onClick={onClick}
      title={label}
      aria-label={label}
      aria-pressed={selected}
      className="group relative flex h-11 w-11 items-center justify-center rounded-full transition-transform duration-[var(--dur)] ease-[var(--ease-out)] hover:scale-110"
      style={{
        background: color,
        boxShadow: selected
          ? `0 0 0 2px var(--color-surface), 0 0 0 4px ${color}, 0 0 20px ${glow}`
          : `0 0 12px ${glow}`,
      }}
    >
      {selected && (
        <Check size={18} strokeWidth={3} className="text-white drop-shadow" />
      )}
    </button>
  );
}

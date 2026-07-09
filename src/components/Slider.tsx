interface SliderProps {
  value: number;
  onChange: (value: number) => void;
  min?: number;
  max?: number;
  step?: number;
  label?: string;
}

/** Slider: track fino 4px, relleno acento, thumb circular con glow. */
export function Slider({
  value,
  onChange,
  min = 0,
  max = 100,
  step = 1,
  label,
}: SliderProps) {
  const pct = ((value - min) / (max - min)) * 100;

  return (
    <input
      type="range"
      aria-label={label}
      min={min}
      max={max}
      step={step}
      value={value}
      onChange={(e) => onChange(Number(e.currentTarget.value))}
      className="ds-slider h-1.5 w-full cursor-pointer appearance-none rounded-full bg-track"
      style={{
        background: `linear-gradient(to right, var(--accent) ${pct}%, var(--color-track) ${pct}%)`,
      }}
    />
  );
}

import { useState, type ReactNode } from "react";
import { ScreenShell } from "./ScreenShell";
import { Card, ColorSwatch, Toggle, Slider, SegmentedControl } from "../components";
import { useTheme } from "../store/theme";
import { ACCENTS, ACCENT_ORDER } from "../types/theme";

/**
 * Pantalla de Themes (dogfooding): elegir un acento reescribe las CSS vars
 * en :root en caliente y recolorea TODA la app. Fuente del requisito de
 * "acento configurable en runtime".
 */
export function ThemesScreen() {
  const accent = useTheme((s) => s.accent);
  const setAccent = useTheme((s) => s.setAccent);

  // Estado local solo para la demo del preview.
  const [on, setOn] = useState(true);
  const [level, setLevel] = useState(65);
  const [seg, setSeg] = useState<"a" | "b" | "c">("b");

  return (
    <ScreenShell
      title="Themes"
      subtitle="Elegí el color de acento. El cambio se aplica en vivo a toda la app."
    >
      <div className="grid max-w-3xl gap-4">
        <Card
          title="Color de acento"
          subtitle={`Actual: ${ACCENTS[accent].label}. Los elementos activos usan este color con glow.`}
        >
          <div className="flex flex-wrap gap-3.5">
            {ACCENT_ORDER.map((key) => {
              const a = ACCENTS[key];
              return (
                <ColorSwatch
                  key={key}
                  color={a.accent}
                  glow={a.glow}
                  selected={key === accent}
                  onClick={() => setAccent(key)}
                  label={a.label}
                />
              );
            })}
          </div>
        </Card>

        <Card
          title="Vista previa"
          subtitle="Estos controles reflejan el acento activo en tiempo real."
        >
          <div className="flex flex-col gap-6">
            <PreviewRow label="Toggle">
              <Toggle checked={on} onChange={setOn} label="Preview toggle" />
            </PreviewRow>

            <PreviewRow label="Slider">
              <div className="w-56">
                <Slider value={level} onChange={setLevel} label="Preview slider" />
              </div>
            </PreviewRow>

            <PreviewRow label="Segmented">
              <SegmentedControl
                label="Preview segmented"
                value={seg}
                onChange={setSeg}
                options={[
                  { value: "a", label: "stdio" },
                  { value: "b", label: "HTTP" },
                  { value: "c", label: "SSE" },
                ]}
              />
            </PreviewRow>

            <PreviewRow label="Botón primario">
              <button
                className="rounded-[var(--radius-sm)] px-4 py-2 text-[13px] font-semibold transition-transform duration-[var(--dur)] hover:scale-[1.03]"
                style={{
                  background: "var(--accent)",
                  color: "var(--accent-contrast)",
                  boxShadow: "0 0 18px var(--accent-glow)",
                }}
              >
                Instalar MCP
              </button>
            </PreviewRow>
          </div>
        </Card>
      </div>
    </ScreenShell>
  );
}

function PreviewRow({
  label,
  children,
}: {
  label: string;
  children: ReactNode;
}) {
  return (
    <div className="flex items-center justify-between gap-6">
      <span className="text-[13px] font-medium text-muted">{label}</span>
      {children}
    </div>
  );
}

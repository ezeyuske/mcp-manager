import { useState, type ReactNode } from "react";
import { ScreenShell } from "./ScreenShell";
import { Card, Toggle, Select } from "../components";

/** Ajustes generales. En Fase 1 los controles son demostrativos (no persisten). */
export function SettingsScreen() {
  const [autoBackup, setAutoBackup] = useState(true);
  const [validate, setValidate] = useState(true);
  const [density, setDensity] = useState<"comfortable" | "compact">(
    "comfortable",
  );

  return (
    <ScreenShell
      title="Ajustes"
      subtitle="Comportamiento general de la app y seguridad de escritura de configs."
    >
      <div className="grid max-w-2xl gap-4">
        <Card
          title="Seguridad de escritura"
          subtitle="Aplica a todo config externo que la app modifique."
        >
          <div className="flex flex-col divide-y divide-[var(--line)]">
            <SettingRow
              label="Backup automático antes de escribir"
              help="Copia timestampeada en ~/.mcp-manager/backups/"
            >
              <Toggle
                checked={autoBackup}
                onChange={setAutoBackup}
                label="Backup automático"
              />
            </SettingRow>
            <SettingRow
              label="Validar JSON resultante"
              help="Reparsea el archivo antes del rename atómico"
            >
              <Toggle
                checked={validate}
                onChange={setValidate}
                label="Validar JSON"
              />
            </SettingRow>
          </div>
        </Card>

        <Card title="Apariencia">
          <SettingRow label="Densidad de la lista" help="Espaciado de las filas">
            <Select
              label="Densidad"
              value={density}
              onChange={setDensity}
              options={[
                { value: "comfortable", label: "Cómoda" },
                { value: "compact", label: "Compacta" },
              ]}
            />
          </SettingRow>
        </Card>
      </div>
    </ScreenShell>
  );
}

function SettingRow({
  label,
  help,
  children,
}: {
  label: string;
  help: string;
  children: ReactNode;
}) {
  return (
    <div className="flex items-center justify-between gap-6 py-3.5 first:pt-0 last:pb-0">
      <div>
        <div className="text-[14px] font-medium text-ink">{label}</div>
        <div className="mt-0.5 text-[12.5px] text-faint">{help}</div>
      </div>
      {children}
    </div>
  );
}

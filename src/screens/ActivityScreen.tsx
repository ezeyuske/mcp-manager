import { useEffect } from "react";
import {
  Plus,
  Pencil,
  Trash2,
  CopyPlus,
  TextCursorInput,
  Power,
  PowerOff,
  ArrowLeftRight,
  RotateCcw,
  AlertTriangle,
  History,
} from "lucide-react";
import { ScreenShell } from "./ScreenShell";
import { Button } from "../components";
import { useChangelog } from "../store/changelog";
import { APP_LABEL, type MutationAction, type MutationLog } from "../types/inventory";

const ACTION_META: Record<
  MutationAction,
  { label: string; icon: typeof Plus; color: string }
> = {
  add: { label: "Agregado", icon: Plus, color: "var(--state-ok)" },
  edit: { label: "Editado", icon: Pencil, color: "var(--accent)" },
  delete: { label: "Eliminado", icon: Trash2, color: "var(--state-danger)" },
  duplicate: { label: "Duplicado", icon: CopyPlus, color: "var(--accent)" },
  rename: { label: "Renombrado", icon: TextCursorInput, color: "var(--accent)" },
  enable: { label: "Habilitado", icon: Power, color: "var(--state-ok)" },
  disable: { label: "Deshabilitado", icon: PowerOff, color: "var(--color-faint)" },
  copy: { label: "Copiado", icon: ArrowLeftRight, color: "var(--accent)" },
  restore: { label: "Restaurado", icon: RotateCcw, color: "var(--state-warn)" },
};

function formatTs(iso: string): string {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return iso;
  return d.toLocaleString(undefined, {
    day: "2-digit",
    month: "short",
    hour: "2-digit",
    minute: "2-digit",
  });
}

export function ActivityScreen() {
  const { status, entries, error, mocked, load, restore } = useChangelog();

  useEffect(() => {
    if (status === "idle") load();
  }, [status, load]);

  return (
    <ScreenShell
      title="Actividad"
      subtitle="Historial de cambios en los configs, con backup y restore por punto."
    >
      {status === "loading" && (
        <div className="flex flex-col gap-2.5">
          {[0, 1, 2].map((i) => (
            <div
              key={i}
              className="ds-card h-16 animate-pulse !rounded-[var(--radius-md)]"
              style={{ opacity: 1 - i * 0.2 }}
            />
          ))}
        </div>
      )}

      {status === "error" && (
        <div className="ds-card flex flex-col items-center gap-3 px-6 py-12 text-center">
          <AlertTriangle size={26} style={{ color: "var(--state-danger-text)" }} />
          <h2 className="text-[16px] font-semibold text-ink">
            No se pudo leer el log
          </h2>
          <p className="max-w-md font-mono text-[12px] text-muted">
            {error ?? "Error desconocido"}
          </p>
        </div>
      )}

      {status === "ready" && entries.length === 0 && (
        <div className="ds-card flex flex-col items-center gap-3 px-6 py-14 text-center">
          <div
            className="flex h-14 w-14 items-center justify-center rounded-[18px] border border-[var(--line)]"
            style={{ background: "var(--accent-soft)" }}
          >
            <History size={26} style={{ color: "var(--accent-strong)" }} />
          </div>
          <h2 className="text-[16px] font-semibold text-ink">Sin cambios aún</h2>
          <p className="max-w-sm text-[13px] leading-relaxed text-muted">
            Cada vez que agregues, edites o elimines un MCP, va a quedar
            registrado acá con su backup.
          </p>
        </div>
      )}

      {status === "ready" && entries.length > 0 && (
        <div className="flex flex-col gap-2.5">
          {entries.map((e) => (
            <ActivityRow key={e.id} entry={e} onRestore={restore} />
          ))}
        </div>
      )}

      {mocked && (
        <p className="mt-6 text-center text-[12px] text-faint">
          Datos de muestra (modo browser).
        </p>
      )}
    </ScreenShell>
  );
}

function ActivityRow({
  entry,
  onRestore,
}: {
  entry: MutationLog;
  onRestore: (backupPath: string, targetPath: string) => void;
}) {
  const meta = ACTION_META[entry.action] ?? ACTION_META.edit;
  const Icon = meta.icon;

  return (
    <div className="ds-card flex items-center gap-4 !rounded-[var(--radius-md)] px-4 py-3.5">
      <div
        className="flex h-10 w-10 shrink-0 items-center justify-center rounded-[12px] border border-[var(--line)]"
        style={{ background: "var(--color-surface-2)" }}
      >
        <Icon size={17} style={{ color: meta.color }} />
      </div>

      <div className="min-w-0 flex-1">
        <div className="flex flex-wrap items-center gap-2">
          <span className="text-[13.5px] font-semibold text-ink">
            {meta.label}
            {entry.mcpName ? (
              <span className="font-normal text-ink-soft"> · {entry.mcpName}</span>
            ) : null}
          </span>
          <span
            className="rounded-full px-2 py-0.5 text-[10.5px] font-medium text-ink-soft"
            style={{ background: "var(--color-surface-3)" }}
          >
            {APP_LABEL[entry.app]}
          </span>
        </div>
        <div className="mt-0.5 truncate font-mono text-[11.5px] text-faint">
          {entry.filePath}
        </div>
      </div>

      <span className="shrink-0 text-[11.5px] tabular-nums text-faint">
        {formatTs(entry.timestamp)}
      </span>

      {entry.backupPath && (
        <Button
          variant="ghost"
          onClick={() => onRestore(entry.backupPath!, entry.filePath)}
        >
          <RotateCcw size={14} /> Restaurar
        </Button>
      )}
    </div>
  );
}

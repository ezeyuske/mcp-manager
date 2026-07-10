import { useEffect, useMemo, useState, type ReactNode } from "react";
import {
  Plus,
  Terminal,
  Globe,
  Radio,
  RefreshCw,
  AlertTriangle,
  PlugZap,
  Pencil,
  CopyPlus,
  ArrowLeftRight,
  Trash2,
} from "lucide-react";
import { ScreenShell } from "./ScreenShell";
import { McpFormModal } from "./mcp/McpFormModal";
import { SegmentedControl, Toggle, Modal, Button, Input } from "../components";
import { useInventory } from "../store/inventory";
import { useMutations } from "../store/mutations";
import {
  APP_LABEL,
  targetOf,
  unify,
  type AppId,
  type McpInstallation,
  type TransportKind,
  type UnifiedMcp,
} from "../types/inventory";

const TRANSPORT_ICON: Record<TransportKind, typeof Terminal> = {
  stdio: Terminal,
  http: Globe,
  sse: Radio,
  unknown: PlugZap,
};

type Dialog =
  | { kind: "add" }
  | { kind: "edit"; inst: McpInstallation }
  | { kind: "delete"; inst: McpInstallation }
  | { kind: "duplicate"; inst: McpInstallation }
  | null;

export function McpsScreen() {
  const { status, inventory, error, mocked, load } = useInventory();
  const [filter, setFilter] = useState<"all" | AppId>("all");
  const [dialog, setDialog] = useState<Dialog>(null);

  useEffect(() => {
    if (status === "idle") load();
  }, [status, load]);

  const unified = useMemo(
    () => unify(inventory?.installations ?? []),
    [inventory],
  );
  const visible = useMemo(
    () =>
      filter === "all"
        ? unified
        : unified.filter((m) => m.apps.includes(filter)),
    [unified, filter],
  );

  const installedApps = (inventory?.apps ?? []).filter((a) => a.installed);
  const brokenCount = unified.filter((m) => m.broken).length;

  return (
    <ScreenShell
      title="MCP Servers"
      subtitle="Inventario unificado de todos los MCP de tus apps de Claude."
      actions={
        <div className="flex items-center gap-2">
          <IconButton onClick={load} label="Refrescar">
            <RefreshCw
              size={16}
              className={status === "loading" ? "animate-spin" : ""}
            />
          </IconButton>
          <Button onClick={() => setDialog({ kind: "add" })}>
            <Plus size={16} strokeWidth={2.5} />
            Agregar MCP
          </Button>
        </div>
      }
    >
      <div className="mb-5 flex flex-wrap gap-2.5">
        {(inventory?.apps ?? []).map((app) => (
          <div
            key={app.id}
            className="flex items-center gap-2 rounded-full border border-[var(--line)] px-3 py-1.5 text-[12px]"
            style={{ background: "var(--color-surface-2)" }}
            title={
              app.error
                ? `Error: ${app.error}`
                : (app.configPath ?? app.notInstalledReason ?? "")
            }
          >
            <span
              className="h-2 w-2 rounded-full"
              style={{
                background: app.error
                  ? "var(--state-warn)"
                  : app.installed
                    ? "var(--state-ok)"
                    : "var(--color-faint)",
                boxShadow:
                  app.installed && !app.error
                    ? "0 0 8px var(--state-ok-glow)"
                    : "none",
              }}
            />
            <span className="font-medium text-ink-soft">{app.label}</span>
            <span className="text-faint">
              {app.error
                ? "error"
                : app.installed
                  ? "detectada"
                  : "no instalada"}
            </span>
          </div>
        ))}
      </div>

      {status === "loading" && <SkeletonList />}

      {status === "error" && (
        <StateCard
          icon={AlertTriangle}
          tone="danger"
          title="No se pudo leer el inventario"
          detail={error ?? "Error desconocido"}
        />
      )}

      {status === "ready" && unified.length === 0 && (
        <StateCard
          icon={PlugZap}
          title="Sin MCP servers"
          detail="Agregá tu primer MCP con el botón de arriba."
        />
      )}

      {status === "ready" && unified.length > 0 && (
        <>
          <div className="mb-5 flex items-center justify-between gap-4">
            <SegmentedControl
              label="Filtrar por app"
              value={filter}
              onChange={setFilter}
              options={[
                { value: "all" as const, label: "Todas" },
                ...installedApps.map((a) => ({ value: a.id, label: a.label })),
              ]}
            />
            <span className="text-[12px] text-faint">
              {unified.length} MCP{unified.length === 1 ? "" : "s"}
              {brokenCount > 0 && (
                <span style={{ color: "var(--state-danger-text)" }}>
                  {" "}
                  · {brokenCount} roto
                </span>
              )}
            </span>
          </div>

          <div className="flex flex-col gap-2.5">
            {visible.map((mcp) => (
              <McpCard
                key={mcp.name}
                mcp={mcp}
                onEdit={(inst) => setDialog({ kind: "edit", inst })}
                onDelete={(inst) => setDialog({ kind: "delete", inst })}
                onDuplicate={(inst) => setDialog({ kind: "duplicate", inst })}
              />
            ))}
          </div>
        </>
      )}

      {mocked && (
        <p className="mt-6 text-center text-[12px] text-faint">
          Datos de muestra (modo browser). En la app real se leen y escriben los
          configs vía el backend.
        </p>
      )}

      <McpFormModal
        open={dialog?.kind === "add" || dialog?.kind === "edit"}
        onClose={() => setDialog(null)}
        initial={dialog?.kind === "edit" ? dialog.inst : null}
      />
      {dialog?.kind === "delete" && (
        <DeleteDialog inst={dialog.inst} onClose={() => setDialog(null)} />
      )}
      {dialog?.kind === "duplicate" && (
        <DuplicateDialog inst={dialog.inst} onClose={() => setDialog(null)} />
      )}
    </ScreenShell>
  );
}

function McpCard({
  mcp,
  onEdit,
  onDelete,
  onDuplicate,
}: {
  mcp: UnifiedMcp;
  onEdit: (i: McpInstallation) => void;
  onDelete: (i: McpInstallation) => void;
  onDuplicate: (i: McpInstallation) => void;
}) {
  const Icon = TRANSPORT_ICON[mcp.transport];
  return (
    <div className="ds-card !rounded-[var(--radius-md)] px-4 py-3.5">
      <div className="flex items-center gap-4">
        <div
          className="flex h-10 w-10 shrink-0 items-center justify-center rounded-[12px] border border-[var(--line)]"
          style={{ background: "var(--color-surface-2)" }}
        >
          <Icon size={18} className="text-ink-soft" />
        </div>
        <div className="min-w-0 flex-1">
          <div className="flex flex-wrap items-center gap-2">
            <span className="text-[14px] font-semibold text-ink">
              {mcp.name}
            </span>
            <Tag>{mcp.transport}</Tag>
            {mcp.broken && <Tag tone="danger">comando no encontrado</Tag>}
          </div>
          <div className="mt-0.5 truncate font-mono text-[12px] text-faint">
            {mcp.target}
          </div>
        </div>
      </div>

      <div className="mt-3 flex flex-col divide-y divide-[var(--line)] border-t border-[var(--line)]">
        {mcp.installations.map((inst) => (
          <InstallationRow
            key={`${inst.app}:${inst.scope}:${inst.projectPath ?? ""}`}
            inst={inst}
            onEdit={onEdit}
            onDelete={onDelete}
            onDuplicate={onDuplicate}
          />
        ))}
      </div>
    </div>
  );
}

function InstallationRow({
  inst,
  onEdit,
  onDelete,
  onDuplicate,
}: {
  inst: McpInstallation;
  onEdit: (i: McpInstallation) => void;
  onDelete: (i: McpInstallation) => void;
  onDuplicate: (i: McpInstallation) => void;
}) {
  const setEnabled = useMutations((s) => s.setEnabled);
  const copy = useMutations((s) => s.copy);

  const otherApp: AppId =
    inst.app === "claude-desktop" ? "claude-code" : "claude-desktop";

  return (
    <div className="flex items-center gap-3 py-2.5 first:pt-3">
      <StatusDot status={inst.status} />
      <div className="min-w-0 flex-1">
        <div className="flex items-center gap-2">
          <span className="text-[12.5px] font-medium text-ink-soft">
            {APP_LABEL[inst.app]}
          </span>
          <Tag>{inst.scope}</Tag>
          {inst.envKeys.length > 0 && (
            <Tag>
              {inst.envKeys.length} env{inst.envKeys.length === 1 ? "" : "s"}
            </Tag>
          )}
          {!inst.enabled && <Tag>deshabilitado</Tag>}
        </div>
        {inst.projectPath && (
          <div className="mt-0.5 truncate font-mono text-[11px] text-faint">
            {inst.projectPath}
          </div>
        )}
      </div>

      <Toggle
        checked={inst.enabled}
        onChange={(v) => setEnabled(targetOf(inst), v)}
        label={`${inst.enabled ? "Deshabilitar" : "Habilitar"} ${inst.name}`}
      />
      <RowAction label="Editar" onClick={() => onEdit(inst)}>
        <Pencil size={15} />
      </RowAction>
      <RowAction label="Duplicar" onClick={() => onDuplicate(inst)}>
        <CopyPlus size={15} />
      </RowAction>
      <RowAction
        label={`Copiar a ${APP_LABEL[otherApp]}`}
        onClick={() => copy(targetOf(inst), otherApp, "user", null)}
      >
        <ArrowLeftRight size={15} />
      </RowAction>
      <RowAction label="Eliminar" danger onClick={() => onDelete(inst)}>
        <Trash2 size={15} />
      </RowAction>
    </div>
  );
}

function DeleteDialog({
  inst,
  onClose,
}: {
  inst: McpInstallation;
  onClose: () => void;
}) {
  const remove = useMutations((s) => s.remove);
  const busy = useMutations((s) => s.busy);
  return (
    <Modal
      open
      onClose={onClose}
      title={`Eliminar ${inst.name}`}
      subtitle={`De ${APP_LABEL[inst.app]} · ${inst.scope}`}
      width={440}
      footer={
        <>
          <Button variant="ghost" onClick={onClose} disabled={busy}>
            Cancelar
          </Button>
          <Button
            variant="danger"
            disabled={busy}
            onClick={async () => {
              const ok = await remove(targetOf(inst));
              if (ok) onClose();
            }}
          >
            Eliminar
          </Button>
        </>
      }
    >
      <p className="text-[13px] leading-relaxed text-muted">
        Se quitará la entrada de{" "}
        <span className="font-mono text-ink-soft">{inst.configPath}</span>. Se
        crea un backup antes de escribir; podés restaurarlo desde Actividad.
      </p>
    </Modal>
  );
}

function DuplicateDialog({
  inst,
  onClose,
}: {
  inst: McpInstallation;
  onClose: () => void;
}) {
  const duplicate = useMutations((s) => s.duplicate);
  const busy = useMutations((s) => s.busy);
  const [newName, setNewName] = useState(`${inst.name}-copy`);
  return (
    <Modal
      open
      onClose={onClose}
      title={`Duplicar ${inst.name}`}
      width={440}
      footer={
        <>
          <Button variant="ghost" onClick={onClose} disabled={busy}>
            Cancelar
          </Button>
          <Button
            disabled={busy || newName.trim() === "" || newName === inst.name}
            onClick={async () => {
              const ok = await duplicate(targetOf(inst), newName.trim());
              if (ok) onClose();
            }}
          >
            Duplicar
          </Button>
        </>
      }
    >
      <Input
        label="Nuevo nombre"
        value={newName}
        mono
        onChange={(e) => setNewName(e.currentTarget.value)}
      />
    </Modal>
  );
}

function RowAction({
  children,
  onClick,
  label,
  danger = false,
}: {
  children: ReactNode;
  onClick: () => void;
  label: string;
  danger?: boolean;
}) {
  return (
    <button
      onClick={onClick}
      aria-label={label}
      title={label}
      className="flex h-8 w-8 items-center justify-center rounded-[10px] text-muted transition-colors duration-[var(--dur)] hover:text-ink"
      onMouseEnter={(e) =>
        (e.currentTarget.style.background = danger
          ? "var(--state-danger-soft)"
          : "rgba(255,255,255,0.06)")
      }
      onMouseLeave={(e) => (e.currentTarget.style.background = "transparent")}
      style={danger ? { color: "var(--state-danger-text)" } : undefined}
    >
      {children}
    </button>
  );
}

function StatusDot({ status }: { status: McpInstallation["status"] }) {
  const map = {
    ok: { bg: "var(--state-ok)", glow: "var(--state-ok-glow)", t: "OK" },
    command_not_found: {
      bg: "var(--state-danger)",
      glow: "var(--state-danger-glow)",
      t: "Comando no encontrado",
    },
    disabled: { bg: "var(--color-faint)", glow: "transparent", t: "Deshabilitado" },
    unknown: { bg: "var(--state-warn)", glow: "transparent", t: "Desconocido" },
  } as const;
  const s = map[status];
  return (
    <span
      title={s.t}
      className="h-2.5 w-2.5 shrink-0 rounded-full"
      style={{ background: s.bg, boxShadow: `0 0 8px ${s.glow}` }}
    />
  );
}

function Tag({
  children,
  tone = "neutral",
}: {
  children: ReactNode;
  tone?: "neutral" | "danger";
}) {
  const danger = tone === "danger";
  return (
    <span
      className="rounded-full px-2 py-0.5 text-[10.5px] font-medium"
      style={{
        background: danger
          ? "var(--state-danger-soft)"
          : "var(--color-surface-3)",
        color: danger ? "var(--state-danger-text)" : "var(--color-ink-soft)",
      }}
    >
      {children}
    </span>
  );
}

function IconButton({
  children,
  onClick,
  label,
}: {
  children: ReactNode;
  onClick: () => void;
  label: string;
}) {
  return (
    <button
      onClick={onClick}
      aria-label={label}
      title={label}
      className="flex h-10 w-10 items-center justify-center rounded-[var(--radius-sm)] border border-[var(--line)] text-muted transition-colors duration-[var(--dur)] hover:border-[var(--line-strong)] hover:text-ink"
      style={{ background: "var(--color-surface-2)" }}
    >
      {children}
    </button>
  );
}

function SkeletonList() {
  return (
    <div className="flex flex-col gap-2.5">
      {[0, 1, 2, 3].map((i) => (
        <div
          key={i}
          className="ds-card flex items-center gap-4 !rounded-[var(--radius-md)] px-4 py-3.5"
          style={{ opacity: 1 - i * 0.18 }}
        >
          <div className="h-10 w-10 shrink-0 animate-pulse rounded-[12px] bg-[var(--color-surface-3)]" />
          <div className="flex-1 space-y-2">
            <div className="h-3.5 w-40 animate-pulse rounded bg-[var(--color-surface-3)]" />
            <div className="h-3 w-64 animate-pulse rounded bg-[var(--color-surface-2)]" />
          </div>
        </div>
      ))}
    </div>
  );
}

function StateCard({
  icon: Icon,
  title,
  detail,
  tone = "neutral",
}: {
  icon: typeof AlertTriangle;
  title: string;
  detail: string;
  tone?: "neutral" | "danger";
}) {
  const danger = tone === "danger";
  return (
    <div className="ds-card flex flex-col items-center gap-3 px-6 py-12 text-center">
      <div
        className="flex h-14 w-14 items-center justify-center rounded-[18px] border border-[var(--line)]"
        style={{
          background: danger ? "var(--state-danger-soft)" : "var(--accent-soft)",
        }}
      >
        <Icon
          size={26}
          style={{ color: danger ? "var(--state-danger-text)" : "var(--accent-strong)" }}
        />
      </div>
      <h2 className="text-[16px] font-semibold text-ink">{title}</h2>
      <p className="max-w-md break-words font-mono text-[12px] leading-relaxed text-muted">
        {detail}
      </p>
    </div>
  );
}

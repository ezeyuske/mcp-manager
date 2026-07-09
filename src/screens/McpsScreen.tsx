import { useEffect, useMemo, useState, type ReactNode } from "react";
import {
  Plus,
  Terminal,
  Globe,
  Radio,
  RefreshCw,
  AlertTriangle,
  PlugZap,
} from "lucide-react";
import { ScreenShell } from "./ScreenShell";
import { SegmentedControl } from "../components";
import { useInventory } from "../store/inventory";
import {
  APP_LABEL,
  unify,
  type AppId,
  type TransportKind,
  type UnifiedMcp,
} from "../types/inventory";

const TRANSPORT_ICON: Record<TransportKind, typeof Terminal> = {
  stdio: Terminal,
  http: Globe,
  sse: Radio,
  unknown: PlugZap,
};

export function McpsScreen() {
  const { status, inventory, error, mocked, load } = useInventory();
  const [filter, setFilter] = useState<"all" | AppId>("all");

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
          <button
            disabled
            title="Disponible en Fase 3"
            className="flex items-center gap-2 rounded-[var(--radius-sm)] px-4 py-2.5 text-[13px] font-semibold opacity-50"
            style={{
              background: "var(--accent)",
              color: "var(--accent-contrast)",
              boxShadow: "0 0 18px var(--accent-glow)",
            }}
          >
            <Plus size={16} strokeWidth={2.5} />
            Agregar MCP
          </button>
        </div>
      }
    >
      {/* Apps detectadas */}
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
          detail="No se encontraron MCP configurados en las apps detectadas."
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
                ...installedApps.map((a) => ({
                  value: a.id,
                  label: a.label,
                })),
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
              <McpRow key={mcp.name} mcp={mcp} />
            ))}
          </div>
        </>
      )}

      {mocked && (
        <p className="mt-6 text-center text-[12px] text-faint">
          Datos de muestra (modo browser). En la app real se leen los configs
          vía el backend.
        </p>
      )}
    </ScreenShell>
  );
}

function McpRow({ mcp }: { mcp: UnifiedMcp }) {
  const Icon = TRANSPORT_ICON[mcp.transport];
  const scopes = [...new Set(mcp.installations.map((i) => i.scope))];
  const envKeys = [...new Set(mcp.installations.flatMap((i) => i.envKeys))];

  return (
    <div className="ds-card flex items-center gap-4 !rounded-[var(--radius-md)] px-4 py-3.5">
      <div
        className="flex h-10 w-10 shrink-0 items-center justify-center rounded-[12px] border border-[var(--line)]"
        style={{ background: "var(--color-surface-2)" }}
      >
        <Icon size={18} className="text-ink-soft" />
      </div>

      <div className="min-w-0 flex-1">
        <div className="flex flex-wrap items-center gap-2">
          <span className="text-[14px] font-semibold text-ink">{mcp.name}</span>
          <Tag>{mcp.transport}</Tag>
          {scopes.map((s) => (
            <Tag key={s}>{s}</Tag>
          ))}
          {envKeys.length > 0 && (
            <Tag>
              {envKeys.length} env{envKeys.length === 1 ? "" : "s"}
            </Tag>
          )}
          {mcp.broken && <Tag tone="danger">comando no encontrado</Tag>}
        </div>
        <div className="mt-0.5 truncate font-mono text-[12px] text-faint">
          {mcp.target}
        </div>
      </div>

      <div className="flex items-center gap-3">
        <div className="flex items-center gap-1">
          {mcp.apps.map((a) => (
            <span
              key={a}
              className="rounded-full border border-[var(--line)] px-2 py-0.5 text-[10.5px] font-medium text-muted"
              style={{ background: "var(--color-surface-2)" }}
            >
              {APP_LABEL[a]}
            </span>
          ))}
        </div>
        <StatusDot broken={mcp.broken} />
      </div>
    </div>
  );
}

function StatusDot({ broken }: { broken: boolean }) {
  return (
    <span
      title={broken ? "Comando no encontrado en PATH" : "OK"}
      className="h-2.5 w-2.5 shrink-0 rounded-full"
      style={{
        background: broken ? "var(--state-danger)" : "var(--state-ok)",
        boxShadow: broken
          ? "0 0 8px var(--state-danger-glow)"
          : "0 0 8px var(--state-ok-glow)",
      }}
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
          background: danger
            ? "var(--state-danger-soft)"
            : "var(--accent-soft)",
        }}
      >
        <Icon
          size={26}
          style={{
            color: danger ? "var(--state-danger-text)" : "var(--accent-strong)",
          }}
        />
      </div>
      <h2 className="text-[16px] font-semibold text-ink">{title}</h2>
      <p className="max-w-md break-words font-mono text-[12px] leading-relaxed text-muted">
        {detail}
      </p>
    </div>
  );
}

import { useMemo, useState, type ReactNode } from "react";
import { Plus, Terminal, Globe, Radio } from "lucide-react";
import { ScreenShell } from "./ScreenShell";
import { Toggle, SegmentedControl } from "../components";

type Transport = "stdio" | "http" | "sse";
type AppTarget = "desktop" | "code";

interface SampleMcp {
  name: string;
  transport: Transport;
  command: string;
  apps: AppTarget[];
  scope: "user" | "project";
  enabled: boolean;
  broken?: boolean;
}

// Datos de muestra para el shell (Fase 1). Los reales llegan con los adapters (Fase 2).
const SAMPLE: SampleMcp[] = [
  {
    name: "context7",
    transport: "stdio",
    command: "npx -y @upstash/context7-mcp",
    apps: ["desktop", "code"],
    scope: "user",
    enabled: true,
  },
  {
    name: "playwright",
    transport: "stdio",
    command: "npx -y @playwright/mcp@latest",
    apps: ["code"],
    scope: "project",
    enabled: true,
  },
  {
    name: "notion",
    transport: "http",
    command: "https://mcp.notion.com/mcp",
    apps: ["desktop"],
    scope: "user",
    enabled: false,
  },
  {
    name: "filesystem",
    transport: "stdio",
    command: "npx -y @modelcontextprotocol/server-filesystem",
    apps: ["desktop", "code"],
    scope: "user",
    enabled: true,
    broken: true,
  },
];

const TRANSPORT_ICON: Record<Transport, typeof Terminal> = {
  stdio: Terminal,
  http: Globe,
  sse: Radio,
};

export function McpsScreen() {
  const [filter, setFilter] = useState<"all" | AppTarget>("all");
  const [rows, setRows] = useState(SAMPLE);

  const visible = useMemo(
    () =>
      filter === "all" ? rows : rows.filter((r) => r.apps.includes(filter)),
    [rows, filter],
  );

  const toggle = (name: string) =>
    setRows((rs) =>
      rs.map((r) => (r.name === name ? { ...r, enabled: !r.enabled } : r)),
    );

  return (
    <ScreenShell
      title="MCP Servers"
      subtitle="Inventario unificado de todos los MCP de tus apps de Claude."
      actions={
        <button
          className="flex items-center gap-2 rounded-[var(--radius-sm)] px-4 py-2.5 text-[13px] font-semibold transition-transform duration-[var(--dur)] ease-[var(--ease-out)] hover:scale-[1.03]"
          style={{
            background: "var(--accent)",
            color: "var(--accent-contrast)",
            boxShadow: "0 0 18px var(--accent-glow)",
          }}
        >
          <Plus size={16} strokeWidth={2.5} />
          Agregar MCP
        </button>
      }
    >
      <div className="mb-5">
        <SegmentedControl
          label="Filtrar por app"
          value={filter}
          onChange={setFilter}
          options={[
            { value: "all", label: "Todas" },
            { value: "desktop", label: "Claude Desktop" },
            { value: "code", label: "Claude Code" },
          ]}
        />
      </div>

      <div className="flex flex-col gap-2.5">
        {visible.map((mcp) => {
          const Icon = TRANSPORT_ICON[mcp.transport];
          return (
            <div
              key={mcp.name}
              className="ds-card flex items-center gap-4 !rounded-[var(--radius-md)] px-4 py-3.5"
            >
              <div
                className="flex h-10 w-10 shrink-0 items-center justify-center rounded-[12px] border border-[var(--line)]"
                style={{ background: "var(--color-surface-2)" }}
              >
                <Icon size={18} className="text-ink-soft" />
              </div>

              <div className="min-w-0 flex-1">
                <div className="flex items-center gap-2">
                  <span className="text-[14px] font-semibold text-ink">
                    {mcp.name}
                  </span>
                  <Tag>{mcp.transport}</Tag>
                  <Tag>{mcp.scope}</Tag>
                  {mcp.broken && <Tag tone="danger">comando no encontrado</Tag>}
                </div>
                <div className="mt-0.5 truncate font-mono text-[12px] text-faint">
                  {mcp.command}
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
                      {a === "desktop" ? "Desktop" : "Code"}
                    </span>
                  ))}
                </div>
                <Toggle
                  checked={mcp.enabled}
                  onChange={() => toggle(mcp.name)}
                  label={`Habilitar ${mcp.name}`}
                />
              </div>
            </div>
          );
        })}
      </div>

      <p className="mt-6 text-center text-[12px] text-faint">
        Datos de muestra · el inventario real se conecta con los adapters en
        Fase 2.
      </p>
    </ScreenShell>
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
        background: danger ? "rgba(242,85,90,0.14)" : "var(--color-surface-3)",
        color: danger ? "#ff9a9e" : "var(--color-ink-soft)",
      }}
    >
      {children}
    </span>
  );
}

/**
 * Tipos de dominio del inventario. Espejo EXACTO del DTO que devuelve el
 * comando Rust `get_inventory` (ver src-tauri/src/domain.rs). No cambiar sin
 * cambiar el backend.
 */

export type AppId = "claude-desktop" | "claude-code";
export type Scope = "user" | "project";
export type TransportKind = "stdio" | "sse" | "http" | "unknown";
export type McpStatus = "ok" | "command_not_found" | "unknown";

export interface McpInstallation {
  name: string;
  app: AppId;
  scope: Scope;
  projectPath?: string | null;
  transport: TransportKind;
  command?: string | null;
  args: string[];
  url?: string | null;
  /** Solo las claves de env; los valores nunca cruzan al frontend. */
  envKeys: string[];
  status: McpStatus;
}

export interface AppInfo {
  id: AppId;
  label: string;
  installed: boolean;
  configPath?: string | null;
  notInstalledReason?: string | null;
  /** Error al leer/parsear el config de esta app (no tumba el resto). */
  error?: string | null;
}

export interface Inventory {
  apps: AppInfo[];
  installations: McpInstallation[];
}

/** Etiquetas legibles para las apps. */
export const APP_LABEL: Record<AppId, string> = {
  "claude-desktop": "Claude Desktop",
  "claude-code": "Claude Code",
};

/**
 * MCP unificado: un servidor por nombre, con todas sus instalaciones
 * (posiblemente en varias apps/scopes). Lo arma el frontend a partir del
 * listado plano de installations.
 */
export interface UnifiedMcp {
  name: string;
  installations: McpInstallation[];
  /** transport representativo (el de la primera instalación). */
  transport: TransportKind;
  /** comando/url representativo para mostrar. */
  target: string;
  apps: AppId[];
  broken: boolean;
}

export function unify(installations: McpInstallation[]): UnifiedMcp[] {
  const byName = new Map<string, McpInstallation[]>();
  for (const inst of installations) {
    const list = byName.get(inst.name);
    if (list) list.push(inst);
    else byName.set(inst.name, [inst]);
  }

  const result: UnifiedMcp[] = [];
  for (const [name, insts] of byName) {
    const first = insts[0];
    const apps = [...new Set(insts.map((i) => i.app))];
    result.push({
      name,
      installations: insts,
      transport: first.transport,
      target: first.url ?? first.command ?? "—",
      apps,
      broken: insts.some((i) => i.status === "command_not_found"),
    });
  }
  return result.sort((a, b) => a.name.localeCompare(b.name));
}

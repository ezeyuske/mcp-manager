/**
 * Tipos de dominio del inventario. Espejo EXACTO del DTO que devuelve el
 * comando Rust `get_inventory` (ver src-tauri/src/domain.rs). No cambiar sin
 * cambiar el backend.
 */

export type AppId = "claude-desktop" | "claude-code";
export type Scope = "user" | "project";
export type TransportKind = "stdio" | "sse" | "http" | "unknown";
export type McpStatus = "ok" | "command_not_found" | "unknown" | "disabled";

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
  /** Subconjunto de envKeys vinculadas a un secreto del vault (candado). */
  vaultKeys: string[];
  status: McpStatus;
  /** Archivo donde vive (o viviría) la entrada. Rutea las mutaciones. */
  configPath: string;
  /** false = deshabilitado (guardado en el sidecar, fuera del config). */
  enabled: boolean;
  /** true solo para la entrada interna del MCP built-in de la app. No se
   *  edita ni elimina directamente: se gobierna con el toggle del built-in. */
  builtin?: boolean;
}

/** Estado del MCP built-in propio de la app (espejo de BuiltinState en Rust). */
export interface BuiltinState {
  enabled: boolean;
  /** Clientes donde está registrado (scope user). */
  targets: AppId[];
  /** Path absoluto del binario sidecar con el que se registró. */
  serverPath?: string | null;
}

/** Secreto del vault (keychain). El valor nunca viaja salvo por vault_reveal. */
export interface VaultSecretInfo {
  name: string;
  /** Cantidad de MCPs vinculados a este secreto. */
  usedBy: number;
}

/** Identifica unívocamente una entrada para las mutaciones (espejo de McpTarget en Rust). */
export interface McpTarget {
  app: AppId;
  scope: Scope;
  projectPath?: string | null;
  name: string;
}

export function targetOf(inst: McpInstallation): McpTarget {
  return {
    app: inst.app,
    scope: inst.scope,
    projectPath: inst.projectPath ?? null,
    name: inst.name,
  };
}

/** Payload de `config` para upsert_mcp (espejo de McpServerConfig en Rust). */
export interface McpServerConfigInput {
  type?: string;
  command?: string;
  args: string[];
  env: Record<string, string>;
  url?: string;
}

export type MutationAction =
  | "add"
  | "edit"
  | "delete"
  | "duplicate"
  | "enable"
  | "disable"
  | "copy"
  | "restore";

export interface MutationLog {
  id: string;
  timestamp: string;
  app: AppId;
  scope: Scope;
  filePath: string;
  action: MutationAction;
  mcpName: string;
  backupPath?: string | null;
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

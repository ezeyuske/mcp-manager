/**
 * Agrega las env vars de TODAS las instalaciones por nombre de clave, para
 * la sección Env & Secrets. "Compartido" = la misma clave aparece en >1
 * MCP; "secreto" = alguna de sus usages está vault-bindeada. Los secretos
 * del vault sin ninguna binding (huérfanos) se listan como "sin usar".
 *
 * Nota: los valores NO participan de la agregación (nunca cruzan al
 * frontend). El "compartido" es por coincidencia de NOMBRE de clave; para
 * secretos del vault eso además implica valor compartido (misma entrada de
 * keychain), que es la semántica de mínimo privilegio del proyecto.
 */
import type {
  AppId,
  McpInstallation,
  McpTarget,
  Scope,
  VaultSecretInfo,
} from "../types/inventory";
import { targetOf } from "../types/inventory";

export interface EnvUsage {
  target: McpTarget;
  app: AppId;
  scope: Scope;
  projectPath?: string | null;
  mcpName: string;
  /** La clave está vault-bindeada en este MCP (candado). */
  isVault: boolean;
}

export interface EnvAggregate {
  key: string;
  usages: EnvUsage[];
  /** Usada por más de un MCP. */
  shared: boolean;
  /** Alguna usage la gestiona el vault. */
  isSecret: boolean;
  /** Secreto del vault sin ninguna binding activa. */
  unused: boolean;
}

export function aggregateEnvs(
  installations: McpInstallation[],
  secrets: VaultSecretInfo[],
): EnvAggregate[] {
  const byKey = new Map<string, EnvUsage[]>();

  for (const inst of installations) {
    if (inst.builtin) continue; // el built-in interno no se edita acá
    for (const key of inst.envKeys) {
      const usage: EnvUsage = {
        target: targetOf(inst),
        app: inst.app,
        scope: inst.scope,
        projectPath: inst.projectPath ?? null,
        mcpName: inst.name,
        isVault: inst.vaultKeys.includes(key),
      };
      const list = byKey.get(key);
      if (list) list.push(usage);
      else byKey.set(key, [usage]);
    }
  }

  const result: EnvAggregate[] = [];
  for (const [key, usages] of byKey) {
    result.push({
      key,
      usages,
      shared: usages.length > 1,
      isSecret: usages.some((u) => u.isVault),
      unused: false,
    });
  }

  // Secretos del vault huérfanos (creados pero sin vincular a ningún MCP).
  for (const s of secrets) {
    if (!byKey.has(s.name) && s.usedBy === 0) {
      result.push({
        key: s.name,
        usages: [],
        shared: false,
        isSecret: true,
        unused: true,
      });
    }
  }

  return result.sort((a, b) => a.key.localeCompare(b.key));
}

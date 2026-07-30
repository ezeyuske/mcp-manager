/**
 * Orquestación compartida de edición de env vars / secretos, usada tanto
 * por el formulario del MCP como por la sección Env & Secrets.
 *
 * El backend nunca precarga valores: se leen on-demand (`read_env_value`
 * para inline, `vault_reveal` para secretos). Por eso las ediciones se
 * aplican QUIRÚRGICAMENTE por clave (`set_mcp_env`), nunca reemplazando el
 * mapa `env` completo. El orden evita clobbering:
 *   1. (fuera de acá) campos de config vía `upsert_mcp` con env vacío.
 *   2. Unbinds primero (secreto→inline y renames de secreto): liberan la
 *      clave del vault antes de reescribirla inline.
 *   3. UN solo `set_mcp_env` con todos los adds/edits/removes inline.
 *   4. Locks al final (`vault_set_secret` + `bind_env_secret`).
 * Cada paso es atómico y validado; si uno intermedio falla, el config
 * queda en un estado válido intermedio (nunca corrupto) y recargamos el
 * inventario para reflejar el estado real.
 */
import { invoke } from "@tauri-apps/api/core";
import { isTauri } from "./tauri";
import { useInventory } from "../store/inventory";
import { useVault } from "../store/vault";
import { useToast } from "../store/toast";
import type { McpTarget, VaultSecretInfo } from "../types/inventory";

/**
 * Fila de edición de env. Estructuralmente idéntica al `KeyValue` del
 * `KeyValueEditor` (por eso son intercambiables sin casts). Los campos de
 * bookkeeping (`originalKey`/`originalSecret`/`loaded`/`dirty`) los usa el
 * diff; están ausentes en filas nuevas.
 */
export interface EnvRow {
  key: string;
  value: string;
  /** Lock: la fila se gestiona como secreto del vault (keychain). */
  secret?: boolean;
  /** Clave tal como está en disco al abrir el editor. undefined = fila nueva. */
  originalKey?: string;
  /** Si la clave estaba vault-bindeada en disco. */
  originalSecret?: boolean;
  /** El valor fue cargado desde el backend o tipeado por el usuario. */
  loaded?: boolean;
  /** El usuario editó el valor (fuerza reescritura). */
  dirty?: boolean;
}

/** Fila seed para el modo edición: valor sin cargar (on-demand). */
export function seedRow(key: string, isVault: boolean): EnvRow {
  return {
    key,
    value: "",
    secret: isVault,
    originalKey: key,
    originalSecret: isVault,
    loaded: false,
    dirty: false,
  };
}

interface Plan {
  unbinds: string[];
  upserts: Record<string, string>;
  removals: string[];
  secretWrites: { name: string; value: string }[];
  binds: { envKey: string; name: string }[];
}

/** Lee on-demand el valor actual de una fila existente (secreto o inline). */
async function ensureValue(target: McpTarget, row: EnvRow): Promise<string> {
  if (row.loaded) return row.value;
  const originalKey = row.originalKey as string;
  if (row.originalSecret) {
    return invoke<string>("vault_reveal", { name: originalKey });
  }
  return invoke<string>("read_env_value", { target, envKey: originalKey });
}

async function buildPlan(
  target: McpTarget,
  initial: EnvRow[],
  rows: EnvRow[],
): Promise<Plan> {
  const plan: Plan = {
    unbinds: [],
    upserts: {},
    removals: [],
    secretWrites: [],
    binds: [],
  };

  const currentOriginals = new Set(
    rows.map((r) => r.originalKey).filter((k): k is string => !!k),
  );

  // Filas quitadas por el usuario (estaban en disco, ya no están).
  for (const o of initial) {
    const key = o.originalKey as string;
    if (!currentOriginals.has(key)) {
      if (o.originalSecret) plan.unbinds.push(key);
      else plan.removals.push(key);
    }
  }

  for (const r of rows) {
    const key = r.key.trim();
    if (!key) continue;

    // Fila nueva.
    if (!r.originalKey) {
      if (r.secret) {
        if (r.value !== "") {
          plan.secretWrites.push({ name: key, value: r.value });
          plan.binds.push({ envKey: key, name: key });
        }
      } else {
        plan.upserts[key] = r.value;
      }
      continue;
    }

    const originalKey = r.originalKey;
    const wasSecret = !!r.originalSecret;
    const renamed = key !== originalKey;

    if (wasSecret && r.secret) {
      if (renamed) {
        const value = await ensureValue(target, r);
        plan.unbinds.push(originalKey);
        plan.secretWrites.push({ name: key, value });
        plan.binds.push({ envKey: key, name: key });
      } else if (r.dirty) {
        // Rotación del secreto (posiblemente compartido → afecta a todos).
        plan.secretWrites.push({ name: originalKey, value: r.value });
      }
    } else if (wasSecret && !r.secret) {
      // Unlock: revelar valor y reescribirlo inline.
      const value = await ensureValue(target, r);
      plan.unbinds.push(originalKey);
      plan.upserts[key] = value;
    } else if (!wasSecret && r.secret) {
      // Lock: mover el valor al keychain.
      const value = await ensureValue(target, r);
      if (renamed) plan.removals.push(originalKey);
      plan.secretWrites.push({ name: key, value });
      plan.binds.push({ envKey: key, name: key });
    } else {
      // inline → inline.
      if (renamed) {
        const value = await ensureValue(target, r);
        plan.removals.push(originalKey);
        plan.upserts[key] = value;
      } else if (r.dirty) {
        plan.upserts[key] = r.value;
      }
    }
  }

  return plan;
}

async function reload() {
  await useInventory.getState().load();
  await useVault.getState().load();
}

/**
 * Aplica el diff de env entre `initial` (estado al abrir) y `rows` (estado
 * del editor). Devuelve true si todo salió bien. Muestra un toast de
 * error y recarga el inventario si algún paso falla.
 */
export async function applyEnvEdits(
  target: McpTarget,
  initial: EnvRow[],
  rows: EnvRow[],
): Promise<boolean> {
  if (!isTauri()) {
    useToast.getState().push("info", "No disponible en modo browser (sin backend).");
    return false;
  }

  try {
    const plan = await buildPlan(target, initial, rows);

    // 2. Unbinds primero.
    for (const envKey of plan.unbinds) {
      await invoke("unbind_env_secret", { target, envKey });
    }

    // 3. Un solo set_mcp_env con todos los cambios inline.
    if (Object.keys(plan.upserts).length > 0 || plan.removals.length > 0) {
      await invoke("set_mcp_env", {
        target,
        upserts: plan.upserts,
        removals: plan.removals,
      });
    }

    // 4. Locks al final: escribir el secreto, luego bindear.
    for (const w of plan.secretWrites) {
      await invoke("vault_set_secret", { name: w.name, value: w.value });
    }
    for (const b of plan.binds) {
      await invoke("bind_env_secret", {
        target,
        envKey: b.envKey,
        secretName: b.name,
      });
    }

    await reload();
    return true;
  } catch (err) {
    useToast.getState().push("error", String(err));
    // El config pudo quedar en un estado intermedio válido: reflejarlo.
    await reload();
    return false;
  }
}

/**
 * Nombres de secretos que una edición ROTARÍA globalmente: filas que pasan
 * a ser secreto (lock nuevo o rename) cuyo nombre ya existe en el vault y
 * es usado por otros MCPs. Se usa para avisar antes de guardar.
 */
export function lockCollisions(
  rows: EnvRow[],
  secrets: VaultSecretInfo[],
): { name: string; usedBy: number }[] {
  const existing = new Map(secrets.map((s) => [s.name, s.usedBy]));
  const out: { name: string; usedBy: number }[] = [];
  for (const r of rows) {
    const key = r.key.trim();
    if (!key || !r.secret) continue;
    const becomingSecret =
      !r.originalKey || !r.originalSecret || key !== r.originalKey;
    const usedBy = existing.get(key);
    if (becomingSecret && usedBy !== undefined && usedBy > 0) {
      out.push({ name: key, usedBy });
    }
  }
  return out;
}

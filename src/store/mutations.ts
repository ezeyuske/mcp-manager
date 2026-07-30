import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import { isTauri } from "../lib/tauri";
import { useInventory } from "./inventory";
import { useToast } from "./toast";
import type {
  AppId,
  McpServerConfigInput,
  McpTarget,
  Scope,
} from "../types/inventory";

interface MutationState {
  busy: boolean;
  upsert: (target: McpTarget, config: McpServerConfigInput) => Promise<boolean>;
  remove: (target: McpTarget) => Promise<boolean>;
  duplicate: (target: McpTarget, newName: string) => Promise<boolean>;
  setEnabled: (target: McpTarget, enabled: boolean) => Promise<boolean>;
  copy: (
    source: McpTarget,
    destApp: AppId,
    destScope: Scope,
    destProjectPath: string | null,
  ) => Promise<boolean>;
  registerProject: (path: string) => Promise<void>;
  /** Lee on-demand el valor inline de una env var. Silencioso (sin toast
   *  ni reload); usado por el editor para revelar valores bajo demanda.
   *  Devuelve null en browser o ante error (con toast de error). */
  readEnvValue: (target: McpTarget, envKey: string) => Promise<string | null>;
}

/** Corre una mutación: no-op informativo en browser; invoke + toast + reload en Tauri. */
async function run(
  set: (p: Partial<MutationState>) => void,
  okMessage: string,
  command: string,
  args: Record<string, unknown>,
): Promise<boolean> {
  const toast = useToast.getState();

  if (!isTauri()) {
    toast.push("info", "No disponible en modo browser (sin backend).");
    return false;
  }

  set({ busy: true });
  try {
    await invoke(command, args);
    await useInventory.getState().load();
    toast.push("success", okMessage);
    return true;
  } catch (err) {
    toast.push("error", String(err));
    return false;
  } finally {
    set({ busy: false });
  }
}

export const useMutations = create<MutationState>((set) => ({
  busy: false,

  upsert: (target, config) =>
    run(set, `MCP "${target.name}" guardado.`, "upsert_mcp", {
      target,
      config,
    }),

  remove: (target) =>
    run(set, `MCP "${target.name}" eliminado.`, "delete_mcp", { target }),

  duplicate: (target, newName) =>
    run(set, `Duplicado como "${newName}".`, "duplicate_mcp", {
      target,
      newName,
    }),

  setEnabled: (target, enabled) =>
    run(
      set,
      `MCP "${target.name}" ${enabled ? "habilitado" : "deshabilitado"}.`,
      "set_mcp_enabled",
      { target, enabled },
    ),

  copy: (source, destApp, destScope, destProjectPath) =>
    run(
      set,
      `"${source.name}" copiado a ${destApp === "claude-desktop" ? "Claude Desktop" : "Claude Code"}.`,
      "copy_mcp",
      { source, destApp, destScope, destProjectPath },
    ),

  registerProject: async (path) => {
    if (!isTauri()) return;
    try {
      await invoke("register_project_dir", { path });
      await useInventory.getState().load();
    } catch (err) {
      useToast.getState().push("error", String(err));
    }
  },

  readEnvValue: async (target, envKey) => {
    if (!isTauri()) return "•••• (modo browser)";
    try {
      return await invoke<string>("read_env_value", { target, envKey });
    } catch (err) {
      useToast.getState().push("error", String(err));
      return null;
    }
  },
}));

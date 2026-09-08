import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import { isTauri } from "../lib/tauri";
import { useInventory } from "./inventory";
import { useToast } from "./toast";
import { APP_LABEL } from "../types/inventory";
import type {
  AppId,
  McpServerConfigInput,
  McpTarget,
  RenameReport,
  Scope,
} from "../types/inventory";

interface MutationState {
  busy: boolean;
  upsert: (target: McpTarget, config: McpServerConfigInput) => Promise<boolean>;
  remove: (target: McpTarget) => Promise<boolean>;
  duplicate: (target: McpTarget, newName: string) => Promise<boolean>;
  /** Renombra un MCP en TODAS las installations recibidas. Devuelve true
   *  solo si se renombraron todas: un parcial reporta por toast qué
   *  targets quedaron sin renombrar y deja el modal abierto. */
  rename: (targets: McpTarget[], newName: string) => Promise<boolean>;
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

  /** No usa `run`: ese helper asume éxito binario y un único toast, y un
   *  rename multi-archivo puede terminar parcial. */
  rename: async (targets, newName) => {
    const toast = useToast.getState();

    if (!isTauri()) {
      toast.push("info", "No disponible en modo browser (sin backend).");
      return false;
    }

    const oldName = targets[0]?.name ?? "";
    set({ busy: true });
    try {
      const report = await invoke<RenameReport>("rename_mcp", {
        targets,
        newName,
      });
      await useInventory.getState().load();

      if (report.failed.length === 0) {
        toast.push(
          "success",
          `"${oldName}" renombrado a "${newName}" en ${report.renamed.length} ` +
            `${report.renamed.length === 1 ? "instalación" : "instalaciones"}.`,
        );
        return true;
      }

      // Parcial: decir exactamente qué quedó afuera, no un "listo".
      const detail = report.failed
        .map((f) => `${APP_LABEL[f.target.app]} (${f.target.scope}): ${f.error}`)
        .join(" · ");
      toast.push(
        "error",
        report.renamed.length > 0
          ? `Renombrado en ${report.renamed.length} de ${
              report.renamed.length + report.failed.length
            } instalaciones. Falló: ${detail}`
          : `No se pudo renombrar: ${detail}`,
      );
      return false;
    } catch (err) {
      toast.push("error", String(err));
      return false;
    } finally {
      set({ busy: false });
    }
  },

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

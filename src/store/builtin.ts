import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import { isTauri } from "../lib/tauri";
import { useInventory } from "./inventory";
import { useToast } from "./toast";
import type { AppId, BuiltinState } from "../types/inventory";

type Status = "idle" | "loading" | "ready" | "error";

interface BuiltinStore {
  status: Status;
  state: BuiltinState | null;
  error: string | null;
  busy: boolean;
  load: () => Promise<void>;
  /** Activa/desactiva el built-in en los clientes elegidos. */
  setEnabled: (targets: AppId[], enabled: boolean) => Promise<boolean>;
}

/** Estado default (apagado) para el modo browser sin backend. */
const MOCK_STATE: BuiltinState = { enabled: false, targets: [], serverPath: null };

export const useBuiltin = create<BuiltinStore>((set, get) => ({
  status: "idle",
  state: null,
  error: null,
  busy: false,

  load: async () => {
    set({ status: "loading", error: null });
    if (!isTauri()) {
      set({ status: "ready", state: MOCK_STATE });
      return;
    }
    try {
      const state = await invoke<BuiltinState>("get_builtin_status");
      set({ status: "ready", state });
    } catch (err) {
      set({ status: "error", error: String(err) });
    }
  },

  setEnabled: async (targets, enabled) => {
    const toast = useToast.getState();
    if (!isTauri()) {
      toast.push("info", "No disponible en modo browser (sin backend).");
      return false;
    }
    set({ busy: true });
    try {
      await invoke("set_builtin_enabled", { targets, enabled });
      await get().load();
      // El inventario también cambia (la entrada se registra/quita del config).
      await useInventory.getState().load();
      toast.push(
        "success",
        enabled && targets.length > 0
          ? "MCP de mcp-manager activado."
          : "MCP de mcp-manager desactivado.",
      );
      return true;
    } catch (err) {
      toast.push("error", String(err));
      return false;
    } finally {
      set({ busy: false });
    }
  },
}));
